// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Oracle tests for the exact fixed-point arithmetic.
//!
//! These live in an integration test rather than beside the code because the
//! crate root denies `clippy::float_arithmetic`, and that denial reaches its own
//! `#[cfg(test)]` modules. The denial is the point — a float must not enter the
//! library — but an ORACLE has to be written in the arithmetic it is checking
//! against, so the `f64` comparison is done from out here, where the crate
//! boundary puts it outside the lint's reach without weakening it.
//!
//! The oracle's role is bounded, and worth stating: `f64::ln` is not the ground
//! truth this crate defers to. It is a second, independent implementation used
//! to catch a gross error in the series — a wrong constant, an off-by-one in the
//! range reduction, a sign inversion. Where the two disagree in the last few
//! digits, the fixed-point answer is the one this crate ships, because it is the
//! one that is the same on every target.

use purrdf_text::{Fixed, SCALE_DIGITS, TextError};

/// `10^SCALE_DIGITS`, for turning a decimal into a raw fixed-point integer.
const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

/// Read a [`Fixed`] as an `f64` by parsing its own exact decimal lexical form.
///
/// Deliberately routed through the lexical form rather than through
/// `into_raw()`: the lexical form is the crate's published rendering, so a bug
/// in it would show up here rather than being bypassed.
fn as_f64(value: Fixed) -> f64 {
    value
        .to_decimal_lexical()
        .parse::<f64>()
        .expect("the lexical form is always a parseable decimal")
}

/// How many points [`sweep`] visits. A constant rather than a floor, because
/// the count is what makes the cross-check dense and a test that accepts "at
/// least most of them" cannot tell a dense sweep from a decimated one.
const POINTS: u32 = 256;

/// The `x` values the oracle sweep visits: a geometric ladder across
/// `[0.001, 100000]`, which spans both signs of the logarithm and roughly 27
/// binades — so every branch of the range reduction is exercised many times.
fn sweep() -> Vec<Fixed> {
    let low = 0.001_f64.ln();
    let high = 100_000.0_f64.ln();
    (0..POINTS)
        .map(|i| {
            let t = f64::from(i) / f64::from(POINTS - 1);
            let x = (high - low).mul_add(t, low).exp();
            // `x` only chooses which point to test; the fixed-point value it
            // becomes is the actual input, and it is what the oracle is then
            // asked about.
            Fixed::from_raw((x * 1e12_f64) as i128)
        })
        .collect()
}

#[test]
fn ln_agrees_with_the_f64_oracle() {
    let points = sweep();
    assert_eq!(
        points.len(),
        POINTS as usize,
        "the sweep is a fixed ladder, so its length is knowable exactly; a floor here would be \
         satisfied by a decimated sweep"
    );
    for x in points {
        let ours = as_f64(x.ln().expect("every swept value is positive"));
        let theirs = as_f64(x).ln();
        let difference = (ours - theirs).abs();
        assert!(
            difference < 1e-10,
            "ln({}) came out as {ours} but the oracle says {theirs} (difference {difference})",
            x.to_decimal_lexical()
        );
    }
}

/// The determinism property, asserted on the raw integer rather than on any
/// rendering: a thousand evaluations of the same input must be bit-identical.
#[test]
fn ln_is_a_pure_function() {
    for x in [
        Fixed::from_raw(1),
        Fixed::from_raw(SCALE / 3),
        Fixed::from_raw(SCALE),
        Fixed::from_raw(3 * SCALE + 1),
        Fixed::from_raw(999_999_999_999_999_999),
    ] {
        let first = x.ln().expect("positive").into_raw();
        for _ in 0..1_000 {
            assert_eq!(
                x.ln().expect("positive").into_raw(),
                first,
                "ln({}) is not a pure function of its input",
                x.to_decimal_lexical()
            );
        }
    }
}

#[test]
fn ln_of_one_is_zero() {
    assert_eq!(Fixed::ONE.ln().expect("1 is positive"), Fixed::ZERO);
}

