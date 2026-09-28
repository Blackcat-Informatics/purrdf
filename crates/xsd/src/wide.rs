// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact 256-bit intermediates over `u128`: the wide multiply and the narrowing
//! division that let a `u128` computation form a product larger than its answer
//! without the answer's range depending on the order the operations were written.
//!
//! Three places in the workspace need the same thing. [`crate::rational`] orders
//! two `i128` rationals by cross-multiplying, which is exact only with a 256-bit
//! product to compare. A fixed-point text score scales by `10^12` before dividing,
//! so the product overflows a `u128` for operands whose quotient fits one. And a
//! duration or a timestamp scaled between units meets the same shape. Each of
//! those is a `u128` computation whose *result* fits; only an intermediate does
//! not. These three functions are that intermediate, spelled once.
//!
//! Everything here is plain integer arithmetic with no allocation; the division
//! is bit-serial (128 steps), which is the cost of exactness at a width the
//! machine has no instruction for, and none of the callers is per-triple.

/// `a × b` as `(high, low)`, the exact 256-bit product split at bit 128.
///
/// Four 64×64 partial products; the middle two are summed with the low product's
/// high half before the carries are folded upward, so no partial sum can exceed a
/// `u128`.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::wide::wide_mul;
///
/// assert_eq!(wide_mul(3, 4), (0, 12));
/// // (2^128 − 1)² = 2^256 − 2^129 + 1: high half 2^128 − 2, low half 1.
/// assert_eq!(wide_mul(u128::MAX, u128::MAX), (u128::MAX - 1, 1));
/// ```
#[must_use]
pub const fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const MASK: u128 = (1u128 << 64) - 1;
    let (a_hi, a_lo) = (a >> 64, a & MASK);
    let (b_hi, b_lo) = (b >> 64, b & MASK);
    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;
    let mid = (ll >> 64) + (lh & MASK) + (hl & MASK);
    let low = (ll & MASK) | (mid << 64);
    let high = hh + (lh >> 64) + (hl >> 64) + (mid >> 64);
    (high, low)
}

/// `(high · 2^128 + low) ÷ divisor` as `(quotient, remainder)`, exactly, or `None`
/// when `divisor` is zero or the quotient does not fit a `u128` (that is, when
/// `high >= divisor`).
///
/// Restoring long division, one bit at a time: the running remainder stays below
/// `divisor`, so doubling it can exceed a `u128` by exactly one bit, and that bit
/// is carried separately rather than left to overflow. `remainder` is below
/// `divisor` on return.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::wide::{div_wide, wide_mul};
///
/// let (high, low) = wide_mul(u128::MAX, 10);
/// assert_eq!(div_wide(high, low, 10), Some((u128::MAX, 0)));
/// assert_eq!(div_wide(0, 17, 5), Some((3, 2)));
/// assert_eq!(div_wide(1, 0, 1), None, "2^128 does not fit");
/// assert_eq!(div_wide(0, 1, 0), None, "no divisor");
/// ```
#[must_use]
pub const fn div_wide(high: u128, low: u128, divisor: u128) -> Option<(u128, u128)> {
    if divisor == 0 || high >= divisor {
        return None;
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
            // With `carry` set the true remainder is `2^128 + remainder`, and the
            // wrapping subtraction computes exactly that value minus the divisor,
            // which is back below the divisor, so the invariant holds.
            remainder = remainder.wrapping_sub(divisor);
            quotient |= 1;
        }
    }
    Some((quotient, remainder))
}

