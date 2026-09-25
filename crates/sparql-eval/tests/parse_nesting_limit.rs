// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parser's nesting budget, end to end through [`NativeSparqlEngine`]: a request
//! nested past it is the parser's typed refusal returned as a diagnostic — never a stack
//! overflow that takes the host down — while the deepest request the budget admits still
//! parses, evaluates, and answers.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::MAX_NESTING_DEPTH;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

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

/// The subjects `query` answers with, or the engine's diagnostic.
fn subjects(query: &str) -> Result<Vec<String>, purrdf_core::RdfDiagnostic> {
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
            Some(TermValue::Iri(iri)) => iri,
            other => panic!("?s binds an IRI, got {other:?}"),
        })
        .collect();
    subjects.sort();
    Ok(subjects)
}

/// `SELECT ?s` over the fixture, filtered by `?o = 1` inside `parentheses` extra pairs.
fn parenthesised_filter(parentheses: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}?o = 1{}) }}",
        "(".repeat(parentheses),
        ")".repeat(parentheses)
    )
}

#[test]
fn a_ten_thousand_deep_parenthesised_filter_is_the_parse_error_not_a_crash() {
    let refused = subjects(&parenthesised_filter(10_000))
        .expect_err("ten thousand nested parentheses are refused");
    assert!(
        refused.message.contains(&format!(
            "bracketted expression nesting exceeds the safety limit of {MAX_NESTING_DEPTH}"
        )),
        "the parser's own typed refusal reaches the caller: {refused:?}"
    );

    // The valid neighbour: as deep as the budget allows (the WHERE group is its first
    // level) it evaluates, and the filter is honoured — one subject of the two.
    assert_eq!(
        subjects(&parenthesised_filter(MAX_NESTING_DEPTH - 1)).expect("the limit evaluates"),
        vec![format!("{EX}s1")]
    );
    // One pair more is the same refusal.
    let refused = subjects(&parenthesised_filter(MAX_NESTING_DEPTH))
        .expect_err("one level past the budget is refused");
    assert!(
        refused.message.contains("nesting exceeds the safety limit"),
        "{refused:?}"
    );
}

/// Deep trees the parser admits are also evaluated, not merely parsed: a negation
/// chain filling the nesting budget, and an operator chain hundreds of levels tall.
#[test]
fn the_deepest_admitted_expressions_evaluate_and_answer() {
    // 126 negations of a bracketted comparison: an even count, so `?o = 1` survives.
    let negations = MAX_NESTING_DEPTH - 2;
    let negated = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}(?o = 1)) }}",
        "!".repeat(negations)
    );
    assert_eq!(
        subjects(&negated).expect("a negation chain filling the budget evaluates"),
        vec![format!("{EX}s1")]
    );

    // Four hundred alternatives, one of which matches `<s2>` only.
    let alternatives = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}?o = 2) }}",
        "?o = 3 || ".repeat(399)
    );
    assert_eq!(
        subjects(&alternatives).expect("a 400-alternative disjunction evaluates"),
        vec![format!("{EX}s2")]
    );
}
