// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact tower's cost estimates bound what the operations actually do: each
//! operation's peak working set, measured by a counting allocator, stays within a
//! small multiple of its estimate's byte bound — for the hostile shapes a short query
//! can build (a coefficient of one digit at a scale of millions, a product of a
//! short and a very long integer) as well as for ordinary ones.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_xsd::exact::{Cost, Decimal, DivisionPolicy, Integer, Rounding};
use purrdf_xsd::numeric::{
    CostOp, numeric_cost, numeric_render_cost, numeric_to_float_cost, numeric_unary_cost,
};
use purrdf_xsd::{XsdDatatype, XsdValue, numeric_total_cmp};
use std::hint::black_box;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Bytes every estimate may be off by in absolute terms: the handful of small
/// vectors an operation allocates whatever its size.
const SLACK: i64 = 4096;

/// Run `operation`, returning its result and what it allocated.
fn measured<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = CurrentThreadWindow::open();
    let result = black_box(operation());
    (result, window.close())
}

#[test]
fn full_native_parser_preserves_all_value_families_and_original_heap_owners() {
    use purrdf_lex::allocation::{Admission, Memory, StorageError};
    use purrdf_xsd::value::{ParsedValue, try_parse_with_memory};
    #[derive(Default)]
    struct ParseAccount {
        live: usize,
        peak: usize,
        limit: usize,
    }
    impl Admission for ParseAccount {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = bytes;
            self.peak = self.peak.max(bytes);
            Ok(())
        }
    }
    use XsdDatatype as D;
    let huge_integer = "7".repeat(4096);
    let very_large_integer = "7".repeat(40_000);
    let huge_decimal = format!("1.{}1", "0".repeat(4096));
    let fixtures = [
        (D::Integer, huge_integer.as_str()),
        (D::Integer, very_large_integer.as_str()),
        (D::Decimal, huge_decimal.as_str()),
        (D::Long, "-9223372036854775808"),
        (D::Int, "2147483647"),
        (D::Short, "-32768"),
        (D::Byte, "127"),
        (D::UnsignedLong, "18446744073709551615"),
        (D::UnsignedInt, "4294967295"),
        (D::UnsignedShort, "65535"),
        (D::UnsignedByte, "255"),
        (D::NonNegativeInteger, "0"),
        (D::PositiveInteger, "1"),
        (D::NonPositiveInteger, "0"),
        (D::NegativeInteger, "-1"),
        (D::Float, "+INF"),
        (D::Double, "-0.0"),
        (D::Boolean, "true"),
        (D::String, "é\ntext"),
        (D::HexBinary, "00aF"),
        (D::Base64Binary, " A P 8 = "),
        (D::DateTime, "2024-02-29T12:34:56Z"),
        (D::DateTimeStamp, "2024-02-29T12:34:56Z"),
        (D::DateTimeStamp, "2024-02-29T12:34:56+05:30"),
        (D::Date, "2024-02-29+05:30"),
        (D::Time, "12:34:56-03:00"),
        (D::Duration, "P1Y2M3DT4H5M6S"),
        (D::DayTimeDuration, "-P2DT3H"),
        (D::YearMonthDuration, "P2Y3M"),
        (D::GYear, "2024Z"),
        (D::GMonth, "--02Z"),
        (D::GDay, "---29Z"),
        (D::GYearMonth, "2024-02Z"),
        (D::GMonthDay, "--02-29Z"),
    ];
    for (datatype, lexical) in fixtures {
        let expected = purrdf_xsd::parse(lexical, datatype).unwrap();
        let mut account = ParseAccount {
            limit: 1_000_000,
            ..Default::default()
        };
        let mut memory = Memory::new(&mut account);
        let (parsed, measured) =
            measured(|| try_parse_with_memory(lexical, datatype, false, &mut memory));
        let ParsedValue::Value(value) = parsed.unwrap() else {
            panic!("valid {datatype:?}");
        };
        assert_eq!(value.datatype(), expected.datatype());
        assert_eq!(format!("{value:?}"), format!("{expected:?}"));
        assert_eq!(value.canonical_lexical(), expected.canonical_lexical());
        let live = memory.admitted_bytes();
        assert_eq!(
            usize::try_from(measured.retained_bytes).unwrap(),
            live,
            "{datatype:?}"
        );
        drop(value);
        memory.release_bytes(live).unwrap();
        assert_eq!(account.live, 0);
        assert!(
            usize::try_from(measured.peak_working_bytes).unwrap() <= account.peak,
            "{datatype:?}"
        );
    }
    let padded_integer = format!("{}123", "0".repeat(1_000));
    let zero_integer = "0".repeat(1_000);
    let padded_fraction = format!("0.{}1", "0".repeat(1_000));
    for (datatype, lexical) in [
        (D::Integer, padded_integer.as_str()),
        (D::Integer, zero_integer.as_str()),
        (D::Decimal, padded_fraction.as_str()),
    ] {
        let mut account = ParseAccount::default();
        let (answer, measurement) = measured(|| {
            try_parse_with_memory(lexical, datatype, false, &mut Memory::new(&mut account))
        });
        assert!(matches!(answer, Ok(ParsedValue::Value(_))));
        assert_eq!(measurement.allocations, 0, "{datatype:?} {lexical}");
        assert_eq!(account.peak, 0);
    }
    let invalid_suffix = format!("{zero_integer}x");
    let mut account = ParseAccount::default();
    let (answer, measurement) = measured(|| {
        try_parse_with_memory(
            &invalid_suffix,
            D::Integer,
            false,
            &mut Memory::new(&mut account),
        )
    });
    assert!(matches!(answer, Ok(ParsedValue::Invalid(_))));
    assert_eq!(measurement.allocations, 0);
    assert_eq!(account.peak, 0);
    for (datatype, lexical) in [
        (D::Integer, "invalid"),
        (D::Decimal, "invalid"),
        (D::Boolean, "invalid"),
        (D::HexBinary, "invalid"),
        (D::Base64Binary, "invalid"),
        (D::DateTime, "invalid"),
        (D::DateTimeStamp, "2024-02-29T12:34:56"),
        (D::DateTimeStamp, "2024-02-29T12:34:56+15:00"),
    ] {
        let mut account = ParseAccount::default();
        let (answer, measured) = measured(|| {
            try_parse_with_memory(lexical, datatype, false, &mut Memory::new(&mut account))
        });
        assert!(matches!(
            answer,
            Ok(ParsedValue::Invalid(Some(purrdf_xsd::ErrorCode::Forg0001)))
        ));
        assert_eq!(measured.allocations, 0);
        assert_eq!(account.live, 0);
    }
    for (datatype, lexical) in [
        (D::Integer, huge_integer.as_str()),
        (D::Integer, very_large_integer.as_str()),
        (D::Decimal, huge_decimal.as_str()),
        (D::String, "owned"),
        (D::HexBinary, "abcd"),
    ] {
        let mut account = ParseAccount::default();
        let (answer, measured) = measured(|| {
            try_parse_with_memory(lexical, datatype, false, &mut Memory::new(&mut account))
        });
        assert!(matches!(answer, Err(StorageError::AdmissionFailed)));
        assert_eq!(measured.allocations, 0);
        assert_eq!(account.live, 0);
    }
}

