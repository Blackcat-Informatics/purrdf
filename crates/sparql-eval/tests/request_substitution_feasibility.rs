// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A request's substitutions are admitted as bound, on every request door, exactly as
//! a prepared execution's declared parameters are.
//!
//! A [`SparqlRequest`] carries its substitutions with their values, so every variable
//! it names is bound on the run — the same promise
//! [`NativeSparqlEngine::prepare_execution`] is given by its parameter list. The
//! request doors used to admit the query text as though those variables were free, so
//! `SELECT ?q ?out WHERE { ?q <rel> ?out }` with `?q` substituted was refused against
//! a relation serving only the bound mode — "reachable only as `ff`" — for every term
//! kind, an IRI included, although the rewrite binds `?q` before the call runs.
//!
//! # The oracle
//!
//! [`Table`] serves only `bf`, and answers a bound input from the value itself:
//! `alpha` has two rows, `gamma` none, for every term kind, each row naming the kind
//! and the name it was produced for. Every invocation is recorded with its access
//! pattern and the exact term it was handed. So each run is held to three things: the
//! exact rows, a relation invoked bound with exactly the substituted value and nothing
//! else, and agreement — rows and invocations both — with the prepared-execution
//! path, which is the admission this one must match.
//!
//! The refusal is kept where it is right: a substitution the request's rewrite does
//! not carry to the call (an `OPTIONAL` arm, under the ordinary rewrite) still leaves
//! the call free, and is still refused — beside the same text under the SHACL
//! pre-binding rewrite, which does reach that arm, and is admitted. A request with no
//! substitution for the call's input is refused too, on the same engine, after the
//! substituted request was admitted: the admission is keyed by what the request
//! binds.

use std::sync::{Arc, Mutex};

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, GovernedOutcome, GovernorState, InternedGoverned,
    InternedOutcome, InternedRequest, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    Prebinding, PropertyFunction, PropertyFunctionRegistry, QueryGovernors, QueryOptions,
    ShaclPrebinding, Volatility,
};

/// The relation IRI every query calls — host configuration, never minted vocabulary.
const REL: &str = "https://example.org/rel/table";

/// The fixture data namespace.
const EX: &str = "https://example.org/d/";

/// The two names a substitution takes: `alpha` has two table rows, `gamma` none.
const NAMES: [&str; 2] = ["alpha", "gamma"];

/// How many rows the table holds for `name`.
const fn rows_for(name: &str) -> u32 {
    match name.as_bytes() {
        b"alpha" => 2,
        _ => 0,
    }
}

/// The kinds of term a substitution can be.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Iri,
    Literal,
    Blank,
    Quoted,
    QuotedWithBlank,
    QuotedWithLiteral,
}

const KINDS: [Kind; 6] = [
    Kind::Iri,
    Kind::Literal,
    Kind::Blank,
    Kind::Quoted,
    Kind::QuotedWithBlank,
    Kind::QuotedWithLiteral,
];

impl Kind {
    const fn tag(self) -> &'static str {
        match self {
            Self::Iri => "iri",
            Self::Literal => "lit",
            Self::Blank => "bn",
            Self::Quoted => "qt",
            Self::QuotedWithBlank => "qtbn",
            Self::QuotedWithLiteral => "qtlit",
        }
    }

    /// The term of this kind named `name`.
    fn term(self, name: &str) -> TermValue {
        let blank = || TermValue::Blank {
            label: format!("{}-{name}", self.tag()),
            scope: BlankScope::DEFAULT,
        };
        let quoted = |object: TermValue| TermValue::Triple {
            s: Box::new(TermValue::iri(format!("{EX}a"))),
            p: Box::new(TermValue::iri(format!("{EX}r"))),
            o: Box::new(object),
        };
        match self {
            Self::Iri => TermValue::iri(format!("{EX}{name}")),
            Self::Literal => simple_literal(name),
            Self::Blank => blank(),
            Self::Quoted => quoted(TermValue::iri(format!("{EX}{name}"))),
            Self::QuotedWithBlank => quoted(blank()),
            Self::QuotedWithLiteral => quoted(simple_literal(name)),
        }
    }

    /// The `n`-th output the table holds for this kind's `name`.
    fn output(self, name: &str, n: u32) -> TermValue {
        simple_literal(&format!("{}:{name}/{n}", self.tag()))
    }
}

fn simple_literal(text: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: text.to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
        language: None,
        direction: None,
    }
}

/// One recorded invocation: the access pattern and the term at the input position.
type Invocation = (String, Option<TermValue>);

/// `?input <table> ?output`, arity `(1, 1)`, serving `bf` — and, when it declares
/// `ff` too, answering a free input with every row it holds.
struct Table {
    modes: Vec<BindingPattern>,
    invocations: Arc<Mutex<Vec<Invocation>>>,
}

impl PropertyFunction for Table {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        2
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let input = args.get(0).cloned();
        self.invocations
            .lock()
            .expect("the fixture recorder is never poisoned")
            .push((args.mode().code(), input.clone()));
        if input.is_none() && !self.modes.contains(&BindingPattern::from_code("ff")) {
            return Err(EvalError::function(
                "the input is free; this relation serves only `bf`".to_owned(),
            ));
        }
        let mut rows: Vec<PfRow> = Vec::new();
        for kind in KINDS {
            for name in NAMES {
                let held = kind.term(name);
                if input.as_ref().is_none_or(|input| *input == held) {
                    for n in 1..=rows_for(name) {
                        rows.push(vec![held.clone(), kind.output(name, n)]);
                    }
                }
            }
        }
        Ok(Box::new(Rows(rows.into_iter())))
    }
}

