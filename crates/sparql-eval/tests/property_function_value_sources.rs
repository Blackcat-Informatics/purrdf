// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What feeds a property function's input position: every pattern that binds a
//! variable on every row the query text lets reach the call, not only a triple.
//!
//! A relation that serves only with its input bound is admitted at prepare time
//! exactly when a feasible order binds that input first. A triple pattern always
//! did; this file pins the rest of the rule — an inline `VALUES` column with no
//! `UNDEF`, a `BIND`, a sub-`SELECT` that projects the variable, a nested group,
//! a `UNION` both of whose branches bind it, the variable naming a `GRAPH`, a
//! `LATERAL`'s left side, and a `LATERAL`'s right side judged with its left side's
//! bindings in hand — and the refusals that bound it: an `UNDEF` cell, a `BIND`
//! written after the call, a `BIND` over an `OPTIONAL` variable, a sub-`SELECT` that
//! does not project it, a `UNION` branch that does not bind it.
//!
//! Every shape the `LATERAL` rule admits is also checked against SPARQL's bottom-up
//! answer: a relation serving its input both bound and free must answer the same
//! rows bound as its free evaluation joined with the rest of the query afterwards.
//!
//! # The oracle
//!
//! [`Expand`] answers from the value it is handed: `"alpha"` expands to two rows,
//! anything else to one, and every row names its input. So a source that was
//! silently dropped cannot pass for one that was honoured — a dropped source leaves
//! the input free, which the bound-only relation refuses, and the free-capable
//! variant answers with a row naming no input at all. Every invocation is recorded
//! with the access pattern it was made in, so "the relation was never invoked with
//! its input free" is observed, not inferred from the answer.

use std::sync::{Arc, Mutex};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, BindingPattern,
    CustomAggregate, EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, ShaclPrebinding, TypeConstraint,
    UserFnBody, UserFnParam, UserFunction, UserFunctionRegistry, Volatility,
};

/// The relation IRI every query calls — host configuration, never minted
/// vocabulary.
const EXPAND: &str = "https://example.org/rel/expand";

/// The fixture data namespace.
const EX: &str = "https://example.org/d/";

/// The row the free-capable variant emits when its input is free: it names no
/// input, so it can never be mistaken for a row a bound input produced.
const FREE_ROW: &str = "free";

/// `?input <expand> ?output`: arity `(1, 1)`, serving `bf` always and `ff` only
/// when built with [`Expand::free_capable`].
struct Expand {
    modes: Vec<BindingPattern>,
    invocations: Arc<Mutex<Vec<String>>>,
}

impl Expand {
    /// The relation that serves ONLY with its input bound.
    fn bound_only(invocations: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            modes: vec![BindingPattern::from_code("bf")],
            invocations,
        }
    }

    /// The relation that also serves with its input free.
    fn free_capable(invocations: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            modes: vec![
                BindingPattern::from_code("bf"),
                BindingPattern::from_code("ff"),
            ],
            invocations,
        }
    }
}

/// The text a bound input term expands from: a literal's lexical form, an IRI's
/// string.
fn key_of(term: &TermValue) -> String {
    match term {
        TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
        TermValue::Iri(iri) => iri.clone(),
        other => format!("{other:?}"),
    }
}

impl PropertyFunction for Expand {
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
        let recorded = input.map_or_else(
            || format!("{}:-", args.mode().code()),
            |term| format!("{}:{}", args.mode().code(), key_of(term)),
        );
        self.invocations
            .lock()
            .expect("the fixture recorder is never poisoned")
            .push(recorded);
        let rows: Vec<PfRow> = match input {
            Some(term) => {
                let key = key_of(term);
                let count = if key == "alpha" { 2 } else { 1 };
                (1..=count)
                    .map(|n| {
                        vec![
                            term.clone(),
                            TermValue::Literal {
                                lexical_form: format!("{key}/{n}"),
                                datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
                                language: None,
                                direction: None,
                            },
                        ]
                    })
                    .collect()
            }
            None => vec![vec![
                TermValue::iri(format!("{EX}{FREE_ROW}")),
                TermValue::Literal {
                    lexical_form: FREE_ROW.to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
                    language: None,
                    direction: None,
                },
            ]],
        };
        Ok(Box::new(Rows(rows.into_iter())))
    }
}

struct Rows(std::vec::IntoIter<PfRow>);

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.0.next())
    }
}

/// `<s> <p> "beta"` in the default graph and `<s> <p> "gamma"` in `<g>`. Nothing
/// holds `<nothing>`, so the `COALESCE` case's `OPTIONAL` leaves its variable
/// unbound and the fallback is what reaches the call.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let beta = builder.intern_literal(RdfLiteral::simple("beta"));
    builder.push_quad(s, p, beta, None);
    let g = builder.intern_iri(&format!("{EX}g"));
    let gamma = builder.intern_literal(RdfLiteral::simple("gamma"));
    builder.push_quad(s, p, gamma, Some(g));
    builder.freeze().expect("the fixture must validate")
}

/// Which variant of [`Expand`] a query runs against.
#[derive(Clone, Copy)]
enum Variant {
    BoundOnly,
    FreeCapable,
}

/// What one query did: its sorted `(?q, ?out)` rows, or its refusal message, plus
/// every invocation the relation saw, sorted.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    answer: Result<Vec<(String, String)>, String>,
    invocations: Vec<String>,
}

fn render(cell: Option<&TermValue>) -> String {
    cell.map_or_else(|| "UNBOUND".to_owned(), key_of)
}

/// Run `body` as `SELECT ?q ?out WHERE { body }` against [`Expand`].
fn run(variant: Variant, body: &str) -> Outcome {
    run_over(&dataset(), variant, body)
}

/// [`run`] over `data` rather than the shared fixture.
fn run_over(data: &RdfDataset, variant: Variant, body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = match variant {
        Variant::BoundOnly => Expand::bound_only(Arc::clone(&invocations)),
        Variant::FreeCapable => Expand::free_capable(Arc::clone(&invocations)),
    };
    run_relation(data, EXPAND, Arc::new(relation), &invocations, body)
}

/// Run `body` as `SELECT ?q ?out WHERE { body }` over `data`, with `relation`
/// registered at `iri` and recording into `invocations`.
fn run_relation(
    data: &RdfDataset,
    iri: &str,
    relation: Arc<dyn PropertyFunction>,
    invocations: &Mutex<Vec<String>>,
    body: &str,
) -> Outcome {
    run_relation_with(
        data,
        iri,
        relation,
        invocations,
        body,
        &[],
        ShaclPrebinding::None,
    )
}

/// [`run_relation`], with `substitutions` bound under `lane`.
fn run_relation_with(
    data: &RdfDataset,
    iri: &str,
    relation: Arc<dyn PropertyFunction>,
    invocations: &Mutex<Vec<String>>,
    body: &str,
    substitutions: &[(String, TermValue)],
    lane: ShaclPrebinding,
) -> Outcome {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(iri.to_owned(), relation);
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    run_in(&env, data, invocations, body, substitutions, lane)
}

/// Run `body` as `SELECT ?q ?out WHERE { body }` over `data` in `env`, whose relation
/// records into `invocations`.
fn run_in(
    env: &ExtensionEnv,
    data: &RdfDataset,
    invocations: &Mutex<Vec<String>>,
    body: &str,
    substitutions: &[(String, TermValue)],
    lane: ShaclPrebinding,
) -> Outcome {
    let query = format!("SELECT ?q ?out WHERE {{ {body} }}");
    let answer = NativeSparqlEngine::new()
        .query_with_options_view(
            data,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions,
            },
            QueryOptions {
                env,
                prebinding: lane,
                ..QueryOptions::EMPTY
            },
        )
        .map(|result| {
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("a SELECT answers with solutions");
            };
            let mut rendered: Vec<(String, String)> = rows
                .iter()
                .map(|row| (render(row[0].as_ref()), render(row[1].as_ref())))
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

fn rows(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|&(q, out)| (q.to_owned(), out.to_owned()))
        .collect()
}

fn calls(recorded: &[&str]) -> Vec<String> {
    recorded.iter().map(|&call| call.to_owned()).collect()
}

/// The two-row answer of `"alpha"` and `"beta"` both reaching the call bound.
fn alpha_and_beta() -> Outcome {
    Outcome {
        answer: Ok(rows(&[
            ("alpha", "alpha/1"),
            ("alpha", "alpha/2"),
            ("beta", "beta/1"),
        ])),
        invocations: calls(&["bf:alpha", "bf:beta"]),
    }
}

/// Assert `outcome` is a prepare-time refusal: the "no feasible order" message, and
/// the relation never invoked.
fn assert_refused_at_prepare(outcome: &Outcome) {
    let Err(message) = &outcome.answer else {
        panic!("this must be refused at prepare, got {outcome:?}");
    };
    assert!(
        message.contains("no feasible evaluation order")
            && message.contains(&format!("<{EXPAND}> reachable only as `ff`")),
        "the refusal names the call and its free input: {message}"
    );
    assert_eq!(outcome.invocations, Vec::<String>::new());
}

// ── the control ──────────────────────────────────────────────────────────────

/// The shape that always worked: a triple binds the input. The baseline every
/// source below is measured against.
#[test]
fn a_triple_feeds_the_input() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("<{EX}s> <{EX}p> ?q . ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
}

/// With nothing binding the input, a bound-only relation is refused at prepare —
/// and the free-capable variant answers its free row, so the refusal is the
/// relation's declaration and not a broken call.
#[test]
fn nothing_binding_the_input_is_refused_and_the_free_capable_neighbour_answers() {
    assert_refused_at_prepare(&run(Variant::BoundOnly, &format!("?q <{EXPAND}> ?out")));
    assert_eq!(
        run(Variant::FreeCapable, &format!("?q <{EXPAND}> ?out")),
        Outcome {
            answer: Ok(rows(&[("https://example.org/d/free", "free")])),
            invocations: calls(&["ff:-"]),
        }
    );
}

// ── VALUES ───────────────────────────────────────────────────────────────────