#[test]
fn wide_temporal_parse_admits_native_peak_and_releases_retained_owner() {
    use purrdf_lex::allocation::{Admission, Memory, StorageError};
    use purrdf_xsd::value::{ParsedValue, try_parse_with_memory};

    #[derive(Default)]
    struct Account {
        live: usize,
        peak: usize,
        limit: usize,
    }
    impl Admission for Account {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = bytes;
            self.peak = self.peak.max(bytes);
            Ok(())
        }
    }

    let wide = "9".repeat(1000);
    let samples = [
        (XsdDatatype::Time, format!("12:34:56.{}1", "0".repeat(999))),
        (
            XsdDatatype::DateTime,
            format!("2024-01-02T03:04:05.{}1Z", "0".repeat(999)),
        ),
        (
            XsdDatatype::Duration,
            format!("P{wide}DT{wide}H{wide}M1.{}1S", "0".repeat(999)),
        ),
        (XsdDatatype::DayTimeDuration, format!("P{wide}D")),
        (XsdDatatype::Duration, format!("P1DT{}S", "9".repeat(5000))),
        (XsdDatatype::Duration, format!("P1DT1{}S", "0".repeat(5000))),
        (
            XsdDatatype::Duration,
            format!("P{wide}DT1.{}1S", "0".repeat(4999)),
        ),
        (XsdDatatype::DayTimeDuration, format!("-PT{wide}.1S")),
        (
            XsdDatatype::DayTimeDuration,
            format!("-PT1{}S", "0".repeat(5000)),
        ),
    ];
    for (datatype, lexical) in samples {
        let mut account = Account {
            limit: 1_000_000,
            ..Account::default()
        };
        let measurement = {
            let mut memory = Memory::new(&mut account);
            let (result, measurement) =
                measured(|| try_parse_with_memory(&lexical, datatype, false, &mut memory));
            let ParsedValue::Value(value) = result.unwrap() else {
                panic!("valid {datatype:?}");
            };
            let retained = memory.admitted_bytes();
            assert_eq!(
                retained,
                usize::try_from(measurement.retained_bytes).unwrap(),
                "{datatype:?}"
            );
            drop(value);
            memory.release_bytes(retained).unwrap();
            measurement
        };
        let admitted = account.peak;
        assert!(
            admitted >= usize::try_from(measurement.peak_working_bytes).unwrap(),
            "{datatype:?}: admitted {admitted}, measured {}",
            measurement.peak_working_bytes
        );
        assert_eq!(account.live, 0);

        let mut refused = Account {
            limit: admitted - 1,
            ..Account::default()
        };
        let (answer, measurement) = measured(|| {
            try_parse_with_memory(&lexical, datatype, false, &mut Memory::new(&mut refused))
        });
        assert!(
            matches!(answer, Err(StorageError::AdmissionFailed)),
            "{datatype:?}: {answer:?}"
        );
        assert_eq!(measurement.allocations, 0, "{datatype:?}");
        assert_eq!(refused.live, 0);
    }

    // A malformed suffix after a very wide, otherwise valid component is a
    // lexical refusal before any numeric scratch is admitted or allocated.
    for (datatype, lexical) in [
        (XsdDatatype::Time, format!("12:34:56.{}1x", "0".repeat(999))),
        (XsdDatatype::Duration, format!("P{wide}Dx")),
    ] {
        let mut account = Account::default();
        let (answer, measurement) = measured(|| {
            try_parse_with_memory(&lexical, datatype, false, &mut Memory::new(&mut account))
        });
        assert!(
            matches!(answer, Ok(ParsedValue::Invalid(_))),
            "{datatype:?}: {answer:?}"
        );
        assert_eq!(measurement.allocations, 0, "{datatype:?}");
        assert_eq!(account.peak, 0);
    }
}

#[test]
fn temporal_comparison_admits_wide_alignment_before_allocating() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::ops::{value_cmp_admitted, value_equal_admitted};

    let tiny_tail = format!("1{}1", "0".repeat(1000));
    for (datatype, left, right) in [
        (
            XsdDatatype::Time,
            "12:00:56.1".to_owned(),
            format!("12:00:56.{tiny_tail}"),
        ),
        (
            XsdDatatype::DateTime,
            "2024-01-02T12:00:56.1Z".to_owned(),
            format!("2024-01-02T12:00:56.{tiny_tail}Z"),
        ),
        (
            XsdDatatype::DayTimeDuration,
            "PT56.1S".to_owned(),
            format!("PT56.{tiny_tail}S"),
        ),
    ] {
        let a = purrdf_xsd::parse(&left, datatype).unwrap();
        let b = purrdf_xsd::parse(&right, datatype).unwrap();
        let mut admitted = 0;
        let (order, measurement) = measured(|| {
            value_cmp_admitted(&a, &b, &mut |layout| {
                admitted = admitted.max(layout.required_bytes());
                Ok(())
            })
        });
        assert!(order.unwrap().is_some(), "{datatype:?}");
        assert!(admitted > 0, "{datatype:?}");
        assert!(
            usize::try_from(measurement.peak_working_bytes).unwrap() <= admitted,
            "{datatype:?}"
        );
        let (refused, measurement) = measured(|| {
            value_cmp_admitted(&a, &b, &mut |_| Err(LimbScratchError::AllocationFailed))
        });
        assert!(
            matches!(refused, Err(LimbScratchError::AllocationFailed)),
            "{datatype:?}"
        );
        assert_eq!(measurement.allocations, 0, "{datatype:?}");
        if datatype == XsdDatatype::DayTimeDuration {
            let (refused, measurement) = measured(|| {
                value_equal_admitted(&a, &b, &mut |_| Err(LimbScratchError::AllocationFailed))
            });
            assert!(matches!(refused, Err(LimbScratchError::AllocationFailed)));
            assert_eq!(measurement.allocations, 0);
        }
    }
}

#[test]
fn calendar_text_prepares_wide_seconds_under_original_admission() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::temporal::prepare_calendar_text;

    for (datatype, lexical) in [
        (
            XsdDatatype::Time,
            format!("12:34:56.{}1Z", "0".repeat(2000)),
        ),
        (
            XsdDatatype::DateTime,
            format!("2024-01-02T12:34:56.{}1Z", "0".repeat(2000)),
        ),
    ] {
        let value = purrdf_xsd::parse(&lexical, datatype).unwrap();
        let mut admitted = 0;
        let (prepared, measurement) = measured(|| {
            prepare_calendar_text(&value, &mut |layout| {
                admitted = admitted.max(layout.required_bytes());
                Ok(())
            })
        });
        let prepared = prepared.unwrap().unwrap();
        assert!(admitted >= prepared.temporary_bytes(), "{datatype:?}");
        assert!(
            usize::try_from(measurement.peak_working_bytes).unwrap() <= admitted,
            "{datatype:?}"
        );
        assert_eq!(prepared.to_string(), lexical);
        let (refused, measurement) = measured(|| {
            prepare_calendar_text(&value, &mut |_| Err(LimbScratchError::AllocationFailed))
        });
        assert!(matches!(refused, Err(LimbScratchError::AllocationFailed)));
        assert_eq!(measurement.allocations, 0, "{datatype:?}");
    }
}

#[test]
fn duration_subtype_cast_admits_kept_seconds_and_drops_other_component() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::temporal::{cast_duration_admitted, parse_duration};

    let wide = format!("P1Y1DT{}S", "9".repeat(2000));
    let source = parse_duration(XsdDatatype::Duration, &wide).unwrap();
    let (refused, measurement) = measured(|| {
        cast_duration_admitted(&source, XsdDatatype::DayTimeDuration, &mut |_| {
            Err(LimbScratchError::AllocationFailed)
        })
    });
    assert!(matches!(refused, Err(purrdf_xsd::XsdError::Storage(_))));
    assert_eq!(measurement.allocations, 0);

    let mut admitted = 0;
    let (kept, measurement) = measured(|| {
        cast_duration_admitted(&source, XsdDatatype::DayTimeDuration, &mut |layout| {
            admitted = admitted.max(layout.required_bytes());
            Ok(())
        })
    });
    let kept = kept.unwrap().unwrap();
    assert_eq!(kept.months(), 0);
    assert!(admitted > 0);
    assert!(usize::try_from(measurement.peak_working_bytes).unwrap() <= admitted);
    let (dropped, measurement) = measured(|| {
        cast_duration_admitted(&source, XsdDatatype::YearMonthDuration, &mut |_| {
            Err(LimbScratchError::AllocationFailed)
        })
    });
    assert_eq!(dropped.unwrap().unwrap().months(), 12);
    assert_eq!(measurement.allocations, 0);
}

