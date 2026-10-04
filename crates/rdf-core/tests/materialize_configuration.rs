// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Materializing a view keeps the caller-supplied configuration its sources were
//! frozen under: a delta snapshot rebuilds under its base's content-id scheme and
//! derivation predicate, and a composite under the configuration its sources agree
//! on — the rule `RdfDataset::union` applies. Disagreeing sources carry none.

use std::sync::Arc;

use purrdf_core::{
    CompositeDatasetView, CompositeSource, ContentIdScheme, DatasetMut, MutableDataset, QuadValues,
    RdfDataset, RdfDatasetBuilder, TermValue, ViewLimits,
};

const DERIVED_FROM: &str = "http://example.org/derivedFrom";
const CONTENT_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTENT_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CONTENT_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

/// A base frozen under `prefix` content addressing with `A derivedFrom B`.
fn configured(prefix: &str) -> Arc<RdfDataset> {
    let scheme = ContentIdScheme::new(prefix).expect("valid scheme");
    let mut b = RdfDatasetBuilder::with_content_addressing(scheme, Some(DERIVED_FROM.into()));
    let a = b.intern_iri(CONTENT_A);
    let bb = b.intern_iri(CONTENT_B);
    let p = b.intern_iri("http://example.org/p");
    let derived_from = b.intern_iri(DERIVED_FROM);
    b.push_quad(a, p, bb, None);
    b.push_annotation(a, derived_from, bb);
    b.freeze().expect("configured base freezes")
}

/// `dataset` carries the `blake3:` scheme, recognizes `expected_ids` content ids,
/// and still resolves `A derivedFrom B`.
fn assert_configured(dataset: &RdfDataset, label: &str, expected_ids: usize) {
    assert_eq!(
        dataset.content_id_scheme().map(ContentIdScheme::prefix),
        Some("blake3:"),
        "{label}: scheme"
    );
    assert_eq!(
        dataset.content_ids().count(),
        expected_ids,
        "{label}: content ids"
    );
    let a = dataset.term_id_by_iri(CONTENT_A).expect("A");
    let b = dataset.term_id_by_iri(CONTENT_B).expect("B");
    assert_eq!(dataset.predecessors(a), &[b], "{label}: predecessors");
}

/// The base plus one added content-id IRI, as a snapshot.
fn snapshot(base: Arc<RdfDataset>) -> purrdf_core::DeltaDatasetView {
    let mut mutable = MutableDataset::new(base);
    mutable
        .insert(QuadValues {
            s: TermValue::Iri(CONTENT_C.into()),
            p: TermValue::Iri("http://example.org/p".into()),
            o: TermValue::Iri(CONTENT_A.into()),
            g: None,
        })
        .expect("insert applies");
    mutable.snapshot_view().expect("snapshot builds")
}

#[test]
fn a_delta_snapshot_materializes_under_its_base_configuration() {
    let view = snapshot(configured("blake3:"));
    assert_configured(&view.materialize().expect("materializes"), "delta", 3);
}

#[test]
fn a_composite_materializes_under_its_sources_agreed_configuration() {
    let base = configured("blake3:");
    let one = CompositeDatasetView::new(vec![Arc::clone(&base)], ViewLimits::default())
        .expect("composes");
    assert_configured(&one.materialize().expect("materializes"), "composite", 2);

    let retained = Arc::new(one);
    let mixed = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::from_delta(Arc::new(snapshot(Arc::clone(&base)))),
            CompositeSource::from_selection(
                retained,
                std::iter::empty::<TermValue>(),
                ViewLimits::default(),
            )
            .expect("an empty selection is held"),
            CompositeSource::new(RdfDatasetBuilder::new().freeze().expect("empty")),
        ],
        ViewLimits::default(),
    )
    .expect("a delta, a selection and an unconfigured source compose");
    assert_configured(
        &mixed.materialize().expect("materializes"),
        "composite/delta+selection",
        3,
    );

    // The neighbouring case: sources configured with different schemes agree on
    // none, so the materialization carries none rather than picking one.
    let disagreeing =
        CompositeDatasetView::new(vec![base, configured("urn:blake3:")], ViewLimits::default())
            .expect("composes");
    let merged = disagreeing.materialize().expect("materializes");
    assert!(merged.content_id_scheme().is_none());
    assert_eq!(merged.content_ids().count(), 0);
}
