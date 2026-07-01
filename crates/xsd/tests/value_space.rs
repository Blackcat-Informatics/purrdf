// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XSD value-space conformance vectors (acceptance artifact #1 for #907):
//! per-datatype lexical → value → canonical round-trips, the parse-by-IRI contract,
//! the zero-dep numeric bounds, and the partial-order edge cases.

use purrdf_xsd::{parse, parse_by_iri, value_cmp, XsdDatatype as D, XsdError, XsdValue};
use std::cmp::Ordering;

/// (lexical, datatype, expected canonical lexical).
const CANONICAL_VECTORS: &[(&str, D, &str)] = &[
    // integer
    ("42", D::Integer, "42"),
    ("+007", D::Integer, "7"),
    ("-0", D::Integer, "0"),
    // derived integer subtypes — canonical form is just the decimal integer value.
    ("127", D::Byte, "127"),
    ("-128", D::Byte, "-128"),
    ("255", D::UnsignedByte, "255"),
    ("0", D::UnsignedByte, "0"),
    ("32767", D::Short, "32767"),
    ("65535", D::UnsignedShort, "65535"),
    ("2147483647", D::Int, "2147483647"),
    ("4294967295", D::UnsignedInt, "4294967295"),
    ("9223372036854775807", D::Long, "9223372036854775807"),
    (
        "18446744073709551615",
        D::UnsignedLong,
        "18446744073709551615",
    ),
    ("1", D::PositiveInteger, "1"),
    ("-1", D::NegativeInteger, "-1"),
    ("0", D::NonNegativeInteger, "0"),
    ("0", D::NonPositiveInteger, "0"),
    // Leading-zero stripping still applies.
    ("007", D::Int, "7"),
    // decimal
    ("12.00", D::Decimal, "12.0"),
    (".5", D::Decimal, "0.5"),
    ("100", D::Decimal, "100.0"),
    ("-0.250", D::Decimal, "-0.25"),
    // double / float
    ("1.0E2", D::Double, "1.0E2"),
    ("0.005", D::Double, "5.0E-3"),
    ("INF", D::Double, "INF"),
    ("-INF", D::Float, "-INF"),
    ("NaN", D::Double, "NaN"),
    // boolean
    ("1", D::Boolean, "true"),
    ("false", D::Boolean, "false"),
    // string (lexical == value)
    ("héllo", D::String, "héllo"),
    // dateTime / date / time
    (
        "2024-02-29T12:30:00.500Z",
        D::DateTime,
        "2024-02-29T12:30:00.5Z",
    ),
    (
        "2024-02-29T00:00:00+00:00",
        D::DateTime,
        "2024-02-29T00:00:00Z",
    ),
    ("2024-02-29-05:00", D::Date, "2024-02-29-05:00"),
    ("12:00:00", D::Time, "12:00:00"),
    // duration + subtypes
    ("P1Y2M3DT4H5M6S", D::Duration, "P1Y2M3DT4H5M6S"),
    ("PT1.500S", D::DayTimeDuration, "PT1.5S"),
    ("P14M", D::YearMonthDuration, "P1Y2M"),
];

#[test]
fn canonical_lexical_round_trips() {
    for (lexical, dt, expected) in CANONICAL_VECTORS {
        let value = parse(lexical, *dt)
            .unwrap_or_else(|e| panic!("parse({lexical:?}, {dt:?}) failed: {e}"));
        assert_eq!(
            value.canonical_lexical(),
            *expected,
            "canonical_lexical({lexical:?}^^{dt:?})"
        );
        assert_eq!(value.datatype(), *dt, "datatype({lexical:?}^^{dt:?})");
    }
}

#[test]
fn canonical_is_idempotent() {
    // Re-parsing the canonical form yields the same canonical form.
    for (lexical, dt, _) in CANONICAL_VECTORS {
        let once = parse(lexical, *dt).unwrap().canonical_lexical();
        let twice = parse(&once, *dt).unwrap().canonical_lexical();
        assert_eq!(once, twice, "idempotent canonical for {lexical:?}^^{dt:?}");
    }
}

