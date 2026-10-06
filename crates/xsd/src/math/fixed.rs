// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Outward exact dyadic-grid arithmetic. Both endpoints include their boundary.

mod range;
mod transcend;

mod sum;
pub use sum::FixedIntervalSum;

use super::{CoordinateMath, FloatInterval, MathError};
use crate::BigInt;
use crate::ieee::dyadic::Binary64Dyadic;

/// An inclusive enclosure `[lower, upper] × 2^-precision_bits`.
/// Endpoints are exact integers; every arithmetic division rounds outward.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixedInterval {
    lower: BigInt,
    upper: BigInt,
    precision_bits: u32,
}

impl FixedInterval {
    /// Construct from exact scaled endpoint integers at the context's precision.
    ///
    /// # Errors
    /// Refuses reversed endpoints or inadmissible work/workspace.
    pub fn from_bounds(
        lower: BigInt,
        upper: BigInt,
        ctx: &mut CoordinateMath,
    ) -> Result<Self, MathError> {
        ctx.charge(
            1,
            lower
                .bits_upper_bound()
                .max(upper.bits_upper_bound())
                .max(ctx.limits.precision_bits as usize),
        )?;
        if lower > upper {
            return Err(MathError::Domain("reversed interval endpoints"));
        }
        let (lower, upper) = if let Some(scratch) = ctx.limb_scratch() {
            let lower = if lower.has_unbounded_limb_storage() {
                lower.copy_in(scratch).map_err(MathError::LimbScratch)?
            } else {
                lower
            };
            let upper = if upper.has_unbounded_limb_storage() {
                upper.copy_in(scratch).map_err(MathError::LimbScratch)?
            } else {
                upper
            };
            (lower, upper)
        } else {
            (lower, upper)
        };
        Ok(Self {
            lower,
            upper,
            precision_bits: ctx.limits.precision_bits,
        })
    }

    /// Enclose an exact rational number. The denominator may have either sign.
    ///
    /// # Errors
    /// Refuses a zero denominator or exhausted numerical admission.
    pub fn from_ratio(
        numerator: &BigInt,
        denominator: &BigInt,
        ctx: &mut CoordinateMath,
    ) -> Result<Self, MathError> {
        ctx.charge(
            1,
            numerator
                .bits_upper_bound()
                .saturating_add(ctx.limits.precision_bits as usize)
                .max(denominator.bits_upper_bound()),
        )?;
        let (lower, upper) = product_bounds(numerator, &ctx.scale, denominator, ctx)?;
        Self::from_bounds(lower, upper, ctx)
    }

    /// An exact signed integer on the dyadic grid.
    ///
    /// # Errors
    /// Refuses exhausted numerical admission.
    pub fn from_i64(value: i64, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        <Self as super::CertifiedInterval>::from_i64(value, &mut super::IntervalContext::fixed(ctx))
    }

    /// Enclose an exact power of two, admitting its complete integer growth
    /// before constructing a reusable dyadic numerator or denominator.
    /// # Errors
    /// Refuses precision, work, memory or reusable destination exhaustion.
    pub fn power_of_two(exponent: i32, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        <Self as super::CertifiedInterval>::power_of_two(
            exponent,
            &mut super::IntervalContext::fixed(ctx),
        )
    }

    /// Enclose the exact IEEE-bit value, without decimal conversion.
    ///
    /// # Errors
    /// Refuses NaN/infinity or exhausted numerical admission.
    pub fn from_binary64(value: f64, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        let value = Binary64Dyadic::decode(value).ok_or(MathError::Domain("nonfinite binary64"))?;
        let mut numerator = BigInt::from_i128(i128::from(value.significand()));
        if value.negative() {
            numerator = numerator.negated();
        }
        let exponent = value.exponent();
        ctx.charge(
            1,
            numerator
                .bits_upper_bound()
                .saturating_add(exponent.unsigned_abs() as usize)
                .saturating_add(ctx.limits.precision_bits as usize),
        )?;
        if exponent >= 0 {
            Self::from_ratio(
                &ctx.integer_pow2(&numerator, exponent.unsigned_abs())?,
                &BigInt::from_i128(1),
                ctx,
            )
        } else {
            Self::from_ratio(
                &numerator,
                &ctx.integer_pow2(&BigInt::from_i128(1), exponent.unsigned_abs())?,
                ctx,
            )
        }
    }

