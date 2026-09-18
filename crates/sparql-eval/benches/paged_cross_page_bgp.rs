// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Cross-page BGP evaluation latency: the SAME 3-pattern join + `FILTER` query
//! evaluated (a) over a single frozen [`RdfDataset`] and (b) over a multi-page
//! [`PagedDataset`] whose `knows`/`name`/`age` triples for any given entity are
//! deliberately spread across DIFFERENT pages, so the join variable `?x` must be
//! unified across page boundaries via the external paged backend.
//!
//! This gives the new cross-page BGP path (`NativeSparqlEngine::query_prepared_view`
//! over `PagedDataset`) bench coverage alongside the single-dataset baseline
//! (`NativeSparqlEngine::query_prepared`) so a reader can compare the two — this bench
//! draws NO conclusion and asserts NO speedup or absolute timing threshold: the
//! machine running `cargo bench` is not quiet, and paged evaluation does strictly
//! more work per pattern (per-page translation + demand materialization) than a
//! single frozen dataset by construction, so a naive "paged should be faster/slower"
//! assertion would be meaningless.
//!
//! A third case, `bench_graph_selective_bgp`, wraps the same BGP in
//! `GRAPH <g> { ... }` over a many-page [`PagedDataset`] whose pages each hold exactly
//! one named graph — the graph-pruned admission path this crate's paged summary work
//! added, alongside the whole-dataset case above (whose default-graph query has no
//! graph component to prune on).
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench paged_cross_page_bgp` (the
//! `make bench` lane) — excluded from `make check`.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use purrdf_core::{
    DatasetView, InMemoryPageProvider, PagedDataset, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    TermId, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

/// Entity count. Each entity contributes 3 quads (`knows`/`name`/`age`), so 200
/// entities is 600 triples — a "few hundred" moderate corpus.
const ENTITIES: usize = 200;

/// Number of pages the corpus is round-robin split across, so a join binding for
/// `?x` (an entity's `knows` target) has its `name`/`age` triples land on different
/// pages from its `knows` triple more often than not.
const PAGE_COUNT: usize = 6;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const EX: &str = "http://example.org/";

type Triple = (TermValue, TermValue, TermValue);

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("{EX}{name}"))
}

/// Intern one dataset-independent value into a builder (no triple terms needed here,
/// so this is the non-recursive subset of the sibling paged-backend test helper).
fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    match v {
        TermValue::Iri(s) => b.intern_iri(s),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => b.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Blank { .. } | TermValue::Triple { .. } => {
            unreachable!("bench corpus contains only IRIs and literals")
        }
    }
}

/// Build the corpus: for each entity `i`, `personI knows person(i+1)`, `personI name
/// "NameI"`, `personI age xsd:integer(18 + i % 60)` — a ring of `knows` edges plus a
/// name/age pair per entity, so `?x ex:knows ?y . ?y ex:name ?n . ?y ex:age ?a` joins
/// on `?y` across all three predicates.
fn corpus() -> Vec<Triple> {
    let mut triples = Vec::with_capacity(ENTITIES * 3);
    for i in 0..ENTITIES {
        let s = iri(&format!("person{i}"));
        let next = iri(&format!("person{}", (i + 1) % ENTITIES));
        triples.push((s.clone(), iri("knows"), next));
        triples.push((
            s.clone(),
            iri("name"),
            TermValue::simple_literal(format!("Name{i}")),
        ));
        let age = TermValue::typed_literal((18 + i % 60).to_string(), XSD_INTEGER);
        triples.push((s, iri("age"), age));
    }
    triples
}

/// Freeze one page (or the single reference dataset) from `(s, p, o)` triples in the
/// default graph.
fn build_page(triples: &[Triple]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = intern_value(&mut b, s);
        let p = intern_value(&mut b, p);
        let o = intern_value(&mut b, o);
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("page freeze")
}

