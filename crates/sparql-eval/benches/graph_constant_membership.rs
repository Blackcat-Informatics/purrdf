// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The cost of addressing one named graph by constant: `GRAPH <iri> { ... }` asks
//! the dataset whether the IRI names a graph before it scopes the inner pattern.
//!
//! The dataset holds 10 000 named graphs of one quad each, plus a default-graph
//! quad whose object IRI is a term that names no graph. Two queries, prepared once:
//! - `hit` — `GRAPH <g5000> { ?s ?p ?o }`, one row;
//! - `phantom` — `GRAPH <phantom> { BIND(1 AS ?x) }`, no row.
//!
//! Each runs over a frozen dataset, a mutable dataset's delta snapshot (one added
//! graph) and a two-source composite view (the second source's graphs named apart),
//! so every membership override is timed against the same membership answer.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench
//! graph_constant_membership` (the `make bench` lane) — excluded from `make check`.
//! No timing is asserted.

use std::convert::Infallible;
use std::sync::Arc;

use purrdf_core::{
    CompositeDatasetView, DatasetMut, DatasetView, MutableDataset, QuadValues, RdfDataset,
    RdfDatasetBuilder, SparqlResult, TermValue, ViewLimits,
};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

const EX: &str = "http://example.org/";

/// Named graphs in the dataset.
const GRAPHS: usize = 10_000;

fn dataset(tag: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let p = b.intern_iri(&format!("{EX}p"));
    let phantom = b.intern_iri(&format!("{EX}phantom"));
    b.push_quad(s, p, phantom, None);
    for i in 0..GRAPHS {
        let o = b.intern_iri(&format!("{EX}o{tag}{i}"));
        let g = b.intern_iri(&format!("{EX}g{tag}{i}"));
        b.push_quad(s, p, o, Some(g));
    }
    b.freeze().expect("freeze the membership dataset")
}

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

fn lane<D>(
    group: &mut purrdf_testkit::bench::BenchmarkGroup<'_>,
    engine: &NativeSparqlEngine,
    backend: &str,
    view: &D,
    queries: &[(&str, Arc<PreparedQuery>, usize)],
) where
    D: DatasetView<ReadError = Infallible> + Sync,
{
    for (shape, plan, expected) in queries {
        assert_eq!(rows(engine, view, plan), *expected, "{backend}/{shape}");
        group.bench_function(format!("{backend}/{shape}"), |bencher| {
            bencher.iter(|| black_box(rows(engine, view, black_box(plan))));
        });
    }
}

fn bench_graph_constant_membership(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let prepare = |q: String| engine.prepare_query(&q, None).expect("prepares");
    let queries = [
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
    ];

    let frozen = dataset("");
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
        vec![Arc::clone(&frozen), dataset("b")],
        ViewLimits::default(),
    )
    .expect("two sources compose");

    let mut group = c.benchmark_group("graph_constant_membership");
    lane(&mut group, &engine, "frozen", &*frozen, &queries);
    lane(&mut group, &engine, "delta", &delta, &queries);
    lane(&mut group, &engine, "composite", &composite, &queries);
    group.finish();
}

bench_group!(benches, bench_graph_constant_membership);
bench_main!(benches);
