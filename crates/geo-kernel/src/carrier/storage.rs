// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original carrier's one container/coordinate storage traversal.

use crate::context::WorkProgress;
use crate::{Coord, GeoError, Geometry, GeometryBody, MetricContext, Rings};

pub(crate) trait GeometryStorageVisitor {
    fn geometry(&mut self, depth: u64) -> Result<(), GeoError>;
    fn structure(&mut self) -> Result<(), GeoError>;
    fn coordinate(&mut self, coordinate: &Coord) -> Result<(), GeoError>;
    fn storage<T>(&mut self, capacity: usize) -> Result<(), GeoError>;
    fn collection(&mut self, children: usize) -> Result<(), GeoError>;
    fn pending_child(&mut self) -> Result<(), GeoError>;
}

fn line(line: &Vec<Coord>, visitor: &mut impl GeometryStorageVisitor) -> Result<(), GeoError> {
    visitor.structure()?;
    visitor.storage::<Coord>(line.capacity())?;
    for coordinate in line {
        visitor.coordinate(coordinate)?;
    }
    Ok(())
}

fn polygon(rings: &Rings, visitor: &mut impl GeometryStorageVisitor) -> Result<(), GeoError> {
    visitor.structure()?;
    visitor.storage::<Vec<Coord>>(rings.capacity())?;
    for coordinates in rings {
        line(coordinates, visitor)?;
    }
    Ok(())
}

/// Preserve the original source walk's event order for admission and output
/// storage. This visits capacity even for empty vectors and empty members.
pub(crate) fn visit_geometry_storage(
    geometry: &Geometry,
    visitor: &mut impl GeometryStorageVisitor,
) -> Result<(), GeoError> {
    let mut pending = purrdf_lex::walk::WorkList::<(&Geometry, u64), 16>::with((geometry, 0));
    while let Some((geometry, depth)) = pending.pop() {
        visitor.geometry(depth)?;
        match geometry.body() {
            GeometryBody::Point(point) => {
                if let Some(point) = point {
                    visitor.coordinate(point)?;
                }
            }
            GeometryBody::LineString(coordinates) => line(coordinates, visitor)?,
            GeometryBody::Polygon(rings) => polygon(rings, visitor)?,
            GeometryBody::MultiPoint(points) => {
                visitor.storage::<Option<Coord>>(points.capacity())?;
                for point in points {
                    visitor.structure()?;
                    if let Some(point) = point {
                        visitor.coordinate(point)?;
                    }
                }
            }
            GeometryBody::MultiLineString(lines) => {
                visitor.storage::<Vec<Coord>>(lines.capacity())?;
                for coordinates in lines {
                    line(coordinates, visitor)?;
                }
            }
            GeometryBody::MultiPolygon(polygons) => {
                visitor.storage::<Rings>(polygons.capacity())?;
                for rings in polygons {
                    polygon(rings, visitor)?;
                }
            }
            GeometryBody::GeometryCollection(members) => {
                visitor.collection(members.len())?;
                visitor.storage::<Geometry>(members.capacity())?;
                let child_depth = depth.checked_add(1).ok_or(GeoError::ArithmeticOverflow(
                    "source geometry nesting depth",
                ))?;
                for member in members.iter().rev() {
                    visitor.pending_child()?;
                    pending.push((member, child_depth));
                }
            }
        }
    }
    Ok(())
}

pub(crate) struct GeometryStorage {
    bytes: u64,
    coordinates: u64,
}

impl GeometryStorage {
    pub(crate) const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub(crate) const fn coordinates(&self) -> u64 {
        self.coordinates
    }
}

struct OwnedStorage<'a, 'observer> {
    owned: GeometryStorage,
    temporary: u64,
    context: &'a mut MetricContext,
    progress: &'a mut WorkProgress<'observer>,
}

/// Reserve metadata after its enclosing byte allowance has been admitted.
/// The original work/cancellation body is shared by contact graphs and image
/// preparations; allocation failure preserves its typed native refusal.
pub(crate) fn reserve_metadata<T>(
    values: &mut Vec<T>,
    additional: usize,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    context.charge_work(
        u64::try_from(additional)
            .map_err(|_| GeoError::ArithmeticOverflow("contact metadata capacity"))?,
    )?;
    progress.context_poll(context)?;
    values
        .try_reserve_exact(additional)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })
}

/// The same admitted reservation for one appended metadata owner. Geometric
/// growth keeps preparation linear; the caller's four-slot-per-owner envelope
/// covers the simultaneously live old/new containers before this call.
pub(crate) fn reserve_metadata_for_push<T>(
    values: &mut Vec<T>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    if values.len() == values.capacity() {
        reserve_metadata(values, values.capacity().max(1), context, progress)?;
    }
    Ok(())
}

