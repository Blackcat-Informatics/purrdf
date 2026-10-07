// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`Integer`]: the unbounded `xsd:integer` value, with an inline `i128` fast path.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

use super::binary::decompose_f64;
use super::cost::{self, Cost};
use super::error::{BoundedTarget, ExactError, ExactKind};
use crate::bigint::BigInt;

/// An `xsd:integer` of any magnitude.
///
/// # Representation
///
/// A value that fits `i128` is held inline and computed on with checked machine
/// arithmetic; only a result that leaves `i128` allocates, as a [`BigInt`], and a
/// result that comes back into range is held inline again. The split is
/// canonical — a value has exactly one representation — so the derived
/// equality and hash are value equality and value hash, and the small-value path
/// costs one checked machine operation and no allocation (the `exact` bench
/// measures it against the bounded `i128` path).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Integer(Repr);

/// See [`Integer`]: `Big` only ever holds a value outside `i128`.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Repr {
    Small(i128),
    Big(BigInt),
}

impl Integer {
    /// Zero.
    pub const ZERO: Self = Self(Repr::Small(0));
    /// One.
    pub const ONE: Self = Self(Repr::Small(1));

    /// The integer `value`, exactly.
    #[must_use]
    pub const fn from_i128(value: i128) -> Self {
        Self(Repr::Small(value))
    }

    /// The integer a [`BigInt`] holds, exactly.
    #[must_use]
    pub fn from_bigint(value: BigInt) -> Self {
        value
            .to_i128()
            .map_or(Self(Repr::Big(value)), |small| Self(Repr::Small(small)))
    }

    /// The value as a [`BigInt`] (allocates for an inline value).
    #[must_use]
    pub fn to_bigint(&self) -> BigInt {
        match &self.0 {
            Repr::Small(value) => BigInt::from_i128(*value),
            Repr::Big(value) => value.clone(),
        }
    }

    /// The value when it fits `i128` — the small-value fast path's own view.
    #[must_use]
    pub const fn as_i128(&self) -> Option<i128> {
        match self.0 {
            Repr::Small(value) => Some(value),
            Repr::Big(_) => None,
        }
    }

