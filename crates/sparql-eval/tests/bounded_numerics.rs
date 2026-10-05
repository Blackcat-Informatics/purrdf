// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bounded numeric contract, from query text.
//!
//! `xsd:integer` computes in `i128` and `xsd:decimal` in an `i128` mantissa with at
//! most 18 fractional digits. Inside those bounds every operator answers; past them an
//! operator either answers with the documented truncation XPath F&O permits or raises
//! a typed error, which a query observes as an unbound result. Comparison, equality,
//! `ORDER BY`, `MIN`/`MAX`, `isNumeric`, the effective boolean value and the
//! conversions to `xsd:float`/`xsd:double` need no arithmetic and so are exact for a
//! literal of ANY size.
//!
//! Every refusal here sits beside a neighbouring case that must still answer.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use purrdf_testkit::exact::Rational;
use purrdf_testkit::prop::prelude::*;
use purrdf_testkit::rng::splitmix64_next;
use purrdf_xsd::numeric::{canonical_double, canonical_float};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// `i128::MAX`.
const MAX: &str = "170141183460469231731687303715884105727";
/// `i128::MAX - 1`.
const MAX_1: &str = "170141183460469231731687303715884105726";
/// `i128::MIN`.
const MIN: &str = "-170141183460469231731687303715884105728";
/// `i128::MIN + 1`.
const MIN_1: &str = "-170141183460469231731687303715884105727";
/// `10^41`, three decimal orders past `i128`.
const BIG: &str = "100000000000000000000000000000000000000000";
/// `10^41 - 1`.
const BIG_1: &str = "99999999999999999999999999999999999999999";

