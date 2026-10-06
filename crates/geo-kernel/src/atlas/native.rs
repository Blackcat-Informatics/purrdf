// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete original-ring selection for native region arrangements.
//! After every pair of distinct rings is proved disjoint, membership in every
//! other ring is constant along a ring. Toggling its Left-side state in the
//! shared region law identifies its orientation or proves it an internal wall.
//! Representatives label already proved connected components; they never prove
//! separation or replace a complete curve image.

use super::arcs::{self, ChartLine};
use crate::context::WorkProgress;
use crate::numerical::geo_math_error;
use crate::{GeoError, LonLat, MetricContext, PreparedCurve, PreparedRegion, Rat, Set};
use purrdf_xsd::math::FixedInterval;

pub(crate) struct NativeRing {
    pub(crate) curve: PreparedCurve,
    pub(crate) reversed: bool,
    pub(crate) south_inside: bool,
}
pub(crate) struct NativeBoundary {
    pub(crate) rings: Vec<NativeRing>,
    pub(crate) south_inside: bool,
    pub(crate) workspace_bytes: u64,
}

pub(crate) fn select_boundary(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<NativeBoundary>, GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        region
    else {
        return Ok(None);
    };
    if polygons
        .iter()
        .any(|polygon| polygon.oriented_interior().is_none())
    {
        return Ok(None);
    }
    let count = polygons
        .iter()
        .map(|polygon| polygon.rings().len())
        .sum::<usize>();
    let bytes = arcs::source_workspace_bound(
        polygons
            .iter()
            .flat_map(crate::PreparedPolygon::rings)
            .flat_map(PreparedCurve::edges),
    )
    .checked_add(
        (count as u64)
            .checked_mul(1024)
            .ok_or(GeoError::ArithmeticOverflow("native boundary inventory"))?,
    )
    .ok_or(GeoError::ArithmeticOverflow("native boundary inventory"))?;
    context.admit_workspace(bytes)?;
    let result = select_disjoint(region, context, progress);
    match result {
        Ok((rings, south_inside)) => Ok(Some(NativeBoundary {
            rings,
            south_inside,
            workspace_bytes: bytes,
        })),
        Err(error) => {
            context.release_workspace(bytes)?;
            Err(error)
        }
    }
}

fn select_disjoint(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Vec<NativeRing>, bool), GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        region
    else {
        unreachable!("native polygon inventory")
    };
    let rings = polygons
        .iter()
        .flat_map(crate::PreparedPolygon::rings)
        .collect::<Vec<_>>();
    let mut representatives = Vec::with_capacity(rings.len());
    for (index, ring) in rings.iter().enumerate() {
        let mut representative = index;
        for (other, previous) in rings[..index].iter().enumerate() {
            if ring.edges().iter().chain(previous.edges()).any(|edge| {
                matches!(
                    edge,
                    crate::PreparedEdge::AzimuthLength(_) | crate::PreparedEdge::Transformed(_)
                )
            }) {
                continue;
            }
            let operands = ring
                .edges()
                .iter()
                .chain(previous.edges())
                .flat_map(crate::PreparedEdge::original_coordinates)
                .flat_map(|point| {
                    [
                        point.x(),
                        point.y(),
                        point.z().unwrap_or_else(|| point.x()),
                        point.m().unwrap_or_else(|| point.y()),
                    ]
                })
                .collect::<Vec<_>>();
            let equal = crate::numerical::ExactAdmission::new(context, progress).rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &operands,
                operands.len() as u64 + 1,
                || Ok(ring == previous),
            )?;
            if equal {
                representative = representatives[other];
                break;
            }
        }
        representatives.push(representative);
    }
    for (index, ring) in rings.iter().enumerate() {
        if representatives[index] != index {
            continue;
        }
        for (other_index, other) in rings.iter().enumerate().skip(index + 1) {
            if representatives[other_index] != other_index {
                continue;
            }
            for edge in ring.edges() {
                for second in other.edges() {
                    let mut child = context.remaining_child()?;
                    let contacts = {
                        let mut observer = progress.nested(
                            context.work_items(),
                            context
                                .policy()
                                .limits()
                                .max_workspace_bytes
                                .saturating_sub(context.remaining_workspace()),
                            context.workspace_peak(),
                        );
                        arcs::intersections_metered(edge, second, &mut child, &mut observer)
                    };
                    if !progress
                        .absorb_child_result(context, &child, contacts)?
                        .is_empty()
                    {
                        return Err(GeoError::PrecisionExhausted {
                            bits: context.policy().limits().max_precision_bits,
                        });
                    }
                }
            }
        }
    }
    let south = LonLat::new(Rat::zero(), Rat::from_i64(-90)).expect("exact south pole");
    let mut south_states = Vec::with_capacity(rings.len());
    for (index, ring) in rings.iter().enumerate() {
        if representatives[index] != index {
            south_states.push(south_states[representatives[index]]);
            continue;
        }
        south_states.push(
            match super::oriented::locate_validated(ring, &south, context, progress)? {
                Set::Interior => true,
                Set::Exterior => false,
                Set::Boundary => {
                    return Err(GeoError::PrecisionExhausted {
                        bits: context.policy().limits().max_precision_bits,
                    });
                }
            },
        );
    }
    let south_inside = membership(region, &south_states, context, progress)?;
    let mut selected = Vec::new();
    for (index, ring) in rings.iter().enumerate() {
        if representatives[index] != index {
            continue;
        }
        let mut states = representative_states(index, &rings, &representatives, context, progress)?;
        for (other, representative) in representatives.iter().enumerate() {
            if *representative == index {
                states[other] = true;
            }
        }
        let left = membership(region, &states, context, progress)?;
        for (other, representative) in representatives.iter().enumerate() {
            if *representative == index {
                states[other] = false;
            }
        }
        let right = membership(region, &states, context, progress)?;
        if left != right {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            selected.push(NativeRing {
                curve: (*ring).clone(),
                reversed: !left,
                south_inside: south_states[index],
            });
        }
    }
    Ok((selected, south_inside))
}