    /// Inclusive lower endpoint in units of `2^-precision_bits`.
    #[must_use]
    pub const fn lower(&self) -> &BigInt {
        &self.lower
    }
    /// Inclusive upper endpoint in units of `2^-precision_bits`.
    #[must_use]
    pub const fn upper(&self) -> &BigInt {
        &self.upper
    }
    /// The grid's fractional bit count.
    #[must_use]
    pub const fn precision_bits(&self) -> u32 {
        self.precision_bits
    }

    /// Preserve both exact endpoints while detaching reusable limb destinations
    /// for an immutable preparation. The caller admits the returned owned bytes.
    #[must_use]
    pub fn detached(&self) -> Self {
        Self {
            lower: self.lower.detached(),
            upper: self.upper.detached(),
            precision_bits: self.precision_bits,
        }
    }

    /// Heap bytes retained by that detached endpoint copy.
    #[must_use]
    pub fn detached_heap_bytes(&self) -> usize {
        self.lower
            .detached_heap_bytes()
            .saturating_add(self.upper.detached_heap_bytes())
    }

    /// Whether both inclusive endpoints are the exact numerical zero.
    #[must_use]
    pub fn is_exact_zero(&self) -> bool {
        self.lower.is_zero() && self.upper.is_zero()
    }

    /// Current owned bytes, including the inline value and both endpoint buffers.
    #[must_use]
    pub fn workspace_bytes(&self) -> usize {
        size_of::<Self>()
            .saturating_add(self.lower.allocated_bytes())
            .saturating_add(self.upper.allocated_bytes())
    }

