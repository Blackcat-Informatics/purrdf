// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original bounded-series binary64 kernels. Every polynomial operation rounds
//! outward. Coefficients are generated from exact integers, never imported arrays.

use super::{CheckedBinary64, FloatInterval};
use crate::BigInt;
use crate::ieee::dyadic::Binary64Dyadic;
use crate::math::fixed::nearest_even;
use crate::math::{CoordinateMath, FixedInterval, MathError};
use std::sync::Arc;

/// Immutable generated constants and coefficient enclosures, shareable across
/// workers. Admission and mutable scratch remain in each worker's context.
#[derive(Debug, Clone)]
pub struct PreparedBinary64 {
    coefficients: Arc<Coefficients>,
}

#[derive(Debug)]
struct Coefficients {
    pi: FloatInterval,
    ln2: FloatInterval,
    sine: [FloatInterval; 10],
    cosine: [FloatInterval; 10],
    arctangent: [FloatInterval; 16],
    logarithm: [FloatInterval; 24],
    exponential: [FloatInterval; 23],
}

impl PreparedBinary64 {
    /// Retained storage including the shared table, Arc counters and handle.
    /// Shared storage is conservatively admitted by each using context.
    #[must_use]
    pub const fn workspace_bytes(&self) -> usize {
        size_of::<Self>() + size_of::<Coefficients>() + 2 * size_of::<usize>()
    }
}

impl CoordinateMath {
    /// Generate and cache exact-equation binary64 coefficient enclosures. Share
    /// the returned immutable handle rather than repeating generation per worker.
    ///
    /// # Errors
    /// Refuses exhausted work/workspace, cancellation or finite range overflow.
    pub fn prepare_binary64(&mut self) -> Result<PreparedBinary64, MathError> {
        self.ensure_binary64()?;
        self.binary64.clone().ok_or(MathError::PrecisionExhausted)
    }

    fn ensure_binary64(&mut self) -> Result<(), MathError> {
        self.charge(1, self.limits.precision_bits as usize)?;
        if self.binary64.is_some() {
            return Ok(());
        }
        let coefficients = Coefficients::generate(self)?;
        let bytes =
            size_of::<PreparedBinary64>() + size_of::<Coefficients>() + 2 * size_of::<usize>();
        self.reserve_workspace(bytes)?;
        let prepared = PreparedBinary64 {
            coefficients: Arc::new(coefficients),
        };
        self.binary64 = Some(prepared);
        self.execution_stats.binary64_preparations += 1;
        Ok(())
    }

    /// Install immutable prepared coefficients in this worker's context.
    ///
    /// # Errors
    /// Refuses cancellation, exhausted work/workspace or unbalanced reservation.
    pub fn install_binary64(&mut self, prepared: PreparedBinary64) -> Result<(), MathError> {
        self.charge(1, self.limits.precision_bits as usize)?;
        let previous = self
            .binary64
            .as_ref()
            .map_or(0, PreparedBinary64::workspace_bytes);
        let bytes = prepared.workspace_bytes();
        if bytes >= previous {
            self.reserve_workspace(bytes - previous)?;
        } else {
            self.release_workspace(previous - bytes)?;
        }
        self.binary64 = Some(prepared);
        Ok(())
    }

    pub(in crate::math) fn binary64_pi(&mut self) -> Result<FloatInterval, MathError> {
        self.ensure_binary64()?;
        Ok(self
            .binary64
            .as_ref()
            .expect("prepared coefficients")
            .coefficients
            .pi)
    }

    fn request_binary64(&mut self, work: u64) -> Result<(), MathError> {
        self.charge(work, self.limits.precision_bits as usize)?;
        self.execution_stats.binary64_requests += 1;
        Ok(())
    }
    fn fixed_fallback(&mut self) {
        self.execution_stats.fixed_fallbacks += 1;
    }
}