/// Each `VALUES` row reaches the call bound, and each answers with its own rows.
#[test]
fn values_before_the_call_feeds_it_per_row() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("VALUES ?q {{ \"alpha\" \"beta\" }} ?q <{EXPAND}> ?out")
        ),
        alpha_and_beta()
    );
}

/// A group's operands join commutatively, so a `VALUES` written after the call is
/// the same table joined into the same group: the planner runs it first and the
/// call sees it bound — exactly as it already did for a triple written after.
#[test]
fn values_after_the_call_in_the_same_group_feeds_it() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("?q <{EXPAND}> ?out VALUES ?q {{ \"alpha\" \"beta\" }}")
        ),
        alpha_and_beta()
    );
    // The triple-after control the rule is borrowed from.
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("?q <{EXPAND}> ?out . <{EX}s> <{EX}p> ?q")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
}

/// An `UNDEF` cell is a row the text itself leaves the input unbound in, so a
/// `VALUES` column holding one is not a source: a bound-only relation is refused at
/// prepare, never invoked free. The free-capable variant is admitted, and each row
/// is invoked in its own access pattern — `"alpha"` bound, the `UNDEF` row free.
#[test]
fn an_undef_cell_is_not_a_source() {
    assert_refused_at_prepare(&run(
        Variant::BoundOnly,
        &format!("VALUES ?q {{ \"alpha\" UNDEF }} ?q <{EXPAND}> ?out"),
    ));
    assert_eq!(
        run(
            Variant::FreeCapable,
            &format!("VALUES ?q {{ \"alpha\" UNDEF }} ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[
                ("alpha", "alpha/1"),
                ("alpha", "alpha/2"),
                ("https://example.org/d/free", "free"),
            ])),
            invocations: calls(&["bf:alpha", "ff:-"]),
        }
    );
    // The neighbour: the same table without the `UNDEF` is a source.
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("VALUES ?q {{ \"alpha\" }} ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("alpha", "alpha/1"), ("alpha", "alpha/2")])),
            invocations: calls(&["bf:alpha"]),
        }
    );
}

/// Only the column's own cells matter: an `UNDEF` in ANOTHER column leaves `?q` a
/// source.
#[test]
fn an_undef_in_another_column_leaves_the_input_a_source() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("VALUES (?q ?other) {{ (\"alpha\" UNDEF) (\"beta\" 1) }} ?q <{EXPAND}> ?out")
        ),
        alpha_and_beta()
    );
}

// ── BIND ─────────────────────────────────────────────────────────────────────

/// A `BIND` before the call feeds it, constant or computed.
#[test]
fn bind_before_the_call_feeds_it() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("BIND(\"beta\" AS ?q) ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("BIND(CONCAT(\"al\", \"pha\") AS ?q) ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("alpha", "alpha/1"), ("alpha", "alpha/2")])),
            invocations: calls(&["bf:alpha"]),
        }
    );
    // Over a triple's rows, the computed value is per row.
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("<{EX}s> <{EX}p> ?v BIND(CONCAT(?v, \"!\") AS ?q) ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("beta!", "beta!/1")])),
            invocations: calls(&["bf:beta!"]),
        }
    );
}

/// A `BIND` ends the basic graph pattern before it and scopes over only what
/// precedes it, so one written after the call is not a source for it. SPARQL goes
/// further and forbids the `BIND` from introducing a variable already used earlier
/// in the group, so the text is refused at parse — before the planner or the
/// relation is reached.
#[test]
fn bind_after_the_call_is_not_a_source() {
    let outcome = run(
        Variant::BoundOnly,
        &format!("?q <{EXPAND}> ?out BIND(\"beta\" AS ?q)"),
    );
    let Err(message) = &outcome.answer else {
        panic!("a BIND after the call must not feed it, got {outcome:?}");
    };
    assert!(
        message.contains("?q") && message.contains("already in scope"),
        "the refusal is the BIND scoping rule: {message}"
    );
    assert_eq!(outcome.invocations, Vec::<String>::new());
    // The same `BIND` over a variable the call does not read is admitted, and runs
    // after the call's own input came from a triple.
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("<{EX}s> <{EX}p> ?q . ?q <{EXPAND}> ?out BIND(\"x\" AS ?later)")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
}

/// A `BIND` whose expression errors on a row leaves `?q` unbound on that row. The
/// plan admits the call — an error is a per-row outcome, not a shape of the text —
/// and the row is then refused at evaluation with the typed access-pattern error: a
/// bound-only relation is never invoked free. The free-capable variant is invoked
/// free on exactly that row.
#[test]
fn a_bind_that_errors_refuses_its_row_and_never_invokes_the_relation_free() {
    let body = format!("BIND(STRLEN(<{EX}not-a-string>) AS ?q) ?q <{EXPAND}> ?out");
    let outcome = run(Variant::BoundOnly, &body);
    let Err(message) = &outcome.answer else {
        panic!("the erroring row must be refused, got {outcome:?}");
    };
    assert!(
        message.contains(&format!(
            "property function <{EXPAND}> cannot serve the invocation `ff`"
        )),
        "the per-row refusal names the relation and the row's access pattern: {message}"
    );
    assert_eq!(outcome.invocations, Vec::<String>::new());
    assert_eq!(
        run(Variant::FreeCapable, &body),
        Outcome {
            answer: Ok(rows(&[("https://example.org/d/free", "free")])),
            invocations: calls(&["ff:-"]),
        }
    );
}

/// A `BIND` that reads a variable the text may leave unbound — here one an
/// `OPTIONAL` binds — inherits that absence and is not a source; the same `BIND`
/// over a required triple is.
#[test]
fn a_bind_over_an_optional_variable_is_not_a_source() {
    assert_refused_at_prepare(&run(
        Variant::BoundOnly,
        &format!("OPTIONAL {{ <{EX}s> <{EX}p> ?v }} BIND(?v AS ?q) ?q <{EXPAND}> ?out"),
    ));
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("<{EX}s> <{EX}p> ?v BIND(?v AS ?q) ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
    // `COALESCE` with a constant fallback is a source even over the optional
    // variable: it answers with its first argument that evaluates.
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!(
                "OPTIONAL {{ <{EX}s> <{EX}nothing> ?v }} BIND(COALESCE(?v, \"alpha\") AS ?q) \
                 ?q <{EXPAND}> ?out"
            )
        ),
        Outcome {
            answer: Ok(rows(&[("alpha", "alpha/1"), ("alpha", "alpha/2")])),
            invocations: calls(&["bf:alpha"]),
        }
    );
}

// ── nested patterns ──────────────────────────────────────────────────────────

/// A sub-`SELECT` that projects the input feeds it; one that does not project it
/// is not a source.
#[test]
fn a_sub_select_projecting_the_input_feeds_it() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!(
                "{{ SELECT ?q WHERE {{ VALUES ?q {{ \"alpha\" \"beta\" }} }} }} ?q <{EXPAND}> ?out"
            )
        ),
        alpha_and_beta()
    );
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("{{ SELECT (\"beta\" AS ?q) WHERE {{}} }} ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
    assert_refused_at_prepare(&run(
        Variant::BoundOnly,
        &format!(
            "{{ SELECT ?other WHERE {{ VALUES (?q ?other) {{ (\"alpha\" 1) }} }} }} \
             ?q <{EXPAND}> ?out"
        ),
    ));
}

/// A nested group that binds the input on every path feeds it; a `UNION` binds it
/// only when both branches do.
#[test]
fn a_nested_group_and_a_union_binding_it_on_every_path_feed_it() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("{{ VALUES ?q {{ \"alpha\" \"beta\" }} }} ?q <{EXPAND}> ?out")
        ),
        alpha_and_beta()
    );
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!(
                "{{ BIND(\"alpha\" AS ?q) }} UNION {{ VALUES ?q {{ \"beta\" }} }} ?q <{EXPAND}> ?out"
            )
        ),
        alpha_and_beta()
    );
    assert_refused_at_prepare(&run(
        Variant::BoundOnly,
        &format!("{{ BIND(\"alpha\" AS ?q) }} UNION {{ BIND(1 AS ?x) }} ?q <{EXPAND}> ?out"),
    ));
}

/// The variable naming a `GRAPH` is bound on every row the `GRAPH` produces.
#[test]
fn a_graph_variable_feeds_it() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("GRAPH ?q {{ <{EX}s> <{EX}p> ?v }} ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[(
                "https://example.org/d/g",
                "https://example.org/d/g/1"
            )])),
            invocations: calls(&["bf:https://example.org/d/g"]),
        }
    );
}

/// A `LATERAL`'s right side is evaluated with each left row in hand, so a `VALUES`
/// on the left feeds a call on the right.
#[test]
fn a_lateral_right_side_sees_its_left_values() {
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("VALUES ?q {{ \"alpha\" \"beta\" }} LATERAL {{ ?q <{EXPAND}> ?out }}")
        ),
        alpha_and_beta()
    );
}

/// The converse: a `LATERAL` written AFTER a call keeps its correlation. Its right
/// side reads the call's `?out`, so it must be driven by the rows the call produced —
/// never re-planned as an independent join operand, which would evaluate it with
/// `?out` unbound and leave `?w` unbound on every row.
#[test]
fn a_lateral_after_a_call_is_driven_by_the_calls_rows() {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        EXPAND.to_owned(),
        Arc::new(Expand::bound_only(Arc::clone(&invocations))),
    );
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    let query = format!(
        "SELECT ?q ?out ?w WHERE {{ <{EX}s> <{EX}p> ?q . ?q <{EXPAND}> ?out \
         LATERAL {{ BIND(CONCAT(?out, \"!\") AS ?w) }} }}"
    );
    let result = NativeSparqlEngine::new()
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
        .unwrap_or_else(|diagnostic| panic!("the query must evaluate: {}", diagnostic.message));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions");
    };
    let rendered: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(|cell| render(cell.as_ref())).collect())
        .collect();
    assert_eq!(
        rendered,
        vec![vec![
            "beta".to_owned(),
            "beta/1".to_owned(),
            "beta/1!".to_owned()
        ]]
    );
    assert_eq!(
        *invocations
            .lock()
            .expect("the fixture recorder is never poisoned"),
        calls(&["bf:beta"])
    );
}

