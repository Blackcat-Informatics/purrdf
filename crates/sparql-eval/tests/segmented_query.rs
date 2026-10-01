// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified operator residency over persistent global IDs and sparse eviction.

mod support;

use support::segmented::{CEILING, QUERY, fixture, fixture_with_layout, open};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use purrdf_core::{
    DatasetView, GraphMatch, SegmentedBytes, SegmentedError, SegmentedProvider,
    SegmentedReadLimits, SegmentedSession, SegmentedSnapshot, SparqlRequest, SparqlResult,
    TermValue,
};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine, QueryOptions};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[derive(Debug)]
struct RefusingProvider {
    bytes: SegmentedBytes,
    refuse: AtomicBool,
    operation: &'static str,
}
impl SegmentedProvider for RefusingProvider {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len()
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        if self.refuse.load(Ordering::Relaxed) {
            return Err(SegmentedError::Provider {
                operation: self.operation,
                host_code: Some(5),
            });
        }
        self.bytes.read_at(position, output)
    }
}

#[test]
fn dense_pages_price_only_the_exact_subject_range_and_preserve_source_refusal() {
    let (image, resident) = fixture_with_layout(8192, 128);
    let source = open(&image, CEILING);
    let target = source
        .term_id_by_value(&TermValue::iri("https://example.org/target"))
        .unwrap()
        .unwrap();
    let p = source
        .term_id_by_value(&TermValue::iri("https://example.org/p"))
        .unwrap()
        .unwrap();
    let q = source
        .term_id_by_value(&TermValue::iri("https://example.org/q"))
        .unwrap()
        .unwrap();
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(p), None, GraphMatch::Default),
        1
    );
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(q), None, GraphMatch::Default),
        1
    );
    assert!(source.cardinality_estimate(None, Some(p), None, GraphMatch::Default) >= 201);
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let expected = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let answer = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let (
        SparqlResult::Solutions { rows: found, .. },
        SparqlResult::Solutions { rows: expected, .. },
    ) = (&answer.result, &expected)
    else {
        panic!("SELECT returns rows")
    };
    assert_eq!(found, expected);

    let provider = Arc::new(RefusingProvider {
        bytes: image.provider(),
        refuse: AtomicBool::new(false),
        operation: "selective cardinality test",
    });
    let source = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8),
    )
    .unwrap();
    provider.refuse.store(true, Ordering::Relaxed);
    assert_eq!(
        source.cardinality_estimate(Some(target), Some(p), None, GraphMatch::Default),
        0
    );
    assert!(matches!(
        source.read_error(),
        Some(SegmentedError::Provider { .. })
    ));
    assert!(matches!(
        engine.query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY),
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Provider { .. },
            ..
        })
    ));
}

#[test]
fn long_host_refusal_keeps_its_typed_cause_without_unbounded_report_allocation() {
    let (image, _) = fixture();
    // Host-owned static label is prepared outside the execution allocation window.
    let operation: &'static str = Box::leak("漢字操作 ".repeat(20_000).into_boxed_str());
    let provider = Arc::new(RefusingProvider {
        bytes: image.provider(),
        refuse: AtomicBool::new(false),
        operation,
    });
    let engine = NativeSparqlEngine::new();
    engine.prepare_query(QUERY, None).unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let source = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(CEILING, 2, 2048, 20_000_000, 8),
    )
    .unwrap();
    provider.refuse.store(true, Ordering::Relaxed);
    let outcome = engine.query_governed_fallible_view(
        &source,
        SparqlRequest {
            query: QUERY,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
        &purrdf_sparql_eval::QueryGovernors::METERED,
    );
    let ledger = source.evidence().peak_bytes();
    let measured = window.close();
    assert!(matches!(outcome, Err(FallibleSparqlError::Operational {
        error: SegmentedError::Provider { operation: found, host_code: Some(5) }, ..
    }) if found == operation));
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= ledger);
    assert!(ledger <= CEILING);
}