    /// Exact endpoint separation as a point interval on the same dyadic grid.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn width(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        let width = ctx.integer_sub(&self.upper, &self.lower)?;
        Self::from_bounds(width.clone(), width, ctx)
    }

    /// A central point on the exact dyadic grid, with the endpoint sum halved
    /// toward zero. The selected point lies inside this closed interval.
    ///
    /// # Errors
    /// Refuses mismatched precision or exhausted numerical admission before
    /// computing the endpoint sum and quotient.
    pub fn midpoint(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        let centre = ctx
            .integer_div_rem(
                &ctx.integer_add(&self.lower, &self.upper)?,
                &BigInt::from_i128(2),
            )?
            .ok_or(MathError::Domain("zero midpoint divisor"))?
            .0;
        Self::from_bounds(centre.clone(), centre, ctx)
    }

    /// Exact inclusive intersection of two enclosures on the same grid.
    /// A proved disjoint pair returns `None`; uncertainty is never discarded.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn intersection(
        &self,
        rhs: &Self,
        ctx: &mut CoordinateMath,
    ) -> Result<Option<Self>, MathError> {
        self.admit(Some(rhs), ctx, 1)?;
        let lower = self.lower.clone().max(rhs.lower.clone());
        let upper = self.upper.clone().min(rhs.upper.clone());
        if lower > upper {
            Ok(None)
        } else {
            Self::from_bounds(lower, upper, ctx).map(Some)
        }
    }

    /// Smallest inclusive dyadic interval containing both input enclosures.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn hull(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 1)?;
        Self::from_bounds(
            self.lower.clone().min(rhs.lower.clone()),
            self.upper.clone().max(rhs.upper.clone()),
            ctx,
        )
    }

    /// Enclose the pointwise minimum of two inclusive intervals.
    ///
    /// Both selected endpoints are copied into admitted reusable destinations.
    /// # Errors
    /// Refuses precision mismatch or exhausted arithmetic/destination admission.
    pub fn minimum(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.extremum(rhs, false, ctx)
    }

    /// Enclose the pointwise maximum of two inclusive intervals.
    ///
    /// # Errors
    /// Preserves the complete admitted endpoint-selection contract of `minimum`.
    pub fn maximum(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.extremum(rhs, true, ctx)
    }

    fn extremum(
        &self,
        rhs: &Self,
        maximum: bool,
        ctx: &mut CoordinateMath,
    ) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 1)?;
        let width = self
            .lower
            .as_integer()
            .bit_len()
            .max(self.upper.as_integer().bit_len())
            .max(rhs.lower.as_integer().bit_len())
            .max(rhs.upper.as_integer().bit_len());
        ctx.admit_exact_cost(
            crate::integer::ExactArithmeticCost::for_operation(
                crate::integer::ExactOperation::Linear,
                width,
                6,
            )
            .ok_or(MathError::WorkExhausted)?,
        )?;
        let (lower, upper) = if maximum {
            ((&self.lower).max(&rhs.lower), (&self.upper).max(&rhs.upper))
        } else {
            ((&self.lower).min(&rhs.lower), (&self.upper).min(&rhs.upper))
        };
        Self::from_bounds(ctx.integer_copy(lower)?, ctx.integer_copy(upper)?, ctx)
    }

    /// Outward sum at the same precision.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn add(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 1)?;
        Self::from_bounds(
            ctx.integer_add(&self.lower, &rhs.lower)?,
            ctx.integer_add(&self.upper, &rhs.upper)?,
            ctx,
        )
    }

    /// Outward difference at the same precision.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn sub(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 1)?;
        Self::from_bounds(
            ctx.integer_sub(&self.lower, &rhs.upper)?,
            ctx.integer_sub(&self.upper, &rhs.lower)?,
            ctx,
        )
    }

    /// Outward product with exact extremal endpoint selection.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn mul(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 2)?;
        // Monotonicity selects the exact extremal corners when at least one
        // operand has a proved sign. Only two zero-crossing operands need all
        // four corners. Directed quantization is monotone as well.
        let zero = BigInt::zero();
        let corners = if self.lower >= zero {
            if rhs.lower >= zero {
                Some((&self.lower, &rhs.lower, &self.upper, &rhs.upper))
            } else if rhs.upper <= zero {
                Some((&self.upper, &rhs.lower, &self.lower, &rhs.upper))
            } else {
                Some((&self.upper, &rhs.lower, &self.upper, &rhs.upper))
            }
        } else if self.upper <= zero {
            if rhs.lower >= zero {
                Some((&self.lower, &rhs.upper, &self.upper, &rhs.lower))
            } else if rhs.upper <= zero {
                Some((&self.upper, &rhs.upper, &self.lower, &rhs.lower))
            } else {
                Some((&self.lower, &rhs.upper, &self.lower, &rhs.lower))
            }
        } else if rhs.lower >= zero {
            Some((&self.lower, &rhs.upper, &self.upper, &rhs.upper))
        } else if rhs.upper <= zero {
            Some((&self.upper, &rhs.lower, &self.lower, &rhs.lower))
        } else {
            None
        };
        if let Some((lo_a, lo_b, hi_a, hi_b)) = corners {
            let lower = product_bounds(lo_a, lo_b, &ctx.scale, ctx)?.0;
            let upper = product_bounds(hi_a, hi_b, &ctx.scale, ctx)?.1;
            return Self::from_bounds(lower, upper, ctx);
        }
        let products = [
            product_bounds(&self.lower, &rhs.lower, &ctx.scale, ctx)?,
            product_bounds(&self.lower, &rhs.upper, &ctx.scale, ctx)?,
            product_bounds(&self.upper, &rhs.lower, &ctx.scale, ctx)?,
            product_bounds(&self.upper, &rhs.upper, &ctx.scale, ctx)?,
        ];
        let lower = products
            .iter()
            .map(|(lower, _)| lower)
            .min()
            .ok_or(MathError::PrecisionExhausted)?
            .clone();
        let upper = products
            .iter()
            .map(|(_, upper)| upper)
            .max()
            .ok_or(MathError::PrecisionExhausted)?
            .clone();
        Self::from_bounds(lower, upper, ctx)
    }

    /// Outward quotient. A denominator reaching zero needs a tighter enclosure.
    ///
    /// # Errors
    /// Refuses exact division by zero, an unresolved zero crossing, mismatched
    /// precision, or exhausted numerical admission.
    pub fn div(&self, rhs: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(Some(rhs), ctx, 2)?;
        if rhs.lower <= BigInt::zero() && rhs.upper >= BigInt::zero() {
            return Err(if rhs.lower.is_zero() && rhs.upper.is_zero() {
                MathError::Domain("division by zero")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let mut lower: Option<BigInt> = None;
        let mut upper: Option<BigInt> = None;
        for numerator in [&self.lower, &self.upper] {
            for denominator in [&rhs.lower, &rhs.upper] {
                let (lo, hi) = product_bounds(numerator, &ctx.scale, denominator, ctx)?;
                lower = Some(match lower {
                    Some(old) => old.min(lo),
                    None => lo,
                });
                upper = Some(match upper {
                    Some(old) => old.max(hi),
                    None => hi,
                });
            }
        }
        Self::from_bounds(
            lower.ok_or(MathError::PrecisionExhausted)?,
            upper.ok_or(MathError::PrecisionExhausted)?,
            ctx,
        )
    }

    /// A square enclosure that retains the operand's correlation across zero.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn square(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 2)?;
        let a = product_bounds(&self.lower, &self.lower, &ctx.scale, ctx)?;
        let b = product_bounds(&self.upper, &self.upper, &ctx.scale, ctx)?;
        let lower = if self.contains_zero() {
            BigInt::zero()
        } else {
            a.0.min(b.0)
        };
        Self::from_bounds(lower, a.1.max(b.1), ctx)
    }

    /// Exact negation with exchanged endpoints.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn neg(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        Self::from_bounds(self.upper.negated(), self.lower.negated(), ctx)
    }

    /// Outward absolute value, including an exact zero when the input crosses it.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn abs(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        let a = self.lower.abs();
        let b = self.upper.abs();
        let lower = if self.contains_zero() {
            BigInt::zero()
        } else {
            a.clone().min(b.clone())
        };
        Self::from_bounds(lower, a.max(b), ctx)
    }

    /// Outward nonnegative square root using exact integer floor roots.
    ///
    /// # Errors
    /// Refuses a negative input, an unresolved domain boundary, a precision
    /// mismatch, or exhausted numerical admission.
    pub fn sqrt(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 2)?;
        if self.lower.is_negative() {
            return Err(if self.upper.is_negative() {
                MathError::Domain("negative square root")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let lower = ctx
            .integer_sqrt_product(&self.lower, &ctx.scale)?
            .ok_or(MathError::Domain("negative square root"))?;
        let mut upper = ctx
            .integer_sqrt_product(&self.upper, &ctx.scale)?
            .ok_or(MathError::Domain("negative square root"))?;
        if ctx.compare_products(&upper, &upper, &self.upper, &ctx.scale)?
            == core::cmp::Ordering::Less
        {
            upper = ctx.integer_add(&upper, &BigInt::from_i128(1))?;
        }
        Self::from_bounds(lower, upper, ctx)
    }

    /// Exact integer floors of both enclosure endpoints.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted arithmetic admission.
    pub fn floor_bounds(&self, ctx: &mut CoordinateMath) -> Result<(BigInt, BigInt), MathError> {
        self.admit(None, ctx, 1)?;
        Ok((
            directed(&self.lower, &ctx.scale, false, ctx)?,
            directed(&self.upper, &ctx.scale, false, ctx)?,
        ))
    }

    /// Exact integer ceilings of both enclosure endpoints.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted arithmetic admission.
    pub fn ceil_bounds(&self, ctx: &mut CoordinateMath) -> Result<(BigInt, BigInt), MathError> {
        self.admit(None, ctx, 1)?;
        Ok((
            directed(&self.lower, &ctx.scale, true, ctx)?,
            directed(&self.upper, &ctx.scale, true, ctx)?,
        ))
    }

    /// Half-even rounding of both enclosure endpoints to `places` decimal places.
    /// Matching integers certify one completed rounded result. Different integers
    /// require refinement; they are never silently collapsed.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn round_decimal(
        &self,
        places: u32,
        ctx: &mut CoordinateMath,
    ) -> Result<(BigInt, BigInt), MathError> {
        self.round_decimal_with(places, crate::ieee::ratio::Rounding::NearestEven, ctx)
    }

    /// Round both exact enclosure endpoints to the selected decimal grid.
    /// Matching integers prove one directed or nearest-even result. Different
    /// integers require refinement; the output grid never follows proof width.
    /// The original fused integer-product quotient decides every boundary and
    /// tie without constructing or normalizing a rational endpoint.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical/scratch admission.
    pub fn round_decimal_with(
        &self,
        places: u32,
        rounding: crate::ieee::ratio::Rounding,
        ctx: &mut CoordinateMath,
    ) -> Result<(BigInt, BigInt), MathError> {
        self.admit(None, ctx, 1)?;
        ctx.charge(
            1,
            self.endpoint_bits()
                .saturating_add((places as usize).saturating_mul(4)),
        )?;
        let factor = ctx.integer_pow10(&BigInt::from_i128(1), places)?;
        let round = |endpoint: &BigInt| {
            match ctx.limb_scratch() {
                Some(scratch) => crate::ieee::ratio::round_integer_product_in(
                    endpoint.as_integer(),
                    factor.as_integer(),
                    ctx.scale.as_integer(),
                    rounding,
                    scratch,
                )
                .map_err(MathError::LimbScratch)?,
                None => crate::ieee::ratio::round_integer_product(
                    endpoint.as_integer(),
                    factor.as_integer(),
                    ctx.scale.as_integer(),
                    rounding,
                ),
            }
            .map(BigInt::from)
            .ok_or(MathError::Domain("division by zero"))
        };
        Ok((round(&self.lower)?, round(&self.upper)?))
    }

    /// A round-toward-zero binary64 approximation of the interval's midpoint,
    /// from the leading 64 bits of each endpoint and without allocation.
    /// It is a proposal for selecting evaluation points, never an enclosure;
    /// `None` marks a value outside the normal binary64 range.
    #[must_use]
    pub fn approximate_binary64(&self) -> Option<f64> {
        let approximate = |value: &BigInt| -> Option<f64> {
            let integer = value.as_integer();
            let length = integer.bit_len();
            if length == 0 {
                return Some(0.0);
            }
            let limbs = integer.limbs();
            // The leading 64 bits as an integer, then its binary exponent.
            let top = length - 1;
            let word = usize::try_from(top / 64).ok()?;
            let offset = top % 64;
            let high = limbs[word] << (63 - offset);
            let low = if offset < 63 && word > 0 {
                limbs[word - 1] >> (offset + 1)
            } else {
                0
            };
            let leading = high | low;
            // leading * 2^(top - 63 - precision) with the leading bit at 2^63.
            let exponent = i64::try_from(top).ok()? - 63 - i64::from(self.precision_bits);
            let biased = exponent + 63 + 1023;
            if !(1..=2046).contains(&biased) {
                return None;
            }
            // The 52 fraction bits below the leading one, truncated.
            let fraction = (leading << 1) >> 12;
            let bits = (u64::try_from(biased).ok()? << 52) | fraction;
            let magnitude = f64::from_bits(bits);
            Some(if integer.is_negative() {
                -magnitude
            } else {
                magnitude
            })
        };
        let lower = approximate(&self.lower)?;
        let upper = approximate(&self.upper)?;
        Some(crate::ieee::f64_add(
            crate::ieee::f64_mul(lower, 0.5),
            crate::ieee::f64_mul(upper, 0.5),
        ))
    }

    /// Convert outward to finite binary64 endpoints by exact dyadic comparisons,
    /// including subnormals and contexts wider than binary64's exponent range.
    /// No integer-to-float conversion or floating multiplication participates.
    ///
    /// # Errors
    /// Refuses finite binary64 range overflow, a precision mismatch, or exhausted
    /// numerical admission.
    pub fn to_binary64(&self, ctx: &mut CoordinateMath) -> Result<FloatInterval, MathError> {
        self.admit(None, ctx, 1)?;
        let (lower, _) = outward_binary64(&self.lower, ctx)?;
        let (_, upper) = outward_binary64(&self.upper, ctx)?;
        FloatInterval::from_bounds(lower, upper)
    }

    fn contains_zero(&self) -> bool {
        self.lower <= BigInt::zero() && self.upper >= BigInt::zero()
    }
    fn endpoint_bits(&self) -> usize {
        self.lower
            .bits_upper_bound()
            .max(self.upper.bits_upper_bound())
    }
    fn admit(
        &self,
        rhs: Option<&Self>,
        ctx: &mut CoordinateMath,
        multiplier: usize,
    ) -> Result<(), MathError> {
        if self.precision_bits != ctx.limits.precision_bits
            || rhs.is_some_and(|rhs| rhs.precision_bits != self.precision_bits)
        {
            return Err(MathError::PrecisionExhausted);
        }
        let bits = self
            .endpoint_bits()
            .saturating_add(rhs.map_or(0, Self::endpoint_bits))
            .max(ctx.limits.precision_bits as usize)
            .saturating_mul(multiplier)
            .saturating_add(2);
        ctx.charge(1, bits)
    }
    fn expand_units(&self, units: u32, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        let amount = BigInt::from_i128(i128::from(units));
        Self::from_bounds(
            ctx.integer_sub(&self.lower, &amount)?,
            ctx.integer_add(&self.upper, &amount)?,
            ctx,
        )
    }
    fn scale_integer(&self, factor: &BigInt, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        ctx.charge(
            1,
            self.endpoint_bits()
                .saturating_add(factor.bits_upper_bound()),
        )?;
        let a = ctx.integer_mul(&self.lower, factor)?;
        let b = ctx.integer_mul(&self.upper, factor)?;
        Self::from_bounds(a.clone().min(b.clone()), a.max(b), ctx)
    }
    fn half(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        let two = BigInt::from_i128(2);
        Self::from_bounds(
            directed(&self.lower, &two, false, ctx)?,
            directed(&self.upper, &two, true, ctx)?,
            ctx,
        )
    }
}

