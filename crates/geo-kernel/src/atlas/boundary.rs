// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete selected physical strata and exact written-region boundaries.

use crate::context::WorkProgress;
use crate::{Coord, GeoError, GeometryBody, LonLat, MetricContext, PreparedRegion, Rat};

pub(crate) struct RegionBoundary {
    pub(crate) edges: Vec<[LonLat; 2]>,
    pub(crate) curves: Vec<crate::PreparedCurve>,
    pub(crate) native: Option<super::arcs::arrangement::NativeBoundaryArrangement>,
    pub(crate) workspace_bytes: u64,
    has_areal_faces: bool,
}

impl RegionBoundary {
    /// Whether the selected set contains a certified open two-dimensional face.
    pub(crate) const fn has_areal_faces(&self) -> bool {
        self.has_areal_faces
    }
}

pub(crate) fn region_boundary(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<RegionBoundary, GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    region.check_binding_admitted(binding, context, progress)?;
    let polygons = match region {
        PreparedRegion::Empty | PreparedRegion::Whole => {
            return Ok(RegionBoundary {
                edges: Vec::new(),
                curves: Vec::new(),
                native: None,
                workspace_bytes: 0,
                has_areal_faces: matches!(region, PreparedRegion::Whole),
            });
        }
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons
        }
    };
    if polygons.len() == 1
        && polygons[0].oriented_interior().is_some()
        && polygons[0].rings().len() == 1
    {
        let curve = &polygons[0].rings()[0];
        let bytes = super::arcs::source_workspace_bound(curve.edges().iter())
            .saturating_add(size_of::<crate::PreparedCurve>() as u64);
        context.admit_workspace(bytes)?;
        context.charge_work(1)?;
        progress.context_poll(context)?;
        return Ok(RegionBoundary {
            edges: Vec::new(),
            curves: vec![curve.clone()],
            native: None,
            workspace_bytes: bytes,
            // Preparation proved that this one original ring is Jordan. Both
            // of its explicitly selected sides contain an open surface face.
            has_areal_faces: true,
        });
    }
    // The written fast path discards pieces whose transverse faces agree.
    // This is complete for unions and closed complements of regular rectangle
    // polygons, including strictly interior pairwise disjoint rectangle holes.
    // Other rings can retain one- or zero-dimensional selected support; publish
    // those through the shared relation graph rather than dropping their image.
    let mut regular_rectangles = true;
    for polygon in polygons.iter() {
        let Some(chart) = polygon.chart() else {
            regular_rectangles = false;
            break;
        };
        let GeometryBody::Polygon(rings) = chart.body() else {
            unreachable!("prepared polygon chart");
        };
        if !regular_rectangle_polygon(rings, context, progress)? {
            regular_rectangles = false;
            break;
        }
    }
    if !regular_rectangles {
        let boundary = super::arcs::arrangement::native_boundary_in(region, context, progress)?;
        let workspace_bytes = boundary.retained_workspace_bytes();
        let has_areal_faces = boundary.has_areal_faces();
        context.admit_workspace(workspace_bytes)?;
        return Ok(RegionBoundary {
            edges: Vec::new(),
            curves: Vec::new(),
            native: Some(boundary),
            workspace_bytes,
            has_areal_faces,
        });
    }
    let bits = polygons
        .iter()
        .map(crate::PreparedPolygon::coordinate_bits)
        .fold(0, u64::saturating_add);
    let coordinates = polygons
        .iter()
        .map(crate::PreparedPolygon::coordinate_count)
        .fold(0, u64::saturating_add);
    let scratch = bits
        .div_ceil(8)
        .saturating_mul(128)
        .saturating_add(coordinates.saturating_mul(512))
        .saturating_add(65_536);
    context.admit_workspace(scratch)?;
    let mut retained = scratch;
    let result = (|| {
        let mut edges = Vec::new();
        for polygon in polygons.iter() {
            let chart = polygon
                .chart()
                .ok_or_else(|| GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                })?;
            let GeometryBody::Polygon(rings) = chart.body() else {
                unreachable!("prepared polygon chart");
            };
            edges.extend(
                rings
                    .iter()
                    .flat_map(|ring| ring.windows(2).map(|pair| (&pair[0], &pair[1]))),
            );
        }
        // The region law unions source sets; repeated and reversed copies of
        // the same complete segment do not add noding events or boundary length.
        // Keep every original polygon for the side labels, and deduplicate only
        // this undirected arrangement inventory before its quadratic pair walk.
        for edge in &mut edges {
            let order = edge_order(*edge, context, progress)?;
            if order.is_gt() {
                *edge = (edge.1, edge.0);
            }
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut edges, |a, b| {
            edge_pair_order(*a, *b, context, progress)
        })?;
        let mut kept = usize::from(!edges.is_empty());
        for read in 1..edges.len() {
            if !edge_pair_order(edges[kept - 1], edges[read], context, progress)?.is_eq() {
                edges.swap(kept, read);
                kept += 1;
            }
        }
        edges.truncate(kept);
        let events = identified_events(
            &edges,
            edges.iter().flat_map(|&pair| <[_; 2]>::from(pair)),
            context,
            progress,
            &mut retained,
        )?;

        let event_bits = events
            .iter()
            .map(coordinate_bytes)
            .fold(0, u64::saturating_add);
        let temporary = event_bits
            .saturating_mul(8)
            .saturating_add((events.len() as u64).saturating_mul(256));
        context.admit_workspace(temporary)?;
        retained = retained
            .checked_add(temporary)
            .ok_or(GeoError::ArithmeticOverflow("boundary sorting workspace"))?;
        let mut output = Vec::new();
        for edge in edges {
            let fragments = crate::topology::noding::fragments_admitted(
                edge,
                &events,
                context,
                progress,
                &mut retained,
            )?;

            for (start, end) in fragments {
                let midpoint = crate::topology::midpoint_admitted(
                    start,
                    end,
                    &mut crate::numerical::ExactAdmission::new(context, progress),
                )?;
                if !super::written_boundary_sides(&midpoint, start, end, region, context, progress)?
                {
                    progress.context_poll(context)?;
                    continue;
                }

                let mut pair = [
                    physical_point(start, context, progress)?,
                    physical_point(end, context, progress)?,
                ];
                // Only a complete cut-meridian spelling is changed. Written
                // long paths retain their original affine longitude variation.
                let on_east_cut = crate::numerical::ExactAdmission::new(context, progress)
                    .rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &[pair[0].longitude(), pair[1].longitude()],
                        2,
                        || {
                            Ok(pair
                                .iter()
                                .all(|point| point.longitude() == &Rat::from_i64(180)))
                        },
                    )?;
                if on_east_cut {
                    for point in &mut pair {
                        *point = crate::numerical::ExactAdmission::new(context, progress)
                            .rational(
                                purrdf_xsd::integer::ExactOperation::RationalCompare,
                                &[point.latitude()],
                                8,
                                || LonLat::new(Rat::from_i64(-180), point.latitude().clone()),
                            )?;
                    }
                }
                let collapsed_pole = crate::numerical::ExactAdmission::new(context, progress)
                    .rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &[
                            pair[0].longitude(),
                            pair[0].latitude(),
                            pair[1].longitude(),
                            pair[1].latitude(),
                        ],
                        4,
                        || Ok(pair[0].same_location(&pair[1]) && pair[0].is_pole()),
                    )?;
                if collapsed_pole {
                    continue;
                }
                if point_order(&pair[0], &pair[1], context, progress)?.is_gt() {
                    pair.swap(0, 1);
                }
                let bytes = coordinate_bytes(start)
                    .saturating_add(coordinate_bytes(end))
                    .saturating_mul(2);
                context.admit_workspace(bytes)?;
                retained = retained
                    .checked_add(bytes)
                    .ok_or(GeoError::ArithmeticOverflow("boundary fragments"))?;
                output.push(pair);
            }
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut output, |a, b| {
            boundary_order(a, b, context, progress)
        })?;
        let mut kept = usize::from(!output.is_empty());
        for read in 1..output.len() {
            if !boundary_order(&output[kept - 1], &output[read], context, progress)?.is_eq() {
                output.swap(kept, read);
                kept += 1;
            }
        }
        output.truncate(kept);

        let output = coalesce_boundary(output, context, progress)?;
        let has_areal_faces = if output.is_empty() {
            // Complete absence of a physical boundary, after regularity and
            // full noding proofs, makes the selected surface both open and
            // closed. Connectedness reduces it to Whole or Empty, so this
            // exact membership chooses between two already proved cases.
            let origin = LonLat::new(Rat::from_i64(0), Rat::from_i64(0))?;
            super::locate_validated_with_progress(&origin, region, context, progress)?
                == crate::de9im::Set::Interior
        } else {
            true
        };
        Ok((output, has_areal_faces))
    })();
    match result {
        Ok((edges, has_areal_faces)) => Ok(RegionBoundary {
            edges,
            curves: Vec::new(),
            native: None,
            workspace_bytes: retained,
            has_areal_faces,
        }),
        Err(error) => {
            context.release_workspace(retained)?;
            Err(error)
        }
    }
}

