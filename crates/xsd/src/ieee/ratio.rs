// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact rational rounding to integer and IEEE binary formats.
//!
//! Remainders decide every tie. No arithmetic on a floating-point value, ambient
//! rounding mode, intermediate narrowing, or decimal string is involved.

use crate::integer::{
    Int, LimbScratch, LimbScratchError,
    scratch::{Allocate, Unbounded},
};
use core::cmp::Ordering;

/// Rounding law for an exact rational value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// Nearest representable value; a halfway value chooses an even significand.
    NearestEven,
    /// Greatest representable value no greater than the exact value.
    Down,
    /// Least representable value no smaller than the exact value.
    Up,
}

/// Canonical coprime numerator and positive denominator for `n / 2^bits`.
/// Binary factors are removed exactly; no general GCD or division is needed.
#[must_use]
pub fn dyadic_ratio(numerator: &Int, fractional_bits: u32) -> (Int, Int) {
    dyadic_ratio_using(numerator, fractional_bits, &Unbounded).expect("unbounded integer storage")
}

/// The same canonical dyadic ratio using only admitted limb destinations.
///
/// # Errors
/// Refuses exhausted destination count or capacity without an allocation fallback.
pub fn dyadic_ratio_in(
    numerator: &Int,
    fractional_bits: u32,
    scratch: &LimbScratch,
) -> Result<(Int, Int), LimbScratchError> {
    dyadic_ratio_using(numerator, fractional_bits, scratch)
}

fn dyadic_ratio_using(
    numerator: &Int,
    fractional_bits: u32,
    storage: &impl Allocate,
) -> Result<(Int, Int), LimbScratchError> {
    let Some(zeros) = numerator.trailing_zeros() else {
        return Ok((Int::zero(), Int::one()));
    };
    let reduction = zeros.min(u64::from(fractional_bits)) as u32;
    Ok((
        numerator.shr_using(reduction, storage)?,
        Int::one().shl_using(fractional_bits - reduction, storage)?,
    ))
}

/// Correctly half-even rounded integer quotient, or `None` for zero denominator.
#[must_use]
pub fn round_even_integer(numerator: &Int, denominator: &Int) -> Option<Int> {
    round_even_integer_using(numerator, denominator, &Unbounded).expect("unbounded integer storage")
}

/// The identical half-even law using only admitted reusable destinations.
/// # Errors
/// Refuses scratch exhaustion; a zero denominator returns `Ok(None)`.
pub fn round_even_integer_in(
    numerator: &Int,
    denominator: &Int,
    scratch: &LimbScratch,
) -> Result<Option<Int>, LimbScratchError> {
    round_even_integer_using(numerator, denominator, scratch)
}

/// Half-even rounded exact product quotient without a retained heap product.
#[must_use]
pub fn round_even_product(numerator: &Int, factor: &Int, denominator: &Int) -> Option<Int> {
    round_integer_product(numerator, factor, denominator, Rounding::NearestEven)
}

/// The same fused product rounding in admitted reusable destinations.
/// # Errors
/// Refuses destination exhaustion; a zero denominator returns `Ok(None)`.
pub fn round_even_product_in(
    numerator: &Int,
    factor: &Int,
    denominator: &Int,
    scratch: &LimbScratch,
) -> Result<Option<Int>, LimbScratchError> {
    round_integer_product_in(
        numerator,
        factor,
        denominator,
        Rounding::NearestEven,
        scratch,
    )
}

/// Round an exact product quotient under its declared integer rounding law.
/// No intermediate product is retained; a zero denominator returns `None`.
#[must_use]
pub fn round_integer_product(
    numerator: &Int,
    factor: &Int,
    denominator: &Int,
    rounding: Rounding,
) -> Option<Int> {
    round_product_using(numerator, factor, denominator, rounding, &Unbounded)
        .expect("unbounded integer storage")
}