// ── a LATERAL's right side, judged with its left side in hand ────────────────

/// `<a> <p> "alpha"`, `<b> <p> "beta"` and `<c> <p> "epsilon"`: three left rows,
/// each a different value, so an answer names which of them reached the call.
fn lateral_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for (subject, value) in [("a", "alpha"), ("b", "beta"), ("c", "epsilon")] {
        let subject = builder.intern_iri(&format!("{EX}{subject}"));
        let value = builder.intern_literal(RdfLiteral::simple(value));
        builder.push_quad(subject, p, value, None);
    }
    // One named graph, so a call inside `GRAPH ?g { … }` is evaluated once. Its
    // predicate is no other pattern's, and the default graph does not include it.
    let graph = builder.intern_iri(&format!("{EX}g"));
    let marker = builder.intern_iri(&format!("{EX}marker"));
    builder.push_quad(graph, marker, graph, Some(graph));
    builder.freeze().expect("the fixture must validate")
}

/// [`Expand`]'s answer when `"alpha"`, `"beta"` and `"epsilon"` each reach the call
/// bound.
fn every_left_value_bound() -> Outcome {
    Outcome {
        answer: Ok(rows(&[
            ("alpha", "alpha/1"),
            ("alpha", "alpha/2"),
            ("beta", "beta/1"),
            ("epsilon", "epsilon/1"),
        ])),
        invocations: calls(&["bf:alpha", "bf:beta", "bf:epsilon"]),
    }
}

/// The right side of a `LATERAL` is evaluated once per left row with that row
/// substituted in, so a `BIND` there reading a left variable binds its target on
/// every row, exactly as the same `BIND` written without the `LATERAL` does. A
/// bound-only relation fed by it is admitted and invoked bound with each left value;
/// the free-capable variant is invoked bound too, never free.
#[test]
fn a_lateral_bind_over_the_left_side_feeds_the_call() {
    let data = lateral_dataset();
    let written_after = format!("?s <{EX}p> ?v LATERAL {{ BIND(?v AS ?q) }} ?q <{EXPAND}> ?out");
    assert_eq!(
        run_over(&data, Variant::BoundOnly, &written_after),
        every_left_value_bound()
    );
    assert_eq!(
        run_over(&data, Variant::FreeCapable, &written_after),
        every_left_value_bound()
    );
    // The call inside the right side, after the `BIND`, is fed the same way.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!("?s <{EX}p> ?v LATERAL {{ BIND(?v AS ?q) ?q <{EXPAND}> ?out }}")
        ),
        every_left_value_bound()
    );
    // A computed value over the left variable is per row.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ BIND(CONCAT(?v, \"!\") AS ?q) }} ?q <{EXPAND}> ?out"
            )
        ),
        Outcome {
            answer: Ok(rows(&[
                ("alpha!", "alpha!/1"),
                ("beta!", "beta!/1"),
                ("epsilon!", "epsilon!/1"),
            ])),
            invocations: calls(&["bf:alpha!", "bf:beta!", "bf:epsilon!"]),
        }
    );
    // The neighbour the rule is borrowed from: the same `BIND` without the `LATERAL`.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!("?s <{EX}p> ?v BIND(?v AS ?q) ?q <{EXPAND}> ?out")
        ),
        every_left_value_bound()
    );
}

/// A sub-`SELECT` on a `LATERAL`'s right side receives the left row only for the
/// variables it projects. Projecting `?v` carries it in, so a `BIND` over it feeds the
/// call; not projecting it leaves the inner `?v` a different, unbound variable — the
/// bound-only relation is refused at prepare, and the free-capable variant shows why:
/// the call really is reached with its input free.
#[test]
fn a_lateral_sub_select_carries_the_left_row_only_for_what_it_projects() {
    let data = lateral_dataset();
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ SELECT ?v ?q WHERE {{ BIND(?v AS ?q) }} }} \
                 ?q <{EXPAND}> ?out"
            )
        ),
        every_left_value_bound()
    );
    // The call inside the sub-`SELECT` itself, fed by the projected left variable.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ SELECT ?v ?q ?out WHERE {{ BIND(?v AS ?q) \
                 ?q <{EXPAND}> ?out }} }}"
            )
        ),
        every_left_value_bound()
    );
    // A call inside the sub-`SELECT` reading a projected left variable directly: each
    // subject IRI reaches it bound.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ SELECT ?s ?out WHERE {{ ?s <{EXPAND}> ?out }} }} \
                 BIND(?s AS ?q)"
            )
        ),
        Outcome {
            answer: Ok(rows(&[
                ("https://example.org/d/a", "https://example.org/d/a/1"),
                ("https://example.org/d/b", "https://example.org/d/b/1"),
                ("https://example.org/d/c", "https://example.org/d/c/1"),
            ])),
            invocations: calls(&[
                "bf:https://example.org/d/a",
                "bf:https://example.org/d/b",
                "bf:https://example.org/d/c",
            ]),
        }
    );
    assert_refused_at_prepare(&run_over(
        &data,
        Variant::BoundOnly,
        &format!(
            "?s <{EX}p> ?v LATERAL {{ SELECT ?out WHERE {{ ?s <{EXPAND}> ?out }} }} BIND(?s AS ?q)"
        ),
    ));
    let unprojected = format!(
        "?s <{EX}p> ?v LATERAL {{ SELECT ?q WHERE {{ BIND(?v AS ?q) }} }} ?q <{EXPAND}> ?out"
    );
    assert_refused_at_prepare(&run_over(&data, Variant::BoundOnly, &unprojected));
    assert_eq!(
        run_over(&data, Variant::FreeCapable, &unprojected),
        Outcome {
            answer: Ok(rows(&[
                ("https://example.org/d/free", "free"),
                ("https://example.org/d/free", "free"),
                ("https://example.org/d/free", "free"),
            ])),
            invocations: calls(&["ff:-", "ff:-", "ff:-"]),
        }
    );
}

/// A `LATERAL` `BIND` over a variable only an `OPTIONAL` binds inherits that absence
/// wherever the `OPTIONAL` sits — on the left side or inside the right — and is not
/// a source: the bound-only relation is refused at prepare, and the free-capable
/// variant is reached with its input free. `COALESCE` over it with a left variable
/// as the fallback is a source.
#[test]
fn a_lateral_bind_over_an_optional_variable_is_not_a_source() {
    let data = lateral_dataset();
    let optional_on_the_left = format!(
        "?s <{EX}p> ?v OPTIONAL {{ ?s <{EX}nothing> ?w }} LATERAL {{ BIND(?w AS ?q) }} \
         ?q <{EXPAND}> ?out"
    );
    let optional_on_the_right = format!(
        "?s <{EX}p> ?v LATERAL {{ OPTIONAL {{ ?s <{EX}nothing> ?w }} BIND(?w AS ?q) }} \
         ?q <{EXPAND}> ?out"
    );
    let three_free_rows = Outcome {
        answer: Ok(rows(&[
            ("https://example.org/d/free", "free"),
            ("https://example.org/d/free", "free"),
            ("https://example.org/d/free", "free"),
        ])),
        invocations: calls(&["ff:-", "ff:-", "ff:-"]),
    };
    for body in [&optional_on_the_left, &optional_on_the_right] {
        assert_refused_at_prepare(&run_over(&data, Variant::BoundOnly, body));
        assert_eq!(run_over(&data, Variant::FreeCapable, body), three_free_rows);
    }
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ OPTIONAL {{ ?s <{EX}nothing> ?w }} \
                 BIND(COALESCE(?w, ?v) AS ?q) }} ?q <{EXPAND}> ?out"
            )
        ),
        every_left_value_bound()
    );
}

/// A `VALUES` table and a triple on a `LATERAL`'s right side feed the call, the
/// right side correlated with the left through the row it is handed.
#[test]
fn a_lateral_values_and_a_lateral_triple_feed_the_call() {
    let data = lateral_dataset();
    // Each left row keeps the one `VALUES` row equal to its own value; `"epsilon"`
    // has none, so only `"alpha"` and `"beta"` reach the call.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!(
                "?s <{EX}p> ?v LATERAL {{ VALUES ?q {{ \"alpha\" \"beta\" }} FILTER(?q = ?v) }} \
                 ?q <{EXPAND}> ?out"
            )
        ),
        alpha_and_beta()
    );
    // The triple reads the left row's `?s`: each subject yields its own value.
    assert_eq!(
        run_over(
            &data,
            Variant::BoundOnly,
            &format!("?s <{EX}p> ?v LATERAL {{ ?s <{EX}p> ?q }} ?q <{EXPAND}> ?out")
        ),
        every_left_value_bound()
    );
}

// ── the differential against bottom-up evaluation ────────────────────────────

/// The relation the differential calls.
const PAIRS: &str = "https://example.org/rel/pairs";

/// The fixed table [`Pairs`] serves. `"delta"` is a key no left row holds and
/// `"epsilon"` a left value the table has no key for, so a join on either side
/// that ignores the other would show.
const PAIR_TABLE: &[(&str, &str)] = &[
    ("alpha", "alpha/1"),
    ("alpha", "alpha/2"),
    ("beta", "beta/1"),
    ("delta", "delta/1"),
];

/// `?input <pairs> ?output` over a fixed table ([`PAIR_TABLE`] by default), serving `bf` and `ff` CONSISTENTLY:
/// bound, it answers the table's rows for that key; free, the whole table. So a
/// query answered with its input bound must equal the free evaluation joined with
/// the rest of the query afterwards — SPARQL's bottom-up answer.
struct Pairs {
    modes: Vec<BindingPattern>,
    /// The `(input, output)` rows it serves — [`PAIR_TABLE`] unless a test says
    /// otherwise.
    table: Vec<(TermValue, TermValue)>,
    /// Answer the whole table whatever the input, leaving the engine to join the rows
    /// with the call's bound arguments afterwards — the bottom-up reference.
    ignores_input: bool,
    invocations: Arc<Mutex<Vec<String>>>,
}

