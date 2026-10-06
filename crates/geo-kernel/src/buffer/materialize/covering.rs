// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A proved cover of the complete original source, followed by the shared
//! ellipsoidal disk materializer. This is a set inclusion proof: every source
//! panel is within .014m of its actual midpoint. Its quantized midpoint is
//! within 1um. Disks inflated by .014002m therefore contain every B_r panel.
//! Conversely their points lie within r+.014002+.069002+.000001m of the source,
//! below the frozen .1m band. Areal interiors come from the labelled source
//! atlas; all their boundary panels remain source points, including hole walls.
//!
//! A geodesic panel instead retains its endpoints. On a complete surface with
//! nonnegative curvature, t²-d(x,gamma(t))² is convex (Greene, Comparison
//! Geometry, Lemma 2.1). For length h, some endpoint is therefore within
//! sqrt(r²+h²/4) of every point within r of the panel. The outward inequality
//! h² <= 4((r+.014)²-r²) proves the same inflated-disk inclusion globally,
//! including cut loci, without asserting that the panel is a shortest branch.
//! <https://library2.msri.org/books/Book30/files/greene.pdf>

use super::{
    DiskRequest, OffsetRegion, admit_vertex_count, admit_vertices, carrier_admitted,
    certify_center_grid, decimal, point_centers, point_polygons, prepare_disks, whole_disk,
    whole_surface_ring,
};
use crate::carrier::MaterializationStorage;
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, fixed_from_rat, geo_math_error, math_quantized_decimal};
use crate::{
    CoordDim, GeoError, Geometry, GeometryBody, MetricContext, PreparedCurve, PreparedPolygon,
    PreparedRegion, Rat,
};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};
use std::borrow::Cow;

pub(super) fn complete_body(
    offset: &OffsetRegion,
    (profile, target): (&crate::GeoProfile, &crate::Crs),
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
    global_output: &mut bool,
) -> Result<GeometryBody, GeoError> {
    if offset.radius.exact().numerator().is_negative() {
        return Ok(GeometryBody::GeometryCollection(Vec::new()));
    }
    let boundary =
        crate::atlas::boundary::region_boundary(offset.source.region(), context, progress)?;
    let mut source_storage = boundary.workspace_bytes;
    let result = complete_selected_body(
        offset,
        (profile, target),
        &boundary,
        &mut source_storage,
        context,
        progress,
        total,
        retained,
        global_output,
    );
    drop(boundary);
    context.release_workspace(source_storage)?;
    result
}

