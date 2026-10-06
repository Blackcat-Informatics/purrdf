// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unique shortest paths coincide between any two proved common points.
//! Interior cross-endpoint witnesses also determine complete partial overlap.
//! Unknown source contact never proves containment or absence of a contact.

use super::{CurveIntersection, admit_contact_count, axis_overlap};
use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{GeoError, LonLat, MetricContext, PreparedEdge, Rat, ShortestGeodesicArc};
use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};

type ParameterBounds = (Rat, Rat);
type ParameterPair = (ParameterBounds, ParameterBounds);

pub(super) fn intersections(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let (PreparedEdge::ShortestGeodesic(a), PreparedEdge::ShortestGeodesic(b)) = (left, right)
    else {
        return Ok(None);
    };
    let container = if contains(left, b, context, progress)? {
        Some((a.as_ref(), b.as_ref(), true))
    } else if contains(right, a, context, progress)? {
        Some((b.as_ref(), a.as_ref(), false))
    } else {
        None
    };
    let Some((outer, inner, outer_is_left)) = container else {
        // Absence of a source equality witness is not absence of an
        // intersection. The complete numerical contact search follows.
        return partial(left, right, a, b, context, progress);
    };
    let constant = ExactAdmission::new(context, progress).rational(
        Linear,
        &[
            inner.start().point().longitude(),
            inner.start().point().latitude(),
            inner.end().point().longitude(),
            inner.end().point().latitude(),
        ],
        8,
        || Ok(inner.start().point().same_location(inner.end().point())),
    )?;
    if constant {
        // Constant images retain their complete parameter domain in the
        // shared front end, rather than producing a positive-length overlap.
        return super::constant::intersections(left, right, context, progress);
    }
    let (parameters, forward) = ordered_parameters(outer, inner, context, progress)?;
    let exact = [(Rat::zero(), Rat::zero()), (Rat::one(), Rat::one())];
    let (mut first, mut second) = if outer_is_left {
        (parameters, exact)
    } else {
        (exact, parameters)
    };
    // The output's endpoint order always follows the original first curve.
    let swap = outer_is_left && !forward;
    if swap {
        first.swap(0, 1);
        second.swap(0, 1);
    }
    let (start, end) = if swap {
        (inner.end().point(), inner.start().point())
    } else {
        (inner.start().point(), inner.end().point())
    };
    materialize(first, second, start, end, context, progress)
}

fn materialize(
    first: [(Rat, Rat); 2],
    second: [(Rat, Rat); 2],
    start: &LonLat,
    end: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    admit_contact_count(1, context)?;
    let retained = first.iter().chain(&second).fold(
        size_of::<CurveIntersection>() as u64 + size_of::<[(Rat, Rat); 2]>() as u64,
        |bytes, (low, high)| {
            bytes
                .saturating_add(low.allocated_bytes() as u64)
                .saturating_add(high.allocated_bytes() as u64)
        },
    );
    let retained = retained
        .saturating_add(start.longitude().allocated_bytes() as u64)
        .saturating_add(start.latitude().allocated_bytes() as u64)
        .saturating_add(end.longitude().allocated_bytes() as u64)
        .saturating_add(end.latitude().allocated_bytes() as u64);
    context.admit_workspace(retained)?;
    let result = (|| {
        let (start, end) = ExactAdmission::new(context, progress).rational(
            Linear,
            &[
                start.longitude(),
                start.latitude(),
                end.longitude(),
                end.latitude(),
            ],
            4,
            || Ok((start.clone(), end.clone())),
        )?;
        Ok(Some(vec![CurveIntersection::GeodesicOverlap {
            left: first,
            right: Box::new(second),
            start,
            end,
        }]))
    })();
    context.release_workspace(retained)?;
    result
}

fn partial(
    left: &PreparedEdge,
    right: &PreparedEdge,
    a: &ShortestGeodesicArc,
    b: &ShortestGeodesicArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    for (i, first) in [a.start().point(), a.end().point()].into_iter().enumerate() {
        if crate::atlas::point::exact_source(right, first, context, progress)? != Some(true) {
            continue;
        }
        for (j, second) in [b.start().point(), b.end().point()].into_iter().enumerate() {
            if crate::atlas::point::exact_source(left, second, context, progress)? != Some(true) {
                continue;
            }
            let distinct = ExactAdmission::new(context, progress).rational(
                Linear,
                &[
                    first.longitude(),
                    first.latitude(),
                    second.longitude(),
                    second.latitude(),
                ],
                4,
                || Ok(!first.same_location(second)),
            )?;
            if !distinct {
                continue;
            }
            let (on_a, on_b) = interior_parameters(a, second, b, first, context, progress)?;
            // Both witnesses lie strictly inside the other original arc.
            // Unique shortest subpaths coincide between the witnesses. Their
            // geodesic tangents therefore continue the same supporting line;
            // the two remaining endpoints extend away from this interval.
            let exact_a = (Rat::from_i64(i as i64), Rat::from_i64(i as i64));
            let exact_b = (Rat::from_i64(j as i64), Rat::from_i64(j as i64));
            let (on_a, on_b, start, end) = if i == 0 {
                ([exact_a, on_a], [on_b, exact_b], first, second)
            } else {
                ([on_a, exact_a], [exact_b, on_b], second, first)
            };
            return materialize(on_a, on_b, start, end, context, progress);
        }
    }
    Ok(None)
}