/// Every row of `query` over the empty dataset, with the `xsd:` prefix declared.
fn run(query: &str) -> Vec<Vec<Option<TermValue>>> {
    let dataset: Arc<RdfDataset> = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    let query = &format!("PREFIX xsd: <{XSD}> {query}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query,
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

/// A cell as `(lexical, datatype)`, or `None` when unbound.
fn cell(value: Option<&TermValue>) -> Option<(String, String)> {
    match value {
        None => None,
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => Some((lexical_form.clone(), datatype.clone())),
        Some(other) => panic!("expected a literal or unbound, got {other:?}"),
    }
}

/// The one value `SELECT (<expression> AS ?y) {}` binds.
fn select(expression: &str) -> Option<(String, String)> {
    let rows = run(&format!("SELECT ({expression} AS ?y) WHERE {{}}"));
    assert_eq!(rows.len(), 1, "{expression}");
    cell(rows[0][0].as_ref())
}

fn typed(lexical: &str, datatype: &str) -> String {
    format!("\"{lexical}\"^^<{XSD}{datatype}>")
}

fn int(lexical: &str) -> String {
    typed(lexical, "integer")
}

fn dec(lexical: &str) -> String {
    typed(lexical, "decimal")
}

fn decimal_answer(lexical: &str) -> Option<(String, String)> {
    Some((lexical.to_owned(), format!("{XSD}decimal")))
}

fn boolean_answer(value: bool) -> Option<(String, String)> {
    Some((value.to_string(), format!("{XSD}boolean")))
}

/// One result cell: `(lexical, datatype)`, or `None` when unbound.
type Cell = Option<(String, String)>;

/// `(AVG(?v), SUM(?v) / COUNT(?v))` over the listed values.
fn avg_and_quotient(values: &[String]) -> (Cell, Cell) {
    let rows = run(&format!(
        "SELECT (AVG(?v) AS ?a) (SUM(?v) / COUNT(?v) AS ?q) WHERE {{ VALUES ?v {{ {} }} }}",
        values.join(" ")
    ));
    assert_eq!(rows.len(), 1);
    (cell(rows[0][0].as_ref()), cell(rows[0][1].as_ref()))
}

// ── AVG: the two acceptance cases ───────────────────────────────────────────────

/// A decimal `AVG` whose running `SUM` passes the `i128` mantissa still answers when
/// the mean is representable, exactly as the same magnitudes do as `xsd:integer`.
#[test]
fn decimal_avg_answers_when_only_the_running_sum_overflows() {
    // {MAX, 1}: the sum is 2^127, one past the mantissa; the mean is exactly 2^126.
    let (avg, _) = avg_and_quotient(&[dec(MAX), dec("1")]);
    assert_eq!(
        avg,
        decimal_answer("85070591730234615865843651857942052864")
    );
    // {MAX, MAX - 1}: the mean is MAX - 0.5, whose scale-1 mantissa overflows, so it
    // truncates toward zero at the finest scale that fits — scale 0.
    let (avg, _) = avg_and_quotient(&[dec(MAX), dec(MAX_1)]);
    assert_eq!(avg, decimal_answer(MAX_1));
    // {MIN, MIN + 1}: MIN + 0.5 truncates toward zero.
    let (avg, _) = avg_and_quotient(&[dec(MIN), dec(MIN_1)]);
    assert_eq!(avg, decimal_answer(MIN_1));
    // The same magnitudes as integers answer the same.
    let (avg, _) = avg_and_quotient(&[int(MAX), int(MAX_1)]);
    assert_eq!(avg, decimal_answer(MAX_1));
    // Neighbour: an ordinary decimal mean is unchanged.
    let (avg, quotient) = avg_and_quotient(&[dec("1.5"), dec("2.25")]);
    assert_eq!(avg, decimal_answer("1.875"));
    assert_eq!(avg, quotient);
}

/// An integer `AVG` never emits a literal outside the decimal value space: the
/// result is ONE representable `xsd:decimal`, and it reads back as a value.
#[test]
fn integer_avg_is_a_representable_decimal() {
    let rows = run(&format!(
        "SELECT ?a (?a + 0 AS ?plus) (xsd:decimal(STR(?a)) AS ?cast) WHERE {{ \
           {{ SELECT (AVG(?v) AS ?a) WHERE {{ VALUES ?v {{ {} {} }} }} }} }}",
        int(MAX),
        int(MAX_1)
    ));
    assert_eq!(rows.len(), 1);
    let avg = cell(rows[0][0].as_ref());
    assert_eq!(avg, decimal_answer(MAX_1));
    assert_eq!(cell(rows[0][1].as_ref()), decimal_answer(MAX_1));
    assert_eq!(cell(rows[0][2].as_ref()), decimal_answer(MAX_1));
}

/// `AVG` is `SUM / COUNT` under one precision rule: where the quotient's scale-18
/// mantissa overflows, both truncate at the finest scale that fits.
#[test]
fn integer_avg_agrees_with_sum_over_count() {
    // The sum 1.6e38 + 1 fits i128; the mean 5.33…e37 fits only at scale 0.
    let values = [
        int("100000000000000000000000000000000000000"),
        int("60000000000000000000000000000000000000"),
        int("1"),
    ];
    let (avg, quotient) = avg_and_quotient(&values);
    assert_eq!(
        avg,
        decimal_answer("53333333333333333333333333333333333333")
    );
    assert_eq!(avg, quotient);
    // Neighbours: a mean with a scale-18 answer, and one exact at scale 1.
    let (avg, quotient) = avg_and_quotient(&[int("1"), int("1"), int("2")]);
    assert_eq!(avg, decimal_answer("1.333333333333333333"));
    assert_eq!(avg, quotient);
    let (avg, quotient) = avg_and_quotient(&[int("2"), int("3")]);
    assert_eq!(avg, decimal_answer("2.5"));
    assert_eq!(avg, quotient);
}

/// A decimal `SUM` is the exact total at any size, as an integer `SUM` is: past the
/// bounds it is a literal every comparison reads exactly and `SUM / COUNT` divides
/// into `AVG`; a total that comes back inside answers exactly even though a running
/// prefix overflowed.
#[test]
fn decimal_sum_is_the_exact_total() {
    let rows = run(&format!(
        "SELECT (SUM(?v) AS ?s) (SUM(?v) / COUNT(?v) AS ?q) (AVG(?v) AS ?a) \
         WHERE {{ VALUES ?v {{ {} {} }} }}",
        dec(MAX),
        dec("1")
    ));
    assert_eq!(
        cell(rows[0][0].as_ref()),
        decimal_answer("170141183460469231731687303715884105728")
    );
    assert_eq!(cell(rows[0][1].as_ref()), cell(rows[0][2].as_ref()));
    assert_eq!(
        cell(rows[0][2].as_ref()),
        decimal_answer("85070591730234615865843651857942052864")
    );
    let rows = run(&format!(
        "SELECT (SUM(?v) AS ?s) WHERE {{ VALUES ?v {{ {} {} {} }} }}",
        dec(MAX),
        dec("1"),
        dec("-2")
    ));
    assert_eq!(cell(rows[0][0].as_ref()), decimal_answer(MAX_1));
}

// ── comparison and equality at any size ─────────────────────────────────────────

#[test]
fn comparison_is_exact_past_i128() {
    assert_eq!(
        select(&format!("{} > {}", int(BIG), int(BIG_1))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} < {}", int(BIG), int(BIG_1))),
        boolean_answer(false)
    );
    assert_eq!(
        select(&format!("{} > {}", int(BIG), int(MAX))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} < {}", int(&format!("-{BIG}")), int(MIN))),
        boolean_answer(true)
    );
    // Promotion: a double compares against the correctly rounded conversion.
    assert_eq!(
        select(&format!("{} = {}", int(BIG), typed("1.0E41", "double"))),
        boolean_answer(true)
    );
    // Bare numeric tokens in the query text are the same literals.
    assert_eq!(select(&format!("{BIG} > {MAX}")), boolean_answer(true));
    assert_eq!(select(&format!("{BIG}.5 > {BIG}")), boolean_answer(true));
    // Neighbour: in-range comparison is untouched.
    assert_eq!(
        select(&format!("{} > {}", int(MAX), int(MAX_1))),
        boolean_answer(true)
    );
}

#[test]
fn comparison_is_exact_past_eighteen_fraction_digits() {
    assert_eq!(
        select(&format!(
            "{} > {}",
            dec("0.10000000000000000001"),
            dec("0.1")
        )),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!(
            "{} < {}",
            dec("0.10000000000000000001"),
            dec("0.10000000000000000002")
        )),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!(
            "{} = {}",
            dec("0.100000000000000000000"),
            dec("0.1")
        )),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!(
            "{} > {}",
            dec("1.0000000000000000000001"),
            int("1")
        )),
        boolean_answer(true)
    );
}

