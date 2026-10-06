// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One interval-operation interface for binary64 and exact fixed-point solvers.

use super::{
    CheckedBinary64, CoordinateMath, FixedInterval, FloatInterval, MathError, Word, WordInterval,
};
use crate::bigint::BigInt;
use core::cmp::Ordering;

/// Scoped arithmetic capability for a single deterministic solver invocation.
/// A floating scope is released before an external observer or suspension.
#[derive(Debug)]
pub struct IntervalContext<'a> {
    math: &'a mut CoordinateMath,
    chunk: Option<CheckedBinary64>,
    floating: bool,
}
impl<'a> IntervalContext<'a> {
    /// Integer-only context, independent of floating control state.
    pub const fn fixed(math: &'a mut CoordinateMath) -> Self {
        Self {
            math,
            chunk: None,
            floating: false,
        }
    }
    /// Validated binary64 context using already prepared original coefficients.
    ///
    /// # Errors
    /// Refuses an unsupported environment or numerical resource exhaustion.
    pub fn binary64(math: &'a mut CoordinateMath) -> Result<Self, MathError> {
        let chunk = math.enter_chunk()?;
        Ok(Self {
            math,
            chunk: Some(chunk),
            floating: true,
        })
    }
    /// Access the integer work, cancellation and memory admission capability.
    pub const fn math(&mut self) -> &mut CoordinateMath {
        self.math
    }
    /// The correctly rounded binary64 operations of the live validated chunk,
    /// or `None` for an integer-only context or a paused floating context.
    #[must_use]
    pub fn binary64_ops(&self) -> Option<crate::ieee::Binary64<'_>> {
        self.chunk.as_ref().map(CheckedBinary64::ops)
    }
    /// Release floating control state and borrow numerical accounting.
    /// The caller may invoke an external observer only after this operation.
    pub fn pause(&mut self) -> &mut CoordinateMath {
        self.chunk.take();
        self.math
    }
    /// Revalidate the environment after an external observer, if floating.
    ///
    /// # Errors
    /// Refuses unsupported control state or numerical exhaustion.
    pub fn resume(&mut self) -> Result<(), MathError> {
        if self.floating && self.chunk.is_none() {
            self.chunk = Some(self.math.enter_chunk()?);
        }
        Ok(())
    }
    fn checked(&self) -> Result<&CheckedBinary64, MathError> {
        self.chunk.as_ref().ok_or(MathError::Domain(
            "binary64 operation outside validated chunk",
        ))
    }
}

impl core::ops::Deref for IntervalContext<'_> {
    type Target = CoordinateMath;
    fn deref(&self) -> &Self::Target {
        self.math
    }
}
impl core::ops::DerefMut for IntervalContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.math
    }
}

/// Ordered exact endpoint capabilities shared by an interval formula.
pub trait IntervalBound: Clone + Ord {
    /// Whether the endpoint is exactly zero.
    fn is_zero(&self) -> bool;
    /// Whether the endpoint is strictly negative.
    fn is_negative(&self) -> bool;
    /// Exact additive inverse.
    #[must_use]
    fn negated(&self) -> Self;
    /// Conservative representation size used for numerical admission.
    fn bits_upper_bound(&self) -> usize;
}
impl IntervalBound for BigInt {
    fn is_zero(&self) -> bool {
        Self::is_zero(self)
    }
    fn is_negative(&self) -> bool {
        Self::is_negative(self)
    }
    fn negated(&self) -> Self {
        Self::negated(self)
    }
    fn bits_upper_bound(&self) -> usize {
        Self::bits_upper_bound(self)
    }
}

/// A finite binary64 bound; its order identifies both spellings of zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatBound(f64);
impl Eq for FloatBound {}
impl Ord for FloatBound {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).expect("finite bounds")
    }
}
impl PartialOrd for FloatBound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl IntervalBound for FloatBound {
    fn is_zero(&self) -> bool {
        self.0.to_bits() << 1 == 0
    }
    fn is_negative(&self) -> bool {
        self.0 < 0.0
    }
    fn negated(&self) -> Self {
        Self(f64::from_bits(self.0.to_bits() ^ (1 << 63)))
    }
    fn bits_upper_bound(&self) -> usize {
        64
    }
}

