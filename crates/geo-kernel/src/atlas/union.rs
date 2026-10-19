// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical rings of an exact, labelled source-coordinate atlas arrangement.
//! Complete strips have interior on their left. Split every vertical shared
//! wall at all exact junctions, cancel opposite walls, then trace the predecessor
//! of each reversed incoming ray. Exact angular order preserves point contacts,
//! holes, complements and chart cuts without perturbing a coordinate.

use super::arrangement::AreaStrip;
use crate::context::{Produced, RetainedBuilder, WorkProgress};
use crate::numerical::ExactAdmission;
use crate::{
    Coord, CoordDim, GeoError, Geometry, GeometryBody, LonLat, MetricContext, PreparedRegion, Set,
};
use core::alloc::Layout;
use core::cmp::Ordering;
use purrdf_lex::allocation::StorageError;
use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};

struct Edge {
    start: Coord,
    end: Coord,
}
crate::context::retained::native_detached_fields!(Edge {
    native: [start, end],
    copy: []
});
type NativeRing = RetainedBuilder<Coord>;
type NativePolygon = RetainedBuilder<NativeRing>;

#[cfg(test)]
pub(crate) fn materialize(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Geometry, GeoError> {
    // This oracle returns a caller-owned resident geometry. Production moves
    // its original account into the enclosing immutable output before extraction.
    materialize_with_output(region, context, progress, |geometry, _, _, _| {
        let (geometry, _, original) = geometry.into_parts();
        drop(original);
        Ok((geometry, 0))
    })
}

/// Admit the completed native carrier while its producer still owns every
/// coordinate and arrangement reservation. The recipient keeps that separate
/// output allowance after this function releases its temporary workspace. The
/// callback returns how many already admitted bytes it adopted; the producer
/// removes those bytes from its local counter without releasing the allowance.
pub(crate) fn materialize_with_output(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    mut retain_output: impl FnMut(
        Produced<Geometry>,
        u64,
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<(Geometry, u64), GeoError>,
) -> Result<Geometry, GeoError> {
    let arrangement = super::arrangement::area_arrangement_for_union(region, context, progress)?;
    let mut retained = arrangement.workspace_bytes;
    let result = (|| {
        let mut edges = RetainedBuilder::default();
        for strip in &arrangement.strips {
            let AreaStrip { lower, upper } = strip;
            for (a, b) in [
                (&lower[0], &lower[1]),
                (&lower[1], &upper[1]),
                (&upper[1], &upper[0]),
                (&upper[0], &lower[0]),
            ] {
                step(context, progress)?;
                if ExactAdmission::new(context, progress).expression_result(
                    RationalCompare,
                    &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
                    2,
                    |values| {
                        Ok(values.compare(a.longitude(), b.longitude())?.is_eq()
                            && values.compare(a.latitude(), b.latitude())?.is_eq())
                    },
                )? {
                    continue;
                }
                let start = xy(a, context, progress)?;
                let end = xy(b, context, progress)?;
                ExactAdmission::new(context, progress).push_detached(
                    &mut edges,
                    Edge { start, end },
                    &mut retained,
                )?;
            }
        }
        // Descendant numerical fields share their original immutable accounts.
        // Only this dead buffer/control is refunded; its scalar allowance stays
        // together until the completed output adopts the original receipts.
        let arrangement_slots = u64::try_from(arrangement.strips.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        drop(arrangement);
        context.release_retained_workspace(arrangement_slots, &mut retained)?;
        // A wall only meets endpoints with its exact original x. Keep borrowed
        // keys so indexing never clones coordinates or changes atom selection.
        let endpoint_count = edges
            .len()
            .checked_mul(2)
            .ok_or(GeoError::ArithmeticOverflow("union endpoint count"))?;
        let mut endpoints = RetainedBuilder::default();
        endpoints.reserve_admitted(endpoint_count, context, progress, &mut retained)?;
        for edge in &edges {
            for point in [&edge.start, &edge.end] {
                step(context, progress)?;
                endpoints.push_reserved(point);
            }
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut endpoints, |a, b| {
            compare(a, b, context, progress)
        })?;
        let mut atomized = RetainedBuilder::default();
        for edge in &edges {
            step(context, progress)?;
            if ExactAdmission::new(context, progress).expression_result(
                RationalCompare,
                &[edge.start.x(), edge.end.x()],
                1,
                |values| Ok(!values.compare(edge.start.x(), edge.end.x())?.is_eq()),
            )? {
                let copied = ExactAdmission::new(context, progress).expression_result(
                    Linear,
                    &[edge.start.x(), edge.start.y(), edge.end.x(), edge.end.y()],
                    4,
                    |values| {
                        Ok(Edge {
                            start: values.coordinate(&edge.start)?,
                            end: values.coordinate(&edge.end)?,
                        })
                    },
                )?;
                ExactAdmission::new(context, progress).push_detached(
                    &mut atomized,
                    copied,
                    &mut retained,
                )?;
                continue;
            }
            let ascending = ExactAdmission::new(context, progress).expression_result(
                RationalCompare,
                &[edge.start.y(), edge.end.y()],
                1,
                |values| Ok(values.compare(edge.start.y(), edge.end.y())?.is_lt()),
            )?;
            let (low, high) = if ascending {
                (edge.start.y(), edge.end.y())
            } else {
                (edge.end.y(), edge.start.y())
            };
            let mut cuts = RetainedBuilder::default();
            let aligned = purrdf_lex::walk::try_equal_range_by(&endpoints, |point| {
                crate::numerical::compare_rat(point.x(), edge.start.x(), context, progress)
            })?;
            for point in &endpoints[aligned] {
                step(context, progress)?;
                if ExactAdmission::new(context, progress).expression_result(
                    RationalCompare,
                    &[low, point.y(), high],
                    2,
                    |values| {
                        Ok(values.compare(low, point.y())?.is_le()
                            && values.compare(point.y(), high)?.is_le())
                    },
                )? {
                    cuts.push_admitted(point.y(), context, progress, &mut retained)?;
                }
            }
            // The exact-x endpoint run is already sorted by y, and filtering
            // retains that order. Repeated endpoints still form zero atoms.
            for pair in cuts.windows(2) {
                if ExactAdmission::new(context, progress).expression_result(
                    RationalCompare,
                    &[pair[0], pair[1]],
                    1,
                    |values| Ok(values.compare(pair[0], pair[1])?.is_eq()),
                )? {
                    continue;
                }
                let (a, b) = if ascending {
                    (pair[0], pair[1])
                } else {
                    (pair[1], pair[0])
                };
                let copied = ExactAdmission::new(context, progress).expression_result(
                    Linear,
                    &[edge.start.x(), a, b],
                    4,
                    |values| {
                        Ok(Edge {
                            start: Coord::xy(values.original(edge.start.x())?, values.original(a)?),
                            end: Coord::xy(values.original(edge.start.x())?, values.original(b)?),
                        })
                    },
                )?;
                ExactAdmission::new(context, progress).push_detached(
                    &mut atomized,
                    copied,
                    &mut retained,
                )?;
            }
            // Only this edge's cuts remain live while its atoms are copied.
            // An error leaves their charge in `retained` for outer cleanup.
            cuts.release_admitted(context, &mut retained)?;
        }
        endpoints.release_admitted(context, &mut retained)?;
        edges.release_admitted(context, &mut retained)?;
        purrdf_lex::walk::try_sort_unstable_by(&mut atomized, |a, b| {
            step(context, progress)?;
            let (a0, a1) = ordered(a, context, progress)?;
            let (b0, b1) = ordered(b, context, progress)?;
            let order = compare(a0, b0, context, progress)?;
            if order.is_eq() {
                compare(a1, b1, context, progress)
            } else {
                Ok(order)
            }
        })?;
        let atomized_slots = u64::try_from(atomized.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let mut boundary = RetainedBuilder::default();
        let mut pending = atomized.into_iter().peekable();
        while let Some(edge) = pending.next() {
            step(context, progress)?;
            let positive = compare(&edge.start, &edge.end, context, progress)?.is_lt();
            let mut opposite = false;
            while let Some(other) = pending.peek() {
                let (a0, a1) = ordered(&edge, context, progress)?;
                let (b0, b1) = ordered(other, context, progress)?;
                if !same_planar(a0, b0, context, progress)?
                    || !same_planar(a1, b1, context, progress)?
                {
                    break;
                }
                step(context, progress)?;
                let other = pending.next().expect("peeked edge");
                opposite |=
                    compare(&other.start, &other.end, context, progress)?.is_lt() != positive;
            }
            if !opposite {
                boundary.push_admitted(edge, context, progress, &mut retained)?;
            }
        }
        drop(pending);
        context.release_retained_workspace(atomized_slots, &mut retained)?;
        // Equal starts preserve the previous boundary-index iteration order,
        // retaining every original angular tie decision at a shared junction.
        context.charge_work(boundary.len() as u64)?;
        progress.context_poll(context)?;
        let mut starts = RetainedBuilder::default();
        starts.reserve_admitted(boundary.len(), context, progress, &mut retained)?;
        for index in 0..boundary.len() {
            starts.push_reserved(index);
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut starts, |a, b| -> Result<_, GeoError> {
            let order = compare(&boundary[*a].start, &boundary[*b].start, context, progress)?;
            Ok(order.then_with(|| a.cmp(b)))
        })?;
        let mut used = RetainedBuilder::default();
        used.reserve_admitted(boundary.len(), context, progress, &mut retained)?;
        for _ in 0..boundary.len() {
            used.push_reserved(false);
        }
        let mut rings = RetainedBuilder::default();
        for first in 0..boundary.len() {
            if used[first] {
                continue;
            }
            let mut index = first;
            let mut ring = RetainedBuilder::default();
            loop {
                step(context, progress)?;
                if used[index] {
                    return Err(GeoError::domain("atlas union boundary did not close"));
                }
                used[index] = true;
                let edge = &boundary[index];
                let copied = ExactAdmission::new(context, progress).expression_result(
                    Linear,
                    &[edge.start.x(), edge.start.y()],
                    2,
                    |values| Ok(values.coordinate(&edge.start)?),
                )?;
                ExactAdmission::new(context, progress).push_detached(
                    &mut ring,
                    copied,
                    &mut retained,
                )?;
                let mut previous: Option<usize> = None;
                let mut wrapped: Option<usize> = None;
                let successors = purrdf_lex::walk::try_equal_range_by(&starts, |candidate| {
                    compare(&boundary[*candidate].start, &edge.end, context, progress)
                })?;
                for &candidate in &starts[successors] {
                    step(context, progress)?;
                    let next = &boundary[candidate];
                    let wraps = if let Some(old) = wrapped {
                        angle(&edge.end, &boundary[old].end, &next.end, context, progress)?.is_lt()
                    } else {
                        true
                    };
                    if wraps {
                        wrapped = Some(candidate);
                    }
                    if !angle(&edge.end, &next.end, &edge.start, context, progress)?.is_gt() {
                        let closer = if let Some(old) = previous {
                            angle(&edge.end, &boundary[old].end, &next.end, context, progress)?
                                .is_lt()
                        } else {
                            true
                        };
                        if closer {
                            previous = Some(candidate);
                        }
                    }
                }
                index = previous
                    .or(wrapped)
                    .ok_or_else(|| GeoError::domain("atlas union has an open boundary"))?;
                if index == first {
                    break;
                }
            }
            if ring.len() < 3 {
                ring.release_admitted(context, &mut retained)?;
                continue;
            }
            let mut first = 0;
            for index in 1..ring.len() {
                if compare(&ring[index], &ring[first], context, progress)?.is_lt() {
                    first = index;
                }
            }
            ring.rotate_left(first);
            // Closing the ring shares its already published native scalars.
            let closing = ring[0].clone();
            ring.push_admitted(closing, context, progress, &mut retained)?;
            rings.push_admitted(ring, context, progress, &mut retained)?;
        }
        starts.release_admitted(context, &mut retained)?;
        boundary.release_admitted(context, &mut retained)?;
        used.release_admitted(context, &mut retained)?;
        let ring_slots = u64::try_from(rings.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let mut polygons: RetainedBuilder<NativePolygon> = RetainedBuilder::default();
        let mut outer_areas = RetainedBuilder::default();
        let mut area_bytes = 0_u64;
        let mut holes = RetainedBuilder::default();
        for ring in rings {
            step(context, progress)?;
            let area_start = retained;
            let area = crate::measure::signed_ring_area_retained(
                &ring,
                &mut ExactAdmission::new(context, progress),
                &mut retained,
            )?;
            let bytes = retained - area_start;
            match area.signum().cmp(&0) {
                Ordering::Greater => {
                    let mut polygon = RetainedBuilder::default();
                    polygon.push_admitted(ring, context, progress, &mut retained)?;
                    polygons.push_admitted(polygon, context, progress, &mut retained)?;
                    outer_areas.push_admitted(area, context, progress, &mut retained)?;
                    area_bytes = area_bytes
                        .checked_add(bytes)
                        .ok_or(GeoError::ArithmeticOverflow("union retained ring areas"))?;
                }
                Ordering::Less => {
                    holes.push_admitted(ring, context, progress, &mut retained)?;
                    drop(area);
                    context.release_retained_workspace(bytes, &mut retained)?;
                }
                Ordering::Equal => {
                    ring.release_admitted(context, &mut retained)?;
                    drop(area);
                    context.release_retained_workspace(bytes, &mut retained)?;
                }
            }
        }
        context.release_retained_workspace(ring_slots, &mut retained)?;
        let hole_slots = u64::try_from(holes.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        for hole in holes {
            let mut owner: Option<usize> = None;
            for (index, polygon) in polygons.iter().enumerate() {
                let located = crate::topology::locate_surface_admitted(
                    &hole[0],
                    polygon,
                    &mut ExactAdmission::new(context, progress),
                )?;
                if located != Set::Exterior {
                    let smaller = if let Some(old) = owner {
                        ExactAdmission::new(context, progress).expression_result(
                            RationalCompare,
                            &[&outer_areas[index], &outer_areas[old]],
                            1,
                            |values| {
                                Ok(values
                                    .compare(&(outer_areas[index]), &(outer_areas[old]))?
                                    .is_lt())
                            },
                        )?
                    } else {
                        true
                    };
                    if smaller {
                        owner = Some(index);
                    }
                }
            }
            polygons
                [owner.ok_or_else(|| GeoError::domain("atlas union hole has no exterior ring"))?]
            .push_admitted(hole, context, progress, &mut retained)?;
        }
        context.release_retained_workspace(hole_slots, &mut retained)?;
        outer_areas.release_admitted(context, &mut retained)?;
        context.release_retained_workspace(area_bytes, &mut retained)?;
        let mut vertices = 0u64;
        let mut validation = purrdf_xsd::integer::ExactArithmeticCost {
            work_items: polygons.len() as u64,
            workspace_bytes: 0,
            output_bits: 0,
        };
        for polygon in &mut polygons {
            step(context, progress)?;
            purrdf_lex::walk::try_sort_unstable_by(&mut polygon[1..], |a, b| {
                step(context, progress)?;
                compare_rings(a, b, context, progress)
            })?;
            validation.work_items = validation
                .work_items
                .checked_add(polygon.len() as u64)
                .ok_or(GeoError::ArithmeticOverflow("union carrier validation"))?;
            for ring in polygon {
                step(context, progress)?;
                let count = ring.len() as u64;
                vertices = vertices
                    .checked_add(count)
                    .ok_or(GeoError::ArithmeticOverflow("union output coordinates"))?;
                validation.work_items = validation
                    .work_items
                    .checked_add(count)
                    .ok_or(GeoError::ArithmeticOverflow("union carrier validation"))?;
                let first = &ring[0];
                let last = &ring[ring.len() - 1];
                let closure = crate::numerical::rational_cost(
                    Linear,
                    &[first.x(), first.y(), last.x(), last.y()],
                    4,
                )
                .ok_or(GeoError::ArithmeticOverflow("union closure validation"))?;
                validation = validation
                    .followed_by(closure)
                    .ok_or(GeoError::ArithmeticOverflow("union carrier validation"))?;
            }
        }
        if vertices > context.policy().limits().max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: context.policy().limits().max_output_elements,
            });
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut polygons, |a, b| {
            step(context, progress)?;
            compare_rings(&a[0], &b[0], context, progress)
        })?;
        let mut buffers = array_bytes::<crate::Rings>(polygons.len())?;
        let mut dead_metadata = u64::try_from(polygons.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        for polygon in &polygons {
            buffers = buffers
                .checked_add(array_bytes::<crate::CoordSeq>(polygon.len())?)
                .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
            dead_metadata = dead_metadata
                .checked_add(
                    u64::try_from(polygon.admitted_bytes())
                        .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?,
                )
                .ok_or(GeoError::ArithmeticOverflow(
                    "union working polygon controls",
                ))?;
            for ring in polygon {
                buffers = buffers
                    .checked_add(array_bytes::<Coord>(ring.len())?)
                    .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
                dead_metadata = dead_metadata
                    .checked_add(
                        u64::try_from(ring.admitted_bytes())
                            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?,
                    )
                    .ok_or(GeoError::ArithmeticOverflow("union working ring controls"))?;
            }
        }
        let output_layout = buffers
            .checked_add(progress.retained_owner_layout().size())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        context.retain_workspace(output_layout, &mut retained)?;
        progress.context_poll(context)?;
        // Reuse the carrier home's dimensional/closure validation. Its complete
        // structural scan and original-limb equality work are admitted before
        // that body runs; the builder does not implement a second validator.
        progress.exact(context, validation, || Ok(()))?;
        let geometry = Produced::produce(
            progress,
            Layout::new::<()>(),
            |memory| {
                let mut output = Vec::new();
                memory
                    .reserve(&mut output, polygons.len())
                    .map_err(GeoError::Storage)?;
                for polygon in &polygons {
                    let mut rings = Vec::new();
                    memory
                        .reserve(&mut rings, polygon.len())
                        .map_err(GeoError::Storage)?;
                    for ring in polygon {
                        let mut coordinates = Vec::new();
                        memory
                            .reserve(&mut coordinates, ring.len())
                            .map_err(GeoError::Storage)?;
                        // All cells have already been published through the native
                        // detached home. These clones share original scalar owners.
                        for coordinate in ring {
                            coordinates.push(coordinate.clone());
                        }
                        rings.push(coordinates);
                    }
                    output.push(rings);
                }
                Geometry::new(CoordDim::Xy, GeometryBody::MultiPolygon(output))
            },
            |_, _| Ok(()),
        )?;
        if u64::try_from(geometry.admitted_bytes()).ok() != Some(output_layout) {
            return Err(GeoError::ArithmeticOverflow(
                "union native output layout certificate",
            ));
        }
        drop(polygons);
        context.release_retained_workspace(dead_metadata, &mut retained)?;
        // Keep original numerical allowances together while descendant fields
        // share those accounts. The amount comes from actual producer receipts,
        // never from a census or a coordinate-width multiplier.
        let available = retained;
        let (geometry, transferred) = retain_output(geometry, available, context, progress)?;
        if transferred > available {
            return Err(GeoError::ArithmeticOverflow(
                "union output transfer exceeds owner",
            ));
        }
        retained = retained
            .checked_sub(transferred)
            .ok_or(GeoError::ArithmeticOverflow("union output transfer"))?;
        Ok(geometry)
    })();
    progress.settle_result(result, context.release_workspace(retained))
}
fn array_bytes<T>(length: usize) -> Result<usize, GeoError> {
    Layout::array::<T>(length)
        .map(|layout| layout.size())
        .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))
}
fn step(context: &mut MetricContext, progress: &mut WorkProgress<'_>) -> Result<(), GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)
}
fn xy(
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Coord, GeoError> {
    ExactAdmission::new(context, progress).expression_result(
        Linear,
        &[point.longitude(), point.latitude()],
        2,
        |values| Ok(values.planar_location(point)?),
    )
}
fn compare(
    a: &Coord,
    b: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Ordering, GeoError> {
    crate::topology::compare_inner(a, b, Some(&mut ExactAdmission::new(context, progress)))
}
fn same_planar(
    a: &Coord,
    b: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    crate::topology::same_planar_inner(a, b, Some(&mut ExactAdmission::new(context, progress)))
}
fn ordered<'a>(
    edge: &'a Edge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(&'a Coord, &'a Coord), GeoError> {
    if compare(&edge.start, &edge.end, context, progress)?.is_lt() {
        Ok((&edge.start, &edge.end))
    } else {
        Ok((&edge.end, &edge.start))
    }
}
fn angle(
    origin: &Coord,
    a: &Coord,
    b: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Ordering, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let half = admission.expression_result(
        RationalCompare,
        &[origin.x(), origin.y(), a.x(), a.y(), b.x(), b.y()],
        6,
        |values| {
            let half = |point: &Coord| -> Result<usize, GeoError> {
                let latitude = values.compare(point.y(), origin.y())?;
                Ok(usize::from(
                    latitude.is_lt()
                        || (latitude.is_eq() && values.compare(point.x(), origin.x())?.is_lt()),
                ))
            };
            Ok(half(a)?.cmp(&half(b)?))
        },
    )?;
    if half.is_eq() {
        Ok(
            crate::topology::orientation_admitted(origin, a, b, &mut admission)?
                .cmp(&0)
                .reverse(),
        )
    } else {
        Ok(half)
    }
}
fn compare_rings(
    a: &[Coord],
    b: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Ordering, GeoError> {
    for (a, b) in a.iter().zip(b) {
        let order = compare(a, b, context, progress)?;
        if !order.is_eq() {
            return Ok(order);
        }
    }
    Ok(a.len().cmp(&b.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rat;
    use crate::{
        Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference, PreparedGeometry,
    };
    use core::fmt::Write as _;

    #[test]
    fn complete_union_transfers_actual_output_and_preserves_live_siblings_on_refusal() {
        use crate::carrier::MaterializationStorage;
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse(
            "MULTIPOLYGON (((0 0,1 0,1 1,0 1,0 0)),((1 0,2 0,2 1,1 1,1 0)))",
            &crs,
        )
        .unwrap();
        let source = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.begin_integer(1).unwrap();
        let mut storage = MaterializationStorage::default();
        let mut progress = WorkProgress::integer(None);
        let output = materialize_with_output(
            source.region(),
            &mut context,
            &mut progress,
            |geometry, available, context, progress| {
                storage.adopt_produced_geometry_output(
                    geometry,
                    available,
                    context,
                    progress,
                    |count, _| {
                        assert_eq!(count, 7);
                        Ok(())
                    },
                )
            },
        )
        .unwrap();
        assert_eq!(context.current_workspace_bytes(), storage.output_bytes());
        let literal = storage
            .publish_literal(
                crate::GeometryLiteral::new(crs, output),
                &mut context,
                &mut progress,
            )
            .unwrap();
        let (output, receipt) = storage
            .finish_result(Ok(literal), &mut context, &mut progress)
            .unwrap();
        let live = context.current_workspace_bytes();
        context.begin_integer(1).unwrap();
        assert_eq!(context.current_workspace_bytes(), live);

        let mut refused_storage = MaterializationStorage::default();
        let mut counted = false;
        let result = materialize_with_output(
            source.region(),
            &mut context,
            &mut progress,
            |geometry, available, context, progress| {
                refused_storage.adopt_produced_geometry_output(
                    geometry,
                    available,
                    context,
                    progress,
                    |count, _| {
                        counted = true;
                        assert_eq!(count, 7);
                        Err(GeoError::OutputExhausted { limit: 0 })
                    },
                )
            },
        );
        assert!(counted);
        assert!(matches!(
            result,
            Err(GeoError::OutputExhausted { limit: 0 })
        ));
        assert_eq!(refused_storage.output_bytes(), 0);
        assert_eq!(context.current_workspace_bytes(), live);
        assert!(
            refused_storage
                .finish(false, &mut context, &mut progress)
                .unwrap()
                .is_none()
        );
        drop(output);
        context.release_materialized_output(receipt).unwrap();
        assert_eq!(context.current_workspace_bytes(), 0);
    }

    #[test]
    fn union_angular_order_admits_original_large_rationals_before_comparing() {
        let tiny = Rat::new(crate::Int::one(), crate::Int::one().shl(65_536)).unwrap();
        let origin = Coord::xy(Rat::zero(), Rat::zero());
        let a = Coord::xy(tiny, Rat::one());
        let b = Coord::xy(Rat::one(), Rat::one());
        for (work, bytes, memory) in [
            (100_000, 64 * 1024 * 1024, false),
            (1_000_000_000_000, 8192, true),
        ] {
            let limits = ExecutionLimits {
                max_work_items: work,
                max_workspace_bytes: bytes,
                ..ExecutionLimits::GEOMETRY
            };
            let mut context = MetricContext::new(
                GeographicReference::wgs84(),
                ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            context.begin(1).unwrap();
            let baseline = context.remaining_workspace();
            let result = angle(&origin, &a, &b, &mut context, &mut WorkProgress::new(None));
            if memory {
                assert!(matches!(result, Err(GeoError::MemoryExhausted { .. })));
            } else {
                assert!(matches!(result, Err(GeoError::WorkExhausted { .. })));
            }
            assert_eq!(context.remaining_workspace(), baseline);
        }

        struct Cancel;
        impl crate::MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 1_000_000_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        context.begin(1).unwrap();
        let baseline = context.remaining_workspace();
        assert!(matches!(
            angle(
                &origin,
                &a,
                &b,
                &mut context,
                &mut WorkProgress::new(Some(&mut Cancel)),
            ),
            Err(GeoError::Cancelled)
        ));
        assert_eq!(context.remaining_workspace(), baseline);
    }

    #[test]
    fn shared_wall_union_releases_temporary_cuts_and_refusal_workspace() {
        let mut text = String::from("MULTIPOLYGON (");
        for x in 0..24 {
            if x != 0 {
                text.push(',');
            }
            write!(&mut text, "(({x} 0,{} 0,{} 1,{x} 1,{x} 0))", x + 1, x + 1).unwrap();
        }
        text.push(')');
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse(&text, &crs).unwrap();
        let source = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 100_000_000;
        limits.max_workspace_bytes = 2 * 1024 * 1024;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        context.begin(1).unwrap();
        let result =
            materialize(source.region(), &mut context, &mut WorkProgress::new(None)).unwrap();
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            limits.max_workspace_bytes
        );
        let GeometryBody::MultiPolygon(polygons) = result.body() else {
            panic!("complete union polygons")
        };
        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 1);
        assert_eq!(
            crate::measure::signed_ring_area(&polygons[0][0]),
            Rat::from_i64(24)
        );

        struct CancelAtWork(u64);
        impl crate::MetricWorkObserver for CancelAtWork {
            fn charge_chunk(&mut self, work: u64, _: u64) -> Result<(), GeoError> {
                self.0 = self.0.saturating_sub(work);
                if self.0 == 0 {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        context.begin(1).unwrap();
        let mut observer = CancelAtWork(20_000);
        assert!(matches!(
            materialize(
                source.region(),
                &mut context,
                &mut WorkProgress::new(Some(&mut observer))
            ),
            Err(GeoError::Cancelled)
        ));
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            limits.max_workspace_bytes
        );
    }

    fn compare_source(text: &str, components: usize, holes: usize) {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse(text, &crs).unwrap();
        let source = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let result =
            materialize(source.region(), &mut context, &mut WorkProgress::new(None)).unwrap();
        let GeometryBody::MultiPolygon(polygons) = result.body() else {
            panic!("union polygons")
        };
        assert_eq!(polygons.len(), components);
        assert_eq!(
            polygons
                .iter()
                .map(|polygon| polygon.len() - 1)
                .sum::<usize>(),
            holes
        );
        for polygon in polygons {
            assert!(crate::measure::signed_ring_area(&polygon[0]) > Rat::zero());
            assert!(
                polygon[1..]
                    .iter()
                    .all(|hole| crate::measure::signed_ring_area(hole) < Rat::zero())
            );
        }
        let target =
            PreparedGeometry::from_literal(&crate::GeometryLiteral::new(crs, result), &profile)
                .unwrap();
        for x in -6..=6 {
            for y in -6..=6 {
                let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
                let point =
                    LonLat::new(Rat::from_i64(x).mul(&half), Rat::from_i64(y).mul(&half)).unwrap();
                assert_eq!(
                    super::super::locate(&point, source.region(), &mut context).unwrap(),
                    super::super::locate(&point, target.region(), &mut context).unwrap()
                );
            }
        }
    }
    #[test]
    fn coalesced_interior_runs_remove_only_artificial_collinear_wall_vertices() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        // The second rectangle lies inside the first and touches its west
        // boundary. Its y=1 and y=3 cuts are interior subdivision artifacts.
        let literal = crate::wkt::parse(
            "MULTIPOLYGON (((0 0,4 0,4 4,0 4,0 0)),((0 1,2 1,2 3,0 3,0 1)))",
            &crs,
        )
        .unwrap();
        let source = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let result =
            materialize(source.region(), &mut context, &mut WorkProgress::new(None)).unwrap();
        let GeometryBody::MultiPolygon(polygons) = result.body() else {
            panic!("complete coalesced carrier")
        };
        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 1);
        assert_eq!(
            polygons[0][0],
            [(0, 0), (2, 0), (4, 0), (4, 4), (2, 4), (0, 4), (0, 0)]
                .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
        );
        assert_eq!(
            crate::measure::signed_ring_area(&polygons[0][0]),
            Rat::from_i64(16)
        );
        assert_eq!(context.current_workspace_bytes(), 0);
        compare_source(
            "MULTIPOLYGON (((0 0,4 0,4 4,0 4,0 0)),((0 1,2 1,2 3,0 3,0 1)))",
            1,
            0,
        );
    }

    #[test]
    fn strips_stitch_overlaps_shared_edges_holes_and_point_contacts() {
        compare_source(
            "MULTIPOLYGON (((0 0,1 0,1 1,0 1,0 0)),((1 0,2 0,2 1,1 1,1 0)))",
            1,
            0,
        );
        compare_source(
            "MULTIPOLYGON (((0 0,2 0,2 2,0 2,0 0)),((1 1,3 1,3 3,1 3,1 1)))",
            1,
            0,
        );
        compare_source(
            "POLYGON ((-2 -2,2 -2,2 2,-2 2,-2 -2),(-1 -1,-1 1,1 1,1 -1,-1 -1))",
            1,
            1,
        );
        compare_source(
            "MULTIPOLYGON (((0 0,1 0,1 1,0 1,0 0)),((1 1,2 1,2 2,1 2,1 1)))",
            2,
            0,
        );
    }

    #[test]
    fn native_source_linear_parallel_materializes_its_complete_selected_cap() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let ring = crate::PreparedCurve::from_source(
            &[-180, 180].map(|x| Coord::xy(Rat::from_i64(x), Rat::from_i64(30))),
            &reference,
        )
        .unwrap();
        let polygon = crate::PreparedPolygon::from_curves(
            vec![ring],
            crate::OrientedInterior::Left,
            &mut context,
        )
        .unwrap();
        let source = PreparedRegion::polygons(vec![polygon]);
        for source in [&source, &source.clone().complement()] {
            context.begin(1).unwrap();
            let result = materialize(source, &mut context, &mut WorkProgress::new(None)).unwrap();
            let target = PreparedGeometry::from_literal(
                &crate::GeometryLiteral::new(crs.clone(), result),
                &profile,
            )
            .unwrap();
            for longitude in [-180, -90, 0, 90, 180] {
                for latitude in [-90, -45, 0, 29, 30, 31, 60, 90] {
                    let point =
                        LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
                    assert_eq!(
                        crate::atlas::locate(&point, source, &mut context).unwrap(),
                        crate::atlas::locate(&point, target.region(), &mut context).unwrap(),
                    );
                }
            }
        }
    }
}
