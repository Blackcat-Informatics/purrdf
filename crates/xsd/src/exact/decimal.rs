// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`Decimal`]: the unbounded `xsd:decimal` value — a big-integer coefficient and
//! an unbounded scale.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};
use std::str::FromStr;

use super::binary::{BINARY32, BINARY64, decompose_f32, decompose_f64, round_ratio};
use super::cost::{self, Cost};
use super::error::{BoundedTarget, ExactError, ExactKind};
use super::integer::{Integer, forward_by_value};
use super::rounding::{DivisionPolicy, Rounding};
use crate::bigint::BigInt;
use crate::numeric::Decimal as BoundedDecimal;

/// The bounded decimal's largest scale.
const BOUNDED_MAX_SCALE: u32 = 18;

/// Powers of ten that fit `i128`: `10^0` through `10^38`.
const MAX_I128_POW10: u32 = 38;

/// An `xsd:decimal` of any magnitude and any number of fractional digits:
/// `unscaled × 10^-scale`.
///
/// # Canonical form
///
/// The coefficient carries no trailing zero while the scale is positive, and zero
/// is `0 × 10^0`, so every value has exactly one representation: the derived
/// equality and hash are value equality and value hash, `scale` is the number of
/// fractional digits the canonical lexical form shows, and that form is a direct
/// rendering with no trimming. Every operation returns the canonical form.
///
/// Addition, subtraction and negation are exact and total. Multiplication is
/// exact and fails only when the result's scale would pass `u32::MAX` digits
/// ([`ExactError::ScaleOverflow`]). Division follows a caller-chosen
/// [`DivisionPolicy`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    unscaled: Integer,
    scale: u32,
}

impl Decimal {
    /// Zero.
    pub const ZERO: Self = Self {
        unscaled: Integer::ZERO,
        scale: 0,
    };

    /// One.
    pub const ONE: Self = Self {
        unscaled: Integer::ONE,
        scale: 0,
    };

    /// The value `unscaled × 10^-scale`, canonicalized.
    #[must_use]
    pub fn new(unscaled: Integer, scale: u32) -> Self {
        if unscaled.is_zero() {
            return Self::ZERO;
        }
        if scale == 0 {
            return Self { unscaled, scale };
        }
        if let Some(value) = unscaled.as_i128() {
            return Self::from_small(value, scale);
        }
        let big = unscaled.to_bigint();
        let zeros = u32::try_from(big.trailing_decimal_zeros().min(u64::from(scale)))
            .expect("bounded by a u32 scale");
        if zeros == 0 {
            return Self { unscaled, scale };
        }
        Self {
            unscaled: Integer::from_bigint(big.div_rem_pow10(u64::from(zeros)).0),
            scale: scale - zeros,
        }
    }

    /// The integer `value` as a decimal.
    #[must_use]
    pub const fn from_integer(value: Integer) -> Self {
        Self {
            unscaled: value,
            scale: 0,
        }
    }

    /// The canonical coefficient: the value is `unscaled × 10^-scale`.
    #[must_use]
    pub const fn unscaled(&self) -> &Integer {
        &self.unscaled
    }

    /// The canonical scale: the number of fractional digits of the canonical
    /// lexical form.
    #[must_use]
    pub const fn scale(&self) -> u32 {
        self.scale
    }