/// Finite controlled binary64 enclosure with typed ordered endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatEnclosure {
    lower: FloatBound,
    upper: FloatBound,
}
impl FloatEnclosure {
    /// Enclose a fixed-point interval through the one exact directed IEEE
    /// conversion home. This preparation uses integer arithmetic only.
    ///
    /// # Errors
    /// Refuses an unrepresentable finite endpoint or exhausted admission.
    pub fn from_fixed(value: &FixedInterval, math: &mut CoordinateMath) -> Result<Self, MathError> {
        Ok(Self::from_float(value.to_binary64(math)?))
    }

    /// The enclosure of two exact finite binary64 endpoints.
    ///
    /// # Errors
    /// Refuses a nonfinite or reversed pair.
    pub fn from_binary64_bounds(lower: f64, upper: f64) -> Result<Self, MathError> {
        Ok(Self::from_float(FloatInterval::from_bounds(lower, upper)?))
    }

    /// The exact inclusive endpoints.
    #[must_use]
    pub const fn bounds(&self) -> (f64, f64) {
        (self.lower.0, self.upper.0)
    }

    fn from_float(value: FloatInterval) -> Self {
        Self {
            lower: FloatBound(value.lower()),
            upper: FloatBound(value.upper()),
        }
    }
    fn interval(self) -> FloatInterval {
        FloatInterval::from_bounds(self.lower.0, self.upper.0).expect("validated endpoints")
    }
}

