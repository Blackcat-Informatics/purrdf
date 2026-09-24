// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A blank node label is ONE non-distinguished variable across its basic graph
//! pattern — including the pieces of it that are not triples.
//!
//! A triples block holding a property-function call or a complex property path is
//! evaluated as several pieces joined together. Each piece used to treat its blank
//! nodes as its own and drop their columns, so a label written in two pieces was two
//! unrelated existentials and the join between them became a cross product. Every
//! case here is therefore run twice — once with the blank, once with a variable in
//! the same places (the CONTROL) — and the two answers must agree. The fixture is
//! built so that the cross product and the join differ: the control's answer is
//! never what the cross product would produce.
//!
//! The rule's other half is SPARQL's: a label may not be used in two DIFFERENT basic
//! graph patterns. That is a syntax error, and each refusal is paired with the
//! neighbouring query that keeps the label inside one pattern and must still run.

use std::sync::{Arc, Mutex};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, MemoryRelation, NativeSparqlEngine, PfArgs, PfArity,
    PfCursor, PropertyFunction, PropertyFunctionRegistry, QueryOptions, Volatility,
};

const EX: &str = "http://example.org/";
/// The relation `( ?subject ) <TAG> ( ?label ?target )`.
const TAG: &str = "http://example.org/pf/tag";

fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("{EX}{local}"))
}

fn text(value: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
        language: None,
        direction: None,
    }
}

/// `ex:s1 ex:p ex:r1`, `ex:s2 ex:p ex:r2`, `ex:r1 ex:q "o"`, `ex:r2 ex:w ex:r1`.
///
/// Only `ex:r1` has an `ex:q`, and only `ex:s1` points at it — so a join through the
/// shared node answers `ex:s1` alone, while a cross product answers both subjects.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{EX}p"));
    let q = b.intern_iri(&format!("{EX}q"));
    let w = b.intern_iri(&format!("{EX}w"));
    let s1 = b.intern_iri(&format!("{EX}s1"));
    let s2 = b.intern_iri(&format!("{EX}s2"));
    let r1 = b.intern_iri(&format!("{EX}r1"));
    let r2 = b.intern_iri(&format!("{EX}r2"));
    let o = b.intern_literal(RdfLiteral::simple("o".to_owned()));
    b.push_quad(s1, p, r1, None);
    b.push_quad(s2, p, r2, None);
    b.push_quad(r1, q, o, None);
    b.push_quad(r2, w, r1, None);
    b.freeze().expect("freeze")
}

/// `( ex:s1 ) TAG ( "x" ex:r1 )` and `( ex:s2 ) TAG ( "x" ex:r9 )`: the relation
/// agrees with the data for `ex:s1` and disagrees for `ex:s2`.
fn tag_rows() -> MemoryRelation {
    MemoryRelation::new(
        1,
        2,
        vec![
            vec![iri("s1"), text("x"), iri("r1")],
            vec![iri("s2"), text("x"), iri("r9")],
        ],
    )
    .expect("uniform rows")
}

/// A relation that answers as [`tag_rows`] and records, per invocation, which of
/// its positions the engine reported unobserved.
#[derive(Debug)]
struct Spy {
    inner: MemoryRelation,
    unobserved: Mutex<Vec<Vec<bool>>>,
}

impl Spy {
    fn new() -> Self {
        Self {
            inner: tag_rows(),
            unobserved: Mutex::new(Vec::new()),
        }
    }

    fn reports(&self) -> Vec<Vec<bool>> {
        self.unobserved.lock().expect("unpoisoned").clone()
    }
}

impl PropertyFunction for Spy {
    fn volatility(&self) -> Volatility {
        self.inner.volatility()
    }