impl OwnedStorage<'_, '_> {
    fn step(&mut self, work: u64) -> Result<(), GeoError> {
        self.context.charge_work(work)?;
        self.progress.context_poll(self.context)
    }

    fn add(&mut self, bytes: u64) -> Result<(), GeoError> {
        self.owned.bytes = self
            .owned
            .bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("owned carrier storage"))?;
        Ok(())
    }
}

impl GeometryStorageVisitor for OwnedStorage<'_, '_> {
    fn geometry(&mut self, _depth: u64) -> Result<(), GeoError> {
        self.step(1)
    }
    fn structure(&mut self) -> Result<(), GeoError> {
        self.step(1)
    }
    fn coordinate(&mut self, coordinate: &Coord) -> Result<(), GeoError> {
        self.step(1)?;
        self.owned.coordinates =
            self.owned
                .coordinates
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow(
                    "owned carrier coordinate count",
                ))?;
        for value in [coordinate.x(), coordinate.y()]
            .into_iter()
            .chain(coordinate.z())
            .chain(coordinate.m())
        {
            // Two constant-time integer allocation descriptors, with no limb
            // copy, rational reduction, decimal rendering or interpretation.
            self.step(2)?;
            let bytes = u64::try_from(value.allocated_bytes())
                .map_err(|_| GeoError::ArithmeticOverflow("owned carrier rational storage"))?;
            self.add(bytes)?;
        }
        Ok(())
    }
    fn storage<T>(&mut self, capacity: usize) -> Result<(), GeoError> {
        self.step(1)?;
        let bytes = u64::try_from(capacity)
            .ok()
            .and_then(|capacity| capacity.checked_mul(size_of::<T>() as u64))
            .ok_or(GeoError::ArithmeticOverflow(
                "owned carrier container storage",
            ))?;
        self.add(bytes)
    }
    fn collection(&mut self, children: usize) -> Result<(), GeoError> {
        self.step(1)?;
        // WorkList's spill Vec grows geometrically. Four pending slots per
        // visited child bound both its minimum allocation and capacity growth;
        // admission precedes every push, including a deeply nested collection.
        let bytes = u64::try_from(children)
            .ok()
            .and_then(|count| count.checked_mul((4 * size_of::<(&Geometry, u64)>()) as u64))
            .ok_or(GeoError::ArithmeticOverflow("carrier storage work list"))?;
        let temporary = self
            .temporary
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "carrier storage temporary allowance",
            ))?;
        self.context.admit_workspace(bytes)?;
        self.temporary = temporary;
        self.progress.context_poll(self.context)
    }
    fn pending_child(&mut self) -> Result<(), GeoError> {
        self.step(1)
    }
}