fn membership(
    region: &PreparedRegion,
    states: &[bool],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let (PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)) =
        region
    else {
        unreachable!("native polygon inventory")
    };
    let mut offset = 0;
    let mut bases = Vec::with_capacity(polygons.len());
    for polygon in polygons.iter() {
        context.charge_work(polygon.rings().len() as u64 + 1)?;
        progress.context_poll(context)?;
        let end = offset + polygon.rings().len();
        bases.push(super::polygon_base_membership(
            polygon,
            &states[offset..end],
        )?);
        offset = end;
    }
    super::region_membership(region, &bases)
}

fn representative_states(
    index: usize,
    rings: &[&PreparedCurve],
    representatives: &[usize],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<bool>, GeoError> {
    let edge = rings[index]
        .edges()
        .first()
        .expect("certified nonempty Jordan ring");
    if let Some(start) = edge.start() {
        let mut states = Vec::with_capacity(rings.len());
        for (other, ring) in rings.iter().enumerate() {
            if representatives[other] == index {
                states.push(false);
                continue;
            }
            states.push(
                match super::oriented::locate_validated(ring, start.point(), context, progress)? {
                    Set::Interior => true,
                    Set::Exterior => false,
                    Set::Boundary => {
                        return Err(GeoError::PrecisionExhausted {
                            bits: context.policy().limits().max_precision_bits,
                        });
                    }
                },
            );
        }
        return Ok(states);
    }
    let mut bits = 96.min(context.policy().limits().max_precision_bits);
    loop {
        let point = representative(edge, bits, context, progress)?;
        let mut states = Vec::with_capacity(rings.len());
        let mut complete = true;
        for (other, ring) in rings.iter().enumerate() {
            if representatives[other] == index {
                states.push(false);
                continue;
            }
            match super::locate_ring_enclosure(&point[0], &point[1], ring, context, progress)? {
                Some(Set::Interior) => states.push(true),
                Some(Set::Exterior) => states.push(false),
                Some(Set::Boundary) | None => {
                    complete = false;
                    break;
                }
            }
        }
        if complete {
            return Ok(states);
        }
        if bits >= context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        bits = bits
            .saturating_mul(2)
            .min(context.policy().limits().max_precision_bits);
    }
}

fn representative(
    edge: &crate::PreparedEdge,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[FixedInterval; 2], GeoError> {
    let policy = context.policy();
    arcs::with_lines(
        core::slice::from_ref(edge),
        bits,
        context,
        progress,
        |lines, math, progress| {
            (|| {
                let half = FixedInterval::from_i64(1, math)?
                    .div(&FixedInterval::from_i64(2, math)?, math)?;
                let (point, _) = ChartLine::image(
                    &lines[0],
                    &half,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )?;
                let degrees =
                    FixedInterval::from_i64(180, math)?.div(&FixedInterval::pi(math)?, math)?;
                Ok([point[0].mul(&degrees, math)?, point[1].mul(&degrees, math)?])
            })()
            .map_err(|error| geo_math_error(&error, policy))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, GeographicReference, OrientedInterior, PreparedGeometry, PreparedPolygon};

    fn parallel(latitude: i64, eastward: bool, reference: &GeographicReference) -> PreparedCurve {
        let ends = if eastward { [-180, 180] } else { [180, -180] };
        PreparedCurve::from_source(
            &ends.map(|longitude| Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude))),
            reference,
        )
        .unwrap()
    }

    #[test]
    fn native_hole_boundary_preserves_analytic_band_area_and_perimeter() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let polygon = PreparedPolygon::from_curves(
            vec![
                parallel(30, true, &reference),
                parallel(45, false, &reference),
            ],
            OrientedInterior::Left,
            &mut context,
        )
        .unwrap();
        let geometry = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap();
        let area = crate::ellipsoidal::area(&geometry, &mut context).unwrap();
        // Independent Q(1/2), Q(1/sqrt(2)) analytic expressions at 105
        // decimal digits; pi uses a bounded Machin series, no geo kernel.
        let expected = Rat::parse_decimal("52842899826632.51500287").unwrap();
        assert!(area.exact().sub(&expected).abs() < Rat::parse_decimal("0.1").unwrap());
        let perimeter = crate::ellipsoidal::perimeter(&geometry, &mut context).unwrap();
        let expected = Rat::parse_decimal("63119921.52415486").unwrap();
        assert!(perimeter.exact().sub(&expected).abs() < Rat::parse_decimal("0.001").unwrap());
    }

    #[test]
    fn native_nested_union_removes_internal_ring_and_complement_preserves_boundary() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let mut polygons = Vec::new();
        for latitude in [30, 45] {
            polygons.push(
                PreparedPolygon::from_curves(
                    vec![parallel(latitude, true, &reference)],
                    OrientedInterior::Left,
                    &mut context,
                )
                .unwrap(),
            );
        }
        let region = PreparedRegion::polygons(polygons);
        let geometry = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            region.clone(),
            context.policy(),
        )
        .unwrap();
        let inverse = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            Vec::new(),
            region.complement(),
            context.policy(),
        )
        .unwrap();
        let a = crate::ellipsoidal::area(&geometry, &mut context).unwrap();
        let b = crate::ellipsoidal::area(&inverse, &mut context).unwrap();
        let expected = Rat::parse_decimal("127944540878342.71812180").unwrap();
        let whole = Rat::parse_decimal("510065621724088.50929491").unwrap();
        assert!(a.exact().sub(&expected).abs() < Rat::parse_decimal("0.1").unwrap());
        assert!(a.exact().add(b.exact()).sub(&whole).abs() < Rat::parse_decimal("0.1").unwrap());
        let a = crate::ellipsoidal::perimeter(&geometry, &mut context).unwrap();
        let b = crate::ellipsoidal::perimeter(&inverse, &mut context).unwrap();
        assert_eq!(a.exact(), b.exact());
        let expected = Rat::parse_decimal("34735060.89032274").unwrap();
        assert!(a.exact().sub(&expected).abs() < Rat::parse_decimal("0.001").unwrap());
    }

    #[test]
    fn exact_coincident_native_rings_share_one_boundary_and_can_fill_the_surface() {
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let ring = parallel(30, true, &reference);
        for second_side in [OrientedInterior::Left, OrientedInterior::Right] {
            let mut polygons = Vec::new();
            for side in [OrientedInterior::Left, second_side] {
                polygons.push(
                    PreparedPolygon::from_curves(vec![ring.clone()], side, &mut context).unwrap(),
                );
            }
            let geometry = PreparedGeometry::from_parts(
                reference.clone(),
                Vec::new(),
                Vec::new(),
                PreparedRegion::polygons(polygons),
                context.policy(),
            )
            .unwrap();
            let area = crate::ellipsoidal::area(&geometry, &mut context).unwrap();
            let perimeter = crate::ellipsoidal::perimeter(&geometry, &mut context).unwrap();
            let (expected_area, expected_perimeter) = if second_side == OrientedInterior::Left {
                ("127944540878342.71812180", "34735060.89032274")
            } else {
                ("510065621724088.50929491", "0")
            };
            assert!(
                area.exact()
                    .sub(&Rat::parse_decimal(expected_area).unwrap())
                    .abs()
                    < Rat::parse_decimal("0.1").unwrap()
            );
            assert!(
                perimeter
                    .exact()
                    .sub(&Rat::parse_decimal(expected_perimeter).unwrap())
                    .abs()
                    < Rat::parse_decimal("0.001").unwrap()
            );
        }
    }
}
