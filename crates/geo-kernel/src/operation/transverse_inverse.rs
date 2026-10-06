// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Global native-zone inversion using the shared original image/differential.

use super::OperationCoordinates;

use super::{Hemisphere, OperationPoint, OperationSolverLimits, TransverseMercator, ZoneFamily};
use crate::Rat;
use crate::context::WorkProgress;
use crate::numerical::{fixed_from_rat, isolate_roots_inline};
use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, MathError, RootBox2, RootIsolationLimits, RootJacobian2,
    RootSystem2,
};

pub(super) fn inverse(
    parameters: &TransverseMercator,
    point: &OperationPoint,
    coordinates: &[FixedInterval],
    limits: OperationSolverLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationCoordinates, MathError> {
    let half_width = if parameters.family == ZoneFamily::GaussKruger3 {
        Rat::parse_decimal("1.5").expect("frozen decimal")
    } else {
        Rat::from_i64(3)
    };
    let west = parameters
        .central_meridian
        .sub(&half_width)
        .max(Rat::from_i64(-180));
    let east = parameters
        .central_meridian
        .add(&half_width)
        .min(Rat::from_i64(180));
    let (south, north) = match (parameters.hemisphere, parameters.family) {
        (Hemisphere::North, ZoneFamily::Utm) => (0, 84),
        (Hemisphere::South, ZoneFamily::Utm) => (-80, 0),
        (Hemisphere::North, _) => (0, 90),
        (Hemisphere::South, _) => (-90, 0),
    };
    // The generated holomorphic meridian has |m(z)|<=3a on |z-q|<=1/2.
    // Northing's even terms beyond m(phi) therefore have total magnitude at
    // most 3a*q²/(1-q²), q=2|lambda|max. Global normal metric factors enclose
    // M, so this complete northing band restricts latitude without discarding
    // any admissible inverse. It is independent of local Newton guesses.
    let pi = FixedInterval::pi(math)?;
    let q = fixed_from_rat(&half_width, math)?
        .mul(&pi, math)?
        .div(&FixedInterval::from_i64(90, math)?, math)?;
    let q2 = q.square(math)?;
    let tail = fixed_from_rat(parameters.ellipsoid.semimajor(), math)?
        .mul(&FixedInterval::from_i64(3, math)?, math)?
        .mul(&q2, math)?
        .div(&FixedInterval::from_i64(1, math)?.sub(&q2, math)?, math)?;
    let northing = coordinates[1]
        .sub(&fixed_from_rat(&parameters.false_northing, math)?, math)?
        .div(&fixed_from_rat(&parameters.scale, math)?, math)?;
    let meridian = northing.add(
        &FixedInterval::from_bounds(tail.upper().negated(), tail.upper().clone(), math)?,
        math,
    )?;
    let (minimum, maximum) = parameters.ellipsoid.normal_metric_bounds();
    let factor = FixedInterval::from_bounds(
        fixed_from_rat(minimum.exact(), math)?.lower().clone(),
        fixed_from_rat(maximum.exact(), math)?.upper().clone(),
        math,
    )?;
    let latitude = meridian
        .div(&factor, math)?
        .mul(&FixedInterval::from_i64(180, math)?, math)?
        .div(&pi, math)?;
    let declared_latitude = FixedInterval::from_bounds(
        FixedInterval::from_i64(south, math)?.lower().clone(),
        FixedInterval::from_i64(north, math)?.upper().clone(),
        math,
    )?;
    let Some(latitude) = latitude.intersection(&declared_latitude, math)? else {
        return Err(MathError::Domain(
            "projected northing is outside its declared source domain",
        ));
    };
    // The odd continuation is m'(q)*lambda plus the independently bounded
    // odd tail. On a nonpolar latitude band its first coefficient is positive,
    // allowing an equally complete easting restriction of longitude.
    let declared_longitude = FixedInterval::from_bounds(
        fixed_from_rat(&west, math)?.lower().clone(),
        fixed_from_rat(&east, math)?.upper().clone(),
        math,
    )?;
    let phi = super::radians(&latitude, math)?;
    let (sine, cosine) = phi.sin_cos_range(math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let w = one
        .sub(
            &fixed_from_rat(&parameters.ellipsoid.eccentricity_squared(), math)?
                .mul(&sine.square(math)?, math)?,
            math,
        )?
        .sqrt(math)?;
    let first = fixed_from_rat(parameters.ellipsoid.semimajor(), math)?
        .mul(&cosine, math)?
        .div(&w, math)?;
    let longitude = if first.lower() > one.sub(&one, math)?.upper() {
        let odd_tail = tail.mul(&q, math)?;
        let prefix = if parameters.zone_prefix {
            Rat::from_i64(i64::from(parameters.zone) * 1_000_000)
        } else {
            Rat::zero()
        };
        let easting = coordinates[0]
            .sub(
                &fixed_from_rat(&parameters.false_easting.add(&prefix), math)?,
                math,
            )?
            .div(&fixed_from_rat(&parameters.scale, math)?, math)?;
        let band = easting
            .add(
                &FixedInterval::from_bounds(
                    odd_tail.upper().negated(),
                    odd_tail.upper().clone(),
                    math,
                )?,
                math,
            )?
            .div(&first, math)?
            .mul(&FixedInterval::from_i64(180, math)?, math)?
            .div(&pi, math)?
            .add(&fixed_from_rat(&parameters.central_meridian, math)?, math)?;
        let Some(longitude) = band.intersection(&declared_longitude, math)? else {
            return Err(MathError::Domain(
                "projected easting is outside its declared source domain",
            ));
        };
        longitude
    } else {
        declared_longitude
    };
    let domain = [longitude, latitude];
    let mut system = ProjectionRoots {
        parameters,
        point,
        target: coordinates,
        quantize: limits.quantize_inverse,
        progress,
    };
    let roots = isolate_roots_inline(
        domain,
        RootIsolationLimits {
            max_depth: limits.subdivisions,
            max_refinements: limits.iterations,
        },
        &mut system,
        math,
    )?;
    if roots.is_empty() {
        return Err(MathError::Domain(
            "projected point has no inverse in its declared zone and hemisphere",
        ));
    }
    if roots.len() != 1 {
        return Err(MathError::AmbiguousRoots { roots: roots.len() });
    }
    let mut output = OperationCoordinates::from_array(roots[0].enclosure().clone());
    if let Some(z) = coordinates.get(2) {
        output.push(z.clone());
    }
    Ok(output)
}

struct ProjectionRoots<'input, 'progress, 'observer> {
    parameters: &'input TransverseMercator,
    point: &'input OperationPoint,
    target: &'input [FixedInterval],
    quantize: bool,
    progress: &'progress mut WorkProgress<'observer>,
}

impl RootSystem2 for ProjectionRoots<'_, '_, '_> {
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        let image = self
            .parameters
            .evaluate(self.point, domain, false, math, self.progress)?;
        Ok([
            image[0].sub(&self.target[0], math)?,
            image[1].sub(&self.target[1], math)?,
        ])
    }
    fn jacobian(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError> {
        Ok(self
            .parameters
            .evaluate_differential(self.point, domain, false, math, self.progress)?
            .1)
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        if self.quantize {
            let zero = FixedInterval::from_i64(0, math)?;
            for coordinate in enclosure {
                if !purrdf_xsd::math::inverse_coordinate_complete(
                    coordinate, &zero, 15, true, math,
                )? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let jacobian = self.jacobian(enclosure, math)?;
        purrdf_xsd::math::root_enclosure_complete(
            enclosure,
            &[self.target[0].clone(), self.target[1].clone()],
            &jacobian,
            15,
            self.quantize,
            math,
        )
    }
    fn poll(&mut self, math: &CoordinateMath) -> Result<(), MathError> {
        self.progress.math_poll(math)
    }
}

pub(super) fn residual(
    parameters: &TransverseMercator,
    target: &[FixedInterval],
    output: &[Rat],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(), MathError> {
    let source = OperationPoint {
        x: output[0].clone(),
        y: output[1].clone(),
        z: output.get(2).cloned(),
        epoch: None,
    };
    parameters
        .validate_point(&source)
        .map_err(|_| MathError::Domain("rounded inverse outside its declared source zone"))?;
    let coordinates = [
        fixed_from_rat(&source.x, math)?,
        fixed_from_rat(&source.y, math)?,
    ];
    let image = parameters.evaluate(&source, &coordinates, true, math, progress)?;
    let norm = image
        .iter()
        .zip(target)
        .take(2)
        .try_fold(
            FixedInterval::from_i64(0, math)?,
            |sum, (actual, expected)| sum.add(&actual.sub(expected, math)?.square(math)?, math),
        )?
        .sqrt(math)?;
    if norm.upper() <= super::decimal("0.000001", math)?.lower() {
        Ok(())
    } else {
        Err(MathError::PrecisionExhausted)
    }
}