/// Prove a regular closed written polygon using the one exact ring-corner home.
/// Strictly interior holes avoid both identified chart seams and poles. Their
/// pairwise closed disjointness prevents lower-dimensional selected remnants.
fn regular_rectangle_polygon(
    rings: &crate::Rings,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let Some((outer, holes)) = rings.split_first() else {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        return Ok(false);
    };
    for ring in rings {
        if !super::chart_index::is_rectangle_ring(ring, context, progress)? {
            return Ok(false);
        }
    }
    if holes.is_empty() {
        return Ok(true);
    }
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    let outer = crate::measure::borrowed_bounds(outer.iter(), Some(&mut admission))?
        .expect("nonempty certified rectangle");
    for (index, hole) in holes.iter().enumerate() {
        let current = crate::measure::borrowed_bounds(hole.iter(), Some(&mut admission))?
            .expect("nonempty certified rectangle");
        for (outside, inside) in [
            (outer.west, current.west),
            (current.east, outer.east),
            (outer.south, current.south),
            (current.north, outer.north),
        ] {
            if !admission.compare(outside, inside)?.is_lt() {
                return Ok(false);
            }
        }
        for previous in &holes[..index] {
            let previous = crate::measure::borrowed_bounds(previous.iter(), Some(&mut admission))?
                .expect("nonempty certified rectangle");
            let mut disjoint = false;
            for (maximum, minimum) in [
                (previous.east, current.west),
                (current.east, previous.west),
                (previous.north, current.south),
                (current.north, previous.south),
            ] {
                if admission.compare(maximum, minimum)?.is_lt() {
                    disjoint = true;
                    break;
                }
            }
            if !disjoint {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(crate) fn coordinate_bytes(point: &Coord) -> u64 {
    ordinate_bytes([point.x(), point.y()])
}
pub(crate) fn ordinate_bytes(ordinates: [&Rat; 2]) -> u64 {
    ordinates.into_iter().fold(256_u64, |bytes, value| {
        bytes.saturating_add(
            value
                .numerator()
                .bit_len()
                .saturating_add(value.denominator().bit_len())
                .div_ceil(8),
        )
    })
}
fn compare(a: &LonLat, b: &LonLat) -> core::cmp::Ordering {
    a.longitude()
        .cmp(b.longitude())
        .then_with(|| a.latitude().cmp(b.latitude()))
}

fn point_order(
    a: &LonLat,
    b: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
        2,
        || Ok(compare(a, b)),
    )
}

fn edge_order(
    edge: (&Coord, &Coord),
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[edge.0.x(), edge.0.y(), edge.1.x(), edge.1.y()],
        2,
        || Ok(crate::topology::cmp_xy(edge.0, edge.1)),
    )
}

fn edge_pair_order(
    a: (&Coord, &Coord),
    b: (&Coord, &Coord),
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[
            a.0.x(),
            a.0.y(),
            a.1.x(),
            a.1.y(),
            b.0.x(),
            b.0.y(),
            b.1.x(),
            b.1.y(),
        ],
        4,
        || Ok(crate::topology::cmp_xy(a.0, b.0).then_with(|| crate::topology::cmp_xy(a.1, b.1))),
    )
}

/// The original physical-coordinate conversion shared by topology and metrics.
pub(crate) fn physical_point(
    point: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<LonLat, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.x(), point.y()],
        8,
        || LonLat::new(point.x().clone(), point.y().clone()),
    )
}

