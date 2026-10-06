// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently generated Taylor continuation of the transverse Mercator map.
//!
//! Write q for isometric latitude and m for meridian distance. The conformal
//! map is m(phi(q+i*lambda)), with
//! dphi/dq=cos(phi)*(1-e²*sin²(phi))/(1-e²), dm/dq=a*cos(phi)/sqrt(1-e²*sin²(phi)).
//! The coefficient recurrences below differentiate those equations. They use no
//! published coefficient arrays. Native zoning bounds |lambda|<=3 degrees.
//!
//! For e²<=0.01, the complex ODE stays within |phi-phi0|<3/4 on |q-q0|<=1/2:
//! |dphi/dq|<=cosh(3/4)*(1+.01*cosh²(3/4))/.99<3/2. Continuation therefore cannot
//! leave that ball. There |dm/dphi|<11*a/10; the real meridian is bounded by
//! 2*a, so |m|<3*a. Cauchy's remainder is at most
//! 3*a*(2*|lambda|)^(n+1)/(1-2*|lambda|). Outward intervals include every
//! coefficient and evaluation error, and that analytic tail is added explicitly.

use super::OperationCoordinates;

use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, MathError, RootJacobian2, with_taylor_workspace_observed,
};

type TransverseEvaluation = (OperationCoordinates, Option<RootJacobian2>);

use super::{OperationPoint, radians, rational_fields};
use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, fixed_from_rat};
use crate::{GeoError, PreparedEllipsoid, Rat};

/// Declared native zone family; no zone is inferred from a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoneFamily {
    /// Three-degree Gauss–Krüger, central meridian `3*zone` degrees.
    GaussKruger3,
    /// Six-degree Gauss–Krüger, central meridian `6*zone-3` degrees.
    GaussKruger6,
    /// Six-degree UTM, central meridian `6*zone-183` degrees.
    Utm,
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_xsd::math::MathLimits;

    #[test]
    fn central_meridian_jacobian_matches_independent_normal_metric_factors() {
        let parameters = TransverseMercator {
            ellipsoid: PreparedEllipsoid::cgcs2000(),
            family: ZoneFamily::GaussKruger6,
            zone: 20,
            central_meridian: Rat::from_i64(117),
            scale: Rat::one(),
            false_easting: Rat::from_i64(500_000),
            false_northing: Rat::zero(),
            hemisphere: Hemisphere::North,
            zone_prefix: false,
        };
        for latitude in [0, 35, 80] {
            let source = OperationPoint {
                x: Rat::from_i64(117),
                y: Rat::from_i64(latitude),
                z: None,
                epoch: None,
            };
            let mut math = CoordinateMath::new(MathLimits {
                precision_bits: 128,
                max_work: 262_144,
                max_workspace_bytes: 64 * 1024 * 1024,
            })
            .unwrap();
            let proof = parameters
                .differential_enclosure(&source, &mut math)
                .unwrap();
            let factor = FixedInterval::pi(&mut math)
                .unwrap()
                .div(&FixedInterval::from_i64(180, &mut math).unwrap(), &mut math)
                .unwrap();
            let phi = fixed_from_rat(&source.y, &mut math)
                .unwrap()
                .mul(&factor, &mut math)
                .unwrap();
            let (sine, cosine) = phi.sin_cos(&mut math).unwrap();
            let one = FixedInterval::from_i64(1, &mut math).unwrap();
            let e2 =
                fixed_from_rat(&parameters.ellipsoid.eccentricity_squared(), &mut math).unwrap();
            let w2 = one
                .sub(
                    &e2.mul(&sine.square(&mut math).unwrap(), &mut math).unwrap(),
                    &mut math,
                )
                .unwrap();
            let n = fixed_from_rat(parameters.ellipsoid.semimajor(), &mut math)
                .unwrap()
                .div(&w2.sqrt(&mut math).unwrap(), &mut math)
                .unwrap();
            let easting = n
                .mul(&cosine, &mut math)
                .unwrap()
                .mul(&factor, &mut math)
                .unwrap();
            let northing = n
                .mul(&one.sub(&e2, &mut math).unwrap(), &mut math)
                .unwrap()
                .div(&w2, &mut math)
                .unwrap()
                .mul(&factor, &mut math)
                .unwrap();
            for (actual, expected) in [
                (&proof.jacobian[0][0], easting),
                (&proof.jacobian[1][1], northing),
            ] {
                assert!(actual.lower() <= expected.upper() && actual.upper() >= expected.lower());
            }
            for off_diagonal in [&proof.jacobian[0][1], &proof.jacobian[1][0]] {
                assert!(
                    off_diagonal.lower() <= &purrdf_xsd::BigInt::zero()
                        && off_diagonal.upper() >= &purrdf_xsd::BigInt::zero()
                );
            }
        }
    }
}