    fn arity(&self) -> PfArity {
        self.inner.arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        self.inner.modes()
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.inner.rows_per_invocation(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.unobserved.lock().expect("unpoisoned").push(
            (0..args.flattened().count())
                .map(|pos| args.is_unobserved(pos))
                .collect(),
        );
        self.inner.open(args, ceiling)
    }
}

fn env_with(relation: Arc<dyn PropertyFunction>) -> ExtensionEnv {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(TAG, relation);
    ExtensionEnv::over_relations(registry).expect("the fixture declarations read cleanly")
}

fn env() -> ExtensionEnv {
    env_with(Arc::new(tag_rows()))
}

/// Run `query`, answering `(variables, sorted rows)` or the error text.
fn run_in(env: &ExtensionEnv, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                env,
                ..QueryOptions::EMPTY
            },
        )
        .map_err(|e| e.to_string())?;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            let mut rows: Vec<Vec<String>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| match cell {
                            None => "UNBOUND".to_owned(),
                            Some(TermValue::Iri(i)) => format!("<{i}>"),
                            Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
                            Some(other) => format!("{other:?}"),
                        })
                        .collect()
                })
                .collect();
            rows.sort();
            Ok((variables, rows))
        }
        SparqlResult::Boolean(b) => Ok((vec![], vec![vec![b.to_string()]])),
        SparqlResult::Graph(graph) => Ok((vec![], vec![vec![graph.quad_count().to_string()]])),
    }
}

fn run(query: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    run_in(&env(), query)
}

fn rows(query: &str) -> Vec<Vec<String>> {
    run(query)
        .unwrap_or_else(|e| panic!("{query} must evaluate: {e}"))
        .1
}

/// `query` with `_:r` in the places `{r}` marks, and the same text with `?r` there.
fn blank_and_control(template: &str) -> (String, String) {
    (
        template.replace("{r}", "_:r"),
        template.replace("{r}", "?r"),
    )
}

/// Assert that the blank form and the variable form answer the same, and that the
/// answer is `expected` — which, by construction of the fixture, is not what two
/// unrelated existentials would answer.
fn assert_joined(template: &str, expected: &[&[&str]]) {
    let (blank, control) = blank_and_control(template);
    let expected: Vec<Vec<String>> = expected
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect();
    assert_eq!(rows(&control), expected, "the control: {control}");
    assert_eq!(
        rows(&blank),
        expected,
        "the blank must join exactly as the variable does: {blank}"
    );
}

const S1: &str = "<http://example.org/s1>";
const S2: &str = "<http://example.org/s2>";

// ── (a)/(b): a blank shared by a call and a triple of one group ─────────────────

#[test]
fn a_blank_shared_by_a_call_and_a_triple_joins_like_the_variable_it_is() {
    assert_joined(
        &format!("SELECT ?s WHERE {{ ?s <{EX}p> {{r}} . ( ?s ) <{TAG}> ( \"x\" {{r}} ) }}"),
        &[&[S1]],
    );
    // The call written first: the textual order is not what makes it join.
    assert_joined(
        &format!("SELECT ?s WHERE {{ ( ?s ) <{TAG}> ( \"x\" {{r}} ) . ?s <{EX}p> {{r}} }}"),
        &[&[S1]],
    );
}

#[test]
fn a_blank_shared_across_the_call_between_two_triples_joins() {
    // The call splits the block into two runs of triples; the label spans them.
    assert_joined(
        &format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> {{r}} . ( ?s ) <{TAG}> ( \"x\" ?z ) . {{r}} <{EX}q> ?o }}"
        ),
        &[&[S1]],
    );
}

// ── (c): a blank shared by two calls of one group ────────────────────────────────

#[test]
fn a_blank_shared_by_two_calls_joins() {
    // Both calls answer (ex:s1, ex:r1) and (ex:s2, ex:r9): joined through the blank
    // that is two pairs, where two unrelated existentials would give four.
    assert_joined(
        &format!(
            "SELECT ?s ?t WHERE {{ ( ?s ) <{TAG}> ( \"x\" {{r}} ) . ( ?t ) <{TAG}> ( \"x\" {{r}} ) }}"
        ),
        &[&[S1, S1], &[S2, S2]],
    );
}

// ── the same defect without a call: a property path ─────────────────────────────

