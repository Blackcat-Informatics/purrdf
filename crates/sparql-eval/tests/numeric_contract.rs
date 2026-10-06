// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The numeric contract from query text: `AVG` answers whenever its mean exists and
//! agrees with `SUM / COUNT`, a `SUM` or `AVG` result is a value of its datatype that
//! reads back as itself, and every numeric error SPARQL absorbs into an unbound value
//! reaches the caller as its XPath F&O code in the governed outcome's evidence.
//!
//! Every refusal sits beside a neighbouring case that answers.

mod support;

use purrdf_core::{SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};
use purrdf_xsd::ErrorCode;
use support::empty_dataset;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// `i128::MAX`.
const MAX: &str = "170141183460469231731687303715884105727";
/// `i128::MAX - 1`.
const MAX_1: &str = "170141183460469231731687303715884105726";
/// `i128::MIN`.
const MIN: &str = "-170141183460469231731687303715884105728";
/// `i128::MIN + 1`.
const MIN_1: &str = "-170141183460469231731687303715884105727";

fn run(query: &str) -> Vec<Vec<Option<TermValue>>> {
    let query = format!("PREFIX xsd: <{XSD}>\n{query}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    rows
}

/// A cell as `lexical^^local-name`, or `-` when unbound.
fn cell(value: Option<&TermValue>) -> String {
    match value {
        None => "-".to_owned(),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => format!(
            "{lexical_form}^^{}",
            datatype.strip_prefix(XSD).unwrap_or(datatype)
        ),
        Some(other) => format!("{other:?}"),
    }
}

fn typed(lexical: &str, datatype: &str) -> String {
    format!("\"{lexical}\"^^xsd:{datatype}")
}

/// `(AVG(?v), SUM(?v) / COUNT(?v))` over the listed values, each as a cell.
fn avg_and_quotient(values: &[String]) -> (String, String) {
    let rows = run(&format!(
        "SELECT (AVG(?v) AS ?a) (SUM(?v) / COUNT(?v) AS ?q) WHERE {{ VALUES ?v {{ {} }} }}",
        values.join(" ")
    ));
    (cell(rows[0][0].as_ref()), cell(rows[0][1].as_ref()))
}

/// A decimal `AVG` whose running `SUM` passes the machine-word mantissa answers its
/// exact mean, as the same magnitudes do as integers, and agrees with
/// `SUM / COUNT`.
#[test]
fn a_decimal_avg_answers_when_its_running_sum_leaves_machine_words() {
    for (values, mean) in [
        // The sum is 2^127; the mean exactly 2^126.
        (
            [typed(MAX, "decimal"), typed("1", "decimal")],
            "85070591730234615865843651857942052864",
        ),
        // MAX − 0.5, exactly.
        (
            [typed(MAX, "decimal"), typed(MAX_1, "decimal")],
            "170141183460469231731687303715884105726.5",
        ),
        (
            [typed(MIN, "decimal"), typed(MIN_1, "decimal")],
            "-170141183460469231731687303715884105727.5",
        ),
        (
            [typed(MAX, "integer"), typed(MAX_1, "integer")],
            "170141183460469231731687303715884105726.5",
        ),
        // The neighbour: an ordinary mean.
        ([typed("1.5", "decimal"), typed("2.25", "decimal")], "1.875"),
    ] {
        let (avg, quotient) = avg_and_quotient(&values);
        assert_eq!(avg, format!("{mean}^^decimal"), "{values:?}");
        assert_eq!(avg, quotient, "AVG agrees with SUM / COUNT over {values:?}");
    }
}

/// `SUM` and `AVG` results are values of their datatype: each reads back as itself,
/// through arithmetic and through a cast of its string.
#[test]
fn sum_and_avg_results_read_back_as_themselves() {
    for (aggregate, values, expected) in [
        (
            "SUM",
            [typed(MAX, "decimal"), typed("1", "decimal")],
            "170141183460469231731687303715884105728^^decimal",
        ),
        (
            "SUM",
            [typed(MAX, "integer"), typed(MAX, "integer")],
            "340282366920938463463374607431768211454^^integer",
        ),
        (
            "AVG",
            [typed(MAX, "integer"), typed(MAX_1, "integer")],
            "170141183460469231731687303715884105726.5^^decimal",
        ),
        // The neighbour, inside machine words.
        (
            "SUM",
            [typed("1.5", "decimal"), typed("1", "decimal")],
            "2.5^^decimal",
        ),
    ] {
        let rows = run(&format!(
            "SELECT ?a (?a + 0 AS ?plus) (xsd:decimal(STR(?a)) AS ?cast) (?a = xsd:decimal(STR(?a)) AS ?same) \
             WHERE {{ {{ SELECT ({aggregate}(?v) AS ?a) WHERE {{ VALUES ?v {{ {} }} }} }} }}",
            values.join(" ")
        ));
        let row: Vec<String> = rows[0].iter().map(|c| cell(c.as_ref())).collect();
        assert_eq!(row[0], expected, "{aggregate} over {values:?}");
        let lexical = expected.split("^^").next().expect("a cell");
        assert_eq!(row[1].split("^^").next(), Some(lexical));
        assert_eq!(row[2], format!("{lexical}^^decimal"));
        assert_eq!(row[3], "true^^boolean");
    }
}

/// The F&O code of every numeric error SPARQL absorbs into an unbound value is
/// counted in the governed outcome's evidence; a query with none reports none.
#[test]
fn absorbed_numeric_errors_reach_the_caller_as_their_codes() {
    let governed = |query: &str| {
        let query = format!("PREFIX xsd: <{XSD}>\n{query}");
        NativeSparqlEngine::new()
            .query_governed(
                &empty_dataset(),
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("a governed outcome")
    };
    let outcome = governed(
        "SELECT (1 / 0 AS ?a) (1.5 / 0.0 AS ?b) (xsd:byte(300) AS ?c) \
                (xsd:integer(\"NaN\"^^xsd:double) AS ?d) (xsd:decimal(\"1.5x\") AS ?e) \
                (\"a\" + 1 AS ?f) WHERE {}",
    );
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &outcome
    else {
        panic!("{outcome:?}");
    };
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("solutions");
    };
    assert!(rows[0].iter().all(Option::is_none), "every cell is unbound");
    assert_eq!(
        evidence.expression_errors(),
        &[
            (ErrorCode::Foar0001, 2),
            (ErrorCode::Foca0002, 1),
            (ErrorCode::Forg0001, 2),
            (ErrorCode::Xpty0004, 1),
        ]
    );
    // The neighbour: the same operations over valid operands report nothing.
    let outcome = governed(
        "SELECT (1 / 2 AS ?a) (1.5 / 3.0 AS ?b) (xsd:byte(100) AS ?c) \
                (xsd:integer(\"2.5\"^^xsd:double) AS ?d) (xsd:decimal(\"1.5\") AS ?e) \
                (1 + 1 AS ?f) WHERE {}",
    );
    assert_eq!(outcome.evidence().expression_errors(), &[]);
    assert!(matches!(outcome, GovernedOutcome::Complete { .. }));
}
