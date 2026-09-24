// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A value reaching a property-function call through CORRELATED substitution
//! arrives in the call's arguments as the term it is, whatever kind of term it is.
//!
//! Correlated substitution is the per-row rewrite behind an `EXISTS`/`NOT EXISTS`
//! body, a `LATERAL` right side that is not the call itself (a `UNION`, an
//! `OPTIONAL`, a sub-`SELECT` …) and everything nested inside one. A call's
//! arguments are invocation inputs, so the row's value has to be IN the call when
//! it runs: nothing else supplies it. An IRI always was written there. A literal,
//! a blank node and a quoted triple were not, so the call ran with its input free
//! and a relation serving only the bound mode refused it on every row, though the
//! plan admitted the query.
//!
//! # The oracle
//!
//! [`Table`] answers from the value it is handed: `alpha` has two rows, `beta`
//! one, `gamma` none — for every term kind, keyed by the kind as well as the name,
//! so a value of the wrong kind cannot pass for the right one. Every answer row
//! names its input. A value that was dropped leaves the input free, which the
//! bound-only relation refuses outright; the free-capable one answers from its
//! whole table instead, which names rows no single bound value produces. Every
//! invocation is recorded with its access pattern and the exact value it was
//! handed, so "the relation was invoked bound, with this value" is observed rather
//! than inferred from the answer.
//!
//! The differential ([`every_shape_answers_the_bottom_up_evaluation`]) checks each
//! shape against SPARQL's bottom-up answer: the relation evaluated FREE on its own,
//! joined or filtered against the left side afterwards.

use std::sync::{Arc, Mutex};

use pretty_assertions::assert_eq;
use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, Volatility,
};

/// The relation IRI every query calls — host configuration, never minted
/// vocabulary.
const REL: &str = "https://example.org/rel/table";

/// The fixture data namespace.
const EX: &str = "https://example.org/d/";

/// The value names the left side holds. `alpha` has two table rows, `beta` one and
/// `gamma` none, so `EXISTS` is true, true and false over them.
const LEFT_NAMES: &[&str] = &["alpha", "beta", "gamma"];

/// The table's own keys: `delta` is a key no left row holds, so a free evaluation
/// left unjoined would show.
const TABLE: &[(&str, u32)] = &[("alpha", 2), ("beta", 1), ("delta", 1)];

/// The four kinds of term a value can be.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Iri,
    Literal,
    Blank,
    Quoted,
}

const KINDS: [Kind; 4] = [Kind::Iri, Kind::Literal, Kind::Blank, Kind::Quoted];

impl Kind {
    /// The rendering prefix, and the local name of the predicate holding the
    /// left values of this kind.
    const fn tag(self) -> &'static str {
        match self {
            Self::Iri => "iri",
            Self::Literal => "lit",
            Self::Blank => "bn",
            Self::Quoted => "qt",
        }
    }

    /// The predicate `?s <predicate> ?q` binds `?q` through.
    fn predicate(self) -> String {
        format!("{EX}holds-{}", self.tag())
    }

    /// The term of this kind named `name`, as the relation emits it.
    fn term(self, name: &str) -> TermValue {
        match self {
            Self::Iri => TermValue::iri(format!("{EX}{name}")),
            Self::Literal => simple_literal(name),
            Self::Blank => TermValue::Blank {
                label: name.to_owned(),
                scope: BlankScope::DEFAULT,
            },
            Self::Quoted => TermValue::Triple {
                s: Box::new(TermValue::iri(format!("{EX}a"))),
                p: Box::new(TermValue::iri(format!("{EX}r"))),
                o: Box::new(TermValue::iri(format!("{EX}{name}"))),
            },
        }
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

/// A term rendered with its kind: `iri:alpha`, `lit:alpha`, `bn:alpha`, `qt:alpha`.
fn key_of(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => format!("iri:{}", iri.strip_prefix(EX).unwrap_or(iri)),
        TermValue::Literal { lexical_form, .. } => format!("lit:{lexical_form}"),
        TermValue::Blank { label, .. } => format!("bn:{label}"),
        // `<<( <a> <r> X )>>` renders by `X`: `qt:alpha` for the IRI `alpha` — the
        // quoted values the dataset holds — and `qt:lit:alpha`, `qt:bn:alpha` for
        // any other kind of object, which a nested argument can build.
        TermValue::Triple { s, p, o } => match (&**s, &**p, &**o) {
            (TermValue::Iri(s), TermValue::Iri(p), TermValue::Iri(o))
                if *s == format!("{EX}a") && *p == format!("{EX}r") =>
            {
                format!("qt:{}", o.strip_prefix(EX).unwrap_or(o))
            }
            (TermValue::Iri(s), TermValue::Iri(p), object)
                if *s == format!("{EX}a") && *p == format!("{EX}r") =>
            {
                format!("qt:{}", key_of(object))
            }
            _ => format!("qt:?{term:?}"),
        },
    }
}