fn boundary_order(
    a: &[LonLat; 2],
    b: &[LonLat; 2],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[
            a[0].longitude(),
            a[0].latitude(),
            a[1].longitude(),
            a[1].latitude(),
            b[0].longitude(),
            b[0].latitude(),
            b[1].longitude(),
            b[1].latitude(),
        ],
        4,
        || Ok(compare(&a[0], &b[0]).then_with(|| compare(&a[1], &b[1]))),
    )
}

/// Join exact collinear touching fragments only after complete union selection.
/// Their closed image union is exactly the joined original-coordinate segment,
/// including branch junction points. The atlas's area labels are unaffected.
fn coalesce_boundary(
    edges: Vec<[LonLat; 2]>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<[LonLat; 2]>, GeoError> {
    let mut output: Vec<[LonLat; 2]> = Vec::with_capacity(edges.len());
    for edge in edges {
        let mut joined = false;
        for previous in &mut output {
            let mut admission = crate::numerical::ExactAdmission::new(context, progress);
            let same = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[
                    previous[1].longitude(),
                    previous[1].latitude(),
                    edge[0].longitude(),
                    edge[0].latitude(),
                ],
                4,
                || Ok(previous[1] == edge[0]),
            )?;
            if !same {
                continue;
            }
            let a = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[previous[0].longitude(), previous[0].latitude()],
                2,
                || {
                    Ok(Coord::xy(
                        previous[0].longitude().clone(),
                        previous[0].latitude().clone(),
                    ))
                },
            )?;
            let b = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[edge[0].longitude(), edge[0].latitude()],
                2,
                || {
                    Ok(Coord::xy(
                        edge[0].longitude().clone(),
                        edge[0].latitude().clone(),
                    ))
                },
            )?;
            let c = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[edge[1].longitude(), edge[1].latitude()],
                2,
                || {
                    Ok(Coord::xy(
                        edge[1].longitude().clone(),
                        edge[1].latitude().clone(),
                    ))
                },
            )?;
            // The common determinant body admits every actual subtraction and
            // product. Lexicographic original endpoints fix the joined order.
            if crate::topology::orientation_admitted(&a, &b, &c, &mut admission)? == 0 {
                previous[1] = admission.rational(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[edge[1].longitude(), edge[1].latitude()],
                    2,
                    || Ok(edge[1].clone()),
                )?;
                joined = true;
                break;
            }
        }
        if !joined {
            output.push(edge);
        }
    }
    Ok(output)
}

