// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Globally certified GCJ roots on both smooth cusp charts L=105±u², u>=0.
//! A triangle-inequality displacement bound excludes the remote part of the
//! caller's exact rectangle. Complete interval traversal then covers every
//! remaining source point, including both sides of turns. The forward equation
//! bodies are shared; only their independently derived Jacobian is written here.

use super::OperationCoordinates;

use purrdf_xsd::{
    BigInt,
    math::{
        CoordinateMath, FixedInterval, MathError, RootBox2, RootIsolationLimits, RootJacobian2,
        RootSystem2, isolate_roots2, root_enclosure_complete,
    },
};

use super::{Applicability, OperationSolverLimits, decimal, degrees, forward, radians};
use crate::{
    Rat,
    context::WorkProgress,
    numerical::{exact_bounds, fixed_from_rat},
};

pub(super) fn evaluate(
    applicability: &Applicability,
    coordinates: &[FixedInterval],
    limits: OperationSolverLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationCoordinates, MathError> {
    let target = [coordinates[0].clone(), coordinates[1].clone()];
    let source = narrow_domain(applicability, &target, math)?;
    let centre = FixedInterval::from_i64(105, math)?;
    let mut retained: Vec<PhysicalRoot> = Vec::new();
    let mut reserved: usize = 0;
    let result = (|| {
        for sign in [-1, 1] {
            progress.math_poll(math)?;
            let Some(domain) = chart_domain(&source, &centre, sign, math)? else {
                continue;
            };
            let mut system = GcjSystem {
                sign,
                quantize: limits.quantize_inverse,
                target: &target,
                progress: &mut *progress,
            };
            let roots = isolate_roots2(
                domain,
                RootIsolationLimits {
                    max_depth: limits.subdivisions,
                    max_refinements: limits.iterations,
                },
                &mut system,
                math,
            )?;
            // The shared solver admitted its returned vector during isolation.
            // Keep that actual storage admitted while evaluating the next chart.
            let bytes = roots
                .iter()
                .try_fold(
                    roots
                        .capacity()
                        .checked_mul(size_of::<purrdf_xsd::math::RootIsolation2>())
                        .ok_or(MathError::WorkspaceExhausted)?,
                    |bytes, root| {
                        root.enclosure()
                            .iter()
                            .chain(root.uniqueness_box())
                            .try_fold(bytes, |bytes, interval| {
                                bytes.checked_add(interval.workspace_bytes())
                            })
                    },
                )
                .ok_or(MathError::WorkspaceExhausted)?;
            math.reserve_workspace(bytes)?;
            reserved = reserved
                .checked_add(bytes)
                .ok_or(MathError::WorkspaceExhausted)?;
            for root in roots {
                let physical = chart_coordinates(root.enclosure(), sign, math)?;
                match source_membership(&physical, applicability) {
                    Membership::Outside => continue,
                    Membership::Unresolved => return Err(MathError::PrecisionExhausted),
                    Membership::Inside => {}
                }
                let candidate = PhysicalRoot {
                    sign,
                    u: root.enclosure()[0].clone(),
                    coordinates: physical,
                };
                retain_distinct(candidate, &mut retained, &mut system, math, &mut reserved)?;
            }
            math.release_workspace(bytes)?;
            reserved -= bytes;
        }
        if retained.len() > 1 {
            return Err(MathError::AmbiguousRoots {
                roots: retained.len(),
            });
        }
        let physical = retained
            .pop()
            .ok_or(MathError::Domain(
                "GCJ inverse has no root in the explicitly declared source applicability",
            ))?
            .coordinates;
        let mut result = OperationCoordinates::from_array(physical);
        if let Some(z) = coordinates.get(2) {
            result.push(z.clone());
        }
        Ok(result)
    })();
    math.release_workspace(reserved)?;
    result
}

struct PhysicalRoot {
    sign: i64,
    u: FixedInterval,
    coordinates: RootBox2,
}

fn retain_distinct(
    candidate: PhysicalRoot,
    retained: &mut Vec<PhysicalRoot>,
    system: &mut GcjSystem<'_, '_>,
    math: &mut CoordinateMath,
    reserved: &mut usize,
) -> Result<(), MathError> {
    for previous in retained.iter_mut() {
        if previous.sign == candidate.sign
            || previous.u.lower() > &BigInt::zero()
            || candidate.u.lower() > &BigInt::zero()
        {
            continue;
        }
        // Two charts describe the same point only at an exactly proved cusp.
        // Equal rounded coordinates never establish a root alias.
        if !previous.u.is_exact_zero() || !candidate.u.is_exact_zero() {
            return Err(MathError::PrecisionExhausted);
        }
        let latitude = FixedInterval::from_bounds(
            previous.coordinates[1]
                .lower()
                .clone()
                .min(candidate.coordinates[1].lower().clone()),
            previous.coordinates[1]
                .upper()
                .clone()
                .max(candidate.coordinates[1].upper().clone()),
            math,
        )?;
        let cusp = [FixedInterval::from_i64(0, math)?, latitude];
        if system.jacobian(&cusp, math)?[1][1].lower() <= &BigInt::zero() {
            return Err(MathError::PrecisionExhausted);
        }
        previous.coordinates[1] = previous.coordinates[1]
            .intersection(&candidate.coordinates[1], math)?
            .ok_or(MathError::PrecisionExhausted)?;
        return Ok(());
    }
    let bytes = candidate
        .coordinates
        .iter()
        .chain(std::iter::once(&candidate.u))
        .try_fold(size_of::<PhysicalRoot>() + 64, |bytes, interval| {
            bytes.checked_add(interval.workspace_bytes())
        })
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    *reserved = reserved
        .checked_add(bytes)
        .ok_or(MathError::WorkspaceExhausted)?;
    retained
        .try_reserve_exact(1)
        .map_err(|_| MathError::WorkspaceExhausted)?;
    retained.push(candidate);
    Ok(())
}

pub(super) fn certify_quantized(
    applicability: &Applicability,
    target: &RootBox2,
    output: &[Rat],
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    if !applicability.contains(&output[0], &output[1]) {
        return Err(MathError::PrecisionExhausted);
    }
    let source = [
        fixed_from_rat(&output[0], math)?,
        fixed_from_rat(&output[1], math)?,
    ];
    let image = forward::gcj(&source, math)?;
    let tolerance = decimal("0.000000000001", math)?;
    for axis in 0..2 {
        if image[axis].sub(&target[axis], math)?.abs(math)?.upper() > tolerance.lower() {
            return Err(MathError::PrecisionExhausted);
        }
    }
    Ok(())
}

struct GcjSystem<'a, 'observer> {
    sign: i64,
    quantize: bool,
    target: &'a RootBox2,
    progress: &'a mut WorkProgress<'observer>,
}

impl RootSystem2 for GcjSystem<'_, '_> {
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        let coordinates = chart_coordinates(domain, self.sign, math)?;
        let x = signed_square(&domain[0], self.sign, math)?;
        let image = forward::gcj_with_root(&coordinates, &x, &domain[0], math)?;
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
        jacobian(domain, self.sign, math)
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        if !self.quantize {
            return root_enclosure_complete(
                enclosure,
                self.target,
                &self.jacobian(enclosure, math)?,
                15,
                false,
                math,
            );
        }
        let physical = chart_coordinates(enclosure, self.sign, math)?;
        for coordinate in physical {
            let (lower, upper) = coordinate.round_decimal(15, math)?;
            if lower != upper {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn poll(&mut self, math: &CoordinateMath) -> Result<(), MathError> {
        self.progress.math_poll(math)
    }
}

fn signed_square(
    u: &FixedInterval,
    sign: i64,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let square = u.square(math)?;
    if sign < 0 {
        square.neg(math)
    } else {
        Ok(square)
    }
}
fn chart_coordinates(
    domain: &RootBox2,
    sign: i64,
    math: &mut CoordinateMath,
) -> Result<RootBox2, MathError> {
    Ok([
        FixedInterval::from_i64(105, math)?.add(&signed_square(&domain[0], sign, math)?, math)?,
        domain[1].clone(),
    ])
}

fn chart_domain(
    source: &RootBox2,
    centre: &FixedInterval,
    sign: i64,
    math: &mut CoordinateMath,
) -> Result<Option<RootBox2>, MathError> {
    let distance = if sign < 0 {
        centre.sub(&source[0], math)?
    } else {
        source[0].sub(centre, math)?
    };
    if distance.upper() < &BigInt::zero() {
        return Ok(None);
    }
    let distance = FixedInterval::from_bounds(
        distance.lower().clone().max(BigInt::zero()),
        distance.upper().clone(),
        math,
    )?;
    Ok(Some([distance.sqrt(math)?, source[1].clone()]))
}

// Absolute harmonic amplitudes are summed analytically. This bound covers all
// oscillations, regardless of how wide the applicability is; it is an exclusion
// certificate, not an iterative seed or an assumption about national geography.
fn narrow_domain(
    applicability: &Applicability,
    target: &RootBox2,
    math: &mut CoordinateMath,
) -> Result<RootBox2, MathError> {
    let mut source = [
        rational_interval(&applicability.west, &applicability.east, math)?,
        rational_interval(&applicability.south, &applicability.north, math)?,
    ];
    let x = absolute_upper(
        &source[0].sub(&FixedInterval::from_i64(105, math)?, math)?,
        math,
    )?;
    let y = absolute_upper(
        &source[1].sub(&FixedInterval::from_i64(35, math)?, math)?,
        math,
    )?;
    let xy = x.mul(&y, math)?;
    let root = x.sqrt(math)?;
    let harmonic_factor =
        FixedInterval::from_i64(2, math)?.div(&FixedInterval::from_i64(3, math)?, math)?;
    let latitude_bound = FixedInterval::from_i64(100, math)?
        .add(&x.mul(&FixedInterval::from_i64(2, math)?, math)?, math)?
        .add(&y.mul(&FixedInterval::from_i64(3, math)?, math)?, math)?
        .add(&y.square(math)?.mul(&decimal("0.2", math)?, math)?, math)?
        .add(&xy.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&root.mul(&decimal("0.2", math)?, math)?, math)?
        .add(
            &harmonic_factor.mul(&FixedInterval::from_i64(580, math)?, math)?,
            math,
        )?;
    let longitude_bound = FixedInterval::from_i64(300, math)?
        .add(&x, math)?
        .add(&y.mul(&FixedInterval::from_i64(2, math)?, math)?, math)?
        .add(&x.square(math)?.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&xy.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&root.mul(&decimal("0.1", math)?, math)?, math)?
        .add(
            &harmonic_factor.mul(&FixedInterval::from_i64(550, math)?, math)?,
            math,
        )?;
    let maximum_latitude = absolute_upper(&source[1], math)?;
    let cosine_minimum = radians(&maximum_latitude, math)?.sin_cos(math)?.1;
    if cosine_minimum.lower() <= &BigInt::zero() {
        return Err(MathError::PrecisionExhausted);
    }
    let a = FixedInterval::from_i64(6_378_245, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let longitude_scale = degrees(&one.div(&a.mul(&cosine_minimum, math)?, math)?, math)?;
    let latitude_scale = degrees(
        &one.div(
            &a.mul(
                &one.sub(&decimal("0.00669342162296594323", math)?, math)?,
                math,
            )?,
            math,
        )?,
        math,
    )?;
    for (axis, bound) in [
        longitude_bound.mul(&longitude_scale, math)?,
        latitude_bound.mul(&latitude_scale, math)?,
    ]
    .iter()
    .enumerate()
    {
        let candidate = FixedInterval::from_bounds(
            target[axis].lower().sub(bound.upper()),
            target[axis].upper().add(bound.upper()),
            math,
        )?;
        source[axis] = source[axis]
            .intersection(&candidate, math)?
            .ok_or(MathError::Domain(
                "GCJ inverse target is outside the image of its explicit source applicability",
            ))?;
    }
    Ok(source)
}

// Convert the independently differentiated cusp-chart law back to longitude
// only when the complete panel excludes the cusp. At L=105 the square-root
// term has no finite longitude derivative; callers retain the image enclosure
// and split the original parameter instead of inventing a Lipschitz constant.
pub(super) fn forward_jacobian(
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<Option<RootJacobian2>, MathError> {
    let x = coordinates[0].sub(&FixedInterval::from_i64(105, math)?, math)?;
    let zero = BigInt::zero();
    let sign = if x.lower() > &zero {
        1
    } else if x.upper() < &zero {
        -1
    } else {
        return Ok(None);
    };
    let u = x.abs(math)?.sqrt(math)?;
    if u.lower() <= &zero {
        return Ok(None);
    }
    let dx_du = u.mul(&FixedInterval::from_i64(2 * sign, math)?, math)?;
    let mut result = jacobian(&[u, coordinates[1].clone()], sign, math)?;
    for row in &mut result {
        row[0] = row[0].div(&dx_du, math)?;
    }
    Ok(Some(result))
}

fn jacobian(
    domain: &RootBox2,
    sign: i64,
    math: &mut CoordinateMath,
) -> Result<RootJacobian2, MathError> {
    let x = signed_square(&domain[0], sign, math)?;
    let y = domain[1].sub(&FixedInterval::from_i64(35, math)?, math)?;
    let dx = domain[0].mul(&FixedInterval::from_i64(2 * sign, math)?, math)?;
    let two_thirds =
        FixedInterval::from_i64(2, math)?.div(&FixedInterval::from_i64(3, math)?, math)?;
    let shared =
        harmonic_derivative(&x, &[(6, 1, 20), (2, 1, 20)], math)?.mul(&two_thirds, math)?;
    let latitude_y = harmonic_derivative(
        &y,
        &[(1, 1, 20), (1, 3, 40), (1, 12, 160), (1, 30, 320)],
        math,
    )?
    .mul(&two_thirds, math)?;
    let longitude_x = harmonic_derivative(
        &x,
        &[(1, 1, 20), (1, 3, 40), (1, 12, 150), (1, 30, 300)],
        math,
    )?
    .mul(&two_thirds, math)?;
    let tb_u = FixedInterval::from_i64(2, math)?
        .add(&y.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&shared, math)?
        .mul(&dx, math)?
        .add(&decimal("0.2", math)?, math)?;
    let tb_b = FixedInterval::from_i64(3, math)?
        .add(&y.mul(&decimal("0.4", math)?, math)?, math)?
        .add(&x.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&latitude_y, math)?;
    let tl_u = FixedInterval::from_i64(1, math)?
        .add(&x.mul(&decimal("0.2", math)?, math)?, math)?
        .add(&y.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&shared, math)?
        .add(&longitude_x, math)?
        .mul(&dx, math)?
        .add(&decimal("0.1", math)?, math)?;
    let tl_b =
        FixedInterval::from_i64(2, math)?.add(&x.mul(&decimal("0.1", math)?, math)?, math)?;
    let [tl, tb] = forward::gcj_corrections(&x, &domain[1], &domain[0], math)?;
    let [cl, cb] = forward::gcj_scales(&domain[1], math)?;
    let [cl_b, cb_b] = scale_derivatives(&domain[1], &cl, &cb, math)?;
    Ok([
        [
            dx.add(&tl_u.mul(&cl, math)?, math)?,
            tl_b.mul(&cl, math)?.add(&tl.mul(&cl_b, math)?, math)?,
        ],
        [
            tb_u.mul(&cb, math)?,
            FixedInterval::from_i64(1, math)?
                .add(&tb_b.mul(&cb, math)?, math)?
                .add(&tb.mul(&cb_b, math)?, math)?,
        ],
    ])
}

fn harmonic_derivative(
    coordinate: &FixedInterval,
    terms: &[(i64, i64, i64)],
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let mut result = FixedInterval::from_i64(0, math)?;
    for &(numerator, denominator, amplitude) in terms {
        let frequency = FixedInterval::pi(math)?
            .mul(&FixedInterval::from_i64(numerator, math)?, math)?
            .div(&FixedInterval::from_i64(denominator, math)?, math)?;
        let cosine = coordinate.mul(&frequency, math)?.sin_cos_range(math)?.1;
        result = result.add(
            &cosine
                .mul(&frequency, math)?
                .mul(&FixedInterval::from_i64(amplitude, math)?, math)?,
            math,
        )?;
    }
    Ok(result)
}

fn scale_derivatives(
    latitude: &FixedInterval,
    cl: &FixedInterval,
    cb: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 2], MathError> {
    let (sine, cosine) = radians(latitude, math)?.sin_cos_range(math)?;
    let e2 = decimal("0.00669342162296594323", math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let w2 = one.sub(&e2.mul(&sine.square(math)?, math)?, math)?;
    let logarithmic_w = e2.mul(&sine, math)?.mul(&cosine, math)?.div(&w2, math)?;
    let degree_scale = radians(&one, math)?;
    Ok([
        cl.mul(&sine.div(&cosine, math)?.sub(&logarithmic_w, math)?, math)?
            .mul(&degree_scale, math)?,
        cb.mul(&logarithmic_w, math)?
            .mul(&FixedInterval::from_i64(-3, math)?, math)?
            .mul(&degree_scale, math)?,
    ])
}

enum Membership {
    Inside,
    Outside,
    Unresolved,
}
fn source_membership(source: &RootBox2, applicability: &Applicability) -> Membership {
    let (west, east) = exact_bounds(&source[0]);
    let (south, north) = exact_bounds(&source[1]);
    if east < applicability.west
        || west > applicability.east
        || north < applicability.south
        || south > applicability.north
    {
        Membership::Outside
    } else if applicability.contains(&west, &south) && applicability.contains(&east, &north) {
        Membership::Inside
    } else {
        Membership::Unresolved
    }
}
fn absolute_upper(
    value: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let value = value.abs(math)?;
    FixedInterval::from_bounds(value.upper().clone(), value.upper().clone(), math)
}
fn rational_interval(
    lower: &Rat,
    upper: &Rat,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let lower = fixed_from_rat(lower, math)?;
    let upper = fixed_from_rat(upper, math)?;
    FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)
}
#[cfg(test)]
mod tests;
