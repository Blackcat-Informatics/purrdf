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
