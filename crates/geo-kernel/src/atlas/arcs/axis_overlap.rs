// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete common-axis overlaps of certified unique shortest geodesics.

use super::{CurveIntersection, admit_contact_count};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, exact_rational, geo_math_error};
use crate::{GeoError, LonLat, MetricContext, PreparedEdge, Rat, ShortestGeodesicArc};
use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalCompare};

enum Axis {
    Equator,
    Meridian(Rat),
}
struct Span<'a> {
    lower: Rat,
    upper: Rat,
    lower_point: &'a LonLat,
    upper_point: &'a LonLat,
    forward: bool,
}

pub(super) fn intersections(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let ((PreparedEdge::ShortestGeodesic(selected), _)
    | (_, PreparedEdge::ShortestGeodesic(selected))) = (left, right)
    else {
        return Ok(None);
    };
    if let Some(crossing) = equator_source_contacts(left, right, context, progress)? {
        return Ok(Some(crossing));
    }
    let (axis, overlaps) = {
        let mut admission = ExactAdmission::new(context, progress);
        let Some(axis) = axis(selected, &mut admission)? else {
            return Ok(None);
        };
        let Some(a_span) = span_edge(left, &axis, &mut admission)? else {
            return Ok(None);
        };
        let Some(b_span) = span_edge(right, &axis, &mut admission)? else {
            return Ok(None);
        };
        let mut overlaps = Vec::new();
        for turn in [-360, 0, 360] {
            let offset = Rat::from_i64(turn);
            let low = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[&b_span.lower, &offset],
                || b_span.lower.add(&offset),
            )?;
            let high = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[&b_span.upper, &offset],
                || b_span.upper.add(&offset),
            )?;
            let lower_is_a =
                admission.rational(RationalCompare, &[&a_span.lower, &low], 1, || {
                    Ok(a_span.lower >= low)
                })?;
            let upper_is_a =
                admission.rational(RationalCompare, &[&a_span.upper, &high], 1, || {
                    Ok(a_span.upper <= high)
                })?;
            let lower = if lower_is_a { &a_span.lower } else { &low };
            let upper = if upper_is_a { &a_span.upper } else { &high };
            if !admission.rational(RationalCompare, &[lower, upper], 1, || Ok(lower < upper))? {
                continue;
            }
            let start = if lower_is_a {
                a_span.lower_point
            } else {
                b_span.lower_point
            };
            let end = if upper_is_a {
                a_span.upper_point
            } else {
                b_span.upper_point
            };
            let (start, end, first, last) = if a_span.forward {
                (start, end, lower, upper)
            } else {
                (end, start, upper, lower)
            };
            let copied = admission.rational(
                Linear,
                &[
                    start.longitude(),
                    start.latitude(),
                    end.longitude(),
                    end.latitude(),
                    first,
                    last,
                    &offset,
                ],
                7,
                || {
                    Ok((
                        [start.clone(), end.clone()],
                        [first.clone(), last.clone()],
                        offset.clone(),
                    ))
                },
            )?;
            overlaps.push(copied);
        }
        (axis, overlaps)
    };
    if overlaps.is_empty() {
        return super::endpoint_contacts(left, right, context, progress).map(Some);
    }
    admit_contact_count(overlaps.len() as u64, context)?;
    let mut output = Vec::with_capacity(overlaps.len());
    for ([start, end], phases, offset) in overlaps {
        let first = [
            edge_parameter(left, &start, &phases[0], &axis, context, progress)?,
            edge_parameter(left, &end, &phases[1], &axis, context, progress)?,
        ];
        let second_phases = {
            let mut admission = ExactAdmission::new(context, progress);
            [0, 1].map(|index| {
                exact_rational(
                    Some(&mut admission),
                    RationalAdd,
                    &[&phases[index], &offset],
                    || phases[index].sub(&offset),
                )
            })
        };
        let [first_phase, last_phase] = second_phases;
        let second = [
            edge_parameter(right, &start, &first_phase?, &axis, context, progress)?,
            edge_parameter(right, &end, &last_phase?, &axis, context, progress)?,
        ];
        output.push(CurveIntersection::GeodesicOverlap {
            left: first,
            right: Box::new(second),
            start,
            end,
        });
    }
    Ok(Some(output))
}