#[test]
fn duration_add_sub_admit_seconds_before_allocation() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};
    use purrdf_xsd::temporal::parse_duration;

    let left = XsdValue::Duration(
        parse_duration(
            XsdDatatype::DayTimeDuration,
            &format!("PT{}.5S", "8".repeat(2000)),
        )
        .unwrap(),
    );
    let right = XsdValue::Duration(
        parse_duration(
            XsdDatatype::DayTimeDuration,
            &format!("PT{}.25S", "1".repeat(2000)),
        )
        .unwrap(),
    );
    assert!(numeric_cost(&left, &right, CostOp::Add).work() > 0);
    let tiny_left =
        XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1.5S").unwrap());
    let tiny_right =
        XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT0.25S").unwrap());
    assert_eq!(
        numeric_cost(&tiny_left, &tiny_right, CostOp::Add),
        Cost::ZERO
    );
    let edge = XsdValue::Duration(
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT{}S", i128::MAX)).unwrap(),
    );
    let one = XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1S").unwrap());
    assert!(numeric_cost(&edge, &one, CostOp::Add).work() > 0);
    let (refused, measurement) = measured(|| {
        purrdf_xsd::ops::value_binary_admitted(
            &edge,
            &one,
            NumericBinaryOperator::Add,
            DivisionPolicy::xsd_default(),
            &mut |_| Err(LimbScratchError::AllocationFailed),
        )
    });
    assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
    assert_eq!(measurement.allocations, 0);
    for op in [NumericBinaryOperator::Add, NumericBinaryOperator::Subtract] {
        let (refused, measurement) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                &left,
                &right,
                op,
                DivisionPolicy::xsd_default(),
                &mut |_| Err(LimbScratchError::AllocationFailed),
            )
        });
        assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
        assert_eq!(measurement.allocations, 0);

        let (admitted, _) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                &left,
                &right,
                op,
                DivisionPolicy::xsd_default(),
                &mut |_| Ok(()),
            )
        });
        let resident = match op {
            NumericBinaryOperator::Add => purrdf_xsd::value_add(&left, &right),
            NumericBinaryOperator::Subtract => purrdf_xsd::value_sub(&left, &right),
            _ => unreachable!(),
        };
        let admitted = admitted.unwrap();
        let resident = resident.unwrap();
        assert_eq!(admitted.datatype(), resident.datatype());
        assert_eq!(admitted.canonical_lexical(), resident.canonical_lexical());
    }
}

#[test]
fn duration_scale_and_ratio_admit_borrowed_exact_parts() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};
    use purrdf_xsd::temporal::parse_duration;

    let wide = XsdValue::Duration(
        parse_duration(
            XsdDatatype::DayTimeDuration,
            &format!("PT{}S", "7".repeat(2000)),
        )
        .unwrap(),
    );
    let two = purrdf_xsd::parse("2", XsdDatatype::Integer).unwrap();
    let wide_factor = purrdf_xsd::parse(&"7".repeat(2000), XsdDatatype::Integer).unwrap();
    let two_seconds =
        XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT2S").unwrap());
    for (left, right, op) in [
        (&two_seconds, &wide_factor, NumericBinaryOperator::Multiply),
        (&wide, &two, NumericBinaryOperator::Divide),
        (&wide, &two_seconds, NumericBinaryOperator::Divide),
    ] {
        let (refused, measurement) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                left,
                right,
                op,
                DivisionPolicy::Exact,
                &mut |_| Err(LimbScratchError::AllocationFailed),
            )
        });
        assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
        assert_eq!(measurement.allocations, 0);
        let admitted = purrdf_xsd::ops::value_binary_admitted(
            left,
            right,
            op,
            DivisionPolicy::Exact,
            &mut |_| Ok(()),
        )
        .unwrap();
        let resident = match op {
            NumericBinaryOperator::Multiply => purrdf_xsd::value_mul(left, right),
            NumericBinaryOperator::Divide => purrdf_xsd::value_div(left, right),
            _ => unreachable!(),
        }
        .unwrap();
        assert_eq!(admitted.datatype(), resident.datatype());
        assert_eq!(admitted.canonical_lexical(), resident.canonical_lexical());
    }

    let one = XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1S").unwrap());
    let three = XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT3S").unwrap());
    let exact_request = purrdf_xsd::ops::value_binary_admitted(
        &one,
        &three,
        NumericBinaryOperator::Divide,
        DivisionPolicy::Exact,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(exact_request.canonical_lexical(), "0.333333333333333333");
}

#[test]
fn duration_unary_minus_admits_wide_seconds_before_copy() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{
        NumericOperationError, NumericUnaryOperator, numeric_unary_admitted,
    };
    use purrdf_xsd::temporal::parse_duration;

    let wide = XsdValue::Duration(
        parse_duration(
            XsdDatatype::DayTimeDuration,
            &format!("PT{}S", "9".repeat(2000)),
        )
        .unwrap(),
    );
    assert!(numeric_unary_cost(&wide).work() > 0);
    let (refused, measurement) = measured(|| {
        numeric_unary_admitted(&wide, NumericUnaryOperator::Minus, &mut |_| {
            Err(LimbScratchError::AllocationFailed)
        })
    });
    assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
    assert_eq!(measurement.allocations, 0);
    let result =
        numeric_unary_admitted(&wide, NumericUnaryOperator::Minus, &mut |_| Ok(())).unwrap();
    assert_eq!(
        result.canonical_lexical(),
        purrdf_xsd::value_unary_minus(&wide)
            .unwrap()
            .canonical_lexical(),
    );
    let tiny = XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1S").unwrap());
    assert_eq!(numeric_unary_cost(&tiny), Cost::ZERO);
}

#[test]
fn calendar_duration_arithmetic_admits_exact_seconds_before_allocation() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};
    use purrdf_xsd::temporal::parse_duration;

    let fraction = "1".repeat(2000);
    let duration = XsdValue::Duration(
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT0.{fraction}S")).unwrap(),
    );
    for (datatype, lexical) in [
        (XsdDatatype::DateTime, "2024-01-02T12:34:56Z"),
        (XsdDatatype::Date, "2024-01-02Z"),
        (XsdDatatype::Time, "12:34:56Z"),
    ] {
        let calendar = purrdf_xsd::parse(lexical, datatype).unwrap();
        for op in [NumericBinaryOperator::Add, NumericBinaryOperator::Subtract] {
            let (refused, measurement) = measured(|| {
                purrdf_xsd::ops::value_binary_admitted(
                    &calendar,
                    &duration,
                    op,
                    DivisionPolicy::xsd_default(),
                    &mut |layout| {
                        if layout.required_bytes() == 0 {
                            Ok(())
                        } else {
                            Err(LimbScratchError::AllocationFailed)
                        }
                    },
                )
            });
            assert!(
                matches!(refused, Err(NumericOperationError::Storage(_))),
                "{datatype:?} {op:?}"
            );
            assert_eq!(measurement.allocations, 0, "{datatype:?} {op:?}");
            let admitted = purrdf_xsd::ops::value_binary_admitted(
                &calendar,
                &duration,
                op,
                DivisionPolicy::xsd_default(),
                &mut |_| Ok(()),
            )
            .unwrap();
            let resident = match op {
                NumericBinaryOperator::Add => purrdf_xsd::value_add(&calendar, &duration),
                NumericBinaryOperator::Subtract => purrdf_xsd::value_sub(&calendar, &duration),
                _ => unreachable!(),
            }
            .unwrap();
            assert_eq!(admitted.datatype(), resident.datatype());
            assert_eq!(admitted.canonical_lexical(), resident.canonical_lexical());
        }
    }
}

