// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The arbitrary-precision tower (`purrdf_xsd::exact`) against the exact rational
//! oracle (`purrdf_testkit::exact`), which shares no code with it: arithmetic,
//! comparison, division under every policy, rounding, conversions to and from
//! binary floating point, bounded narrowing and canonical lexical round trips,
//! over operands from one digit to hundreds and scales into the subnormal range.
//!
//! Every refusal the tower makes is executed beside a valid neighbour that must
//! still succeed.

use std::cmp::Ordering;
use std::str::FromStr;

use purrdf_testkit::exact::{Direction, Rational as Oracle};
use purrdf_testkit::prop::prelude::*;
use purrdf_xsd::exact::{
    BoundedTarget, Decimal, DivisionPolicy, ExactError, Integer, Rational, Rounding,
};
use purrdf_xsd::{XsdDatatype, numeric_add, numeric_div, numeric_mul, numeric_sub, parse};

// ----- generators ------------------------------------------------------------

/// A digit string of `len` digits drawn from `0..=9`.
fn digits(len: std::ops::RangeInclusive<usize>) -> impl Strategy<Value = String> {
    prop::collection::vec(0_u8..10, len)
        .prop_map(|digits| digits.into_iter().map(|d| char::from(b'0' + d)).collect())
}

/// Digit strings from one digit to hundreds, weighted toward the `i128` edge.
fn magnitude_digits() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => digits(1..=6),
        3 => digits(30..=45),
        2 => digits(1..=90),
        1 => digits(150..=320),
    ]
}

/// An `xsd:integer` lexical form, signed or not, leading zeros included.
fn integer_lexical() -> impl Strategy<Value = String> {
    (0_u8..3, magnitude_digits()).prop_map(|(sign, body)| {
        let sign = ["", "-", "+"][usize::from(sign)];
        format!("{sign}{body}")
    })
}

/// An `xsd:decimal` lexical form: any-length whole and fractional parts.
fn decimal_lexical() -> impl Strategy<Value = String> {
    (
        0_u8..3,
        prop_oneof![1 => Just(String::new()), 3 => magnitude_digits()],
        prop_oneof![1 => Just(String::new()), 3 => magnitude_digits()],
        any::<bool>(),
    )
        .prop_map(|(sign, whole, fraction, point)| {
            let sign = ["", "-", "+"][usize::from(sign)];
            if whole.is_empty() && fraction.is_empty() {
                return format!("{sign}0");
            }
            if fraction.is_empty() && !point {
                return format!("{sign}{whole}");
            }
            format!("{sign}{whole}.{fraction}")
        })
}

/// A nonzero `owl:rational` lexical form.
fn rational_lexical() -> impl Strategy<Value = String> {
    (integer_lexical(), magnitude_digits()).prop_map(|(numerator, denominator)| {
        let denominator = if denominator.bytes().all(|b| b == b'0') {
            format!("{denominator}7")
        } else {
            denominator
        };
        format!("{numerator}/{denominator}")
    })
}

/// A rounding direction and its oracle twin.
fn rounding() -> impl Strategy<Value = (Rounding, Direction)> {
    prop::sample::select(vec![
        (Rounding::TowardZero, Direction::TowardZero),
        (Rounding::AwayFromZero, Direction::AwayFromZero),
        (Rounding::Floor, Direction::Floor),
        (Rounding::Ceiling, Direction::Ceiling),
        (Rounding::HalfEven, Direction::HalfEven),
        (Rounding::HalfAwayFromZero, Direction::HalfAwayFromZero),
        (Rounding::HalfTowardZero, Direction::HalfTowardZero),
        (Rounding::HalfCeiling, Direction::HalfCeiling),
        (Rounding::HalfFloor, Direction::HalfFloor),
    ])
}

// ----- oracle bridges ----------------------------------------------------------

fn oracle(text: &str) -> Oracle {
    Oracle::parse(text).unwrap_or_else(|| panic!("{text} is a numeral"))
}

fn oracle_rational(value: &Rational) -> Oracle {
    oracle(&value.numerator().to_string())
        .div(&oracle(&value.denominator().to_string()))
        .expect("a positive denominator")
}

fn oracle_rational_lexical(text: &str) -> Oracle {
    let (numerator, denominator) = text.split_once('/').expect("a rational lexical");
    oracle(numerator)
        .div(&oracle(denominator))
        .expect("a nonzero denominator")
}

