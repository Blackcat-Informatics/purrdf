// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Owned vs. borrowed flat-assertion canonicalization, over five fixture shapes.
//!
//! `cargo bench -p purrdf-rdf --bench flat_view_canon`. Report-only; no winner is
//! asserted anywhere in this file — the machine is not quiet.
//!
//! Two routes reach the SAME RDFC-1.0 flat-assertion output:
//!
//! * `owned` — the pre-delegation shape a caller without a borrowed entry point
//!   had to assemble by hand: [`flat_rdf_quads_from_dataset`] walks a dataset ONCE
//!   into an owned `Vec<RdfQuad>`, [`flat_dataset_from_quads`] re-freezes that
//!   vector into a SECOND `RdfDataset`, and [`canonicalize_with`] runs over the
//!   frozen result.
//! * `borrowed` — [`try_canonicalize_flat_view`] over the SAME view directly. Its
//!   own RDFC-1.0 n-degree search runs as usual, and the
//!   [`checkpointed_drain`](purrdf_core::checkpointed_drain) completeness law
//!   drains a [`FallibleDatasetView`] more than once internally, but no second
//!   dataset is ever frozen.
//!
//! Neither route is "the fast one" in general: the owned route pays one dataset
//! freeze the borrowed route never pays; the borrowed route pays extra view drains
//! the owned route never pays. This bench exists so that trade is OBSERVABLE over
//! several shapes, not to declare a winner.
//!
//! Five shapes, all built from the same fixture generator so a difference between
//! them is attributable to the axis each one isolates:
//!
//! * `plain` — a frozen `RdfDataset`, no RDF 1.2 statement-layer rows.
//! * `statements` — the identical row groups WITH reifier + annotation rows added,
//!   so the flat route's re-materialization of the statement layer is actually
//!   exercised on both sides.
//! * `composite` — a single-source `CompositeDatasetView` wrapping `statements`.
//!   `owned` has no concrete `&RdfDataset` to flatten here, so it first pays
//!   [`dataset_from_view`]'s own freeze — THREE freezes total on this shape, not
//!   two — while `borrowed` still pays none.
//! * `delta` — a `DeltaDatasetView` reached through a real mutation round trip
//!   over `statements` (insert a scratch row, then remove it), so the delta
//!   machinery is genuinely in the read path. Same extra `owned` freeze as
//!   `composite`.
//! * `paged` — the same row groups split across four pages behind an
//!   `InMemoryPageProvider`, read through a fresh `PagedQueryView` per call (a
//!   paged query view is an operation-scoped read, so reusing one across calls
//!   would let a later call ride an earlier call's page cache). Same extra `owned`
//!   freeze as `composite` and `delta`, plus the multi-page drain itself.
//!
//! Every shape's `owned` and `borrowed` bytes are asserted equal exactly once,
//! before anything is timed — a correctness check on the fixtures, not a
//! benchmark result.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_core::InMemoryPageProvider;
use purrdf_rdf::{
    BlankScope, CanonHash, Canonicalized, CompositeDatasetView, DatasetMut, DatasetView,
    DeltaDatasetView, FallibleDatasetView, MutableDataset, PagedDataset, PagedQueryLimits,
    QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermValue, ViewLimits,
    canonicalize_with, dataset_from_view, flat_dataset_from_quads, flat_rdf_quads_from_dataset,
    try_canonicalize_flat_view,
};

/// Row-groups per dataset — medium, matching this crate's other fixture-scale
/// benches (`native_codecs`'s `ROWS = 2_000` text rows; this fixture's rows carry
/// structural canonicalization cost rather than parse cost, so a much smaller
/// group count already exercises the RDFC-1.0 search this bench cares about).
const GROUPS: usize = 200;
/// How many pages the `paged` shape splits `GROUPS` row-groups across.
const PAGE_COUNT: usize = 4;

