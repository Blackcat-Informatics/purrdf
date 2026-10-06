// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact `u128` arithmetic past `u128`: the full 256-bit product, the division
//! of a 256-bit dividend, the fused `a × b / c`, and the greatest common
//! divisor.
//!
//! The workspace's exact fixed-point and rational types (`owl:rational` here,
//! BM25's scale-`10^12` fixed point in `purrdf-text`, the geometry kernel's
//! small-magnitude fast path in `purrdf-geo`) keep their values in `u128`
//! magnitudes and need an intermediate one bit or one word wider than the
//! answer. These are the one implementation of each: portable integer code with
//! no dependency, so every target computes the same bits.

/// The full 256-bit product of two `u128`s, as `(high, low)`.
///
/// Schoolbook over 64-bit halves: each partial product of two halves fits a
/// `u128`, and the middle column's sum of three 64-bit terms carries at most two
/// bits into the high word.
#[must_use]
pub const fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const HALF: u32 = 64;
    let mask = u64::MAX as u128;

    let (a_high, a_low) = (a >> HALF, a & mask);
    let (b_high, b_low) = (b >> HALF, b & mask);

    let low_low = a_low * b_low;
    let low_high = a_low * b_high;
    let high_low = a_high * b_low;
    let high_high = a_high * b_high;

    let middle = (low_low >> HALF) + (low_high & mask) + (high_low & mask);
    let low = (low_low & mask) | (middle << HALF);
    let high = high_high + (low_high >> HALF) + (high_low >> HALF) + (middle >> HALF);
    (high, low)
}

/// `(high · 2^128 + low) / divisor`, requiring `high < divisor` so the quotient
/// fits a `u128`.
///
/// Restoring long division, one bit at a time. The running remainder is always
/// below `divisor`, so doubling it stays below `2 · divisor`; that can exceed a
/// `u128` by exactly one bit, which is why the carry is tracked separately
/// rather than left to overflow.
#[must_use]
pub const fn div_wide(high: u128, low: u128, divisor: u128) -> u128 {
    div_wide_rem(high, low, divisor).0
}

/// Divide the full 256-bit dividend, retaining the exact remainder.
/// The caller must supply `high < divisor`, so the quotient fits a `u128`.
#[must_use]
pub const fn div_wide_rem(high: u128, low: u128, divisor: u128) -> (u128, u128) {
    debug_assert!(high < divisor, "the quotient must fit a u128");
    if divisor.is_power_of_two() {
        let shift = divisor.trailing_zeros();
        if shift == 0 {
            return (low, 0);
        }
        // high<divisor proves high has at most `shift` bits, so combining
        // these disjoint fields fits the same exact 128-bit quotient. The
        // discarded low bits are the complete remainder, not an approximation.
        return (
            (high << (128 - shift)) | (low >> shift),
            low & (divisor - 1),
        );
    }
    let mut remainder = high;
    let mut quotient: u128 = 0;
    let mut bit = 128_u32;
    while bit > 0 {
        bit -= 1;
        let carry = remainder >> 127;
        remainder = (remainder << 1) | ((low >> bit) & 1);
        quotient <<= 1;
        if carry == 1 || remainder >= divisor {
            // When `carry` is set the true remainder is `2^128 + remainder`, and
            // the wrapping subtraction computes exactly that value minus the
            // divisor — which is back below the divisor, so the invariant holds.
            remainder = remainder.wrapping_sub(divisor);
            quotient |= 1;
        }
    }
    (quotient, remainder)
}

/// `a × b / c`, truncated, over the full `u128` range — `None` if `c` is zero or
/// the quotient does not fit a `u128`.
///
/// The fast path is one multiplication and one division, taken whenever the
/// product fits. The slow path exists because the products fixed-point
/// arithmetic forms are routinely larger than their quotients: scaling by
/// `10^12` before dividing overflows a `u128` for any operand above about
/// `3.4 × 10^26`, while the answer is perfectly representable. Reporting that
/// as an overflow would make the arithmetic's range depend on the order the
/// operations were written in.
#[must_use]
pub const fn mul_div(a: u128, b: u128, c: u128) -> Option<u128> {
    match mul_div_rem(a, b, c) {
        Some((quotient, _)) => Some(quotient),
        None => None,
    }
}