/// The identical directed or nearest product rounding in admitted destinations.
/// # Errors
/// Refuses destination exhaustion; a zero denominator returns `Ok(None)`.
pub fn round_integer_product_in(
    numerator: &Int,
    factor: &Int,
    denominator: &Int,
    rounding: Rounding,
    scratch: &LimbScratch,
) -> Result<Option<Int>, LimbScratchError> {
    round_product_using(numerator, factor, denominator, rounding, scratch)
}

fn round_even_integer_using(
    numerator: &Int,
    denominator: &Int,
    storage: &impl Allocate,
) -> Result<Option<Int>, LimbScratchError> {
    round_product_using(
        numerator,
        &Int::one(),
        denominator,
        Rounding::NearestEven,
        storage,
    )
}

pub(crate) fn round_product_using(
    numerator: &Int,
    factor: &Int,
    denominator: &Int,
    rounding: Rounding,
    storage: &impl Allocate,
) -> Result<Option<Int>, LimbScratchError> {
    let divisor = denominator.abs_using(storage)?;
    let Some((quotient, remainder)) = numerator.abs_using(storage)?.mul_div_rem_using(
        &factor.abs_using(storage)?,
        &divisor,
        storage,
    )?
    else {
        return Ok(None);
    };
    let negative = (numerator.is_negative() != factor.is_negative()) != denominator.is_negative();
    let increment = match rounding {
        Rounding::NearestEven => match remainder.cmp(&divisor.sub_using(&remainder, storage)?) {
            Ordering::Greater => true,
            Ordering::Equal => quotient.is_odd(),
            Ordering::Less => false,
        },
        Rounding::Down => negative && !remainder.is_zero(),
        Rounding::Up => !negative && !remainder.is_zero(),
    };
    let rounded = if increment {
        quotient.add_using(&Int::one(), storage)?
    } else {
        quotient
    };
    Ok(Some(if negative { rounded.neg() } else { rounded }))
}

/// Exact rational to binary64, including directed overflow and subnormal results.
/// A negative nonzero rational that underflows to zero retains the negative sign.
/// The canonical integer zero is positive zero. Zero denominator returns `None`.
#[must_use]
pub fn to_binary64(numerator: &Int, denominator: &Int, rounding: Rounding) -> Option<f64> {
    ratio_bits(
        numerator,
        denominator,
        rounding,
        Format::BINARY64,
        &Unbounded,
    )
    .expect("unbounded integer storage")
    .map(f64::from_bits)
}

/// The identical rational-to-binary64 law in admitted reusable destinations.
/// No arithmetic is performed on floating-point values.
/// # Errors
/// Refuses destination exhaustion; a zero denominator returns `Ok(None)`.
pub fn to_binary64_in(
    numerator: &Int,
    denominator: &Int,
    rounding: Rounding,
    scratch: &LimbScratch,
) -> Result<Option<f64>, LimbScratchError> {
    Ok(
        ratio_bits(numerator, denominator, rounding, Format::BINARY64, scratch)?
            .map(f64::from_bits),
    )
}

/// A monotone unsigned ordering key for an original exact rational.
///
/// This calls the same integer-only binary64 half-even rounding body. Both
/// rounded zero signs share a key. Distinct keys prove the original rational
/// order; equal keys, including underflow and overflow, require an exact
/// comparison. No floating operation or ambient rounding state participates.
/// A zero denominator returns `None`.
#[must_use]
pub fn binary64_order_key(numerator: &Int, denominator: &Int) -> Option<u64> {
    binary64_order_key_using(numerator, denominator, &Unbounded).expect("unbounded integer storage")
}

/// The identical ordering key in admitted reusable limb destinations.
/// [`crate::integer::ExactArithmeticCost::rational_binary64`] bounds its
/// original rounding body; the following bit transformation is constant work.
/// # Errors
/// Refuses scratch exhaustion without allocating a fallback. A zero
/// denominator returns `Ok(None)`.
pub fn binary64_order_key_in(
    numerator: &Int,
    denominator: &Int,
    scratch: &LimbScratch,
) -> Result<Option<u64>, LimbScratchError> {
    binary64_order_key_using(numerator, denominator, scratch)
}

