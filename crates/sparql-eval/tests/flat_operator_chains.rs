// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Flat operator chains of any length answer, end to end, with the value the
//! left-nested binary tree they stand for has — and what really nests is still
//! refused.
//!
//! A generated `?x = 1 || ?x = 2 || …`, `?a + ?b + …` or `{ … } UNION { … } UNION …`
//! nests nothing: the parser reads it with a loop into one n-ary node. These tests
//! drive ten thousand terms of each through the whole request path and hold the
//! answer to an oracle that would see a dropped, reordered or re-associated term:
//!
//! * the `||` chain against the `IN` list of the same constants, and against a
//!   neighbour one term shorter whose answer differs;
//! * the `+` chain against a left fold computed here, over literals where a
//!   re-associated sum is a different binary64 — and a neighbour one term shorter;
//! * the `UNION` chain against the exact sequence of rows its arms bind, in arm order.
//!
//! Beside them, true nesting past the parser's budgets — brackets, groups, and an
//! operator tree taller than the height budget — is still the typed refusal, and one
//! level under each answers. Every chain also crosses a `SERVICE` as forwarded text:
//! a five-thousand-term body re-parses in the in-process endpoint and answers what the
//! same body answers locally.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlRequest};
use purrdf_core::{SparqlResult, TermValue};
use purrdf_sparql_eval::{InProcessServiceResolver, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// `<a> <p> 9999`, `<b> <p> 2500`, `<c> <p> 20000`: the `||` chain's last constant
/// matches `a` only, a constant in its middle matches `b` only, and none matches `c`.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for (subject, value) in [("a", "9999"), ("b", "2500"), ("c", "20000")] {
        let s = builder.intern_iri(&format!("{EX}{subject}"));
        let o = builder.intern_literal(RdfLiteral {
            lexical_form: value.to_owned(),
            datatype: Some(format!("{XSD}integer")),
            language: None,
            direction: None,
        });
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture dataset")
}

/// Evaluate `query` against [`dataset`], with an in-process `SERVICE` endpoint
/// `<svc>` serving the same dataset.
fn run(query: &str) -> Result<SparqlResult, RdfDiagnostic> {
    let resolver = InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), dataset());
    NativeSparqlEngine::new().query_with_source(
        &dataset(),
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        &resolver,
        QueryOptions::EMPTY,
    )
}

/// The solutions as `(variable → rendered value)` rows, in answer order. A literal
/// renders as its lexical form and an IRI by its local name; an unbound cell is absent.
fn rows(result: Result<SparqlResult, RdfDiagnostic>) -> Vec<Vec<(String, String)>> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result.unwrap_or_else(|e| panic!("the query answers: {e:?}"))
    else {
        panic!("a SELECT answers with solutions");
    };
    rows.into_iter()
        .map(|row| {
            variables
                .iter()
                .zip(row)
                .filter_map(|(name, cell)| {
                    let rendered = match cell? {
                        TermValue::Iri(iri) => iri.trim_start_matches(EX).to_owned(),
                        TermValue::Literal { lexical_form, .. } => lexical_form,
                        other => panic!("an IRI or a literal, got {other:?}"),
                    };
                    Some((name.clone(), rendered))
                })
                .collect()
        })
        .collect()
}

/// `rows`, sorted, for an answer whose order the query does not fix.
fn sorted_rows(result: Result<SparqlResult, RdfDiagnostic>) -> Vec<Vec<(String, String)>> {
    let mut rows = rows(result);
    rows.sort();
    rows
}