#[test]
fn named_duration_calendar_operations_use_the_exact_general_path() {
    use purrdf_xsd::temporal::{
        add_day_time_duration_to_date, add_day_time_duration_to_datetime,
        add_day_time_duration_to_time, add_duration_to_date, add_duration_to_datetime,
        add_duration_to_time, add_year_month_duration_to_date, add_year_month_duration_to_datetime,
        parse_date, parse_datetime, parse_duration, parse_time,
        subtract_day_time_duration_from_date, subtract_day_time_duration_from_datetime,
        subtract_day_time_duration_from_time, subtract_duration_from_date,
        subtract_duration_from_datetime, subtract_duration_from_time,
        subtract_year_month_duration_from_date, subtract_year_month_duration_from_datetime,
    };

    let fraction = "7".repeat(512);
    let datetime = parse_datetime(&format!("2024-01-31T12:34:56.{fraction}Z")).unwrap();
    let date = parse_date("2024-01-31Z").unwrap();
    let time = parse_time(&format!("12:34:56.{fraction}Z")).unwrap();
    let months = parse_duration(XsdDatatype::YearMonthDuration, "P1M").unwrap();
    let seconds =
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT0.{fraction}S")).unwrap();

    macro_rules! same_value {
        ($named:expr, $general:expr $(,)?) => {
            assert_eq!(
                $named.unwrap().canonical_lexical(),
                $general.unwrap().canonical_lexical(),
            )
        };
    }

    same_value!(
        add_year_month_duration_to_datetime(&datetime, &months),
        add_duration_to_datetime(&datetime, &months),
    );
    same_value!(
        subtract_year_month_duration_from_datetime(&datetime, &months),
        subtract_duration_from_datetime(&datetime, &months),
    );
    same_value!(
        add_day_time_duration_to_datetime(&datetime, &seconds),
        add_duration_to_datetime(&datetime, &seconds),
    );
    same_value!(
        subtract_day_time_duration_from_datetime(&datetime, &seconds),
        subtract_duration_from_datetime(&datetime, &seconds),
    );
    same_value!(
        add_year_month_duration_to_date(&date, &months),
        add_duration_to_date(&date, &months),
    );
    same_value!(
        subtract_year_month_duration_from_date(&date, &months),
        subtract_duration_from_date(&date, &months),
    );
    same_value!(
        add_day_time_duration_to_date(&date, &seconds),
        add_duration_to_date(&date, &seconds),
    );
    same_value!(
        subtract_day_time_duration_from_date(&date, &seconds),
        subtract_duration_from_date(&date, &seconds),
    );
    same_value!(
        add_day_time_duration_to_time(&time, &seconds),
        add_duration_to_time(&time, &seconds),
    );
    same_value!(
        subtract_day_time_duration_from_time(&time, &seconds),
        subtract_duration_from_time(&time, &seconds),
    );
    assert!(add_year_month_duration_to_datetime(&datetime, &seconds).is_err());
    assert!(add_day_time_duration_to_datetime(&datetime, &months).is_err());
}

#[test]
fn instant_difference_borrows_wide_seconds_and_admits_each_total() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};

    let fraction = "1".repeat(2000);
    for (datatype, left, right) in [
        (
            XsdDatatype::DateTime,
            format!("2024-01-02T12:34:56.{fraction}Z"),
            "2024-01-01T12:34:56Z".to_owned(),
        ),
        (
            XsdDatatype::Time,
            format!("12:34:56.{fraction}Z"),
            "12:34:55Z".to_owned(),
        ),
    ] {
        let a = purrdf_xsd::parse(&left, datatype).unwrap();
        let b = purrdf_xsd::parse(&right, datatype).unwrap();
        let (refused, measurement) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                &a,
                &b,
                NumericBinaryOperator::Subtract,
                DivisionPolicy::xsd_default(),
                &mut |layout| {
                    if layout.required_bytes() == 0 {
                        Ok(())
                    } else {
                        Err(LimbScratchError::AllocationFailed)
                    }
                },
            )
        });
        assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
        assert_eq!(measurement.allocations, 0);
        let admitted = purrdf_xsd::ops::value_binary_admitted(
            &a,
            &b,
            NumericBinaryOperator::Subtract,
            DivisionPolicy::xsd_default(),
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            admitted.canonical_lexical(),
            purrdf_xsd::value_sub(&a, &b).unwrap().canonical_lexical()
        );
    }

    let zoned = purrdf_xsd::parse(&format!("12:34:56.{fraction}Z"), XsdDatatype::Time).unwrap();
    let unzoned = purrdf_xsd::parse("12:34:56", XsdDatatype::Time).unwrap();
    let (indeterminate, measurement) = measured(|| {
        purrdf_xsd::ops::value_binary_admitted(
            &zoned,
            &unzoned,
            NumericBinaryOperator::Subtract,
            DivisionPolicy::xsd_default(),
            &mut |_| panic!("timezone mix must fail before admission"),
        )
    });
    assert!(matches!(
        indeterminate,
        Err(NumericOperationError::Value(
            purrdf_xsd::XsdError::Indeterminate { .. }
        ))
    ));
    assert_eq!(measurement.allocations, 0);
}

#[test]
fn timezone_adjustment_admits_wide_seconds_before_copy_or_shift() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::temporal::{
        adjust_datetime_to_timezone, adjust_datetime_to_timezone_admitted, adjust_time_to_timezone,
        adjust_time_to_timezone_admitted, parse_datetime, parse_time,
    };

    let fraction = "1".repeat(2000);
    let datetime = parse_datetime(&format!("2024-01-02T12:34:56.{fraction}Z")).unwrap();
    let time = parse_time(&format!("12:34:56.{fraction}Z")).unwrap();
    for offset in [None, Some(3600)] {
        let (refused, measurement) = measured(|| {
            adjust_datetime_to_timezone_admitted(&datetime, offset, &mut |layout| {
                if layout.required_bytes() == 0 {
                    Ok(())
                } else {
                    Err(LimbScratchError::AllocationFailed)
                }
            })
        });
        assert!(matches!(refused, Err(purrdf_xsd::XsdError::Storage(_))));
        assert_eq!(measurement.allocations, 0);
        let admitted =
            adjust_datetime_to_timezone_admitted(&datetime, offset, &mut |_| Ok(())).unwrap();
        assert_eq!(
            admitted.canonical_lexical(),
            adjust_datetime_to_timezone(&datetime, offset)
                .unwrap()
                .canonical_lexical()
        );

        let (refused, measurement) = measured(|| {
            adjust_time_to_timezone_admitted(&time, offset, &mut |layout| {
                if layout.required_bytes() == 0 {
                    Ok(())
                } else {
                    Err(LimbScratchError::AllocationFailed)
                }
            })
        });
        assert!(matches!(refused, Err(purrdf_xsd::XsdError::Storage(_))));
        assert_eq!(measurement.allocations, 0);
        let admitted = adjust_time_to_timezone_admitted(&time, offset, &mut |_| Ok(())).unwrap();
        assert_eq!(
            admitted.canonical_lexical(),
            adjust_time_to_timezone(&time, offset)
                .unwrap()
                .canonical_lexical()
        );
    }
}