/// The canonical form the oracle writes for an exact value known to terminate.
fn canonical(value: &Oracle) -> String {
    value.to_canonical_decimal().expect("a terminating value")
}

fn assert_same(got: &str, expected: &Oracle, context: &str) -> Result<(), TestCaseError> {
    prop_assert_eq!(got, canonical(expected).as_str(), "{}", context);
    Ok(())
}

// ----- properties --------------------------------------------------------------

prop_test! {
    #![prop_config(Config::with_env_cases(2048))]

    /// `+ − ×`, order, equality, truncating division and the canonical form of
    /// unbounded integers agree with the oracle.
    #[test]
    fn integer_arithmetic_matches_the_oracle(a in integer_lexical(), b in integer_lexical()) {
        let (x, y) = (Integer::from_str(&a).expect("valid"), Integer::from_str(&b).expect("valid"));
        let (ox, oy) = (oracle(&a), oracle(&b));
        assert_same(&x.canonical_lexical(), &ox, "canonical")?;
        assert_same(&(&x + &y).to_string(), &ox.add(&oy), "sum")?;
        assert_same(&(&x - &y).to_string(), &ox.sub(&oy), "difference")?;
        assert_same(&(&x * &y).to_string(), &ox.mul(&oy), "product")?;
        assert_same(&(-&x).to_string(), &ox.neg(), "negation")?;
        prop_assert_eq!(x.cmp(&y), ox.cmp_value(&oy));
        prop_assert_eq!(x == y, ox.cmp_value(&oy) == Ordering::Equal);
        prop_assert_eq!(Integer::from_str(&x.canonical_lexical()).expect("canonical"), x);
        if !y.is_zero() {
            let (q, r) = x.div_rem(&y).expect("nonzero divisor");
            let exact = ox.div(&oy).expect("nonzero");
            assert_same(&q.to_string(), &exact.round_to_scale(0, Direction::TowardZero), "quotient")?;
            assert_same(&r.to_string(), &ox.sub(&oracle(&q.to_string()).mul(&oy)), "remainder")?;
        }
    }

    /// Unbounded decimals: arithmetic, order, canonical form and its round trip.
    #[test]
    fn decimal_arithmetic_matches_the_oracle(a in decimal_lexical(), b in decimal_lexical()) {
        let (x, y) = (Decimal::from_str(&a).expect("valid"), Decimal::from_str(&b).expect("valid"));
        let (ox, oy) = (oracle(&a), oracle(&b));
        assert_same(&x.canonical_lexical(), &ox, "canonical")?;
        assert_same(&(&x + &y).to_string(), &ox.add(&oy), "sum")?;
        assert_same(&(&x - &y).to_string(), &ox.sub(&oy), "difference")?;
        assert_same(&x.try_mul(&y).expect("small scales").to_string(), &ox.mul(&oy), "product")?;
        prop_assert_eq!(x.cmp(&y), ox.cmp_value(&oy));
        prop_assert_eq!(x == y, ox.cmp_value(&oy) == Ordering::Equal);
        let reparsed = Decimal::from_str(&x.canonical_lexical()).expect("canonical");
        prop_assert_eq!(&reparsed, &x);
        prop_assert_eq!(reparsed.canonical_lexical(), x.canonical_lexical());
    }

    /// Division under a scale policy is the oracle's quotient rounded once; under
    /// the exact policy it is the terminating expansion or a typed refusal.
    #[test]
    fn decimal_division_matches_the_oracle(
        a in decimal_lexical(),
        b in decimal_lexical(),
        scale in 0_u32..60,
        (mode, direction) in rounding(),
    ) {
        let (x, y) = (Decimal::from_str(&a).expect("valid"), Decimal::from_str(&b).expect("valid"));
        let (ox, oy) = (oracle(&a), oracle(&b));
        let Some(exact) = ox.div(&oy) else {
            prop_assert_eq!(x.div(&y, DivisionPolicy::scale(scale, mode)), Err(ExactError::DivisionByZero));
            prop_assert_eq!(x.div(&y, DivisionPolicy::Exact), Err(ExactError::DivisionByZero));
            return Ok(());
        };
        let rounded = x.div(&y, DivisionPolicy::scale(scale, mode)).expect("nonzero divisor");
        assert_same(&rounded.to_string(), &exact.round_to_scale(i64::from(scale), direction), "rounded quotient")?;
        match (x.div(&y, DivisionPolicy::Exact), exact.to_canonical_decimal()) {
            (Ok(value), Some(expected)) => prop_assert_eq!(value.to_string(), expected),
            (Err(ExactError::NonTerminating), None) => {}
            (got, expected) => prop_assert!(false, "exact policy {:?} vs oracle {:?}", got, expected),
        }
    }

    /// `round` at every precision, negative ones included, in every direction.
    #[test]
    fn decimal_rounding_matches_the_oracle(
        a in decimal_lexical(),
        precision in -12_i32..70,
        (mode, direction) in rounding(),
    ) {
        let x = Decimal::from_str(&a).expect("valid");
        let expected = oracle(&a).round_to_scale(i64::from(precision), direction);
        assert_same(&x.round(precision, mode).to_string(), &expected, "rounded")?;
        assert_same(
            &x.round_to_integer(mode).to_string(),
            &oracle(&a).round_to_scale(0, direction),
            "integer rounding",
        )?;
    }

    /// Exact rationals: the field operations, order and the decimal projection.
    #[test]
    fn rational_arithmetic_matches_the_oracle(
        a in rational_lexical(),
        b in rational_lexical(),
        scale in 0_u32..40,
        (mode, direction) in rounding(),
    ) {
        let (x, y) = (Rational::from_str(&a).expect("valid"), Rational::from_str(&b).expect("valid"));
        let (ox, oy) = (oracle_rational_lexical(&a), oracle_rational_lexical(&b));
        prop_assert!(oracle_rational(&x).value_eq(&ox));
        prop_assert!(!x.denominator().is_negative() && !x.denominator().is_zero());
        prop_assert_eq!(x.numerator().gcd(x.denominator()), Integer::ONE, "reduced");
        prop_assert!(oracle_rational(&(&x + &y)).value_eq(&ox.add(&oy)));
        prop_assert!(oracle_rational(&(&x - &y)).value_eq(&ox.sub(&oy)));
        prop_assert!(oracle_rational(&(&x * &y)).value_eq(&ox.mul(&oy)));
        match (x.checked_div(&y), ox.div(&oy)) {
            (Ok(q), Some(expected)) => prop_assert!(oracle_rational(&q).value_eq(&expected)),
            (Err(ExactError::DivisionByZero), None) => {}
            (got, expected) => prop_assert!(false, "{:?} vs {:?}", got, expected.is_some()),
        }
        prop_assert_eq!(x.cmp(&y), ox.cmp_value(&oy));
        prop_assert_eq!(Rational::from_str(&x.canonical_lexical()).expect("canonical"), x);
        let projected = x.to_decimal(DivisionPolicy::scale(scale, mode)).expect("nonzero");
        assert_same(&projected.to_string(), &ox.round_to_scale(i64::from(scale), direction), "projection")?;
        prop_assert_eq!(x.is_terminating(), ox.to_canonical_decimal().is_some());
        assert_same(&x.round_to_integer(mode).to_string(), &ox.round_to_scale(0, direction), "integer")?;
    }
}

