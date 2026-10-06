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

/// `ex:a ex:p ex:o1, ex:o2 ; ex:q ex:x . ex:b ex:p ex:o3 ; ex:q ex:y .`
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in [
        ("a", "p", "o1"),
        ("a", "p", "o2"),
        ("b", "p", "o3"),
        ("a", "q", "x"),
        ("b", "q", "y"),
    ] {
        let s = builder.intern_iri(&format!("{EX}{s}"));
        let p = builder.intern_iri(&format!("{EX}{p}"));
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

/// An assignment of a pre-bound name where it is not in scope (a `BIND` there is
/// SPARQL, §18.2.1) joins with the bound value (§18.5) at the assignment, wherever it
/// sits: an assigned `ex:z` is incompatible with the bound `ex:a`, so the assigning
/// pattern has no row. In an `OPTIONAL` arm that leaves each left row unextended; in
/// the query's own group, a nested group, a projected sub-`SELECT` or a `SELECT`
/// expression it leaves no row. An assignment of the bound value itself is compatible
/// and keeps its rows.
#[test]
fn an_assignment_of_the_pre_bound_name_out_of_its_scope_answers_by_join() {
    let a_row = || vec![cell("this", a())];
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ ?this <{EX}p> ?o OPTIONAL {{ BIND(<{EX}z> AS ?this) }} }}"),
        &[a_row(), a_row()],
    );
    for query in [
        format!("SELECT ?this WHERE {{ ?this <{EX}p> ?o {{ BIND(<{EX}z> AS ?this) }} }}"),
        format!("SELECT ?o WHERE {{ ?x <{EX}p> ?o BIND(<{EX}z> AS ?this) }}"),
        format!(
            "SELECT ?this ?o WHERE {{ {{ SELECT ?this ?o WHERE {{ ?x <{EX}p> ?o \
             BIND(<{EX}b> AS ?this) }} }} }}"
        ),
        format!("SELECT ?this WHERE {{ {{ SELECT ?this WHERE {{ BIND(<{EX}b> AS ?this) }} }} }}"),
        format!("SELECT (<{EX}z> AS ?this) WHERE {{ }}"),
    ] {
        assert_every_lane(&query, &[]);
    }
    assert_every_lane(
        &format!("SELECT ?o WHERE {{ ?x <{EX}p> ?o BIND(<{EX}a> AS ?this) }}"),
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

/// The answer to an assignment does not depend on an unrelated pattern beside it:
/// each assigning shape answers no row alone, beside an unrelated triple pattern
/// before it, and beside one after it, on every engine lane and the trait door.
#[test]
fn an_assignment_answers_alike_beside_an_unrelated_sibling() {
    let sibling = format!("?s <{EX}p> ?q");
    for assigning in [
        format!("{{ SELECT ?this ?o WHERE {{ ?x <{EX}p> ?o BIND(<{EX}b> AS ?this) }} }}"),
        format!("{{ ?x <{EX}p> ?o BIND(<{EX}z> AS ?this) }}"),
        format!("{{ SELECT ?this WHERE {{ BIND(<{EX}b> AS ?this) }} }}"),
    ] {
        for query in [
            format!("SELECT ?this WHERE {{ {assigning} }}"),
            format!("SELECT ?this WHERE {{ {sibling} {assigning} }}"),
            format!("SELECT ?this WHERE {{ {assigning} {sibling} }}"),
        ] {
            assert_every_lane(&query, &[]);
            assert_eq!(trait_query(&query), Vec::<Row>::new(), "{query}");
        }
    }
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

/// The three `ex:p` rows as `(?x, ?o)`.
fn every_p_row() -> Vec<Row> {
    [("a", "o1"), ("a", "o2"), ("b", "o3")]
        .into_iter()
        .map(|(x, o)| {
            vec![
                cell("x", TermValue::Iri(format!("{EX}{x}"))),
                cell("o", TermValue::Iri(format!("{EX}{o}"))),
            ]
        })
        .collect()
}

/// The request-substitution lanes answer `query` with exactly `expected`. For a
/// query that never mentions `?this`, which a prepared execution refuses as
/// declaring a parameter nothing reads.
fn assert_every_substitution_lane(query: &str, expected: &[Row]) {
    let expected = sorted(expected.to_vec());
    for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
        assert_eq!(substituted(query, lane), expected, "{lane:?}: {query}");
    }
}

/// The bound value is one value for the whole evaluation, so `MINUS` sees it on both
/// operands: each side's rows carry `?this = ex:a`, whether the side assigns it, reads
/// it, or neither. A right row compatible with the bound value therefore shares
/// `?this` with every left row and subtracts it; a right side with no row, or one
/// whose `?this` is incompatible, subtracts nothing. An assignment of the bound value
/// on the left is a row carrying `?this` like any other.
#[test]
fn a_minus_subtracts_with_the_bound_value_on_both_sides() {
    let p = format!("<{EX}p>");
    let q = format!("<{EX}q>");
    for (query, subtracts) in [
        // Assigned on the left, a VALUES of the bound value on the right.
        (
            format!(
                "SELECT ?x ?o WHERE {{ {{ ?x {p} ?o BIND(<{EX}a> AS ?this) }} \
                 MINUS {{ VALUES ?this {{ <{EX}a> }} }} }}"
            ),
            true,
        ),
        (
            format!(
                "SELECT ?x ?o WHERE {{ {{ ?x {p} ?o BIND(<{EX}a> AS ?this) }} \
                 MINUS {{ VALUES ?this {{ <{EX}b> }} }} }}"
            ),
            false,
        ),
        // Assigned on the right.
        (
            format!("SELECT ?x ?o WHERE {{ {{ ?x {p} ?o }} MINUS {{ BIND(<{EX}a> AS ?this) }} }}"),
            true,
        ),
        (
            format!("SELECT ?x ?o WHERE {{ {{ ?x {p} ?o }} MINUS {{ BIND(<{EX}b> AS ?this) }} }}"),
            false,
        ),
        // Read on the right only, and read on both sides: the same answer.
        (
            format!(
                "SELECT ?x ?o WHERE {{ ?x {p} ?o . <{EX}a> {q} ?any MINUS {{ ?this {q} ?w }} }}"
            ),
            true,
        ),
        (
            format!("SELECT ?x ?o WHERE {{ ?x {p} ?o . ?this {q} ?any MINUS {{ ?this {q} ?w }} }}"),
            true,
        ),
        // A VALUES of the bound value, and of another value, with no read on the left.
        (
            format!("SELECT ?x ?o WHERE {{ ?x {p} ?o MINUS {{ VALUES ?this {{ <{EX}a> }} }} }}"),
            true,
        ),
        (
            format!("SELECT ?x ?o WHERE {{ ?x {p} ?o MINUS {{ VALUES ?this {{ <{EX}b> }} }} }}"),
            false,
        ),
    ] {
        let expected = if subtracts { Vec::new() } else { every_p_row() };
        assert_every_lane(&query, &expected);
    }
    // A right side that never mentions `?this` still carries it: any right row
    // subtracts, and a right side with no row subtracts nothing.
    assert_every_substitution_lane(
        &format!("SELECT ?x ?o WHERE {{ ?x {p} ?o MINUS {{ ?s {q} ?w }} }}"),
        &[],
    );
    assert_every_substitution_lane(
        &format!("SELECT ?x ?o WHERE {{ ?x {p} ?o MINUS {{ <{EX}z> {q} ?w }} }}"),
        &every_p_row(),
    );
    // Inside EXISTS, the same rule.
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ FILTER EXISTS {{ ?x {p} ?o MINUS {{ ?s {q} ?w }} }} }}"),
        &[],
    );
    assert_every_lane(
        &format!(
            "SELECT ?this WHERE {{ FILTER EXISTS {{ ?x {p} ?o MINUS {{ <{EX}z> {q} ?w }} }} }}"
        ),
        &[vec![cell("this", a())]],
    );
}

