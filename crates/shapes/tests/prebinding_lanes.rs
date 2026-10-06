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
//! variables, an `OPTIONAL` arm and an `EXISTS`/`NOT EXISTS` body reading the pre-bound
//! variable, a `BIND` or `AS` over a pre-bound name, and a `$currentShape` read.
//!
//! One rule is split by surface. The SHACL lanes refuse what SHACL 1.2 SPARQL
//! Extensions, Appendix A forbids in a query with pre-bound variables — a `MINUS`, a
//! `VALUES` over a pre-bound name, an `AS` over one — for every name they pre-bind.
//! The engine lanes refuse none of them: an assignment binds the name where SPARQL
//! scoping puts it, and `VALUES` and `MINUS` answer by join semantics, as rdflib's `initBindings` does: `VALUES $this { ex:b }` with
//! `$this` bound to `ex:a` answers no row. A row whose SHACL answer differs carries it.
//!
//! A refusal is compared by its kind, not merely by having happened: a reassigned
//! pre-bound name and a variable no lane binds read in an aggregate are different
//! refusals, and any other error is a failure of the table.
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
    Refused(Refusal),
}

/// Why a lane refused a query.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refusal {
    /// The query assigns a pre-bound name (`BIND(… AS ?p)` or `(… AS ?p)`).
    Reassigns,
    /// An aggregate query reads a variable the lane does not bind (§11.4).
    NotAKey,
    /// A construct Appendix A forbids in a query with pre-bound variables (`VALUES` or
    /// `MINUS`), refused by the SHACL lanes.
    Restricted,
    /// Any other error, verbatim: never what a row expects.
    Other(String),
}

/// The kind of refusal `message` states.
fn refusal(message: &str) -> Refusal {
    if message.contains("neither a GROUP BY key") {
        Refusal::NotAKey
    } else if message.contains("which is pre-bound")
        || message.contains("assigning a potentially pre-bound variable")
    {
        Refusal::Reassigns
    } else if message.contains("is not allowed in a query with pre-bound variables")
        || message.contains("a VALUES clause that mentions the potentially pre-bound variable")
    {
        Refusal::Restricted
    } else {
        Refusal::Other(message.to_owned())
    }
}

fn refused(message: &impl ToString) -> Answer {
    Answer::Refused(refusal(&message.to_string()))
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
    /// What the SHACL lanes (`sh:sparql`, a node expression, `sh:expression`) answer
    /// where it differs from the engine lanes': SHACL refuses what Appendix A forbids
    /// (`VALUES` or `MINUS` over a pre-bound name), which the engine lanes answer by
    /// join semantics.
    shacl: Option<fn() -> Answer>,
    /// What `sh:sparql` alone answers, where it differs from the other SHACL lanes:
    /// the constraint lane refuses EVERY `VALUES`, as the W3C SHACL case
    /// `unsupported-sparql-002` (`VALUES ?any { true }`) requires, where a node
    /// expression refuses only one that mentions a pre-bound name (Appendix A).
    sh_sparql: Option<fn() -> Answer>,
    /// Lanes whose context binds fewer names than the query reads.
    unprovided: &'static [Lane],
}

