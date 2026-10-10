// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! Public [`Fixed`] operations report [`TextError::Overflow`] only when their
//! result does not fit; products use the shared wide-integer home. Ranking
//! intermediates additionally promote through the existing exact [`Integer`]
//! home and return to [`Fixed`] under their prepared query's score certificate.
//! Neither path wraps or saturates a score.

use purrdf_xsd::{exact::Integer, wide::mul_div};

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
pub(crate) const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

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
        product_ratio(self.0, other.0, SCALE)
            .ok_or_else(|| TextError::overflow("multiplication left the fixed-point range"))
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
        product_ratio(self.0, SCALE, other.0)
            .ok_or_else(|| TextError::overflow("division left the fixed-point range"))
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

/// The magnitude of [`i128::MIN`] — `2^127`, the one magnitude that is
/// representable negative and not positive.
///
/// Two's complement is asymmetric, and a magnitude-plus-sign reassembly that
/// forgets it refuses a value the range does hold. Named rather than spelled
/// inline so the asymmetry is visible at the one place it matters.
const NEGATIVE_LIMIT_MAGNITUDE: u128 = i128::MIN.unsigned_abs();

/// Reassemble a magnitude and sign in the public raw representation.
///
/// The `negative` branch is not symmetric with the positive one, and cannot be.
/// `i128` spans `-2^127 ..= 2^127 - 1`, so the magnitude `2^127` is
/// representable *only* with a negative sign. Deciding representability with
/// `i128::try_from` alone — which is a positive-range test — refuses that one
/// value, and it refuses it for a computation whose exact answer is
/// [`i128::MIN`]: `Fixed::from_raw(i128::MIN).checked_mul(Fixed::ONE)` is a
/// multiplication by one, and it came back an overflow. That is an over-refusal
/// rather than a rounding detail — the arithmetic's usable range depended on the
/// sign of the operands rather than on the answer — so the negative extreme is
/// admitted here explicitly.
fn signed_value(magnitude: u128, negative: bool) -> Option<i128> {
    if negative && magnitude == NEGATIVE_LIMIT_MAGNITUDE {
        return Some(i128::MIN);
    }
    let raw = i128::try_from(magnitude).ok()?;
    Some(if negative { -raw } else { raw })
}

/// One scaled-product kernel, including signed truncation toward zero.
fn product_ratio(left: i128, right: i128, divisor: i128) -> Option<Fixed> {
    let magnitude = mul_div(
        left.unsigned_abs(),
        right.unsigned_abs(),
        divisor.unsigned_abs(),
    )?;
    signed_value(magnitude, (left < 0) ^ (right < 0) ^ (divisor < 0)).map(Fixed::from_raw)
}

/// Ranking intermediates at the same scale as Fixed. The existing exact
/// Integer owns the canonical i128/BigInt representation; this adapter supplies
/// only the original scaling and rounding boundaries, not an integer engine.
pub(crate) struct Scaled(Integer);

impl Scaled {
    pub(crate) const ZERO: Self = Self(Integer::ZERO);

    pub(crate) const fn from_fixed(value: Fixed) -> Self {
        Self(Integer::from_i128(value.into_raw()))
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self(&self.0 + &other.0)
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self(Self::quotient_product(
            &self.0,
            &other.0,
            &Integer::from_i128(SCALE),
        ))
    }

    pub(crate) fn div(&self, other: &Self) -> Result<Self, TextError> {
        if other.0.is_zero() {
            return Err(TextError::domain("division by zero"));
        }
        Ok(Self(Self::quotient_product(
            &self.0,
            &Integer::from_i128(SCALE),
            &other.0,
        )))
    }

    pub(crate) fn into_fixed(self) -> Result<Fixed, TextError> {
        self.0
            .as_i128()
            .map(Fixed::from_raw)
            .ok_or_else(|| TextError::overflow("result left the fixed-point range"))
    }

    /// Stay on the existing wide kernel when the rounded result fits, even if
    /// the unscaled product exceeds i128; otherwise use exact Integer once.
    fn quotient_product(left: &Integer, right: &Integer, divisor: &Integer) -> Integer {
        if let (Some(a), Some(b), Some(c)) = (left.as_i128(), right.as_i128(), divisor.as_i128())
            && let Some(value) = product_ratio(a, b, c)
        {
            return Integer::from_i128(value.into_raw());
        }
        (left * right)
            .div_rem(divisor)
            .expect("the caller verified a nonzero divisor")
            .0
    }
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

#[cfg(test)]
mod tests {

    use super::{Fixed, SCALE, SCALE_DIGITS, binade};
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