#[test]
fn value_equality_is_exact_past_the_bounds() {
    // One value, two spellings, two datatypes: `=` holds, `sameTerm` does not.
    let integer = int(&format!("+000{BIG}"));
    let decimal = dec(&format!("{BIG}.000"));
    assert_eq!(
        select(&format!("{integer} = {decimal}")),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("sameTerm({integer}, {decimal})")),
        boolean_answer(false)
    );
    assert_eq!(
        select(&format!("{} != {}", int(BIG), int(BIG_1))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} IN ({}, {})", int(BIG), int("1"), decimal)),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} IN ({}, {})", int(BIG), int("1"), int(BIG_1))),
        boolean_answer(false)
    );
}

/// `ORDER BY`, `MIN` and `MAX` sort literals past the bounds by value, among the
/// in-range numbers, not as opaque text.
#[test]
fn order_by_min_and_max_order_by_value_at_any_size() {
    let values = format!(
        "{} {} {} {} {} {} {}",
        int(BIG),
        int("9"),
        int(&format!("-{BIG}")),
        dec("1.5"),
        dec("0.10000000000000000001"),
        dec("0.1"),
        typed("1.0E40", "double"),
    );
    let rows = run(&format!(
        "SELECT ?v WHERE {{ VALUES ?v {{ {values} }} }} ORDER BY ?v"
    ));
    let ordered: Vec<String> = rows
        .iter()
        .map(|row| cell(row[0].as_ref()).expect("bound").0)
        .collect();
    assert_eq!(
        ordered,
        [
            format!("-{BIG}"),
            "0.1".to_owned(),
            "0.10000000000000000001".to_owned(),
            "1.5".to_owned(),
            "9".to_owned(),
            "1.0E40".to_owned(),
            BIG.to_owned(),
        ]
    );
    let rows = run(&format!(
        "SELECT (MIN(?v) AS ?lo) (MAX(?v) AS ?hi) WHERE {{ VALUES ?v {{ {values} }} }}"
    ));
    assert_eq!(
        cell(rows[0][0].as_ref()).expect("bound").0,
        format!("-{BIG}")
    );
    assert_eq!(cell(rows[0][1].as_ref()).expect("bound").0, BIG);
}