/// Complete certified operations used by mathematical solver formulas.
/// Implementations may tighten proofs, but never substitute an approximate value
/// for an enclosure. Every operation consumes explicit admission.
#[allow(clippy::missing_errors_doc)] // The trait has one error contract: certified domain and resource refusal.
pub trait CertifiedInterval: Clone {
    /// Exact ordered endpoint representation.
    type Bound: IntervalBound;
    /// Inclusive lower bound.
    fn lower(&self) -> &Self::Bound;
    /// Inclusive upper bound.
    fn upper(&self) -> &Self::Bound;
    /// Build a valid inclusive enclosure from complete ordered bounds.
    fn from_bounds(
        lower: Self::Bound,
        upper: Self::Bound,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError>;
    /// Enclose an exact signed ratio.
    fn from_ratio(
        numerator: &BigInt,
        denominator: &BigInt,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError>;
    /// Enclose an exact machine integer.
    fn from_i64(value: i64, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::from_ratio(
            &BigInt::from_i128(i128::from(value)),
            &BigInt::from_i128(1),
            ctx,
        )
    }
    /// Enclose the exact signed power of two without an unbounded temporary.
    /// Operand growth is admitted before constructing the reusable integer.
    fn power_of_two(exponent: i32, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        let magnitude = exponent.unsigned_abs();
        ctx.admit_exact(1, magnitude as usize + 1)?;
        let one = BigInt::from_i128(1);
        let factor = ctx.integer_pow2(&one, magnitude)?;
        if exponent < 0 {
            Self::from_ratio(&one, &factor, ctx)
        } else {
            Self::from_ratio(&factor, &one, ctx)
        }
    }
    /// Independently generated bounded Machin pi.
    fn pi(ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward sum.
    fn add(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward difference.
    fn sub(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward product.
    fn mul(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward quotient, refusing unresolved zero crossings.
    fn div(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward square.
    fn square(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Exact negation.
    fn neg(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward absolute value.
    fn abs(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward nonnegative square root.
    fn sqrt(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Outward sine and cosine using original generated coefficients and analytic tails.
    fn sin_cos(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError>;
    /// Whole-interval sine and cosine; wider admitted ranges use analytic derivative bounds.
    fn sin_cos_range(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError> {
        self.sin_cos(ctx)
    }
    /// Outward principal angle; unresolved origin and branch cuts refuse.
    fn atan2(y: &Self, x: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError>;
    /// Exact half-even decimal rounding of both complete endpoints.
    fn round_decimal(
        &self,
        places: u32,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<(BigInt, BigInt), MathError>;
    /// Enclose these exact endpoints on the caller's fixed grid.
    fn to_fixed(&self, ctx: &mut IntervalContext<'_>) -> Result<FixedInterval, MathError>;
    /// Outward width, including the endpoint subtraction's arithmetic enclosure.
    fn width(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        let lower = Self::from_bounds(self.lower().clone(), self.lower().clone(), ctx)?;
        let upper = Self::from_bounds(self.upper().clone(), self.upper().clone(), ctx)?;
        upper.sub(&lower, ctx)
    }
}

impl CertifiedInterval for FixedInterval {
    type Bound = BigInt;
    fn lower(&self) -> &BigInt {
        Self::lower(self)
    }
    fn upper(&self) -> &BigInt {
        Self::upper(self)
    }
    fn from_bounds(
        lower: BigInt,
        upper: BigInt,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        Self::from_bounds(lower, upper, ctx.math)
    }
    fn from_ratio(
        n: &BigInt,
        d: &BigInt,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        Self::from_ratio(n, d, ctx.math)
    }
    fn pi(ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::pi(ctx.math)
    }
    fn add(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::add(self, rhs, ctx.math)
    }
    fn sub(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::sub(self, rhs, ctx.math)
    }
    fn mul(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::mul(self, rhs, ctx.math)
    }
    fn div(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::div(self, rhs, ctx.math)
    }
    fn square(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::square(self, ctx.math)
    }
    fn neg(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::neg(self, ctx.math)
    }
    fn abs(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::abs(self, ctx.math)
    }
    fn sqrt(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::sqrt(self, ctx.math)
    }
    fn sin_cos(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError> {
        Self::sin_cos(self, ctx.math)
    }
    fn sin_cos_range(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError> {
        Self::sin_cos_range(self, ctx.math)
    }
    fn atan2(y: &Self, x: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Self::atan2(y, x, ctx.math)
    }
    fn round_decimal(
        &self,
        places: u32,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<(BigInt, BigInt), MathError> {
        Self::round_decimal(self, places, ctx.math)
    }
    fn to_fixed(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(
            1,
            self.lower()
                .bits_upper_bound()
                .max(self.upper().bits_upper_bound()),
        )?;
        Ok(self.clone())
    }
}

impl CertifiedInterval for FloatEnclosure {
    type Bound = FloatBound;
    fn lower(&self) -> &FloatBound {
        &self.lower
    }
    fn upper(&self) -> &FloatBound {
        &self.upper
    }
    fn from_bounds(
        lower: FloatBound,
        upper: FloatBound,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(FloatInterval::from_bounds(
            lower.0, upper.0,
        )?))
    }
    fn from_ratio(
        n: &BigInt,
        d: &BigInt,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        Ok(Self::from_float(FloatInterval::from_ratio(n, d, ctx.math)?))
    }
    fn from_i64(value: i64, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        // Every integer of magnitude at most 2^31 is an exact binary64 value;
        // the general exact-ratio conversion encloses the remainder.
        match i32::try_from(value) {
            Ok(small) => {
                ctx.math.charge(1, 64)?;
                Ok(Self::from_float(FloatInterval::point(f64::from(small))?))
            }
            Err(_) => Self::from_ratio(
                &BigInt::from_i128(i128::from(value)),
                &BigInt::from_i128(1),
                ctx,
            ),
        }
    }
    fn power_of_two(exponent: i32, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        // Normal binary64 powers of two are exact bit patterns.
        if (-1022..=1023).contains(&exponent) {
            ctx.math.charge(1, 64)?;
            let bits = u64::try_from(exponent + 1023).expect("normal biased exponent") << 52;
            return Ok(Self::from_float(FloatInterval::point(f64::from_bits(
                bits,
            ))?));
        }
        let magnitude = exponent.unsigned_abs();
        ctx.admit_exact(1, magnitude as usize + 1)?;
        let one = BigInt::from_i128(1);
        let factor = ctx.integer_pow2(&one, magnitude)?;
        if exponent < 0 {
            Self::from_ratio(&one, &factor, ctx)
        } else {
            Self::from_ratio(&factor, &one, ctx)
        }
    }
    fn pi(ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        Ok(Self::from_float(ctx.math.binary64_pi()?))
    }
    fn add(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(
            self.interval().add(rhs.interval(), ctx.checked()?)?,
        ))
    }
    fn sub(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(
            self.interval().sub(rhs.interval(), ctx.checked()?)?,
        ))
    }
    fn mul(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(
            self.interval().mul(rhs.interval(), ctx.checked()?)?,
        ))
    }
    fn div(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(
            self.interval().div(rhs.interval(), ctx.checked()?)?,
        ))
    }
    fn square(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(self.interval().square(ctx.checked()?)?))
    }
    fn neg(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(self.interval().neg()))
    }
    fn abs(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(self.interval().abs()))
    }
    fn sqrt(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 64)?;
        Ok(Self::from_float(self.interval().sqrt(ctx.checked()?)?))
    }
    fn sin_cos(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError> {
        let chunk = ctx.chunk.as_ref().ok_or(MathError::Domain(
            "binary64 operation outside validated chunk",
        ))?;
        let (sin, cos) = self.interval().sin_cos(ctx.math, chunk)?;
        Ok((Self::from_float(sin), Self::from_float(cos)))
    }
    fn atan2(y: &Self, x: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        let chunk = ctx.chunk.as_ref().ok_or(MathError::Domain(
            "binary64 operation outside validated chunk",
        ))?;
        Ok(Self::from_float(FloatInterval::atan2(
            y.interval(),
            x.interval(),
            ctx.math,
            chunk,
        )?))
    }
    fn round_decimal(
        &self,
        places: u32,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<(BigInt, BigInt), MathError> {
        let chunk = ctx.chunk.as_ref().ok_or(MathError::Domain(
            "binary64 operation outside validated chunk",
        ))?;
        self.interval().round_decimal(places, ctx.math, chunk)
    }
    fn to_fixed(&self, ctx: &mut IntervalContext<'_>) -> Result<FixedInterval, MathError> {
        let lower = FixedInterval::from_binary64(self.lower.0, ctx.math)?;
        let upper = FixedInterval::from_binary64(self.upper.0, ctx.math)?;
        FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), ctx.math)
    }
}

impl IntervalBound for Word {
    fn is_zero(&self) -> bool {
        Self::is_zero(*self)
    }
    fn is_negative(&self) -> bool {
        Self::is_negative(*self)
    }
    fn negated(&self) -> Self {
        -*self
    }
    fn bits_upper_bound(&self) -> usize {
        128
    }
}

impl CertifiedInterval for WordInterval {
    type Bound = Word;
    fn lower(&self) -> &Word {
        Self::lower(self)
    }
    fn upper(&self) -> &Word {
        Self::upper(self)
    }
    fn from_bounds(
        lower: Word,
        upper: Word,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::from_bounds(lower, upper)
    }
    fn from_ratio(
        n: &BigInt,
        d: &BigInt,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        ctx.math.charge(
            4,
            n.bits_upper_bound()
                .saturating_add(d.bits_upper_bound())
                .saturating_add(1074),
        )?;
        Self::from_ratio(n.as_integer(), d.as_integer())
    }
    fn from_i64(value: i64, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        match i32::try_from(value) {
            Ok(small) => {
                ctx.math.charge(1, 128)?;
                let value = f64::from(small);
                Self::from_binary64(value, value)
            }
            Err(_) => <Self as CertifiedInterval>::from_ratio(
                &BigInt::from_i128(i128::from(value)),
                &BigInt::from_i128(1),
                ctx,
            ),
        }
    }
    fn power_of_two(exponent: i32, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        if (-400..=400).contains(&exponent) {
            ctx.math.charge(1, 128)?;
            let bits = u64::try_from(exponent + 1023).expect("normal biased exponent") << 52;
            let value = f64::from_bits(bits);
            return Self::from_binary64(value, value);
        }
        Err(MathError::PrecisionExhausted)
    }
    fn pi(ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::pi()
    }
    fn add(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::add(*self, *rhs, ctx.checked()?.ops())
    }
    fn sub(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::sub(*self, *rhs, ctx.checked()?.ops())
    }
    fn mul(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::mul(*self, *rhs, ctx.checked()?.ops())
    }
    fn div(&self, rhs: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::div(*self, *rhs, ctx.checked()?.ops())
    }
    fn square(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::square(*self, ctx.checked()?.ops())
    }
    fn neg(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Ok(-*self)
    }
    fn abs(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Ok(Self::abs(*self))
    }
    fn sqrt(&self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(1, 128)?;
        Self::sqrt(*self, ctx.checked()?.ops())
    }
    fn sin_cos(&self, ctx: &mut IntervalContext<'_>) -> Result<(Self, Self), MathError> {
        ctx.math.charge(64, 128)?;
        Self::sin_cos(*self, ctx.checked()?.ops())
    }
    fn atan2(y: &Self, x: &Self, ctx: &mut IntervalContext<'_>) -> Result<Self, MathError> {
        ctx.math.charge(128, 128)?;
        Self::atan2(*y, *x, ctx.checked()?.ops())
    }
    fn round_decimal(
        &self,
        places: u32,
        ctx: &mut IntervalContext<'_>,
    ) -> Result<(BigInt, BigInt), MathError> {
        Ok((
            round_word(*Self::lower(self), places, ctx.math)?,
            round_word(*Self::upper(self), places, ctx.math)?,
        ))
    }
    fn to_fixed(&self, ctx: &mut IntervalContext<'_>) -> Result<FixedInterval, MathError> {
        let enclose = |word: Word, ctx: &mut CoordinateMath| {
            let hi = FixedInterval::from_binary64(word.hi(), ctx)?;
            let lo = FixedInterval::from_binary64(word.lo(), ctx)?;
            hi.add(&lo, ctx)
        };
        let lower = enclose(*Self::lower(self), ctx.math)?;
        let upper = enclose(*Self::upper(self), ctx.math)?;
        FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), ctx.math)
    }
}

impl WordInterval {
    /// Enclose a fixed-point interval through its exact dyadic endpoints.
    ///
    /// # Errors
    /// Refuses an endpoint outside the checked double-word range.
    pub fn from_fixed(value: &FixedInterval, math: &mut CoordinateMath) -> Result<Self, MathError> {
        let bits = value.precision_bits();
        math.charge(
            4,
            value
                .lower()
                .bits_upper_bound()
                .max(value.upper().bits_upper_bound())
                .saturating_add(bits as usize),
        )?;
        let scale = BigInt::from_i128(1).mul_pow2(bits);
        let lower = Self::from_ratio(value.lower().as_integer(), scale.as_integer())?;
        let upper = Self::from_ratio(value.upper().as_integer(), scale.as_integer())?;
        Self::from_bounds(*lower.lower(), *upper.upper())
    }
}

/// Half-even decimal mantissa of one exact double-word value.
fn round_word(value: Word, places: u32, math: &mut CoordinateMath) -> Result<BigInt, MathError> {
    use crate::ieee::dyadic::Binary64Dyadic;
    let parts = [value.hi(), value.lo()].map(Binary64Dyadic::decode);
    let [Some(hi), Some(lo)] = parts else {
        return Err(MathError::Binary64Range);
    };
    let minimum = hi.exponent().min(lo.exponent()).min(0);
    math.charge(
        2,
        128_usize
            .saturating_add(minimum.unsigned_abs() as usize)
            .saturating_add((places as usize).saturating_mul(4)),
    )?;
    let mut numerator = BigInt::from_i128(0);
    for part in [hi, lo] {
        let mut term = BigInt::from_i128(i128::from(part.significand()))
            .mul_pow2((part.exponent() - minimum).unsigned_abs());
        if part.negative() {
            term = term.negated();
        }
        numerator = numerator.add(&term);
    }
    let numerator = numerator.mul_pow10(places);
    let denominator = BigInt::from_i128(1).mul_pow2(minimum.unsigned_abs());
    super::fixed::nearest_even(&numerator, &denominator, math)
}
