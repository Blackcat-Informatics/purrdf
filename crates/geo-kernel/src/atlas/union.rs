// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical rings of an exact, labelled source-coordinate atlas arrangement.
//! Complete strips have interior on their left. Split every vertical shared
//! wall at all exact junctions, cancel opposite walls, then trace the predecessor
//! of each reversed incoming ray. Exact angular order preserves point contacts,
//! holes, complements and chart cuts without perturbing a coordinate.

use super::arrangement::AreaStrip;
use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{
    Coord, CoordDim, GeoError, Geometry, GeometryBody, LonLat, MetricContext, PreparedRegion, Rat,
    Set,
};
use core::cmp::Ordering;
use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};

struct Edge {
    start: Coord,
    end: Coord,
}

#[cfg(test)]
pub(crate) fn materialize(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Geometry, GeoError> {
    materialize_with_output(region, context, progress, |_, _, _, _| Ok(0))
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
        &Geometry,
        u64,
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<u64, GeoError>,
) -> Result<Geometry, GeoError> {
    let arrangement = super::arrangement::area_arrangement_for_union(region, context, progress)?;
    let mut retained = arrangement.workspace_bytes;
    let arrangement_bytes = retained;
    let mut output_bytes = 0_u64;
    let result = (|| {
        let mut edges = Vec::new();
        for strip in &arrangement.strips {
            let AreaStrip { lower, upper } = strip;
            for (a, b) in [
                (&lower[0], &lower[1]),
                (&lower[1], &upper[1]),
                (&upper[1], &upper[0]),
                (&upper[0], &lower[0]),
            ] {
                step(context, progress)?;
                if ExactAdmission::new(context, progress).rational(
                    Linear,
                    &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
                    4,
                    || Ok(a == b),
                )? {
                    continue;
                }
                admit_ordinates([a.longitude(), a.latitude()], context, &mut retained)?;
                admit_ordinates([b.longitude(), b.latitude()], context, &mut retained)?;
                let start = xy(a);
                let end = xy(b);
                edges.push(Edge { start, end });
            }
        }
        let edge_bytes = retained
            .checked_sub(arrangement_bytes)
            .ok_or(GeoError::ArithmeticOverflow("union edge storage"))?;
        // The copied edges now own all coordinates needed by atomization.
        drop(arrangement);
        context.release_retained_workspace(arrangement_bytes, &mut retained)?;
        // A wall only meets endpoints with its exact original x. Keep borrowed
        // keys so indexing never clones coordinates or changes atom selection.
        let endpoint_bytes = (edges.len() as u64)
            .checked_mul((2 * size_of::<&Coord>()) as u64)
            .ok_or(GeoError::ArithmeticOverflow("union endpoint index"))?;
        context.retain_workspace(endpoint_bytes, &mut retained)?;
        progress.context_poll(context)?;
        let endpoint_count = edges
            .len()
            .checked_mul(2)
            .ok_or(GeoError::ArithmeticOverflow("union endpoint count"))?;
        let mut endpoints = Vec::with_capacity(endpoint_count);
        for edge in &edges {
            for point in [&edge.start, &edge.end] {
                step(context, progress)?;
                endpoints.push(point);
            }
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut endpoints, |a, b| {
            compare(a, b, context, progress)
        })?;
        let atomized_start = retained;
        let mut atomized = Vec::new();
        for edge in &edges {
            step(context, progress)?;
            if ExactAdmission::new(context, progress).rational(
                Linear,
                &[edge.start.x(), edge.end.x()],
                1,
                || Ok(edge.start.x() != edge.end.x()),
            )? {
                admit(&edge.start, context, &mut retained)?;
                admit(&edge.end, context, &mut retained)?;
                atomized.push(Edge {
                    start: edge.start.clone(),
                    end: edge.end.clone(),
                });
                continue;
            }
            let ascending = ExactAdmission::new(context, progress).rational(
                RationalCompare,
                &[edge.start.y(), edge.end.y()],
                1,
                || Ok(edge.start.y() < edge.end.y()),
            )?;
            let (low, high) = if ascending {
                (edge.start.y(), edge.end.y())
            } else {
                (edge.end.y(), edge.start.y())
            };
            let mut cuts = Vec::new();
            let mut cut_bytes = 0u64;
            let aligned = purrdf_lex::walk::try_equal_range_by(&endpoints, |point| {
                crate::numerical::compare_rat(point.x(), edge.start.x(), context, progress)
            })?;
            for point in &endpoints[aligned] {
                step(context, progress)?;
                if ExactAdmission::new(context, progress).rational(
                    RationalCompare,
                    &[low, point.y(), high],
                    2,
                    || Ok(low <= point.y() && point.y() <= high),
                )? {
                    cut_bytes = cut_bytes
                        .checked_add(admit(point, context, &mut retained)?)
                        .ok_or(GeoError::ArithmeticOverflow("union cut storage"))?;
                    cuts.push(point.y().clone());
                }
            }
            // The exact-x endpoint run is already sorted by y, and filtering
            // retains that order. Repeated endpoints still form zero atoms.
            for pair in cuts.windows(2) {
                if ExactAdmission::new(context, progress).rational(
                    Linear,
                    &[&pair[0], &pair[1]],
                    1,
                    || Ok(pair[0] == pair[1]),
                )? {
                    continue;
                }
                let (a, b) = if ascending {
                    (&pair[0], &pair[1])
                } else {
                    (&pair[1], &pair[0])
                };
                admit_ordinates([edge.start.x(), a], context, &mut retained)?;
                admit_ordinates([edge.start.x(), b], context, &mut retained)?;
                let start = Coord::xy(edge.start.x().clone(), a.clone());
                let end = Coord::xy(edge.start.x().clone(), b.clone());
                atomized.push(Edge { start, end });
            }
            // Only this edge's cuts remain live while its atoms are copied.
            // An error leaves their charge in `retained` for outer cleanup.
            drop(cuts);
            context.release_retained_workspace(cut_bytes, &mut retained)?;
        }
        let atomized_bytes = retained
            .checked_sub(atomized_start)
            .ok_or(GeoError::ArithmeticOverflow("union atom storage"))?;
        drop(endpoints);
        context.release_retained_workspace(endpoint_bytes, &mut retained)?;
        drop(edges);
        context.release_retained_workspace(edge_bytes, &mut retained)?;
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
        let mut boundary = Vec::new();
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
                boundary.push(edge);
            }
        }
        drop(pending);
        // Equal starts preserve the previous boundary-index iteration order,
        // retaining every original angular tie decision at a shared junction.
        let start_bytes = (boundary.len() as u64)
            .checked_mul(size_of::<usize>() as u64)
            .ok_or(GeoError::ArithmeticOverflow("union successor index"))?;
        context.retain_workspace(start_bytes, &mut retained)?;
        context.charge_work(boundary.len() as u64)?;
        progress.context_poll(context)?;
        let mut starts = Vec::with_capacity(boundary.len());
        starts.extend(0..boundary.len());
        purrdf_lex::walk::try_sort_unstable_by(&mut starts, |a, b| {
            let order = compare(&boundary[*a].start, &boundary[*b].start, context, progress)?;
            Ok(order.then_with(|| a.cmp(b)))
        })?;
        let mut used = vec![false; boundary.len()];
        let mut rings = Vec::new();
        for first in 0..boundary.len() {
            if used[first] {
                continue;
            }
            let mut index = first;
            let mut ring = Vec::new();
            loop {
                step(context, progress)?;
                if used[index] {
                    return Err(GeoError::domain("atlas union boundary did not close"));
                }
                used[index] = true;
                let edge = &boundary[index];
                output_bytes = output_bytes
                    .checked_add(admit(&edge.start, context, &mut retained)?)
                    .ok_or(GeoError::ArithmeticOverflow("union output storage"))?;
                ring.push(edge.start.clone());
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
                continue;
            }
            let mut first = 0;
            for index in 1..ring.len() {
                if compare(&ring[index], &ring[first], context, progress)?.is_lt() {
                    first = index;
                }
            }
            ring.rotate_left(first);
            output_bytes = output_bytes
                .checked_add(admit(&ring[0], context, &mut retained)?)
                .ok_or(GeoError::ArithmeticOverflow("union output storage"))?;
            ring.push(ring[0].clone());
            rings.push(ring);
        }
        drop(starts);
        context.release_retained_workspace(start_bytes, &mut retained)?;
        drop(boundary);
        drop(used);
        context.release_retained_workspace(atomized_bytes, &mut retained)?;
        let mut polygons: Vec<crate::Rings> = Vec::new();
        let mut outer_areas = Vec::new();
        let mut area_bytes = 0_u64;
        let mut holes = Vec::new();
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
                    polygons.push(vec![ring]);
                    outer_areas.push(area);
                    area_bytes = area_bytes
                        .checked_add(bytes)
                        .ok_or(GeoError::ArithmeticOverflow("union retained ring areas"))?;
                }
                Ordering::Less => {
                    holes.push(ring);
                    drop(area);
                    context.release_retained_workspace(bytes, &mut retained)?;
                }
                Ordering::Equal => {
                    drop(area);
                    context.release_retained_workspace(bytes, &mut retained)?;
                }
            }
        }
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
                        ExactAdmission::new(context, progress).rational(
                            RationalCompare,
                            &[&outer_areas[index], &outer_areas[old]],
                            1,
                            || Ok(outer_areas[index] < outer_areas[old]),
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
            .push(hole);
        }
        drop(outer_areas);
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
        let container = size_of::<Geometry>() as u64;
        context.retain_workspace(container, &mut retained)?;
        output_bytes = output_bytes
            .checked_add(container)
            .ok_or(GeoError::ArithmeticOverflow("union output container"))?;
        // Reuse the carrier home's dimensional/closure validation. Its complete
        // structural scan and original-limb equality work are admitted before
        // that body runs; the builder does not implement a second validator.
        let geometry = progress.exact(context, validation, || {
            Geometry::new(CoordDim::Xy, GeometryBody::MultiPolygon(polygons))
        })?;
        let transferred = retain_output(&geometry, output_bytes, context, progress)?;
        if transferred > output_bytes {
            return Err(GeoError::ArithmeticOverflow(
                "union output transfer exceeds owner",
            ));
        }
        retained = retained
            .checked_sub(transferred)
            .ok_or(GeoError::ArithmeticOverflow("union output transfer"))?;
        Ok(geometry)
    })();
    context.release_workspace(retained)?;
    result
}
fn step(context: &mut MetricContext, progress: &mut WorkProgress<'_>) -> Result<(), GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)
}
fn xy(point: &LonLat) -> Coord {
    Coord::xy(point.longitude().clone(), point.latitude().clone())
}
fn admit(point: &Coord, context: &mut MetricContext, retained: &mut u64) -> Result<u64, GeoError> {
    admit_ordinates([point.x(), point.y()], context, retained)
}
fn admit_ordinates(
    ordinates: [&Rat; 2],
    context: &mut MetricContext,
    retained: &mut u64,
) -> Result<u64, GeoError> {
    let bytes = super::boundary::ordinate_bytes(ordinates).saturating_mul(4);
    context.retain_workspace(bytes, retained)?;
    Ok(bytes)
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
    let half = admission.rational(
        RationalCompare,
        &[origin.x(), origin.y(), a.x(), a.y(), b.x(), b.y()],
        6,
        || {
            let half = |point: &Coord| {
                usize::from(
                    point.y() < origin.y() || (point.y() == origin.y() && point.x() < origin.x()),
                )
            };
            Ok(half(a).cmp(&half(b)))
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
                storage.adopt_geometry_output(geometry, available, context, progress, |count, _| {
                    assert_eq!(count, 7);
                    Ok(())
                })
            },
        )
        .unwrap();
        assert_eq!(context.current_workspace_bytes(), storage.output_bytes());
        let receipt = storage
            .finish(true, &mut context, &mut progress)
            .unwrap()
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
                refused_storage.adopt_geometry_output(
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