/// A sub-`SELECT` that assigns `?this` without projecting it binds a variable of its
/// own inside `EXISTS` too: `ex:z` there is not the bound `ex:a`, and the sub-`SELECT`
/// has its row. Projected, the assigned `ex:z` joins with the bound `ex:a` and has none.
#[test]
fn an_unprojected_sub_select_inside_exists_binds_its_own_variable() {
    let body = format!("WHERE {{ BIND(<{EX}z> AS ?this) <{EX}a> <{EX}q> ?x }}");
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ FILTER EXISTS {{ {{ SELECT ?x {body} }} }} }}"),
        &[vec![cell("this", a())]],
    );
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ FILTER NOT EXISTS {{ {{ SELECT ?x {body} }} }} }}"),
        &[],
    );
    assert_every_lane(
        &format!("SELECT ?this WHERE {{ FILTER EXISTS {{ {{ SELECT ?this ?x {body} }} }} }}"),
        &[],
    );
}

/// A query whose `WHERE` is a lone sub-`SELECT` that does not project `?this` still
/// answers with the bound column, exactly as it does beside an unrelated pattern.
#[test]
fn a_where_of_a_lone_sub_select_carries_the_bound_column() {
    let sub = format!("{{ SELECT ?x WHERE {{ <{EX}b> <{EX}p> ?x }} }}");
    let row = || {
        vec![
            cell("this", a()),
            cell("x", TermValue::Iri(format!("{EX}o3"))),
        ]
    };
    assert_every_lane(&format!("SELECT ?this ?x WHERE {{ {sub} }}"), &[row()]);
    assert_every_lane(
        &format!("SELECT ?this ?x WHERE {{ ?s <{EX}q> ?q {sub} }}"),
        &[row(), row()],
    );
    assert_every_lane(
        &format!("SELECT ?this ?x WHERE {{ {sub} ?s <{EX}q> ?q }}"),
        &[row(), row()],
    );
}

