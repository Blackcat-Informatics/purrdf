// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How deep a request may nest is the stack of the thread answering it, end to end
//! through [`NativeSparqlEngine`]: nested brackets, built-in calls, unary minus, groups
//! and property-path groups answer — with the value the nesting computes, not merely
//! without an error — as deep as the thread's stack holds their evaluation, and the
//! only answer past that is the evaluator's typed stack refusal returned as a
//! diagnostic, never a stack overflow that takes the host down. The parser admits every
//! depth; brackets, a group around a single element and a bracketed path build no node
//! of their own, so those shapes answer at any depth.
//!
//! Every limit here is found by bisection on the thread it is asserted on, because the
//! C library can hand a thread a cached stack up to four times the size it asked for:
//! what is asserted is that the deepest level that answers and the first that is
//! refused are neighbours, not where they fall.

mod support;

use purrdf_testkit::text::nested;
use support::two_integer_objects;

use purrdf_core::{RdfDiagnostic, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{EvalError, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

/// The local names of the subjects `query` answers with, sorted, or the engine's
/// diagnostic.
fn subjects(query: &str) -> Result<Vec<String>, RdfDiagnostic> {
    let dataset = two_integer_objects();
    let result = NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
    )?;
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions");
    };
    let mut subjects: Vec<String> = rows
        .into_iter()
        .map(|row| match row.into_iter().next().flatten() {
            Some(TermValue::Iri(iri)) => iri.trim_start_matches(EX).to_owned(),
            other => panic!("?s binds an IRI, got {other:?}"),
        })
        .collect();
    subjects.sort();
    Ok(subjects)
}

/// Whether `diagnostic` is the evaluator's stack refusal
/// ([`EvalError::STACK_EXHAUSTED_CODE`]).
fn is_stack_refusal(diagnostic: &RdfDiagnostic) -> bool {
    diagnostic.code == EvalError::STACK_EXHAUSTED_CODE
}

/// A nesting shape, written `n` levels deep, and the subjects it must answer with: each
/// shape's answer depends on every level being honoured, so a truncated or dropped
/// level would answer differently.
struct Shape {
    name: &'static str,
    text: fn(usize) -> String,
    answer: fn(usize) -> &'static [&'static str],
}

fn shapes() -> [Shape; 5] {
    [
        Shape {
            name: "nested parentheses",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                    nested("(", "?o", ")", n)
                )
            },
            answer: |_| &["s1"],
        },
        // |1 - 3| = 2 and |2 - 3| = 1: only `s2` equals 1, and only if the innermost
        // `ABS` was applied — without it neither subject would.
        Shape {
            name: "nested ABS(",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                    nested("ABS(", "?o - 3", ")", n)
                )
            },
            answer: |_| &["s2"],
        },
        // n negations of `?o` equal (-1)^n times it: `s1` answers only if every one of
        // them was applied.
        Shape {
            name: "nested -(",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = {}) }}",
                    nested("-(", "?o", ")", n),
                    if n % 2 == 0 { "1" } else { "-1" }
                )
            },
            answer: |_| &["s1"],
        },
        Shape {
            name: "nested groups",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ {} }}",
                    nested("{ ", &format!("?s <{EX}p> ?o FILTER(?o = 1)"), " }", n)
                )
            },
            answer: |_| &["s1"],
        },
        Shape {
            name: "nested property-path groups",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s {} ?o FILTER(?o = 2) }}",
                    nested("(", &format!("<{EX}p>"), ")", n)
                )
            },
            answer: |_| &["s2"],
        },
    ]
}

/// The stack of a Linux process's main thread.
const MAIN_THREAD: usize = 8 * 1024 * 1024;
/// The stack Rust gives a spawned thread, and `cargo test` each test.
const SPAWNED_THREAD: usize = 2 * 1024 * 1024;

/// Every shape at 128, 500 and 1 000 levels on a main-thread-sized stack and on a
/// spawned-thread-sized one: each answers with exactly the subjects its nesting computes,
/// or — only where that thread's stack cannot hold it — is the typed stack refusal.
/// On the main-thread-sized stack every shape answers at every depth; on the spawned
/// one every shape answers at 128 and at 500.
#[test]
fn every_shape_answers_at_128_500_and_1000_where_the_stack_holds_it() {
    for (lane, bytes) in [("8 MiB", MAIN_THREAD), ("2 MiB", SPAWNED_THREAD)] {
        let outcomes = purrdf_stack::on_stack(bytes, || {
            let mut outcomes = Vec::new();
            for shape in shapes() {
                for levels in [128, 500, 1_000] {
                    let outcome = subjects(&(shape.text)(levels));
                    outcomes.push((shape.name, levels, outcome, (shape.answer)(levels)));
                }
            }
            outcomes
        })
        .expect("spawn");
        for (name, levels, outcome, answer) in outcomes {
            match outcome {
                Ok(subjects) => assert_eq!(
                    subjects, answer,
                    "{name} {levels} deep on the {lane} stack answers what it computes"
                ),
                Err(refused) => {
                    assert!(
                        is_stack_refusal(&refused),
                        "{name} {levels} deep on the {lane} stack: {refused:?}"
                    );
                    assert!(
                        bytes == SPAWNED_THREAD && levels == 1_000,
                        "{name} {levels} deep must answer on the {lane} stack: {refused:?}"
                    );
                }
            }
        }
    }
}