fn span_edge<'a>(
    edge: &'a PreparedEdge,
    axis: &Axis,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<Span<'a>>, GeoError> {
    if let PreparedEdge::ShortestGeodesic(arc) = edge {
        return span(arc, axis, admission);
    }
    let PreparedEdge::SourceLinear(line) = edge else {
        return Ok(None);
    };
    let Some([start, end]) = source_phases(line, axis, admission)? else {
        return Ok(None);
    };
    let order = admission.rational(RationalCompare, &[&start, &end], 1, || Ok(start.cmp(&end)))?;
    if order.is_eq() {
        return Ok(None);
    }
    let (lower, upper, lower_point, upper_point) = if order.is_lt() {
        (start, end, line.start().point(), line.end().point())
    } else {
        (end, start, line.end().point(), line.start().point())
    };
    Ok(Some(Span {
        lower,
        upper,
        lower_point,
        upper_point,
        forward: order.is_lt(),
    }))
}

fn source_phases(
    line: &crate::SourceLinearEdge,
    axis: &Axis,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<[Rat; 2]>, GeoError> {
    let a = line.start().point();
    let b = line.end().point();
    match axis {
        Axis::Equator => {
            if !admission.rational(Linear, &[a.latitude(), b.latitude()], 2, || {
                Ok(a.latitude().is_zero() && b.latitude().is_zero())
            })? {
                return Ok(None);
            }
            admission.rational(Linear, &[a.longitude(), b.longitude()], 2, || {
                Ok(Some([a.longitude().clone(), b.longitude().clone()]))
            })
        }
        Axis::Meridian(frame) => {
            if !admission.rational(Linear, &[a.longitude(), b.longitude()], 1, || {
                Ok(a.longitude() == b.longitude())
            })? {
                return Ok(None);
            }
            if !same_plane(a.longitude(), frame, admission)? {
                return Ok(None);
            }
            let delta = exact_rational(
                Some(admission),
                RationalAdd,
                &[a.longitude(), frame],
                || a.longitude().sub(frame),
            )?;
            let opposite = admission.rational(RationalCompare, &[&delta], 2, || {
                Ok(delta == Rat::from_i64(180) || delta == Rat::from_i64(-180))
            })?;
            if opposite {
                let half = Rat::from_i64(180);
                let values = [a, b].map(|point| {
                    exact_rational(
                        Some(admission),
                        RationalAdd,
                        &[&half, point.latitude()],
                        || half.sub(point.latitude()),
                    )
                });
                let [first, last] = values;
                Ok(Some([first?, last?]))
            } else {
                admission.rational(Linear, &[a.latitude(), b.latitude()], 2, || {
                    Ok(Some([a.latitude().clone(), b.latitude().clone()]))
                })
            }
        }
    }
}

fn edge_parameter(
    edge: &PreparedEdge,
    point: &LonLat,
    phase: &Rat,
    axis: &Axis,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Rat, Rat), GeoError> {
    if let PreparedEdge::ShortestGeodesic(arc) = edge {
        return parameter(arc, point, context, progress);
    }
    let PreparedEdge::SourceLinear(line) = edge else {
        unreachable!("proved source-axis span");
    };
    let mut admission = ExactAdmission::new(context, progress);
    let [start, end] =
        source_phases(line, axis, &mut admission)?.expect("proved source-axis phases");
    let delta = exact_rational(Some(&mut admission), RationalAdd, &[&end, &start], || {
        end.sub(&start)
    })?;
    let offset = exact_rational(Some(&mut admission), RationalAdd, &[phase, &start], || {
        phase.sub(&start)
    })?;
    let parameter = exact_rational(
        Some(&mut admission),
        purrdf_xsd::integer::ExactOperation::RationalDivide,
        &[&offset, &delta],
        || offset.div(&delta).expect("nonconstant axis span"),
    )?;
    admission.rational(Linear, &[&parameter], 2, || {
        Ok((parameter.clone(), parameter.clone()))
    })
}

fn axis(
    arc: &ShortestGeodesicArc,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<Axis>, GeoError> {
    let a = arc.start().point();
    let b = arc.end().point();
    // Reflection in the equator fixes both endpoints. The private prepared
    // arc contract certifies a unique shortest branch, so the reflected path
    // is that same path; a positive path between distinct nonpolar equatorial
    // endpoints therefore lies on the equator. Near-antipodal competing paths
    // are excluded by preparation's certified multiplicity, never by a rounded
    // endpoint azimuth. The same reflection argument fixes meridian planes.
    if admission.rational(Linear, &[a.latitude(), b.latitude()], 2, || {
        Ok(a.latitude().is_zero() && b.latitude().is_zero())
    })? {
        return Ok(Some(Axis::Equator));
    }
    let polar = admission.rational(Linear, &[a.latitude(), b.latitude()], 2, || {
        Ok([a.is_pole(), b.is_pole()])
    })?;
    let frame = if polar[0] {
        b.longitude()
    } else {
        a.longitude()
    };
    if !polar[0] && !polar[1] && !same_plane(a.longitude(), b.longitude(), admission)? {
        return Ok(None);
    }
    admission.rational(Linear, &[frame], 1, || {
        Ok(Some(Axis::Meridian(frame.clone())))
    })
}

fn same_plane(a: &Rat, b: &Rat, admission: &mut ExactAdmission<'_, '_>) -> Result<bool, GeoError> {
    let delta = exact_rational(Some(admission), RationalAdd, &[a, b], || a.sub(b))?;
    admission.rational(RationalCompare, &[&delta], 4, || {
        Ok(delta.is_zero()
            || [-360, -180, 180, 360]
                .into_iter()
                .any(|v| delta == Rat::from_i64(v)))
    })
}

fn phase(
    point: &LonLat,
    axis: &Axis,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<Rat>, GeoError> {
    match axis {
        Axis::Equator => {
            if !admission.rational(Linear, &[point.latitude()], 1, || {
                Ok(point.latitude().is_zero())
            })? {
                return Ok(None);
            }
            admission.rational(Linear, &[point.longitude()], 1, || {
                Ok(Some(point.longitude().clone()))
            })
        }
        Axis::Meridian(frame) => {
            if admission.rational(Linear, &[point.latitude()], 1, || Ok(point.is_pole()))? {
                return Ok(Some(Rat::from_i64(if point.latitude().signum() > 0 {
                    90
                } else {
                    -90
                })));
            }
            let delta = exact_rational(
                Some(admission),
                RationalAdd,
                &[point.longitude(), frame],
                || point.longitude().sub(frame),
            )?;
            let opposite = admission.rational(RationalCompare, &[&delta], 2, || {
                Ok(delta == Rat::from_i64(180) || delta == Rat::from_i64(-180))
            })?;
            if opposite {
                let half = Rat::from_i64(180);
                return exact_rational(
                    Some(admission),
                    RationalAdd,
                    &[&half, point.latitude()],
                    || half.sub(point.latitude()),
                )
                .map(Some);
            }
            if !admission.rational(RationalCompare, &[&delta], 2, || {
                Ok(delta.is_zero() || delta == Rat::from_i64(360) || delta == Rat::from_i64(-360))
            })? {
                return Ok(None);
            }
            admission.rational(Linear, &[point.latitude()], 1, || {
                Ok(Some(point.latitude().clone()))
            })
        }
    }
}

fn span<'a>(
    arc: &'a ShortestGeodesicArc,
    axis: &Axis,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Option<Span<'a>>, GeoError> {
    let a = arc.start().point();
    let b = arc.end().point();
    let (Some(start), Some(end)) = (phase(a, axis, admission)?, phase(b, axis, admission)?) else {
        return Ok(None);
    };
    let mut delta = exact_rational(Some(admission), RationalAdd, &[&end, &start], || {
        end.sub(&start)
    })?;
    let direction = admission.rational(RationalCompare, &[&delta], 2, || {
        Ok(if delta > Rat::from_i64(180) {
            -1
        } else {
            i64::from(delta < Rat::from_i64(-180))
        })
    })?;
    if direction != 0 {
        let shift = Rat::from_i64(direction * 360);
        delta = exact_rational(Some(admission), RationalAdd, &[&delta, &shift], || {
            delta.add(&shift)
        })?;
    }
    if delta.is_zero() {
        // A constant curve has no positive-length overlap; its point-contact
        // law remains in the shared point/curve predicate.
        return Ok(None);
    }
    let end = exact_rational(Some(admission), RationalAdd, &[&start, &delta], || {
        start.add(&delta)
    })?;
    Ok(Some(if delta.signum() > 0 {
        Span {
            lower: start,
            upper: end,
            lower_point: a,
            upper_point: b,
            forward: true,
        }
    } else {
        Span {
            lower: end,
            upper: start,
            lower_point: b,
            upper_point: a,
            forward: false,
        }
    }))
}

pub(super) fn parameter(
    arc: &ShortestGeodesicArc,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Rat, Rat), GeoError> {
    parameter_with_precision(arc, point, None, context, progress)
}

pub(super) fn parameter_with_precision(
    arc: &ShortestGeodesicArc,
    point: &LonLat,
    refinement: Option<u32>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Rat, Rat), GeoError> {
    let known = ExactAdmission::new(context, progress).rational(
        Linear,
        &[
            point.longitude(),
            point.latitude(),
            arc.start().point().longitude(),
            arc.start().point().latitude(),
            arc.end().point().longitude(),
            arc.end().point().latitude(),
        ],
        8,
        || {
            Ok(if point.same_location(arc.start().point()) {
                Some(0)
            } else if point.same_location(arc.end().point()) {
                Some(1)
            } else {
                None
            })
        },
    )?;
    if let Some(parameter) = known {
        return Ok((Rat::from_i64(parameter), Rat::from_i64(parameter)));
    }
    if let Some(parameter) = equator_parameter(arc, point, context, progress)? {
        return Ok(parameter);
    }
    let reference = crate::numerical::reference_clone(context, progress)?;
    let geodesic = crate::geodesic::PreparedGeodesic::new(reference);
    if let Some(bits) = refinement {
        let whole = refined_view(&geodesic, arc, None, bits, context, progress)?;
        let whole_bytes = whole.retained_workspace_bytes();
        context.admit_workspace(whole_bytes)?;
        let result = (|| {
            let partial = refined_view(&geodesic, arc, Some(point), bits, context, progress)?;
            let partial_bytes = partial.retained_workspace_bytes();
            context.admit_workspace(partial_bytes)?;
            let result = ratio(
                partial.length_bounds(),
                whole.length_bounds(),
                bits,
                context,
                progress,
            );
            context.release_workspace(partial_bytes)?;
            result
        })();
        context.release_workspace(whole_bytes)?;
        return result;
    }
    let (_, distance) = in_child(context, progress, |child, observer| {
        geodesic.distance_with_proof_metered(arc.start().point(), point, child, observer)
    })?;
    let bits = 128
        .max(arc.proof().distance.precision_bits)
        .max(distance.precision_bits)
        .min(context.policy().limits().max_precision_bits);
    ratio(
        (&distance.lower, &distance.upper),
        (&arc.proof().distance.lower, &arc.proof().distance.upper),
        bits,
        context,
        progress,
    )
}

/// On the independently certified unique equatorial branch, ds=a*dλ.
/// Its arclength fraction is the exact lifted longitude fraction; the
/// ellipsoid scale cancels before arithmetic. No numerical distance output or
/// approximate endpoint is an input to this ratio.
fn equator_parameter(
    arc: &ShortestGeodesicArc,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<(Rat, Rat)>, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let Some(Axis::Equator) = axis(arc, &mut admission)? else {
        return Ok(None);
    };
    let Some(span) = span(arc, &Axis::Equator, &mut admission)? else {
        return Ok(None);
    };
    let Some(phase) = phase(point, &Axis::Equator, &mut admission)? else {
        return Ok(None);
    };
    for turn in [-360, 0, 360] {
        let offset = Rat::from_i64(turn);
        let value = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[&phase, &offset],
            || phase.add(&offset),
        )?;
        if !admission.rational(
            RationalCompare,
            &[&value, &span.lower, &span.upper],
            2,
            || Ok(value >= span.lower && value <= span.upper),
        )? {
            continue;
        }
        let (start, end) = if span.forward {
            (&span.lower, &span.upper)
        } else {
            (&span.upper, &span.lower)
        };
        let numerator =
            exact_rational(Some(&mut admission), RationalAdd, &[&value, start], || {
                value.sub(start)
            })?;
        let denominator = exact_rational(Some(&mut admission), RationalAdd, &[end, start], || {
            end.sub(start)
        })?;
        let parameter = exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&numerator, &denominator],
            || numerator.div(&denominator).expect("positive branch span"),
        )?;
        return admission.rational(Linear, &[&parameter], 2, || {
            Ok(Some((parameter.clone(), parameter.clone())))
        });
    }
    Err(GeoError::domain("point outside proved equatorial branch"))
}