/// A decimal whose magnitude reaches the binary floating-point extremes: a
/// short coefficient and a scale across the overflow, normal, subnormal and
/// underflow ranges of both formats.
fn extreme_decimal() -> impl Strategy<Value = String> {
    (any::<bool>(), magnitude_digits(), -340_i32..400).prop_map(|(negative, body, exponent)| {
        let sign = if negative { "-" } else { "" };
        if exponent >= 0 {
            let scale = usize::try_from(exponent).expect("non-negative");
            let padded = format!("{body:0>width$}", width = scale + 1);
            let split = padded.len() - scale;
            format!("{sign}{}.{}", &padded[..split], &padded[split..])
        } else {
            let zeros = "0".repeat(exponent.unsigned_abs() as usize);
            format!("{sign}{body}{zeros}")
        }
    })
}

prop_test! {
    #![prop_config(Config::with_env_cases(2048))]

    /// Every conversion to `f64`/`f32` is the oracle's single rounding, bit for
    /// bit, across overflow, subnormals and underflow.
    #[test]
    fn float_conversions_round_once(text in extreme_decimal()) {
        let value = Decimal::from_str(&text).expect("valid");
        // An exact zero has no sign (the oracle keeps the one it was written
        // with); a nonzero value that rounds to zero keeps its own.
        let exact = if oracle(&text).is_zero() { oracle("0") } else { oracle(&text) };
        prop_assert_eq!(value.to_f64().to_bits(), exact.to_f64().to_bits(), "f64 of {}", text);
        prop_assert_eq!(value.to_f32().to_bits(), exact.to_f32().to_bits(), "f32 of {}", text);
        let rational = Rational::from_decimal(&value);
        prop_assert_eq!(rational.to_f64().to_bits(), exact.to_f64().to_bits());
        prop_assert_eq!(rational.to_f32().to_bits(), exact.to_f32().to_bits());
        let integer = value.to_integer_truncated();
        let truncated = exact.round_to_scale(0, Direction::TowardZero);
        let truncated = if truncated.is_zero() { oracle("0") } else { truncated };
        prop_assert_eq!(integer.to_f64().to_bits(), truncated.to_f64().to_bits());
        prop_assert_eq!(integer.to_f32().to_bits(), truncated.to_f32().to_bits());
    }

    /// Every finite `f64` and `f32` converts to its exact decimal and rational
    /// value, and back to itself.
    #[test]
    fn float_sources_convert_exactly(bits in any::<u64>()) {
        let double = f64::from_bits(bits);
        let single = f32::from_bits((bits >> 32) as u32);
        for (value, exact, back) in [
            (Decimal::from_f64(double), double.is_finite().then(|| Oracle::from_f64(double)), double),
            (
                Decimal::from_f32(single),
                single.is_finite().then(|| Oracle::from_f32(single)),
                f64::from(single),
            ),
        ] {
            match (value, exact) {
                (Ok(decimal), Some(expected)) => {
                    assert_same(&decimal.to_string(), &expected, "exact decimal")?;
                    let zero_folded = if back == 0.0 { 0.0 } else { back };
                    prop_assert_eq!(decimal.to_f64().to_bits(), zero_folded.to_bits());
                    let rational = Rational::from_f64(back).expect("finite");
                    prop_assert!(oracle_rational(&rational).value_eq(&expected));
                }
                (Err(ExactError::NotFinite), None) => {}
                (got, expected) => prop_assert!(false, "{:?} vs {:?}", got, expected.is_some()),
            }
        }
        let truncated = Integer::from_f64_truncated(double);
        if double.is_finite() {
            let expected = Oracle::from_f64(double).round_to_scale(0, Direction::TowardZero);
            assert_same(&truncated.expect("finite").to_string(), &expected, "truncation")?;
        } else {
            prop_assert_eq!(truncated, Err(ExactError::NotFinite));
        }
    }

    /// Bounded narrowing returns the exact value exactly when the bounded type
    /// holds it, and a typed refusal otherwise — never a wrapped value.
    #[test]
    fn bounded_narrowing_is_exact_or_refused(text in decimal_lexical()) {
        let value = Decimal::from_str(&text).expect("valid");
        let exact = oracle(&text);
        // From the oracle's own canonical form: its fractional digits are the
        // scale, its digits without the point the coefficient.
        let written = canonical(&exact);
        let fits_scale = written.split_once('.').map_or(0, |(_, fraction)| fraction.len()) <= 18;
        let coefficient: String = written.chars().filter(|&c| c != '.').collect();
        let fits_mantissa = coefficient.parse::<i128>().is_ok();
        match value.to_bounded() {
            Ok(bounded) => {
                prop_assert!(fits_scale && fits_mantissa);
                prop_assert_eq!(bounded.canonical_lexical(), value.canonical_lexical());
                prop_assert_eq!(Decimal::from_bounded(&bounded), value);
            }
            Err(ExactError::OutOfRange { target: BoundedTarget::BoundedDecimal, .. }) => {
                prop_assert!(!(fits_scale && fits_mantissa));
            }
            Err(other) => prop_assert!(false, "unexpected {:?}", other),
        }
        let integer = value.to_integer_truncated();
        match integer.to_i128() {
            Ok(small) => prop_assert_eq!(small.to_string(), integer.to_string()),
            Err(error) => {
                prop_assert_eq!(error.code(), "FOCA0003");
                prop_assert!(integer.to_string().parse::<i128>().is_err());
            }
        }
    }

    /// Wherever the bounded 3.x arithmetic produces a value, the exact tower
    /// produces the same one — the property that makes the default switch a
    /// change of representation, not of answers.
    #[test]
    fn the_tower_agrees_with_the_bounded_arithmetic(a in decimal_lexical(), b in decimal_lexical()) {
        let (Ok(bx), Ok(by)) = (parse(&a, XsdDatatype::Decimal), parse(&b, XsdDatatype::Decimal)) else {
            return Ok(());
        };
        let (x, y) = (Decimal::from_str(&a).expect("valid"), Decimal::from_str(&b).expect("valid"));
        if let Ok(sum) = numeric_add(&bx, &by) {
            prop_assert_eq!(sum.to_exact_decimal().expect("decimal"), &x + &y);
        }
        if let Ok(difference) = numeric_sub(&bx, &by) {
            prop_assert_eq!(difference.to_exact_decimal().expect("decimal"), &x - &y);
        }
        let product = x.try_mul(&y).expect("small scales");
        if let Ok(bounded) = numeric_mul(&bx, &by)
            && product.scale() <= 18
        {
            prop_assert_eq!(bounded.to_exact_decimal().expect("decimal"), product);
        }
        if let Ok(bounded) = numeric_div(&bx, &by) {
            let quotient = x.div(&y, DivisionPolicy::default()).expect("nonzero");
            if quotient.to_bounded().is_ok() {
                prop_assert_eq!(bounded.to_exact_decimal().expect("decimal"), quotient);
            }
        }
    }

    /// The governor's estimate bounds the bytes every result actually holds.
    #[test]
    fn cost_estimates_bound_the_result(a in integer_lexical(), b in integer_lexical(), exp in 0_u32..40) {
        let (x, y) = (Integer::from_str(&a).expect("valid"), Integer::from_str(&b).expect("valid"));
        prop_assert!((&x + &y).heap_bytes() <= x.add_cost(&y).bytes());
        prop_assert!((&x * &y).heap_bytes() <= x.mul_cost(&y).bytes());
        prop_assert!(x.pow(exp).heap_bytes() <= x.pow_cost(exp).bytes());
        if !y.is_zero() {
            let (q, r) = x.div_rem(&y).expect("nonzero");
            prop_assert!(q.heap_bytes() + r.heap_bytes() <= x.div_rem_cost(&y).bytes());
        }
        let (dx, dy) = (Decimal::from_integer(x), Decimal::new(y, exp));
        prop_assert!((&dx + &dy).heap_bytes() <= dx.add_cost(&dy).bytes());
        if !dy.is_zero() {
            let quotient = dx.div(&dy, DivisionPolicy::default()).expect("nonzero");
            prop_assert!(quotient.heap_bytes() <= dx.div_cost(&dy, DivisionPolicy::default()).bytes());
        }
    }
}

