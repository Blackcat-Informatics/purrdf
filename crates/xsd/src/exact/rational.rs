// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`Rational`]: the unbounded `owl:rational` value — the exact carrier for a
//! reasoned result whose decimal expansion does not terminate.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

use super::binary::{BINARY32, BINARY64, decompose_f64, round_ratio};
use super::cost::{self, Cost};
use super::decimal::{Decimal, round_quotient, strip_factor};
use super::error::{BoundedTarget, ExactError, ExactKind};
use super::integer::{Integer, forward_by_value};
use super::rounding::{DivisionPolicy, Rounding};
use crate::bigint::BigInt;
use crate::rational::Rational as BoundedRational;
use crate::value::XsdValue;

/// An exact rational of any size: `numerator / denominator`, reduced, with a
/// positive denominator.
///
/// The field `+ − × ÷` is closed here — every operation is exact, and only
/// division by zero fails — which is why reasoned mathematics (exact quotients,
/// probabilities, partition-function ratios) computes in this type and projects
/// to `xsd:decimal` once, at the end, with [`Self::to_decimal`] under a stated
/// [`DivisionPolicy`]: exactly when the value terminates
/// ([`DivisionPolicy::Exact`]), or rounded once to a stated scale.
///
/// # Canonical form
///
/// `gcd(numerator, denominator) = 1` and `denominator > 0`, so the derived
/// equality and hash are value equality and value hash.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: Integer,
    denominator: Integer,
}

impl Rational {
    /// Zero.
    pub const ZERO: Self = Self {
        numerator: Integer::ZERO,
        denominator: Integer::ONE,
    };

    /// One.
    pub const ONE: Self = Self {
        numerator: Integer::ONE,
        denominator: Integer::ONE,
    };

    /// `numerator / denominator`, reduced.
    ///
    /// # Errors
    ///
    /// [`ExactError::DivisionByZero`] (`err:FOAR0001`) for a zero denominator.
    pub fn new(numerator: Integer, denominator: Integer) -> Result<Self, ExactError> {
        if denominator.is_zero() {
            return Err(ExactError::DivisionByZero);
        }
        Ok(Self::reduced(numerator, denominator))
    }

    /// `numerator / denominator` for a nonzero denominator, reduced.
    fn reduced(numerator: Integer, denominator: Integer) -> Self {
        let (numerator, denominator) = if denominator.is_negative() {
            (-numerator, -denominator)
        } else {
            (numerator, denominator)
        };
        if numerator.is_zero() {
            return Self::ZERO;
        }
        let divisor = numerator.gcd(&denominator);
        if divisor == Integer::ONE {
            return Self {
                numerator,
                denominator,
            };
        }
        let exact = |value: &Integer| {
            value
                .div_rem(&divisor)
                .expect("a gcd of a nonzero value is nonzero")
                .0
        };
        Self {
            numerator: exact(&numerator),
            denominator: exact(&denominator),
        }
    }

    /// The integer `value`.
    #[must_use]
    pub const fn from_integer(value: Integer) -> Self {
        Self {
            numerator: value,
            denominator: Integer::ONE,
        }
    }

    /// The exact value of a decimal: `unscaled / 10^scale`, reduced.
    #[must_use]
    pub fn from_decimal(value: &Decimal) -> Self {
        if value.scale() == 0 {
            return Self::from_integer(value.unscaled().clone());
        }
        Self::reduced(
            value.unscaled().clone(),
            Integer::from_bigint(BigInt::pow10(value.scale())),
        )
    }

    /// The exact value of a finite `f64` (a dyadic rational).
    ///
    /// # Errors
    ///
    /// [`ExactError::NotFinite`] (`err:FOCA0002`) for `NaN` and the infinities.
    pub fn from_f64(value: f64) -> Result<Self, ExactError> {
        let (negative, significand, exponent) =
            decompose_f64(value).ok_or(ExactError::NotFinite)?;
        let significand = Integer::from(significand);
        let numerator = if negative { -significand } else { significand };
        Ok(if exponent >= 0 {
            Self::from_integer(Integer::from_bigint(
                numerator.to_bigint().mul_pow2(exponent.unsigned_abs()),
            ))
        } else {
            Self::reduced(
                numerator,
                Integer::from_bigint(BigInt::one().mul_pow2(exponent.unsigned_abs())),
            )
        })
    }

    /// The exact value of a finite `f32`.
    ///
    /// # Errors
    ///
    /// [`ExactError::NotFinite`] (`err:FOCA0002`) for `NaN` and the infinities.
    pub fn from_f32(value: f32) -> Result<Self, ExactError> {
        Self::from_f64(f64::from(value))
    }

    /// The reduced numerator (carrying the sign).
    #[must_use]
    pub const fn numerator(&self) -> &Integer {
        &self.numerator
    }

    /// The reduced denominator (positive).
    #[must_use]
    pub const fn denominator(&self) -> &Integer {
        &self.denominator
    }

