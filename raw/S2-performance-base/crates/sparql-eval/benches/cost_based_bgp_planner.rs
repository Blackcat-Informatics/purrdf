// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Cost-based BGP join-planner latency benchmark: end-to-end evaluation of a skewed
//! multi-join star through the cost planner, at two scales.
//!
//! The deterministic correctness win — that the cost order materialises strictly fewer
//! intermediate rows than the retired structural heuristic — is proven in the `bgp`
//! unit tests by counting REAL prefix-result rows (a non-flaky integer comparison). A
//! wall-time A/B against the structural order is not possible here: that heuristic was
//! removed (greenfield, no fallback), so the planner has no "off" switch. This bench is
//! therefore a **regression watch** on the absolute planning + evaluation latency of a
//! skewed join — it tracks that the planner stays cheap (the planning cost is
//! `O(patterns · log n)` cardinality probes) as the dataset grows.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval` (the `make bench` lane) — excluded
//! from `make check`.

#[path = "../tests/support/mod.rs"]
mod support;

use support::{boundary_joins, result_size, skewed_star};

use purrdf_testkit::bench::{Bench, bench_group, bench_main};

use purrdf_core::RdfDataset;
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{
    EvalCtx, NativeSparqlEngine, QueryGovernors, QueryOptions, evaluate_query,
};

const STAR_QUERY: &str = "SELECT ?a ?b ?c ?d WHERE { \
     ?s <http://ex/hot> ?a . ?s <http://ex/warm> ?b . \
     ?s <http://ex/mid> ?c . ?s <http://ex/rare> ?d }";

/// Parse once, then re-plan + evaluate per iteration over a fresh `EvalCtx` (no order
/// cache), so each sample measures planning + execution, not parsing.
fn eval(ds: &RdfDataset, parsed: &purrdf_sparql_algebra::Query) {
    let mut ctx = EvalCtx::new(ds);
    let outcome = evaluate_query(parsed, &mut ctx).expect("eval");
    std::hint::black_box(outcome);
}

fn bench_skewed_star(c: &mut Bench) {
    let parsed = SparqlParser::new().parse_query(STAR_QUERY).expect("parse");

    let mut group = c.benchmark_group("cost_based_bgp_planner");
    for (label, spec) in [
        (
            "small",
            &[("hot", 20), ("warm", 10), ("mid", 5), ("rare", 1)][..],
        ),
        (
            "large",
            &[("hot", 400), ("warm", 200), ("mid", 100), ("rare", 1)][..],
        ),
    ] {
        let ds = skewed_star(spec);
        group.bench_function(label, |bencher| bencher.iter(|| eval(&ds, &parsed)));
    }
    group.finish();
}

/// Compare boundary-sensitive shapes with their equivalent selective forms on
/// the same full-scale dataset, prepared outside timing and under one allowance.
fn bench_boundary_joins(c: &mut Bench) {
    let dataset = boundary_joins::dataset();
    let engine = NativeSparqlEngine::new();
    let governors = QueryGovernors::METERED.with_max_intermediate_cells(boundary_joins::CELLS);
    let mut group = c.benchmark_group("cost_based_bgp_planner/boundary_joins");
    for (label, original, equivalent, expected) in boundary_joins::QUERIES {
        for (form, body) in [("original", original), ("equivalent", equivalent)] {
            let prepared = engine
                .prepare_query(&boundary_joins::query(body), None)
                .expect("boundary query parses");
            let run = || {
                let outcome = engine
                    .query_prepared_governed_view(
                        &*dataset,
                        &prepared,
                        &[],
                        QueryOptions::EMPTY,
                        &governors,
                    )
                    .expect("boundary benchmark evaluates");
                let result = outcome
                    .into_complete()
                    .expect("bounded benchmark completes");
                result_size(&result)
            };
            assert_eq!(run(), expected, "{label}/{form}");
            group.bench_function(format!("{label}/{form}"), |bencher| {
                bencher.iter(|| std::hint::black_box(run()));
            });
        }
    }
    group.finish();
}

bench_group!(benches, bench_skewed_star, bench_boundary_joins);
bench_main!(benches);