/// Explicit hemisphere declaration for a transverse Mercator profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hemisphere {
    /// Nonnegative geographic latitude.
    North,
    /// Nonpositive geographic latitude.
    South,
}

/// Complete metre/degree declarations for one native transverse Mercator zone.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransverseMercator {
    /// Explicit ellipsoid, independent of the datum realization.
    pub ellipsoid: PreparedEllipsoid,
    /// Caller-selected zone family.
    pub family: ZoneFamily,
    /// Caller-declared zone number, validated against the central meridian.
    pub zone: u16,
    /// Exact central meridian in degrees; never inferred.
    pub central_meridian: Rat,
    /// Explicit positive central scale factor.
    pub scale: Rat,
    /// Explicit false easting in metres.
    pub false_easting: Rat,
    /// Explicit false northing in metres.
    pub false_northing: Rat,
    /// Declared hemisphere, without automatic selection.
    pub hemisphere: Hemisphere,
    /// Add `zone*1,000,000` to the easting when true.
    pub zone_prefix: bool,
}

/// Refinement evidence for a transverse Mercator image and differential.
/// Image rows are easting/northing in metres; Jacobian columns are original
/// longitude/latitude in degrees. These are enclosures, not output grids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionDifferential {
    /// Complete metre image enclosure of the supplied original point.
    pub image: [FixedInterval; 2],
    /// Complete first-derivative enclosures in metres per degree.
    pub jacobian: RootJacobian2,
}

