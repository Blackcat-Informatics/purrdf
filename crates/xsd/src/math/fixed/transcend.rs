// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Constants and transcendental enclosures derived from exact series.
//! Machin's identity, Taylor's theorem, the alternating-series theorem and the
//! geometric-series bound supply remainders independently of rounded evaluation.

use super::{CoordinateMath, FixedInterval, MathError, directed, nearest_even};
use crate::BigInt;

impl FixedInterval {
    /// Enclose pi using `16 atan(1/5) - 4 atan(1/239)` and alternating rational
    /// series, generated at the context's precision and retained in that context.
    ///
    /// # Errors
    /// Refuses exhausted numerical admission.
    pub fn pi(ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        ctx.charge(1, ctx.limits.precision_bits as usize)?;
        if let Some(value) = &ctx.pi {
            return Ok(value.clone());
        }
        let a = atan_reciprocal_integer(5, ctx)?.scale_integer(&BigInt::from_i128(16), ctx)?;
        let b = atan_reciprocal_integer(239, ctx)?.scale_integer(&BigInt::from_i128(4), ctx)?;
        let pi = a.sub(&b, ctx)?;
        ctx.reserve_workspace(pi.workspace_bytes())?;
        ctx.pi = Some(pi.clone());
        Ok(pi)
    }

    /// Enclose sine and cosine after exact quarter-turn reduction. Taylor
    /// coefficients are generated through exact factorial ratios. Taylor's
    /// theorem bounds the remainder by the next omitted monomial over the
    /// complete reduced argument interval inside `[-1,1]`.
    ///
    /// # Errors
    /// Refuses insufficient precision for angular reduction, mismatched
    /// precision, or exhausted numerical admission.
    pub fn sin_cos(&self, ctx: &mut CoordinateMath) -> Result<(Self, Self), MathError> {
        self.admit(None, ctx, 1)?;
        if self.is_exact_zero() {
            return Ok((self.clone(), Self::from_i64(1, ctx)?));
        }
        let quarter = Self::pi(ctx)?.half(ctx)?;
        let midpoint_twice = ctx.integer_add(&self.lower, &self.upper)?;
        let quarter_midpoint_twice = ctx.integer_add(&quarter.lower, &quarter.upper)?;
        let turns = nearest_even(&midpoint_twice, &quarter_midpoint_twice, ctx)?;
        let reduced = self.sub(&quarter.scale_integer(&turns, ctx)?, ctx)?;
        if reduced.lower < ctx.scale.negated() || reduced.upper > ctx.scale {
            return Err(MathError::PrecisionExhausted);
        }
        let square = reduced.square(ctx)?;
        let mut sine = reduced.clone();
        let mut sine_term = reduced;
        let mut cosine = Self::from_i64(1, ctx)?;
        let mut cosine_term = cosine.clone();
        let mut index = 0_u32;
        loop {
            index = index.checked_add(1).ok_or(MathError::WorkExhausted)?;
            let twice = index.checked_mul(2).ok_or(MathError::WorkExhausted)?;
            sine_term = divide_integer(
                &sine_term.mul(&square, ctx)?.neg(ctx)?,
                u64::from(twice) * u64::from(twice + 1),
                ctx,
            )?;
            cosine_term = divide_integer(
                &cosine_term.mul(&square, ctx)?.neg(ctx)?,
                u64::from(twice - 1) * u64::from(twice),
                ctx,
            )?;
            sine = sine.add(&sine_term, ctx)?;
            cosine = cosine.add(&cosine_term, ctx)?;
            // Each term encloses its exact signed monomial, including all
            // grid rounding. Multiplication by the complete argument square
            // and the exact next factorial ratio therefore bounds the true
            // analytic Taylor remainder; no successive-sum heuristic is used.
            let next_sine = divide_integer(
                &sine_term.mul(&square, ctx)?.abs(ctx)?,
                u64::from(twice + 2) * u64::from(twice + 3),
                ctx,
            )?;
            let next_cosine = divide_integer(
                &cosine_term.mul(&square, ctx)?.abs(ctx)?,
                u64::from(twice + 1) * u64::from(twice + 2),
                ctx,
            )?;
            if next_sine.upper() <= &BigInt::from_i128(1)
                && next_cosine.upper() <= &BigInt::from_i128(1)
            {
                break;
            }
        }
        sine = sine.expand_units(1, ctx)?;
        cosine = cosine.expand_units(1, ctx)?;
        let (_, remainder) = turns
            .div_rem_u64(4)
            .ok_or(MathError::Domain("zero quarter-turn divisor"))?;
        let quadrant = if turns.is_negative() && remainder != 0 {
            4 - remainder
        } else {
            remainder
        };
        let (sine, cosine) = match quadrant {
            0 => (sine, cosine),
            1 => (cosine, sine.neg(ctx)?),
            2 => (sine.neg(ctx)?, cosine.neg(ctx)?),
            _ => (cosine.neg(ctx)?, sine),
        };
        Ok((clamp_unit(&sine, ctx)?, clamp_unit(&cosine, ctx)?))
    }