/// [`PAIR_TABLE`] as the terms [`Pairs`] serves.
fn pair_table() -> Vec<(TermValue, TermValue)> {
    PAIR_TABLE
        .iter()
        .map(|&(input, output)| (simple_literal(input), simple_literal(output)))
        .collect()
}

fn simple_literal(text: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: text.to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
        language: None,
        direction: None,
    }
}

impl PropertyFunction for Pairs {
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
        4
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let key = args.get(0).map(key_of);
        self.invocations
            .lock()
            .expect("the fixture recorder is never poisoned")
            .push(format!(
                "{}:{}",
                args.mode().code(),
                key.as_deref().unwrap_or("-")
            ));
        let mut rows: Vec<PfRow> = Vec::new();
        for (input, output) in &self.table {
            if self.ignores_input || key.as_deref().is_none_or(|key| key == key_of(input)) {
                rows.push(vec![input.clone(), output.clone()]);
            }
        }
        Ok(Box::new(Rows(rows.into_iter())))
    }
}

/// Run `body` against [`Pairs`] declaring `modes`.
fn run_pairs(modes: &[&str], body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = Pairs {
        modes: modes
            .iter()
            .map(|code| BindingPattern::from_code(code))
            .collect(),
        table: pair_table(),
        ignores_input: false,
        invocations: Arc::clone(&invocations),
    };
    run_relation(
        &lateral_dataset(),
        PAIRS,
        Arc::new(relation),
        &invocations,
        body,
    )
}

/// Run `body` against [`Pairs`] declaring `modes` — or, with `ignores_input`, against
/// the bottom-up reference — with `substitutions` bound under `lane`.
fn run_pairs_with(
    modes: &[&str],
    ignores_input: bool,
    body: &str,
    substitutions: &[(String, TermValue)],
    lane: ShaclPrebinding,
) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = Pairs {
        modes: modes
            .iter()
            .map(|code| BindingPattern::from_code(code))
            .collect(),
        table: pair_table(),
        ignores_input,
        invocations: Arc::clone(&invocations),
    };
    run_relation_with(
        &lateral_dataset(),
        PAIRS,
        Arc::new(relation),
        &invocations,
        body,
        substitutions,
        lane,
    )
}

/// Each shape this rule newly admits answers, with its input bound, exactly what
/// SPARQL's bottom-up evaluation answers: the left pattern on its own, joined on
/// `?q` with the relation evaluated FREE on its own. Checked for a relation serving
/// only `bf` and for one serving both, and the reference's own evaluation is checked
/// to have been the free one.
#[test]
fn newly_admitted_lateral_shapes_answer_the_bottom_up_join() {
    let call = format!("?q <{PAIRS}> ?out");
    // (the left pattern alone, the same pattern with the call inside the `LATERAL`'s
    // right side); each is also checked with the call written after the `LATERAL`.
    let lefts_and_insides: [(String, String); 4] = [
        (
            format!("?s <{EX}p> ?v LATERAL {{ BIND(?v AS ?q) }}"),
            format!("?s <{EX}p> ?v LATERAL {{ BIND(?v AS ?q) {call} }}"),
        ),
        (
            format!("?s <{EX}p> ?v LATERAL {{ BIND(CONCAT(\"al\", SUBSTR(?v, 3)) AS ?q) }}"),
            format!("?s <{EX}p> ?v LATERAL {{ BIND(CONCAT(\"al\", SUBSTR(?v, 3)) AS ?q) {call} }}"),
        ),
        (
            format!(
                "?s <{EX}p> ?v LATERAL {{ OPTIONAL {{ ?s <{EX}nothing> ?w }} \
                 BIND(COALESCE(?w, ?v) AS ?q) }}"
            ),
            format!(
                "?s <{EX}p> ?v LATERAL {{ OPTIONAL {{ ?s <{EX}nothing> ?w }} \
                 BIND(COALESCE(?w, ?v) AS ?q) {call} }}"
            ),
        ),
        (
            format!("?s <{EX}p> ?v LATERAL {{ SELECT ?v ?q WHERE {{ BIND(?v AS ?q) }} }}"),
            format!(
                "?s <{EX}p> ?v LATERAL {{ SELECT ?v ?q ?out WHERE {{ BIND(?v AS ?q) {call} }} }}"
            ),
        ),
    ];
    let shapes: Vec<(String, String)> = lefts_and_insides
        .into_iter()
        .flat_map(|(left, inside)| {
            let after = format!("{left} {call}");
            [(left.clone(), after), (left, inside)]
        })
        .collect();

    // The relation evaluated free on its own, once.
    let free = run_pairs(&["bf", "ff"], &call);
    assert_eq!(
        free.invocations,
        calls(&["ff:-"]),
        "the reference is the free one"
    );
    let free_rows = free.answer.expect("the free evaluation answers");

    for (left, full) in &shapes {
        let left_rows = run_pairs(&["bf", "ff"], left)
            .answer
            .unwrap_or_else(|message| panic!("the left pattern {left} answers: {message}"));
        let mut bottom_up: Vec<(String, String)> = left_rows
            .iter()
            .flat_map(|(q, _)| {
                free_rows
                    .iter()
                    .filter(move |(input, _)| input == q)
                    .cloned()
            })
            .collect();
        bottom_up.sort();
        assert!(
            !bottom_up.is_empty(),
            "the shape {full} must have answers to compare"
        );
        for modes in [&["bf"][..], &["bf", "ff"][..]] {
            let outcome = run_pairs(modes, full);
            assert_eq!(
                outcome.answer.as_ref(),
                Ok(&bottom_up),
                "{full} declaring {modes:?} answers the bottom-up join"
            );
            assert!(
                outcome
                    .invocations
                    .iter()
                    .all(|invocation| invocation.starts_with("bf:")),
                "{full} declaring {modes:?} invokes the relation bound only: {:?}",
                outcome.invocations
            );
        }
    }
}

// ── aggregates ───────────────────────────────────────────────────────────────

/// **`SAMPLE`, `MIN` and `MAX` of a variable every row binds, under a `GROUP BY`, feed
/// the call.** A group a `GROUP BY` produces holds at least one row, and each of the
/// three answers a value over a non-empty list — so the output is bound on every group
/// row, and a bound-only relation fed by it is admitted and invoked bound. It used to
/// be refused as though every aggregate's output could be unbound. The free-capable
/// variant answers the same, invoked bound.
#[test]
fn an_aggregate_over_a_group_by_feeds_the_call() {
    let call = format!("?q <{EXPAND}> ?out");
    for aggregate in ["SAMPLE", "MIN", "MAX"] {
        let body = format!(
            "{{ SELECT ?s ({aggregate}(?v) AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}"
        );
        let expected = Outcome {
            answer: Ok(rows(&[("beta", "beta/1")])),
            invocations: calls(&["bf:beta"]),
        };
        assert_eq!(run(Variant::BoundOnly, &body), expected, "{body}");
        assert_eq!(run(Variant::FreeCapable, &body), expected, "{body}");
    }
    // The shape a correlated per-group lookup takes: a `LATERAL` sub-`SELECT` picks one
    // value per group, and a second `LATERAL` sub-`SELECT` hands it to the call.
    let lateral = format!(
        "?a <{EX}p> ?w LATERAL {{ SELECT ?y (SAMPLE(?nn) AS ?q) WHERE {{ ?y <{EX}p> ?nn }} \
         GROUP BY ?y }} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} }}"
    );
    let expected = Outcome {
        answer: Ok(rows(&[("beta", "beta/1")])),
        invocations: calls(&["bf:beta"]),
    };
    assert_eq!(run(Variant::BoundOnly, &lateral), expected, "{lateral}");
    assert_eq!(run(Variant::FreeCapable, &lateral), expected, "{lateral}");
}

/// **Without a `GROUP BY`, `SAMPLE`, `MIN`, `MAX` and a custom aggregate are not a
/// source; `COUNT`, `SUM`, `AVG`, `GROUP_CONCAT` and `FOLD` are.**
///
/// An aggregate with no `GROUP BY` answers one row even over an empty input. The
/// first four may answer unbound there for want of any value — a structural absence,
/// like an `OPTIONAL`'s — so over `?s <nothing> ?v`, which matches nothing, the
/// bound-only relation is refused at prepare and the free-capable one is observed
/// invoked FREE. The last five answer a value over no values — `0`, `0`, `0`, `""` and
/// the empty list — so each feeds the call, invoked bound with exactly that value.
#[test]
fn an_aggregate_without_group_by_over_a_possibly_empty_input() {
    let call = format!("?q <{EXPAND}> ?out");
    let free = Outcome {
        answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
        invocations: calls(&["ff:-"]),
    };
    for aggregate in [
        "SAMPLE(?v)".to_owned(),
        "MIN(?v)".to_owned(),
        "MAX(?v)".to_owned(),
        format!("AGG(<{FIRST_LITERAL}>, ?v)"),
    ] {
        let body =
            format!("{{ SELECT ({aggregate} AS ?q) WHERE {{ ?s <{EX}nothing> ?v }} }} {call}");
        assert_refused_at_prepare(&run_aggregating(&dataset(), Variant::BoundOnly, &body));
        assert_eq!(
            run_aggregating(&dataset(), Variant::FreeCapable, &body),
            free,
            "{body}: the one implicit group's {aggregate} is unbound"
        );
    }
    for (aggregate, value) in [
        ("COUNT(?v)", "0"),
        ("SUM(?v)", "0"),
        ("AVG(?v)", "0"),
        ("GROUP_CONCAT(?v)", ""),
        ("FOLD(?v)", "[]"),
    ] {
        let body =
            format!("{{ SELECT ({aggregate} AS ?q) WHERE {{ ?s <{EX}nothing> ?v }} }} {call}");
        let expected = Outcome {
            answer: Ok(rows(&[(value, &format!("{value}/1"))])),
            invocations: calls(&[&format!("bf:{value}")]),
        };
        assert_eq!(run(Variant::BoundOnly, &body), expected, "{body}");
        assert_eq!(run(Variant::FreeCapable, &body), expected, "{body}");
    }
}