struct Rows(std::vec::IntoIter<PfRow>);

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.0.next())
    }
}

/// Every substitution value, held by the dataset as the object of a triple, so each
/// blank node is one the dataset knows.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let holds = builder.intern_iri(&format!("{EX}holds"));
    let a = builder.intern_iri(&format!("{EX}a"));
    let r = builder.intern_iri(&format!("{EX}r"));
    for kind in KINDS {
        for name in NAMES {
            let label = format!("{}-{name}", kind.tag());
            let value = match kind {
                Kind::Iri => builder.intern_iri(&format!("{EX}{name}")),
                Kind::Literal => builder.intern_literal(RdfLiteral::simple(name)),
                Kind::Blank => builder.intern_blank(&label, BlankScope::DEFAULT),
                Kind::Quoted => {
                    let object = builder.intern_iri(&format!("{EX}{name}"));
                    builder.intern_triple(a, r, object)
                }
                Kind::QuotedWithBlank => {
                    let object = builder.intern_blank(&label, BlankScope::DEFAULT);
                    builder.intern_triple(a, r, object)
                }
                Kind::QuotedWithLiteral => {
                    let object = builder.intern_literal(RdfLiteral::simple(name));
                    builder.intern_triple(a, r, object)
                }
            };
            let subject = builder.intern_iri(&format!("{EX}s-{label}"));
            builder.push_quad(subject, holds, value, None);
        }
    }
    // One named graph, so a call written inside `GRAPH ?g { … }` is evaluated once. Its
    // predicate is no other pattern's, so nothing else reads it.
    let graph = builder.intern_iri(&format!("{EX}g"));
    let marker = builder.intern_iri(&format!("{EX}marker"));
    builder.push_quad(graph, marker, graph, Some(graph));
    builder.freeze().expect("the fixture dataset freezes")
}

/// The query shapes a substituted `?q` reaches the call in, under both rewrites.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// The call alone.
    Plain,
    /// The call inside `FILTER EXISTS`.
    Exists,
    /// The call inside `FILTER NOT EXISTS`.
    NotExists,
}

const SHAPES: [Shape; 3] = [Shape::Plain, Shape::Exists, Shape::NotExists];

impl Shape {
    fn query(self) -> String {
        let call = format!("?q <{REL}> ?out");
        match self {
            Self::Plain => format!("SELECT ?q ?out WHERE {{ {call} }}"),
            Self::Exists => format!("SELECT ?q WHERE {{ FILTER EXISTS {{ {call} }} }}"),
            Self::NotExists => format!("SELECT ?q WHERE {{ FILTER NOT EXISTS {{ {call} }} }}"),
        }
    }

    /// The exact rows `?q` substituted with `kind`'s `name` answers.
    fn expected(self, kind: Kind, name: &str) -> Vec<Vec<Option<TermValue>>> {
        let q = kind.term(name);
        let held = rows_for(name) > 0;
        match self {
            Self::Plain => (1..=rows_for(name))
                .map(|n| vec![Some(q.clone()), Some(kind.output(name, n))])
                .collect(),
            Self::Exists => {
                if held {
                    vec![vec![Some(q)]]
                } else {
                    Vec::new()
                }
            }
            Self::NotExists => {
                if held {
                    Vec::new()
                } else {
                    vec![vec![Some(q)]]
                }
            }
        }
    }
}

/// The request doors a substitution list reaches.
#[derive(Clone, Copy, Debug)]
enum Door {
    OptionsView,
    InternedView,
    Governed,
    GovernedInOperation,
    GovernedInternedInOperation,
    FallibleView,
}

const DOORS: [Door; 6] = [
    Door::OptionsView,
    Door::InternedView,
    Door::Governed,
    Door::GovernedInOperation,
    Door::GovernedInternedInOperation,
    Door::FallibleView,
];

/// A registry holding one bound-only [`Table`], and the recorder it writes to.
fn relations() -> (ExtensionEnv, Arc<Mutex<Vec<Invocation>>>) {
    relations_declaring(&["bf"])
}

/// A registry holding one [`Table`] declaring `modes`, and the recorder it writes to.
fn relations_declaring(modes: &[&str]) -> (ExtensionEnv, Arc<Mutex<Vec<Invocation>>>) {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(Table {
            modes: modes
                .iter()
                .map(|code| BindingPattern::from_code(code))
                .collect(),
            invocations: Arc::clone(&invocations),
        }),
    );
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declaration reads");
    (env, invocations)
}

/// Rows in a fixed order, so two bags compare as bags.
fn sorted(mut rows: Vec<Vec<Option<TermValue>>>) -> Vec<Vec<Option<TermValue>>> {
    rows.sort_by_key(|row| format!("{row:?}"));
    rows
}

fn solutions(result: SparqlResult) -> Vec<Vec<Option<TermValue>>> {
    match result {
        SparqlResult::Solutions { rows, .. } => sorted(rows),
        other => panic!("a SELECT answers with solutions, got {other:?}"),
    }
}

