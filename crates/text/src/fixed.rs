// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact base-10 fixed-point arithmetic — the determinism foundation of the
//! text index.
//!
//! # Why not floats
//!
//! Ranking needs a natural logarithm, and the obvious way to get one is the
//! double-precision `ln` in the standard library. That would make this crate's
//! answers depend on which libm the target links: a libm `ln` is permitted to
//! differ by a unit in the last place
//! between implementations, and it does: the same `x` can come back as two
//! adjacent doubles on x86-64 and on `wasm32-unknown-unknown`. One such
//! difference is enough to swap the order of two documents whose scores are
//! nearly tied, so the same engine over the same data would return rows in a
//! different order depending on where it was built. That is an answer
//! divergence, not a rounding detail, and this workspace forbids it.
//!
//! So there is no floating point here at all. [`Fixed`] is an `i128` read as a
//! multiple of `10^-12` ([`SCALE_DIGITS`] fractional digits), every operation is
//! integer arithmetic, and [`Fixed::ln`] is a fixed-length integer series. The
//! crate root carries `#![deny(clippy::float_arithmetic)]`, so a float cannot be
//! reintroduced by accident.
//!
//! # Rounding
//!
//! Every operation that cannot be represented exactly truncates **toward zero**,
//! and does so exactly once, at the end. Truncation is chosen over
//! round-half-to-even because it needs no tie-break rule and therefore has no
//! second convention anyone can implement differently; doing it once rather than
//! per intermediate is what keeps the error bounded by a single unit in the last
//! place rather than by the length of the computation.
//!
//! # Overflow
//!
//! An intermediate that does not fit is a [`TextError::Overflow`], never a
//! wrapped or saturated value. A wrapped score is a wrong ranking presented as a
//! right one, which is precisely the failure this crate exists to rule out.

use crate::error::TextError;

/// How many base-10 fractional digits a [`Fixed`] carries.
///
/// Twelve: enough that the accumulated truncation error across a BM25 term sum
/// stays many orders of magnitude below the gap between two scores that are
/// meaningfully different, and small enough that the `i128` representation still
/// spans values up to roughly `1.7 × 10^26`.
pub const SCALE_DIGITS: u32 = 12;

/// `10^SCALE_DIGITS` — the divisor relating a [`Fixed`]'s raw integer to its
/// value.
const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

/// `SCALE` as an unsigned value, for the magnitude arithmetic below.
const SCALE_U: u128 = SCALE as u128;

/// How many base-10 fractional digits [`Fixed::ln`] works at internally.
///
/// Six more than [`SCALE_DIGITS`], so the series' own truncation error is six
/// digits below anything the returned value can express and the single rounding
/// back to [`SCALE_DIGITS`] is the only one that shows.
const INTERNAL_DIGITS: u32 = 18;

/// `10^INTERNAL_DIGITS` — the scale [`Fixed::ln`]'s series runs at.
///
/// The headroom argument for `i128`: every reduced operand in the series is
/// below `2 × 10^18`, so every product is below `4 × 10^36`, comfortably inside
/// `i128::MAX ≈ 1.7 × 10^38`.
const INTERNAL_SCALE: i128 = 10_i128.pow(INTERNAL_DIGITS);

/// The factor between [`INTERNAL_SCALE`] and [`SCALE`].
const INTERNAL_TO_SCALE: i128 = 10_i128.pow(INTERNAL_DIGITS - SCALE_DIGITS);

/// `ln 2` at [`INTERNAL_SCALE`], truncated toward zero.
///
/// The exact value begins
/// `0.693147180559945309417232121458176568075500134360255254120680...`
/// so the first eighteen fractional digits are `693147180559945309` and the
/// next digit is `4` — truncating and rounding to nearest agree here, and the
/// constant is therefore the closest `10^-18` multiple below `ln 2`, with an
/// error under `4.2 × 10^-19`.
const LN2_INTERNAL: i128 = 693_147_180_559_945_309;

/// How many terms of the `atanh` series [`Fixed::ln`] sums — a **fixed count**,
/// never a convergence test.
///
/// This is the single most important line in the module. A loop that stops when
/// successive terms differ by less than some epsilon produces a result that
/// depends on the order the compiler evaluated the comparison in and on how the
/// intermediate happened to be held; a loop that always runs exactly this many
/// times produces a result that is a pure function of its input, on every
/// target, forever. The count is therefore part of the crate's contract and not
/// a tuning parameter.
///
/// Twenty is enough by a wide margin. The reduced argument satisfies
/// `z < 1/3`, so the first omitted term is below `3^-41 / 41 ≈ 10^-21` — three
/// orders of magnitude below one unit at [`INTERNAL_SCALE`], and nine below one
/// unit at [`SCALE_DIGITS`].
const SERIES_TERMS: u32 = 20;

