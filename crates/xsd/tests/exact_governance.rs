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
    let huge_decimal = format!("1.{}1", "0".repeat(4096));
    let fixtures = [
        (D::Integer, huge_integer.as_str()),
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
        value: 1,
        datatype: XsdDatatype::Integer,
    };
    let add = numeric_cost(&small, &one, CostOp::Add);
    assert!(add.bytes() >= scale as u64 / 3, "{add:?}");
    let (sum, measurement) = measured(|| purrdf_xsd::numeric_add(&small, &one));
    assert_bounded("1e-2000000 + 1", add, measurement);
    assert!(matches!(sum, Ok(XsdValue::BigDecimal(_))));

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
        value: 1,
        datatype: XsdDatatype::Integer,
    };
    let three = XsdValue::Integer {
        value: 3,
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
    // Neighbour: the default policy over bounded operands is machine arithmetic.
    assert_eq!(
        numeric_cost(&one, &three, CostOp::Div(DivisionPolicy::xsd_default())),
        Cost::ZERO
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
    check(D::Time, &format!("00:00:00.{zeros}1Z"), Range);
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
