// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original antipodal seed from Karney's astroid tangent equation (53)--(54).
//!
//! The plane tangent equation `x/sin(alpha)+y/cos(alpha)+1=0` is
//! multiplied by sin(alpha)cos(alpha) and bisected on [pi/2,pi]. This
//! independently avoids quartic coefficient arrays and closed-form root code.
//! It is only a sample proposal; all solver endpoints remain certified by the
//! full ellipsoidal longitude law, so its approximation cannot certify an answer.

use super::{ChunkObserver, IntegralCoefficients, SolveError, SpherePair};
use purrdf_xsd::math::{CertifiedInterval, IntervalContext, MathError};

#[allow(clippy::too_many_arguments)] // The original ellipsoid integral supplies the conjugate astroid scale.
pub(super) fn seed<I: CertifiedInterval>(
    sphere: &SpherePair<I>,
    longitude: &I,
    c: &I,
    e2: &I,
    ep2: &I,
    coefficients: &IntegralCoefficients<I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Option<I>, SolveError> {
    let one = I::from_i64(1, math)?;
    let zero = I::from_i64(0, math)?;
    let pi = I::pi(math)?;
    let flattening = one.sub(c, math)?;
    if flattening.lower() <= zero.upper() {
        return Ok(None);
    }
    let rough_scale = flattening.mul(&pi, math)?.mul(&sphere.cos1, math)?;
    let four = I::from_i64(4, math)?;
    if longitude.lower() < pi.sub(&rough_scale.mul(&four, math)?, math)?.upper() {
        return Ok(None);
    }
    // At alpha1=pi/2 and beta2=-beta1, sigma runs from -pi/2 to
    // +pi/2 and sin(alpha0)=cos(beta1). Its exact longitude deficit is
    // e²*cos(beta1)*I3. This is Karney's f*pi*A3*cos(beta1) scale,
    // generated through the same original certified auxiliary integral.
    let vertex_q = ep2.mul(&sphere.sin1.square(math)?, math)?;
    let longitude_scale = e2.mul(&sphere.cos1, math)?.mul(
        &coefficients
            .evaluate_period_with(&vertex_q, chunks, math)?
            .longitude,
        math,
    )?;
    let latitude_scale = longitude_scale.mul(&sphere.cos1, math)?;
    if longitude_scale.lower() <= zero.upper() || latitude_scale.lower() <= zero.upper() {
        return Ok(None);
    }
    let beta1 = I::atan2(&sphere.sin1, &sphere.cos1, math)?;
    let beta2 = I::atan2(&sphere.sin2, &sphere.cos2, math)?;
    let beta_sum = if sphere.opposite_latitude {
        zero.clone()
    } else {
        beta1.add(&beta2, math)?
    };
    if longitude.lower() < pi.sub(&longitude_scale.mul(&four, math)?, math)?.upper()
        || beta_sum.abs(math)?.upper() > latitude_scale.mul(&four, math)?.lower()
    {
        return Ok(None);
    }
    let x = longitude.sub(&pi, math)?.div(&longitude_scale, math)?;
    let y = beta_sum.div(&latitude_scale, math)?;
    let two = I::from_i64(2, math)?;
    if sphere.opposite_latitude {
        let radicand = one.sub(&x.square(math)?, math)?;
        let radicand = super::nonnegative(&radicand, math).or_else(|error| match error {
            MathError::PrecisionExhausted => I::from_i64(0, math),
            error => Err(error),
        })?;
        return Ok(Some(I::atan2(
            &x.neg(math)?,
            &radicand.sqrt(math)?.neg(math)?,
            math,
        )?));
    }
    let mut lower = pi.div(&two, math)?;
    let mut upper = pi;
    for _ in 0..16 {
        let alpha = lower.add(&upper, math)?.div(&two, math)?;
        let (sin, cos) = alpha.sin_cos(math)?;
        let equation = x
            .mul(&cos, math)?
            .add(&y.mul(&sin, math)?, math)?
            .add(&sin.mul(&cos, math)?, math)?;
        if equation.upper() < zero.lower() {
            lower = alpha;
        } else if equation.lower() > zero.upper() {
            upper = alpha;
        } else {
            return Ok(Some(alpha));
        }
        chunks.finish_interval(math)?;
    }
    Ok(Some(lower.add(&upper, math)?.div(&two, math)?))
}
