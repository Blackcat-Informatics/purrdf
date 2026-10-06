// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact source-coordinate longitude atlas with identified seams and poles.
//!
//! Source-linear polygon membership uses the one planar predicate home inside
//! charts. At identified chart boundaries, exact local sectors classify the
//! physical neighbourhood, removing internal cut edges without sampling guesses.

pub mod arcs;
pub(crate) mod area;
pub(crate) mod arrangement;
pub(crate) mod boundary;
mod chart_index;
pub(crate) use chart_index::is_rectangle_cycle;
pub(crate) mod enclosure;
pub(crate) mod native;
pub mod oriented;
pub mod point;
mod relate;
#[cfg(test)]
pub(crate) use relate::tests::closed_intersection as test_closed_intersection;
pub(crate) mod union;
pub use relate::{
    dimension, relate, relate_metered, relate_prepared, relate_prepared_metered,
    relation_source_admission, topology_law_id,
};

use crate::context::WorkProgress;
use crate::prepared::{PreparedPolygon, PreparedRegion, RegionInterior};
use crate::{
    Coord, GeoError, GeometryBody, LonLat, MetricContext, MetricWorkObserver, PreparedCurve, Rat,
    Set,
};
use core::cmp::Ordering;

/// Classify exact physical membership in an explicitly selected region.
///
/// This identifies both longitude-cut spellings and all longitudes at exact
/// poles. Z/M metadata is retained by preparation and has no invented metric role.
///
/// # Errors
/// Refuses cancellation, arithmetic overflow and incomplete work/workspace admission.
pub fn locate(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
) -> Result<Set, GeoError> {
    locate_impl(point, region, context, None)
}
/// Classify a region with bounded governor/cancellation charging.
/// # Errors
/// Adds observer refusal to the exact atlas contract.
pub fn locate_metered(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Set, GeoError> {
    locate_impl(point, region, context, Some(observer))
}
fn locate_impl(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<Set, GeoError> {
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    context.begin(1)?;
    check_region_reference(region, context, &mut progress)?;
    let coordinates = vertex_count(region);
    let mut bits = point
        .longitude()
        .numerator()
        .bit_len()
        .saturating_add(point.longitude().denominator().bit_len())
        .saturating_add(point.latitude().numerator().bit_len())
        .saturating_add(point.latitude().denominator().bit_len());
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        region
    {
        for polygon in polygons.iter() {
            charge(context, &mut progress, 1)?;
            bits = bits.saturating_add(polygon.coordinate_bits());
        }
    }
    let bytes = coordinates
        .checked_mul(2048)
        .and_then(|value| value.checked_add(bits.div_ceil(8).saturating_mul(32)))
        .and_then(|value| value.checked_add(65_536))
        .ok_or_else(|| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    context.admit_workspace(bytes)?;
    let result = locate_validated_with_progress(point, region, context, &mut progress);
    context.release_workspace(bytes)?;
    if result.is_ok() {
        progress.context_poll(context)?;
    }
    result
}

pub(crate) fn locate_inner(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
) -> Result<Set, GeoError> {
    locate_with_progress(point, region, context, &mut WorkProgress::new(None))
}
pub(crate) fn locate_inner_metered(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Set, GeoError> {
    let mut progress = WorkProgress::new(Some(observer));
    progress.initial()?;
    let result = locate_with_progress(point, region, context, &mut progress);
    if result.is_ok() {
        progress.context_poll(context)?;
    }
    result
}
pub(crate) fn locate_with_progress(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    check_region_reference(region, context, progress)?;
    locate_validated_with_progress(point, region, context, progress)
}

/// Classify an immutable region whose complete reference inventory has already
/// been checked against this context in the current admitted operation.
///
/// Callers retain that validated region and context for the entire operation;
/// this shares the checked entry's membership body without walking the same
/// reference inventory again for every arrangement face.
pub(crate) fn locate_validated_with_progress(
    point: &LonLat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    locate_with_chart_index(point, region, None, context, progress)
}

fn locate_with_chart_index(
    point: &LonLat,
    region: &PreparedRegion,
    index: Option<&chart_index::ChartIndex<'_>>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    match region {
        PreparedRegion::Empty => Ok(Set::Exterior),
        PreparedRegion::Whole => Ok(Set::Interior),
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            let complementary = matches!(region, PreparedRegion::ComplementOfPolygons(_));
            // An index is built only after its complete immutable inventory
            // has proved every chart ordinary; reuse that preparation witness.
            if index.is_none() && polygons.iter().any(|polygon| polygon.chart().is_none()) {
                return oriented_union_location(
                    point,
                    region,
                    polygons,
                    complementary,
                    context,
                    progress,
                );
            }
            if point.is_pole() {
                return pole(point, polygons, complementary, context, progress);
            }
            let chart = chart_point(point, context, progress)?;
            let raw = if let Some(index) = index {
                index.locate(&chart, complementary, context, progress)?
            } else {
                union_location(&chart, polygons, complementary, context, progress)?
            };
            if point.longitude().abs() != Rat::from_i64(180) && raw != Set::Boundary {
                return Ok(raw);
            }
            sectors(&chart, polygons, complementary, context, progress)
        }
    }
}

/// Prove one uniform physical membership for a complete validated angular box.
/// Coordinates are inclusive unrounded degree enclosures. `None` means a
/// possible boundary contact or unresolved uniform class and requires refinement.
pub(crate) fn locate_enclosure(
    longitude: &purrdf_xsd::math::FixedInterval,
    latitude: &purrdf_xsd::math::FixedInterval,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    check_region_reference(region, context, progress)?;
    match region {
        PreparedRegion::Empty => return Ok(Some(Set::Exterior)),
        PreparedRegion::Whole => return Ok(Some(Set::Interior)),
        PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => {}
    }
    let scratch = longitude
        .workspace_bytes()
        .saturating_add(latitude.workspace_bytes())
        .saturating_mul(32)
        .saturating_add(65_536) as u64;
    context.admit_workspace(scratch)?;
    let result = (|| {
        let (west, east) = crate::numerical::exact_bounds(longitude);
        let (south, north) = crate::numerical::exact_bounds(latitude);
        // The caller supplies a certified physical geographic image. Outward
        // arithmetic latitude guards can be intersected with that known domain.
        let south = south.max(Rat::from_i64(-90));
        let north = north.min(Rat::from_i64(90));
        if west > east || south > north {
            return Err(GeoError::domain("empty validated geographic enclosure"));
        }
        let intervals = longitude_intervals(&west, &east, context, progress)?;
        let mut uniform = None;
        for (west, east) in intervals {
            let status = locate_rectangle(&west, &east, &south, &north, region, context, progress)?;
            let Some(status) = status else {
                return Ok(None);
            };
            if uniform.is_some_and(|prior| prior != status) {
                return Ok(None);
            }
            uniform = Some(status);
        }
        Ok(uniform)
    })();
    context.release_workspace(scratch)?;
    if result.is_ok() {
        progress.context_poll(context)?;
    }
    result
}

/// Prove a complete angular enclosure's uniform Left-side class against a
/// validated native Jordan ring. The caller retains the ring's preparation
/// witness; this borrows the exact curve without synthesizing a carrier polygon.
pub(crate) fn locate_ring_enclosure(
    longitude: &purrdf_xsd::math::FixedInterval,
    latitude: &purrdf_xsd::math::FixedInterval,
    curve: &PreparedCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    for edge in curve.edges() {
        charge(context, progress, 1)?;
        if edge.binding_id() != binding {
            return Err(crate::numerical::missing_operation_ids(
                edge.binding_id(),
                binding,
                context,
                progress,
            )?);
        }
    }
    let scratch = longitude
        .workspace_bytes()
        .saturating_add(latitude.workspace_bytes())
        .saturating_mul(32)
        .saturating_add(65_536) as u64;
    context.admit_workspace(scratch)?;
    let result = (|| {
        let (west, east) = crate::numerical::exact_bounds(longitude);
        let (south, north) = crate::numerical::exact_bounds(latitude);
        let south = south.max(Rat::from_i64(-90));
        let north = north.min(Rat::from_i64(90));
        if west > east || south > north {
            return Err(GeoError::domain("empty validated geographic enclosure"));
        }
        let mut uniform = None;
        for (west, east) in longitude_intervals(&west, &east, context, progress)? {
            let center = rectangle_center([&west, &east, &south, &north], context, progress)?;
            let point = south == north && (west == east || south.abs() == Rat::from_i64(90));
            if !point
                && enclosure::ring_intersects_rectangle(
                    curve,
                    [&west, &east, &south, &north],
                    context,
                    progress,
                )?
            {
                return Ok(None);
            }
            let status = oriented::locate_validated(curve, &center, context, progress)?;
            if status == Set::Boundary && !point {
                return Ok(None);
            }
            if uniform.is_some_and(|prior| prior != status) {
                return Ok(None);
            }
            uniform = Some(status);
        }
        Ok(uniform)
    })();
    context.release_workspace(scratch)?;
    if result.is_ok() {
        progress.context_poll(context)?;
    }
    result
}

pub(super) fn longitude_intervals(
    west: &Rat,
    east: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<(Rat, Rat)>, GeoError> {
    use purrdf_xsd::integer::ExactOperation::{RationalAdd, RationalCompare};
    let unchanged = crate::numerical::ExactAdmission::new(context, progress).rational(
        RationalCompare,
        &[west, east],
        4,
        || {
            Ok(west >= &Rat::from_i64(-180)
                && west < &Rat::from_i64(180)
                && east <= &Rat::from_i64(180)
                && west <= east)
        },
    )?;
    if unchanged {
        return Ok(vec![(
            crate::numerical::copy_rat(west, context, progress)?,
            crate::numerical::copy_rat(east, context, progress)?,
        )]);
    }
    let width = {
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        crate::numerical::exact_rational(Some(&mut admission), RationalAdd, &[west, east], || {
            east.sub(west)
        })?
    };
    let full = crate::numerical::ExactAdmission::new(context, progress).rational(
        RationalCompare,
        &[&width],
        1,
        || Ok(width >= Rat::from_i64(360)),
    )?;
    if full {
        return Ok(vec![(Rat::from_i64(-180), Rat::from_i64(180))]);
    }
    let reduced = point::normalize_admitted(west, context, progress)?;
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    let reduced = if admission.rational(RationalCompare, &[&reduced], 1, || {
        Ok(reduced >= Rat::from_i64(180))
    })? {
        crate::numerical::exact_rational(Some(&mut admission), RationalAdd, &[&reduced], || {
            reduced.sub(&Rat::from_i64(360))
        })?
    } else {
        reduced
    };
    let upper = crate::numerical::exact_rational(
        Some(&mut admission),
        RationalAdd,
        &[&reduced, &width],
        || reduced.add(&width),
    )?;
    if admission.rational(RationalCompare, &[&upper], 1, || {
        Ok(upper <= Rat::from_i64(180))
    })? {
        return Ok(vec![(reduced, upper)]);
    }
    let wrapped =
        crate::numerical::exact_rational(Some(&mut admission), RationalAdd, &[&upper], || {
            upper.sub(&Rat::from_i64(360))
        })?;
    Ok(vec![
        (reduced, Rat::from_i64(180)),
        (Rat::from_i64(-180), wrapped),
    ])
}

fn rectangle_center(
    rectangle: [&Rat; 4],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<LonLat, GeoError> {
    let [west, east, south, north] = rectangle;
    let two = Rat::from_i64(2);
    let center = {
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        let longitude = crate::numerical::exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[west, east],
            || west.add(east),
        )?;
        let latitude = crate::numerical::exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[south, north],
            || south.add(north),
        )?;
        let longitude = crate::numerical::exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&longitude, &two],
            || longitude.div(&two).expect("positive divisor"),
        )?;
        let latitude = crate::numerical::exact_rational(
            Some(&mut admission),
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&latitude, &two],
            || latitude.div(&two).expect("positive divisor"),
        )?;
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[&longitude, &latitude],
            4,
        )
        .ok_or(GeoError::ArithmeticOverflow("geographic center admission"))?;
        progress.exact(context, cost, || LonLat::new(longitude, latitude))?
    };
    Ok(center)
}
fn locate_rectangle(
    west: &Rat,
    east: &Rat,
    south: &Rat,
    north: &Rat,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    let center = rectangle_center([west, east, south, north], context, progress)?;
    if south == north && (west == east || south.abs() == Rat::from_i64(90)) {
        return locate_with_progress(&center, region, context, progress).map(Some);
    }
    let curved = match region {
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons.iter().any(|polygon| polygon.chart().is_none())
        }
        PreparedRegion::Empty | PreparedRegion::Whole => false,
    };
    if curved {
        if enclosure::boundary_intersects_rectangle(
            region, west, east, south, north, context, progress,
        )? {
            return Ok(None);
        }
        return locate_with_progress(&center, region, context, progress).map(Some);
    }
    let center_status = locate_with_progress(&center, region, context, progress)?;
    // An exact physical boundary point inside a nontrivial rectangle already
    // disproves a uniform interior/exterior class. This witness does not
    // require noding unrelated source edges into a complete arrangement.
    if center_status == Set::Boundary {
        return Ok(None);
    }
    // Original rings contain every physical boundary. Excluding this complete
    // superset proves a uniform rectangle without constructing its union
    // arrangement. Possible chart cuts/internal pieces retain the tight path.
    if !enclosure::boundary_intersects_rectangle(
        region, west, east, south, north, context, progress,
    )? {
        return Ok(Some(center_status));
    }
    let boundary = boundary::region_boundary(region, context, progress)?;
    let separated = enclosure::selected_boundary_intersects_rectangle(
        &boundary,
        [west, east, south, north],
        context,
        progress,
    )
    .map(|intersects| !intersects);
    let bytes = boundary.workspace_bytes;
    drop(boundary);
    context.release_workspace(bytes)?;
    if !separated? {
        return Ok(None);
    }
    Ok(Some(center_status))
}