#[test]
fn parse_by_iri_contract() {
    // A known XSD value-space datatype IRI parses.
    let v = parse_by_iri("42", "http://www.w3.org/2001/XMLSchema#integer").unwrap();
    assert!(matches!(v, Some(XsdValue::Integer { value: 42, .. })));
    // A non-XSD datatype IRI is Ok(None) — caller treats as a plain term.
    // (XsdValue has no PartialEq by design, so assert on `is_none`.)
    assert!(parse_by_iri(
        "hi",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
    )
    .unwrap()
    .is_none());
    // An XSD datatype with a malformed lexical is Err (NOT None).
    assert!(parse_by_iri("nope", "http://www.w3.org/2001/XMLSchema#integer").is_err());
}

#[test]
fn numeric_bounds_are_hard_failed_not_saturated() {
    // i128::MAX round-trips.
    let max = i128::MAX.to_string();
    assert!(matches!(
        parse(&max, D::Integer),
        Ok(XsdValue::Integer { value, .. }) if value == i128::MAX
    ));
    // i128::MAX + 1 is a hard OutOfRange error, not a saturated value.
    let overflow = "170141183460469231731687303715884105728";
    assert!(matches!(
        parse(overflow, D::Integer),
        Err(XsdError::OutOfRange { .. })
    ));
    // NOTE: corpus-range exposure (that our actual literals stay within i128 /
    // scale-18) is proven downstream at S5 #911 / S6 #912 integration; this test
    // proves only that the bound itself hard-fails.
}

#[test]
fn partial_order_edge_cases() {
    let nan = parse("NaN", D::Double).unwrap();
    assert_eq!(value_cmp(&nan, &nan), None, "NaN is unordered");

    let p1m = parse("P1M", D::Duration).unwrap();
    let p30d = parse("P30D", D::Duration).unwrap();
    assert_eq!(value_cmp(&p1m, &p30d), None, "P1M vs P30D indeterminate");

    let no_tz = parse("2024-01-01T12:00:00", D::DateTime).unwrap();
    let tzd = parse("2024-01-01T12:00:00Z", D::DateTime).unwrap();
    assert_eq!(value_cmp(&no_tz, &tzd), None, "tz-indeterminate dateTime");

    // Cross-family is incomparable, never a silent ordering.
    let int = parse("1", D::Integer).unwrap();
    let s = parse("1", D::String).unwrap();
    assert_eq!(value_cmp(&int, &s), None);
}

#[test]
fn determinate_orderings() {
    let a = parse("1", D::Integer).unwrap();
    let b = parse("1.5", D::Decimal).unwrap();
    assert_eq!(value_cmp(&a, &b), Some(Ordering::Less)); // promotion
    let t1 = parse("2024-01-01T00:00:00Z", D::DateTime).unwrap();
    let t2 = parse("2024-01-01T01:00:00+01:00", D::DateTime).unwrap();
    assert_eq!(value_cmp(&t1, &t2), Some(Ordering::Equal)); // same instant
}

// ── Derived-integer range NEGATIVE vectors (must be Err) ────────────────────────

/// (lexical, datatype): each must parse as `Err` (out of the subtype's value space).
const INTEGER_RANGE_NEGATIVE: &[(&str, D)] = &[
    ("128", D::Byte),                          // xsd:byte max is 127
    ("-129", D::Byte),                         // xsd:byte min is -128
    ("256", D::UnsignedByte),                  // xsd:unsignedByte max is 255
    ("-1", D::UnsignedByte),                   // xsd:unsignedByte min is 0
    ("0", D::PositiveInteger),                 // xsd:positiveInteger must be >= 1
    ("0", D::NegativeInteger),                 // xsd:negativeInteger must be <= -1
    ("-1", D::NonNegativeInteger),             // xsd:nonNegativeInteger must be >= 0
    ("1", D::NonPositiveInteger),              // xsd:nonPositiveInteger must be <= 0
    ("18446744073709551616", D::UnsignedLong), // u64::MAX + 1
    ("2147483648", D::Int),                    // i32::MAX + 1
];

#[test]
fn integer_range_negative_vectors_are_hard_errors() {
    for (lexical, dt) in INTEGER_RANGE_NEGATIVE {
        assert!(
            parse(lexical, *dt).is_err(),
            "expected Err for {lexical:?}^^{dt:?} but got Ok"
        );
    }
}

// ── Derived-integer range POSITIVE (boundary) vectors (must be Ok) ──────────────

