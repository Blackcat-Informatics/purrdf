// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SPARQL operator-mapping table (acceptance artifact 2 for).
//!
//! Each row asserts `value_cmp(lhs, rhs)`, which is the value-space primitive behind
//! the SPARQL `=`, `<`, `>`, `<=`, `>=` operators (the evaluator derives those and
//! maps the incomparable `None` to a type error). Covers the numeric promotion tower
//! (`integer ⊂ decimal ⊂ float ⊂ double`), string, boolean, and the temporal families.

use purrdf_xsd::{parse, value_cmp, value_eq, XsdDatatype as D};
use std::cmp::Ordering;

const EQ: Option<Ordering> = Some(Ordering::Equal);
const LT: Option<Ordering> = Some(Ordering::Less);
const GT: Option<Ordering> = Some(Ordering::Greater);
const NC: Option<Ordering> = None; // incomparable / type error

/// (lhs_lexical, lhs_dt, rhs_lexical, rhs_dt, expected `value_cmp`).
#[allow(clippy::type_complexity)]
const TABLE: &[(&str, D, &str, D, Option<Ordering>)] = &[
    // ── numeric tower: promotion across types ──
    ("1", D::Integer, "1", D::Integer, EQ),
    ("1", D::Integer, "2", D::Integer, LT),
    ("1", D::Integer, "1.0", D::Decimal, EQ),
    ("3", D::Integer, "2.5", D::Decimal, GT),
    ("1", D::Integer, "1.0E0", D::Double, EQ),
    ("2", D::Integer, "2.5", D::Double, LT),
    ("1.5", D::Decimal, "1.25", D::Float, GT),
    ("1.5", D::Decimal, "1.5", D::Double, EQ),
    ("0.1", D::Float, "0.2", D::Double, LT),
    // ── IEEE specials ──
    ("INF", D::Double, "1.0E0", D::Double, GT),
    ("-INF", D::Double, "0", D::Integer, LT),
    ("NaN", D::Double, "NaN", D::Double, NC),
    ("NaN", D::Double, "1", D::Integer, NC),
    // ── string (codepoint order) ──
    ("abc", D::String, "abc", D::String, EQ),
    ("abc", D::String, "abd", D::String, LT),
    ("Z", D::String, "a", D::String, LT), // 'Z'(0x5A) < 'a'(0x61)
    // ── boolean ──
    ("false", D::Boolean, "true", D::Boolean, LT),
    ("true", D::Boolean, "true", D::Boolean, EQ),
    // ── dateTime / date / time ──
    (
        "2024-01-01T00:00:00Z",
        D::DateTime,
        "2024-01-01T01:00:00+01:00",
        D::DateTime,
        EQ,
    ),
    (
        "2024-01-01T00:00:00Z",
        D::DateTime,
        "2024-01-01T00:00:01Z",
        D::DateTime,
        LT,
    ),
    (
        "2024-01-01T12:00:00",
        D::DateTime,
        "2024-01-01T12:00:00Z",
        D::DateTime,
        NC,
    ),
    ("2023-12-31Z", D::Date, "2024-01-01Z", D::Date, LT),
    ("09:00:00Z", D::Time, "10:00:00Z", D::Time, LT),
    // ── duration partial order ──
    ("P1Y", D::Duration, "P13M", D::Duration, LT),
    ("P1M", D::Duration, "P30D", D::Duration, NC),
    ("PT1H", D::DayTimeDuration, "PT2H", D::DayTimeDuration, LT),
    // ── cross-family: incomparable ──
    ("1", D::Integer, "1", D::String, NC),
    ("true", D::Boolean, "1", D::Integer, NC),
    ("2024-01-01T00:00:00Z", D::DateTime, "P1Y", D::Duration, NC),
    // ── derived-integer cross-subtype equality/comparison ──
    // xsd:int 5 = xsd:long 5
    ("5", D::Int, "5", D::Long, EQ),
    // xsd:byte 5 = xsd:integer 5
    ("5", D::Byte, "5", D::Integer, EQ),
    // xsd:short 3 < xsd:int 4
    ("3", D::Short, "4", D::Int, LT),
    // xsd:unsignedByte 2 < xsd:double 2.5 (cross-tower promotion)
    ("2", D::UnsignedByte, "2.5", D::Double, LT),
];