#[test]
fn gregorian_whole_days_admits_wide_duration_before_division() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};
    use purrdf_xsd::temporal::{parse_duration, parse_gregorian};

    let calendar =
        XsdValue::Gregorian(parse_gregorian(XsdDatatype::GMonthDay, "--03-15Z").unwrap());
    let wide = XsdValue::Duration(
        parse_duration(
            XsdDatatype::DayTimeDuration,
            &format!("P{}D", "9".repeat(2000)),
        )
        .unwrap(),
    );
    for op in [NumericBinaryOperator::Add, NumericBinaryOperator::Subtract] {
        let (refused, measurement) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                &calendar,
                &wide,
                op,
                DivisionPolicy::xsd_default(),
                &mut |layout| {
                    if layout.required_bytes() == 0 {
                        Ok(())
                    } else {
                        Err(LimbScratchError::AllocationFailed)
                    }
                },
            )
        });
        assert!(matches!(refused, Err(NumericOperationError::Storage(_))));
        assert_eq!(measurement.allocations, 0);
    }
    let ordinary = XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "P1D").unwrap());
    let admitted = purrdf_xsd::ops::value_binary_admitted(
        &calendar,
        &ordinary,
        NumericBinaryOperator::Add,
        DivisionPolicy::xsd_default(),
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(
        admitted.canonical_lexical(),
        purrdf_xsd::value_add(&calendar, &ordinary)
            .unwrap()
            .canonical_lexical()
    );
}

#[test]
fn calendar_exact_work_cost_tracks_wide_seconds_and_keeps_tiny_zero() {
    use purrdf_xsd::numeric::{temporal_subtract_cost, timezone_adjust_cost};
    use purrdf_xsd::temporal::parse_duration;

    let fraction = "1".repeat(2000);
    let wide_datetime = purrdf_xsd::parse(
        &format!("2024-01-02T12:34:56.{fraction}Z"),
        XsdDatatype::DateTime,
    )
    .unwrap();
    let tiny_datetime = purrdf_xsd::parse("2024-01-02T12:34:56Z", XsdDatatype::DateTime).unwrap();
    let tiny_duration =
        XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1S").unwrap());
    let wide_duration = XsdValue::Duration(
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT0.{fraction}S")).unwrap(),
    );
    assert_eq!(
        numeric_cost(&tiny_datetime, &tiny_duration, CostOp::Add),
        Cost::ZERO
    );
    assert!(numeric_cost(&tiny_datetime, &wide_duration, CostOp::Add).work() > 0);
    assert!(numeric_cost(&wide_datetime, &tiny_duration, CostOp::Add).work() > 0);
    assert!(temporal_subtract_cost(&wide_datetime, &tiny_datetime).work() > 0);
    assert_eq!(
        temporal_subtract_cost(&tiny_datetime, &tiny_datetime),
        Cost::ZERO
    );
    assert_eq!(
        numeric_cost(&wide_datetime, &tiny_datetime, CostOp::Add),
        Cost::ZERO,
        "invalid calendar addition keeps its type error ahead of any exact-work charge"
    );
    assert_eq!(
        temporal_subtract_cost(&wide_duration, &tiny_datetime),
        Cost::ZERO,
        "invalid duration-minus-calendar keeps its type error ahead of fuel"
    );
    let max_duration = XsdValue::Duration(
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT{}S", i128::MAX)).unwrap(),
    );
    assert!(
        numeric_cost(&tiny_datetime, &max_duration, CostOp::Add).work() > 0,
        "hour/minute base plus inline maximum spills after the first seconds stage"
    );
    assert_eq!(timezone_adjust_cost(&tiny_datetime, Some(3600)), Cost::ZERO);
    assert!(timezone_adjust_cost(&wide_datetime, Some(3600)).work() > 0);
    assert!(timezone_adjust_cost(&wide_datetime, None).work() > 0);
    assert_eq!(
        timezone_adjust_cost(&wide_datetime, Some(15 * 3600)),
        Cost::ZERO,
        "invalid timezone reaches its typed XSD error before exact work"
    );
}

#[test]
fn calendar_exact_work_bounds_actual_native_traffic() {
    use purrdf_xsd::numeric::{temporal_subtract_cost, timezone_adjust_cost};
    use purrdf_xsd::temporal::{adjust_datetime_to_timezone, parse_datetime, parse_duration};

    let fraction = "1".repeat(2000);
    let wide =
        XsdValue::DateTime(parse_datetime(&format!("2024-01-02T12:34:56.{fraction}Z")).unwrap());
    let tiny = purrdf_xsd::parse("2024-01-01T12:34:56Z", XsdDatatype::DateTime).unwrap();
    let duration =
        XsdValue::Duration(parse_duration(XsdDatatype::DayTimeDuration, "PT1S").unwrap());
    let (result, allocation) = measured(|| purrdf_xsd::value_add(&wide, &duration));
    result.unwrap();
    assert_bounded(
        "calendar plus duration",
        numeric_cost(&wide, &duration, CostOp::Add),
        allocation,
    );

    let (result, allocation) = measured(|| purrdf_xsd::value_sub(&wide, &tiny));
    result.unwrap();
    assert_bounded(
        "instant difference",
        temporal_subtract_cost(&wide, &tiny),
        allocation,
    );

    let XsdValue::DateTime(datetime) = &wide else {
        unreachable!()
    };
    let (result, allocation) = measured(|| adjust_datetime_to_timezone(datetime, Some(3600)));
    result.unwrap();
    assert_bounded(
        "timezone adjustment",
        timezone_adjust_cost(&wide, Some(3600)),
        allocation,
    );
}

#[test]
fn invalid_temporal_pairs_keep_type_error_ahead_of_admission() {
    use purrdf_xsd::numeric::{NumericBinaryOperator, NumericOperationError};
    use purrdf_xsd::temporal::parse_duration;

    let fraction = "1".repeat(2000);
    let calendar = purrdf_xsd::parse(
        &format!("2024-01-02T12:34:56.{fraction}Z"),
        XsdDatatype::DateTime,
    )
    .unwrap();
    let duration = XsdValue::Duration(
        parse_duration(XsdDatatype::DayTimeDuration, &format!("PT0.{fraction}S")).unwrap(),
    );
    for (left, right, op) in [
        (&calendar, &calendar, NumericBinaryOperator::Add),
        (&duration, &calendar, NumericBinaryOperator::Subtract),
    ] {
        let (result, allocation) = measured(|| {
            purrdf_xsd::ops::value_binary_admitted(
                left,
                right,
                op,
                DivisionPolicy::xsd_default(),
                &mut |_| panic!("invalid operand pair must fail before native admission"),
            )
        });
        assert!(matches!(
            result,
            Err(NumericOperationError::Value(
                purrdf_xsd::XsdError::TypeMismatch { .. }
            ))
        ));
        assert_eq!(allocation.allocations, 0);
    }
}

#[test]
fn duration_text_prepares_wide_days_and_fraction_without_quotient_owner() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::temporal::{parse_duration, prepare_duration_text};

    for lexical in [
        format!("PT{}S", "9".repeat(3000)),
        format!("-PT{}.{:0<width$}1S", "9".repeat(1000), "", width = 2000),
        "P1Y1DT2.5S".to_owned(),
    ] {
        let source = parse_duration(XsdDatatype::Duration, &lexical).unwrap();
        let mut admitted = 0;
        let (prepared, measurement) = measured(|| {
            prepare_duration_text(&source, &mut |layout| {
                admitted = admitted.max(layout.required_bytes());
                Ok(())
            })
        });
        let prepared = prepared.unwrap();
        assert!(admitted >= prepared.temporary_bytes());
        assert!(usize::try_from(measurement.peak_working_bytes).unwrap() <= admitted);
        let text = prepared.to_string();
        let reread = parse_duration(XsdDatatype::Duration, &text).unwrap();
        assert_eq!(reread.months(), source.months());
        assert_eq!(reread.seconds_ref(), source.seconds_ref());
        let (refused, measurement) = measured(|| {
            prepare_duration_text(&source, &mut |_| Err(LimbScratchError::AllocationFailed))
        });
        assert!(matches!(refused, Err(LimbScratchError::AllocationFailed)));
        assert_eq!(measurement.allocations, 0);
    }
}