#[expect(
    clippy::too_many_arguments,
    reason = "the selected support owner and its original retained allowance span the complete materialization phase"
)]
fn complete_selected_body(
    offset: &OffsetRegion,
    (profile, target): (&crate::GeoProfile, &crate::Crs),
    boundary: &crate::atlas::boundary::RegionBoundary,
    source_storage: &mut u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
    global_output: &mut bool,
) -> Result<GeometryBody, GeoError> {
    if offset.radius.exact().is_zero() {
        return exact_closure(
            offset,
            boundary,
            source_storage,
            context,
            progress,
            total,
            retained,
        );
    }
    let mut centers = point_centers(&offset.source, context, progress, retained)?;
    let native_areal = if let Some(native) = &boundary.native {
        let mut implicit = false;
        for fragment in native.fragments() {
            implicit |= !exact_linear_domain(
                fragment.original_edge(),
                fragment.parameters().each_ref(),
                context,
                progress,
            )?;
        }
        for point in native.isolated_points() {
            implicit |= !exact_linear_domain(
                point.original_edge(),
                [point.parameter(); 2],
                context,
                progress,
            )?;
        }
        implicit
    } else {
        match offset.source.region() {
            PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
                polygons
                    .iter()
                    .flat_map(PreparedPolygon::rings)
                    .flat_map(PreparedCurve::edges)
                    .any(|edge| !matches!(edge, crate::PreparedEdge::SourceLinear(_)))
            }
            PreparedRegion::Empty | PreparedRegion::Whole => false,
        }
    };
    if native_areal {
        *global_output = true;
        return super::global::materialize(
            &super::global::Request {
                source: super::global::Source::Complete(offset),
                radius: offset.radius.exact(),
                profile,
                target,
                band: &decimal("0.099998"),
            },
            context,
            progress,
            total,
            retained,
        );
    }
    let polygons = match offset.source.region() {
        PreparedRegion::Empty | PreparedRegion::Whole => &[][..],
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            &polygons[..]
        }
    };
    let ring_polygons = if boundary.native.is_some() {
        &[][..]
    } else {
        polygons
    };
    if let Some(native) = &boundary.native {
        for point in native.isolated_points() {
            retained.admit_transient(2048, context)?;
            progress.context_poll(context)?;
            centers
                .try_reserve(1)
                .map_err(|_| GeoError::MemoryExhausted {
                    limit: context.policy().limits().max_workspace_bytes,
                })?;
            centers.push(Cow::Owned(selected_point(
                point.original_edge(),
                point.parameter(),
                context,
                progress,
                source_storage,
            )?));
        }
    }
    let prepared = prepare_disks(context, progress, retained)?;
    let source_empty = centers.is_empty()
        && offset.source.curves().is_empty()
        && !boundary.has_areal_faces()
        && boundary.edges.is_empty()
        && boundary.curves.is_empty()
        && boundary.native.as_ref().is_none_or(|native| {
            native.fragments().is_empty() && native.isolated_points().is_empty()
        });
    if matches!(offset.source.region(), PreparedRegion::Whole)
        || (!source_empty && whole_disk(offset.radius.exact(), &prepared, context, progress)?)
    {
        // Validate every original operation image before the global shortcut.
        for curve in offset
            .source
            .curves()
            .iter()
            .chain(polygons.iter().flat_map(PreparedPolygon::rings))
        {
            validate_curve(curve, context, progress)?;
        }
        admit_vertices(5, total, context, retained)?;
        return Ok(GeometryBody::MultiPolygon(vec![whole_surface_ring(
            context.reference().axes(),
        )]));
    }
    certify_center_grid(context, progress)?;
    for curve in offset
        .source
        .curves()
        .iter()
        .chain(ring_polygons.iter().flat_map(PreparedPolygon::rings))
    {
        let remaining = context
            .policy()
            .limits()
            .max_output_elements
            .saturating_sub(*total)
            .div_euclid(9)
            .saturating_sub(centers.len() as u64);
        let generated = cover_curve(curve, offset.radius.exact(), remaining, context, progress)?;
        append_centers(generated, &mut centers, context, progress, retained)?;
    }
    if let Some(native) = &boundary.native {
        for fragment in native.fragments() {
            let remaining = context
                .policy()
                .limits()
                .max_output_elements
                .saturating_sub(*total)
                .div_euclid(9)
                .saturating_sub(centers.len() as u64);
            let parameters = fragment.parameters();
            let generated = cover_edges(
                core::slice::from_ref(fragment.original_edge()),
                Some([parameters[0].bounds().0, parameters[1].bounds().0]),
                offset.radius.exact(),
                remaining,
                context,
                progress,
            )?;
            append_centers(generated, &mut centers, context, progress, retained)?;
        }
    }
    let mut members = Vec::new();
    if boundary.has_areal_faces() {
        let mut area = crate::atlas::union::materialize_with_output(
            offset.source.region(),
            context,
            progress,
            |geometry, available, context, progress| {
                retained.adopt_geometry_output(
                    geometry,
                    available,
                    context,
                    progress,
                    |count, context| admit_vertex_count(count, total, context),
                )
            },
        )?;
        if context.reference().axes() == crate::AxisOrder::LatLon {
            area = swap_area_axes(area, context, progress)?;
        }
        members.push(area);
    }
    if !centers.is_empty() {
        let extra = decimal("0.014002");
        let radius = ExactAdmission::new(context, progress).rational(
            ExactOperation::RationalAdd,
            &[offset.radius.exact(), &extra],
            1,
            || Ok(offset.radius.exact().add(&extra)),
        )?;
        let disks = point_polygons(
            DiskRequest {
                centers: &centers,
                radius: &radius,
                prepared: &prepared,
                profile,
                target,
                global_band: &decimal("0.080"),
            },
            context,
            progress,
            total,
            retained,
            global_output,
        )?;
        members.push(Geometry::new(CoordDim::Xy, disks)?);
    }
    Ok(GeometryBody::GeometryCollection(members))
}

