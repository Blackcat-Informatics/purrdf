// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every lane that binds variables before evaluation answers one query the same way.
//!
//! A pre-bound variable is one value for the whole evaluation, at every depth, and five
//! lanes bind one:
//!
//! * **`sh:sparql`** — a SHACL-SPARQL constraint, with `$this` and the shape context;
//! * **a prepared execution** — `NativeSparqlEngine::prepare_execution`'s parameters;
//! * **a request's substitutions** — `query_with_options_view`;
//! * **`node-expr`** — a free SHACL node expression, with `$this` and its `--scope`;
//! * **`sh:expression`** — a node-expression constraint, with `$this` and `$value`.
//!
//! They share one rewrite (`crate::substitute::apply_shacl_probes`), one admission and
//! one grouping check, and the parser's declared pre-bound set is the set each lane's
//! evaluation binds. This table runs each query shape through every lane and requires
//! the same answer — a set of `?value` nodes, or a refusal — wherever the lane's
//! context provides the variables the query reads, and a refusal at load wherever it
//! does not. The shapes: a top-level group, a sub-`SELECT` group, an implicit group
//! over empty input, `HAVING` and `ORDER BY` on the pre-bound variable, two pre-bound
//! variables, a `BIND` or `AS` over a pre-bound name, and a `$currentShape` read.
//!
//! Fixture IRIs are `example.org`.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{SparqlRequest, SparqlResult, TermValue};
use purrdf_rdf::RdfDataset;
use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_shapes::free_expression::{FreeExpression, evaluate};
use purrdf_shapes::term::{Term, term_value_to_native};
use purrdf_shapes::text_ingest::{parse_turtle_document, parse_turtle_to_dataset};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

const DATA: &str = "@prefix ex: <http://example.org/> .\n\
                    ex:a ex:p ex:o1, ex:o2 .\n\
                    ex:b ex:p ex:o3 .\n";

/// One lane's answer: the `?value` nodes, as N-Triples terms, or a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    Values(BTreeSet<String>),
    Refused,
}

/// The lanes, in the order the table reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    ShSparql,
    Prepared,
    Substituted,
    NodeExpr,
    ShExpression,
}

const LANES: [Lane; 5] = [
    Lane::ShSparql,
    Lane::Prepared,
    Lane::Substituted,
    Lane::NodeExpr,
    Lane::ShExpression,
];

fn data() -> Arc<RdfDataset> {
    parse_turtle_to_dataset(DATA, None).expect("the fixture parses")
}

fn node(value: &TermValue) -> String {
    term_value_to_native(value).to_string()
}

fn values(nodes: impl IntoIterator<Item = String>) -> Answer {
    Answer::Values(nodes.into_iter().collect())
}

/// A row of the table: the query, the extra pre-bindings each lane's context
/// provides beyond `$this`, the answer where the context provides every variable the
/// query reads, and the lanes whose context does not (refused at load there).
struct Row {
    name: &'static str,
    query: &'static str,
    /// Extra bindings a prepared execution and a request's substitutions supply.
    engine_binds: &'static [(&'static str, &'static str)],
    /// Extra scope bindings a free node expression's `--scope` supplies.
    node_scope: &'static [(&'static str, &'static str)],
    expected: fn() -> Answer,
    /// Lanes whose context binds fewer names than the query reads.
    unprovided: &'static [Lane],
}

fn true_literal() -> String {
    "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()
}