/// A base-10 fixed-point number: the value is the raw integer divided by ten
/// raised to [`SCALE_DIGITS`].
///
/// Ordering, equality and hashing are the raw integer's, which is exactly the
/// numeric ordering because the scale is shared: there is no unnormalized
/// representation of a value, so two [`Fixed`]s are equal if and only if they
/// denote the same number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Fixed(i128);

impl Fixed {
    /// The additive identity.
    pub const ZERO: Self = Self(0);

    /// The multiplicative identity.
    pub const ONE: Self = Self(SCALE);

    /// The exact value `value`, as a fixed-point number.
    ///
    /// An `i64` scaled by `10^12` is at most about `9.2 × 10^30`, so this never
    /// actually overflows an `i128`; the [`Result`] is kept so that widening the
    /// input type later cannot silently become a wrapping conversion.
    pub fn from_integer(value: i64) -> Result<Self, TextError> {
        i128::from(value)
            .checked_mul(SCALE)
            .map(Self)
            .ok_or_else(|| {
                TextError::overflow(format!("{value} does not fit the fixed-point range"))
            })
    }

    /// Reinterpret a raw integer as a fixed-point number: the result denotes
    /// `raw` divided by ten raised to [`SCALE_DIGITS`].
    pub const fn from_raw(raw: i128) -> Self {
        Self(raw)
    }

    /// This number's raw integer — the value multiplied by ten raised to
    /// [`SCALE_DIGITS`].
    pub const fn into_raw(self) -> i128 {
        self.0
    }