#[test]
fn selective_join_matches_resident_and_actual_peak_stays_below_shared_ledger() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let expected = engine
        .query_prepared(&resident, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let source = open(&image, CEILING);
    let answer = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let SparqlResult::Solutions {
        variables: actual_variables,
        rows: actual_rows,
        ..
    } = &answer.result
    else {
        panic!("SELECT returns rows")
    };
    let SparqlResult::Solutions {
        variables: expected_variables,
        rows: expected_rows,
        ..
    } = &expected
    else {
        panic!("SELECT returns rows")
    };
    assert_eq!(actual_variables, expected_variables);
    assert_eq!(actual_rows, expected_rows);
    assert!(matches!(&answer.result, SparqlResult::Solutions { rows, .. } if rows.len() == 1));
    assert!(answer.evidence.evictions() > 0);
    let ledger = answer.evidence.peak_bytes();
    let measured = window.close();
    assert!(
        u64::try_from(measured.peak_working_bytes).unwrap() <= ledger,
        "actual {} exceeds shared reservation {ledger}",
        measured.peak_working_bytes
    );
    assert!(ledger <= CEILING);
    assert!(source.read_error().is_none());
}

#[test]
fn tiny_capacity_and_unpriced_order_refuse_before_any_row_probe_or_output() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).unwrap();
    let small = open(&image, 80_000);
    let before = small.evidence().request_count();
    let refused = engine.query_prepared_fallible_view(&small, &prepared, &[], QueryOptions::EMPTY);
    assert!(matches!(
        refused,
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Residency { .. },
            ..
        })
    ));
    assert_eq!(small.evidence().request_count(), before);
    let source = open(&image, CEILING);
    let ordered = engine
        .prepare_query(&format!("{QUERY} ORDER BY ?label"), None)
        .unwrap();
    let before = source.evidence().request_count();
    let error = engine
        .query_prepared_fallible_view(&source, &ordered, &[], QueryOptions::EMPTY)
        .unwrap_err();
    assert!(
        matches!(error, FallibleSparqlError::Query { diagnostic, .. } if diagnostic.code == "native-sparql-workspace-unpriced")
    );
    assert_eq!(source.evidence().request_count(), before);
    assert!(source.read_error().is_none());
    // The raw evaluator cannot bypass the engine's held drain reservation.
    let mut ctx = purrdf_sparql_eval::EvalCtx::new(&source);
    let error = purrdf_sparql_eval::evaluate_query(prepared.query(), &mut ctx).unwrap_err();
    assert!(matches!(
        error,
        purrdf_sparql_eval::EvalError::WorkspaceUnpriced(_)
    ));
    assert_eq!(source.evidence().request_count(), before);
}

#[test]
fn scoped_and_prepared_engine_egresses_hold_the_reservation_through_consumption() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let source = open(&image, CEILING);
    let before = source.evidence().live_bytes();
    let mut execution = engine
        .prepare_execution(QUERY, None, &[], QueryOptions::EMPTY)
        .unwrap();
    engine
        .execute_fallible(&mut execution, &source, QueryOptions::EMPTY, |outcome| {
            assert!(source.evidence().live_bytes() > before + 50_000);
            let purrdf_sparql_eval::InternedOutcome::Solutions(rows) = outcome else {
                panic!("SELECT returns rows")
            };
            assert_eq!(rows.rows().len(), 1);
        })
        .unwrap();
    let request = SparqlRequest {
        query: QUERY,
        base_iri: None,
        substitutions: &[],
    };
    let answer = engine
        .query_fallible_view(&open(&image, CEILING), request, QueryOptions::EMPTY)
        .unwrap();
    assert!(matches!(answer.result, SparqlResult::Solutions { rows, .. } if rows.len() == 1));
}

#[test]
fn governed_refusal_prices_its_reporting_owner_before_allocating() {
    let (image, _) = fixture();
    let initial = open(&image, CEILING);
    let opening_bytes = initial.evidence().live_bytes();
    drop(initial);
    let engine = NativeSparqlEngine::new();
    engine.prepare_query(QUERY, None).unwrap();
    let ordered = format!("{QUERY} ORDER BY ?label");
    engine.prepare_query(&ordered, None).unwrap();
    for (headroom, query) in [(512, QUERY), (16_384, ordered.as_str())] {
        let ceiling = opening_bytes + headroom;
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let source = open(&image, ceiling);
        let before = source.evidence().request_count();
        let outcome = engine.query_governed_fallible_view(
            &source,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &purrdf_sparql_eval::QueryGovernors::METERED,
        );
        let ledger = source.evidence().peak_bytes();
        let measured = window.close();
        assert!(outcome.is_err());
        assert_eq!(source.evidence().request_count(), before);
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= ledger,
            "refusal allocated {} beyond charged {ledger}",
            measured.peak_working_bytes
        );
        assert!(ledger <= ceiling);
        if headroom == 512 {
            assert!(matches!(
                outcome,
                Err(FallibleSparqlError::Operational {
                    error: SegmentedError::Residency { .. },
                    ..
                })
            ));
        } else {
            assert!(
                matches!(outcome, Err(FallibleSparqlError::Query { diagnostic, .. })
                if diagnostic.code == "native-sparql-workspace-unpriced")
            );
        }
    }
}

