// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The fixed cost of one query: what a request pays that does not grow with the
//! data, measured over a dataset so small that the rows cost next to nothing.
//!
//! Three query shapes:
//! - `bgp_1` — a single triple pattern, the least a `SELECT` can do.
//! - `join_3_optional_filter` — three joined patterns, an `OPTIONAL` and a
//!   `FILTER`: every operator the common query touches, each once.
//! - `operator_dense` — sixteen nested `SELECT DISTINCT` levels, each a join, a
//!   `BIND` and a `FILTER` around the level inside it: about a hundred algebra
//!   nodes over eight rows, so the time is the evaluator's per-node dispatch
//!   (`eval::eval_node` and the out-of-line operators it calls), not row work.
//!
//! Each shape is measured three ways:
//! - `cold_text` — a fresh engine per sample, so the text is parsed, admitted,
//!   planned and evaluated from nothing;
//! - `warm_text` — one engine across samples, so its plan cache answers the parse
//!   and only the request lookup and evaluation remain;
//! - `prepared` — a plan prepared once and handed back each sample, the reuse a
//!   host that runs the same query many times has.
//!
//! Beside them, `bgp_heavy/prepared` runs `join_3_optional_filter`'s text over
//! 50 000 people: a handful of nodes over tens of thousands of rows, so the time is
//! the scans and the join, and the dispatch is noise — the control a dispatcher
//! change must not move.
//!
//! The dataset is eight `example.org` people. Report-only,
//! `cargo bench -p purrdf-sparql-eval --bench pattern_dispatch` (the `make bench`
//! lane) — excluded from `make check`. No timing is asserted.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

const EX: &str = "https://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// People in the dataset.
const PEOPLE: usize = 8;

/// People in the `bgp_heavy` dataset.
const HEAVY_PEOPLE: usize = 50_000;

/// Nested levels of the `operator_dense` shape.
const DENSE_LEVELS: usize = 16;

/// Per person `i` of `people`: `ex:name`, `ex:age 18 + 3i`, `ex:knows` the next
/// person, and `ex:email` for even `i` only, so the `OPTIONAL` both extends and pads.
fn dataset(people: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p_name = b.intern_iri(&format!("{EX}name"));
    let p_age = b.intern_iri(&format!("{EX}age"));
    let p_knows = b.intern_iri(&format!("{EX}knows"));
    let p_email = b.intern_iri(&format!("{EX}email"));
    let people: Vec<_> = (0..people)
        .map(|i| b.intern_iri(&format!("{EX}person{i}")))
        .collect();
    for (i, &person) in people.iter().enumerate() {
        let name = b.intern_literal(RdfLiteral::simple(format!("Name{i}")));
        b.push_quad(person, p_name, name, None);
        let age = b.intern_literal(RdfLiteral::typed((18 + 3 * i).to_string(), XSD_INTEGER));
        b.push_quad(person, p_age, age, None);
        b.push_quad(person, p_knows, people[(i + 1) % people.len()], None);
        if i % 2 == 0 {
            let email = b.intern_literal(RdfLiteral::simple(format!("p{i}@example.org")));
            b.push_quad(person, p_email, email, None);
        }
    }
    b.freeze().expect("freeze the dispatch dataset")
}

/// The single-pattern `SELECT`.
const BGP_1: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?s ?n WHERE { ?s ex:name ?n }";

/// Three joined patterns, an `OPTIONAL` and a `FILTER`: the ages run 18..=39, so
/// the filter keeps the six people past 22.
const JOIN_3_OPTIONAL_FILTER: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?s ?n ?a ?f ?e WHERE {
  ?s ex:name ?n .
  ?s ex:age ?a .
  ?s ex:knows ?f .
  OPTIONAL { ?s ex:email ?e }
  FILTER(?a > 22)
}";

