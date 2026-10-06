// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original area coefficients from the differential equation for `t(z)`.
//!
//! `t(z)=z+sqrt(1+z)*asinh(sqrt(z))/sqrt(z)` obeys
//! `2z(1+z)t'=1+4z+2z²−t`. Its removable value at zero is one.
//! The ordinary series is used for small eccentricity. A Taylor series centered
//! at the positive eccentricity parameter converges throughout the complete
//! oblate domain, with a Cauchy remainder on a circle strictly above the cut.

use super::super::{ChunkObserver, SolveError};
use crate::{GeographicReference, numerical::fixed_from_rat};
use purrdf_core::SmallVec;
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

type F = FixedInterval;

pub(crate) fn authalic_radius_squared(
    reference: &GeographicReference,
    math: &mut CoordinateMath,
) -> Result<F, MathError> {
    let a = fixed_from_rat(reference.ellipsoid().semimajor(), math)?;
    let c = fixed_from_rat(reference.ellipsoid().semiminor(), math)?.div(&a, math)?;
    let e2 = fixed_from_rat(&reference.ellipsoid().eccentricity_squared(), math)?;
    let one = F::from_i64(1, math)?;
    if reference.ellipsoid().eccentricity_squared().is_zero() {
        return a.square(math);
    }
    let e = e2.sqrt(math)?;
    let atanh = one
        .add(&e, math)?
        .div(&one.sub(&e, math)?, math)?
        .log(math)?
        .div(&F::from_i64(2, math)?, math)?;
    one.add(&c.square(math)?.mul(&atanh.div(&e, math)?, math)?, math)?
        .div(&F::from_i64(2, math)?, math)?
        .mul(&a.square(math)?, math)
}