/// **An aggregate whose every value may be absent is not a source, and neither is a
/// grouping key some row leaves unbound — exactly as a `BIND` over the same variable
/// is not.**
///
/// `GROUP BY ?q` over an `OPTIONAL` that matched nothing forms a group whose key is
/// unbound; `SAMPLE(?w)` over an `OPTIONAL`'s `?w` has no value in a group none of
/// whose rows binds it, and neither has `SAMPLE(?y)` after `BIND(?w AS ?y)`. Each is
/// refused for the bound-only relation, and the free-capable variant is observed
/// invoked free on exactly those rows. The neighbour: `SUM(?w)` over the same rows
/// answers `0` over no values, so it feeds the call, invoked bound.
#[test]
fn a_key_or_an_aggregate_over_a_variable_the_text_may_leave_unbound_is_not_a_source() {
    let call = format!("?q <{EXPAND}> ?out");
    let free = Outcome {
        answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
        invocations: calls(&["ff:-"]),
    };
    let optional = format!("?s <{EX}p> ?v OPTIONAL {{ ?s <{EX}nothing> ?w }}");
    for body in [
        format!(
            "{{ SELECT ?q WHERE {{ ?s <{EX}p> ?v OPTIONAL {{ ?s <{EX}nothing> ?q }} }} \
             GROUP BY ?q }} {call}"
        ),
        format!("{{ SELECT ?s (SAMPLE(?w) AS ?q) WHERE {{ {optional} }} GROUP BY ?s }} {call}"),
        format!(
            "{{ SELECT ?s (SAMPLE(?y) AS ?q) WHERE {{ {optional} BIND(?w AS ?y) }} \
             GROUP BY ?s }} {call}"
        ),
    ] {
        assert_refused_at_prepare(&run(Variant::BoundOnly, &body));
        assert_eq!(run(Variant::FreeCapable, &body), free, "{body}");
    }
    let sum = format!("{{ SELECT ?s (SUM(?w) AS ?q) WHERE {{ {optional} }} GROUP BY ?s }} {call}");
    let zero = Outcome {
        answer: Ok(rows(&[("0", "0/1")])),
        invocations: calls(&["bf:0"]),
    };
    assert_eq!(run(Variant::BoundOnly, &sum), zero, "{sum}");
    assert_eq!(run(Variant::FreeCapable, &sum), zero, "{sum}");
}

/// A dataset whose one `?s <p> ?v` row has a blank-node object — the input on which
/// `STR` errors.
fn blank_object_data() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let b = builder.intern_blank("b0", purrdf_core::BlankScope::DEFAULT);
    builder.push_quad(s, p, b, None);
    builder.freeze().expect("the fixture must validate")
}

/// **A `FILTER` binds every variable its condition requires bound for it to be true.**
///
/// After `OPTIONAL { ?s <p> ?q }`, `?q` may be unbound, so a `GROUP BY ?q` key or a
/// sub-`SELECT`'s `?q` over it used to be no source. A `FILTER` on `BOUND(?q)`, on
/// `sameTerm(?q, ?o)` with `?o` bound, on `isLiteral(?q)`, or on a conjunction holding
/// one of them passes only rows that bind `?q`, so each feeds the call: the bound-only
/// relation is admitted and invoked bound, and the free-capable one answers the same,
/// invoked bound only.
#[test]
fn a_filter_binds_what_its_condition_requires() {
    let call = format!("?q <{EXPAND}> ?out");
    let optional = format!("?s <{EX}p> ?o OPTIONAL {{ ?s <{EX}p> ?q }}");
    let expected = Outcome {
        answer: Ok(rows(&[("beta", "beta/1")])),
        invocations: calls(&["bf:beta"]),
    };
    for condition in [
        "BOUND(?q)",
        "sameTerm(?q, ?o)",
        "isLiteral(?q)",
        "BOUND(?q) && ?o != \"zzz\"",
        "!(!BOUND(?q))",
        "BOUND(?q) || sameTerm(?q, ?o)",
        // A built-in strict in the argument reading `?q` has no value without it.
        "REGEX(STR(?q), \".\")",
        "STRLEN(STR(?q)) > 0",
        "STRLEN(CONCAT(STR(?q))) >= 0",
        "-(STRLEN(STR(?q))) <= 0",
        "IF(BOUND(?q), ?q = ?q, false)",
    ] {
        for body in [
            format!(
                "{{ SELECT ?q WHERE {{ {optional} FILTER({condition}) }} GROUP BY ?q }} {call}"
            ),
            format!("{{ SELECT ?q WHERE {{ {optional} FILTER({condition}) }} }} {call}"),
            format!("{{ {optional} FILTER({condition}) }} {call}"),
        ] {
            assert_eq!(run(Variant::BoundOnly, &body), expected, "{body}");
            assert_eq!(run(Variant::FreeCapable, &body), expected, "{body}");
        }
    }
}

/// **A `FILTER` binds nothing its condition can be true without.** `!BOUND(?q)` is true
/// exactly when `?q` is unbound; `BOUND(?q) || BOUND(?w)` is true on a row binding only
/// `?w`; `COALESCE(STR(?q), "x") != ""` falls back to `"x"` without `?q`; and
/// `IF(BOUND(?q), true, true)` is true whichever way its condition goes. A custom
/// function over `?q` is one the host decides — the one registered here has an
/// optional parameter and answers `true` without it — so it binds nothing either. Each
/// is refused for the bound-only relation, and the free-capable one is observed invoked
/// free on the row the condition let through.
#[test]
fn a_filter_binds_nothing_its_condition_can_be_true_without() {
    let call = format!("?q <{EXPAND}> ?out");
    let free = Outcome {
        answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
        invocations: calls(&["ff:-"]),
    };
    let unbound = format!("?s <{EX}p> ?o OPTIONAL {{ ?s <{EX}nothing> ?q }}");
    let mut bodies = vec![format!(
        "{{ SELECT ?q WHERE {{ {unbound} OPTIONAL {{ ?s <{EX}p> ?w }} \
         FILTER(BOUND(?q) || BOUND(?w)) }} }} {call}"
    )];
    for condition in [
        "!BOUND(?q)",
        "COALESCE(STR(?q), \"x\") != \"\"",
        "IF(BOUND(?q), true, true)",
    ] {
        bodies.push(format!(
            "{{ SELECT ?q WHERE {{ {unbound} FILTER({condition}) }} }} {call}"
        ));
        bodies.push(format!(
            "{{ SELECT ?q WHERE {{ {unbound} FILTER({condition}) }} GROUP BY ?q }} {call}"
        ));
        bodies.push(format!("{{ {unbound} FILTER({condition}) }} {call}"));
    }
    for body in &bodies {
        assert_refused_at_prepare(&run(Variant::BoundOnly, body));
        assert_eq!(run(Variant::FreeCapable, body), free, "{body}");
    }

    let custom = format!("{{ SELECT ?q WHERE {{ {unbound} FILTER(<{HOST_FN}>(?q)) }} }} {call}");
    assert_refused_at_prepare(&run_with_host_function(Variant::BoundOnly, &custom));
    assert_eq!(
        run_with_host_function(Variant::FreeCapable, &custom),
        free,
        "{custom}"
    );
}

/// A host function IRI — configuration, never minted vocabulary.
const HOST_FN: &str = "https://example.org/fn/host";