/// (lexical, datatype): each must parse successfully (boundary/edge controls).
const INTEGER_RANGE_POSITIVE: &[(&str, D)] = &[
    ("127", D::Byte),
    ("-128", D::Byte),
    ("255", D::UnsignedByte),
    ("0", D::UnsignedByte),
    ("1", D::PositiveInteger),
    ("18446744073709551615", D::UnsignedLong), // u64::MAX
];

#[test]
fn integer_range_positive_vectors_parse_ok() {
    for (lexical, dt) in INTEGER_RANGE_POSITIVE {
        assert!(
            parse(lexical, *dt).is_ok(),
            "expected Ok for {lexical:?}^^{dt:?} but got Err: {:?}",
            parse(lexical, *dt).unwrap_err()
        );
    }
}

// ── Canonical lexical sanity for derived integers ───────────────────────────────

#[test]
fn int_leading_zeros_strip_to_canonical() {
    let v = parse("007", D::Int).expect("xsd:int '007' must parse");
    assert_eq!(v.canonical_lexical(), "7", "canonical strips leading zeros");
    assert_eq!(v.datatype(), D::Int, "datatype is preserved as Int");
}

// ── Temporal calendar/time validation vectors ────────────────────────────────────

/// Negative: these must all parse as `Err` (out of calendar/time value space).
const TEMPORAL_INVALID: &[(&str, D)] = &[
    // Date: day exceeds month length
    ("2024-02-30", D::Date), // Feb has at most 29 days (2024 is leap)
    ("2023-02-29", D::Date), // 2023 is not a leap year
    ("2024-04-31", D::Date), // April has 30 days
    ("1900-02-29", D::Date), // 1900 is a century non-leap year
    // Same bad dates embedded in dateTime
    ("2024-02-30T00:00:00", D::DateTime),
    ("2023-02-29T12:00:00", D::DateTime),
    ("2024-04-31T00:00:00Z", D::DateTime),
    ("1900-02-29T00:00:00", D::DateTime),
    // Time: XSD has NO leap seconds
    ("23:59:60", D::Time),
    // Time: hour 24 only valid as 24:00:00
    ("24:30:00", D::Time),
    ("24:00:01", D::Time),
    // Time: trailing decimal point in seconds is ill-formed
    ("12:00:00.", D::Time),
    // Same bad times in dateTime
    ("2024-01-01T23:59:60", D::DateTime),
    ("2024-01-01T24:30:00", D::DateTime),
    ("2024-01-01T24:00:01", D::DateTime),
    ("2024-01-01T12:00:00.", D::DateTime),
];

/// Positive: these MUST parse successfully (boundary / edge-case controls).
const TEMPORAL_VALID: &[(&str, D)] = &[
    ("2024-02-29", D::Date),              // 2024 IS a leap year
    ("2000-02-29", D::Date),              // 2000 is a 400-year leap
    ("24:00:00", D::Time),                // end-of-day sentinel — valid
    ("23:59:59.999", D::Time),            // max valid fractional second
    ("2024-02-29T00:00:00", D::DateTime), // leap day in dateTime
    ("2000-02-29T12:00:00Z", D::DateTime),
    ("2024-01-01T24:00:00", D::DateTime), // end-of-day dateTime
];

#[test]
fn temporal_calendar_invalid_lexicals_are_hard_errors() {
    for (lexical, dt) in TEMPORAL_INVALID {
        assert!(
            parse(lexical, *dt).is_err(),
            "expected Err for {lexical:?}^^{dt:?} but got Ok"
        );
    }
}

#[test]
fn temporal_calendar_valid_lexicals_parse_ok() {
    for (lexical, dt) in TEMPORAL_VALID {
        assert!(
            parse(lexical, *dt).is_ok(),
            "expected Ok for {lexical:?}^^{dt:?} but got Err: {:?}",
            parse(lexical, *dt).unwrap_err()
        );
    }
}

// ── Year-width / XSD 1.1 year-zero vectors ──────────────────────────────────────

/// Negative: year field is >4 digits AND starts with '0' — must be Err.
const YEAR_WIDTH_INVALID: &[(&str, D)] = &[
    ("00044-03-15", D::Date),
    ("012345-01-01", D::Date),
    ("00044-03-15T00:00:00", D::DateTime),
    ("012345-01-01T00:00:00", D::DateTime),
];