    /// The XSD 1.1 canonical `xsd:integer` lexical form: an optional `-`, then
    /// digits with no leading zero (`"0"` for zero).
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        self.to_string()
    }

    /// Whether the value is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        matches!(self.0, Repr::Small(0))
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        match &self.0 {
            Repr::Small(value) => *value < 0,
            Repr::Big(value) => value.is_negative(),
        }
    }

    /// `-1`, `0` or `1` by the sign of the value.
    #[must_use]
    pub fn signum(&self) -> i32 {
        match &self.0 {
            Repr::Small(value) => i32::try_from(value.signum()).expect("a sign"),
            Repr::Big(value) => value.signum(),
        }
    }

    /// Whether the value is odd.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        match &self.0 {
            Repr::Small(value) => value & 1 == 1,
            Repr::Big(value) => value.is_odd(),
        }
    }

    /// The absolute value (`fn:abs`).
    #[must_use]
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            -self
        } else {
            self.clone()
        }
    }

    /// Truncating division (`op:numeric-integer-divide` and `op:numeric-mod`):
    /// `(quotient, remainder)` with the quotient rounded toward zero and the
    /// remainder carrying the dividend's sign.
    ///
    /// # Errors
    ///
    /// [`ExactError::DivisionByZero`] (`err:FOAR0001`) for a zero divisor.
    pub fn div_rem(&self, divisor: &Self) -> Result<(Self, Self), ExactError> {
        if divisor.is_zero() {
            return Err(ExactError::DivisionByZero);
        }
        if let (Repr::Small(a), Repr::Small(b)) = (&self.0, &divisor.0)
            && let (Some(quotient), Some(remainder)) = (a.checked_div(*b), a.checked_rem(*b))
        {
            return Ok((Self::from_i128(quotient), Self::from_i128(remainder)));
        }
        let (quotient, remainder) = self
            .to_bigint()
            .div_rem(&divisor.to_bigint())
            .ok_or(ExactError::DivisionByZero)?;
        Ok((Self::from_bigint(quotient), Self::from_bigint(remainder)))
    }

    /// `self^exp`, exactly (`0^0 = 1`). The result has up to
    /// `limb_len × exp` limbs: charge [`Self::pow_cost`] first when `exp` is
    /// caller-controlled.
    #[must_use]
    pub fn pow(&self, exp: u32) -> Self {
        if let Repr::Small(value) = self.0
            && let Some(result) = value.checked_pow(exp)
        {
            return Self::from_i128(result);
        }
        Self::from_bigint(self.to_bigint().pow(exp))
    }

    /// The greatest common divisor of the magnitudes (non-negative; `gcd(0, 0) = 0`).
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        if let (Repr::Small(a), Repr::Small(b)) = (&self.0, &other.0) {
            let divisor = crate::wide::gcd(a.unsigned_abs(), b.unsigned_abs());
            return Self::from_bigint(BigInt::from_u128(divisor));
        }
        Self::from_bigint(self.to_bigint().gcd(&other.to_bigint()))
    }

    /// The value as an `i128`, the machine-word `xsd:integer` representation
    /// ([`crate::XsdValue::Integer`]).
    ///
    /// # Errors
    ///
    /// [`ExactError::OutOfRange`] (`err:FOCA0003`) when the value does not fit;
    /// never a wrapped or saturated value.
    pub fn to_i128(&self) -> Result<i128, ExactError> {
        self.as_i128().ok_or(ExactError::out_of_range(
            BoundedTarget::I128,
            "integer magnitude exceeds i128",
        ))
    }

    /// The value as an `i64`.
    ///
    /// # Errors
    ///
    /// [`ExactError::OutOfRange`] (`err:FOCA0003`) when the value does not fit.
    pub fn to_i64(&self) -> Result<i64, ExactError> {
        self.as_i128()
            .and_then(|value| i64::try_from(value).ok())
            .ok_or(ExactError::out_of_range(
                BoundedTarget::I64,
                "integer magnitude exceeds i64",
            ))
    }

    /// The correctly rounded `f64` (to nearest, ties to even; `±∞` past the
    /// largest finite value) — the `xs:integer` to `xs:double` cast.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        match &self.0 {
            // The integer-to-float cast is itself correctly rounded.
            Repr::Small(value) => *value as f64,
            Repr::Big(value) => value.to_f64(),
        }
    }

    /// The correctly rounded `f32`, rounded once (never through `f64`).
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        match &self.0 {
            Repr::Small(value) => *value as f32,
            Repr::Big(value) => value.to_f32(),
        }
    }

    /// The `xs:double`/`xs:float` to `xs:integer` cast (XPath F&O 3.1
    /// §19.1.2.4): the value with its fractional part discarded, of any magnitude.
    ///
    /// # Errors
    ///
    /// [`ExactError::NotFinite`] (`err:FOCA0002`) for `NaN` and the infinities.
    pub fn from_f64_truncated(value: f64) -> Result<Self, ExactError> {
        let (negative, significand, exponent) =
            decompose_f64(value).ok_or(ExactError::NotFinite)?;
        let magnitude = if exponent >= 0 {
            BigInt::from_u128(u128::from(significand)).mul_pow2(exponent.unsigned_abs())
        } else if exponent <= -64 {
            BigInt::zero()
        } else {
            BigInt::from_u128(u128::from(significand >> exponent.unsigned_abs()))
        };
        let value = Self::from_bigint(magnitude);
        Ok(if negative { -value } else { value })
    }

    // ----- resource governance ---------------------------------------------

    /// The size of the magnitude in base-`1e9` limbs — the unit every cost
    /// estimate is stated in (an inline value counts the limbs it would occupy,
    /// at most five).
    #[must_use]
    pub fn limb_len(&self) -> u64 {
        match &self.0 {
            Repr::Small(0) => 0,
            Repr::Small(value) => u64::from(value.unsigned_abs().ilog10() / 9 + 1),
            Repr::Big(value) => value.limb_len() as u64,
        }
    }

    /// The number of decimal digits in the magnitude (`1` for zero).
    #[must_use]
    pub fn decimal_digits(&self) -> u64 {
        match &self.0 {
            Repr::Small(0) => 1,
            Repr::Small(value) => u64::from(value.unsigned_abs().ilog10()) + 1,
            Repr::Big(value) => value.decimal_digits(),
        }
    }

    /// Heap bytes the value holds (zero inline).
    #[must_use]
    pub fn heap_bytes(&self) -> u64 {
        match &self.0 {
            Repr::Small(_) => 0,
            Repr::Big(value) => cost::limb_bytes(value.limb_len() as u64),
        }
    }

    /// The cost of `self + other` or `self − other`.
    #[must_use]
    pub fn add_cost(&self, other: &Self) -> Cost {
        cost::add(self.limb_len(), other.limb_len())
    }

    /// The cost of `self × other`.
    #[must_use]
    pub fn mul_cost(&self, other: &Self) -> Cost {
        cost::mul(self.limb_len(), other.limb_len())
    }

    /// The value's size, read without touching its digits.
    pub(crate) fn shape(&self) -> cost::Shape {
        match &self.0 {
            Repr::Small(value) => cost::Shape::of_i128(*value, 0),
            Repr::Big(value) => cost::Shape {
                limbs: value.limb_len() as u64,
                digits: value.decimal_digits(),
                scale: 0,
                sign: value.signum(),
            },
        }
    }

    /// The cost of comparing `self` with `other`: constant unless the signs and
    /// lengths agree, then one pass over the limbs.
    #[must_use]
    pub fn cmp_cost(&self, other: &Self) -> Cost {
        cost::decimal_cmp(self.shape(), other.shape())
    }

    /// The cost of negation or the absolute value.
    #[must_use]
    pub fn unary_cost(&self) -> Cost {
        cost::decimal_unary(self.shape())
    }

    /// The cost of [`Self::to_f64`] / [`Self::to_f32`]: constant past `10^309`
    /// (an infinity, read off the length), and otherwise a base conversion of at
    /// most 35 limbs.
    #[must_use]
    pub fn to_float_cost(&self) -> Cost {
        if self.decimal_digits() >= 310 {
            return Cost::new(1, 0);
        }
        let limbs = self.limb_len();
        Cost::new(
            limbs.saturating_mul(limbs).saturating_add(1),
            cost::limb_bytes(limbs),
        )
    }

    /// The cost of rendering [`Self::canonical_lexical`].
    #[must_use]
    pub fn render_cost(&self) -> Cost {
        cost::render_shape(self.shape())
    }

    /// The cost of [`Self::cmp_f64`].
    #[must_use]
    pub fn cmp_f64_cost(&self) -> Cost {
        cost::decimal_cmp_f64(self.shape())
    }

    /// The exact order of the value against `value`; `None` only for `NaN`, and an
    /// infinity is past every integer. Linear in the limbs, with no rounding of
    /// either side.
    #[must_use]
    pub fn cmp_f64(&self, value: f64) -> Option<Ordering> {
        super::decimal::cmp_scaled_f64(self, 0, value)
    }

    /// The cost of [`Self::div_rem`].
    #[must_use]
    pub fn div_rem_cost(&self, divisor: &Self) -> Cost {
        cost::div(self.limb_len(), divisor.limb_len())
    }

    /// The cost of [`Self::pow`] — charge it before raising to a
    /// caller-controlled exponent: the result alone is `limb_len × exp` limbs.
    #[must_use]
    pub fn pow_cost(&self, exp: u32) -> Cost {
        cost::pow(self.log2_upper_q16(), exp)
    }

    /// An upper bound on `log2|self|` in Q16 fixed point (zero for zero and one).
    pub(crate) fn log2_upper_q16(&self) -> u64 {
        match &self.0 {
            Repr::Small(value) => cost::log2_upper_q16(value.unsigned_abs()),
            Repr::Big(value) => {
                // |value| < (leading + 1) × 1e9^(limbs − 2).
                let below = (value.limb_len() as u64).saturating_sub(2);
                cost::log2_upper_q16(u128::from(value.leading_u64()) + 1)
                    .saturating_add(below.saturating_mul(cost::LOG2_LIMB_Q16_UP))
            }
        }
    }

    /// The cost of [`Self::gcd`].
    #[must_use]
    pub fn gcd_cost(&self, other: &Self) -> Cost {
        cost::gcd(self.limb_len(), other.limb_len())
    }

    /// Both operands as [`BigInt`]s, for the slow path.
    fn big_pair(&self, other: &Self) -> (BigInt, BigInt) {
        (self.to_bigint(), other.to_bigint())
    }
}