    /// `self + other`, or [`TextError::Overflow`] if the sum does not fit.
    pub fn checked_add(self, other: Self) -> Result<Self, TextError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| TextError::overflow("addition left the fixed-point range"))
    }

    /// `self - other`, or [`TextError::Overflow`] if the difference does not
    /// fit.
    pub fn checked_sub(self, other: Self) -> Result<Self, TextError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| TextError::overflow("subtraction left the fixed-point range"))
    }

    /// `self × other`, truncated toward zero.
    ///
    /// The raw computation is `self.raw × other.raw / 10^12`, whose intermediate
    /// product routinely exceeds an `i128` even when the result does not — two
    /// values near `10^13` already overflow it. It is therefore computed through
    /// a helper that falls back to a 256-bit intermediate rather than reporting
    /// an overflow the answer does not have.
    pub fn checked_mul(self, other: Self) -> Result<Self, TextError> {
        let magnitude = mul_div(self.0.unsigned_abs(), other.0.unsigned_abs(), SCALE_U)
            .ok_or_else(|| TextError::overflow("multiplication left the fixed-point range"))?;
        signed(magnitude, (self.0 < 0) != (other.0 < 0))
    }

    /// `self ÷ other`, truncated toward zero.
    ///
    /// The raw computation is `self.raw × 10^12 / other.raw`, again through that
    /// same helper so that the scaling multiplication cannot overflow a result
    /// that is representable.
    ///
    /// Division by zero is a [`TextError::Domain`]: there is no value to return,
    /// so none is invented.
    pub fn checked_div(self, other: Self) -> Result<Self, TextError> {
        if other.0 == 0 {
            return Err(TextError::domain("division by zero"));
        }
        let magnitude = mul_div(self.0.unsigned_abs(), SCALE_U, other.0.unsigned_abs())
            .ok_or_else(|| TextError::overflow("division left the fixed-point range"))?;
        signed(magnitude, (self.0 < 0) != (other.0 < 0))
    }

    /// The natural logarithm of `self`, exact to the last representable digit
    /// and identical on every target.
    ///
    /// # Method
    ///
    /// 1. **Range reduction.** Find the integer `e` with `m = self / 2^e` in
    ///    `[1, 2)`, so that `ln self = e·ln 2 + ln m`. `e` is found by comparing
    ///    bit lengths and correcting by at most one step — no logarithm is
    ///    needed to find it, which is the point.
    /// 2. **Series.** With `z = (m - 1) / (m + 1)`, `ln m = 2·atanh z =
    ///    2·Σ z^(2k+1)/(2k+1)`. Because `m < 2`, `z < 1/3`, so the series
    ///    converges geometrically and twenty terms are more than enough. The
    ///    count is FIXED and is never a convergence test: that is what makes the
    ///    result a pure function of its input on every target, so a ranking
    ///    cannot differ between a native and a WebAssembly build.
    /// 3. **One rounding.** Steps 1 and 2 run at eighteen fractional digits; the
    ///    result is truncated to [`SCALE_DIGITS`] exactly once, at the end.
    ///
    /// # Errors
    ///
    /// `self <= 0` is a [`TextError::Domain`]: the natural logarithm is not
    /// defined there, and returning a sentinel would let a caller rank against a
    /// number that means nothing.
    pub fn ln(self) -> Result<Self, TextError> {
        if self.0 <= 0 {
            return Err(TextError::domain(format!(
                "ln is undefined at {}, which is not positive",
                self.to_decimal_lexical()
            )));
        }

        let raw = self.0.unsigned_abs();
        let exponent = binade(raw);

        // m at INTERNAL_SCALE. For a non-negative exponent the reduction is a
        // division by 2^e, for a negative one a multiplication by 2^-e; both are
        // folded into the same exact `mul · a / b` so the scaling to
        // INTERNAL_SCALE never rounds twice.
        let (numerator, denominator) = if exponent >= 0 {
            (INTERNAL_TO_SCALE_U, 1_u128 << exponent.unsigned_abs())
        } else {
            (INTERNAL_TO_SCALE_U << exponent.unsigned_abs(), 1_u128)
        };
        let mantissa_magnitude = mul_div(raw, numerator, denominator)
            .ok_or_else(|| TextError::overflow("range reduction left the fixed-point range"))?;
        let mantissa = i128::try_from(mantissa_magnitude)
            .map_err(|_| TextError::overflow("range reduction left the fixed-point range"))?;

        // z = (m - 1) / (m + 1) at INTERNAL_SCALE. `mantissa` is in
        // [10^18, 2·10^18), so the numerator is at most 10^18 and the product
        // below is at most 10^36 — inside i128 by two orders of magnitude.
        let z = (mantissa - INTERNAL_SCALE) * INTERNAL_SCALE / (mantissa + INTERNAL_SCALE);

        // 2·atanh(z), summed over a fixed number of terms.
        let z_squared = z * z / INTERNAL_SCALE;
        let mut term = z;
        let mut sum: i128 = 0;
        for k in 0..SERIES_TERMS {
            sum += term / i128::from(2 * k + 1);
            term = term * z_squared / INTERNAL_SCALE;
        }
        let ln_mantissa = 2 * sum;

        let total = i128::from(exponent)
            .checked_mul(LN2_INTERNAL)
            .and_then(|scaled| scaled.checked_add(ln_mantissa))
            .ok_or_else(|| TextError::overflow("logarithm left the fixed-point range"))?;

        // The one and only rounding.
        Ok(Self(total / INTERNAL_TO_SCALE))
    }

    /// The exact `xsd:decimal` lexical form of this value: an optional `-`, the
    /// integer part, `.`, and exactly [`SCALE_DIGITS`] fractional digits.
    ///
    /// Exact, not a rendering choice: `10^-12` is representable in decimal
    /// without residue, so twelve fractional digits reproduce the raw integer
    /// with nothing lost. Trailing zeros are kept rather than trimmed, because a
    /// fixed field width makes the output byte-deterministic and orders
    /// lexically the same way the values order numerically for any two
    /// same-signed numbers with the same number of integer digits.
    pub fn to_decimal_lexical(self) -> String {
        let magnitude = self.0.unsigned_abs();
        let integer = magnitude / SCALE_U;
        let fraction = magnitude % SCALE_U;
        let sign = if self.0 < 0 { "-" } else { "" };
        let width = SCALE_DIGITS as usize;
        format!("{sign}{integer}.{fraction:0width$}")
    }
}

/// [`INTERNAL_TO_SCALE`] as an unsigned value, for the magnitude arithmetic in
/// [`Fixed::ln`]'s range reduction.
const INTERNAL_TO_SCALE_U: u128 = INTERNAL_TO_SCALE as u128;