/// Positive: year-zero (XSD 1.1 1 BCE), negative years, long years without leading
/// zeros, and a 4-digit leading-zero year — all must parse successfully.
const YEAR_WIDTH_VALID: &[(&str, D)] = &[
    ("0000-01-01", D::Date),   // XSD 1.1: 0000 = 1 BCE (forbidden in XSD 1.0)
    ("-0001-01-01", D::Date),  // negative year (1 BCE alt encoding / proleptic)
    ("12345-06-15", D::Date),  // 5-digit year, no leading zero — valid
    ("-12345-06-15", D::Date), // negative 5-digit year, no leading zero — valid
    ("0044-03-15", D::Date),   // exactly 4 digits with leading zero — valid
];

#[test]
fn year_width_invalid_lexicals_are_hard_errors() {
    for (lexical, dt) in YEAR_WIDTH_INVALID {
        assert!(
            parse(lexical, *dt).is_err(),
            "expected Err for {lexical:?}^^{dt:?} but got Ok"
        );
    }
}

#[test]
fn year_width_valid_lexicals_parse_ok() {
    for (lexical, dt) in YEAR_WIDTH_VALID {
        assert!(
            parse(lexical, *dt).is_ok(),
            "expected Ok for {lexical:?}^^{dt:?} but got Err: {:?}",
            parse(lexical, *dt).unwrap_err()
        );
    }
}

mod prop {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Integer canonical form re-parses to the same value and is idempotent.
        #[test]
        fn integer_canonical_idempotent(n in any::<i64>()) {
            let v = parse(&n.to_string(), D::Integer).unwrap();
            let canon = v.canonical_lexical();
            prop_assert_eq!(&canon, &parse(&canon, D::Integer).unwrap().canonical_lexical());
        }

        /// Decimal canonical form is idempotent.
        #[test]
        fn decimal_canonical_idempotent(mantissa in any::<i64>(), scale in 0u8..=6) {
            let lexical = {
                let s = mantissa.unsigned_abs().to_string();
                let sign = if mantissa < 0 { "-" } else { "" };
                let scale = scale as usize;
                if scale == 0 {
                    format!("{sign}{s}")
                } else if s.len() > scale {
                    format!("{sign}{}.{}", &s[..s.len()-scale], &s[s.len()-scale..])
                } else {
                    format!("{sign}0.{}{s}", "0".repeat(scale - s.len()))
                }
            };
            let canon = parse(&lexical, D::Decimal).unwrap().canonical_lexical();
            prop_assert_eq!(&canon, &parse(&canon, D::Decimal).unwrap().canonical_lexical());
        }
    }
}

// ── Gregorian family value-space vectors ─────────────────────────────────────────

/// Positive parse: these must all parse successfully.
const GREGORIAN_VALID: &[(&str, D)] = &[
    // gYear
    ("2024", D::GYear),
    ("-0044", D::GYear),
    ("12345", D::GYear),
    ("2024Z", D::GYear),
    ("2024+05:30", D::GYear),
    // gMonth
    ("--05", D::GMonth),
    ("--01", D::GMonth),
    ("--12", D::GMonth),
    ("--07Z", D::GMonth),
    // gDay
    ("---15", D::GDay),
    ("---31", D::GDay),
    ("---01", D::GDay),
    ("---15+05:30", D::GDay),
    // gYearMonth
    ("2024-05", D::GYearMonth),
    ("2024-05Z", D::GYearMonth),
    ("-0044-02", D::GYearMonth),
    ("12345-11", D::GYearMonth),
    // gMonthDay
    ("--02-29", D::GMonthDay), // Feb 29 is valid without a year (leap reference)
    ("--12-31", D::GMonthDay),
    ("--02-29+05:00", D::GMonthDay),
    ("--01-01", D::GMonthDay),
];