/// `a × b / c`, rounded once to the nearest integer, with ties to even.
/// Returns `None` for a zero divisor, a quotient past `u128`, or a rounding
/// increment that would overflow. The product is evaluated in all 256 bits.
#[must_use]
pub const fn mul_div_round_even(a: u128, b: u128, c: u128) -> Option<u128> {
    let Some((quotient, remainder)) = mul_div_rem(a, b, c) else {
        return None;
    };
    // Comparing against `c - remainder` is exactly comparing `2 * remainder`
    // against `c`, without overflowing even when `c` occupies all 128 bits.
    let complement = c - remainder;
    if remainder > complement || (remainder == complement && quotient & 1 != 0) {
        quotient.checked_add(1)
    } else {
        Some(quotient)
    }
}

/// Shared checked product/division path for truncated and rounded results.
const fn mul_div_rem(a: u128, b: u128, c: u128) -> Option<(u128, u128)> {
    if c == 0 {
        return None;
    }
    if let Some(product) = a.checked_mul(b) {
        return Some((product / c, product % c));
    }
    let (high, low) = wide_mul(a, b);
    // The quotient is at least `high · 2^128 / c`, so it exceeds a `u128` unless
    // the high half is itself below the divisor.
    if high >= c {
        return None;
    }
    Some(div_wide_rem(high, low, c))
}

