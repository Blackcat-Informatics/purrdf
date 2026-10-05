// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// The pre-binding lane is deprecated and inert; these tests still name it.
#![allow(deprecated)]

//! A pre-bound variable survives `GROUP BY` and aggregation.
//!
//! The grouping constraint (SPARQL 1.1 §11.4) exempts a variable the caller declares
//! pre-bound — a prepared execution's parameter, a request substitution, SHACL's
//! `$this` — because it holds one value for the whole evaluation, so every group reads
//! the same value. That exemption is only sound if the evaluator carries the value
//! past the `Group`, which keeps only its keys and its aggregates: reading the
//! variable above the group must give the bound value, never an unbound cell.
//!
//! Each case runs on every lane that pre-binds — a prepared execution, a request's
//! substitutions naming either `ShaclPrebinding` value, which select the one rewrite —
//! and checks the bound value appears in the grouped output, beside the aggregate it
//! was grouped with. Fixture IRIs are `example.org`.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions, ShaclPrebinding};

const EX: &str = "http://example.org/";

/// `ex:a ex:p ex:o1, ex:o2 . ex:b ex:p ex:o3 .`
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for (s, o) in [("a", "o1"), ("a", "o2"), ("b", "o3")] {
        let s = builder.intern_iri(&format!("{EX}{s}"));
        let o = builder.intern_iri(&format!("{EX}{o}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("a dataset")
}

fn a() -> TermValue {
    TermValue::Iri(format!("{EX}a"))
}

/// One answer row as `(variable, value)` pairs, sorted for comparison.
type Row = Vec<(String, Option<TermValue>)>;

fn sorted(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|x, y| format!("{x:?}").cmp(&format!("{y:?}")));
    rows
}

/// The rows of `query` with `$this` bound to `ex:a`, on the request-substitution lane
/// `lane`.
fn substituted(query: &str, lane: ShaclPrebinding) -> Vec<Row> {
    let dataset = dataset();
    let substitutions = [("this".to_owned(), a())];
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &substitutions,
            },
            QueryOptions::EMPTY.with_prebinding(lane),
        )
        .unwrap_or_else(|e| panic!("{query}: {e:?}"));
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("{query}: expected solutions");
    };
    sorted(
        rows.into_iter()
            .map(|row| variables.iter().cloned().zip(row).collect())
            .collect(),
    )
}

/// The rows of `query` run as a prepared execution with the parameter `this` bound to
/// `ex:a`.
fn prepared(query: &str) -> Vec<Row> {
    let dataset = dataset();
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(query, None, &["this"], QueryOptions::EMPTY)
        .unwrap_or_else(|e| panic!("{query}: {e:?}"));
    execution.bind(0, a()).expect("bind");
    let rows = engine
        .execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("{query}: expected solutions");
            };
            solutions
                .rows()
                .iter()
                .map(|row| {
                    solutions
                        .variables()
                        .iter()
                        .enumerate()
                        .map(|(column, var)| (var.as_str().to_owned(), solutions.cell(row, column)))
                        .collect::<Row>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|e| panic!("{query}: {e:?}"));
    sorted(rows)
}

/// Every pre-binding lane answers `query` with exactly `expected`.
fn assert_every_lane(query: &str, expected: &[Row]) {
    let expected = sorted(expected.to_vec());
    assert_eq!(prepared(query), expected, "prepared execution: {query}");
    for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
        assert_eq!(substituted(query, lane), expected, "{lane:?}: {query}");
    }
}

fn cell(name: &str, value: TermValue) -> (String, Option<TermValue>) {
    (name.to_owned(), Some(value))
}

fn integer(value: u32) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_string(),
        datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
        language: None,
        direction: None,
    }
}

#[test]
fn a_projected_pre_bound_variable_survives_an_implicit_group() {
    assert_every_lane(
        &format!("SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}"),
        &[vec![cell("this", a()), cell("c", integer(2))]],
    );
}

#[test]
fn a_select_expression_reads_the_pre_bound_value_above_the_group() {
    assert_every_lane(
        &format!("SELECT (STR($this) AS ?t) (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}"),
        &[vec![
            cell(
                "t",
                TermValue::Literal {
                    lexical_form: format!("{EX}a"),
                    datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    language: None,
                    direction: None,
                },
            ),
            cell("c", integer(2)),
        ]],
    );
}

#[test]
fn a_pre_bound_variable_survives_an_explicit_group_by_another_key() {
    assert_every_lane(
        &format!("SELECT $this ?o WHERE {{ $this <{EX}p> ?o }} GROUP BY ?o"),
        &[
            vec![
                cell("this", a()),
                cell("o", TermValue::Iri(format!("{EX}o1"))),
            ],
            vec![
                cell("this", a()),
                cell("o", TermValue::Iri(format!("{EX}o2"))),
            ],
        ],
    );
    assert_every_lane(
        &format!(
            "SELECT ?o (BOUND($this) AS ?r) WHERE {{ $this <{EX}p> ?o }} GROUP BY ?o HAVING (BOUND($this))"
        ),
        &[
            vec![
                cell("o", TermValue::Iri(format!("{EX}o1"))),
                cell("r", boolean(true)),
            ],
            vec![
                cell("o", TermValue::Iri(format!("{EX}o2"))),
                cell("r", boolean(true)),
            ],
        ],
    );
}

#[test]
fn a_group_keyed_by_the_pre_bound_variable_is_unchanged() {
    // The neighbour that worked before: grouping BY the pre-bound variable keeps it
    // as a key.
    assert_every_lane(
        &format!("SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }} GROUP BY $this"),
        &[vec![cell("this", a()), cell("c", integer(2))]],
    );
}

fn boolean(value: bool) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_string(),
        datatype: "http://www.w3.org/2001/XMLSchema#boolean".to_owned(),
        language: None,
        direction: None,
    }
}