#[cfg(test)]
pub(crate) fn segment_rectangle(
    a: &Coord,
    b: &Coord,
    west: &Rat,
    east: &Rat,
    south: &Rat,
    north: &Rat,
) -> bool {
    segment_rectangle_inner(a, b, [west, east, south, north], None)
        .expect("pure exact rectangle has no admission refusal")
}
pub(crate) fn segment_rectangle_admitted(
    a: &Coord,
    b: &Coord,
    rectangle: [&Rat; 4],
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<bool, GeoError> {
    segment_rectangle_inner(a, b, rectangle, Some(admission))
}
fn segment_rectangle_inner(
    a: &Coord,
    b: &Coord,
    rectangle: [&Rat; 4],
    mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
) -> Result<bool, GeoError> {
    let [west, east, south, north] = rectangle;
    let decisive = segment_rectangle_bounds(a, b, rectangle, admission.as_deref_mut())?;
    if let Some(decisive) = decisive {
        return Ok(decisive);
    }
    let construct = || {
        [
            Coord::xy(west.clone(), south.clone()),
            Coord::xy(east.clone(), south.clone()),
            Coord::xy(east.clone(), north.clone()),
            Coord::xy(west.clone(), north.clone()),
        ]
    };
    let corners = if let Some(admission) = admission.as_deref_mut() {
        admission.rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &rectangle,
            8,
            || Ok(construct()),
        )?
    } else {
        construct()
    };
    for index in 0..4 {
        let contact = if let Some(admission) = admission.as_deref_mut() {
            crate::topology::intersect_admitted(
                a,
                b,
                &corners[index],
                &corners[(index + 1) % 4],
                admission,
            )?
        } else {
            crate::topology::intersect(a, b, &corners[index], &corners[(index + 1) % 4])
        };
        if !matches!(contact, crate::SegmentIntersection::None) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The exact endpoint/bounding-box phase of the one closed-rectangle predicate.
/// Axis-aligned segments need no rational crossing construction after overlap.
fn segment_rectangle_bounds(
    a: &Coord,
    b: &Coord,
    rectangle: [&Rat; 4],
    mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
) -> Result<Option<bool>, GeoError> {
    let [west, east, south, north] = rectangle;
    let mut compare = |a: &Rat, b: &Rat| {
        crate::numerical::exact_rational(
            admission.as_deref_mut(),
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[a, b],
            || a.cmp(b),
        )
    };
    let mut contained = |point: &Coord| -> Result<bool, GeoError> {
        Ok(!compare(point.x(), west)?.is_lt()
            && !compare(point.x(), east)?.is_gt()
            && !compare(point.y(), south)?.is_lt()
            && !compare(point.y(), north)?.is_gt())
    };
    if contained(a)? || contained(b)? {
        return Ok(Some(true));
    }
    let x_order = compare(a.x(), b.x())?;
    let (x_lower, x_upper) = if x_order.is_gt() {
        (b.x(), a.x())
    } else {
        (a.x(), b.x())
    };
    if compare(x_upper, west)?.is_lt() || compare(x_lower, east)?.is_gt() {
        return Ok(Some(false));
    }
    let y_order = compare(a.y(), b.y())?;
    let (y_lower, y_upper) = if y_order.is_gt() {
        (b.y(), a.y())
    } else {
        (a.y(), b.y())
    };
    if compare(y_upper, south)?.is_lt() || compare(y_lower, north)?.is_gt() {
        return Ok(Some(false));
    }
    if x_order.is_eq() || y_order.is_eq() {
        return Ok(Some(true));
    }
    Ok(None)
}

fn charge(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    count: u64,
) -> Result<(), GeoError> {
    let before = context.work_items();
    context.charge_work(count)?;
    if before / 64 != context.work_items() / 64 {
        progress.context_poll(context)?;
    }
    Ok(())
}

fn check_region_reference(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    region.check_binding_admitted(binding, context, progress)
}

fn vertex_count(region: &PreparedRegion) -> u64 {
    match region {
        PreparedRegion::Empty | PreparedRegion::Whole => 0,
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons
                .iter()
                .map(PreparedPolygon::coordinate_count)
                .fold(0, u64::saturating_add)
        }
    }
}
fn union_location(
    point: &Coord,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let mut polygons = polygons.iter();
    union_location_with(point, complementary, context, progress, |_, _| {
        Ok(polygons.next().map(|polygon| ChartCandidate {
            polygon,
            base_location: None,
        }))
    })
}

struct ChartCandidate<'a> {
    polygon: &'a PreparedPolygon,
    base_location: Option<Set>,
}