/// Count root inline storage, actual nested Vec capacities and original Rat
/// allocations while the producer's reservations still cover the live graph.
/// Shared integer storage is counted conservatively for each owner; numerical
/// arena storage stays covered by its independent retained context baseline.
pub(crate) fn owned_geometry_storage(
    geometry: &Geometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<GeometryStorage, GeoError> {
    let temporary = size_of::<purrdf_lex::walk::WorkList<(&Geometry, u64), 16>>() as u64;
    context.admit_workspace(temporary)?;
    let mut visitor = OwnedStorage {
        owned: GeometryStorage {
            bytes: size_of::<Geometry>() as u64,
            coordinates: 0,
        },
        temporary,
        context,
        progress,
    };
    let result = visit_geometry_storage(geometry, &mut visitor);
    visitor.context.release_workspace(visitor.temporary)?;
    result.map(|()| visitor.owned)
}

/// Sum original Rat heap ownership through the same coordinate metadata body.
/// The caller's live producer allowance covers these borrowed coordinates.
/// Inline coordinates, enclosing containers and iterator storage are excluded.
pub(crate) fn owned_coordinates_storage<'a>(
    coordinates: impl IntoIterator<Item = &'a Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<GeometryStorage, GeoError> {
    progress.context_poll(context)?;
    let mut visitor = OwnedStorage {
        owned: GeometryStorage {
            bytes: 0,
            coordinates: 0,
        },
        temporary: 0,
        context,
        progress,
    };
    for coordinate in coordinates {
        visitor.coordinate(coordinate)?;
    }
    // Even a terminal iterator callback is followed by the existing observer
    // and arithmetic-state validation home before a successful census escapes.
    visitor.progress.context_poll(visitor.context)?;
    Ok(visitor.owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoordDim, ExecutionPolicy, GeographicReference, Int, MetricWorkObserver, Rat};

    #[test]
    fn storage_counts_empty_capacities_and_each_original_limb_owner() {
        let large = Rat::from_int(Int::one().shl(4096));
        let limb_bytes = large.allocated_bytes() as u64;
        assert!(limb_bytes > 0);
        let mut points = Vec::with_capacity(7);
        points.push(Some(Coord::xy(large.clone(), Rat::zero())));
        points.push(Some(Coord::xy(large, Rat::one())));
        let point_capacity = points.capacity();
        let empty = Vec::<Coord>::with_capacity(11);
        let empty_capacity = empty.capacity();
        let mut members = Vec::with_capacity(5);
        members.push(Geometry::new(CoordDim::Xy, GeometryBody::MultiPoint(points)).unwrap());
        members.push(Geometry::new(CoordDim::Xy, GeometryBody::LineString(empty)).unwrap());
        let member_capacity = members.capacity();
        let geometry =
            Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(members)).unwrap();
        let expected = (size_of::<Geometry>()
            + member_capacity * size_of::<Geometry>()
            + point_capacity * size_of::<Option<Coord>>()
            + empty_capacity * size_of::<Coord>()) as u64
            + 2 * limb_bytes;
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::integer(None);
        let stored = owned_geometry_storage(&geometry, &mut context, &mut progress).unwrap();
        assert_eq!(stored.bytes(), expected);
        assert_eq!(stored.coordinates(), 2);
        assert_eq!(context.current_workspace_bytes(), 0);
    }

    #[test]
    fn borrowed_coordinate_storage_uses_actual_owners_without_allocating() {
        let large = Rat::from_int(Int::one().shl(4_096));
        let bytes = large.allocated_bytes() as u64;
        let coordinates = [Coord::new(
            large.clone(),
            Rat::zero(),
            Some(large),
            Some(Rat::one()),
        )];
        let mut context = MetricContext::wgs84().unwrap();
        context.admit_workspace(123).unwrap();
        let mut progress = WorkProgress::integer(None);
        let empty = owned_coordinates_storage([], &mut context, &mut progress).unwrap();
        assert_eq!(empty.bytes(), 0);
        assert_eq!(empty.coordinates(), 0);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let storage = owned_coordinates_storage(&coordinates, &mut context, &mut progress).unwrap();
        let allocations = window.close();
        assert_eq!(storage.bytes(), 2 * bytes);
        assert_eq!(storage.coordinates(), 1);
        assert_eq!(context.current_workspace_bytes(), 123);
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);

        let policy = ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 1,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut tiny = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        tiny.admit_workspace(123).unwrap();
        let mut progress = WorkProgress::integer(None);
        assert!(matches!(
            owned_coordinates_storage(&coordinates, &mut tiny, &mut progress),
            Err(GeoError::WorkExhausted { limit: 1 })
        ));
        assert_eq!(tiny.current_workspace_bytes(), 123);
    }

    #[test]
    fn borrowed_coordinate_cancellation_preserves_the_live_producer_allowance() {
        let coordinates = [Coord::xy(Rat::zero(), Rat::zero())];
        let mut context = MetricContext::wgs84().unwrap();
        context.admit_workspace(123).unwrap();
        let mut observer = CancelAfter(1);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        assert!(matches!(
            owned_coordinates_storage(&coordinates, &mut context, &mut progress),
            Err(GeoError::Cancelled)
        ));
        assert_eq!(context.current_workspace_bytes(), 123);
    }

    struct CancelAfter(u64);
    impl MetricWorkObserver for CancelAfter {
        fn charge_chunk(&mut self, work: u64, _workspace: u64) -> Result<(), GeoError> {
            self.0 = self.0.checked_sub(work).ok_or(GeoError::Cancelled)?;
            Ok(())
        }
    }

    #[test]
    fn wide_collection_refusal_releases_only_its_temporary_walk_storage() {
        let members = (0..40)
            .map(|_| Geometry::new(CoordDim::Xy, GeometryBody::Point(None)).unwrap())
            .collect();
        let geometry =
            Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(members)).unwrap();
        let mut context =
            MetricContext::new(GeographicReference::wgs84(), ExecutionPolicy::geometry()).unwrap();
        context.admit_workspace(123).unwrap();
        // Reach more than sixteen pending children, so the actual walk has
        // spilled before the callback refuses a following admitted push.
        let mut observer = CancelAfter(40);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        assert!(matches!(
            owned_geometry_storage(&geometry, &mut context, &mut progress),
            Err(GeoError::Cancelled)
        ));
        assert_eq!(context.current_workspace_bytes(), 123);
        assert!(context.workspace_peak() > 123);
    }
}