fn interior_parameters(
    a: &ShortestGeodesicArc,
    on_a: &LonLat,
    b: &ShortestGeodesicArc,
    on_b: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<ParameterPair, GeoError> {
    let policy = context.policy();
    let mut refinement = None;
    loop {
        let attempt = (|| {
            let first =
                axis_overlap::parameter_with_precision(a, on_a, refinement, context, progress)?;
            let second =
                axis_overlap::parameter_with_precision(b, on_b, refinement, context, progress)?;
            let zero = Rat::zero();
            let one = Rat::one();
            let mut admission = ExactAdmission::new(context, progress);
            for parameter in [&first, &second] {
                if !admission.rational(RationalCompare, &[&parameter.0, &zero], 1, || {
                    Ok(parameter.0 > zero)
                })? || !admission.rational(RationalCompare, &[&parameter.1, &one], 1, || {
                    Ok(parameter.1 < one)
                })? {
                    return Ok(None);
                }
            }
            Ok(Some((first, second)))
        })();
        match attempt {
            Ok(Some(value)) => return Ok(value),
            Ok(None) | Err(GeoError::PrecisionExhausted { .. }) => {}
            Err(error) => return Err(error),
        }
        let bits = refinement.unwrap_or(64);
        if bits >= policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        refinement = Some(
            bits.saturating_mul(2)
                .min(policy.limits().max_precision_bits),
        );
    }
}

fn contains(
    container: &PreparedEdge,
    candidate: &ShortestGeodesicArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    for point in [candidate.start().point(), candidate.end().point()] {
        if crate::atlas::point::exact_source(container, point, context, progress)? != Some(true) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn ordered_parameters(
    outer: &ShortestGeodesicArc,
    inner: &ShortestGeodesicArc,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<([(Rat, Rat); 2], bool), GeoError> {
    let policy = context.policy();
    let mut refinement = None;
    loop {
        let attempt = (|| {
            let parameters = [
                axis_overlap::parameter_with_precision(
                    outer,
                    inner.start().point(),
                    refinement,
                    context,
                    progress,
                )?,
                axis_overlap::parameter_with_precision(
                    outer,
                    inner.end().point(),
                    refinement,
                    context,
                    progress,
                )?,
            ];
            let order = ExactAdmission::new(context, progress).rational(
                RationalCompare,
                &[
                    &parameters[0].0,
                    &parameters[0].1,
                    &parameters[1].0,
                    &parameters[1].1,
                ],
                2,
                || {
                    Ok(if parameters[0].1 < parameters[1].0 {
                        Some(true)
                    } else if parameters[1].1 < parameters[0].0 {
                        Some(false)
                    } else {
                        None
                    })
                },
            )?;
            Ok(order.map(|forward| (parameters, forward)))
        })();
        match attempt {
            Ok(Some(value)) => return Ok(value),
            Ok(None) | Err(GeoError::PrecisionExhausted { .. }) => {}
            Err(error) => return Err(error),
        }
        let bits = refinement.unwrap_or(64);
        if bits >= policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        refinement = Some(
            bits.saturating_mul(2)
                .min(policy.limits().max_precision_bits),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::arcs::{intersections, tests::shortest};

    #[test]
    fn cross_endpoint_witnesses_define_complete_partial_overlap_in_both_orders() {
        let mut context = MetricContext::wgs84().unwrap();
        for a_reversed in [false, true] {
            for b_reversed in [false, true] {
                let a = if a_reversed {
                    shortest((10, 0), (-10, 0), &mut context)
                } else {
                    shortest((-10, 0), (10, 0), &mut context)
                };
                let b = if b_reversed {
                    shortest((20, 0), (0, 0), &mut context)
                } else {
                    shortest((0, 0), (20, 0), &mut context)
                };
                for swapped in [false, true] {
                    let (left, right, reversed) = if swapped {
                        (&b, &a, b_reversed)
                    } else {
                        (&a, &b, a_reversed)
                    };
                    context.begin(1).unwrap();
                    let mut progress = WorkProgress::new(None);
                    // Consume the general supporting-geodesic witness law
                    // directly; the public axial fast path is independently
                    // qualified against these same original curves.
                    let contacts = super::intersections(left, right, &mut context, &mut progress)
                        .unwrap()
                        .unwrap();
                    let [
                        CurveIntersection::GeodesicOverlap {
                            left,
                            right,
                            start,
                            end,
                        },
                    ] = contacts.as_slice()
                    else {
                        panic!("one complete partial overlap")
                    };
                    assert!(left[0].1 < left[1].0);
                    assert_eq!(right[0].1 < right[1].0, a_reversed == b_reversed);
                    assert_eq!(
                        start.longitude(),
                        &Rat::from_i64(if reversed { 10 } else { 0 })
                    );
                    assert_eq!(
                        end.longitude(),
                        &Rat::from_i64(if reversed { 0 } else { 10 })
                    );
                    assert!(start.latitude().is_zero() && end.latitude().is_zero());
                }
            }
        }
    }

    #[test]
    fn symmetric_general_subsets_keep_exact_endpoints_and_reversed_parameters() {
        let mut context = MetricContext::wgs84().unwrap();
        for (outer_start, outer_end, middle) in [
            ((-10, -10), (10, 10), (0, 0)),
            ((170, -10), (-170, 10), (180, 0)),
        ] {
            let outer = shortest(outer_start, outer_end, &mut context);
            for backwards in [false, true] {
                let (a, b) = if backwards {
                    (outer_end, middle)
                } else {
                    (middle, outer_end)
                };
                let inner = shortest(a, b, &mut context);
                for outer_is_left in [false, true] {
                    let (left, right) = if outer_is_left {
                        (&outer, &inner)
                    } else {
                        (&inner, &outer)
                    };
                    let contacts = intersections(left, right, &mut context).unwrap();
                    let [
                        CurveIntersection::GeodesicOverlap {
                            left,
                            right,
                            start,
                            end,
                        },
                    ] = contacts.as_slice()
                    else {
                        panic!("one complete unique original subcurve")
                    };
                    assert!(left[0].1 < left[1].0);
                    let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
                    let (whole, part) = if outer_is_left {
                        (left, right.as_ref())
                    } else {
                        (right.as_ref(), left)
                    };
                    let (middle_parameter, end_parameter) = if outer_is_left || !backwards {
                        (&whole[0], &whole[1])
                    } else {
                        (&whole[1], &whole[0])
                    };
                    assert!(middle_parameter.0 <= half && half <= middle_parameter.1);
                    assert_eq!(end_parameter, &(Rat::one(), Rat::one()));
                    if outer_is_left && backwards {
                        assert_eq!(
                            part,
                            &[(Rat::one(), Rat::one()), (Rat::zero(), Rat::zero())]
                        );
                    } else {
                        assert_eq!(
                            part,
                            &[(Rat::zero(), Rat::zero()), (Rat::one(), Rat::one())]
                        );
                    }
                    let start_parameter = if outer_is_left || !backwards {
                        middle
                    } else {
                        outer_end
                    };
                    let end_parameter = if outer_is_left || !backwards {
                        outer_end
                    } else {
                        middle
                    };
                    assert_eq!(start.longitude(), &Rat::from_i64(start_parameter.0));
                    assert_eq!(start.latitude(), &Rat::from_i64(start_parameter.1));
                    assert_eq!(end.longitude(), &Rat::from_i64(end_parameter.0));
                    assert_eq!(end.latitude(), &Rat::from_i64(end_parameter.1));
                }
            }
        }
    }

    #[test]
    fn unknown_endpoint_containment_never_excludes_other_contacts() {
        let mut context = MetricContext::wgs84().unwrap();
        let left = shortest((-10, -10), (10, 10), &mut context);
        let right = shortest((-10, 10), (10, -10), &mut context);
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        assert_eq!(
            super::intersections(&left, &right, &mut context, &mut progress).unwrap(),
            None
        );
        let contacts = intersections(&left, &right, &mut context).unwrap();
        let [CurveIntersection::Isolated { left, right, .. }] = contacts.as_slice() else {
            panic!("complete transverse contact search remains active")
        };
        let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
        assert!(left.0 <= half && half <= left.1 && right.0 <= half && half <= right.1);
    }
}
