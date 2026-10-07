// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SPARQL numeric results of `xsd:integer`/`xsd:decimal` values of any size on
//! `wasm32-unknown-unknown`, held to the native ones byte for byte.
//!
//! WebAssembly lowers every `i128` operation, every `u64` carry lane and every `u128`
//! rounding step of the arbitrary-precision tower through 32- and 64-bit operations.
//! A lowering bug would not crash; it would bind a different digit. This target runs
//! the evaluator over the numeric surface a query reaches — arithmetic past the machine
//! words, every division policy, the casts in both directions, comparison and
//! `ORDER BY`, the aggregates, the numeric functions, the F&O codes of the errors it
//! absorbs — and folds every bound cell into one transcript whose digest both targets
//! must reproduce. The governor's refusals are in it too: a squaring chain refused for
//! fuel and for scratch bytes beside the small neighbour that answers under the same
//! ceiling, so the charge and the refusal are proved on wasm32 as well as natively.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner: `cargo test
//! --workspace` runs it natively on every `make check`, and `make wasm-test` runs the
//! same named cases on wasm32 in Node (`docs/WASM_TESTING.md`).

mod support;

use std::fmt::Write as _;

use purrdf_core::{ResourceDimension, SparqlRequest, SparqlResult, TrippedGovernor};
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};
use purrdf_testkit::harness::assert_transcript_digest;
use purrdf_xsd::exact::{DivisionPolicy, Rounding};
use support::{empty_dataset, numeric_cell as cell, squaring_chain_from};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// The transcript's FNV-1a digest, as both targets compute it.
const GOLDEN_DIGEST: u64 = 0xda39_00a9_3723_c4ac;

/// `i128::MAX`.
const MAX: &str = "170141183460469231731687303715884105727";

/// Every row of `result`, one line per row.
fn rows(result: &SparqlResult) -> String {
    match result {
        SparqlResult::Solutions { rows, .. } => rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|value| cell(value.as_ref()))
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        SparqlResult::Boolean(answer) => answer.to_string(),
        other @ SparqlResult::Graph(_) => format!("{other:?}"),
    }
}

fn try_governed(
    query: &str,
    options: QueryOptions<'_>,
    governors: &QueryGovernors,
) -> Result<GovernedOutcome, purrdf_core::RdfDiagnostic> {
    let query = format!("PREFIX xsd: <{XSD}>\n{query}");
    NativeSparqlEngine::new().query_governed(
        &empty_dataset(),
        SparqlRequest {
            query: &query,
            base_iri: None,
            substitutions: &[],
        },
        options,
        governors,
    )
}

fn governed(query: &str, options: QueryOptions<'_>, governors: &QueryGovernors) -> GovernedOutcome {
    try_governed(query, options, governors).unwrap_or_else(|e| panic!("{query}: {e}"))
}

/// One query's outcome as transcript text: its rows and the F&O codes of the errors it
/// absorbed when it completed, or the governor that stopped it.
fn outcome_text(outcome: &GovernedOutcome) -> String {
    match outcome {
        GovernedOutcome::Complete {
            result, evidence, ..
        } => {
            let mut text = rows(result);
            for (code, count) in evidence.expression_errors() {
                let _ = write!(text, "\n{}×{count}", code.qname());
            }
            text
        }
        GovernedOutcome::BudgetExhausted(exhausted) => match exhausted.tripped {
            TrippedGovernor::Budget { dimension, .. } => {
                format!("refused: {}", dimension.label())
            }
            other => format!("refused: {other:?}"),
        },
    }
}

/// The queries of the transcript, each with the division policy it runs under.
fn queries() -> Vec<(String, DivisionPolicy)> {
    let default = DivisionPolicy::xsd_default();
    let mut out: Vec<(String, DivisionPolicy)> = Vec::new();
    let mut add = |query: String, policy: DivisionPolicy| out.push((query, policy));
    let big = format!("1{}7", "0".repeat(60));
    let fine = format!("0.{}3", "0".repeat(40));
    add(
        format!(
            "SELECT ({MAX} + 1 AS ?a) ({big} * {big} AS ?b) ({fine} * {fine} AS ?c) \
             ({big} - {MAX} AS ?d) (-{big} AS ?e) WHERE {{}}"
        ),
        default,
    );
    for policy in [
        default,
        DivisionPolicy::Exact,
        DivisionPolicy::scale(40, Rounding::HalfEven),
        DivisionPolicy::scale(3, Rounding::Ceiling),
    ] {
        add(
            format!(
                "SELECT (1 / 8 AS ?a) (2 / 3 AS ?b) ({big} / 7 AS ?c) ({fine} / 16 AS ?d) \
                 (AVG(?v) AS ?e) WHERE {{ VALUES ?v {{ 1 2 {big} }} }}"
            ),
            policy,
        );
    }
    add(
        format!(
            "SELECT (xsd:decimal(\"0.1\"^^xsd:double) AS ?a) (xsd:decimal(\"1e-30\"^^xsd:double) AS ?b) \
             (xsd:integer(\"1e40\"^^xsd:double) AS ?c) (xsd:double({big}) AS ?d) \
             (xsd:float({fine}) AS ?e) (xsd:integer({fine}) AS ?f) (xsd:string({big}) AS ?g) \
             ({big} + 1.0e0 AS ?h) WHERE {{}}"
        ),
        default,
    );
    add(
        format!(
            "SELECT ?v WHERE {{ VALUES ?v {{ {big} {MAX} 1.5 {fine} -{big} 1.0e300 \"7\"^^xsd:byte }} }} \
             ORDER BY ?v"
        ),
        default,
    );
    add(
        format!(
            "SELECT (SUM(?v) AS ?s) (MIN(?v) AS ?lo) (MAX(?v) AS ?hi) (COUNT(?v) AS ?n) \
             WHERE {{ VALUES ?v {{ {big} {MAX} 1.5 {fine} -{big} }} }}"
        ),
        default,
    );
    add(
        format!(
            "SELECT (ABS(-{big}) AS ?a) (CEIL({fine}) AS ?b) (FLOOR(-{fine}) AS ?c) \
             (ROUND(2.5) AS ?d) (ROUND({MAX}.5) AS ?e) ({big} > {MAX} AS ?f) \
             ({fine} = {fine}0 AS ?g) WHERE {{}}"
        ),
        default,
    );
    add(
        "SELECT (1 / 0 AS ?a) (xsd:byte(300) AS ?b) (xsd:integer(\"NaN\"^^xsd:double) AS ?c) \
         (\"a\" + 1 AS ?d) WHERE {}"
            .to_owned(),
        default,
    );
    out
}