/// The real limit of every shape on both stacks, found by bisection: the deepest level
/// that answers holds its computed value, and one level more is the typed stack
/// refusal — a refusal pair at the stack's real end, not at a count. Every limit is past
/// 127 levels. A shape whose nesting builds no node answers twenty thousand levels deep.
#[test]
fn the_deepest_answer_and_the_first_refusal_are_neighbours() {
    for (lane, bytes) in [("8 MiB", MAIN_THREAD), ("2 MiB", SPAWNED_THREAD)] {
        purrdf_stack::on_stack(bytes, move || {
            for shape in shapes() {
                let answers = |levels: usize| subjects(&(shape.text)(levels));
                let (mut deepest, mut refused) = (1_usize, 20_000_usize);
                assert!(
                    answers(deepest).is_ok(),
                    "{}: one level answers",
                    shape.name
                );
                match answers(refused) {
                    Ok(subjects) => {
                        assert_eq!(
                            subjects,
                            (shape.answer)(refused),
                            "{} {refused} deep on the {lane} stack answers what it computes",
                            shape.name
                        );
                        continue;
                    }
                    Err(refusal) => assert!(
                        is_stack_refusal(&refusal),
                        "{} {refused} deep on the {lane} stack: {refusal:?}",
                        shape.name
                    ),
                }
                while refused - deepest > 1 {
                    let mid = deepest.midpoint(refused);
                    if answers(mid).is_ok() {
                        deepest = mid;
                    } else {
                        refused = mid;
                    }
                }
                assert_eq!(
                    answers(deepest).expect("the bisected limit answers"),
                    (shape.answer)(deepest),
                    "{} {deepest} deep on the {lane} stack answers what it computes",
                    shape.name
                );
                let refusal = answers(refused).expect_err("one level more is refused");
                assert!(
                    is_stack_refusal(&refusal),
                    "{} {refused} deep on the {lane} stack: {refusal:?}",
                    shape.name
                );
                assert!(
                    deepest >= 128,
                    "{}: only {deepest} levels answer on the {lane} stack",
                    shape.name
                );
                eprintln!(
                    "{lane} stack: {} answers {deepest} deep, refused {refused} deep",
                    shape.name
                );
            }
        })
        .expect("spawn");
    }
}

/// Ten thousand nested parentheses build no node, and answer on a test thread; ten
/// thousand nested negations do build one each, and answer what they compute or are the
/// evaluator's typed stack refusal returned as a diagnostic — never a crash — after
/// which the engine answers the next request.
#[test]
fn ten_thousand_deep_parentheses_answer_and_negations_answer_or_are_the_typed_refusal() {
    let [parentheses, _, negations, ..] = shapes();
    assert_eq!(
        subjects(&(parentheses.text)(10_000)).expect("ten thousand parentheses answer"),
        ["s1"]
    );
    match subjects(&(negations.text)(10_000)) {
        Ok(answered) => assert_eq!(answered, (negations.answer)(10_000)),
        Err(refused) => assert!(is_stack_refusal(&refused), "{refused:?}"),
    }
    assert_eq!(
        subjects(&(parentheses.text)(1)).expect("the next request answers"),
        ["s1"]
    );
}

/// An operator chain is one node however long, so it costs no depth: four hundred
/// alternatives, one of which matches `<s2>` only, answer on a test thread, and so does
/// a negation chain as long as the removed count admitted.
#[test]
fn flat_and_formerly_admitted_expressions_answer_on_a_test_thread() {
    let negated = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}(?o = 1)) }}",
        "!".repeat(126)
    );
    assert_eq!(subjects(&negated).expect("126 negations evaluate"), ["s1"]);
    let alternatives = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}?o = 2) }}",
        "?o = 3 || ".repeat(399)
    );
    assert_eq!(
        subjects(&alternatives).expect("a 400-alternative disjunction evaluates"),
        ["s2"]
    );
}
