// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{evaluate_query, EvalCtx};

/// A skewed star: `:hub --pred--> N leaves` for each `(name, N)` pair.
fn skewed_star(spec: &[(&str, usize)]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let hub = b.intern_iri("http://ex/hub");
    for &(name, count) in spec {
        let pred = b.intern_iri(&format!("http://ex/{name}"));
        for i in 0..count {
            let leaf = b.intern_iri(&format!("http://ex/{name}{i}"));
            b.push_quad(hub, pred, leaf, None);
        }
    }
    b.freeze().expect("freeze")
}

const STAR_QUERY: &str = "SELECT ?a ?b ?c ?d WHERE { \
     ?s <http://ex/hot> ?a . ?s <http://ex/warm> ?b . \
     ?s <http://ex/mid> ?c . ?s <http://ex/rare> ?d }";

/// Parse once, then re-plan + evaluate per iteration over a fresh `EvalCtx` (no order
/// cache), so each sample measures planning + execution, not parsing.
fn eval(ds: &RdfDataset, parsed: &purrdf_sparql_algebra::Query) {
    let mut ctx = EvalCtx::new(ds);
    let outcome = evaluate_query(parsed, &mut ctx).expect("eval");
    criterion::black_box(outcome);
}

fn bench_skewed_star(c: &mut Criterion) {
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

criterion_group!(benches, bench_skewed_star);
criterion_main!(benches);