#[test]
fn admitted_operator_comparison_preserves_lossy_promotion_and_physical_refusal() {
    use purrdf_xsd::bigint::LimbScratchError;
    use purrdf_xsd::ops::{value_cmp_admitted, value_equal_admitted};
    use std::cmp::Ordering;

    let exact = big_decimal(format!("1.{}1", "0".repeat(2000)).parse().unwrap());
    for promoted in [XsdValue::Float(1.0), XsdValue::Double(1.0)] {
        let mut admitted_peak = 0;
        let (answer, measurement) = measured(|| {
            value_cmp_admitted(&exact, &promoted, &mut |layout| {
                admitted_peak = admitted_peak.max(layout.required_bytes());
                Ok(())
            })
        });
        assert_eq!(answer.unwrap(), Some(Ordering::Equal));
        assert!(
            admitted_peak > 0,
            "the native magnitude conversion allocated"
        );
        assert!(usize::try_from(measurement.peak_working_bytes).unwrap() <= admitted_peak);
        assert_eq!(measurement.retained_bytes, 0);
        assert_eq!(
            numeric_total_cmp(&exact, &promoted),
            Some(Ordering::Greater)
        );
        assert_eq!(
            value_equal_admitted(&exact, &promoted, &mut |_| Ok(())).unwrap(),
            Some(true)
        );

        let (failure, measurement) = measured(|| {
            value_cmp_admitted(&exact, &promoted, &mut |layout| {
                if layout.required_bytes() == 0 {
                    Ok(())
                } else {
                    Err(LimbScratchError::AllocationFailed)
                }
            })
        });
        assert_eq!(failure, Err(LimbScratchError::AllocationFailed));
        assert_eq!(
            measurement.allocations, 0,
            "refusal precedes the first native destination"
        );
        assert_eq!(measurement.retained_bytes, 0);
    }
}

/// Hold one operation's measured peak and allocator traffic to its estimate.
fn assert_bounded(what: &str, cost: Cost, measurement: Measurement) {
    let bound = i64::try_from(cost.bytes()).expect("a test-sized bound");
    assert!(
        measurement.peak_working_bytes <= 4 * bound + SLACK,
        "{what}: peak {} bytes against an estimate of {} bytes ({cost:?})",
        measurement.peak_working_bytes,
        cost.bytes()
    );
    // Traffic is work's shadow: an operation that copied its running total once per
    // step would request far more than its estimated work in limbs.
    let traffic = measurement.requested_bytes;
    assert!(
        traffic <= 16 * cost.work().max(cost.bytes()) + 16 * 4096,
        "{what}: {traffic} bytes requested against an estimate of {cost:?}"
    );
}

#[test]
fn wide_gap_gcd_stays_within_its_work_and_physical_cost() {
    let narrow: Integer = "9".repeat(200).parse().expect("integer");
    let wide: Integer = format!("{}{}", "9".repeat(200), "0".repeat(1_000))
        .parse()
        .expect("integer");
    let cost = wide.gcd_cost(&narrow);
    let (result, measurement) = measured(|| wide.gcd(&narrow));
    assert_eq!(result, narrow);
    assert_bounded("wide-gap gcd", cost, measurement);
}

/// `1 × 10^-scale`: one digit of coefficient, `scale` fractional digits.
fn tiny(scale: usize) -> Decimal {
    format!("0.{}1", "0".repeat(scale - 1))
        .parse()
        .expect("a decimal")
}

fn big_decimal(decimal: Decimal) -> XsdValue {
    XsdValue::from_exact_decimal(decimal)
}

/// A product of one short and one very long integer is `long / short` balanced
/// products and one pass of carries, not a copy of the running total per piece.
#[test]
fn a_lopsided_product_costs_what_it_is_charged() {
    let short: Integer = "7".repeat(64 * 9).parse().expect("an integer");
    let long: Integer = "3".repeat(20_000 * 9).parse().expect("an integer");
    let cost = short.mul_cost(&long);
    let (product, measurement) = measured(|| &short * &long);
    assert_bounded("64 × 20000 limbs", cost, measurement);
    // The product is right: (7…7)(3…3) ends in …1 and has the digits it should.
    assert_eq!(product.decimal_digits(), 64 * 9 + 20_000 * 9);
    assert!(product.canonical_lexical().ends_with('1'));
    // The neighbour: a balanced product of the same total size is bounded too.
    let half: Integer = "3".repeat(10_032 * 9).parse().expect("an integer");
    let cost = half.mul_cost(&half);
    let (_, measurement) = measured(|| &half * &half);
    assert_bounded("10032 × 10032 limbs", cost, measurement);
}

/// The staged physical layout must include the live three-way evaluation and
/// interpolation destinations, not just the final product.
#[test]
fn balanced_large_products_fit_their_admitted_peak() {
    for (digits, limbs) in [
        (2_429, 127),
        (2_448, 128),
        (2_468, 129),
        (5_000, 260),
        (9_827, 511),
        (9_846, 512),
        (9_866, 513),
        (10_000, 520),
        (19_691, 1023),
        (19_711, 1024),
        (19_730, 1025),
        (40_000, 2077),
    ] {
        let a: Decimal = "7".repeat(digits).parse().expect("integer-valued decimal");
        let b: Decimal = "3".repeat(digits).parse().expect("integer-valued decimal");
        assert_eq!(a.limb_len(), limbs);
        assert_eq!(b.limb_len(), limbs);
        let mut admitted_peak = 0;
        let (answer, measurement) = measured(|| {
            a.try_mul_admitted(&b, &mut |layout| {
                admitted_peak = admitted_peak.max(layout.required_bytes());
                Ok(())
            })
        });
        let product = answer.expect("sufficient admission");
        assert_eq!(product.canonical_lexical().len(), digits * 2);
        assert!(
            usize::try_from(measurement.peak_working_bytes).expect("test-sized peak")
                <= admitted_peak,
            "{digits} digits: measured {measurement:?}, admitted {admitted_peak}"
        );
        assert_bounded("balanced Toom product", a.mul_cost(&b), measurement);
    }
}

