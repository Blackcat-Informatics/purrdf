// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! The per-row cost of a `FILTER` or `BIND` expression, over 100 000 rows.
//!
//! Every case is a single triple pattern — one scan, no join — followed by one
//! `FILTER` or `BIND`, so what grows with the row count is the expression's own
//! evaluation. `scan_only` is the same scan with no expression at all: the floor
//! every other row sits on, and the control that makes the expression's share
//! readable. The plan is prepared once, outside the timed loop, so each sample is
//! evaluation alone.
//!
//! Cases:
//! - `scan_only` — the control: `?s ex:v ?v` and nothing else.
//! - `numeric_compare` — `FILTER(?v > 500)`, one value-space comparison per row.
//! - `and_chain_2`, `and_chain_4`, `and_chain_8` — `&&` of 2, 4 and 8 comparisons,
//!   every one of them true on every row, so no row short-circuits and the whole
//!   chain is evaluated each time.
//! - `arithmetic` — `BIND((?v * 3 + 7) / 2 - ?v AS ?x)`.
//! - `if_branch` — `BIND(IF(?v > 500, ?v, -?v) AS ?x)`, both arms taken.
//! - `coalesce` — `BIND(COALESCE(?unbound, ?v) AS ?x)`: the first argument raises
//!   on every row, so every row takes the fallback.
//! - `in_10` — `FILTER(?v IN (…ten integers…))`.
//! - `regex_constant` — `FILTER(REGEX(?n, "^Name[0-9]*7$"))`, a constant pattern
//!   compiled once and matched per row.
//! - `bnode_str` — `BIND(BNODE(STR(?s)) AS ?b)`: a stateful built-in, which keeps
//!   the loop on its sequential path and mints one blank node per row.
//! - `str_concat` — `BIND(CONCAT(STR(?s), "/", ?n) AS ?c)`, one string minted per row.
//! - `same_term_iri` — `FILTER(sameTerm(?r, ex:ref7))`, term identity on IRIs.
//! - `equal_iri` — `FILTER(?r = ex:ref7)`, the `=` operator on the same IRIs.
//!
//! All IRIs are `example.org` fixtures. The rayon global pool is left at its
//! default; a case whose expression is parallel-safe forks its row loop exactly as
//! production does.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench expr_vm` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::sync::Arc;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};

/// Subjects in the dataset, and so rows per scan.
const ROWS: usize = 100_000;

const EX: &str = "https://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Per subject `i`: `ex:v i % 1000` (an `xsd:integer`), `ex:name "Name{i}"` and
/// `ex:ref ex:ref{i % 100}`. Built by a counter — no RNG, no iteration-order
/// dependence.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p_v = b.intern_iri(&format!("{EX}v"));
    let p_name = b.intern_iri(&format!("{EX}name"));
    let p_ref = b.intern_iri(&format!("{EX}ref"));
    let refs: Vec<_> = (0..100)
        .map(|r| b.intern_iri(&format!("{EX}ref{r}")))
        .collect();
    for i in 0..ROWS {
        let s = b.intern_iri(&format!("{EX}s{i}"));
        let v = b.intern_literal(RdfLiteral::typed((i % 1000).to_string(), XSD_INTEGER));
        b.push_quad(s, p_v, v, None);
        let name = b.intern_literal(RdfLiteral::simple(format!("Name{i}")));
        b.push_quad(s, p_name, name, None);
        b.push_quad(s, p_ref, refs[i % 100], None);
    }
    b.freeze().expect("freeze the expression dataset")
}

/// `?v != -1 && ?v != -2 && …`, `terms` comparisons that hold on every row.
fn and_chain(terms: usize) -> String {
    let mut chain = String::new();
    for k in 1..=terms {
        if k > 1 {
            chain.push_str(" && ");
        }
        let _ = write!(chain, "?v != -{k}");
    }
    format!("SELECT ?s WHERE {{ ?s <{EX}v> ?v FILTER({chain}) }}")
}

