// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One source-law point/curve contact predicate for all atlas consumers.

use super::arcs;
use crate::geographic::same_longitude as same_meridian;
use crate::{
    GeoError, LonLat, MetricContext, MetricWorkObserver, PreparedEdge, Rat,
    context::WorkProgress,
    numerical::{ExactAdmission, exact_rational, geo_math_error},
};
use purrdf_core::SmallVec;
use purrdf_xsd::{
    BigInt,
    integer::{ExactArithmeticCost, ExactOperation},
    math::{CoordinateMath, FixedInterval},
};

/// Decide whether an exact original geographic point lies on a modeled edge.
///
/// Exact source, seam, pole and symmetry witnesses prove contact. Complete
/// unrounded normal-image separation proves noncontact. Unknown equality never
/// becomes an approximate contact and exhausts precision or other admission.
///
/// # Errors
/// Refuses reference mismatch, unresolved contact, and numerical/governor limits.
pub fn contact(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
) -> Result<bool, GeoError> {
    observed(edge, point, context, None)
}
/// Decide the same original-law contact with bounded governor charging.
///
/// # Errors
/// Adds observer refusal to [`contact`].
pub fn contact_metered(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<bool, GeoError> {
    observed(edge, point, context, Some(observer))
}
fn observed(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<bool, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    arcs::check_bindings(&[edge], context, &mut progress)?;
    let payload = [point.longitude(), point.latitude()].into_iter().fold(
        arcs::source_workspace_bound(core::iter::once(edge)),
        |sum, v| {
            sum.saturating_add(
                v.numerator()
                    .bit_len()
                    .saturating_add(v.denominator().bit_len())
                    .div_ceil(8)
                    .saturating_mul(32),
            )
        },
    );
    context.admit_workspace(payload)?;
    let result = contact_with_progress(edge, point, context, &mut progress);
    context.release_workspace(payload)?;
    result
}

pub(crate) fn contact_with_progress(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    // Public entry or enclosing atlas inventory validated the immutable source
    // binding once. Internal sectors and graph faces retain that same context.
    if let Some(value) = exact_source(edge, point, context, progress)? {
        return Ok(value);
    }
    let policy = context.policy();
    let mut bits = 80.min(policy.limits().max_precision_bits);
    loop {
        let result = attempt(edge, point, bits, context, progress);
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result,
        }
    }
}

pub(super) fn exact_source(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<bool>, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let mut operands = [point.longitude(), point.latitude()]
        .into_iter()
        .collect::<SmallVec<[&Rat; 6]>>();
    if let Some(start) = edge.start() {
        operands.extend([start.point().longitude(), start.point().latitude()]);
    }
    match edge {
        PreparedEdge::SourceLinear(line) => operands.extend([
            line.end().point().longitude(),
            line.end().point().latitude(),
        ]),
        PreparedEdge::ShortestGeodesic(arc) => {
            operands.extend([arc.end().point().longitude(), arc.end().point().latitude()]);
        }
        _ => {}
    }
    let known = admission.rational(ExactOperation::RationalCompare, &operands, 16, || {
        if edge.start().is_some_and(|p| p.point().same_location(point)) {
            return Ok(Some(true));
        }
        match edge {
            PreparedEdge::SourceLinear(line) => {
                if line.end().point().same_location(point) {
                    return Ok(Some(true));
                }
                let a = line.start().point().latitude();
                let b = line.end().point().latitude();
                if point.latitude() < a.min(b) || point.latitude() > a.max(b) {
                    return Ok(Some(false));
                }
                Ok(None)
            }
            PreparedEdge::ShortestGeodesic(arc) => {
                if arc.end().point().same_location(point) {
                    Ok(Some(true))
                } else {
                    Ok(None)
                }
            }
            PreparedEdge::AzimuthLength(arc) if arc.length().exact().is_zero() => Ok(Some(false)),
            PreparedEdge::AzimuthLength(_) | PreparedEdge::Transformed(_) => Ok(None),
        }
    })?;
    if known.is_some() {
        return Ok(known);
    }
    match edge {
        PreparedEdge::SourceLinear(line) => linear_parameters(line, point, &mut admission)
            .map(|parameters| Some(!parameters.is_empty())),
        PreparedEdge::ShortestGeodesic(arc) => {
            let symmetry = shortest_symmetry(
                arc.start().point(),
                arc.end().point(),
                point,
                &mut admission,
            )?;
            if symmetry.is_some() {
                return Ok(symmetry);
            }
            let midpoint = midpoint_candidate(arc, point, &mut admission)?;
            if !midpoint {
                return Ok(None);
            }
            midpoint_contact(edge, point, context, progress).map(Some)
        }

        PreparedEdge::Transformed(_) => {
            let bits = 80.min(context.policy().limits().max_precision_bits);
            let policy = context.policy();
            let result = arcs::with_lines(
                core::slice::from_ref(edge),
                bits,
                context,
                progress,
                |lines, math, progress| {
                    let unit = FixedInterval::from_bounds(
                        BigInt::zero(),
                        BigInt::from_i128(1).mul_pow2(bits),
                        math,
                    )
                    .map_err(|e| geo_math_error(&e, policy))?;
                    if let Some(contact) = axis_panel_witness(
                        &lines[0],
                        &unit,
                        point,
                        policy.limits().max_iterations,
                        math,
                        progress,
                    )
                    .map_err(|e| geo_math_error(&e, policy))?
                    {
                        return Ok(Some(contact));
                    }
                    let query = super::oriented::query_normal(point, math)
                        .map_err(|e| geo_math_error(&e, policy))?;
                    let half = FixedInterval::from_i64(1, math)
                        .and_then(|one| one.div(&FixedInterval::from_i64(2, math)?, math))
                        .map_err(|e| geo_math_error(&e, policy))?;
                    // Original transformed endpoints need not have a
                    // geographic writing. Exact normal equality at either
                    // endpoint supplies the same existence witness as at the
                    // midpoint; it does not claim uniqueness or completeness.
                    for parameter in [
                        FixedInterval::from_i64(0, math).map_err(|e| geo_math_error(&e, policy))?,
                        FixedInterval::from_i64(1, math).map_err(|e| geo_math_error(&e, policy))?,
                        half,
                    ] {
                        if midpoint_normal_witness(
                            &lines[0],
                            &parameter,
                            &query,
                            policy.limits().max_iterations,
                            math,
                            progress,
                        )
                        .map_err(|e| geo_math_error(&e, policy))?
                        {
                            return Ok(Some(true));
                        }
                    }
                    Ok(None)
                },
            );
            match result {
                Ok(value) => Ok(value),
                Err(GeoError::PrecisionExhausted { .. }) => Ok(None),
                Err(error) => Err(error),
            }
        }
        PreparedEdge::AzimuthLength(_) => Ok(None),
    }
}