    /// Monotone arctangent enclosure. Reciprocal and two half-angle reductions
    /// place the alternating series inside `[-1/4,1/4]`.
    ///
    /// # Errors
    /// Refuses a precision mismatch or exhausted numerical admission.
    pub fn atan(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        let lower = atan_point(&self.lower, ctx)?;
        let upper = atan_point(&self.upper, ctx)?;
        Self::from_bounds(lower.lower, upper.upper, ctx)
    }

    /// Principal angle in `[-pi,pi]`, with exact negative-axis zero choosing pi.
    /// The origin and rectangles reaching the branch cut require explicit
    /// continuous phase framing or a tighter enclosure.
    ///
    /// # Errors
    /// Refuses the exact origin, unresolved origin/branch-cut crossings,
    /// mismatched precision, or exhausted numerical admission.
    pub fn atan2(y: &Self, x: &Self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        y.admit(Some(x), ctx, 1)?;
        let zero = BigInt::zero();
        if x.lower > zero {
            return y.div(x, ctx)?.atan(ctx);
        }
        if x.upper < zero {
            let pi = Self::pi(ctx)?;
            if y.lower.is_zero() && y.upper.is_zero() {
                return Ok(pi);
            }
            if y.lower >= zero {
                return y.div(x, ctx)?.atan(ctx)?.add(&pi, ctx);
            }
            if y.upper < zero {
                return y.div(x, ctx)?.atan(ctx)?.sub(&pi, ctx);
            }
            return Err(MathError::PrecisionExhausted);
        }
        if y.lower > zero {
            return Self::pi(ctx)?
                .half(ctx)?
                .sub(&x.div(y, ctx)?.atan(ctx)?, ctx);
        }
        if y.upper < zero {
            return Self::pi(ctx)?
                .half(ctx)?
                .neg(ctx)?
                .sub(&x.div(y, ctx)?.atan(ctx)?, ctx);
        }
        Err(
            if x.lower.is_zero() && x.upper.is_zero() && y.lower.is_zero() && y.upper.is_zero() {
                MathError::Domain("atan2 origin")
            } else {
                MathError::PrecisionExhausted
            },
        )
    }