impl Coefficients {
    fn generate(math: &mut CoordinateMath) -> Result<Self, MathError> {
        let zero = FloatInterval::point(0.0)?;
        let one = BigInt::from_i128(1);
        let pi = FixedInterval::pi(math)?.to_binary64(math)?;
        let ln2 = FixedInterval::from_i64(2, math)?
            .log(math)?
            .to_binary64(math)?;
        let mut sine = [zero; 10];
        let mut cosine = [zero; 10];
        let mut arctangent = [zero; 16];
        let mut logarithm = [zero; 24];
        let mut exponential = [zero; 23];
        let mut factorial = one.clone();
        for (degree, coefficient) in exponential.iter_mut().enumerate() {
            math.charge(1, factorial.bits_upper_bound().saturating_add(8))?;
            if degree != 0 {
                factorial = factorial
                    .mul_small(u32::try_from(degree).map_err(|_| MathError::WorkExhausted)?);
            }
            let value = FloatInterval::from_ratio(&one, &factorial, math)?;
            *coefficient = value;
            if degree <= 19 {
                let index = degree / 2;
                let signed = if index & 1 == 0 { value } else { value.neg() };
                if degree & 1 == 0 {
                    cosine[index] = signed;
                } else {
                    sine[index] = signed;
                }
            }
        }
        for (index, coefficient) in arctangent.iter_mut().enumerate() {
            let denominator = BigInt::from_i128(
                i128::try_from(2 * index + 1).map_err(|_| MathError::WorkExhausted)?,
            );
            let value = FloatInterval::from_ratio(&one, &denominator, math)?;
            *coefficient = if index & 1 == 0 { value } else { value.neg() };
        }
        for (index, coefficient) in logarithm.iter_mut().enumerate() {
            let denominator = BigInt::from_i128(
                i128::try_from(2 * index + 1).map_err(|_| MathError::WorkExhausted)?,
            );
            *coefficient = FloatInterval::from_ratio(&one, &denominator, math)?;
        }
        Ok(Self {
            pi,
            ln2,
            sine,
            cosine,
            arctangent,
            logarithm,
            exponential,
        })
    }
}

impl FloatInterval {
    /// Outward conversion of an exact rational through the shared integer grid
    /// and exact IEEE-bit comparison. No decimal strings participate.
    ///
    /// # Errors
    /// Refuses zero denominator, range overflow or exhausted numerical admission.
    pub fn from_ratio(
        numerator: &BigInt,
        denominator: &BigInt,
        math: &mut CoordinateMath,
    ) -> Result<Self, MathError> {
        use crate::ieee::ratio::{self, Rounding};
        math.charge(
            2,
            numerator
                .bits_upper_bound()
                .saturating_add(denominator.bits_upper_bound())
                .saturating_add(1074),
        )?;
        let lower = ratio::to_binary64(
            numerator.as_integer(),
            denominator.as_integer(),
            Rounding::Down,
        )
        .ok_or(MathError::Domain("division by zero"))?;
        let upper = ratio::to_binary64(
            numerator.as_integer(),
            denominator.as_integer(),
            Rounding::Up,
        )
        .ok_or(MathError::Domain("division by zero"))?;
        Self::from_bounds(lower, upper)
    }

    /// Half-even decimal mantissas of the exact binary64 enclosure endpoints.
    /// Matching integers certify the completed rounded result. This method does
    /// not quantize through the context's grid and preserves subnormal endpoints.
    ///
    /// # Errors
    /// Refuses exhausted numerical admission.
    pub fn round_decimal(
        self,
        places: u32,
        math: &mut CoordinateMath,
        _chunk: &CheckedBinary64,
    ) -> Result<(BigInt, BigInt), MathError> {
        Ok((
            round_endpoint(self.lower, places, math)?,
            round_endpoint(self.upper, places, math)?,
        ))
    }