fn midpoint_normal_witness(
    line: &arcs::ChartLine,
    parameter: &FixedInterval,
    query: &[FixedInterval; 3],
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, purrdf_xsd::math::MathError> {
    if !query.iter().all(|value| value.lower() == value.upper()) {
        return Ok(false);
    }
    let midpoint = parameter.midpoint(math)?;
    let (witness, _) = line.normal(&midpoint, iterations, math, progress)?;
    // An exact original dyadic parameter maps to an enclosure containing
    // one physical normal equal to the query. This is existence alone.
    Ok(witness
        .iter()
        .zip(query)
        .all(|(value, query)| value.lower() == value.upper() && value.lower() == query.lower()))
}

/// A complete continuous panel on one exact angular axis contains every value
/// between its endpoint images. This supplies existence for off-grid targets;
/// no numerical root proposal or rounded image coordinate supplies a witness.
fn axis_panel_witness(
    line: &arcs::ChartLine,
    parameter: &FixedInterval,
    point: &LonLat,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<bool>, purrdf_xsd::math::MathError> {
    let arcs::ChartLine::Transformed {
        curve,
        subdivisions,
    } = line
    else {
        return Ok(None);
    };
    if point.is_pole() {
        return Ok(None);
    }
    let limits = crate::operation::OperationSolverLimits {
        iterations,
        subdivisions: *subdivisions,
        quantize_inverse: false,
    };
    let (lower, upper) = crate::numerical::exact_bounds_in(parameter, math)?;
    let image = match curve.enclosure_in(&lower, &upper, limits, math, progress) {
        Err(purrdf_xsd::math::MathError::PrecisionExhausted) => return Ok(None),
        result => result?,
    };
    let latitude = crate::numerical::fixed_from_rat(point.latitude(), math)?;
    if image.latitude.upper() < latitude.lower() || latitude.upper() < image.latitude.lower() {
        return Ok(Some(false));
    }
    if image.derivative.is_none() {
        return Ok(None);
    }
    // A complete chart-cut exclusion plus the whole-panel differential proves
    // continuity in this one longitude chart. Wrapped endpoint values alone
    // cannot apply the intermediate value theorem across the identified cut.
    let cut = FixedInterval::from_i64(180, math)?;
    let negative_cut = cut.neg(math)?;
    if image.longitude.lower() <= negative_cut.lower() || image.longitude.upper() >= cut.upper() {
        return Ok(None);
    }
    let pole = FixedInterval::from_i64(90, math)?;
    let negative_pole = pole.neg(math)?;
    if image.latitude.lower() <= negative_pole.lower() || image.latitude.upper() >= pole.upper() {
        return Ok(None);
    }
    let longitude = crate::numerical::fixed_from_rat(point.longitude(), math)?;
    if image.longitude.upper() < longitude.lower() || longitude.upper() < image.longitude.lower() {
        return Ok(Some(false));
    }
    let coordinates = [image.longitude, image.latitude];
    let query = [longitude, latitude];
    let Some(constant) = coordinates.iter().zip(&query).position(|(image, query)| {
        image.lower() == image.upper()
            && query.lower() == query.upper()
            && image.lower() == query.lower()
    }) else {
        return Ok(None);
    };
    let varying = 1 - constant;
    let start = curve.enclosure_in(&lower, &lower, limits, math, progress)?;
    let end = curve.enclosure_in(&upper, &upper, limits, math, progress)?;
    let start = [start.longitude, start.latitude];
    let end = [end.longitude, end.latitude];
    let (start, end, query) = (&start[varying], &end[varying], &query[varying]);
    Ok(
        (start.upper() <= query.lower() && query.upper() <= end.lower()
            || end.upper() <= query.lower() && query.upper() <= start.lower())
        .then_some(true),
    )
}

/// All original written-coordinate visits. Pole endpoints retain their actual
/// parameters even when the query uses another longitude frame; nonpolar
/// 360-degree paths retain both endpoint visits rather than becoming constant.
fn linear_parameters(
    line: &crate::SourceLinearEdge,
    point: &LonLat,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<ParameterIntervals, GeoError> {
    let start = line.start().point();
    let end = line.end().point();
    let constant = admission.rational(
        ExactOperation::Linear,
        &[
            start.longitude(),
            start.latitude(),
            end.longitude(),
            end.latitude(),
        ],
        8,
        || {
            Ok(start == end
                || (start.is_pole() && end.is_pole() && start.latitude() == end.latitude()))
        },
    )?;
    if constant {
        let same = admission.rational(
            ExactOperation::Linear,
            &[
                start.longitude(),
                start.latitude(),
                point.longitude(),
                point.latitude(),
            ],
            8,
            || Ok(start.same_location(point)),
        )?;
        return Ok(if same {
            vec![(Rat::zero(), Rat::one())]
        } else {
            Vec::new()
        });
    }
    let mut output = Vec::with_capacity(2);
    let polar = admission.rational(ExactOperation::Linear, &[point.latitude()], 1, || {
        Ok(point.is_pole())
    })?;
    if polar {
        for (index, endpoint) in [start, end].into_iter().enumerate() {
            if admission.rational(
                ExactOperation::Linear,
                &[
                    endpoint.longitude(),
                    endpoint.latitude(),
                    point.longitude(),
                    point.latitude(),
                ],
                8,
                || Ok(endpoint.same_location(point)),
            )? {
                let parameter = Rat::from_i64(index as i64);
                output.push((parameter.clone(), parameter));
            }
        }
        return Ok(output);
    }
    for shift in [-360, 0, 360] {
        let shift = Rat::from_i64(shift);
        let longitude = exact_rational(
            Some(admission),
            ExactOperation::RationalAdd,
            &[point.longitude(), &shift],
            || point.longitude().add(&shift),
        )?;
        let alias = admission.rational(
            ExactOperation::RationalCompare,
            &[&longitude, point.latitude()],
            4,
            || Ok(LonLat::new(longitude.clone(), point.latitude().clone())),
        )?;
        let Ok(alias) = alias else { continue };
        if let Some(parameter) =
            crate::SourceLinearEdge::parameter_on_admitted(start, end, &alias, admission)?
        {
            let repeated = output.iter().try_fold(false, |repeated, (prior, _)| {
                admission.rational(ExactOperation::Linear, &[prior, &parameter], 2, || {
                    Ok(repeated || prior == &parameter)
                })
            })?;
            if !repeated {
                let upper = admission.rational(ExactOperation::Linear, &[&parameter], 1, || {
                    Ok(parameter.clone())
                })?;
                output.push((parameter, upper));
            }
        }
    }
    Ok(output)
}

/// Complete parameters of every independently proved exact point visit.
/// Unknown equality remains a precision refusal; it never becomes a contact.
pub(crate) fn parameter_enclosures(
    edge: &PreparedEdge,
    point: &LonLat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<ParameterIntervals, GeoError> {
    if edge.binding_id() != context.reference().id() {
        return Err(GeoError::MissingOperation {
            source: edge.binding_id().digest().to_string(),
            target: context.reference().id().digest().to_string(),
        });
    }
    if let PreparedEdge::SourceLinear(line) = edge {
        return linear_parameters(line, point, &mut ExactAdmission::new(context, progress));
    }
    if let PreparedEdge::AzimuthLength(arc) = edge
        && let Some(parameters) =
            axis_contact_parameters_with_precision(arc, point, bits, context, progress)?
    {
        return Ok(parameters);
    }
    if let PreparedEdge::Transformed(_) = edge
        && let Some(parameters) = monotone_parameters(edge, point, bits, context, progress)?
    {
        return Ok(parameters);
    }
    if !contact_with_progress(edge, point, context, progress)? {
        return Ok(Vec::new());
    }
    if let PreparedEdge::ShortestGeodesic(arc) = edge {
        let endpoint = ExactAdmission::new(context, progress).rational(
            ExactOperation::Linear,
            &[
                point.longitude(),
                point.latitude(),
                arc.start().point().longitude(),
                arc.start().point().latitude(),
                arc.end().point().longitude(),
                arc.end().point().latitude(),
            ],
            12,
            || {
                Ok(if point.same_location(arc.start().point()) {
                    Some(Rat::zero())
                } else if point.same_location(arc.end().point()) {
                    Some(Rat::one())
                } else {
                    None
                })
            },
        )?;
        if let Some(parameter) = endpoint {
            return Ok(vec![(parameter.clone(), parameter)]);
        }
        if midpoint_candidate(arc, point, &mut ExactAdmission::new(context, progress))? {
            // Contact is already proved above; this exact endpoint symmetry
            // has only the arclength midpoint as its possible fixed visit.
            let half = Rat::new(crate::Int::one(), crate::Int::from_i64(2)).expect("half");
            return Ok(vec![(half.clone(), half)]);
        }
        return shortest_parameter(arc, point, bits, context, progress);
    }
    Err(GeoError::PrecisionExhausted {
        bits: context.policy().limits().max_precision_bits,
    })
}

/// Complete exact visits of an injective normal component. A strict whole-
/// panel derivative proves uniqueness; exact singleton normal equality at an
/// original dyadic parameter proves existence independently of root rounding.
pub(crate) fn monotone_parameters(
    edge: &PreparedEdge,
    point: &LonLat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ParameterIntervals>, GeoError> {
    let policy = context.policy();
    let analytic = if let PreparedEdge::Transformed(image) = edge {
        image.analytic_in_parameter(context, progress)?
    } else {
        false
    };
    arcs::with_line_refs_with_precision(
        &[edge],
        bits,
        false,
        context,
        progress,
        |lines, math, progress| {
            let result = (|| {
                let query = super::oriented::query_normal(point, math)?;
                let whole = crate::numerical::fixed_from_bounds(&Rat::zero(), &Rat::one(), math)?;
                let (normal, derivative) =
                    lines[0].normal(&whole, policy.limits().max_iterations, math, progress)?;
                if normal.iter().zip(&query).any(|(value, query)| {
                    value.upper() < query.lower() || query.upper() < value.lower()
                }) {
                    return Ok(Some(Vec::new()));
                }
                let injective = injective_component(
                    &lines[0],
                    derivative.as_ref(),
                    (&whole, analytic, policy.limits().max_iterations),
                    math,
                    progress,
                )?;
                if !injective {
                    return Ok(None);
                }
                for numerator in [0, 1, 2] {
                    let parameter = FixedInterval::from_i64(numerator, math)?
                        .div(&FixedInterval::from_i64(2, math)?, math)?;
                    if midpoint_normal_witness(
                        &lines[0],
                        &parameter,
                        &query,
                        policy.limits().max_iterations,
                        math,
                        progress,
                    )? {
                        let cost = ExactArithmeticCost::dyadic_rational(
                            u64::from(numerator.unsigned_abs().bit_width()),
                            1,
                        )
                        .ok_or(purrdf_xsd::math::MathError::WorkExhausted)?;
                        math.admit_exact(cost.work_items, cost.output_bits as usize)?;
                        math.reserve_workspace(size_of::<(Rat, Rat)>())?;
                        // The original dyadic candidate is exactly 0, 1/2 or1.
                        // Inline integers avoid pinning a pooled destination.
                        let value = Rat::from_dyadic_integer(&crate::Int::from_i64(numerator), 1);
                        return Ok(Some(vec![(value.clone(), value)]));
                    }
                }
                Ok(None)
            })();
            result.map_err(|error| geo_math_error(&error, policy))
        },
    )
}

/// Complete injectivity shared by literal and symbolic point contacts.
pub(crate) fn image_injective(
    edge: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let analytic = if let PreparedEdge::Transformed(image) = edge {
        image.analytic_in_parameter(context, progress)?
    } else {
        false
    };
    let policy = context.policy();
    arcs::with_line_refs_with_precision(
        &[edge],
        80.min(policy.limits().max_precision_bits),
        false,
        context,
        progress,
        |lines, math, progress| {
            let result = (|| {
                let whole = crate::numerical::fixed_from_bounds(&Rat::zero(), &Rat::one(), math)?;
                let (_, derivative) =
                    lines[0].normal(&whole, policy.limits().max_iterations, math, progress)?;
                injective_component(
                    &lines[0],
                    derivative.as_ref(),
                    (&whole, analytic, policy.limits().max_iterations),
                    math,
                    progress,
                )
            })();
            result.map_err(|error| geo_math_error(&error, policy))
        },
    )
}

pub(super) fn injective_component(
    line: &arcs::ChartLine,
    derivative: Option<&[FixedInterval; 3]>,
    (whole, analytic, iterations): (&FixedInterval, bool, u32),
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, purrdf_xsd::math::MathError> {
    let strict = |value: &FixedInterval| {
        value.upper().is_negative() || (!value.lower().is_negative() && !value.lower().is_zero())
    };
    if derivative.is_some_and(|values| values.iter().any(strict)) {
        return Ok(true);
    }
    if !analytic {
        return Ok(false);
    }
    let (angles, rates) = line.image(whole, iterations, math, progress)?;
    let Some(rates) = rates else { return Ok(false) };
    let half = whole.midpoint(math)?;
    let (_, middle_rates) = line.image(&half, iterations, math, progress)?;
    let Some(middle_rates) = middle_rates else {
        return Ok(false);
    };
    let period = FixedInterval::pi(math)?.mul(&FixedInterval::from_i64(2, math)?, math)?;
    for axis in 0..2 {
        let weak = !rates[axis].lower().is_negative()
            || rates[axis].upper().is_negative()
            || rates[axis].upper().is_zero();
        let unaliased = axis == 1 || angles[axis].width(math)?.upper() < period.lower();
        // An analytic nonconstant component cannot be constant on an open
        // interval. The complete weak sign therefore gives strict monotonicity.
        if weak && strict(&middle_rates[axis]) && unaliased {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A certified on-curve point splits a unique shortest arc into shortest
/// subarcs. Its unrounded parameter is the first subarc length divided by the
/// original length; public rounded distance fields never enter this ratio.
fn shortest_parameter(
    arc: &crate::ShortestGeodesicArc,
    point: &LonLat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<ParameterIntervals, GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    let prepared = crate::PreparedGeodesic::new(reference);
    let mut retained = 0;
    let result = (|| {
        let mut views = Vec::with_capacity(2);
        for end in [point, arc.end().point()] {
            let mut child = context.remaining_child()?;
            let view = {
                let mut observer = progress.nested(
                    context.work_items(),
                    context
                        .policy()
                        .limits()
                        .max_workspace_bytes
                        .saturating_sub(context.remaining_workspace()),
                    context.workspace_peak(),
                );
                prepared.prepare_shortest_arc_view(
                    arc.start().point(),
                    end,
                    bits,
                    &mut child,
                    Some(&mut observer),
                )
            };
            let view = progress.absorb_child_result(context, &child, view)?;
            let storage = view.retained_workspace_bytes();
            context.admit_workspace(storage)?;
            retained += storage;
            let mut child = context.remaining_child()?;
            let fine = {
                let mut observer = progress.nested(
                    context.work_items(),
                    context
                        .policy()
                        .limits()
                        .max_workspace_bytes
                        .saturating_sub(context.remaining_workspace()),
                    context.workspace_peak(),
                );
                view.refined(bits, &mut child, Some(&mut observer))
            };
            let fine = progress.absorb_child_result(context, &child, fine)?;
            let storage = fine.retained_workspace_bytes();
            context.admit_workspace(storage)?;
            retained += storage;
            views.push(fine);
        }
        let first = views[0].length_bounds();
        let total = views[1].length_bounds();
        let mut admission = ExactAdmission::new(context, progress);
        let lower = exact_rational(
            Some(&mut admission),
            ExactOperation::RationalDivide,
            &[first.0.exact(), total.1.exact()],
            || first.0.exact().div(total.1.exact()),
        )?
        .ok_or_else(|| GeoError::domain("positive original shortest length"))?;
        let upper = exact_rational(
            Some(&mut admission),
            ExactOperation::RationalDivide,
            &[first.1.exact(), total.0.exact()],
            || first.1.exact().div(total.0.exact()),
        )?
        .ok_or_else(|| GeoError::domain("positive original shortest length"))?;
        Ok(vec![(lower, upper)])
    })();
    context.release_workspace(retained)?;
    result
}
fn midpoint_candidate(
    arc: &crate::ShortestGeodesicArc,
    point: &LonLat,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    if !point.latitude().is_zero() {
        return Ok(false);
    }
    let sum = exact_rational(
        Some(admission),
        ExactOperation::RationalAdd,
        &[arc.start().point().latitude(), arc.end().point().latitude()],
        || {
            arc.start()
                .point()
                .latitude()
                .add(arc.end().point().latitude())
        },
    )?;
    if !sum.is_zero() {
        return Ok(false);
    }
    let longitude = exact_rational(
        Some(admission),
        ExactOperation::RationalAdd,
        &[
            arc.start().point().longitude(),
            arc.end().point().longitude(),
        ],
        || {
            arc.start()
                .point()
                .longitude()
                .add(arc.end().point().longitude())
        },
    )?;
    let two = Rat::from_i64(2);
    let double_query = exact_rational(
        Some(admission),
        ExactOperation::RationalMultiply,
        &[point.longitude(), &two],
        || point.longitude().mul(&two),
    )?;
    let delta = exact_rational(
        Some(admission),
        ExactOperation::RationalAdd,
        &[&longitude, &double_query],
        || longitude.sub(&double_query),
    )?;
    admission.rational(ExactOperation::RationalCompare, &[&delta], 3, || {
        Ok(delta == Rat::zero()
            || delta.abs() == Rat::from_i64(360)
            || delta.abs() == Rat::from_i64(720))
    })
}

fn midpoint_contact(
    edge: &PreparedEdge,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    // The half-turn about the query's equatorial axis interchanges the exact
    // original endpoints. Uniqueness therefore fixes the arclength midpoint
    // at the query or its antipode. A shortest arc has no repeated point, so
    // either fixed point can occur only at that midpoint. Full normal-image
    // exclusion of the other fixed point proves exact contact, without a
    // tolerance or an interval-containing-zero equality assertion.
    let policy = context.policy();
    let mut bits = 80.min(policy.limits().max_precision_bits);
    loop {
        let result = arcs::with_lines(
            core::slice::from_ref(edge),
            bits,
            context,
            progress,
            |lines, math, progress| {
                let half =
                    FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(2), math)
                        .map_err(|error| geo_math_error(&error, policy))?;
                let (normal, _) = lines[0]
                    .normal(&half, policy.limits().max_iterations, math, progress)
                    .map_err(|error| geo_math_error(&error, policy))?;
                let query = super::oriented::query_normal(point, math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                if normal
                    .iter()
                    .zip(&query)
                    .any(|(a, b)| a.upper() < b.lower() || b.upper() < a.lower())
                {
                    return Ok(false);
                }
                for (a, b) in normal.iter().zip(&query) {
                    let antipode = b
                        .neg(math)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    if a.upper() < antipode.lower() || antipode.upper() < a.lower() {
                        return Ok(true);
                    }
                }
                Err(GeoError::PrecisionExhausted { bits })
            },
        );
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result,
        }
    }
}

fn between(value: &Rat, a: &Rat, b: &Rat) -> bool {
    a.min(b) <= value && value <= a.max(b)
}
pub(super) enum ShortestAxis {
    Equator,
    Meridian(Rat),
}
pub(super) fn shortest_axis(
    arc: &crate::ShortestGeodesicArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ShortestAxis>, GeoError> {
    let a = arc.start().point();
    let b = arc.end().point();
    let mut admission = ExactAdmission::new(context, progress);
    let equator = admission.rational(
        ExactOperation::RationalCompare,
        &[a.latitude(), b.latitude()],
        2,
        || Ok(a.latitude().is_zero() && b.latitude().is_zero()),
    )?;
    if equator {
        return Ok(Some(ShortestAxis::Equator));
    }
    let delta = exact_rational(
        Some(&mut admission),
        ExactOperation::RationalAdd,
        &[a.longitude(), b.longitude()],
        || a.longitude().sub(b.longitude()),
    )?;
    let meridian = admission.rational(
        ExactOperation::RationalCompare,
        &[a.latitude(), b.latitude(), &delta],
        8,
        || {
            Ok(a.is_pole()
                || b.is_pole()
                || same_meridian(a.longitude(), b.longitude())
                || delta.abs() == Rat::from_i64(180))
        },
    )?;
    if !meridian {
        return Ok(None);
    }
    let longitude = if a.is_pole() {
        b.longitude()
    } else {
        a.longitude()
    };
    admission.rational(ExactOperation::Linear, &[longitude], 1, || {
        Ok(Some(ShortestAxis::Meridian(longitude.clone())))
    })
}
fn shortest_symmetry(
    start: &LonLat,
    end: &LonLat,
    point: &LonLat,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<bool>, GeoError> {
    use ExactOperation::{RationalAdd, RationalCompare};
    if start.latitude().is_zero() && end.latitude().is_zero() {
        if !point.latitude().is_zero() {
            return Ok(Some(false));
        }
        let mut delta = exact_rational(
            Some(admission),
            RationalAdd,
            &[end.longitude(), start.longitude()],
            || end.longitude().sub(start.longitude()),
        )?;
        let full = Rat::from_i64(360);
        let reduce = admission.rational(RationalCompare, &[&delta], 2, || {
            Ok(delta.abs() > Rat::from_i64(180))
        })?;
        if reduce {
            delta = exact_rational(Some(admission), RationalAdd, &[&delta, &full], || {
                if delta.signum() > 0 {
                    delta.sub(&full)
                } else {
                    delta.add(&full)
                }
            })?;
        }
        let offset = exact_rational(
            Some(admission),
            RationalAdd,
            &[point.longitude(), start.longitude()],
            || point.longitude().sub(start.longitude()),
        )?;
        for shift in [-360, 0, 360] {
            let shift = Rat::from_i64(shift);
            let alias = exact_rational(Some(admission), RationalAdd, &[&offset, &shift], || {
                offset.add(&shift)
            })?;
            if admission.rational(RationalCompare, &[&alias, &delta], 4, || {
                Ok(between(&alias, &Rat::zero(), &delta))
            })? {
                return Ok(Some(true));
            }
        }
        return Ok(Some(false));
    }
    if start.is_pole() || end.is_pole() || same_meridian(start.longitude(), end.longitude()) {
        let longitude = if start.is_pole() {
            end.longitude()
        } else {
            start.longitude()
        };
        return admission.rational(
            RationalCompare,
            &[point.latitude(), start.latitude(), end.latitude()],
            4,
            || {
                Ok(Some(
                    (point.is_pole() || same_meridian(point.longitude(), longitude))
                        && between(point.latitude(), start.latitude(), end.latitude()),
                ))
            },
        );
    }
    let delta = exact_rational(
        Some(admission),
        RationalAdd,
        &[start.longitude(), end.longitude()],
        || start.longitude().sub(end.longitude()),
    )?;
    if delta.abs() == Rat::from_i64(180) {
        // Reflection fixes both endpoints; uniqueness fixes every point of the
        // shortest curve. The shorter meridian route follows the exact sign of
        // the latitude sum because the meridian scale factor is even.
        let sum = exact_rational(
            Some(admission),
            RationalAdd,
            &[start.latitude(), end.latitude()],
            || start.latitude().add(end.latitude()),
        )?;
        let pole = Rat::from_i64(if sum.signum() > 0 { 90 } else { -90 });
        return admission.rational(
            RationalCompare,
            &[point.latitude(), start.latitude(), end.latitude(), &pole],
            8,
            || {
                Ok(Some(
                    point.is_pole() && point.latitude() == &pole
                        || same_meridian(point.longitude(), start.longitude())
                            && between(point.latitude(), start.latitude(), &pole)
                        || same_meridian(point.longitude(), end.longitude())
                            && between(point.latitude(), end.latitude(), &pole),
                ))
            },
        );
    }
    Ok(None)
}

fn attempt(
    edge: &PreparedEdge,
    point: &LonLat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let policy = context.policy();
    let axis = if let PreparedEdge::AzimuthLength(arc) = edge {
        axis_hint(arc, point, context, progress)?
    } else {
        None
    };
    if matches!(axis, Some(AxisHint::Outside)) {
        return Ok(false);
    }
    if let (Some(axis), PreparedEdge::AzimuthLength(arc)) = (axis, edge) {
        let ellipsoid = context.reference().ellipsoid().clone();
        let reservation = (bits.div_ceil(8) as usize + 64).saturating_mul(8192);
        return crate::numerical::with_math(
            context,
            progress,
            bits,
            reservation,
            |math, progress| {
                let travel = axis
                    .distance(point, arc.start().point(), &ellipsoid, math, progress)
                    .map_err(|e| geo_math_error(&e, policy))?;
                let threshold = crate::numerical::fixed_from_rat(arc.length().exact(), math)
                    .map_err(|e| geo_math_error(&e, policy))?;
                if travel.upper() <= threshold.lower() {
                    return Ok(true);
                }
                if travel.lower() > threshold.upper() {
                    return Ok(false);
                }
                Err(GeoError::PrecisionExhausted { bits })
            },
        );
    }
    arcs::with_lines(
        core::slice::from_ref(edge),
        bits,
        context,
        progress,
        |lines, math, progress| {
            let q = super::oriented::query_normal(point, math)
                .map_err(|e| geo_math_error(&e, policy))?;
            let parameter = FixedInterval::from_bounds(
                BigInt::zero(),
                BigInt::from_i128(1).mul_pow2(bits),
                math,
            )
            .map_err(|e| geo_math_error(&e, policy))?;
            let mut pending =
                purrdf_lex::walk::WorkList::<(FixedInterval, u32), 8>::with((parameter, 0));
            while let Some((parameter, depth)) = pending.pop() {
                progress
                    .math_poll(math)
                    .map_err(|e| geo_math_error(&e, policy))?;
                if let Some(contact) = axis_panel_witness(
                    &lines[0],
                    &parameter,
                    point,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )
                .map_err(|e| geo_math_error(&e, policy))?
                {
                    if contact {
                        return Ok(true);
                    }
                    continue;
                }
                let (normal, _) = lines[0]
                    .normal(&parameter, policy.limits().max_iterations, math, progress)
                    .map_err(|e| geo_math_error(&e, policy))?;
                if normal
                    .iter()
                    .zip(&q)
                    .any(|(n, q)| n.upper() < q.lower() || q.upper() < n.lower())
                {
                    continue;
                }
                if midpoint_normal_witness(
                    &lines[0],
                    &parameter,
                    &q,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )
                .map_err(|e| geo_math_error(&e, policy))?
                {
                    return Ok(true);
                }
                if depth >= policy.limits().max_subdivision_levels {
                    return Err(GeoError::PrecisionExhausted { bits });
                }
                for child in super::oriented::split_parameter(&parameter, math)
                    .map_err(|e| geo_math_error(&e, policy))?
                {
                    pending.push((child, depth + 1));
                }
            }
            Ok(false)
        },
    )
}

enum AxisHint {
    Outside,
    Equator(Rat),
    Meridian {
        quarter_turns: i64,
        start_weight: i64,
        target_weight: i64,
    },
}

/// The exact equatorial conserved line covers the complete selected circle
/// once its original length is at least 2πa. This is a set certificate only;
/// repeated traversal remains part of the original scalar arclength law.
pub(crate) fn covers_equator(
    arc: &crate::AzimuthLengthArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let azimuth = normalize_admitted(arc.azimuth(), context, progress)?;
    let on_equator = ExactAdmission::new(context, progress).rational(
        ExactOperation::Linear,
        &[arc.start().point().latitude(), &azimuth],
        3,
        || {
            Ok(arc.start().point().latitude().is_zero()
                && (azimuth == Rat::from_i64(90) || azimuth == Rat::from_i64(270)))
        },
    )?;
    if !on_equator {
        return Ok(false);
    }
    let policy = context.policy();
    let cost = crate::numerical::rational_cost(
        ExactOperation::Linear,
        &[context.reference().ellipsoid().semimajor()],
        1,
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "equatorial circumference source",
    ))?;
    context.admit_workspace(cost.workspace_bytes)?;
    let major = match progress.exact_context(context, cost, |context| {
        Ok(context.reference().ellipsoid().semimajor().clone())
    }) {
        Ok(value) => value,
        Err(error) => {
            context.release_workspace(cost.workspace_bytes)?;
            return Err(error);
        }
    };
    let mut bits = 80.min(policy.limits().max_precision_bits);
    let result = (|| loop {
        let decision = crate::numerical::with_math_for_sources(
            context,
            progress,
            bits,
            4096,
            &[arc.length().exact(), &major],
            |math, _| {
                let result = (|| {
                    let length = crate::numerical::fixed_from_rat(arc.length().exact(), math)?;
                    let circumference = FixedInterval::pi(math)?
                        .mul(&FixedInterval::from_i64(2, math)?, math)?
                        .mul(&crate::numerical::fixed_from_rat(&major, math)?, math)?;
                    Ok(if length.lower() >= circumference.upper() {
                        Some(true)
                    } else if length.upper() < circumference.lower() {
                        Some(false)
                    } else {
                        None
                    })
                })();
                result.map_err(|error| geo_math_error(&error, policy))
            },
        )?;
        if let Some(value) = decision {
            return Ok(value);
        }
        if bits == policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        bits = bits
            .saturating_mul(2)
            .min(policy.limits().max_precision_bits);
    })();
    drop(major);
    context.release_workspace(cost.workspace_bytes)?;
    result
}
/// A conserved meridian has period four original quarter-meridian lengths.
/// Return its exact selected plane only after proving complete-period coverage.
pub(crate) fn covered_meridian(
    arc: &crate::AzimuthLengthArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<Rat>, GeoError> {
    let Some(plane) = crate::geodesic::meridian_plane_degrees_admitted(
        arc.start().point(),
        arc.azimuth(),
        context,
        progress,
        retained,
    )?
    else {
        return Ok(None);
    };
    let cost = crate::numerical::reference_copy_cost(context.reference())?;
    context.retain_workspace(cost.workspace_bytes, retained)?;
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    let mut bits = 80.min(policy.limits().max_precision_bits);
    loop {
        let decision = crate::numerical::with_math_for_sources(
            context,
            progress,
            bits,
            4096,
            &[arc.length().exact()],
            |math, progress| {
                let result = (|| {
                    let quarter = crate::geodesic::meridian_arc_observed(
                        &Rat::from_i64(90),
                        reference.ellipsoid(),
                        math,
                        progress,
                    )?;
                    let period = quarter.mul(&FixedInterval::from_i64(4, math)?, math)?;
                    let length = crate::numerical::fixed_from_rat(arc.length().exact(), math)?;
                    Ok(if length.lower() >= period.upper() {
                        Some(true)
                    } else if length.upper() < period.lower() {
                        Some(false)
                    } else {
                        None
                    })
                })();
                result.map_err(|error| geo_math_error(&error, policy))
            },
        )?;
        if let Some(covered) = decision {
            return Ok(covered.then_some(plane));
        }
        if bits == policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        bits = bits
            .saturating_mul(2)
            .min(policy.limits().max_precision_bits);
    }
}

impl AxisHint {
    fn distance(
        &self,
        target: &LonLat,
        start: &LonLat,
        ellipsoid: &crate::PreparedEllipsoid,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<FixedInterval, purrdf_xsd::math::MathError> {
        match self {
            Self::Outside => Err(purrdf_xsd::math::MathError::Domain("excluded axial point")),
            Self::Equator(degrees) => crate::numerical::fixed_from_rat(degrees, math)?
                .mul(&FixedInterval::pi(math)?, math)?
                .div(&FixedInterval::from_i64(180, math)?, math)?
                .mul(
                    &crate::numerical::fixed_from_rat(ellipsoid.semimajor(), math)?,
                    math,
                ),
            Self::Meridian {
                quarter_turns,
                start_weight,
                target_weight,
            } => {
                let a = crate::geodesic::meridian_arc_observed(
                    start.latitude(),
                    ellipsoid,
                    math,
                    progress,
                )?;
                let b = crate::geodesic::meridian_arc_observed(
                    target.latitude(),
                    ellipsoid,
                    math,
                    progress,
                )?;
                let mut travel = a
                    .mul(&FixedInterval::from_i64(*start_weight, math)?, math)?
                    .add(
                        &b.mul(&FixedInterval::from_i64(*target_weight, math)?, math)?,
                        math,
                    )?;
                if *quarter_turns != 0 {
                    let quarter = crate::geodesic::meridian_arc_observed(
                        &Rat::from_i64(90),
                        ellipsoid,
                        math,
                        progress,
                    )?;
                    travel = travel.add(
                        &quarter.mul(&FixedInterval::from_i64(*quarter_turns, math)?, math)?,
                        math,
                    )?;
                }
                Ok(travel)
            }
        }
    }
}
pub(super) fn normalize_admitted(
    angle: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    normalize_from_admitted(angle, 0, context, progress)
}
pub(super) fn longitude_admitted(
    angle: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    normalize_from_admitted(angle, -180, context, progress)
}
fn normalize_from_admitted(
    angle: &Rat,
    origin: i64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    // The degree law's fixed integer offsets are at most nine bits. All reduced
    // intermediates have width <=2B+16; four rational-add cost bounds include
    // both offset additions, modulus product/division and canonical reduction.
    let bits = angle
        .numerator()
        .bit_len()
        .max(angle.denominator().bit_len())
        .max(16)
        .checked_mul(2)
        .and_then(|b| b.checked_add(16))
        .ok_or(GeoError::ArithmeticOverflow("degree reduction admission"))?;
    let cost = ExactArithmeticCost::for_operation(ExactOperation::RationalAdd, bits, 4)
        .ok_or(GeoError::ArithmeticOverflow("degree reduction admission"))?;
    progress.exact(context, cost, || {
        Ok(crate::geographic::normalize_degrees(angle, origin))
    })
}
fn axis_hint(
    arc: &crate::AzimuthLengthArc,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<AxisHint>, GeoError> {
    let azimuth = normalize_admitted(arc.azimuth(), context, progress)?;
    let start = arc.start().point();
    if start.latitude().is_zero() && (azimuth == Rat::from_i64(90) || azimuth == Rat::from_i64(270))
    {
        if !point.latitude().is_zero() {
            return Ok(Some(AxisHint::Outside));
        }
        let delta = {
            let mut admission = ExactAdmission::new(context, progress);
            exact_rational(
                Some(&mut admission),
                ExactOperation::RationalAdd,
                &[start.longitude(), point.longitude()],
                || {
                    if azimuth == Rat::from_i64(90) {
                        point.longitude().sub(start.longitude())
                    } else {
                        start.longitude().sub(point.longitude())
                    }
                },
            )?
        };
        return Ok(Some(AxisHint::Equator(normalize_admitted(
            &delta, context, progress,
        )?)));
    }
    if !start.is_pole() && azimuth != Rat::zero() && azimuth != Rat::from_i64(180) {
        return Ok(None);
    }
    let north = if start.is_pole() {
        start.latitude().signum() < 0
    } else {
        azimuth.is_zero()
    };
    let primary = if start.is_pole() {
        let raw = if start.latitude().signum() > 0 {
            let mut admission = ExactAdmission::new(context, progress);
            let half = Rat::from_i64(180);
            exact_rational(
                Some(&mut admission),
                ExactOperation::RationalAdd,
                &[&half, &azimuth],
                || half.sub(&azimuth),
            )?
        } else {
            azimuth
        };
        let normalized = normalize_admitted(&raw, context, progress)?;
        if normalized > Rat::from_i64(180) {
            normalized.sub(&Rat::from_i64(360))
        } else {
            normalized
        }
    } else {
        start.longitude().clone()
    };
    let same = point.is_pole() || same_meridian(point.longitude(), &primary);
    let opposite = if same {
        false
    } else {
        let mut admission = ExactAdmission::new(context, progress);
        let delta = exact_rational(
            Some(&mut admission),
            ExactOperation::RationalAdd,
            &[point.longitude(), &primary],
            || point.longitude().sub(&primary),
        )?;
        delta.abs() == Rat::from_i64(180)
    };
    if !same && !opposite {
        return Ok(Some(AxisHint::Outside));
    }
    let (quarter_turns, start_weight, target_weight) = if same {
        let no_wrap = {
            let mut admission = ExactAdmission::new(context, progress);
            admission.rational(
                ExactOperation::RationalCompare,
                &[point.latitude(), start.latitude()],
                1,
                || {
                    Ok(if north {
                        point.latitude() >= start.latitude()
                    } else {
                        point.latitude() <= start.latitude()
                    })
                },
            )?
        };
        (
            if no_wrap { 0 } else { 4 },
            if north { -1 } else { 1 },
            if north { 1 } else { -1 },
        )
    } else {
        (2, if north { -1 } else { 1 }, if north { -1 } else { 1 })
    };
    Ok(Some(AxisHint::Meridian {
        quarter_turns,
        start_weight,
        target_weight,
    }))
}

pub(super) type ParameterIntervals = Vec<(Rat, Rat)>;

/// Complete original-law axial visits, including every full-revolution revisit.
/// Every returned interval encloses one isolated nonconstant parameter event.
pub(super) fn axis_contact_parameters(
    arc: &crate::AzimuthLengthArc,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ParameterIntervals>, GeoError> {
    axis_contact_parameters_with_precision(arc, point, 80, context, progress)
}
fn axis_contact_parameters_with_precision(
    arc: &crate::AzimuthLengthArc,
    point: &LonLat,
    minimum_bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ParameterIntervals>, GeoError> {
    let Some(axis) = axis_hint(arc, point, context, progress)? else {
        return injective_start_parameter(arc, point, context, progress);
    };
    if matches!(axis, AxisHint::Outside) {
        return Ok(Some(Vec::new()));
    }
    let first_is_start = ExactAdmission::new(context, progress).rational(
        ExactOperation::Linear,
        &[
            point.longitude(),
            point.latitude(),
            arc.start().point().longitude(),
            arc.start().point().latitude(),
        ],
        8,
        || Ok(point.same_location(arc.start().point())),
    )?;
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    let mut bits = minimum_bits.max(1).min(policy.limits().max_precision_bits);
    loop {
        // AxisHint contains the exact source phase and is only borrowed across
        // refinement, so an unresolved cutoff cannot select an earlier list.
        let result =
            crate::numerical::with_math(context, progress, bits, 65_536, |math, progress| {
                let ellipsoid = reference.ellipsoid();
                let base = if first_is_start {
                    FixedInterval::from_i64(0, math)
                } else {
                    axis.distance(point, arc.start().point(), ellipsoid, math, progress)
                }
                .map_err(|error| geo_math_error(&error, policy))?;
                let period = if matches!(axis, AxisHint::Equator(_)) {
                    FixedInterval::pi(math)
                        .and_then(|pi| pi.mul(&FixedInterval::from_i64(2, math)?, math))
                        .and_then(|turn| {
                            turn.mul(
                                &crate::numerical::fixed_from_rat(ellipsoid.semimajor(), math)?,
                                math,
                            )
                        })
                } else {
                    crate::geodesic::meridian_arc_observed(
                        &Rat::from_i64(90),
                        ellipsoid,
                        math,
                        progress,
                    )
                    .and_then(|quarter| quarter.mul(&FixedInterval::from_i64(4, math)?, math))
                }
                .map_err(|error| geo_math_error(&error, policy))?;
                let length = crate::numerical::fixed_from_rat(arc.length().exact(), math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                if period.lower().is_negative()
                    || period.lower().is_zero()
                    || length.lower().is_negative()
                    || length.lower().is_zero()
                {
                    return Err(GeoError::PrecisionExhausted { bits });
                }
                let mut output = Vec::new();
                let mut turn = 0i64;
                loop {
                    progress
                        .math_poll(math)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    let distance = if turn == 0 {
                        base.clone()
                    } else {
                        period
                            .mul(
                                &FixedInterval::from_i64(turn, math)
                                    .map_err(|error| geo_math_error(&error, policy))?,
                                math,
                            )
                            .and_then(|travel| base.add(&travel, math))
                            .map_err(|error| geo_math_error(&error, policy))?
                    };
                    // The original phase construction proves nonnegative
                    // first travel; intersect arithmetic width with that fact.
                    let distance = if distance.lower().is_negative() {
                        FixedInterval::from_bounds(BigInt::zero(), distance.upper().clone(), math)
                            .map_err(|error| geo_math_error(&error, policy))?
                    } else {
                        distance
                    };
                    let comparison_bits = [
                        distance.lower(),
                        distance.upper(),
                        length.lower(),
                        length.upper(),
                    ]
                    .into_iter()
                    .map(BigInt::bits_upper_bound)
                    .max()
                    .unwrap_or(0);
                    math.admit_exact(2, comparison_bits)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    if distance.lower() > length.upper() {
                        return Ok(output);
                    }
                    if distance.upper() > length.lower() {
                        return Err(GeoError::PrecisionExhausted { bits });
                    }
                    if output.len() as u64 >= policy.limits().max_output_elements {
                        return Err(GeoError::OutputExhausted {
                            limit: policy.limits().max_output_elements,
                        });
                    }
                    let parameter = distance
                        .div(&length, math)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    let storage = (bits as usize)
                        .div_ceil(8)
                        .saturating_add(64)
                        .saturating_mul(32)
                        .saturating_add(size_of::<(Rat, Rat)>() * 2);
                    math.reserve_workspace(storage)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    output.push(crate::numerical::exact_bounds(&parameter));
                    turn = turn
                        .checked_add(1)
                        .ok_or(GeoError::ArithmeticOverflow("axial revolution count"))?;
                }
            });
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result.map(Some),
        }
    }
}

/// Principal radii M,N are at least m. A repeated-point subarc of length
/// L<pi*m would have every unit tangent within L/(2m)<pi/2 of its midpoint
/// tangent, so its integrated chord would have strictly positive projection.
/// Thus an original start witness is the unique event under this exact bound.
fn injective_start_parameter(
    arc: &crate::AzimuthLengthArc,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ParameterIntervals>, GeoError> {
    let same = ExactAdmission::new(context, progress).rational(
        ExactOperation::Linear,
        &[
            point.longitude(),
            point.latitude(),
            arc.start().point().longitude(),
            arc.start().point().latitude(),
        ],
        8,
        || Ok(point.same_location(arc.start().point())),
    )?;
    if !same {
        return Ok(None);
    }
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    let mut bits = 80.min(policy.limits().max_precision_bits);
    loop {
        let result =
            crate::numerical::with_math(context, progress, bits, 16_384, |math, progress| {
                let minimum = reference.ellipsoid().normal_metric_bounds_ref().0.exact();
                let bound = crate::numerical::fixed_from_rat(minimum, math)
                    .and_then(|m| m.mul(&FixedInterval::pi(math)?, math))
                    .map_err(|error| geo_math_error(&error, policy))?;
                let length = crate::numerical::fixed_from_rat(arc.length().exact(), math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                math.admit_exact(
                    2,
                    [length.lower(), length.upper(), bound.lower(), bound.upper()]
                        .into_iter()
                        .map(BigInt::bits_upper_bound)
                        .max()
                        .unwrap_or(0),
                )
                .map_err(|error| geo_math_error(&error, policy))?;
                progress
                    .math_poll(math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                if length.upper() < bound.lower() {
                    Ok(Some(vec![(Rat::zero(), Rat::zero())]))
                } else if length.lower() >= bound.upper() {
                    Ok(None)
                } else {
                    Err(GeoError::PrecisionExhausted { bits })
                }
            });
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, GeographicReference, PreparedCoordinate, ShortestGeodesicArc};
    fn coordinate(lon: i64, lat: i64) -> PreparedCoordinate {
        PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(lon), Rat::from_i64(lat)),
            &GeographicReference::wgs84(),
        )
        .expect("source")
    }
    #[test]
    fn mapped_midpoint_requires_exact_normal_equality_and_never_accepts_a_neighbor() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference,
        };
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let reference = GeographicReference::wgs84();
        let operation = CoordinateOperation::compile(
            OperationReference {
                realization: Digest32::new([95; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            OperationReference {
                realization: reference.id().digest(),
                unit: CoordinateUnit::Degrees,
                swapped_axes: false,
            },
            OperationModel::MercatorToGeographic {
                radius: Rat::one(),
                eccentricity_squared: Rat::zero(),
                square_domain: false,
            },
        )
        .unwrap();
        let chain = Arc::new(OperationChain::compile(vec![operation]).unwrap());
        let edge = PreparedEdge::Transformed(Box::new(
            OperationImageCurve::new(
                Coord::xy(Rat::parse_decimal("-0.01").unwrap(), Rat::zero()),
                Coord::xy(Rat::parse_decimal("0.01").unwrap(), Rat::zero()),
                chain.clone(),
                reference.clone(),
                None,
                crate::ExecutionPolicy::geometry(),
            )
            .unwrap(),
        ));
        let mut context = MetricContext::wgs84().unwrap();
        assert!(
            contact(
                &edge,
                &LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
                &mut context
            )
            .unwrap()
        );
        // This degree value has no exact dyadic source parameter through the
        // inverse-Mercator mapping. Whole-panel parallel continuity proves
        // contact without accepting a rounded source or normal sample.
        let off_grid = LonLat::new(Rat::parse_decimal("0.2").unwrap(), Rat::zero()).unwrap();
        assert!(contact(&edge, &off_grid, &mut context).unwrap());
        let outside = LonLat::new(Rat::one(), Rat::zero()).unwrap();
        assert!(!contact(&edge, &outside, &mut context).unwrap());
        let neighbor = LonLat::new(Rat::zero(), Rat::parse_decimal("0.000001").unwrap()).unwrap();
        assert!(!contact(&edge, &neighbor, &mut context).unwrap());
        let uncertain = LonLat::new(
            Rat::zero(),
            Rat::new(crate::Int::one(), crate::Int::one().shl(600)).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            contact(&edge, &uncertain, &mut context),
            Err(GeoError::PrecisionExhausted { .. } | GeoError::WorkExhausted { .. })
        ));
        for reverse in [false, true] {
            let endpoints = [
                Coord::xy(Rat::zero(), Rat::parse_decimal("-0.01").unwrap()),
                Coord::xy(Rat::zero(), Rat::parse_decimal("0.01").unwrap()),
            ];
            let [start, end] = if reverse {
                let [start, end] = endpoints;
                [end, start]
            } else {
                endpoints
            };
            let meridian = PreparedEdge::Transformed(Box::new(
                OperationImageCurve::new(
                    start,
                    end,
                    chain.clone(),
                    reference.clone(),
                    None,
                    crate::ExecutionPolicy::geometry(),
                )
                .unwrap(),
            ));
            let off_grid = LonLat::new(Rat::zero(), Rat::parse_decimal("0.2").unwrap()).unwrap();
            assert!(contact(&meridian, &off_grid, &mut context).unwrap());
            assert!(
                !contact(
                    &meridian,
                    &LonLat::new(Rat::zero(), Rat::one()).unwrap(),
                    &mut context,
                )
                .unwrap()
            );
            assert!(
                !contact(
                    &meridian,
                    &LonLat::new(Rat::parse_decimal("0.000001").unwrap(), Rat::zero()).unwrap(),
                    &mut context,
                )
                .unwrap()
            );
        }
    }
    #[test]
    fn unique_shortest_symmetries_preserve_seams_meridians_and_poles() {
        let mut context = MetricContext::wgs84().expect("context");
        for (a, b, p, expected) in [
            ((170, 0), (-170, 0), (180, 0), true),
            ((170, 0), (-170, 0), (0, 0), false),
            ((25, -30), (25, 75), (25, 0), true),
            ((25, -30), (25, 75), (26, 0), false),
            ((0, 70), (180, 80), (-75, 90), true),
            ((0, 70), (180, 80), (0, 65), false),
            ((-40, -25), (40, 25), (0, 0), true),
            ((-40, -25), (40, 25), (180, 0), false),
            ((150, -25), (-130, 25), (-170, 0), true),
        ] {
            let edge = PreparedEdge::ShortestGeodesic(Box::new(
                ShortestGeodesicArc::new(coordinate(a.0, a.1), coordinate(b.0, b.1), &mut context)
                    .expect("unique branch"),
            ));
            let p = LonLat::new(Rat::from_i64(p.0), Rat::from_i64(p.1)).expect("point");
            assert_eq!(
                contact(&edge, &p, &mut context).unwrap_or_else(|error| {
                    panic!("complete original-law contact {a:?} -> {b:?}, {p:?}: {error:?}")
                }),
                expected
            );
        }
        let edge = PreparedEdge::ShortestGeodesic(Box::new(
            ShortestGeodesicArc::new(coordinate(-30, -20), coordinate(50, 35), &mut context)
                .expect("unique branch"),
        ));
        assert!(
            !contact(
                &edge,
                &LonLat::new(Rat::from_i64(120), Rat::from_i64(-60)).expect("point"),
                &mut context
            )
            .expect("whole normal-image exclusion")
        );
    }
    #[test]
    fn selected_axis_arcs_use_original_length_and_pole_frame() {
        let mut limits = *crate::ExecutionPolicy::geometry().limits();
        limits.max_work_items = 2_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).expect("work admission"),
        )
        .expect("context");
        for (start, azimuth, length, point, expected) in [
            ((170, 0), 90, 2_000_000, (-175, 0), true),
            ((170, 0), 90, 2_000_000, (0, 0), false),
            ((170, 0), 90, 45_000_000, (0, 0), true),
            ((25, -30), 0, 10_000_000, (25, 0), true),
            ((25, -30), 0, 10_000_000, (25, -40), false),
            ((0, 90), 37, 1_000_000, (143, 85), true),
            ((0, 90), 37, 1_000_000, (37, 85), false),
            ((0, -90), 37, 1_000_000, (37, -85), true),
        ] {
            let edge = PreparedEdge::AzimuthLength(Box::new(
                crate::AzimuthLengthArc::new(
                    coordinate(start.0, start.1),
                    Rat::from_i64(azimuth),
                    crate::Metres::new(Rat::from_i64(length)),
                    &mut context,
                )
                .expect("selected original line"),
            ));
            let point = LonLat::new(Rat::from_i64(point.0), Rat::from_i64(point.1)).expect("query");
            assert_eq!(
                contact(&edge, &point, &mut context).expect("certified axis extent"),
                expected
            );
        }
    }
}