/// Negative parse: these must all produce `Err`.
const GREGORIAN_INVALID: &[(&str, D)] = &[
    // gMonth errors
    ("--13", D::GMonth),  // month 13 out of range
    ("--00", D::GMonth),  // month 00 out of range
    ("-05", D::GMonth),   // missing second dash
    ("--1", D::GMonth),   // only 1 digit
    ("---05", D::GMonth), // three dashes (gDay prefix, wrong type)
    // gDay errors
    ("---32", D::GDay),  // day 32 out of range
    ("---00", D::GDay),  // day 00 out of range
    ("--15", D::GDay),   // only 2 dashes (gMonth prefix)
    ("----15", D::GDay), // four dashes
    // gYearMonth errors
    ("2024-13", D::GYearMonth),    // month 13
    ("2024-00", D::GYearMonth),    // month 00
    ("2024-05-01", D::GYearMonth), // has day part (gYearMonth must stop at month)
    ("--2024-05", D::GYearMonth),  // starts with "--" (gMonthDay prefix, not gYearMonth)
    // gMonthDay errors
    ("--02-30", D::GMonthDay), // Feb only has 29 days (leap reference)
    ("--04-31", D::GMonthDay), // April has 30 days
    ("--13-01", D::GMonthDay), // month 13
    ("--00-15", D::GMonthDay), // month 00
    ("--02-00", D::GMonthDay), // day 00
    ("2024-05", D::GMonthDay), // no leading "--"
];

#[test]
fn gregorian_valid_lexicals_parse_ok() {
    for (lexical, dt) in GREGORIAN_VALID {
        assert!(
            parse(lexical, *dt).is_ok(),
            "expected Ok for {lexical:?}^^{dt:?} but got Err: {:?}",
            parse(lexical, *dt).unwrap_err()
        );
    }
}

#[test]
fn gregorian_invalid_lexicals_are_hard_errors() {
    for (lexical, dt) in GREGORIAN_INVALID {
        assert!(
            parse(lexical, *dt).is_err(),
            "expected Err for {lexical:?}^^{dt:?} but got Ok"
        );
    }
}

// ── Gregorian ordering tests ─────────────────────────────────────────────────────

#[test]
fn gregorian_ordering() {
    use std::cmp::Ordering;

    // gYear: 2023 < 2024
    let a = parse("2023", D::GYear).unwrap();
    let b = parse("2024", D::GYear).unwrap();
    assert_eq!(value_cmp(&a, &b), Some(Ordering::Less), "gYear 2023 < 2024");

    // gMonth: --03 < --11
    let c = parse("--03", D::GMonth).unwrap();
    let d = parse("--11", D::GMonth).unwrap();
    assert_eq!(
        value_cmp(&c, &d),
        Some(Ordering::Less),
        "gMonth --03 < --11"
    );

    // gMonthDay: --02-29 < --03-01
    let e = parse("--02-29", D::GMonthDay).unwrap();
    let f = parse("--03-01", D::GMonthDay).unwrap();
    assert_eq!(
        value_cmp(&e, &f),
        Some(Ordering::Less),
        "gMonthDay --02-29 < --03-01"
    );

    // tz-indeterminate: gYear "2024" (no tz) vs "2024Z" → None (within ±14h overlap)
    let g = parse("2024", D::GYear).unwrap();
    let h = parse("2024Z", D::GYear).unwrap();
    assert_eq!(
        value_cmp(&g, &h),
        None,
        "gYear 2024 vs 2024Z is tz-indeterminate"
    );

    // determinate pair: gYear "2020" vs "2024Z" → Some(Less)
    // 4 years apart = 4 * 365.25 * 86400 ≈ 126.2M seconds >> 14h = 50400 seconds
    let i = parse("2020", D::GYear).unwrap();
    let j = parse("2024Z", D::GYear).unwrap();
    assert_eq!(
        value_cmp(&i, &j),
        Some(Ordering::Less),
        "gYear 2020 < 2024Z (determinate)"
    );

    // cross-family: gYear "2024" vs gMonth "--05" → None
    let k = parse("2024", D::GYear).unwrap();
    let l = parse("--05", D::GMonth).unwrap();
    assert_eq!(
        value_cmp(&k, &l),
        None,
        "gYear vs gMonth is cross-family incomparable"
    );
}

// ── Gregorian canonical_lexical round-trips ──────────────────────────────────────