impl TransverseMercator {
    /// Build integer-arithmetic differential evidence under the supplied
    /// precision/work/workspace admission. Coefficients and derivative tails
    /// come from the same original holomorphic recurrence as the projection.
    ///
    /// # Errors
    /// Refuses an invalid declared projection/source domain, a singular polar
    /// differential, and incomplete arithmetic or resource admission.
    pub fn differential_enclosure(
        &self,
        point: &OperationPoint,
        math: &mut CoordinateMath,
    ) -> Result<ProjectionDifferential, MathError> {
        self.validate()
            .and_then(|()| self.validate_point(point))
            .map_err(|_| MathError::Domain("invalid declared projection or source point"))?;
        let coordinates = [
            fixed_from_rat(&point.x, math)?,
            fixed_from_rat(&point.y, math)?,
        ];
        let (image, jacobian) = self.evaluate_differential(
            point,
            &coordinates,
            true,
            math,
            &mut WorkProgress::new(None),
        )?;
        Ok(ProjectionDifferential {
            image: [image[0].clone(), image[1].clone()],
            jacobian,
        })
    }
    pub(super) fn validate(&self) -> Result<(), GeoError> {
        if self.scale <= Rat::zero() {
            return Err(GeoError::config(
                "transverse Mercator scale must be positive",
            ));
        }
        if self.ellipsoid.eccentricity_squared()
            > Rat::parse_decimal("0.01").expect("frozen decimal")
        {
            return Err(GeoError::config(
                "native transverse Mercator Taylor certificate requires squared eccentricity <=0.01",
            ));
        }
        let central = match self.family {
            ZoneFamily::GaussKruger3 => Rat::from_i64(3 * i64::from(self.zone)),
            ZoneFamily::GaussKruger6 => Rat::from_i64(6 * i64::from(self.zone) - 3),
            ZoneFamily::Utm => {
                if !(1..=60).contains(&self.zone) || self.zone_prefix {
                    return Err(GeoError::config(
                        "UTM requires zone 1..=60 and no zone prefix",
                    ));
                }
                Rat::from_i64(6 * i64::from(self.zone) - 183)
            }
        };
        if self.central_meridian != central
            || central < Rat::from_i64(-180)
            || central > Rat::from_i64(180)
        {
            return Err(GeoError::config(
                "declared transverse Mercator zone and meridian disagree",
            ));
        }
        Ok(())
    }
    pub(super) fn validate_point(&self, point: &OperationPoint) -> Result<(), GeoError> {
        crate::LonLat::new(point.x.clone(), point.y.clone())?;
        if (self.hemisphere == Hemisphere::North && point.y < Rat::zero())
            || (self.hemisphere == Hemisphere::South && point.y > Rat::zero())
        {
            return Err(GeoError::domain(
                "point contradicts the explicitly declared projection hemisphere",
            ));
        }
        if self.family == ZoneFamily::Utm
            && (point.y < Rat::from_i64(-80) || point.y > Rat::from_i64(84))
        {
            return Err(GeoError::domain("point is outside the UTM latitude domain"));
        }
        let half_width = if self.family == ZoneFamily::GaussKruger3 {
            Rat::parse_decimal("1.5").expect("frozen decimal")
        } else {
            Rat::from_i64(3)
        };
        if point.y.abs() != Rat::from_i64(90)
            && point.x.sub(&self.central_meridian).abs() > half_width
        {
            return Err(GeoError::domain(
                "point is outside the explicitly declared transverse Mercator zone",
            ));
        }
        Ok(())
    }
    pub(super) fn validate_interval(&self, coordinates: &[FixedInterval]) -> Result<(), MathError> {
        let (latitude_lower, latitude_upper) = exact_bounds(&coordinates[1]);
        let (minimum, maximum) = match (self.hemisphere, self.family) {
            (Hemisphere::North, ZoneFamily::Utm) => (Rat::zero(), Rat::from_i64(84)),
            (Hemisphere::South, ZoneFamily::Utm) => (Rat::from_i64(-80), Rat::zero()),
            (Hemisphere::North, _) => (Rat::zero(), Rat::from_i64(90)),
            (Hemisphere::South, _) => (Rat::from_i64(-90), Rat::zero()),
        };
        if latitude_lower > maximum || latitude_upper < minimum {
            return Err(MathError::Domain(
                "intermediate coordinate outside declared projection latitude/hemisphere",
            ));
        }
        if latitude_lower < minimum || latitude_upper > maximum {
            return Err(MathError::PrecisionExhausted);
        }
        if latitude_lower == latitude_upper && latitude_lower.abs() == Rat::from_i64(90) {
            return Ok(());
        }
        let half_width = if self.family == ZoneFamily::GaussKruger3 {
            Rat::parse_decimal("1.5").expect("frozen decimal")
        } else {
            Rat::from_i64(3)
        };
        let (longitude_lower, longitude_upper) = exact_bounds(&coordinates[0]);
        let minimum = self.central_meridian.sub(&half_width);
        let maximum = self.central_meridian.add(&half_width);
        if longitude_lower > maximum || longitude_upper < minimum {
            return Err(MathError::Domain(
                "intermediate coordinate outside declared projection zone",
            ));
        }
        if longitude_lower < minimum || longitude_upper > maximum {
            return Err(MathError::PrecisionExhausted);
        }
        Ok(())
    }

    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        fields.push(vec![
            match self.family {
                ZoneFamily::GaussKruger3 => 0,
                ZoneFamily::GaussKruger6 => 1,
                ZoneFamily::Utm => 2,
            },
            u8::from(self.hemisphere == Hemisphere::South),
            u8::from(self.zone_prefix),
        ]);
        fields.push(self.zone.to_be_bytes().to_vec());
        for value in [
            self.ellipsoid.semimajor(),
            self.ellipsoid.inverse_flattening(),
            &self.central_meridian,
            &self.scale,
            &self.false_easting,
            &self.false_northing,
        ] {
            rational_fields(value, fields);
        }
    }
    pub(super) fn evaluate(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        source_is_exact: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        Ok(self
            .evaluate_with_derivative(point, coordinates, source_is_exact, false, math, progress)?
            .0)
    }

    pub(super) fn evaluate_differential(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        source_is_exact: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, [[FixedInterval; 2]; 2]), MathError> {
        let (image, derivative) = self.evaluate_with_derivative(
            point,
            coordinates,
            source_is_exact,
            true,
            math,
            progress,
        )?;
        Ok((image, derivative.expect("requested projection derivative")))
    }

    #[allow(clippy::too_many_arguments)] // One value/derivative recurrence shares its admitted operands.
    fn evaluate_with_derivative(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        source_is_exact: bool,
        derivative: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<TransverseEvaluation, MathError> {
        if !source_is_exact {
            self.validate_interval(coordinates)?;
        }
        let (latitude_lower, latitude_upper) = exact_bounds(&coordinates[1]);
        let exact_pole = (source_is_exact && point.y.abs() == Rat::from_i64(90))
            || (latitude_lower == latitude_upper && latitude_lower.abs() == Rat::from_i64(90));
        let lambda = if exact_pole {
            FixedInterval::from_i64(0, math)?
        } else {
            radians(
                &coordinates[0].sub(&fixed_from_rat(&self.central_meridian, math)?, math)?,
                math,
            )?
        };
        let ratio = lambda
            .abs(math)?
            .mul(&FixedInterval::from_i64(2, math)?, math)?;
        if ratio.upper() >= FixedInterval::from_i64(1, math)?.lower() {
            return Err(MathError::Domain(
                "transverse Mercator continuation outside its certified chart",
            ));
        }
        let phi = radians(&coordinates[1], math)?;
        let (mut sine, mut cosine) = phi.sin_cos(math)?;
        if exact_pole {
            sine = FixedInterval::from_i64(i64::from(latitude_lower.signum()), math)?;
            cosine = FixedInterval::from_i64(0, math)?;
        }
        let e2 = fixed_from_rat(&self.ellipsoid.eccentricity_squared(), math)?;
        let one = FixedInterval::from_i64(1, math)?;
        let a = fixed_from_rat(self.ellipsoid.semimajor(), math)?;
        // Raised precision automatically increases the original Taylor order;
        // successful quantized output remains the same completed law.
        // Choose the smallest order whose *proved* Cauchy tail meets the
        // current refinement target. A precision retry tightens this target;
        // neither a heuristic difference nor a resource budget selects order.
        let target = FixedInterval::from_ratio(
            &purrdf_xsd::BigInt::from_i128(1),
            &purrdf_xsd::BigInt::from(
                crate::Int::one().shl(math.limits().precision_bits.saturating_sub(40)),
            ),
            math,
        )?;
        let mut power = one.clone();
        let mut order = 0;
        loop {
            power = power.mul(&ratio, math)?;
            let tail = power
                .mul(&a.mul(&FixedInterval::from_i64(3, math)?, math)?, math)?
                .div(&one.sub(&ratio, math)?, math)?;
            if order >= 8 && tail.upper() <= target.lower() {
                break;
            }
            order += 1;
            if order > math.limits().precision_bits as usize {
                return Err(MathError::PrecisionExhausted);
            }
        }
        self.series(
            point,
            coordinates,
            &lambda,
            &ratio,
            &sine,
            &cosine,
            &e2,
            &one,
            &a,
            order,
            derivative,
            math,
            progress,
        )
    }

    #[allow(clippy::too_many_arguments)] // Original ODE operands and admitted scratch remain explicit.
    fn series(
        &self,
        _point: &OperationPoint,
        coordinates: &[FixedInterval],
        lambda: &FixedInterval,
        ratio: &FixedInterval,
        sine: &FixedInterval,
        cosine: &FixedInterval,
        e2: &FixedInterval,
        one: &FixedInterval,
        a: &FixedInterval,
        order: usize,
        derivative: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<TransverseEvaluation, MathError> {
        with_taylor_workspace_observed(
            order,
            7,
            math,
            progress,
            |math, progress| progress.math_poll(math),
            |workspace, math, progress| {
                let phi_coefficients = workspace.constant(radians(&coordinates[1], math)?, math)?;
                let sin_coefficients = workspace.constant(sine.clone(), math)?;
                let cos_coefficients = workspace.constant(cosine.clone(), math)?;
                let squared_sin = workspace.constant(FixedInterval::from_i64(0, math)?, math)?;
                let w0 = one
                    .sub(&e2.mul(&sine.square(math)?, math)?, math)?
                    .sqrt(math)?;
                let square_root = workspace.constant(w0.clone(), math)?;
                let reciprocal = workspace.constant(one.div(&w0, math)?, math)?;
                let (latitude_lower, latitude_upper) = exact_bounds(&coordinates[1]);
                let lower_meridian = crate::geodesic::meridian_arc_observed(
                    &latitude_lower,
                    &self.ellipsoid,
                    math,
                    progress,
                )?;
                let upper_meridian = crate::geodesic::meridian_arc_observed(
                    &latitude_upper,
                    &self.ellipsoid,
                    math,
                    progress,
                )?;
                let meridian = workspace.constant(
                    FixedInterval::from_bounds(
                        lower_meridian.lower().clone(),
                        upper_meridian.upper().clone(),
                        math,
                    )?,
                    math,
                )?;
                let denominator = one.sub(e2, math)?;
                for n in 0..order {
                    progress.math_poll(math)?;
                    if n != 0 {
                        let mut sin_n = FixedInterval::from_i64(0, math)?;
                        let mut cos_n = FixedInterval::from_i64(0, math)?;
                        for j in 1..=n {
                            let weighted = workspace
                                .coefficient(phi_coefficients, j)?
                                .mul(&FixedInterval::from_i64(j as i64, math)?, math)?;
                            sin_n = sin_n.add(
                                &weighted
                                    .mul(workspace.coefficient(cos_coefficients, n - j)?, math)?,
                                math,
                            )?;
                            cos_n = cos_n.sub(
                                &weighted
                                    .mul(workspace.coefficient(sin_coefficients, n - j)?, math)?,
                                math,
                            )?;
                        }
                        let count = FixedInterval::from_i64(n as i64, math)?;
                        workspace.set_coefficient(
                            sin_coefficients,
                            n,
                            sin_n.div(&count, math)?,
                            math,
                        )?;
                        workspace.set_coefficient(
                            cos_coefficients,
                            n,
                            cos_n.div(&count, math)?,
                            math,
                        )?;
                    }
                    let mut ss_n = FixedInterval::from_i64(0, math)?;
                    for j in 0..=n {
                        ss_n = ss_n.add(
                            &workspace
                                .coefficient(sin_coefficients, j)?
                                .mul(workspace.coefficient(sin_coefficients, n - j)?, math)?,
                            math,
                        )?;
                    }
                    workspace.set_coefficient(squared_sin, n, ss_n, math)?;
                    if n != 0 {
                        let mut value = e2
                            .mul(workspace.coefficient(squared_sin, n)?, math)?
                            .neg(math)?;
                        for j in 1..n {
                            value = value.sub(
                                &workspace
                                    .coefficient(square_root, j)?
                                    .mul(workspace.coefficient(square_root, n - j)?, math)?,
                                math,
                            )?;
                        }
                        workspace.set_coefficient(
                            square_root,
                            n,
                            value.div(&w0.mul(&FixedInterval::from_i64(2, math)?, math)?, math)?,
                            math,
                        )?;
                        let mut inverse = FixedInterval::from_i64(0, math)?;
                        for j in 1..=n {
                            inverse = inverse.sub(
                                &workspace
                                    .coefficient(square_root, j)?
                                    .mul(workspace.coefficient(reciprocal, n - j)?, math)?,
                                math,
                            )?;
                        }
                        workspace.set_coefficient(reciprocal, n, inverse.div(&w0, math)?, math)?;
                    }
                    let mut phi_derivative = workspace.coefficient(cos_coefficients, n)?.clone();
                    let mut m_derivative = FixedInterval::from_i64(0, math)?;
                    let mut correction = FixedInterval::from_i64(0, math)?;
                    for j in 0..=n {
                        correction = correction.add(
                            &workspace
                                .coefficient(cos_coefficients, j)?
                                .mul(workspace.coefficient(squared_sin, n - j)?, math)?,
                            math,
                        )?;
                        m_derivative = m_derivative.add(
                            &workspace
                                .coefficient(cos_coefficients, j)?
                                .mul(workspace.coefficient(reciprocal, n - j)?, math)?,
                            math,
                        )?;
                    }
                    phi_derivative = phi_derivative
                        .sub(&e2.mul(&correction, math)?, math)?
                        .div(&denominator, math)?;
                    let count = FixedInterval::from_i64((n + 1) as i64, math)?;
                    workspace.set_coefficient(
                        phi_coefficients,
                        n + 1,
                        phi_derivative.div(&count, math)?,
                        math,
                    )?;
                    workspace.set_coefficient(
                        meridian,
                        n + 1,
                        a.mul(&m_derivative, math)?.div(&count, math)?,
                        math,
                    )?;
                }
                let mut northing = workspace.coefficient(meridian, 0)?.clone();
                let mut easting = FixedInterval::from_i64(0, math)?;
                let mut lambda_power = one.clone();
                for n in 1..=order {
                    let coefficient = workspace.coefficient(meridian, n)?;
                    lambda_power = lambda_power.mul(lambda, math)?;
                    let mut term = coefficient.mul(&lambda_power, math)?;
                    if n % 4 >= 2 {
                        term = term.neg(math)?;
                    }
                    if n % 2 == 0 {
                        northing = northing.add(&term, math)?;
                    } else {
                        easting = easting.add(&term, math)?;
                    }
                }
                let mut tail = one.clone();
                for _ in 0..=order {
                    tail = tail.mul(ratio, math)?;
                }
                tail = tail
                    .mul(&a.mul(&FixedInterval::from_i64(3, math)?, math)?, math)?
                    .div(&one.sub(ratio, math)?, math)?;
                let padding =
                    FixedInterval::from_bounds(tail.upper().negated(), tail.upper().clone(), math)?;
                let scale = fixed_from_rat(&self.scale, math)?;
                let prefix = if self.zone_prefix {
                    Rat::from_i64(i64::from(self.zone) * 1_000_000)
                } else {
                    Rat::zero()
                };
                let jacobian = if derivative {
                    let mut real = FixedInterval::from_i64(0, math)?;
                    let mut imaginary = FixedInterval::from_i64(0, math)?;
                    let mut power = one.clone();
                    for n in 1..=order {
                        let coefficient = workspace.coefficient(meridian, n)?;
                        let mut term = coefficient
                            .mul(&FixedInterval::from_i64(n as i64, math)?, math)?
                            .mul(&power, math)?;
                        if (n - 1) % 4 >= 2 {
                            term = term.neg(math)?;
                        }
                        if n.is_multiple_of(2) {
                            imaginary = imaginary.add(&term, math)?;
                        } else {
                            real = real.add(&term, math)?;
                        }
                        power = power.mul(lambda, math)?;
                    }
                    // Sum_{n>N} n*3a*2^n*|lambda|^(n-1) exactly bounds
                    // the derivative tail of the same holomorphic continuation.
                    let mut power = one.clone();
                    for _ in 0..order {
                        power = power.mul(ratio, math)?;
                    }
                    let derivative_tail = a
                        .mul(&FixedInterval::from_i64(6, math)?, math)?
                        .mul(&power, math)?
                        .mul(
                            &FixedInterval::from_i64((order + 1) as i64, math)?.sub(
                                &ratio.mul(&FixedInterval::from_i64(order as i64, math)?, math)?,
                                math,
                            )?,
                            math,
                        )?
                        .div(&one.sub(ratio, math)?.square(math)?, math)?;
                    let guard = FixedInterval::from_bounds(
                        derivative_tail.upper().negated(),
                        derivative_tail.upper().clone(),
                        math,
                    )?;
                    real = real.add(&guard, math)?;
                    imaginary = imaginary.add(&guard, math)?;
                    let degrees_factor =
                        FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
                    let q_prime = one.sub(e2, math)?.div(
                        &one.sub(&e2.mul(&sine.square(math)?, math)?, math)?
                            .mul(cosine, math)?,
                        math,
                    )?;
                    let factor = degrees_factor.mul(&scale, math)?;
                    Some([
                        [
                            real.mul(&factor, math)?,
                            imaginary.mul(&q_prime, math)?.mul(&factor, math)?,
                        ],
                        [
                            imaginary.neg(math)?.mul(&factor, math)?,
                            real.mul(&q_prime, math)?.mul(&factor, math)?,
                        ],
                    ])
                } else {
                    None
                };
                let mut output: OperationCoordinates = purrdf_core::smallvec![
                    easting.add(&padding, math)?.mul(&scale, math)?.add(
                        &fixed_from_rat(&self.false_easting.add(&prefix), math)?,
                        math,
                    )?,
                    northing
                        .add(&padding, math)?
                        .mul(&scale, math)?
                        .add(&fixed_from_rat(&self.false_northing, math)?, math)?,
                ];
                if let Some(z) = coordinates.get(2) {
                    output.push(z.clone());
                }
                Ok((output, jacobian))
            },
        )
    }
}