impl Default for Integer {
    fn default() -> Self {
        Self::ZERO
    }
}

/// `From` every machine integer that `i128` holds: an inline value.
macro_rules! from_machine_integers {
    ($($source:ty),+) => {$(
        impl From<$source> for Integer {
            fn from(value: $source) -> Self {
                Self(Repr::Small(i128::from(value)))
            }
        }
    )+};
}

from_machine_integers!(i8, i16, i32, i64, i128, u8, u16, u32, u64);

impl From<u128> for Integer {
    fn from(value: u128) -> Self {
        Self::from_bigint(BigInt::from_u128(value))
    }
}

impl From<&Integer> for BigInt {
    fn from(value: &Integer) -> Self {
        value.to_bigint()
    }
}

impl TryFrom<&Integer> for i128 {
    type Error = ExactError;
    fn try_from(value: &Integer) -> Result<Self, ExactError> {
        value.to_i128()
    }
}

impl TryFrom<&Integer> for i64 {
    type Error = ExactError;
    fn try_from(value: &Integer) -> Result<Self, ExactError> {
        value.to_i64()
    }
}

impl FromStr for Integer {
    type Err = ExactError;
    /// Parse the `xsd:integer` lexical space (XSD 1.1 Part 2 §3.4.13.1): an
    /// optional `+` or `-`, then one or more ASCII digits, of any length. No
    /// whitespace is trimmed (a caller applying the `collapse` facet trims first),
    /// matching [`crate::numeric::parse_integer`]. Linear in the length: the
    /// base-`1e9` limbs are read straight off nine-digit groups.
    ///
    /// # Errors
    ///
    /// [`ExactError::InvalidLexical`] for text outside the lexical space.
    fn from_str(lexical: &str) -> Result<Self, ExactError> {
        if !crate::numeric::is_integer_lexical(lexical) {
            return Err(ExactError::invalid(
                lexical,
                ExactKind::Integer,
                "expected an optional sign then digits",
            ));
        }
        // Up to 38 digits always fits i128; the machine parse is the fast path.
        let body = lexical
            .strip_prefix(['+', '-'])
            .map_or(lexical.len(), str::len);
        if body <= 38
            && let Ok(value) = lexical.parse::<i128>()
        {
            return Ok(Self::from_i128(value));
        }
        let value = BigInt::from_digits(lexical).ok_or_else(|| {
            ExactError::invalid(
                lexical,
                ExactKind::Integer,
                "expected an optional sign then digits",
            )
        })?;
        Ok(Self::from_bigint(value))
    }
}

