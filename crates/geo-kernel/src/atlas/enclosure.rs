// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete original-curve exclusion against closed angular atlas rectangles.
//! A straddling witness is an exact original endpoint or a certified curve
//! point inside the rectangle. Otherwise every original parameter panel must
//! be proved outside; unresolved tangencies and arithmetic never become success.

use super::arcs::{self, ChartLine};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, exact_rational, fixed_from_rat, geo_math_error};
use crate::{Coord, GeoError, MetricContext, PreparedEdge, PreparedPolygon, PreparedRegion, Rat};
use purrdf_xsd::{
    BigInt,
    integer::ExactOperation,
    math::{CoordinateMath, FixedInterval, MathError},
};

pub(crate) fn boundary_intersects_rectangle(
    region: &PreparedRegion,
    west: &Rat,
    east: &Rat,
    south: &Rat,
    north: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        region
    else {
        return Ok(false);
    };
    edges_intersect_rectangle(
        polygons
            .iter()
            .flat_map(PreparedPolygon::rings)
            .flat_map(crate::PreparedCurve::edges),
        [west, east, south, north],
        context,
        progress,
    )
}
pub(super) fn ring_intersects_rectangle(
    curve: &crate::PreparedCurve,
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    edges_intersect_rectangle(curve.edges().iter(), rectangle, context, progress)
}
fn edges_intersect_rectangle<'a>(
    edges: impl IntoIterator<Item = &'a PreparedEdge>,
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    for edge in edges {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if exact_intersection(edge, &rectangle, context, progress)? {
            return Ok(true);
        }
        if matches!(edge, PreparedEdge::SourceLinear(_)) {
            continue;
        }
        let mut bits = 80.min(context.policy().limits().max_precision_bits);
        loop {
            match curve_intersection(edge, &rectangle, bits, context, progress) {
                Err(GeoError::PrecisionExhausted { .. })
                    if bits < context.policy().limits().max_precision_bits =>
                {
                    bits = bits
                        .saturating_mul(2)
                        .min(context.policy().limits().max_precision_bits);
                }
                result => {
                    if result? {
                        return Ok(true);
                    }
                    break;
                }
            }
        }
    }
    Ok(false)
}

/// Classify a closed rectangle against the selected physical union boundary,
/// prepared once by the arrangement home, without restoring internal walls.
pub(crate) fn selected_boundary_intersects_rectangle(
    boundary: &super::boundary::RegionBoundary,
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    for [a, b] in &boundary.edges {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if linear_intersection(a, b, rectangle, &mut ExactAdmission::new(context, progress))? {
            return Ok(true);
        }
    }
    if let Some(native) = &boundary.native {
        for fragment in native.fragments() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if fragment_intersection(fragment, rectangle, context, progress)? {
                return Ok(true);
            }
        }
        for point in native.isolated_points() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if parameter_intersection(
                point.original_edge(),
                [point.parameter(); 2],
                rectangle,
                context,
                progress,
            )? {
                return Ok(true);
            }
        }
    }
    edges_intersect_rectangle(
        boundary.curves.iter().flat_map(crate::PreparedCurve::edges),
        rectangle,
        context,
        progress,
    )
}

fn fragment_intersection(
    fragment: &arcs::NativeBoundaryFragment,
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    parameter_intersection(
        fragment.original_edge(),
        [&fragment.parameters()[0], &fragment.parameters()[1]],
        rectangle,
        context,
        progress,
    )
}

