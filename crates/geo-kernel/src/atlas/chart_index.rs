// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed exact chart broad phase. Closed bounds can only exclude an ordinary
//! polygon's base; complements stay visible through every ancestor. Candidates
//! retain the original polygon and share the atlas's one union predicate body.

use super::{ChartCandidate, charge};
use crate::context::WorkProgress;
use crate::measure::{OrderedBounds, borrowed_bounds};
use crate::numerical::ExactAdmission;
use crate::{Coord, GeoError, GeometryBody, MetricContext, PreparedPolygon, PreparedRegion, Set};

#[derive(Clone, Copy, Default)]
struct Node<'a> {
    bounds: Option<OrderedBounds<'a>>,
    unbounded: bool,
    rectangle: bool,
}

pub(super) struct ChartIndex<'a> {
    polygons: &'a [PreparedPolygon],
    nodes: Vec<Node<'a>>,
    leaves: usize,
    storage_bytes: u64,
}
impl<'a> ChartIndex<'a> {
    pub(super) fn prepare(
        region: &'a PreparedRegion,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Option<Self>, GeoError> {
        let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
            region
        else {
            return Ok(None);
        };
        if polygons.is_empty() {
            return Ok(None);
        }
        for polygon in polygons.iter() {
            charge(context, progress, 1)?;
            if polygon.oriented_interior().is_some() || polygon.chart().is_none() {
                return Ok(None);
            }
        }
        let leaves = polygons
            .len()
            .checked_next_power_of_two()
            .ok_or(GeoError::ArithmeticOverflow("chart bounds tree size"))?;
        let slots = leaves
            .checked_mul(2)
            .ok_or(GeoError::ArithmeticOverflow("chart bounds tree nodes"))?;
        let storage_bytes = slots
            .checked_mul(size_of::<Node<'_>>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::ArithmeticOverflow("chart bounds tree storage"))?;
        // Initializing every node is known work; refuse before allocation when
        // the complete initialization cannot be admitted.
        charge(context, progress, slots as u64)?;
        context.retain_workspace(storage_bytes, retained)?;
        progress.context_poll(context)?;
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(slots)
            .map_err(|_| GeoError::MemoryExhausted {
                limit: context.policy().limits().max_workspace_bytes,
            })?;
        nodes.resize(slots, Node::default());
        for (ordinal, polygon) in polygons.iter().enumerate() {
            charge(context, progress, 1)?;
            let GeometryBody::Polygon(rings) = polygon.chart().expect("checked chart").body()
            else {
                unreachable!("prepared polygon chart");
            };
            nodes[leaves + ordinal] = Node {
                bounds: {
                    let mut admission = ExactAdmission::new(context, progress);
                    borrowed_bounds(
                        rings.iter().flat_map(|ring| ring.iter()),
                        Some(&mut admission),
                    )?
                    .map(|bounds| bounds.ordered(&mut admission))
                    .transpose()?
                },
                unbounded: polygon.interior() == crate::RegionInterior::Complement,
                rectangle: is_rectangle(rings, context, progress)?,
            };
        }
        for parent in (1..leaves).rev() {
            charge(context, progress, 1)?;
            let left = nodes[2 * parent];
            let right = nodes[2 * parent + 1];
            let bounds = match (left.bounds, right.bounds) {
                (Some(left), Some(right)) => {
                    let mut bounds = left.original();
                    let mut admission = ExactAdmission::new(context, progress);
                    bounds.include_bounds(right.original(), Some(&mut admission))?;
                    Some(bounds.ordered(&mut admission)?)
                }
                (left, right) => left.or(right),
            };
            nodes[parent] = Node {
                bounds,
                unbounded: left.unbounded || right.unbounded,
                rectangle: false,
            };
        }
        Ok(Some(Self {
            polygons,
            nodes,
            leaves,
            storage_bytes,
        }))
    }

    pub(super) const fn storage_bytes(&self) -> u64 {
        self.storage_bytes
    }

    pub(super) fn locate(
        &self,
        point: &Coord,
        complementary: bool,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Set, GeoError> {
        let point_keys = {
            let mut admission = ExactAdmission::new(context, progress);
            [
                admission.order_key(point.x())?,
                admission.order_key(point.y())?,
            ]
        };
        // A tree with 2*leaves representable nodes has height below usize::BITS.
        // Depth-first traversal retains at most one sibling at each depth.
        let mut pending = [0_usize; usize::BITS as usize];
        pending[0] = 1;
        let mut count = 1;
        super::union_location_with(
            point,
            complementary,
            context,
            progress,
            |context, progress| {
                while count > 0 {
                    count -= 1;
                    let ordinal = pending[count];
                    charge(context, progress, 1)?;
                    let node = self.nodes[ordinal];
                    let box_location = match node.bounds {
                        Some(bounds) => bounds.locate_admitted(
                            point,
                            point_keys,
                            &mut ExactAdmission::new(context, progress),
                        )?,
                        None => Set::Exterior,
                    };
                    let base_exterior = box_location == Set::Exterior;
                    if base_exterior && !node.unbounded {
                        continue;
                    }
                    if ordinal >= self.leaves {
                        return Ok(self.polygons.get(ordinal - self.leaves).map(|polygon| {
                            ChartCandidate {
                                polygon,
                                base_location: if base_exterior || node.rectangle {
                                    Some(box_location)
                                } else {
                                    None
                                },
                            }
                        }));
                    }
                    let children = pending
                        .get_mut(count..count + 2)
                        .ok_or(GeoError::ArithmeticOverflow("chart bounds tree frontier"))?;
                    children[0] = 2 * ordinal + 1;
                    children[1] = 2 * ordinal;
                    count += 2;
                }
                Ok(None)
            },
        )
    }
}

/// A closed four-edge ring with nonzero alternating coordinate directions has
/// exactly two x values and two y values, and traverses their four corners.
/// This proves equality with its complete bounding box without approximation.
pub(super) fn is_rectangle(
    rings: &crate::Rings,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let [ring] = rings.as_slice() else {
        charge(context, progress, 1)?;
        return Ok(false);
    };
    is_rectangle_ring(ring, context, progress)
}

/// The same original corner proof for a borrowed individual polygon ring.
pub(super) fn is_rectangle_ring(
    ring: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    charge(context, progress, 1)?;
    if ring.len() != 5 {
        return Ok(false);
    }
    {
        let mut admission = ExactAdmission::new(context, progress);
        if !admission.compare(ring[0].x(), ring[4].x())?.is_eq()
            || !admission.compare(ring[0].y(), ring[4].y())?.is_eq()
        {
            return Ok(false);
        }
    }
    rectangle_cycle(&ring[..4], context, progress)
}

/// Prove an original four-vertex cyclic rectangle without allocating a
/// duplicate closure. The closed-ring and source-cell consumers share the
/// same alternating, nonzero XY edge proof.
pub(crate) fn is_rectangle_cycle(
    vertices: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    charge(context, progress, 1)?;
    if vertices.len() != 4 {
        return Ok(false);
    }
    rectangle_cycle(vertices, context, progress)
}

fn rectangle_cycle(
    vertices: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let mut previous_vertical = None;
    for index in 0..4 {
        let first = &vertices[index];
        let second = &vertices[(index + 1) % 4];
        let mut admission = ExactAdmission::new(context, progress);
        let x_equal = admission.compare(first.x(), second.x())?.is_eq();
        let y_equal = admission.compare(first.y(), second.y())?.is_eq();
        if x_equal == y_equal || previous_vertical == Some(x_equal) {
            return Ok(false);
        }
        previous_vertical = Some(x_equal);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, LonLat, PreparedGeometry, Rat};

    #[test]
    fn original_rectangle_cycle_matches_closed_ring_and_rejects_nonrectangular_cells() {
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        for (vertices, expected) in [
            (vec![(0, 0), (2, 0), (2, 1), (0, 1)], true),
            (vec![(0, 0), (0, 1), (2, 1), (2, 0)], true),
            (vec![(0, 0), (2, 0), (3, 1), (0, 1)], false),
            (vec![(0, 0), (2, 0), (2, 0), (0, 0)], false),
            (vec![(0, 0), (2, 0), (0, 1)], false),
        ] {
            let mut vertices = vertices
                .into_iter()
                .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
                .collect::<Vec<_>>();
            assert_eq!(
                is_rectangle_cycle(&vertices, &mut context, &mut progress).unwrap(),
                expected
            );
            vertices.push(vertices[0].clone());
            assert_eq!(
                is_rectangle_ring(&vertices, &mut context, &mut progress).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn indexed_membership_matches_full_atlas_for_holes_contacts_complements_and_aliases() {
        let profile = GeoProfile::standard();
        let target = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        for text in [
            "POLYGON ((-10 -10,10 -10,10 10,-10 10,-10 -10),(-2 -2,2 -2,2 2,-2 2,-2 -2))",
            "MULTIPOLYGON (((-10 -10,0 -10,0 10,-10 10,-10 -10)),((0 -10,10 -10,10 10,0 10,0 -10)))",
            "POLYGON ((170 -10,-170 -10,-170 10,170 10,170 -10))",
            "POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))",
            "POLYGON ((4 4,4 0,0 0,0 4,4 4))",
            "POLYGON ((0 0,4 0,3 4,0 4,0 0))",
            "POLYGON ((0 0,4 0,0 4,0 0))",
            "POLYGON ((0 0,4 0,4 0,0 0,0 0))",
            "POLYGON ZM ((0 0 1 2,4 0 3 4,4 4 5 6,0 4 7 8,0 0 9 10))",
        ] {
            let literal = crate::wkt::parse(text, &target).unwrap();
            let prepared = PreparedGeometry::from_literal(&literal, &profile).unwrap();
            let PreparedRegion::Polygons(polygons) = prepared.region() else {
                panic!("ordinary polygons");
            };
            let per_polygon_complement = PreparedRegion::polygons(
                polygons
                    .iter()
                    .cloned()
                    .map(|polygon| polygon.with_interior(crate::RegionInterior::Complement))
                    .collect(),
            );
            for region in [
                prepared.region().clone(),
                prepared.region().clone().complement(),
                per_polygon_complement.clone(),
                per_polygon_complement.complement(),
            ] {
                let mut limits = ExecutionLimits::GEOMETRY;
                limits.max_work_items = 10_000_000;
                let mut context = MetricContext::new(
                    crate::GeographicReference::wgs84(),
                    ExecutionPolicy::new(limits).unwrap(),
                )
                .unwrap();
                context.begin(1).unwrap();
                let mut progress = WorkProgress::new(None);
                let mut retained = 0;
                let index =
                    ChartIndex::prepare(&region, &mut context, &mut progress, &mut retained)
                        .unwrap()
                        .unwrap();
                for latitude in [-90, -10, -2, 0, 2, 10, 80, 90] {
                    for longitude in [-180, -170, -10, -2, 0, 2, 10, 170, 180] {
                        let point =
                            LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
                        let mut oracle = MetricContext::wgs84().unwrap();
                        let expected = crate::atlas::locate(&point, &region, &mut oracle).unwrap();
                        assert_eq!(
                            super::super::locate_with_chart_index(
                                &point,
                                &region,
                                Some(&index),
                                &mut context,
                                &mut progress,
                            )
                            .unwrap(),
                            expected,
                            "{text}: {longitude},{latitude}",
                        );
                    }
                }
                for (longitude, latitude) in [
                    ("3.5", "3.5"),
                    ("4", "3.5"),
                    ("4.0000000000000000000000000000001", "3.5"),
                ] {
                    let point = LonLat::new(
                        Rat::parse_decimal(longitude).unwrap(),
                        Rat::parse_decimal(latitude).unwrap(),
                    )
                    .unwrap();
                    let mut oracle = MetricContext::wgs84().unwrap();
                    let expected = crate::atlas::locate(&point, &region, &mut oracle).unwrap();
                    assert_eq!(
                        super::super::locate_with_chart_index(
                            &point,
                            &region,
                            Some(&index),
                            &mut context,
                            &mut progress,
                        )
                        .unwrap(),
                        expected,
                        "{text}: {longitude},{latitude}",
                    );
                }
                let bytes = index.storage_bytes();
                drop(index);
                context
                    .release_retained_workspace(bytes, &mut retained)
                    .unwrap();
                assert_eq!(retained, 0);
                assert_eq!(context.current_workspace_bytes(), 0);
            }
        }
    }

    #[test]
    fn open_alternating_path_does_not_certify_a_rectangle() {
        let coordinates = [(0, 0), (4, 0), (4, 4), (0, 4), (0, 2)]
            .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
            .to_vec();
        let mut context = MetricContext::wgs84().unwrap();
        assert!(
            !is_rectangle(
                &vec![coordinates],
                &mut context,
                &mut WorkProgress::integer(None),
            )
            .unwrap()
        );
        assert_eq!(context.current_workspace_bytes(), 0);
    }

    #[test]
    fn sparse_tree_prunes_before_winding_and_admits_storage_before_allocation() {
        let reference = crate::GeographicReference::wgs84();
        let polygons = (0..64)
            .map(|ordinal| {
                let west = 2 * ordinal - 64;
                let ring = [
                    (west, 0),
                    (west + 1, 0),
                    (west + 1, 1),
                    (west, 1),
                    (west, 0),
                ]
                .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
                .to_vec();
                PreparedPolygon::from_source(
                    &vec![ring],
                    &reference,
                    crate::RegionInterior::Written,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let region = PreparedRegion::polygons(polygons);
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 2_000_000;
        let mut context =
            MetricContext::new(reference.clone(), ExecutionPolicy::new(limits).unwrap()).unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        let mut retained = 0;
        let index = ChartIndex::prepare(&region, &mut context, &mut progress, &mut retained)
            .unwrap()
            .unwrap();
        let point = Coord::xy(Rat::from_i64(100), Rat::from_i64(0));
        let before = context.work_items();
        assert_eq!(
            index
                .locate(&point, false, &mut context, &mut progress)
                .unwrap(),
            Set::Exterior
        );
        let indexed_work = context.work_items() - before;
        let PreparedRegion::Polygons(polygons) = &region else {
            unreachable!();
        };
        let before = context.work_items();
        assert_eq!(
            super::super::union_location(&point, polygons, false, &mut context, &mut progress)
                .unwrap(),
            Set::Exterior,
        );
        assert!(indexed_work < context.work_items() - before);
        let bytes = index.storage_bytes();
        drop(index);
        context
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), 0);

        limits.max_work_items = 64;
        let mut refused =
            MetricContext::new(reference.clone(), ExecutionPolicy::new(limits).unwrap()).unwrap();
        refused.begin(1).unwrap();
        assert!(matches!(
            ChartIndex::prepare(&region, &mut refused, &mut progress, &mut retained),
            Err(GeoError::WorkExhausted { limit: 64 }),
        ));
        assert_eq!(retained, 0);
        assert_eq!(refused.current_workspace_bytes(), 0);

        limits.max_work_items = 2_000_000;
        limits.max_workspace_bytes = 1;
        let mut refused =
            MetricContext::new(reference, ExecutionPolicy::new(limits).unwrap()).unwrap();
        refused.begin(1).unwrap();
        assert!(matches!(
            ChartIndex::prepare(&region, &mut refused, &mut progress, &mut retained),
            Err(GeoError::MemoryExhausted { limit: 1 }),
        ));
        assert_eq!(retained, 0);
        assert_eq!(refused.current_workspace_bytes(), 0);
    }
}