/// Reassemble a magnitude and a sign into a [`Fixed`], or report the overflow.
fn signed(magnitude: u128, negative: bool) -> Result<Fixed, TextError> {
    let raw = i128::try_from(magnitude)
        .map_err(|_| TextError::overflow("result left the fixed-point range"))?;
    Ok(Fixed::from_raw(if negative { -raw } else { raw }))
}

/// `floor(log2(raw / 10^SCALE_DIGITS))` for a strictly positive `raw`.
///
/// Found by comparing bit lengths rather than by any logarithm. If `raw` has `b`
/// significant bits and the scale has `s`, then `log2(raw/10^12)` lies strictly
/// between `b - s - 1` and `b - s + 1`, so its floor is one of two candidates
/// and a single comparison settles which.
///
/// Both shifts below are provably in range: `raw` is at most `i128::MAX`, so
/// `b <= 127` and the candidate is at most `87`, and `SCALE_U << 87` is about
/// `1.5 × 10^38`, inside `u128`. In the other direction the candidate is at
/// least `-39`, and a `raw` that small has fewer than 40 significant bits, so
/// `raw << 39` is below `2^79`.
fn binade(raw: u128) -> i32 {
    debug_assert!(raw > 0, "binade is only defined for a positive magnitude");
    let raw_bits = 128 - raw.leading_zeros() as i32;
    let scale_bits = 128 - SCALE_U.leading_zeros() as i32;
    let candidate = raw_bits - scale_bits;
    let below = if candidate >= 0 {
        raw < (SCALE_U << candidate.unsigned_abs())
    } else {
        (raw << candidate.unsigned_abs()) < SCALE_U
    };
    if below { candidate - 1 } else { candidate }
}