// ----- refusals, each beside a valid neighbour ----------------------------------

#[test]
fn division_by_zero_is_refused_and_a_unit_divisor_is_not() {
    let seven = Integer::from(7_i128);
    assert_eq!(
        seven.div_rem(&Integer::ZERO),
        Err(ExactError::DivisionByZero)
    );
    assert_eq!(ExactError::DivisionByZero.code(), "FOAR0001");
    assert_eq!(seven.div_rem(&Integer::ONE), Ok((seven, Integer::ZERO)));
    let big = Integer::from_str(&"9".repeat(80)).expect("valid");
    assert_eq!(big.div_rem(&Integer::ZERO), Err(ExactError::DivisionByZero));
    assert_eq!(big.div_rem(&big), Ok((Integer::ONE, Integer::ZERO)));

    let d = Decimal::from_str("1.5").expect("valid");
    for policy in [DivisionPolicy::default(), DivisionPolicy::Exact] {
        assert_eq!(
            d.div(&Decimal::ZERO, policy),
            Err(ExactError::DivisionByZero)
        );
        assert_eq!(d.div(&Decimal::ONE, policy), Ok(d.clone()));
    }
    let tiny = Decimal::from_str("0.000000000000000000000000000000000000000001").expect("valid");
    assert_eq!(
        d.div(&tiny, DivisionPolicy::Exact)
            .expect("nonzero")
            .to_string(),
        format!("15{}", "0".repeat(41))
    );

    assert_eq!(
        Rational::new(Integer::ONE, Integer::ZERO),
        Err(ExactError::DivisionByZero)
    );
    assert_eq!(Rational::new(Integer::ONE, Integer::ONE), Ok(Rational::ONE));
    assert_eq!(Rational::from_str("1/0"), Err(ExactError::DivisionByZero));
    assert_eq!(Rational::from_str("0/1"), Ok(Rational::ZERO));
    assert_eq!(Rational::ZERO.recip(), Err(ExactError::DivisionByZero));
    assert_eq!(Rational::ONE.recip(), Ok(Rational::ONE));
    assert_eq!(
        Rational::ONE.checked_div(&Rational::ZERO),
        Err(ExactError::DivisionByZero)
    );
    assert_eq!(Rational::ONE.checked_div(&Rational::ONE), Ok(Rational::ONE));
}