/// [`run`], with [`HOST_FN`] registered as a SPARQL-bodied function whose one parameter
/// is optional and whose body is `ASK {}`: `true`, whether or not its argument is bound.
fn run_with_host_function(variant: Variant, body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation: Arc<dyn PropertyFunction> = match variant {
        Variant::BoundOnly => Arc::new(Expand::bound_only(Arc::clone(&invocations))),
        Variant::FreeCapable => Arc::new(Expand::free_capable(Arc::clone(&invocations))),
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(EXPAND.to_owned(), relation);
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    let mut functions = UserFunctionRegistry::new();
    functions.insert(
        HOST_FN,
        UserFunction {
            params: vec![UserFnParam {
                var: "value".to_owned(),
                constraint: TypeConstraint::default(),
            }],
            required: 0,
            body: Arc::from("ASK {}"),
            kind: UserFnBody::Ask,
            return_constraint: TypeConstraint::default(),
        },
    );
    let engine = NativeSparqlEngine::new();
    let bound = engine
        .bind_functions(functions, &env)
        .expect("the fixture body parses");
    let query = format!("SELECT ?q ?out WHERE {{ {body} }}");
    let answer = engine
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                env: &env,
                functions: &bound,
                ..QueryOptions::EMPTY
            },
        )
        .map(|result| {
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("a SELECT answers with solutions");
            };
            let mut rendered: Vec<(String, String)> = rows
                .iter()
                .map(|row| (render(row[0].as_ref()), render(row[1].as_ref())))
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

/// **A `FILTER` over a built-in strict in `?q` drops the row that leaves `?q` unbound,
/// and so binds `?q` in every row it passes.** Over `<a> "alpha"`, `<b> "beta"` and
/// `<c> "epsilon"`, the `OPTIONAL` binds `?q` on `<a>`'s and `<c>`'s rows and leaves it
/// unbound on `<b>`'s. Each condition below has no value on `<b>`'s row — `STR` of an
/// unbound argument has none, nor has anything strict over it — so the call sees
/// `"alpha"` and `"epsilon"` only: the bound-only relation is admitted and invoked
/// bound with exactly those, and the free-capable one answers the same and is never
/// invoked free. `!BOUND(?q)`, the neighbour that passes exactly `<b>`'s row, is
/// refused for the bound-only relation and invokes the free-capable one free.
#[test]
fn a_filter_over_a_strict_built_in_drops_the_unbound_row() {
    let call = format!("?q <{EXPAND}> ?out");
    let some = format!("?s <{EX}p> ?o OPTIONAL {{ ?s <{EX}p> ?q FILTER(?q != \"beta\") }}");
    let expected = Outcome {
        answer: Ok(rows(&[
            ("alpha", "alpha/1"),
            ("alpha", "alpha/2"),
            ("epsilon", "epsilon/1"),
        ])),
        invocations: calls(&["bf:alpha", "bf:epsilon"]),
    };
    for condition in [
        "REGEX(STR(?q), \".\")",
        "STRLEN(STR(?q)) > 0",
        "IF(BOUND(?q), ?q = ?q, false)",
        "-(STRLEN(STR(?q))) <= 0",
        "STRLEN(CONCAT(STR(?q))) >= 0",
    ] {
        for body in [
            format!("{{ SELECT ?q WHERE {{ {some} FILTER({condition}) }} GROUP BY ?q }} {call}"),
            format!("{{ SELECT ?q WHERE {{ {some} FILTER({condition}) }} }} {call}"),
            format!("{{ {some} FILTER({condition}) }} {call}"),
        ] {
            for variant in [Variant::BoundOnly, Variant::FreeCapable] {
                assert_eq!(
                    run_over(&lateral_dataset(), variant, &body),
                    expected,
                    "{body}"
                );
            }
        }
    }
    let neighbour = format!("{{ {some} FILTER(!BOUND(?q)) }} {call}");
    assert_refused_at_prepare(&run_over(
        &lateral_dataset(),
        Variant::BoundOnly,
        &neighbour,
    ));
    assert_eq!(
        run_over(&lateral_dataset(), Variant::FreeCapable, &neighbour),
        Outcome {
            answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
            invocations: calls(&["ff:-"]),
        },
        "{neighbour}"
    );
}

/// Assert `outcome` is the evaluator's per-row refusal of `iri`, a relation serving
/// only `bf`: a row reached the call with its input unbound, and the relation was never
/// invoked free — every invocation it did see was bound.
fn assert_refused_per_row(outcome: &Outcome, iri: &str) {
    let Err(message) = &outcome.answer else {
        panic!("the row whose input is unbound must be refused, got {outcome:?}");
    };
    assert!(
        message.contains(&format!(
            "property function <{iri}> cannot serve the invocation `ff`; it declares [bf]"
        )),
        "the per-row refusal names the relation and the row's access pattern: {message}"
    );
    assert!(
        outcome
            .invocations
            .iter()
            .all(|invocation| invocation.starts_with("bf:")),
        "a bound-only relation is never invoked free: {:?}",
        outcome.invocations
    );
}

/// **`SAMPLE`, `MIN` and `MAX` of an expression reading only what every row binds feed
/// the call, whether or not the expression can error — exactly as the same expression
/// `BIND` before the aggregate does.**
///
/// Over `"beta"` every form below binds, so the bound-only relation is admitted and
/// invoked bound with `"beta"`. `STR` errors on a blank node, and `IRI(STR(?v))` with
/// it, so over the blank-node object `MAX(STR(?v))` and `SAMPLE(IRI(STR(?v)))` are
/// unbound in that group — a failure on a present value, met per row: the bound-only
/// relation is refused with the evaluator's typed access-pattern error and never
/// invoked, and the free-capable one is invoked free on that row. Each form's `BIND`
/// chain — `BIND(STR(?v) AS ?y)` then `MAX(?y)` — does exactly the same on both data.
#[test]
fn an_aggregate_over_an_expression_reading_only_bound_variables_is_a_source() {
    let call = format!("?q <{EXPAND}> ?out");
    let beta = Outcome {
        answer: Ok(rows(&[("beta", "beta/1")])),
        invocations: calls(&["bf:beta"]),
    };
    let grouped = |aggregate: &str, pattern: &str| {
        format!("{{ SELECT ?s ({aggregate} AS ?q) WHERE {{ {pattern} }} GROUP BY ?s }} {call}")
    };
    let triple = format!("?s <{EX}p> ?v");
    for aggregate in [
        "SAMPLE(COALESCE(STR(?v), ?v))",
        "MAX(IF(isLiteral(?v), ?v, \"none\"))",
        "MIN(COALESCE(IRI(STR(?v)), ?v))",
        "MAX(STR(?v))",
        "MIN(STR(?v))",
        "SAMPLE(STR(?v))",
    ] {
        let body = grouped(aggregate, &triple);
        assert_eq!(run(Variant::BoundOnly, &body), beta, "{body}");
        assert_eq!(run(Variant::FreeCapable, &body), beta, "{body}");
    }

    let free = Outcome {
        answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
        invocations: calls(&["ff:-"]),
    };
    let blank = blank_object_data();
    for (aggregate, bind, over_bound) in [
        ("MAX(STR(?v))", "BIND(STR(?v) AS ?y)", "MAX(?y)"),
        (
            "SAMPLE(IRI(STR(?v)))",
            "BIND(IRI(STR(?v)) AS ?y)",
            "SAMPLE(?y)",
        ),
    ] {
        let direct = grouped(aggregate, &triple);
        let chain = grouped(over_bound, &format!("{triple} {bind}"));
        for body in [&direct, &chain] {
            assert_refused_per_row(&run_over(&blank, Variant::BoundOnly, body), EXPAND);
            assert_eq!(
                run_over(&blank, Variant::FreeCapable, body),
                free,
                "{body}: the group's every argument errored, so the aggregate is unbound"
            );
        }
        for data in [&dataset(), &blank] {
            for variant in [Variant::BoundOnly, Variant::FreeCapable] {
                assert_eq!(
                    run_over(data, variant, &direct),
                    run_over(data, variant, &chain),
                    "{direct} behaves as its BIND chain {chain}"
                );
            }
        }
    }
}

/// The custom aggregate the aggregate differential registers — host configuration,
/// never minted vocabulary.
const FIRST_LITERAL: &str = "https://example.org/agg/first-literal";

/// A host aggregate answering its group's first value when that value is a literal, and
/// declining — unbound — when it is not, or when it was handed no value at all.
struct FirstLiteral;

/// [`FirstLiteral`]'s state: whether a value has arrived, and that first value if it is
/// a literal.
struct FirstLiteralState {
    seen: bool,
    first: Option<TermValue>,
}

impl AggregateAccumulator for FirstLiteralState {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if !self.seen {
            self.seen = true;
            self.first = args
                .first()
                .filter(|value| matches!(value, TermValue::Literal { .. }))
                .cloned();
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let other = other
            .into_any()
            .downcast::<Self>()
            .map_err(|_| EvalError::internal("a first-literal state combined with another"))?;
        if !self.seen {
            self.seen = other.seen;
            self.first = other.first;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.first)
    }
}

impl CustomAggregate for FirstLiteral {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(FirstLiteralState {
            seen: false,
            first: None,
        })
    }
}

/// The environment `relation`, registered at `iri`, runs in beside [`FirstLiteral`].
fn aggregating_env(iri: &str, relation: Arc<dyn PropertyFunction>) -> ExtensionEnv {
    let mut aggregates = AggregateRegistry::new();
    aggregates.register(FIRST_LITERAL, Arc::new(FirstLiteral));
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(iri.to_owned(), relation);
    ExtensionEnv::over_aggregates(aggregates)
        .and_then(|env| env.with_relations(relations))
        .expect("the fixture declarations read")
}

/// [`run_over`], with [`FirstLiteral`] registered.
fn run_aggregating(data: &RdfDataset, variant: Variant, body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = match variant {
        Variant::BoundOnly => Expand::bound_only(Arc::clone(&invocations)),
        Variant::FreeCapable => Expand::free_capable(Arc::clone(&invocations)),
    };
    let env = aggregating_env(EXPAND, Arc::new(relation));
    run_in(&env, data, &invocations, body, &[], ShaclPrebinding::None)
}

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

/// Two subjects, `<a>` and `<b>`, each with a string (`<p>`), an IRI (`<r>`) and
/// integers (`<n>`): `"alpha"`, `<x>`, `1` and `2` for `<a>`; `"beta"`, `<y>` and `4`
/// for `<b>`. `poisoned` adds `<c>`, whose `<p>` and `<r>` are a blank node — on which
/// `STR` errors and `GROUP_CONCAT` poisons — and whose `<n>` is the string `"zzz"`, on
/// which `SUM` and `AVG` poison.
fn aggregate_data(poisoned: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let r = builder.intern_iri(&format!("{EX}r"));
    let n = builder.intern_iri(&format!("{EX}n"));
    for (subject, text, iri, numbers) in [
        ("a", "alpha", "x", &["1", "2"][..]),
        ("b", "beta", "y", &["4"][..]),
    ] {
        let subject = builder.intern_iri(&format!("{EX}{subject}"));
        let text = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, p, text, None);
        let iri = builder.intern_iri(&format!("{EX}{iri}"));
        builder.push_quad(subject, r, iri, None);
        for number in numbers {
            let number = builder.intern_literal(RdfLiteral::typed(*number, XSD_INTEGER));
            builder.push_quad(subject, n, number, None);
        }
    }
    if poisoned {
        let c = builder.intern_iri(&format!("{EX}c"));
        let blank = builder.intern_blank("b0", purrdf_core::BlankScope::DEFAULT);
        builder.push_quad(c, p, blank, None);
        builder.push_quad(c, r, blank, None);
        let zzz = builder.intern_literal(RdfLiteral::simple("zzz"));
        builder.push_quad(c, n, zzz, None);
    }
    builder.freeze().expect("the fixture must validate")
}

/// The table the aggregate differential's [`Pairs`] serves: a row for every value an
/// aggregate below answers over the clean data, and `"delta"`, which none answers.
fn aggregate_table() -> Vec<(TermValue, TermValue)> {
    let x = format!("{EX}x");
    let y = format!("{EX}y");
    [
        (simple_literal("alpha"), "alpha/1"),
        (simple_literal("alpha"), "alpha/2"),
        (simple_literal("beta"), "beta/1"),
        (TermValue::iri(x), "x/1"),
        (TermValue::iri(y), "y/1"),
        (TermValue::typed_literal("3", XSD_INTEGER), "3/1"),
        (TermValue::typed_literal("4", XSD_INTEGER), "4/1"),
        (TermValue::typed_literal("1.5", XSD_DECIMAL), "1.5/1"),
        (TermValue::typed_literal("4", XSD_DECIMAL), "4.0/1"),
        (simple_literal("delta"), "delta/1"),
    ]
    .into_iter()
    .map(|(input, output)| (input, simple_literal(output)))
    .collect()
}

