// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A sub-`SELECT` whose `WHERE` group is directly the next sub-`SELECT` —
//! `{ SELECT * WHERE { SELECT * WHERE … } }`, no group of its own between them — nested a
//! hundred thousand levels deep, is parsed, prepared, refused with the evaluator's typed
//! stack refusal and dropped on a thread with a 128 KiB stack, in a `SELECT`, a
//! `CONSTRUCT` and a `FILTER EXISTS` body alike. A stack overflow aborts the whole test
//! process, so every assertion reached is itself the proof that nothing overflowed.
//!
//! The neighbours: the same shapes three levels deep answer, on an ordinary test thread,
//! exactly the rows and triples their innermost group computes.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlRequest, SparqlResult,
    TermValue,
};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{EvalError, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

/// The stack every deep request below runs on.
const SMALL_STACK: usize = 128 * 1024;

/// How many levels the deep requests are written.
const LEVELS: usize = 100_000;

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

/// `core`, a group, as the `WHERE` group of `levels` sub-`SELECT`s, each of which is
/// directly the `WHERE` group of the one around it.
fn nested(levels: usize, core: &str) -> String {
    format!(
        "{}{core}{}",
        "{ SELECT * WHERE ".repeat(levels),
        " }".repeat(levels)
    )
}

/// The three request forms the nesting is written in, `levels` deep around `core`.
fn requests(levels: usize, core: &str) -> [(&'static str, String); 3] {
    let body = nested(levels, core);
    [
        ("SELECT", format!("SELECT ?s WHERE {{ {body} }}")),
        (
            "CONSTRUCT",
            format!("CONSTRUCT {{ ?s <{EX}q> ?o }} WHERE {{ {body} }}"),
        ),
        (
            "FILTER EXISTS",
            format!("SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER EXISTS {{ {body} }} }}"),
        ),
    ]
}

/// Run `query` through the engine over [`dataset`].
fn answer(query: &str) -> Result<SparqlResult, RdfDiagnostic> {
    let dataset = dataset();
    NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
    )
}

/// Whether `diagnostic` is one of the evaluator's typed stack refusals.
fn is_stack_refusal(diagnostic: &RdfDiagnostic) -> bool {
    diagnostic.code == EvalError::STACK_EXHAUSTED_CODE
        || diagnostic.code == EvalError::HOST_STACK_EXHAUSTED_CODE
}

/// Run `body` on a thread spawned with a [`SMALL_STACK`] stack.
fn on_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(body)
        .expect("spawn a small-stack thread")
        .join()
        .expect("the small-stack thread returned rather than aborting")
}

/// A hundred thousand levels, in every form: the parse succeeds and its tree is dropped,
/// preparation and a query are each the typed stack refusal, all on a 128 KiB stack.
#[test]
fn a_hundred_thousand_directly_nested_sub_selects_are_refused_typed_on_a_small_stack() {
    let outcomes = on_small_stack(|| {
        let mut outcomes = Vec::new();
        for (form, text) in requests(LEVELS, &format!("{{ ?s <{EX}p> ?o }}")) {
            let parsed = SparqlParser::new().parse_query(&text).map(drop);
            let prepared = NativeSparqlEngine::new()
                .prepare_query(&text, None)
                .map(drop);
            let queried = answer(&text).map(drop);
            outcomes.push((form, parsed, prepared, queried));
        }
        outcomes
    });
    for (form, parsed, prepared, queried) in outcomes {
        parsed.unwrap_or_else(|error| panic!("{form} {LEVELS} deep parses: {error}"));
        for (step, outcome) in [("prepare", prepared), ("query", queried)] {
            let refused = outcome.expect_err("a 128 KiB stack cannot evaluate it");
            assert!(
                is_stack_refusal(&refused),
                "{form} {LEVELS} deep, {step}: the typed stack refusal, got {refused:?}"
            );
        }
    }
}

/// The local names of the subjects a `SELECT ?s` answers with, sorted.
fn subjects(result: SparqlResult) -> Vec<String> {
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
    subjects
}

/// Three levels, in every form, answer what the innermost group computes: `?o = 1`
/// holds for `s1` alone, so the `SELECT` answers `s1` and the `CONSTRUCT` builds its one
/// triple; `?x <p> 2` holds for some `?x`, so the `FILTER EXISTS` keeps both subjects,
/// and `?x <p> 3` holds for none, so it keeps neither.
#[test]
fn three_directly_nested_sub_selects_answer_exactly() {
    let core = format!("{{ ?s <{EX}p> ?o FILTER(?o = 1) }}");
    let [(_, select), (_, construct), _] = requests(3, &core);
    assert_eq!(subjects(answer(&select).expect("answers")), ["s1"]);
    let SparqlResult::Graph(graph) = answer(&construct).expect("answers") else {
        panic!("a CONSTRUCT answers with a graph");
    };
    assert_eq!(
        purrdf_core::canonicalize(&graph).nquads,
        format!("<{EX}s1> <{EX}q> \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n")
    );
    for (value, expected) in [("2", &["s1", "s2"][..]), ("3", &[][..])] {
        let [_, _, (_, exists)] = requests(3, &format!("{{ ?x <{EX}p> {value} }}"));
        assert_eq!(
            subjects(answer(&exists).expect("answers")),
            expected,
            "FILTER EXISTS over ?x <p> {value}"
        );
    }
}
