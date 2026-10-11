// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The constructors that derive a new geometry from an old one: boundary,
//! envelope, convex hull and centroid.
//!
//! # Everything here is planar, and every output is `XY`
//!
//! OGC GeoSPARQL 1.1, Clause 10.2, verbatim:
//!
//! > Geometric functions working with Geometries that have Z values will ignore
//! > Z values in calculations and first project geometry onto the Z=0 level. …
//! > Like Z values in coordinates, M values are to be ignored.
//!
//! So every function here reads only `x` and `y`, and every geometry it builds
//! carries [`CoordDim::Xy`]. That is not a simplification, it is what "first
//! project geometry onto the Z=0 level" means: the [`boundary`] of a `POLYGON Z`
//! is a **two-dimensional** `MULTILINESTRING`, because the object whose boundary
//! was taken is the *projected* polygon, and the projected polygon has no
//! elevation to hand on. A constructor that copied the input's `Z` through would
//! be claiming to have worked in three dimensions, which it did not.
//!
//! # The empty answers, spelled out
//!
//! Three of these constructors can produce nothing, and each spells it the way
//! the *kind* of the answer requires rather than by echoing the input's kind:
//!
//! * [`boundary`] returns the container its answer belongs in — an empty
//!   `MULTIPOINT` for a closed curve (a curve's boundary is a point set), an
//!   empty `GEOMETRYCOLLECTION` for a point (a point's boundary is empty of any
//!   type at all).
//! * [`envelope`] and [`convex_hull`] return an empty `GEOMETRYCOLLECTION` for
//!   an empty input, which is the canonical empty geometry.
//!
//! # Exactness
//!
//! [`boundary`], [`envelope`] and [`convex_hull`] are exact with nothing to
//! round: they select and reorder ordinates that were already in the input, and
//! the hull's only decision is [`crate::topology::orientation`], the
//! sign of an exact rational cross product. [`centroid`] is exact for areal and
//! point geometries and carries the fixed-scale truncation of
//! [`crate::measure::LENGTH_SCALE_DIGITS`] for the length-weighted linear case,
//! because segment lengths are irrational. See each function for the specifics.

use crate::context::{Retained, WorkProgress};
use crate::exact::{Int, Rat};
use crate::geom::{
    Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, GeometryLiteral,
};
use crate::measure::BorrowedParts;
use crate::{GeoError, MaterializedOutputReceipt, MetricContext, MetricWorkObserver};
use purrdf_lex::allocation::{Admission, Memory, Resident};
use std::cell::RefCell;

pub(crate) mod arithmetic;
use arithmetic::Arithmetic;

/// One exact planar construction, with the original GeoSPARQL XY laws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Derivation {
    /// The mod-2 curve boundary and every surface ring in written order.
    Boundary,
    /// The exact minimum bounding box, including point and line degeneracies.
    Envelope,
    /// Andrew's strictly convex monotone chain in canonical coordinate order.
    ConvexHull,
    /// The area, fixed-scale length, or point-weighted centroid.
    Centroid,
}

/// A constructed literal and its original immutable buffer owner. Ordinary
/// clones share that owner; consuming this carrier cannot detach its payload.
#[derive(Clone, Debug)]
pub struct DerivedGeometry {
    literal: Retained<GeometryLiteral>,
    receipt: MaterializedOutputReceipt,
}

impl DerivedGeometry {
    /// Borrow the complete output while its original allocations remain owned.
    #[must_use]
    pub fn literal(&self) -> &GeometryLiteral {
        &self.literal
    }

    /// Obtain the original policy receipt before dropping all result clones.
    /// Release it through the producing context after those payloads die.
    #[must_use]
    pub fn output_receipt(&self) -> MaterializedOutputReceipt {
        self.receipt.clone()
    }
}

