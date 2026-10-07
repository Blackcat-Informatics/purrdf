// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `xsd:integer` and `xsd:decimal` are exact at every size in SPARQL evaluation:
//! arithmetic, casts in both directions, comparison, `ORDER BY`, `MIN`/`MAX`,
//! `SUM`/`AVG`, the numeric functions, `isNumeric` and promotion to `xsd:double`,
//! under the query's division policy and the fuel governor. Each answer is
//! checked against the exact rational oracle (`purrdf_testkit::exact`) or written
//! out by hand; every refusal runs beside a neighbour that answers.

mod support;

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
    TrippedGovernor,
};
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};
use purrdf_testkit::exact::{Direction, Rational as Oracle};
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::exact::DivisionPolicy;
use support::{empty_dataset as empty, squaring_chain};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const EX: &str = "http://example.org/";

/// `2^127`, one past `i128::MAX`.
const PAST_I128: &str = "170141183460469231731687303715884105728";

/// A dataset of `ex:s{i} ex:v "{value}"^^xsd:{datatype}` for each value.
fn values(values: &[(&str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}v"));
    for (index, (lexical, datatype)) in values.iter().enumerate() {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let o = builder.intern_literal(RdfLiteral::typed(
            (*lexical).to_owned(),
            format!("{XSD}{datatype}"),
        ));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("a values dataset")
}

fn run(
    dataset: &RdfDataset,
    query: &str,
    options: QueryOptions<'_>,
) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    let query = format!("PREFIX xsd: <{XSD}>\nPREFIX ex: <{EX}>\n{query}");
    NativeSparqlEngine::new().query_with_options_view(
        dataset,
        SparqlRequest {
            query: &query,
            base_iri: None,
            substitutions: &[],
        },
        options,
    )
}

/// Every cell of every row, as `lexical^^datatype-local-name` (or `-` unbound).
fn cells(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions, got {result:?}");
    };
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
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
                })
                .collect()
        })
        .collect()
}

/// The one cell of `SELECT (<expression> AS ?y) {}`.
fn eval(expression: &str) -> String {
    eval_with(expression, QueryOptions::EMPTY)
}

fn eval_with(expression: &str, options: QueryOptions<'_>) -> String {
    let result = run(
        &empty(),
        &format!("SELECT ({expression} AS ?y) WHERE {{}}"),
        options,
    )
    .unwrap_or_else(|e| panic!("{expression}: {e}"));
    cells(&result).remove(0).remove(0)
}

#[test]
fn arithmetic_past_machine_words_is_exact() {
    let max = i128::MAX;
    assert_eq!(eval(&format!("{max} + 1")), format!("{PAST_I128}^^integer"));
    assert_eq!(
        eval(&format!("{PAST_I128} - 1")),
        format!("{max}^^integer"),
        "back in range"
    );
    assert_eq!(
        eval(&format!("{max} * {max}")),
        "28948022309329048855892746252171976962977213799489202546401021394546514198529^^integer"
    );
    assert_eq!(
        eval(&format!("-(-{PAST_I128})")),
        format!("{PAST_I128}^^integer")
    );
    assert_eq!(
        eval("0.1000000000000000000000000001 + 0"),
        "0.1000000000000000000000000001^^decimal"
    );
    assert_eq!(
        eval("0.0000000001 * 0.0000000001"),
        "0.00000000000000000001^^decimal"
    );
    // The in-range neighbours answer as they always did.
    assert_eq!(eval("40 + 2"), "42^^integer");
    assert_eq!(eval("1.5 * 2.25"), "3.375^^decimal");
}