fn interned_rows<D: purrdf_core::DatasetView + Sync>(
    outcome: InternedOutcome<'_, '_, D>,
) -> Vec<Vec<Option<TermValue>>> {
    match outcome {
        InternedOutcome::Solutions(solutions) => sorted(
            solutions
                .rows()
                .iter()
                .map(|row| {
                    (0..solutions.variables().len())
                        .map(|column| solutions.cell(row, column))
                        .collect()
                })
                .collect(),
        ),
        InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
            panic!("a SELECT answers with solutions")
        }
    }
}

/// Run `query` through `door` with `substitutions`, under `lane`.
fn request(
    engine: &NativeSparqlEngine,
    data: &Arc<RdfDataset>,
    env: &ExtensionEnv,
    door: Door,
    lane: ShaclPrebinding,
    query: &str,
    substitutions: &[(String, TermValue)],
) -> Result<Vec<Vec<Option<TermValue>>>, String> {
    let options = QueryOptions {
        env,
        prebinding: lane,
        ..QueryOptions::EMPTY
    };
    let owned = SparqlRequest {
        query,
        base_iri: None,
        substitutions,
    };
    let borrowed: Vec<Prebinding<'_>> = substitutions
        .iter()
        .map(|(variable, value)| Prebinding {
            variable,
            value: value.clone(),
        })
        .collect();
    let interned = InternedRequest {
        query,
        base_iri: None,
        substitutions: &borrowed,
    };
    let governed = |outcome: GovernedOutcome| match outcome {
        GovernedOutcome::Complete { result, .. } => solutions(result),
        GovernedOutcome::BudgetExhausted(_) => panic!("an unbounded run completes"),
    };
    let state = || Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let answer = match door {
        Door::OptionsView => engine
            .query_with_options_view(&**data, owned, options)
            .map(solutions),
        Door::InternedView => engine.query_interned_view(&**data, interned, options, interned_rows),
        Door::Governed => engine
            .query_governed(data, owned, options, &QueryGovernors::UNBOUNDED)
            .map(governed),
        Door::GovernedInOperation => engine
            .query_governed_in_operation(&**data, owned, options, &state())
            .map(governed),
        Door::GovernedInternedInOperation => engine
            .query_governed_interned_in_operation(&**data, interned, options, &state(), |outcome| {
                interned_rows(outcome)
            })
            .map(|outcome| match outcome {
                InternedGoverned::Complete { value, .. } => value,
                InternedGoverned::BudgetExhausted(_) => panic!("an unbounded run completes"),
            }),
        Door::FallibleView => {
            return engine
                .query_fallible_view(&**data, owned, options)
                .map(|complete| solutions(complete.into_parts().0))
                .map_err(|error| format!("{error:?}"));
        }
    };
    answer.map_err(|diagnostic| diagnostic.message)
}

/// Run `query` as a prepared execution declaring `?q`, bound to `value`, under `lane`.
fn prepared(
    engine: &NativeSparqlEngine,
    data: &Arc<RdfDataset>,
    env: &ExtensionEnv,
    lane: ShaclPrebinding,
    query: &str,
    value: &TermValue,
) -> Result<Vec<Vec<Option<TermValue>>>, String> {
    let options = QueryOptions {
        env,
        prebinding: lane,
        ..QueryOptions::EMPTY
    };
    let mut execution = engine
        .prepare_execution(query, None, &["q"], options)
        .map_err(|diagnostic| diagnostic.message)?;
    execution
        .bind(0, value.clone())
        .expect("the value door binds every term kind");
    engine
        .execute(&mut execution, &**data, options, interned_rows)
        .map_err(|diagnostic| diagnostic.message)
}

/// Take the recorded invocations, sorted, leaving the recorder empty.
fn drain(invocations: &Mutex<Vec<Invocation>>) -> Vec<Invocation> {
    let mut seen = std::mem::take(
        &mut *invocations
            .lock()
            .expect("the fixture recorder is never poisoned"),
    );
    seen.sort_by_key(|invocation| format!("{invocation:?}"));
    seen
}

/// **Every term kind, every lane, every request door: the substituted request runs,
/// invokes the bound-only relation bound with exactly the substituted value, and
/// answers — rows and invocations — as the prepared execution does.**
#[test]
fn a_substituted_input_is_admitted_bound_on_every_request_door() {
    let (env, invocations) = relations();
    let data = dataset();
    for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
        for shape in SHAPES {
            let query = shape.query();
            for kind in KINDS {
                for name in NAMES {
                    let value = kind.term(name);
                    let context = format!("{lane:?} {shape:?} ?q = {}:{name}", kind.tag());

                    let engine = NativeSparqlEngine::new();
                    drain(&invocations);
                    let reference = prepared(&engine, &data, &env, lane, &query, &value)
                        .unwrap_or_else(|message| {
                            panic!("{context}: the prepared execution runs: {message}")
                        });
                    let reference_invocations = drain(&invocations);
                    assert_eq!(
                        reference,
                        sorted(shape.expected(kind, name)),
                        "{context}: the prepared execution's rows"
                    );
                    assert!(
                        !reference_invocations.is_empty(),
                        "{context}: the relation is invoked"
                    );
                    for invocation in &reference_invocations {
                        assert_eq!(
                            invocation,
                            &("bf".to_owned(), Some(value.clone())),
                            "{context}: every invocation is bound, with the substituted value"
                        );
                    }

                    for door in DOORS {
                        // A fresh engine per door, so no door answers from a plan
                        // another door admitted.
                        let engine = NativeSparqlEngine::new();
                        let substitutions = [("q".to_owned(), value.clone())];
                        let answer =
                            request(&engine, &data, &env, door, lane, &query, &substitutions)
                                .unwrap_or_else(|message| {
                                    panic!("{context} via {door:?}: the request runs: {message}")
                                });
                        assert_eq!(
                            answer, reference,
                            "{context} via {door:?}: the rows the prepared execution answers"
                        );
                        assert_eq!(
                            drain(&invocations),
                            reference_invocations,
                            "{context} via {door:?}: the relation is invoked exactly as the \
                             prepared execution invokes it — bound, with the substituted value"
                        );
                    }
                }
            }
        }
    }
}

