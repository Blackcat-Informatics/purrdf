// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What feeds a property function's input position: every pattern that binds a
//! variable on every row the query text lets reach the call, not only a triple.
//!
//! A relation that serves only with its input bound is admitted at prepare time
//! exactly when a feasible order binds that input first. A triple pattern always
//! did; this file pins the rest of the rule — an inline `VALUES` column with no
//! `UNDEF`, a `BIND`, a sub-`SELECT` that projects the variable, a nested group,
//! a `UNION` both of whose branches bind it, the variable naming a `GRAPH`, and a
//! `LATERAL`'s left side — and the refusals that bound it: an `UNDEF` cell, a
//! `BIND` written after the call, a `BIND` over an `OPTIONAL` variable, a
//! sub-`SELECT` that does not project it, a `UNION` branch that does not bind it.
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
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, Volatility,
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
                                lexical_form: format!("{key}#{n}"),
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
    let invocations = Arc::new(Mutex::new(Vec::new()));
    let relation = match variant {
        Variant::BoundOnly => Expand::bound_only(Arc::clone(&invocations)),
        Variant::FreeCapable => Expand::free_capable(Arc::clone(&invocations)),
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(EXPAND.to_owned(), Arc::new(relation));
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
            ("alpha", "alpha#1"),
            ("alpha", "alpha#2"),
            ("beta", "beta#1"),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
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
                ("alpha", "alpha#1"),
                ("alpha", "alpha#2"),
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
            answer: Ok(rows(&[("alpha", "alpha#1"), ("alpha", "alpha#2")])),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
            invocations: calls(&["bf:beta"]),
        }
    );
    assert_eq!(
        run(
            Variant::BoundOnly,
            &format!("BIND(CONCAT(\"al\", \"pha\") AS ?q) ?q <{EXPAND}> ?out")
        ),
        Outcome {
            answer: Ok(rows(&[("alpha", "alpha#1"), ("alpha", "alpha#2")])),
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
            answer: Ok(rows(&[("beta!", "beta!#1")])),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
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
            answer: Ok(rows(&[("alpha", "alpha#1"), ("alpha", "alpha#2")])),
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
            answer: Ok(rows(&[("beta", "beta#1")])),
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
                "https://example.org/d/g#1"
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
            "beta#1".to_owned(),
            "beta#1!".to_owned()
        ]]
    );
    assert_eq!(
        *invocations
            .lock()
            .expect("the fixture recorder is never poisoned"),
        calls(&["bf:beta"])
    );
}