/// `?input <table> ?output`: arity `(1, 1)`, serving `bf` always and `ff` when built
/// free-capable. Bound, it answers [`TABLE`]'s rows for the value's name — of the
/// value's own kind; free, every row of every kind.
struct Table {
    modes: Vec<BindingPattern>,
    invocations: Arc<Mutex<Vec<String>>>,
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
        let input = args.get(0);
        let key = input.map(key_of);
        self.invocations
            .lock()
            .expect("the fixture recorder is never poisoned")
            .push(format!(
                "{}:{}",
                args.mode().code(),
                key.as_deref().unwrap_or("-")
            ));
        let mut rows: Vec<PfRow> = Vec::new();
        for kind in KINDS {
            for &(name, count) in TABLE {
                let term = kind.term(name);
                let term_key = key_of(&term);
                if key.as_deref().is_some_and(|key| key != term_key) {
                    continue;
                }
                for n in 1..=count {
                    rows.push(vec![
                        term.clone(),
                        simple_literal(&format!("{term_key}/{n}")),
                    ]);
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

/// For every kind, `<s-alpha> <holds-kind> alpha`, the same for `beta` and
/// `gamma`, and a second subject `<t-alpha>` holding `alpha` again — two left rows
/// sharing a value, which an `EXISTS` memo may answer once for both.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let r = builder.intern_iri(&format!("{EX}r"));
    for kind in KINDS {
        let predicate = builder.intern_iri(&kind.predicate());
        for &name in LEFT_NAMES {
            let value = match kind {
                Kind::Iri => builder.intern_iri(&format!("{EX}{name}")),
                Kind::Literal => builder.intern_literal(RdfLiteral::simple(name)),
                Kind::Blank => builder.intern_blank(name, BlankScope::DEFAULT),
                Kind::Quoted => {
                    let object = builder.intern_iri(&format!("{EX}{name}"));
                    builder.intern_triple(a, r, object)
                }
            };
            let subject = builder.intern_iri(&format!("{EX}s-{name}"));
            builder.push_quad(subject, predicate, value, None);
            if name == "alpha" {
                let twin = builder.intern_iri(&format!("{EX}t-{name}"));
                builder.push_quad(twin, predicate, value, None);
            }
        }
    }
    builder.freeze().expect("the fixture must validate")
}

/// Which variant of [`Table`] a query runs against.
#[derive(Clone, Copy, Debug)]
enum Variant {
    BoundOnly,
    FreeCapable,
}

/// What one query did: its sorted `(?q, ?out)` rows or its refusal message, plus
/// every invocation the relation saw, sorted.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    answer: Result<Vec<(String, String)>, String>,
    invocations: Vec<String>,
}

fn render_q(cell: Option<&TermValue>) -> String {
    cell.map_or_else(|| "UNBOUND".to_owned(), key_of)
}

fn render_out(cell: Option<&TermValue>) -> String {
    match cell {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(other) => format!("?{other:?}"),
    }
}

/// Run `SELECT ?q ?out WHERE { body }` against [`Table`].
fn run(variant: Variant, body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let modes = match variant {
        Variant::BoundOnly => vec![BindingPattern::from_code("bf")],
        Variant::FreeCapable => vec![
            BindingPattern::from_code("bf"),
            BindingPattern::from_code("ff"),
        ],
    };
    let relation = Table {
        modes,
        invocations: Arc::clone(&invocations),
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(REL.to_owned(), Arc::new(relation));
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    let query = format!("SELECT ?q ?out WHERE {{ {body} }}");
    let answer = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                env: &env,
                ..QueryOptions::EMPTY
            },
        )
        .map(|result| {
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("a SELECT answers with solutions");
            };
            let mut rendered: Vec<(String, String)> = rows
                .iter()
                .map(|row| (render_q(row[0].as_ref()), render_out(row[1].as_ref())))
                .collect();
            rendered.sort();
            rendered
        })
        .map_err(|diagnostic| diagnostic.message);
    let mut invocations = invocations
        .lock()
        .expect("the fixture recorder is never poisoned")
        .clone();
    invocations.sort();
    Outcome {
        answer,
        invocations,
    }
}