/// (lexical, datatype, expected canonical).
const GREGORIAN_CANONICAL: &[(&str, D, &str)] = &[
    // +00:00 normalizes to Z
    ("2024+00:00", D::GYear, "2024Z"),
    // negative year
    ("-0044", D::GYear, "-0044"),
    // gMonth
    ("--07", D::GMonth, "--07"),
    ("--07Z", D::GMonth, "--07Z"),
    // gDay
    ("---15", D::GDay, "---15"),
    ("---15+05:30", D::GDay, "---15+05:30"),
    // gYearMonth
    ("2024-05", D::GYearMonth, "2024-05"),
    ("-0044-02", D::GYearMonth, "-0044-02"),
    // gMonthDay
    ("--02-29", D::GMonthDay, "--02-29"),
    ("--12-31+05:30", D::GMonthDay, "--12-31+05:30"),
];

#[test]
fn gregorian_canonical_round_trips() {
    for (lexical, dt, expected) in GREGORIAN_CANONICAL {
        let value = parse(lexical, *dt)
            .unwrap_or_else(|e| panic!("parse({lexical:?}, {dt:?}) failed: {e}"));
        assert_eq!(
            value.canonical_lexical(),
            *expected,
            "canonical_lexical({lexical:?}^^{dt:?})"
        );
        assert_eq!(value.datatype(), *dt, "datatype({lexical:?}^^{dt:?})");
    }
}

#[test]
fn gregorian_canonical_is_idempotent() {
    for (lexical, dt, _) in GREGORIAN_CANONICAL {
        let once = parse(lexical, *dt).unwrap().canonical_lexical();
        let twice = parse(&once, *dt).unwrap().canonical_lexical();
        assert_eq!(once, twice, "idempotent canonical for {lexical:?}^^{dt:?}");
    }
}

// ── xsd:hexBinary / xsd:base64Binary value-space vectors ─────────────────────────

#[test]
fn hex_binary_canonical_vectors() {
    // "0FB7" → [0x0F, 0xB7], canonical = "0FB7".
    let v = parse("0FB7", D::HexBinary).unwrap();
    assert_eq!(v.canonical_lexical(), "0FB7");
    assert_eq!(v.datatype(), D::HexBinary);

    // Empty string is valid.
    let empty = parse("", D::HexBinary).unwrap();
    assert_eq!(empty.canonical_lexical(), "");
    assert_eq!(empty.datatype(), D::HexBinary);

    // Lowercase input → same bytes; canonical is UPPERCASE.
    let lower = parse("0fb7", D::HexBinary).unwrap();
    assert_eq!(lower.canonical_lexical(), "0FB7");
}

#[test]
fn hex_binary_case_insensitive_value_equality() {
    // Parsing "0F" and "0f" yields byte-equal values; value_cmp = Equal.
    let upper = parse("0F", D::HexBinary).unwrap();
    let lower = parse("0f", D::HexBinary).unwrap();
    assert_eq!(
        value_cmp(&upper, &lower),
        Some(Ordering::Equal),
        "\"0F\" and \"0f\" are value-equal hexBinary"
    );
}

#[test]
fn hex_binary_negative_vectors() {
    // Odd-length lexical.
    assert!(parse("0F0", D::HexBinary).is_err(), "odd-length must fail");
    // Non-hex character.
    assert!(parse("0G", D::HexBinary).is_err(), "non-hex char must fail");
    // Whitespace in lexical.
    assert!(parse("0 F", D::HexBinary).is_err(), "whitespace must fail");
    // 'z' is not a hex digit.
    assert!(parse("zz", D::HexBinary).is_err(), "'z' must fail");
}

#[test]
fn base64_binary_canonical_vectors() {
    // "TWFu" = "Man" bytes.
    let man = parse("TWFu", D::Base64Binary).unwrap();
    assert_eq!(man.canonical_lexical(), "TWFu");
    assert_eq!(man.datatype(), D::Base64Binary);

    // One-pad: "Ma".
    let ma = parse("TWE=", D::Base64Binary).unwrap();
    assert_eq!(ma.canonical_lexical(), "TWE=");

    // Two-pad: "M".
    let m = parse("TQ==", D::Base64Binary).unwrap();
    assert_eq!(m.canonical_lexical(), "TQ==");

    // Three zeros.
    let zeros = parse("AAAA", D::Base64Binary).unwrap();
    assert_eq!(zeros.canonical_lexical(), "AAAA");

    // Empty.
    let empty = parse("", D::Base64Binary).unwrap();
    assert_eq!(empty.canonical_lexical(), "");
    assert_eq!(empty.datatype(), D::Base64Binary);

    // Whitespace-tolerant: "TW Fu" same bytes as "TWFu".
    let ws = parse("TW Fu", D::Base64Binary).unwrap();
    assert_eq!(ws.canonical_lexical(), "TWFu");
    let no_ws = parse("TWFu", D::Base64Binary).unwrap();
    assert_eq!(ws.canonical_lexical(), no_ws.canonical_lexical());
}