#[test]
fn narrowing_refuses_one_past_each_bound_and_accepts_the_bound() {
    let max = Integer::from_i128(i128::MAX);
    let min = Integer::from_i128(i128::MIN);
    assert_eq!(max.to_i128(), Ok(i128::MAX));
    assert_eq!(min.to_i128(), Ok(i128::MIN));
    let above = &max + &Integer::ONE;
    let below = &min - &Integer::ONE;
    for refused in [&above, &below] {
        let error = refused.to_i128().expect_err("out of range");
        assert_eq!(error.code(), "FOCA0003");
        assert!(matches!(
            error,
            ExactError::OutOfRange {
                target: BoundedTarget::I128,
                ..
            }
        ));
    }
    // Coming back into range restores the inline value.
    assert_eq!((&above - &Integer::ONE).as_i128(), Some(i128::MAX));
    assert_eq!(Integer::from(i64::MAX).to_i64(), Ok(i64::MAX));
    assert_eq!(
        (&Integer::from(i64::MAX) + &Integer::ONE)
            .to_i64()
            .map_err(|e| e.code()),
        Err("FOCA0003")
    );

    let at_scale = Decimal::from_str("0.000000000000000001").expect("valid");
    assert_eq!(
        at_scale.to_bounded().expect("scale 18").canonical_lexical(),
        "0.000000000000000001"
    );
    let past_scale = Decimal::from_str("0.0000000000000000001").expect("valid");
    assert_eq!(
        past_scale.to_bounded().err().map(|e| e.code()),
        Some("FOCA0001")
    );
    // Trailing zeros are not significant: scale 30 written, scale 1 meant.
    let padded = Decimal::from_str("2.500000000000000000000000000000").expect("valid");
    assert_eq!(
        padded
            .to_bounded()
            .expect("canonical scale 1")
            .canonical_lexical(),
        "2.5"
    );
    let huge = Decimal::from_integer(Integer::from_i128(i128::MAX));
    assert!(huge.to_bounded().is_ok());
    let huger = &huge + &Decimal::ONE;
    assert_eq!(huger.to_bounded().err().map(|e| e.code()), Some("FOCA0001"));
    let negative_edge = Decimal::from_integer(Integer::from_i128(i128::MIN));
    assert!(negative_edge.to_bounded().is_ok());
    assert!((&negative_edge - &Decimal::ONE).to_bounded().is_err());
    // The rounded neighbour of a refused value narrows.
    let rounded = past_scale.round(18, Rounding::HalfEven);
    assert_eq!(
        rounded.to_bounded().expect("scale 18").canonical_lexical(),
        "0"
    );

    let bounded = Rational::new(Integer::from_i128(i128::MAX), Integer::from(2_i128)).expect("ok");
    assert!(bounded.to_bounded().is_ok());
    let unbounded = Rational::new(
        &Integer::from_i128(i128::MAX) + &Integer::from(2_i128),
        Integer::from(2_i128),
    )
    .expect("ok");
    assert_eq!(
        unbounded.to_bounded().err().map(|e| e.code()),
        Some("FOAR0002")
    );
}