/// Construct through the existing complete algorithms with before-birth native
/// storage and exact arithmetic. The result retains its original buffers;
/// scratch dies and is refunded before immutable publication.
///
/// # Errors
/// Preserves selected output/work/precision/physical limits, cancellation and
/// exact native errors. No partial construction or resident fallback escapes.
pub fn derive_metered(
    operation: Derivation,
    literal: &GeometryLiteral,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<DerivedGeometry, GeoError> {
    produce_metered(
        literal,
        context,
        observer,
        |geometry, memory, arithmetic| {
            derive_inner(operation, geometry, memory, arithmetic).map(Some)
        },
    )?
    .ok_or(GeoError::ArithmeticOverflow("complete derived geometry"))
}

/// Copy the original 1-based member through the same owned output producer.
/// A missing member remains absent, rather than becoming an empty geometry.
///
/// # Errors
/// Preserves original physical, policy, arithmetic and cancellation failures.
pub fn member_metered(
    literal: &GeometryLiteral,
    index: usize,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Option<DerivedGeometry>, GeoError> {
    produce_metered(
        literal,
        context,
        observer,
        |geometry, memory, arithmetic| {
            crate::measure::geometry_n_with_memory(geometry, index, memory, arithmetic)
        },
    )
}

fn produce_metered(
    literal: &GeometryLiteral,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
    produce: impl FnOnce(
        &Geometry,
        &mut Memory<'_, dyn Admission + '_>,
        &Arithmetic<'_, '_, '_>,
    ) -> Result<Option<Geometry>, GeoError>,
) -> Result<Option<DerivedGeometry>, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(Some(observer));
    let (value, bytes) =
        progress.with_native_storage(|outer, observer| {
            let original = outer.admitted_bytes();
            let context = RefCell::new(&mut *context);
            let numerical = RefCell::new(WorkProgress::new(Some(observer)));
            let arithmetic = Arithmetic::admitted(&context, &numerical);
            let value = crate::context::with_policy_memory(&context, outer, |memory| {
                memory.scope(|memory| {
                    let geometry = produce(literal.geometry(), memory, &arithmetic)?;
                    Ok(geometry
                        .map(|geometry| GeometryLiteral::new(literal.crs().clone(), geometry)))
                })
            })?;
            let bytes = outer.admitted_bytes().checked_sub(original).ok_or(
                GeoError::ArithmeticOverflow("constructed original buffer total"),
            )?;
            if value.is_none() {
                outer.release_bytes(bytes).map_err(GeoError::Storage)?;
                context.borrow_mut().release_workspace(bytes as u64)?;
            }
            Ok((
                value,
                u64::try_from(bytes)
                    .map_err(|_| GeoError::ArithmeticOverflow("constructed buffer width"))?,
            ))
        })?;
    let Some(value) = value else {
        return Ok(None);
    };
    let controls = Retained::<GeometryLiteral>::required_bytes(&progress)?;
    let total = bytes
        .checked_add(controls)
        .ok_or(GeoError::ArithmeticOverflow(
            "constructed immutable controls",
        ))?;
    if let Err(error) = context.admit_workspace(controls) {
        drop(value);
        let _ = context.release_workspace(bytes);
        return Err(error);
    }
    let published = progress
        .context_poll(context)
        .and_then(|()| Retained::publish_original(value, &mut progress));
    let literal = match published {
        Ok(literal) => literal,
        Err(original) => {
            let _ = context.release_workspace(total);
            return Err(original);
        }
    };
    match context.register_materialized_output(total, &mut progress) {
        Ok(receipt) => Ok(Some(DerivedGeometry { literal, receipt })),
        Err(original) => {
            drop(literal);
            let _ = context.release_workspace(total);
            Err(original)
        }
    }
}

fn resident<T>(body: impl FnOnce(&mut Memory<'_, dyn Admission>) -> Result<T, GeoError>) -> T {
    let mut resident = Resident;
    body(&mut Memory::new(&mut resident as &mut dyn Admission))
        .expect("resident exact geometry construction")
}

fn derive_inner<S: Admission + ?Sized>(
    operation: Derivation,
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    match operation {
        Derivation::Boundary => boundary_inner(geometry, memory, arithmetic),
        Derivation::Envelope => envelope_inner(geometry, memory, arithmetic),
        Derivation::ConvexHull => convex_hull_inner(geometry, memory, arithmetic),
        Derivation::Centroid => {
            let center = centroid_inner(geometry, memory, arithmetic)?;
            if center.is_some() {
                arithmetic.output(1)?;
            }
            arithmetic.geometry(CoordDim::Xy, GeometryBody::Point(center))
        }
    }
}

fn nothing() -> Geometry {
    Geometry::empty(CoordDim::Xy, GeometryKind::GeometryCollection)
}

fn planar(body: GeometryBody, arithmetic: &Arithmetic<'_, '_, '_>) -> Result<Geometry, GeoError> {
    arithmetic.geometry(CoordDim::Xy, body)
}

fn run<S: Admission + ?Sized>(
    points: &[Coord],
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<CoordSeq, GeoError> {
    arithmetic.output(points.len())?;
    memory.try_map(points, |point, _| arithmetic.plane(point))
}

/// The original boundary: empty for points, mod-2 endpoints for curves, every
/// surface ring in written order, and memberwise boundaries for collections.
/// Every returned coordinate is XY.
#[must_use]
pub fn boundary(geometry: &Geometry) -> Geometry {
    resident(|memory| boundary_inner(geometry, memory, &Arithmetic::resident()))
}

fn boundary_inner<S: Admission + ?Sized>(
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    crate::geom::try_fold_with_memory(
        geometry,
        memory,
        |member, memory| member_boundary(member, memory, arithmetic),
        |_, boundaries, _| planar(GeometryBody::GeometryCollection(boundaries), arithmetic),
    )
    .map_err(crate::geom::FoldError::into_geo_error)
}

fn member_boundary<S: Admission + ?Sized>(
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    arithmetic.visit()?;
    match geometry.body() {
        GeometryBody::Point(_) | GeometryBody::MultiPoint(_) => Ok(nothing()),
        GeometryBody::LineString(_) | GeometryBody::MultiLineString(_) => {
            let mut endpoints = crate::topology::curve_boundary_points_with_memory(
                geometry,
                memory,
                &mut |left, right| arithmetic.compare_coord(left, right),
                &mut |point| {
                    arithmetic.output(1)?;
                    arithmetic.plane(point)
                },
            )?;
            let mut points = Vec::new();
            memory
                .reserve(&mut points, endpoints.len())
                .map_err(GeoError::Storage)?;
            #[expect(
                clippy::iter_with_drain,
                reason = "The original endpoint Vec buffer must remain owned until release_vec destroys it and refunds its grant"
            )]
            points.extend(endpoints.drain(..).map(Some));
            memory.release_vec(endpoints).map_err(GeoError::Storage)?;
            planar(GeometryBody::MultiPoint(points), arithmetic)
        }
        GeometryBody::Polygon(rings) => {
            surface_boundary(core::iter::once(rings.as_slice()), memory, arithmetic)
        }
        GeometryBody::MultiPolygon(polygons) => {
            surface_boundary(polygons.iter().map(Vec::as_slice), memory, arithmetic)
        }
        GeometryBody::GeometryCollection(_) => {
            unreachable!("the bottom-up fold handles collections")
        }
    }
}

fn surface_boundary<'a, S: Admission + ?Sized>(
    polygons: impl Iterator<Item = &'a [CoordSeq]>,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    let mut rings = Vec::new();
    for polygon in polygons {
        for ring in polygon {
            memory
                .reserve_for_push(&mut rings)
                .map_err(GeoError::Storage)?;
            rings.push(run(ring, memory, arithmetic)?);
        }
    }
    planar(GeometryBody::MultiLineString(rings), arithmetic)
}

/// The exact minimum XY box. A point extent remains a Point, a one-axis extent
/// remains a LineString, and an empty input remains an empty collection.
#[must_use]
pub fn envelope(geometry: &Geometry) -> Geometry {
    resident(|memory| envelope_inner(geometry, memory, &Arithmetic::resident()))
}

fn borrowed_bounds<'a, S: Admission + ?Sized>(
    geometry: &'a Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Option<crate::measure::BorrowedBounds<'a>>, GeoError> {
    use core::ops::ControlFlow;
    let mut bounds: Option<crate::measure::BorrowedBounds<'a>> = None;
    let mut failed = None;
    let _ = geometry
        .visit_coords_with_memory(memory, |coord| {
            let result = (|| {
                arithmetic.visit()?;
                if let Some(bounds) = &mut bounds {
                    bounds.include_using(coord.x(), coord.y(), &mut |left, right| {
                        arithmetic.compare(left, right)
                    })?;
                } else {
                    bounds = Some(crate::measure::BorrowedBounds {
                        west: coord.x(),
                        south: coord.y(),
                        east: coord.x(),
                        north: coord.y(),
                    });
                }
                Ok(())
            })();
            match result {
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => {
                    failed = Some(error);
                    ControlFlow::Break(())
                }
            }
        })
        .map_err(GeoError::Storage)?;
    match failed {
        Some(error) => Err(error),
        None => Ok(bounds),
    }
}

