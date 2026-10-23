// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::index::{GeoIndex, GeoIndexConfig, GraphSelector};
use crate::{
    ExecutionLimits, ExecutionPolicy, GeoError, MetricContext, MetricWorkObserver, SpatialRelation,
};
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, TermBox, TermFactory as _, TermValue,
};
use purrdf_iri::vocab::ogc;
use std::sync::Arc;

struct Receipt {
    work: u64,
    peak: u64,
    calls: u64,
    stop: Option<u64>,
}
impl MetricWorkObserver for Receipt {
    fn charge_chunk(&mut self, work: u64, peak: u64) -> Result<(), GeoError> {
        self.work += work;
        self.peak += peak;
        self.calls += 1;
        if self.stop == Some(self.calls) {
            Err(GeoError::Cancelled)
        } else {
            Ok(())
        }
    }
}
impl Receipt {
    fn fresh(stop: Option<u64>) -> Self {
        Self {
            work: 0,
            peak: 0,
            calls: 0,
            stop,
        }
    }
}

fn dataset(reverse: bool, datatype: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let property = builder.intern_iri(ogc::geo::AS_WKT).unwrap();
    let default_geometry = builder.intern_iri(ogc::geo::HAS_DEFAULT_GEOMETRY).unwrap();
    let equals = builder.intern_iri(ogc::geo::SF_EQUALS).unwrap();
    let mut rows = [
        ("http://example.org/a", "POINT ZM(1.23456789 2 3 4)"),
        ("http://example.org/b", "POINT ZM(1.234567891 2 3 4)"),
        ("http://example.org/a", "POINT ZM(1.234567890 2 3 4)"),
    ];
    if reverse {
        rows.reverse();
    }
    for (subject, text) in rows {
        let subject = builder.intern_iri(subject).unwrap();
        let object = builder
            .intern_literal(RdfLiteral::typed(text, datatype))
            .unwrap();
        builder.push_quad(subject, property, object, None).unwrap();
    }
    let feature = builder.intern_iri("http://example.org/feature").unwrap();
    let geometry = builder.intern_iri("http://example.org/a").unwrap();
    builder
        .push_quad(feature, default_geometry, geometry, None)
        .unwrap();
    let nested = builder
        .intern_value(&TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/s")),
            p: TermBox::new(TermValue::iri("http://example.org/p")),
            o: TermBox::new(TermValue::typed_literal(
                "quoted",
                purrdf_xsd::datatype::XSD_STRING,
            )),
        })
        .unwrap();
    builder.push_quad(feature, equals, nested, None).unwrap();
    builder.freeze().unwrap()
}

fn config() -> GeoIndexConfig {
    GeoIndexConfig::new(vec![TermValue::iri(ogc::geo::AS_WKT)], GraphSelector::Any).unwrap()
}

#[test]
fn serialization_object_refusal_retains_original_terms_in_both_production_paths() {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let property = TermValue::iri(ogc::geo::AS_WKT);
    let object = TermValue::Triple {
        s: TermBox::new(TermValue::Blank {
            label: "original".to_owned(),
            scope: purrdf_core::BlankScope(73),
        }),
        p: TermBox::new(TermValue::iri("http://example.org/p")),
        o: TermBox::new(TermValue::typed_literal(
            "original lexical",
            purrdf_xsd::datatype::XSD_STRING,
        )),
    };
    let plain = super::ProjectionWork::plain().invalid_serialization(&property, &object);
    let mut context = MetricContext::wgs84().unwrap();
    let mut receipt = Receipt::fresh(None);
    let mut work = super::ProjectionWork::admitted(&mut context, Some(&mut receipt));
    let admitted = work.invalid_serialization(&property, &object);
    work.finish(false).unwrap();
    drop(work);
    drop(property);
    drop(object);
    assert_eq!(plain.original(), admitted.original());
    for error in [plain, admitted] {
        let last = error.clone();
        drop(error);
        let fields = Capture::read(last.original()).child("cause");
        assert_eq!(fields.code, "geo-index-serialization-object");
        assert!(
            matches!(fields.child("property").field("iri"), ErrorValue::Iri(value) if value == ogc::geo::AS_WKT)
        );
        assert_eq!(fields.child("object").code, "rdf-term-triple");
        let GeoError::IndexSerialization(cause) = last.original() else {
            panic!("original index data cause")
        };
        let TermValue::Triple { s, p: _, o: _ } = &cause.object else {
            panic!("original nested object")
        };
        assert!(
            matches!(&**s, TermValue::Blank { scope: purrdf_core::BlankScope(73), label } if label == "original")
        );
        assert!(last.is_expression_error());
        assert!(
            last.detail()
                .starts_with("the object of the configured serialization property")
        );
    }
    assert!(receipt.calls > 0);
}