/// One original union body; the broad phase supplies certified base locations
/// or candidates for the same exact surface predicate.
fn union_location_with<'a>(
    point: &Coord,
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    mut next: impl FnMut(
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<Option<ChartCandidate<'a>>, GeoError>,
) -> Result<Set, GeoError> {
    let mut boundary = false;
    while let Some(ChartCandidate {
        polygon,
        base_location,
    }) = next(context, progress)?
    {
        let GeometryBody::Polygon(rings) = polygon.chart().expect("written polygon chart").body()
        else {
            unreachable!("prepared polygon chart")
        };
        let mut located = if let Some(location) = base_location {
            location
        } else {
            crate::topology::locate_surface_admitted(
                point,
                rings,
                &mut crate::numerical::ExactAdmission::new(context, progress),
            )?
        };
        if polygon.interior() == RegionInterior::Complement {
            located = complement(located);
        }
        if located == Set::Interior {
            return Ok(if complementary {
                Set::Exterior
            } else {
                Set::Interior
            });
        }
        boundary |= located == Set::Boundary;
    }
    Ok(if boundary {
        Set::Boundary
    } else if complementary {
        Set::Interior
    } else {
        Set::Exterior
    })
}
/// Evaluate the base region from complete, non-boundary per-ring classes.
/// Native bases intersect the rings' Left sides. Written carrier bases select
/// the exterior ring and remove the hole interiors. Counts are exact contracts.
pub(crate) fn polygon_base_membership(
    polygon: &PreparedPolygon,
    ring_interiors: &[bool],
) -> Result<bool, GeoError> {
    if ring_interiors.len() != polygon.rings().len() || ring_interiors.is_empty() {
        return Err(GeoError::domain("polygon membership ring count mismatch"));
    }
    if polygon.closed_support() {
        return Ok(false);
    }
    Ok(if polygon.oriented_interior().is_some() {
        ring_interiors.iter().all(|inside| *inside)
    } else {
        ring_interiors[0] && !ring_interiors[1..].iter().any(|inside| *inside)
    })
}
fn polygon_selected_membership(polygon: &PreparedPolygon, base: bool) -> bool {
    base ^ ((polygon.oriented_interior() == Some(crate::OrientedInterior::Right))
        != (polygon.interior() == RegionInterior::Complement))
}
/// Evaluate the one union/complement law from complete polygon-base classes.
/// Each base has already been established by the relevant ring or written-chart
/// law. Boundary points require the sector arrangement and cannot be booleans.
pub(crate) fn region_membership(
    region: &PreparedRegion,
    polygon_bases: &[bool],
) -> Result<bool, GeoError> {
    let (polygons, complementary) = match region {
        PreparedRegion::Empty | PreparedRegion::Whole => {
            if !polygon_bases.is_empty() {
                return Err(GeoError::domain(
                    "constant region membership count mismatch",
                ));
            }
            return Ok(matches!(region, PreparedRegion::Whole));
        }
        PreparedRegion::Polygons(polygons) => (polygons, false),
        PreparedRegion::ComplementOfPolygons(polygons) => (polygons, true),
    };
    if polygon_bases.len() != polygons.len() {
        return Err(GeoError::domain("region membership polygon count mismatch"));
    }
    Ok(polygons
        .iter()
        .zip(polygon_bases)
        .any(|(polygon, base)| polygon_selected_membership(polygon, *base))
        ^ complementary)
}
pub(crate) fn polygon_selected_location(polygon: &PreparedPolygon, base: Set) -> Set {
    match base {
        Set::Boundary => Set::Boundary,
        Set::Interior | Set::Exterior => {
            if polygon_selected_membership(polygon, base == Set::Interior) {
                Set::Interior
            } else {
                Set::Exterior
            }
        }
    }
}