// ── the literal is a number: isNumeric, EBV, casts ──────────────────────────────

#[test]
fn a_literal_past_the_bounds_is_numeric_and_has_a_truth_value() {
    assert_eq!(
        select(&format!("isNumeric({})", int(BIG))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("isNumeric({})", dec("0.10000000000000000001"))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("IF({}, true, false)", int(BIG))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!(
            "IF({}, true, false)",
            dec("0.0000000000000000000000")
        )),
        boolean_answer(false)
    );
    // Neighbour: a lexically invalid integer is still not numeric.
    assert_eq!(
        select(&format!("isNumeric({})", int("1.5"))),
        boolean_answer(false)
    );
}

#[test]
fn casts_from_a_literal_past_the_bounds() {
    assert_eq!(
        select(&format!("xsd:boolean({})", int(BIG))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!(
            "xsd:integer({})",
            dec("-7.0000000000000000000001")
        )),
        Some(("-7".to_owned(), format!("{XSD}integer")))
    );
    assert_eq!(
        select(&format!("xsd:string({})", int(&format!("+000{BIG}")))),
        Some((BIG.to_owned(), format!("{XSD}string")))
    );
    assert_eq!(
        select(&format!("xsd:string({})", dec("-00.100000000000000000010"))),
        Some(("-0.10000000000000000001".to_owned(), format!("{XSD}string")))
    );
    // Typed refusals: the value does not fit the target's bounded value space.
    assert_eq!(select(&format!("xsd:decimal({})", int(BIG))), None);
    assert_eq!(select(&format!("xsd:integer({})", int(BIG))), None);
    // Neighbours that fit still cast.
    assert_eq!(
        select(&format!("xsd:decimal({})", int(MAX))),
        decimal_answer(MAX)
    );
}