fn binary64_order_key_using(
    numerator: &Int,
    denominator: &Int,
    storage: &impl Allocate,
) -> Result<Option<u64>, LimbScratchError> {
    Ok(ratio_bits(
        numerator,
        denominator,
        Rounding::NearestEven,
        Format::BINARY64,
        storage,
    )?
    .map(|bits| {
        const SIGN: u64 = 1 << 63;
        let bits = if bits << 1 == 0 { 0 } else { bits };
        if bits & SIGN == 0 { bits ^ SIGN } else { !bits }
    }))
}

/// Exact rational to binary32 without a binary64 intermediate or double rounding.
/// Sign, zero-denominator, and directed-rounding behavior matches [`to_binary64`].
#[must_use]
pub fn to_binary32(numerator: &Int, denominator: &Int, rounding: Rounding) -> Option<f32> {
    ratio_bits(
        numerator,
        denominator,
        rounding,
        Format::BINARY32,
        &Unbounded,
    )
    .expect("unbounded integer storage")
    .map(|bits| f32::from_bits(bits as u32))
}

#[derive(Clone, Copy)]
struct Format {
    fraction: u32,
    bias: i64,
    minimum: i64,
    maximum: i64,
    sign: u32,
}
impl Format {
    const BINARY64: Self = Self {
        fraction: 52,
        bias: 1023,
        minimum: -1022,
        maximum: 1023,
        sign: 63,
    };
    const BINARY32: Self = Self {
        fraction: 23,
        bias: 127,
        minimum: -126,
        maximum: 127,
        sign: 31,
    };
    const fn infinity(self) -> u64 {
        ((2 * self.bias + 1) as u64) << self.fraction
    }
    const fn overflow(self, rounding: Rounding) -> u64 {
        match rounding {
            Rounding::Down => self.infinity() - 1,
            Rounding::NearestEven | Rounding::Up => self.infinity(),
        }
    }
    const fn tiny(rounding: Rounding) -> u64 {
        match rounding {
            Rounding::Up => 1,
            Rounding::NearestEven | Rounding::Down => 0,
        }
    }
}

fn ratio_bits(
    n: &Int,
    d: &Int,
    rounding: Rounding,
    format: Format,
    storage: &impl Allocate,
) -> Result<Option<u64>, LimbScratchError> {
    if d.is_zero() {
        return Ok(None);
    }
    if n.is_zero() {
        return Ok(Some(0));
    }
    let negative = n.is_negative() != d.is_negative();
    let magnitude_rounding = match (negative, rounding) {
        (true, Rounding::Down) => Rounding::Up,
        (true, Rounding::Up) => Rounding::Down,
        _ => rounding,
    };
    if let (Some(small_n), Some(small_d)) = (n.unsigned_abs_u128(), d.unsigned_abs_u128())
        && let Some(bits) = positive_bits_u128(small_n, small_d, magnitude_rounding, format)
    {
        return Ok(Some(bits | (u64::from(negative) << format.sign)));
    }
    let bits = positive_bits(
        &n.abs_using(storage)?,
        &d.abs_using(storage)?,
        magnitude_rounding,
        format,
        storage,
    )?;
    Ok(Some(bits | (u64::from(negative) << format.sign)))
}

/// Compare a rational to a power of two without losing any low bits.
fn at_least_pow2(
    n: &Int,
    d: &Int,
    exponent: i64,
    storage: &impl Allocate,
) -> Result<bool, LimbScratchError> {
    Ok(if exponent >= 0 {
        n >= &d.shl_using(exponent as u32, storage)?
    } else {
        n.shl_using(exponent.unsigned_abs() as u32, storage)? >= *d
    })
}

fn positive_bits(
    n: &Int,
    d: &Int,
    rounding: Rounding,
    format: Format,
    storage: &impl Allocate,
) -> Result<u64, LimbScratchError> {
    if let (Some(small_n), Some(small_d)) = (n.unsigned_abs_u128(), d.unsigned_abs_u128())
        && let Some(bits) = positive_bits_u128(small_n, small_d, rounding, format)
    {
        return Ok(bits);
    }
    positive_bits_wide(n, d, rounding, format, storage)
}

