// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified monotone inverses, without a last-iterate success path.

use super::OperationCoordinates;

use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, MathError},
};

use super::degrees;
use crate::context::WorkProgress;
use crate::numerical::fixed_from_rat;
use crate::{PreparedEllipsoid, Rat};

pub(super) fn bd09(
    coordinates: &[FixedInterval],
    iterations: u32,
    quantize: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationCoordinates, MathError> {
    let k = FixedInterval::pi(math)?
        .mul(&FixedInterval::from_i64(3000, math)?, math)?
        .div(&FixedInterval::from_i64(180, math)?, math)?;
    let alpha = super::decimal("0.00002", math)?;
    let beta = super::decimal("0.000003", math)?;
    // For r<=sqrt(180²+90²)<202, H(p)=F(p)-p has operator
    // norm <= beta+2*alpha*k+(202+alpha)*beta*k < .04.
    // The continuous origin is included by the same Lipschitz bound.
    let bound = beta
        .add(
            &alpha
                .mul(&k, math)?
                .mul(&FixedInterval::from_i64(2, math)?, math)?,
            math,
        )?
        .add(
            &FixedInterval::from_i64(202, math)?
                .add(&alpha, math)?
                .mul(&beta, math)?
                .mul(&k, math)?,
            math,
        )?;
    if bound.upper() >= super::decimal("0.04", math)?.lower() {
        return Err(MathError::PrecisionExhausted);
    }
    let target = [
        coordinates[0].sub(&super::decimal("0.0065", math)?, math)?,
        coordinates[1].sub(&super::decimal("0.006", math)?, math)?,
    ];
    let target_width = target[0].width(math)?.add(&target[1].width(math)?, math)?;
    let inverse_condition = FixedInterval::from_i64(1, math)?.sub(&bound, math)?;
    let conditioned_width = target_width.div(&inverse_condition, math)?;
    if target
        .iter()
        .all(|value| value.lower().is_zero() && value.upper().is_zero())
    {
        let mut output: OperationCoordinates = purrdf_core::smallvec![
            FixedInterval::from_i64(0, math)?,
            FixedInterval::from_i64(0, math)?,
        ];
        if let Some(z) = coordinates.get(2) {
            output.push(z.clone());
        }
        return Ok(output);
    }
    let mut enclosure = [
        FixedInterval::from_bounds(
            FixedInterval::from_i64(-180, math)?.lower().clone(),
            FixedInterval::from_i64(180, math)?.upper().clone(),
            math,
        )?,
        FixedInterval::from_bounds(
            FixedInterval::from_i64(-90, math)?.lower().clone(),
            FixedInterval::from_i64(90, math)?.upper().clone(),
            math,
        )?,
    ];
    let mut exists = false;
    for _ in 0..iterations {
        progress.math_poll(math)?;
        let offset = super::forward::bd09_offset(&enclosure, math)?;
        let image = [
            target[0].sub(&offset[0], math)?,
            target[1].sub(&offset[1], math)?,
        ];
        // Banach proves existence when the full continuous map sends this
        // closed rectangle into itself; the global .04 bound proves uniqueness.
        exists |= image
            .iter()
            .zip(&enclosure)
            .all(|(next, old)| next.lower() >= old.lower() && next.upper() <= old.upper());
        for (old, next) in enclosure.iter_mut().zip(image) {
            let lower = old.lower().clone().max(next.lower().clone());
            let upper = old.upper().clone().min(next.upper().clone());
            if lower > upper {
                return Err(MathError::Domain(
                    "BD09 inverse has no root in its source domain",
                ));
            }
            *old = FixedInterval::from_bounds(lower, upper, math)?;
        }
        // The Cartesian offset contains factors as large as the source
        // longitude. Its certified fixed-grid evaluation width therefore need
        // not fit an arbitrary number of output-grid ulps. Evaluate at one
        // exact dyadic midpoint, then transport that actual enclosure width
        // through Banach's 1/(1-L) conditioning bound. This accounts for the
        // arithmetic floor separately from the unrounded target family.
        let evaluation_width = if exists && !quantize {
            let midpoint = [enclosure[0].midpoint(math)?, enclosure[1].midpoint(math)?];
            let midpoint_offset = super::forward::bd09_offset(&midpoint, math)?;
            midpoint_offset[0]
                .width(math)?
                .add(&midpoint_offset[1].width(math)?, math)?
                .div(&inverse_condition, math)?
        } else {
            FixedInterval::from_i64(0, math)?
        };
        let completed = enclosure
            .iter()
            .map(|value| {
                purrdf_xsd::math::inverse_coordinate_complete_with_error(
                    value,
                    &conditioned_width,
                    &evaluation_width,
                    15,
                    quantize,
                    math,
                )
            })
            .collect::<Result<purrdf_core::SmallVec<[bool; 2]>, _>>()?;
        if exists && completed.iter().all(|value| *value) {
            let mut output = OperationCoordinates::from_array(enclosure);
            if let Some(z) = coordinates.get(2) {
                output.push(z.clone());
            }
            return Ok(output);
        }
    }
    Err(MathError::ConvergenceExhausted { iterations })
}

pub(super) fn bd09_residual(
    input: &[FixedInterval],
    output: &[Rat],
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    let source = [
        fixed_from_rat(&output[0], math)?,
        fixed_from_rat(&output[1], math)?,
    ];
    let restored = super::forward::bd09(&source, math)?;
    let norm = restored
        .iter()
        .zip(input)
        .take(2)
        .try_fold(
            FixedInterval::from_i64(0, math)?,
            |sum, (actual, expected)| sum.add(&actual.sub(expected, math)?.square(math)?, math),
        )?
        .sqrt(math)?;
    if norm.upper() <= super::decimal("0.000000000001", math)?.lower() {
        Ok(())
    } else {
        Err(MathError::PrecisionExhausted)
    }
}

fn hull(
    lower: &FixedInterval,
    upper: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)
}