/// Correctly rounded conversion to `xsd:double`/`xsd:float` from a literal of any
/// size, held to the exact rational oracle — by cast and by arithmetic promotion.
#[test]
fn conversion_to_binary_floating_point_is_correctly_rounded_at_any_size() {
    let mut state = 0x0042_2422_u64;
    let mut cases: Vec<String> = vec![
        // 2^53 + 1 written with 60 digits of fraction zeros: a tie at f64 precision.
        format!("9007199254740993.{}", "0".repeat(60)),
        format!("9007199254740993.{}1", "0".repeat(59)),
        format!("16777217.{}1", "0".repeat(40)),
        format!("-16777217.{}", "0".repeat(40)),
        "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497791.0".to_owned(),
        format!("0.{}1", "0".repeat(330)),
    ];
    for _ in 0..300 {
        let int_len = 1 + (splitmix64_next(&mut state) % 70) as usize;
        let frac_len = (splitmix64_next(&mut state) % 70) as usize;
        let mut text = String::new();
        if splitmix64_next(&mut state) & 1 == 1 {
            text.push('-');
        }
        for i in 0..int_len {
            let digit = splitmix64_next(&mut state) % 10;
            let digit = if i == 0 && digit == 0 { 1 } else { digit };
            text.push(char::from(b'0' + digit as u8));
        }
        if frac_len > 0 {
            text.push('.');
            for _ in 0..frac_len {
                text.push(char::from(b'0' + (splitmix64_next(&mut state) % 10) as u8));
            }
        }
        cases.push(text);
    }
    for lexical in &cases {
        let exact = Rational::parse(lexical).expect("a decimal numeral");
        let want_double = canonical_double(exact.to_f64());
        let want_float = canonical_float(exact.to_f32());
        assert_eq!(
            select(&format!("xsd:double({})", dec(lexical))),
            Some((want_double.clone(), format!("{XSD}double"))),
            "xsd:double of {lexical}"
        );
        assert_eq!(
            select(&format!("xsd:float({})", dec(lexical))),
            Some((want_float.clone(), format!("{XSD}float"))),
            "xsd:float of {lexical}"
        );
        assert_eq!(
            select(&format!("{} * \"1\"^^<{XSD}double>", dec(lexical))),
            Some((want_double, format!("{XSD}double"))),
            "promotion of {lexical} to double"
        );
        assert_eq!(
            select(&format!("{} * \"1\"^^<{XSD}float>", dec(lexical))),
            Some((want_float, format!("{XSD}float"))),
            "promotion of {lexical} to float"
        );
    }
}

// ── arithmetic: typed refusals beside answers ───────────────────────────────────

#[test]
fn arithmetic_overflow_is_an_error_beside_answers_that_fit() {
    assert_eq!(select(&format!("{} + 1", int(MAX))), None);
    assert_eq!(
        select(&format!("{} + 1", int(MAX_1))),
        Some((MAX.to_owned(), format!("{XSD}integer")))
    );
    assert_eq!(select(&format!("{} * 2", int(MAX))), None);
    assert_eq!(select(&format!("-({})", int(MIN))), None);
    assert_eq!(
        select(&format!("-({})", int(MIN_1))),
        Some((MAX.to_owned(), format!("{XSD}integer")))
    );
    assert_eq!(select(&format!("ABS({})", int(MIN))), None);
    // An operand past the bounded representation computes exactly: the result
    // answers where the bounded value space holds it, and is a typed overflow
    // (unbound) where it does not.
    let integer = |lexical: &str| Some((lexical.to_owned(), format!("{XSD}integer")));
    assert_eq!(
        select(&format!("{} - {}", int(BIG), int(BIG))),
        integer("0")
    );
    assert_eq!(
        select(&format!("{} - {}", int(BIG), int(BIG_1))),
        integer("1")
    );
    assert_eq!(select(&format!("{} + 1", int(BIG))), None);
    assert_eq!(select(&format!("{} * 0", int(BIG))), integer("0"));
    assert_eq!(select(&format!("{} * 2", int(BIG))), None);
    assert_eq!(
        select(&format!("{} / {}", int(BIG), int(BIG))),
        decimal_answer("1")
    );
    assert_eq!(select(&format!("{} / 0", int(BIG))), None);
    assert_eq!(select(&format!("ABS({})", int(BIG))), None);
    assert_eq!(select(&format!("-({})", int(BIG))), None);
    assert_eq!(select(&format!("ROUND({})", int(BIG))), None);
    // A decimal past eighteen fractional digits rounds into the bounds.
    let fine = dec("0.50000000000000000001");
    assert_eq!(select(&format!("ROUND({fine})")), decimal_answer("1"));
    assert_eq!(select(&format!("FLOOR({fine})")), decimal_answer("0"));
    assert_eq!(select(&format!("CEIL({fine})")), decimal_answer("1"));
    assert_eq!(
        select(&format!("ROUND({})", dec("-0.50000000000000000001"))),
        decimal_answer("-1")
    );
    assert_eq!(
        select(&format!("FLOOR({})", dec("-0.00000000000000000001"))),
        decimal_answer("-1")
    );
    assert_eq!(select(&format!("{fine} + 0")), decimal_answer("0.5"));
}