/// `e` truncated to twelve fractional digits is a hair below the real `e`, so
/// its logarithm is a hair BELOW one — within a few units in the last place,
/// which is the fixed-point epsilon.
///
/// The direction is the interesting half of that claim and is asserted rather
/// than thrown away by an absolute value: `ln` of a value below `e` that came
/// out above one would be a sign error the magnitude bound could not see.
#[test]
fn ln_of_e_is_one() {
    let e = Fixed::from_raw(2_718_281_828_459);
    let ln_e = e.ln().expect("e is positive").into_raw();
    assert!(
        ln_e < SCALE,
        "the input is below e, so its logarithm must be below one; got {ln_e} against {SCALE}"
    );
    let distance = SCALE - ln_e;
    assert!(
        distance <= 10,
        "ln(e) came out {distance} raw units from 1 ({ln_e} against {SCALE})"
    );
}

#[test]
fn ln_of_zero_and_negative_are_domain_errors() {
    for x in [
        Fixed::ZERO,
        Fixed::from_raw(-1),
        Fixed::from_integer(-7).expect("representable"),
        Fixed::from_raw(i128::MIN),
    ] {
        assert!(
            matches!(x.ln(), Err(TextError::Domain(_))),
            "ln({}) must be a domain error",
            x.to_decimal_lexical()
        );
    }
}

#[test]
fn to_decimal_lexical_is_exact_and_fixed_width() {
    let cases = [
        (Fixed::ZERO, "0.000000000000"),
        (Fixed::ONE, "1.000000000000"),
        (Fixed::from_raw(1), "0.000000000001"),
        (Fixed::from_raw(-1), "-0.000000000001"),
        (Fixed::from_raw(SCALE / 2), "0.500000000000"),
        (Fixed::from_raw(-(SCALE / 2)), "-0.500000000000"),
        (Fixed::from_raw(-(3 * SCALE) - 1), "-3.000000000001"),
    ];
    for (value, expected) in cases {
        let rendered = value.to_decimal_lexical();
        assert_eq!(
            rendered,
            expected,
            "raw {} rendered wrongly",
            value.into_raw()
        );
        let (_, fraction) = rendered
            .split_once('.')
            .expect("the lexical form always has a decimal point");
        assert_eq!(
            fraction.len(),
            SCALE_DIGITS as usize,
            "`{rendered}` does not carry exactly {SCALE_DIGITS} fractional digits"
        );
    }
}

/// Multiplying and then dividing by the same value returns the original, up to
/// the units in the last place the two truncations can cost.
///
/// The bound is TWO, one per truncation, and it is stated as two here rather
/// than as "a single unit" because two is what the operation can actually
/// reach: `checked_mul` truncates once and `checked_div` truncates again, and
/// the two errors can point the same way. The attained maximum is asserted as
/// well as the bound — a drift ceiling nothing reaches is a ceiling that has
/// never been tested, and it would sit unchanged if the arithmetic became
/// exact or became four times worse in a case this table does not hold.
#[test]
fn checked_mul_and_div_round_trip() {
    let values = [
        Fixed::from_raw(SCALE),
        Fixed::from_raw(3 * SCALE + 500_000_000_000),
        Fixed::from_raw(-(7 * SCALE) - 1),
        Fixed::from_raw(123_456_789_012_345),
    ];
    let mut worst = 0_i128;
    for a in values {
        for b in values {
            let round_trip = a
                .checked_mul(b)
                .expect("no overflow")
                .checked_div(b)
                .expect("b is never zero");
            let drift = (round_trip.into_raw() - a.into_raw()).abs();
            assert!(
                drift <= 2,
                "({} * {}) / {} drifted {drift} raw units",
                a.to_decimal_lexical(),
                b.to_decimal_lexical(),
                b.to_decimal_lexical()
            );
            worst = worst.max(drift);
        }
    }
    assert_eq!(
        worst, 1,
        "this table's worst round trip is one unit in the last place; a change to that number is \
         a change to the arithmetic and must be seen rather than absorbed by the ceiling"
    );
}