    /// Bounded sine/cosine Taylor evaluation after quarter-turn reduction.
    /// The degree-19 sine and degree-18 cosine remainders are bounded separately
    /// by `1/21!` and `1/20!` on `[-1,1]`; `2^-61` encloses both.
    ///
    /// # Errors
    /// Refuses an unresolved fixed fallback or exhausted numerical admission.
    pub fn sin_cos(
        self,
        math: &mut CoordinateMath,
        chunk: &CheckedBinary64,
    ) -> Result<(Self, Self), MathError> {
        math.request_binary64(64)?;
        math.ensure_binary64()?;
        let prepared = math
            .binary64
            .as_ref()
            .ok_or(MathError::PrecisionExhausted)?;
        match fast_sin_cos(self, &prepared.coefficients, chunk) {
            Ok(result) => Ok(result),
            Err(MathError::PrecisionExhausted | MathError::Binary64Range) => {
                math.fixed_fallback();
                let (sine, cosine) = fixed_input(self, math)?.sin_cos(math)?;
                Ok((sine.to_binary64(math)?, cosine.to_binary64(math)?))
            }
            Err(error) => Err(error),
        }
    }

    /// Monotone arctangent using reciprocal/two half-angle reductions and a
    /// degree-31 alternating polynomial. Its remainder is at most `2^-66`.
    ///
    /// # Errors
    /// Refuses an unresolved fixed fallback or exhausted numerical admission.
    pub fn atan(
        self,
        math: &mut CoordinateMath,
        chunk: &CheckedBinary64,
    ) -> Result<Self, MathError> {
        math.request_binary64(128)?;
        math.ensure_binary64()?;
        let prepared = math
            .binary64
            .as_ref()
            .ok_or(MathError::PrecisionExhausted)?;
        match fast_atan(self, &prepared.coefficients, chunk) {
            Ok(result) => Ok(result),
            Err(MathError::PrecisionExhausted | MathError::Binary64Range) => {
                math.fixed_fallback();
                fixed_input(self, math)?.atan(math)?.to_binary64(math)
            }
            Err(error) => Err(error),
        }
    }

    /// Principal angle with the same inclusive branch-cut rule as the fixed
    /// engine: exact negative-axis zero chooses pi; reaching from below refuses.
    ///
    /// # Errors
    /// Refuses the origin, unresolved branch cuts or exhausted admission.
    pub fn atan2(
        y: Self,
        x: Self,
        math: &mut CoordinateMath,
        chunk: &CheckedBinary64,
    ) -> Result<Self, MathError> {
        math.request_binary64(128)?;
        math.ensure_binary64()?;
        let prepared = math
            .binary64
            .as_ref()
            .ok_or(MathError::PrecisionExhausted)?;
        match fast_atan2(y, x, &prepared.coefficients, chunk) {
            Ok(result) => Ok(result),
            Err(MathError::PrecisionExhausted | MathError::Binary64Range) => {
                math.fixed_fallback();
                let y = fixed_input(y, math)?;
                let x = fixed_input(x, math)?;
                FixedInterval::atan2(&y, &x, math)?.to_binary64(math)
            }
            Err(error) => Err(error),
        }
    }

    /// Monotone logarithm using exact IEEE significand/exponent reduction and
    /// the atanh series on `[0,1/3]`; the tail is at most `2^-76`.
    ///
    /// # Errors
    /// Refuses nonpositive inputs, unresolved boundaries or exhausted admission.
    pub fn log(
        self,
        math: &mut CoordinateMath,
        chunk: &CheckedBinary64,
    ) -> Result<Self, MathError> {
        math.request_binary64(160)?;
        math.ensure_binary64()?;
        let prepared = math
            .binary64
            .as_ref()
            .ok_or(MathError::PrecisionExhausted)?;
        match fast_log(self, &prepared.coefficients, chunk) {
            Ok(result) => Ok(result),
            Err(MathError::PrecisionExhausted | MathError::Binary64Range) => {
                math.fixed_fallback();
                fixed_input(self, math)?.log(math)?.to_binary64(math)
            }
            Err(error) => Err(error),
        }
    }