fn parameter_intersection(
    edge: &PreparedEdge,
    original: [&arcs::SourceParameter; 2],
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let limit = context.policy().limits().max_precision_bits;
    let mut bits = 80.min(limit);
    let mut refined = None;
    let mut retained = 0;
    let result = (|| loop {
        let parameters = refined
            .as_ref()
            .map_or(original, |parameters: &[arcs::SourceParameter; 2]| {
                [&parameters[0], &parameters[1]]
            });
        let [first, last] = parameters;
        let span = [
            first.bounds().0,
            first.bounds().1,
            last.bounds().0,
            last.bounds().1,
        ];
        if let PreparedEdge::SourceLinear(line) = edge
            && let Some(result) =
                linear_fragment_intersection(line, span, rectangle, context, progress)?
        {
            return Ok(result);
        }
        let result = curve_intersection_span(edge, &rectangle, Some(span), bits, context, progress);
        match result {
            Err(GeoError::PrecisionExhausted { .. }) if bits < limit => {
                bits = bits.saturating_mul(2).min(limit);
                let storage = first
                    .refinement_storage_bound(bits)?
                    .checked_add(last.refinement_storage_bound(bits)?)
                    .ok_or(GeoError::ArithmeticOverflow("fragment cut storage"))?;
                context.admit_workspace(storage)?;
                let next = (|| {
                    Ok::<_, GeoError>([
                        first.refined_in(bits, context, progress)?,
                        last.refined_in(bits, context, progress)?,
                    ])
                })();
                match next {
                    Ok(next) => {
                        refined = Some(next);
                        context.release_workspace(retained)?;
                        retained = storage;
                    }
                    Err(error) => {
                        context.release_workspace(storage)?;
                        return Err(error);
                    }
                }
            }
            result => return result,
        }
    })();
    drop(refined);
    context.release_workspace(retained)?;
    result
}

/// The outer source-linear segment encloses every possible implicit endpoint;
/// the inner segment consists entirely of points on the actual fragment.
fn linear_fragment_intersection(
    line: &crate::SourceLinearEdge,
    span: [&Rat; 4],
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<bool>, GeoError> {
    let operands = [
        line.start().point().longitude(),
        line.end().point().longitude(),
        line.start().point().latitude(),
        line.end().point().latitude(),
        span[0],
        span[1],
        span[2],
        span[3],
    ];
    let mut width = operands
        .into_iter()
        .map(crate::numerical::rational_operand_bits)
        .max()
        .unwrap_or(0);
    for operation in [
        ExactOperation::RationalAdd,
        ExactOperation::RationalMultiply,
        ExactOperation::RationalAdd,
    ] {
        width = purrdf_xsd::integer::ExactArithmeticCost::for_operation(operation, width, 1)
            .ok_or(GeoError::ArithmeticOverflow(
                "fragment interpolation storage",
            ))?
            .output_bits;
    }
    let storage =
        purrdf_xsd::integer::ExactArithmeticCost::for_operation(ExactOperation::Linear, width, 1)
            .ok_or(GeoError::ArithmeticOverflow(
                "fragment interpolation storage",
            ))?
            .workspace_bytes
            .saturating_add(size_of::<[Rat; 4]>() as u64)
            .saturating_add(size_of::<[crate::LonLat; 4]>() as u64);
    context.admit_workspace(storage)?;
    let result = (|| {
        let zero = Rat::zero();
        let one = Rat::one();
        let parameters = span.map(|value| {
            let mut admission = ExactAdmission::new(context, progress);
            let value = if exact_rational(
                Some(&mut admission),
                ExactOperation::RationalCompare,
                &[value, &zero],
                || value < &zero,
            )? {
                &zero
            } else if exact_rational(
                Some(&mut admission),
                ExactOperation::RationalCompare,
                &[value, &one],
                || value > &one,
            )? {
                &one
            } else {
                value
            };
            admission.rational(ExactOperation::Linear, &[value], 1, || Ok(value.clone()))
        });
        let [a, b, c, d] = parameters;
        let parameters = [a?, b?, c?, d?];
        let points = parameters.each_ref().map(|parameter| {
            let [longitude, latitude] = {
                let mut admission = ExactAdmission::new(context, progress);
                [
                    crate::SourceLinearEdge::interpolate_ordinate_admitted(
                        line.start().point().longitude(),
                        line.end().point().longitude(),
                        parameter,
                        &mut admission,
                    )?,
                    crate::SourceLinearEdge::interpolate_ordinate_admitted(
                        line.start().point().latitude(),
                        line.end().point().latitude(),
                        parameter,
                        &mut admission,
                    )?,
                ]
            };
            let longitude_bound = Rat::from_i64(180);
            let latitude_bound = Rat::from_i64(90);
            let cost = crate::numerical::rational_cost(
                ExactOperation::RationalCompare,
                &[&longitude, &latitude, &longitude_bound, &latitude_bound],
                4,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "fragment coordinate admission",
            ))?;
            progress.exact(context, cost, || crate::LonLat::new(longitude, latitude))
        });
        let [a, b, c, d] = points;
        let points = [a?, b?, c?, d?];
        let mut admission = ExactAdmission::new(context, progress);
        if !linear_intersection(&points[0], &points[3], rectangle, &mut admission)? {
            return Ok(Some(false));
        }
        if exact_rational(
            Some(&mut admission),
            ExactOperation::RationalCompare,
            &[&parameters[1], &parameters[2]],
            || parameters[1] <= parameters[2],
        )? && linear_intersection(&points[1], &points[2], rectangle, &mut admission)?
        {
            return Ok(Some(true));
        }
        Ok(None)
    })();
    context.release_workspace(storage)?;
    result
}