/// `(name, value)` cells naming `example.org` IRIs, as one row.
fn iri_row(cells: &[(&str, &str)]) -> Row {
    cells
        .iter()
        .map(|(name, local)| cell(name, TermValue::Iri(format!("{EX}{local}"))))
        .collect()
}

/// Every lane answers `group` with `expected` alone and beside the unrelated, always
/// matching `ex:b ex:p ex:o3`, before it and after it: a pattern binding none of the
/// projected names that matches exactly one row cannot change the answer.
fn assert_alike_beside_a_sibling(head: &str, group: &str, expected: &[Row]) {
    let sibling = format!("<{EX}b> <{EX}p> <{EX}o3> .");
    for query in [
        format!("{head} WHERE {{ {group} }}"),
        format!("{head} WHERE {{ {sibling} {group} }}"),
        format!("{head} WHERE {{ {group} {sibling} }}"),
    ] {
        assert_every_lane(&query, expected);
        assert_eq!(trait_query(&query), sorted(expected.to_vec()), "{query}");
    }
}

/// A `VALUES` of the pre-bound name in an `OPTIONAL` arm joins with the bound value
/// where it is made (§18.5), as an assignment there does: a row of another value is
/// incompatible with it, so the arm has no row and every left row survives unextended,
/// still carrying the bound value. A row of the bound value extends them.
#[test]
fn a_values_of_the_pre_bound_name_in_an_optional_arm_joins_with_the_bound_value() {
    let p = format!("<{EX}p>");
    let q = format!("<{EX}q>");
    let left = |s: &str, o: &str| iri_row(&[("s", s), ("o", o), ("this", "a")]);
    let every_left = || vec![left("a", "o1"), left("a", "o2"), left("b", "o3")];
    let unextended = |s: &str, o: &str| {
        let mut row = left(s, o);
        row.insert(2, ("w".to_owned(), None));
        row
    };
    assert_alike_beside_a_sibling(
        "SELECT ?s ?o ?this",
        &format!("?s {p} ?o OPTIONAL {{ VALUES ?this {{ <{EX}b> }} }}"),
        &every_left(),
    );
    assert_alike_beside_a_sibling(
        "SELECT ?s ?o ?this",
        &format!("?s {p} ?o OPTIONAL {{ VALUES ?this {{ <{EX}a> }} }}"),
        &every_left(),
    );
    assert_alike_beside_a_sibling(
        "SELECT ?s ?o ?w ?this",
        &format!("?s {p} ?o OPTIONAL {{ ?s {q} ?w VALUES ?this {{ <{EX}b> }} }}"),
        &[
            unextended("a", "o1"),
            unextended("a", "o2"),
            unextended("b", "o3"),
        ],
    );
    let extended =
        |s: &str, o: &str, w: &str| iri_row(&[("s", s), ("o", o), ("w", w), ("this", "a")]);
    for values in [
        format!("<{EX}a>"),
        format!("<{EX}a> <{EX}b>"),
        "UNDEF".to_owned(),
    ] {
        assert_alike_beside_a_sibling(
            "SELECT ?s ?o ?w ?this",
            &format!("?s {p} ?o OPTIONAL {{ ?s {q} ?w VALUES ?this {{ {values} }} }}"),
            &[
                extended("a", "o1", "x"),
                extended("a", "o2", "x"),
                extended("b", "o3", "y"),
            ],
        );
    }
}