    /// Monotone natural logarithm enclosure using exact power-of-two reduction
    /// and the positive `2 atanh((m-1)/(m+1))` series for `1 <= m <= 2`.
    ///
    /// # Errors
    /// Refuses nonpositive arguments, unresolved domain boundaries, mismatched
    /// precision, or exhausted numerical admission.
    pub fn log(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        if self.lower <= BigInt::zero() {
            return Err(if self.upper <= BigInt::zero() {
                MathError::Domain("nonpositive logarithm")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let lower = log_point(&self.lower, ctx)?;
        let upper = log_point(&self.upper, ctx)?;
        Self::from_bounds(lower.lower, upper.upper, ctx)
    }

    /// Monotone exponential enclosure using logarithmic power-of-two reduction
    /// and Taylor's theorem with `exp(|r|) < 3` on `[-1,1]`.
    ///
    /// # Errors
    /// Refuses insufficient reduction precision, mismatched precision, or
    /// exhausted numerical admission.
    pub fn exp(&self, ctx: &mut CoordinateMath) -> Result<Self, MathError> {
        self.admit(None, ctx, 1)?;
        let lower = exp_point(&self.lower, ctx)?;
        let upper = exp_point(&self.upper, ctx)?;
        Self::from_bounds(lower.lower, upper.upper, ctx)
    }
}

fn atan_reciprocal_integer(q: u32, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    let mut power = BigInt::from_i128(i128::from(q));
    let mut sum = FixedInterval::from_i64(0, ctx)?;
    let mut index = 0_u32;
    loop {
        let odd = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(MathError::WorkExhausted)?;
        ctx.charge(1, power.bits_upper_bound().saturating_add(32))?;
        let denominator = ctx.integer_mul(&power, &BigInt::from_i128(i128::from(odd)))?;
        if denominator >= ctx.scale {
            // Alternating remainder is no larger than the first omitted term.
            return sum.expand_units(1, ctx);
        }
        let term = FixedInterval::from_ratio(&BigInt::from_i128(1), &denominator, ctx)?;
        sum = if index & 1 == 0 {
            sum.add(&term, ctx)?
        } else {
            sum.sub(&term, ctx)?
        };
        power = ctx.integer_mul(
            &ctx.integer_mul(&power, &BigInt::from_i128(i128::from(q)))?,
            &BigInt::from_i128(i128::from(q)),
        )?;
        index = index.checked_add(1).ok_or(MathError::WorkExhausted)?;
    }
}

fn divide_integer(
    value: &FixedInterval,
    denominator: u64,
    ctx: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    value.admit(None, ctx, 1)?;
    let denominator = BigInt::from_i128(i128::from(denominator));
    FixedInterval::from_bounds(
        directed(&value.lower, &denominator, false, ctx)?,
        directed(&value.upper, &denominator, true, ctx)?,
        ctx,
    )
}

fn clamp_unit(value: &FixedInterval, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    FixedInterval::from_bounds(
        value.lower.clone().max(ctx.scale.negated()),
        value.upper.clone().min(ctx.scale.clone()),
        ctx,
    )
}

fn atan_point(value: &BigInt, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    if value.is_zero() {
        return FixedInterval::from_i64(0, ctx);
    }
    let negative = value.is_negative();
    let mut reduced = FixedInterval::from_bounds(value.abs(), value.abs(), ctx)?;
    let one = FixedInterval::from_i64(1, ctx)?;
    let reciprocal = reduced.lower > ctx.scale;
    if reciprocal {
        reduced = one.div(&reduced, ctx)?;
    }
    for _ in 0..2 {
        let denominator = one.add(&one.add(&reduced.square(ctx)?, ctx)?.sqrt(ctx)?, ctx)?;
        reduced = reduced.div(&denominator, ctx)?;
    }
    if ctx.integer_mul(&reduced.upper, &BigInt::from_i128(4))? > ctx.scale {
        return Err(MathError::PrecisionExhausted);
    }
    let squared = reduced.square(ctx)?;
    let mut power = reduced;
    let mut sum = power.clone();
    // On |z|<=1/4 the alternating-series theorem bounds the remaining
    // analytic tail by the first omitted monomial. Every rounded power
    // interval includes that exact monomial, including for tiny arguments.
    let mut index = 1_u64;
    loop {
        power = power.mul(&squared, ctx)?.neg(ctx)?;
        let denominator = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(MathError::WorkExhausted)?;
        let term = divide_integer(&power, denominator, ctx)?;
        if term.abs(ctx)?.upper() <= &BigInt::from_i128(1) {
            break;
        }
        sum = sum.add(&term, ctx)?;
        index = index.checked_add(1).ok_or(MathError::WorkExhausted)?;
    }
    let mut result = sum
        .expand_units(1, ctx)?
        .scale_integer(&BigInt::from_i128(4), ctx)?;
    if reciprocal {
        result = FixedInterval::pi(ctx)?.half(ctx)?.sub(&result, ctx)?;
    }
    if negative {
        result = result.neg(ctx)?;
    }
    Ok(result)
}

fn ln2(ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    if let Some(value) = &ctx.ln2 {
        return Ok(value.clone());
    }
    let third = FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(3), ctx)?;
    let value = atanh_series(&third, ctx)?;
    ctx.reserve_workspace(value.workspace_bytes())?;
    ctx.ln2 = Some(value.clone());
    Ok(value)
}

fn atanh_series(z: &FixedInterval, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    // Rounding can put the 1/3 enclosure one grid unit above 1/3. The actual
    // mathematical argument remains <=1/3; the remainder bound uses that fact,
    // while interval evaluation encloses each finite polynomial independently.
    let squared = z.square(ctx)?;
    let mut power = z.clone();
    let mut sum = power.clone();
    let mut denominator_bound = BigInt::from_i128(3);
    let required = ctx.integer_mul(&ctx.scale, &BigInt::from_i128(4))?;
    let mut index = 0_u32;
    while denominator_bound < required {
        index = index.checked_add(1).ok_or(MathError::WorkExhausted)?;
        ctx.charge(1, denominator_bound.bits_upper_bound().saturating_add(4))?;
        power = power.mul(&squared, ctx)?;
        sum = sum.add(&divide_integer(&power, u64::from(index) * 2 + 1, ctx)?, ctx)?;
        denominator_bound = ctx.integer_mul(&denominator_bound, &BigInt::from_i128(9))?;
        ctx.charge(1, denominator_bound.bits_upper_bound())?;
    }
    // Tail of 2 sum z^(2k+1)/(2k+1) <= (9/4)3^-(2N+1).
    sum.scale_integer(&BigInt::from_i128(2), ctx)?
        .expand_units(1, ctx)
}

fn bit_length(value: &BigInt, ctx: &mut CoordinateMath) -> Result<u32, MathError> {
    let mut low = 0_usize;
    let mut high = value.bits_upper_bound();
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        ctx.charge(1, middle.saturating_add(1))?;
        let exponent = u32::try_from(middle).map_err(|_| MathError::WorkspaceExhausted)?;
        if ctx.integer_pow2(&BigInt::from_i128(1), exponent)? <= *value {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    u32::try_from(low + 1).map_err(|_| MathError::WorkspaceExhausted)
}

fn log_point(value: &BigInt, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    let bits = bit_length(value, ctx)?;
    let exponent = i64::from(bits) - 1 - i64::from(ctx.limits.precision_bits);
    let reduced = if exponent >= 0 {
        let divisor = ctx.integer_pow2(
            &BigInt::from_i128(1),
            u32::try_from(exponent).map_err(|_| MathError::WorkspaceExhausted)?,
        )?;
        FixedInterval::from_bounds(
            directed(value, &divisor, false, ctx)?,
            directed(value, &divisor, true, ctx)?,
            ctx,
        )?
    } else {
        let scaled = ctx.integer_pow2(
            value,
            u32::try_from(-exponent).map_err(|_| MathError::WorkspaceExhausted)?,
        )?;
        FixedInterval::from_bounds(scaled.clone(), scaled, ctx)?
    };
    let one = FixedInterval::from_i64(1, ctx)?;
    let z = reduced.sub(&one, ctx)?.div(&reduced.add(&one, ctx)?, ctx)?;
    atanh_series(&z, ctx)?.add(
        &ln2(ctx)?.scale_integer(&BigInt::from_i128(i128::from(exponent)), ctx)?,
        ctx,
    )
}

fn exp_point(value: &BigInt, ctx: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    let logarithm = ln2(ctx)?;
    let exponent = nearest_even(
        &ctx.integer_mul(value, &BigInt::from_i128(2))?,
        &ctx.integer_add(&logarithm.lower, &logarithm.upper)?,
        ctx,
    )?;
    let point = FixedInterval::from_bounds(value.clone(), value.clone(), ctx)?;
    let reduced = point.sub(&logarithm.scale_integer(&exponent, ctx)?, ctx)?;
    if reduced.lower < ctx.scale.negated() || reduced.upper > ctx.scale {
        return Err(MathError::PrecisionExhausted);
    }
    if exponent < BigInt::from_i128(-i128::from(ctx.limits.precision_bits) - 2) {
        return FixedInterval::from_bounds(BigInt::zero(), BigInt::from_i128(1), ctx);
    }
    let exponent = exponent
        .to_i128()
        .and_then(|value| i64::try_from(value).ok())
        .ok_or(MathError::WorkspaceExhausted)?;
    let mut term = FixedInterval::from_i64(1, ctx)?;
    let mut sum = term.clone();
    let mut factorial = BigInt::from_i128(1);
    let required = ctx.integer_mul(&ctx.scale, &BigInt::from_i128(3))?;
    let mut index = 0_u32;
    loop {
        index = index.checked_add(1).ok_or(MathError::WorkExhausted)?;
        ctx.charge(1, factorial.bits_upper_bound().saturating_add(64))?;
        term = divide_integer(&term.mul(&reduced, ctx)?, u64::from(index), ctx)?;
        sum = sum.add(&term, ctx)?;
        factorial = ctx.integer_mul(&factorial, &BigInt::from_i128(i128::from(index)))?;
        ctx.charge(1, factorial.bits_upper_bound())?;
        if ctx.integer_mul(&factorial, &BigInt::from_i128(i128::from(index + 1)))? >= required {
            break;
        }
    }
    let sum = sum.expand_units(1, ctx)?;
    let shift =
        u32::try_from(exponent.unsigned_abs()).map_err(|_| MathError::WorkspaceExhausted)?;
    ctx.charge(1, sum.endpoint_bits().saturating_add(shift as usize))?;
    if exponent >= 0 {
        FixedInterval::from_bounds(
            ctx.integer_pow2(&sum.lower, shift)?,
            ctx.integer_pow2(&sum.upper, shift)?,
            ctx,
        )
    } else {
        let divisor = ctx.integer_pow2(&BigInt::from_i128(1), shift)?;
        FixedInterval::from_bounds(
            directed(&sum.lower, &divisor, false, ctx)?.max(BigInt::zero()),
            directed(&sum.upper, &divisor, true, ctx)?,
            ctx,
        )
    }
}
