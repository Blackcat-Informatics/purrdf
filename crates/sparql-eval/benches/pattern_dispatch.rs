// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The fixed cost of one query: what a request pays that does not grow with the
//! data, measured over a dataset so small that the rows cost next to nothing.
//!
//! Two query shapes:
//! - `bgp_1` — a single triple pattern, the least a `SELECT` can do.
//! - `join_3_optional_filter` — three joined patterns, an `OPTIONAL` and a
//!   `FILTER`: every operator the common query touches, each once.
//!
//! Each shape is measured three ways:
//! - `cold_text` — a fresh engine per sample, so the text is parsed, admitted,
//!   planned and evaluated from nothing;
//! - `warm_text` — one engine across samples, so its plan cache answers the parse
//!   and only the request lookup and evaluation remain;
//! - `prepared` — a plan prepared once and handed back each sample, the reuse a
//!   host that runs the same query many times has.
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

/// Per person `i`: `ex:name`, `ex:age 18 + 3i`, `ex:knows` the next person, and
/// `ex:email` for even `i` only, so the `OPTIONAL` both extends and pads.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p_name = b.intern_iri(&format!("{EX}name"));
    let p_age = b.intern_iri(&format!("{EX}age"));
    let p_knows = b.intern_iri(&format!("{EX}knows"));
    let p_email = b.intern_iri(&format!("{EX}email"));
    let people: Vec<_> = (0..PEOPLE)
        .map(|i| b.intern_iri(&format!("{EX}person{i}")))
        .collect();
    for (i, &person) in people.iter().enumerate() {
        let name = b.intern_literal(RdfLiteral::simple(format!("Name{i}")));
        b.push_quad(person, p_name, name, None);
        let age = b.intern_literal(RdfLiteral::typed((18 + 3 * i).to_string(), XSD_INTEGER));
        b.push_quad(person, p_age, age, None);
        b.push_quad(person, p_knows, people[(i + 1) % PEOPLE], None);
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

/// `(bench id, query text, exact expected rows)`.
const SHAPES: &[(&str, &str, usize)] = &[
    ("bgp_1", BGP_1, PEOPLE),
    ("join_3_optional_filter", JOIN_3_OPTIONAL_FILTER, PEOPLE - 2),
];

fn rows(result: SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(value) => usize::from(value),
    }
}

fn run_text(engine: &NativeSparqlEngine, data: &Arc<RdfDataset>, query: &str) -> usize {
    rows(
        engine
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
    let data = dataset();
    let warm = NativeSparqlEngine::new();
    let mut group = c.benchmark_group("pattern_dispatch");
    for &(shape, query, expected) in SHAPES {
        let prepared = warm
            .prepare_query(query, None)
            .expect("the dispatch benchmark query parses");
        // Every lane answers the same rows before any is timed, and the warm lane's
        // plan cache is populated by this pass.
        assert_eq!(run_text(&NativeSparqlEngine::new(), &data, query), expected);
        assert_eq!(run_text(&warm, &data, query), expected);
        assert_eq!(
            rows(
                warm.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
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
                black_box(rows(
                    warm.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                        .expect("the prepared plan evaluates"),
                ))
            });
        });
    }
    group.finish();
}

bench_group!(benches, bench_pattern_dispatch);
bench_main!(benches);