    /// Whether the value is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.unscaled.is_zero()
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.unscaled.is_negative()
    }

    /// `-1`, `0` or `1` by the sign of the value.
    #[must_use]
    pub fn signum(&self) -> i32 {
        self.unscaled.signum()
    }

    /// Whether the value is an integer.
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        self.scale == 0
    }

    /// The XSD 1.1 canonical `xsd:decimal` lexical form (§3.3.3.1, §E.1
    /// `decimalCanonicalMap`), identical to the bounded
    /// [`crate::numeric::Decimal::canonical_lexical`] wherever both represent the
    /// value: an integer value has no decimal point (`"3"`, `"-2"`, `"0"`); any
    /// other keeps exactly its significant fractional digits (`"2.5"`, `"-0.025"`).
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        self.to_string()
    }

    /// The exact value of a finite `f64`: every binary floating-point value is a
    /// finite decimal (`2^-k = 5^k / 10^k`), so this never rounds. `-0.0` is zero.
    ///
    /// # Errors
    ///
    /// [`ExactError::NotFinite`] (`err:FOCA0002`) for `NaN` and the infinities.
    pub fn from_f64(value: f64) -> Result<Self, ExactError> {
        Self::from_binary(decompose_f64(value).ok_or(ExactError::NotFinite)?)
    }

    /// The exact value of a finite `f32`; see [`Self::from_f64`].
    ///
    /// # Errors
    ///
    /// [`ExactError::NotFinite`] (`err:FOCA0002`) for `NaN` and the infinities.
    pub fn from_f32(value: f32) -> Result<Self, ExactError> {
        Self::from_binary(decompose_f32(value).ok_or(ExactError::NotFinite)?)
    }

    fn from_binary(
        (negative, significand, exponent): (bool, u64, i32),
    ) -> Result<Self, ExactError> {
        let signed = if negative {
            -i128::from(significand)
        } else {
            i128::from(significand)
        };
        let (unscaled, scale) = BigInt::from_binary(signed, exponent);
        Ok(Self::new(Integer::from_bigint(unscaled), scale))
    }

    /// The correctly rounded `f64` (to nearest, ties to even, subnormals
    /// included; `±∞` past the largest finite value) — the `xs:decimal` to
    /// `xs:double` cast, rounded once at any scale. A negative value that rounds
    /// to zero is `-0.0`.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        if let Some(value) = self.unscaled.as_i128()
            && self.scale <= MAX_I128_POW10
        {
            let scale = u8::try_from(self.scale).expect("at most 38");
            return crate::decimal_float::decimal_to_f64(value, scale);
        }
        // Past the format's range the answer is read off the leading-digit
        // position, so a long fraction never forms its `10^scale`.
        match self.beyond(cost::F64_DECIMAL_EXPONENTS) {
            Some(Ordering::Greater) => return self.signed(f64::INFINITY),
            Some(_) => return self.signed(0.0),
            None => {}
        }
        let bits = self.ratio_bits(&BINARY64);
        f64::from_bits(bits)
    }

    /// `Greater` when the value's magnitude is past `(overflow, underflow)`'s
    /// overflow exponent, `Less` when it is below the underflow one, `None`
    /// otherwise (zero included).
    fn beyond(&self, (overflow, underflow): (i64, i64)) -> Option<Ordering> {
        if self.is_zero() {
            return None;
        }
        let shape = self.shape();
        let exponent = shape.magnitude_exponent();
        let minimum = if shape.digits_exact {
            exponent
        } else {
            i64::try_from(cost::minimum_digits_for_bits(self.unscaled.binary_bits()))
                .unwrap_or(i64::MAX / 4)
                - i64::from(self.scale)
        };
        if minimum >= overflow {
            Some(Ordering::Greater)
        } else if exponent <= underflow {
            Some(Ordering::Less)
        } else {
            None
        }
    }

    /// `magnitude` with the value's sign.
    fn signed<F: Neg<Output = F>>(&self, magnitude: F) -> F {
        if self.is_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    /// The correctly rounded `f32`, rounded once (never through `f64`).
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        if let Some(value) = self.unscaled.as_i128()
            && self.scale <= MAX_I128_POW10
        {
            let scale = u8::try_from(self.scale).expect("at most 38");
            return crate::decimal_float::decimal_to_f32(value, scale);
        }
        match self.beyond(cost::F32_DECIMAL_EXPONENTS) {
            Some(Ordering::Greater) => return self.signed(f32::INFINITY),
            Some(_) => return self.signed(0.0),
            None => {}
        }
        let bits = self.ratio_bits(&BINARY32);
        f32::from_bits(u32::try_from(bits).expect("a binary32 pattern"))
    }

    fn ratio_bits(&self, format: &super::binary::Format) -> u64 {
        round_ratio(
            self.is_negative(),
            &self.unscaled.abs().to_bigint(),
            &BigInt::pow10(self.scale),
            format,
        )
    }

    /// The exact value of a bounded [`crate::numeric::Decimal`].
    #[must_use]
    pub fn from_bounded(value: &BoundedDecimal) -> Self {
        Self::new(
            Integer::from_i128(value.mantissa()),
            u32::from(value.scale()),
        )
    }

    /// The value as a bounded [`crate::numeric::Decimal`], exactly.
    ///
    /// # Errors
    ///
    /// [`ExactError::OutOfRange`] (`err:FOCA0001`) when the value needs more than
    /// eighteen fractional digits or a coefficient beyond `i128`. Narrowing never
    /// rounds: a caller that wants the nearest bounded value rounds first
    /// ([`Self::round`] to scale 18).
    pub fn to_bounded(&self) -> Result<BoundedDecimal, ExactError> {
        if self.scale > BOUNDED_MAX_SCALE {
            return Err(ExactError::out_of_range(
                BoundedTarget::BoundedDecimal,
                "decimal scale exceeds 18",
            ));
        }
        let mantissa = self.unscaled.as_i128().ok_or(ExactError::out_of_range(
            BoundedTarget::BoundedDecimal,
            "decimal coefficient exceeds i128",
        ))?;
        let scale = u8::try_from(self.scale).expect("at most 18");
        Ok(BoundedDecimal::from_parts(mantissa, scale))
    }

    /// The value with its fractional part discarded — the `xs:decimal` to
    /// `xs:integer` cast, of any magnitude.
    #[must_use]
    pub fn to_integer_truncated(&self) -> Integer {
        if self.scale == 0 {
            return self.unscaled.clone();
        }
        self.round_to_integer(Rounding::TowardZero)
    }

    /// The value rounded to an integer in direction `rounding` (`fn:floor` is
    /// [`Rounding::Floor`], `fn:ceiling` [`Rounding::Ceiling`], `fn:round`
    /// [`Rounding::HalfCeiling`]).
    #[must_use]
    pub fn round_to_integer(&self, rounding: Rounding) -> Integer {
        self.round(0, rounding).unscaled
    }

    /// The value rounded to `precision` fractional digits in direction `rounding`
    /// — `fn:round($v, $precision)` with [`Rounding::HalfCeiling`],
    /// `fn:round-half-to-even` with [`Rounding::HalfEven`]. A negative precision
    /// rounds to a multiple of `10^-precision`, as XPath F&O 3.1 defines.
    #[must_use]
    pub fn round(&self, precision: i32, rounding: Rounding) -> Self {
        let precision_wide = i64::from(precision);
        let scale = i64::from(self.scale);
        if precision_wide >= scale {
            return self.clone();
        }
        let negative = self.is_negative();
        // Digits to discard from the coefficient: positive, below 2^33.
        let drop = (scale - precision_wide).unsigned_abs();
        let (kept, increment) = discard_digits(&self.unscaled, drop, rounding, negative);
        let kept = if increment {
            if negative {
                &kept - &Integer::ONE
            } else {
                &kept + &Integer::ONE
            }
        } else {
            kept
        };
        if precision >= 0 {
            Self::new(kept, precision.unsigned_abs())
        } else {
            Self::new(scale_up(&kept, precision.unsigned_abs()), 0)
        }
    }

    /// `self × rhs`, exactly.
    ///
    /// # Errors
    ///
    /// [`ExactError::ScaleOverflow`] when the product's scale would pass
    /// `u32::MAX` digits.
    #[inline]
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, ExactError> {
        let scale = self
            .scale
            .checked_add(rhs.scale)
            .ok_or(ExactError::ScaleOverflow)?;
        if let (Some(a), Some(b)) = (self.unscaled.as_i128(), rhs.unscaled.as_i128())
            && let Some(product) = a.checked_mul(b)
        {
            return Ok(Self::from_small(product, scale));
        }
        Ok(self.mul_big(rhs, scale))
    }

    // Spilled multiplication/canonicalization owns a larger working frame;
    // keep it separate from the checked machine-coefficient caller.
    #[cold]
    fn mul_big(&self, rhs: &Self, scale: u32) -> Self {
        Self::new(&self.unscaled * &rhs.unscaled, scale)
    }

    /// The canonical decimal `value × 10^-scale` for an inline coefficient: no
    /// trailing zero to strip is the common case, decided by one remainder.
    #[inline]
    fn from_small(value: i128, scale: u32) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        let has_trailing_zero = mod10(value.unsigned_abs()) == 0;
        if scale == 0 || !has_trailing_zero {
            return Self {
                unscaled: Integer::from_i128(value),
                scale,
            };
        }
        Self::normalize_small(value, scale)
    }

    // Strip inline coefficients without constructing an owned Integer for the
    // generic spilled canonicalizer and carrying its larger temporary frame.
    #[cold]
    fn normalize_small(value: i128, scale: u32) -> Self {
        let negative = value < 0;
        let (magnitude, stripped) = strip_zeros(value.unsigned_abs(), scale);
        let magnitude = i128::try_from(magnitude).unwrap_or(i128::MIN);
        Self {
            // Only i128::MIN's magnitude fails the conversion, and it is
            // stripped of nothing (it is not a multiple of ten).
            unscaled: Integer::from_i128(if negative && magnitude != i128::MIN {
                -magnitude
            } else {
                magnitude
            }),
            scale: scale - stripped,
        }
    }

    /// Both coefficients at the larger scale, when both are inline and the
    /// shifted one still fits `i128`.
    fn aligned_small(&self, rhs: &Self) -> Option<(i128, i128, u32)> {
        let (a, b) = (self.unscaled.as_i128()?, rhs.unscaled.as_i128()?);
        let shift = |value: i128, digits: u32| {
            let power = i128::try_from(*POW10.get(usize::try_from(digits).ok()?)?).ok()?;
            value.checked_mul(power)
        };
        match self.scale.cmp(&rhs.scale) {
            Ordering::Equal => Some((a, b, self.scale)),
            Ordering::Less => Some((shift(a, rhs.scale - self.scale)?, b, rhs.scale)),
            Ordering::Greater => Some((a, shift(b, self.scale - rhs.scale)?, self.scale)),
        }
    }

    /// `op:numeric-divide`: `self ÷ rhs` under `policy` — see [`DivisionPolicy`].
    ///
    /// # Errors
    ///
    /// [`ExactError::DivisionByZero`] (`err:FOAR0001`) for a zero divisor;
    /// [`ExactError::NonTerminating`] under [`DivisionPolicy::Exact`] for a
    /// quotient with no finite expansion; [`ExactError::ScaleOverflow`] for a
    /// quotient scale past `u32::MAX`.
    pub fn div(&self, rhs: &Self, policy: DivisionPolicy) -> Result<Self, ExactError> {
        if rhs.is_zero() {
            return Err(ExactError::DivisionByZero);
        }
        if self.is_zero() {
            return Ok(Self::ZERO);
        }
        match policy {
            DivisionPolicy::Scale { scale, rounding } => self.div_rounded(rhs, scale, rounding),
            DivisionPolicy::Exact => self.div_exact(rhs),
        }
    }

    /// The quotient rounded to `scale` digits: with the operands
    /// `a·10^-sa` and `b·10^-sb`, the coefficient is
    /// `round(a · 10^(scale + sb − sa) / b)`.
    fn div_rounded(&self, rhs: &Self, scale: u32, rounding: Rounding) -> Result<Self, ExactError> {
        let negative = self.is_negative() != rhs.is_negative();
        let shift = i64::from(scale) + i64::from(rhs.scale) - i64::from(self.scale);
        let digits = u32::try_from(shift.unsigned_abs()).map_err(|_| ExactError::ScaleOverflow)?;
        if let Some(quotient) = self.div_rounded_small(rhs, shift, rounding, negative) {
            return Ok(Self::new(Integer::from(quotient), scale));
        }
        let (a, b) = (self.unscaled.abs(), rhs.unscaled.abs());
        let (numerator, denominator) = if shift >= 0 {
            (scale_up(&a, digits), b)
        } else {
            (a, scale_up(&b, digits))
        };
        let quotient = round_quotient(&numerator, &denominator, rounding, negative);
        let signed = if negative { -quotient } else { quotient };
        Ok(Self::new(signed, scale))
    }

    /// [`Self::div_rounded`] in machine arithmetic, when both coefficients and
    /// the scaled operand fit `u128`: the signed rounded quotient, or `None` to
    /// take the general path.
    fn div_rounded_small(
        &self,
        rhs: &Self,
        shift: i64,
        rounding: Rounding,
        negative: bool,
    ) -> Option<i128> {
        let (a, b) = (
            self.unscaled.as_i128()?.unsigned_abs(),
            rhs.unscaled.as_i128()?.unsigned_abs(),
        );
        let power = POW10.get(usize::try_from(shift.unsigned_abs()).ok()?)?;
        let (numerator, denominator) = if shift >= 0 {
            (a.checked_mul(*power)?, b)
        } else {
            (a, b.checked_mul(*power)?)
        };
        let quotient = numerator / denominator;
        let remainder = numerator - quotient * denominator;
        // Twice the remainder against the denominator, as the remainder against
        // what is left of the denominator: a scaled divisor can pass `2^127`, where
        // doubling the remainder would overflow `u128`.
        let half = remainder.cmp(&(denominator - remainder));
        let magnitude = quotient
            + u128::from(rounding.increments(negative, quotient & 1 == 1, half, remainder != 0));
        let magnitude = i128::try_from(magnitude).ok()?;
        Some(if negative { -magnitude } else { magnitude })
    }

    /// The exact quotient, or [`ExactError::NonTerminating`]. With
    /// `g = gcd(a, b)` and `b / g = 2^x · 5^y · r`, the quotient terminates exactly
    /// when `r = 1`, and then `a/b = (a/g) · 2^(k−x) · 5^(k−y) / 10^k` with
    /// `k = max(x, y)`.
    fn div_exact(&self, rhs: &Self) -> Result<Self, ExactError> {
        let negative = self.is_negative() != rhs.is_negative();
        let (a, b) = (
            self.unscaled.abs().to_bigint(),
            rhs.unscaled.abs().to_bigint(),
        );
        let g = a.gcd(&b);
        let reduced_a = a.div_rem(&g).expect("gcd of nonzero values is nonzero").0;
        let reduced_b = b.div_rem(&g).expect("gcd of nonzero values is nonzero").0;
        let (rest, twos) = strip_factor(reduced_b, 2);
        let (rest, fives) = strip_factor(rest, 5);
        if rest != BigInt::one() {
            return Err(ExactError::NonTerminating);
        }
        let k = twos.max(fives);
        let coefficient = reduced_a.mul_pow2(k - twos).mul_pow5(k - fives);
        // value = coefficient · 10^(sb − sa − k).
        let exponent = i64::from(rhs.scale) - i64::from(self.scale) - i64::from(k);
        let signed = Integer::from_bigint(if negative {
            coefficient.negated()
        } else {
            coefficient
        });
        if exponent >= 0 {
            let up = u32::try_from(exponent).map_err(|_| ExactError::ScaleOverflow)?;
            Ok(Self::new(scale_up(&signed, up), 0))
        } else {
            let scale = u32::try_from(-exponent).map_err(|_| ExactError::ScaleOverflow)?;
            Ok(Self::new(signed, scale))
        }
    }

    /// Both coefficients at the larger scale.
    fn aligned(&self, rhs: &Self) -> (Integer, Integer, u32) {
        match self.scale.cmp(&rhs.scale) {
            Ordering::Equal => (self.unscaled.clone(), rhs.unscaled.clone(), self.scale),
            Ordering::Less => (
                scale_up(&self.unscaled, rhs.scale - self.scale),
                rhs.unscaled.clone(),
                rhs.scale,
            ),
            Ordering::Greater => (
                self.unscaled.clone(),
                scale_up(&rhs.unscaled, self.scale - rhs.scale),
                self.scale,
            ),
        }
    }

    /// The position of the leading digit: `|value| ∈ [10^(e−1), 10^e)` for the
    /// returned `e` (a nonzero value only).
    fn magnitude_exponent(&self) -> i64 {
        i64::try_from(self.unscaled.decimal_digits()).unwrap_or(i64::MAX) - i64::from(self.scale)
    }

    // ----- resource governance ---------------------------------------------

    /// The coefficient's size in binary `u64` limbs.
    #[must_use]
    pub fn limb_len(&self) -> u64 {
        self.unscaled.limb_len()
    }

    /// Heap bytes the value holds.
    #[must_use]
    pub fn heap_bytes(&self) -> u64 {
        self.unscaled.heap_bytes()
    }

    /// The value's size, read without touching its digits.
    pub(crate) fn shape(&self) -> cost::Shape {
        match self.unscaled.as_i128() {
            Some(value) => cost::Shape::of_i128(value, u64::from(self.scale)),
            None => {
                let mut shape = self.unscaled.shape();
                shape.scale = u64::from(self.scale);
                shape
            }
        }
    }

    /// The cost of `self + rhs` or `self − rhs`: the coefficient with the smaller
    /// scale is shifted up to the larger one first.
    #[must_use]
    pub fn add_cost(&self, rhs: &Self) -> Cost {
        cost::decimal_add(self.shape(), rhs.shape())
    }

    /// The cost of comparing `self` with `rhs` (`Ord`): constant when the signs or
    /// the leading-digit positions differ, and an alignment no longer than the
    /// longer coefficient otherwise.
    #[must_use]
    pub fn cmp_cost(&self, rhs: &Self) -> Cost {
        cost::decimal_cmp(self.shape(), rhs.shape())
    }

    /// The cost of [`Self::try_mul`]: the coefficient product and its canonical
    /// form. The product's scale is the sum of the operands', so what it costs to
    /// render grows with the scales even when the coefficients stay short; that is
    /// [`Self::render_cost`] of the result, which a caller that renders it charges
    /// too.
    #[must_use]
    pub fn mul_cost(&self, rhs: &Self) -> Cost {
        cost::decimal_mul(self.shape(), rhs.shape())
    }

    /// The cost of negation, the absolute value or [`Self::round_to_integer`].
    #[must_use]
    pub fn unary_cost(&self) -> Cost {
        cost::decimal_unary(self.shape())
    }

    /// The cost of [`Self::cmp_f64`].
    #[must_use]
    pub fn cmp_f64_cost(&self) -> Cost {
        cost::decimal_cmp_f64(self.shape())
    }

    /// The cost of [`Self::div`] under `policy`.
    #[must_use]
    pub fn div_cost(&self, rhs: &Self, policy: DivisionPolicy) -> Cost {
        cost::decimal_div(self.shape(), rhs.shape(), policy)
    }

    /// The cost of [`Self::round`] (and the integer roundings).
    #[must_use]
    pub fn round_cost(&self, precision: i32) -> Cost {
        let limbs = self.limb_len();
        if i64::from(precision) >= i64::from(self.scale) {
            return Cost::new(limbs, cost::limb_bytes(limbs));
        }
        let shape = self.shape();
        // Excess fractional padding is discarded without constructing its power
        // of ten. Only the coefficient's digits can participate in a division.
        let drop = (i64::from(self.scale) - i64::from(precision))
            .unsigned_abs()
            .min(shape.digits);
        let conversion = cost::coefficient_conversion(shape);
        let up = if precision < 0 {
            u64::from(precision.unsigned_abs())
        } else {
            0
        };
        conversion
            .saturating_add(conversion)
            .saturating_add(conversion)
            .saturating_add(cost::shift10(0, drop))
            .saturating_add(cost::div(
                limbs,
                cost::limbs_for_digits(drop).saturating_add(1),
            ))
            .saturating_add(cost::decimal_unary(shape))
            .saturating_add(cost::shift10(limbs, up))
    }

    /// The cost of [`Self::to_f64`] / [`Self::to_f32`]: constant past the format's
    /// range, otherwise forming `10^scale` and one division whose quotient is at
    /// most five limbs.
    #[must_use]
    pub fn to_float_cost(&self) -> Cost {
        cost::decimal_to_float(self.shape())
    }

    /// The cost of rendering [`Self::canonical_lexical`]: every byte of the text,
    /// the leading zeros of a long fraction included.
    #[must_use]
    pub fn render_cost(&self) -> Cost {
        cost::render_shape(self.shape())
    }

    /// The exact order of the value against `value`; `None` only for `NaN`, and an
    /// infinity is past every decimal. Linear in the coefficient: the signs or the
    /// two magnitudes' leading positions decide most pairs outright, and otherwise
    /// the value's scale is within about 330 digits of its coefficient's length and
    /// both sides are scaled to integers and compared ([`Self::cmp_f64_cost`]).
    #[must_use]
    pub fn cmp_f64(&self, value: f64) -> Option<Ordering> {
        cmp_scaled_f64(&self.unscaled, self.scale, value)
    }
}