impl fmt::Display for Integer {
    /// The canonical lexical form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Small(value) => fmt::Display::fmt(value, f),
            Repr::Big(value) => fmt::Display::fmt(value, f),
        }
    }
}

impl fmt::Debug for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Integer({self})")
    }
}

impl Ord for Integer {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Repr::Small(a), Repr::Small(b)) => a.cmp(b),
            // A big value lies outside i128, so its sign alone places it.
            (Repr::Small(_), Repr::Big(b)) => {
                if b.is_negative() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Repr::Big(a), Repr::Small(_)) => {
                if a.is_negative() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (Repr::Big(a), Repr::Big(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Integer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<&Integer> for &Integer {
    type Output = Integer;
    fn add(self, rhs: &Integer) -> Integer {
        if let (Repr::Small(a), Repr::Small(b)) = (&self.0, &rhs.0)
            && let Some(sum) = a.checked_add(*b)
        {
            return Integer::from_i128(sum);
        }
        let (a, b) = self.big_pair(rhs);
        Integer::from_bigint(&a + &b)
    }
}

impl Sub<&Integer> for &Integer {
    type Output = Integer;
    fn sub(self, rhs: &Integer) -> Integer {
        if let (Repr::Small(a), Repr::Small(b)) = (&self.0, &rhs.0)
            && let Some(difference) = a.checked_sub(*b)
        {
            return Integer::from_i128(difference);
        }
        let (a, b) = self.big_pair(rhs);
        Integer::from_bigint(&a - &b)
    }
}

impl Mul<&Integer> for &Integer {
    type Output = Integer;
    fn mul(self, rhs: &Integer) -> Integer {
        if let (Repr::Small(a), Repr::Small(b)) = (&self.0, &rhs.0)
            && let Some(product) = a.checked_mul(*b)
        {
            return Integer::from_i128(product);
        }
        let (a, b) = self.big_pair(rhs);
        Integer::from_bigint(&a * &b)
    }
}

impl Neg for &Integer {
    type Output = Integer;
    fn neg(self) -> Integer {
        match &self.0 {
            Repr::Small(value) => value.checked_neg().map_or_else(
                || Integer::from_bigint(BigInt::from_i128(*value).negated()),
                Integer::from_i128,
            ),
            Repr::Big(value) => Integer::from_bigint(value.negated()),
        }
    }
}

impl Neg for Integer {
    type Output = Self;
    fn neg(self) -> Self {
        -&self
    }
}

/// By-value operator forms, forwarding to the borrowed ones.
macro_rules! forward_by_value {
    ($ty:ty: $($trait:ident $method:ident),+) => {$(
        impl $trait<$ty> for $ty {
            type Output = $ty;
            fn $method(self, rhs: $ty) -> $ty {
                $trait::$method(&self, &rhs)
            }
        }
        impl $trait<&$ty> for $ty {
            type Output = $ty;
            fn $method(self, rhs: &$ty) -> $ty {
                $trait::$method(&self, rhs)
            }
        }
    )+};
}
pub(crate) use forward_by_value;

forward_by_value!(Integer: Add add, Sub sub, Mul mul);