/// The binary64 rounding of a nonzero signed ratio of 128-bit magnitudes, or
/// `None` when an operand or its shift does not fit the narrow path. Identical
/// to [`to_binary64`] wherever it answers.
pub(crate) fn binary64_u128(
    negative: bool,
    numerator: u128,
    denominator: u128,
    rounding: Rounding,
) -> Option<f64> {
    let magnitude_rounding = match (negative, rounding) {
        (true, Rounding::Down) => Rounding::Up,
        (true, Rounding::Up) => Rounding::Down,
        _ => rounding,
    };
    let bits = positive_bits_u128(numerator, denominator, magnitude_rounding, Format::BINARY64)?;
    Some(f64::from_bits(bits | (u64::from(negative) << 63)))
}

/// The same rounding law as [`positive_bits_wide`] when every shifted operand
/// fits in 128 bits. `None` defers to the arbitrary-width body; both paths
/// return identical bits wherever this one answers.
fn positive_bits_u128(n: u128, d: u128, rounding: Rounding, format: Format) -> Option<u64> {
    if n == 0 || d == 0 {
        return None;
    }
    let estimate = i64::from(128 - n.leading_zeros()) - i64::from(128 - d.leading_zeros());
    let quantum_exponent = format.minimum - i64::from(format.fraction);
    if estimate >= format.maximum + 2 {
        return Some(format.overflow(rounding));
    }
    if estimate <= quantum_exponent - 2 {
        return Some(Format::tiny(rounding));
    }
    // n >= d*2^e  <=>  floor(n/2^e) >= d (e>=0), and d <= n*2^k (k=-e>0).
    let at_least = if estimate >= 0 {
        let e = u32::try_from(estimate).ok()?;
        (if e >= 128 { 0 } else { n >> e }) >= d
    } else {
        let k = u32::try_from(-estimate).ok()?;
        k >= n.leading_zeros() || (n << k) >= d
    };
    let exponent = if at_least { estimate } else { estimate - 1 };
    if exponent > format.maximum {
        return Some(format.overflow(rounding));
    }
    if exponent <= quantum_exponent - 2 {
        return Some(Format::tiny(rounding));
    }
    let normal = exponent >= format.minimum;
    let shift = if normal {
        i64::from(format.fraction) - exponent
    } else {
        -quantum_exponent
    };
    let (numerator, denominator) = if shift >= 0 {
        let shift = u32::try_from(shift).ok()?;
        if shift >= n.leading_zeros() {
            return None;
        }
        (n << shift, d)
    } else {
        let shift = u32::try_from(-shift).ok()?;
        if shift >= d.leading_zeros() {
            return None;
        }
        (n, d << shift)
    };
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let mut significand = quotient;
    let increment = match rounding {
        Rounding::Down => false,
        Rounding::Up => remainder != 0,
        Rounding::NearestEven => match remainder.cmp(&(denominator - remainder)) {
            Ordering::Greater => true,
            Ordering::Equal => significand & 1 == 1,
            Ordering::Less => false,
        },
    };
    significand += u128::from(increment);
    if !normal {
        return Some(significand as u64);
    }
    let mut binade = exponent;
    if significand == 1u128 << (format.fraction + 1) {
        significand = 1u128 << format.fraction;
        binade += 1;
    }
    if binade > format.maximum {
        return Some(format.overflow(rounding));
    }
    Some(
        (((binade + format.bias) as u64) << format.fraction)
            | ((significand as u64) - (1u64 << format.fraction)),
    )
}