/// Complete written noding on the identified cut. Ordinary interior edge
/// crossings share the exact planar noder; only the endpoint aliases are added.
pub(crate) fn identified_events<'source>(
    edges: &[(&Coord, &Coord)],
    seeds: impl IntoIterator<Item = &'source Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Vec<Coord>, GeoError> {
    let mut original = Vec::new();
    for point in seeds {
        // Cover capacity growth of borrowed references before allocating it.
        context.admit_workspace(64)?;
        *retained = retained
            .checked_add(64)
            .ok_or(GeoError::ArithmeticOverflow("identified noding seeds"))?;
        context.charge_work(1)?;
        progress.context_poll(context)?;
        original.push(point);
    }
    let mut cut_points = Vec::new();
    for &point in &original {
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        if admission.rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[point.x()],
            2,
            || Ok(point.x() == &Rat::from_i64(-180) || point.x() == &Rat::from_i64(180)),
        )? {
            let other = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[point.x(), point.y()],
                2,
                || Ok(Coord::xy(point.x().neg(), point.y().clone())),
            )?;
            let bytes = coordinate_bytes(&other).saturating_mul(4);
            context.admit_workspace(bytes)?;
            *retained = retained
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow("identified cut noding"))?;
            cut_points.push(other);
        }
    }
    crate::topology::noding::events_admitted(
        edges,
        original.into_iter().chain(&cut_points),
        context,
        progress,
        retained,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, PreparedGeometry};

    #[test]
    fn complete_selected_support_and_areal_dimension_survive_boundary_routing() {
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let profile = GeoProfile::standard();
        for (text, native, areal, curves, points) in [
            ("POLYGON ((0 0,1 0,1 1,0 1,0 0))", false, true, 0, 0),
            (
                "POLYGON ((-10 -10,10 -10,10 10,-10 10,-10 -10),(-2 -2,2 -2,2 2,-2 2,-2 -2))",
                false,
                true,
                0,
                0,
            ),
            (
                "POLYGON ((-180 -90,180 -90,180 90,-180 90,-180 -90))",
                false,
                true,
                0,
                0,
            ),
            ("POLYGON ((0 0,1 0,0 1,0 0))", true, true, 0, 0),
            ("POLYGON ((0 0,1 0,2 0,0 0))", true, false, 2, 0),
            ("POLYGON ((1 1,1 1,1 1,1 1))", true, false, 0, 1),
        ] {
            let literal = crate::wkt::parse(text, &crs).unwrap();
            let geometry = PreparedGeometry::from_literal(&literal, &profile).unwrap();
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            let baseline = context.current_workspace_bytes();
            let selected = region_boundary(
                geometry.region(),
                &mut context,
                &mut WorkProgress::new(None),
            )
            .unwrap_or_else(|error| panic!("{text}: {error:?}"));
            assert_eq!(selected.native.is_some(), native, "{text}");
            assert_eq!(selected.has_areal_faces(), areal, "{text}");
            if let Some(graph) = &selected.native {
                assert_eq!(
                    graph
                        .fragments()
                        .iter()
                        .filter(|fragment| {
                            fragment.stratum()
                                == super::super::arcs::SelectedFragmentStratum::CurveInterior
                        })
                        .count(),
                    curves,
                    "{text}",
                );
                assert_eq!(graph.isolated_points().len(), points, "{text}");
            }
            let retained = selected.workspace_bytes;
            drop(selected);
            context.release_workspace(retained).unwrap();
            assert_eq!(context.current_workspace_bytes(), baseline, "{text}");
        }
    }

    #[test]
    fn boundary_absence_selects_whole_or_empty_only_after_regular_chart_proof() {
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal =
            crate::wkt::parse("POLYGON ((-180 -90,180 -90,180 90,-180 90,-180 -90))", &crs)
                .unwrap();
        let geometry = PreparedGeometry::from_literal(&literal, &GeoProfile::standard()).unwrap();
        for (region, expected) in [
            (PreparedRegion::Empty, false),
            (PreparedRegion::Whole, true),
            (geometry.region().clone(), true),
            (geometry.region().clone().complement(), false),
        ] {
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            let selected =
                region_boundary(&region, &mut context, &mut WorkProgress::new(None)).unwrap();
            assert_eq!(selected.edges, [] as [[LonLat; 2]; 0]);
            assert_eq!(selected.curves, [] as [crate::PreparedCurve; 0]);
            assert!(selected.native.is_none());
            assert_eq!(selected.has_areal_faces(), expected);
            let retained = selected.workspace_bytes;
            drop(selected);
            context.release_workspace(retained).unwrap();
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }

    #[test]
    fn boundary_inventory_refuses_before_allocation_and_preserves_original_cancellation() {
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("POLYGON ((0 0,1 0,1 1,0 1,0 0))", &crs).unwrap();
        let geometry = PreparedGeometry::from_literal(&literal, &GeoProfile::standard()).unwrap();
        for (work, memory, cancelled, error) in [
            (
                1,
                64 * 1024 * 1024,
                false,
                GeoError::WorkExhausted { limit: 1 },
            ),
            (262_144, 1, false, GeoError::MemoryExhausted { limit: 1 }),
            (262_144, 64 * 1024 * 1024, true, GeoError::Cancelled),
        ] {
            let mut limits = ExecutionLimits::GEOMETRY;
            limits.max_work_items = work;
            limits.max_workspace_bytes = memory;
            let mut context = MetricContext::new(
                crate::GeographicReference::wgs84(),
                ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            context.begin(1).unwrap();
            if cancelled {
                context.cancel();
            }
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = region_boundary(
                geometry.region(),
                &mut context,
                &mut WorkProgress::new(None),
            );
            let measured = window.close();
            assert_eq!(result.err(), Some(error));
            assert_eq!(measured.allocations, 0);
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }

    #[test]
    fn regular_hole_certificate_requires_strict_containment_and_closed_disjointness() {
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        for (holes, expected) in [
            ("(1 1,3 1,3 3,1 3,1 1)", true),
            ("(1 1,3 1,3 3,1 3,1 1),(4 1,6 1,6 3,4 3,4 1)", true),
            ("(0 1,3 1,3 3,0 3,0 1)", false),
            ("(1 1,3 1,3 3,1 3,1 1),(3 1,5 1,5 3,3 3,3 1)", false),
            ("(1 1,3 1,3 3,1 3,1 1),(3 3,5 3,5 5,3 5,3 3)", false),
            ("(1 1,3 1,3 3,1 3,1 1),(2 2,4 2,4 4,2 4,2 2)", false),
            ("(9 1,11 1,11 3,9 3,9 1)", false),
            ("(1 1,3 1,1 3,1 1)", false),
            ("(1 1,3 1,1 1,1 1)", false),
        ] {
            let text = format!("POLYGON ((0 0,10 0,10 10,0 10,0 0),{holes})");
            let literal = crate::wkt::parse(&text, &crs).unwrap();
            let GeometryBody::Polygon(rings) = literal.geometry().body() else {
                panic!("polygon fixture");
            };
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            assert_eq!(
                regular_rectangle_polygon(rings, &mut context, &mut WorkProgress::new(None))
                    .unwrap(),
                expected,
                "{holes}",
            );
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }
}