#[test]
fn a_blank_shared_by_a_path_and_a_triple_joins() {
    assert_joined(
        &format!("SELECT ?s WHERE {{ ?s <{EX}p> {{r}} . {{r}} <{EX}q>+ ?o }}"),
        &[&[S1]],
    );
    // A path at both ends of the blank.
    assert_joined(
        &format!("SELECT ?s WHERE {{ ?s <{EX}p>+ {{r}} . {{r}} (<{EX}q>|<{EX}w>)+ \"o\" }}"),
        &[&[S1], &[S2]],
    );
}

#[test]
fn the_join_holds_inside_exists_and_optional_groups_too() {
    // Each group is its own basic graph pattern, and inside it the rule is the same.
    assert_joined(
        &format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> ?any FILTER EXISTS {{ ?s <{EX}p> {{r}} . {{r}} <{EX}q>+ ?o }} }}"
        ),
        &[&[S1]],
    );
    assert_joined(
        &format!(
            "SELECT ?s ?o WHERE {{ ?s <{EX}p> ?any OPTIONAL {{ ?s <{EX}p> {{r}} . {{r}} <{EX}q>+ ?o }} }}"
        ),
        &[&[S1, "o"], &[S2, "UNBOUND"]],
    );
}

// ── the shared blank stays non-distinguished ────────────────────────────────────

#[test]
fn a_shared_blank_is_never_a_column_of_the_answer() {
    let (variables, answer) = run(&format!(
        "SELECT * WHERE {{ ?s <{EX}p> _:r . ( ?s ) <{TAG}> ( \"x\" _:r ) }}"
    ))
    .expect("evaluates");
    assert_eq!(variables, vec!["s".to_owned()], "SELECT * names no blank");
    assert_eq!(answer, vec![vec![S1.to_owned()]]);

    // Two rows that differ only in the shared blank are one solution. `ex:r2 ex:w
    // ex:r1`, so the node between the two paths is either end: two matches, which
    // COUNT(*) counts, but one solution — it binds no variable at all — which
    // COUNT(DISTINCT *) counts. With a variable there, the two are two solutions.
    let count = |aggregate: &str, r: &str| {
        rows(&format!(
            "SELECT ({aggregate} AS ?n) WHERE {{ <{EX}r2> <{EX}w>* {r} . {r} <{EX}w>* <{EX}r1> }}"
        ))
    };
    assert_eq!(count("COUNT(*)", "?r"), vec![vec!["2".to_owned()]]);
    assert_eq!(count("COUNT(*)", "_:r"), vec![vec!["2".to_owned()]]);
    assert_eq!(count("COUNT(DISTINCT *)", "?r"), vec![vec!["2".to_owned()]]);
    assert_eq!(
        count("COUNT(DISTINCT *)", "_:r"),
        vec![vec!["1".to_owned()]],
        "a blank is not part of a solution, so the two matches are one solution"
    );
}

#[test]
fn describe_star_describes_the_pattern_s_variables_and_not_a_shared_blank() {
    // `?s` is ex:s1, whose symmetric description is its one triple. The node between
    // the triple and the path is ex:r1, which a variable there would describe too,
    // adding the two other triples that touch it.
    let describe = |r: &str| {
        rows(&format!(
            "DESCRIBE * WHERE {{ ?s <{EX}p> {r} . {r} <{EX}q>+ ?o }}"
        ))
    };
    assert_eq!(describe("?r"), vec![vec!["3".to_owned()]], "the control");
    assert_eq!(
        describe("_:r"),
        vec![vec!["1".to_owned()]],
        "DESCRIBE * names variables, and a blank node is not one"
    );
}

// ── what the relation is told ───────────────────────────────────────────────────