#[test]
fn unpriced_construct_refuses_before_allocating_a_destination_mint_prefix() {
    let (image, _) = fixture();
    let source = open(&image, CEILING);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(
            "CONSTRUCT { _:fresh <http://example.org/p> ?o } WHERE { ?s ?p ?o }",
            None,
        )
        .unwrap();
    let state = Arc::new(purrdf_sparql_eval::GovernorState::new(
        &purrdf_sparql_eval::QueryGovernors::METERED,
    ));
    let mut destination = purrdf_core::RdfDatasetBuilder::new();
    let blank = destination.intern_blank("existing", purrdf_core::BlankScope::DEFAULT);
    let predicate = destination.intern_iri("http://example.org/p");
    let object = destination.intern_iri("http://example.org/o");
    destination.push_quad(blank, predicate, object, None);
    let authored_prefix = "x".repeat(100_000);
    let options = QueryOptions::EMPTY.with_bnode_mint_prefix(Some(&authored_prefix));
    let before = source.evidence().request_count();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refused = engine.construct_prepared_fallible_in_operation_into_view(
        &source,
        &prepared,
        &[],
        options,
        &state,
        &mut destination,
    );
    let allocations = window.close();
    assert!(
        matches!(refused, Err(FallibleSparqlError::Query { diagnostic, .. }) if diagnostic.code == "native-sparql-workspace-unpriced")
    );
    assert!(
        allocations.peak_working_bytes <= 8192,
        "unpriced request allocated authored mint prefix: {}",
        allocations.peak_working_bytes
    );
    assert_eq!(source.evidence().request_count(), before);
    assert_eq!(destination.freeze().unwrap().quad_count(), 1);
    assert!(source.read_error().is_none());
}

#[test]
fn unpriced_request_inputs_refuse_before_copying_parameter_metadata() {
    let (image, _) = fixture();
    let source = open(&image, CEILING);
    let engine = NativeSparqlEngine::new();
    let long_prefix = "x".repeat(100_000);
    let substitutions: Vec<_> = (0..20_000)
        .map(|i| {
            (
                format!("parameter{i}"),
                TermValue::iri("http://example.org/o"),
            )
        })
        .collect();
    for (values, options) in [
        (substitutions.as_slice(), QueryOptions::EMPTY),
        (
            &[][..],
            QueryOptions::EMPTY.with_bnode_mint_prefix(Some(&long_prefix)),
        ),
    ] {
        let request = SparqlRequest {
            query: QUERY,
            base_iri: None,
            substitutions: values,
        };
        let before = source.evidence().request_count();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let ordinary = engine.query_fallible_view(&source, request, options);
        let governed = engine.query_governed_fallible_view(
            &source,
            request,
            options,
            &purrdf_sparql_eval::QueryGovernors::METERED,
        );
        let allocations = window.close();
        assert!(
            matches!(ordinary, Err(FallibleSparqlError::Query { diagnostic, .. }) if diagnostic.code == "native-sparql-workspace-unpriced")
        );
        assert!(
            matches!(governed, Err(FallibleSparqlError::Query { diagnostic, .. }) if diagnostic.code == "native-sparql-workspace-unpriced")
        );
        assert!(
            allocations.peak_working_bytes <= 8192,
            "known unpriced inputs copied caller payload: {}",
            allocations.peak_working_bytes
        );
        assert_eq!(source.evidence().request_count(), before);
        assert_eq!(engine.cached_plan_count(), 0);
        assert!(source.read_error().is_none());
    }
    assert!(
        engine
            .query_prepared_fallible_view(
                &source,
                &engine.prepare_query(QUERY, None).unwrap(),
                &[],
                QueryOptions::EMPTY
            )
            .is_ok()
    );
}