/// Run `body` over `data` against [`Pairs`] serving [`aggregate_table`] in `modes` —
/// or, with `ignores_input`, against the bottom-up reference — beside [`FirstLiteral`].
fn run_table(data: &RdfDataset, modes: &[&str], ignores_input: bool, body: &str) -> Outcome {
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = Pairs {
        modes: modes
            .iter()
            .map(|code| BindingPattern::from_code(code))
            .collect(),
        table: aggregate_table(),
        ignores_input,
        invocations: Arc::clone(&invocations),
    };
    let env = aggregating_env(PAIRS, Arc::new(relation));
    run_in(&env, data, &invocations, body, &[], ShaclPrebinding::None)
}

/// **Every aggregate whose output can be unbound only by failing on present values
/// feeds the call, and a group where it failed is refused per row — never invoked
/// free.**
///
/// `MIN`/`MAX` of `STR(?v)`, `SAMPLE(IRI(STR(?v)))`, `SUM`, `AVG`, `GROUP_CONCAT`, a
/// custom aggregate, and the `BIND` chains of the first forms, each under `GROUP BY`:
///
/// * over the clean data every group binds, and the answer is exactly the listed rows
///   — the bottom-up reference's too — for a relation serving only `bf` and for one
///   serving both, invoked bound with exactly the listed values;
/// * over the poisoned data the `<c>` group's aggregate is unbound. The bound-only
///   relation is refused with the evaluator's typed per-row error and never invoked
///   free; the free-capable one is invoked bound on the other groups and free on
///   `<c>`'s row, answering the listed rows plus the whole table (the unbound `?q`
///   joins every row) — exactly the bottom-up reference.
#[test]
fn aggregates_failing_on_present_values_answer_the_bottom_up_join_and_never_invoke_free() {
    let call = format!("?q <{PAIRS}> ?out");
    let text = format!("?s <{EX}p> ?v");
    let iri = format!("?s <{EX}r> ?v");
    let number = format!("?s <{EX}n> ?v");
    let strings: (&[(&str, &str)], &[&str]) = (
        &[
            ("alpha", "alpha/1"),
            ("alpha", "alpha/2"),
            ("beta", "beta/1"),
        ],
        &["bf:alpha", "bf:beta"],
    );
    let iris: (&[(&str, &str)], &[&str]) = (
        &[
            ("https://example.org/d/x", "x/1"),
            ("https://example.org/d/y", "y/1"),
        ],
        &["bf:https://example.org/d/x", "bf:https://example.org/d/y"],
    );
    let sums: (&[(&str, &str)], &[&str]) = (&[("3", "3/1"), ("4", "4/1")], &["bf:3", "bf:4"]);
    let averages: (&[(&str, &str)], &[&str]) =
        (&[("1.5", "1.5/1"), ("4", "4.0/1")], &["bf:1.5", "bf:4"]);
    let first_literal = format!("AGG(<{FIRST_LITERAL}>, ?v)");
    let forms: [(&str, String, _); 9] = [
        ("MAX(STR(?v))", text.clone(), strings),
        ("MIN(STR(?v))", text.clone(), strings),
        ("MAX(?y)", format!("{text} BIND(STR(?v) AS ?y)"), strings),
        ("GROUP_CONCAT(?v)", text.clone(), strings),
        (&first_literal, text, strings),
        ("SAMPLE(IRI(STR(?v)))", iri.clone(), iris),
        (
            "SAMPLE(?y)",
            format!("{iri} BIND(IRI(STR(?v)) AS ?y)"),
            iris,
        ),
        ("SUM(?v)", number.clone(), sums),
        ("AVG(?v)", number, averages),
    ];
    let whole_table: Vec<(String, String)> = aggregate_table()
        .iter()
        .map(|(input, output)| (key_of(input), key_of(output)))
        .collect();
    let (clean, poisoned) = (aggregate_data(false), aggregate_data(true));
    for (aggregate, pattern, (listed, invoked)) in forms {
        let body =
            format!("{{ SELECT ?s ({aggregate} AS ?q) WHERE {{ {pattern} }} GROUP BY ?s }} {call}");

        let expected = rows(listed);
        let reference = run_table(&clean, &["ff"], true, &body);
        assert_eq!(
            reference.answer.as_ref(),
            Ok(&expected),
            "{body}: the reference"
        );
        for modes in [&["bf"][..], &["bf", "ff"][..]] {
            assert_eq!(
                run_table(&clean, modes, false, &body),
                Outcome {
                    answer: Ok(expected.clone()),
                    invocations: calls(invoked),
                },
                "{body} declaring {modes:?} over the clean data"
            );
        }

        let mut with_free_row = expected;
        with_free_row.extend(whole_table.iter().cloned());
        with_free_row.sort();
        let reference = run_table(&poisoned, &["ff"], true, &body);
        assert_eq!(
            reference.answer.as_ref(),
            Ok(&with_free_row),
            "{body}: the reference over the poisoned data"
        );
        assert_refused_per_row(&run_table(&poisoned, &["bf"], false, &body), PAIRS);
        let mut invoked_free = calls(invoked);
        invoked_free.push("ff:-".to_owned());
        invoked_free.sort();
        assert_eq!(
            run_table(&poisoned, &["bf", "ff"], false, &body),
            Outcome {
                answer: Ok(with_free_row),
                invocations: invoked_free,
            },
            "{body} declaring both modes over the poisoned data"
        );
    }
}

/// One differential case: the query body, its substitutions, the rewrite they are
/// bound under, and the exact answer.
type Shape = (
    String,
    Vec<(String, TermValue)>,
    ShaclPrebinding,
    Vec<(String, String)>,
);

/// Every shape this change newly admits answers, with its input bound, exactly what
/// SPARQL's bottom-up evaluation answers — the relation's whole table joined with the
/// rest of the query afterwards ([`Pairs`] built to ignore its input) — and exactly the
/// rows listed, for a relation serving only `bf` and for one serving both, invoked
/// bound only.
#[test]
fn newly_admitted_substituted_and_aggregate_shapes_answer_the_bottom_up_join() {
    let call = format!("?q <{PAIRS}> ?out");
    let alpha = || vec![("q".to_owned(), simple_literal("alpha"))];
    let none = Vec::new;
    let each_left_row = |out: &[&str]| -> Vec<(String, String)> {
        let mut expected: Vec<(String, String)> = (0..3)
            .flat_map(|_| out.iter().map(|&out| ("alpha".to_owned(), out.to_owned())))
            .collect();
        expected.sort();
        expected
    };
    let shapes: Vec<Shape> = vec![
        (
            format!("?s <{EX}p> ?v LATERAL {{ {call} }}"),
            alpha(),
            ShaclPrebinding::None,
            each_left_row(&["alpha/1", "alpha/2"]),
        ),
        (
            format!("?s <{EX}p> ?v LATERAL {{ {call} }}"),
            alpha(),
            ShaclPrebinding::Applied,
            each_left_row(&["alpha/1", "alpha/2"]),
        ),
        (
            format!("?s <{EX}p> ?v LATERAL {{ LATERAL {{ {call} }} }}"),
            alpha(),
            ShaclPrebinding::None,
            each_left_row(&["alpha/1", "alpha/2"]),
        ),
        (
            format!("BIND(?q AS ?x) ?x <{PAIRS}> ?out"),
            alpha(),
            ShaclPrebinding::Applied,
            rows(&[("alpha", "alpha/1"), ("alpha", "alpha/2")]),
        ),
        (
            format!("?s <{EX}p> ?v BIND(?q AS ?x) FILTER EXISTS {{ ?x <{PAIRS}> ?out }}"),
            alpha(),
            ShaclPrebinding::None,
            each_left_row(&["UNBOUND"]),
        ),
        (
            format!(
                "{{ SELECT ?s (SAMPLE(?v) AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}"
            ),
            none(),
            ShaclPrebinding::None,
            rows(&[
                ("alpha", "alpha/1"),
                ("alpha", "alpha/2"),
                ("beta", "beta/1"),
            ]),
        ),
        (
            format!("{{ SELECT (MIN(?v) AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}"),
            none(),
            ShaclPrebinding::None,
            rows(&[
                ("alpha", "alpha/1"),
                ("alpha", "alpha/2"),
                ("beta", "beta/1"),
            ]),
        ),
        (
            format!(
                "?a <{EX}p> ?w LATERAL {{ SELECT ?y (MAX(?nn) AS ?q) WHERE {{ ?y <{EX}p> ?nn }} \
                 GROUP BY ?y }} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} }}"
            ),
            none(),
            ShaclPrebinding::None,
            rows(&[
                ("alpha", "alpha/1"),
                ("alpha", "alpha/1"),
                ("alpha", "alpha/1"),
                ("alpha", "alpha/2"),
                ("alpha", "alpha/2"),
                ("alpha", "alpha/2"),
                ("beta", "beta/1"),
                ("beta", "beta/1"),
                ("beta", "beta/1"),
            ]),
        ),
    ];
    // A `LATERAL` right side the ordinary rewrite writes the substitution into beyond a
    // bare call, each answering every left row joined with its rows for "alpha" — as
    // many times over as the right side repeats them.
    let times = |copies: usize| -> Vec<(String, String)> {
        let mut expected: Vec<(String, String)> = (0..3 * copies)
            .flat_map(|_| {
                ["alpha/1", "alpha/2"]
                    .into_iter()
                    .map(|out| ("alpha".to_owned(), out.to_owned()))
            })
            .collect();
        expected.sort();
        expected
    };
    let other = format!("?s2 <{EX}p> ?v2");
    let lateral_sides: [(String, usize); 10] = [
        (format!("BIND(1 AS ?one) {call}"), 1),
        (format!("VALUES ?z {{ 1 }} {call}"), 1),
        (format!("{other} . {call}"), 3),
        (format!("{call} FILTER(BOUND(?out))"), 1),
        (format!("{other} LATERAL {{ {call} }}"), 3),
        (format!("SELECT ?q ?out WHERE {{ {call} }}"), 1),
        (format!("SELECT * WHERE {{ {call} }}"), 1),
        (
            format!("SELECT DISTINCT ?q ?out WHERE {{ {call} }} ORDER BY ?out"),
            1,
        ),
        (format!("{{ {call} }} UNION {{ {call} }}"), 2),
        (format!("GRAPH ?g {{ {call} }}"), 1),
    ];
    let mut shapes = shapes;
    for (right, copies) in lateral_sides {
        shapes.push((
            format!("?s <{EX}p> ?v LATERAL {{ {right} }}"),
            alpha(),
            ShaclPrebinding::None,
            times(copies),
        ));
    }
    // A key a `FILTER` makes certain, and an aggregate over a non-erroring argument.
    let every_value = || {
        rows(&[
            ("alpha", "alpha/1"),
            ("alpha", "alpha/2"),
            ("beta", "beta/1"),
        ])
    };
    let optional = format!("?s <{EX}p> ?o OPTIONAL {{ ?s <{EX}p> ?q }}");
    for condition in ["BOUND(?q)", "sameTerm(?q, ?o)"] {
        shapes.push((
            format!(
                "{{ SELECT ?q WHERE {{ {optional} FILTER({condition}) }} GROUP BY ?q }} {call}"
            ),
            none(),
            ShaclPrebinding::None,
            every_value(),
        ));
    }
    for aggregate in [
        "SAMPLE(COALESCE(STR(?v), ?v))",
        "MAX(IF(isLiteral(?v), ?v, \"none\"))",
    ] {
        shapes.push((
            format!(
                "{{ SELECT ?s ({aggregate} AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}"
            ),
            none(),
            ShaclPrebinding::None,
            every_value(),
        ));
    }
    // A `GROUP BY` the substituted variable is a key of, which the rewrite enters.
    for (body, expected) in group_key_shapes() {
        shapes.push((body, alpha(), ShaclPrebinding::None, expected));
    }
    // A `FILTER` over a built-in strict in the variable it reads, which drops `<b>`'s
    // row, where the `OPTIONAL` leaves `?q` unbound.
    let some = format!("?s <{EX}p> ?o OPTIONAL {{ ?s <{EX}p> ?q FILTER(?q != \"beta\") }}");
    for condition in [
        "REGEX(STR(?q), \".\")",
        "STRLEN(STR(?q)) > 0",
        "IF(BOUND(?q), ?q = ?q, false)",
        "-(STRLEN(STR(?q))) <= 0",
    ] {
        for body in [
            format!("{{ SELECT ?q WHERE {{ {some} FILTER({condition}) }} GROUP BY ?q }} {call}"),
            format!("{{ {some} FILTER({condition}) }} {call}"),
        ] {
            shapes.push((
                body,
                none(),
                ShaclPrebinding::None,
                rows(&[("alpha", "alpha/1"), ("alpha", "alpha/2")]),
            ));
        }
    }
    for (body, substitutions, lane, expected) in &shapes {
        let reference = run_pairs_with(&["ff"], true, body, substitutions, *lane);
        assert_eq!(
            reference.answer.as_ref(),
            Ok(expected),
            "{body} under {lane:?}: the bottom-up reference"
        );
        for modes in [&["bf"][..], &["bf", "ff"][..]] {
            let outcome = run_pairs_with(modes, false, body, substitutions, *lane);
            assert_eq!(
                outcome.answer.as_ref(),
                Ok(expected),
                "{body} under {lane:?} declaring {modes:?} answers the bottom-up join"
            );
            assert!(
                !outcome.invocations.is_empty()
                    && outcome
                        .invocations
                        .iter()
                        .all(|invocation| invocation.starts_with("bf:")),
                "{body} under {lane:?} declaring {modes:?} invokes the relation bound only: {:?}",
                outcome.invocations
            );
        }
    }
}