#[test]
fn a_shared_blank_argument_is_reported_observed_and_a_lone_one_unobserved() {
    // A blank written once, nowhere else: its value is read by nothing.
    let spy = Arc::new(Spy::new());
    run_in(
        &env_with(Arc::clone(&spy) as Arc<dyn PropertyFunction>),
        &format!("SELECT ?s WHERE {{ ?s <{EX}p> ?any . ( ?s ) <{TAG}> ( \"x\" _:lone ) }}"),
    )
    .expect("evaluates");
    let reports = spy.reports();
    assert!(!reports.is_empty(), "the relation was invoked");
    assert!(
        reports.iter().all(|r| r == &[false, false, true]),
        "a lone blank is unobserved: {reports:?}"
    );

    // The same position, its blank shared with a triple: the triple reads it.
    let spy = Arc::new(Spy::new());
    let (_, answer) = run_in(
        &env_with(Arc::clone(&spy) as Arc<dyn PropertyFunction>),
        &format!("SELECT ?s WHERE {{ ?s <{EX}p> _:r . ( ?s ) <{TAG}> ( \"x\" _:r ) }}"),
    )
    .expect("evaluates");
    assert_eq!(answer, vec![vec![S1.to_owned()]]);
    let reports = spy.reports();
    assert!(!reports.is_empty(), "the relation was invoked");
    assert!(
        reports
            .iter()
            .all(|r| r.iter().all(|unobserved| !unobserved)),
        "a shared blank is observed: {reports:?}"
    );
}

// ── UPDATE `WHERE` ──────────────────────────────────────────────────────────────

#[test]
fn an_update_where_joins_a_shared_blank_like_a_query() {
    let tagged = |r: &str| {
        let mut data = dataset();
        NativeSparqlEngine::new()
            .update_with_options(
                &mut data,
                SparqlRequest {
                    query: &format!(
                        "INSERT {{ ?s <{EX}tagged> true }} WHERE {{ ?s <{EX}p> {r} . {r} <{EX}q>+ ?o }}"
                    ),
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    env: &env(),
                    ..QueryOptions::EMPTY
                },
            )
            .expect("the update applies");
        let answer = NativeSparqlEngine::new()
            .query_with_options_view(
                &*data,
                SparqlRequest {
                    query: &format!("SELECT ?s WHERE {{ ?s <{EX}tagged> true }}"),
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .expect("the read evaluates");
        let SparqlResult::Solutions { rows, .. } = answer else {
            panic!("a SELECT");
        };
        rows.len()
    };
    assert_eq!(tagged("?r"), 1, "the control tags ex:s1 alone");
    assert_eq!(tagged("_:r"), 1, "and so does the blank");
}

// ── (d): a label in two basic graph patterns is refused ─────────────────────────

#[test]
fn a_blank_reused_across_basic_graph_patterns_is_refused_and_its_neighbour_runs() {
    for query in [
        format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> _:r . OPTIONAL {{ ( ?s ) <{TAG}> ( \"x\" _:r ) }} }}"
        ),
        format!("SELECT ?s WHERE {{ ?s <{EX}p> _:r . {{ ( ?s ) <{TAG}> ( \"x\" _:r ) }} }}"),
        format!("SELECT ?s WHERE {{ ?s <{EX}p> _:r . OPTIONAL {{ _:r <{EX}q> ?o }} }}"),
        format!("SELECT ?s WHERE {{ ?s <{EX}p> _:r . BIND(1 AS ?one) _:r <{EX}q> ?o }}"),
    ] {
        let error = run(&query).expect_err("one label in two basic graph patterns");
        assert!(
            error.contains("_:r is used in two different basic graph patterns"),
            "{query}: {error}"
        );
    }
    // The neighbours: the same labels kept inside one basic graph pattern.
    assert_eq!(
        rows(&format!(
            "SELECT ?s WHERE {{ OPTIONAL {{ ?s <{EX}p> _:r . ( ?s ) <{TAG}> ( \"x\" _:r ) }} }}"
        )),
        vec![vec![S1.to_owned()]]
    );
    assert_eq!(
        rows(&format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> _:r . FILTER(true) _:r <{EX}q> ?o }}"
        )),
        vec![vec![S1.to_owned()]],
        "a FILTER does not end a basic graph pattern"
    );
}