/// `⌊a × b ÷ d⌋` over the full `u128` range — `None` when `d` is zero or the
/// quotient does not fit a `u128`.
///
/// One multiplication and one division whenever the product fits; otherwise the
/// 256-bit product goes through [`div_wide`]. So a scaling that overflows as
/// written (`a × 10^12 / d` for `a` above about `3.4 × 10^26`) still answers when
/// its quotient is representable, and refuses only when the answer itself is not.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::wide::mul_div;
///
/// assert_eq!(mul_div(7, 3, 2), Some(10));
/// // The product overflows a u128; the quotient does not.
/// assert_eq!(mul_div(u128::MAX, 1_000_000_000_000, 1_000_000_000_000), Some(u128::MAX));
/// assert_eq!(mul_div(u128::MAX, 2, 1), None);
/// assert_eq!(mul_div(1, 1, 0), None);
/// ```
#[must_use]
pub const fn mul_div(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }
    if let Some(product) = a.checked_mul(b) {
        return Some(product / d);
    }
    let (high, low) = wide_mul(a, b);
    match div_wide(high, low, d) {
        Some((quotient, _)) => Some(quotient),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{div_wide, mul_div, wide_mul};

    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    /// A random `u128` with a uniformly drawn bit length, so both halves of the
    /// wide product are exercised.
    fn random_u128(state: &mut u64) -> u128 {
        let bits = splitmix64(state) % 129;
        if bits == 0 {
            return 0;
        }
        let raw = (u128::from(splitmix64(state)) << 64) | u128::from(splitmix64(state));
        let mask = if bits == 128 {
            u128::MAX
        } else {
            (1u128 << bits) - 1
        };
        (raw & mask) | (1u128 << (bits - 1))
    }

    /// `(high, low) + addend`, as a 256-bit value; asserts no carry out of bit 256.
    fn add_wide(high: u128, low: u128, addend: u128) -> (u128, u128) {
        let (low, carry) = low.overflowing_add(addend);
        (
            high.checked_add(u128::from(carry))
                .expect("no carry past 256 bits"),
            low,
        )
    }

    #[test]
    fn wide_mul_small_values_and_the_corners() {
        assert_eq!(wide_mul(0, 0), (0, 0));
        assert_eq!(wide_mul(0, u128::MAX), (0, 0));
        assert_eq!(wide_mul(1, u128::MAX), (0, u128::MAX));
        assert_eq!(wide_mul(6, 7), (0, 42));
        assert_eq!(wide_mul(1 << 64, 1 << 64), (1, 0));
        assert_eq!(wide_mul(1 << 127, 2), (1, 0));
        assert_eq!(wide_mul(1 << 127, 1 << 127), (1 << 126, 0));
        assert_eq!(wide_mul(u128::MAX, 2), (1, u128::MAX - 1));
        assert_eq!(wide_mul(u128::MAX, u128::MAX), (u128::MAX - 1, 1));
        // The middle-sum carry path: both operands have both halves set.
        let a = (1u128 << 64) | 1;
        assert_eq!(wide_mul(a, a), (1, (1u128 << 65) | 1));
    }

    #[test]
    fn wide_mul_matches_u128_where_the_product_fits_and_commutes() {
        let mut state = 0x71DE_0000_0000_0001_u64;
        for _ in 0..4000 {
            let a = random_u128(&mut state);
            let b = random_u128(&mut state);
            assert_eq!(wide_mul(a, b), wide_mul(b, a));
            match a.checked_mul(b) {
                Some(product) => assert_eq!(wide_mul(a, b), (0, product), "{a} × {b}"),
                None => assert_ne!(wide_mul(a, b).0, 0, "{a} × {b} overflows"),
            }
            // The low half is always the wrapping product.
            assert_eq!(wide_mul(a, b).1, a.wrapping_mul(b));
        }
    }

    #[test]
    fn div_wide_small_values_and_the_corners() {
        assert_eq!(div_wide(0, 0, 1), Some((0, 0)));
        assert_eq!(div_wide(0, 17, 5), Some((3, 2)));
        assert_eq!(div_wide(0, u128::MAX, u128::MAX), Some((1, 0)));
        assert_eq!(div_wide(0, u128::MAX, 1), Some((u128::MAX, 0)));
        // 2^128 + 1 over 2: quotient 2^127, remainder 1.
        assert_eq!(div_wide(1, 1, 2), Some((1 << 127, 1)));
        // (2^256 − 2^129 + 1) / (2^128 − 1) = 2^128 − 1, exactly.
        assert_eq!(div_wide(u128::MAX - 1, 1, u128::MAX), Some((u128::MAX, 0)));
        // The largest dividend whose quotient fits: high = divisor − 1.
        assert_eq!(
            div_wide(u128::MAX - 1, u128::MAX, u128::MAX),
            Some((u128::MAX, u128::MAX - 1))
        );
        // Refusals: a zero divisor, and a quotient that would need bit 128.
        assert_eq!(div_wide(0, 5, 0), None);
        assert_eq!(div_wide(1, 0, 1), None);
        assert_eq!(div_wide(u128::MAX, u128::MAX, u128::MAX), None);
        // The neighbour of each refusal succeeds.
        assert_eq!(div_wide(0, 5, 1), Some((5, 0)));
        assert_eq!(div_wide(0, u128::MAX, 1), Some((u128::MAX, 0)));
        assert_eq!(
            div_wide(u128::MAX - 1, u128::MAX, u128::MAX),
            Some((u128::MAX, u128::MAX - 1))
        );
    }

    #[test]
    fn div_wide_inverts_wide_mul_and_matches_u128_division() {
        let mut state = 0xD1F1_DE00_0000_0002_u64;
        for _ in 0..4000 {
            let a = random_u128(&mut state);
            let d = random_u128(&mut state).max(1);
            // With no high half this is u128 division.
            assert_eq!(div_wide(0, a, d), Some((a / d, a % d)), "{a} / {d}");
            // a × d + r, r < d: the division recovers (a, r).
            let r = random_u128(&mut state) % d;
            let (high, low) = add_wide(wide_mul(a, d).0, wide_mul(a, d).1, r);
            assert_eq!(
                div_wide(high, low, d),
                Some((a, r)),
                "({a} × {d} + {r}) / {d}"
            );
        }
    }

    #[test]
    fn div_wide_answers_random_dividends_that_fit() {
        let mut state = 0xD1F1_DE00_0000_0003_u64;
        for _ in 0..4000 {
            let d = random_u128(&mut state).max(1);
            let high = random_u128(&mut state) % d;
            let low = random_u128(&mut state);
            let (quotient, remainder) = div_wide(high, low, d).expect("high < d");
            assert!(remainder < d);
            // quotient × d + remainder reproduces (high, low) exactly.
            let (product_high, product_low) = wide_mul(quotient, d);
            assert_eq!(add_wide(product_high, product_low, remainder), (high, low));
        }
    }

    #[test]
    fn mul_div_small_values_and_the_overflow_edges() {
        assert_eq!(mul_div(0, 0, 1), Some(0));
        assert_eq!(mul_div(7, 3, 2), Some(10), "floor(21 / 2)");
        assert_eq!(mul_div(7, 3, 21), Some(1));
        assert_eq!(mul_div(7, 3, 22), Some(0));
        assert_eq!(mul_div(1, 1, 0), None);
        assert_eq!(mul_div(0, 0, 0), None);
        // The product overflows; the quotient is exactly representable.
        assert_eq!(mul_div(u128::MAX, u128::MAX, u128::MAX), Some(u128::MAX));
        assert_eq!(mul_div(u128::MAX, 2, 2), Some(u128::MAX));
        assert_eq!(
            mul_div(u128::MAX, 3, 4),
            Some(u128::MAX / 4 * 3 + (u128::MAX % 4) * 3 / 4)
        );
        assert_eq!(mul_div(1 << 127, 4, 2), None, "2^128 does not fit");
        assert_eq!(
            mul_div(1 << 127, 4, 4),
            Some(1 << 127),
            "and its neighbour does"
        );
        assert_eq!(mul_div(u128::MAX, 2, 1), None);
        assert_eq!(mul_div(u128::MAX, 1, 1), Some(u128::MAX));
    }

    #[test]
    fn mul_div_matches_u128_where_the_product_fits_and_the_wide_path_beyond() {
        let mut state = 0x3D1F_0000_0000_0004_u64;
        for _ in 0..4000 {
            let a = random_u128(&mut state);
            let b = random_u128(&mut state);
            let d = random_u128(&mut state).max(1);
            let expected = match a.checked_mul(b) {
                Some(product) => Some(product / d),
                None => {
                    let (high, low) = wide_mul(a, b);
                    div_wide(high, low, d).map(|(quotient, _)| quotient)
                }
            };
            assert_eq!(mul_div(a, b, d), expected, "{a} × {b} / {d}");
            // Order independence of the operands, whichever path is taken.
            assert_eq!(mul_div(a, b, d), mul_div(b, a, d));
        }
    }
}