fn directed(
    numerator: &BigInt,
    denominator: &BigInt,
    upward: bool,
    ctx: &CoordinateMath,
) -> Result<BigInt, MathError> {
    let (quotient, remainder) = ctx
        .integer_div_rem(numerator, denominator)?
        .ok_or(MathError::Domain("division by zero"))?;
    let (lower, upper) = quotient_bounds(
        quotient,
        &remainder,
        numerator.is_negative() != denominator.is_negative(),
        ctx,
    )?;
    Ok(if upward { upper } else { lower })
}

fn product_bounds(
    a: &BigInt,
    b: &BigInt,
    divisor: &BigInt,
    ctx: &CoordinateMath,
) -> Result<(BigInt, BigInt), MathError> {
    let (quotient, remainder) = ctx
        .integer_product_div_rem(a, b, divisor)?
        .ok_or(MathError::Domain("division by zero"))?;
    quotient_bounds(
        quotient,
        &remainder,
        (a.is_negative() != b.is_negative()) != divisor.is_negative(),
        ctx,
    )
}
fn quotient_bounds(
    quotient: BigInt,
    remainder: &BigInt,
    negative: bool,
    ctx: &CoordinateMath,
) -> Result<(BigInt, BigInt), MathError> {
    if remainder.is_zero() {
        Ok((quotient.clone(), quotient))
    } else if negative {
        Ok((ctx.integer_sub(&quotient, &BigInt::from_i128(1))?, quotient))
    } else {
        Ok((
            quotient.clone(),
            ctx.integer_add(&quotient, &BigInt::from_i128(1))?,
        ))
    }
}