#[test]
fn serialization_capture_refunds_the_first_term_when_the_second_is_cancelled() {
    use purrdf_lex::allocation::{Memory, Resident};

    let property = TermValue::iri("http://example.org/original-property");
    let object = TermValue::iri("http://example.org/original-object");
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, 17);
    let mut visited = 0;
    let error = crate::IndexSerializationError::capture(&property, &object, &mut memory, |_| {
        visited += 1;
        if visited == 2 {
            Err(GeoError::Cancelled)
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(error, GeoError::Cancelled);
    assert_eq!(visited, 2);
    assert_eq!(memory.admitted_bytes(), 17);

    let mut context = MetricContext::wgs84().unwrap();
    let mut receipt = Receipt::fresh(Some(1));
    let mut work = super::ProjectionWork::admitted(&mut context, Some(&mut receipt));
    assert_eq!(
        work.invalid_serialization(&property, &object),
        GeoError::Cancelled
    );
    work.finish(false).unwrap();
}

#[test]
fn binding_refusals_retain_both_original_configurations_and_source_fingerprints() {
    use crate::IndexBindingError;
    use crate::index::verify_binding;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let source = dataset(false, ogc::geo::WKT_LITERAL);
    let config = config();
    let index = GeoIndex::from_dataset(&*source, crate::standard_vocabulary(), &config).unwrap();
    let actual =
        GeoIndexConfig::new(config.serializations().to_vec(), GraphSelector::Default).unwrap();
    let error =
        verify_binding(&index, &*source, crate::standard_vocabulary(), &actual).unwrap_err();
    drop(actual);
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "geo-index-binding-configuration");
    assert_eq!(
        fields.child("expected").child("graph").code,
        "graph-selector-any"
    );
    assert_eq!(
        fields.child("actual").child("graph").code,
        "graph-selector-default"
    );
    assert!(!error.is_expression_error());

    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let rebuilt = GeoIndex::from_dataset(&*empty, crate::standard_vocabulary(), &config).unwrap();
    let expected = index.source_fingerprint();
    let actual = rebuilt.source_fingerprint();
    let source_error =
        verify_binding(&index, &*empty, crate::standard_vocabulary(), &config).unwrap_err();
    drop(rebuilt);
    drop(index);
    drop(source);
    drop(config);
    let last = source_error.clone();
    drop(source_error);
    let GeoError::IndexBinding(cause) = last.original() else {
        panic!("original binding cause")
    };
    assert_eq!(&**cause, &IndexBindingError::Source { expected, actual });
    let fields = Capture::read(&last).child("cause");
    assert!(
        matches!(fields.field("expected"), ErrorValue::Unsigned(value) if value == u128::from(expected))
    );
    assert!(
        matches!(fields.field("actual"), ErrorValue::Unsigned(value) if value == u128::from(actual))
    );
    assert!(!last.is_expression_error());
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.child("expected").code, "geo-index-config");

    let wide = GeoError::resident_failure(
        IndexBindingError::Source {
            expected: u64::MAX,
            actual: 0,
        },
        GeoError::IndexBinding,
    );
    assert!(
        matches!(Capture::read(&wide).child("cause").field("expected"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
}

#[test]
fn index_configuration_retains_original_terms_and_refusal_priority() {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let input = vec![
        TermValue::iri(ogc::geo::AS_WKT),
        TermValue::Blank {
            label: String::from("original-property"),
            scope: purrdf_core::BlankScope(42),
        },
    ];
    let error = GeoIndexConfig::new(input, GraphSelector::Any).unwrap_err();
    let cloned = error.clone();
    drop(error);
    let fields = Capture::read(&cloned).child("cause");
    assert_eq!(fields.code, "geo-index-non-iri-property");
    assert!(matches!(fields.field("position"), ErrorValue::Unsigned(1)));
    let term = fields.child("property");
    assert_eq!(term.code, "rdf-term-blank");
    assert!(matches!(
        term.field("label"),
        ErrorValue::Text("original-property")
    ));
    assert!(matches!(term.field("scope"), ErrorValue::Unsigned(42)));
    assert!(!cloned.is_expression_error());

    let error = GeoIndexConfig::new(
        Vec::new(),
        GraphSelector::Named(TermValue::Blank {
            label: String::from("also-invalid"),
            scope: purrdf_core::BlankScope::DEFAULT,
        }),
    )
    .unwrap_err();
    assert_eq!(
        Capture::read(&error).child("cause").code,
        "geo-index-no-serializations"
    );

    let error = GeoIndexConfig::new(
        vec![
            TermValue::iri("https://example.org/z"),
            TermValue::iri("https://example.org/a"),
            TermValue::iri("https://example.org/z"),
            TermValue::iri("https://example.org/a"),
        ],
        GraphSelector::Any,
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "geo-index-repeated-property");
    assert!(matches!(
        fields.child("property").field("iri"),
        ErrorValue::Iri("https://example.org/a")
    ));
    assert!(
        GeoIndexConfig::new(vec![TermValue::iri(ogc::geo::AS_WKT)], GraphSelector::Any).is_ok()
    );
}

#[test]
fn projection_retains_the_original_native_source_failure_after_view_destruction() {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let dataset = purrdf_core::TripleDatasetBuilder::default()
        .freeze()
        .unwrap();
    let read = dataset.read().unwrap();
    let config = GeoIndexConfig::new(
        vec![TermValue::iri(ogc::geo::AS_WKT)],
        GraphSelector::Named(TermValue::iri("https://example.org/original-graph")),
    )
    .unwrap();
    let error = GeoIndex::from_dataset(&read, crate::standard_vocabulary(), &config).unwrap_err();
    drop(read);
    drop((dataset, config));
    let cloned = error.clone();
    drop(error);
    let fields = Capture::read(&cloned)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(fields.code, "triple-read-named-graphs-unsupported");
    assert!(!cloned.is_expression_error());
    assert_eq!(
        cloned.to_string(),
        "geometry source read error: the triple-only carrier does not support named graphs"
    );

    let error =
        GeoError::native_source(purrdf_testkit::error_capture::CallerRefusal { limit: u128::MAX });
    let cloned = error.clone();
    drop(error);
    let fields = Capture::read(&cloned)
        .child("cause")
        .child("cause")
        .child("cause");
    assert_eq!(fields.code, "caller-refused");
    assert!(matches!(
        fields.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    let mut next: &dyn purrdf_hash::error::StructuredError = &cloned;
    let mut events = Vec::with_capacity(8);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    loop {
        let fields = Capture::read_with_buffer(next, events);
        let nested = fields.events.iter().find_map(|event| match event {
            purrdf_testkit::error_capture::Event::Value(
                Some("cause"),
                ErrorValue::Cause(cause),
            ) => Some(*cause),
            _ => None,
        });
        events = fields.events;
        match nested {
            Some(cause) => next = cause,
            None => break,
        }
    }
    let allocations = window.close();
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
}

#[test]
fn admitted_projection_preserves_exact_planar_content_and_complete_ledger() {
    let config = config();
    let mut fingerprint = None;
    for reverse in [false, true] {
        let dataset = dataset(reverse, ogc::geo::WKT_LITERAL);
        let plain =
            GeoIndex::from_dataset(&*dataset, crate::standard_vocabulary(), &config).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let mut receipt = Receipt::fresh(None);
        let admitted = GeoIndex::from_dataset_metered(
            &*dataset,
            crate::standard_vocabulary(),
            &config,
            &mut context,
            &mut receipt,
        )
        .unwrap();
        assert_eq!(admitted.source_fingerprint(), plain.source_fingerprint());
        assert_eq!(admitted.entries().len(), plain.entries().len());
        for (a, b) in admitted.entries().iter().zip(plain.entries()) {
            assert_eq!(a.subject(), b.subject());
            assert_eq!(a.geometries(), b.geometries());
        }
        for relation in SpatialRelation::ALL {
            assert_eq!(admitted.asserted(relation), plain.asserted(relation));
        }
        if let Some(expected) = fingerprint {
            assert_eq!(admitted.source_fingerprint(), expected);
        }
        fingerprint = Some(admitted.source_fingerprint());
        assert_eq!(receipt.work, context.work_items());
        assert_eq!(receipt.peak, context.workspace_peak());
        assert_eq!(context.preparation_work_items(), context.work_items());
        assert!(receipt.calls > 50);
    }
}

#[test]
fn every_projection_callback_refusal_returns_no_index_and_releases_transient_storage() {
    let dataset = dataset(false, ogc::geo::WKT_LITERAL);
    let config = config();
    let mut context = MetricContext::wgs84().unwrap();
    let mut measured = Receipt::fresh(None);
    GeoIndex::from_dataset_metered(
        &*dataset,
        crate::standard_vocabulary(),
        &config,
        &mut context,
        &mut measured,
    )
    .unwrap();
    for stop in 1..=measured.calls {
        let mut context = MetricContext::wgs84().unwrap();
        let baseline = context.remaining_workspace();
        let mut receipt = Receipt::fresh(Some(stop));
        assert!(
            matches!(
                GeoIndex::from_dataset_metered(
                    &*dataset,
                    crate::standard_vocabulary(),
                    &config,
                    &mut context,
                    &mut receipt
                ),
                Err(GeoError::Cancelled)
            ),
            "callback {stop}"
        );
        assert_eq!(receipt.calls, stop);
        assert_eq!(context.remaining_workspace(), baseline, "callback {stop}");
        assert_eq!(context.preparation_work_items(), 0, "callback {stop}");
    }
}

#[test]
fn projection_operational_and_carrier_refusals_remain_hard_failures() {
    let config = config();
    let source = dataset(false, ogc::geo::WKT_LITERAL);
    for (work, bytes) in [
        (1, ExecutionLimits::GEOMETRY.max_workspace_bytes),
        (ExecutionLimits::GEOMETRY.max_work_items, 1),
    ] {
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: work,
            max_workspace_bytes: bytes,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
        let error = GeoIndex::from_dataset_in_context(
            &*source,
            crate::standard_vocabulary(),
            &config,
            &mut context,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. }
        ));
        assert_eq!(context.remaining_workspace(), bytes);
    }
    let unsupported = dataset(false, ogc::geo::GML_LITERAL);
    let mut context = MetricContext::wgs84().unwrap();
    assert!(matches!(
        GeoIndex::from_dataset_in_context(
            &*unsupported,
            crate::standard_vocabulary(),
            &config,
            &mut context
        )
        .as_ref()
        .map_err(GeoError::original),
        Err(GeoError::Unsupported(_))
    ));
    assert_eq!(
        context.remaining_workspace(),
        ExecutionLimits::GEOMETRY.max_workspace_bytes
    );
}