/// **The admission is keyed by what a request binds.** On one engine, the substituted
/// request is admitted and then the same text with no substitution is refused: the
/// call's input is free there, and the plan admitted for the substituted request is
/// not the plan it gets.
#[test]
fn the_same_text_without_the_substitution_is_still_refused() {
    let (env, invocations) = relations();
    let data = dataset();
    let query = Shape::Plain.query();
    for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
        for door in DOORS {
            let engine = NativeSparqlEngine::new();
            let substitutions = [("q".to_owned(), Kind::Iri.term("alpha"))];
            let admitted = request(&engine, &data, &env, door, lane, &query, &substitutions)
                .unwrap_or_else(|message| panic!("{lane:?} via {door:?}: runs: {message}"));
            assert_eq!(
                admitted,
                sorted(Shape::Plain.expected(Kind::Iri, "alpha")),
                "{lane:?} via {door:?}"
            );
            drain(&invocations);
            let refused = request(&engine, &data, &env, door, lane, &query, &[])
                .expect_err("with nothing substituted the call's input is free");
            assert!(
                refused.contains("no feasible evaluation order")
                    && refused.contains("reachable only as `ff`"),
                "{lane:?} via {door:?}: refused for the free input, got {refused}"
            );
            assert_eq!(
                drain(&invocations),
                Vec::<Invocation>::new(),
                "{lane:?} via {door:?}: a refused request invokes nothing"
            );
        }
    }
}

/// **A substitution the rewrite does not carry to the call stays refused, and the
/// same text under the rewrite that does carry it is admitted.**
///
/// The ordinary rewrite binds `?q` at the core and does not enter an `OPTIONAL`'s
/// right arm, so the call there is invoked with `?q` free: refused, for every term
/// kind, exactly as the prepared execution refuses it. The SHACL pre-binding rewrite
/// binds `?q` in that arm too, so the same request is admitted there, answering the
/// substituted value's rows — and, for `gamma`, which the relation holds nothing for,
/// the left row alone — with the relation invoked bound, with exactly that value.
#[test]
fn a_substitution_that_does_not_reach_the_call_stays_refused() {
    let (env, invocations) = relations();
    let data = dataset();
    let query =
        format!("SELECT ?q ?out WHERE {{ BIND(1 AS ?one) OPTIONAL {{ ?q <{REL}> ?out }} }}");
    for kind in KINDS {
        for name in NAMES {
            let value = kind.term(name);
            let context = format!("?q = {}:{name}", kind.tag());
            let substitutions = [("q".to_owned(), value.clone())];

            let engine = NativeSparqlEngine::new();
            let refused = prepared(&engine, &data, &env, ShaclPrebinding::None, &query, &value)
                .expect_err("the ordinary rewrite leaves the OPTIONAL arm's input free");
            assert!(
                refused.contains("reachable only as `ff`"),
                "{context}: the prepared execution's refusal, got {refused}"
            );
            for door in DOORS {
                let engine = NativeSparqlEngine::new();
                let message = request(
                    &engine,
                    &data,
                    &env,
                    door,
                    ShaclPrebinding::None,
                    &query,
                    &substitutions,
                )
                .expect_err("the ordinary rewrite leaves the OPTIONAL arm's input free");
                assert!(
                    message.contains("no feasible evaluation order")
                        && message.contains("reachable only as `ff`"),
                    "{context} via {door:?}: refused for the free input, got {message}"
                );
            }
            assert_eq!(
                drain(&invocations),
                Vec::<Invocation>::new(),
                "{context}: a refused request invokes nothing"
            );

            let expected: Vec<Vec<Option<TermValue>>> = if rows_for(name) == 0 {
                vec![vec![Some(value.clone()), None]]
            } else {
                (1..=rows_for(name))
                    .map(|n| vec![Some(value.clone()), Some(kind.output(name, n))])
                    .collect()
            };
            for door in DOORS {
                let engine = NativeSparqlEngine::new();
                let answer = request(
                    &engine,
                    &data,
                    &env,
                    door,
                    ShaclPrebinding::Applied,
                    &query,
                    &substitutions,
                )
                .unwrap_or_else(|message| {
                    panic!("{context} via {door:?}: the SHACL rewrite reaches the arm: {message}")
                });
                assert_eq!(answer, sorted(expected.clone()), "{context} via {door:?}");
                let seen = drain(&invocations);
                assert!(!seen.is_empty(), "{context} via {door:?}: invoked");
                for invocation in seen {
                    assert_eq!(
                        invocation,
                        ("bf".to_owned(), Some(value.clone())),
                        "{context} via {door:?}: bound, with the substituted value"
                    );
                }
            }
        }
    }
}