fn append_centers(
    generated: Vec<[Rat; 2]>,
    centers: &mut Vec<Cow<'_, crate::LonLat>>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut MaterializationStorage,
) -> Result<(), GeoError> {
    let bytes = (generated.len() as u64)
        .checked_mul(2048)
        .ok_or(GeoError::ArithmeticOverflow("retained buffer source cover"))?;
    retained.admit_transient(bytes, context)?;
    centers
        .try_reserve(generated.len())
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    for [longitude, latitude] in generated {
        let point = ExactAdmission::new(context, progress).rational(
            ExactOperation::RationalDivide,
            &[&longitude, &latitude],
            8,
            || {
                crate::geodesic::canonical_endpoint(
                    crate::geographic::normalize_degrees(&longitude, -180),
                    latitude.clone(),
                )
            },
        )?;
        centers.push(Cow::Owned(point));
    }
    Ok(())
}

/// Zero is the exact source closure, rather than a positive polygon envelope.
/// An irrational native curve image has no exact coordinate-linear WKT carrier.
fn exact_closure(
    offset: &OffsetRegion,
    boundary: &crate::atlas::boundary::RegionBoundary,
    source_storage: &mut u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
) -> Result<GeometryBody, GeoError> {
    if !offset.source.symbolic_points().is_empty() {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    let mut members = Vec::new();
    for curve in offset.source.curves() {
        let mut points = Vec::new();
        for edge in curve.edges() {
            let crate::PreparedEdge::SourceLinear(edge) = edge else {
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            };
            if points.is_empty() {
                admit_vertices(1, total, context, retained)?;
                points.push(carrier_admitted(
                    edge.start().point(),
                    context,
                    progress,
                    retained,
                )?);
            }
            admit_vertices(1, total, context, retained)?;
            points.push(carrier_admitted(
                edge.end().point(),
                context,
                progress,
                retained,
            )?);
        }
        if !points.is_empty() {
            members.push(Geometry::new(
                CoordDim::Xy,
                GeometryBody::LineString(points),
            )?);
        }
    }
    for point in offset.source.points() {
        admit_vertices(1, total, context, retained)?;
        members.push(Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(carrier_admitted(
                point.point(),
                context,
                progress,
                retained,
            )?)),
        )?);
    }
    if let Some(native) = &boundary.native {
        for fragment in native.fragments().iter().filter(|fragment| {
            fragment.stratum() == crate::atlas::arcs::SelectedFragmentStratum::CurveInterior
        }) {
            admit_vertices(2, total, context, retained)?;
            let mut points = Vec::with_capacity(2);
            for parameter in fragment.parameters() {
                let point = selected_point(
                    fragment.original_edge(),
                    parameter,
                    context,
                    progress,
                    source_storage,
                )?;
                points.push(carrier_admitted(&point, context, progress, retained)?);
            }
            members.push(Geometry::new(
                CoordDim::Xy,
                GeometryBody::LineString(points),
            )?);
        }
        for node in native.isolated_points() {
            let point = selected_point(
                node.original_edge(),
                node.parameter(),
                context,
                progress,
                source_storage,
            )?;
            admit_vertices(1, total, context, retained)?;
            members.push(Geometry::new(
                CoordDim::Xy,
                GeometryBody::Point(Some(carrier_admitted(&point, context, progress, retained)?)),
            )?);
        }
    }
    if boundary.has_areal_faces() {
        let area = crate::atlas::union::materialize_with_output(
            offset.source.region(),
            context,
            progress,
            |geometry, available, context, progress| {
                retained.adopt_geometry_output(
                    geometry,
                    available,
                    context,
                    progress,
                    |count, context| admit_vertex_count(count, total, context),
                )
            },
        )?;
        members.push(if context.reference().axes() == crate::AxisOrder::LatLon {
            swap_area_axes(area, context, progress)?
        } else {
            area
        });
    }
    Ok(GeometryBody::GeometryCollection(members))
}