/// Round-robin split `triples` across `page_count` quad-disjoint pages, so a single
/// entity's three triples (pushed consecutively by [`corpus`]) land on DIFFERENT
/// pages — the cross-page join condition this bench exists to measure.
fn split_pages(triples: &[Triple], page_count: usize) -> Vec<Arc<RdfDataset>> {
    let mut buckets: Vec<Vec<Triple>> = vec![Vec::new(); page_count];
    for (i, t) in triples.iter().enumerate() {
        buckets[i % page_count].push(t.clone());
    }
    buckets.iter().map(|b| build_page(b)).collect()
}

/// The representative 3-pattern BGP join + numeric `FILTER`: entities known by some
/// other entity, whose name and age (age > 30) are pulled across the join.
const QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?x ?n ?a WHERE {
  ?p ex:knows ?x .
  ?x ex:name ?n .
  ?x ex:age ?a .
  FILTER(?a > 30)
}";

/// Number of named graphs in the graph-selective corpus below, each pinned to its own
/// page — so a `GRAPH <g> { ... }` query naming ONE graph is exactly the case the
/// paged view's page-admission law (its internal graph-page index, consulted before
/// any page is materialized) exists to prune: every page but the one page carrying
/// `g` should never be materialized.
const GRAPH_COUNT: usize = 6;

/// Entities per named graph in the graph-selective corpus, kept smaller than
/// [`ENTITIES`] so the total corpus (`GRAPH_COUNT * GRAPH_ENTITIES` entities) stays
/// comparable in scale to the whole-dataset corpus above.
const GRAPH_ENTITIES: usize = 60;

/// Read a query's row count regardless of `SparqlResult` shape (`SELECT` is the only
/// shape either bench issues; the other arms are unreachable in practice but keep the
/// helper total).
fn row_count(r: &purrdf_core::SparqlResult) -> usize {
    match r {
        purrdf_core::SparqlResult::Solutions { rows, .. } => rows.len(),
        purrdf_core::SparqlResult::Boolean(_) | purrdf_core::SparqlResult::Graph(_) => 0,
    }
}

/// Build one named graph's entity corpus: the same ring-of-`knows` shape as
/// [`corpus`], but with entity names scoped by `graph_index` so distinct graphs never
/// share a subject/object term — mirroring how distinct named graphs (e.g. per-tenant
/// data) rarely share entities in practice, and keeping each page's content wholly
/// specific to its one graph.
fn graph_corpus(graph_index: usize, entity_count: usize) -> Vec<Triple> {
    let mut triples = Vec::with_capacity(entity_count * 3);
    for i in 0..entity_count {
        let s = iri(&format!("g{graph_index}_person{i}"));
        let next = iri(&format!("g{graph_index}_person{}", (i + 1) % entity_count));
        triples.push((s.clone(), iri("knows"), next));
        triples.push((
            s.clone(),
            iri("name"),
            TermValue::simple_literal(format!("Name{graph_index}_{i}")),
        ));
        let age = TermValue::typed_literal((18 + i % 60).to_string(), XSD_INTEGER);
        triples.push((s, iri("age"), age));
    }
    triples
}

/// Freeze one page holding every triple of `triples`, all inside the SAME named graph
/// `graph` — the "one page, one named graph" shape the graph-selective bench pages
/// its `PagedDataset` with.
fn build_graph_page(triples: &[Triple], graph: &TermValue) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let g = intern_value(&mut b, graph);
    for (s, p, o) in triples {
        let s = intern_value(&mut b, s);
        let p = intern_value(&mut b, p);
        let o = intern_value(&mut b, o);
        b.push_quad(s, p, o, Some(g));
    }
    b.freeze().expect("graph page freeze")
}

/// Freeze ONE dataset holding every `(graph, triples)` pair's quads together, all in
/// their respective named graphs — the whole-dataset baseline for the graph-selective
/// bench (every graph co-resident, same as `single` is for the default-graph bench
/// above).
fn build_multi_graph_dataset(graphs: &[(TermValue, Vec<Triple>)]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for (graph, triples) in graphs {
        let g = intern_value(&mut b, graph);
        for (s, p, o) in triples {
            let s = intern_value(&mut b, s);
            let p = intern_value(&mut b, p);
            let o = intern_value(&mut b, o);
            b.push_quad(s, p, o, Some(g));
        }
    }
    b.freeze().expect("multi-graph dataset freeze")
}

