// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one binomial/reciprocal coefficient generator for auxiliary integrals.

use purrdf_xsd::math::{CertifiedInterval, IntervalBound, IntervalContext, MathError};

use purrdf_core::SmallVec;

use super::{ChunkObserver, SolveError};

type CoefficientArray<I> = SmallVec<[I; 9]>;

/// Generate sqrt(1+z) and 1/(1+d sqrt(1+z)) coefficients from their equations.
/// The caller admits both complete arrays before entering this function.
pub(super) fn generate<I: CertifiedInterval>(
    d: &I,
    order: usize,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<(CoefficientArray<I>, CoefficientArray<I>), SolveError> {
    let one = I::from_i64(1, math)?;
    let zero = I::from_i64(0, math)?;
    if d.upper() < zero.lower() {
        return Err(
            MathError::Domain("reciprocal coefficient parameter must be nonnegative").into(),
        );
    }
    // Both callers prove the actual parameter is nonnegative. Intersect with
    // that analytic domain if outward arithmetic straddles its boundary.
    let d = I::from_bounds(d.lower().max(zero.lower()).clone(), d.upper().clone(), math)?;
    let capacity = order.checked_add(1).ok_or(MathError::WorkspaceExhausted)?;
    let mut square_root: SmallVec<[I; 9]> = SmallVec::with_capacity(capacity);
    let mut reciprocal: SmallVec<[I; 9]> = SmallVec::with_capacity(capacity);
    square_root.push(one.clone());
    let denominator = one.add(&d, math)?;
    reciprocal.push(one.div(&denominator, math)?);
    let mut items = 0_u8;
    for n in 1..=order {
        let n_integer = i64::try_from(n).map_err(|_| MathError::WorkExhausted)?;
        let twice_n = n_integer.checked_mul(2).ok_or(MathError::WorkExhausted)?;
        let numerator = I::from_i64(
            3_i64.checked_sub(twice_n).ok_or(MathError::WorkExhausted)?,
            math,
        )?;
        let divisor = I::from_i64(twice_n, math)?;
        let coefficient = square_root[n - 1]
            .mul(&numerator, math)?
            .div(&divisor, math)?;
        // Every binomial coefficient has magnitude at most one.
        square_root.push(unit_coefficient(&coefficient, &one, math)?);
        let mut convolution = I::from_i64(0, math)?;
        for k in 1..=n {
            convolution = convolution.add(&square_root[k].mul(&reciprocal[n - k], math)?, math)?;
            items += 1;
            if items == 64 {
                chunks.finish_interval(math)?;
                items = 0;
            }
        }
        let coefficient = d
            .mul(&convolution, math)?
            .neg(math)?
            .div(&denominator, math)?;
        // For d>=0 this reciprocal is holomorphic on |z|<1, with modulus<=1.
        // Taking Cauchy radii up to one proves |coefficient|<=1 independently
        // of arithmetic width and prevents retained magnitudes from growing.
        reciprocal.push(unit_coefficient(&coefficient, &one, math)?);
        chunks.finish_interval(math)?;
    }
    Ok((square_root, reciprocal))
}

fn unit_coefficient<I: CertifiedInterval>(
    value: &I,
    one: &I,
    math: &mut IntervalContext<'_>,
) -> Result<I, MathError> {
    let lower = value.lower().max(&one.upper().negated()).clone();
    let upper = value.upper().min(one.upper()).clone();
    if lower > upper {
        return Err(MathError::PrecisionExhausted);
    }
    I::from_bounds(lower, upper, math)
}