/// One row's `(variable, value)` pairs, from string slices.
fn row(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// `?v = 0 || ?v = 1 || … || ?v = (terms - 1)`, and, when `with_errors`, a type error
/// (`?v < "x"`: an integer does not compare with a string) before every thousandth
/// term.
fn or_chain(terms: usize, with_errors: bool) -> String {
    let mut chain = String::new();
    for k in 0..terms {
        if k > 0 {
            chain.push_str(" || ");
        }
        if with_errors && k % 1000 == 0 {
            chain.push_str("?v < \"x\" || ");
        }
        let _ = write!(chain, "?v = {k}");
    }
    chain
}

/// Each subject with `(chain AS ?r)`: `true`, `false`, or no binding for an error.
fn or_query(chain: &str) -> String {
    format!("SELECT ?s ?r WHERE {{ ?s <{EX}p> ?v BIND(({chain}) AS ?r) }}")
}

/// Ten thousand `||` operands answer, and answer what the `IN` list of the same
/// constants answers: `a` matches only the LAST operand and `b` only one in the
/// middle, so a chain that dropped any operand from either place would answer
/// differently — which the neighbour one operand shorter shows it does.
#[test]
fn a_ten_thousand_operand_or_chain_answers_what_the_in_list_answers() {
    let treatment = sorted_rows(run(&or_query(&or_chain(10_000, false))));
    let expected = vec![
        row(&[("s", "a"), ("r", "true")]),
        row(&[("s", "b"), ("r", "true")]),
        row(&[("s", "c"), ("r", "false")]),
    ];
    assert_eq!(treatment, expected);

    let list = (0..10_000)
        .map(|k| k.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let oracle = sorted_rows(run(&format!(
        "SELECT ?s ?r WHERE {{ ?s <{EX}p> ?v BIND((?v IN ({list})) AS ?r) }}"
    )));
    assert_eq!(
        treatment, oracle,
        "the chain answers what the IN list answers"
    );

    // The valid neighbour, one operand shorter: `a`'s one match is gone, and the
    // answer differs from the treatment's in exactly that row.
    let neighbour = sorted_rows(run(&or_query(&or_chain(9_999, false))));
    assert_eq!(
        neighbour,
        vec![
            row(&[("s", "a"), ("r", "false")]),
            row(&[("s", "b"), ("r", "true")]),
            row(&[("s", "c"), ("r", "false")]),
        ]
    );
    assert_ne!(neighbour, treatment);
}

/// SPARQL's three-valued `||` through the whole chain: operands that raise type errors
/// do not stop a later `true` (`error || true` is `true`), and with no `true` anywhere
/// the chain is an error — no binding — rather than `false`. The error-free chain beside
/// it answers `false` for the same subject, so the two rows observe the difference.
#[test]
fn a_ten_thousand_operand_or_chain_keeps_three_valued_errors() {
    let with_errors = sorted_rows(run(&or_query(&or_chain(10_000, true))));
    assert_eq!(
        with_errors,
        vec![
            row(&[("s", "a"), ("r", "true")]),
            row(&[("s", "b"), ("r", "true")]),
            row(&[("s", "c")]),
        ]
    );
    // As a FILTER, the error drops the row exactly as `false` does.
    let filtered = sorted_rows(run(&format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?v FILTER({}) }}",
        or_chain(10_000, true)
    )));
    assert_eq!(filtered, vec![row(&[("s", "a")]), row(&[("s", "b")])]);

    // `&&` is the dual: ten thousand `?v != k` for k other than `c`'s value are all
    // true for `c`; a type error among them makes the chain an error for `c`, while `a`
    // and `b` each hit one `false`, which beats an error (`false && error` is `false`).
    let and_chain = |with_errors: bool| {
        let mut chain = String::new();
        for k in 0..10_000 {
            if k > 0 {
                chain.push_str(" && ");
            }
            if with_errors && k == 5_000 {
                chain.push_str("?v < \"x\" && ");
            }
            let _ = write!(chain, "?v != {k}");
        }
        sorted_rows(run(&or_query(&chain)))
    };
    assert_eq!(
        and_chain(false),
        vec![
            row(&[("s", "a"), ("r", "false")]),
            row(&[("s", "b"), ("r", "false")]),
            row(&[("s", "c"), ("r", "true")]),
        ]
    );
    assert_eq!(
        and_chain(true),
        vec![
            row(&[("s", "a"), ("r", "false")]),
            row(&[("s", "b"), ("r", "false")]),
            row(&[("s", "c")]),
        ]
    );
}

/// The terms of the `+` chain: 3 000 `xsd:integer`s, 3 000 `xsd:decimal`s and 4 000
/// `xsd:double`s, in that order, the doubles opening with `1.0E16` and continuing with
/// `1.0E0`s — each of which a left fold rounds away (`1e16 + 1` is a tie that binary64
/// rounds to even), and which a re-associated sum would add up first.
fn sum_terms(count: usize) -> Vec<&'static str> {
    (0..count)
        .map(|i| match i {
            0..3_000 => ["1", "2", "3", "4", "5", "6", "7"][i % 7],
            3_000..6_000 => ["0.25", "1.5"][i % 2],
            6_000 => "1.0E16",
            _ => "1.0E0",
        })
        .collect()
}