    /// Whether the value is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.numerator.is_negative()
    }

    /// Whether the value is an integer.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.denominator == Integer::ONE
    }

    /// Whether the value has a finite decimal expansion (its reduced denominator
    /// has no prime factor but 2 and 5) — that is, whether [`Self::to_decimal`]
    /// under [`DivisionPolicy::Exact`] succeeds.
    #[must_use]
    pub fn is_terminating(&self) -> bool {
        let (rest, _) = strip_factor(self.denominator.to_bigint(), 2);
        let (rest, _) = strip_factor(rest, 5);
        rest == BigInt::one()
    }

    /// The canonical `owl:rational` lexical form: the reduced
    /// `numerator/denominator`, the denominator always written (`"1/3"`,
    /// `"-5/1"`, `"0/1"`), so the text stays in the `owl:rational` lexical space.
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        self.to_string()
    }

    /// The absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            -self
        } else {
            self.clone()
        }
    }

    /// `1 / self`.
    ///
    /// # Errors
    ///
    /// [`ExactError::DivisionByZero`] for zero.
    pub fn recip(&self) -> Result<Self, ExactError> {
        Self::new(self.denominator.clone(), self.numerator.clone())
    }

    /// `self ÷ rhs`, exactly.
    ///
    /// # Errors
    ///
    /// [`ExactError::DivisionByZero`] (`err:FOAR0001`) for a zero divisor.
    pub fn checked_div(&self, rhs: &Self) -> Result<Self, ExactError> {
        if rhs.is_zero() {
            return Err(ExactError::DivisionByZero);
        }
        Ok(Self::reduced(
            &self.numerator * &rhs.denominator,
            &self.denominator * &rhs.numerator,
        ))
    }

    /// The integer nearest the value in direction `rounding`.
    #[must_use]
    pub fn round_to_integer(&self, rounding: Rounding) -> Integer {
        let negative = self.is_negative();
        let magnitude =
            round_quotient(&self.numerator.abs(), &self.denominator, rounding, negative);
        if negative { -magnitude } else { magnitude }
    }

    /// Project to `xsd:decimal` under `policy`: under [`DivisionPolicy::Exact`]
    /// the exact expansion or [`ExactError::NonTerminating`]; under
    /// [`DivisionPolicy::Scale`] the value rounded once to that scale.
    ///
    /// # Errors
    ///
    /// [`ExactError::NonTerminating`] as above; [`ExactError::ScaleOverflow`] for
    /// a scale past `u32::MAX`.
    pub fn to_decimal(&self, policy: DivisionPolicy) -> Result<Decimal, ExactError> {
        Decimal::from_integer(self.numerator.clone())
            .div(&Decimal::from_integer(self.denominator.clone()), policy)
    }

    /// The correctly rounded `f64` (to nearest, ties to even, subnormals
    /// included, `±∞` past the largest finite value; a negative value that rounds
    /// to zero is `-0.0`).
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        f64::from_bits(self.ratio_bits(&BINARY64))
    }

    /// The correctly rounded `f32`, rounded once.
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        f32::from_bits(u32::try_from(self.ratio_bits(&BINARY32)).expect("a binary32 pattern"))
    }

    fn ratio_bits(&self, format: &super::binary::Format) -> u64 {
        round_ratio(
            self.is_negative(),
            &self.numerator.abs().to_bigint(),
            &self.denominator.to_bigint(),
            format,
        )
    }

    /// The exact value of a bounded [`crate::rational::Rational`].
    #[must_use]
    pub fn from_bounded(value: &BoundedRational) -> Self {
        // Already reduced with a positive denominator.
        Self {
            numerator: Integer::from_i128(value.numerator()),
            denominator: Integer::from_i128(value.denominator()),
        }
    }

    /// The value as a bounded [`crate::rational::Rational`].
    ///
    /// # Errors
    ///
    /// [`ExactError::OutOfRange`] (`err:FOAR0002`) when the numerator or the
    /// denominator does not fit `i128`.
    pub fn to_bounded(&self) -> Result<BoundedRational, ExactError> {
        let refuse = || {
            ExactError::out_of_range(
                BoundedTarget::BoundedRational,
                "rational component exceeds i128",
            )
        };
        let numerator = self.numerator.as_i128().ok_or_else(refuse)?;
        let denominator = self.denominator.as_i128().ok_or_else(refuse)?;
        BoundedRational::new(numerator, denominator).map_err(|_| refuse())
    }

    /// The exact value an [`XsdValue`] holds on the integer or decimal branch;
    /// `None` otherwise (the OWL 2 datatype map keeps `xsd:float`/`xsd:double`
    /// disjoint from the rationals, so they convert only explicitly, through
    /// [`Self::from_f64`]).
    #[must_use]
    pub fn from_xsd(value: &XsdValue) -> Option<Self> {
        value
            .to_exact_decimal()
            .map(|decimal| Self::from_decimal(&decimal))
    }

    // ----- resource governance ---------------------------------------------

    /// The larger component's size in binary `u64` limbs.
    #[must_use]
    pub fn limb_len(&self) -> u64 {
        self.numerator.limb_len().max(self.denominator.limb_len())
    }

    /// Heap bytes the value holds.
    #[must_use]
    pub fn heap_bytes(&self) -> u64 {
        self.numerator.heap_bytes() + self.denominator.heap_bytes()
    }

    /// The cost of `self + rhs` or `self − rhs`: three products, a sum and the
    /// reduction.
    #[must_use]
    pub fn add_cost(&self, rhs: &Self) -> Cost {
        let (la, lb) = (self.limb_len(), rhs.limb_len());
        let cross = cost::mul(la, lb);
        let sum = la.saturating_add(lb).saturating_add(1);
        cross
            .saturating_add(cross)
            .saturating_add(cross)
            .saturating_add(cost::add(sum, sum))
            .saturating_add(reduce_cost(sum))
    }

    /// The cost of `self × rhs` or [`Self::checked_div`]: two products and the
    /// reduction.
    #[must_use]
    pub fn mul_cost(&self, rhs: &Self) -> Cost {
        let (la, lb) = (self.limb_len(), rhs.limb_len());
        let cross = cost::mul(la, lb);
        cross
            .saturating_add(cross)
            .saturating_add(reduce_cost(la.saturating_add(lb)))
    }

    /// The cost of [`Self::to_decimal`] under `policy`.
    #[must_use]
    pub fn to_decimal_cost(&self, policy: DivisionPolicy) -> Cost {
        Decimal::from_integer(self.numerator.clone())
            .div_cost(&Decimal::from_integer(self.denominator.clone()), policy)
    }

    /// The cost of [`Self::to_f64`] / [`Self::to_f32`].
    #[must_use]
    pub fn to_float_cost(&self) -> Cost {
        let longer = self.limb_len().saturating_add(5);
        cost::div(longer, self.denominator.limb_len().saturating_add(5))
    }
}