/// Overflow is reported, never wrapped or saturated — in every operation that
/// can reach it.
#[test]
fn overflow_is_a_hard_error_not_a_wrap() {
    let biggest = Fixed::from_raw(i128::MAX);
    let smallest = Fixed::from_raw(i128::MIN);
    let two = Fixed::from_integer(2).expect("representable");

    assert!(
        matches!(biggest.checked_add(Fixed::ONE), Err(TextError::Overflow(_))),
        "addition past the top must be an error"
    );
    assert!(
        matches!(
            smallest.checked_sub(Fixed::ONE),
            Err(TextError::Overflow(_))
        ),
        "subtraction past the bottom must be an error"
    );
    assert!(
        matches!(biggest.checked_mul(two), Err(TextError::Overflow(_))),
        "a product past the top must be an error"
    );
    assert!(
        matches!(
            biggest.checked_div(Fixed::from_raw(1)),
            Err(TextError::Overflow(_))
        ),
        "a quotient past the top must be an error"
    );
}

/// The neighbouring cases the four refusals above must not have swallowed:
/// the same four operations, one unit inside the range, must SUCCEED — and must
/// return the exact value, not merely an `Ok`.
///
/// A refusal test with no successful neighbour is satisfied by an
/// implementation that refuses everything, and this one was: `signed` decided
/// representability with `i128::try_from`, a positive-range test, so the
/// magnitude `2^127` was refused even carrying a negative sign — where it is
/// exactly [`i128::MIN`]. Multiplying [`i128::MIN`] by **one** came back an
/// overflow. The extremes are therefore exercised from the succeeding side
/// here, at both ends and in both directions.
#[test]
fn the_range_extremes_are_admitted_not_only_refused() {
    let biggest = Fixed::from_raw(i128::MAX);
    let smallest = Fixed::from_raw(i128::MIN);

    // Addition and subtraction have no successful test anywhere else, so the
    // Ok path is pinned here rather than assumed from the failing one.
    assert_eq!(
        Fixed::from_raw(i128::MAX - 1)
            .checked_add(Fixed::from_raw(1))
            .expect("i128::MAX is representable"),
        biggest
    );
    assert_eq!(
        Fixed::from_raw(i128::MIN + 1)
            .checked_sub(Fixed::from_raw(1))
            .expect("i128::MIN is representable"),
        smallest
    );
    assert_eq!(
        Fixed::ONE
            .checked_add(Fixed::ONE)
            .expect("two is representable"),
        Fixed::from_integer(2).expect("representable")
    );
    assert_eq!(
        Fixed::ONE
            .checked_sub(Fixed::from_integer(3).expect("representable"))
            .expect("minus two is representable"),
        Fixed::from_integer(-2).expect("representable")
    );

    // The asymmetric extreme: `i128::MIN`'s magnitude is one past `i128::MAX`,
    // and it is representable only because it is negative. Multiplying and
    // dividing it by one must return it unchanged.
    assert_eq!(
        smallest
            .checked_mul(Fixed::ONE)
            .expect("multiplying the negative extreme by one is the extreme"),
        smallest
    );
    assert_eq!(
        smallest
            .checked_div(Fixed::ONE)
            .expect("dividing the negative extreme by one is the extreme"),
        smallest
    );
    assert_eq!(
        Fixed::ONE
            .checked_mul(smallest)
            .expect("the same product with the operands swapped"),
        smallest
    );

    // And the positive extreme, which has no such asymmetry, still round-trips.
    assert_eq!(
        biggest
            .checked_mul(Fixed::ONE)
            .expect("multiplying the positive extreme by one is the extreme"),
        biggest
    );
    assert_eq!(
        biggest
            .checked_div(Fixed::ONE)
            .expect("dividing the positive extreme by one is the extreme"),
        biggest
    );

    // A magnitude of `2^127` reached with a POSITIVE sign is genuinely
    // unrepresentable and stays an error — the refusal must discriminate on the
    // sign rather than simply having been loosened.
    assert!(
        matches!(
            smallest.checked_div(Fixed::from_integer(-1).expect("representable")),
            Err(TextError::Overflow(_))
        ),
        "negating the negative extreme leaves the range and must be refused"
    );
    assert!(
        matches!(
            smallest.checked_mul(Fixed::from_integer(-1).expect("representable")),
            Err(TextError::Overflow(_))
        ),
        "the same magnitude with a positive sign is not representable"
    );
}