// ── the positions the rewrite writes a substitution into ────────────────────

/// `?h <holds> ?v`: one row per substitution value of every kind, twelve in all.
fn holds() -> String {
    format!("?h <{EX}holds> ?v")
}

/// How many `?h <holds> ?v` rows [`dataset`] holds.
const HELD: usize = KINDS.len() * NAMES.len();

/// Run `query` with `?q` bound to `value` under `lane` — as a prepared execution and
/// through every request door, each on a fresh engine — and hold every run to
/// `expected` and to a relation invoked bound, with exactly `value`, every time.
fn admitted_bound_everywhere(
    env: &ExtensionEnv,
    invocations: &Mutex<Vec<Invocation>>,
    lane: ShaclPrebinding,
    query: &str,
    value: &TermValue,
    expected: &[Vec<Option<TermValue>>],
    context: &str,
) {
    let data = dataset();
    let expected = sorted(expected.to_vec());
    drain(invocations);
    let engine = NativeSparqlEngine::new();
    let reference = prepared(&engine, &data, env, lane, query, value)
        .unwrap_or_else(|message| panic!("{context}: the prepared execution runs: {message}"));
    assert_eq!(
        reference, expected,
        "{context}: the prepared execution's rows"
    );
    let reference_invocations = drain(invocations);
    assert!(
        !reference_invocations.is_empty(),
        "{context}: the relation is invoked"
    );
    for invocation in &reference_invocations {
        assert_eq!(
            invocation,
            &("bf".to_owned(), Some(value.clone())),
            "{context}: every invocation is bound, with the substituted value"
        );
    }
    for door in DOORS {
        let engine = NativeSparqlEngine::new();
        let substitutions = [("q".to_owned(), value.clone())];
        let answer = request(&engine, &data, env, door, lane, query, &substitutions)
            .unwrap_or_else(|message| {
                panic!("{context} via {door:?}: the request runs: {message}")
            });
        assert_eq!(answer, expected, "{context} via {door:?}: the rows");
        assert_eq!(
            drain(invocations),
            reference_invocations,
            "{context} via {door:?}: invoked exactly as the prepared execution invokes it"
        );
    }
}

/// Run `query` with `?q` bound to `value` under `lane` against the bound-only relation —
/// refused at prepare, on every door and as a prepared execution, invoking nothing —
/// and against the free-capable one, whose invocations must include a FREE one: the
/// observation that the refusal is right, and not merely strict.
fn refused_and_free_when_served(
    lane: ShaclPrebinding,
    query: &str,
    value: &TermValue,
    context: &str,
) {
    let data = dataset();
    let (env, invocations) = relations();
    let engine = NativeSparqlEngine::new();
    let refused = prepared(&engine, &data, &env, lane, query, value)
        .expect_err("the rewrite leaves the call's input free");
    assert!(
        refused.contains("reachable only as `ff`"),
        "{context}: the prepared execution's refusal, got {refused}"
    );
    for door in DOORS {
        let engine = NativeSparqlEngine::new();
        let substitutions = [("q".to_owned(), value.clone())];
        let message = request(&engine, &data, &env, door, lane, query, &substitutions)
            .expect_err("the rewrite leaves the call's input free");
        assert!(
            message.contains("no feasible evaluation order")
                && message.contains("reachable only as `ff`"),
            "{context} via {door:?}: refused for the free input, got {message}"
        );
    }
    assert_eq!(
        drain(&invocations),
        Vec::<Invocation>::new(),
        "{context}: a refused request invokes nothing"
    );

    let (env, invocations) = relations_declaring(&["bf", "ff"]);
    let engine = NativeSparqlEngine::new();
    let substitutions = [("q".to_owned(), value.clone())];
    request(
        &engine,
        &data,
        &env,
        Door::OptionsView,
        lane,
        query,
        &substitutions,
    )
    .unwrap_or_else(|message| panic!("{context}: the free-capable relation runs: {message}"));
    let seen = drain(&invocations);
    assert!(
        seen.contains(&("ff".to_owned(), None)),
        "{context}: the free-capable relation is invoked FREE, which is what the refusal \
         refuses — {seen:?}"
    );
}