fn envelope_inner<S: Admission + ?Sized>(
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    let Some(crate::measure::BorrowedBounds {
        west,
        south,
        east,
        north,
    }) = borrowed_bounds(geometry, memory, arithmetic)?
    else {
        return Ok(nothing());
    };
    let flat_x = arithmetic.compare(west, east)?.is_eq();
    let flat_y = arithmetic.compare(south, north)?.is_eq();
    if flat_x && flat_y {
        arithmetic.output(1)?;
        return planar(
            GeometryBody::Point(Some(Coord::xy(
                arithmetic.original(west)?,
                arithmetic.original(south)?,
            ))),
            arithmetic,
        );
    }
    let positions = [
        (west, south),
        (east, south),
        (east, north),
        (west, north),
        (west, south),
    ];
    let line_positions = [(west, south), (east, north)];
    let positions = if flat_x || flat_y {
        &line_positions[..]
    } else {
        &positions[..]
    };
    arithmetic.output(positions.len())?;
    let mut run = memory.try_map(positions, |&(x, y), _| {
        Ok::<_, GeoError>(Coord::xy(arithmetic.original(x)?, arithmetic.original(y)?))
    })?;
    if flat_x || flat_y {
        return planar(GeometryBody::LineString(run), arithmetic);
    }
    let mut rings = Vec::new();
    memory
        .reserve_for_push(&mut rings)
        .map_err(GeoError::Storage)?;
    rings.push(core::mem::take(&mut run));
    planar(GeometryBody::Polygon(rings), arithmetic)
}

/// The strictly convex XY monotone chain, counter-clockwise from its smallest
/// position. Empty, point and collinear cases retain their proper dimensions.
#[must_use]
pub fn convex_hull(geometry: &Geometry) -> Geometry {
    resident(|memory| convex_hull_inner(geometry, memory, &Arithmetic::resident()))
}

fn convex_hull_inner<S: Admission + ?Sized>(
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Geometry, GeoError> {
    use core::ops::ControlFlow;
    let mut points = Vec::new();
    let mut failed = None;
    // The positions stay borrowed through sorting and chain selection.
    let _ =
        crate::geom::visit_coords_with_memory_using(geometry, memory, |point, memory| match memory
            .push(&mut points, point)
        {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => {
                failed = Some(error);
                ControlFlow::Break(())
            }
        })
        .map_err(GeoError::Storage)?;
    if let Some(error) = failed {
        return Err(GeoError::Storage(error));
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut points, |left, right| {
        arithmetic.compare_coord(left, right)
    })?;
    let unique = purrdf_lex::walk::try_partition_unique_by(&mut points, |left, right| {
        Ok::<_, GeoError>(arithmetic.compare_coord(left, right)?.is_eq())
    })?;
    points.truncate(unique);
    let result = match points.len() {
        0 => nothing(),
        1 => {
            arithmetic.output(1)?;
            planar(
                GeometryBody::Point(Some(arithmetic.plane(points[0])?)),
                arithmetic,
            )?
        }
        _ => {
            let hull = monotone_chain(&points, memory, arithmetic)?;
            let closed = hull.len() >= 3;
            arithmetic.output(
                hull.len()
                    .checked_add(usize::from(closed))
                    .ok_or(GeoError::ArithmeticOverflow("convex hull closure"))?,
            )?;
            let mut run = memory.try_map(&hull, |point, _| arithmetic.plane(point))?;
            if closed {
                memory
                    .reserve_for_push(&mut run)
                    .map_err(GeoError::Storage)?;
                run.push(arithmetic.plane(hull[0])?);
            }
            memory.release_vec(hull).map_err(GeoError::Storage)?;
            if closed {
                let mut rings = Vec::new();
                memory.push(&mut rings, run).map_err(GeoError::Storage)?;
                planar(GeometryBody::Polygon(rings), arithmetic)?
            } else {
                planar(GeometryBody::LineString(run), arithmetic)?
            }
        }
    };
    memory.release_vec(points).map_err(GeoError::Storage)?;
    Ok(result)
}

fn monotone_chain<'a, S: Admission + ?Sized>(
    points: &[&'a Coord],
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Vec<&'a Coord>, GeoError> {
    let mut lower = half_chain(points.iter().copied(), memory, arithmetic)?;
    let mut upper = half_chain(points.iter().rev().copied(), memory, arithmetic)?;
    lower.pop();
    upper.pop();
    let required = lower
        .len()
        .checked_add(upper.len())
        .ok_or(GeoError::ArithmeticOverflow(
            "convex hull chain concatenation",
        ))?;
    memory
        .reserve(&mut lower, required)
        .map_err(GeoError::Storage)?;
    lower.append(&mut upper);
    memory.release_vec(upper).map_err(GeoError::Storage)?;
    Ok(lower)
}

fn half_chain<'a, S: Admission + ?Sized>(
    points: impl Iterator<Item = &'a Coord>,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Vec<&'a Coord>, GeoError> {
    let mut chain = Vec::new();
    for point in points {
        arithmetic.visit()?;
        while chain.len() >= 2
            && arithmetic.orientation(chain[chain.len() - 2], chain[chain.len() - 1], point)? <= 0
        {
            chain.pop();
        }
        memory.push(&mut chain, point).map_err(GeoError::Storage)?;
    }
    Ok(chain)
}

/// The original highest-dimension centroid: exact areal moments, fixed-scale
/// length weights, then point means and finally all written vertices.
#[must_use]
pub fn centroid(geometry: &Geometry) -> Option<Coord> {
    resident(|memory| centroid_inner(geometry, memory, &Arithmetic::resident()))
}

fn centroid_inner<S: Admission + ?Sized>(
    geometry: &Geometry,
    memory: &mut Memory<'_, S>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Option<Coord>, GeoError> {
    let parts = BorrowedParts::with_memory(geometry, memory).map_err(GeoError::Storage)?;
    let result = match areal_centroid(&parts, arithmetic)? {
        Some(value) => Some(value),
        None => match linear_centroid(&parts, arithmetic)? {
            Some(value) => Some(value),
            None => match mean(parts.points.iter().copied(), arithmetic)? {
                Some(value) => Some(value),
                None => mean(parts.vertices(), arithmetic)?,
            },
        },
    };
    parts.release(memory).map_err(GeoError::Storage)?;
    Ok(result)
}

fn mean<'a>(
    points: impl Iterator<Item = &'a Coord>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Option<Coord>, GeoError> {
    let mut count = 0_i64;
    let mut sum_x = Rat::zero();
    let mut sum_y = Rat::zero();
    for point in points {
        count = count
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("centroid point count"))?;
        sum_x = arithmetic.add(&sum_x, point.x())?;
        sum_y = arithmetic.add(&sum_y, point.y())?;
    }
    if count == 0 {
        return Ok(None);
    }
    let divisor = Rat::from_i64(count);
    Ok(Some(Coord::xy(
        arithmetic.divide(&sum_x, &divisor)?,
        arithmetic.divide(&sum_y, &divisor)?,
    )))
}