    /// Monotone exponential using ln2 reduction and degree-22 Taylor evaluation.
    /// Taylor's remainder is at most `3/23! < 2^-72` on `[-1,1]`.
    ///
    /// # Errors
    /// Refuses finite range overflow, unresolved reduction or exhausted admission.
    pub fn exp(
        self,
        math: &mut CoordinateMath,
        chunk: &CheckedBinary64,
    ) -> Result<Self, MathError> {
        math.request_binary64(128)?;
        math.ensure_binary64()?;
        let prepared = math
            .binary64
            .as_ref()
            .ok_or(MathError::PrecisionExhausted)?;
        match fast_exp(self, &prepared.coefficients, chunk) {
            Ok(result) => Ok(result),
            Err(MathError::PrecisionExhausted | MathError::Binary64Range) => {
                math.fixed_fallback();
                fixed_input(self, math)?.exp(math)?.to_binary64(math)
            }
            Err(error) => Err(error),
        }
    }
}

fn fixed_input(
    value: FloatInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let lower = FixedInterval::from_binary64(value.lower, math)?;
    let upper = FixedInterval::from_binary64(value.upper, math)?;
    FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)
}

fn round_endpoint(value: f64, places: u32, math: &mut CoordinateMath) -> Result<BigInt, MathError> {
    let dyadic = Binary64Dyadic::decode(value).ok_or(MathError::Binary64Range)?;
    math.charge(
        1,
        64_usize
            .saturating_add(dyadic.exponent().unsigned_abs() as usize)
            .saturating_add((places as usize).saturating_mul(4)),
    )?;
    let mut numerator = BigInt::from_i128(i128::from(dyadic.significand())).mul_pow10(places);
    if dyadic.negative() {
        numerator = numerator.negated();
    }
    if dyadic.exponent() >= 0 {
        nearest_even(
            &numerator.mul_pow2(dyadic.exponent().unsigned_abs()),
            &BigInt::from_i128(1),
            math,
        )
    } else {
        nearest_even(
            &numerator,
            &BigInt::from_i128(1).mul_pow2(dyadic.exponent().unsigned_abs()),
            math,
        )
    }
}

fn polynomial(
    coefficients: &[FloatInterval],
    x: FloatInterval,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    let Some((&last, preceding)) = coefficients.split_last() else {
        return Err(MathError::Domain("empty polynomial"));
    };
    let mut result = last;
    for &coefficient in preceding.iter().rev() {
        result = result.mul(x, chunk)?.add(coefficient, chunk)?;
    }
    Ok(result)
}

fn padding(exponent: u32) -> FloatInterval {
    let value = f64::from_bits(u64::from(1023 - exponent) << 52);
    FloatInterval {
        lower: -value,
        upper: value,
    }
}

fn point(value: f64) -> FloatInterval {
    FloatInterval {
        lower: value,
        upper: value,
    }
}