/// One row-group's worth of RDF: two default-graph quads, two named-graph quads
/// covering all three literal shapes between them, and one blank LABEL living in
/// two `BlankScope`s. Structurally distinguishable by its own `index` (no two
/// groups share a label), so RDFC-1.0 canonicalization stays linear rather than
/// exploring automorphisms — the same device `carrier_base`
/// (`crates/rdf-core/benches/shared_views.rs`) and `keystone_base`
/// (`crates/rdf/src/gts_fixtures.rs`) use elsewhere in this workspace. When
/// `with_statements` is set, every eighth group additionally carries a triple
/// term, a reifier binding, and a statement annotation.
fn labeled_dataset(range: std::ops::Range<usize>, with_statements: bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let q = b.intern_iri("https://example.org/q");
    let g = b.intern_iri("https://example.org/g");
    let plain = b.intern_literal(RdfLiteral::simple("bare"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("cat", "en"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    for index in range {
        let s = b.intern_iri(&format!("https://example.org/s{index}"));
        let o = b.intern_iri(&format!("https://example.org/o{index}"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, q, plain, None);
        b.push_quad(s, p, directional, Some(g));
        b.push_quad(s, q, tagged, Some(g));
        let shared = b.intern_blank(&format!("n{index}"), BlankScope::DEFAULT);
        let scoped = b.intern_blank(&format!("n{index}"), BlankScope(4));
        b.push_quad(shared, p, o, Some(g));
        b.push_quad(scoped, q, o, Some(g));
        if with_statements && index % 8 == 0 {
            let triple = b.intern_triple(shared, p, directional);
            let reifier = b.intern_blank(&format!("st{index}"), BlankScope(7));
            b.push_reifier_in_graph(reifier, triple, Some(g));
            b.push_annotation_in_graph(reifier, q, tagged, Some(g));
        }
    }
    b.freeze().expect("the labeled dataset freezes")
}

/// `plain` shape: `GROUPS` row-groups, no statement layer.
fn plain_dataset() -> Arc<RdfDataset> {
    labeled_dataset(0..GROUPS, false)
}

/// `statements` shape: the identical `GROUPS` row-groups, WITH the statement
/// layer — the fixture `composite`, `delta`, and `paged` all wrap or split.
fn statement_dataset() -> Arc<RdfDataset> {
    labeled_dataset(0..GROUPS, true)
}

/// `GROUPS` row-groups split into `PAGE_COUNT` disjoint, independently-frozen
/// pages. Every group index is globally unique (`sN`/`oN`/`nN` are never reused
/// across pages), so the split changes nothing about content or blank identity —
/// only how many separately-frozen `RdfDataset`s the rows are read through.
fn statement_pages() -> Vec<Arc<RdfDataset>> {
    let chunk = GROUPS / PAGE_COUNT;
    (0..PAGE_COUNT)
        .map(|page| labeled_dataset(page * chunk..(page + 1) * chunk, true))
        .collect()
}

/// The `composite` shape: a single-source `CompositeDatasetView` over
/// `statements` — the minimal composite that still forces the owned route
/// through a real `DatasetView` rather than a concrete `RdfDataset`.
fn composite_view(base: &Arc<RdfDataset>) -> CompositeDatasetView {
    CompositeDatasetView::new(vec![Arc::clone(base)], ViewLimits::default())
        .expect("the single-source composite composes")
}

/// The `delta` shape: a `DeltaDatasetView` whose EFFECTIVE content is exactly
/// `statements`'s, reached through a real mutation round trip (insert a scratch
/// row, then remove it) so the delta machinery is genuinely in the read path —
/// not an untouched passthrough.
fn delta_view(base: &Arc<RdfDataset>) -> DeltaDatasetView {
    let mut mutable = MutableDataset::new(Arc::clone(base));
    let scratch = QuadValues::triple(
        TermValue::iri("https://example.org/scratch"),
        TermValue::iri("https://example.org/p"),
        TermValue::iri("https://example.org/o"),
    );
    assert!(
        mutable
            .insert(scratch.clone())
            .expect("the scratch row inserts")
    );
    assert!(mutable.remove(&scratch), "and comes back out again");
    mutable.snapshot_view().expect("the delta publishes")
}

/// The owned route over an already-frozen `&RdfDataset`: flatten once (no
/// freeze — [`flat_rdf_quads_from_dataset`] only walks and clones terms),
/// re-freeze the flattened stream into a SECOND dataset, canonicalize that.
fn owned_route(dataset: &RdfDataset, hash: CanonHash) -> Canonicalized {
    let quads = flat_rdf_quads_from_dataset(dataset);
    let refrozen = flat_dataset_from_quads(&quads).expect("the flattened quads refreeze");
    canonicalize_with(&refrozen, hash)
}

/// The owned route over a view that is not already a concrete `RdfDataset`: an
/// EXTRA freeze ([`dataset_from_view`]) is required before [`owned_route`] has
/// anything to flatten — the second and third freezes this shape pays, against
/// the borrowed route's zero.
fn owned_route_over_view<D: DatasetView>(view: &D, hash: CanonHash) -> Canonicalized {
    let materialized = dataset_from_view(view).expect("the view materializes");
    owned_route(&materialized, hash)
}

/// The borrowed route: [`try_canonicalize_flat_view`] straight over `view`, no
/// freeze, whatever `D` happens to be.
fn borrowed_route<D: FallibleDatasetView>(view: &D, hash: CanonHash) -> Canonicalized<D::Id> {
    try_canonicalize_flat_view(view, hash).expect("the view admits under the flat presentation")
}

fn benches(c: &mut Criterion) {
    let hash = CanonHash::Sha256;

    let plain = plain_dataset();
    let statements = statement_dataset();
    assert!(
        statements.reifier_quads().count() >= 20,
        "the statements shape must genuinely carry statement-layer rows: {}",
        statements.reifier_quads().count()
    );
    assert_eq!(
        plain.reifier_quads().count(),
        0,
        "the plain shape must carry NO statement-layer rows, or the pair is not \
         isolating the axis its name claims"
    );

    let composite = composite_view(&statements);
    let delta = delta_view(&statements);

    let pages = statement_pages();
    let paged_dataset = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(pages)))
        .expect("the pages seal into one paged dataset");

    // PARITY, not speed: `owned` and `borrowed` must land on the SAME flat bytes
    // for the SAME content, asserted once per shape, before anything is timed. If
    // they disagreed, the pairs below would be timing two different answers.
    let plain_owned = owned_route(&plain, hash).nquads;
    let plain_borrowed = borrowed_route(&*plain, hash).nquads;
    assert_eq!(
        plain_owned, plain_borrowed,
        "plain: owned and borrowed routes must agree"
    );
    assert!(!plain_owned.is_empty(), "plain: the fixture must emit rows");

    let statements_owned = owned_route(&statements, hash).nquads;
    let statements_borrowed = borrowed_route(&*statements, hash).nquads;
    assert_eq!(
        statements_owned, statements_borrowed,
        "statements: owned and borrowed routes must agree"
    );

    let composite_owned = owned_route_over_view(&composite, hash).nquads;
    let composite_borrowed = borrowed_route(&composite, hash).nquads;
    assert_eq!(
        composite_owned, composite_borrowed,
        "composite: owned and borrowed routes must agree"
    );
    assert_eq!(
        composite_owned, statements_owned,
        "composite wraps the statements dataset's content unchanged"
    );

    let delta_owned = owned_route_over_view(&delta, hash).nquads;
    let delta_borrowed = borrowed_route(&delta, hash).nquads;
    assert_eq!(
        delta_owned, delta_borrowed,
        "delta: owned and borrowed routes must agree"
    );
    assert_eq!(
        delta_owned, statements_owned,
        "delta's effective content is exactly the statements dataset's"
    );

    let paged_probe = paged_dataset.query_view(PagedQueryLimits::UNBOUNDED);
    let paged_owned = owned_route_over_view(&paged_probe, hash).nquads;
    let paged_borrowed = borrowed_route(&paged_probe, hash).nquads;
    assert_eq!(
        paged_owned, paged_borrowed,
        "paged: owned and borrowed routes must agree"
    );
    assert_eq!(
        paged_owned, statements_owned,
        "the same content split across pages must canonicalize identically to the \
         single-dataset statements shape"
    );

    let mut group = c.benchmark_group("flat_view_canon");
    // The machine is shared and contended and these numbers are report-only, so
    // the sample count is small on purpose: precision buys nothing here.
    group.sample_size(10);

    group.bench_function(BenchmarkId::new("owned", "plain"), |b| {
        b.iter(|| black_box(owned_route(&plain, hash)));
    });
    group.bench_function(BenchmarkId::new("borrowed", "plain"), |b| {
        b.iter(|| black_box(borrowed_route(&*plain, hash)));
    });

    group.bench_function(BenchmarkId::new("owned", "statements"), |b| {
        b.iter(|| black_box(owned_route(&statements, hash)));
    });
    group.bench_function(BenchmarkId::new("borrowed", "statements"), |b| {
        b.iter(|| black_box(borrowed_route(&*statements, hash)));
    });

    group.bench_function(BenchmarkId::new("owned", "composite"), |b| {
        b.iter(|| black_box(owned_route_over_view(&composite, hash)));
    });
    group.bench_function(BenchmarkId::new("borrowed", "composite"), |b| {
        b.iter(|| black_box(borrowed_route(&composite, hash)));
    });

    group.bench_function(BenchmarkId::new("owned", "delta"), |b| {
        b.iter(|| black_box(owned_route_over_view(&delta, hash)));
    });
    group.bench_function(BenchmarkId::new("borrowed", "delta"), |b| {
        b.iter(|| black_box(borrowed_route(&delta, hash)));
    });

    // A fresh `PagedQueryView` per call on both sides: a paged query view is an
    // operation-scoped read that caches materialized pages internally, so reusing
    // one instance across repeated calls would let a later call ride an earlier
    // call's cache instead of paying the drain this shape exists to observe.
    group.bench_function(BenchmarkId::new("owned", "paged"), |b| {
        b.iter(|| {
            let view = paged_dataset.query_view(PagedQueryLimits::UNBOUNDED);
            black_box(owned_route_over_view(&view, hash))
        });
    });
    group.bench_function(BenchmarkId::new("borrowed", "paged"), |b| {
        b.iter(|| {
            let view = paged_dataset.query_view(PagedQueryLimits::UNBOUNDED);
            black_box(borrowed_route(&view, hash))
        });
    });

    group.finish();
}

criterion_group! {
    name = flat_view_canon_benches;
    // Report-only and on a contended machine, like the other view-canon benches
    // in this workspace: a small sample is the honest budget, not a precision
    // compromise.
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(2));
    targets = benches
}
criterion_main!(flat_view_canon_benches);
