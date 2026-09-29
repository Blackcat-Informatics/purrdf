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
    debug_assert!(high < divisor, "the quotient must fit a u128");
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
    quotient
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
    if c == 0 {
        return None;
    }
    if let Some(product) = a.checked_mul(b) {
        return Some(product / c);
    }
    let (high, low) = wide_mul(a, b);
    // The quotient is at least `high · 2^128 / c`, so it exceeds a `u128` unless
    // the high half is itself below the divisor.
    if high >= c {
        return None;
    }
    Some(div_wide(high, low, c))
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
    use super::{div_wide, gcd, mul_div, wide_mul};

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