fn areal_centroid(
    parts: &BorrowedParts<'_>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Option<Coord>, GeoError> {
    let mut area = Rat::zero();
    let mut moment_x = Rat::zero();
    let mut moment_y = Rat::zero();
    for polygon in &parts.polygons {
        if let Some((exterior, holes)) = polygon.split_first() {
            accumulate_ring(
                exterior,
                true,
                &mut area,
                &mut moment_x,
                &mut moment_y,
                arithmetic,
            )?;
            for hole in holes {
                accumulate_ring(
                    hole,
                    false,
                    &mut area,
                    &mut moment_x,
                    &mut moment_y,
                    arithmetic,
                )?;
            }
        }
    }
    if area.is_zero() {
        return Ok(None);
    }
    Ok(Some(Coord::xy(
        arithmetic.divide(&moment_x, &area)?,
        arithmetic.divide(&moment_y, &area)?,
    )))
}

fn accumulate_ring(
    ring: &[Coord],
    additive: bool,
    area: &mut Rat,
    moment_x: &mut Rat,
    moment_y: &mut Rat,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<(), GeoError> {
    let signed = arithmetic.ring_area(ring)?;
    if signed.is_zero() {
        return Ok(());
    }
    let winding = i64::from(signed.signum());
    let factor = Rat::from_i64(if additive { winding } else { -winding });
    let six = Rat::from_i64(6);
    let mut sum_x = Rat::zero();
    let mut sum_y = Rat::zero();
    for pair in ring.windows(2) {
        let (p, q) = (&pair[0], &pair[1]);
        let cross = arithmetic.subtract(
            &arithmetic.multiply(p.x(), q.y())?,
            &arithmetic.multiply(q.x(), p.y())?,
        )?;
        sum_x = arithmetic.add(
            &sum_x,
            &arithmetic.multiply(&arithmetic.add(p.x(), q.x())?, &cross)?,
        )?;
        sum_y = arithmetic.add(
            &sum_y,
            &arithmetic.multiply(&arithmetic.add(p.y(), q.y())?, &cross)?,
        )?;
    }
    *area = arithmetic.add(area, &arithmetic.multiply(&factor, &signed)?)?;
    *moment_x = arithmetic.add(
        moment_x,
        &arithmetic.multiply(&factor, &arithmetic.divide(&sum_x, &six)?)?,
    )?;
    *moment_y = arithmetic.add(
        moment_y,
        &arithmetic.multiply(&factor, &arithmetic.divide(&sum_y, &six)?)?,
    )?;
    Ok(())
}

fn linear_centroid(
    parts: &BorrowedParts<'_>,
    arithmetic: &Arithmetic<'_, '_, '_>,
) -> Result<Option<Coord>, GeoError> {
    let half = Rat::new(Int::one(), Int::from_i64(2)).expect("two is not zero");
    let mut weight = Int::zero();
    let mut moment_x = Rat::zero();
    let mut moment_y = Rat::zero();
    for (p, q) in parts.segments() {
        let scaled = crate::measure::scaled_segment_length_using(p, q, arithmetic)?;
        if scaled.is_zero() {
            continue;
        }
        let w = crate::measure::from_scaled_using(&scaled, arithmetic)?;
        moment_x = arithmetic.add(
            &moment_x,
            &arithmetic.multiply(
                &w,
                &arithmetic.multiply(&arithmetic.add(p.x(), q.x())?, &half)?,
            )?,
        )?;
        moment_y = arithmetic.add(
            &moment_y,
            &arithmetic.multiply(
                &w,
                &arithmetic.multiply(&arithmetic.add(p.y(), q.y())?, &half)?,
            )?,
        )?;
        weight = arithmetic.integer_add(&weight, &scaled)?;
    }
    if weight.is_zero() {
        return Ok(None);
    }
    let total = crate::measure::from_scaled_using(&weight, arithmetic)?;
    Ok(Some(Coord::xy(
        arithmetic.divide(&moment_x, &total)?,
        arithmetic.divide(&moment_y, &total)?,
    )))
}

#[cfg(test)]
mod tests {
    use super::{boundary, centroid, convex_hull, envelope};
    use crate::exact::{Int, Rat};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind};

    // ---- fixtures --------------------------------------------------------

    fn r(value: i64) -> Rat {
        Rat::from_i64(value)
    }

    fn ratio(numerator: i64, denominator: i64) -> Rat {
        Rat::new(Int::from_i64(numerator), Int::from_i64(denominator))
            .expect("a non-zero denominator")
    }

    fn seq(points: &[(i64, i64)]) -> CoordSeq {
        points.iter().map(|&(x, y)| Coord::xy(r(x), r(y))).collect()
    }

    fn point(x: i64, y: i64) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(r(x), r(y)))),
        )
        .expect("a well-formed point")
    }

    fn multipoint(points: &[(i64, i64)]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(
                points
                    .iter()
                    .map(|&(x, y)| Some(Coord::xy(r(x), r(y))))
                    .collect(),
            ),
        )
        .expect("a well-formed multipoint")
    }

    fn line(points: &[(i64, i64)]) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::LineString(seq(points)))
            .expect("a well-formed line")
    }

    fn multiline(lines: &[&[(i64, i64)]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiLineString(lines.iter().map(|l| seq(l)).collect()),
        )
        .expect("a well-formed multiline")
    }

    fn polygon(rings: &[&[(i64, i64)]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::Polygon(rings.iter().map(|ring| seq(ring)).collect()),
        )
        .expect("a well-formed polygon")
    }

    fn multipolygon(polygons: &[&[&[(i64, i64)]]]) -> Geometry {
        Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPolygon(
                polygons
                    .iter()
                    .map(|rings| rings.iter().map(|ring| seq(ring)).collect())
                    .collect(),
            ),
        )
        .expect("a well-formed multipolygon")
    }

    fn collection(members: Vec<Geometry>) -> Geometry {
        Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(members))
            .expect("uniform dimension")
    }

    const UNIT_SQUARE: &[(i64, i64)] = &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)];

    /// The written positions of a geometry, for pinning an exact vertex order.
    fn written(geometry: &Geometry) -> Vec<(Rat, Rat)> {
        geometry
            .coords()
            .map(|c| (c.x().clone(), c.y().clone()))
            .collect()
    }

    fn pairs(points: &[(i64, i64)]) -> Vec<(Rat, Rat)> {
        points.iter().map(|&(x, y)| (r(x), r(y))).collect()
    }

    // ---- boundary --------------------------------------------------------

    #[test]
    fn a_points_boundary_is_nothing_at_all() {
        for g in [point(1, 2), multipoint(&[(0, 0), (5, 5)])] {
            let b = boundary(&g);
            assert!(b.is_empty(), "a point set has an empty boundary");
            assert_eq!(
                b.kind(),
                GeometryKind::GeometryCollection,
                "and it is empty of any kind, not a degenerate point set"
            );
        }
    }

    #[test]
    fn an_open_curves_boundary_is_its_two_ends_and_a_closed_ones_is_empty() {
        let open = boundary(&line(&[(0, 0), (1, 1), (4, 0)]));
        assert_eq!(
            open.kind(),
            GeometryKind::MultiPoint,
            "a curve's boundary is a point set"
        );
        assert_eq!(
            written(&open),
            pairs(&[(0, 0), (4, 0)]),
            "the two endpoints, in lexicographic order"
        );
        let closed = boundary(&line(UNIT_SQUARE));
        assert!(
            closed.is_empty(),
            "a ring writes its endpoint twice, so the mod-2 rule cancels it"
        );
        assert_eq!(
            closed.kind(),
            GeometryKind::MultiPoint,
            "and the empty answer is still a point set"
        );
    }

    /// The mod-2 rule with teeth: a shared join is an endpoint of two members, so
    /// it cancels and only the far ends survive; a three-way join is an endpoint
    /// of three, an odd count, so it does not.
    #[test]
    fn a_multicurve_boundary_follows_the_mod_two_rule() {
        let joined = boundary(&multiline(&[&[(0, 0), (1, 0)], &[(1, 0), (2, 0)]]));
        assert_eq!(
            written(&joined),
            pairs(&[(0, 0), (2, 0)]),
            "the shared (1,0) is an endpoint of both members and cancels"
        );
        let disjoint = boundary(&multiline(&[&[(0, 0), (1, 0)], &[(5, 0), (6, 0)]]));
        assert_eq!(
            written(&disjoint),
            pairs(&[(0, 0), (1, 0), (5, 0), (6, 0)]),
            "nothing cancels when nothing is shared"
        );
        let three_way = boundary(&multiline(&[
            &[(0, 0), (1, 0)],
            &[(1, 0), (2, 0)],
            &[(1, 0), (1, 5)],
        ]));
        assert_eq!(
            written(&three_way),
            pairs(&[(0, 0), (1, 0), (1, 5), (2, 0)]),
            "(1,0) is an endpoint of three members, an odd count, so it survives"
        );
    }

    #[test]
    fn a_polygons_boundary_is_every_ring_exterior_first() {
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
        ]);
        let b = boundary(&with_hole);
        assert_eq!(
            b.kind(),
            GeometryKind::MultiLineString,
            "a surface's boundary is a curve set"
        );
        let GeometryBody::MultiLineString(lines) = b.body() else {
            panic!("the boundary of a polygon must be a MultiLineString");
        };
        assert_eq!(lines.len(), 2, "the exterior ring and the one hole");
        assert_eq!(
            written(&b),
            pairs(&[
                (0, 0),
                (4, 0),
                (4, 4),
                (0, 4),
                (0, 0),
                (1, 1),
                (3, 1),
                (3, 3),
                (1, 3),
                (1, 1)
            ]),
            "in written order, each ring closed"
        );
        let two = boundary(&multipolygon(&[
            &[UNIT_SQUARE],
            &[&[(5, 5), (6, 5), (6, 6), (5, 6), (5, 5)]],
        ]));
        let GeometryBody::MultiLineString(members) = two.body() else {
            panic!("the boundary of a multipolygon must be a MultiLineString");
        };
        assert_eq!(members.len(), 2, "one ring per component");
    }

    /// "First project geometry onto the Z=0 level" is what makes this a 2D
    /// answer: the boundary of a 3D polygon carries no elevation.
    #[test]
    fn the_boundary_of_a_three_dimensional_polygon_is_a_two_dimensional_curve() {
        let raised = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::Polygon(vec![
                [
                    Coord::new(r(0), r(0), Some(r(7)), None),
                    Coord::new(r(1), r(0), Some(r(7)), None),
                    Coord::new(r(1), r(1), Some(r(7)), None),
                    Coord::new(r(0), r(0), Some(r(7)), None),
                ]
                .into_iter()
                .collect(),
            ]),
        )
        .expect("a closed 3D ring");
        let b = boundary(&raised);
        assert_eq!(b.dim(), CoordDim::Xy, "the answer is planar");
        assert!(
            b.coords().all(|c| c.z().is_none()),
            "and carries no elevation at all"
        );
    }

    #[test]
    fn a_collections_boundary_is_its_members_boundaries() {
        let b = boundary(&collection(vec![
            point(9, 9),
            line(&[(0, 0), (3, 0)]),
            polygon(&[UNIT_SQUARE]),
        ]));
        let GeometryBody::GeometryCollection(members) = b.body() else {
            panic!("the boundary of a collection must be a collection");
        };
        assert_eq!(members.len(), 3, "member for member");
        assert_eq!(
            members[0].kind(),
            GeometryKind::GeometryCollection,
            "the point's boundary is nothing"
        );
        assert_eq!(
            members[1].kind(),
            GeometryKind::MultiPoint,
            "the curve's is a point set"
        );
        assert_eq!(
            members[2].kind(),
            GeometryKind::MultiLineString,
            "the surface's is a curve set"
        );
    }

    // ---- envelope --------------------------------------------------------

    #[test]
    fn a_degenerate_envelope_is_a_point_or_a_curve_rather_than_a_padded_rectangle() {
        let single = envelope(&point(3, 4));
        assert_eq!(
            single.kind(),
            GeometryKind::Point,
            "no extent in either axis"
        );
        assert_eq!(written(&single), pairs(&[(3, 4)]), "at the position itself");
        assert_eq!(
            envelope(&multipoint(&[(3, 4), (3, 4), (3, 4)])).kind(),
            GeometryKind::Point,
            "duplicates give no extent either"
        );

        let horizontal = envelope(&line(&[(1, 5), (9, 5), (4, 5)]));
        assert_eq!(
            horizontal.kind(),
            GeometryKind::LineString,
            "zero extent in y is a segment, not a zero-width polygon"
        );
        assert_eq!(
            written(&horizontal),
            pairs(&[(1, 5), (9, 5)]),
            "from (min_x, min_y) to (max_x, max_y)"
        );
        let vertical = envelope(&line(&[(2, 0), (2, 8)]));
        assert_eq!(
            vertical.kind(),
            GeometryKind::LineString,
            "and so is zero extent in x"
        );
        assert_eq!(written(&vertical), pairs(&[(2, 0), (2, 8)]));
    }

    #[test]
    fn a_normal_envelope_is_the_rectangle_wound_counter_clockwise_from_the_lower_left() {
        let e = envelope(&multipoint(&[(2, 3), (7, 11), (5, 5)]));
        assert_eq!(e.kind(), GeometryKind::Polygon, "two axes of extent");
        assert_eq!(
            written(&e),
            pairs(&[(2, 3), (7, 3), (7, 11), (2, 11), (2, 3)]),
            "the vertex order is pinned so the output is byte-deterministic"
        );
    }

    #[test]
    fn the_envelope_of_an_empty_geometry_is_the_empty_geometry() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            let e = envelope(&Geometry::empty(CoordDim::Xy, kind));
            assert!(e.is_empty(), "{kind:?} EMPTY has no extent to box");
            assert_eq!(
                e.kind(),
                GeometryKind::GeometryCollection,
                "and the empty answer is the canonical empty geometry"
            );
        }
    }

    // ---- convex hull -----------------------------------------------------

    #[test]
    fn the_hull_of_a_square_with_an_interior_point_is_the_square_with_a_pinned_vertex_order() {
        let hull = convex_hull(&multipoint(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 2)]));
        assert_eq!(hull.kind(), GeometryKind::Polygon, "a 2-dimensional hull");
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)]),
            "counter-clockwise from the lexicographically smallest vertex, closed; the interior \
             point is gone"
        );
    }

    #[test]
    fn a_position_on_a_hull_edge_is_not_a_hull_vertex() {
        let hull = convex_hull(&multipoint(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 0)]));
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)]),
            "(2,0) sits on the bottom edge, so the ring stays strictly convex"
        );
    }

    #[test]
    fn a_collinear_input_hulls_to_a_curve_and_a_single_position_to_a_point() {
        let collinear = convex_hull(&multipoint(&[(0, 0), (1, 1), (2, 2)]));
        assert_eq!(
            collinear.kind(),
            GeometryKind::LineString,
            "the hull of collinear points is a segment, not a zero-area ring"
        );
        assert_eq!(
            written(&collinear),
            pairs(&[(0, 0), (2, 2)]),
            "from the smallest position to the largest"
        );

        let duplicated = convex_hull(&multipoint(&[(1, 1), (1, 1), (1, 1)]));
        assert_eq!(
            duplicated.kind(),
            GeometryKind::Point,
            "duplicates collapse to one distinct position"
        );
        assert_eq!(written(&duplicated), pairs(&[(1, 1)]));

        let single = convex_hull(&point(6, 7));
        assert_eq!(
            single.kind(),
            GeometryKind::Point,
            "one position, one point"
        );
        assert_eq!(written(&single), pairs(&[(6, 7)]));

        let two = convex_hull(&multipoint(&[(1, 0), (0, 0)]));
        assert_eq!(
            two.kind(),
            GeometryKind::LineString,
            "two positions, a segment"
        );
        assert_eq!(
            written(&two),
            pairs(&[(0, 0), (1, 0)]),
            "sorted, so the output does not depend on the written order"
        );
    }

    #[test]
    fn the_hull_of_an_empty_geometry_is_the_empty_geometry() {
        let hull = convex_hull(&Geometry::empty(CoordDim::Xy, GeometryKind::MultiPolygon));
        assert!(hull.is_empty(), "there is nothing to hull");
        assert_eq!(hull.kind(), GeometryKind::GeometryCollection);
    }

    #[test]
    fn the_hull_takes_every_position_including_ring_and_curve_vertices() {
        let hull = convex_hull(&collection(vec![
            polygon(&[UNIT_SQUARE]),
            line(&[(3, 0), (3, 1)]),
        ]));
        assert_eq!(
            written(&hull),
            pairs(&[(0, 0), (3, 0), (3, 1), (0, 1), (0, 0)]),
            "ring vertices and curve vertices are both candidates"
        );
    }

    // ---- centroid --------------------------------------------------------

    #[test]
    fn the_areal_centroid_is_exact_with_nothing_rounded() {
        assert_eq!(
            centroid(&polygon(&[UNIT_SQUARE])),
            Some(Coord::xy(ratio(1, 2), ratio(1, 2))),
            "the unit square's centre is exactly (1/2, 1/2)"
        );
        // An L: a 2x1 base and a 1x2 upright, total area 4, worked by hand as the
        // area-weighted mean of the two rectangles' centres —
        // Cx = (2*1 + 2*(1/2))/4 = 3/4, Cy = (2*(1/2) + 2*2)/4 = 5/4.
        let l_shape = polygon(&[&[(0, 0), (2, 0), (2, 1), (1, 1), (1, 3), (0, 3), (0, 0)]]);
        assert_eq!(
            centroid(&l_shape),
            Some(Coord::xy(ratio(3, 4), ratio(5, 4))),
            "the L-shape's centroid, hand-computed"
        );
        assert_eq!(
            centroid(&polygon(&[&[(0, 0), (3, 0), (0, 3), (0, 0)]])),
            Some(Coord::xy(r(1), r(1))),
            "a triangle's centroid is the mean of its corners"
        );
    }

    /// Winding is a spelling choice: the same ring written the other way round
    /// has the same centroid, because each ring is normalized by the sign of its
    /// own area before it is combined.
    #[test]
    fn the_areal_centroid_does_not_depend_on_ring_winding() {
        let counter_clockwise = polygon(&[UNIT_SQUARE]);
        let clockwise = polygon(&[&[(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]]);
        assert_eq!(
            centroid(&counter_clockwise),
            centroid(&clockwise),
            "the two windings must agree"
        );
    }

    #[test]
    fn a_hole_pulls_the_areal_centroid_away_from_it() {
        // A 4x4 square, area 16, centroid (2,2); minus a 1x1 hole spanning
        // x in [2,3] and y in [2,3], area 1, centroid (5/2, 5/2).
        // Cx = (16*2 - 1*(5/2)) / 15 = (59/2)/15 = 59/30.
        let with_hole = polygon(&[
            &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
            &[(2, 2), (3, 2), (3, 3), (2, 3), (2, 2)],
        ]);
        assert_eq!(
            centroid(&with_hole),
            Some(Coord::xy(ratio(59, 30), ratio(59, 30))),
            "the hole subtracts both its area and its moment"
        );
    }

    #[test]
    fn a_linear_centroid_is_length_weighted() {
        assert_eq!(
            centroid(&line(&[(0, 0), (2, 0)])),
            Some(Coord::xy(r(1), Rat::zero())),
            "one segment's centroid is its midpoint"
        );
        assert_eq!(
            centroid(&line(&[(0, 0), (1, 0), (1, 1)])),
            Some(Coord::xy(ratio(3, 4), ratio(1, 4))),
            "two equal unit segments, midpoints (1/2,0) and (1,1/2)"
        );
        // Unequal weights: a length-3 segment and a length-1 segment.
        // Cx = (3*(3/2) + 1*(7/2))/4 = (9/2 + 7/2)/4 = 2.
        assert_eq!(
            centroid(&line(&[(0, 0), (3, 0), (4, 0)])),
            Some(Coord::xy(r(2), Rat::zero())),
            "the longer segment pulls harder, which a plain vertex mean would miss"
        );
    }

    #[test]
    fn a_point_centroid_is_the_arithmetic_mean() {
        assert_eq!(
            centroid(&multipoint(&[(0, 0), (2, 4)])),
            Some(Coord::xy(r(1), r(2))),
            "a two-point multipoint sits halfway between them"
        );
        assert_eq!(
            centroid(&point(5, 6)),
            Some(Coord::xy(r(5), r(6))),
            "a point is its own centroid"
        );
        assert_eq!(
            centroid(&multipoint(&[(0, 0), (1, 0), (2, 0)])),
            Some(Coord::xy(r(1), Rat::zero())),
            "three points, equally weighted"
        );
    }

    /// The highest dimension present wins, so a point beside a polygon does not
    /// move the polygon's centroid at all.
    #[test]
    fn the_highest_dimension_present_decides_the_centroid() {
        let areal_and_more = collection(vec![
            polygon(&[UNIT_SQUARE]),
            point(1000, 1000),
            line(&[(500, 500), (600, 600)]),
        ]);
        assert_eq!(
            centroid(&areal_and_more),
            Some(Coord::xy(ratio(1, 2), ratio(1, 2))),
            "the areal component alone decides it"
        );
        let linear_and_points = collection(vec![line(&[(0, 0), (2, 0)]), point(1000, 1000)]);
        assert_eq!(
            centroid(&linear_and_points),
            Some(Coord::xy(r(1), Rat::zero())),
            "and with no area, the curve decides it"
        );
    }

    /// A zero-area polygon has no area to weight by, so it falls to the linear
    /// case rather than having no centroid at all.
    #[test]
    fn a_degenerate_geometry_still_has_a_centroid() {
        let flat = polygon(&[&[(0, 0), (2, 0), (4, 0), (0, 0)]]);
        assert!(
            centroid(&flat).is_some(),
            "a collapsed ring still has rings with length"
        );
        assert_eq!(
            centroid(&line(&[(3, 4), (3, 4)])),
            Some(Coord::xy(r(3), r(4))),
            "a curve written as one repeated position is at that position"
        );
    }

    #[test]
    fn only_an_empty_geometry_has_no_centroid() {
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            assert_eq!(
                centroid(&Geometry::empty(CoordDim::Xy, kind)),
                None,
                "{kind:?} EMPTY has no position"
            );
        }
        assert!(
            centroid(&point(0, 0)).is_some(),
            "and the neighbouring non-empty case does have one"
        );
    }

    #[test]
    fn every_constructed_output_is_planar() {
        let raised = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::MultiPoint(vec![
                Some(Coord::new(r(0), r(0), Some(r(1)), None)),
                Some(Coord::new(r(4), r(0), Some(r(2)), None)),
                Some(Coord::new(r(4), r(4), Some(r(3)), None)),
                Some(Coord::new(r(0), r(4), Some(r(4)), None)),
            ]),
        )
        .expect("a well-formed 3D multipoint");
        for built in [boundary(&raised), envelope(&raised), convex_hull(&raised)] {
            assert_eq!(built.dim(), CoordDim::Xy, "Clause 10.2 projects onto Z=0");
            assert!(
                built.coords().all(|c| c.z().is_none() && c.m().is_none()),
                "so no ordinate beyond x and y survives"
            );
        }
        assert_eq!(
            centroid(&raised),
            Some(Coord::xy(r(2), r(2))),
            "and the centroid is planar too"
        );
    }

    // ---- over-refusal control -------------------------------------------

    /// None of these constructors may refuse a well-formed geometry.
    /// `boundary`, `envelope` and `convex_hull` are total — they have no failure
    /// channel at all — and `centroid` returns `None` exactly for the empty
    /// geometry and nothing else. This is the assertion that keeps a future
    /// "guard" from quietly turning a degenerate input into a refusal.
    #[test]
    fn the_constructors_answer_for_every_well_formed_geometry_including_degenerate_ones() {
        let mut corpus = vec![
            point(0, 0),
            multipoint(&[(1, 1), (1, 1)]),
            line(&[(0, 0), (0, 0)]),
            line(&[(0, 0), (1, 1), (2, 2)]),
            line(UNIT_SQUARE),
            polygon(&[UNIT_SQUARE]),
            polygon(&[&[(0, 0), (1, 0), (2, 0), (0, 0)]]),
            polygon(&[
                &[(0, 0), (4, 0), (4, 4), (0, 4), (0, 0)],
                &[(1, 1), (3, 1), (3, 3), (1, 3), (1, 1)],
            ]),
            multipolygon(&[&[UNIT_SQUARE]]),
            multiline(&[&[(0, 0), (1, 1)]]),
            collection(vec![point(0, 0), line(&[(1, 1), (2, 2)])]),
            Geometry::new(CoordDim::Xy, GeometryBody::MultiPoint(vec![None]))
                .expect("MULTIPOINT(EMPTY)"),
        ];
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            corpus.push(Geometry::empty(CoordDim::Xy, kind));
        }

        for g in &corpus {
            assert_eq!(
                boundary(g).dim(),
                CoordDim::Xy,
                "boundary answers, planar: {:?}",
                g.kind()
            );
            let e = envelope(g);
            assert_eq!(e.dim(), CoordDim::Xy, "envelope answers, planar");
            assert_eq!(
                e.is_empty(),
                g.is_empty(),
                "an envelope is empty exactly when its input is: {:?}",
                g.kind()
            );
            let h = convex_hull(g);
            assert_eq!(h.dim(), CoordDim::Xy, "convex_hull answers, planar");
            assert_eq!(
                h.is_empty(),
                g.is_empty(),
                "a hull is empty exactly when its input is: {:?}",
                g.kind()
            );
            assert_eq!(
                centroid(g).is_some(),
                !g.is_empty(),
                "a centroid exists exactly when the geometry is non-empty: {:?}",
                g.kind()
            );
        }
    }
}