/// The left fold of `terms` under SPARQL's numeric promotion, computed here and not by
/// the engine: an exact integer sum, then exact decimal steps (every decimal term is a
/// multiple of a quarter, so the sum is kept in quarters), then — from the first double
/// on — the decimal converted to binary64 and each double added in order.
fn left_fold(terms: &[&str]) -> f64 {
    let mut quarters: i64 = 0;
    let mut double: Option<f64> = None;
    for term in terms {
        if let Some(acc) = double.as_mut() {
            *acc += term.parse::<f64>().expect("a double term");
        } else if term.contains('E') {
            #[allow(
                clippy::cast_precision_loss,
                reason = "the quarter count is far below 2^53, so it converts exactly"
            )]
            let exact = quarters as f64 / 4.0;
            double = Some(exact + term.parse::<f64>().expect("a double term"));
        } else if let Some((whole, fraction)) = term.split_once('.') {
            let whole: i64 = whole.parse().expect("a decimal term");
            let fraction = match fraction {
                "25" => 1,
                "5" => 2,
                other => panic!("a quarter, got .{other}"),
            };
            quarters += whole * 4 + fraction;
        } else {
            quarters += 4 * term.parse::<i64>().expect("an integer term");
        }
    }
    double.expect("the chain reaches a double")
}

/// The engine's value of `BIND(t0 + t1 + … AS ?r)`.
fn engine_sum(terms: &[&str]) -> f64 {
    let answer = rows(run(&format!(
        "SELECT ?r WHERE {{ BIND({} AS ?r) }}",
        terms.join(" + ")
    )));
    let [single] = answer.as_slice() else {
        panic!("one row, got {answer:?}");
    };
    let [(name, lexical)] = single.as_slice() else {
        panic!("one binding, got {single:?}");
    };
    assert_eq!(name, "r");
    lexical.parse().expect("an xsd:double lexical")
}

/// A ten-thousand-term `+` chain over mixed integers, decimals and doubles answers the
/// left fold, bit for bit. The oracle observes the order: the right fold of the same
/// terms is a different double, and so is the chain one term shorter.
#[test]
fn a_ten_thousand_term_sum_is_the_left_fold_bit_for_bit() {
    let terms = sum_terms(10_000);
    let expected = left_fold(&terms);
    let right_fold = terms
        .iter()
        .rev()
        .map(|t| t.parse::<f64>().expect("every term reads as a double"))
        .fold(0.0_f64, |acc, t| t + acc);
    assert_ne!(
        expected.to_bits(),
        right_fold.to_bits(),
        "the oracle tells a re-associated sum apart"
    );
    assert_eq!(engine_sum(&terms).to_bits(), expected.to_bits());

    // The valid neighbour one term shorter answers its own left fold, which differs:
    // dropping a term is visible.
    let shorter = sum_terms(10_000);
    let shorter = [&shorter[..6], &shorter[7..]].concat();
    let shorter_expected = left_fold(&shorter);
    assert_ne!(shorter_expected.to_bits(), expected.to_bits());
    assert_eq!(engine_sum(&shorter).to_bits(), shorter_expected.to_bits());
}

/// One type error anywhere in a long `+` chain makes the whole chain an error — no
/// binding — while the same chain without it has a value.
#[test]
fn a_type_error_anywhere_in_a_long_sum_is_the_chain_s_error() {
    let mut terms = vec!["1"; 10_000];
    let clean = rows(run(&format!(
        "SELECT ?r WHERE {{ BIND({} AS ?r) }}",
        terms.join(" + ")
    )));
    assert_eq!(clean, vec![row(&[("r", "10000")])]);
    terms[7_777] = "\"x\"";
    let broken = rows(run(&format!(
        "SELECT ?r WHERE {{ BIND({} AS ?r) }}",
        terms.join(" + ")
    )));
    assert_eq!(broken, vec![row(&[])]);
}

/// `{ BIND(k AS ?v) } UNION …` for k in `0..arms`, even arms binding `?v` and odd
/// arms `?w`.
fn union_chain(arms: usize) -> String {
    (0..arms)
        .map(|k| {
            let variable = if k % 2 == 0 { "v" } else { "w" };
            format!("{{ BIND({k} AS ?{variable}) }}")
        })
        .collect::<Vec<_>>()
        .join(" UNION ")
}

/// The rows [`union_chain`] binds, in arm order.
fn union_rows(arms: usize) -> Vec<Vec<(String, String)>> {
    (0..arms)
        .map(|k| {
            let variable = if k % 2 == 0 { "v" } else { "w" };
            vec![(variable.to_owned(), k.to_string())]
        })
        .collect()
}

/// A ten-thousand-arm flat `UNION` answers exactly the rows its arms bind, one per arm,
/// in arm order — the concatenation the left-nested binary chain produced.
#[test]
fn a_ten_thousand_arm_union_answers_every_arm_in_order() {
    let answer = rows(run(&format!(
        "SELECT ?v ?w WHERE {{ {} }}",
        union_chain(10_000)
    )));
    assert_eq!(answer.len(), 10_000);
    assert_eq!(answer, union_rows(10_000));
}

