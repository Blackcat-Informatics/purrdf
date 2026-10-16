// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real XSD refusals consumed through the shared typed protocol alone.

use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_testkit::error_capture::Capture;
use purrdf_xsd::{XsdDatatype, parse};

struct CallerRefusal {
    limit: u128,
}

impl core::fmt::Display for CallerRefusal {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("the original caller refusal must not be formatted");
    }
}

impl core::fmt::Debug for CallerRefusal {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, formatter)
    }
}

impl StructuredError for CallerRefusal {
    purrdf_hash::error_visit! {
        @record self { limit } => "caller-refused" { "limit" => Unsigned(self.limit) }
    }
}

#[test]
fn xsd_parser_and_exact_division_retain_original_lexical_and_typed_cause() {
    for (text, datatype, expected) in [
        ("not-a-boolean", XsdDatatype::Boolean, "xsd-invalid-lexical"),
        ("300", XsdDatatype::Byte, "xsd-out-of-range"),
    ] {
        let error = parse(text, datatype).expect_err("the input is outside this value space");
        let record = Capture::read(&error);
        assert_eq!(record.code, expected);
        assert_eq!(record.events.len(), 3);
        assert!(matches!(record.field("datatype"), ErrorValue::Iri(iri) if iri == datatype.iri()));
        assert!(matches!(record.field("lexical"), ErrorValue::Text(value) if value == text));
        assert!(matches!(record.field("reason"), ErrorValue::Text(reason) if !reason.is_empty()));
    }
    let one = parse("1", XsdDatatype::Integer).expect("an exact integer");
    let three = parse("3", XsdDatatype::Integer).expect("an exact integer");
    let error = purrdf_xsd::numeric::numeric_div_with_policy(
        &one,
        &three,
        purrdf_xsd::exact::DivisionPolicy::Exact,
    )
    .expect_err("one third has no finite decimal expansion");
    let record = Capture::read(&error);
    assert_eq!(record.code, "xsd-exact");
    let cause = record.child("cause");
    assert_eq!(cause.code, "exact-non-terminating");
    assert!(cause.events.is_empty());
}

#[test]
fn actual_scratch_and_caller_admission_refusals_keep_counts_and_identity() {
    use purrdf_xsd::bigint::LimbScratch;
    use purrdf_xsd::exact::{Integer, Rational};

    let scratch = LimbScratch::new(1, 1).expect("one admitted native destination");
    let error = scratch
        .require_available(2)
        .expect_err("two destinations were not admitted");
    let record = Capture::read(&error);
    assert_eq!(record.code, "limb-scratch-destinations");
    assert!(matches!(record.field("required"), ErrorValue::Unsigned(2)));
    assert!(matches!(record.field("admitted"), ErrorValue::Unsigned(1)));
    assert_eq!(scratch.available(), 1);
    scratch
        .require_available(1)
        .expect("refusal did not consume a destination");

    let left = Rational::new(Integer::from(1), Integer::from(3)).expect("one third");
    let right = Rational::new(Integer::from(2), Integer::from(5)).expect("two fifths");
    let mut calls = 0;
    let error = left
        .compare_admitted(&right, |_, _| {
            calls += 1;
            Err(CallerRefusal { limit: u128::MAX })
        })
        .expect_err("the original caller refused before the scoped body ran");
    assert_eq!(calls, 1);
    let record = Capture::read(&error);
    assert_eq!(record.code, "rational-comparison-admission");
    let caller = record.child("cause");
    assert_eq!(caller.code, "caller-refused");
    assert!(matches!(
        caller.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
}

#[test]
fn rfc3339_refusal_reports_private_offset_and_all_original_range_fields() {
    use purrdf_xsd::rfc3339::{self, LeapSecond, Separator};

    let error = rfc3339::parse(
        "2026-13-01T00:00:00Z",
        Separator::Rfc3339Abnf,
        LeapSecond::MonthEnd,
    )
    .expect_err("month thirteen is outside RFC 3339");
    let record = Capture::read(&error);
    assert_eq!(record.code, "rfc3339-parse");
    assert!(matches!(record.field("at"), ErrorValue::ByteCount(5)));
    let kind = record.child("kind");
    assert_eq!(kind.code, "rfc3339-out-of-range");
    assert_eq!(kind.events.len(), 4);
    assert!(matches!(kind.field("field"), ErrorValue::Code("month")));
    assert!(matches!(kind.field("value"), ErrorValue::Unsigned(13)));
    assert!(matches!(kind.field("min"), ErrorValue::Unsigned(1)));
    assert!(matches!(kind.field("max"), ErrorValue::Unsigned(12)));
}

#[test]
fn constructed_environment_evidence_preserves_float_bits_without_arithmetic() {
    use purrdf_xsd::ieee::environment::{FloatEnvironmentError, FloatEnvironmentEvidence};
    use purrdf_xsd::math::MathError;

    let error = MathError::Environment(FloatEnvironmentError::DoubleRounding {
        evidence: FloatEnvironmentEvidence::Probe {
            operation: "original probe",
            expected: 0x7ff8_0000_0000_0042,
            observed: 0xfff8_0000_0000_0099,
        },
    });
    let record = Capture::read(&error);
    assert_eq!(record.code, "math-environment");
    let environment = record.child("cause");
    assert_eq!(environment.code, "float-environment-double-rounding");
    let evidence = environment.child("evidence");
    assert_eq!(evidence.code, "float-environment-probe");
    assert!(matches!(
        evidence.field("operation"),
        ErrorValue::Text("original probe")
    ));
    assert!(matches!(
        evidence.field("expected"),
        ErrorValue::Float64Bits(0x7ff8_0000_0000_0042)
    ));
    assert!(matches!(
        evidence.field("observed"),
        ErrorValue::Float64Bits(0xfff8_0000_0000_0099)
    ));
}