/// The correlated shapes under test, each reading `?q` from the left side.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// `FILTER EXISTS { ?q <rel> ?out }`.
    Exists,
    /// `FILTER NOT EXISTS { ?q <rel> ?out }`.
    NotExists,
    /// A `LATERAL` whose right side is a `UNION` of the call with itself.
    LateralUnion,
    /// A `LATERAL` whose right side is an `OPTIONAL` containing the call.
    LateralOptional,
    /// A `LATERAL` whose right side is a sub-`SELECT` projecting `?q`, read directly
    /// by the call.
    LateralSubSelect,
    /// `FILTER EXISTS { atom . call }`: the call follows an atom, so it is the
    /// direct right operand of a `LATERAL` inside the body, and a driven value is
    /// carried on that `LATERAL`'s left side.
    ExistsAfterAtom,
    /// The same inside a `LATERAL`: `LATERAL { atom . call }`.
    LateralAfterAtom,
    /// A `LATERAL` nested in another, the call in an `OPTIONAL` of the inner one:
    /// the inner window substitutes a body the outer window already bound the call
    /// in, and must neither drop nor re-drive what is already there.
    NestedLateralOptional,
}

const SHAPES: [Shape; 8] = [
    Shape::Exists,
    Shape::NotExists,
    Shape::LateralUnion,
    Shape::LateralOptional,
    Shape::LateralSubSelect,
    Shape::ExistsAfterAtom,
    Shape::LateralAfterAtom,
    Shape::NestedLateralOptional,
];

impl Shape {
    fn body(self, kind: Kind) -> String {
        let left = format!("?s <{}> ?q", kind.predicate());
        let call = format!("?q <{REL}> ?out");
        match self {
            Self::Exists => format!("{left} FILTER EXISTS {{ {call} }}"),
            Self::NotExists => format!("{left} FILTER NOT EXISTS {{ {call} }}"),
            Self::LateralUnion => {
                format!("{left} LATERAL {{ {{ {call} }} UNION {{ {call} }} }}")
            }
            Self::LateralOptional => format!("{left} LATERAL {{ OPTIONAL {{ {call} }} }}"),
            Self::LateralSubSelect => {
                format!("{left} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} }}")
            }
            // `?s` holds exactly one value per predicate, so the atom yields one row
            // per left row and changes no multiplicity.
            Self::ExistsAfterAtom => format!(
                "{left} FILTER EXISTS {{ ?s <{}> ?w . {call} }}",
                kind.predicate()
            ),
            Self::LateralAfterAtom => {
                format!("{left} LATERAL {{ ?s <{}> ?w . {call} }}", kind.predicate())
            }
            Self::NestedLateralOptional => format!(
                "{left} LATERAL {{ ?s <{}> ?w LATERAL {{ OPTIONAL {{ {call} }} }} }}",
                kind.predicate()
            ),
        }
    }

    /// SPARQL's answer for this shape over `left` (the left side's `?q` values,
    /// with multiplicity) and `free` (the relation's free rows): the join or filter
    /// applied AFTER the relation was evaluated on its own.
    fn bottom_up(self, left: &[String], free: &[(String, String)]) -> Vec<(String, String)> {
        let matches = |q: &String| -> Vec<(String, String)> {
            free.iter()
                .filter(|(input, _)| input == q)
                .cloned()
                .collect()
        };
        let mut out: Vec<(String, String)> = Vec::new();
        for q in left {
            let found = matches(q);
            match self {
                Self::Exists | Self::ExistsAfterAtom => {
                    if !found.is_empty() {
                        out.push((q.clone(), "UNBOUND".to_owned()));
                    }
                }
                Self::NotExists => {
                    if found.is_empty() {
                        out.push((q.clone(), "UNBOUND".to_owned()));
                    }
                }
                Self::LateralUnion => {
                    out.extend(found.iter().cloned());
                    out.extend(found);
                }
                Self::LateralOptional | Self::NestedLateralOptional => {
                    if found.is_empty() {
                        out.push((q.clone(), "UNBOUND".to_owned()));
                    } else {
                        out.extend(found);
                    }
                }
                Self::LateralSubSelect | Self::LateralAfterAtom => out.extend(found),
            }
        }
        out.sort();
        out
    }
}

