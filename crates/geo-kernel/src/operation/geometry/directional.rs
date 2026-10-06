// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original differential of complete geographic/Cartesian coordinate images.
//! The three columns are longitude degrees, latitude degrees and actual height
//! metres. No height, epoch, datum operation or rotation law is inferred.

use super::{OperationCoordinates, OperationModel, OperationPoint, OperationSolverLimits};
use crate::PreparedEllipsoid;
use crate::context::WorkProgress;
use crate::numerical::fixed_from_rat;
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, Matrix3};

pub(super) struct DirectionalImage {
    pub(super) coordinates: OperationCoordinates,
    pub(super) derivative: Option<OperationCoordinates>,
}

pub(super) fn evaluate(
    model: &OperationModel,
    point: &OperationPoint,
    (coordinates, derivative): (&[FixedInterval], Option<&[FixedInterval]>),
    limits: OperationSolverLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<DirectionalImage>, MathError> {
    match model {
        OperationModel::Helmert(parameters) => {
            let (image, matrix) = parameters.evaluate_with_matrix(point, coordinates, math)?;
            with_matrix(image, &matrix, derivative, math).map(Some)
        }
        OperationModel::GeographicToGeocentric { ellipsoid } => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            with_matrix(
                image,
                &geocentric_jacobian(coordinates, ellipsoid, math)?,
                derivative,
                math,
            )
            .map(Some)
        }
        OperationModel::GeocentricToGeographic { ellipsoid } => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            // Longitude has no Cartesian differential on the polar axis, but
            // a curve proved to stay on that axis has a unique constant
            // canonical longitude and latitude. Its actual height derivative
            // is sign(Z)*Z'. This is a restricted exact image law, rather than
            // inversion of a singular longitude Jacobian.
            if coordinates[..2].iter().all(FixedInterval::is_exact_zero) {
                let derivative = derivative
                    .filter(|rate| {
                        rate.len() == 3 && rate[..2].iter().all(FixedInterval::is_exact_zero)
                    })
                    .map(|rate| {
                        Ok(purrdf_core::smallvec![
                            FixedInterval::from_i64(0, math)?,
                            FixedInterval::from_i64(0, math)?,
                            if coordinates[2].upper().is_negative() {
                                rate[2].neg(math)?
                            } else {
                                rate[2].clone()
                            },
                        ])
                    })
                    .transpose()?;
                return Ok(Some(DirectionalImage {
                    coordinates: image,
                    derivative,
                }));
            }
            let forward = geocentric_jacobian(&image, ellipsoid, math)?;
            with_matrix(
                image,
                &purrdf_xsd::math::invert_matrix3(&forward, math)?,
                derivative,
                math,
            )
            .map(Some)
        }
        _ => Ok(None),
    }
}

fn with_matrix(
    coordinates: OperationCoordinates,
    matrix: &Matrix3,
    derivative: Option<&[FixedInterval]>,
    math: &mut CoordinateMath,
) -> Result<DirectionalImage, MathError> {
    let derivative = derivative
        .map(|rate| {
            let rate: &[FixedInterval; 3] = rate
                .try_into()
                .map_err(|_| MathError::Domain("three actual source derivatives are required"))?;
            purrdf_xsd::math::matrix_vector3(matrix, rate, math)
                .map(|values| values.into_iter().collect())
        })
        .transpose()?;
    Ok(DirectionalImage {
        coordinates,
        derivative,
    })
}