/// `a × b / c`, truncated, over the full `u128` range — `None` if `c` is zero or
/// the quotient does not fit a `u128`.
///
/// The fast path is one multiplication and one division, taken whenever the
/// product fits. The slow path exists because the products this crate forms are
/// routinely larger than their quotients: scaling by `10^12` before dividing
/// overflows a `u128` for any operand above about `3.4 × 10^26`, while the
/// answer is perfectly representable. Reporting that as an overflow would make
/// the arithmetic's range depend on the order the operations were written in.
fn mul_div(a: u128, b: u128, c: u128) -> Option<u128> {
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

/// The full 256-bit product of two `u128`s, as `(high, low)`.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const HALF: u32 = 64;
    let mask = u128::from(u64::MAX);

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
fn div_wide(high: u128, low: u128, divisor: u128) -> u128 {
    debug_assert!(high < divisor, "the quotient must fit a u128");
    let mut remainder = high;
    let mut quotient: u128 = 0;
    for bit in (0..128_u32).rev() {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{Fixed, SCALE, SCALE_DIGITS, binade, mul_div};
    use crate::error::TextError;

    #[test]
    fn the_constants_denote_what_they_say() {
        assert_eq!(Fixed::ZERO.into_raw(), 0);
        assert_eq!(Fixed::ONE.into_raw(), SCALE);
        assert_eq!(Fixed::from_integer(1).expect("representable"), Fixed::ONE);
        assert_eq!(SCALE_DIGITS, 12);
    }

    /// Ordering is numeric, including across the sign.
    #[test]
    fn ordering_is_numeric() {
        let minus_one = Fixed::from_integer(-1).expect("representable");
        let half = Fixed::from_raw(SCALE / 2);
        assert!(minus_one < Fixed::ZERO);
        assert!(Fixed::ZERO < half);
        assert!(half < Fixed::ONE);
    }

    /// The range reduction's exponent, at the boundaries where an off-by-one
    /// would hide: an exact power of two, one unit below it, and both sides of
    /// the value 1.
    #[test]
    fn the_binade_brackets_its_input() {
        // 1.0 -> 2^0
        assert_eq!(binade(SCALE.unsigned_abs()), 0);
        // just under 1.0 -> 2^-1
        assert_eq!(binade((SCALE - 1).unsigned_abs()), -1);
        // 2.0 -> 2^1
        assert_eq!(binade((2 * SCALE).unsigned_abs()), 1);
        // just under 2.0 -> 2^0
        assert_eq!(binade((2 * SCALE - 1).unsigned_abs()), 0);
        // 0.5 -> 2^-1
        assert_eq!(binade((SCALE / 2).unsigned_abs()), -1);
        // the smallest representable positive value, 10^-12
        assert_eq!(binade(1), -40);
    }

    /// `mul_div` must agree with the direct computation wherever the direct one
    /// is available, and must keep answering past the point where it is not.
    #[test]
    fn mul_div_matches_the_direct_route_and_outlives_it() {
        assert_eq!(mul_div(7, 6, 3), Some(14));
        assert_eq!(mul_div(1, 1, 0), None, "a zero divisor has no quotient");

        // Past the fast path: the product needs 256 bits, the quotient 128.
        let big = u128::MAX / 2;
        assert!(big.checked_mul(4).is_none(), "the fast path must be closed");
        assert_eq!(mul_div(big, 4, 4), Some(big));
        assert_eq!(mul_div(big, 4, 2), Some(big * 2));

        // A quotient that genuinely does not fit is refused rather than wrapped.
        assert_eq!(mul_div(u128::MAX, u128::MAX, 1), None);
    }

    #[test]
    fn multiplication_and_division_are_exact_on_representable_values() {
        let three = Fixed::from_integer(3).expect("representable");
        let four = Fixed::from_integer(4).expect("representable");
        let twelve = Fixed::from_integer(12).expect("representable");
        assert_eq!(three.checked_mul(four).expect("no overflow"), twelve);
        assert_eq!(twelve.checked_div(four).expect("no overflow"), three);
    }

    #[test]
    fn signs_compose_as_they_should() {
        let minus_three = Fixed::from_integer(-3).expect("representable");
        let two = Fixed::from_integer(2).expect("representable");
        assert_eq!(
            minus_three.checked_mul(two).expect("no overflow"),
            Fixed::from_integer(-6).expect("representable")
        );
        assert_eq!(
            minus_three
                .checked_mul(minus_three)
                .expect("no overflow")
                .into_raw(),
            9 * SCALE
        );
    }

    #[test]
    fn division_by_zero_is_a_domain_error() {
        let one = Fixed::ONE;
        assert!(matches!(
            one.checked_div(Fixed::ZERO),
            Err(TextError::Domain(_))
        ));
    }

    #[test]
    fn ln_of_one_is_exactly_zero() {
        assert_eq!(Fixed::ONE.ln().expect("1 is in the domain"), Fixed::ZERO);
    }

    #[test]
    fn ln_of_a_non_positive_value_is_a_domain_error() {
        assert!(matches!(Fixed::ZERO.ln(), Err(TextError::Domain(_))));
        assert!(matches!(
            Fixed::from_integer(-1).expect("representable").ln(),
            Err(TextError::Domain(_))
        ));
    }

    /// `ln 2` must reproduce the module's own constant, truncated to the public
    /// scale — the one place the hard-coded digits are checked against the
    /// series that has to agree with them.
    #[test]
    fn ln_of_two_reproduces_the_constant() {
        let two = Fixed::from_integer(2).expect("representable");
        assert_eq!(
            two.ln().expect("2 is in the domain").to_decimal_lexical(),
            "0.693147180559"
        );
    }

    /// The lexical form is fixed-width and signed, with no trailing-zero
    /// trimming and no rounding.
    #[test]
    fn the_lexical_form_is_exact_and_fixed_width() {
        assert_eq!(Fixed::ZERO.to_decimal_lexical(), "0.000000000000");
        assert_eq!(Fixed::ONE.to_decimal_lexical(), "1.000000000000");
        assert_eq!(Fixed::from_raw(1).to_decimal_lexical(), "0.000000000001");
        assert_eq!(Fixed::from_raw(-1).to_decimal_lexical(), "-0.000000000001");
        assert_eq!(
            Fixed::from_integer(-3)
                .expect("representable")
                .to_decimal_lexical(),
            "-3.000000000000"
        );
    }

    /// The same input always yields the same bytes — the property the whole
    /// module exists for, asserted at the level a caller can observe.
    #[test]
    fn ln_is_a_pure_function() {
        let x = Fixed::from_raw(1_234_567_890_123);
        let first = x.ln().expect("positive");
        for _ in 0..1_000 {
            assert_eq!(x.ln().expect("positive"), first);
        }
    }

    #[test]
    fn addition_and_subtraction_report_overflow() {
        let huge = Fixed::from_raw(i128::MAX);
        assert!(matches!(
            huge.checked_add(Fixed::ONE),
            Err(TextError::Overflow(_))
        ));
        let tiny = Fixed::from_raw(i128::MIN);
        assert!(matches!(
            tiny.checked_sub(Fixed::ONE),
            Err(TextError::Overflow(_))
        ));
    }
}