/// The left side's `?q` values for `kind`, with multiplicity: `alpha` twice.
fn left_values(kind: Kind) -> Vec<String> {
    let mut left = Vec::new();
    for &name in LEFT_NAMES {
        let key = format!("{}:{name}", kind.tag());
        if name == "alpha" {
            left.push(key.clone());
        }
        left.push(key);
    }
    left
}

/// The table's rows for `kind`, as the bound relation answers them — the model the
/// bound-only assertions compare against, independent of any engine evaluation.
fn modelled_rows(kind: Kind) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for &(name, count) in TABLE {
        let key = format!("{}:{name}", kind.tag());
        for n in 1..=count {
            rows.push((key.clone(), format!("{key}/{n}")));
        }
    }
    rows
}

/// The distinct invocations of a run, sorted.
fn distinct(invocations: &[String]) -> Vec<String> {
    let mut distinct = invocations.to_vec();
    distinct.dedup();
    distinct
}

/// The bound invocation set every shape must make for `kind`: each left value,
/// bound, with its exact term — and nothing else.
fn bound_invocations(kind: Kind) -> Vec<String> {
    LEFT_NAMES
        .iter()
        .map(|name| format!("bf:{}:{name}", kind.tag()))
        .collect()
}

/// Every shape, every term kind, a relation serving ONLY the bound mode: the query
/// answers, the relation is invoked bound with exactly each left value and never
/// free, and the answer is the one the table gives for those values.
#[test]
fn every_value_kind_reaches_a_bound_only_call_through_correlated_substitution() {
    for kind in KINDS {
        for shape in SHAPES {
            let body = shape.body(kind);
            let outcome = run(Variant::BoundOnly, &body);
            let expected = shape.bottom_up(&left_values(kind), &modelled_rows(kind));
            assert!(
                !expected.is_empty(),
                "{shape:?}/{kind:?} must have rows to compare"
            );
            assert_eq!(
                outcome.answer.as_ref(),
                Ok(&expected),
                "{shape:?} over a {kind:?} value answers from the value itself: {body}"
            );
            assert_eq!(
                distinct(&outcome.invocations),
                bound_invocations(kind),
                "{shape:?} over a {kind:?} value invokes the relation bound with each \
                 exact value, never free: {body}"
            );
        }
    }
}

/// The free-capable relation must answer exactly as the bound-only one: a value
/// that was dropped would leave the input free, and a relation that serves the
/// free mode would then answer from its WHOLE table — including `delta`, which no
/// left row holds, and rows of other kinds.
#[test]
fn a_free_capable_relation_is_still_invoked_bound() {
    for kind in KINDS {
        for shape in SHAPES {
            let body = shape.body(kind);
            let bound_only = run(Variant::BoundOnly, &body);
            let free_capable = run(Variant::FreeCapable, &body);
            assert_eq!(
                free_capable, bound_only,
                "{shape:?} over a {kind:?} value answers and invokes identically whether \
                 or not the relation also serves `ff`: {body}"
            );
        }
    }
}

/// Each shape's answer equals SPARQL's bottom-up evaluation, computed from the
/// ENGINE's own answers: the left side on its own, and the relation evaluated free
/// on its own, joined or filtered afterwards.
#[test]
fn every_shape_answers_the_bottom_up_evaluation() {
    let free = run(Variant::FreeCapable, &format!("?q <{REL}> ?out"));
    assert_eq!(
        free.invocations,
        vec!["ff:-".to_owned()],
        "the reference is the free evaluation"
    );
    let free_rows = free.answer.expect("the free evaluation answers");
    for kind in KINDS {
        let left = run(
            Variant::FreeCapable,
            &format!("?s <{}> ?q", kind.predicate()),
        )
        .answer
        .expect("the left side answers");
        let left: Vec<String> = left.into_iter().map(|(q, _)| q).collect();
        assert_eq!(left, left_values(kind), "the left side of {kind:?}");
        for shape in SHAPES {
            let body = shape.body(kind);
            let reference = shape.bottom_up(&left, &free_rows);
            for variant in [Variant::BoundOnly, Variant::FreeCapable] {
                let outcome = run(variant, &body);
                assert_eq!(
                    outcome.answer.as_ref(),
                    Ok(&reference),
                    "{shape:?} over a {kind:?} value, {variant:?}, answers the bottom-up \
                     evaluation: {body}"
                );
                assert!(
                    outcome
                        .invocations
                        .iter()
                        .all(|invocation| invocation.starts_with("bf:")),
                    "{shape:?} over a {kind:?} value, {variant:?}, is invoked bound only: {:?}",
                    outcome.invocations
                );
            }
        }
    }
}

