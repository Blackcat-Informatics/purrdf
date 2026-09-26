// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How deep a request may nest is the stack of the thread answering it, end to end
//! through [`NativeSparqlEngine`]: nested brackets, built-in calls, unary minus, groups
//! and property-path groups answer — with the value the nesting computes, not merely
//! without an error — as deep as the thread's stack holds them, and the first level
//! past that is a typed stack refusal returned as a diagnostic, never a stack overflow
//! that takes the host down.
//!
//! Every limit here is found by bisection on the thread it is asserted on, because the
//! C library can hand a thread a cached stack up to four times the size it asked for:
//! what is asserted is that the deepest level that answers and the first that is
//! refused are neighbours, not where they fall.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlRequest, SparqlResult,
    TermValue,
};
use purrdf_sparql_eval::{EvalError, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

/// `<s1> <p> 1` and `<s2> <p> 2`.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for n in 1..=2 {
        let s = builder.intern_iri(&format!("{EX}s{n}"));
        let o = builder.intern_literal(RdfLiteral::typed(
            n.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture dataset")
}

/// The local names of the subjects `query` answers with, sorted, or the engine's
/// diagnostic.
fn subjects(query: &str) -> Result<Vec<String>, RdfDiagnostic> {
    let dataset = dataset();
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

/// Whether `diagnostic` is a stack refusal: the parser's (a parse diagnostic whose
/// message is the typed [`purrdf_sparql_algebra::ParseError::StackExhausted`]) or the
/// evaluator's ([`EvalError::STACK_EXHAUSTED_CODE`]).
fn is_stack_refusal(diagnostic: &RdfDiagnostic) -> bool {
    diagnostic.code == EvalError::STACK_EXHAUSTED_CODE
        || (diagnostic.code == "native-sparql-query-parse"
            && diagnostic.message.contains("SPARQL parse stack exhausted"))
}

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
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

/// Run `body` on a thread with a `bytes`-sized stack.
fn on_thread<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(bytes)
        .spawn(body)
        .expect("spawn")
        .join()
        .expect("the thread returned rather than aborting")
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
        let outcomes = on_thread(bytes, || {
            let mut outcomes = Vec::new();
            for shape in shapes() {
                for levels in [128, 500, 1_000] {
                    let outcome = subjects(&(shape.text)(levels));
                    outcomes.push((shape.name, levels, outcome, (shape.answer)(levels)));
                }
            }
            outcomes
        });
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
/// the 127 levels the removed count admitted.
#[test]
fn the_deepest_answer_and_the_first_refusal_are_neighbours() {
    for (lane, bytes) in [("8 MiB", MAIN_THREAD), ("2 MiB", SPAWNED_THREAD)] {
        on_thread(bytes, move || {
            for shape in shapes() {
                let answers = |levels: usize| subjects(&(shape.text)(levels));
                let (mut deepest, mut refused) = (1_usize, 20_000_usize);
                assert!(
                    answers(deepest).is_ok(),
                    "{}: one level answers",
                    shape.name
                );
                assert!(
                    answers(refused).is_err(),
                    "{}: twenty thousand levels exceed the {lane} stack",
                    shape.name
                );
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
        });
    }
}

/// Ten thousand nested parentheses on a test thread are the parser's typed stack
/// refusal returned as a diagnostic — never a crash — and the engine answers the next
/// request.
#[test]
fn a_ten_thousand_deep_parenthesised_filter_is_the_typed_refusal_not_a_crash() {
    let deep = &shapes()[0];
    let refused = subjects(&(deep.text)(10_000))
        .expect_err("ten thousand nested parentheses do not fit a test thread's stack");
    assert!(is_stack_refusal(&refused), "{refused:?}");
    assert!(
        refused.message.contains("bracketted expression"),
        "the refusal names the construct: {refused:?}"
    );
    assert_eq!(
        subjects(&(deep.text)(1)).expect("the next request answers"),
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
