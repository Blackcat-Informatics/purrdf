// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! End-to-end SPARQL numeric evaluation through [`NativeSparqlEngine`], over a
//! 20 000-row dataset of `xsd:integer` and `xsd:decimal` values.
//!
//! Every value of the `in_range` group fits the bounded representations (`i128`,
//! and an `i128` mantissa with at most eighteen fractional digits), so these
//! lanes measure the bounded path and its dispatch:
//!
//! - `a_integer_arithmetic_filter` — `+ − × ÷` over `?i` inside a `FILTER`.
//! - `b_decimal_arithmetic_sum`    — `×` and `+` over `?d`, folded by `SUM`.
//! - `c_order_by_decimal`          — whole-relation `ORDER BY` over `?d`, `LIMIT 10`.
//! - `d_sum_avg_integer`           — `SUM` and `AVG` over `?i`.
//! - `e_sum_avg_decimal`           — `SUM` and `AVG` over `?d`.
//! - `f_filter_compare`            — `FILTER` comparing `?i` and `?d` to constants.
//!
//! The `exact` group runs values past machine words, on the arbitrary-precision
//! path: a forty-digit `?b` column through `×`, `SUM` and `ORDER BY`, and a chain of
//! squarings of a hundred-digit integer, ungoverned and under a fuel ceiling it fits
//! (so the governor charges every product).
//!
//! The `group` group runs a per-group `SUM` over that column into 5 000 groups of four
//! (enough groups that the fold forks), ungoverned, under a fuel ceiling it fits, under a
//! scratch ceiling it fits, and metered: the governed folds go through the ordered
//! ledger that keeps their trip in group order (`crate::row_checkpoint`), and their
//! cost against the ungoverned fold is that ledger's.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench numeric_eval` (the
//! `make bench` lane) — excluded from `make check`. Timings are not asserted.

use std::sync::Arc;

#[path = "../tests/support/mod.rs"]
mod support;

use purrdf_testkit::bench::{Bench, bench_group, bench_main};

use purrdf_core::{RdfDataset, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};
use support::governor_workloads::{GROUPED, grouped_answer, numeric_dataset};
use support::squaring_chain;

const IN_RANGE: &[(&str, &str)] = &[
    (
        "a_integer_arithmetic_filter",
        "PREFIX ex: <https://example.org/> SELECT ?s WHERE { ?s ex:i ?i \
         FILTER((?i * 3 + 7) - ?i / 4 > 250000) }",
    ),
    (
        "b_decimal_arithmetic_sum",
        "PREFIX ex: <https://example.org/> SELECT (SUM(?d * 1.5 + 0.25) AS ?t) \
         WHERE { ?s ex:d ?d }",
    ),
    (
        "c_order_by_decimal",
        "PREFIX ex: <https://example.org/> SELECT ?s ?d WHERE { ?s ex:d ?d } \
         ORDER BY DESC(?d) ?s LIMIT 10",
    ),
    (
        "d_sum_avg_integer",
        "PREFIX ex: <https://example.org/> SELECT (SUM(?i) AS ?t) (AVG(?i) AS ?m) \
         WHERE { ?s ex:i ?i }",
    ),
    (
        "e_sum_avg_decimal",
        "PREFIX ex: <https://example.org/> SELECT (SUM(?d) AS ?t) (AVG(?d) AS ?m) \
         WHERE { ?s ex:d ?d }",
    ),
    (
        "f_filter_compare",
        "PREFIX ex: <https://example.org/> SELECT ?s WHERE { ?s ex:i ?i ; ex:d ?d \
         FILTER(?d >= 500.5 && ?i < 600000) }",
    ),
];