/// The boundary's walk over nested collections, against a recursive reference on
/// generated trees and a hundred thousand levels deep.
#[cfg(test)]
mod nesting_tests {
    use super::{
        Arithmetic, boundary, centroid, convex_hull, envelope, member_boundary, planar, resident,
    };
    use crate::exact::Rat;
    use crate::geom::arbitrary::{self, Lcg};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody};
    use crate::wkt;

    /// The recursive boundary: a collection's is the collection of its members'.
    fn reference_boundary(geometry: &Geometry) -> Geometry {
        match geometry.body() {
            GeometryBody::GeometryCollection(members) => planar(
                GeometryBody::GeometryCollection(members.iter().map(reference_boundary).collect()),
                &Arithmetic::resident(),
            )
            .unwrap(),
            _ => resident(|memory| member_boundary(geometry, memory, &Arithmetic::resident())),
        }
    }

    fn triangle() -> Geometry {
        let ring: CoordSeq = [(0, 0), (1, 0), (1, 1), (0, 0)]
            .iter()
            .map(|&(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
            .collect();
        Geometry::new(CoordDim::Xy, GeometryBody::Polygon(vec![ring])).expect("a closed ring")
    }

    /// Over generated planar and elevated trees the fold and the recursive
    /// reference build the same boundary.
    #[test]
    fn the_boundary_agrees_with_the_recursive_reference_on_generated_trees() {
        let mut rng = Lcg::new(0x5eed_6001);
        for round in 0..300 {
            let dim = if round % 2 == 0 {
                CoordDim::Xy
            } else {
                CoordDim::Xyz
            };
            let tree = arbitrary::geometry(&mut rng, dim, 4);
            assert_eq!(
                boundary(&tree),
                reference_boundary(&tree),
                "round {round}: {tree:?}"
            );
        }
    }

    /// The boundary of a triangle nested a hundred thousand collections deep is its
    /// ring as a `MULTILINESTRING`, nested as deep, written on a 128 KiB stack; the
    /// expected text is a closed form pinned at depths one and two by literal
    /// strings, and the envelope, hull and centroid see through the nesting.
    #[test]
    fn a_hundred_thousand_deep_collection_has_its_boundary_envelope_hull_and_centroid() {
        const OPEN: &str = "GEOMETRYCOLLECTION(";
        const INNER: &str = "MULTILINESTRING((0 0,1 0,1 1,0 0))";
        const ONE: &str = "GEOMETRYCOLLECTION(MULTILINESTRING((0 0,1 0,1 1,0 0)))";
        const TWO: &str =
            "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(MULTILINESTRING((0 0,1 0,1 1,0 0))))";
        assert_eq!(format!("{OPEN}{INNER})"), ONE);
        assert_eq!(format!("{OPEN}{OPEN}{INNER}))"), TWO);
        let triangle = triangle();
        assert_eq!(wkt::write_bare(&boundary(&triangle), 0), INNER);
        assert_eq!(
            wkt::write_bare(&boundary(&arbitrary::nest(triangle.clone(), 1)), 0),
            ONE
        );
        assert_eq!(
            wkt::write_bare(&boundary(&arbitrary::nest(triangle.clone(), 2)), 0),
            TWO
        );
        let depth = arbitrary::DEEP;
        let mut expected = String::with_capacity(OPEN.len() * depth + INNER.len() + depth);
        for _ in 0..depth {
            expected.push_str(OPEN);
        }
        expected.push_str(INNER);
        for _ in 0..depth {
            expected.push(')');
        }

        purrdf_stack::on_stack(arbitrary::SMALL_STACK, move || {
            let deep = arbitrary::nest(triangle.clone(), depth);
            assert_eq!(wkt::write_bare(&boundary(&deep), 0), expected);
            assert_eq!(envelope(&deep), envelope(&triangle));
            assert_eq!(convex_hull(&deep), convex_hull(&triangle));
            assert_eq!(centroid(&deep), centroid(&triangle));
        })
        .expect("the thread starts");
    }
}
