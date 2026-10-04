// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The cost of addressing a named graph by constant: `GRAPH <iri> { ... }` asks the
//! dataset whether the IRI names a graph before it scopes the inner pattern.
//!
//! The corpus ([`graph_membership_corpus`]) holds 10 000 named graphs of one quad
//! each, a default-graph row naming each graph as an object, and a default-graph
//! object that names no graph. Three queries, prepared once:
//! - `hit` — `GRAPH <g5000> { ?s ?p ?o }`, one row;
//! - `phantom` — `GRAPH <phantom> { BIND(1 AS ?x) }`, no row;
//! - `lateral` — `?s ?p ?o LATERAL { GRAPH ?o { ?a ?b ?c } }`, one membership probe
//!   per default-graph row, so a probe that enumerates the graphs is quadratic here.
//!
//! Each runs over a frozen dataset, a mutable dataset's delta snapshot (one added
//! graph), a two-source composite view (the second source's graphs named apart) and
//! the frozen dataset's pack. The SHACL data view's lanes are in `purrdf-shapes`'
//! `graph_membership` bench.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench
//! graph_constant_membership` (the `make bench` lane) — excluded from `make check`.
//! No timing is asserted.

use std::convert::Infallible;
use std::sync::Arc;

use purrdf_core::term_fixture::{graph_membership_corpus, pack_bytes};
use purrdf_core::{
    CompositeDatasetView, DatasetMut, DatasetView, MutableDataset, PackView, QuadValues,
    SparqlResult, TermValue, ViewLimits,
};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

const EX: &str = "http://example.org/";

/// Named graphs in the corpus.
const GRAPHS: usize = 10_000;

fn rows<D>(engine: &NativeSparqlEngine, view: &D, plan: &PreparedQuery) -> usize
where
    D: DatasetView<ReadError = Infallible> + Sync,
{
    match engine
        .query_prepared_view(view, plan, &[], QueryOptions::EMPTY)
        .expect("the membership query evaluates")
    {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        _ => unreachable!("a SELECT returns solutions"),
    }
}

/// `(shape, plan, rows over one source)`; `sources` scales the `lateral` count.
type Queries = [(&'static str, Arc<PreparedQuery>, usize); 3];

fn lane<D>(
    group: &mut purrdf_testkit::bench::BenchmarkGroup<'_>,
    engine: &NativeSparqlEngine,
    backend: &str,
    view: &D,
    queries: &Queries,
    sources: usize,
) where
    D: DatasetView<ReadError = Infallible> + Sync,
{
    for (shape, plan, expected) in queries {
        let expected = if *shape == "lateral" {
            expected * sources
        } else {
            *expected
        };
        assert_eq!(rows(engine, view, plan), expected, "{backend}/{shape}");
        group.bench_function(format!("{backend}/{shape}"), |bencher| {
            bencher.iter(|| black_box(rows(engine, view, black_box(plan))));
        });
    }
}

fn bench_graph_constant_membership(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let prepare = |q: String| engine.prepare_query(&q, None).expect("prepares");
    let queries: Queries = [
        (
            "hit",
            prepare(format!(
                "SELECT * WHERE {{ GRAPH <{EX}g5000> {{ ?s ?p ?o }} }}"
            )),
            1,
        ),
        (
            "phantom",
            prepare(format!(
                "SELECT * WHERE {{ GRAPH <{EX}phantom> {{ BIND(1 AS ?x) }} }}"
            )),
            0,
        ),
        (
            "lateral",
            prepare("SELECT ?a WHERE { ?s ?p ?o LATERAL { GRAPH ?o { ?a ?b ?c } } }".to_owned()),
            GRAPHS,
        ),
    ];

    let frozen = graph_membership_corpus("", GRAPHS);
    let mut mutable = MutableDataset::new(Arc::clone(&frozen));
    mutable
        .insert(QuadValues {
            s: TermValue::Iri(format!("{EX}s")),
            p: TermValue::Iri(format!("{EX}p")),
            o: TermValue::Iri(format!("{EX}added")),
            g: Some(TermValue::Iri(format!("{EX}added-graph"))),
        })
        .expect("insert applies");
    let delta = mutable.snapshot_view().expect("snapshot builds");
    let composite = CompositeDatasetView::new(
        vec![Arc::clone(&frozen), graph_membership_corpus("b", GRAPHS)],
        ViewLimits::default(),
    )
    .expect("two sources compose");
    let bytes = pack_bytes(&frozen);
    let pack = PackView::from_bytes(&bytes).expect("the pack opens");

    let mut group = c.benchmark_group("graph_constant_membership");
    lane(&mut group, &engine, "frozen", &*frozen, &queries, 1);
    lane(&mut group, &engine, "delta", &delta, &queries, 1);
    lane(&mut group, &engine, "composite", &composite, &queries, 2);
    lane(&mut group, &engine, "pack", &pack, &queries, 1);
    group.finish();
}

bench_group!(benches, bench_graph_constant_membership);
bench_main!(benches);