fn positive_bits_wide(
    n: &Int,
    d: &Int,
    rounding: Rounding,
    format: Format,
    storage: &impl Allocate,
) -> Result<u64, LimbScratchError> {
    let estimate = i128::from(n.bit_len()) - i128::from(d.bit_len());
    let quantum_exponent = format.minimum - i64::from(format.fraction);
    // The true binade is estimate or estimate-1. Bound before any shifted
    // allocation, including grossly over- and under-range source rationals.
    if estimate >= i128::from(format.maximum + 2) {
        return Ok(format.overflow(rounding));
    }
    if estimate <= i128::from(quantum_exponent - 2) {
        return Ok(Format::tiny(rounding));
    }
    let estimate = estimate as i64;
    let exponent = if at_least_pow2(n, d, estimate, storage)? {
        estimate
    } else {
        estimate - 1
    };
    if exponent > format.maximum {
        return Ok(format.overflow(rounding));
    }
    if exponent <= quantum_exponent - 2 {
        return Ok(Format::tiny(rounding));
    }
    let normal = exponent >= format.minimum;
    let shift = if normal {
        i64::from(format.fraction) - exponent
    } else {
        -quantum_exponent
    };
    let (numerator, denominator) = if shift >= 0 {
        (n.shl_using(shift as u32, storage)?, d.abs_using(storage)?)
    } else {
        (
            n.abs_using(storage)?,
            d.shl_using(shift.unsigned_abs() as u32, storage)?,
        )
    };
    let (quotient, remainder) = numerator
        .div_rem_using(&denominator, storage)?
        .expect("positive divisor");
    let mut significand = quotient
        .unsigned_abs_u128()
        .expect("placed significand fits 54 bits");
    let increment = match rounding {
        Rounding::Down => false,
        Rounding::Up => !remainder.is_zero(),
        Rounding::NearestEven => {
            match remainder.cmp(&denominator.sub_using(&remainder, storage)?) {
                Ordering::Greater => true,
                Ordering::Equal => significand & 1 == 1,
                Ordering::Less => false,
            }
        }
    };
    significand += u128::from(increment);
    if !normal {
        return Ok(significand as u64);
    }
    let mut binade = exponent;
    if significand == 1u128 << (format.fraction + 1) {
        significand = 1u128 << format.fraction;
        binade += 1;
    }
    if binade > format.maximum {
        return Ok(format.overflow(rounding));
    }
    Ok((((binade + format.bias) as u64) << format.fraction)
        | ((significand as u64) - (1u64 << format.fraction)))
}

#[cfg(test)]
mod tests {
    use super::{
        Int, Rounding, round_even_integer, round_even_product, round_integer_product, to_binary32,
        to_binary64,
    };

    #[test]
    fn the_u128_ratio_path_matches_the_arbitrary_width_body() {
        use super::{Format, Unbounded, positive_bits_u128, positive_bits_wide};
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || purrdf_hash::mix::splitmix64_next(&mut state);
        let mut checked = 0_u32;
        for case in 0..40_000_u32 {
            let width = |draw: u64, bits: u32| {
                let wide = (u128::from(draw) << 64) | u128::from(draw.rotate_left(17));
                if bits == 0 {
                    1
                } else {
                    (wide >> (128 - bits)).max(1)
                }
            };
            let n = width(next(), (case % 129).min(128));
            let d = width(next(), ((case / 129) % 129).min(128));
            for format in [Format::BINARY64, Format::BINARY32] {
                for rounding in [Rounding::Down, Rounding::Up, Rounding::NearestEven] {
                    if let Some(fast) = positive_bits_u128(n, d, rounding, format) {
                        let slow = positive_bits_wide(
                            &Int::from_u128(n),
                            &Int::from_u128(d),
                            rounding,
                            format,
                            &Unbounded,
                        )
                        .unwrap();
                        assert_eq!(fast, slow, "{n}/{d} {rounding:?}");
                        checked += 1;
                    }
                }
            }
        }
        // Exact decimals of geographic magnitude take the narrow path.
        assert!(
            positive_bits_u128(
                36_530_042_355_041,
                1_000_000_000_000,
                Rounding::Up,
                Format::BINARY64
            )
            .is_some()
        );
        assert!(checked > 100_000, "{checked}");
    }

