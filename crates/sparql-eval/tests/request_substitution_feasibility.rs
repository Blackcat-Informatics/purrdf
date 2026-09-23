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

use pretty_assertions::assert_eq;
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

/// `?input <table> ?output`, arity `(1, 1)`, serving `bf` only.
struct Table {
    modes: [BindingPattern; 1],
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
        let Some(input) = input else {
            return Err(EvalError::function(
                "the input is free; this relation serves only `bf`".to_owned(),
            ));
        };
        let mut rows: Vec<PfRow> = Vec::new();
        for kind in KINDS {
            for name in NAMES {
                if kind.term(name) == input {
                    for n in 1..=rows_for(name) {
                        rows.push(vec![input.clone(), kind.output(name, n)]);
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
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(Table {
            modes: [BindingPattern::from_code("bf")],
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