#[test]
fn operator_mapping_table() {
    for (la, da, lb, db, want) in TABLE {
        let a = parse(la, *da).unwrap_or_else(|e| panic!("parse {la:?}^^{da:?}: {e}"));
        let b = parse(lb, *db).unwrap_or_else(|e| panic!("parse {lb:?}^^{db:?}: {e}"));
        assert_eq!(
            value_cmp(&a, &b),
            *want,
            "value_cmp({la:?}^^{da:?}, {lb:?}^^{db:?})"
        );
        // value_eq agrees with value_cmp == Equal.
        assert_eq!(
            value_eq(&a, &b),
            *want == EQ,
            "value_eq({la:?}^^{da:?}, {lb:?}^^{db:?})"
        );
    }
}

#[test]
fn value_cmp_is_antisymmetric_on_determinate_rows() {
    for (la, da, lb, db, want) in TABLE {
        let a = parse(la, *da).unwrap();
        let b = parse(lb, *db).unwrap();
        let forward = value_cmp(&a, &b);
        let backward = value_cmp(&b, &a);
        match want {
            Some(Ordering::Equal) => assert_eq!(backward, EQ),
            Some(Ordering::Less) => assert_eq!(backward, GT),
            Some(Ordering::Greater) => assert_eq!(backward, LT),
            None => assert_eq!(backward, NC), // incomparable both ways
        }
        let _ = forward;
    }
}

// ── Duration cross-subtype operator rows ─────────────────────────────────────────

#[test]
fn duration_cross_subtype_operator_rows() {
    // dayTimeDuration vs yearMonthDuration: non-zero cross-subtype → incomparable.
    // "P1D" = (months=0, seconds=86400) vs "P1Y" = (months=12, seconds=0):
    // months disagree (0 vs 12) and seconds disagree (86400 vs 0) → None.
    let day = parse("P1D", D::DayTimeDuration).unwrap();
    let year = parse("P1Y", D::YearMonthDuration).unwrap();
    assert_eq!(
        value_cmp(&day, &year),
        NC,
        "P1D (dayTime) vs P1Y (yearMonth) is incomparable"
    );

    // Same-subtype determinate pair (dayTimeDuration total order).
    let h1 = parse("PT1H", D::DayTimeDuration).unwrap();
    let h2 = parse("PT2H", D::DayTimeDuration).unwrap();
    assert_eq!(value_cmp(&h1, &h2), LT, "dayTimeDuration PT1H < PT2H");
    assert_eq!(value_cmp(&h2, &h1), GT, "dayTimeDuration PT2H > PT1H");

    // Zero-component cross-subtype: "P0M"^^yearMonthDuration vs "PT0S"^^dayTimeDuration.
    // Both reduce to the zero-duration value pair (months=0, seconds=0) → Equal.
    // This is NOT a cross-subtype incomparability: the two-component partial order is
    // defined on values, not on subtype labels. See cmp_duration doc for chosen = semantics.
    let zero_ym = parse("P0M", D::YearMonthDuration).unwrap();
    let zero_dt = parse("PT0S", D::DayTimeDuration).unwrap();
    assert_eq!(
        value_cmp(&zero_ym, &zero_dt),
        EQ,
        "zero yearMonthDuration = zero dayTimeDuration (value pair is (0,0) for both)"
    );
}

// ── Gregorian operator-mapping rows ──────────────────────────────────────────────

/// Additional Gregorian rows for the operator-mapping table.
#[allow(clippy::type_complexity)]
const GREGORIAN_TABLE: &[(&str, D, &str, D, Option<Ordering>)] = &[
    // same-type ordering
    ("2023", D::GYear, "2024", D::GYear, LT),
    ("--03", D::GMonth, "--11", D::GMonth, LT),
    ("---01", D::GDay, "---15", D::GDay, LT),
    ("2024-01", D::GYearMonth, "2024-05", D::GYearMonth, LT),
    ("--02-01", D::GMonthDay, "--03-01", D::GMonthDay, LT),
    // same value → Equal
    ("2024", D::GYear, "2024", D::GYear, EQ),
    ("--05", D::GMonth, "--05", D::GMonth, EQ),
    // cross-family incomparable
    ("2024", D::GYear, "--05", D::GMonth, NC),
    ("---15", D::GDay, "--02-15", D::GMonthDay, NC),
    // cross with other temporal families
    ("2024", D::GYear, "2024-01-01", D::Date, NC),
];