/// Exact image of an original arclength fraction on a privately certified
/// unique equatorial branch. This is the inverse of equator_parameter's same
/// lifted span equation; no quantized direct endpoint enters the atlas.
pub(super) fn equator_point(
    arc: &ShortestGeodesicArc,
    parameter: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<LonLat>, GeoError> {
    let longitude = {
        let mut admission = ExactAdmission::new(context, progress);
        let Some(Axis::Equator) = axis(arc, &mut admission)? else {
            return Ok(None);
        };
        let Some(span) = span(arc, &Axis::Equator, &mut admission)? else {
            return Ok(None);
        };
        let (start, end) = if span.forward {
            (&span.lower, &span.upper)
        } else {
            (&span.upper, &span.lower)
        };
        crate::SourceLinearEdge::interpolate_ordinate_admitted(
            start,
            end,
            parameter,
            &mut admission,
        )?
    };
    let longitude = super::super::point::longitude_admitted(&longitude, context, progress)?;
    let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude], 4)
        .ok_or(GeoError::ArithmeticOverflow("equatorial image witness"))?;
    progress
        .exact(context, cost, || LonLat::new(longitude, Rat::zero()))
        .map(Some)
}

/// Complete equator/written-source crossings. Latitude is affine in the
/// original source parameter, so a non-equatorial edge has at most one zero.
/// The unique equatorial arc's exact lifted span then admits or excludes that
/// physical point, including long written paths and identified seam aliases.
fn equator_source_contacts(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let (arc, line, reversed) = match (left, right) {
        (PreparedEdge::ShortestGeodesic(arc), PreparedEdge::SourceLinear(line)) => {
            (arc, line, false)
        }
        (PreparedEdge::SourceLinear(line), PreparedEdge::ShortestGeodesic(arc)) => {
            (arc, line, true)
        }
        _ => return Ok(None),
    };
    let candidate = {
        let mut admission = ExactAdmission::new(context, progress);
        if !matches!(axis(arc, &mut admission)?, Some(Axis::Equator)) {
            return Ok(None);
        }
        let start = line.start().point();
        let end = line.end().point();
        let signs = admission.rational(Linear, &[start.latitude(), end.latitude()], 2, || {
            Ok([start.latitude().signum(), end.latitude().signum()])
        })?;
        if signs == [0, 0] {
            return Ok(None);
        }
        if signs[0] == signs[1] {
            return Ok(Some(Vec::new()));
        }
        let delta = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[end.latitude(), start.latitude()],
            || end.latitude().sub(start.latitude()),
        )?;
        let negative = admission.rational(Linear, &[start.latitude()], 1, || {
            Ok(start.latitude().neg())
        })?;
        let parameter = exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&negative, &delta],
            || negative.div(&delta).expect("nonzero latitude span"),
        )?;
        let longitude = crate::SourceLinearEdge::interpolate_ordinate_admitted(
            start.longitude(),
            end.longitude(),
            &parameter,
            &mut admission,
        )?;
        (longitude, parameter)
    };
    let (longitude, source_parameter) = candidate;
    let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude], 4).ok_or(
        GeoError::ArithmeticOverflow("equatorial coordinate witness"),
    )?;
    let point = progress.exact(context, cost, || LonLat::new(longitude, Rat::zero()))?;
    let selected_edge = if reversed { right } else { left };
    if super::super::point::exact_source(selected_edge, &point, context, progress)? != Some(true) {
        return Ok(Some(Vec::new()));
    }
    let (parameter, _) =
        equator_parameter(arc, &point, context, progress)?.expect("equatorial branch");
    let (left, right) = if reversed {
        (source_parameter, parameter)
    } else {
        (parameter, source_parameter)
    };
    admit_contact_count(1, context)?;
    Ok(Some(vec![CurveIntersection::Exact { left, right, point }]))
}