pub(super) fn nearest_even(
    numerator: &BigInt,
    denominator: &BigInt,
    ctx: &CoordinateMath,
) -> Result<BigInt, MathError> {
    match ctx.limb_scratch() {
        Some(scratch) => crate::ieee::ratio::round_even_integer_in(
            numerator.as_integer(),
            denominator.as_integer(),
            scratch,
        )
        .map_err(MathError::LimbScratch)?,
        None => {
            crate::ieee::ratio::round_even_integer(numerator.as_integer(), denominator.as_integer())
        }
    }
    .map(BigInt::from)
    .ok_or(MathError::Domain("division by zero"))
}

fn outward_binary64(value: &BigInt, ctx: &mut CoordinateMath) -> Result<(f64, f64), MathError> {
    use crate::ieee::ratio::{self, Rounding};
    ctx.charge(
        2,
        value
            .bits_upper_bound()
            .saturating_add(ctx.limits.precision_bits as usize)
            .saturating_add(1074),
    )?;
    let lower = ratio::to_binary64(value.as_integer(), ctx.scale.as_integer(), Rounding::Down)
        .ok_or(MathError::Domain("division by zero"))?;
    let upper = ratio::to_binary64(value.as_integer(), ctx.scale.as_integer(), Rounding::Up)
        .ok_or(MathError::Domain("division by zero"))?;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(MathError::Binary64Range);
    }
    Ok((lower, upper))
}