fn integer(value: u32) -> String {
    format!("\"{value}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
}

const ROWS: &[Row] = &[
    Row {
        name: "top-level implicit group",
        query: "SELECT ((sameTerm($this, <http://example.org/a>) && COUNT(*) = 2) AS ?value) \
                WHERE { $this <http://example.org/p> ?o }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([true_literal()]),
        unprovided: &[],
    },
    Row {
        name: "top-level group by another key, projecting the pre-bound variable",
        query: "SELECT (STR($this) AS ?value) WHERE { $this <http://example.org/p> ?o } GROUP BY ?o",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["\"http://example.org/a\"".to_owned()]),
        unprovided: &[],
    },
    Row {
        name: "sub-SELECT implicit group, read above it",
        query: "SELECT ?value WHERE { { SELECT $this (COUNT(*) AS ?c) \
                WHERE { $this <http://example.org/p> ?o } } \
                BIND((sameTerm($this, <http://example.org/a>) && ?c = 2) AS ?value) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([true_literal()]),
        unprovided: &[],
    },
    Row {
        name: "sub-SELECT group by another key, projecting the pre-bound variable",
        query: "SELECT (STR($this) AS ?value) WHERE { { SELECT $this ?o \
                WHERE { $this <http://example.org/p> ?o } GROUP BY ?o } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["\"http://example.org/a\"".to_owned()]),
        unprovided: &[],
    },
    Row {
        name: "sub-SELECT group filtered on the pre-bound variable",
        query: "SELECT ?value WHERE { { SELECT $this (COUNT(*) AS ?value) \
                WHERE { $this <http://example.org/p> ?o } } FILTER(BOUND($this)) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(2)]),
        unprovided: &[],
    },
    Row {
        name: "implicit group over empty input",
        query: "SELECT ((sameTerm($this, <http://example.org/a>) && COUNT(?z) = 0) AS ?value) \
                WHERE { $this <http://example.org/none> ?z }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([true_literal()]),
        unprovided: &[],
    },
    Row {
        name: "HAVING on the pre-bound variable",
        query: "SELECT (COUNT(*) AS ?value) WHERE { $this <http://example.org/p> ?o } \
                HAVING (sameTerm($this, <http://example.org/a>))",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(2)]),
        unprovided: &[],
    },
    Row {
        name: "ORDER BY the pre-bound variable above a group",
        query: "SELECT (COUNT(*) AS ?value) WHERE { $this <http://example.org/p> ?o } \
                GROUP BY ?o ORDER BY $this",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(1)]),
        unprovided: &[],
    },
    Row {
        name: "two pre-bound variables",
        query: "SELECT ((sameTerm($this, <http://example.org/a>) && $k = 5 && COUNT(*) = 2) \
                AS ?value) WHERE { $this <http://example.org/p> ?o }",
        engine_binds: &[("k", "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>")],
        node_scope: &[("k", "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>")],
        expected: || values([true_literal()]),
        // A SHACL constraint binds no `$k`.
        unprovided: &[Lane::ShSparql, Lane::ShExpression],
    },
    Row {
        name: "BIND over the pre-bound variable",
        query: "SELECT ?value WHERE { BIND(<http://example.org/b> AS $this) \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        expected: || Answer::Refused,
        unprovided: &[],
    },
    Row {
        name: "BIND over a fresh variable (the valid neighbour)",
        query: "SELECT ?value WHERE { BIND(<http://example.org/b> AS ?fresh) \
                ?fresh <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["<http://example.org/o3>".to_owned()]),
        unprovided: &[],
    },
    Row {
        name: "BIND over the pre-bound variable, grouped",
        query: "SELECT (COUNT(*) AS ?value) WHERE { ?s <http://example.org/p> ?o \
                BIND(<http://example.org/b> AS $this) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || Answer::Refused,
        unprovided: &[],
    },
    Row {
        name: "AS over the pre-bound variable in a sub-SELECT",
        query: "SELECT ?value WHERE { { SELECT (<http://example.org/b> AS $this) WHERE {} } \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        expected: || Answer::Refused,
        unprovided: &[],
    },
    Row {
        name: "$currentShape read in an aggregate projection",
        query: "SELECT ((BOUND($currentShape) && COUNT(*) >= 0) AS ?value) WHERE {}",
        engine_binds: &[("currentShape", "<http://example.org/S>")],
        node_scope: &[],
        expected: || values([true_literal()]),
        // A node expression runs with no shape context.
        unprovided: &[Lane::NodeExpr, Lane::ShExpression],
    },
];

fn parse_value(text: &str) -> TermValue {
    term_value_of(&purrdf_shapes::free_expression::parse_term(text).expect("a term"))
}

fn term_value_of(term: &Term) -> TermValue {
    term.to_term_value()
}

/// `sh:sparql`: a node shape targeting `ex:a` whose constraint is `query`; every result's
/// value.
fn sh_sparql(query: &str) -> Answer {
    let turtle = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <{EX}> .\n\
         ex:S a sh:NodeShape ; sh:targetNode ex:a ;\n\
           sh:sparql [ a sh:SPARQLConstraint ; sh:select \"\"\"{query}\"\"\" ] .\n"
    );
    let Ok(shapes) = parse_shapes(&turtle, None) else {
        return Answer::Refused;
    };
    let report = validate_dataset(&data(), &shapes).expect("a loaded shapes graph validates");
    values(
        report
            .results
            .iter()
            .filter_map(|result| result.value.as_ref().map(ToString::to_string)),
    )
}

fn engine_bindings(row: &Row) -> Vec<(String, TermValue)> {
    std::iter::once(("this".to_owned(), TermValue::Iri(format!("{EX}a"))))
        .chain(
            row.engine_binds
                .iter()
                .map(|(name, value)| ((*name).to_owned(), parse_value(value))),
        )
        .collect()
}

/// A prepared execution binding `$this` and the row's engine bindings.
fn prepared(row: &Row) -> Answer {
    let dataset = data();
    let engine = NativeSparqlEngine::new();
    let bindings = engine_bindings(row);
    let names: Vec<&str> = bindings.iter().map(|(name, _)| name.as_str()).collect();
    let Ok(mut execution) = engine.prepare_execution(row.query, None, &names, QueryOptions::EMPTY)
    else {
        return Answer::Refused;
    };
    for (slot, (_, value)) in bindings.into_iter().enumerate() {
        execution.bind(slot, value).expect("bind");
    }
    let Ok(answer) = engine.execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
        let InternedOutcome::Solutions(solutions) = outcome else {
            panic!("{}: expected solutions", row.name);
        };
        let column = solutions
            .column("value")
            .expect("the query projects ?value");
        solutions
            .rows()
            .iter()
            .filter_map(|solution| solutions.cell(solution, column))
            .map(|value| node(&value))
            .collect::<Vec<_>>()
    }) else {
        return Answer::Refused;
    };
    values(answer)
}