#[allow(clippy::too_many_arguments)] // One auxiliary arc and its exact ellipsoid parameters.
pub(crate) fn quadrilateral(
    alpha1: &F,
    alpha2: &F,
    sigma1: &F,
    sigma2: &F,
    sin0: &F,
    cos0_squared: &F,
    q: &F,
    c: &F,
    ep2: &F,
    reference: &GeographicReference,
    prepared: Option<&PreparedArea>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<F, SolveError> {
    if cos0_squared.is_exact_zero() {
        // The equatorial line has phi=0 and Q(0)=0 along its complete lift.
        return Ok(F::from_i64(0, math)?);
    }
    let authalic = authalic_radius_squared(reference, math)?;
    let leading = authalic.mul(&alpha2.sub(alpha1, math)?, math)?;
    if sin0.is_exact_zero() {
        return Ok(leading);
    }
    let a = fixed_from_rat(reference.ellipsoid().semimajor(), math)?;
    let one = F::from_i64(1, math)?;
    let e2 = one.sub(&c.square(math)?, math)?;
    let correction = i4(sigma1, sigma2, q, ep2, prepared, chunks, math)?
        .mul(&e2, math)?
        .mul(&a.square(math)?, math)?
        .mul(&cos0_squared.sqrt(math)?, math)?
        .mul(sin0, math)?;
    Ok(leading.add(&correction, math)?)
}

/// Immutable coefficients and a per-radian Cauchy tail bound from the original
/// t equation. Reference ownership guarantees matching ellipsoid parameters.
#[derive(Debug)]
pub(crate) struct PreparedArea {
    bits: u32,
    center: F,
    coefficients: SmallVec<[F; 17]>,
    tail_per_radian: F,
}
impl PreparedArea {
    pub(crate) fn detached_values(&self) -> usize {
        self.coefficients.len() + 2
    }

    pub(crate) fn detached_bytes(&self) -> usize {
        size_of::<Self>()
            + 2 * size_of::<usize>()
            + if self.coefficients.spilled() {
                self.coefficients.len() * size_of::<F>()
            } else {
                0
            }
            + core::iter::once(&self.center)
                .chain(core::iter::once(&self.tail_per_radian))
                .chain(self.coefficients.iter())
                .map(F::detached_heap_bytes)
                .sum::<usize>()
    }

    pub(crate) fn detached(&self) -> Self {
        let mut coefficients = SmallVec::with_capacity(self.coefficients.len());
        coefficients.extend(self.coefficients.iter().map(F::detached));
        Self {
            bits: self.bits,
            center: self.center.detached(),
            coefficients,
            tail_per_radian: self.tail_per_radian.detached(),
        }
    }

    pub(crate) fn workspace_bytes(&self) -> usize {
        size_of::<Self>()
            + 2 * size_of::<usize>()
            + if self.coefficients.spilled() {
                self.coefficients.capacity() * size_of::<F>()
            } else {
                0
            }
            + core::iter::once(&self.center)
                .chain(core::iter::once(&self.tail_per_radian))
                .chain(self.coefficients.iter())
                .map(|value| value.workspace_bytes() - size_of::<F>())
                .sum::<usize>()
    }
}

pub(crate) fn prepare(
    ep2: &F,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<Option<PreparedArea>, SolveError> {
    let small = F::power_of_two(-6, math)?;
    if ep2.upper() >= small.lower() {
        return Ok(None);
    }
    // Eight radians encloses every shortest auxiliary arc, including its
    // outward proof endpoints. Longer explicit lines check the same tail.
    let eight = F::from_i64(8, math)?;
    let (native, center, n, tail) = series_plan(ep2, &eight, chunks, math)?;
    let reservation = (n + 1)
        .checked_mul(size_of::<F>() + (math.limits().precision_bits as usize).div_ceil(8) * 8 + 128)
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(reservation)?;
    chunks.finish(math)?;
    let coefficients = generate_series(native, &center, ep2, n, chunks, math);
    math.release_workspace(reservation)?;
    let coefficients = coefficients?;
    Ok(Some(PreparedArea {
        bits: math.limits().precision_bits,
        center,
        coefficients,
        tail_per_radian: tail.div(&eight, math)?,
    }))
}

#[allow(clippy::too_many_arguments)] // One arc panel and an optional equivalent prepared proof table.
fn i4(
    sigma1: &F,
    sigma2: &F,
    q: &F,
    ep2: &F,
    prepared: Option<&PreparedArea>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<F, SolveError> {
    let delta = sigma2.sub(sigma1, math)?.abs(math)?;
    if let Some(prepared) = prepared.filter(|value| value.bits == math.limits().precision_bits) {
        let tail = prepared.tail_per_radian.mul(&delta, math)?;
        if tail.upper() < area_target(math)?.lower() {
            return evaluate_series(
                sigma1,
                sigma2,
                q,
                &prepared.center,
                &prepared.coefficients,
                &tail,
                chunks,
                math,
            );
        }
    }
    let (native, center, n, tail) = series_plan(ep2, &delta, chunks, math)?;
    let retained_bits = ep2
        .lower()
        .bits_upper_bound()
        .max(ep2.upper().bits_upper_bound())
        .max(math.limits().precision_bits as usize)
        .saturating_add(4);
    let interval_bytes = retained_bits
        .div_ceil(8)
        .saturating_mul(8)
        .saturating_add(128);
    let reservation = n
        .checked_add(2)
        .and_then(|count| count.checked_mul(interval_bytes))
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(reservation)?;
    let result = (|| {
        let coefficients = generate_series(native, &center, ep2, n, chunks, math)?;
        evaluate_series(
            sigma1,
            sigma2,
            q,
            &center,
            &coefficients,
            &tail,
            chunks,
            math,
        )
    })();
    math.release_workspace(reservation)?;
    result
}

fn series_plan(
    ep2: &F,
    delta: &F,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(bool, F, usize, F), SolveError> {
    let one = F::from_i64(1, math)?;
    let two = F::from_i64(2, math)?;
    let half = one.div(&two, math)?;
    let native = ep2.upper() < half.lower();
    let center = if native {
        F::from_i64(0, math)?
    } else {
        ep2.clone()
    };
    // On |z-E|=E+1/2 the real part of 1+z and 1+zu² is
    // at least 1/2. The integral representation of t proves this M bound.
    let radius = if native {
        half.clone()
    } else {
        center.add(&half, math)?
    };
    let r = if native {
        ep2.div(&radius, math)?
    } else {
        center.div(&radius, math)?
    };
    let modulus = if native {
        F::from_i64(4, math)?
    } else {
        center.mul(&two, math)?.add(&half, math)?.add(
            &F::from_i64(3, math)?
                .add(&center.mul(&F::from_i64(4, math)?, math)?, math)?
                .sqrt(math)?,
            math,
        )?
    };
    let complement = one.sub(&r, math)?;
    if complement.lower().is_zero() || complement.lower().is_negative() {
        return Err(MathError::PrecisionExhausted.into());
    }
    let target = area_target(math)?;
    let mut n = 1_usize;
    let mut power = r.clone();
    let tail = loop {
        let mut tail = modulus
            .div(&radius, math)?
            .mul(&power, math)?
            .div(&complement, math)?
            .mul(delta, math)?
            .div(&two, math)?;
        if native {
            let index = i64::try_from(n).map_err(|_| MathError::WorkExhausted)?;
            let numerator = F::from_i64(index + 1, math)?
                .sub(&r.mul(&F::from_i64(index, math)?, math)?, math)?;
            tail = tail.mul(&numerator, math)?.div(&complement, math)?;
        }
        if tail.upper() < target.lower() {
            break tail;
        }
        power = power.mul(&r, math)?;
        n = n.checked_add(1).ok_or(MathError::WorkExhausted)?;
        if n.is_multiple_of(32) {
            chunks.finish(math)?;
        }
    };
    Ok((native, center, n, tail))
}

fn area_target(math: &mut CoordinateMath) -> Result<F, MathError> {
    let exponent = i32::try_from(math.limits().precision_bits.saturating_sub(24))
        .map_err(|_| MathError::PrecisionExhausted)?;
    F::power_of_two(-exponent, math)
}

#[allow(clippy::too_many_arguments)] // One certified series panel and its admitted coefficient table.
fn evaluate_series(
    sigma1: &F,
    sigma2: &F,
    q: &F,
    center: &F,
    coefficients: &[F],
    tail: &F,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<F, SolveError> {
    let one = F::from_i64(1, math)?;
    let two = F::from_i64(2, math)?;
    let n = coefficients.len() - 1;
    let (sin1, u1) = sigma1.sin_cos(math)?;
    let (sin2, u2) = sigma2.sin_cos(math)?;
    let d1 = q.mul(&sin1.square(math)?, math)?.sub(center, math)?;
    let d2 = q.mul(&sin2.square(math)?, math)?.sub(center, math)?;
    let a = q.sub(center, math)?;
    let mut p1 = one.clone();
    let mut p2 = one;
    let mut integral = u2.sub(&u1, math)?;
    let mut total = coefficients[1].mul(&integral, math)?;
    for index in 1..n {
        p1 = p1.mul(&d1, math)?;
        p2 = p2.mul(&d2, math)?;
        let index = i64::try_from(index).map_err(|_| MathError::WorkExhausted)?;
        integral = u2
            .mul(&p2, math)?
            .sub(&u1.mul(&p1, math)?, math)?
            .add(
                &a.mul(&F::from_i64(2 * index, math)?, math)?
                    .mul(&integral, math)?,
                math,
            )?
            .div(&F::from_i64(2 * index + 1, math)?, math)?;
        total = total.add(
            &coefficients[index as usize + 1].mul(&integral, math)?,
            math,
        )?;
        if index % 32 == 0 {
            chunks.finish(math)?;
        }
    }
    let total = total.div(&two, math)?;
    let padding = F::from_bounds(tail.upper().negated(), tail.upper().clone(), math)?;
    Ok(total.add(&padding, math)?)
}

/// Generate the t-equation and divided-difference coefficients in one body.
/// Native 112-bit tables fit inline; longer tables are admitted by the caller.
fn generate_series(
    native: bool,
    center: &F,
    ep2: &F,
    n: usize,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<SmallVec<[F; 17]>, SolveError> {
    let one = F::from_i64(1, math)?;
    let two = F::from_i64(2, math)?;
    let mut coefficients: SmallVec<[F; 17]> = SmallVec::with_capacity(n + 1);
    if native {
        coefficients.push(one.clone());
        coefficients.push(F::from_i64(4, math)?.div(&F::from_i64(3, math)?, math)?);
        for index in 2..=n {
            let index = i64::try_from(index).map_err(|_| MathError::WorkExhausted)?;
            let coefficient = if index == 2 {
                F::from_i64(-2, math)?.div(&F::from_i64(15, math)?, math)?
            } else {
                coefficients[(index - 1) as usize]
                    .mul(&F::from_i64(-2 * (index - 1), math)?, math)?
                    .div(&F::from_i64(2 * index + 1, math)?, math)?
            };
            coefficients.push(coefficient);
            if index % 32 == 0 {
                chunks.finish(math)?;
            }
        }
    } else {
        let root = center.sqrt(math)?;
        let h = one.add(center, math)?.sqrt(math)?;
        let t0 = center.add(
            &h.mul(&root.add(&h, math)?.log(math)?, math)?
                .div(&root, math)?,
            math,
        )?;
        coefficients.push(t0);
        let a0 = center.mul(&one.add(center, math)?, math)?.mul(&two, math)?;
        let a1 = two.add(&center.mul(&F::from_i64(4, math)?, math)?, math)?;
        for index in 0..n {
            let index = i64::try_from(index).map_err(|_| MathError::WorkExhausted)?;
            let rhs = match index {
                0 => one
                    .add(&center.mul(&F::from_i64(4, math)?, math)?, math)?
                    .add(&center.square(math)?.mul(&two, math)?, math)?,
                1 => F::from_i64(4, math)?.add(&center.mul(&F::from_i64(4, math)?, math)?, math)?,
                2 => two.clone(),
                _ => F::from_i64(0, math)?,
            };
            let mut numerator = rhs.sub(
                &one.add(&a1.mul(&F::from_i64(index, math)?, math)?, math)?
                    .mul(&coefficients[index as usize], math)?,
                math,
            )?;
            if index >= 1 {
                numerator = numerator.sub(
                    &coefficients[(index - 1) as usize]
                        .mul(&F::from_i64(2 * (index - 1), math)?, math)?,
                    math,
                )?;
            }
            coefficients.push(numerator.div(&a0.mul(&F::from_i64(index + 1, math)?, math)?, math)?);
            if index % 32 == 0 {
                chunks.finish(math)?;
            }
        }
    }
    if native {
        // Divided difference of the zero-centered t series:
        // a_j=sum(n=j+1..N) t_n E^(n-1-j), generated by Horner.
        for index in (1..n).rev() {
            coefficients[index] =
                coefficients[index].add(&ep2.mul(&coefficients[index + 1], math)?, math)?;
        }
    }
    Ok(coefficients)
}