/// Every query's outcome, then the governor's refusals beside their neighbours, folded
/// into one transcript.
fn transcript() -> String {
    let mut text = String::new();
    for (query, policy) in queries() {
        let answered = match try_governed(
            &query,
            QueryOptions::EMPTY.with_division(policy),
            &QueryGovernors::METERED,
        ) {
            Ok(outcome) => outcome_text(&outcome),
            // A query error is transcribed with its code (a quotient the policy cannot
            // express is an expression error instead, unbound in the rows).
            Err(diagnostic) => format!("query error: {}: {}", diagnostic.code, diagnostic.message),
        };
        let _ = writeln!(text, "{policy}\n{answered}");
    }
    for dimension in [ResourceDimension::Fuel, ResourceDimension::ScratchBytes] {
        let (refused, neighbour) = refusal_beside_neighbour(dimension);
        let _ = writeln!(text, "{}\n{refused}\n{neighbour}", dimension.label());
    }
    text
}

/// A twenty-step squaring of `0.1` (its scale doubles to a million digits) and of `1.0`
/// (a machine word throughout), under a `dimension` ceiling equal to the neighbour's own
/// metered consumption: the neighbour answers, the vast chain is refused before it runs.
fn refusal_beside_neighbour(dimension: ResourceDimension) -> (String, String) {
    let neighbour = squaring_chain_from("1.0", 20);
    let metered = governed(&neighbour, QueryOptions::EMPTY, &QueryGovernors::METERED);
    let spent = metered.evidence().consumed_in(dimension);
    let ceiling = match dimension {
        ResourceDimension::Fuel => QueryGovernors::UNBOUNDED.with_fuel(spent),
        _ => QueryGovernors::UNBOUNDED.with_max_scratch_bytes(spent),
    };
    let answered = governed(&neighbour, QueryOptions::EMPTY, &ceiling);
    let refused = governed(
        &squaring_chain_from("0.1", 20),
        QueryOptions::EMPTY,
        &ceiling,
    );
    assert!(
        matches!(answered, GovernedOutcome::Complete { .. }),
        "the neighbour answers under its own consumption: {answered:?}"
    );
    assert!(
        matches!(&refused, GovernedOutcome::BudgetExhausted(exhausted)
            if matches!(exhausted.tripped, TrippedGovernor::Budget { dimension: d, .. } if d == dimension)),
        "the vast chain is refused for {dimension:?}: {refused:?}"
    );
    (outcome_text(&refused), outcome_text(&answered))
}

/// The transcript of every numeric query reproduces on this target.
fn the_numeric_transcript_digest_is_reproduced_on_this_target() {
    assert_transcript_digest("sparql-numeric-transcript", &transcript(), GOLDEN_DIGEST);
}

/// Answers checkable by hand reproduce on this target, beside the digest.
fn the_hand_answers_are_reproduced_on_this_target() {
    let answer = |query: &str, policy: DivisionPolicy| {
        outcome_text(&governed(
            query,
            QueryOptions::EMPTY.with_division(policy),
            &QueryGovernors::METERED,
        ))
    };
    let default = DivisionPolicy::xsd_default();
    assert_eq!(
        answer(&format!("SELECT ({MAX} + 1 AS ?a) WHERE {{}}"), default),
        "170141183460469231731687303715884105728^^integer"
    );
    assert_eq!(
        answer(&format!("SELECT ({MAX} / 2 AS ?a) WHERE {{}}"), default),
        "85070591730234615865843651857942052863.5^^decimal"
    );
    assert_eq!(
        answer(
            "SELECT (xsd:decimal(\"0.1\"^^xsd:double) AS ?a) WHERE {}",
            default
        ),
        "0.1000000000000000055511151231257827021181583404541015625^^decimal"
    );
    assert_eq!(
        answer(
            "SELECT (1 / 3 AS ?a) WHERE {}",
            DivisionPolicy::scale(5, Rounding::HalfEven)
        ),
        "0.33333^^decimal"
    );
    assert_eq!(
        answer("SELECT (1 / 0 AS ?a) WHERE {}", default),
        "-\nerr:FOAR0001×1"
    );
}

purrdf_testkit::harness_main!(
    the_hand_answers_are_reproduced_on_this_target,
    the_numeric_transcript_digest_is_reproduced_on_this_target,
);
