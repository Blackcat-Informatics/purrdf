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
    let property = builder.intern_iri(ogc::geo::AS_WKT);
    let default_geometry = builder.intern_iri(ogc::geo::HAS_DEFAULT_GEOMETRY);
    let equals = builder.intern_iri(ogc::geo::SF_EQUALS);
    let mut rows = [
        ("http://example.org/a", "POINT ZM(1.23456789 2 3 4)"),
        ("http://example.org/b", "POINT ZM(1.234567891 2 3 4)"),
        ("http://example.org/a", "POINT ZM(1.234567890 2 3 4)"),
    ];
    if reverse {
        rows.reverse();
    }
    for (subject, text) in rows {
        let subject = builder.intern_iri(subject);
        let object = builder.intern_literal(RdfLiteral::typed(text, datatype));
        builder.push_quad(subject, property, object, None);
    }
    let feature = builder.intern_iri("http://example.org/feature");
    let geometry = builder.intern_iri("http://example.org/a");
    builder.push_quad(feature, default_geometry, geometry, None);
    let nested = builder.intern_value(&TermValue::Triple {
        s: TermBox::new(TermValue::iri("http://example.org/s")),
        p: TermBox::new(TermValue::iri("http://example.org/p")),
        o: TermBox::new(TermValue::typed_literal(
            "quoted",
            purrdf_xsd::datatype::XSD_STRING,
        )),
    });
    builder.push_quad(feature, equals, nested, None);
    builder.freeze().unwrap()
}

fn config() -> GeoIndexConfig {
    GeoIndexConfig::new(vec![TermValue::iri(ogc::geo::AS_WKT)], GraphSelector::Any).unwrap()
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
        ),
        Err(GeoError::Unsupported(_))
    ));
    assert_eq!(
        context.remaining_workspace(),
        ExecutionLimits::GEOMETRY.max_workspace_bytes
    );
}