/// Closed intersection of complete native ring classes. Boundary-only strata
/// remain part of the intersection even when neither adjacent open face is
/// Interior. Union sector labels still determine the final union topology.
pub(crate) fn ring_intersection_location(
    locations: impl IntoIterator<Item = Result<Set, GeoError>>,
) -> Result<Set, GeoError> {
    let mut location = Set::Interior;
    for next in locations {
        match next? {
            Set::Exterior => return Ok(Set::Exterior),
            Set::Boundary => location = Set::Boundary,
            Set::Interior => {}
        }
    }
    Ok(location)
}
fn oriented_union_location(
    point: &LonLat,
    region: &PreparedRegion,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let (raw, boundaries) =
        oriented_union_status(point, polygons, complementary, context, progress)?;
    if boundaries <= 1 || only_closed_support(polygons, context, progress)? {
        return Ok(raw);
    }
    if let Some(selected) =
        rectangle_union_status(point, polygons, complementary, context, progress)?
    {
        return Ok(selected);
    }
    let boundary = arcs::arrangement::native_boundary_in(region, context, progress)?;
    let retained = boundary.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    let result = selected_union_status(point, &boundary, complementary, context, progress);
    drop(boundary);
    context.release_workspace(retained)?;
    result
}

/// Reuse a complete original native arrangement for union membership, including
/// exact pole/seam nodes and boundaries removed from the actual union.
pub(crate) fn locate_selected_boundary(
    point: &LonLat,
    boundary: &arcs::arrangement::NativeBoundaryArrangement,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        boundary.region()
    else {
        return locate_with_progress(point, boundary.region(), context, progress);
    };
    let complementary = matches!(boundary.region(), PreparedRegion::ComplementOfPolygons(_));
    let (raw, boundaries) =
        oriented_union_status(point, polygons, complementary, context, progress)?;
    if boundaries <= 1 || only_closed_support(polygons, context, progress)? {
        return Ok(raw);
    }
    if let Some(selected) =
        rectangle_union_status(point, polygons, complementary, context, progress)?
    {
        return Ok(selected);
    }
    selected_union_status(point, boundary, complementary, context, progress)
}