/// Reducing a fraction whose components have at most `limbs` limbs.
const fn reduce_cost(limbs: u64) -> Cost {
    cost::gcd(limbs, limbs)
        .saturating_add(cost::div(limbs, limbs))
        .saturating_add(cost::div(limbs, limbs))
}

impl Default for Rational {
    fn default() -> Self {
        Self::ZERO
    }
}

impl TryFrom<&Rational> for BoundedRational {
    type Error = ExactError;
    fn try_from(value: &Rational) -> Result<Self, ExactError> {
        value.to_bounded()
    }
}

impl FromStr for Rational {
    type Err = ExactError;
    /// Parse the `owl:rational` lexical form (OWL 2 Structural Specification
    /// §4.1): an integer, `/`, and an unsigned integer, with no whitespace, of any
    /// length (`"1/3"`, `"-22/7"`, `"+4/6"`).
    ///
    /// # Errors
    ///
    /// [`ExactError::InvalidLexical`] for any other text;
    /// [`ExactError::DivisionByZero`] for a zero denominator.
    fn from_str(lexical: &str) -> Result<Self, ExactError> {
        let invalid = || {
            ExactError::invalid(
                lexical,
                ExactKind::Rational,
                "expected an integer, '/', then an unsigned integer",
            )
        };
        let (numerator, denominator) = lexical.split_once('/').ok_or_else(invalid)?;
        if denominator.is_empty() || !denominator.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        let numerator = Integer::from_str(numerator).map_err(|_| invalid())?;
        let denominator = Integer::from_str(denominator).map_err(|_| invalid())?;
        Self::new(numerator, denominator)
    }
}

impl fmt::Display for Rational {
    /// The canonical `owl:rational` lexical form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rational({self})")
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        let (sa, sb) = (self.numerator.signum(), other.numerator.signum());
        if sa != sb {
            return sa.cmp(&sb);
        }
        if self.denominator == other.denominator {
            return self.numerator.cmp(&other.numerator);
        }
        // Both denominators are positive, so cross-multiplying keeps the order.
        (&self.numerator * &other.denominator).cmp(&(&other.numerator * &self.denominator))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<&Rational> for &Rational {
    type Output = Rational;
    fn add(self, rhs: &Rational) -> Rational {
        if self.denominator == rhs.denominator {
            return Rational::reduced(&self.numerator + &rhs.numerator, self.denominator.clone());
        }
        Rational::reduced(
            &(&self.numerator * &rhs.denominator) + &(&rhs.numerator * &self.denominator),
            &self.denominator * &rhs.denominator,
        )
    }
}

impl Sub<&Rational> for &Rational {
    type Output = Rational;
    fn sub(self, rhs: &Rational) -> Rational {
        self + &(-rhs)
    }
}

impl Mul<&Rational> for &Rational {
    type Output = Rational;
    fn mul(self, rhs: &Rational) -> Rational {
        Rational::reduced(
            &self.numerator * &rhs.numerator,
            &self.denominator * &rhs.denominator,
        )
    }
}

impl Neg for &Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        Rational {
            numerator: -&self.numerator,
            denominator: self.denominator.clone(),
        }
    }
}

impl Neg for Rational {
    type Output = Self;
    fn neg(self) -> Self {
        -&self
    }
}

forward_by_value!(Rational: Add add, Sub sub, Mul mul);
