// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::{ExecutionLimits, ExecutionPolicy, MetricWorkObserver};

fn literal(text: &str, json: bool) -> TermValue {
    TermValue::typed_literal(
        text,
        if json {
            purrdf_iri::vocab::ogc::geo::GEO_JSON_LITERAL
        } else {
            purrdf_iri::vocab::ogc::geo::WKT_LITERAL
        },
    )
}
struct CancelAfter {
    calls: usize,
    at: usize,
    work: u64,
}
impl MetricWorkObserver for CancelAfter {
    fn charge_chunk(&mut self, work: u64, _growth: u64) -> Result<(), GeoError> {
        self.calls += 1;
        self.work += work;
        if self.calls >= self.at {
            Err(GeoError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn borrowed_term_capacities_are_admitted_before_parser_or_walk_allocation() {
    let vocab = crate::standard_vocabulary();
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_workspace_bytes: 8_192,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    for field in 0..3 {
        let mut lexical_form = String::new();
        let mut datatype = String::new();
        let mut language = String::new();
        match field {
            0 => lexical_form.reserve_exact(131_072),
            1 => datatype.reserve_exact(131_072),
            _ => language.reserve_exact(131_072),
        }
        lexical_form.push_str("POINT(0 0)");
        datatype.push_str(purrdf_iri::vocab::ogc::geo::WKT_LITERAL);
        let source = TermValue::Literal {
            lexical_form,
            datatype,
            language: Some(language),
            direction: None,
        };
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = geometry_arg_in_policy(vocab, &source, policy);
        let allocations = window.close();
        assert!(matches!(
            result,
            Err(GeoError::MemoryExhausted { limit: 8_192 })
        ));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
    }
    let mut nested = TermValue::Iri(String::new());
    for _ in 0..64 {
        nested = TermValue::Triple {
            s: nested.into(),
            p: TermValue::Iri(String::new()).into(),
            o: TermValue::Iri(String::new()).into(),
        };
    }
    let mut observer = CancelAfter {
        calls: 0,
        at: 1,
        work: 0,
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let cancelled = term_source_metered(&nested, policy, &mut observer);
    let allocations = window.close();
    assert_eq!(cancelled, Err(GeoError::Cancelled));
    assert_eq!(observer.calls, 1);
    assert_eq!(allocations.allocations, 0);
    let tiny = ExecutionPolicy::new(ExecutionLimits {
        max_workspace_bytes: 2_048,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = term_source_in_policy(&nested, tiny);
    let allocations = window.close();
    assert_eq!(result, Err(GeoError::MemoryExhausted { limit: 2_048 }));
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn term_source_receipt_matches_exact_ownership_and_metered_counts() {
    struct Counts {
        work: u64,
        bytes: u64,
    }
    impl MetricWorkObserver for Counts {
        fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), GeoError> {
            self.work += work;
            self.bytes += bytes;
            Ok(())
        }
    }
    let source = TermValue::Iri(String::with_capacity(4_096));
    let mut counts = Counts { work: 0, bytes: 0 };
    let receipt = term_source_metered(&source, ExecutionPolicy::geometry(), &mut counts).unwrap();
    assert_eq!(
        receipt,
        term_source_in_policy(&source, ExecutionPolicy::geometry()).unwrap()
    );
    assert_eq!(receipt.work_items(), 2);
    assert_eq!(
        receipt.workspace_bytes(),
        1_024 + source.owned_heap_bytes().unwrap() as u64
    );
    assert_eq!(counts.work, receipt.work_items());
    assert_eq!(counts.bytes, receipt.workspace_bytes());
}

#[test]
fn long_invalid_datatype_refuses_before_interpretation_or_error_copy() {
    let source = TermValue::typed_literal("POINT(0 0)", "x".repeat(131_072));
    let vocab = crate::standard_vocabulary();
    for limits in [
        ExecutionLimits {
            max_work_items: 64,
            ..ExecutionLimits::GEOMETRY
        },
        ExecutionLimits {
            max_workspace_bytes: 8_192,
            ..ExecutionLimits::GEOMETRY
        },
    ] {
        let policy = ExecutionPolicy::new(limits).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = geometry_arg_in_policy(vocab, &source, policy);
        let allocations = window.close();
        if limits.max_work_items == 64 {
            assert_eq!(result, Err(GeoError::WorkExhausted { limit: 64 }));
        } else {
            assert_eq!(result, Err(GeoError::MemoryExhausted { limit: 8_192 }));
        }
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
    }
}

#[test]
fn scientific_coordinate_admission_precedes_large_integer_construction() {
    for source in [
        literal("POINT(1e100000 0)", false),
        literal(r#"{"coordinates":[1e100000,0],"type":"Point"}"#, true),
    ] {
        assert!(matches!(
            geometry_arg_in_policy(
                crate::standard_vocabulary(),
                &source,
                ExecutionPolicy::geometry()
            ),
            Err(GeoError::WorkExhausted { .. })
        ));
    }
}

#[test]
fn metered_source_refusals_preserve_original_cancellation_inside_lexical_runs() {
    let wkt = literal(&format!("POINT({} 0)", "1".repeat(10000)), false);
    let json = literal(
        &format!(
            r#"{{"foreign":"{}","type":"Point","coordinates":[0,0]}}"#,
            "x".repeat(10000)
        ),
        true,
    );
    for source in [wkt, json] {
        let mut observer = CancelAfter {
            calls: 0,
            // Entry and three owned-source metadata chunks precede the
            // original lexical polls; refuse during the long lexical run.
            at: 5,
            work: 0,
        };
        assert_eq!(
            geometry_arg_metered(
                crate::standard_vocabulary(),
                &source,
                ExecutionPolicy::geometry(),
                &mut observer
            ),
            Err(GeoError::Cancelled)
        );
        assert_eq!(observer.calls, 5);
        assert!(
            observer.work <= 64,
            "cancelled before the complete long token"
        );
    }
}

#[test]
fn complete_carrier_limits_do_not_change_original_coordinates_or_serialization() {
    let source = literal("LINESTRING(0.123456789012345678901234567890 1,2 3)", false);
    let legacy = geometry_arg(crate::standard_vocabulary(), &source).expect("legacy source");
    let admitted = geometry_arg_in_policy(
        crate::standard_vocabulary(),
        &source,
        ExecutionPolicy::geometry(),
    )
    .expect("admitted source");
    assert_eq!(admitted.literal(), &legacy);
    assert!(admitted.receipt().work_items() > 0);
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_output_elements: 1,
        ..ExecutionLimits::GEOMETRY
    })
    .expect("policy");
    assert_eq!(
        geometry_arg_in_policy(crate::standard_vocabulary(), &source, policy),
        Err(GeoError::OutputExhausted { limit: 1 })
    );
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_workspace_bytes: 1,
        ..ExecutionLimits::GEOMETRY
    })
    .expect("memory policy");
    assert_eq!(
        geometry_arg_in_policy(crate::standard_vocabulary(), &source, policy),
        Err(GeoError::MemoryExhausted { limit: 1 })
    );
}

#[test]
fn carrier_term_refuses_datatype_storage_before_allocating_and_drops_source() {
    let lexical = String::from("POINT (0 0)");
    let lexical_bytes = lexical.capacity() as u64;
    let required = (size_of::<TermValue>() + purrdf_iri::vocab::ogc::geo::WKT_LITERAL.len()) as u64;
    let limit = lexical_bytes + required - 1;
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_workspace_bytes: limit,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context =
        crate::MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
    context.admit_workspace(lexical_bytes).unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refused = term_in_context(lexical, CarrierFormat::Wkt, &mut context);
    let measured = window.close();
    assert_eq!(refused, Err(GeoError::MemoryExhausted { limit }));
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(context.remaining_workspace(), limit);
}

#[test]
fn carrier_term_moves_the_writer_string_and_reports_exact_new_work() {
    let lexical = String::from("POINT (0.1234567890123456789 0)");
    let original = lexical.as_ptr();
    let lexical_bytes = lexical.capacity() as u64;
    let datatype = purrdf_iri::vocab::ogc::geo::WKT_LITERAL;
    let mut context = crate::MetricContext::wgs84().unwrap();
    context.admit_workspace(lexical_bytes).unwrap();
    let mut observer = CancelAfter {
        calls: 0,
        at: usize::MAX,
        work: 0,
    };
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let term =
        term_in_context_metered(lexical, CarrierFormat::Wkt, &mut context, &mut observer).unwrap();
    let measured = window.close();
    let TermValue::Literal {
        lexical_form,
        datatype: output_datatype,
        ..
    } = &term
    else {
        panic!("carrier output is a typed literal")
    };
    assert_eq!(lexical_form.as_ptr(), original);
    assert_eq!(output_datatype, datatype);
    assert_eq!(measured.allocations, 1);
    assert_eq!(measured.requested_bytes, datatype.len() as u64);
    assert_eq!(observer.work, datatype.len() as u64 + 2);
    assert_eq!(observer.work, context.work_items());
    drop(term);
    context
        .release_workspace(lexical_bytes + (size_of::<TermValue>() + datatype.len()) as u64)
        .unwrap();
    assert_eq!(
        context.remaining_workspace(),
        context.policy().limits().max_workspace_bytes
    );
}