const EXACT: &[(&str, &str)] = &[
    (
        "x_big_multiply_sum",
        "PREFIX ex: <https://example.org/> SELECT (SUM(?b * ?b) AS ?t) \
         WHERE { ?s ex:b ?b }",
    ),
    (
        "y_big_order_by",
        "PREFIX ex: <https://example.org/> SELECT ?s ?b WHERE { ?s ex:b ?b } \
         ORDER BY DESC(?b) LIMIT 10",
    ),
    (
        "z_big_avg",
        "PREFIX ex: <https://example.org/> SELECT (AVG(?b) AS ?m) WHERE { ?s ex:b ?b }",
    ),
];

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// Run one query, returning its solution count, or zero for an evaluation error
/// (the same file builds against an engine that refuses the exact lanes).
fn run(engine: &NativeSparqlEngine, ds: &Arc<RdfDataset>, query: &str) -> usize {
    match engine.query(ds, request(query)) {
        Ok(SparqlResult::Solutions { rows, .. }) => rows.len(),
        Ok(_) | Err(_) => 0,
    }
}

fn governed(
    engine: &NativeSparqlEngine,
    ds: &Arc<RdfDataset>,
    query: &str,
    governors: &QueryGovernors,
) -> bool {
    matches!(
        engine.query_governed(ds, request(query), QueryOptions::EMPTY, governors),
        Ok(GovernedOutcome::Complete { .. })
    )
}

fn bench_in_range(c: &mut Bench) {
    let ds = numeric_dataset();
    let engine = NativeSparqlEngine::new();
    for &(label, query) in IN_RANGE {
        assert!(run(&engine, &ds, query) > 0, "case {label} is a no-op");
    }
    let mut group = c.benchmark_group("numeric_eval_in_range");
    group.sample_size(10);
    for &(label, query) in IN_RANGE {
        group.bench_function(label, |bencher| {
            bencher.iter(|| std::hint::black_box(run(&engine, &ds, query)));
        });
    }
    group.finish();
}

fn bench_exact(c: &mut Bench) {
    let ds = numeric_dataset();
    let engine = NativeSparqlEngine::new();
    let chain = squaring_chain(100, 6);
    let fits = QueryGovernors::UNBOUNDED.with_fuel(1_000_000_000);
    // An error or a trip would be timed as if it were the work: prove each lane answers.
    for &(label, query) in EXACT {
        assert!(run(&engine, &ds, query) > 0, "case {label} is a no-op");
    }
    assert!(
        run(&engine, &ds, &chain) > 0,
        "the squaring chain is a no-op"
    );
    assert!(
        governed(&engine, &ds, &chain, &fits),
        "the squaring chain completes under its fuel ceiling"
    );
    let mut group = c.benchmark_group("numeric_eval_exact");
    group.sample_size(10);
    for &(label, query) in EXACT {
        group.bench_function(label, |bencher| {
            bencher.iter(|| std::hint::black_box(run(&engine, &ds, query)));
        });
    }
    group.bench_function("squaring_chain_ungoverned", |bencher| {
        bencher.iter(|| std::hint::black_box(run(&engine, &ds, &chain)));
    });
    group.bench_function("squaring_chain_governed", |bencher| {
        bencher.iter(|| std::hint::black_box(governed(&engine, &ds, &chain, &fits)));
    });
    group.finish();
}

fn bench_group_by(c: &mut Bench) {
    let ds = numeric_dataset();
    let engine = NativeSparqlEngine::new();
    let expected = grouped_answer(&engine, &ds, None);
    let lanes = [
        ("fuel", QueryGovernors::UNBOUNDED.with_fuel(1_000_000_000)),
        (
            "scratch",
            QueryGovernors::UNBOUNDED.with_max_scratch_bytes(1 << 32),
        ),
        ("metered", QueryGovernors::METERED),
    ];
    for (label, governors) in &lanes {
        assert_eq!(
            grouped_answer(&engine, &ds, Some(governors)),
            expected,
            "all group SUM values agree {label}"
        );
    }
    let mut group = c.benchmark_group("numeric_eval_group");
    group.sample_size(10);
    group.bench_function("group_by_exact_ungoverned", |bencher| {
        bencher.iter(|| std::hint::black_box(run(&engine, &ds, GROUPED)));
    });
    for (label, governors) in &lanes {
        group.bench_function(format!("group_by_exact_{label}"), |bencher| {
            bencher.iter(|| std::hint::black_box(governed(&engine, &ds, GROUPED, governors)));
        });
    }
    group.finish();
}

bench_group!(benches, bench_in_range, bench_exact, bench_group_by);
bench_main!(benches);