/// **A call that is a `LATERAL`'s whole right operand is admitted bound, on every door
/// and lane, for every term kind.**
///
/// `?h <holds> ?v LATERAL { ?q <table> ?out }` is planned as the `LATERAL` over the
/// call itself — the group's opening empty pattern is the identity — and that is the
/// one right operand the rewrite writes a substitution into. It used to be refused as
/// "reachable only as `ff`" while a relation serving both modes was invoked `bf` with
/// the substituted value on every run. Each of the twelve left rows is answered with
/// the substituted value's rows, and the same query against the free-capable relation
/// is invoked bound only.
#[test]
fn a_call_a_lateral_drives_is_admitted_bound() {
    let (env, invocations) = relations();
    let query = format!(
        "SELECT ?q ?out WHERE {{ {} LATERAL {{ ?q <{REL}> ?out }} }}",
        holds()
    );
    let twice = format!(
        "SELECT ?q ?out WHERE {{ {} LATERAL {{ LATERAL {{ ?q <{REL}> ?out }} }} }}",
        holds()
    );
    for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
        for kind in KINDS {
            for name in NAMES {
                let value = kind.term(name);
                let expected: Vec<Vec<Option<TermValue>>> = (0..HELD)
                    .flat_map(|_| {
                        (1..=rows_for(name))
                            .map(|n| vec![Some(value.clone()), Some(kind.output(name, n))])
                    })
                    .collect();
                for text in [&query, &twice] {
                    let context = format!("{lane:?} {text} ?q = {}:{name}", kind.tag());
                    admitted_bound_everywhere(
                        &env,
                        &invocations,
                        lane,
                        text,
                        &value,
                        &expected,
                        &context,
                    );

                    let (free_env, free_invocations) = relations_declaring(&["bf", "ff"]);
                    let substitutions = [("q".to_owned(), value.clone())];
                    let answer = request(
                        &NativeSparqlEngine::new(),
                        &dataset(),
                        &free_env,
                        Door::OptionsView,
                        lane,
                        text,
                        &substitutions,
                    )
                    .unwrap_or_else(|message| panic!("{context}: free-capable: {message}"));
                    assert_eq!(answer, sorted(expected.clone()), "{context}: free-capable");
                    let seen = drain(&free_invocations);
                    assert!(!seen.is_empty(), "{context}: free-capable invoked");
                    for invocation in seen {
                        assert_eq!(
                            invocation,
                            ("bf".to_owned(), Some(value.clone())),
                            "{context}: the free-capable relation is invoked bound, with \
                             the substituted value — the proof the admission is right"
                        );
                    }
                }
            }
        }
    }
}

/// **A right operand the rewrite does not write into stays refused.** An `OPTIONAL`'s
/// right arm and a `MINUS`'s right arm — at the top, or inside a `LATERAL`'s right side
/// — a sub-`SELECT` inside a `LATERAL` that does not project `?q`, and one whose rows a
/// `LIMIT` cuts: restricting the call to the substituted value there is not the same as
/// joining the value on above it, so the ordinary rewrite leaves the call's input free
/// in each. The bound-only relation is refused, and the free-capable one is observed
/// invoked free.
#[test]
fn a_right_operand_the_rewrite_does_not_reach_stays_refused() {
    let held = holds();
    let call = format!("?q <{REL}> ?out");
    let other = format!("?h2 <{EX}holds> ?v2");
    let shapes = [
        format!("SELECT ?q ?out WHERE {{ {held} OPTIONAL {{ {call} }} }}"),
        format!("SELECT ?q ?out WHERE {{ {held} MINUS {{ {call} }} }}"),
        format!("SELECT ?q ?out WHERE {{ {held} LATERAL {{ OPTIONAL {{ {call} }} }} }}"),
        format!("SELECT ?q ?out WHERE {{ {held} LATERAL {{ {other} MINUS {{ {call} }} }} }}"),
        format!("SELECT ?q ?out WHERE {{ {held} LATERAL {{ SELECT ?out WHERE {{ {call} }} }} }}"),
        format!(
            "SELECT ?q ?out WHERE {{ {held} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} LIMIT 1 \
             }} }}"
        ),
    ];
    for query in &shapes {
        for kind in KINDS {
            let value = kind.term("alpha");
            let context = format!("{query} ?q = {}:alpha", kind.tag());
            refused_and_free_when_served(ShaclPrebinding::None, query, &value, &context);
        }
    }
}

