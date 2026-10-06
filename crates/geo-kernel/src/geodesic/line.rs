// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one prepared auxiliary line and direct-distance bracket contraction.

use super::{ChunkObserver, IntegralCoefficients, PreparedGeodesic, SolveError, reduced_latitude};
use crate::numerical::fixed_from_rat;
use crate::{LonLat, Rat};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

#[derive(Clone, Debug)]
pub(super) struct LineState {
    pub(super) a: FixedInterval,
    pub(super) b: FixedInterval,
    pub(super) c: FixedInterval,
    pub(super) e2: FixedInterval,
    pub(super) q: FixedInterval,
    pub(super) radians_per_degree: FixedInterval,
    pub(super) sin0: FixedInterval,
    pub(super) cos0: FixedInterval,
    pub(super) sigma1: FixedInterval,
    pub(super) omega_start: Option<FixedInterval>,
    pub(super) coefficients: std::sync::Arc<IntegralCoefficients>,
}

impl LineState {
    #[allow(clippy::too_many_arguments)] // Original line source, immutable equivalent tables and checked scratch.
    pub(super) fn new_with_coefficients(
        start: &LonLat,
        azimuth: &FixedInterval,
        exact_azimuth: Option<&Rat>,
        prepared: &PreparedGeodesic,
        coefficients: Option<&std::sync::Arc<IntegralCoefficients>>,
        order: usize,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<Self, SolveError> {
        // Inverse proof angles may use the equivalent signed representative
        // (for example -90 for a westward equator). Preserve exact quarter-turn
        // decisions independently of that representative's spelling.
        let canonical_azimuth = exact_azimuth.map(|angle| super::direct::normalize(angle, 0));
        let exact_azimuth = canonical_azimuth.as_ref();
        let one = FixedInterval::from_i64(1, math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let pi = FixedInterval::pi(math)?;
        let radians_per_degree = pi.div(&FixedInterval::from_i64(180, math)?, math)?;
        let a = fixed_from_rat(prepared.reference.ellipsoid().semimajor(), math)?;
        let b = fixed_from_rat(prepared.reference.ellipsoid().semiminor(), math)?;
        let c = b.div(&a, math)?;
        let e2 = fixed_from_rat(&prepared.reference.ellipsoid().eccentricity_squared(), math)?;
        let ep2 = e2.div(&c.square(math)?, math)?;
        let angle = azimuth.mul(&radians_per_degree, math)?;
        let (sin_alpha, cos_alpha) = if exact_azimuth.is_some_and(Rat::is_zero) {
            (zero.clone(), one)
        } else if exact_azimuth == Some(&Rat::from_i64(90)) {
            (one, zero.clone())
        } else if exact_azimuth == Some(&Rat::from_i64(180)) {
            (zero.clone(), one.neg(math)?)
        } else if exact_azimuth == Some(&Rat::from_i64(270)) {
            (one.neg(math)?, zero.clone())
        } else {
            angle.sin_cos(math)?
        };
        let (sin_beta, cos_beta) =
            reduced_latitude(start.latitude(), &c, &radians_per_degree, math)?;
        let pole = start.is_pole();
        let sin0 = if pole {
            zero.clone()
        } else {
            sin_alpha.mul(&cos_beta, math)?
        };
        let x1 = cos_alpha.mul(&cos_beta, math)?;
        // cos²(alpha0)=x1²+sin²(beta1) avoids cancellation at an equatorial line.
        let cos0 = x1
            .square(math)?
            .add(&sin_beta.square(math)?, math)?
            .sqrt(math)?;
        let sigma1 = if pole {
            let half_pi = pi.div(&FixedInterval::from_i64(2, math)?, math)?;
            if start.latitude() < &Rat::zero() {
                half_pi.neg(math)?
            } else {
                half_pi
            }
        } else if start.latitude().is_zero()
            && exact_azimuth
                .is_some_and(|angle| angle == &Rat::from_i64(90) || angle == &Rat::from_i64(270))
        {
            zero
        } else {
            FixedInterval::atan2(&sin_beta, &x1, math)?
        };
        let omega_start = if sin0.lower().is_zero() && sin0.upper().is_zero() {
            None
        } else {
            Some(super::arc_view::omega_lift(&sigma1, &sin0, math)?)
        };
        let q = ep2.mul(&cos0.square(math)?, math)?;
        let coefficients = if let Some(coefficients) = coefficients.or_else(|| {
            prepared
                .preparation
                .as_ref()
                .and_then(|cache| cache.fixed_shared(math.limits().precision_bits, order))
        }) {
            coefficients.clone()
        } else {
            std::sync::Arc::new(IntegralCoefficients::new(&c, order, chunks, math)?)
        };
        Ok(Self {
            a,
            b,
            c,
            e2,
            q,
            radians_per_degree,
            sin0,
            cos0,
            sigma1,
            omega_start,
            coefficients,
        })
    }

    pub(super) fn initial_sigma(
        &self,
        distance: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<(FixedInterval, FixedInterval), MathError> {
        // For q=0, ds/dsigma=b identically. Otherwise ds/dsigma lies in
        // [b,a] on the complete oblate line; the hull admits both forward
        // and signed continuation distances.
        let displacement = distance.div(&self.b, math)?;
        let displacement = if self.q.is_exact_zero() {
            displacement
        } else {
            displacement.hull(&distance.div(&self.a, math)?, math)?
        };
        let phase = self.sigma1.add(&displacement, math)?;
        Ok((
            FixedInterval::from_bounds(phase.lower().clone(), phase.lower().clone(), math)?,
            FixedInterval::from_bounds(phase.upper().clone(), phase.upper().clone(), math)?,
        ))
    }

    pub(super) fn contract_sigma(
        &self,
        distance: &FixedInterval,
        lower: &mut FixedInterval,
        upper: &mut FixedInterval,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<bool, SolveError> {
        let previous_lower = lower.lower().clone();
        let previous_upper = upper.upper().clone();
        chunks.finish(math)?;
        let sigma = lower
            .add(upper, math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?;
        let propagated = self
            .coefficients
            .evaluate(&self.sigma1, &sigma, &self.q, chunks, math)?
            .distance
            .mul(&self.b, math)?;
        // The mean-value theorem encloses every solution in the Newton image
        // regardless of the midpoint residual's sign. The complete oblate
        // derivative ds/dsigma lies in [b,a], so its contraction is available
        // before the midpoint enclosure reaches the requested distance.
        let derivative =
            FixedInterval::from_bounds(self.b.lower().clone(), self.a.upper().clone(), math)?;
        let image = sigma.sub(
            &propagated.sub(distance, math)?.div(&derivative, math)?,
            math,
        )?;
        let next_lower = image.lower().max(lower.lower()).clone();
        let next_upper = image.upper().min(upper.upper()).clone();
        if next_lower > next_upper {
            return Err(MathError::PrecisionExhausted.into());
        }
        *lower = FixedInterval::from_bounds(next_lower.clone(), next_lower, math)?;
        *upper = FixedInterval::from_bounds(next_upper.clone(), next_upper, math)?;
        Ok(lower.lower() != &previous_lower || upper.upper() != &previous_upper)
    }

    pub(super) fn sigma_enclosure(
        &self,
        distance: &FixedInterval,
        iterations: u32,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<FixedInterval, SolveError> {
        let (mut lower, mut upper) = self.initial_sigma(distance, math)?;
        if self.q.is_exact_zero() {
            return Ok(FixedInterval::from_bounds(
                lower.lower().clone(),
                upper.upper().clone(),
                math,
            )?);
        }
        let source_width = distance.width(math)?.div(&self.b, math)?;
        // Retain sixteen guard bits for outward arithmetic; the remaining
        // phase enclosure is fine enough for the frozen angular output grid.
        // Source uncertainty remains separate and is never contracted away.
        let exponent = i32::try_from(math.limits().precision_bits.saturating_sub(16))
            .map_err(|_| MathError::PrecisionExhausted)?;
        let guard = FixedInterval::power_of_two(-exponent, math)?;
        let maximum_width = source_width.add(&guard, math)?;
        for _ in 0..iterations {
            let contracted = self.contract_sigma(distance, &mut lower, &mut upper, chunks, math)?;
            let enclosure =
                FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)?;
            if enclosure.width(math)?.upper() <= maximum_width.lower() {
                return Ok(enclosure);
            }
            if !contracted {
                // The initial monotone distance bounds and every outward
                // Newton intersection enclose every admitted source law. A
                // retained inverse azimuth or start phase may limit further
                // contraction independently of distance width. Preserve this
                // proved enclosure; the image's rounding or geometric decision
                // determines whether its precision is sufficient.
                return Ok(enclosure);
            }
        }
        Err(SolveError::Iterations)
    }
}