/// Smooth normal and its Cartesian directional derivative, including either
/// polar axis. For N and M the principal radii, Dn=P/(M+h)+Cwwᵀ,
/// P=I−nnᵀ, w=(-ny,nx,0), and
/// C=−N e²/[(1−e² nz²)(N+h)(M+h)]. The apparent 1/(1−nz²)
/// from the east unit vector cancels exactly. This is the differential of the
/// same unique latitude/height family used by the geographic inverse.
pub(super) fn geocentric_normal(
    coordinates: &[FixedInterval],
    derivative: Option<&[FixedInterval]>,
    ellipsoid: &PreparedEllipsoid,
    limits: OperationSolverLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<super::OperationNormalEnclosure, MathError> {
    let family = super::super::inverse::geocentric_family(
        coordinates,
        ellipsoid,
        limits.iterations,
        false,
        math,
        progress,
    )?;
    let zero = FixedInterval::from_i64(0, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let nz = match family.pole {
        Some(sign) => FixedInterval::from_i64(sign, math)?,
        None => family.latitude.sin_cos_range(math)?.0,
    };
    let e2 = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
    let w2 = one.sub(&e2.mul(&nz.square(math)?, math)?, math)?;
    let n = fixed_from_rat(ellipsoid.semimajor(), math)?.div(&w2.sqrt(math)?, math)?;
    let prime = n.add(&family.height, math)?;
    let normal = [
        coordinates[0].div(&prime, math)?,
        coordinates[1].div(&prime, math)?,
        nz,
    ];
    let Some(rate) = derivative else {
        return Ok((normal, None));
    };
    let rate: &[FixedInterval; 3] = rate
        .try_into()
        .map_err(|_| MathError::Domain("three actual Cartesian derivatives are required"))?;
    let meridian = n
        .mul(&one.sub(&e2, math)?, math)?
        .div(&w2, math)?
        .add(&family.height, math)?;
    let coefficient = n
        .mul(&e2, math)?
        .neg(math)?
        .div(&w2.mul(&prime, math)?.mul(&meridian, math)?, math)?;
    let east = [normal[1].neg(math)?, normal[0].clone(), zero.clone()];
    let mut differential = core::array::from_fn(|_| zero.clone());
    for row in 0..3 {
        for column in 0..3 {
            let identity = if row == column { &one } else { &zero };
            differential[row * 3 + column] = identity
                .sub(&normal[row].mul(&normal[column], math)?, math)?
                .div(&meridian, math)?
                .add(
                    &coefficient
                        .mul(&east[row], math)?
                        .mul(&east[column], math)?,
                    math,
                )?;
        }
    }
    let derivative = purrdf_xsd::math::matrix_vector3(&differential, rate, math)?;
    Ok((normal, Some(derivative)))
}

fn geocentric_jacobian(
    coordinates: &[FixedInterval],
    ellipsoid: &PreparedEllipsoid,
    math: &mut CoordinateMath,
) -> Result<Matrix3, MathError> {
    let height = coordinates
        .get(2)
        .ok_or(MathError::Domain("actual height is required"))?;
    let factor = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
    let (sl, cl, sp, cp) = super::super::forward::geocentric_angles(coordinates, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let e2 = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
    let w2 = one.sub(&e2.mul(&sp.square(math)?, math)?, math)?;
    let n = fixed_from_rat(ellipsoid.semimajor(), math)?.div(&w2.sqrt(math)?, math)?;
    let dn = n
        .mul(&e2, math)?
        .mul(&sp, math)?
        .mul(&cp, math)?
        .div(&w2, math)?;
    let horizontal = n.add(height, math)?.mul(&cp, math)?;
    let north_horizontal = dn
        .mul(&cp, math)?
        .sub(&n.add(height, math)?.mul(&sp, math)?, math)?;
    let vertical = dn.mul(&one.sub(&e2, math)?, math)?.mul(&sp, math)?.add(
        &n.mul(&one.sub(&e2, math)?, math)?
            .add(height, math)?
            .mul(&cp, math)?,
        math,
    )?;
    Ok([
        horizontal.mul(&sl, math)?.neg(math)?.mul(&factor, math)?,
        north_horizontal.mul(&cl, math)?.mul(&factor, math)?,
        cp.mul(&cl, math)?,
        horizontal.mul(&cl, math)?.mul(&factor, math)?,
        north_horizontal.mul(&sl, math)?.mul(&factor, math)?,
        cp.mul(&sl, math)?,
        FixedInterval::from_i64(0, math)?,
        vertical.mul(&factor, math)?,
        sp,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerical::exact_bounds;
    use crate::{GeographicReference, Rat};
    use purrdf_xsd::math::MathLimits;

    #[test]
    fn equatorial_cartesian_columns_match_independent_metric_factors() {
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let ellipsoid = GeographicReference::wgs84().ellipsoid().clone();
        let coordinates = [
            FixedInterval::from_i64(0, &mut math).unwrap(),
            FixedInterval::from_i64(0, &mut math).unwrap(),
            FixedInterval::from_i64(12, &mut math).unwrap(),
        ];
        let jacobian = geocentric_jacobian(&coordinates, &ellipsoid, &mut math).unwrap();
        // At the equator, the prime-vertical and meridional factors are exactly
        // N=a and M=a(1-e²). Height adds equally to both angular factors.
        let degree = FixedInterval::pi(&mut math)
            .unwrap()
            .div(&FixedInterval::from_i64(180, &mut math).unwrap(), &mut math)
            .unwrap();
        let east = fixed_from_rat(&ellipsoid.semimajor().add(&Rat::from_i64(12)), &mut math)
            .unwrap()
            .mul(&degree, &mut math)
            .unwrap();
        let north = fixed_from_rat(
            &ellipsoid
                .semimajor()
                .mul(&Rat::one().sub(&ellipsoid.eccentricity_squared()))
                .add(&Rat::from_i64(12)),
            &mut math,
        )
        .unwrap()
        .mul(&degree, &mut math)
        .unwrap();
        for (entry, expected) in [(3, east), (7, north)] {
            assert!(jacobian[entry].lower() <= expected.upper());
            assert!(jacobian[entry].upper() >= expected.lower());
            let (lower, upper) = exact_bounds(&jacobian[entry]);
            assert!(upper.sub(&lower) < Rat::parse_decimal("0.000000000000000001").unwrap());
        }
        for entry in [0, 1, 4, 5, 6, 8] {
            assert!(jacobian[entry].is_exact_zero());
        }
        let (lower, upper) = exact_bounds(&jacobian[2]);
        assert_eq!((lower, upper), (Rat::one(), Rat::one()));
    }

    #[test]
    fn cartesian_normal_is_regular_on_both_poles_and_complete_crossing_panels() {
        for sign in [-1, 1] {
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let ellipsoid = GeographicReference::wgs84().ellipsoid().clone();
            let height = Rat::from_i64(12);
            let z = ellipsoid.semiminor().add(&height).mul(&Rat::from_i64(sign));
            let mut coordinates = [
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(0, &mut math).unwrap(),
                fixed_from_rat(&z, &mut math).unwrap(),
            ];
            let rate = [
                FixedInterval::from_i64(2, &mut math).unwrap(),
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(0, &mut math).unwrap(),
            ];
            let limits = OperationSolverLimits {
                iterations: 128,
                subdivisions: 64,
                quantize_inverse: false,
            };
            let (normal, derivative) = geocentric_normal(
                &coordinates,
                Some(&rate),
                &ellipsoid,
                limits,
                &mut math,
                &mut WorkProgress::new(None),
            )
            .unwrap();
            assert!(normal[..2].iter().all(FixedInterval::is_exact_zero));
            assert_eq!(
                exact_bounds(&normal[2]),
                (Rat::from_i64(sign), Rat::from_i64(sign))
            );
            // At a pole M=N=a²/b; a Cartesian east displacement changes the
            // normal by exactly that displacement divided by N+h.
            let polar_radius = ellipsoid
                .semimajor()
                .mul(ellipsoid.semimajor())
                .div(ellipsoid.semiminor())
                .unwrap()
                .add(&height);
            let expected = Rat::from_i64(2).div(&polar_radius).unwrap();
            let derivative = derivative.unwrap();
            let (lower, upper) = exact_bounds(&derivative[0]);
            assert!(lower <= expected && expected <= upper);
            assert!(derivative[1..].iter().all(FixedInterval::is_exact_zero));

            // This whole interval contains the pole. Longitude has two limits,
            // while the unique normal family and its derivative are finite.
            coordinates[0] =
                crate::numerical::fixed_from_bounds(&Rat::from_i64(-1), &Rat::one(), &mut math)
                    .unwrap();
            let (normal, derivative) = geocentric_normal(
                &coordinates,
                Some(&rate),
                &ellipsoid,
                limits,
                &mut math,
                &mut WorkProgress::new(None),
            )
            .unwrap();
            assert!(normal[0].lower().is_negative());
            assert!(!normal[0].upper().is_negative());
            assert!(normal[1].is_exact_zero());
            assert!(derivative.unwrap()[0].lower() > &purrdf_xsd::BigInt::zero());
        }
    }

    #[test]
    fn polar_axis_curve_has_an_exact_restricted_geographic_derivative() {
        for sign in [-1, 1] {
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let ellipsoid = GeographicReference::wgs84().ellipsoid().clone();
            let model = OperationModel::GeocentricToGeographic { ellipsoid };
            let coordinates = [
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(sign * 6_357_000, &mut math).unwrap(),
            ];
            let rate = [
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(0, &mut math).unwrap(),
                FixedInterval::from_i64(10, &mut math).unwrap(),
            ];
            let point = OperationPoint {
                x: Rat::zero(),
                y: Rat::zero(),
                z: Some(Rat::from_i64(sign * 6_357_000)),
                epoch: None,
            };
            let image = evaluate(
                &model,
                &point,
                (&coordinates, Some(&rate)),
                OperationSolverLimits {
                    iterations: 128,
                    subdivisions: 64,
                    quantize_inverse: false,
                },
                &mut math,
                &mut WorkProgress::new(None),
            )
            .unwrap()
            .unwrap();
            assert!(image.coordinates[0].is_exact_zero());
            let (lower, upper) = exact_bounds(&image.coordinates[1]);
            assert_eq!(
                (lower, upper),
                (Rat::from_i64(sign * 90), Rat::from_i64(sign * 90))
            );
            let derivative = image.derivative.unwrap();
            assert!(derivative[..2].iter().all(FixedInterval::is_exact_zero));
            let (lower, upper) = exact_bounds(&derivative[2]);
            assert_eq!(
                (lower, upper),
                (Rat::from_i64(sign * 10), Rat::from_i64(sign * 10))
            );
        }
    }
}