/// A `VALUES` of the pre-bound name in a `MINUS` operand joins with the bound value
/// where it is made: a right side of another value has no row and subtracts nothing,
/// one of the bound value subtracts every left row it shares `?x` with, and one in an
/// `OPTIONAL` arm or a sub-`SELECT` of the right side answers there as it does in the
/// query's own group.
#[test]
fn a_values_of_the_pre_bound_name_in_a_minus_operand_joins_with_the_bound_value() {
    let p = format!("<{EX}p>");
    let q = format!("<{EX}q>");
    let rows = |pairs: &[(&str, &str)]| -> Vec<Row> {
        pairs
            .iter()
            .map(|(x, o)| iri_row(&[("x", x), ("o", o)]))
            .collect()
    };
    assert_alike_beside_a_sibling(
        "SELECT ?x ?o",
        &format!("?x {p} ?o MINUS {{ ?x {q} ?w VALUES ?this {{ <{EX}b> }} }}"),
        &rows(&[("a", "o1"), ("a", "o2"), ("b", "o3")]),
    );
    assert_alike_beside_a_sibling(
        "SELECT ?x ?o",
        &format!(
            "?x {p} ?o MINUS {{ VALUES (?x ?this) {{ (<{EX}a> <{EX}a>) (<{EX}b> <{EX}b>) }} }}"
        ),
        &rows(&[("b", "o3")]),
    );
    // In an `OPTIONAL` arm of the right side: the arm has no row, so every right row
    // is kept, carries the bound value, and subtracts.
    assert_alike_beside_a_sibling(
        "SELECT ?x ?o",
        &format!("?x {p} ?o MINUS {{ ?x {q} ?w OPTIONAL {{ VALUES ?this {{ <{EX}b> }} }} }}"),
        &[],
    );
    // In a sub-`SELECT` of the right side: it has no row, so nothing is subtracted.
    assert_alike_beside_a_sibling(
        "SELECT ?x ?o",
        &format!(
            "?x {p} ?o MINUS {{ {{ SELECT ?x WHERE {{ ?x {q} ?w VALUES ?this {{ <{EX}b> }} }} }} }}"
        ),
        &rows(&[("a", "o1"), ("a", "o2"), ("b", "o3")]),
    );
    assert_alike_beside_a_sibling(
        "SELECT ?x ?o",
        &format!(
            "?x {p} ?o MINUS {{ {{ SELECT ?x WHERE {{ ?x {q} ?w VALUES ?this {{ <{EX}a> }} }} }} }}"
        ),
        &[],
    );
}

/// A sub-`SELECT` that does not project the pre-bound name but holds a `VALUES` of it
/// reads the one bound value there (it assigns no copy of its own): its rows are
/// joined with the bound value before it projects, groups or counts them.
#[test]
fn a_values_of_the_pre_bound_name_in_an_unprojected_sub_select_joins_with_the_bound_value() {
    let q = format!("<{EX}q>");
    // Another value: the sub-`SELECT` has no row.
    assert_alike_beside_a_sibling(
        "SELECT ?x",
        &format!("{{ SELECT ?x WHERE {{ ?x {q} ?y VALUES ?this {{ <{EX}b> }} }} }}"),
        &[],
    );
    // The bound value: its rows, once each.
    assert_alike_beside_a_sibling(
        "SELECT ?x",
        &format!("{{ SELECT ?x WHERE {{ ?x {q} ?y VALUES ?this {{ <{EX}a> <{EX}b> }} }} }}"),
        &[iri_row(&[("x", "a")]), iri_row(&[("x", "b")])],
    );
    // Two rows, one of the bound value: the empty projection has one row, not two.
    assert_alike_beside_a_sibling(
        "SELECT ?s ?y",
        &format!("?s {q} ?y . {{ SELECT ?w WHERE {{ VALUES ?this {{ <{EX}b> <{EX}a> }} }} }}"),
        &[
            iri_row(&[("s", "a"), ("y", "x")]),
            iri_row(&[("s", "b"), ("y", "y")]),
        ],
    );
    // Counted inside the sub-`SELECT`: one row is compatible with the bound value.
    assert_alike_beside_a_sibling(
        "SELECT ?c",
        &format!(
            "{{ SELECT (COUNT(*) AS ?c) WHERE {{ VALUES ?this {{ <{EX}a> <{EX}b> UNDEF }} }} }}"
        ),
        &[vec![cell("c", integer(2))]],
    );
    // A sub-`SELECT` that assigns `?this` without projecting it has a `?this` of its
    // own (§18.2.1), its `VALUES` included: every one of its rows survives.
    assert_alike_beside_a_sibling(
        "SELECT ?w",
        &format!(
            "{{ SELECT ?w WHERE {{ {{ BIND(<{EX}z> AS ?this) }} UNION \
             {{ VALUES ?this {{ <{EX}a> <{EX}b> }} }} BIND(?this AS ?w) }} }}"
        ),
        &[
            iri_row(&[("w", "a")]),
            iri_row(&[("w", "b")]),
            iri_row(&[("w", "z")]),
        ],
    );
}