#[test]
fn gregorian_operator_mapping_table() {
    for (la, da, lb, db, want) in GREGORIAN_TABLE {
        let a = parse(la, *da).unwrap_or_else(|e| panic!("parse {la:?}^^{da:?}: {e}"));
        let b = parse(lb, *db).unwrap_or_else(|e| panic!("parse {lb:?}^^{db:?}: {e}"));
        assert_eq!(
            value_cmp(&a, &b),
            *want,
            "value_cmp({la:?}^^{da:?}, {lb:?}^^{db:?})"
        );
    }
}

#[test]
fn gregorian_effective_boolean_value_is_type_error() {
    use purrdf_xsd::effective_boolean_value;
    // Gregorian values have no EBV — must return None (SPARQL type error).
    let gyear = parse("2024", D::GYear).unwrap();
    assert_eq!(effective_boolean_value(&gyear), None, "gYear has no EBV");
    let gmonth = parse("--05", D::GMonth).unwrap();
    assert_eq!(effective_boolean_value(&gmonth), None, "gMonth has no EBV");
    let gday = parse("---15", D::GDay).unwrap();
    assert_eq!(effective_boolean_value(&gday), None, "gDay has no EBV");
    let gym = parse("2024-05", D::GYearMonth).unwrap();
    assert_eq!(effective_boolean_value(&gym), None, "gYearMonth has no EBV");
    let gmd = parse("--02-29", D::GMonthDay).unwrap();
    assert_eq!(effective_boolean_value(&gmd), None, "gMonthDay has no EBV");
}

// ── xsd:hexBinary / xsd:base64Binary operator-mapping ───────────────────────────

#[test]
fn binary_cross_datatype_incomparable() {
    // hexBinary "4D" and base64Binary "TQ==" both decode to [0x4D]
    // but they are from DIFFERENT value spaces → incomparable.
    let hex = parse("4D", D::HexBinary).unwrap();
    let b64 = parse("TQ==", D::Base64Binary).unwrap();
    assert_eq!(
        value_cmp(&hex, &b64),
        NC,
        "hexBinary and base64Binary are incomparable even with identical bytes"
    );
    assert!(
        !value_eq(&hex, &b64),
        "value_eq must be false for cross-datatype binary"
    );
}

#[test]
fn binary_same_datatype_ordering() {
    // hexBinary byte-lexicographic order: "00" < "FF".
    let lo = parse("00", D::HexBinary).unwrap();
    let hi = parse("FF", D::HexBinary).unwrap();
    assert_eq!(value_cmp(&lo, &hi), LT, "hexBinary \"00\" < \"FF\"");
    assert_eq!(value_cmp(&hi, &lo), GT, "hexBinary \"FF\" > \"00\"");

    // Equal bytes → Equal.
    let a = parse("0FB7", D::HexBinary).unwrap();
    let b = parse("0fb7", D::HexBinary).unwrap();
    assert_eq!(value_cmp(&a, &b), EQ, "\"0FB7\" == \"0fb7\" (same bytes)");
    assert!(value_eq(&a, &b));

    // base64Binary: "AAAA" (0,0,0) < "////"/base64 (255,255,255).
    let lo64 = parse("AAAA", D::Base64Binary).unwrap();
    let hi64 = parse("////", D::Base64Binary).unwrap();
    assert_eq!(value_cmp(&lo64, &hi64), LT, "base64Binary AAAA < ////");
}

#[test]
fn binary_effective_boolean_value_is_type_error() {
    use purrdf_xsd::effective_boolean_value;
    let hex = parse("0F", D::HexBinary).unwrap();
    assert_eq!(effective_boolean_value(&hex), None, "hexBinary has no EBV");
    let b64 = parse("TQ==", D::Base64Binary).unwrap();
    assert_eq!(
        effective_boolean_value(&b64),
        None,
        "base64Binary has no EBV"
    );
}

#[test]
fn binary_cross_family_with_other_types_incomparable() {
    // Binary vs integer is incomparable.
    let hex = parse("01", D::HexBinary).unwrap();
    let int = parse("1", D::Integer).unwrap();
    assert_eq!(value_cmp(&hex, &int), NC, "hexBinary vs integer is NC");

    // Binary vs string is incomparable.
    let s = parse("0F", D::String).unwrap();
    assert_eq!(value_cmp(&hex, &s), NC, "hexBinary vs string is NC");
}