/// A decimal of one digit at a scale of millions renders, converts and compares at
/// the price its estimates state: the text it becomes is charged, and the float
/// conversion and the comparison with a double never form `10^scale` when the
/// value is plainly below every binary one.
#[test]
fn a_vast_scale_is_charged_where_it_is_paid() {
    let scale = 4_000_000;
    let value = big_decimal(tiny(scale));
    let render = numeric_render_cost(&value);
    assert!(render.bytes() >= scale as u64, "{render:?}");
    let (text, measurement) = measured(|| value.canonical_lexical());
    assert_eq!(text.len(), scale + 2);
    assert_bounded("rendering 1e-4000000", render, measurement);

    let to_float = numeric_to_float_cost(&value);
    assert!(
        to_float.bytes() < 64,
        "decided from the shape: {to_float:?}"
    );
    let (float, measurement) = measured(|| value.to_exact_decimal().map(|d| d.to_f64()));
    assert_eq!(float, Some(0.0));
    // `to_exact_decimal` clones the one-limb coefficient: nothing else.
    assert!(measurement.peak_working_bytes < SLACK, "{measurement:?}");

    let double = XsdValue::Double(1e-300);
    let compare = numeric_cost(&value, &double, CostOp::Compare);
    let (order, measurement) = measured(|| numeric_total_cmp(&value, &double));
    assert_eq!(order, Some(std::cmp::Ordering::Less));
    assert_bounded("1e-4000000 against 1e-300", compare, measurement);

    // Neighbour: a long fraction near a double's magnitude compares exactly, at a
    // cost linear in its digits.
    let near: Decimal = format!("1.{}1", "0".repeat(5_000))
        .parse()
        .expect("a decimal");
    let near = big_decimal(near);
    let compare = numeric_cost(&near, &XsdValue::Double(1.0), CostOp::Compare);
    let (order, measurement) = measured(|| numeric_total_cmp(&near, &XsdValue::Double(1.0)));
    assert_eq!(order, Some(std::cmp::Ordering::Greater));
    assert_bounded("1.0…01 against 1.0", compare, measurement);
}

/// Aligning two scales — a sum, a comparison of equal leading positions — is paid
/// at the gap, and a comparison whose leading positions differ costs nothing.
#[test]
fn alignment_is_charged_at_the_scale_gap() {
    let scale = 2_000_000;
    let small = big_decimal(tiny(scale));
    let one = XsdValue::Integer {
        value: Integer::from_i128(1),
        datatype: XsdDatatype::Integer,
    };
    let add = numeric_cost(&small, &one, CostOp::Add);
    assert!(add.bytes() >= scale as u64 / 3, "{add:?}");
    let (sum, measurement) = measured(|| purrdf_xsd::numeric_add(&small, &one));
    assert_bounded("1e-2000000 + 1", add, measurement);
    assert!(matches!(sum, Ok(XsdValue::Decimal(value)) if value.scale() > 18));

    let compare = numeric_cost(&small, &one, CostOp::Compare);
    assert_eq!(
        compare.bytes(),
        0,
        "the leading positions decide: {compare:?}"
    );
    let (order, measurement) = measured(|| purrdf_xsd::value_cmp(&small, &one));
    assert_eq!(order, Some(std::cmp::Ordering::Less));
    assert!(measurement.peak_working_bytes < SLACK, "{measurement:?}");
}

/// A division under a stated scale forms its shifted operand at the price
/// `div_cost` states; a bounded pair under a non-default policy runs on the tower
/// and is charged even though neither operand is big.
#[test]
fn division_is_charged_at_its_policy() {
    let one = XsdValue::Integer {
        value: Integer::from_i128(1),
        datatype: XsdDatatype::Integer,
    };
    let three = XsdValue::Integer {
        value: Integer::from_i128(3),
        datatype: XsdDatatype::Integer,
    };
    let policy = DivisionPolicy::scale(200_000, Rounding::TowardZero);
    let cost = numeric_cost(&one, &three, CostOp::Div(policy));
    assert!(cost.work() > 20_000, "{cost:?}");
    let (quotient, measurement) =
        measured(|| purrdf_xsd::numeric::numeric_div_with_policy(&one, &three, policy));
    assert_bounded("1 / 3 at 200000 digits", cost, measurement);
    let quotient = quotient.expect("a quotient");
    assert_eq!(quotient.canonical_lexical().len(), 200_002);
    // The exact-first default must also charge its tower attempt and fallback.
    let default = DivisionPolicy::xsd_default();
    let default_cost = numeric_cost(&one, &three, CostOp::Div(default));
    assert!(default_cost.work() > 0);
    let (default_result, default_measurement) =
        measured(|| purrdf_xsd::numeric::numeric_div_with_policy(&one, &three, default));
    assert_bounded(
        "1 / 3 under the exact-first default",
        default_cost,
        default_measurement,
    );
    assert_eq!(
        default_result
            .expect("default quotient")
            .canonical_lexical(),
        "0.333333333333333333"
    );
}

/// The unary functions are one pass over the coefficient.
#[test]
fn unary_functions_are_linear() {
    let long: Decimal = format!("{}.5", "9".repeat(100_000))
        .parse()
        .expect("a decimal");
    let value = big_decimal(long);
    let cost = numeric_unary_cost(&value);
    for (name, op) in [
        ("abs", purrdf_xsd::numeric_abs as fn(&XsdValue) -> _),
        ("ceiling", purrdf_xsd::numeric_ceil),
        ("floor", purrdf_xsd::numeric_floor),
        ("round", purrdf_xsd::numeric_round),
        ("negation", purrdf_xsd::numeric_unary_minus),
    ] {
        let (result, measurement) = measured(|| op(&value));
        assert!(result.is_ok(), "{name}");
        assert_bounded(name, cost, measurement);
    }
}

/// An integer past the double range converts from its length alone; one inside it
/// pays a base conversion of at most 35 limbs.
#[test]
fn a_long_integer_converts_to_a_float_from_its_length() {
    let long: Integer = format!("1{}", "0".repeat(400_000))
        .parse()
        .expect("an integer");
    assert!(
        long.to_float_cost().bytes() == 0,
        "{:?}",
        long.to_float_cost()
    );
    let ((double, single), measurement) = measured(|| (long.to_f64(), long.to_f32()));
    assert_eq!((double, single), (f64::INFINITY, f32::INFINITY));
    assert!(measurement.peak_working_bytes < SLACK, "{measurement:?}");
    let negative = -&long;
    assert_eq!(negative.to_f64(), f64::NEG_INFINITY);
    // Neighbour: 10^308 is finite and correctly rounded, at a bounded cost.
    let near: Integer = format!("1{}", "0".repeat(308)).parse().expect("an integer");
    let cost = near.to_float_cost();
    let (double, measurement) = measured(|| near.to_f64());
    assert_eq!(double, 1e308);
    assert_bounded("10^308 to f64", cost, measurement);
}

/// A final exact scale-up is part of the quotient's actual retained storage.
#[test]
fn exact_division_positive_scale_gap_covers_retained_result() {
    let divisor = Decimal::new(Integer::ONE, 10_000);
    let cost = Decimal::ONE.div_cost(&divisor, DivisionPolicy::Exact);
    let expected = format!("1{}", "0".repeat(10_000));
    let (result, measurement) = measured(|| {
        Decimal::ONE
            .div(&divisor, DivisionPolicy::Exact)
            .expect("a power-of-ten divisor terminates")
    });
    assert_eq!(result.scale(), 0);
    assert!(result.heap_bytes() <= cost.bytes(), "{cost:?}");
    assert_eq!(result.canonical_lexical(), expected);
    assert_bounded("exact positive scale gap", cost, measurement);
}

/// (3*10^2000 + 1)/3 = 10^2000 + 1/3, rounded to two places.
#[test]
fn rounded_division_prices_a_long_scaled_dividend() {
    let coefficient: Integer = format!("3{}1", "0".repeat(1_999))
        .parse()
        .expect("an integer");
    let dividend = Decimal::from_integer(coefficient);
    let divisor = Decimal::from_integer(Integer::from(3));
    let policy = DivisionPolicy::scale(2, Rounding::HalfEven);
    let cost = dividend.div_cost(&divisor, policy);
    let expected = format!("1{}.33", "0".repeat(2_000));
    let (result, measurement) =
        measured(|| dividend.div(&divisor, policy).expect("a rounded quotient"));
    assert_eq!(result.canonical_lexical(), expected);
    assert!(result.heap_bytes() <= cost.bytes(), "{cost:?}");
    // This measures the real native computation, including its temporary
    // coefficients, rather than reconstructing the estimate's formula.
    assert_bounded("long rounded dividend", cost, measurement);
}