/// A request binding `$this` and the row's engine bindings as substitutions.
fn substituted(row: &Row) -> Answer {
    let dataset = data();
    let bindings = engine_bindings(row);
    let Ok(result) = NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query: row.query,
            base_iri: None,
            substitutions: &bindings,
        },
        QueryOptions::EMPTY,
    ) else {
        return Answer::Refused;
    };
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("{}: expected solutions", row.name);
    };
    let column = variables
        .iter()
        .position(|variable| variable == "value")
        .expect("the query projects ?value");
    values(
        rows.iter()
            .filter_map(|solution| solution[column].as_ref())
            .map(node),
    )
}

/// A free node expression `ex:E sh:select query`, focus `ex:a`, with the row's scope.
fn node_expr(row: &Row) -> Answer {
    let shapes = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <{EX}> .\n\
         ex:E sh:select \"\"\"{}\"\"\" .\n",
        row.query
    );
    let document = parse_turtle_document(&shapes, None).expect("the shapes document parses");
    let root = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(format!(
        "{EX}E"
    )));
    let focus = term_value_to_native(&TermValue::Iri(format!("{EX}a")));
    let scope: Vec<(String, Term)> = row
        .node_scope
        .iter()
        .map(|(name, value)| {
            (
                (*name).to_owned(),
                term_value_to_native(&parse_value(value)),
            )
        })
        .collect();
    let data = data();
    match evaluate(&FreeExpression {
        shapes: &document.dataset,
        prefixes: &document.prefixes,
        root: &root,
        data: data.as_ref(),
        focus: &focus,
        scope: &scope,
        imports: &purrdf_shapes::ShapesImports::new(),
    }) {
        Ok(evaluated) => values(evaluated.outputs.iter().map(ToString::to_string)),
        Err(_) => Answer::Refused,
    }
}

/// `sh:expression`: a node shape targeting `ex:a` whose expression is the query; the
/// verdict, as the answer it implies — conforming exactly when the expression's one
/// output is `true`.
///
/// `sh:expression` binds `value` to the value node under test, so the expression may
/// not assign it: the output column is renamed `?out` here, and nothing else changes.
fn sh_expression(row: &Row) -> Result<bool, String> {
    let query = row.query.replace("?value", "?out");
    let turtle = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <{EX}> .\n\
         ex:S a sh:NodeShape ; sh:targetNode ex:a ;\n\
           sh:expression [ sh:select \"\"\"{query}\"\"\" ] .\n"
    );
    let shapes = parse_shapes(&turtle, None).map_err(|error| error.to_string())?;
    let report = validate_dataset(&data(), &shapes).map_err(|error| error.to_string())?;
    Ok(report.conforms)
}

#[test]
fn every_pre_binding_lane_answers_every_query_shape_alike() {
    let mut failures = Vec::new();
    for row in ROWS {
        let expected = (row.expected)();
        for lane in LANES {
            let want = if row.unprovided.contains(&lane) {
                Answer::Refused
            } else {
                expected.clone()
            };
            let got = match lane {
                Lane::ShSparql => sh_sparql(row.query),
                Lane::Prepared => prepared(row),
                Lane::Substituted => substituted(row),
                Lane::NodeExpr => node_expr(row),
                Lane::ShExpression => {
                    // The verdict carries one bit: conforming means the one output was
                    // `true`, so it is compared with what the answer implies.
                    let verdict = sh_expression(row);
                    let implied = match &want {
                        Answer::Refused => None,
                        Answer::Values(set) => {
                            Some(set.len() == 1 && set.contains(&true_literal()))
                        }
                    };
                    if verdict.as_ref().ok().copied() != implied {
                        failures.push(format!(
                            "{} on {lane:?}: verdict {verdict:?}, the answer implies {implied:?}",
                            row.name
                        ));
                    }
                    continue;
                }
            };
            if got != want {
                failures.push(format!(
                    "{} on {lane:?}: got {got:?}, want {want:?}",
                    row.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_prepared_execution_answers_alike_across_runs_and_its_memo() {
    // The memo is built on the second run of one value shape and replayed after; every
    // run must answer as the first did, for each value in turn.
    let dataset = data();
    let engine = NativeSparqlEngine::new();
    let query =
        "SELECT (STR($this) AS ?value) (COUNT(*) AS ?n) WHERE { $this <http://example.org/p> ?o }";
    let mut execution = engine
        .prepare_execution(query, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    for (subject, count) in [("a", 2), ("b", 1), ("a", 2), ("b", 1), ("a", 2)] {
        execution
            .bind(0, TermValue::Iri(format!("{EX}{subject}")))
            .expect("bind");
        let row = engine
            .execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                let rows = solutions.rows();
                assert_eq!(rows.len(), 1);
                (
                    solutions.cell(&rows[0], 0).map(|value| node(&value)),
                    solutions.cell(&rows[0], 1).map(|value| node(&value)),
                )
            })
            .expect("execute");
        assert_eq!(
            row,
            (Some(format!("\"{EX}{subject}\"")), Some(integer(count))),
            "{subject}"
        );
    }
}
