// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A complete physical-distance sublevel cover in the existing cube hierarchy.
//!
//! The chart home supplies a closed, outward15 coordinate rectangle containing
//! each complete assigned footprint, and H enclosing every point of that actual
//! rectangle about its axis q. Distance to any closed source is 1-Lipschitz.
//! Discard only D(q,S)>r+H. Emit inside only D(q,S)<=r-H. Otherwise refine until
//! 2H<=0.099998m; failure of the outside predicate proves every rectangle point
//! is within r+2H of S. Six roots cover the surface, so these rules prove both
//! required inclusions without treating a direct geodesic circle as a boundary
//! beyond its cut locus. The original physical comparator proves each decision.
//!
//! Rectangle walls are already exact decimals on the output grid. Their union
//! uses the shared labelled coordinate atlas; every intersection is a pair of
//! those same walls, so the union does not move an outer boundary when rounded.

use super::{OffsetRegion, admit_vertex_count, admit_vertices, carrier};
use crate::carrier::MaterializationStorage;
use crate::cells::{CellCarrierBounds, CellId, NativeGridProfile, cell_carrier_bounds};
use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{
    Crs, GeoError, GeoProfile, Geometry, GeometryBody, GeometryLiteral, LonLat, Metres,
    MetricContext, PreparedGeodesic, Rat,
};
use purrdf_xsd::integer::ExactOperation;

pub(super) enum Source<'a> {
    Point(&'a LonLat, &'a PreparedGeodesic),
    Complete(&'a OffsetRegion),
}

pub(super) struct Request<'a> {
    pub(super) source: Source<'a>,
    pub(super) radius: &'a Rat,
    pub(super) profile: &'a GeoProfile,
    pub(super) band: &'a Rat,
    pub(super) target: &'a Crs,
}

pub(super) fn materialize(
    request: &Request<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    storage: &mut MaterializationStorage,
) -> Result<GeometryBody, GeoError> {
    // A cell is only the common angular partition here. The actual reference's
    // axes and metric factors bound its carrier; no datum conversion occurs.
    let grid = NativeGridProfile::Wgs84;
    let output_start = storage.output_bytes();
    let total_start = *total;
    // Depth-first traversal has at most six roots plus three siblings per level.
    const FRONTIER: usize = 96;
    let bytes = (FRONTIER * size_of::<CellId>()) as u64;
    storage.admit_transient(bytes, context)?;
    progress.context_poll(context)?;
    let mut pending = Vec::with_capacity(FRONTIER);
    for face in (0..6).rev() {
        pending.push(CellId::root(grid.id(), face)?);
    }
    let mut rectangles = Vec::new();
    let result = (|| {
        while let Some(cell) = pending.pop() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            let bounds = cell_carrier_bounds(cell, context, progress)?;
            let retained = bounds.retained_workspace_bytes;
            let visit = (|| {
                let decision =
                    classify(&request.source, request.radius, &bounds, context, progress)?;
                let use_cell = match decision {
                    Decision::Outside => false,
                    Decision::Inside => true,
                    Decision::Boundary => narrow(&bounds, request.band, context, progress)?,
                };
                if use_cell {
                    append_rectangles(&bounds, &mut rectangles, context, progress, total, storage)
                } else if decision == Decision::Boundary {
                    // At a native leaf the complete wall guard is far below the
                    // band. Larger caller axes may make the fixed output grid
                    // incapable of certifying the requested displacement.
                    if cell.level() == crate::cells::MAX_LEVEL {
                        return Err(GeoError::PrecisionExhausted {
                            bits: context.policy().limits().max_precision_bits,
                        });
                    }
                    for child in cell.children()?.into_iter().rev() {
                        pending.push(child);
                    }
                    Ok(())
                } else {
                    Ok(())
                }
            })();
            drop(bounds);
            context.release_workspace(retained)?;
            visit?;
        }
        if rectangles.is_empty() {
            return Ok(GeometryBody::GeometryCollection(Vec::new()));
        }
        let literal = GeometryLiteral::new(
            request.target.clone(),
            Geometry::new(crate::CoordDim::Xy, GeometryBody::MultiPolygon(rectangles))?,
        );
        let original_output = storage.output_bytes() - output_start;
        let body = union(
            literal,
            request.profile,
            context,
            progress,
            total_start,
            total,
            storage,
        )?;
        storage.release_output(original_output, context)?;
        Ok(body)
    })();
    storage.release_transient(bytes, context)?;
    result
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Outside,
    Inside,
    Boundary,
}