fn fast_sin_cos(
    value: FloatInterval,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<(FloatInterval, FloatInterval), MathError> {
    if value.lower < -1_000_000.0 || value.upper > 1_000_000.0 {
        return Err(MathError::PrecisionExhausted);
    }
    let ops = chunk.ops();
    let quarter = coefficients.pi.mul(point(0.5), chunk)?;
    let middle = ops.mul(ops.add(value.lower, value.upper), 0.5);
    let divisor = ops.mul(ops.add(quarter.lower, quarter.upper), 0.5);
    let turns = ops.div(middle, divisor).round_ties_even();
    let reduced = value.sub(quarter.mul(point(turns), chunk)?, chunk)?;
    if reduced.lower < -1.0 || reduced.upper > 1.0 {
        return Err(MathError::PrecisionExhausted);
    }
    let square = reduced.square(chunk)?;
    let sine = polynomial(&coefficients.sine, square, chunk)?
        .mul(reduced, chunk)?
        .add(padding(61), chunk)?;
    let cosine = polynomial(&coefficients.cosine, square, chunk)?.add(padding(61), chunk)?;
    // Admission above proves |turns| < 637000, so this exact integer conversion
    // and four-way residue cannot truncate or change the chosen rotation.
    let quadrant = (turns as i64).rem_euclid(4);
    let (sine, cosine) = match quadrant {
        0 => (sine, cosine),
        1 => (cosine, sine.neg()),
        2 => (sine.neg(), cosine.neg()),
        _ => (cosine.neg(), sine),
    };
    Ok((clamp_unit(sine)?, clamp_unit(cosine)?))
}

fn clamp_unit(value: FloatInterval) -> Result<FloatInterval, MathError> {
    FloatInterval::from_bounds(value.lower.max(-1.0), value.upper.min(1.0))
}

fn fast_atan(
    value: FloatInterval,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    if value.lower == value.upper {
        return atan_point(value.lower, coefficients, chunk);
    }
    if value.lower < 0.0 && value.upper > 0.0 {
        let lower = atan_point(value.lower, coefficients, chunk)?;
        let upper = atan_point(value.upper, coefficients, chunk)?;
        return FloatInterval::from_bounds(lower.lower, upper.upper);
    }
    // Mean-value form about one interior point m. On a sign-definite
    // interval arctangent is increasing with derivative at most
    // 1/(1+min|x|^2), so atan(x)-atan(m) lies between 0 and slope*(x-m).
    let ops = chunk.ops();
    let middle = ops
        .add(ops.mul(value.lower, 0.5), ops.mul(value.upper, 0.5))
        .clamp(value.lower, value.upper);
    let center = atan_point(middle, coefficients, chunk)?;
    let near = value.lower.abs().min(value.upper.abs());
    let slope = point(1.0)
        .div(point(1.0).add(point(near).square(chunk)?, chunk)?, chunk)?
        .upper
        .min(1.0);
    let below = point(value.lower).sub(point(middle), chunk)?.lower.min(0.0);
    let above = point(value.upper).sub(point(middle), chunk)?.upper.max(0.0);
    let offsets = FloatInterval::from_bounds(below, above)?.mul(point(slope), chunk)?;
    center.add(offsets, chunk)
}

fn atan_point(
    value: f64,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    if value == 0.0 {
        return Ok(point(0.0));
    }
    let mut reduced = point(value.abs());
    let reciprocal = value.abs() > 1.0;
    if reciprocal {
        reduced = point(1.0).div(reduced, chunk)?;
    }
    for _ in 0..2 {
        reduced = reduced.div(
            point(1.0).add(
                point(1.0).add(reduced.square(chunk)?, chunk)?.sqrt(chunk)?,
                chunk,
            )?,
            chunk,
        )?;
    }
    if reduced.upper > 0.25 {
        return Err(MathError::PrecisionExhausted);
    }
    let mut result = polynomial(&coefficients.arctangent, reduced.square(chunk)?, chunk)?
        .mul(reduced, chunk)?
        .add(padding(66), chunk)?
        .mul(point(4.0), chunk)?;
    if reciprocal {
        result = coefficients.pi.mul(point(0.5), chunk)?.sub(result, chunk)?;
    }
    Ok(if value < 0.0 { result.neg() } else { result })
}

fn fast_atan2(
    y: FloatInterval,
    x: FloatInterval,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    // Choose the nonsingular axis chart when the y component dominates. This
    // avoids overflowing y/x at finite points adjacent to the y axis.
    let x_magnitude = x.lower.abs().max(x.upper.abs());
    if y.lower > 0.0 && y.lower >= x_magnitude {
        return coefficients
            .pi
            .mul(point(0.5), chunk)?
            .sub(fast_atan(x.div(y, chunk)?, coefficients, chunk)?, chunk);
    }
    if y.upper < 0.0 && -y.upper >= x_magnitude {
        return coefficients
            .pi
            .mul(point(-0.5), chunk)?
            .sub(fast_atan(x.div(y, chunk)?, coefficients, chunk)?, chunk);
    }
    if x.lower > 0.0 {
        return fast_atan(y.div(x, chunk)?, coefficients, chunk);
    }
    if x.upper < 0.0 {
        if y.lower == 0.0 && y.upper == 0.0 {
            return Ok(coefficients.pi);
        }
        if y.lower >= 0.0 {
            return fast_atan(y.div(x, chunk)?, coefficients, chunk)?.add(coefficients.pi, chunk);
        }
        if y.upper < 0.0 {
            return fast_atan(y.div(x, chunk)?, coefficients, chunk)?.sub(coefficients.pi, chunk);
        }
        return Err(MathError::PrecisionExhausted);
    }
    if y.lower > 0.0 {
        return coefficients
            .pi
            .mul(point(0.5), chunk)?
            .sub(fast_atan(x.div(y, chunk)?, coefficients, chunk)?, chunk);
    }
    if y.upper < 0.0 {
        return coefficients
            .pi
            .mul(point(-0.5), chunk)?
            .sub(fast_atan(x.div(y, chunk)?, coefficients, chunk)?, chunk);
    }
    Err(
        if x.lower == 0.0 && x.upper == 0.0 && y.lower == 0.0 && y.upper == 0.0 {
            MathError::Domain("atan2 origin")
        } else {
            MathError::PrecisionExhausted
        },
    )
}

fn fast_log(
    value: FloatInterval,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    if value.lower <= 0.0 {
        return Err(if value.upper <= 0.0 {
            MathError::Domain("nonpositive logarithm")
        } else {
            MathError::PrecisionExhausted
        });
    }
    let lower = log_point(value.lower, coefficients, chunk)?;
    let upper = log_point(value.upper, coefficients, chunk)?;
    FloatInterval::from_bounds(lower.lower, upper.upper)
}

fn log_point(
    value: f64,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    let dyadic = Binary64Dyadic::decode(value).ok_or(MathError::Binary64Range)?;
    let significand_bits = 64 - dyadic.significand().leading_zeros();
    let normalized = dyadic.significand() << (53 - significand_bits);
    let mantissa = point(f64::from_bits(
        (1023_u64 << 52) | (normalized & ((1_u64 << 52) - 1)),
    ));
    let exponent = dyadic.exponent()
        + i32::try_from(significand_bits).map_err(|_| MathError::PrecisionExhausted)?
        - 1;
    let z = mantissa
        .sub(point(1.0), chunk)?
        .div(mantissa.add(point(1.0), chunk)?, chunk)?;
    let log_mantissa = polynomial(&coefficients.logarithm, z.square(chunk)?, chunk)?
        .mul(z, chunk)?
        .mul(point(2.0), chunk)?
        .add(padding(76), chunk)?;
    log_mantissa.add(
        coefficients.ln2.mul(point(f64::from(exponent)), chunk)?,
        chunk,
    )
}

fn fast_exp(
    value: FloatInterval,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    if value.lower < -740.0 || value.upper > 709.0 {
        return Err(MathError::PrecisionExhausted);
    }
    let lower = exp_point(value.lower, coefficients, chunk)?;
    let upper = exp_point(value.upper, coefficients, chunk)?;
    FloatInterval::from_bounds(lower.lower.max(0.0), upper.upper)
}

fn exp_point(
    value: f64,
    coefficients: &Coefficients,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    let ops = chunk.ops();
    let logarithm = ops.mul(ops.add(coefficients.ln2.lower, coefficients.ln2.upper), 0.5);
    let exponent = ops.div(value, logarithm).round_ties_even();
    let reduced = point(value).sub(coefficients.ln2.mul(point(exponent), chunk)?, chunk)?;
    if reduced.lower < -1.0 || reduced.upper > 1.0 {
        return Err(MathError::PrecisionExhausted);
    }
    // The admitted input range proves -1068 <= exponent <= 1023; each power
    // below is an exactly encoded binary64, including subnormal powers.
    let exponent = exponent as i32;
    let factor = if exponent >= -1022 {
        f64::from_bits(
            u64::try_from(exponent + 1023).map_err(|_| MathError::PrecisionExhausted)? << 52,
        )
    } else {
        f64::from_bits(
            1_u64 << u32::try_from(exponent + 1074).map_err(|_| MathError::PrecisionExhausted)?,
        )
    };
    polynomial(&coefficients.exponential, reduced, chunk)?
        .add(padding(72), chunk)?
        .mul(point(factor), chunk)
}