// ── a GROUP BY the pushdown enters ──────────────────────────────────────────

/// The three shapes whose `GROUP BY` the substituted `?q` is a key of — a grouped
/// sub-`SELECT` joined, the same reached through a `LATERAL`, and one grouping a `UNION`
/// whose second arm leaves `?q` unbound — with their exact answers for `?q = "alpha"`
/// over [`lateral_dataset`]'s three left rows. `"alpha"` has two table rows, so its
/// group counts `2`; the unbound arm's group counts the three `<p>` triples, and keeps
/// its row, which the seed then binds to `"alpha"`.
fn group_key_shapes() -> Vec<(String, Vec<(String, String)>)> {
    let call = format!("?q <{PAIRS}> ?x");
    let left = format!("?s <{EX}p> ?v");
    let times = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        let mut expected: Vec<(String, String)> = (0..3).flat_map(|_| rows(pairs)).collect();
        expected.sort();
        expected
    };
    vec![
        (
            format!("{left} {{ SELECT ?q (COUNT(?x) AS ?out) WHERE {{ {call} }} GROUP BY ?q }}"),
            times(&[("alpha", "2")]),
        ),
        (
            format!(
                "{left} LATERAL {{ SELECT ?q (COUNT(?x) AS ?out) WHERE {{ {call} }} GROUP BY ?q }}"
            ),
            times(&[("alpha", "2")]),
        ),
        (
            format!(
                "{left} {{ SELECT ?q (COUNT(*) AS ?out) WHERE {{ {{ {call} }} UNION \
                 {{ ?z <{EX}p> ?w }} }} GROUP BY ?q }}"
            ),
            times(&[("alpha", "2"), ("alpha", "3")]),
        ),
    ]
}

/// `?q = "alpha"`, the substitution the `GROUP BY` shapes run under.
fn alpha_substitution() -> Vec<(String, TermValue)> {
    vec![("q".to_owned(), simple_literal("alpha"))]
}

/// **A `GROUP BY` whose key is the substituted variable is entered by the rewrite.**
/// Restricting the grouped rows to those binding `?q = "alpha"` or leaving it unbound
/// removes whole groups keyed by some other value — exactly the groups whose rows the
/// substitution's seed drops — so the rewrite writes `"alpha"` into the call inside.
/// Each shape is admitted for the relation serving only `bf`, which is invoked with
/// `"alpha"` and nothing else, and answers exactly what the query with the value
/// written in by hand as `VALUES` answers — the documented meaning of a substitution —
/// evaluated bottom-up by a relation that ignores its input.
#[test]
fn a_group_by_keyed_by_the_substituted_variable_is_entered() {
    for (body, expected) in group_key_shapes() {
        let by_hand = format!("{body} VALUES ?q {{ \"alpha\" }}");
        let oracle = run_pairs_with(&["ff"], true, &by_hand, &[], ShaclPrebinding::None);
        assert_eq!(
            oracle.answer.as_ref(),
            Ok(&expected),
            "{by_hand}: the oracle"
        );
        for modes in [&["bf"][..], &["bf", "ff"][..]] {
            let outcome = run_pairs_with(
                modes,
                false,
                &body,
                &alpha_substitution(),
                ShaclPrebinding::None,
            );
            assert_eq!(
                outcome.answer.as_ref(),
                Ok(&expected),
                "{body} declaring {modes:?} answers the hand-written VALUES"
            );
            let mut invoked = outcome.invocations;
            invoked.dedup();
            assert_eq!(
                invoked,
                calls(&["bf:alpha"]),
                "{body} declaring {modes:?} invokes the relation with \"alpha\" only"
            );
        }
    }
}

/// **A `GROUP BY` the substituted variable is not a key of is not entered.** An
/// expression key `(STR(?q) AS ?k)` groups by `?k`, and two values of `?q` can share
/// one; an aggregate `COUNT(?q)` folds every value into one row that does not carry
/// `?q`; a key `?x` other than `?q` partitions by something else. In each the
/// substitution cannot reach the call, so the relation serving only `bf` is refused at
/// prepare and never invoked, and the free-capable one is invoked free and answers
/// exactly the bottom-up reference — every group, joined with the seed afterwards.
#[test]
fn a_group_by_not_keyed_by_the_substituted_variable_is_not_entered() {
    let call = format!("?q <{PAIRS}> ?x");
    let left = format!("?s <{EX}p> ?v");
    let times = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        let mut expected: Vec<(String, String)> = (0..3).flat_map(|_| rows(pairs)).collect();
        expected.sort();
        expected
    };
    for (body, expected) in [
        (
            format!(
                "{left} {{ SELECT ?k (COUNT(?x) AS ?out) WHERE {{ {call} }} \
                 GROUP BY (STR(?q) AS ?k) }}"
            ),
            times(&[("alpha", "2"), ("alpha", "1"), ("alpha", "1")]),
        ),
        (
            format!("{left} {{ SELECT (COUNT(?q) AS ?out) WHERE {{ {call} }} }}"),
            times(&[("alpha", "4")]),
        ),
        (
            format!("{left} {{ SELECT ?x (COUNT(*) AS ?out) WHERE {{ {call} }} GROUP BY ?x }}"),
            times(&[
                ("alpha", "1"),
                ("alpha", "1"),
                ("alpha", "1"),
                ("alpha", "1"),
            ]),
        ),
    ] {
        let refused = run_pairs_with(
            &["bf"],
            false,
            &body,
            &alpha_substitution(),
            ShaclPrebinding::None,
        );
        let Err(message) = &refused.answer else {
            panic!("{body}: the bound-only relation is refused, got {refused:?}");
        };
        assert!(
            message.contains("no feasible evaluation order")
                && message.contains(&format!("<{PAIRS}> reachable only as `ff`")),
            "{body}: the refusal names the call and its free input: {message}"
        );
        assert_eq!(refused.invocations, Vec::<String>::new(), "{body}");

        let reference = run_pairs_with(
            &["ff"],
            true,
            &body,
            &alpha_substitution(),
            ShaclPrebinding::None,
        );
        assert_eq!(
            reference.answer.as_ref(),
            Ok(&expected),
            "{body}: the reference"
        );
        let free = run_pairs_with(
            &["bf", "ff"],
            false,
            &body,
            &alpha_substitution(),
            ShaclPrebinding::None,
        );
        assert_eq!(
            free.answer.as_ref(),
            Ok(&expected),
            "{body}: the free-capable answer"
        );
        assert_eq!(
            free.invocations,
            calls(&["ff:-"]),
            "{body}: the free-capable relation is invoked free"
        );
    }
}