fn exact_linear_domain(
    edge: &crate::PreparedEdge,
    parameters: [&crate::atlas::arcs::arrangement::SourceParameter; 2],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)?;
    if !matches!(edge, crate::PreparedEdge::SourceLinear(_)) {
        return Ok(false);
    }
    for parameter in parameters {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if parameter.exact_value().is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn selected_point(
    edge: &crate::PreparedEdge,
    parameter: &crate::atlas::arcs::arrangement::SourceParameter,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    source_storage: &mut u64,
) -> Result<crate::LonLat, GeoError> {
    if !exact_linear_domain(edge, [parameter; 2], context, progress)? {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    let crate::PreparedEdge::SourceLinear(edge) = edge else {
        unreachable!("certified exact affine domain")
    };
    let parameter = parameter.exact_value().expect("original exact affine cut");
    let mut admission = ExactAdmission::new(context, progress);
    let longitude = crate::SourceLinearEdge::interpolate_ordinate_retained(
        edge.start().point().longitude(),
        edge.end().point().longitude(),
        parameter,
        &mut admission,
        source_storage,
    )?;
    let latitude = crate::SourceLinearEdge::interpolate_ordinate_retained(
        edge.start().point().latitude(),
        edge.end().point().latitude(),
        parameter,
        &mut admission,
        source_storage,
    )?;
    admission.rational_owner(
        ExactOperation::RationalCompare,
        &[&longitude, &latitude],
        source_storage,
        || crate::LonLat::new(longitude.clone(), latitude.clone()),
    )?
}

fn validate_curve(
    curve: &PreparedCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let policy = context.policy();
    crate::atlas::arcs::with_lines(
        curve.edges(),
        80,
        context,
        progress,
        |lines, math, progress| {
            let unit = (|| {
                FixedInterval::from_i64(0, math)?.hull(&FixedInterval::from_i64(1, math)?, math)
            })()
            .map_err(|error| geo_math_error(&error, policy))?;
            for line in lines {
                line.normal(&unit, policy.limits().max_iterations, math, progress)
                    .map_err(|error| geo_math_error(&error, policy))?;
            }
            Ok(())
        },
    )
}

fn cover_curve(
    curve: &PreparedCurve,
    source_radius: &Rat,
    maximum: u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<[Rat; 2]>, GeoError> {
    cover_edges(
        curve.edges(),
        None,
        source_radius,
        maximum,
        context,
        progress,
    )
}

fn cover_edges(
    edges: &[crate::PreparedEdge],
    parameters: Option<[&Rat; 2]>,
    source_radius: &Rat,
    maximum: u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<[Rat; 2]>, GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    context.prepare_integer_scratch_for_observed(
        parameters
            .into_iter()
            .flatten()
            .chain([source_radius])
            .map(crate::numerical::rational_operand_bits)
            .max()
            .unwrap_or(1),
        progress,
    )?;
    crate::atlas::arcs::with_lines(edges, 80, context, progress, |lines, math, progress| {
        let result = (|| {
            let (zero, one) = if let Some([lower, upper]) = parameters {
                (fixed_from_rat(lower, math)?, fixed_from_rat(upper, math)?)
            } else {
                (
                    FixedInterval::from_i64(0, math)?,
                    FixedInterval::from_i64(1, math)?,
                )
            };
            let pi = FixedInterval::pi(math)?;
            let half = FixedInterval::from_i64(2, math)?;
            let radius = fixed_from_rat(
                reference.ellipsoid().normal_metric_bounds_ref().1.exact(),
                math,
            )?;
            let angular_scale = radius.mul(&pi, math)?.div(&half, math)?;
            let target = fixed_from_rat(&decimal("0.014"), math)?;
            let degrees = FixedInterval::from_i64(180, math)?.div(&pi, math)?;
            let source_radius = fixed_from_rat(source_radius, math)?;
            // Expand the difference of squares to avoid a cancellation
            // enclosure whose uncertainty could select a successful panel.
            let geodesic_square = source_radius
                .mul(&target, math)?
                .mul(&FixedInterval::from_i64(8, math)?, math)?
                .add(
                    &target
                        .square(math)?
                        .mul(&FixedInterval::from_i64(4, math)?, math)?,
                    math,
                )?;
            let mut positions = Positions {
                degrees: &degrees,
                iterations: policy.limits().max_iterations,
                maximum,
                output: Vec::new(),
            };
            math.reserve_workspace((policy.limits().max_subdivision_levels as usize + 2) * 4096)?;
            for line in lines {
                let geodesic_length = if let crate::atlas::arcs::ChartLine::Geodesic(line) = line {
                    Some(line.travel_length().clone())
                } else {
                    None
                };
                if geodesic_length.is_some() {
                    positions.append(line, &zero, math, progress)?;
                }
                let mut pending = purrdf_lex::walk::WorkList::<
                    (FixedInterval, FixedInterval, u32),
                    8,
                >::with((zero.clone(), one.clone(), 0));
                while let Some((lower, upper, depth)) = pending.pop() {
                    progress.math_poll(math)?;
                    let panel = lower.hull(&upper, math)?;
                    let complete = if let Some(length) = &geodesic_length {
                        upper
                            .sub(&lower, math)?
                            .mul(length, math)?
                            .square(math)?
                            .upper()
                            <= geodesic_square.lower()
                    } else {
                        let (normal, _) =
                            line.normal(&panel, policy.limits().max_iterations, math, progress)?;
                        let mut chord = FixedInterval::from_i64(0, math)?;
                        for component in normal {
                            chord = chord.add(&component.width(math)?, math)?;
                        }
                        chord.mul(&angular_scale, math)?.upper() <= target.lower()
                    };
                    if complete {
                        let parameter = if geodesic_length.is_some() {
                            upper
                        } else {
                            panel.midpoint(math)?
                        };
                        positions.append(line, &parameter, math, progress)?;
                    } else {
                        if depth >= policy.limits().max_subdivision_levels {
                            return Err(MathError::PrecisionExhausted);
                        }
                        let midpoint = panel.midpoint(math)?;
                        pending.push((midpoint.clone(), upper, depth + 1));
                        pending.push((lower, midpoint, depth + 1));
                    }
                }
            }
            Ok(positions.output)
        })();
        match result {
            Err(MathError::Domain("complete buffer cover exceeds output admission")) => {
                Err(GeoError::OutputExhausted {
                    limit: policy.limits().max_output_elements,
                })
            }
            result => result.map_err(|error| geo_math_error(&error, policy)),
        }
    })
}

struct Positions<'a> {
    degrees: &'a FixedInterval,
    iterations: u32,
    maximum: u64,
    output: Vec<[Rat; 2]>,
}
impl Positions<'_> {
    fn append(
        &mut self,
        line: &crate::atlas::arcs::ChartLine,
        parameter: &FixedInterval,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), MathError> {
        if self.output.len() as u64 >= self.maximum {
            return Err(MathError::Domain(
                "complete buffer cover exceeds output admission",
            ));
        }
        let ([longitude, latitude], _) = line.image(parameter, self.iterations, math, progress)?;
        let latitude = quantize(&latitude.mul(self.degrees, math)?, math, progress)?;
        let longitude = if latitude.abs() == Rat::from_i64(90) {
            Rat::zero()
        } else {
            quantize(&longitude.mul(self.degrees, math)?, math, progress)?
        };
        math.reserve_workspace(2048)?;
        self.output
            .try_reserve(1)
            .map_err(|_| MathError::WorkspaceExhausted)?;
        self.output.push([longitude, latitude]);
        Ok(())
    }
}
fn quantize(
    value: &FixedInterval,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, MathError> {
    let (lower, upper) = value.round_decimal(15, math)?;
    math.admit_exact_cost(
        ExactArithmeticCost::for_operation(
            ExactOperation::Linear,
            lower.bits_upper_bound().max(upper.bits_upper_bound()) as u64,
            1,
        )
        .ok_or(MathError::WorkExhausted)?,
    )?;
    if lower != upper {
        return Err(MathError::PrecisionExhausted);
    }
    math_quantized_decimal(&lower, 15, math, progress)
}
pub(super) fn swap_area_axes(
    area: Geometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Geometry, GeoError> {
    let (dim, body) = area.into_parts();
    let GeometryBody::MultiPolygon(mut polygons) = body else {
        unreachable!("labelled atlas union produces polygons");
    };
    // The producer's adopted receipt remains exact: every container capacity
    // and original limb owner stays in place while only the axis slots move.
    for polygon in &mut polygons {
        for ring in polygon {
            for point in ring {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                point.swap_xy();
            }
        }
    }
    Geometry::new(dim, GeometryBody::MultiPolygon(polygons))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AzimuthLengthArc, Coord, LonLat, Metres, PreparedCoordinate, PreparedEdge, PreparedGeodesic,
    };

    #[test]
    fn geodesic_source_cover_uses_the_complete_squared_distance_bound() {
        let mut context = MetricContext::wgs84().unwrap();
        let reference = context.reference().clone();
        let start =
            PreparedCoordinate::new(Coord::xy(Rat::zero(), Rat::zero()), &reference).unwrap();
        let arc = AzimuthLengthArc::new(
            start,
            Rat::from_i64(90),
            Metres::new(Rat::from_i64(100)),
            &mut context,
        )
        .unwrap();
        let curve = PreparedCurve::new(vec![PreparedEdge::AzimuthLength(Box::new(arc))]);
        context.begin(1).unwrap();
        let positions = cover_curve(
            &curve,
            &Rat::from_i64(1000),
            32,
            &mut context,
            &mut WorkProgress::new(None),
        )
        .unwrap();
        assert_eq!(positions.len(), 17);
        let positions = positions
            .into_iter()
            .map(|[longitude, latitude]| LonLat::new(longitude, latitude).unwrap())
            .collect::<Vec<_>>();
        let geodesic = PreparedGeodesic::new(reference);
        let origin = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        for travel in [0, 25, 50, 75, 100] {
            let source = geodesic
                .direct(
                    &origin,
                    &Rat::from_i64(90),
                    &Metres::new(Rat::from_i64(travel)),
                    &mut context,
                )
                .unwrap()
                .endpoint()
                .clone();
            for azimuth in [0, 90, 180, 270] {
                let point = geodesic
                    .direct(
                        &source,
                        &Rat::from_i64(azimuth),
                        &Metres::new(Rat::from_i64(1000)),
                        &mut context,
                    )
                    .unwrap()
                    .endpoint()
                    .clone();
                let nearest = positions
                    .iter()
                    .map(|center| {
                        geodesic
                            .distance(center, &point, &mut context)
                            .unwrap()
                            .value()
                            .exact()
                            .clone()
                    })
                    .min()
                    .unwrap();
                assert!(nearest.add(&decimal("0.0000005")) < decimal("1000.014003"));
            }
        }
    }
}