/// A decimal written with trailing fractional zeros is the value without them: the
/// spelling's scale never refuses a value the representation holds.
#[test]
fn trailing_fractional_zeros_never_refuse_a_representable_value() {
    let max_point_zero = dec(&format!("{MAX}.0"));
    for expression in [
        format!("{max_point_zero} + 0"),
        format!("{max_point_zero} * 1"),
        format!("ABS({max_point_zero})"),
    ] {
        assert_eq!(select(&expression), decimal_answer(MAX), "{expression}");
    }
    assert_eq!(
        select(&format!("{max_point_zero} - 1")),
        decimal_answer(MAX_1)
    );
    assert_eq!(
        select(&format!("-({max_point_zero})")),
        decimal_answer(MIN_1)
    );
    assert_eq!(
        select(&format!(
            "{} + 0",
            dec("17014118346046923173168730371588410572.70")
        )),
        decimal_answer("17014118346046923173168730371588410572.7")
    );
    assert_eq!(
        select(&format!(
            "{} + 0",
            dec("100000000000000000000000000000000000000.0")
        )),
        decimal_answer("100000000000000000000000000000000000000")
    );
    // A significant fractional digit past what the mantissa holds truncates (the
    // precision rule); an integer part past the bounds overflows.
    assert_eq!(
        select(&format!("{} + 0", dec(&format!("{MAX}.5")))),
        decimal_answer(MAX)
    );
    assert_eq!(
        select(&format!(
            "{} + 0",
            dec("170141183460469231731687303715884105728.0")
        )),
        None
    );
}

/// `AVG` is `SUM / COUNT` on every group, a total past `i128` included.
#[test]
fn avg_is_sum_over_count_past_i128() {
    for values in [
        [int(MAX), int(MAX_1)],
        [int(MIN), int(MIN_1)],
        [int(MAX), int(MAX)],
    ] {
        let (avg, quotient) = avg_and_quotient(&values);
        assert!(avg.is_some(), "{values:?}");
        assert_eq!(avg, quotient, "{values:?}");
    }
    let (avg, _) = avg_and_quotient(&[int(MAX), int(MAX_1)]);
    assert_eq!(avg, decimal_answer(MAX_1));
}

/// Decimal arithmetic keeps every digit the value space holds and truncates the rest
/// toward zero — XPath F&O's implementation-defined precision rule — rather than
/// refusing a result whose integer part fits.
#[test]
fn decimal_arithmetic_truncates_precision_and_overflows_only_on_magnitude() {
    // 10^30 + 10^-18: the exact sum needs 49 digits; the integer part fits.
    assert_eq!(
        select(&format!(
            "{} + {}",
            dec("1000000000000000000000000000000"),
            dec("0.000000000000000001")
        )),
        decimal_answer("1000000000000000000000000000000")
    );
    // 0.5 × 4·10^37: the scale-1 product overflows; the value 2·10^37 does not.
    assert_eq!(
        select(&format!(
            "{} * {}",
            dec("0.5"),
            dec("40000000000000000000000000000000000000")
        )),
        decimal_answer("20000000000000000000000000000000000000")
    );
    // Overflow of the integer part is still an error.
    assert_eq!(select(&format!("{} * {}", dec(MAX), dec("2"))), None);
    assert_eq!(select(&format!("{} + {}", dec(MAX), dec("1"))), None);
}

// ── AVG and SUM against the exact oracle ────────────────────────────────────────

/// The canonical `xsd:decimal` lexical of `mantissa × 10^-scale`.
fn canonical_decimal(mantissa: i128, scale: u32) -> String {
    let digits = mantissa.unsigned_abs().to_string();
    let scale = scale as usize;
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = padded.split_at(padded.len() - scale);
    let sign = if mantissa < 0 { "-" } else { "" };
    let lexical = if scale == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    };
    purrdf_xsd::parse(&lexical, purrdf_xsd::XsdDatatype::Decimal)
        .expect("a bounded decimal")
        .canonical_lexical()
}