/// `levels` nested `SELECT DISTINCT` subqueries around `?s ex:name ?n`, each
/// joining its own `?s ex:age ?aN`, binding `?aN + N` and filtering on it. Every person
/// has an age of at least 18, so every filter keeps every row: the answer is one
/// row per person, whatever the depth.
fn operator_dense(levels: usize) -> String {
    let mut body = String::from("?s ex:name ?n");
    for level in 1..=levels {
        body = format!(
            "{{ SELECT DISTINCT ?s ?n WHERE {{ {{ {body} }} ?s ex:age ?a{level} \
             BIND(?a{level} + {level} AS ?b{level}) FILTER(?b{level} > {level}) }} }}"
        );
    }
    format!("PREFIX ex: <https://example.org/>\nSELECT ?s ?n WHERE {{ {body} }}")
}

/// The size of any result: its rows, its graph's quads, or `1` for a `true` `ASK`.
fn result_size(result: &SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(value) => usize::from(*value),
    }
}

/// `(bench id, query text, exact expected rows)`.
fn shapes() -> Vec<(&'static str, String, usize)> {
    vec![
        ("bgp_1", BGP_1.to_owned(), PEOPLE),
        (
            "join_3_optional_filter",
            JOIN_3_OPTIONAL_FILTER.to_owned(),
            PEOPLE - 2,
        ),
        ("operator_dense", operator_dense(DENSE_LEVELS), PEOPLE),
    ]
}

fn run_text(engine: &NativeSparqlEngine, data: &Arc<RdfDataset>, query: &str) -> usize {
    result_size(
        &engine
            .query(
                data,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("the dispatch benchmark query evaluates"),
    )
}

fn bench_pattern_dispatch(c: &mut Bench) {
    let data = dataset(PEOPLE);
    let warm = NativeSparqlEngine::new();
    let mut group = c.benchmark_group("pattern_dispatch");
    for (shape, query, expected) in shapes() {
        let query = query.as_str();
        let prepared = warm
            .prepare_query(query, None)
            .expect("the dispatch benchmark query parses");
        // Every lane answers the same rows before any is timed, and the warm lane's
        // plan cache is populated by this pass.
        assert_eq!(run_text(&NativeSparqlEngine::new(), &data, query), expected);
        assert_eq!(run_text(&warm, &data, query), expected);
        assert_eq!(
            result_size(
                &warm
                    .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                    .expect("the prepared plan evaluates")
            ),
            expected
        );

        group.bench_function(format!("{shape}/cold_text"), |bencher| {
            bencher.iter(|| {
                black_box(run_text(
                    &NativeSparqlEngine::new(),
                    &data,
                    black_box(query),
                ))
            });
        });
        group.bench_function(format!("{shape}/warm_text"), |bencher| {
            bencher.iter(|| black_box(run_text(&warm, &data, black_box(query))));
        });
        group.bench_function(format!("{shape}/prepared"), |bencher| {
            bencher.iter(|| {
                black_box(result_size(
                    &warm
                        .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                        .expect("the prepared plan evaluates"),
                ))
            });
        });
    }

    // The control: few nodes, many rows. Only the prepared lane, so parsing and
    // planning are not in it either.
    let heavy = dataset(HEAVY_PEOPLE);
    let prepared = warm
        .prepare_query(JOIN_3_OPTIONAL_FILTER, None)
        .expect("the dispatch benchmark query parses");
    let run_heavy = || {
        result_size(
            &warm
                .query_prepared(&heavy, &prepared, &[], QueryOptions::EMPTY)
                .expect("the prepared plan evaluates"),
        )
    };
    // Ages run 18 + 3i, so every person but the first two is past 22.
    assert_eq!(run_heavy(), HEAVY_PEOPLE - 2);
    group.bench_function("bgp_heavy/prepared", |bencher| {
        bencher.iter(|| black_box(run_heavy()));
    });
    group.finish();
}

bench_group!(benches, bench_pattern_dispatch);
bench_main!(benches);