fn refined_view(
    geodesic: &crate::geodesic::PreparedGeodesic,
    arc: &ShortestGeodesicArc,
    point: Option<&LonLat>,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<crate::geodesic::ArcIntervalView, GeoError> {
    let view = in_child(context, progress, |child, observer| {
        if let Some(point) = point {
            geodesic.prepare_shortest_arc_view(
                arc.start().point(),
                point,
                bits,
                child,
                Some(observer),
            )
        } else {
            geodesic.prepare_shortest_arc_view_with_proof(arc, bits, child, Some(observer))
        }
    })?;
    let retained = view.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    let result = in_child(context, progress, |child, observer| {
        view.refined(bits, child, Some(observer))
    });
    context.release_workspace(retained)?;
    result
}

fn in_child<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(&mut MetricContext, &mut dyn crate::MetricWorkObserver) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let mut child = context.remaining_child()?;
    let retained = context
        .policy()
        .limits()
        .max_workspace_bytes
        .saturating_sub(context.remaining_workspace());
    let result = {
        let mut observer =
            progress.nested(context.work_items(), retained, context.workspace_peak());
        evaluate(&mut child, &mut observer)
    };
    progress.absorb_child_result(context, &child, result)
}

fn ratio(
    distance: (&crate::Metres, &crate::Metres),
    length: (&crate::Metres, &crate::Metres),
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Rat, Rat), GeoError> {
    let policy = context.policy();
    crate::numerical::with_math(context, progress, bits, 16_384, |math, progress| {
        let length = crate::numerical::fixed_from_bounds(length.0.exact(), length.1.exact(), math)
            .map_err(|error| geo_math_error(&error, policy))?;
        if length.lower().is_negative() || length.lower().is_zero() {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        let distance =
            crate::numerical::fixed_from_bounds(distance.0.exact(), distance.1.exact(), math)
                .map_err(|error| geo_math_error(&error, policy))?;
        let parameter = distance
            .div(&length, math)
            .map_err(|error| geo_math_error(&error, policy))?;
        progress
            .math_poll(math)
            .map_err(|error| geo_math_error(&error, policy))?;
        let (lower, upper) = crate::numerical::exact_bounds(&parameter);
        let (negative, above_one) = crate::numerical::math_exact_rationals(
            RationalCompare,
            &[&lower, &upper],
            2,
            math,
            progress,
            || (lower < Rat::zero(), upper > Rat::one()),
        )
        .map_err(|error| geo_math_error(&error, policy))?;
        Ok((
            if negative { Rat::zero() } else { lower },
            if above_one { Rat::one() } else { upper },
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Int;

    fn contains(bounds: &(Rat, Rat), numerator: i64, denominator: i64) {
        let exact = Rat::new(Int::from_i64(numerator), Int::from_i64(denominator)).unwrap();
        assert!(bounds.0 <= exact && exact <= bounds.1);
    }

    #[test]
    fn source_parameter_refinement_is_independent_of_completed_distance_grids() {
        let mut context = MetricContext::wgs84().unwrap();
        let edge = super::super::tests::shortest((-10, -10), (10, 10), &mut context);
        let PreparedEdge::ShortestGeodesic(arc) = edge else {
            panic!("endpoint-defined source")
        };
        let point = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        context.begin(1).unwrap();
        let bounds = parameter_with_precision(
            &arc,
            &point,
            Some(128),
            &mut context,
            &mut WorkProgress::new(None),
        )
        .unwrap();
        contains(&bounds, 1, 2);
        assert!(bounds.1.sub(&bounds.0) < Rat::parse_decimal("0.00000000000000000001").unwrap());
    }

    #[test]
    fn common_equatorial_and_meridian_paths_keep_original_overlap_endpoints() {
        let mut context = MetricContext::wgs84().unwrap();
        for (a, b, c, d) in [
            ((-40, 0), (40, 0), (0, 0), (80, 0)),
            ((10, -40), (10, 40), (10, 0), (10, 80)),
        ] {
            let left = super::super::tests::shortest(a, b, &mut context);
            let right = super::super::tests::shortest(c, d, &mut context);
            let contacts = super::super::intersections(&left, &right, &mut context).unwrap();
            let [
                CurveIntersection::GeodesicOverlap {
                    left: lp,
                    right: rp,
                    start,
                    end,
                },
            ] = contacts.as_slice()
            else {
                panic!("one complete original-law overlap")
            };
            assert_eq!(
                start,
                &LonLat::new(Rat::from_i64(c.0), Rat::from_i64(c.1)).unwrap()
            );
            assert_eq!(
                end,
                &LonLat::new(Rat::from_i64(b.0), Rat::from_i64(b.1)).unwrap()
            );
            contains(&lp[0], 1, 2);
            assert_eq!(lp[1], (Rat::one(), Rat::one()));
            assert_eq!(rp[0], (Rat::zero(), Rat::zero()));
            if a.1 == 0 && b.1 == 0 {
                contains(&rp[1], 1, 2);
            } else {
                // M(latitude) strictly increases northward on an oblate
                // ellipsoid: the first40° is shorter than the second40°.
                assert!(rp[1].1 < Rat::one().div(&Rat::from_i64(2)).unwrap());
            }
        }
    }

    #[test]
    fn seam_pole_and_reversed_overlaps_keep_the_selected_shortest_path() {
        let mut context = MetricContext::wgs84().unwrap();
        for (a, b, c, d) in [
            ((170, 0), (-170, 0), (175, 0), (-175, 0)),
            ((0, 60), (180, 60), (0, 80), (180, 80)),
            ((10, -40), (10, 40), (10, 80), (10, 0)),
        ] {
            let left = super::super::tests::shortest(a, b, &mut context);
            let right = super::super::tests::shortest(c, d, &mut context);
            let contacts = super::super::intersections(&left, &right, &mut context).unwrap();
            let [
                CurveIntersection::GeodesicOverlap {
                    left: lp,
                    right: rp,
                    start,
                    end,
                },
            ] = contacts.as_slice()
            else {
                panic!("one complete selected overlap")
            };
            assert!(lp[0].1 < lp[1].0);
            if c.1 == 80 && d.1 == 0 {
                assert_eq!(start.latitude(), &Rat::zero());
                assert_eq!(end.latitude(), &Rat::from_i64(40));
                assert_eq!(rp[0], (Rat::one(), Rat::one()));
                assert!(rp[1].0 > Rat::one().div(&Rat::from_i64(2)).unwrap());
            } else {
                assert_eq!(
                    start,
                    &LonLat::new(Rat::from_i64(c.0), Rat::from_i64(c.1)).unwrap()
                );
                assert_eq!(
                    end,
                    &LonLat::new(Rat::from_i64(d.0), Rat::from_i64(d.1)).unwrap()
                );
                assert_eq!(
                    rp.as_ref(),
                    &[(Rat::zero(), Rat::zero()), (Rat::one(), Rat::one())]
                );
            }
        }
        let left = super::super::tests::shortest((0, 0), (10, 0), &mut context);
        let separate = super::super::tests::shortest((20, 0), (30, 0), &mut context);
        assert_eq!(
            super::super::intersections(&left, &separate, &mut context).unwrap(),
            []
        );
        let touching = super::super::tests::shortest((10, 0), (20, 0), &mut context);
        assert!(matches!(
            super::super::intersections(&left, &touching, &mut context)
                .unwrap()
                .as_slice(),
            [CurveIntersection::Endpoint { .. }]
        ));
    }

    #[test]
    fn zero_shortest_curves_keep_complete_constant_parameter_domains() {
        let mut context = MetricContext::wgs84().unwrap();
        let constant = super::super::tests::shortest((0, 0), (0, 0), &mut context);
        for (start, end) in [((0, -10), (0, 10)), ((-10, -10), (10, 10))] {
            let curve = super::super::tests::shortest(start, end, &mut context);
            for reversed in [false, true] {
                let (a, b) = if reversed {
                    (&curve, &constant)
                } else {
                    (&constant, &curve)
                };
                let contacts = super::super::intersections(a, b, &mut context).unwrap();
                let [CurveIntersection::ConstantContact { left, right, point }] =
                    contacts.as_slice()
                else {
                    panic!("complete constant contact")
                };
                assert_eq!(point, &LonLat::new(Rat::zero(), Rat::zero()).unwrap());
                let (whole, other) = if reversed {
                    (right, left)
                } else {
                    (left, right)
                };
                assert_eq!(whole, &(Rat::zero(), Rat::one()));
                contains(other, 1, 2);
            }
        }
        let away = super::super::tests::shortest((10, 20), (10, 30), &mut context);
        assert_eq!(
            super::super::intersections(&constant, &away, &mut context).unwrap(),
            []
        );
    }
}