/// 0.01/(3*10^2000 + 1) rounds to zero; its denominator still grows by 10^2.
#[test]
fn rounded_division_prices_a_long_scaled_divisor() {
    let coefficient: Integer = format!("3{}1", "0".repeat(1_999))
        .parse()
        .expect("an integer");
    let dividend = Decimal::new(Integer::ONE, 2);
    let divisor = Decimal::from_integer(coefficient);
    let policy = DivisionPolicy::scale(0, Rounding::HalfEven);
    let cost = dividend.div_cost(&divisor, policy);
    let (result, measurement) =
        measured(|| dividend.div(&divisor, policy).expect("a rounded quotient"));
    assert_eq!(result.canonical_lexical(), "0");
    assert_bounded("long rounded divisor", cost, measurement);
}
#[test]
fn temporal_native_values_and_deferred_refusals_allocate_no_heap() {
    use purrdf_xsd::temporal::{TemporalReadError, read_temporal};
    enum Expect {
        Value(&'static str),
        Invalid,
        Range,
    }
    use Expect::{Invalid, Range, Value};
    use XsdDatatype as D;

    fn check(datatype: XsdDatatype, lexical: &str, expected: Expect) {
        let (parsed, allocation) = measured(|| read_temporal(lexical, datatype));
        assert_eq!(
            allocation.requested_bytes, 0,
            "{datatype:?}: native parser traffic"
        );
        assert_eq!(
            allocation.peak_working_bytes, 0,
            "{datatype:?}: native parser peak"
        );
        let parsed = parsed.expect("supported temporal datatype");
        match (parsed, expected) {
            (Ok(value), Value(canonical)) => assert_eq!(value.canonical_lexical(), canonical),
            (Err(TemporalReadError::Invalid { .. }), Invalid)
            | (Err(TemporalReadError::OutOfRange { .. }), Range) => {}
            (actual, _) => panic!("unexpected temporal outcome: {actual:?}"),
        }
    }

    for (datatype, lexical, expected) in [
        (
            D::DateTime,
            "2026-01-01T00:00:00Z",
            Value("2026-01-01T00:00:00Z"),
        ),
        (D::Date, "2026-01-01+05:30", Value("2026-01-01+05:30")),
        (D::Time, "00:00:00Z", Value("00:00:00Z")),
        (D::Duration, "P1Y2M3DT4H5M6.5S", Value("P1Y2M3DT4H5M6.5S")),
        (D::YearMonthDuration, "P1Y", Value("P1Y")),
        (D::DayTimeDuration, "PT0S", Value("PT0S")),
        (D::GYear, "2026Z", Value("2026Z")),
        (D::GMonth, "--02Z", Value("--02Z")),
        (D::GDay, "---29Z", Value("---29Z")),
        (D::GYearMonth, "2026-02Z", Value("2026-02Z")),
        (D::GMonthDay, "--02-29Z", Value("--02-29Z")),
        (D::Time, "00:00:00+14:01", Invalid),
        (D::DateTime, "2026-02-30T00:00:00Z", Invalid),
        (D::YearMonthDuration, "P1Y1D", Invalid),
        (D::Time, "00:00:00.", Invalid),
    ] {
        check(datatype, lexical, expected);
    }

    // Construct hostile authored input outside each measured execution window.
    let zeros = "0".repeat(8192);
    check(D::Time, &format!("00:00:00.{zeros}Z"), Value("00:00:00Z"));
    let long_fraction = format!("00:00:00.{zeros}1Z");
    let (parsed, allocation) = measured(|| read_temporal(&long_fraction, D::Time));
    assert_eq!(
        allocation.requested_bytes, 0,
        "inline exact second parser traffic"
    );
    assert_eq!(
        allocation.peak_working_bytes, 0,
        "inline exact second parser peak"
    );
    let XsdValue::Time(time) = parsed.unwrap().unwrap() else {
        panic!("wide fractional seconds must remain a Time");
    };
    assert_eq!(time.second().scale(), 8193);
    assert_eq!(time.second().unscaled().as_i128(), Some(1));
    // A deferred range refusal cannot hide a later lexical failure.
    check(D::Time, &format!("25:00:00.{zeros}1Z"), Invalid);
    let wide_year = "9".repeat(8192);
    check(D::Date, &format!("{wide_year}-01-01"), Range);
    check(D::Date, &format!("{wide_year}-02-30"), Invalid);
    check(D::Duration, &format!("P{wide_year}Y"), Range);
    check(D::Duration, &format!("P{wide_year}YQ"), Invalid);
}
#[test]
fn inline_ieee_text_preserves_extreme_values_without_heap_traffic() {
    use purrdf_xsd::numeric::{canonical_double_text, canonical_float_text};
    let doubles = [
        (0.0, "0.0E0"),
        (-0.0, "-0.0E0"),
        (1.0, "1.0E0"),
        (-1.5, "-1.5E0"),
        (f64::MAX, "1.7976931348623157E308"),
        (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
        (f64::from_bits(1), "5.0E-324"),
        (-f64::from_bits(1), "-5.0E-324"),
        (f64::INFINITY, "INF"),
        (f64::NEG_INFINITY, "-INF"),
        (f64::NAN, "NaN"),
    ];
    for (value, expected) in doubles {
        let (text, allocation) = measured(|| canonical_double_text(value));
        assert_eq!(text.as_str(), expected);
        assert_eq!(text.len(), expected.len());
        assert_eq!(allocation.requested_bytes, 0);
        assert_eq!(allocation.peak_working_bytes, 0);
    }
    let singles = [
        (0.0_f32, "0.0E0"),
        (-0.0_f32, "-0.0E0"),
        (1.234_567_8_f32, "1.2345678E0"),
        (f32::MAX, "3.4028235E38"),
        (f32::MIN_POSITIVE, "1.1754944E-38"),
        (f32::from_bits(1), "1.0E-45"),
        (f32::INFINITY, "INF"),
        (f32::NEG_INFINITY, "-INF"),
        (f32::NAN, "NaN"),
    ];
    for (value, expected) in singles {
        let (text, allocation) = measured(|| canonical_float_text(value));
        assert_eq!(text.as_str(), expected);
        assert_eq!(text.len(), expected.len());
        assert_eq!(allocation.requested_bytes, 0);
        assert_eq!(allocation.peak_working_bytes, 0);
    }
}

#[test]
fn inline_ieee_append_refuses_before_mutation_and_never_grows() {
    use purrdf_xsd::numeric::{NumericRenderError, canonical_double_text};
    let text = canonical_double_text(f64::MAX);
    let mut output = String::with_capacity(text.len() + 3);
    output.push_str("rdf");
    let capacity = output.capacity();
    let (result, allocation) = measured(|| text.write_to(&mut output));
    result.expect("the admitted destination fits the full lexical");
    assert_eq!(output, "rdf1.7976931348623157E308");
    assert_eq!(output.capacity(), capacity);
    assert_eq!(allocation.requested_bytes, 0);

    // Make free capacity smaller than the lexical regardless of any allocator
    // over-allocation. Setup occurs outside the measured native operation.
    output.clear();
    while output.len() < output.capacity() {
        output.push('x');
    }
    let before = output.clone();
    let (result, allocation) = measured(|| text.write_to(&mut output));
    assert_eq!(
        result,
        Err(NumericRenderError::DestinationTooSmall {
            required_bytes: text.len(),
            available_bytes: 0,
        })
    );
    assert_eq!(output, before);
    assert_eq!(output.capacity(), capacity);
    assert_eq!(allocation.requested_bytes, 0);
    assert_eq!(allocation.peak_working_bytes, 0);
}