/// The measured cases as `(criterion id, query text, minimum expected rows)`. The
/// floor proves each case does real work: an empty result would benchmark a no-op.
fn cases() -> Vec<(&'static str, String, usize)> {
    let v = format!("<{EX}v>");
    let name = format!("<{EX}name>");
    let r = format!("<{EX}ref>");
    vec![
        (
            "scan_only",
            format!("SELECT ?s WHERE {{ ?s {v} ?v }}"),
            ROWS,
        ),
        (
            "numeric_compare",
            format!("SELECT ?s WHERE {{ ?s {v} ?v FILTER(?v > 500) }}"),
            ROWS / 2 - ROWS / 1000,
        ),
        ("and_chain_2", and_chain(2), ROWS),
        ("and_chain_4", and_chain(4), ROWS),
        ("and_chain_8", and_chain(8), ROWS),
        (
            "arithmetic",
            format!("SELECT ?s ?x WHERE {{ ?s {v} ?v BIND((?v * 3 + 7) / 2 - ?v AS ?x) }}"),
            ROWS,
        ),
        (
            "if_branch",
            format!("SELECT ?s ?x WHERE {{ ?s {v} ?v BIND(IF(?v > 500, ?v, -?v) AS ?x) }}"),
            ROWS,
        ),
        (
            "coalesce",
            format!("SELECT ?s ?x WHERE {{ ?s {v} ?v BIND(COALESCE(?unbound, ?v) AS ?x) }}"),
            ROWS,
        ),
        (
            "in_10",
            format!(
                "SELECT ?s WHERE {{ ?s {v} ?v \
                 FILTER(?v IN (3, 17, 101, 256, 333, 500, 641, 777, 900, 999)) }}"
            ),
            10 * (ROWS / 1000),
        ),
        (
            "regex_constant",
            format!("SELECT ?s WHERE {{ ?s {name} ?n FILTER(REGEX(?n, \"^Name[0-9]*7$\")) }}"),
            ROWS / 10,
        ),
        (
            "bnode_str",
            format!("SELECT ?s ?b WHERE {{ ?s {v} ?v BIND(BNODE(STR(?s)) AS ?b) }}"),
            ROWS,
        ),
        (
            "str_concat",
            format!("SELECT ?s ?c WHERE {{ ?s {name} ?n BIND(CONCAT(STR(?s), \"/\", ?n) AS ?c) }}"),
            ROWS,
        ),
        (
            "same_term_iri",
            format!("SELECT ?s WHERE {{ ?s {r} ?r FILTER(sameTerm(?r, <{EX}ref7>)) }}"),
            ROWS / 100,
        ),
        (
            "equal_iri",
            format!("SELECT ?s WHERE {{ ?s {r} ?r FILTER(?r = <{EX}ref7>) }}"),
            ROWS / 100,
        ),
    ]
}

/// Evaluate a prepared plan and return its solution count.
fn run(engine: &NativeSparqlEngine, data: &Arc<RdfDataset>, prepared: &PreparedQuery) -> usize {
    match engine
        .query_prepared(data, prepared, &[], QueryOptions::EMPTY)
        .expect("the expression benchmark query evaluates")
    {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(value) => usize::from(value),
    }
}

fn bench_expr_vm(c: &mut Criterion) {
    let data = dataset();
    let engine = NativeSparqlEngine::new();
    let prepared: Vec<_> = cases()
        .into_iter()
        .map(|(label, query, floor)| {
            let plan = engine
                .prepare_query(&query, None)
                .expect("the expression benchmark query parses");
            let observed = run(&engine, &data, &plan);
            assert!(
                observed >= floor,
                "case {label} returned {observed} rows (< {floor}) — the benchmark would be a no-op"
            );
            (label, plan)
        })
        .collect();

    let mut group = c.benchmark_group("expr_vm");
    // Whole-relation evaluations over 100 000 rows run milliseconds; keep sampling
    // light, like `query_eval`, so the full case list stays tractable.
    group.sample_size(10);
    group.throughput(Throughput::Elements(ROWS as u64));
    for (label, plan) in &prepared {
        group.bench_function(*label, |bencher| {
            bencher.iter(|| black_box(run(&engine, &data, plan)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_expr_vm);
criterion_main!(benches);