#[test]
fn non_finite_sources_are_refused_and_finite_extremes_are_not() {
    for refused in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Decimal::from_f64(refused), Err(ExactError::NotFinite));
        assert_eq!(Rational::from_f64(refused), Err(ExactError::NotFinite));
        assert_eq!(
            Integer::from_f64_truncated(refused),
            Err(ExactError::NotFinite)
        );
    }
    assert_eq!(ExactError::NotFinite.code(), "FOCA0002");
    for accepted in [
        f64::MAX,
        f64::MIN,
        f64::from_bits(1),
        -f64::from_bits(1),
        0.0,
        -0.0,
    ] {
        let decimal = Decimal::from_f64(accepted).expect("finite");
        assert_eq!(decimal.to_f64().to_bits(), (accepted + 0.0).to_bits());
        assert!(Integer::from_f64_truncated(accepted).is_ok());
    }
    assert_eq!(
        Decimal::from_f64(f64::MAX)
            .expect("finite")
            .to_string()
            .len(),
        309
    );
    assert_eq!(
        Decimal::from_f64(f64::from_bits(1))
            .expect("finite")
            .scale(),
        1074
    );
    assert!(Decimal::from_f32(f32::NAN).is_err());
    assert_eq!(
        Decimal::from_f32(0.1).expect("finite").to_string(),
        "0.100000001490116119384765625"
    );
}