/// A union of complete closed curve/point supports has no open areal face,
/// so several original contacts cannot form an internal areal wall. This is
/// a complete admitted inventory witness, not a sampled dimension estimate.
fn only_closed_support(
    polygons: &[PreparedPolygon],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    for polygon in polygons {
        charge(context, progress, 1)?;
        if !polygon.closed_support() {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Complete local open-face labels supplied only by certified separable
/// rectangles and proved lower-dimensional support. Unknown contributors keep
/// the original arrangement path. A known original closed contact survives
/// an empty positive open-face union; its closed complement is the whole local
/// neighbourhood.
fn rectangle_union_status(
    point: &LonLat,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    let mut sectors = 0;
    for polygon in polygons {
        charge(context, progress, 1)?;
        let Some(selected) = certified_polygon_sectors(point, polygon, context, progress)? else {
            return Ok(None);
        };
        sectors |= selected;
    }
    Ok(Some(match (sectors, complementary) {
        (15, false) | (0, true) => Set::Interior,
        (15, true) => Set::Exterior,
        _ => Set::Boundary,
    }))
}

/// Four angular quadrant labels around the original point. Strict inner
/// inclusion or exact agreement of an inner/outer wall proves each axis side;
/// interval overlap never substitutes for equality with the true wall.
fn certified_polygon_sectors(
    point: &LonLat,
    polygon: &PreparedPolygon,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<u8>, GeoError> {
    let selected = if polygon.closed_support() {
        0
    } else {
        let Some((outer, inner)) = polygon.certified_angular_bounds() else {
            return Ok(None);
        };
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        let point_keys = [
            admission.order_key(point.longitude())?,
            admission.order_key(point.latitude())?,
        ];
        let mut classify = |[west, south, east, north]: &[Rat; 4]| {
            crate::measure::BorrowedBounds {
                west,
                south,
                east,
                north,
            }
            .ordered(&mut admission)?
            .locate_ordinates_admitted(
                point.longitude(),
                point.latitude(),
                point_keys,
                &mut admission,
            )
        };
        if classify(outer)? == Set::Exterior {
            0
        } else {
            let Some(inner) = inner else {
                return Ok(None);
            };
            if classify(inner)? == Set::Interior {
                15
            } else {
                let Some(x) = rectangle_axis_sides(
                    point.longitude(),
                    [&outer[0], &outer[2], &inner[0], &inner[2]],
                    &mut admission,
                )?
                else {
                    return Ok(None);
                };
                let Some(y) = rectangle_axis_sides(
                    point.latitude(),
                    [&outer[1], &outer[3], &inner[1], &inner[3]],
                    &mut admission,
                )?
                else {
                    return Ok(None);
                };
                let mut selected = 0;
                for horizontal in 0..2 {
                    for vertical in 0..2 {
                        if x & (1 << horizontal) != 0 && y & (1 << vertical) != 0 {
                            selected |= 1 << (2 * horizontal + vertical);
                        }
                    }
                }
                selected
            }
        }
    };
    Ok(Some(if polygon.interior() == RegionInterior::Complement {
        selected ^ 15
    } else {
        selected
    }))
}

/// Whether a validated geographic enclosure lies strictly outside the
/// certified outer walls of an operation-cell polygon's oriented base set.
/// The walls contain the closed base set and lie strictly inside the
/// canonical nonpolar chart, so a disjoint enclosure on that same chart proves
/// the base Exterior. Any enclosure touching the chart seam, a wall, or a
/// polygon without walls is left to the original ring predicate.
pub(crate) fn enclosure_outside_certified_walls(
    polygon: &PreparedPolygon,
    longitude: &purrdf_xsd::math::FixedInterval,
    latitude: &purrdf_xsd::math::FixedInterval,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if polygon.closed_support() {
        return Ok(false);
    }
    let Some(([west_wall, south_wall, east_wall, north_wall], _)) =
        polygon.certified_angular_bounds()
    else {
        return Ok(false);
    };
    charge(context, progress, 1)?;
    let scratch = longitude
        .workspace_bytes()
        .saturating_add(latitude.workspace_bytes())
        .saturating_mul(4)
        .saturating_add(1024) as u64;
    context.admit_workspace(scratch)?;
    let result = (|| {
        let (west, east) = crate::numerical::exact_bounds(longitude);
        let (south, north) = crate::numerical::exact_bounds(latitude);
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        let seam = Rat::from_i64(180);
        if admission.compare(&west, &seam.neg())?.is_le()
            || admission.compare(&east, &seam)?.is_ge()
        {
            return Ok(false);
        }
        Ok(admission.compare(&east, west_wall)?.is_lt()
            || admission.compare(&west, east_wall)?.is_gt()
            || admission.compare(&north, south_wall)?.is_lt()
            || admission.compare(&south, north_wall)?.is_gt())
    })();
    context.release_workspace(scratch)?;
    result
}

fn rectangle_axis_sides(
    point: &Rat,
    [outer_min, outer_max, inner_min, inner_max]: [&Rat; 4],
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<Option<u8>, GeoError> {
    let lower = admission.compare(point, inner_min)?;
    let upper = admission.compare(point, inner_max)?;
    Ok(if lower.is_gt() && upper.is_lt() {
        Some(3)
    } else if lower.is_eq() && admission.compare(point, outer_min)?.is_eq() {
        Some(1)
    } else if upper.is_eq() && admission.compare(point, outer_max)?.is_eq() {
        Some(2)
    } else {
        None
    })
}

fn selected_union_status(
    point: &LonLat,
    boundary: &arcs::arrangement::NativeBoundaryArrangement,
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    if arcs::arrangement::selected_boundary_contact(point, boundary, context, progress)? {
        Ok(Set::Boundary)
    } else {
        // The query already belongs to at least two original closed polygon
        // boundaries. Complete selected-boundary exclusion therefore removes
        // an internal wall: the regular closed union contains a neighbourhood.
        Ok(if complementary {
            Set::Exterior
        } else {
            Set::Interior
        })
    }
}

fn oriented_union_status(
    point: &LonLat,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Set, u64), GeoError> {
    raw_union_location(
        polygons.iter().map(|polygon| {
            // A closed complement removes only the union's open areal side.
            // Proved curve/point support contributes no such side, even at an
            // exact contact. A polygon's own complementary mode is whole.
            if complementary
                && polygon.closed_support()
                && polygon.interior() == RegionInterior::Written
            {
                Ok(Set::Exterior)
            } else {
                polygon_location(point, polygon, context, progress)
            }
        }),
        complementary,
    )
}

/// Complete selected polygon classes before identifying internal union walls.
/// Multiple Boundary classes require the caller's fully noded sector proof.
pub(crate) fn raw_union_location(
    locations: impl IntoIterator<Item = Result<Set, GeoError>>,
    complementary: bool,
) -> Result<(Set, u64), GeoError> {
    let mut boundaries = 0_u64;
    for located in locations {
        match located? {
            Set::Interior => {
                return Ok((
                    if complementary {
                        Set::Exterior
                    } else {
                        Set::Interior
                    },
                    0,
                ));
            }
            Set::Boundary => {
                boundaries = boundaries
                    .checked_add(1)
                    .ok_or(GeoError::ArithmeticOverflow("union boundary classes"))?;
            }
            Set::Exterior => {}
        }
    }
    Ok((
        if boundaries >= 1 {
            Set::Boundary
        } else if complementary {
            Set::Interior
        } else {
            Set::Exterior
        },
        boundaries,
    ))
}
/// Original single-polygon location without manufacturing a cloned region.
/// The caller has already checked this immutable polygon's actual reference.
pub(crate) fn polygon_location(
    point: &LonLat,
    polygon: &PreparedPolygon,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    if !polygon.closed_support()
        && let Some(sectors) = certified_polygon_sectors(point, polygon, context, progress)?
    {
        return Ok(match sectors {
            0 => Set::Exterior,
            15 => Set::Interior,
            _ => Set::Boundary,
        });
    }
    if polygon.closed_support() {
        // The closed complement of a proved lower-dimensional image is the
        // whole surface: its base has no open face to remove.
        if polygon.interior() == RegionInterior::Complement {
            return Ok(Set::Interior);
        }
        for edge in polygon.rings().iter().flat_map(PreparedCurve::edges) {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if point::contact_with_progress(edge, point, context, progress)? {
                return Ok(Set::Boundary);
            }
        }
        return Ok(Set::Exterior);
    }
    if polygon.oriented_interior().is_some() {
        let location = ring_intersection_location(
            polygon
                .rings()
                .iter()
                .map(|ring| oriented::locate_validated(ring, point, context, progress)),
        )?;
        return Ok(polygon_selected_location(polygon, location));
    }
    let single = core::slice::from_ref(polygon);
    if point.is_pole() {
        return pole(point, single, false, context, progress);
    }
    let chart = chart_point(point, context, progress)?;
    let raw = union_location(&chart, single, false, context, progress)?;
    let seam =
        crate::numerical::ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[point.longitude()],
            1,
            || {
                Ok(point.longitude() == &Rat::from_i64(-180)
                    || point.longitude() == &Rat::from_i64(180))
            },
        )?;
    if seam || raw == Set::Boundary {
        sectors(&chart, single, false, context, progress)
    } else {
        Ok(raw)
    }
}

const fn complement(set: Set) -> Set {
    match set {
        Set::Interior => Set::Exterior,
        Set::Exterior => Set::Interior,
        Set::Boundary => Set::Boundary,
    }
}
fn edges(polygons: &[PreparedPolygon]) -> impl Iterator<Item = (&Coord, &Coord)> {
    polygons
        .iter()
        .filter_map(PreparedPolygon::chart)
        .flat_map(|chart| match chart.body() {
            GeometryBody::Polygon(rings) => rings.iter(),
            _ => unreachable!("prepared chart is a polygon"),
        })
        .flat_map(|ring| ring.windows(2).map(|pair| (&pair[0], &pair[1])))
}
fn chart_point(
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Coord, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[point.longitude(), point.latitude()],
        2,
        || {
            Ok(Coord::xy(
                point.longitude().clone(),
                point.latitude().clone(),
            ))
        },
    )
}
fn vector(
    from: &Coord,
    to: &Coord,
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<[Rat; 2], GeoError> {
    let x = crate::numerical::exact_rational(
        Some(admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[to.x(), from.x()],
        || to.x().sub(from.x()),
    )?;
    let y = crate::numerical::exact_rational(
        Some(admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[to.y(), from.y()],
        || to.y().sub(from.y()),
    )?;
    Ok([x, y])
}
fn cross(
    a: &[Rat; 2],
    b: &[Rat; 2],
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<Rat, GeoError> {
    crate::topology::determinant2_admitted(&a[0], &a[1], &b[0], &b[1], admission)
}
fn translated(
    point: &Coord,
    longitude: i64,
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<Coord, GeoError> {
    let offset = Rat::from_i64(longitude);
    let x = crate::numerical::exact_rational(
        Some(admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[point.x(), &offset],
        || point.x().add(&offset),
    )?;
    let y = admission.rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[point.y()],
        1,
        || Ok(point.y().clone()),
    )?;
    Ok(Coord::xy(x, y))
}
fn upper_half(direction: &[Rat; 2]) -> bool {
    direction[1] > Rat::zero() || (direction[1] == Rat::zero() && direction[0] >= Rat::zero())
}
fn direction_order(
    a: &[Rat; 2],
    b: &[Rat; 2],
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<Ordering, GeoError> {
    let half = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[&a[0], &a[1], &b[0], &b[1]],
        6,
        || Ok(upper_half(b).cmp(&upper_half(a))),
    )?;
    if half.is_eq() {
        Ok(cross(a, b, admission)?.signum().cmp(&0).reverse())
    } else {
        Ok(half)
    }
}

/// Classify the two transverse sides of a caller-proved complete open noded
/// written fragment. Exact formal directions reuse the physical union law.
/// The caller has validated the region binding and excluded every non-collinear
/// contact from the fragment's interior, including longitude-cut aliases.
pub(crate) fn written_boundary_sides(
    point: &Coord,
    from: &Coord,
    to: &Coord,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.y()],
        2,
        || Ok(point.y() == &Rat::from_i64(90) || point.y() == &Rat::from_i64(-90)),
    )? {
        return Ok(false);
    }
    let [left, right] = written_boundary_locations(point, from, to, region, context, progress)?;
    Ok(left != right)
}

/// Complete formal transverse face labels of an open, fully noded written
/// fragment. This is the same classifier used to select physical boundaries.
pub(crate) fn written_boundary_locations(
    point: &Coord,
    from: &Coord,
    to: &Coord,
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[Set; 2], GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        region
    else {
        let location = if matches!(region, PreparedRegion::Whole) {
            Set::Interior
        } else {
            Set::Exterior
        };
        return Ok([location; 2]);
    };
    written_boundary_locations_in(
        point,
        from,
        to,
        polygons,
        matches!(region, PreparedRegion::ComplementOfPolygons(_)),
        context,
        progress,
    )
}

pub(crate) fn written_boundary_locations_in(
    point: &Coord,
    from: &Coord,
    to: &Coord,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[Set; 2], GeoError> {
    use crate::numerical::{ExactAdmission, exact_rational};
    use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalCompare};
    for polygon in polygons {
        charge(context, progress, 1)?;
        if polygon.chart().is_none() {
            return Err(GeoError::domain(
                "written boundary sides require source charts",
            ));
        }
    }
    let directions = {
        let mut admission = ExactAdmission::new(context, progress);
        if admission.rational(RationalCompare, &[point.y()], 2, || {
            Ok(point.y() == &Rat::from_i64(90) || point.y() == &Rat::from_i64(-90))
        })? {
            // A written fragment with an interior pole point lies completely
            // on that pole row. The row is one physical point, not a 1D edge.
            return Err(GeoError::domain(
                "a physical pole has no open chart tangent",
            ));
        }
        let dx = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[to.x(), from.x()],
            || to.x().sub(from.x()),
        )?;
        let dy = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[to.y(), from.y()],
            || to.y().sub(from.y()),
        )?;
        if dx.is_zero() && dy.is_zero() {
            return Err(GeoError::domain("zero written noded tangent"));
        }
        admission.rational(Linear, &[&dx, &dy], 4, || {
            Ok([[dy.neg(), dx.clone()], [dy.clone(), dx.neg()]])
        })?
    };
    let first = union_direction(
        point,
        &directions[0],
        polygons,
        complementary,
        true,
        context,
        progress,
    )?;
    let second = union_direction(
        point,
        &directions[1],
        polygons,
        complementary,
        true,
        context,
        progress,
    )?;
    if first == Set::Boundary || second == Set::Boundary {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    Ok([first, second])
}

fn sectors(
    point: &Coord,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let mut rays = vec![
        [Rat::one(), Rat::zero()],
        [Rat::zero(), Rat::one()],
        [Rat::from_i64(-1), Rat::zero()],
        [Rat::zero(), Rat::from_i64(-1)],
    ];
    let alias = cut_alias(
        point,
        &mut crate::numerical::ExactAdmission::new(context, progress),
    )?;
    let mut on_boundary = false;
    let mut on_written_boundary = false;
    for polygon in polygons {
        for (a, b) in edges(core::slice::from_ref(polygon)) {
            for query in core::iter::once(point).chain(alias.as_ref()) {
                let mut admission = crate::numerical::ExactAdmission::new(context, progress);
                if crate::topology::on_segment_admitted(query, a, b, &mut admission)? {
                    on_boundary = true;
                    on_written_boundary |= polygon.interior() == RegionInterior::Written;
                    for end in [a, b] {
                        let ray = vector(query, end, &mut admission)?;
                        if ray != [Rat::zero(), Rat::zero()] {
                            rays.push(ray);
                        }
                    }
                }
            }
        }
    }
    if !on_boundary {
        let canonical = canonical_point(point, context, progress)?;
        return union_location(
            &chart_point(&canonical, context, progress)?,
            polygons,
            complementary,
            context,
            progress,
        );
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut rays, |a, b| {
        direction_order(
            a,
            b,
            &mut crate::numerical::ExactAdmission::new(context, progress),
        )
    })?;
    let mut kept = 1;
    for read in 1..rays.len() {
        if !direction_order(
            &rays[kept - 1],
            &rays[read],
            &mut crate::numerical::ExactAdmission::new(context, progress),
        )?
        .is_eq()
        {
            rays.swap(kept, read);
            kept += 1;
        }
    }
    rays.truncate(kept);
    let mut inside = false;
    let mut outside = false;
    for index in 0..rays.len() {
        let next = (index + 1) % rays.len();
        // Axes make every sector less than pi; a positive combination is interior.
        let direction = {
            let mut admission = crate::numerical::ExactAdmission::new(context, progress);
            let mut add = |axis: usize| {
                crate::numerical::exact_rational(
                    Some(&mut admission),
                    purrdf_xsd::integer::ExactOperation::RationalAdd,
                    &[&rays[index][axis], &rays[next][axis]],
                    || rays[index][axis].add(&rays[next][axis]),
                )
            };
            [add(0)?, add(1)?]
        };
        let location = union_direction(
            point,
            &direction,
            polygons,
            complementary,
            // Every incident original edge and cut alias contributed its two
            // rays above. A direction strictly between consecutive rays cannot
            // meet one of those edges; a nonincident finite edge stays away for
            // sufficiently small positive epsilon. The incidence pass in the
            // directional locator is therefore unnecessary for this sector.
            true,
            context,
            progress,
        )?;
        match location {
            Set::Interior => inside = true,
            Set::Exterior => outside = true,
            Set::Boundary => {
                return Err(GeoError::ArithmeticOverflow(
                    "atlas sector meets a boundary",
                ));
            }
        }
        if inside && outside {
            return Ok(Set::Boundary);
        }
    }
    Ok(if inside {
        Set::Interior
    } else if complementary || !on_written_boundary {
        Set::Exterior
    } else {
        Set::Boundary
    })
}
/// A validated chart point has at most one other incident longitude spelling.
/// Original chart vertices lie in [-180,180], so translating an edge by +/-360
/// can meet this point only at an exact cut longitude. Moving the query instead
/// preserves both incidence and the edge-to-query direction, and avoids copying
/// every edge three times. Poles use their separate identified-point atlas.
fn cut_alias(
    point: &Coord,
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<Option<Coord>, GeoError> {
    let west = Rat::from_i64(-180);
    let east = Rat::from_i64(180);
    let shift = admission.rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[point.x(), &west, &east],
        2,
        || {
            Ok(if point.x() == &west {
                360
            } else if point.x() == &east {
                -360
            } else {
                0
            })
        },
    )?;
    if shift == 0 {
        Ok(None)
    } else {
        translated(point, shift, admission).map(Some)
    }
}

fn union_direction(
    point: &Coord,
    direction: &[Rat; 2],
    polygons: &[PreparedPolygon],
    complementary: bool,
    transverse: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    let shift = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.x()],
        2,
        || {
            Ok(
                if point.x() == &Rat::from_i64(180) && direction[0].signum() > 0 {
                    -360
                } else if point.x() == &Rat::from_i64(-180) && direction[0].signum() < 0 {
                    360
                } else {
                    0
                },
            )
        },
    )?;
    let canonical = translated(point, shift, &mut admission)?;
    let mut boundary = false;
    for polygon in polygons {
        let GeometryBody::Polygon(rings) = polygon.chart().expect("written polygon chart").body()
        else {
            unreachable!("prepared polygon chart")
        };
        let locate = if transverse {
            crate::topology::locate_surface_direction_transverse_admitted
        } else {
            crate::topology::locate_surface_direction_admitted
        };
        let mut location = locate(&canonical, direction, rings, &mut admission)?;
        if polygon.interior() == RegionInterior::Complement {
            location = complement(location);
        }
        if location == Set::Interior {
            return Ok(if complementary {
                Set::Exterior
            } else {
                Set::Interior
            });
        }
        boundary |= location == Set::Boundary;
    }
    Ok(if boundary {
        Set::Boundary
    } else if complementary {
        Set::Interior
    } else {
        Set::Exterior
    })
}
fn positive_min(
    bound: &mut Rat,
    parameter: Rat,
    admission: &mut crate::numerical::ExactAdmission<'_, '_>,
) -> Result<(), GeoError> {
    if admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[bound, &parameter],
        2,
        || Ok(parameter > Rat::zero() && parameter < *bound),
    )? {
        *bound = parameter;
    }
    Ok(())
}
fn canonical_point(
    point: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<LonLat, GeoError> {
    use crate::numerical::{ExactAdmission, exact_rational};
    use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalCompare};
    let mut admission = ExactAdmission::new(context, progress);
    let mut longitude = admission.rational(Linear, &[point.x()], 1, || Ok(point.x().clone()))?;
    let turn = Rat::from_i64(360);
    while admission.rational(RationalCompare, &[&longitude], 1, || {
        Ok(longitude < Rat::from_i64(-180))
    })? {
        longitude = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[&longitude, &turn],
            || longitude.add(&turn),
        )?;
    }
    while admission.rational(RationalCompare, &[&longitude], 1, || {
        Ok(longitude >= Rat::from_i64(180))
    })? {
        longitude = exact_rational(
            Some(&mut admission),
            RationalAdd,
            &[&longitude, &turn],
            || longitude.sub(&turn),
        )?;
    }
    let latitude = admission.rational(Linear, &[point.y()], 1, || Ok(point.y().clone()))?;
    let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude, &latitude], 4)
        .ok_or(GeoError::ArithmeticOverflow("canonical point admission"))?;
    progress.exact(context, cost, || LonLat::new(longitude, latitude))
}