/// The `GRAPH <g> { ... }`-wrapped variant of [`QUERY`], scoped to exactly one named
/// graph.
fn graph_query(graph: &str) -> String {
    format!(
        "\
PREFIX ex: <{EX}>
SELECT ?x ?n ?a WHERE {{
  GRAPH <{graph}> {{
    ?p ex:knows ?x .
    ?x ex:name ?n .
    ?x ex:age ?a .
    FILTER(?a > 30)
  }}
}}"
    )
}

fn bench_cross_page_bgp(c: &mut Criterion) {
    let corpus = corpus();

    // (1) The single-dataset baseline: every triple in one frozen `RdfDataset`.
    let single = build_page(&corpus);

    // (2) The paged view over the SAME triples, split across several pages so the
    // join crosses page boundaries.
    let pages = split_pages(&corpus, PAGE_COUNT);
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal pages");
    assert_eq!(
        paged.page_count(),
        PAGE_COUNT,
        "the corpus spans every page"
    );

    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare");

    // Sanity pass: both backends must do real join work (a broken fixture would
    // silently benchmark a no-op). `age > 30` keeps roughly (60 - 13) / 60 of the
    // ring, so this is comfortably non-empty for `ENTITIES = 200`.
    let single_rows = engine
        .query_prepared(&single, &prepared, &[], QueryOptions::EMPTY)
        .expect("single query");
    let paged_rows = engine
        .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
        .expect("paged query");
    assert!(
        row_count(&single_rows) > 0,
        "single-dataset query must return rows"
    );
    assert_eq!(
        row_count(&single_rows),
        row_count(&paged_rows),
        "single and paged backends must agree on row count"
    );

    let mut group = c.benchmark_group("cross_page_bgp");
    group.bench_function("single", |bencher| {
        bencher.iter(|| {
            let result = engine
                .query_prepared(
                    criterion::black_box(&single),
                    criterion::black_box(&prepared),
                    &[],
                    QueryOptions::EMPTY,
                )
                .expect("single query");
            criterion::black_box(result);
        });
    });
    group.bench_function("paged", |bencher| {
        bencher.iter(|| {
            let result = engine
                .query_prepared_view(
                    criterion::black_box(&paged),
                    criterion::black_box(&prepared),
                    &[],
                    QueryOptions::EMPTY,
                )
                .expect("paged query");
            criterion::black_box(result);
        });
    });
    group.finish();
}

/// Whole-dataset scan latency through the read path: the BGP bench above drives
/// `quads_for_pattern`; this drives the streaming `DatasetView::quads` full scan over
/// the multi-page paged backend (which materializes each page lazily and translates its
/// quads on the fly) against the single-dataset inherent scan. Same report-only caveats:
/// a noisy machine and strictly-more per-quad work on the paged side make any
/// "faster/slower" assertion meaningless — this exists for comparison, not a threshold.
fn bench_paged_full_scan(c: &mut Criterion) {
    let corpus = corpus();
    let single = build_page(&corpus);
    let pages = split_pages(&corpus, PAGE_COUNT);
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(pages)))
        .expect("seal pages");

    // Sanity: both scans see every triple (a broken fixture would benchmark a no-op).
    assert_eq!(single.quads().count(), corpus.len());
    assert_eq!(
        DatasetView::quads(&paged).count(),
        corpus.len(),
        "the paged scan streams every quad across all pages"
    );

    let mut group = c.benchmark_group("paged_full_scan");
    group.bench_function("single", |bencher| {
        bencher.iter(|| {
            let n = criterion::black_box(&single).quads().count();
            criterion::black_box(n);
        });
    });
    group.bench_function("paged", |bencher| {
        bencher.iter(|| {
            let n = DatasetView::quads(criterion::black_box(&paged)).count();
            criterion::black_box(n);
        });
    });
    group.finish();
}