#[test]
fn non_terminating_quotients_are_refused_and_terminating_ones_are_exact() {
    let one = Decimal::ONE;
    assert_eq!(
        one.div(&Decimal::from(3), DivisionPolicy::Exact),
        Err(ExactError::NonTerminating)
    );
    assert_eq!(ExactError::NonTerminating.code(), "FOAR0002");
    assert_eq!(
        one.div(&Decimal::from(8), DivisionPolicy::Exact)
            .expect("terminates")
            .to_string(),
        "0.125"
    );
    let deep = one
        .div(
            &Decimal::from_integer(Integer::from(2_i128).pow(70)),
            DivisionPolicy::Exact,
        )
        .expect("terminates");
    assert_eq!(deep.scale(), 70);
    assert_eq!(
        Rational::from_decimal(&deep),
        Rational::new(Integer::ONE, Integer::from(2_i128).pow(70)).expect("ok")
    );
    // 3/6 reduces to 1/2 before the expansion is decided.
    assert_eq!(
        Decimal::from(3)
            .div(&Decimal::from(6), DivisionPolicy::Exact)
            .expect("1/2")
            .to_string(),
        "0.5"
    );
    let third = Rational::new(Integer::ONE, Integer::from(3_i128)).expect("ok");
    assert!(!third.is_terminating());
    assert_eq!(
        third.to_decimal(DivisionPolicy::Exact),
        Err(ExactError::NonTerminating)
    );
    assert_eq!(
        third
            .to_decimal(DivisionPolicy::scale(4, Rounding::HalfEven))
            .expect("rounded")
            .to_string(),
        "0.3333"
    );
    // The point of the rational: (1/3) × 3 is exactly one; through the default
    // decimal policy it is not.
    assert_eq!(
        &third * &Rational::from_integer(Integer::from(3_i128)),
        Rational::ONE
    );
    let decimal_third = one
        .div(&Decimal::from(3), DivisionPolicy::default())
        .expect("rounded");
    assert_eq!(
        decimal_third
            .try_mul(&Decimal::from(3))
            .expect("ok")
            .to_string(),
        "0.999999999999999999"
    );
}