/// A `LATERAL` right side the rewrite writes a substitution into: where the call sits,
/// the right side's text, and how many rows it answers per left row for a value with a
/// given number of table rows.
type LateralRightSide = (&'static str, String, fn(usize) -> usize);

/// Every [`LateralRightSide`] beyond a bare call.
fn lateral_right_sides() -> Vec<LateralRightSide> {
    let call = format!("?q <{REL}> ?out");
    let other = format!("?h2 <{EX}holds> ?v2");
    vec![
        (
            "a BIND beside the call",
            format!("BIND(1 AS ?one) {call}"),
            |rows| rows,
        ),
        (
            "a VALUES beside the call",
            format!("VALUES ?z {{ 1 }} {call}"),
            |rows| rows,
        ),
        (
            "an atom beside the call",
            format!("{other} . {call}"),
            |rows| HELD * rows,
        ),
        (
            "a FILTER after the call",
            format!("{call} FILTER(BOUND(?out))"),
            |rows| rows,
        ),
        (
            "a nested LATERAL",
            format!("{other} LATERAL {{ {call} }}"),
            |rows| HELD * rows,
        ),
        (
            "a projecting sub-SELECT",
            format!("SELECT ?q ?out WHERE {{ {call} }}"),
            |rows| rows,
        ),
        (
            "a SELECT *",
            format!("SELECT * WHERE {{ {call} }}"),
            |rows| rows,
        ),
        (
            "an ordered DISTINCT sub-SELECT",
            format!("SELECT DISTINCT ?q ?out WHERE {{ {call} }} ORDER BY ?out"),
            |rows| rows,
        ),
        (
            "a UNION",
            format!("{{ {call} }} UNION {{ {call} }}"),
            |rows| 2 * rows,
        ),
        ("a GRAPH", format!("GRAPH ?g {{ {call} }}"), |rows| rows),
    ]
}

/// **A call anywhere in a `LATERAL`'s right side where substituting is joining is
/// admitted bound, and answers the bottom-up join.**
///
/// Beyond the bare call: a `BIND`, a `VALUES` or an atom beside it, a `FILTER` after
/// it, a nested `LATERAL`, a sub-`SELECT` projecting `?q` (`SELECT *` included, and
/// under `DISTINCT` and `ORDER BY`), a `UNION`, a `GRAPH`. The right side is evaluated
/// once per left row and inner-joined with it, so writing the value into the call there
/// is the same as joining it on above — and each used to be refused, as "reachable only
/// as `ff`", while the same text with `VALUES ?q { … }` written in was admitted.
///
/// Each shape, for every term kind and name, under both rewrites and on every door and
/// the prepared execution, answers the rows the bottom-up join gives — every left row
/// joined with the right side's rows for the substituted value, counted by hand per
/// shape — and invokes the bound-only relation bound with exactly that value. The
/// free-capable relation is invoked bound only, which is the proof the admission is
/// right. For the kinds a query can spell, the same text with the value written into a
/// `VALUES` block answers the same rows.
#[test]
fn a_call_in_a_lateral_right_side_is_admitted_bound_and_answers_the_bottom_up_join() {
    let (env, invocations) = relations();
    for (position, right, per_left_row) in lateral_right_sides() {
        let query = format!(
            "SELECT ?q ?out WHERE {{ {} LATERAL {{ {right} }} }}",
            holds()
        );
        for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
            for kind in KINDS {
                for name in NAMES {
                    let value = kind.term(name);
                    let context =
                        format!("{lane:?} {position}: {query} ?q = {}:{name}", kind.tag());
                    let per_value: Vec<Vec<Option<TermValue>>> = (1..=rows_for(name))
                        .map(|n| vec![Some(value.clone()), Some(kind.output(name, n))])
                        .collect();
                    // Every left row, joined with the right side's rows for the
                    // substituted value: `per_left_row(1)` copies of them each.
                    let expected: Vec<Vec<Option<TermValue>>> = (0..HELD * per_left_row(1))
                        .flat_map(|_| per_value.iter().cloned())
                        .collect();
                    admitted_bound_everywhere(
                        &env,
                        &invocations,
                        lane,
                        &query,
                        &value,
                        &expected,
                        &context,
                    );

                    let (free_env, free_invocations) = relations_declaring(&["bf", "ff"]);
                    let substitutions = [("q".to_owned(), value.clone())];
                    let answer = request(
                        &NativeSparqlEngine::new(),
                        &dataset(),
                        &free_env,
                        Door::OptionsView,
                        lane,
                        &query,
                        &substitutions,
                    )
                    .unwrap_or_else(|message| panic!("{context}: free-capable: {message}"));
                    assert_eq!(answer, sorted(expected), "{context}: free-capable");
                    let seen = drain(&free_invocations);
                    assert!(!seen.is_empty(), "{context}: free-capable invoked");
                    for invocation in seen {
                        assert_eq!(
                            invocation,
                            ("bf".to_owned(), Some(value.clone())),
                            "{context}: the free-capable relation is invoked bound, with the \
                             substituted value — the proof the admission is right"
                        );
                    }
                }
            }
        }
        // The in-text neighbour: `VALUES ?q { … }` written at the top of the group,
        // for the kinds a query can spell, answers the substituted request's rows.
        for (kind, spelled) in [
            (Kind::Iri, format!("<{EX}alpha>")),
            (Kind::Literal, "\"alpha\"".to_owned()),
        ] {
            let in_text =
                query.replacen("WHERE {", &format!("WHERE {{ VALUES ?q {{ {spelled} }}"), 1);
            let context = format!("{position}: {in_text}");
            let written = request(
                &NativeSparqlEngine::new(),
                &dataset(),
                &env,
                Door::OptionsView,
                ShaclPrebinding::None,
                &in_text,
                &[],
            )
            .unwrap_or_else(|message| {
                panic!("{context}: the in-text VALUES is admitted: {message}")
            });
            let substituted = request(
                &NativeSparqlEngine::new(),
                &dataset(),
                &env,
                Door::OptionsView,
                ShaclPrebinding::None,
                &query,
                &[("q".to_owned(), kind.term("alpha"))],
            )
            .unwrap_or_else(|message| panic!("{context}: the substituted request: {message}"));
            assert!(!written.is_empty(), "{context}: rows to compare");
            assert_eq!(
                substituted, written,
                "{context}: the in-text neighbour's rows"
            );
            drain(&invocations);
        }
    }
}