/// `SELECT ?s WHERE { ?s <p> ?v FILTER(open…(?v = 9999)…close) }`, `depth` brackets.
fn bracketed(depth: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?v FILTER({}?v = 9999{}) }}",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

/// `depth` nested groups around `?s <p> ?v FILTER(?v = 2500)`.
fn grouped(depth: usize) -> String {
    format!(
        "SELECT ?s WHERE {} ?s <{EX}p> ?v FILTER(?v = 2500) {}",
        "{ ".repeat(depth),
        " }".repeat(depth)
    )
}

/// `levels` bracket levels of `(?v = 9999 || ?v = 1 && ?v != 2 + 3 * …)`: seven levels
/// of operator tree per bracket, so the tree is taller than the height budget long
/// before the brackets reach the recursion budget.
fn tall(levels: usize) -> String {
    let mut expression = String::from("?v");
    for _ in 0..levels {
        expression = format!("(?v = 9999 || ?v = 1 && ?v != 2 + 3 * {expression})");
    }
    format!("SELECT ?s WHERE {{ ?s <{EX}p> ?v FILTER({expression}) }}")
}

/// Assert `result` is the parser's typed nesting refusal naming `limit`.
fn assert_nesting_refusal(result: Result<SparqlResult, RdfDiagnostic>, limit: usize, what: &str) {
    let diagnostic = result.expect_err(&format!("{what} is refused"));
    assert!(
        diagnostic
            .message
            .contains(&format!("nesting exceeds the safety limit of {limit}")),
        "{what}: {diagnostic:?}"
    );
}

/// What really nests is still the typed refusal: brackets and groups past the recursion
/// budget, and an operator tree past the height budget. One step under each answers —
/// with the one subject its innermost condition names, not the whole dataset.
#[test]
fn true_nesting_past_the_budgets_is_still_refused() {
    assert_nesting_refusal(run(&bracketed(200)), 128, "200 nested brackets");
    assert_eq!(sorted_rows(run(&bracketed(100))), vec![row(&[("s", "a")])]);

    assert_nesting_refusal(run(&grouped(200)), 128, "200 nested groups");
    assert_eq!(sorted_rows(run(&grouped(100))), vec![row(&[("s", "b")])]);

    assert_nesting_refusal(run(&tall(75)), 512, "a 525-level operator tree");
    assert_eq!(sorted_rows(run(&tall(70))), vec![row(&[("s", "a")])]);
}

/// A `SERVICE` forwards its body as text the in-process endpoint re-parses. A body
/// holding a five-thousand-term chain — `||`, `+` or `UNION` — is forwarded as the flat
/// chain it is, re-parses, and answers what the same body answers without the
/// `SERVICE`. Under `SILENT` a refused re-parse would answer the join identity; the
/// `||` body answers one subject of three, so the rows observe that too.
#[test]
fn five_thousand_term_chains_cross_a_service_as_text() {
    let or_body = format!(
        "?s <{EX}p> ?v FILTER({} || ?v = 9999)",
        or_chain(5_000, true)
    );
    let local = sorted_rows(run(&format!("SELECT ?s WHERE {{ {or_body} }}")));
    assert_eq!(local, vec![row(&[("s", "a")]), row(&[("s", "b")])]);
    for silent in ["", "SILENT "] {
        let forwarded = sorted_rows(run(&format!(
            "SELECT ?s WHERE {{ SERVICE {silent}<{EX}svc> {{ {or_body} }} }}"
        )));
        assert_eq!(forwarded, local, "SERVICE {silent}");
    }

    let terms = sum_terms(5_000);
    let sum_body = format!("BIND({} AS ?r)", terms.join(" + "));
    let local = rows(run(&format!("SELECT ?r WHERE {{ {sum_body} }}")));
    let forwarded = rows(run(&format!(
        "SELECT ?r WHERE {{ SERVICE <{EX}svc> {{ {sum_body} }} }}"
    )));
    assert_eq!(forwarded, local);
    let [single] = local.as_slice() else {
        panic!("one row");
    };
    assert_eq!(single.len(), 1, "the sum has a value");

    let union_body = union_chain(5_000);
    let forwarded = rows(run(&format!(
        "SELECT ?v ?w WHERE {{ SERVICE <{EX}svc> {{ {union_body} }} }}"
    )));
    let mut expected = union_rows(5_000);
    let mut forwarded_sorted = forwarded;
    forwarded_sorted.sort();
    expected.sort();
    assert_eq!(forwarded_sorted, expected);
}