/// Graph-selective BGP evaluation latency: the SAME 3-pattern join + `FILTER` query as
/// `bench_cross_page_bgp`, wrapped in `GRAPH <g> { ... }` naming exactly ONE named
/// graph, evaluated (a) over a single frozen `RdfDataset` holding every graph and (b)
/// over a many-page `PagedDataset` whose pages each hold exactly one named graph — so
/// the paged case exercises the page-admission law's graph pruning (candidate pages
/// narrowed to the one page carrying `g` before any page is materialized), which the
/// whole-dataset case above never exercises (its default-graph query has no graph
/// component to prune on).
///
/// Report-only, same as every bench in this file: do NOT add a timing threshold or an
/// `assert!` on a duration here. The machine running `cargo bench` is not quiet, and
/// pruning fewer pages is not the same claim as "faster wall-clock time" — a
/// deterministic page-count assertion (e.g. that only one page was admitted) belongs
/// in the test suite, not here. This bench is evidence for a human reader comparing
/// the two groups, nothing more.
fn bench_graph_selective_bgp(c: &mut Criterion) {
    // Each named graph gets its own entity ring (`graph_corpus` scopes entity names by
    // graph index, so no two graphs share a subject/object term).
    let per_graph_corpus: Vec<Vec<Triple>> = (0..GRAPH_COUNT)
        .map(|i| graph_corpus(i, GRAPH_ENTITIES))
        .collect();
    let graphs: Vec<TermValue> = (0..GRAPH_COUNT)
        .map(|i| iri(&format!("graph{i}")))
        .collect();

    // (1) The whole-dataset baseline: every named graph's triples in ONE frozen
    // `RdfDataset`.
    let multi: Vec<(TermValue, Vec<Triple>)> = graphs
        .iter()
        .cloned()
        .zip(per_graph_corpus.iter().cloned())
        .collect();
    let single = build_multi_graph_dataset(&multi);

    // (2) The paged view: ONE named graph per page (`GRAPH_COUNT` pages total), so a
    // `GRAPH <g>` query for a single graph should admit exactly one page.
    let pages: Vec<Arc<RdfDataset>> = graphs
        .iter()
        .zip(per_graph_corpus.iter())
        .map(|(g, triples)| build_graph_page(triples, g))
        .collect();
    let provider = Arc::new(InMemoryPageProvider::new(pages));
    let paged = PagedDataset::from_provider(provider).expect("seal graph pages");
    assert_eq!(paged.page_count(), GRAPH_COUNT, "one page per named graph");

    let target_graph = match &graphs[0] {
        TermValue::Iri(s) => s.clone(),
        TermValue::Blank { .. } | TermValue::Literal { .. } | TermValue::Triple { .. } => {
            unreachable!("graph iri is always an IRI")
        }
    };
    let query_text = graph_query(&target_graph);

    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(&query_text, None).expect("prepare");

    // Sanity pass: both backends must do real work scoped to the one named graph (a
    // broken fixture would silently benchmark a no-op).
    let single_rows = engine
        .query_prepared(&single, &prepared, &[], QueryOptions::EMPTY)
        .expect("single graph query");
    let paged_rows = engine
        .query_prepared_view(&paged, &prepared, &[], QueryOptions::EMPTY)
        .expect("paged graph query");
    assert!(
        row_count(&single_rows) > 0,
        "single-dataset graph query must return rows"
    );
    assert_eq!(
        row_count(&single_rows),
        row_count(&paged_rows),
        "single and paged backends must agree on row count"
    );

    let mut group = c.benchmark_group("graph_selective_bgp");
    group.bench_function("single", |bencher| {
        bencher.iter(|| {
            let result = engine
                .query_prepared(
                    criterion::black_box(&single),
                    criterion::black_box(&prepared),
                    &[],
                    QueryOptions::EMPTY,
                )
                .expect("single graph query");
            criterion::black_box(result);
        });
    });
    group.bench_function("paged", |bencher| {
        bencher.iter(|| {
            let result = engine
                .query_prepared_view(
                    criterion::black_box(&paged),
                    criterion::black_box(&prepared),
                    &[],
                    QueryOptions::EMPTY,
                )
                .expect("paged graph query");
            criterion::black_box(result);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_cross_page_bgp,
    bench_paged_full_scan,
    bench_graph_selective_bgp
);
criterion_main!(benches);