#[test]
fn casts_in_both_directions_are_exact() {
    assert_eq!(
        eval(r#"xsd:integer("1e300"^^xsd:double)"#),
        exact_integer_of_double(1e300)
    );
    assert_eq!(
        eval(r#"xsd:decimal("1e300"^^xsd:double)"#),
        format!(
            "{}^^decimal",
            exact_integer_of_double(1e300).trim_end_matches("^^integer")
        )
    );
    assert_eq!(
        eval(&format!(r#"xsd:double("{PAST_I128}"^^xsd:integer)"#)),
        "1.7014118346046923E38^^double"
    );
    assert_eq!(
        eval(&format!("xsd:decimal({PAST_I128})")),
        format!("{PAST_I128}^^decimal")
    );
    assert_eq!(
        eval(&format!("xsd:integer({PAST_I128}.75)")),
        format!("{PAST_I128}^^integer")
    );
    assert_eq!(
        eval(&format!("xsd:string({PAST_I128})")),
        format!("{PAST_I128}^^string")
    );
    assert_eq!(eval(&format!("xsd:boolean({PAST_I128})")), "true^^boolean");
    // A derived type still bounds the result: one past unsignedLong is refused,
    // the bound itself is not.
    assert_eq!(eval("xsd:unsignedLong(18446744073709551616)"), "-");
    assert_eq!(
        eval("xsd:unsignedLong(18446744073709551615)"),
        "18446744073709551615^^unsignedLong"
    );
    // NaN and the infinities have no exact value; a finite neighbour does.
    assert_eq!(eval(r#"xsd:integer("INF"^^xsd:double)"#), "-");
    assert_eq!(eval(r#"xsd:integer("1.5E0"^^xsd:double)"#), "1^^integer");
}

/// The integer a finite double denotes, as an `^^integer` cell.
fn exact_integer_of_double(value: f64) -> String {
    let exact = Oracle::from_f64(value).round_to_scale(0, Direction::TowardZero);
    format!(
        "{}^^integer",
        exact.to_canonical_decimal().expect("an integer terminates")
    )
}

#[test]
fn comparison_order_and_extremes_see_every_digit() {
    let a = format!("1{}1", "0".repeat(41));
    let b = format!("1{}2", "0".repeat(41));
    assert_eq!(eval(&format!("{b} > {a}")), "true^^boolean");
    assert_eq!(eval(&format!("{a} = {b}")), "false^^boolean");
    assert_eq!(eval(&format!("{a} = {a}.0")), "true^^boolean");
    assert_eq!(eval(&format!("isNumeric({b})")), "true^^boolean");
    assert_eq!(
        eval(&format!("isNumeric(\"{b}\"^^xsd:integer)")),
        "true^^boolean"
    );

    let data = values(&[
        (&b, "integer"),
        ("7", "integer"),
        (&a, "integer"),
        ("0.1000000000000000000000000001", "decimal"),
        ("0.1", "decimal"),
        (&format!("-{b}"), "integer"),
    ]);
    let ordered = run(
        &data,
        "SELECT ?v WHERE { ?s ex:v ?v } ORDER BY ?v",
        QueryOptions::EMPTY,
    )
    .expect("order");
    assert_eq!(
        cells(&ordered),
        vec![
            vec![format!("-{b}^^integer")],
            vec!["0.1^^decimal".to_owned()],
            vec!["0.1000000000000000000000000001^^decimal".to_owned()],
            vec!["7^^integer".to_owned()],
            vec![format!("{a}^^integer")],
            vec![format!("{b}^^integer")],
        ]
    );
    let extremes = run(
        &data,
        "SELECT (MIN(?v) AS ?lo) (MAX(?v) AS ?hi) WHERE { ?s ex:v ?v }",
        QueryOptions::EMPTY,
    )
    .expect("extremes");
    assert_eq!(
        cells(&extremes),
        vec![vec![format!("-{b}^^integer"), format!("{b}^^integer")]]
    );
    let filtered = run(
        &data,
        &format!("SELECT ?v WHERE {{ ?s ex:v ?v FILTER(?v > {a}) }}"),
        QueryOptions::EMPTY,
    )
    .expect("filter");
    assert_eq!(cells(&filtered), vec![vec![format!("{b}^^integer")]]);
}

#[test]
fn numeric_functions_and_promotion_are_exact() {
    let big = format!("{}.5", "1".repeat(40));
    assert_eq!(
        eval(&format!("ROUND({big})")),
        format!("{}2^^decimal", "1".repeat(39))
    );
    assert_eq!(
        eval(&format!("FLOOR({big})")),
        format!("{}^^decimal", "1".repeat(40))
    );
    assert_eq!(
        eval(&format!("CEIL({big})")),
        format!("{}2^^decimal", "1".repeat(39))
    );
    assert_eq!(eval(&format!("ABS(-{big})")), format!("{big}^^decimal"));
    assert_eq!(
        eval(&format!("ABS(-{PAST_I128})")),
        format!("{PAST_I128}^^integer")
    );
    // Promotion to double rounds once, correctly.
    assert_eq!(
        eval(&format!("{PAST_I128} + 0.0e0")),
        "1.7014118346046923E38^^double"
    );
    assert_eq!(eval(&format!("{} + 1.0e0", "9".repeat(400))), "INF^^double");
}

#[test]
fn sum_count_and_avg_agree_under_every_policy() {
    let mut state = 0x50A5_A5E5_u64;
    for round in 0..40 {
        let rows = 1 + (splitmix64_next(&mut state) % 7) as usize;
        let mut owned = Vec::new();
        for _ in 0..rows {
            let digits = 1 + (splitmix64_next(&mut state) % 45) as usize;
            let mut text: String = (0..digits)
                .map(|_| char::from(b'0' + (splitmix64_next(&mut state) % 10) as u8))
                .collect();
            let decimal = splitmix64_next(&mut state).is_multiple_of(3);
            if decimal {
                let places = (splitmix64_next(&mut state) % 30) as usize;
                text = format!("{text}.{}", "7".repeat(places + 1));
            }
            if splitmix64_next(&mut state).is_multiple_of(2) {
                text.insert(0, '-');
            }
            owned.push((text, if decimal { "decimal" } else { "integer" }));
        }
        let pairs: Vec<(&str, &str)> = owned.iter().map(|(t, d)| (t.as_str(), *d)).collect();
        let data = values(&pairs);
        // The exact oracle's sum, and its mean at eighteen digits.
        let sum = owned.iter().fold(Oracle::from_i128(0), |acc, (text, _)| {
            acc.add(&Oracle::parse(text).expect("numeral"))
        });
        let mean = sum
            .div(&Oracle::from_i128(i128::try_from(rows).expect("small")))
            .expect("nonzero");
        for policy in [DivisionPolicy::default(), DivisionPolicy::Exact] {
            let options = QueryOptions::EMPTY.with_division(policy);
            let answered = run(
                &data,
                "SELECT (SUM(?v) AS ?s) (COUNT(?v) AS ?n) (AVG(?v) AS ?a) (SUM(?v) / COUNT(?v) AS ?q) \
                 WHERE { ?s ex:v ?v }",
                options,
            );
            let row = cells(&answered.expect("an answer")).remove(0);
            if policy == DivisionPolicy::Exact && mean.to_canonical_decimal().is_none() {
                // A non-terminating mean is an aggregate error: AVG and the quotient
                // are both unbound, and agree.
                assert_eq!(row[2], "-", "round {round}: AVG is unbound");
                assert_eq!(row[3], "-", "round {round}: SUM/COUNT is unbound");
                continue;
            }
            let lexical = |cell: &str| cell.split("^^").next().expect("a cell").to_owned();
            assert!(
                Oracle::parse(&lexical(&row[0]))
                    .expect("sum")
                    .value_eq(&sum),
                "round {round}: SUM {}",
                row[0]
            );
            assert_eq!(lexical(&row[1]), rows.to_string());
            assert_eq!(
                row[2], row[3],
                "round {round}: AVG must equal SUM/COUNT ({policy:?})"
            );
            let expected = if policy == DivisionPolicy::Exact {
                mean.clone()
            } else {
                mean.round_to_scale(18, Direction::TowardZero)
            };
            assert!(
                Oracle::parse(&lexical(&row[2]))
                    .expect("mean")
                    .value_eq(&expected),
                "round {round}: AVG {} ({policy:?})",
                row[2]
            );
        }
    }
}

#[test]
fn the_division_policy_is_the_query_s_and_leaves_only_non_terminating_quotients_unbound() {
    let exact = QueryOptions::EMPTY.with_division(DivisionPolicy::Exact);
    assert_eq!(eval_with("1 / 8", exact), "0.125^^decimal");
    assert_eq!(
        eval_with(&format!("1 / {}", "2".to_owned() + &"0".repeat(60)), exact),
        format!("0.{}5^^decimal", "0".repeat(60))
    );
    // A quotient with no finite expansion is an expression error: unbound.
    assert_eq!(eval_with("1 / 3", exact), "-");
    // The default policy answers the same quotient at eighteen digits.
    assert_eq!(eval("1 / 3"), "0.333333333333333333^^decimal");
    // A scale policy rounds as stated.
    let rounded = QueryOptions::EMPTY.with_division(DivisionPolicy::scale(
        4,
        purrdf_xsd::exact::Rounding::HalfEven,
    ));
    assert_eq!(eval_with("2 / 3", rounded), "0.6667^^decimal");
    // Division by zero stays an expression error under every policy.
    assert_eq!(eval_with("1 / 0", exact), "-");
    assert_eq!(eval("1 / 0"), "-");
}

fn governed(query: &str, governors: &QueryGovernors) -> GovernedOutcome {
    let query = format!("PREFIX xsd: <{XSD}>\n{query}");
    NativeSparqlEngine::new()
        .query_governed(
            &empty(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            governors,
        )
        .expect("a tripped governor is an outcome, not a query error")
}

#[test]
fn the_governor_refuses_a_squaring_blow_up_and_admits_its_large_neighbour() {
    let ceiling = QueryGovernors::UNBOUNDED.with_fuel(2_000_000);
    // Twenty squarings of a thousand-digit integer would reach a billion digits;
    // the fuel ceiling stops the chain at the first product it cannot afford,
    // before that product is computed.
    let blow_up = governed(&squaring_chain(1000, 20), &ceiling);
    let GovernedOutcome::BudgetExhausted(exhausted) = blow_up else {
        panic!("the blow-up must be refused");
    };
    assert!(
        matches!(
            exhausted.tripped,
            TrippedGovernor::Budget { .. } | TrippedGovernor::Refused { .. }
        ),
        "{:?}",
        exhausted.tripped
    );
    // The neighbour: three squarings of the same integer — an 8,000-digit
    // product — answer within the same ceiling, exactly.
    let neighbour = governed(&squaring_chain(1000, 3), &ceiling);
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = neighbour
    else {
        panic!("the large legitimate query must answer");
    };
    let base = Oracle::parse(&"7".repeat(1000)).expect("a numeral");
    let eighth = base.mul(&base);
    let eighth = eighth.mul(&eighth);
    let eighth = eighth.mul(&eighth);
    let length = eighth.to_canonical_decimal().expect("an integer").len();
    assert_eq!(cells(&result), vec![vec![format!("{length}^^integer")]]);
    assert!(
        evidence.consumed_in(purrdf_core::ResourceDimension::Fuel) > 100_000,
        "the exact products are charged"
    );
    // Machine-word arithmetic charges no tower work.
    let small = governed("SELECT (2 * 3 AS ?y) WHERE {}", &QueryGovernors::METERED);
    assert!(
        small
            .evidence()
            .consumed_in(purrdf_core::ResourceDimension::Fuel)
            < 50
    );
}