impl Lane {
    /// Whether this lane is a SHACL surface, held to Appendix A.
    const fn is_shacl(self) -> bool {
        matches!(self, Self::ShSparql | Self::NodeExpr | Self::ShExpression)
    }
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
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "top-level group by another key, projecting the pre-bound variable",
        query: "SELECT (STR($this) AS ?value) WHERE { $this <http://example.org/p> ?o } GROUP BY ?o",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["\"http://example.org/a\"".to_owned()]),
        shacl: None,
        sh_sparql: None,
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
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "sub-SELECT group by another key, projecting the pre-bound variable",
        query: "SELECT (STR($this) AS ?value) WHERE { { SELECT $this ?o \
                WHERE { $this <http://example.org/p> ?o } GROUP BY ?o } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["\"http://example.org/a\"".to_owned()]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "sub-SELECT group filtered on the pre-bound variable",
        query: "SELECT ?value WHERE { { SELECT $this (COUNT(*) AS ?value) \
                WHERE { $this <http://example.org/p> ?o } } FILTER(BOUND($this)) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(2)]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "implicit group over empty input",
        query: "SELECT ((sameTerm($this, <http://example.org/a>) && COUNT(?z) = 0) AS ?value) \
                WHERE { $this <http://example.org/none> ?z }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([true_literal()]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "HAVING on the pre-bound variable",
        query: "SELECT (COUNT(*) AS ?value) WHERE { $this <http://example.org/p> ?o } \
                HAVING (sameTerm($this, <http://example.org/a>))",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(2)]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "ORDER BY the pre-bound variable above a group",
        query: "SELECT (COUNT(*) AS ?value) WHERE { $this <http://example.org/p> ?o } \
                GROUP BY ?o ORDER BY $this",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([integer(1)]),
        shacl: None,
        sh_sparql: None,
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
        shacl: None,
        sh_sparql: None,
        unprovided: &[Lane::ShSparql, Lane::ShExpression],
    },
    Row {
        name: "BIND over the pre-bound variable",
        query: "SELECT ?value WHERE { BIND(<http://example.org/b> AS $this) \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        // The engine lanes evaluate it: the name is not in scope where it is assigned
        // (§18.2.1), so the assignment binds it and joins with the bound value. SHACL
        // forbids an `AS` over a pre-bound name (Appendix A).
        expected: || values(Vec::new()),
        shacl: Some(|| Answer::Refused(Refusal::Reassigns)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "BIND over a fresh variable (the valid neighbour)",
        query: "SELECT ?value WHERE { BIND(<http://example.org/b> AS ?fresh) \
                ?fresh <http://example.org/p> ?value FILTER(BOUND($this)) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["<http://example.org/o3>".to_owned()]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "BIND over the pre-bound variable, grouped",
        query: "SELECT (COUNT(*) AS ?value) WHERE { ?s <http://example.org/p> ?o \
                BIND(<http://example.org/b> AS $this) }",
        engine_binds: &[],
        node_scope: &[],
        // The engine lanes evaluate it: the name is not in scope where it is assigned
        // (§18.2.1), so the assignment binds it and joins with the bound value. SHACL
        // forbids an `AS` over a pre-bound name (Appendix A).
        expected: || values([integer(3)]),
        shacl: Some(|| Answer::Refused(Refusal::Reassigns)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "AS over the pre-bound variable in a sub-SELECT",
        query: "SELECT ?value WHERE { { SELECT (<http://example.org/b> AS $this) WHERE {} } \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        // The engine lanes evaluate it: the name is not in scope where it is assigned
        // (§18.2.1), so the assignment binds it and joins with the bound value. SHACL
        // forbids an `AS` over a pre-bound name (Appendix A).
        expected: || values(Vec::new()),
        shacl: Some(|| Answer::Refused(Refusal::Reassigns)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "an OPTIONAL arm reading the pre-bound variable",
        query: "SELECT ?value WHERE { <http://example.org/b> <http://example.org/p> ?x \
                OPTIONAL { $this <http://example.org/p> ?value } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || {
            values([
                "<http://example.org/o1>".to_owned(),
                "<http://example.org/o2>".to_owned(),
            ])
        },
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "an EXISTS body reading the pre-bound variable",
        query: "SELECT ?value WHERE { ?s <http://example.org/p> ?value \
                FILTER EXISTS { $this <http://example.org/p> ?value } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || {
            values([
                "<http://example.org/o1>".to_owned(),
                "<http://example.org/o2>".to_owned(),
            ])
        },
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "a NOT EXISTS body reading the pre-bound variable",
        query: "SELECT ?value WHERE { ?s <http://example.org/p> ?value \
                FILTER NOT EXISTS { $this <http://example.org/p> ?value } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["<http://example.org/o3>".to_owned()]),
        shacl: None,
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "VALUES over a fresh variable",
        query: "SELECT ?value WHERE { VALUES ?v { true } BIND(?v AS ?value) }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values([true_literal()]),
        // A node expression and `sh:expression` admit it: it mentions no pre-bound
        // name. `sh:sparql` refuses every VALUES.
        shacl: None,
        sh_sparql: Some(|| Answer::Refused(Refusal::Restricted)),
        unprovided: &[],
    },
    Row {
        name: "VALUES over the pre-bound variable, disjoint from its binding",
        query: "SELECT ?value WHERE { VALUES $this { <http://example.org/b> } \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        // The engine joins: `ex:a` and `ex:b` are incompatible, so no row, as rdflib's
        // initBindings answers.
        expected: || values(Vec::new()),
        shacl: Some(|| Answer::Refused(Refusal::Restricted)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "VALUES over the pre-bound variable, agreeing with its binding",
        query: "SELECT ?value WHERE { VALUES $this { <http://example.org/a> } \
                $this <http://example.org/p> ?value }",
        engine_binds: &[],
        node_scope: &[],
        expected: || {
            values([
                "<http://example.org/o1>".to_owned(),
                "<http://example.org/o2>".to_owned(),
            ])
        },
        shacl: Some(|| Answer::Refused(Refusal::Restricted)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "MINUS reading the pre-bound variable",
        query: "SELECT ?value WHERE { ?s <http://example.org/p> ?value \
                MINUS { $this <http://example.org/p> ?value } }",
        engine_binds: &[],
        node_scope: &[],
        expected: || values(["<http://example.org/o3>".to_owned()]),
        shacl: Some(|| Answer::Refused(Refusal::Restricted)),
        sh_sparql: None,
        unprovided: &[],
    },
    Row {
        name: "$currentShape read in an aggregate projection",
        query: "SELECT ((BOUND($currentShape) && COUNT(*) >= 0) AS ?value) WHERE {}",
        engine_binds: &[("currentShape", "<http://example.org/S>")],
        node_scope: &[],
        expected: || values([true_literal()]),
        // A node expression runs with no shape context.
        shacl: None,
        sh_sparql: None,
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
    let shapes = match parse_shapes(&turtle, None) {
        Ok(shapes) => shapes,
        Err(error) => return refused(&error),
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
    let mut execution = match engine.prepare_execution(row.query, None, &names, QueryOptions::EMPTY)
    {
        Ok(execution) => execution,
        Err(error) => return refused(&error),
    };
    for (slot, (_, value)) in bindings.into_iter().enumerate() {
        execution.bind(slot, value).expect("bind");
    }
    let answer = engine.execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
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
    });
    match answer {
        Ok(answer) => values(answer),
        Err(error) => refused(&error),
    }
}

/// A request binding `$this` and the row's engine bindings as substitutions.
fn substituted(row: &Row) -> Answer {
    let dataset = data();
    let bindings = engine_bindings(row);
    let result = match NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query: row.query,
            base_iri: None,
            substitutions: &bindings,
        },
        QueryOptions::EMPTY,
    ) {
        Ok(result) => result,
        Err(error) => return refused(&error),
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
        Err(error) => refused(&error),
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
                Answer::Refused(Refusal::NotAKey)
            } else if let (Lane::ShSparql, Some(sh_sparql)) = (lane, row.sh_sparql) {
                sh_sparql()
            } else if let (true, Some(shacl)) = (lane.is_shacl(), row.shacl) {
                shacl()
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
                    // `true`, so it is compared with what the answer implies. A wrong
                    // verdict, an error where an answer was due, and the wrong refusal
                    // are reported apart.
                    let problem = match (sh_expression(row), &want) {
                        (Ok(conforms), Answer::Values(set)) => {
                            let implied = set.len() == 1 && set.contains(&true_literal());
                            (conforms != implied).then(|| {
                                format!("wrong verdict: conforms {conforms}, the answer implies {implied}")
                            })
                        }
                        (Err(message), Answer::Values(_)) => {
                            Some(format!("an error where an answer was due: {message}"))
                        }
                        (Ok(conforms), Answer::Refused(kind)) => {
                            Some(format!("conforms {conforms}, but {kind:?} was due"))
                        }
                        (Err(message), Answer::Refused(kind)) => {
                            let got = refusal(&message);
                            (got != *kind).then(|| format!("refused {got:?}, but {kind:?} was due"))
                        }
                    };
                    if let Some(problem) = problem {
                        failures.push(format!("{} on {lane:?}: {problem}", row.name));
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

/// **`$value` in `sh:expression` is pre-bound like `$this`.** The constraint binds
/// `value` to the value node under test, so Appendix A holds for it exactly as for
/// `$this`: a `VALUES` over `?value` is refused at load, while a `VALUES` over a name the
/// constraint does not bind is not.
#[test]
fn sh_expression_holds_value_to_the_rule_this_is_held_to() {
    let shapes = |select: &str| {
        format!(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <{EX}> .\n\
             ex:S a sh:NodeShape ; sh:targetNode ex:a ;\n\
               sh:expression [ sh:select \"\"\"{select}\"\"\" ] .\n"
        )
    };
    for name in ["value", "this"] {
        let select =
            format!("SELECT ?out WHERE {{ VALUES ?{name} {{ true }} BIND(true AS ?out) }}");
        let error = parse_shapes(&shapes(&select), None)
            .err()
            .unwrap_or_else(|| panic!("VALUES over ?{name} must be refused at load"));
        assert_eq!(
            refusal(&error.to_string()),
            Refusal::Restricted,
            "?{name}: {error}"
        );
    }
    let neighbour = "SELECT ?out WHERE { VALUES ?free { true } BIND(?free AS ?out) }";
    let loaded = parse_shapes(&shapes(neighbour), None).expect("a VALUES over a free name loads");
    let report = validate_dataset(&data(), &loaded).expect("it validates");
    assert!(report.conforms, "the free VALUES answers true");
}
