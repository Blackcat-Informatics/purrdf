// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Constant-`GRAPH` addressing through the SHACL data view: the
//! `?s ?p ?o LATERAL { GRAPH ?o { ?a ?b ?c } }` shape of `purrdf-sparql-eval`'s
//! `graph_constant_membership` bench, over 10 000 named graphs read through
//! [`ShaclDatasetView`] on a native source and on a mutation snapshot. Each
//! default-graph row is one membership probe on the view, so a probe that
//! enumerates the graphs makes this lane quadratic.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench graph_membership` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.
#![allow(missing_docs)]

use std::sync::Arc;

use purrdf_core::term_fixture::{graph_membership_corpus, iri};
use purrdf_rdf::ir::ViewLimits;
use purrdf_rdf::{DatasetMut, MutableDataset, QuadValues};
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

/// Named graphs in the corpus.
const GRAPHS: usize = 10_000;

/// The solution count of `plan` over `view`.
fn answer(engine: &NativeSparqlEngine, view: &ShaclDatasetView, plan: &PreparedQuery) -> usize {
    let result = engine.query_prepared_view(view, plan, &[], QueryOptions::EMPTY);
    result
        .expect("the lateral query evaluates")
        .into_solutions()
        .map_or(0, |(_, rows)| rows.len())
}

fn bench_graph_membership(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let plan = engine
        .prepare_query(
            "SELECT ?a WHERE { ?s ?p ?o LATERAL { GRAPH ?o { ?a ?b ?c } } }",
            None,
        )
        .expect("prepares");
    let source = graph_membership_corpus("", GRAPHS);
    let mut mutable = MutableDataset::new(Arc::clone(&source));
    mutable
        .insert(QuadValues {
            s: iri("s"),
            p: iri("p"),
            o: iri("added"),
            g: Some(iri("added-graph")),
        })
        .expect("insert applies");
    let views = [
        ("native", ShaclDatasetView::native(source)),
        (
            "delta",
            ShaclDatasetView::delta(
                Arc::new(mutable.snapshot_view().expect("snapshot")),
                false,
                ViewLimits::default(),
            )
            .expect("delta view"),
        ),
    ];
    let mut group = c.benchmark_group("shacl_view_graph_membership");
    for (name, view) in &views {
        assert_eq!(answer(&engine, view, &plan), GRAPHS, "{name}");
        group.bench_function(format!("{name}/lateral"), |bencher| {
            bencher.iter(|| black_box(answer(&engine, view, black_box(&plan))));
        });
    }
    group.finish();
}

bench_group!(benches, bench_graph_membership);
bench_main!(benches);