/// The precision rule's answer for the exact value `exact`: truncated toward zero at
/// the finest scale ≤ 18 whose mantissa fits `i128`, or `None` (an overflow).
fn rule(exact: &Rational) -> Option<String> {
    (0..=18_u32).rev().find_map(|scale| {
        exact
            .truncate_at_scale(scale)
            .map(|m| canonical_decimal(m, scale))
    })
}

/// One fold operand: integers near the `i128` ceiling, small integers, and
/// decimals with up to eighteen fractional digits.
fn fold_operand() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::regex("-?1[0-6][0-9]{37}"),
        prop::string::regex("-?[0-9]{1,5}"),
        prop::string::regex("-?[0-9]{1,20}\\.[0-9]{1,18}"),
        prop::string::regex("-?1[0-6][0-9]{19}\\.[0-9]{18}"),
    ]
}

prop_test! {
    #![prop_config(Config::with_cases(512))]

    /// `SUM` is the exact total, and `AVG` is one representable `xsd:decimal`: the
    /// exact mean under `numeric_div`'s precision rule, unbound when the mean itself
    /// overflows — equal to `SUM / COUNT` on every group.
    #[test]
    fn avg_and_sum_match_the_exact_oracle(
        values in prop::collection::vec(fold_operand(), 1..10),
    ) {
        let literals: Vec<String> = values
            .iter()
            .map(|v| if v.contains('.') { dec(v) } else { int(v) })
            .collect();
        let rows = run(&format!(
            "SELECT (AVG(?v) AS ?a) (SUM(?v) AS ?s) (SUM(?v) / COUNT(?v) AS ?q) \
             WHERE {{ VALUES ?v {{ {} }} }}",
            literals.join(" ")
        ));
        let (avg, sum, quotient) = (
            cell(rows[0][0].as_ref()),
            cell(rows[0][1].as_ref()),
            cell(rows[0][2].as_ref()),
        );
        let exact_sum = values
            .iter()
            .fold(Rational::from_i128(0), |acc, v| acc.add(&Rational::parse(v).expect("numeral")));
        let count = Rational::from_i128(values.len() as i128);
        prop_assert_eq!(
            avg.as_ref().map(|(lexical, _)| lexical.clone()),
            rule(&exact_sum.div(&count)),
            "AVG of {:?}", values
        );
        if let Some((_, datatype)) = &avg {
            prop_assert_eq!(datatype.as_str(), format!("{XSD}decimal"));
        }
        let (lexical, _) = sum.expect("SUM always answers");
        prop_assert!(
            Rational::parse(&lexical).expect("a numeral").value_eq(&exact_sum),
            "SUM of {:?}", values
        );
        // AVG = SUM / COUNT on every group, a SUM past i128 included.
        prop_assert_eq!(avg, quotient, "AVG vs SUM/COUNT of {:?}", values);
    }
}

// ── composite values (SEP-0009) ──────────────────────────────────────────────────

/// A `cdt:List` literal.
fn list(elements: &str) -> String {
    format!("\"[{elements}]\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>")
}

/// A list element past the bounded representation is a number inside a composite
/// too: element-wise `=` and `<` compare it exactly.
#[test]
fn composite_elements_past_the_bounds_compare_exactly() {
    let one = "100000000000000000000000000000000000000001";
    let two = "100000000000000000000000000000000000000002";
    assert_eq!(
        select(&format!("{} = {}", list(one), list(&format!("{one}.0")))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} = {}", list(one), list(two))),
        boolean_answer(false)
    );
    assert_eq!(
        select(&format!("{} < {}", list(one), list(two))),
        boolean_answer(true)
    );
    assert_eq!(
        select(&format!("{} < {}", list(two), list(one))),
        boolean_answer(false)
    );
    // Neighbour: an ill-typed element is still an error.
    assert_eq!(
        select(&format!(
            "{} = {}",
            list("\\\"x\\\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
            list("1")
        )),
        None
    );
}