/// A correlated body holding a call is evaluated once per left row, with that row's
/// own value. For an `EXISTS` this pins the memo question: the definition path's
/// restriction memo is skipped for a body reaching a relation (host code of unknown
/// volatility), so the two left rows sharing `alpha` each hand the relation `alpha`,
/// and `gamma`'s row is never answered from `alpha`'s evaluation. The exact multiset
/// of invocations is pinned for every shape: one per left row (per `UNION` branch),
/// each with that row's own value.
#[test]
fn an_exists_over_a_call_is_evaluated_with_each_rows_own_value() {
    for kind in KINDS {
        let tag = kind.tag();
        for shape in SHAPES {
            // A `UNION` of the call with itself invokes it once per branch.
            let per_left_row = if matches!(shape, Shape::LateralUnion) {
                2
            } else {
                1
            };
            let mut per_row = Vec::new();
            for name in ["alpha", "alpha", "beta", "gamma"] {
                for _ in 0..per_left_row {
                    per_row.push(format!("bf:{tag}:{name}"));
                }
            }
            let body = shape.body(kind);
            let outcome = run(Variant::BoundOnly, &body);
            assert_eq!(
                outcome.invocations, per_row,
                "{shape:?} over a {kind:?} value invokes the relation once per left row, \
                 with that row's value: {body}"
            );
        }
    }
}

// ── the prepared-execution path ──────────────────────────────────────────────

/// A prepared execution whose declared parameter is `?q` binds it by the run, not
/// the text: the plain lane's rewrite writes or seeds it at the core (and its
/// retained substituted plan — the pre-binding memo — is replayed on every run after
/// the first of a value shape), and the rows the correlated body is evaluated over
/// carry it from there. Each run is answered with exactly the bound value's rows
/// and invokes the relation bound with it, and re-binding an earlier value answers
/// as it did the first time — so a memo replaying a stale cell would show.
#[test]
fn a_prepared_parameter_reaches_the_correlated_call_on_every_run() {
    use purrdf_sparql_eval::InternedOutcome;

    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = Table {
        modes: vec![BindingPattern::from_code("bf")],
        invocations: Arc::clone(&invocations),
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(REL.to_owned(), Arc::new(relation));
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let engine = NativeSparqlEngine::new();
    let data = dataset();

    for kind in KINDS {
        let tag = kind.tag();
        for shape in SHAPES {
            let text = format!("SELECT ?q ?out WHERE {{ {} }}", shape.body(kind));
            let mut execution = engine
                .prepare_execution(&text, None, &["q"], options)
                .unwrap_or_else(|diagnostic| panic!("{text} prepares: {diagnostic}"));
            let slot = execution.slot("q").expect("declared");
            for name in ["alpha", "beta", "gamma", "alpha"] {
                invocations
                    .lock()
                    .expect("the fixture recorder is never poisoned")
                    .clear();
                execution
                    .bind(slot, kind.term(name))
                    .expect("the value door binds every term kind");
                let mut answered = engine
                    .execute(&mut execution, &*data, options, |outcome| match outcome {
                        InternedOutcome::Solutions(solutions) => solutions
                            .rows()
                            .iter()
                            .map(|row| {
                                (
                                    render_q(solutions.cell(row, 0).as_ref()),
                                    render_out(solutions.cell(row, 1).as_ref()),
                                )
                            })
                            .collect::<Vec<_>>(),
                        InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
                            panic!("a SELECT answers with solutions")
                        }
                    })
                    .unwrap_or_else(|diagnostic| {
                        panic!("{text} bound to {tag}:{name} runs: {diagnostic}")
                    });
                answered.sort();
                let key = format!("{tag}:{name}");
                let left: Vec<String> = left_values(kind)
                    .into_iter()
                    .filter(|value| *value == key)
                    .collect();
                assert_eq!(
                    answered,
                    shape.bottom_up(&left, &modelled_rows(kind)),
                    "{text} bound to {key}"
                );
                let mut seen = invocations
                    .lock()
                    .expect("the fixture recorder is never poisoned")
                    .clone();
                seen.sort();
                seen.dedup();
                // A blank-node parameter is never written into the left side's
                // triple — a blank in a pattern is an anonymous variable — so that
                // side is narrowed by the seed join above the `LATERAL`, not before
                // it. A call the rewrite reaches inside the `LATERAL`'s right side is
                // driven with the parameter itself and runs for it alone; a call in an
                // `OPTIONAL` arm there, which the rewrite does not enter, is handed each
                // left row's own value, bound, so it also runs for the left rows the
                // seed then discards. Every other kind is written into the triple.
                let expected: Vec<String> = match kind {
                    Kind::Blank
                        if matches!(
                            shape,
                            Shape::LateralOptional | Shape::NestedLateralOptional
                        ) =>
                    {
                        bound_invocations(kind)
                    }
                    _ => vec![format!("bf:{key}")],
                };
                assert_eq!(
                    seen, expected,
                    "{text} bound to {key} invokes the relation bound, with {key} for \
                     every row it answers"
                );
            }
        }
    }
}

