// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact vertical cells of a labelled closed planar region. Every endpoint and
//! segment crossing is an event, so edge order is fixed within each open slab.
//! The caller supplies the complete original region classifier and bounding
//! rectangle; neither datum nor geographic axis interpretation is inferred.

use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, compare_rat as compare, copy_rat as copy};
use crate::{Coord, GeoError, MetricContext, Rat, SegmentIntersection, Set};
use purrdf_xsd::integer::ExactOperation::{RationalAdd, RationalDivide};

pub(crate) struct PlanarStrip {
    pub(crate) lower: [Coord; 2],
    pub(crate) upper: [Coord; 2],
}

type Crossing<'a> = (Rat, Option<(&'a Coord, &'a Coord)>);

/// Borrowed closed bounds preserve the original exact edge and avoid visiting
/// edges whose x or y projections prove that an intersection is impossible.
struct BoundedEdge<'a> {
    a: &'a Coord,
    b: &'a Coord,
    west: &'a Rat,
    east: &'a Rat,
    south: &'a Rat,
    north: &'a Rat,
    west_key: u64,
    vertical: bool,
    horizontal: bool,
}

pub(crate) fn decompose(
    edges: &[(&Coord, &Coord)],
    domain: [&Rat; 4],
    coalesce_interior: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    mut locate: impl FnMut(&Coord, &mut MetricContext, &mut WorkProgress<'_>) -> Result<Set, GeoError>,
) -> Result<Vec<PlanarStrip>, GeoError> {
    let [west, east, south, north] = domain;
    context.charge_work(edges.len() as u64)?;
    progress.context_poll(context)?;
    let bits = |value: &Rat| {
        value
            .numerator()
            .bit_len()
            .saturating_add(value.denominator().bit_len())
    };
    let mut maximum_bits = domain.into_iter().map(bits).max().unwrap_or(32);
    for chunk in edges.chunks(256) {
        progress.context_poll(context)?;
        for &(a, b) in chunk {
            for value in [a.x(), a.y(), b.x(), b.y()] {
                maximum_bits = maximum_bits.max(bits(value));
            }
        }
    }
    let event_bytes = maximum_bits
        .div_ceil(8)
        .saturating_mul(32)
        .saturating_add(256);
    let index_bytes = (edges.len() as u64)
        .checked_mul((size_of::<BoundedEdge<'_>>() + size_of::<usize>()) as u64)
        .ok_or(GeoError::ArithmeticOverflow("arrangement edge index"))?;
    context.retain_workspace(index_bytes, retained)?;
    progress.context_poll(context)?;
    let mut indexed = Vec::with_capacity(edges.len());
    for &(a, b) in edges {
        let x_order = compare(a.x(), b.x(), context, progress)?;
        let (west, east) = if x_order.is_le() {
            (a.x(), b.x())
        } else {
            (b.x(), a.x())
        };
        let y_order = compare(a.y(), b.y(), context, progress)?;
        let (south, north) = if y_order.is_le() {
            (a.y(), b.y())
        } else {
            (b.y(), a.y())
        };
        indexed.push(BoundedEdge {
            a,
            b,
            west,
            east,
            south,
            north,
            west_key: ExactAdmission::new(context, progress).order_key(west)?,
            vertical: x_order.is_eq(),
            horizontal: y_order.is_eq(),
        });
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut indexed, |a, b| {
        // Equal west runs enter together, and every unordered candidate pair
        // still occurs in the suffix. Coincident slab graphs have identical
        // endpoints, so no source-ordinal tie participates in the output law.
        context.charge_work(1)?;
        progress.context_poll(context)?;
        let order = a.west_key.cmp(&b.west_key);
        if order.is_eq() {
            compare(a.west, b.west, context, progress)
        } else {
            Ok(order)
        }
    })?;
    let mut active = Vec::with_capacity(edges.len());
    let mut next_edge = 0;
    let events_start = *retained;
    let mut events = Vec::new();
    for value in [west, east] {
        push_event(value, &mut events, event_bytes, context, progress, retained)?;
    }
    for &(a, b) in edges {
        for value in [a.x(), b.x()] {
            push_event(value, &mut events, event_bytes, context, progress, retained)?;
        }
    }
    // A vertical contact's x is already an endpoint event. Two horizontal
    // edges are parallel, or their overlap has original endpoints as its x
    // extrema. Only a pair containing a slanted edge can add a new x event.
    context.charge_work(indexed.len() as u64)?;
    progress.context_poll(context)?;
    let has_slanted = indexed
        .iter()
        .any(|edge| !edge.vertical && !edge.horizontal);
    for (index, edge) in indexed
        .iter()
        .enumerate()
        .filter(|(_, edge)| has_slanted && !edge.vertical)
    {
        for other in &indexed[index + 1..] {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            // Sorted left endpoints make every later edge disjoint as soon as
            // this closed x bound is passed. Equal bounds retain all contacts.
            if compare(other.west, edge.east, context, progress)?.is_gt() {
                break;
            }
            if other.vertical || edge.horizontal && other.horizontal {
                continue;
            }
            if compare(other.south, edge.north, context, progress)?.is_gt()
                || compare(edge.south, other.north, context, progress)?.is_gt()
            {
                continue;
            }
            let intersection = super::intersect_admitted(
                edge.a,
                edge.b,
                other.a,
                other.b,
                &mut ExactAdmission::new(context, progress),
            )?;
            match intersection {
                SegmentIntersection::Point(point) => push_event(
                    point.x(),
                    &mut events,
                    event_bytes,
                    context,
                    progress,
                    retained,
                )?,
                SegmentIntersection::Collinear { from, to } => {
                    push_event(
                        from.x(),
                        &mut events,
                        event_bytes,
                        context,
                        progress,
                        retained,
                    )?;
                    push_event(
                        to.x(),
                        &mut events,
                        event_bytes,
                        context,
                        progress,
                        retained,
                    )?;
                }
                SegmentIntersection::None => {}
            }
        }
    }
    let events_bytes = retained
        .checked_sub(events_start)
        .ok_or(GeoError::ArithmeticOverflow("arrangement event storage"))?;
    purrdf_lex::walk::try_sort_unstable_by(&mut events, |a, b| compare(a, b, context, progress))?;
    purrdf_lex::walk::try_dedup_by(&mut events, |a, b| {
        compare(a, b, context, progress).map(core::cmp::Ordering::is_eq)
    })?;
    let mut strips = Vec::new();
    for bounds in events.windows(2) {
        if compare(&bounds[1], west, context, progress)?.is_le()
            || compare(&bounds[0], east, context, progress)?.is_ge()
        {
            continue;
        }
        let middle = average(&bounds[0], &bounds[1], context, progress)?;
        // Open slab midpoints increase monotonically. Each edge enters once;
        // expired right endpoints never become active again. Vertical edges
        // supply events but have no ordinate inside an open slab.
        while let Some(edge) = indexed.get(next_edge) {
            if compare(edge.west, &middle, context, progress)?.is_ge() {
                break;
            }
            active.push(next_edge);
            next_edge += 1;
        }
        for slot in (0..active.len()).rev() {
            if compare(indexed[active[slot]].east, &middle, context, progress)?.is_le() {
                active.swap_remove(slot);
            }
        }
        let crossings_start = *retained;
        let mut crossings: Vec<Crossing<'_>> = Vec::new();
        for value in [south, north] {
            context.retain_workspace(event_bytes, retained)?;
            crossings.push((copy(value, context, progress)?, None));
        }
        for &index in &active {
            let edge = &indexed[index];
            context.retain_workspace(event_bytes, retained)?;
            crossings.push((
                ordinate(edge.a, edge.b, &middle, context, progress)?,
                Some((edge.a, edge.b)),
            ));
        }
        let crossing_bytes = retained
            .checked_sub(crossings_start)
            .ok_or(GeoError::ArithmeticOverflow("arrangement crossing storage"))?;
        purrdf_lex::walk::try_sort_unstable_by(&mut crossings, |a, b| {
            compare(&a.0, &b.0, context, progress)
        })?;
        purrdf_lex::walk::try_dedup_by(&mut crossings, |a, b| {
            compare(&a.0, &b.0, context, progress).map(core::cmp::Ordering::is_eq)
        })?;
        let mut selected_lower = None;
        for (ordinal, pair) in crossings.windows(2).enumerate() {
            let representative = Coord::xy(
                copy(&middle, context, progress)?,
                average(&pair[0].0, &pair[1].0, context, progress)?,
            );
            let interior = locate(&representative, context, progress)? == Set::Interior;
            if coalesce_interior && interior {
                selected_lower.get_or_insert(ordinal);
            } else {
                if let Some(lower) = selected_lower.take() {
                    push_strip(
                        [&crossings[lower], &crossings[ordinal]],
                        bounds,
                        event_bytes,
                        context,
                        progress,
                        retained,
                        &mut strips,
                    )?;
                }
                if interior {
                    push_strip(
                        [&pair[0], &pair[1]],
                        bounds,
                        event_bytes,
                        context,
                        progress,
                        retained,
                        &mut strips,
                    )?;
                }
            }
        }
        if let Some(lower) = selected_lower {
            push_strip(
                [
                    &crossings[lower],
                    crossings.last().expect("selected interval"),
                ],
                bounds,
                event_bytes,
                context,
                progress,
                retained,
                &mut strips,
            )?;
        }
        drop(crossings);
        context.release_retained_workspace(crossing_bytes, retained)?;
    }
    drop(events);
    context.release_retained_workspace(events_bytes, retained)?;
    drop(active);
    drop(indexed);
    context.release_retained_workspace(index_bytes, retained)?;
    Ok(strips)
}

fn push_strip(
    pair: [&Crossing<'_>; 2],
    bounds: &[Rat],
    event_bytes: u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    strips: &mut Vec<PlanarStrip>,
) -> Result<(), GeoError> {
    // The complete construction bound stays live until the original
    // coordinate visitor measures all four resulting limb owners.
    // Only those owners survive the construction; vector capacity is
    // admitted separately, once per allocation.
    let construction_bytes = event_bytes
        .checked_mul(16)
        .ok_or(GeoError::ArithmeticOverflow(
            "arrangement strip construction",
        ))?;
    context.retain_workspace(construction_bytes, retained)?;
    let strip = PlanarStrip {
        lower: corners(pair[0], bounds, context, progress)?,
        upper: corners(pair[1], bounds, context, progress)?,
    };
    let storage = crate::carrier::owned_coordinates_storage(
        strip.lower.iter().chain(&strip.upper),
        context,
        progress,
    )?;
    let mut surplus =
        construction_bytes
            .checked_sub(storage.bytes())
            .ok_or(GeoError::ArithmeticOverflow(
                "arrangement strip construction bound",
            ))?;
    if strips.len() == strips.capacity() {
        let old_capacity = strips.capacity();
        let new_capacity = old_capacity
            .checked_mul(2)
            .map(|capacity| capacity.max(4))
            .ok_or(GeoError::ArithmeticOverflow("arrangement strip capacity"))?;
        let allocation_bytes = (new_capacity as u64)
            .checked_mul(size_of::<PlanarStrip>() as u64)
            .ok_or(GeoError::ArithmeticOverflow("arrangement strip storage"))?;
        // A reallocating allocator may hold both buffers while it copies.
        // Keep the old capacity admitted through reservation.
        context.retain_workspace(allocation_bytes, retained)?;
        progress.context_poll(context)?;
        strips
            .try_reserve_exact(new_capacity - strips.len())
            .map_err(|_| GeoError::MemoryExhausted {
                limit: context.policy().limits().max_workspace_bytes,
            })?;
        // Preserve any reported extra slots from the still-live construction
        // allowance before releasing either buffer.
        let actual_bytes = (strips.capacity() as u64)
            .checked_mul(size_of::<PlanarStrip>() as u64)
            .ok_or(GeoError::ArithmeticOverflow("arrangement strip storage"))?;
        let extra_bytes =
            actual_bytes
                .checked_sub(allocation_bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "arrangement strip reservation",
                ))?;
        surplus = surplus
            .checked_sub(extra_bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "arrangement strip allocation bound",
            ))?;
        context.release_retained_workspace(
            (old_capacity * size_of::<PlanarStrip>()) as u64,
            retained,
        )?;
    }
    strips.push(strip);
    context.release_retained_workspace(surplus, retained)
}

fn corners(
    crossing: &Crossing<'_>,
    bounds: &[Rat],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[Coord; 2], GeoError> {
    let mut corner = |x: &Rat| {
        let y = if let Some((a, b)) = crossing.1 {
            ordinate(a, b, x, context, progress)?
        } else {
            copy(&crossing.0, context, progress)?
        };
        Ok(Coord::xy(copy(x, context, progress)?, y))
    };
    Ok([corner(&bounds[0])?, corner(&bounds[1])?])
}
fn ordinate(
    a: &Coord,
    b: &Coord,
    x: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let offset = admission.rational(RationalAdd, &[x, a.x()], 1, || Ok(x.sub(a.x())))?;
    let width = admission.rational(RationalAdd, &[b.x(), a.x()], 1, || Ok(b.x().sub(a.x())))?;
    let parameter = admission.rational(RationalDivide, &[&offset, &width], 1, || {
        offset
            .div(&width)
            .ok_or_else(|| GeoError::domain("vertical edge has no open-slab ordinate"))
    })?;
    crate::SourceLinearEdge::interpolate_ordinate_admitted(a.y(), b.y(), &parameter, &mut admission)
}
fn average(
    a: &Rat,
    b: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let sum = admission.rational(RationalAdd, &[a, b], 1, || Ok(a.add(b)))?;
    let two = Rat::from_i64(2);
    admission.rational(RationalDivide, &[&sum, &two], 1, || {
        Ok(sum.div(&two).expect("positive divisor"))
    })
}
fn push_event(
    value: &Rat,
    events: &mut Vec<Rat>,
    bytes: u64,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<(), GeoError> {
    context.retain_workspace(bytes, retained)?;
    events.push(copy(value, context, progress)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy};

    fn point(x: i64, y: i64) -> Coord {
        Coord::xy(Rat::from_i64(x), Rat::from_i64(y))
    }

    #[test]
    fn rectangular_contacts_and_collinear_overlap_need_only_endpoint_events() {
        let rings = [
            [
                point(-2, -1),
                point(0, -1),
                point(0, 1),
                point(-2, 1),
                point(-2, -1),
            ],
            [
                point(-1, 0),
                point(1, 0),
                point(1, 2),
                point(-1, 2),
                point(-1, 0),
            ],
        ];
        let edges = rings
            .iter()
            .flat_map(|ring| ring.windows(2).map(|pair| (&pair[0], &pair[1])))
            .collect::<Vec<_>>();
        let domain = [-3, 3, -3, 3].map(Rat::from_i64);
        let mut context = MetricContext::wgs84().unwrap();
        let mut retained = 0;
        let strips = decompose(
            &edges,
            domain.each_ref(),
            false,
            &mut context,
            &mut WorkProgress::integer(None),
            &mut retained,
            |p, _, _| {
                let first = p.x() > &Rat::from_i64(-2)
                    && p.x() < &Rat::zero()
                    && p.y() > &Rat::from_i64(-1)
                    && p.y() < &Rat::one();
                let second = p.x() > &Rat::from_i64(-1)
                    && p.x() < &Rat::one()
                    && p.y() > &Rat::zero()
                    && p.y() < &Rat::from_i64(2);
                Ok(if first || second {
                    Set::Interior
                } else {
                    Set::Exterior
                })
            },
        )
        .unwrap();
        let expected = [
            ([-2, -1, -1, -1], [-2, 1, -1, 1]),
            ([-1, -1, 0, -1], [-1, 0, 0, 0]),
            ([-1, 0, 0, 0], [-1, 1, 0, 1]),
            ([-1, 1, 0, 1], [-1, 2, 0, 2]),
            ([0, 0, 1, 0], [0, 2, 1, 2]),
        ];
        assert_eq!(strips.len(), expected.len());
        for (strip, (lower, upper)) in strips.iter().zip(expected) {
            assert_eq!(
                strip.lower,
                [point(lower[0], lower[1]), point(lower[2], lower[3])]
            );
            assert_eq!(
                strip.upper,
                [point(upper[0], upper[1]), point(upper[2], upper[3])]
            );
        }
        drop(strips);
        let bytes = retained;
        context
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), 0);
    }

    #[test]
    fn exact_crossings_contacts_and_coincident_edges_keep_complete_strips() {
        let source = [
            [point(-1, -1), point(1, 1)],
            [point(1, -1), point(-1, 1)],
            [point(0, -2), point(0, 2)],
            [point(1, 1), point(1, 1)],
            [point(-1, -1), point(1, 1)],
        ];
        let edges: Vec<_> = source.iter().map(|pair| (&pair[0], &pair[1])).collect();
        let domain = [-2, 2, -2, 2].map(Rat::from_i64);
        let mut context = MetricContext::wgs84().unwrap();
        let before = context.current_workspace_bytes();
        let mut retained = 0;
        let strips = decompose(
            &edges,
            domain.each_ref(),
            false,
            &mut context,
            &mut WorkProgress::integer(None),
            &mut retained,
            |p, _, _| {
                // Independent closed hourglass membership: its two triangles
                // meet at the crossing, with no interior outside |x| < 1.
                let x = p.x().abs();
                let y = p.y().abs();
                Ok(if x < Rat::one() && y < x {
                    Set::Interior
                } else {
                    Set::Exterior
                })
            },
        )
        .unwrap();
        assert_eq!(strips.len(), 2);
        assert_eq!(strips[0].lower, [point(-1, -1), point(0, 0)]);
        assert_eq!(strips[0].upper, [point(-1, 1), point(0, 0)]);
        assert_eq!(strips[1].lower, [point(0, 0), point(1, -1)]);
        assert_eq!(strips[1].upper, [point(0, 0), point(1, 1)]);
        // All these exact integers fit the shared inline representation. The
        // returned owner therefore retains precisely its allocated slots.
        assert_eq!(
            retained,
            (strips.capacity() * size_of::<PlanarStrip>()) as u64
        );
        assert_eq!(context.current_workspace_bytes(), before + retained);
        drop(strips);
        let bytes = retained;
        context
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), before);
    }

    #[test]
    fn union_coalescing_keeps_positive_width_holes_and_original_affine_boundaries() {
        let source = [-3, -2, -1, 0, 1, 2, 3].map(|y| [point(-1, y), point(1, y)]);
        let edges = source
            .iter()
            .map(|pair| (&pair[0], &pair[1]))
            .collect::<Vec<_>>();
        let domain = [-1, 1, -4, 4].map(Rat::from_i64);
        for coalesce in [false, true] {
            let mut context = MetricContext::wgs84().unwrap();
            let mut retained = 0;
            let mut classified = 0;
            let strips = decompose(
                &edges,
                domain.each_ref(),
                coalesce,
                &mut context,
                &mut WorkProgress::integer(None),
                &mut retained,
                |p, _, _| {
                    classified += 1;
                    let inside = p.y() > &Rat::from_i64(-3) && p.y() < &Rat::from_i64(-1)
                        || p.y() > &Rat::zero() && p.y() < &Rat::from_i64(3);
                    Ok(if inside { Set::Interior } else { Set::Exterior })
                },
            )
            .unwrap();
            // Every original open interval is still classified, including the
            // hole, although only the outer edges of each selected run survive.
            assert_eq!(classified, 8);
            let expected: &[(i64, i64)] = if coalesce {
                &[(-3, -1), (0, 3)]
            } else {
                &[(-3, -2), (-2, -1), (0, 1), (1, 2), (2, 3)]
            };
            assert_eq!(strips.len(), expected.len());
            for (strip, &(south, north)) in strips.iter().zip(expected) {
                assert_eq!(strip.lower, [point(-1, south), point(1, south)]);
                assert_eq!(strip.upper, [point(-1, north), point(1, north)]);
            }
            drop(strips);
            let bytes = retained;
            context
                .release_retained_workspace(bytes, &mut retained)
                .unwrap();
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }

    #[test]
    fn sparse_vertical_events_release_all_temporary_storage() {
        let source: Vec<_> = (0..128)
            .map(|x| [point(2 * x, 0), point(2 * x, 1)])
            .collect();
        let edges: Vec<_> = source.iter().map(|pair| (&pair[0], &pair[1])).collect();
        let domain = [-1, 256, -1, 2].map(Rat::from_i64);
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_workspace_bytes = 128 * 1024;
        // Preserve the actual native default work refusal for this complete
        // sparse inventory, then admit enough work to qualify its memory peak.
        let mut limited = MetricContext::new(
            crate::GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let mut refused_storage = 0;
        assert!(matches!(
            decompose(
                &edges,
                domain.each_ref(),
                false,
                &mut limited,
                &mut WorkProgress::integer(None),
                &mut refused_storage,
                |_, _, _| Ok(Set::Exterior),
            ),
            Err(GeoError::WorkExhausted { .. })
        ));
        let bytes = refused_storage;
        limited
            .release_retained_workspace(bytes, &mut refused_storage)
            .unwrap();
        assert_eq!(refused_storage, 0);
        assert_eq!(limited.current_workspace_bytes(), 0);
        limits.max_work_items = 2_000_000;
        let mut context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let before = context.current_workspace_bytes();
        let mut retained = 0;
        let strips = decompose(
            &edges,
            domain.each_ref(),
            false,
            &mut context,
            &mut WorkProgress::integer(None),
            &mut retained,
            |_, _, _| Ok(Set::Exterior),
        )
        .unwrap();
        assert!(strips.is_empty());
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), before);
        assert!(context.work_items() <= limits.max_work_items);
        assert!(context.workspace_peak() <= limits.max_workspace_bytes);
    }

    #[test]
    fn exact_wide_corners_keep_limb_owners_after_construction() {
        let wide = Rat::from_int(crate::Int::one().shl(256));
        let limb_bytes = wide.allocated_bytes() as u64;
        assert!(limb_bytes > 0);
        let domain = [Rat::zero(), Rat::one(), wide.neg(), wide.clone()];
        let mut context = MetricContext::wgs84().unwrap();
        context.admit_workspace(123).unwrap();
        let mut retained = 0;
        let strips = decompose(
            &[],
            domain.each_ref(),
            false,
            &mut context,
            &mut WorkProgress::integer(None),
            &mut retained,
            |_, _, _| Ok(Set::Interior),
        )
        .unwrap();
        assert_eq!(strips.len(), 1);
        assert_eq!(
            strips[0].lower,
            [
                Coord::xy(Rat::zero(), wide.neg()),
                Coord::xy(Rat::one(), wide.neg())
            ]
        );
        assert_eq!(
            strips[0].upper,
            [
                Coord::xy(Rat::zero(), wide.clone()),
                Coord::xy(Rat::one(), wide)
            ]
        );
        assert_eq!(
            retained,
            (strips.capacity() * size_of::<PlanarStrip>()) as u64 + 4 * limb_bytes
        );
        assert_eq!(context.current_workspace_bytes(), 123 + retained);
        drop(strips);
        let bytes = retained;
        context
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), 123);
    }

    #[test]
    fn strip_growth_and_exact_memory_ceiling_keep_complete_or_refused_outcomes() {
        let source: Vec<_> = (0..16).map(|x| [point(x, 0), point(x, 1)]).collect();
        let edges: Vec<_> = source.iter().map(|pair| (&pair[0], &pair[1])).collect();
        let domain = [-1, 16, -1, 2].map(Rat::from_i64);
        let mut context = MetricContext::wgs84().unwrap();
        let mut retained = 0;
        let strips = decompose(
            &edges,
            domain.each_ref(),
            false,
            &mut context,
            &mut WorkProgress::integer(None),
            &mut retained,
            |_, _, _| Ok(Set::Interior),
        )
        .unwrap();
        // Sixteen vertical contacts split the complete domain into seventeen
        // consecutive rectangles, with no intersections or omitted slabs.
        assert_eq!(strips.len(), 17);
        for (index, strip) in strips.iter().enumerate() {
            let x = i64::try_from(index).unwrap() - 1;
            assert_eq!(strip.lower, [point(x, -1), point(x + 1, -1)]);
            assert_eq!(strip.upper, [point(x, 2), point(x + 1, 2)]);
        }
        assert_eq!(
            retained,
            (strips.capacity() * size_of::<PlanarStrip>()) as u64
        );
        let peak = context.workspace_peak();
        drop(strips);
        let bytes = retained;
        context
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(context.current_workspace_bytes(), 0);

        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_workspace_bytes = peak - 1;
        let mut limited = MetricContext::new(
            crate::GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let mut retained = 0;
        assert!(matches!(
            decompose(
                &edges,
                domain.each_ref(),
                false,
                &mut limited,
                &mut WorkProgress::integer(None),
                &mut retained,
                |_, _, _| Ok(Set::Interior),
            ),
            Err(GeoError::MemoryExhausted { .. })
        ));
        let bytes = retained;
        limited
            .release_retained_workspace(bytes, &mut retained)
            .unwrap();
        assert_eq!(limited.current_workspace_bytes(), 0);
        assert_eq!(retained, 0);
    }

    #[test]
    fn inventory_work_and_cancellation_refuse_before_index_reservation() {
        let source = [[point(-1, -1), point(1, 1)], [point(-1, 1), point(1, -1)]];
        let edges: Vec<_> = source.iter().map(|pair| (&pair[0], &pair[1])).collect();
        let domain = [-2, 2, -2, 2].map(Rat::from_i64);
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 1;
        let mut context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let mut retained = 0;
        assert!(matches!(
            decompose(
                &edges,
                domain.each_ref(),
                false,
                &mut context,
                &mut WorkProgress::integer(None),
                &mut retained,
                |_, _, _| panic!("refusal must precede region membership"),
            ),
            Err(GeoError::WorkExhausted { limit: 1 })
        ));
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), 0);

        struct Cancel(usize);
        impl crate::MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                self.0 += 1;
                Err(GeoError::Cancelled)
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        let mut observer = Cancel(0);
        assert!(matches!(
            decompose(
                &edges,
                domain.each_ref(),
                false,
                &mut context,
                &mut WorkProgress::integer(Some(&mut observer)),
                &mut retained,
                |_, _, _| panic!("cancellation must precede region membership"),
            ),
            Err(GeoError::Cancelled)
        ));
        assert_eq!(observer.0, 1);
        assert_eq!(retained, 0);
        assert_eq!(context.current_workspace_bytes(), 0);
    }
}