/// `unscaled × 10^-scale` against `value`, exactly; see [`Decimal::cmp_f64`].
pub(crate) fn cmp_scaled_f64(unscaled: &Integer, scale: u32, value: f64) -> Option<Ordering> {
    if value.is_nan() {
        return None;
    }
    if value.is_infinite() {
        return Some(if value > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let (negative, significand, exponent) = decompose_f64(value).expect("finite was checked");
    let other = if significand == 0 {
        0
    } else if negative {
        -1
    } else {
        1
    };
    let sign = unscaled.signum();
    if sign != other {
        return Some(sign.cmp(&other));
    }
    if sign == 0 {
        return Some(Ordering::Equal);
    }
    let magnitude = cmp_magnitude_binary(unscaled, scale, significand, exponent);
    Some(if sign < 0 {
        magnitude.reverse()
    } else {
        magnitude
    })
}

/// `|unscaled| × 10^-scale` against `significand × 2^exponent` (both nonzero).
fn cmp_magnitude_binary(
    unscaled: &Integer,
    scale: u32,
    significand: u64,
    exponent: i32,
) -> Ordering {
    // |decimal| ∈ [10^(e−1), 10^e) and binary ∈ [2^b, 2^(b+1)).
    let decimal_exponent =
        i64::try_from(unscaled.decimal_digits()).unwrap_or(i64::MAX / 4) - i64::from(scale);
    let binade = i64::from(significand.ilog2()) + i64::from(exponent);
    // `log2_of_pow10(x)` is within one unit of `⌊x·log2(10)⌋`, so
    // `x·log2(10) ∈ [L − 1, L + 2)`. 10^e < 2^b: decimal < binary.
    if super::binary::log2_of_pow10(decimal_exponent) + 2 <= binade {
        return Ordering::Less;
    }
    // 10^(e−1) ≥ 2^(b+1): decimal ≥ 2^(b+1) > binary.
    if super::binary::log2_of_pow10(decimal_exponent - 1) > binade + 1 {
        return Ordering::Greater;
    }
    // Close magnitudes: |unscaled| × 2^max(0, −exponent) against
    // significand × 2^max(0, exponent) × 10^scale, both integers.
    let mut left = unscaled.abs().to_bigint();
    let mut right = BigInt::from_u128(u128::from(significand));
    if exponent < 0 {
        left = left.mul_pow2(exponent.unsigned_abs());
    } else {
        right = right.mul_pow2(exponent.unsigned_abs());
    }
    left.cmp(&right.mul_pow10(scale))
}

/// `10^k` for `k ≤ 38`, every power of ten a `u128` holds.
const POW10: [u128; 39] = {
    let mut table = [1_u128; 39];
    let mut k = 1;
    while k < 39 {
        table[k] = table[k - 1] * 10;
        k += 1;
    }
    table
};

/// `x mod 10` from two 64-bit halves (`2^64 ≡ 6 (mod 10)`), so the
/// trailing-zero test of a 128-bit coefficient costs two word remainders rather
/// than a 128-bit division.
const fn mod10(x: u128) -> u64 {
    let high = (x >> 64) as u64;
    let low = x as u64;
    ((high % 10) * 6 + low % 10) % 10
}

/// `magnitude` with up to `limit` trailing decimal zeros removed, and how many
/// were. Halving steps (16, 8, 4, 2, 1 digits) on a `u64` when the magnitude
/// fits one, so the common case is a handful of multiply-shift remainders
/// rather than a library call per digit.
fn strip_zeros(magnitude: u128, limit: u32) -> (u128, u32) {
    let mut removed = 0_u32;
    if let Ok(mut small) = u64::try_from(magnitude) {
        if limit == 0 || !small.is_multiple_of(10) {
            return (magnitude, 0);
        }
        for (step, unit) in [
            (16_u32, 10_000_000_000_000_000_u64),
            (8, 100_000_000),
            (4, 10_000),
            (2, 100),
            (1, 10),
        ] {
            while removed + step <= limit && small != 0 && small.is_multiple_of(unit) {
                small /= unit;
                removed += step;
            }
        }
        return (u128::from(small), removed);
    }
    let mut value = magnitude;
    while removed < limit && value != 0 && mod10(value) == 0 {
        value /= 10;
        removed += 1;
    }
    if removed > 0 {
        // Back in u64 range, the rest strips cheaply.
        let (value, more) = strip_zeros(value, limit - removed);
        return (value, removed + more);
    }
    (value, removed)
}

/// `value × 10^digits`, exactly.
fn scale_up(value: &Integer, digits: u32) -> Integer {
    if digits == 0 || value.is_zero() {
        return value.clone();
    }
    if digits <= MAX_I128_POW10
        && let Some(small) = value.as_i128()
        && let Some(product) =
            small.checked_mul(i128::try_from(POW10[digits as usize]).expect("10^38 < 2^127"))
    {
        return Integer::from_i128(product);
    }
    Integer::from_bigint(value.to_bigint().mul_pow10(digits))
}

/// `⌊n / d⌋` rounded in direction `rounding` for the exact sign `negative`
/// (`n`, `d` non-negative magnitudes, `d` nonzero); the magnitude of the result.
pub(crate) fn round_quotient(
    numerator: &Integer,
    denominator: &Integer,
    rounding: Rounding,
    negative: bool,
) -> Integer {
    let (quotient, remainder) = numerator
        .div_rem(denominator)
        .expect("the denominator is nonzero");
    if remainder.is_zero() {
        return quotient;
    }
    let twice = &remainder + &remainder;
    let half = twice.cmp(denominator);
    if rounding.increments(negative, quotient.is_odd(), half, true) {
        &quotient + &Integer::ONE
    } else {
        quotient
    }
}

/// Split `coefficient`'s magnitude at `drop` digits: the kept (truncated) signed
/// coefficient, and whether `rounding` increments its magnitude.
fn discard_digits(
    coefficient: &Integer,
    drop: u64,
    rounding: Rounding,
    negative: bool,
) -> (Integer, bool) {
    if coefficient.is_zero() {
        return (Integer::ZERO, false);
    }
    if drop > coefficient.decimal_digits() {
        // Everything is discarded and the excess is below half a unit
        // (`|c| < 10^digits ≤ 10^(drop − 1)`), but nonzero.
        let increment = rounding.increments(negative, false, Ordering::Less, true);
        return (Integer::ZERO, increment);
    }
    if let Some(value) = coefficient.as_i128()
        && drop <= u64::from(MAX_I128_POW10)
    {
        let unit = POW10[usize::try_from(drop).expect("at most 38")];
        let magnitude = value.unsigned_abs();
        let (kept, rest) = (magnitude / unit, magnitude % unit);
        // `rest < 10^38`, so doubling it stays inside u128.
        let half = (rest * 2).cmp(&unit);
        let increment = rounding.increments(negative, kept & 1 == 1, half, rest != 0);
        let kept = Integer::from(kept);
        return (if negative { -kept } else { kept }, increment);
    }
    let (kept, rest) = coefficient.to_bigint().div_rem_pow10(drop);
    let increment = if rest.is_zero() {
        false
    } else {
        // 0 < 2·rest < 2·10^drop, so it reaches 10^drop exactly when it has
        // drop + 1 digits, and equals it when those are a one and drop zeros.
        let twice = rest.abs().mul_small(2);
        let half = if twice.decimal_digits() <= drop {
            Ordering::Less
        } else if twice.trailing_decimal_zeros() == drop {
            Ordering::Equal
        } else {
            Ordering::Greater
        };
        rounding.increments(negative, kept.is_odd(), half, true)
    };
    (Integer::from_bigint(kept), increment)
}

/// `value` with every factor `prime` (2 or 5) divided out, and how many there
/// were, in chunks of the largest power that fits one machine division.
pub(crate) fn strip_factor(mut value: BigInt, prime: u64) -> (BigInt, u32) {
    let (chunk, chunk_power) = if prime == 2 {
        (1_u64 << 32, 32)
    } else {
        (5_u64.pow(27), 27)
    };
    let mut count = 0_u32;
    loop {
        match value.div_rem_u64(chunk) {
            Some((quotient, 0)) if !value.is_zero() => {
                value = quotient;
                count += chunk_power;
            }
            _ => break,
        }
    }
    loop {
        match value.div_rem_u64(prime) {
            Some((quotient, 0)) if !value.is_zero() => {
                value = quotient;
                count += 1;
            }
            _ => break,
        }
    }
    (value, count)
}

impl Default for Decimal {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<i128> for Decimal {
    fn from(value: i128) -> Self {
        Self::from_integer(Integer::from_i128(value))
    }
}

impl TryFrom<&Decimal> for BoundedDecimal {
    type Error = ExactError;
    fn try_from(value: &Decimal) -> Result<Self, ExactError> {
        value.to_bounded()
    }
}

impl FromStr for Decimal {
    type Err = ExactError;
    /// Parse the `xsd:decimal` lexical space (XSD 1.1 Part 2 §3.3.3.1) of any
    /// length: an optional sign, then digits with at most one `.` and at least one
    /// digit (`.5`, `1.`, `-0012.3400`). No whitespace is trimmed, matching
    /// [`crate::numeric::parse_decimal`]. Linear in the length.
    ///
    /// # Errors
    ///
    /// [`ExactError::InvalidLexical`] for text outside the lexical space;
    /// [`ExactError::ScaleOverflow`] for more than `u32::MAX` fractional digits.
    fn from_str(lexical: &str) -> Result<Self, ExactError> {
        if !crate::numeric::is_decimal_lexical(lexical) {
            return Err(ExactError::invalid(
                lexical,
                ExactKind::Decimal,
                "expected an optional sign then digits with at most one '.'",
            ));
        }
        let negative = lexical.starts_with('-');
        let body = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
        let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
        let scale = u32::try_from(fraction.len()).map_err(|_| ExactError::ScaleOverflow)?;
        let whole = whole.trim_start_matches('0');
        let unscaled = if whole.len() + fraction.len()
            <= usize::try_from(MAX_I128_POW10).expect("a small constant")
        {
            // At most 38 digits: the coefficient is an i128, read in one pass.
            let magnitude = whole
                .bytes()
                .chain(fraction.bytes())
                .fold(0_i128, |acc, digit| acc * 10 + i128::from(digit - b'0'));
            Integer::from_i128(if negative { -magnitude } else { magnitude })
        } else {
            let mut digits = String::with_capacity(whole.len() + fraction.len() + 1);
            if negative {
                digits.push('-');
            }
            digits.push_str(whole);
            digits.push_str(fraction);
            Integer::from_bigint(
                BigInt::from_digits(&digits).expect("validated as a decimal lexical"),
            )
        };
        Ok(Self::new(unscaled, scale))
    }
}

impl fmt::Display for Decimal {
    /// The canonical lexical form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return fmt::Display::fmt(&self.unscaled, f);
        }
        let digits = self.unscaled.abs().to_string();
        if self.is_negative() {
            f.write_str("-")?;
        }
        let scale = usize::try_from(self.scale).unwrap_or(usize::MAX);
        if digits.len() > scale {
            let split = digits.len() - scale;
            f.write_str(&digits[..split])?;
            f.write_str(".")?;
            f.write_str(&digits[split..])
        } else {
            f.write_str("0.")?;
            for _ in 0..scale - digits.len() {
                f.write_str("0")?;
            }
            f.write_str(&digits)
        }
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decimal({self})")
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let (sa, sb) = (self.signum(), other.signum());
        if sa != sb {
            return sa.cmp(&sb);
        }
        if sa == 0 {
            return Ordering::Equal;
        }
        // Same sign, both nonzero: the leading-digit position decides first, so
        // two values of wildly different scale compare without aligning.
        let (a_low, a_high) = self.shape().comparison_exponents();
        let (b_low, b_high) = other.shape().comparison_exponents();
        let separated = if a_low > b_high {
            Some(Ordering::Greater)
        } else if b_low > a_high {
            Some(Ordering::Less)
        } else {
            None
        };
        let magnitude = match separated
            .unwrap_or_else(|| self.magnitude_exponent().cmp(&other.magnitude_exponent()))
        {
            Ordering::Equal => {
                let (a, b, _) = self.aligned(other);
                a.abs().cmp(&b.abs())
            }
            unequal => unequal,
        };
        if sa < 0 {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<&Decimal> for &Decimal {
    type Output = Decimal;
    fn add(self, rhs: &Decimal) -> Decimal {
        if let Some((a, b, scale)) = self.aligned_small(rhs)
            && let Some(sum) = a.checked_add(b)
        {
            return Decimal::from_small(sum, scale);
        }
        let (a, b, scale) = self.aligned(rhs);
        Decimal::new(&a + &b, scale)
    }
}

impl Sub<&Decimal> for &Decimal {
    type Output = Decimal;
    fn sub(self, rhs: &Decimal) -> Decimal {
        if let Some((a, b, scale)) = self.aligned_small(rhs)
            && let Some(difference) = a.checked_sub(b)
        {
            return Decimal::from_small(difference, scale);
        }
        let (a, b, scale) = self.aligned(rhs);
        Decimal::new(&a - &b, scale)
    }
}

impl Neg for &Decimal {
    type Output = Decimal;
    fn neg(self) -> Decimal {
        Decimal {
            unscaled: -&self.unscaled,
            scale: self.scale,
        }
    }
}

impl Neg for Decimal {
    type Output = Self;
    fn neg(self) -> Self {
        -&self
    }
}

forward_by_value!(Decimal: Add add, Sub sub);