fn pole(
    point: &LonLat,
    polygons: &[PreparedPolygon],
    complementary: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    use crate::numerical::{ExactAdmission, exact_rational};
    use purrdf_xsd::integer::ExactOperation::{
        Linear, RationalAdd, RationalCompare, RationalDivide, RationalMultiply,
    };
    let north = point.latitude().signum() > 0;
    let mut gap = Rat::one();
    let mut touches = false;
    for polygon in polygons {
        for vertex in polygon.chart().expect("written polygon chart").coords() {
            let mut admission = ExactAdmission::new(context, progress);
            let distance = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[point.latitude(), vertex.y()],
                || point.latitude().sub(vertex.y()),
            )?;
            let distance = admission.rational(Linear, &[&distance], 1, || Ok(distance.abs()))?;
            if distance.is_zero() {
                touches |= polygon.interior() == RegionInterior::Written;
            } else {
                positive_min(&mut gap, distance, &mut admission)?;
            }
        }
    }
    let two = Rat::from_i64(2);
    let latitude = {
        let mut admission = ExactAdmission::new(context, progress);
        let delta = exact_rational(Some(&mut admission), RationalDivide, &[&gap, &two], || {
            gap.div(&two).expect("positive polar annulus")
        })?;
        let pole = Rat::from_i64(if north { 90 } else { -90 });
        exact_rational(Some(&mut admission), RationalAdd, &[&pole, &delta], || {
            if north {
                pole.sub(&delta)
            } else {
                pole.add(&delta)
            }
        })?
    };
    let mut cuts = vec![Rat::from_i64(-180), Rat::from_i64(180)];
    for (a, b) in edges(polygons) {
        let mut admission = ExactAdmission::new(context, progress);
        if admission.rational(RationalCompare, &[a.y(), b.y(), &latitude], 2, || {
            Ok((a.y() > &latitude) != (b.y() > &latitude))
        })? {
            let dy = exact_rational(Some(&mut admission), RationalAdd, &[b.y(), a.y()], || {
                b.y().sub(a.y())
            })?;
            let delta = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[&latitude, a.y()],
                || latitude.sub(a.y()),
            )?;
            let parameter =
                exact_rational(Some(&mut admission), RationalDivide, &[&delta, &dy], || {
                    delta.div(&dy).expect("crossing has distinct latitudes")
                })?;
            let dx = exact_rational(Some(&mut admission), RationalAdd, &[b.x(), a.x()], || {
                b.x().sub(a.x())
            })?;
            let offset = exact_rational(
                Some(&mut admission),
                RationalMultiply,
                &[&dx, &parameter],
                || dx.mul(&parameter),
            )?;
            let longitude =
                exact_rational(Some(&mut admission), RationalAdd, &[a.x(), &offset], || {
                    a.x().add(&offset)
                })?;
            cuts.push(longitude);
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut cuts, |a, b| {
        ExactAdmission::new(context, progress).rational(
            RationalCompare,
            &[a, b],
            1,
            || Ok(a.cmp(b)),
        )
    })?;
    let mut kept = 1;
    for read in 1..cuts.len() {
        let distinct = ExactAdmission::new(context, progress).rational(
            Linear,
            &[&cuts[kept - 1], &cuts[read]],
            1,
            || Ok(cuts[kept - 1] != cuts[read]),
        )?;
        if distinct {
            cuts.swap(kept, read);
            kept += 1;
        }
    }
    cuts.truncate(kept);
    let mut inside = false;
    let mut outside = false;
    for pair in cuts.windows(2) {
        let chart = {
            let mut admission = ExactAdmission::new(context, progress);
            let sum = exact_rational(
                Some(&mut admission),
                RationalAdd,
                &[&pair[0], &pair[1]],
                || pair[0].add(&pair[1]),
            )?;
            let longitude =
                exact_rational(Some(&mut admission), RationalDivide, &[&sum, &two], || {
                    sum.div(&two).expect("positive midpoint")
                })?;
            let latitude = admission.rational(Linear, &[&latitude], 1, || Ok(latitude.clone()))?;
            Coord::xy(longitude, latitude)
        };
        match union_location(&chart, polygons, complementary, context, progress)? {
            Set::Interior => inside = true,
            Set::Exterior => outside = true,
            Set::Boundary => {
                return Err(GeoError::ArithmeticOverflow(
                    "polar sector meets a boundary",
                ));
            }
        }
        if inside && outside {
            return Ok(Set::Boundary);
        }
    }
    Ok(if inside {
        Set::Interior
    } else if touches && !complementary {
        Set::Boundary
    } else {
        Set::Exterior
    })
}

impl PreparedRegion {
    /// Exact location under the shared identified source-coordinate atlas.
    ///
    /// # Errors
    /// Refuses incomplete work/workspace admission and cancellation.
    pub fn locate(&self, point: &LonLat, context: &mut MetricContext) -> Result<Set, GeoError> {
        locate(point, self, context)
    }
}

#[cfg(test)]
mod tests;