/// The greatest common divisor of two `u128`s by Euclid's algorithm;
/// `gcd(0, n) == n`, `gcd(n, 0) == n` and `gcd(0, 0) == 0`.
#[must_use]
pub const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::{div_wide, div_wide_rem, gcd, mul_div, mul_div_round_even, wide_mul};

    #[test]
    fn dyadic_divisors_preserve_the_full_quotient_remainder_and_even_ties() {
        use crate::BigInt;
        let mut state = 0x2971_4646_57d4_83ab;
        for shift in 0..128 {
            let divisor = 1_u128 << shift;
            state = purrdf_hash::mix::splitmix64_step(state);
            let high = u128::from(state) & (divisor - 1);
            state = purrdf_hash::mix::splitmix64_step(state);
            let low = (u128::from(state) << 64) | u128::from(!state);
            let (quotient, remainder) = div_wide_rem(high, low, divisor);
            let dividend = BigInt::from_u128(high)
                .mul_pow2(128)
                .add(&BigInt::from_u128(low));
            let divisor = BigInt::from_u128(divisor);
            let expected = dividend.div_rem(&divisor).unwrap();
            assert_eq!(
                (BigInt::from_u128(quotient), BigInt::from_u128(remainder)),
                expected
            );
            if shift > 0 {
                assert_eq!(
                    mul_div_round_even(3, 1_u128 << (shift - 1), 1_u128 << shift),
                    Some(2)
                );
                assert_eq!(
                    mul_div_round_even(5, 1_u128 << (shift - 1), 1_u128 << shift),
                    Some(2)
                );
            }
        }
    }

    #[test]
    fn rounded_products_handle_ties_full_width_and_overflow() {
        for (a, b, c, wanted) in [
            (1, 1, 2, Some(0)),
            (3, 1, 2, Some(2)),
            (5, 1, 2, Some(2)),
            (7, 1, 2, Some(4)),
            (5, 1, 3, Some(2)),
            (4, 1, 3, Some(1)),
            (0, u128::MAX, 7, Some(0)),
            (u128::MAX, u128::MAX, u128::MAX, Some(u128::MAX)),
            (u128::MAX, u128::MAX, 1, None),
            (1, 1, 0, None),
        ] {
            assert_eq!(mul_div_round_even(a, b, c), wanted);
        }
        let u = (1_u128 << 64) - 2;
        let v = u + 1;
        let c = u128::MAX - u - v;
        let (hi, lo) = wide_mul(u128::MAX - u, u128::MAX - v);
        assert_eq!(div_wide_rem(hi, lo, c), (u128::MAX, u * v));
        assert_eq!(mul_div_round_even(u128::MAX - u, u128::MAX - v, c), None);
        // A full-width divisor and remainder exercise the overflow-free comparison.
        assert_eq!(mul_div_round_even(u128::MAX - 1, 1, u128::MAX), Some(1));
    }

    #[test]
    fn full_width_rounding_lies_in_the_exact_nearest_even_cell() {
        use crate::BigInt;
        use purrdf_hash::mix::splitmix64_next;
        let mut state = 0x40de_61c3_826b_9a7f;
        let mut draw = || {
            (u128::from(splitmix64_next(&mut state)) << 64)
                | u128::from(splitmix64_next(&mut state))
        };
        for _ in 0..4096 {
            let (a, b, c) = (draw(), draw(), draw());
            let doubled = BigInt::from_u128(a).mul(&BigInt::from_u128(b)).mul_small(2);
            let divisor = BigInt::from_u128(c);
            match mul_div_round_even(a, b, c) {
                Some(value) => {
                    let mut lower = BigInt::from_u128(value).mul_small(2);
                    lower.add_i128(-1);
                    let lower = lower.mul(&divisor);
                    let mut upper = BigInt::from_u128(value).mul_small(2);
                    upper.add_i128(1);
                    let upper = upper.mul(&divisor);
                    assert!(doubled >= lower && doubled <= upper);
                    if doubled == lower || doubled == upper {
                        assert_eq!(value & 1, 0, "ties belong to the even neighbor");
                    }
                }
                None => {
                    let mut beyond = BigInt::from_u128(u128::MAX).mul_small(2);
                    beyond.add_i128(1);
                    assert!(c == 0 || doubled >= beyond.mul(&divisor));
                }
            }
        }
    }

    /// `mul_div` must agree with the direct computation wherever the direct one
    /// fits, and keep answering where it does not.
    #[test]
    fn mul_div_matches_the_direct_route_and_outlives_it() {
        assert_eq!(mul_div(7, 6, 3), Some(14));
        assert_eq!(mul_div(1, 1, 0), None, "a zero divisor has no quotient");
        // The product overflows a u128 but the quotient does not.
        let big = u128::MAX / 3;
        assert_eq!(mul_div(big, 4, 4), Some(big));
        assert_eq!(mul_div(big, 4, 2), Some(big * 2));
        // A quotient that genuinely does not fit is refused.
        assert_eq!(mul_div(u128::MAX, u128::MAX, 1), None);
        assert_eq!(mul_div(u128::MAX, u128::MAX, u128::MAX), Some(u128::MAX));
    }

    #[test]
    fn wide_mul_reaches_both_words() {
        assert_eq!(wide_mul(0, u128::MAX), (0, 0));
        assert_eq!(wide_mul(1 << 64, 1 << 64), (1, 0));
        assert_eq!(wide_mul(u128::MAX, u128::MAX), (u128::MAX - 1, 1));
        assert_eq!(wide_mul(u128::MAX, 2), (1, u128::MAX - 1));
    }

    #[test]
    fn div_wide_inverts_wide_mul() {
        for (a, b) in [
            (u128::MAX, u128::MAX - 7),
            (1 << 100, 3 << 90),
            (12_345_678_901_234_567_890, 98_765_432_109_876_543_210),
        ] {
            let (high, low) = wide_mul(a, b);
            assert_eq!(div_wide(high, low, b), a);
            assert_eq!(div_wide(high, low, a), b);
        }
    }

    #[test]
    fn gcd_absorbs_zero_and_reduces() {
        assert_eq!(gcd(0, 0), 0);
        assert_eq!(gcd(0, 12), 12);
        assert_eq!(gcd(12, 0), 12);
        assert_eq!(gcd(12, 18), 6);
        assert_eq!(gcd(u128::MAX, u128::MAX - 1), 1);
        assert_eq!(gcd(1 << 127, 1 << 64), 1 << 64);
    }
}