pub(super) fn mercator(
    coordinates: &[FixedInterval],
    (radius, e2): (&Rat, &Rat),
    square_domain: bool,
    iterations: u32,
    quantize: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationCoordinates, MathError> {
    let radius = fixed_from_rat(radius, math)?;
    let psi = coordinates[1].div(&radius, math)?;
    let lambda = coordinates[0].div(&radius, math)?;
    let pi = FixedInterval::pi(math)?;
    if square_domain {
        for value in [&psi, &lambda] {
            if value.abs(math)?.lower() > pi.upper() {
                return Err(MathError::Domain(
                    "inverse Web Mercator coordinate outside square",
                ));
            }
            if value.abs(math)?.upper() > pi.lower() {
                return Err(MathError::PrecisionExhausted);
            }
        }
    }
    let one = FixedInterval::from_i64(1, math)?;
    let two = FixedInterval::from_i64(2, math)?;
    let e2 = fixed_from_rat(e2, math)?;
    let eccentricity = e2.sqrt(math)?;
    let minimum_derivative = one.sub(&e2, math)?;
    // Invert each exact dyadic endpoint separately. Monotonicity then encloses
    // the complete ordinate family without repeatedly treating that family's
    // width as numerical uncertainty in an inverse iteration.
    let half = one.div(&two, math)?;
    if e2.upper() < half.lower() {
        let latitude = mercator_contraction_family(
            &psi,
            &eccentricity,
            &minimum_derivative,
            iterations,
            math,
            progress,
        )?;
        let mut output: OperationCoordinates =
            purrdf_core::smallvec![degrees(&lambda, math)?, degrees(&latitude, math)?];
        if quantize {
            for value in &output {
                let (lower, upper) = value.round_decimal(15, math)?;
                if lower != upper {
                    return Err(MathError::PrecisionExhausted);
                }
            }
        }
        if let Some(z) = coordinates.get(2) {
            output.push(z.clone());
        }
        return Ok(output);
    }
    let conditioned_width = degrees(&psi.width(math)?.div(&minimum_derivative, math)?, math)?;
    // psi(phi) is strictly increasing; its limiting values at both open poles
    // are infinite. Endpoint singularities therefore need no numerical evaluation.
    let half_pi = pi.div(&two, math)?;
    let mut lower = half_pi.neg(math)?;
    let mut upper = half_pi;
    let mut certified = if psi.lower().is_zero() && psi.upper().is_zero() {
        Some(FixedInterval::from_i64(0, math)?)
    } else {
        None
    };
    for _ in 0..iterations {
        progress.math_poll(math)?;
        if certified.is_some() {
            break;
        }
        let middle = lower.add(&upper, math)?.div(&two, math)?;
        let (sine, cosine) = middle.sin_cos(math)?;
        let mut value = one.add(&sine, math)?.div(&cosine, math)?.log(math)?;
        if !e2.lower().is_zero() || !e2.upper().is_zero() {
            let es = eccentricity.mul(&sine, math)?;
            value = value.add(
                &one.sub(&es, math)?
                    .div(&one.add(&es, math)?, math)?
                    .log(math)?
                    .mul(&eccentricity, math)?
                    .div(&two, math)?,
                math,
            )?;
        }
        if value.upper() < psi.lower() {
            lower = middle;
        } else if value.lower() > psi.upper() {
            upper = middle;
        } else {
            // Mean value theorem: dpsi/dphi >= 1-e² throughout the entire
            // open latitude interval. This encloses the root despite arithmetic
            // uncertainty in the middle's ordinate.
            let error = value
                .sub(&psi, math)?
                .abs(math)?
                .div(&minimum_derivative, math)?;
            let proposed_lower = middle.sub(&error, math)?;
            let proposed_upper = middle.add(&error, math)?;
            lower = FixedInterval::from_bounds(
                lower.lower().clone().max(proposed_lower.lower().clone()),
                lower.upper().clone().max(proposed_lower.lower().clone()),
                math,
            )?;
            upper = FixedInterval::from_bounds(
                upper.lower().clone().min(proposed_upper.upper().clone()),
                upper.upper().clone().min(proposed_upper.upper().clone()),
                math,
            )?;
        }
        let enclosure = hull(&lower, &upper, math)?;
        if purrdf_xsd::math::inverse_coordinate_complete(
            &degrees(&enclosure, math)?,
            &conditioned_width,
            15,
            quantize,
            math,
        )? {
            certified = Some(enclosure);
        }
    }
    let phi = certified.ok_or(MathError::ConvergenceExhausted { iterations })?;
    let longitude = degrees(&lambda, math)?;
    let latitude = degrees(&phi, math)?;
    let mut output: OperationCoordinates = purrdf_core::smallvec![longitude, latitude];
    if let Some(z) = coordinates.get(2) {
        output.push(z.clone());
    }
    Ok(output)
}

/// The spherical inverse is pi/2-2 atan(exp(-psi)) for psi>=0, with odd
/// reflection for negative psi. The negative exponential keeps all temporary
/// magnitudes bounded even for a very large finite projected ordinate.
fn spherical_mercator_inverse_point(
    psi: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    if psi.lower().is_zero() && psi.upper().is_zero() {
        return FixedInterval::from_i64(0, math);
    }
    let negative = psi.upper().is_negative();
    let magnitude = if negative {
        psi.neg(math)?
    } else {
        psi.clone()
    };
    let exponential = magnitude.neg(math)?.exp(math)?;
    let two = FixedInterval::from_i64(2, math)?;
    let latitude = FixedInterval::pi(math)?
        .div(&two, math)?
        .sub(&exponential.atan(math)?.mul(&two, math)?, math)?;
    if negative {
        latitude.neg(math)
    } else {
        Ok(latitude)
    }
}

fn spherical_mercator_inverse(
    psi: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let lower = FixedInterval::from_bounds(psi.lower().clone(), psi.lower().clone(), math)?;
    let upper = FixedInterval::from_bounds(psi.upper().clone(), psi.upper().clone(), math)?;
    hull(
        &spherical_mercator_inverse_point(&lower, math)?,
        &spherical_mercator_inverse_point(&upper, math)?,
        math,
    )
}

fn mercator_contraction_family(
    psi: &FixedInterval,
    eccentricity: &FixedInterval,
    one_minus_e2: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    if eccentricity.lower().is_zero() && eccentricity.upper().is_zero() {
        return spherical_mercator_inverse(psi, math);
    }
    let lower = FixedInterval::from_bounds(psi.lower().clone(), psi.lower().clone(), math)?;
    let upper = FixedInterval::from_bounds(psi.upper().clone(), psi.upper().clone(), math)?;
    let lower = mercator_contraction_point(
        &lower,
        eccentricity,
        one_minus_e2,
        iterations,
        math,
        progress,
    )?;
    let upper = mercator_contraction_point(
        &upper,
        eccentricity,
        one_minus_e2,
        iterations,
        math,
        progress,
    )?;
    hull(&lower, &upper, math)
}

fn mercator_contraction_point(
    psi: &FixedInterval,
    eccentricity: &FixedInterval,
    one_minus_e2: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    if psi.lower().is_zero() && psi.upper().is_zero() {
        return FixedInterval::from_i64(0, math);
    }
    let one = FixedInterval::from_i64(1, math)?;
    let two = FixedInterval::from_i64(2, math)?;
    let half_pi = FixedInterval::pi(math)?.div(&two, math)?;
    let mut latitude =
        FixedInterval::from_bounds(half_pi.upper().negated(), half_pi.upper().clone(), math)?;
    let zero = FixedInterval::from_i64(0, math)?;
    // T(phi)=gd(psi+e atanh(e sin(phi))) maps the closed latitude interval
    // into its interior. |gd'|<=1 and |(e atanh(e sin(phi)))'|
    // <=e²/(1-e²)<1 prove existence and uniqueness on the entire interval.
    // The exact projected endpoint is fixed throughout this contraction.
    let contraction = one.sub(one_minus_e2, math)?.div(one_minus_e2, math)?;
    if contraction.upper() >= one.lower() {
        return Err(MathError::PrecisionExhausted);
    }
    for _ in 0..iterations {
        progress.math_poll(math)?;
        let image = mercator_contraction_image(&latitude, psi, eccentricity, math)?;
        let next = FixedInterval::from_bounds(
            latitude.lower().clone().max(image.lower().clone()),
            latitude.upper().clone().min(image.upper().clone()),
            math,
        )?;
        let contracted = next.lower() != latitude.lower() || next.upper() != latitude.upper();
        latitude = next;
        if purrdf_xsd::math::inverse_coordinate_complete(&latitude, &zero, 15, false, math)? {
            return Ok(latitude);
        }
        if !contracted {
            // The fixed-point image has a proved contraction on the complete
            // source latitude interval. Its exact-midpoint evaluation encloses
            // all admitted eccentricity values and arithmetic rounding. The
            // resulting width/(1-L) bounds the enclosure floor independently
            // of the unknown true root. A fixed count of output-grid ulps
            // cannot demand that this real evaluation floor vanish.
            let midpoint = latitude.midpoint(math)?;
            let evaluation = mercator_contraction_image(&midpoint, psi, eccentricity, math)?;
            let evaluation_width = evaluation
                .width(math)?
                .div(&one.sub(&contraction, math)?, math)?;
            if purrdf_xsd::math::inverse_coordinate_complete_with_error(
                &latitude,
                &zero,
                &evaluation_width,
                15,
                false,
                math,
            )? {
                return Ok(latitude);
            }
            return Err(MathError::PrecisionExhausted);
        }
    }
    Err(MathError::ConvergenceExhausted { iterations })
}

fn mercator_contraction_image(
    latitude: &FixedInterval,
    psi: &FixedInterval,
    eccentricity: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let one = FixedInterval::from_i64(1, math)?;
    let (sine, _) = latitude.sin_cos_range(math)?;
    let es = eccentricity.mul(&sine, math)?;
    let correction = one
        .add(&es, math)?
        .div(&one.sub(&es, math)?, math)?
        .log(math)?
        .mul(eccentricity, math)?
        .div(&FixedInterval::from_i64(2, math)?, math)?;
    spherical_mercator_inverse(&psi.add(&correction, math)?, math)
}

pub(super) fn geocentric(
    coordinates: &[FixedInterval],
    ellipsoid: &PreparedEllipsoid,
    iterations: u32,
    quantize: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationCoordinates, MathError> {
    let family = geocentric_family(coordinates, ellipsoid, iterations, quantize, math, progress)?;
    let latitude = match family.pole {
        Some(sign) => FixedInterval::from_i64(sign * 90, math)?,
        None => degrees(&family.latitude, math)?,
    };
    let x = &coordinates[0];
    let y = &coordinates[1];
    // Exact negative X and zero Y identify the longitude seam without an epsilon.
    let longitude = if family.pole.is_some() {
        FixedInterval::from_i64(0, math)?
    } else if x.upper().is_negative() && y.is_exact_zero() {
        FixedInterval::from_i64(-180, math)?
    } else {
        degrees(&FixedInterval::atan2(y, x, math)?, math)?
    };
    Ok(purrdf_core::smallvec![longitude, latitude, family.height])
}

/// The unique normal latitude and height, before choosing a longitude chart.
/// Both the geographic response and pole-regular Cartesian normal call this
/// family solver; a normal request never evaluates longitude at the axis.
pub(super) struct GeocentricFamily {
    pub(super) latitude: FixedInterval,
    pub(super) height: FixedInterval,
    pub(super) pole: Option<i64>,
}

struct GeocentricMap<'a> {
    one: &'a FixedInterval,
    radius: &'a FixedInterval,
    vertical: &'a FixedInterval,
    semimajor: &'a FixedInterval,
    eccentricity_squared: &'a FixedInterval,
    crosses_equator: bool,
}

impl GeocentricMap<'_> {
    fn image(
        &self,
        latitude: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<FixedInterval, MathError> {
        let sine = if self.crosses_equator {
            latitude.sin_cos_range(math)?.0
        } else {
            latitude.sin_cos(math)?.0
        };
        let n = self.semimajor.div(
            &self
                .one
                .sub(
                    &self.eccentricity_squared.mul(&sine.square(math)?, math)?,
                    math,
                )?
                .sqrt(math)?,
            math,
        )?;
        FixedInterval::atan2(
            &self.vertical.add(
                &self.eccentricity_squared.mul(&n, math)?.mul(&sine, math)?,
                math,
            )?,
            self.radius,
            math,
        )
    }
}

pub(super) fn geocentric_family(
    coordinates: &[FixedInterval],
    ellipsoid: &PreparedEllipsoid,
    iterations: u32,
    quantize: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<GeocentricFamily, MathError> {
    let x = &coordinates[0];
    let y = &coordinates[1];
    let z = coordinates.get(2).ok_or(MathError::Domain(
        "three actual geocentric coordinates are required",
    ))?;
    let a = fixed_from_rat(ellipsoid.semimajor(), math)?;
    let b = fixed_from_rat(ellipsoid.semiminor(), math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let p = x.square(math)?.add(&y.square(math)?, math)?.sqrt(math)?;
    let e2 = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
    // The normal fixed-point map is increasing. Its derivative is bounded by
    // e²*a / ((1-e²)^(3/2) * hypot(p,z)); a bound below one proves a unique
    // root throughout the latitude bracket, including slightly negative heights.
    let radial = p.square(math)?.add(&z.square(math)?, math)?.sqrt(math)?;
    let c2 = one.sub(&e2, math)?;
    let contraction_radius = e2
        .mul(&a, math)?
        .div(&c2.mul(&c2.sqrt(math)?, math)?, math)?;
    if radial.lower() <= contraction_radius.upper() {
        return Err(MathError::PrecisionExhausted);
    }
    if p.lower().is_zero() && p.upper().is_zero() {
        if z.lower().is_zero() && z.upper().is_zero() {
            return Err(MathError::Domain(
                "geocentric origin has no unique geographic latitude",
            ));
        }
        if z.lower() <= &BigInt::zero() && z.upper() >= &BigInt::zero() {
            return Err(MathError::PrecisionExhausted);
        }
        let sign = if z.lower().is_negative() { -1 } else { 1 };
        return Ok(GeocentricFamily {
            latitude: FixedInterval::pi(math)?
                .mul(&FixedInterval::from_i64(sign, math)?, math)?
                .div(&FixedInterval::from_i64(2, math)?, math)?,
            height: z.abs(math)?.sub(&b, math)?,
            pole: Some(sign),
        });
    }
    let negative = z.upper().is_negative();
    let crosses_equator = !negative && z.lower().is_negative();
    let zz = if crosses_equator {
        z.clone()
    } else {
        z.abs(math)?
    };
    let lower = if crosses_equator {
        FixedInterval::pi(math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?
            .neg(math)?
    } else {
        FixedInterval::atan2(&zz, &p, math)?
    };
    // The normal fixed-point map has contraction cr/r. Its derivative with
    // respect to (p,z) has norm at most 1/r, hence the complete inverse family
    // has latitude Lipschitz bound 1/(r-cr), before conversion to degrees.
    let conditioned_width = degrees(
        &p.width(math)?
            .add(&zz.width(math)?, math)?
            .div(&radial.sub(&contraction_radius, math)?, math)?,
        math,
    )?;
    let upper = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(2, math)?, math)?;
    let mut phi = hull(&lower, &upper, math)?;
    let mut certified = zz.is_exact_zero();
    if certified {
        phi = FixedInterval::from_i64(0, math)?;
    }
    let map = GeocentricMap {
        one: &one,
        radius: &p,
        vertical: &zz,
        semimajor: &a,
        eccentricity_squared: &e2,
        crosses_equator,
    };
    for _ in 0..iterations {
        progress.math_poll(math)?;
        if certified {
            break;
        }
        let next = map.image(&phi, math)?;
        let proposal = FixedInterval::from_bounds(
            phi.lower().clone().max(next.lower().clone()),
            phi.upper().clone().min(next.upper().clone()),
            math,
        )?;
        let contracted = proposal.lower() != phi.lower() || proposal.upper() != phi.upper();
        phi = proposal;
        if purrdf_xsd::math::inverse_coordinate_complete(
            &degrees(&phi, math)?,
            &conditioned_width,
            15,
            quantize,
            math,
        )? {
            certified = true;
            break;
        }
        if !contracted && !quantize {
            // The original map has Lipschitz constant cr/r<1 on the complete
            // bracket. Its exact midpoint image contains the whole source
            // family and arithmetic width. Transport that independent width
            // through 1/(1-cr/r); no successive-approximation heuristic or
            // fixed output-ulp count certifies the root.
            let evaluation = map.image(&phi.midpoint(math)?, math)?;
            let evaluation_width = degrees(
                &evaluation.width(math)?.div(
                    &one.sub(&contraction_radius.div(&radial, math)?, math)?,
                    math,
                )?,
                math,
            )?;
            certified = purrdf_xsd::math::inverse_coordinate_complete_with_error(
                &degrees(&phi, math)?,
                &conditioned_width,
                &evaluation_width,
                15,
                false,
                math,
            )?;
            if certified {
                break;
            }
            return Err(MathError::PrecisionExhausted);
        }
    }
    if !certified {
        return Err(MathError::ConvergenceExhausted { iterations });
    }
    if negative {
        phi = phi.neg(math)?;
    }
    let (sine, cosine) = if crosses_equator {
        phi.sin_cos_range(math)?
    } else {
        phi.sin_cos(math)?
    };
    let height = p.mul(&cosine, math)?.add(&z.mul(&sine, math)?, math)?.sub(
        &a.mul(
            &one.sub(&e2.mul(&sine.square(math)?, math)?, math)?
                .sqrt(math)?,
            math,
        )?,
        math,
    )?;
    Ok(GeocentricFamily {
        latitude: phi,
        height,
        pole: None,
    })
}

pub(super) fn projected_residual(
    input: &[FixedInterval],
    output: &[Rat],
    radius: &Rat,
    e2: &Rat,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    let coordinates = [
        fixed_from_rat(&output[0], math)?,
        fixed_from_rat(&output[1], math)?,
    ];
    let restored = super::forward::mercator(&coordinates, radius, e2, false, math)?;
    let tolerance = super::decimal("0.000001", math)?;
    let mut norm_squared = FixedInterval::from_i64(0, math)?;
    for (restored, expected) in restored.iter().zip(input).take(2) {
        norm_squared = norm_squared.add(&restored.sub(expected, math)?.square(math)?, math)?;
    }
    if norm_squared.sqrt(math)?.upper() <= tolerance.lower() {
        Ok(())
    } else {
        Err(MathError::PrecisionExhausted)
    }
}

pub(super) fn geocentric_residual(
    input: &[FixedInterval],
    output: &[Rat],
    ellipsoid: &PreparedEllipsoid,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    let coordinates: Result<OperationCoordinates, _> = output
        .iter()
        .map(|value| fixed_from_rat(value, math))
        .collect();
    let restored = super::forward::geocentric(&coordinates?, ellipsoid, math)?;
    let mut norm_squared = FixedInterval::from_i64(0, math)?;
    for (actual, expected) in restored.iter().zip(input).take(3) {
        norm_squared = norm_squared.add(&actual.sub(expected, math)?.square(math)?, math)?;
    }
    if norm_squared.sqrt(math)?.upper() <= super::decimal("0.000001", math)?.lower() {
        Ok(())
    } else {
        Err(MathError::PrecisionExhausted)
    }
}

#[cfg(test)]
mod tests {
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, OperationModel, OperationPoint, OperationReference,
    };
    use crate::{GeographicReference, MetricContext, Rat};
    use purrdf_hash::hex::Digest32;

    #[test]
    fn analytic_mercator_inverse_completes_at_its_certified_arithmetic_floor() {
        let geographic = OperationReference {
            realization: GeographicReference::wgs84().id().digest(),
            unit: CoordinateUnit::Degrees,
            swapped_axes: false,
        };
        let projected = OperationReference {
            realization: Digest32::new([31; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        let operation = CoordinateOperation::compile(
            geographic,
            projected,
            OperationModel::BaiduMercatorAnalyticV1,
        )
        .unwrap();
        let inverse = operation.inverse().unwrap();
        for latitude in ["0", "0.000000000001", "31.23", "-31.23", "85", "-85"] {
            let input = OperationPoint {
                x: Rat::parse_decimal("121.47").unwrap(),
                y: Rat::parse_decimal(latitude).unwrap(),
                z: Some(Rat::from_i64(10)),
                epoch: None,
            };
            let mut context = MetricContext::wgs84().unwrap();
            let forward = operation.apply(&input, &mut context).unwrap();
            let restored = inverse.apply(forward.point(), &mut context).unwrap();
            let error = Rat::parse_decimal("0.00000000001").unwrap();
            assert!(restored.point().x.sub(&input.x).abs() < error);
            assert!(restored.point().y.sub(&input.y).abs() < error);
            assert_eq!(restored.point().z, input.z);
            assert!(context.work_items() <= context.policy().limits().max_work_items);
        }
    }
}
