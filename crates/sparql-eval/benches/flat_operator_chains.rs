// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Parsing and evaluating a flat operator chain: `?v = 0 || ?v = 1 || …`,
//! `?v + 1 + 2.5 + …`, `{ … } UNION { … } UNION …`, and the property paths
//! `<next>/<next>/…` and `<q1>|<q2>|…|<next>`, each read into one n-ary algebra node,
//! at 2, 100 and 10 000 terms.
//!
//! `parse/*` is the parser alone (`SparqlParser::parse_query`); `evaluate/*` is a
//! whole cold request through [`NativeSparqlEngine::query`] with a fresh engine per
//! sample, so the plan cache never answers it — parse, admission and evaluation over
//! a 30-subject dataset — or, for the paths, a 10 000-hop `<next>` chain, which the
//! sequence walks from its first node as far as it has steps and the alternative's
//! last arm steps once. The 2-term cases are the common shape the change must not
//! slow; the 100- and 10 000-term cases show how the cost grows with the chain.
//! Report-only: no speedup is asserted, and none of these is a gate.

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::NativeSparqlEngine;

const EX: &str = "http://example.org/";

/// `<s{i}> <p> i` for `i` in `0..30`.
fn dataset() -> std::sync::Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for i in 0..30 {
        let s = builder.intern_iri(&format!("{EX}s{i}"));
        let o = builder.intern_literal(RdfLiteral {
            lexical_form: i.to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
            language: None,
            direction: None,
        });
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the bench dataset")
}

/// `<c{i}> <next> <c{i+1}>` for `i` in `0..10 000`: the chain the path shapes walk.
fn path_dataset() -> std::sync::Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let next = builder.intern_iri(&format!("{EX}next"));
    let mut from = builder.intern_iri(&format!("{EX}c0"));
    for i in 1..=10_000 {
        let to = builder.intern_iri(&format!("{EX}c{i}"));
        builder.push_quad(from, next, to, None);
        from = to;
    }
    builder.freeze().expect("the path bench dataset")
}

/// The five chain shapes, `terms` long, as whole queries.
fn queries(terms: usize) -> [(&'static str, String); 5] {
    let mut or = String::new();
    let mut sum = String::from("?v");
    for k in 0..terms {
        if k > 0 {
            or.push_str(" || ");
        }
        let _ = write!(or, "?v = {}", k * 7);
        if k > 0 {
            sum.push_str([" + 1", " + 2.5", " * 1.0E0", " - 3"][k % 4]);
        }
    }
    let union = (0..terms)
        .map(|k| format!("{{ ?s <{EX}p> ?v FILTER(?v = {}) }}", k % 30))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let sequence = vec![format!("<{EX}next>"); terms].join("/");
    let alternative = (1..terms)
        .map(|k| format!("<{EX}q{k}>"))
        .chain([format!("<{EX}next>")])
        .collect::<Vec<_>>()
        .join("|");
    [
        (
            "or",
            format!("SELECT ?s WHERE {{ ?s <{EX}p> ?v FILTER({or}) }}"),
        ),
        (
            "sum",
            format!("SELECT ?s ?r WHERE {{ ?s <{EX}p> ?v BIND({sum} AS ?r) }}"),
        ),
        ("union", format!("SELECT ?s WHERE {{ {union} }}")),
        (
            "path_sequence",
            format!("SELECT ?o WHERE {{ <{EX}c0> {sequence} ?o }}"),
        ),
        (
            "path_alternative",
            format!("SELECT ?o WHERE {{ <{EX}c0> {alternative} ?o }}"),
        ),
    ]
}

fn bench_flat_chains(c: &mut Criterion) {
    let data = dataset();
    let path_data = path_dataset();
    let mut group = c.benchmark_group("flat_operator_chains");
    group.sample_size(10);
    for terms in [2, 100, 10_000] {
        for (shape, query) in queries(terms) {
            let data = if shape.starts_with("path_") {
                &path_data
            } else {
                &data
            };
            group.bench_function(format!("parse/{shape}/{terms}"), |b| {
                b.iter(|| {
                    black_box(
                        SparqlParser::new()
                            .parse_query(black_box(&query))
                            .expect("parses"),
                    )
                });
            });
            group.bench_function(format!("evaluate/{shape}/{terms}"), |b| {
                b.iter(|| {
                    black_box(
                        NativeSparqlEngine::new()
                            .query(
                                black_box(data),
                                SparqlRequest {
                                    query: &query,
                                    base_iri: None,
                                    substitutions: &[],
                                },
                            )
                            .expect("answers"),
                    )
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_flat_chains);
criterion_main!(benches);