// ── a variable nested inside a quoted-triple argument ────────────────────────

/// A call's argument may be a quoted triple naming the correlated variable inside
/// it: `( <<( <a> <r> ?q )>> ) <rel> ( ?out )`. That component is an input of the
/// call exactly as a bare argument is, so it is written (an IRI, a literal) or
/// driven (a blank node) at any depth. Left unwritten, the relation was handed the
/// triple with that component free: refused by the bound-only relation, and — the
/// worse half — answered by the free-capable one from its whole table, so an
/// `EXISTS` over `gamma`, which the relation holds nothing for, answered true.
///
/// Only an IRI `?q` builds a triple the table holds (`qt:alpha`, `qt:beta`); a
/// literal or blank `?q` builds one it does not, so those answer from nothing — and
/// the invocations, each bound with exactly the triple its row built, are what
/// distinguish them.
#[test]
fn a_variable_nested_in_a_quoted_triple_argument_arrives_bound() {
    for kind in [Kind::Iri, Kind::Literal, Kind::Blank] {
        let left = format!("?s <{}> ?q", kind.predicate());
        let call = format!("( <<( <{EX}a> <{EX}r> ?q )>> ) <{REL}> ( ?out )");
        let nested_key = |name: &str| match kind {
            Kind::Iri => format!("qt:{name}"),
            _ => format!("qt:{}:{name}", kind.tag()),
        };
        let table_rows: Vec<(String, String)> = match kind {
            Kind::Iri => modelled_rows(Kind::Quoted),
            _ => Vec::new(),
        };
        for (body, exists_like) in [
            (format!("{left} FILTER EXISTS {{ {call} }}"), true),
            (format!("{left} LATERAL {{ OPTIONAL {{ {call} }} }}"), false),
        ] {
            let mut expected: Vec<(String, String)> = Vec::new();
            for q in left_values(kind) {
                let name = q.split_once(':').map_or("", |(_, name)| name).to_owned();
                let found: Vec<&(String, String)> = table_rows
                    .iter()
                    .filter(|(input, _)| *input == nested_key(&name))
                    .collect();
                if exists_like {
                    if !found.is_empty() {
                        expected.push((q.clone(), "UNBOUND".to_owned()));
                    }
                } else if found.is_empty() {
                    expected.push((q.clone(), "UNBOUND".to_owned()));
                } else {
                    expected.extend(found.into_iter().map(|(_, out)| (q.clone(), out.clone())));
                }
            }
            expected.sort();
            let bound_only = run(Variant::BoundOnly, &body);
            assert_eq!(
                bound_only.answer.as_ref(),
                Ok(&expected),
                "a {kind:?} value nested in a quoted-triple argument: {body}"
            );
            let mut seen = bound_only.invocations.clone();
            seen.dedup();
            assert_eq!(
                seen,
                LEFT_NAMES
                    .iter()
                    .map(|name| format!("bf:{}", nested_key(name)))
                    .collect::<Vec<_>>(),
                "a {kind:?} value nested in a quoted-triple argument is invoked bound with \
                 the triple its own row builds: {body}"
            );
            assert_eq!(
                run(Variant::FreeCapable, &body),
                bound_only,
                "the free-capable relation answers and is invoked identically: {body}"
            );
        }
    }
}
