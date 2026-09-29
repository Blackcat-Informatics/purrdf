// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen vectors of the numeric, calendar and RFC 3339 kernels, replayed
//! against their one implementation here.
//!
//! Each file under `tests/vectors/` holds answers recorded once and never
//! edited: a disagreement is a defect in the kernel, never a reason to touch a
//! vector.

use std::cmp::Ordering;

use purrdf_testkit::vectors::{VectorFile, decode_str};
use purrdf_xsd::bigint::BigInt;
use purrdf_xsd::numeric::{is_decimal_lexical, is_integer_lexical};
use purrdf_xsd::rfc3339::{self, LeapSecond, ParseErrorKind, Separator};
use purrdf_xsd::temporal::{civil_from_days, days_from_civil, days_in_month, is_leap};
use purrdf_xsd::wide::{div_wide, gcd, mul_div, wide_mul};
use purrdf_xsd::{XsdDatatype, json_number};

fn vectors(text: &'static str) -> VectorFile<'static> {
    VectorFile::parse(text).unwrap_or_else(|error| panic!("{error}"))
}

/// A `date-time` answer as the vectors spell it: `ok:<seconds>:<nanos>` or
/// `err:<byte offset>:<kind>`.
fn date_time_answer(text: &str, separator: Separator, leap_second: LeapSecond) -> String {
    match rfc3339::parse(text, separator, leap_second) {
        Ok((seconds, nanos)) => format!("ok:{seconds}:{nanos}"),
        Err(error) => {
            let kind = match error.kind() {
                ParseErrorKind::EndOfInput { .. } => "end".to_owned(),
                ParseErrorKind::Unexpected { .. } => "byte".to_owned(),
                ParseErrorKind::OutOfRange { field, .. } => {
                    format!("range-{}", field.replace(' ', "-"))
                }
                ParseErrorKind::LeapSecond(_) => "leap".to_owned(),
                ParseErrorKind::Trailing { .. } => "trailing".to_owned(),
            };
            format!("err:{}:{kind}", error.at())
        }
    }
}

/// Every frozen spelling: a `date-time` under the GTS profiles' grammar gives
/// its frozen instant or error, and each production under JSON Schema's
/// grammar is accepted exactly when the frozen verdict says so.
#[test]
fn rfc3339_matches_every_frozen_spelling() {
    let file = vectors(include_str!("vectors/rfc3339_vectors.txt"));
    let mut date_times = 0;
    for record in file.records() {
        let text = decode_str(record.fields[1]).expect("an encoded spelling");
        let json_schema = match record.fields[0] {
            "date-time" => {
                assert_eq!(
                    date_time_answer(&text, Separator::GtsSpaceAllowed, LeapSecond::MonthEnd),
                    record.fields[2],
                    "line {}: {text:?}",
                    record.line
                );
                date_times += 1;
                rfc3339::parse(&text, Separator::Rfc3339Abnf, LeapSecond::AnyDay).is_ok()
            }
            "full-date" => rfc3339::parse_date(&text).is_ok(),
            "full-time" => rfc3339::parse_time(&text).is_ok(),
            other => panic!("line {}: unknown production {other}", record.line),
        };
        assert_eq!(
            json_schema,
            record.fields[3] == "1",
            "line {}: {text:?}",
            record.line
        );
    }
    assert!(date_times > 400_000, "{date_times} date-time spellings");
}

/// Every month of years −800 through 2800: its length, the day number of its
/// first day, its year's leap-ness, and the inverse mapping at both ends.
#[test]
fn calendar_matches_the_frozen_months() {
    let file = vectors(include_str!("vectors/calendar_vectors.txt"));
    let replayed = file
        .replay(2, |fields| {
            let year: i64 = fields[0].parse().expect("a year");
            let month: u8 = fields[1].parse().expect("a month");
            let last = days_in_month(year, month);
            let first = days_from_civil(year, month, 1);
            assert_eq!(civil_from_days(first), (i128::from(year), month, 1));
            assert_eq!(
                civil_from_days(first + i128::from(last) - 1),
                (i128::from(year), month, last)
            );
            vec![
                last.to_string(),
                first.to_string(),
                u8::from(is_leap(year)).to_string(),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 3601 * 12);
}

/// The 256-bit product, the fused `a × b / c` and the wide division over the
/// edge-value cube and a SplitMix corpus.
#[test]
fn wide_arithmetic_matches_the_frozen_vectors() {
    let file = vectors(include_str!("vectors/wide_vectors.txt"));
    file.replay(3, |fields| {
        let [a, b, c] = [0, 1, 2].map(|index| fields[index].parse::<u128>().expect("a u128"));
        let (high, low) = wide_mul(a, b);
        let dash = || "-".to_owned();
        vec![
            high.to_string(),
            low.to_string(),
            mul_div(a, b, c).map_or_else(dash, |quotient| quotient.to_string()),
            if c != 0 && high < c {
                div_wide(high, low, c).to_string()
            } else {
                dash()
            },
        ]
    })
    .unwrap_or_else(|mismatch| panic!("{mismatch}"));
}

#[test]
fn gcd_matches_the_frozen_divisors() {
    let file = vectors(include_str!("vectors/gcd_vectors.txt"));
    file.replay(2, |fields| {
        let [a, b] = [0, 1].map(|index| fields[index].parse::<u128>().expect("a u128"));
        vec![gcd(a, b).to_string()]
    })
    .unwrap_or_else(|mismatch| panic!("{mismatch}"));
}

/// `numerator × 2^exponent` written out in full, for every exponent a binary64
/// or binary32 value or midpoint reaches.
#[test]
fn from_binary_matches_the_frozen_expansions() {
    let file = vectors(include_str!("vectors/binary_decimal_vectors.txt"));
    file.replay(2, |fields| {
        let numerator: i128 = fields[0].parse().expect("a numerator");
        let exponent: i32 = fields[1].parse().expect("an exponent");
        let (mantissa, scale) = BigInt::from_binary(numerator, exponent);
        vec![mantissa.to_decimal_lexical(scale)]
    })
    .unwrap_or_else(|mismatch| panic!("{mismatch}"));
}

/// Every ordered pair of the frozen JSON number lexemes: order and divisibility.
#[test]
fn json_number_order_matches_the_frozen_pairs() {
    let file = vectors(include_str!("vectors/json_number_vectors.txt"));
    for record in file.records() {
        let order = match json_number::cmp(record.fields[0], record.fields[1]) {
            Some(Ordering::Less) => "<",
            Some(Ordering::Equal) => "=",
            Some(Ordering::Greater) => ">",
            None => panic!("line {}: a JSON number was refused", record.line),
        };
        assert_eq!(order, record.fields[2], "line {}", record.line);
        let number = |text| {
            json_number::JsonNumber::parse(text)
                .unwrap_or_else(|| panic!("line {}: a JSON number was refused", record.line))
        };
        let multiple = number(record.fields[0]).is_multiple_of(&number(record.fields[1]));
        assert_eq!(
            if multiple { "1" } else { "0" },
            record.fields[3],
            "line {}",
            record.line
        );
    }
}

/// Every string of up to four characters over the digits, signs, point,
/// exponent letter, space and a letter; every modelled XSD local name and some
/// that are not.
#[test]
fn numeric_predicates_match_the_frozen_vectors() {
    let file = vectors(include_str!("vectors/numeric_predicate_vectors.txt"));
    file.replay(2, |fields| {
        let input = decode_str(fields[1]).expect("an encoded input");
        let (first, second) = if fields[0] == "lexical" {
            (is_integer_lexical(&input), is_decimal_lexical(&input))
        } else {
            let datatype = XsdDatatype::from_local(&input);
            (
                datatype.is_some_and(XsdDatatype::is_integer_family),
                datatype.is_some_and(XsdDatatype::is_numeric),
            )
        };
        vec![u8::from(first).to_string(), u8::from(second).to_string()]
    })
    .unwrap_or_else(|mismatch| panic!("{mismatch}"));
}