#[test]
fn base64_binary_negative_vectors() {
    // Not a multiple of 4.
    assert!(parse("AAA", D::Base64Binary).is_err(), "len%4!=0 must fail");
    // Four pad chars.
    assert!(parse("====", D::Base64Binary).is_err(), "==== must fail");
    // Internal pad.
    assert!(
        parse("AB=C", D::Base64Binary).is_err(),
        "internal = must fail"
    );
    // Bad char.
    assert!(parse("@@@@", D::Base64Binary).is_err(), "'@' must fail");
    // Over-padding: triple pad.
    assert!(
        parse("TQ===", D::Base64Binary).is_err(),
        "triple pad must fail"
    );
}

#[test]
fn binary_canonical_is_idempotent() {
    for (lexical, dt) in [("0FB7", D::HexBinary), ("TWFu", D::Base64Binary)] {
        let once = parse(lexical, dt).unwrap().canonical_lexical();
        let twice = parse(&once, dt).unwrap().canonical_lexical();
        assert_eq!(once, twice, "idempotent canonical for {lexical:?}^^{dt:?}");
    }
}

#[test]
fn binary_parse_by_iri_contract() {
    use purrdf_xsd::XsdValue;
    let v = parse_by_iri("0FB7", "http://www.w3.org/2001/XMLSchema#hexBinary").unwrap();
    assert!(matches!(v, Some(XsdValue::Binary { .. })));
    let v2 = parse_by_iri("TWFu", "http://www.w3.org/2001/XMLSchema#base64Binary").unwrap();
    assert!(matches!(v2, Some(XsdValue::Binary { .. })));
}

// ── xsd:double extreme-magnitude canonical vectors ────────────────────────────────

/// Pairs of `(lexical, expected_canonical)` for extreme-magnitude `xsd:double` values.
const DOUBLE_EXTREME_CANONICAL: &[(&str, &str)] = &[
    // Near f64::MAX: 1e308 is a convenient large finite value; canonical adds ".0".
    ("1.0E308", "1.0E308"),
    // Smallest positive subnormal (5 × 10^-324 = f64::MIN_POSITIVE / 2^52).
    ("5.0E-324", "5.0E-324"),
    // IEEE specials: already covered by CANONICAL_VECTORS but pinned here explicitly.
    ("INF", "INF"),
    ("-INF", "-INF"),
    ("NaN", "NaN"),
];

#[test]
fn double_extreme_canonical_vectors() {
    for (lexical, expected_canonical) in DOUBLE_EXTREME_CANONICAL {
        let v = parse(lexical, D::Double)
            .unwrap_or_else(|e| panic!("parse({lexical:?}, Double) failed: {e}"));
        let canon = v.canonical_lexical();
        assert_eq!(
            canon, *expected_canonical,
            "canonical_lexical({lexical:?}^^Double)"
        );
    }
}

#[test]
fn double_extreme_canonical_roundtrip() {
    // For finite extreme doubles: parse the canonical form and confirm value-equality
    // with the original (i.e. the canonical form round-trips through parse).
    for (lexical, _) in DOUBLE_EXTREME_CANONICAL {
        let original = parse(lexical, D::Double).unwrap();
        // Skip NaN: NaN is never equal to itself (per IEEE and XSD).
        if original.canonical_lexical() == "NaN" {
            continue;
        }
        let canon = original.canonical_lexical();
        let reparsed = parse(&canon, D::Double)
            .unwrap_or_else(|e| panic!("re-parse({canon:?}, Double) failed: {e}"));
        // value_cmp(original, reparsed) == Equal asserts bit-for-bit round-trip
        // (no real bugs found in canonical_double for these extremes).
        assert_eq!(
            value_cmp(&original, &reparsed),
            Some(Ordering::Equal),
            "round-trip failed for {lexical:?}: canonical={canon:?}"
        );
    }
}