#[test]
fn lexical_refusals_have_valid_neighbours() {
    for refused in [
        "", "+", "-", "1.2.3", "1e5", " 1", "1 ", "0x10", "\u{0661}", "--1", "1/2",
    ] {
        assert!(
            matches!(
                Decimal::from_str(refused),
                Err(ExactError::InvalidLexical { .. })
            ),
            "{refused:?}"
        );
        assert_eq!(
            Decimal::from_str(refused).map_err(|e| e.code()),
            Err("FORG0001")
        );
    }
    for accepted in [".5", "1.", "+0", "-0", "-.0", "007.700", "0000"] {
        assert!(Decimal::from_str(accepted).is_ok(), "{accepted:?}");
    }
    assert_eq!(Decimal::from_str("-0.0").expect("valid").to_string(), "0");
    assert_eq!(
        Decimal::from_str("007.700").expect("valid").to_string(),
        "7.7"
    );
    for refused in ["", "1.0", "+", "1_000", "１"] {
        assert!(Integer::from_str(refused).is_err(), "{refused:?}");
    }
    for accepted in ["+0", "-0", "0001", &"9".repeat(500)] {
        assert!(Integer::from_str(accepted).is_ok(), "{accepted:?}");
    }
    for refused in ["1/-2", "1/", "/2", "1.5/2", "1/2/3", " 1/2", "1/ 2"] {
        assert!(
            matches!(
                Rational::from_str(refused),
                Err(ExactError::InvalidLexical { .. })
            ),
            "{refused:?}"
        );
    }
    for accepted in ["-1/2", "+1/2", "0/5", "6/4"] {
        assert!(Rational::from_str(accepted).is_ok(), "{accepted:?}");
    }
    assert_eq!(
        Rational::from_str("6/-4").map_err(|e| e.code()),
        Err("FORG0001")
    );
    assert_eq!(
        Rational::from_str("-6/4").expect("valid").to_string(),
        "-3/2"
    );
}

#[test]
fn scale_overflow_is_refused_and_a_representable_product_is_not() {
    let fine = Decimal::new(Integer::ONE, u32::MAX);
    assert_eq!(fine.scale(), u32::MAX);
    assert_eq!(fine.try_mul(&fine), Err(ExactError::ScaleOverflow));
    assert_eq!(
        fine.try_mul(&Decimal::from(7)).expect("same scale").scale(),
        u32::MAX
    );
    assert!(fine < Decimal::from_str("0.000001").expect("valid"));
    assert!(fine > Decimal::ZERO);
}

#[test]
fn power_estimates_track_the_actual_size() {
    // The estimator is a refusal surface: an estimate far above the real size
    // refuses a valid query. Hold it within a few percent of the result.
    for (base, exp) in [
        (2_i128, 100_000_u32),
        (10, 9_000),
        (3, 20_001),
        (999_999_999_999, 977),
        (7, 1),
    ] {
        let value = Integer::from(base);
        let estimate = value.pow_cost(exp).bytes();
        let actual = value.pow(exp).heap_bytes().max(4);
        assert!(estimate >= actual, "{base}^{exp}: {estimate} < {actual}");
        assert!(
            estimate <= actual + actual / 25 + 32,
            "{base}^{exp}: {estimate} vs {actual}"
        );
    }
    let big = Integer::from_str(&"9".repeat(100)).expect("valid");
    let estimate = big.pow_cost(50).bytes();
    let actual = big.pow(50).heap_bytes();
    assert!(
        estimate >= actual && estimate <= actual + actual / 25 + 32,
        "{estimate} vs {actual}"
    );
}

#[test]
fn zero_rounds_to_zero_in_every_direction_at_every_precision() {
    // A directed rounding increments only an inexact value; zero is exact at
    // any grid, the coarse negative-precision ones included.
    for mode in [
        Rounding::AwayFromZero,
        Rounding::Ceiling,
        Rounding::Floor,
        Rounding::HalfEven,
    ] {
        for precision in [-12, -1, 0, 3] {
            assert_eq!(
                Decimal::ZERO.round(precision, mode),
                Decimal::ZERO,
                "{mode:?} {precision}"
            );
        }
        // The neighbour: a tiny nonzero value away from zero does move.
        let tiny = Decimal::from_str("0.001").expect("valid");
        if mode == Rounding::AwayFromZero || mode == Rounding::Ceiling {
            assert_eq!(tiny.round(-12, mode).to_string(), "1000000000000");
        }
    }
}

#[test]
fn factorials_past_i128_are_exact() {
    let factorial = (1..=34_i128).fold(Integer::ONE, |acc, k| &acc * &Integer::from(k));
    assert_eq!(
        factorial.to_string(),
        "295232799039604140847618609643520000000"
    );
    let hundred = (1..=100_i128).fold(Integer::ONE, |acc, k| &acc * &Integer::from(k));
    assert_eq!(hundred.decimal_digits(), 158);
    assert_eq!(hundred.to_bigint().trailing_decimal_zeros(), 24);
    let (quotient, remainder) = hundred.div_rem(&factorial).expect("nonzero");
    assert_eq!(remainder, Integer::ZERO);
    assert_eq!(&quotient * &factorial, hundred);
}