    #[test]
    fn rational_order_keys_are_monotone_with_exact_ties_and_bounded_storage() {
        use crate::rational::Rat;
        let scratch = crate::integer::LimbScratch::new(16, 96).unwrap();
        let mut values = Vec::new();
        for (numerator, denominator) in [
            (Int::zero(), Int::one()),
            (Int::one(), Int::one().shl(1075)),
            (Int::one(), Int::one().shl(1074)),
            (Int::from_i64(3), Int::one().shl(1075)),
            (Int::one(), Int::from_i64(3)),
            (Int::one(), Int::one()),
            (Int::one().shl(54).add(&Int::one()), Int::one().shl(54)),
            (Int::one().shl(53).add(&Int::one()), Int::one().shl(53)),
            (
                Int::one().shl(53).add(&Int::from_i64(3)),
                Int::one().shl(53),
            ),
            (Int::one().shl(1024), Int::one()),
            (Int::one().shl(2000), Int::one()),
        ] {
            values.push(Rat::new(numerator.clone(), denominator.clone()).unwrap());
            values.push(Rat::new(numerator.neg(), denominator).unwrap());
        }
        values.sort_unstable();
        let expected = values
            .iter()
            .map(|value| super::binary64_order_key(value.numerator(), value.denominator()).unwrap())
            .collect::<Vec<_>>();
        assert!(expected.windows(2).all(|pair| pair[0] <= pair[1]));
        let zero = super::binary64_order_key(&Int::zero(), &Int::one()).unwrap();
        for sign in [-1, 1] {
            assert_eq!(
                super::binary64_order_key(&Int::from_i64(sign), &Int::one().shl(1075)),
                Some(zero)
            );
        }
        for (left, left_key) in values.iter().zip(&expected) {
            for (right, right_key) in values.iter().zip(&expected) {
                if left_key != right_key {
                    assert_eq!(left_key.cmp(right_key), left.cmp(right));
                }
            }
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for (value, key) in values.iter().zip(&expected) {
            assert_eq!(
                super::binary64_order_key_in(value.numerator(), value.denominator(), &scratch),
                Ok(Some(*key))
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), scratch.destination_capacity());
        assert_eq!(super::binary64_order_key(&Int::one(), &Int::zero()), None);
        let insufficient = crate::integer::LimbScratch::new(8, 3).unwrap();
        assert!(
            super::binary64_order_key_in(
                &Int::one().shl(2000).add(&Int::one()),
                &Int::one().shl(2000),
                &insufficient
            )
            .is_err()
        );
    }

    #[test]
    fn bounded_binary64_preserves_ties_directed_edges_and_signed_underflow() {
        let scratch = crate::integer::LimbScratch::new(16, 96).unwrap();
        let mut cases = Vec::new();
        for (numerator, denominator) in [
            (Int::one().shl(2000).add(&Int::one()), Int::one().shl(2000)),
            (Int::one().shl(53).add(&Int::one()), Int::one().shl(53)),
            (
                Int::one().shl(53).add(&Int::from_i64(3)),
                Int::one().shl(53),
            ),
            (Int::one(), Int::one().shl(1075)),
            (Int::from_i64(3), Int::one().shl(1075)),
            (Int::one().shl(1024).sub(&Int::one().shl(970)), Int::one()),
        ] {
            for numerator in [numerator.clone(), numerator.neg()] {
                for rounding in [Rounding::NearestEven, Rounding::Down, Rounding::Up] {
                    let expected = to_binary64(&numerator, &denominator, rounding)
                        .unwrap()
                        .to_bits();
                    cases.push((numerator.clone(), denominator.clone(), rounding, expected));
                }
            }
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for (n, d, rounding, expected) in &cases {
            assert_eq!(
                super::to_binary64_in(n, d, *rounding, &scratch)
                    .unwrap()
                    .unwrap()
                    .to_bits(),
                *expected
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), scratch.destination_capacity());
        assert!(
            super::to_binary64_in(&Int::one(), &Int::zero(), Rounding::Up, &scratch)
                .unwrap()
                .is_none()
        );
        let insufficient = crate::integer::LimbScratch::new(8, 3).unwrap();
        assert!(matches!(
            super::to_binary64_in(
                &cases[0].0,
                &cases[0].1,
                Rounding::NearestEven,
                &insufficient
            ),
            Err(crate::integer::LimbScratchError::Capacity { .. })
        ));
    }

    #[test]
    fn canonical_dyadic_ratios_preserve_signed_values_and_reuse_bounded_storage() {
        let scratch = crate::integer::LimbScratch::new(16, 32).unwrap();
        let mut expected = Vec::new();
        for bits in [0, 1, 64, 192, 224, 448, 1074] {
            for source in [
                Int::zero(),
                Int::one(),
                Int::from_i64(-12),
                Int::one().shl(300),
            ] {
                let pair = super::dyadic_ratio(&source, bits);
                assert_eq!(pair.0.mul(&Int::one().shl(bits)), source.mul(&pair.1));
                assert!(pair.0.abs().gcd(&pair.1).is_one());
                assert!(!pair.1.is_negative());
                expected.push((source, bits, pair));
            }
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for (source, bits, expected) in &expected {
            assert_eq!(
                &super::dyadic_ratio_in(source, *bits, &scratch).unwrap(),
                expected
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), scratch.destination_capacity());
        let insufficient = crate::integer::LimbScratch::new(4, 3).unwrap();
        assert!(matches!(
            super::dyadic_ratio_in(&Int::one(), 224, &insufficient),
            Err(crate::integer::LimbScratchError::Capacity { .. })
        ));
    }

    #[test]
    fn signed_integer_ties_use_one_symmetric_law() {
        for (n, d, expected) in [
            (5, 2, 2),
            (7, 2, 4),
            (-5, 2, -2),
            (-7, 2, -4),
            (7, -2, -4),
            (-7, -2, 4),
        ] {
            assert_eq!(
                round_even_integer(&Int::from_i64(n), &Int::from_i64(d)).and_then(|v| v.to_i128()),
                Some(expected)
            );
        }
        assert!(round_even_integer(&Int::one(), &Int::zero()).is_none());
    }

    #[test]
    fn fused_product_rounding_matches_full_exact_product_at_signed_ties() {
        for numerator in -9..=9 {
            for factor in -9..=9 {
                for divisor in -9..=9 {
                    let n = Int::from_i64(numerator);
                    let f = Int::from_i64(factor);
                    let d = Int::from_i64(divisor);
                    assert_eq!(
                        round_even_product(&n, &f, &d),
                        round_even_integer(&n.mul(&f), &d)
                    );
                }
            }
        }
        let n = Int::one().shl(180).add(&Int::one());
        let f = Int::pow10(6);
        let d = Int::one().shl(112);
        assert_eq!(
            round_even_product(&n, &f, &d),
            round_even_integer(&n.mul(&f), &d)
        );
    }

    #[test]
    fn binary64_halfway_normal_and_subnormal_neighbors() {
        let numerator = Int::one().shl(53).add(&Int::one());
        let denominator = Int::one().shl(53);
        assert_eq!(
            to_binary64(&numerator, &denominator, Rounding::NearestEven)
                .unwrap()
                .to_bits(),
            1.0f64.to_bits()
        );
        assert_eq!(
            to_binary64(&numerator, &denominator, Rounding::Down)
                .unwrap()
                .to_bits(),
            1.0f64.to_bits()
        );
        assert_eq!(
            to_binary64(&numerator, &denominator, Rounding::Up)
                .unwrap()
                .to_bits(),
            1.0f64.to_bits() + 1
        );
        let denominator = Int::one().shl(1075);
        for (numerator, nearest, lower, upper) in [(1, 0, 0, 1), (3, 2, 1, 2), (5, 2, 2, 3)] {
            let value = Int::from_i64(numerator);
            assert_eq!(
                to_binary64(&value, &denominator, Rounding::NearestEven)
                    .unwrap()
                    .to_bits(),
                nearest
            );
            assert_eq!(
                to_binary64(&value, &denominator, Rounding::Down)
                    .unwrap()
                    .to_bits(),
                lower
            );
            assert_eq!(
                to_binary64(&value, &denominator, Rounding::Up)
                    .unwrap()
                    .to_bits(),
                upper
            );
            assert_eq!(
                to_binary64(&value.neg(), &denominator, Rounding::Down)
                    .unwrap()
                    .to_bits(),
                (1 << 63) | upper
            );
            assert_eq!(
                to_binary64(&value.neg(), &denominator, Rounding::Up)
                    .unwrap()
                    .to_bits(),
                (1 << 63) | lower
            );
        }
    }

    #[test]
    fn both_widths_round_overflow_midpoint_and_directed_neighbors() {
        let midpoint64 = Int::one().shl(54).sub(&Int::one()).shl(970);
        assert_eq!(
            to_binary64(&midpoint64, &Int::one(), Rounding::NearestEven),
            Some(f64::INFINITY)
        );
        assert_eq!(
            to_binary64(&midpoint64, &Int::one(), Rounding::Down),
            Some(f64::MAX)
        );
        assert_eq!(
            to_binary64(
                &midpoint64.sub(&Int::one()),
                &Int::one(),
                Rounding::NearestEven
            ),
            Some(f64::MAX)
        );
        assert_eq!(
            to_binary64(&midpoint64.neg(), &Int::one(), Rounding::Up),
            Some(-f64::MAX)
        );
        let midpoint32 = Int::one().shl(25).sub(&Int::one()).shl(103);
        assert_eq!(
            to_binary32(&midpoint32, &Int::one(), Rounding::NearestEven),
            Some(f32::INFINITY)
        );
        assert_eq!(
            to_binary32(&midpoint32, &Int::one(), Rounding::Down),
            Some(f32::MAX)
        );
        assert_eq!(
            to_binary32(
                &midpoint32.sub(&Int::one()),
                &Int::one(),
                Rounding::NearestEven
            ),
            Some(f32::MAX)
        );
        assert!(to_binary64(&Int::one(), &Int::zero(), Rounding::Up).is_none());
        assert!(to_binary32(&Int::one(), &Int::zero(), Rounding::Down).is_none());
    }

    #[test]
    fn gross_range_refusal_is_bounded_and_zero_denominator_sign_is_explicit() {
        let huge = Int::one().shl(120_000);
        assert_eq!(
            to_binary64(&huge, &Int::one(), Rounding::Down),
            Some(f64::MAX)
        );
        assert_eq!(
            to_binary64(&Int::one(), &huge, Rounding::Up)
                .unwrap()
                .to_bits(),
            1
        );
        assert_eq!(
            to_binary64(&Int::one(), &huge.neg(), Rounding::NearestEven)
                .unwrap()
                .to_bits(),
            1 << 63
        );
        assert_eq!(
            to_binary64(&Int::zero(), &huge.neg(), Rounding::NearestEven)
                .unwrap()
                .to_bits(),
            0
        );
    }
    #[test]
    fn directed_product_integer_rounding_matches_signed_euclidean_quotients() {
        for numerator in -19_i64..=19 {
            for factor in -3_i64..=3 {
                for denominator in -7_i64..=7 {
                    let n = Int::from_i64(numerator);
                    let f = Int::from_i64(factor);
                    let d = Int::from_i64(denominator);
                    if denominator == 0 {
                        assert_eq!(round_integer_product(&n, &f, &d, Rounding::Down), None,);
                        continue;
                    }
                    let signed = numerator * factor * denominator.signum();
                    let divisor = denominator.abs();
                    for (rounding, expected) in [
                        (Rounding::Down, signed.div_euclid(divisor)),
                        (Rounding::Up, -(-signed).div_euclid(divisor)),
                    ] {
                        assert_eq!(
                            round_integer_product(&n, &f, &d, rounding),
                            Some(Int::from_i64(expected)),
                        );
                    }
                }
            }
        }
    }
}
