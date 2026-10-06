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

/// The message of `query`'s refusal on the prepared lane, preparing with the
/// parameter `this` and further declaring `declared` pre-bound; `None` when it is
/// admitted.
fn prepared_refusal(query: &str, declared: &[&str]) -> Option<String> {
    NativeSparqlEngine::new()
        .prepare_execution(
            query,
            None,
            &["this"],
            QueryOptions::EMPTY.with_declared_prebound(declared),
        )
        .err()
        .map(|e| e.message)
}

/// The refusal of `query` on the request lane with `$this` substituted, or `None`.
fn substituted_refusal(query: &str) -> Option<String> {
    let dataset = dataset();
    let substitutions = [("this".to_owned(), a())];
    NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &substitutions,
            },
            QueryOptions::EMPTY,
        )
        .err()
        .map(|e| e.message)
}

/// The rows of `query` through [`SparqlEngine::query`] with `$this` substituted.
fn trait_query(query: &str) -> Vec<Row> {
    use purrdf_core::SparqlEngine;
    let dataset = dataset();
    let substitutions = [("this".to_owned(), a())];
    let result = NativeSparqlEngine::new()
        .query(
            &dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &substitutions,
            },
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

/// SPARQL scoping (§18.2.1) ends a variable at a sub-`SELECT`'s projection, so a
/// sub-`SELECT` that assigns `?this` without projecting it binds a variable of its own:
/// the caller's binding of the outer `?this` is not involved, and every row answers
/// as it would with any other name there — on every engine lane, read inside a
/// `SELECT` expression too. A sub-`SELECT` that DOES project its assigned `?this`
/// hands the outer query that variable, which joins with the bound value like any
/// other binding: `ex:b` against `ex:a` is no row.
#[test]
fn a_sub_select_assigning_the_pre_bound_name_unprojected_binds_its_own_variable() {
    let every_object = [
        vec![cell("o", TermValue::Iri(format!("{EX}o1")))],
        vec![cell("o", TermValue::Iri(format!("{EX}o2")))],
        vec![cell("o", TermValue::Iri(format!("{EX}o3")))],
    ];
    for name in ["this", "fresh"] {
        let query = format!(
            "SELECT ?o WHERE {{ {{ SELECT ?o WHERE {{ ?s <{EX}p> ?o BIND(?s AS ?{name}) \
             FILTER(?{name} = ?s) }} }} }}"
        );
        assert_eq!(prepared_refusal(&query, &[]), None, "{query}");
        assert_eq!(substituted_refusal(&query), None, "{query}");
        assert_every_lane(&query, &every_object);
        // The trait door prepares the text without the substitution's names and
        // rewrites per run; it answers the same.
        assert_eq!(
            trait_query(&query),
            sorted(every_object.to_vec()),
            "{query}"
        );
    }
    let read_in_projection = format!(
        "SELECT ?t WHERE {{ {{ SELECT (STR(?this) AS ?t) WHERE {{ ?s <{EX}p> ?o \
         BIND(<{EX}z> AS ?this) }} }} }}"
    );
    let z = TermValue::Literal {
        lexical_form: format!("{EX}z"),
        datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
        language: None,
        direction: None,
    };
    assert_every_lane(
        &read_in_projection,
        &[
            vec![cell("t", z.clone())],
            vec![cell("t", z.clone())],
            vec![cell("t", z)],
        ],
    );
    let projected = format!(
        "SELECT ?o WHERE {{ {{ SELECT ?this WHERE {{ BIND(<{EX}b> AS ?this) }} }} \
         ?s <{EX}p> ?o }}"
    );
    assert_every_lane(&projected, &[]);
}

/// An assignment of a pre-bound name where it is not in scope binds that name for the
/// rows the assignment produces, which then join with the bound value as SPARQL joins
/// any two bindings: an `OPTIONAL` arm assigning `ex:z` is incompatible with the bound
/// `ex:a`, so each row keeps its left side; a nested group assigning it joins no row;
/// and an assignment no other pattern meets answers with the assigned value.
#[test]
fn an_assignment_of_the_pre_bound_name_out_of_its_scope_answers_by_join() {
    let a_row = || vec![cell("this", a())];
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ ?this <{EX}p> ?o OPTIONAL {{ BIND(<{EX}z> AS ?this) }} }}"),
        &[a_row(), a_row()],
    );
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ ?this <{EX}p> ?o {{ BIND(<{EX}z> AS ?this) }} }}"),
        &[],
    );
    assert_every_lane(
        &format!("SELECT ?o WHERE {{ ?x <{EX}p> ?o BIND(<{EX}z> AS ?this) }}"),
        &[
            vec![cell("o", TermValue::Iri(format!("{EX}o1")))],
            vec![cell("o", TermValue::Iri(format!("{EX}o2")))],
            vec![cell("o", TermValue::Iri(format!("{EX}o3")))],
        ],
    );
    // Where `?this` is already in scope, a `BIND` to it is no SPARQL query at all
    // (§18.2.1), whether or not it is pre-bound.
    let in_scope = format!("SELECT ?o WHERE {{ ?this <{EX}p> ?o BIND(<{EX}z> AS ?this) }}");
    let refusal = prepared_refusal(&in_scope, &[]).expect("not a SPARQL query");
    assert!(refusal.contains("already in scope"), "{refusal}");
}

/// A prepared execution reads `QueryOptions::declared_prebound` exactly as a request
/// does: a declared name with no slot is a constant to the grouping check (unbound
/// when it runs). Undeclared, the same read is the ordinary grouping error. An
/// assignment of it where it is not in scope binds it, declared or not.
#[test]
fn a_prepared_execution_honours_its_declared_pre_bound_names() {
    let reads =
        format!("SELECT ((COUNT(*) = 2 && !BOUND(?ctx)) AS ?r) WHERE {{ $this <{EX}p> ?o }}");
    assert_eq!(prepared_refusal(&reads, &["ctx"]), None);
    let refusal = prepared_refusal(&reads, &[]).expect("an undeclared ?ctx is no key");
    assert!(refusal.contains("neither a GROUP BY key"), "{refusal}");
    // Declared, it runs, unbound.
    let dataset = dataset();
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(
            &reads,
            None,
            &["this"],
            QueryOptions::EMPTY.with_declared_prebound(&["ctx"]),
        )
        .expect("prepares");
    execution.bind(0, a()).expect("bind");
    let answer = engine
        .execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            let rows = solutions.rows();
            assert_eq!(rows.len(), 1);
            solutions.cell(&rows[0], 0)
        })
        .expect("runs");
    assert_eq!(answer, Some(boolean(true)));
    let assigns = format!("SELECT ?o WHERE {{ $this <{EX}p> ?o BIND(?o AS ?ctx) }}");
    assert_eq!(prepared_refusal(&assigns, &["ctx"]), None);
    assert_eq!(prepared_refusal(&assigns, &[]), None);
    // A declared name that is also a slot is just the slot.
    assert_eq!(
        prepared_refusal(
            &format!("SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}"),
            &["this"]
        ),
        None
    );
}
