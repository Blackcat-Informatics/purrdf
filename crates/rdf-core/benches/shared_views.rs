// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Criterion generates public entry points for this executable only.
#![allow(missing_docs)]

//! Controlled view/publication comparisons using identical deterministic RDF.
//! Native and shared paths are checked before timing. Report-only: no winner is
//! assumed. `profile.bench` inherits release O3, full LTO and one codegen unit.

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{
    BlankScope, CompositeDatasetView, DatasetMut, DatasetView, GraphMatch, MutableDataset,
    QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue, ViewLimits,
};

fn fixture(rows: u32, payload_bytes: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let literal = builder.intern_literal(RdfLiteral::simple("x".repeat(payload_bytes)));
    let inner = builder.intern_triple(predicate, predicate, literal);
    let outer = builder.intern_triple(predicate, predicate, inner);
    let empty = builder.intern_iri("http://example.org/empty");
    builder.declare_named_graph(empty);
    for index in 0..rows {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        builder.push_quad(subject, predicate, outer, None);
    }
    builder.freeze().unwrap()
}

fn changed(base: Arc<RdfDataset>) -> MutableDataset {
    let mut mutable = MutableDataset::new(base);
    mutable
        .insert(QuadValues::triple(
            TermValue::iri("http://example.org/addition"),
            TermValue::iri("http://example.org/p"),
            TermValue::iri("http://example.org/value"),
        ))
        .unwrap();
    mutable
}

fn scoped_blanks(rows: u32, scope: BlankScope) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..rows {
        let s = builder.intern_blank(&format!("blank{index}"), scope);
        let o = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().unwrap()
}

fn benches(c: &mut Criterion) {
    let base = fixture(10_000, 65_536);
    let contribution = fixture(20, 65_536);
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![base.clone(), contribution.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    let construction = shared.stats();
    let frozen = shared.materialize().unwrap();
    assert_eq!(shared.quads().count(), frozen.quad_count());
    assert_eq!(frozen.quad_count(), base.quad_count());
    assert_eq!(shared.named_graphs().count(), frozen.named_graphs().count());
    let subject = TermValue::iri("http://example.org/s42");
    let shared_subject = shared.term_id_by_value(&subject).unwrap();
    let frozen_subject = frozen.term_id_by_value(&subject).unwrap();
    assert_eq!(
        shared
            .quads_for_pattern(Some(shared_subject), None, None, GraphMatch::Any)
            .count(),
        1
    );
    assert_eq!(
        frozen
            .quads_for_pattern(Some(frozen_subject), None, None, GraphMatch::Any)
            .count(),
        1
    );
    let mutable = changed(base.clone());
    let snapshot = mutable.snapshot_view().unwrap();
    assert_eq!(snapshot.quads().count(), base.quad_count() + 1);
    assert_eq!(snapshot.stats().work.copied_rows, 1);
    eprintln!(
        "shared_views input base_terms={} base_rows={} base_payload={} delta_work={:?} composite={:?}",
        base.term_count(),
        base.rdf_row_count(),
        base.rdf_payload_bytes(),
        snapshot.stats().work,
        construction
    );

    assert!(purrdf_core::datasets_isomorphic(
        &scoped_blanks(16, BlankScope::DEFAULT),
        &scoped_blanks(16, BlankScope(17)),
    ));
    let mut group = c.benchmark_group("shared_views");
    group.bench_function("indexed_native_probe", |b| {
        b.iter(|| {
            black_box(
                frozen
                    .quads_for_pattern(Some(frozen_subject), None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("indexed_composite_probe", |b| {
        b.iter(|| {
            black_box(
                shared
                    .quads_for_pattern(Some(shared_subject), None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("small_delta_snapshot", |b| {
        b.iter(|| black_box(mutable.snapshot_view().unwrap()));
    });
    group.bench_function("small_delta_full_freeze", |b| {
        b.iter(|| black_box(mutable.freeze().unwrap()));
    });
    group.bench_function("compose_shared_nested_payload", |b| {
        b.iter(|| {
            black_box(
                CompositeDatasetView::with_shared_scopes(
                    vec![base.clone(), contribution.clone()],
                    ViewLimits::default(),
                )
                .unwrap(),
            )
        });
    });
    group.bench_function("materialize_shared_nested_payload", |b| {
        b.iter(|| black_box(shared.materialize().unwrap()));
    });
    group.bench_function("retained_snapshot_clone_drop", |b| {
        b.iter(|| black_box(snapshot.clone()));
    });
    group.bench_function("builder_default_blank_scope", |b| {
        b.iter(|| black_box(scoped_blanks(10_000, BlankScope::DEFAULT)));
    });
    group.bench_function("builder_explicit_blank_scope", |b| {
        b.iter(|| black_box(scoped_blanks(10_000, BlankScope(17))));
    });
    group.finish();
}

criterion_group!(shared_views, benches);
criterion_main!(shared_views);
