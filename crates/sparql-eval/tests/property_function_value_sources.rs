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

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, ShaclPrebinding, Volatility,
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
                env: &env,
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

/// `?input <pairs> ?output` over [`PAIR_TABLE`], serving `bf` and `ff` CONSISTENTLY:
/// bound, it answers the table's rows for that key; free, the whole table. So a
/// query answered with its input bound must equal the free evaluation joined with
/// the rest of the query afterwards — SPARQL's bottom-up answer.
struct Pairs {
    modes: Vec<BindingPattern>,
    /// Answer the whole table whatever the input, leaving the engine to join the rows
    /// with the call's bound arguments afterwards — the bottom-up reference.
    ignores_input: bool,
    invocations: Arc<Mutex<Vec<String>>>,
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
        for &(input, output) in PAIR_TABLE {
            if self.ignores_input || key.as_deref().is_none_or(|key| key == input) {
                rows.push(vec![simple_literal(input), simple_literal(output)]);
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

/// **Without a `GROUP BY`, `SAMPLE`, `MIN` and `MAX` are not a source, and `COUNT` is.**
/// An aggregate with no `GROUP BY` answers one row even over an empty input, where
/// the first three are unbound: over `?s <nothing> ?v`, which matches nothing, the
/// free-capable variant is invoked FREE — the observation the refusal rests on. `COUNT`
/// answers `0` there, bound, so it feeds the call.
#[test]
fn an_aggregate_without_group_by_over_a_possibly_empty_input() {
    let call = format!("?q <{EXPAND}> ?out");
    for aggregate in ["SAMPLE", "MIN", "MAX"] {
        let body =
            format!("{{ SELECT ({aggregate}(?v) AS ?q) WHERE {{ ?s <{EX}nothing> ?v }} }} {call}");
        assert_refused_at_prepare(&run(Variant::BoundOnly, &body));
        assert_eq!(
            run(Variant::FreeCapable, &body),
            Outcome {
                answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
                invocations: calls(&["ff:-"]),
            },
            "{body}: the one implicit group's {aggregate} is unbound"
        );
    }
    let body = format!("{{ SELECT (COUNT(?v) AS ?q) WHERE {{ ?s <{EX}nothing> ?v }} }} {call}");
    let expected = Outcome {
        answer: Ok(rows(&[("0", "0/1")])),
        invocations: calls(&["bf:0"]),
    };
    assert_eq!(run(Variant::BoundOnly, &body), expected, "{body}");
    assert_eq!(run(Variant::FreeCapable, &body), expected, "{body}");
}

/// **An aggregate that can answer unbound over a non-empty group is not a source, and
/// neither is a grouping key some row leaves unbound.**
///
/// `SUM` of a string poisons to unbound; `GROUP_CONCAT` of a blank node does (it has no
/// lexical form); `GROUP BY ?q` over an `OPTIONAL` that matched nothing forms a group
/// whose key is unbound. Each is refused for the bound-only relation, and the
/// free-capable variant is observed invoked free on exactly those rows.
#[test]
fn an_aggregate_or_key_that_can_be_unbound_is_not_a_source() {
    let call = format!("?q <{EXPAND}> ?out");
    let free = Outcome {
        answer: Ok(rows(&[(&format!("{EX}{FREE_ROW}"), FREE_ROW)])),
        invocations: calls(&["ff:-"]),
    };
    let sum =
        format!("{{ SELECT ?s (SUM(?v) AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}");
    let key = format!(
        "{{ SELECT ?q WHERE {{ ?s <{EX}p> ?v OPTIONAL {{ ?s <{EX}nothing> ?q }} }} \
         GROUP BY ?q }} {call}"
    );
    for body in [&sum, &key] {
        assert_refused_at_prepare(&run(Variant::BoundOnly, body));
        assert_eq!(run(Variant::FreeCapable, body), free, "{body}");
    }

    let blank_data = {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri(&format!("{EX}s"));
        let p = builder.intern_iri(&format!("{EX}p"));
        let b = builder.intern_blank("b0", purrdf_core::BlankScope::DEFAULT);
        builder.push_quad(s, p, b, None);
        builder.freeze().expect("the fixture must validate")
    };
    let concat = format!(
        "{{ SELECT ?s (GROUP_CONCAT(?v) AS ?q) WHERE {{ ?s <{EX}p> ?v }} GROUP BY ?s }} {call}"
    );
    assert_refused_at_prepare(&run_over(&blank_data, Variant::BoundOnly, &concat));
    assert_eq!(
        run_over(&blank_data, Variant::FreeCapable, &concat),
        free,
        "{concat}"
    );
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