fn point_intersection(
    endpoint: &crate::LonLat,
    rectangle: [&Rat; 4],
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    let [west, east, south, north] = rectangle;
    if exact_rational(
        Some(admission),
        ExactOperation::RationalCompare,
        &[endpoint.latitude(), south],
        || endpoint.latitude() < south,
    )? || exact_rational(
        Some(admission),
        ExactOperation::RationalCompare,
        &[endpoint.latitude(), north],
        || endpoint.latitude() > north,
    )? {
        return Ok(false);
    }
    if endpoint.is_pole() {
        return Ok(true);
    }
    for shift in [-360, 0, 360] {
        let shift = Rat::from_i64(shift);
        let longitude = exact_rational(
            Some(admission),
            ExactOperation::RationalAdd,
            &[endpoint.longitude(), &shift],
            || endpoint.longitude().add(&shift),
        )?;
        if !exact_rational(
            Some(admission),
            ExactOperation::RationalCompare,
            &[&longitude, west],
            || longitude < *west,
        )? && !exact_rational(
            Some(admission),
            ExactOperation::RationalCompare,
            &[&longitude, east],
            || longitude > *east,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn linear_intersection(
    start: &crate::LonLat,
    end: &crate::LonLat,
    rectangle: [&Rat; 4],
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    if point_intersection(start, rectangle, admission)?
        || point_intersection(end, rectangle, admission)?
    {
        return Ok(true);
    }
    for shift in [-360, 0, 360] {
        let shift = Rat::from_i64(shift);
        let a = exact_rational(
            Some(admission),
            ExactOperation::RationalAdd,
            &[start.longitude(), &shift],
            || start.longitude().add(&shift),
        )?;
        let b = exact_rational(
            Some(admission),
            ExactOperation::RationalAdd,
            &[end.longitude(), &shift],
            || end.longitude().add(&shift),
        )?;
        let [latitude_a, latitude_b] = admission.rational(
            ExactOperation::Linear,
            &[start.latitude(), end.latitude()],
            2,
            || Ok([start.latitude().clone(), end.latitude().clone()]),
        )?;
        if super::segment_rectangle_admitted(
            &Coord::xy(a, latitude_a),
            &Coord::xy(b, latitude_b),
            rectangle,
            admission,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn exact_intersection(
    edge: &PreparedEdge,
    rectangle: &[&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    if let PreparedEdge::SourceLinear(line) = edge {
        return linear_intersection(
            line.start().point(),
            line.end().point(),
            *rectangle,
            &mut admission,
        );
    }
    let endpoints = [
        edge.start().map(crate::PreparedCoordinate::point),
        match edge {
            PreparedEdge::ShortestGeodesic(arc) => Some(arc.end().point()),
            PreparedEdge::SourceLinear(_)
            | PreparedEdge::AzimuthLength(_)
            | PreparedEdge::Transformed(_) => None,
        },
    ];
    for endpoint in endpoints.into_iter().flatten() {
        if point_intersection(endpoint, *rectangle, &mut admission)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn curve_intersection(
    edge: &PreparedEdge,
    rectangle: &[&Rat; 4],
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    curve_intersection_span(edge, rectangle, None, bits, context, progress)
}

/// Parameter receipts enclose the complete original fragment. Only the inner
/// interval is an existence witness; uncertain endpoint tails are excluded only
/// after their complete images have been proved outside.
fn curve_intersection_span(
    edge: &PreparedEdge,
    rectangle: &[&Rat; 4],
    span: Option<[&Rat; 4]>,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let policy = context.policy();
    let poles = [
        *rectangle[2] == Rat::from_i64(-90),
        *rectangle[3] == Rat::from_i64(90),
    ];
    arcs::with_lines(
        core::slice::from_ref(edge),
        bits,
        context,
        progress,
        |lines, math, progress| {
            (|| {
                let radians =
                    FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
                let bounds = rectangle.map(|value| {
                    fixed_from_rat(value, math).and_then(|value| value.mul(&radians, math))
                });
                let [west, east, south, north] = bounds;
                let rectangle = AngularRectangle {
                    bounds: [west?, east?, south?, north?],
                    poles,
                };
                let unit = FixedInterval::from_bounds(
                    BigInt::zero(),
                    BigInt::from_i128(1).mul_pow2(bits),
                    math,
                )?;
                let (parameter, interior, endpoints) = if let Some([a0, a1, b0, b1]) = span {
                    let [a0, a1, b0, b1] = [a0, a1, b0, b1].map(|v| fixed_from_rat(v, math));
                    let (a0, a1, b0, b1) = (a0?, a1?, b0?, b1?);
                    let parameter =
                        FixedInterval::from_bounds(a0.lower().clone(), b1.upper().clone(), math)?
                            .intersection(&unit, math)?
                            .ok_or(MathError::PrecisionExhausted)?;
                    let interior = if a1.upper() <= b0.lower() {
                        FixedInterval::from_bounds(a1.upper().clone(), b0.lower().clone(), math)?
                            .intersection(&unit, math)?
                    } else {
                        None
                    };
                    let endpoints = [
                        FixedInterval::from_bounds(a0.lower().clone(), a1.upper().clone(), math)?,
                        FixedInterval::from_bounds(b0.lower().clone(), b1.upper().clone(), math)?,
                    ];
                    (parameter, interior, Some(endpoints))
                } else {
                    (unit.clone(), Some(unit), None)
                };
                if let Some(endpoints) = endpoints {
                    for endpoint in endpoints {
                        if image_inside_with(
                            &lines[0],
                            &endpoint,
                            &rectangle,
                            policy.limits().max_iterations,
                            false,
                            math,
                            progress,
                        )? {
                            return Ok(true);
                        }
                    }
                }
                let stack_bytes = (policy.limits().max_subdivision_levels as usize + 8)
                    .saturating_mul(
                        size_of::<(FixedInterval, u32)>() + bits.div_ceil(8) as usize * 4,
                    )
                    .saturating_mul(4);
                math.reserve_workspace(stack_bytes)?;
                let mut pending =
                    purrdf_lex::walk::WorkList::<(FixedInterval, u32), 8>::with((parameter, 0));
                while let Some((panel, depth)) = pending.pop() {
                    progress.math_poll(math)?;
                    if image_outside(
                        &lines[0],
                        &panel,
                        &rectangle,
                        policy.limits().max_iterations,
                        math,
                        progress,
                    )? {
                        continue;
                    }
                    let inside_parameters = interior.as_ref().is_some_and(|interior| {
                        panel.lower() >= interior.lower() && panel.upper() <= interior.upper()
                    });
                    if inside_parameters
                        && (poles[0] || poles[1])
                        && pole_contact(
                            &lines[0],
                            &panel,
                            poles,
                            policy.limits().max_iterations,
                            math,
                            progress,
                        )?
                    {
                        return Ok(true);
                    }
                    let middle = panel.midpoint(math)?;
                    if interior.as_ref().is_some_and(|interior| {
                        middle.lower() >= interior.lower() && middle.upper() <= interior.upper()
                    }) && image_inside(
                        &lines[0],
                        &middle,
                        &rectangle,
                        policy.limits().max_iterations,
                        math,
                        progress,
                    )? {
                        return Ok(true);
                    }
                    if depth >= policy.limits().max_subdivision_levels {
                        return Err(MathError::PrecisionExhausted);
                    }
                    for child in super::oriented::split_parameter(&panel, math)? {
                        pending.push((child, depth + 1));
                    }
                }
                Ok::<_, MathError>(false)
            })()
            .map_err(|error| geo_math_error(&error, policy))
        },
    )
}

struct AngularRectangle {
    bounds: [FixedInterval; 4],
    poles: [bool; 2],
}

fn image_outside(
    line: &ChartLine,
    parameter: &FixedInterval,
    rectangle: &AngularRectangle,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let AngularRectangle { bounds, poles } = rectangle;
    let ([longitude, latitude], _) = line.image(parameter, iterations, math, progress)?;
    if latitude.upper() < bounds[2].lower() || latitude.lower() > bounds[3].upper() {
        return Ok(true);
    }
    let half_pi = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(2, math)?, math)?;
    if (poles[1] && latitude.upper() >= half_pi.lower())
        || (poles[0] && latitude.lower() <= half_pi.neg(math)?.upper())
    {
        return Ok(false);
    }
    longitude_relation(&longitude, &bounds[0], &bounds[1], false, math, progress)
        .map(|overlap| !overlap)
}
fn image_inside(
    line: &ChartLine,
    parameter: &FixedInterval,
    rectangle: &AngularRectangle,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    image_inside_with(line, parameter, rectangle, iterations, true, math, progress)
}

fn image_inside_with(
    line: &ChartLine,
    parameter: &FixedInterval,
    rectangle: &AngularRectangle,
    iterations: u32,
    pole_witness: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let AngularRectangle { bounds, poles } = rectangle;
    if pole_witness
        && (poles[0] || poles[1])
        && pole_contact(line, parameter, *poles, iterations, math, progress)?
    {
        return Ok(true);
    }
    let ([longitude, latitude], _) = line.image(parameter, iterations, math, progress)?;
    if latitude.lower() < bounds[2].upper() || latitude.upper() > bounds[3].lower() {
        return Ok(false);
    }
    longitude_relation(&longitude, &bounds[0], &bounds[1], true, math, progress)
}

fn longitude_relation(
    longitude: &FixedInterval,
    west: &FixedInterval,
    east: &FixedInterval,
    inside: bool,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let period = FixedInterval::pi(math)?.mul(&FixedInterval::from_i64(2, math)?, math)?;
    let quotient = longitude.div(&period, math)?;
    let (first, last) = quotient.floor_bounds(math)?;
    let first = first
        .to_i128()
        .and_then(|value| value.checked_sub(1))
        .ok_or(MathError::WorkExhausted)?;
    let last = last
        .to_i128()
        .and_then(|value| value.checked_add(1))
        .ok_or(MathError::WorkExhausted)?;
    let count = last
        .checked_sub(first)
        .and_then(|value| value.checked_add(1))
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(
        purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            ExactOperation::Linear,
            u64::from(math.limits().precision_bits) + 128,
            count,
        )
        .ok_or(MathError::WorkExhausted)?,
    )?;
    for offset in first..=last {
        progress.math_poll(math)?;
        let offset =
            FixedInterval::from_ratio(&BigInt::from_i128(offset), &BigInt::from_i128(1), math)?
                .mul(&period, math)?;
        let west = west.add(&offset, math)?;
        let east = east.add(&offset, math)?;
        if inside {
            if longitude.lower() >= west.upper() && longitude.upper() <= east.lower() {
                return Ok(true);
            }
        } else if longitude.upper() >= west.lower() && longitude.lower() <= east.upper() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn pole_contact(
    line: &ChartLine,
    parameter: &FixedInterval,
    poles: [bool; 2],
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    let ChartLine::Geodesic(line) = line else {
        return Ok(false);
    };
    let image = line.continuation_enclosure_in(parameter, iterations, math, progress)?;
    let Some(events) = image.meridian_poles else {
        return Ok(false);
    };
    let first = events.first.to_i128().ok_or(MathError::WorkExhausted)?;
    let last = events.last.to_i128().ok_or(MathError::WorkExhausted)?;
    let count = last
        .checked_sub(first)
        .and_then(|value| value.checked_add(1))
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(
        purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            ExactOperation::Linear,
            u64::from(math.limits().precision_bits) + 128,
            count,
        )
        .ok_or(MathError::WorkExhausted)?,
    )?;
    for index in first..=last {
        progress.math_poll(math)?;
        if !poles[usize::from(index.rem_euclid(2) == 0)] {
            continue;
        }
        let numerator = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(MathError::WorkExhausted)?;
        let phase =
            FixedInterval::from_ratio(&BigInt::from_i128(numerator), &BigInt::from_i128(2), math)?
                .mul(&events.pi, math)?;
        // Strict containment of a pole phase in the full monotone sigma image
        // proves an original-curve pole event by the intermediate value theorem.
        if image.sigma.lower() < phase.lower() && phase.upper() < image.sigma.upper() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeographicReference, LonLat, PreparedCoordinate, ShortestGeodesicArc};
    fn arc(start: (i64, i64), end: (i64, i64), context: &mut MetricContext) -> PreparedEdge {
        let coordinate = |(lon, lat)| {
            PreparedCoordinate::new(
                Coord::xy(Rat::from_i64(lon), Rat::from_i64(lat)),
                &GeographicReference::wgs84(),
            )
            .expect("original coordinate")
        };
        PreparedEdge::ShortestGeodesic(Box::new(
            ShortestGeodesicArc::new(coordinate(start), coordinate(end), context)
                .expect("unique branch"),
        ))
    }
    #[test]
    fn complete_arc_rectangle_exclusion_uses_true_image_and_identified_pole() {
        let mut limits = *crate::ExecutionPolicy::geometry().limits();
        limits.max_work_items = 2_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).expect("work admission"),
        )
        .expect("context");
        let edge = arc((-50, -25), (50, -25), &mut context);
        for (box_degrees, expected) in [
            ([-1, 1, -45, -26], true),
            ([-1, 1, 10, 20], false),
            ([100, 120, -90, 90], false),
        ] {
            let bounds = box_degrees.map(Rat::from_i64);
            assert_eq!(
                curve_intersection(
                    &edge,
                    &bounds.each_ref(),
                    80,
                    &mut context,
                    &mut WorkProgress::new(None)
                )
                .expect("complete original image"),
                expected
            );
        }
        let polar = arc((0, 70), (180, 80), &mut context);
        let bounds = [20, 25, 85, 90].map(Rat::from_i64);
        assert!(
            curve_intersection(
                &polar,
                &bounds.each_ref(),
                80,
                &mut context,
                &mut WorkProgress::new(None)
            )
            .expect("explicit interior pole event")
        );
        let pole = LonLat::new(Rat::from_i64(-75), Rat::from_i64(90)).expect("identified pole");
        assert!(
            crate::atlas::point::contact(&polar, &pole, &mut context).expect("same physical pole")
        );
    }
}