/// **A `BIND` reading a promised parameter binds its target where the rewrite hands
/// the `BIND` the value.**
///
/// Under the SHACL pre-binding rewrite the value reaches every expression — written as
/// a constant, or, for a blank node or a quoted triple, driven into the `BIND`'s
/// operand — so `BIND(?q AS ?x) ?x <table> ?out` feeds the call bound and is admitted.
/// It used to be refused while a free-capable relation was invoked `bf`. Under the
/// ordinary rewrite the same text is still refused — the `BIND` sits beside the seed,
/// not above it — and the free-capable relation is observed invoked free there. A
/// `BIND` ABOVE the core, which the seed is joined beneath, reads the value under the
/// ordinary rewrite too, and feeds a call in the `EXISTS` filtering its rows.
#[test]
fn a_bind_of_a_promised_parameter_feeds_the_call() {
    let (env, invocations) = relations();
    let alias = format!("SELECT ?x ?out WHERE {{ BIND(?q AS ?x) ?x <{REL}> ?out }}");
    let above_core = format!(
        "SELECT ?x WHERE {{ {} BIND(?q AS ?x) FILTER EXISTS {{ ?x <{REL}> ?out }} }}",
        holds()
    );
    for kind in KINDS {
        for name in NAMES {
            let value = kind.term(name);
            let expected: Vec<Vec<Option<TermValue>>> = (1..=rows_for(name))
                .map(|n| vec![Some(value.clone()), Some(kind.output(name, n))])
                .collect();
            let context = format!("{alias} ?q = {}:{name}", kind.tag());
            admitted_bound_everywhere(
                &env,
                &invocations,
                ShaclPrebinding::Applied,
                &alias,
                &value,
                &expected,
                &context,
            );

            let (free_env, free_invocations) = relations_declaring(&["bf", "ff"]);
            let substitutions = [("q".to_owned(), value.clone())];
            let answer = request(
                &NativeSparqlEngine::new(),
                &dataset(),
                &free_env,
                Door::OptionsView,
                ShaclPrebinding::Applied,
                &alias,
                &substitutions,
            )
            .unwrap_or_else(|message| panic!("{context}: free-capable: {message}"));
            assert_eq!(answer, sorted(expected), "{context}: free-capable");
            for invocation in drain(&free_invocations) {
                assert_eq!(
                    invocation,
                    ("bf".to_owned(), Some(value.clone())),
                    "{context}: the free-capable relation is invoked bound"
                );
            }

            let passing: Vec<Vec<Option<TermValue>>> = if rows_for(name) == 0 {
                Vec::new()
            } else {
                (0..HELD).map(|_| vec![Some(value.clone())]).collect()
            };
            let context = format!("{above_core} ?q = {}:{name}", kind.tag());
            for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
                admitted_bound_everywhere(
                    &env,
                    &invocations,
                    lane,
                    &above_core,
                    &value,
                    &passing,
                    &format!("{lane:?} {context}"),
                );
            }
        }
        let context = format!("ordinary {alias} ?q = {}:alpha", kind.tag());
        refused_and_free_when_served(ShaclPrebinding::None, &alias, &kind.term("alpha"), &context);
    }
}

/// **A `BIND` the promise does not make certain stays refused, and one that errors on
/// the substituted value is refused per row — never invoked free.**
///
/// `BIND(?w AS ?x)` over an `OPTIONAL`'s variable is unbound wherever the `OPTIONAL`
/// did not match — here, everywhere — so the call's input is free under either rewrite:
/// refused, with the free-capable relation observed invoked free. `BIND(?q + 1 AS ?x)`
/// reads only the promised parameter, so it is admitted; the addition errors on every
/// value of every kind, leaving `?x` unbound in that row, and the evaluator refuses the
/// row's free invocation with a typed error rather than invoking the bound-only
/// relation free.
#[test]
fn a_bind_the_promise_does_not_make_certain_is_refused() {
    let optional_only = format!(
        "SELECT ?x ?out WHERE {{ {} OPTIONAL {{ ?h <{EX}absent> ?w }} BIND(?w AS ?x) \
         ?x <{REL}> ?out }}",
        holds()
    );
    let erroring = format!("SELECT ?x ?out WHERE {{ BIND(?q + 1 AS ?x) ?x <{REL}> ?out }}");
    let data = dataset();
    for kind in KINDS {
        let value = kind.term("alpha");
        for lane in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
            let context = format!("{lane:?} {optional_only} ?q = {}:alpha", kind.tag());
            refused_and_free_when_served(lane, &optional_only, &value, &context);
        }

        let (env, invocations) = relations();
        let context = format!("{erroring} ?q = {}:alpha", kind.tag());
        let engine = NativeSparqlEngine::new();
        let message = prepared(
            &engine,
            &data,
            &env,
            ShaclPrebinding::Applied,
            &erroring,
            &value,
        )
        .expect_err("the row's input is unbound, so its invocation is refused");
        assert!(
            message.contains("cannot serve the invocation `ff`"),
            "{context}: the per-row refusal, got {message}"
        );
        for door in DOORS {
            let engine = NativeSparqlEngine::new();
            let substitutions = [("q".to_owned(), value.clone())];
            let message = request(
                &engine,
                &data,
                &env,
                door,
                ShaclPrebinding::Applied,
                &erroring,
                &substitutions,
            )
            .expect_err("the row's input is unbound, so its invocation is refused");
            assert!(
                message.contains("cannot serve the invocation `ff`"),
                "{context} via {door:?}: the per-row refusal, got {message}"
            );
        }
        assert_eq!(
            drain(&invocations),
            Vec::<Invocation>::new(),
            "{context}: the bound-only relation is never invoked free"
        );
    }
}