fn classify(
    source: &Source<'_>,
    radius: &Rat,
    bounds: &CellCarrierBounds,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Decision, GeoError> {
    let h = bounds.bbox_radius.exact();
    let outer = ExactAdmission::new(context, progress).rational(
        ExactOperation::RationalAdd,
        &[radius, h],
        1,
        || Ok(Metres::new(radius.add(h))),
    )?;
    if !within(source, &bounds.axis, &outer, context, progress)? {
        return Ok(Decision::Outside);
    }
    let inner = ExactAdmission::new(context, progress).rational(
        ExactOperation::RationalAdd,
        &[radius, h],
        1,
        || Ok(Metres::new(radius.sub(h))),
    )?;
    if within(source, &bounds.axis, &inner, context, progress)? {
        Ok(Decision::Inside)
    } else {
        Ok(Decision::Boundary)
    }
}

fn within(
    source: &Source<'_>,
    point: &LonLat,
    radius: &Metres,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if radius.exact().signum() < 0 {
        return Ok(false);
    }
    match source {
        Source::Complete(offset) => {
            offset.contains_at_radius_with_progress(point, radius, context, progress)
        }
        Source::Point(center, prepared) => {
            let mut child = context.remaining_child()?;
            let answer = {
                let mut observer = progress.nested(
                    context.work_items(),
                    context
                        .policy()
                        .limits()
                        .max_workspace_bytes
                        .saturating_sub(context.remaining_workspace()),
                    context.workspace_peak(),
                );
                prepared.within_physical_metered(center, point, radius, &mut child, &mut observer)
            };
            progress.absorb_child_result(context, &child, answer)
        }
    }
}

fn narrow(
    bounds: &CellCarrierBounds,
    band: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let two = Rat::from_i64(2);
    ExactAdmission::new(context, progress).rational(
        ExactOperation::RationalMultiply,
        &[bounds.bbox_radius.exact(), &two, band],
        2,
        || Ok(bounds.bbox_radius.exact().mul(&two) <= *band),
    )
}

fn append_rectangles(
    bounds: &CellCarrierBounds,
    output: &mut Vec<Vec<Vec<crate::Coord>>>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    storage: &mut MaterializationStorage,
) -> Result<(), GeoError> {
    for (west, east) in &bounds.longitudes {
        admit_vertices(5, total, context, storage)?;
        progress.context_poll(context)?;
        let axes = context.reference().axes();
        let ring = ExactAdmission::new(context, progress).rational(
            ExactOperation::Linear,
            &[west, east, &bounds.south, &bounds.north],
            10,
            || {
                [
                    (west, &bounds.south),
                    (east, &bounds.south),
                    (east, &bounds.north),
                    (west, &bounds.north),
                    (west, &bounds.south),
                ]
                .into_iter()
                .map(|(longitude, latitude)| {
                    Ok(carrier(
                        &LonLat::new(longitude.clone(), latitude.clone())?,
                        axes,
                    ))
                })
                .collect::<Result<Vec<_>, GeoError>>()
            },
        )?;
        output
            .try_reserve_exact(1)
            .map_err(|_| GeoError::MemoryExhausted {
                limit: context.policy().limits().max_workspace_bytes,
            })?;
        output.push(vec![ring]);
    }
    Ok(())
}

fn union(
    literal: GeometryLiteral,
    profile: &GeoProfile,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total_start: u64,
    total: &mut u64,
    storage: &mut MaterializationStorage,
) -> Result<GeometryBody, GeoError> {
    let occupied = context
        .policy()
        .limits()
        .max_workspace_bytes
        .saturating_sub(context.remaining_workspace());
    let policy = context
        .policy()
        .remaining_after(context.work_items(), occupied)?;
    let preparation = {
        let mut observer =
            progress.nested(context.work_items(), occupied, context.workspace_peak());
        crate::PreparedGeometry::from_literal_in_policy_metered(
            &literal,
            profile,
            policy,
            &mut observer,
        )
    };
    let prepared = progress.absorb_nested(context, preparation)?;
    let retained = prepared.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    drop(literal);
    let result = crate::atlas::union::materialize_with_output(
        prepared.region(),
        context,
        progress,
        |geometry, available, context, progress| {
            storage.adopt_geometry_output(
                geometry,
                available,
                context,
                progress,
                |count, context| {
                    let mut replacement = total_start;
                    admit_vertex_count(count, &mut replacement, context)?;
                    *total = replacement;
                    Ok(())
                },
            )
        },
    );
    drop(prepared);
    context.release_workspace(retained)?;
    let mut result = result?;
    if context.reference().axes() == crate::AxisOrder::LatLon {
        result = super::covering::swap_area_axes(result, context, progress)?;
    }
    Ok(result.into_parts().1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AxisOrder, ExecutionLimits, ExecutionPolicy, GeographicReference, PreparedEllipsoid,
    };
    use purrdf_hash::hex::Digest32;
    use std::sync::Arc;

    fn rat(value: &str) -> Rat {
        Rat::parse_decimal(value).unwrap()
    }

    fn small_reference() -> (GeoProfile, Crs, GeographicReference) {
        let reference = GeographicReference::new(
            PreparedEllipsoid::new(Rat::one(), rat("298.257223563")).unwrap(),
            Digest32::new([73; 32]),
            AxisOrder::LonLat,
        );
        let target = Crs::new("https://example.org/geographic-unit-ellipsoid").unwrap();
        let mut profile = GeoProfile::standard();
        profile
            .register_reference(target.clone(), reference.clone())
            .unwrap();
        (profile, target, reference)
    }

    fn offset(profile: &GeoProfile, target: &Crs, radius: &str) -> OffsetRegion {
        let literal = crate::wkt::parse("POINT (19 45)", target).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, profile).unwrap());
        OffsetRegion::new(
            source,
            Metres::new(rat(radius)),
            ExecutionPolicy::geometry(),
        )
        .unwrap()
    }

    #[test]
    fn global_cube_sublevel_refuses_actual_100m_work_without_partial_output() {
        let (profile, target, reference) = small_reference();
        let offset = offset(&profile, &target, "3");
        let limits = ExecutionLimits {
            max_work_items: 100_000_000,
            max_workspace_bytes: 256 * 1024 * 1024,
            ..ExecutionLimits::GEOMETRY
        };
        let mut context =
            MetricContext::new(reference, ExecutionPolicy::new(limits).unwrap()).unwrap();
        let result = offset.materialize(&profile, &target, &mut context);
        assert!(
            // A refused child preserves its original remaining-work limit;
            // the enclosing invocation is explicitly admitted at 100M.
            matches!(result, Err(GeoError::WorkExhausted { limit }) if limit <= limits.max_work_items),
            "unexpected complete materialization result: {result:?}"
        );
        assert!(context.work_items() <= limits.max_work_items);
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            limits.max_workspace_bytes
        );
    }

    #[test]
    fn adequately_admitted_cube_sublevel_contains_disk_and_excludes_outer_band() {
        let (profile, target, reference) = small_reference();
        // This is beyond the local 3m proof but below the meridian diameter's
        // whole-surface band. All metric comparisons retain the original axes.
        let offset = offset(&profile, &target, "3");
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 50_000_000_000;
        // This complete global approximation retains the outward cell carriers,
        // their original union source and its exact arrangement simultaneously.
        // The invocation explicitly admits those outputs; native defaults stay
        // unchanged and their independent refusal regression remains in force.
        limits.max_workspace_bytes = 256 * 1024 * 1024;
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::new(limits).unwrap()).unwrap();
        let output = offset.materialize(&profile, &target, &mut context).unwrap();
        assert_eq!(
            output.law_id(),
            super::super::BufferMaterialization::global_output_law_id()
        );
        let prepared = crate::PreparedGeometry::from_literal_in_policy(
            output.literal(),
            &profile,
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let geodesic = PreparedGeodesic::new(reference.clone());
        let center = &offset.source.points()[0];
        let grid = [-90, -75, -45, 0, 45, 75, 90]
            .into_iter()
            .flat_map(|latitude| {
                [-180, -120, -60, 0, 60, 120, 180]
                    .into_iter()
                    .map(move |longitude| (longitude, latitude))
            });
        let mut included = 0;
        let mut excluded = 0;
        for (longitude, latitude) in grid.chain([(-161, -45), (-160, -45), (-162, -45)]) {
            let point = LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
            let mut metric =
                MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
            let inside = geodesic
                .within_physical(center.point(), &point, &Metres::new(rat("3")), &mut metric)
                .unwrap();
            let outside = !geodesic
                .within_physical(
                    center.point(),
                    &point,
                    &Metres::new(rat("3.1")),
                    &mut metric,
                )
                .unwrap();
            let mut locate =
                MetricContext::new(reference.clone(), ExecutionPolicy::new(limits).unwrap())
                    .unwrap();
            let carrier = crate::atlas::locate(&point, prepared.region(), &mut locate).unwrap();
            if inside {
                assert_ne!(carrier, crate::Set::Exterior);
                included += 1;
            }
            if outside {
                assert_eq!(carrier, crate::Set::Exterior);
                excluded += 1;
            }
        }
        assert!(included > 0 && excluded > 0);
        assert!(!matches!(prepared.region(), crate::PreparedRegion::Whole));
        assert!(output.literal().geometry().coords().count() as u64 <= limits.max_output_elements);
        let receipt = output.output_receipt();
        drop(output);
        context.release_materialized_output(receipt).unwrap();
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            limits.max_workspace_bytes
        );
    }

    #[test]
    fn native_pole_crossing_refuses_actual_work_without_partial_output() {
        let profile = GeoProfile::standard();
        let target = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("POINT (19 89.999)", &target).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let offset =
            OffsetRegion::new(source, Metres::new(rat("200")), ExecutionPolicy::geometry())
                .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        assert!(matches!(
            offset.materialize(&profile, &target, &mut context),
            Err(GeoError::WorkExhausted { .. })
        ));
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy().limits().max_workspace_bytes
        );
    }

    #[test]
    fn cell_band_closed_boundary_is_exact_and_not_proof_selected() {
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        let mut bounds = CellCarrierBounds {
            axis: LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
            south: Rat::zero(),
            north: Rat::zero(),
            longitudes: Vec::new(),
            bbox_radius: Metres::new(rat("0.049999")),
            retained_workspace_bytes: 0,
        };
        assert!(narrow(&bounds, &rat("0.099998"), &mut context, &mut progress).unwrap());
        bounds.bbox_radius = Metres::new(rat("0.049999000000000001"));
        assert!(!narrow(&bounds, &rat("0.099998"), &mut context, &mut progress).unwrap());
    }
}
