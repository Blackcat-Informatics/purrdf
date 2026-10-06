// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified source-defined curve intersections in a smooth geodetic-normal atlas.
//! A local iterate never proves a contact. Complete interval images exclude
//! boxes; the shared Krawczyk home proves existence and uniqueness of roots.

use crate::context::WorkProgress;
use crate::geodesic::normal::{FixedArithmetic, NormalArithmetic, TaylorArithmetic};
use crate::geodesic::{ArcIntervalView, arc_view::FixedArcLine};
use crate::numerical::{ExactAdmission, exact_bounds, fixed_from_rat, geo_math_error};
use crate::{
    Coord, GeoError, LonLat, MetricContext, MetricWorkObserver, PreparedEdge, Rat,
    SegmentIntersection, SourceLinearEdge,
};
use purrdf_xsd::{
    BigInt,
    math::{
        CoordinateMath, FixedInterval, MathError, RootBox2, RootIsolation2, RootIsolationLimits,
        RootJacobian2, RootSystem2, TaylorJet, TaylorWorkspace, isolate_roots2,
    },
};

pub mod arrangement;
pub use arrangement::{
    NativeBoundaryArrangement, NativeBoundaryFragment, NativeSelectedPoint, SelectedContactStratum,
    SelectedFragmentStratum, SourceParameter, native_boundary, native_boundary_metered,
};
mod axis_overlap;
mod constant;
mod polynomial;
mod source_join;
mod subset;

/// Exact source endpoint contact or certified unrounded parameter-root evidence.
/// Enclosures are proof receipts; they are never substituted for curve inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurveIntersection {
    /// Exact equality of an original source position under one complete
    /// operation mapping. The target point remains an unrounded symbolic image;
    /// parameters may be strictly inside either original carrier edge.
    SymbolicSourceContact {
        /// Exact original parameter on the first curve.
        left: Rat,
        /// Exact original parameter on the second curve.
        right: Rat,
    },
    /// Exact equality of original endpoints under the same complete operation
    /// chain, target realization and epoch. Its geographic value stays symbolic.
    SymbolicEndpoint {
        /// Exact endpoint parameter on the first curve.
        left: Rat,
        /// Exact endpoint parameter on the second curve.
        right: Rat,
    },
    /// An exact written-coordinate contact, including its physical seam alias.
    Exact {
        /// Exact parameter on the first curve.
        left: Rat,
        /// Exact parameter on the second curve.
        right: Rat,
        /// Original physical coordinate witness.
        point: LonLat,
    },
    /// A contact established by exact seam/pole-aware source endpoint equality.
    Endpoint {
        /// Exact endpoint parameter on the first curve.
        left: Rat,
        /// Exact endpoint parameter on the second curve.
        right: Rat,
        /// Exact source point, with seam and pole aliases identified.
        point: LonLat,
    },
    /// An existing unique isolated contact in the open parameter domains.
    Isolated {
        /// Inclusive exact rational parameter enclosure on the first curve.
        left: (Rat, Rat),
        /// Inclusive exact rational parameter enclosure on the second curve.
        right: (Rat, Rat),
        /// Inclusive unrounded continuous longitude enclosure.
        longitude_radians: (Rat, Rat),
        /// Inclusive unrounded geodetic latitude enclosure.
        latitude_radians: (Rat, Rat),
    },
    /// The entire selected curves coincide, possibly with reversed orientation.
    Coincident {
        /// Whether the second curve traverses the first in reverse.
        reversed: bool,
    },
    /// A complete shared selected-branch subcurve. Exact parameter intervals
    /// identify the same source-defined path; its endpoints remain symbolic.
    BranchOverlap {
        /// Original first-curve parameters of the complete shared subcurve.
        left: (Rat, Rat),
        /// Original second-curve parameters of the same selected subcurve.
        right: (Rat, Rat),
    },
    /// A complete shared unique geodesic subcurve. Exact original endpoints
    /// retain the path; generally irrational arclength parameters stay enclosed.
    GeodesicOverlap {
        /// Inclusive parameter bounds at the start and end on the first arc.
        left: [(Rat, Rat); 2],
        /// Inclusive parameter bounds at the same endpoints on the second arc.
        right: Box<[(Rat, Rat); 2]>,
        /// Exact original physical overlap start, oriented along the first arc.
        start: LonLat,
        /// Exact original physical overlap end, oriented along the first arc.
        end: LonLat,
    },
    /// One exact physical contact involving a certified constant image. A
    /// constant curve retains its full parameter domain instead of a fake root.
    ConstantContact {
        /// Complete first-curve parameter enclosure for the contact.
        left: (Rat, Rat),
        /// Complete second-curve parameter enclosure for the contact.
        right: (Rat, Rat),
        /// Exact original physical point of the constant curve.
        point: LonLat,
    },
    /// An exact positive-length shared coordinate-linear subcurve. Parameter
    /// intervals are ordered from the first physical endpoint to the second.
    Overlap {
        /// First-curve parameters corresponding to the shared endpoints.
        left: (Rat, Rat),
        /// Second-curve parameters corresponding to the shared endpoints.
        right: (Rat, Rat),
        /// Exact source coordinate at the first shared endpoint.
        start: LonLat,
        /// Exact source coordinate at the second shared endpoint.
        end: LonLat,
    },
}

/// Find every curve contact, retaining exact source endpoints and arc branches.
///
/// # Errors
/// Refuses mismatched references, uncertified singular contacts, or incomplete
/// work, memory, precision, subdivision and convergence admission. No partial
/// contact list is returned after an unresolved chart or root decision.
pub fn intersections(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
) -> Result<Vec<CurveIntersection>, GeoError> {
    intersect(left, right, context, None)
}

/// Find every source-defined contact with bounded governor charging.
///
/// # Errors
/// Adds observer refusal to [`intersections`].
pub fn intersections_metered(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Vec<CurveIntersection>, GeoError> {
    intersect(left, right, context, Some(observer))
}

/// Refine isolated contacts and unique-shortest overlap endpoints to a
/// parameter width no greater than `2^-minimum_parameter_bits`.
/// This changes proof tightness, not the original curves or contact identities.
/// Non-isolated constant domains remain complete domains.
///
/// # Errors
/// Refuses insufficient precision or any incomplete contact/resource proof.
pub fn intersections_refined(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
) -> Result<Vec<CurveIntersection>, GeoError> {
    intersect_with_goal(left, right, minimum_parameter_bits, context, None)
}

/// Refine the same complete contact proof with bounded governor charging.
///
/// # Errors
/// Adds observer refusal to [`intersections_refined`].
pub fn intersections_refined_metered(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Vec<CurveIntersection>, GeoError> {
    intersect_with_goal(left, right, minimum_parameter_bits, context, Some(observer))
}

/// Complete off-diagonal contacts on one original edge. The enclosing graph
/// validates its immutable inventory once; local injectivity excludes the
/// diagonal and every remaining pair uses the same all-root solver.
pub(super) fn self_intersections_refined_in(
    edge: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let policy = context.policy();
    if minimum_parameter_bits == 0 || minimum_parameter_bits > policy.limits().max_precision_bits {
        return Err(GeoError::PrecisionExhausted {
            bits: policy.limits().max_precision_bits,
        });
    }
    let mut bits = 80
        .max(minimum_parameter_bits.saturating_mul(2).saturating_add(32))
        .min(policy.limits().max_precision_bits);
    loop {
        let result = with_line_refs_with_precision(
            &[edge],
            bits,
            minimum_parameter_bits > 32,
            context,
            progress,
            |lines, math, progress| {
                let unit = fixed_from_rat(&Rat::zero(), math)
                    .and_then(|zero| zero.hull(&fixed_from_rat(&Rat::one(), math)?, math))
                    .map_err(|e| geo_math_error(&e, policy))?;
                search_contacts_refined(
                    &lines[0],
                    &lines[0],
                    &[unit.clone(), unit],
                    &[],
                    ContactGoal {
                        minimum_parameter_bits,
                        policy,
                        common: None,
                        self_contact: true,
                    },
                    math,
                    progress,
                )
            },
        );
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result,
        }
    }
}

fn intersect(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    intersect_with_goal(left, right, 32, context, observer)
}

fn intersect_with_goal(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    check_bindings(&[left, right], context, &mut progress)?;
    intersections_refined_in(left, right, minimum_parameter_bits, context, &mut progress)
}

/// The caller has validated this immutable edge inventory against the active
/// context once. Private contact-graph pairs reuse that admission without
/// rendering the same reference again for every pair or local face.
pub(super) fn intersections_refined_in(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let source_bytes = source_workspace_bound([left, right].into_iter());
    context.admit_workspace(source_bytes)?;
    let result = intersect_admitted(left, right, minimum_parameter_bits, context, progress);
    context.release_workspace(source_bytes)?;
    result
}

pub(super) fn check_bindings(
    edges: &[&PreparedEdge],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    for edge in edges {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if edge.binding_id() != binding {
            return Err(crate::numerical::missing_operation_ids(
                edge.binding_id(),
                binding,
                context,
                progress,
            )?);
        }
    }
    Ok(())
}
pub(crate) fn source_workspace_bound<'a>(edges: impl Iterator<Item = &'a PreparedEdge>) -> u64 {
    edges.fold(65_536u64, |bytes, edge| {
        let bytes = endpoints(edge)
            .into_iter()
            .flatten()
            .fold(bytes, |bytes, point| {
                [point.longitude(), point.latitude()]
                    .into_iter()
                    .fold(bytes, |bytes, value| {
                        bytes
                            .saturating_add(
                                value
                                    .numerator()
                                    .bit_len()
                                    .saturating_add(value.denominator().bit_len())
                                    .div_ceil(8)
                                    .saturating_mul(32),
                            )
                            .saturating_add(value.numerator().allocated_bytes() as u64)
                            .saturating_add(value.denominator().allocated_bytes() as u64)
                    })
            });
        match edge {
            PreparedEdge::AzimuthLength(arc) => [arc.azimuth(), arc.length().exact()]
                .into_iter()
                .fold(bytes, |bytes, value| {
                    bytes
                        .saturating_add(
                            value
                                .numerator()
                                .bit_len()
                                .saturating_add(value.denominator().bit_len())
                                .div_ceil(8)
                                .saturating_mul(32),
                        )
                        .saturating_add(value.numerator().allocated_bytes() as u64)
                        .saturating_add(value.denominator().allocated_bytes() as u64)
                }),
            PreparedEdge::Transformed(image) => {
                bytes.saturating_add(image.retained_workspace_bytes())
            }
            _ => bytes,
        }
    })
}

fn intersect_admitted(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    if minimum_parameter_bits == 0
        || minimum_parameter_bits > context.policy().limits().max_precision_bits
    {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    if let (PreparedEdge::SourceLinear(a), PreparedEdge::SourceLinear(b)) = (left, right) {
        return linear_contacts(a, b, context, progress);
    }
    if let Some(overlap) = selected_branch_overlap(left, right, context, progress)? {
        admit_contact_count(1, context)?;
        return Ok(vec![overlap]);
    }
    let coincidence = coincident_admitted(left, right, context, progress)?;
    if let Some(reversed) = coincidence {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        admit_contact_count(1, context)?;
        return Ok(vec![CurveIntersection::Coincident { reversed }]);
    }
    intersect_noncoincident(left, right, minimum_parameter_bits, context, progress)
}

pub(super) fn coincident_admitted(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<bool>, GeoError> {
    let same_mapping = match (left, right) {
        (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) => {
            a.same_mapping_admitted(b, context, progress)?
        }
        _ => false,
    };
    let operands = coincidence_operands(left)
        .into_iter()
        .chain(coincidence_operands(right))
        .flatten()
        .collect::<purrdf_core::SmallVec<[&Rat; 20]>>();
    ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &operands,
        40,
        || Ok(coincident(left, right, same_mapping)),
    )
}

fn intersect_noncoincident(
    left: &PreparedEdge,
    right: &PreparedEdge,
    minimum_parameter_bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    if let Some(contacts) = symbolic_constant_contacts(left, right, context, progress)? {
        return Ok(contacts);
    }
    if let Some(contacts) = constant::intersections(left, right, context, progress)? {
        return Ok(contacts);
    }
    if let Some(contacts) = axis_overlap::intersections(left, right, context, progress)? {
        return refine_overlaps(
            contacts,
            [left, right],
            minimum_parameter_bits,
            context,
            progress,
        );
    }
    if let Some(contacts) = subset::intersections(left, right, context, progress)? {
        return refine_overlaps(
            contacts,
            [left, right],
            minimum_parameter_bits,
            context,
            progress,
        );
    }
    if let Some(contacts) = polynomial::intersections(left, right, context, progress)? {
        return Ok(contacts);
    }
    let mut retained = 0;
    context.retain_workspace(
        source_workspace_bound([left, right].into_iter()),
        &mut retained,
    )?;
    let common = source_join::prepare(left, right, context, progress, &mut retained);
    let result = match common {
        Err(error) => Err(error),
        Ok(mut common) => {
            let result = (|| {
                if let Some(source) = common.as_ref()
                    && source_join::complete(source, [left, right], context, progress)?
                {
                    admit_contact_count(1, context)?;
                    return Ok(vec![source_join::into_contact(
                        common.take().expect("proved common source"),
                    )]);
                }
                let endpoints = endpoint_contacts(left, right, context, progress)?;
                admit_contact_count(endpoints.len() as u64, context)?;
                if exact_axis_contacts(left, right, &endpoints, context, progress)? {
                    return Ok(endpoints);
                }
                let policy = context.policy();
                let mut bits = 80
                    .max(minimum_parameter_bits.saturating_mul(2).saturating_add(32))
                    .min(policy.limits().max_precision_bits);
                loop {
                    let result = attempt(
                        left,
                        right,
                        &endpoints,
                        ContactAttempt {
                            bits,
                            minimum_parameter_bits,
                            common: common.as_ref(),
                        },
                        context,
                        progress,
                    );
                    match result {
                        Err(GeoError::PrecisionExhausted { .. })
                            if bits < policy.limits().max_precision_bits =>
                        {
                            bits = bits
                                .saturating_mul(2)
                                .min(policy.limits().max_precision_bits);
                        }
                        result => return result,
                    }
                }
            })();
            drop(common);
            result
        }
    };
    context.release_workspace(retained)?;
    result
}

fn linear_contacts(
    a: &SourceLinearEdge,
    b: &SourceLinearEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let max_contacts = context.policy().limits().max_output_elements;
    let mut admission = ExactAdmission::new(context, progress);
    let a1 = chart_coordinate(a.start().point(), 0, &mut admission)?;
    let a2 = chart_coordinate(a.end().point(), 0, &mut admission)?;
    let mut output = Vec::new();
    for shift in [-360, 0, 360] {
        let b1 = chart_coordinate(b.start().point(), shift, &mut admission)?;
        let b2 = chart_coordinate(b.end().point(), shift, &mut admission)?;
        match crate::topology::intersect_admitted(&a1, &a2, &b1, &b2, &mut admission)? {
            SegmentIntersection::None => {}
            SegmentIntersection::Point(point) => {
                let (left, right, point) = linear_parameters(&point, shift, a, b, &mut admission)?;
                let contact = CurveIntersection::Exact { left, right, point };
                let mut repeated = false;
                for prior in &output {
                    let operands = exact_contact_operands(prior)
                        .into_iter()
                        .flatten()
                        .chain(exact_contact_operands(&contact).into_iter().flatten())
                        .collect::<Vec<_>>();
                    if admission.rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &operands,
                        operands.len() as u64,
                        || Ok(prior == &contact),
                    )? {
                        repeated = true;
                        break;
                    }
                }
                if !repeated {
                    // The source guard admits this bounded original contact
                    // inventory; the output cap is checked before each push.
                    if output.len() as u64 + 1 > max_contacts {
                        return Err(GeoError::OutputExhausted {
                            limit: max_contacts,
                        });
                    }
                    output.push(contact);
                }
            }
            SegmentIntersection::Collinear { from, to } => {
                let (a0, b0, start) = linear_parameters(&from, shift, a, b, &mut admission)?;
                let (a1, b1, end) = linear_parameters(&to, shift, a, b, &mut admission)?;
                if output.len() as u64 + 1 > max_contacts {
                    return Err(GeoError::OutputExhausted {
                        limit: max_contacts,
                    });
                }
                output.push(CurveIntersection::Overlap {
                    left: (a0, a1),
                    right: (b0, b1),
                    start,
                    end,
                });
            }
        }
    }
    // A physical pole is identified independently of written chart longitude.
    // Borrow original endpoints; no cloned edge or boxed source graph is needed.
    for (i, left) in [a.start().point(), a.end().point()].into_iter().enumerate() {
        for (j, right) in [b.start().point(), b.end().point()].into_iter().enumerate() {
            let pole_equal = admission.rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[
                    left.longitude(),
                    left.latitude(),
                    right.longitude(),
                    right.latitude(),
                ],
                4,
                || Ok(left.is_pole() && left.same_location(right)),
            )?;
            if !pole_equal {
                continue;
            }
            let mut repeated = false;
            for item in &output {
                let (CurveIntersection::Exact { point, .. }
                | CurveIntersection::Endpoint { point, .. }) = item
                else {
                    continue;
                };
                if admission.rational(
                    purrdf_xsd::integer::ExactOperation::RationalCompare,
                    &[
                        left.longitude(),
                        left.latitude(),
                        point.longitude(),
                        point.latitude(),
                    ],
                    4,
                    || Ok(left.same_location(point)),
                )? {
                    repeated = true;
                    break;
                }
            }
            if repeated {
                continue;
            }
            if output.len() as u64 + 1 > max_contacts {
                return Err(GeoError::OutputExhausted {
                    limit: max_contacts,
                });
            }
            let point = admission.rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[left.longitude(), left.latitude()],
                2,
                || Ok(left.clone()),
            )?;
            output.push(CurveIntersection::Exact {
                left: Rat::from_i64(i as i64),
                right: Rat::from_i64(j as i64),
                point,
            });
        }
    }
    Ok(output)
}

fn refine_overlaps(
    mut contacts: Vec<CurveIntersection>,
    edges: [&PreparedEdge; 2],
    goal: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    if goal <= 32 {
        return Ok(contacts);
    }
    let limit = context.policy().limits().max_precision_bits;
    context.charge_work(1)?;
    progress.context_poll(context)?;
    let width = Rat::new(crate::Int::one(), crate::Int::one().shl(goal))
        .ok_or(GeoError::ArithmeticOverflow("contact parameter quantum"))?;
    for contact in &mut contacts {
        let CurveIntersection::GeodesicOverlap {
            left,
            right,
            start,
            end,
        } = contact
        else {
            continue;
        };
        let mut bits = 80.max(goal.saturating_mul(2).saturating_add(32)).min(limit);
        loop {
            let first =
                refined_axis_parameters(edges[0], [start, end], left, bits, context, progress)?;
            let second =
                refined_axis_parameters(edges[1], [start, end], right, bits, context, progress)?;
            let mut decisive = true;
            for (lower, upper) in first.iter().chain(&second) {
                let mut admission = ExactAdmission::new(context, progress);
                let delta = crate::numerical::exact_rational(
                    Some(&mut admission),
                    purrdf_xsd::integer::ExactOperation::RationalAdd,
                    &[lower, upper],
                    || upper.sub(lower),
                )?;
                decisive &= admission.rational(
                    purrdf_xsd::integer::ExactOperation::RationalCompare,
                    &[&delta, &width],
                    1,
                    || Ok(delta <= width),
                )?;
            }
            if decisive {
                *left = first;
                **right = second;
                break;
            }
            if bits == limit {
                return Err(GeoError::PrecisionExhausted { bits });
            }
            bits = bits.saturating_mul(2).min(limit);
        }
    }
    Ok(contacts)
}

fn refined_axis_parameters(
    edge: &PreparedEdge,
    endpoints: [&LonLat; 2],
    retained: &[(Rat, Rat); 2],
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[(Rat, Rat); 2], GeoError> {
    if let PreparedEdge::ShortestGeodesic(arc) = edge {
        let values = endpoints.map(|point| {
            axis_overlap::parameter_with_precision(arc, point, Some(bits), context, progress)
        });
        let [first, last] = values;
        return Ok([first?, last?]);
    }
    // Written phases were solved exactly on the original affine axis. Preserve
    // their selected phase, including both visits at a full-turn seam endpoint.
    let operands = [
        &retained[0].0,
        &retained[0].1,
        &retained[1].0,
        &retained[1].1,
    ];
    ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &operands,
        4,
        || Ok(retained.clone()),
    )
}
fn exact_contact_operands(contact: &CurveIntersection) -> [Option<&Rat>; 8] {
    match contact {
        CurveIntersection::Exact { left, right, point }
        | CurveIntersection::Endpoint { left, right, point } => [
            Some(left),
            Some(right),
            Some(point.longitude()),
            Some(point.latitude()),
            None,
            None,
            None,
            None,
        ],
        CurveIntersection::Overlap {
            left: (a, b),
            right: (c, d),
            start,
            end,
        } => [
            Some(a),
            Some(b),
            Some(c),
            Some(d),
            Some(start.longitude()),
            Some(start.latitude()),
            Some(end.longitude()),
            Some(end.latitude()),
        ],
        _ => [None; 8],
    }
}
fn chart_coordinate(
    point: &LonLat,
    shift: i64,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<Coord, GeoError> {
    let shift = Rat::from_i64(shift);
    let longitude = crate::numerical::exact_rational(
        Some(admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[point.longitude(), &shift],
        || point.longitude().add(&shift),
    )?;
    let latitude = admission.rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[point.latitude()],
        1,
        || Ok(point.latitude().clone()),
    )?;
    Ok(Coord::xy(longitude, latitude))
}
fn linear_parameters(
    point: &Coord,
    shift: i64,
    a: &SourceLinearEdge,
    b: &SourceLinearEdge,
    admission: &mut ExactAdmission<'_, '_>,
) -> Result<(Rat, Rat, LonLat), GeoError> {
    let left = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.x(), point.y()],
        4,
        || LonLat::new(point.x().clone(), point.y().clone()),
    )?;
    let shift = Rat::from_i64(shift);
    let longitude = crate::numerical::exact_rational(
        Some(admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[point.x(), &shift],
        || point.x().sub(&shift),
    )?;
    let right = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[&longitude, point.y()],
        4,
        || LonLat::new(longitude.clone(), point.y().clone()),
    )?;
    let left_parameter = SourceLinearEdge::parameter_on_admitted(
        a.start().point(),
        a.end().point(),
        &left,
        admission,
    )?
    .ok_or_else(|| GeoError::domain("proved source-linear contact parameter"))?;
    let right_parameter = SourceLinearEdge::parameter_on_admitted(
        b.start().point(),
        b.end().point(),
        &right,
        admission,
    )?
    .ok_or_else(|| GeoError::domain("proved source-linear contact parameter"))?;
    Ok((left_parameter, right_parameter, left))
}

fn admit_contact_count(count: u64, context: &MetricContext) -> Result<(), GeoError> {
    if count > context.policy().limits().max_output_elements {
        return Err(GeoError::OutputExhausted {
            limit: context.policy().limits().max_output_elements,
        });
    }
    Ok(())
}

fn symbolic_constant_contacts(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Vec<CurveIntersection>>, GeoError> {
    let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right) else {
        return Ok(None);
    };
    if !a.same_mapping_admitted(b, context, progress)? {
        return Ok(None);
    }
    for (point, image, other) in [(a, b, right), (b, a, left)] {
        let (p0, p1) = point.source_endpoints();
        let (a, b) = image.source_endpoints();
        let operands = [
            Some(p0.x()),
            Some(p0.y()),
            p0.z(),
            Some(p1.x()),
            Some(p1.y()),
            p1.z(),
        ]
        .into_iter()
        .flatten()
        .collect::<purrdf_core::SmallVec<[&Rat; 6]>>();
        let constant = ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &operands,
            3,
            || Ok(p0.same_planar(p1) && p0.z() == p1.z()),
        )?;
        if !constant {
            continue;
        }
        let mut admission = ExactAdmission::new(context, progress);
        let Some(parameter) =
            SourceLinearEdge::parameter_on_coord_admitted(a, b, p0, &mut admission)?
        else {
            continue;
        };
        let z = match (a.z(), b.z()) {
            (Some(a), Some(b)) => Some(SourceLinearEdge::interpolate_ordinate_admitted(
                a,
                b,
                &parameter,
                &mut admission,
            )?),
            (None, None) => None,
            _ => return Err(GeoError::MissingHeight),
        };
        let heights = z
            .iter()
            .chain(p0.z())
            .collect::<purrdf_core::SmallVec<[&Rat; 2]>>();
        if !admission.rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &heights,
            1,
            || Ok(z.as_ref() == p0.z()),
        )? {
            continue;
        }
        // Exact common-mapping source equality supplies existence. A complete
        // injectivity witness supplies uniqueness on the nonconstant image.
        // Original symbolic endpoint/contact witnesses retain the constant's
        // identified endpoint domain without fabricating a target coordinate.
        if crate::atlas::point::image_injective(other, context, progress)? {
            return endpoint_contacts(left, right, context, progress).map(Some);
        }
    }
    Ok(None)
}

fn endpoints(edge: &PreparedEdge) -> [Option<&LonLat>; 2] {
    [
        edge.start().map(crate::PreparedCoordinate::point),
        match edge {
            PreparedEdge::SourceLinear(edge) => Some(edge.end().point()),
            PreparedEdge::ShortestGeodesic(arc) => Some(arc.end().point()),
            PreparedEdge::AzimuthLength(_) => None,
            PreparedEdge::Transformed(_) => None,
        },
    ]
}
fn endpoint_contacts(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let same_mapping = match (left, right) {
        (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) => {
            a.same_mapping_admitted(b, context, progress)?
        }
        _ => false,
    };
    // A polynomial fold maps distinct original endpoints to one image point.
    // The certified even-parity reflection of the left source proves that
    // pointwise equality; matched reflected endpoints are exact contacts.
    let mut reflected_retained = 0;
    let reflected = match (left, right) {
        (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) if same_mapping => {
            a.reflected_source_towards_in(b, context, progress, &mut reflected_retained)?
        }
        _ => None,
    };
    let result = endpoint_contacts_with(
        left,
        right,
        same_mapping,
        reflected.as_ref(),
        context,
        progress,
    );
    drop(reflected);
    context.release_workspace(reflected_retained)?;
    result
}

fn endpoint_contacts_with(
    left: &PreparedEdge,
    right: &PreparedEdge,
    same_mapping: bool,
    reflected: Option<&crate::operation::OperationImageCurve>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};
    let maximum = context.policy().limits().max_output_elements;
    let mut admission = ExactAdmission::new(context, progress);
    let mut output = Vec::new();
    if let (Some(a), PreparedEdge::Transformed(b)) = (reflected, right) {
        // The reflected source keeps the original parameterization, so an
        // exact crossing of the two original lines is an exact image contact.
        let (a0, a1) = a.source_endpoints();
        let (b0, b1) = b.source_endpoints();
        if let SegmentIntersection::Point(point) =
            crate::topology::intersect_admitted(a0, a1, b0, b1, &mut admission)?
        {
            let first =
                SourceLinearEdge::parameter_on_coord_admitted(a0, a1, &point, &mut admission)?
                    .ok_or_else(|| GeoError::domain("proved reflected source contact parameter"))?;
            let second =
                SourceLinearEdge::parameter_on_coord_admitted(b0, b1, &point, &mut admission)?
                    .ok_or_else(|| GeoError::domain("proved reflected source contact parameter"))?;
            let endpoint = |value: &Rat| value.is_zero() || value == &Rat::one();
            // Endpoint pairs are recorded once below, as symbolic endpoints.
            if !(endpoint(&first) && endpoint(&second)) {
                if output.len() as u64 >= maximum {
                    return Err(GeoError::OutputExhausted { limit: maximum });
                }
                output.push(CurveIntersection::SymbolicSourceContact {
                    left: first,
                    right: second,
                });
            }
        }
        let a: [_; 2] = a.source_endpoints().into();
        let b: [_; 2] = b.source_endpoints().into();
        for (i, a) in a.into_iter().enumerate() {
            for (j, b) in b.into_iter().enumerate() {
                let operands = [
                    Some(a.x()),
                    Some(a.y()),
                    a.z(),
                    Some(b.x()),
                    Some(b.y()),
                    b.z(),
                ]
                .into_iter()
                .flatten()
                .collect::<purrdf_core::SmallVec<[&Rat; 6]>>();
                if admission.rational(RationalCompare, &operands, 3, || {
                    Ok(a.same_planar(b) && a.z() == b.z())
                })? {
                    if output.len() as u64 >= maximum {
                        return Err(GeoError::OutputExhausted { limit: maximum });
                    }
                    output.push(CurveIntersection::SymbolicEndpoint {
                        left: Rat::from_i64(i as i64),
                        right: Rat::from_i64(j as i64),
                    });
                }
            }
        }
    }
    if let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right)
        && same_mapping
    {
        let a: [_; 2] = a.source_endpoints().into();
        let b: [_; 2] = b.source_endpoints().into();
        for (i, a) in a.into_iter().enumerate() {
            for (j, b) in b.into_iter().enumerate() {
                let operands = [
                    Some(a.x()),
                    Some(a.y()),
                    a.z(),
                    Some(b.x()),
                    Some(b.y()),
                    b.z(),
                ]
                .into_iter()
                .flatten()
                .collect::<purrdf_core::SmallVec<[&Rat; 6]>>();
                let known = output.iter().any(|contact| {
                    matches!(contact, CurveIntersection::SymbolicEndpoint { left, right }
                        if *left == Rat::from_i64(i as i64) && *right == Rat::from_i64(j as i64))
                });
                if !known
                    && admission.rational(RationalCompare, &operands, 3, || {
                        Ok(a.same_planar(b) && a.z() == b.z())
                    })?
                {
                    if output.len() as u64 >= maximum {
                        return Err(GeoError::OutputExhausted { limit: maximum });
                    }
                    output.push(CurveIntersection::SymbolicEndpoint {
                        left: Rat::from_i64(i as i64),
                        right: Rat::from_i64(j as i64),
                    });
                }
            }
        }
    }
    if let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right)
        && same_mapping
    {
        let (a0, a1) = a.source_endpoints();
        let (b0, b1) = b.source_endpoints();
        if let SegmentIntersection::Point(point) =
            crate::topology::intersect_admitted(a0, a1, b0, b1, &mut admission)?
        {
            let first =
                SourceLinearEdge::parameter_on_coord_admitted(a0, a1, &point, &mut admission)?
                    .ok_or_else(|| GeoError::domain("proved original source contact parameter"))?;
            let second =
                SourceLinearEdge::parameter_on_coord_admitted(b0, b1, &point, &mut admission)?
                    .ok_or_else(|| GeoError::domain("proved original source contact parameter"))?;
            let height = |start: &Coord,
                          end: &Coord,
                          parameter: &Rat,
                          admission: &mut ExactAdmission<'_, '_>|
             -> Result<Option<Rat>, GeoError> {
                match (start.z(), end.z()) {
                    (Some(start), Some(end)) => SourceLinearEdge::interpolate_ordinate_admitted(
                        start, end, parameter, admission,
                    )
                    .map(Some),
                    (None, None) => Ok(None),
                    _ => Err(GeoError::MissingHeight),
                }
            };
            let h0 = height(a0, a1, &first, &mut admission)?;
            let h1 = height(b0, b1, &second, &mut admission)?;
            let operands = h0
                .iter()
                .chain(h1.iter())
                .collect::<purrdf_core::SmallVec<[&Rat; 2]>>();
            if admission.rational(Linear, &operands, 2, || Ok(h0 == h1))? {
                if output.len() as u64 >= maximum {
                    return Err(GeoError::OutputExhausted { limit: maximum });
                }
                output.push(CurveIntersection::SymbolicSourceContact {
                    left: first,
                    right: second,
                });
            }
        }
    }
    for (i, a) in endpoints(left).into_iter().enumerate() {
        for (j, b) in endpoints(right).into_iter().enumerate() {
            let (Some(a), Some(b)) = (a, b) else {
                continue;
            };
            if admission.rational(
                RationalCompare,
                &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
                4,
                || Ok(a.same_location(b)),
            )? {
                if output.len() as u64 >= maximum {
                    return Err(GeoError::OutputExhausted { limit: maximum });
                }
                let point =
                    admission
                        .rational(Linear, &[a.longitude(), a.latitude()], 2, || Ok(a.clone()))?;
                output.push(CurveIntersection::Endpoint {
                    left: Rat::from_i64(i as i64),
                    right: Rat::from_i64(j as i64),
                    point,
                });
            }
        }
    }
    Ok(output)
}
/// Reflection invariance and the certified unique shortest branch place these
/// arcs exactly on the equator or a meridian plane. Distinct axis planes have
/// only the two explicitly constructed intersection candidates; testing every
/// candidate through the shared original point law excludes all other contacts.
fn exact_axis_contacts(
    left: &PreparedEdge,
    right: &PreparedEdge,
    endpoints: &[CurveIntersection],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let (PreparedEdge::ShortestGeodesic(a), PreparedEdge::ShortestGeodesic(b)) = (left, right)
    else {
        return Ok(false);
    };
    let axis_a = super::point::shortest_axis(a, context, progress)?;
    let axis_b = super::point::shortest_axis(b, context, progress)?;
    let candidates = match (axis_a, axis_b) {
        (
            Some(super::point::ShortestAxis::Meridian(a)),
            Some(super::point::ShortestAxis::Meridian(b)),
        ) => {
            let mut admission = ExactAdmission::new(context, progress);
            let delta = crate::numerical::exact_rational(
                Some(&mut admission),
                purrdf_xsd::integer::ExactOperation::RationalAdd,
                &[&a, &b],
                || a.sub(&b),
            )?;
            if admission.rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[&delta],
                3,
                || {
                    Ok(delta.is_zero()
                        || delta.abs() == Rat::from_i64(180)
                        || delta.abs() == Rat::from_i64(360))
                },
            )? {
                return Ok(false);
            }
            [
                LonLat::new(Rat::zero(), Rat::from_i64(-90))?,
                LonLat::new(Rat::zero(), Rat::from_i64(90))?,
            ]
        }
        (
            Some(super::point::ShortestAxis::Equator),
            Some(super::point::ShortestAxis::Meridian(longitude)),
        )
        | (
            Some(super::point::ShortestAxis::Meridian(longitude)),
            Some(super::point::ShortestAxis::Equator),
        ) => {
            let longitude = super::point::normalize_admitted(&longitude, context, progress)?;
            let mut admission = ExactAdmission::new(context, progress);
            let other = crate::numerical::exact_rational(
                Some(&mut admission),
                purrdf_xsd::integer::ExactOperation::RationalAdd,
                &[&longitude],
                || longitude.sub(&Rat::from_i64(180)),
            )?;
            let longitude = if admission.rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[&longitude],
                1,
                || Ok(longitude > Rat::from_i64(180)),
            )? {
                crate::numerical::exact_rational(
                    Some(&mut admission),
                    purrdf_xsd::integer::ExactOperation::RationalAdd,
                    &[&longitude],
                    || longitude.sub(&Rat::from_i64(360)),
                )?
            } else {
                longitude
            };
            let cost = crate::numerical::rational_cost(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[&longitude, &other],
                8,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "axis candidate coordinate admission",
            ))?;
            progress.exact(context, cost, || {
                Ok([
                    LonLat::new(longitude, Rat::zero())?,
                    LonLat::new(other, Rat::zero())?,
                ])
            })?
        }
        _ => return Ok(false),
    };
    for candidate in &candidates {
        let a = super::point::exact_source(left, candidate, context, progress)?;
        let b = super::point::exact_source(right, candidate, context, progress)?;
        if a == Some(false) || b == Some(false) {
            continue;
        }
        if a != Some(true) || b != Some(true) {
            return Ok(false);
        }
        let mut known = false;
        for endpoint in endpoints {
            let CurveIntersection::Endpoint { point, .. } = endpoint else {
                continue;
            };
            if ExactAdmission::new(context, progress).rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[
                    candidate.longitude(),
                    candidate.latitude(),
                    point.longitude(),
                    point.latitude(),
                ],
                4,
                || Ok(point.same_location(candidate)),
            )? {
                known = true;
                break;
            }
        }
        if !known {
            return Ok(false);
        }
    }
    Ok(true)
}
fn selected_branch_overlap(
    left: &PreparedEdge,
    right: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<CurveIntersection>, GeoError> {
    let (PreparedEdge::AzimuthLength(left), PreparedEdge::AzimuthLength(right)) = (left, right)
    else {
        return Ok(None);
    };
    let same_start = ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[
            left.start().point().longitude(),
            left.start().point().latitude(),
            right.start().point().longitude(),
            right.start().point().latitude(),
        ],
        4,
        || Ok(left.start().point().same_location(right.start().point())),
    )?;
    if !same_start || left.length().exact().is_zero() || right.length().exact().is_zero() {
        return Ok(None);
    }
    let forward_left = super::point::normalize_admitted(left.azimuth(), context, progress)?;
    let forward_right = super::point::normalize_admitted(right.azimuth(), context, progress)?;
    if forward_left != forward_right {
        return Ok(None);
    }
    let mut admission = ExactAdmission::new(context, progress);
    let order = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[left.length().exact(), right.length().exact()],
        1,
        || Ok(left.length().exact().cmp(right.length().exact())),
    )?;
    if order == core::cmp::Ordering::Equal {
        return Ok(Some(CurveIntersection::Coincident { reversed: false }));
    }
    let minimum = if order == core::cmp::Ordering::Less {
        left.length().exact()
    } else {
        right.length().exact()
    };
    let mut endpoint = |length: &Rat| {
        admission.rational(
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[minimum, length],
            1,
            || {
                minimum
                    .div(length)
                    .ok_or_else(|| GeoError::domain("positive selected branch length"))
            },
        )
    };
    Ok(Some(CurveIntersection::BranchOverlap {
        left: (Rat::zero(), endpoint(left.length().exact())?),
        right: (Rat::zero(), endpoint(right.length().exact())?),
    }))
}

fn coincident(left: &PreparedEdge, right: &PreparedEdge, same_mapping: bool) -> Option<bool> {
    match (left, right) {
        (PreparedEdge::SourceLinear(a), PreparedEdge::SourceLinear(b)) => {
            if a.start().point() == b.start().point() && a.end().point() == b.end().point() {
                Some(false)
            } else if a.start().point() == b.end().point() && a.end().point() == b.start().point() {
                Some(true)
            } else {
                None
            }
        }
        (PreparedEdge::ShortestGeodesic(a), PreparedEdge::ShortestGeodesic(b)) => {
            if a.start().point().same_location(b.start().point())
                && a.end().point().same_location(b.end().point())
            {
                Some(false)
            } else if a.start().point().same_location(b.end().point())
                && a.end().point().same_location(b.start().point())
            {
                Some(true)
            } else {
                None
            }
        }
        (PreparedEdge::AzimuthLength(a), PreparedEdge::AzimuthLength(b))
            if a.start().point() == b.start().point()
                && a.azimuth() == b.azimuth()
                && a.length() == b.length() =>
        {
            Some(false)
        }
        (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) if same_mapping => {
            let (a0, a1) = a.source_endpoints();
            let (b0, b1) = b.source_endpoints();
            let equal = |a: &Coord, b: &Coord| a.same_planar(b) && a.z() == b.z();
            if equal(a0, b0) && equal(a1, b1) {
                Some(false)
            } else if equal(a0, b1) && equal(a1, b0) {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}
fn coincidence_operands(edge: &PreparedEdge) -> [Option<&Rat>; 10] {
    if let PreparedEdge::Transformed(image) = edge {
        let (start, end) = image.source_endpoints();
        return [
            Some(start.x()),
            Some(start.y()),
            start.z(),
            start.m(),
            Some(end.x()),
            Some(end.y()),
            end.z(),
            end.m(),
            image.exact_epoch(),
            None,
        ];
    }
    let [start, end] = endpoints(edge);
    let (azimuth, length) = if let PreparedEdge::AzimuthLength(arc) = edge {
        (Some(arc.azimuth()), Some(arc.length().exact()))
    } else {
        (None, None)
    };
    [
        start.map(LonLat::longitude),
        start.map(LonLat::latitude),
        end.map(LonLat::longitude),
        end.map(LonLat::latitude),
        azimuth,
        length,
        None,
        None,
        None,
        None,
    ]
}

pub(crate) enum ChartLine {
    Linear(Box<LinearChart>),
    Geodesic(Box<FixedArcLine>),
    Transformed {
        curve: Box<crate::operation::OperationImageCurve>,
        subdivisions: u32,
    },
}
pub(crate) struct LinearChart {
    start: [FixedInterval; 2],
    delta: [FixedInterval; 2],
}
type AngularNormalImage<T> = ([T; 3], Option<[T; 3]>);
pub(crate) type NormalImage = AngularNormalImage<FixedInterval>;
impl ChartLine {
    pub(crate) fn normal(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<NormalImage, MathError> {
        if let Self::Geodesic(line) = self {
            let image = line.continuation_enclosure_in(parameter, iterations, math, progress)?;
            return Ok((image.normal, Some(image.normal_derivative)));
        }
        if let Self::Transformed {
            curve,
            subdivisions,
        } = self
        {
            let (lower, upper) = exact_bounds(parameter);
            if let Some(normal) = curve.geocentric_normal_in(
                &lower,
                &upper,
                crate::operation::OperationSolverLimits {
                    iterations,
                    subdivisions: *subdivisions,
                    quantize_inverse: false,
                },
                math,
                progress,
            )? {
                return Ok(normal);
            }
        }
        let ([longitude, latitude], derivative) =
            self.image(parameter, iterations, math, progress)?;
        normal_from_angles(&longitude, &latitude, derivative, math)
    }
    pub(super) fn prepare(
        edge: &PreparedEdge,
        view: Option<ArcIntervalView>,
        image: Option<Box<crate::operation::OperationImageCurve>>,
        context: &MetricContext,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        if matches!(edge, PreparedEdge::Transformed(_)) {
            return Ok(Self::Transformed {
                curve: image.ok_or_else(|| GeoError::domain("admitted original image owner"))?,
                subdivisions: context.policy().limits().max_subdivision_levels,
            });
        }
        if let PreparedEdge::SourceLinear(edge) = edge {
            let radians = FixedInterval::pi(math)
                .and_then(|pi| pi.div(&FixedInterval::from_i64(180, math)?, math))
                .map_err(|error| geo_math_error(&error, context.policy()))?;
            let source = [
                edge.start().point().longitude(),
                edge.start().point().latitude(),
            ];
            let target = [
                edge.end().point().longitude(),
                edge.end().point().latitude(),
            ];
            let start = source.map(|value| {
                fixed_from_rat(value, math).and_then(|value| value.mul(&radians, math))
            });
            let [x, y] = start;
            let delta = [0, 1].map(|axis| {
                let value = crate::numerical::math_exact_rational(
                    purrdf_xsd::integer::ExactOperation::RationalAdd,
                    &[target[axis], source[axis]],
                    math,
                    progress,
                    || target[axis].sub(source[axis]),
                )?;
                fixed_from_rat(&value, math).and_then(|value| value.mul(&radians, math))
            });
            let [dx, dy] = delta;
            let policy = context.policy();
            return Ok(Self::Linear(Box::new(LinearChart {
                start: [
                    x.map_err(|error| geo_math_error(&error, policy))?,
                    y.map_err(|error| geo_math_error(&error, policy))?,
                ],
                delta: [
                    dx.map_err(|error| geo_math_error(&error, policy))?,
                    dy.map_err(|error| geo_math_error(&error, policy))?,
                ],
            })));
        }
        let line = view
            .ok_or_else(|| GeoError::domain("expected selected geodesic edge"))?
            .prepare_in(math, progress)
            .map_err(|error| geo_math_error(&error, context.policy()))?;
        Ok(Self::Geodesic(Box::new(line)))
    }
    pub(super) fn normal_taylor_in<'scope>(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        workspace: &mut TaylorWorkspace<'scope>,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<crate::geodesic::normal::NormalImage<TaylorJet<'scope>>>, MathError> {
        if let Self::Geodesic(line) = self {
            return line
                .normal_taylor_in(parameter, iterations, workspace, math, progress)
                .map(Some);
        }
        if let Self::Linear(_) = self {
            let ([longitude, latitude], derivative) =
                self.image(parameter, iterations, math, progress)?;
            let [dl, dp] = derivative.ok_or(MathError::PrecisionExhausted)?;
            let longitude = workspace.argument(longitude, dl.clone(), math)?;
            let latitude = workspace.argument(latitude, dp.clone(), math)?;
            let dl = workspace.constant(dl, math)?;
            let dp = workspace.constant(dp, math)?;
            let (normal, derivative) = normal_from_angles_with(
                &longitude,
                &latitude,
                Some([dl, dp]),
                &mut TaylorArithmetic { workspace, math },
            )?;
            return Ok(Some((
                normal,
                derivative.ok_or(MathError::PrecisionExhausted)?,
            )));
        }
        Ok(None)
    }
    pub(crate) fn image(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<([FixedInterval; 2], Option<[FixedInterval; 2]>), MathError> {
        match self {
            Self::Linear(line) => {
                let LinearChart { start, delta } = line.as_ref();
                Ok((
                    [
                        start[0].add(&delta[0].mul(parameter, math)?, math)?,
                        start[1].add(&delta[1].mul(parameter, math)?, math)?,
                    ],
                    Some(delta.clone()),
                ))
            }
            Self::Geodesic(line) => {
                let image =
                    line.continuation_enclosure_in(parameter, iterations, math, progress)?;
                Ok(([image.longitude, image.latitude], image.derivative))
            }
            Self::Transformed {
                curve,
                subdivisions,
            } => {
                let (lower, upper) = exact_bounds(parameter);
                let image = curve.enclosure_in(
                    &lower,
                    &upper,
                    crate::operation::OperationSolverLimits {
                        iterations,
                        subdivisions: *subdivisions,
                        quantize_inverse: false,
                    },
                    math,
                    progress,
                )?;
                let radians =
                    FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
                let derivative = image
                    .derivative
                    .map(|[x, y]| {
                        Ok::<_, MathError>([x.mul(&radians, math)?, y.mul(&radians, math)?])
                    })
                    .transpose()?;
                Ok((
                    [
                        image.longitude.mul(&radians, math)?,
                        image.latitude.mul(&radians, math)?,
                    ],
                    derivative,
                ))
            }
        }
    }
}

pub(super) fn normal_from_angles(
    longitude: &FixedInterval,
    latitude: &FixedInterval,
    derivative: Option<[FixedInterval; 2]>,
    math: &mut CoordinateMath,
) -> Result<NormalImage, MathError> {
    normal_from_angles_with(longitude, latitude, derivative, &mut FixedArithmetic(math))
}
fn normal_from_angles_with<A: NormalArithmetic>(
    longitude: &A::Value,
    latitude: &A::Value,
    derivative: Option<[A::Value; 2]>,
    arithmetic: &mut A,
) -> Result<AngularNormalImage<A::Value>, MathError> {
    let (sl, cl) = arithmetic.sin_cos(longitude)?;
    let (sp, cp) = arithmetic.sin_cos(latitude)?;
    let normal = [
        arithmetic.mul(&cp, &cl)?,
        arithmetic.mul(&cp, &sl)?,
        sp.clone(),
    ];
    let derivative = if let Some([dl, dp]) = derivative {
        let sp_cl = arithmetic.mul(&sp, &cl)?;
        let sp_sl = arithmetic.mul(&sp, &sl)?;
        let x_vertical = arithmetic.mul(&sp_cl, &dp)?;
        let x_vertical = arithmetic.neg(&x_vertical)?;
        let x_horizontal = arithmetic.mul(&normal[1], &dl)?;
        let y_vertical = arithmetic.mul(&sp_sl, &dp)?;
        let y_vertical = arithmetic.neg(&y_vertical)?;
        let y_horizontal = arithmetic.mul(&normal[0], &dl)?;
        Some([
            arithmetic.sub(&x_vertical, &x_horizontal)?,
            arithmetic.add(&y_vertical, &y_horizontal)?,
            arithmetic.mul(&cp, &dp)?,
        ])
    } else {
        None
    };
    Ok((normal, derivative))
}

fn prepare_view_with_precision(
    edge: &PreparedEdge,
    bits: u32,
    strict: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<ArcIntervalView>, GeoError> {
    let mut child = context.remaining_child()?;
    let result = {
        let mut nested = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        edge.geodesic_view_metered(bits, &mut child, &mut nested)
    };
    let Some(view) = progress.absorb_child_result(context, &child, result)? else {
        return Ok(None);
    };
    if !strict {
        return Ok(Some(view));
    }
    let retained = view.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    let result = (|| {
        let mut child = context.remaining_child()?;
        let result = {
            let mut nested = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            view.refined(bits, &mut child, Some(&mut nested))
        };
        progress
            .absorb_child_result(context, &child, result)
            .map(Some)
    })();
    context.release_workspace(retained)?;
    result
}

/// Prepare original images once and account both view and line storage while
/// one shared numerical phase evaluates the complete borrowed line inventory.
pub(crate) fn with_lines<T>(
    edges: &[PreparedEdge],
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &[ChartLine],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let bytes = (edges.len() as u64)
        .checked_mul(size_of::<&PreparedEdge>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("borrowed line inventory"))?;
    context.admit_workspace(bytes)?;
    let borrowed = edges.iter().collect::<Vec<_>>();
    let result = with_line_refs(&borrowed, bits, context, progress, evaluate);
    drop(borrowed);
    context.release_workspace(bytes)?;
    result
}
pub(crate) fn with_line_refs<T>(
    edges: &[&PreparedEdge],
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &[ChartLine],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    with_line_refs_with_precision(edges, bits, false, context, progress, evaluate)
}

pub(super) fn with_line_refs_with_precision<T>(
    edges: &[&PreparedEdge],
    bits: u32,
    strict: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &[ChartLine],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    with_line_refs_retained(
        edges,
        bits,
        strict,
        context,
        progress,
        |lines, math, progress| evaluate(lines, math, progress).map(|value| (value, 0)),
    )
    .map(|(value, _)| value)
}

pub(super) fn with_line_refs_retained<T>(
    edges: &[&PreparedEdge],
    bits: u32,
    strict: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &[ChartLine],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<(T, u64), GeoError>,
) -> Result<(T, u64), GeoError> {
    context.charge_work(edges.len() as u64)?;
    progress.context_poll(context)?;
    let source_bits = edges
        .iter()
        .map(|edge| edge.max_original_operand_bits())
        .max()
        .unwrap_or(0)
        .max(crate::numerical::scratch_source_bits(
            &[],
            context.reference().ellipsoid(),
        ));
    context.prepare_integer_scratch_for_observed(source_bits, progress)?;
    let container = (edges.len() as u64)
        .checked_mul(size_of::<(
            Option<ArcIntervalView>,
            Option<Box<crate::operation::OperationImageCurve>>,
        )>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("curve view storage"))?;
    context.admit_workspace(container)?;
    let mut retained = container;
    let result = (|| {
        let mut views = Vec::with_capacity(edges.len());
        for edge in edges {
            let image = if matches!(edge, PreparedEdge::Transformed(_)) {
                let PreparedEdge::Transformed(image) =
                    edge.clone_admitted(context, progress, &mut retained)?
                else {
                    unreachable!("same cloned edge variant");
                };
                Some(image)
            } else {
                None
            };
            let view = prepare_view_with_precision(edge, bits, strict, context, progress)?;
            let bytes = view
                .as_ref()
                .map_or(0, ArcIntervalView::retained_workspace_bytes);
            context.admit_workspace(bytes)?;
            retained = retained
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow("curve view storage"))?;
            views.push((view, image));
        }
        let line_context = context.remaining_child()?;
        let reservation = usize::try_from(
            retained
                .saturating_mul(2)
                .saturating_add(source_workspace_bound(edges.iter().copied()))
                .saturating_add(
                    (edges.len() as u64)
                        .saturating_mul(u64::from(bits.div_ceil(8)) + 64)
                        .saturating_mul(8192),
                ),
        )
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
        crate::numerical::with_math_for_sources_retained(
            context,
            progress,
            bits,
            reservation,
            &[],
            |math, progress| {
                let mut lines = Vec::with_capacity(edges.len());
                for (edge, (view, image)) in edges.iter().zip(views) {
                    lines.push(ChartLine::prepare(
                        edge,
                        view,
                        image,
                        &line_context,
                        math,
                        progress,
                    )?);
                }
                evaluate(&lines, math, progress)
            },
        )
    })();
    context.release_workspace(retained)?;
    result
}
struct ContactSystem<'a, 'observer> {
    left: &'a ChartLine,
    right: &'a ChartLine,
    components: [usize; 2],
    iterations: u32,
    progress: &'a mut WorkProgress<'observer>,
    endpoints: &'a [CurveIntersection],
    minimum_parameter_bits: u32,
}
impl RootSystem2 for ContactSystem<'_, '_> {
    fn exact_root(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<Option<RootBox2>, MathError> {
        for endpoint in self.endpoints {
            let (CurveIntersection::Endpoint { left, right, .. }
            | CurveIntersection::SymbolicEndpoint { left, right }
            | CurveIntersection::SymbolicSourceContact { left, right }) = endpoint
            else {
                continue;
            };
            let point = [fixed_from_rat(left, math)?, fixed_from_rat(right, math)?];
            if point
                .iter()
                .zip(domain)
                .all(|(p, d)| d.lower() <= p.lower() && p.upper() <= d.upper())
            {
                // Independent original-law equality is the existence proof.
                // Smooth normals identify every seam and exact pole frame.
                return Ok(Some(point));
            }
        }
        Ok(None)
    }
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        let (a, _) = self
            .left
            .normal(&domain[0], self.iterations, math, self.progress)?;
        let (b, _) = self
            .right
            .normal(&domain[1], self.iterations, math, self.progress)?;
        Ok([
            a[self.components[0]].sub(&b[self.components[0]], math)?,
            a[self.components[1]].sub(&b[self.components[1]], math)?,
        ])
    }
    fn jacobian(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError> {
        let (_, a) = self
            .left
            .normal(&domain[0], self.iterations, math, self.progress)?;
        let (_, b) = self
            .right
            .normal(&domain[1], self.iterations, math, self.progress)?;
        let a = a.ok_or(MathError::PrecisionExhausted)?;
        let b = b.ok_or(MathError::PrecisionExhausted)?;
        Ok([
            [
                a[self.components[0]].clone(),
                b[self.components[0]].neg(math)?,
            ],
            [
                a[self.components[1]].clone(),
                b[self.components[1]].neg(math)?,
            ],
        ])
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        let width = BigInt::from_i128(1).mul_pow2(
            math.limits()
                .precision_bits
                .saturating_sub(self.minimum_parameter_bits),
        );
        Ok(enclosure.iter().all(|v| v.upper().sub(v.lower()) <= width))
    }
    fn poll(&mut self, math: &CoordinateMath) -> Result<(), MathError> {
        self.progress.math_poll(math)
    }
}

#[derive(Clone, Copy)]
struct ContactAttempt<'a> {
    bits: u32,
    minimum_parameter_bits: u32,
    common: Option<&'a source_join::SourceJoin>,
}

fn attempt(
    left: &PreparedEdge,
    right: &PreparedEdge,
    endpoints: &[CurveIntersection],
    request: ContactAttempt<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let ContactAttempt {
        bits,
        minimum_parameter_bits,
        common,
    } = request;
    let policy = context.policy();
    let transformed = matches!(left, PreparedEdge::Transformed(_))
        || matches!(right, PreparedEdge::Transformed(_));
    with_line_refs_with_precision(
        &[left, right],
        bits,
        minimum_parameter_bits > 32,
        context,
        progress,
        |lines, math, progress| {
            let lower =
                Rat::parse_decimal(if transformed { "0" } else { "-0.125" }).expect("constant");
            let upper =
                Rat::parse_decimal(if transformed { "1" } else { "1.125" }).expect("constant");
            let expanded = fixed_from_rat(&lower, math)
                .and_then(|lower| {
                    let upper = fixed_from_rat(&upper, math)?;
                    FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)
                })
                .map_err(|error| geo_math_error(&error, policy))?;
            search_contacts_refined(
                &lines[0],
                &lines[1],
                &[expanded.clone(), expanded],
                endpoints,
                ContactGoal {
                    minimum_parameter_bits,
                    policy,
                    common,
                    self_contact: false,
                },
                math,
                progress,
            )
        },
    )
}

struct ContactPanel {
    domain: RootBox2,
    depth: u32,
}
struct ContactProof {
    root: RootIsolation2,
    components: [usize; 2],
}

#[derive(Clone, Copy)]
struct ContactGoal<'a> {
    minimum_parameter_bits: u32,
    policy: crate::ExecutionPolicy,
    common: Option<&'a source_join::SourceJoin>,
    self_contact: bool,
}

pub(super) fn search_contacts(
    a: &ChartLine,
    b: &ChartLine,
    original: &RootBox2,
    endpoints: &[CurveIntersection],
    policy: crate::ExecutionPolicy,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    search_contacts_refined(
        a,
        b,
        original,
        endpoints,
        ContactGoal {
            minimum_parameter_bits: 32,
            policy,
            common: None,
            self_contact: false,
        },
        math,
        progress,
    )
}

fn search_contacts_refined(
    a: &ChartLine,
    b: &ChartLine,
    original: &RootBox2,
    endpoints: &[CurveIntersection],
    goal: ContactGoal<'_>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let ContactGoal {
        minimum_parameter_bits,
        policy,
        common,
        self_contact,
    } = goal;
    let limits = RootIsolationLimits {
        max_depth: policy.limits().max_subdivision_levels,
        max_refinements: policy.limits().max_iterations,
    };
    let iterations = policy.limits().max_iterations;
    let mut pending = purrdf_lex::walk::WorkList::<ContactPanel, 8>::with(ContactPanel {
        domain: original.clone(),
        depth: 0,
    });
    let mut proofs: Vec<ContactProof> = Vec::new();
    let mut reserved = 0usize;
    let result = (|| {
        while let Some(panel) = pending.pop() {
            progress
                .math_poll(math)
                .map_err(|e| geo_math_error(&e, policy))?;
            if self_contact {
                // Work on one strict half of the parameter square. The
                // reflected half denotes the same physical contact, and the
                // diagonal denotes the original curve itself, not a root.
                if panel.domain[0].lower() >= panel.domain[1].upper() {
                    continue;
                }
                let hull = panel.domain[0]
                    .hull(&panel.domain[1], math)
                    .map_err(|e| geo_math_error(&e, policy))?;
                let (_, derivative) = a
                    .normal(&hull, iterations, math, progress)
                    .map_err(|e| geo_math_error(&e, policy))?;
                if super::point::injective_component(
                    a,
                    derivative.as_ref(),
                    (&hull, false, iterations),
                    math,
                    progress,
                )
                .map_err(|e| geo_math_error(&e, policy))?
                {
                    continue;
                }
                if panel.domain[0].upper() >= panel.domain[1].lower() {
                    split_contact_panel(panel, original, limits.max_depth, &mut pending, math)
                        .map_err(|e| geo_math_error(&e, policy))?;
                    continue;
                }
            }
            let (an, _) = a
                .normal(&panel.domain[0], iterations, math, progress)
                .map_err(|e| geo_math_error(&e, policy))?;
            let (bn, _) = b
                .normal(&panel.domain[1], iterations, math, progress)
                .map_err(|e| geo_math_error(&e, policy))?;
            if an
                .iter()
                .zip(&bn)
                .any(|(a, b)| a.upper() < b.lower() || b.upper() < a.lower())
            {
                continue;
            }
            if let Some(common) = common {
                if source_join::complete_local(common, &panel.domain, iterations, math, progress)
                    .map_err(|error| geo_math_error(&error, policy))?
                {
                    // The original source intervals meet at most once, and
                    // the complete common image is injective on their hull.
                    // Every possible contact is already an exact source node.
                    continue;
                }
                if source_join::contains_source_endpoint(&panel.domain, endpoints, math)
                    .map_err(|error| geo_math_error(&error, policy))?
                {
                    split_contact_panel(panel, original, limits.max_depth, &mut pending, math)
                        .map_err(|error| geo_math_error(&error, policy))?;
                    continue;
                }
            }
            let zero = BigInt::from_i128(0);
            // Every sign-separated component supplies a valid hemisphere
            // chart. Prefer the largest proved power-of-two lower magnitude:
            // a tiny nonzero horizontal component near a pole must not force
            // inversion of the nearly singular (horizontal, vertical) chart.
            // These borrowed integer bit lengths require no limb copies.
            let chart = (0..3)
                .filter_map(|k| {
                    let magnitude = if an[k].lower() > &zero && bn[k].lower() > &zero {
                        an[k].lower().min(bn[k].lower())
                    } else if an[k].upper() < &zero && bn[k].upper() < &zero {
                        an[k].upper().max(bn[k].upper())
                    } else {
                        return None;
                    };
                    Some((k, magnitude.as_integer().bit_len()))
                })
                .max_by_key(|(_, magnitude)| *magnitude)
                .map(|(k, _)| k);
            let Some(omitted) = chart else {
                split_contact_panel(panel, original, limits.max_depth, &mut pending, math)
                    .map_err(|e| geo_math_error(&e, policy))?;
                continue;
            };
            let components = match omitted {
                0 => [1, 2],
                1 => [0, 2],
                _ => [0, 1],
            };
            let roots = {
                let mut system = ContactSystem {
                    left: a,
                    right: b,
                    components,
                    iterations,
                    progress,
                    endpoints,
                    minimum_parameter_bits,
                };
                isolate_roots2(panel.domain, limits, &mut system, math)
                    .map_err(|e| geo_math_error(&e, policy))?
            };
            // Root storage was admitted inside the shared isolator. Continue
            // retaining it before any other allocation or scratch reuse.
            let storage = roots.iter().map(root_storage).fold(
                roots.capacity().saturating_mul(size_of::<RootIsolation2>()),
                usize::saturating_add,
            );
            math.reserve_workspace(storage)
                .map_err(|e| geo_math_error(&e, policy))?;
            let processed = (|| {
                for root in roots {
                    if proofs.iter().any(|prior| {
                        subset(root.enclosure(), prior.root.uniqueness_box())
                            || subset(prior.root.enclosure(), root.uniqueness_box())
                    }) {
                        continue;
                    }
                    let bytes = root_storage(&root)
                        .saturating_mul(2)
                        .saturating_add(size_of::<ContactProof>() * 2);
                    math.reserve_workspace(bytes)
                        .map_err(|e| geo_math_error(&e, policy))?;
                    reserved =
                        reserved
                            .checked_add(bytes)
                            .ok_or_else(|| GeoError::MemoryExhausted {
                                limit: policy.limits().max_workspace_bytes,
                            })?;
                    proofs.push(ContactProof { root, components });
                }
                Ok::<_, GeoError>(())
            })();
            math.release_workspace(storage)
                .map_err(|e| geo_math_error(&e, policy))?;
            processed?;
        }
        materialize_contacts(
            [a, b],
            proofs,
            endpoints,
            goal,
            math,
            progress,
            &mut reserved,
        )
    })();
    math.release_workspace(reserved)
        .map_err(|e| geo_math_error(&e, policy))?;
    result
}

fn subset(a: &RootBox2, b: &RootBox2) -> bool {
    a.iter()
        .zip(b)
        .all(|(a, b)| b.lower() <= a.lower() && a.upper() <= b.upper())
}
fn root_storage(root: &RootIsolation2) -> usize {
    root.enclosure()
        .iter()
        .chain(root.uniqueness_box())
        .map(FixedInterval::workspace_bytes)
        .fold(0usize, usize::saturating_add)
}
fn split_contact_panel(
    panel: ContactPanel,
    original: &RootBox2,
    max_depth: u32,
    pending: &mut purrdf_lex::walk::WorkList<ContactPanel, 8>,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    if panel.depth >= max_depth {
        return Err(MathError::PrecisionExhausted);
    }
    let widths = panel.domain.each_ref().map(|v| v.upper().sub(v.lower()));
    let axis = usize::from(widths[1] > widths[0]);
    let value = &panel.domain[axis];
    let (midpoint, _) = value
        .lower()
        .add(value.upper())
        .div_rem_u64(2)
        .ok_or(MathError::PrecisionExhausted)?;
    let (margin, _) = widths[axis]
        .div_rem_u64(16)
        .ok_or(MathError::PrecisionExhausted)?;
    if margin.is_zero() {
        return Err(MathError::PrecisionExhausted);
    }
    let lower = value
        .lower()
        .sub(&margin)
        .max(original[axis].lower().clone());
    let upper = value
        .upper()
        .add(&margin)
        .min(original[axis].upper().clone());
    let mut left = panel.domain.clone();
    let mut right = panel.domain;
    left[axis] = FixedInterval::from_bounds(lower, midpoint.add(&margin), math)?;
    right[axis] = FixedInterval::from_bounds(midpoint.sub(&margin), upper, math)?;
    pending.push(ContactPanel {
        domain: right,
        depth: panel.depth + 1,
    });
    pending.push(ContactPanel {
        domain: left,
        depth: panel.depth + 1,
    });
    Ok(())
}

fn materialize_contacts(
    lines: [&ChartLine; 2],
    proofs: Vec<ContactProof>,
    endpoints: &[CurveIntersection],
    goal: ContactGoal<'_>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    reserved: &mut usize,
) -> Result<Vec<CurveIntersection>, GeoError> {
    let ContactGoal {
        minimum_parameter_bits,
        policy,
        common: _,
        self_contact: _,
    } = goal;
    let [a, b] = lines;
    let iterations = policy.limits().max_iterations;
    if endpoints.len() as u64 > policy.limits().max_output_elements {
        return Err(GeoError::OutputExhausted {
            limit: policy.limits().max_output_elements,
        });
    }
    let zero = BigInt::from_i128(0);
    let one = BigInt::from_i128(1).mul_pow2(math.limits().precision_bits);
    let mut output = endpoints.to_vec();
    for ContactProof { root, components } in proofs {
        let enclosure = root.enclosure();
        if enclosure
            .iter()
            .any(|v| v.upper() < &zero || v.lower() > &one)
        {
            continue;
        }
        let known = ContactSystem {
            left: a,
            right: b,
            components,
            iterations,
            progress,
            endpoints,
            minimum_parameter_bits,
        }
        .exact_root(root.uniqueness_box(), math)
        .map_err(|e| geo_math_error(&e, policy))?
        .is_some();
        if known {
            continue;
        }
        if enclosure
            .iter()
            .any(|v| v.lower() <= &zero || v.upper() >= &one)
        {
            return Err(GeoError::PrecisionExhausted {
                bits: math.limits().precision_bits,
            });
        }
        if output.len() as u64 >= policy.limits().max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: policy.limits().max_output_elements,
            });
        }
        let bytes = root_storage(&root).saturating_mul(16);
        math.reserve_workspace(bytes)
            .map_err(|e| geo_math_error(&e, policy))?;
        *reserved = reserved
            .checked_add(bytes)
            .ok_or_else(|| GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            })?;
        let (point, _) = a
            .image(&enclosure[0], iterations, math, progress)
            .map_err(|e| geo_math_error(&e, policy))?;
        output.push(CurveIntersection::Isolated {
            left: exact_bounds(&enclosure[0]),
            right: exact_bounds(&enclosure[1]),
            longitude_radians: exact_bounds(&point[0]),
            latitude_radians: exact_bounds(&point[1]),
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeographicReference, PreparedCoordinate, ShortestGeodesicArc};

    pub(super) fn coordinate(lon: i64, lat: i64) -> PreparedCoordinate {
        PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(lon), Rat::from_i64(lat)),
            &GeographicReference::wgs84(),
        )
        .expect("exact valid source")
    }
    pub(super) fn linear(a: (i64, i64), b: (i64, i64)) -> PreparedEdge {
        PreparedEdge::SourceLinear(Box::new(
            SourceLinearEdge::new(coordinate(a.0, a.1), coordinate(b.0, b.1))
                .expect("same binding"),
        ))
    }
    pub(super) fn shortest(
        a: (i64, i64),
        b: (i64, i64),
        context: &mut MetricContext,
    ) -> PreparedEdge {
        PreparedEdge::ShortestGeodesic(Box::new(
            ShortestGeodesicArc::new(coordinate(a.0, a.1), coordinate(b.0, b.1), context)
                .expect("unique branch"),
        ))
    }

    #[test]
    fn original_curve_width_is_admitted_before_any_image_phase() {
        let tiny = Rat::new(crate::Int::one(), crate::Int::one().shl(65_536)).unwrap();
        let start =
            PreparedCoordinate::new(Coord::xy(tiny, Rat::zero()), &GeographicReference::wgs84())
                .unwrap();
        let edge = PreparedEdge::SourceLinear(Box::new(
            SourceLinearEdge::new(start, coordinate(1, 0)).unwrap(),
        ));
        assert_eq!(edge.max_original_operand_bits(), 65_537);
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_workspace_bytes = 1_048_576;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        let mut evaluated = false;
        let result = with_lines(&[edge], 80, &mut context, &mut progress, |_, _, _| {
            evaluated = true;
            Ok(())
        });
        assert_eq!(
            result.unwrap_err(),
            GeoError::MemoryExhausted { limit: 1_048_576 }
        );
        assert!(!evaluated);
    }

    #[test]
    fn exact_chart_crossings_overlaps_long_paths_and_identified_poles() {
        let mut context = MetricContext::wgs84().expect("environment");
        let crossed = intersections(
            &linear((-10, 0), (10, 0)),
            &linear((0, -10), (0, 10)),
            &mut context,
        )
        .expect("complete exact crossing");
        let half = Rat::one().div(&Rat::from_i64(2)).expect("half");
        assert_eq!(
            crossed,
            vec![CurveIntersection::Exact {
                left: half.clone(),
                right: half,
                point: LonLat::new(Rat::zero(), Rat::zero()).expect("origin")
            }]
        );
        let overlap = intersections(
            &linear((-10, 0), (10, 0)),
            &linear((5, 0), (-5, 0)),
            &mut context,
        )
        .expect("exact opposite overlap");
        assert!(
            matches!(&overlap[..], [CurveIntersection::Overlap {left,right,..}]
            if left.0 < left.1 && right.0 > right.1)
        );
        assert_eq!(
            intersections(
                &linear((170, 0), (-170, 0)),
                &linear((0, -10), (0, 10)),
                &mut context
            )
            .expect("written long path")
            .len(),
            1
        );
        let pole = intersections(
            &linear((5, 80), (5, 90)),
            &linear((-75, 80), (-75, 90)),
            &mut context,
        )
        .expect("exact physical pole contact");
        assert!(matches!(&pole[..], [CurveIntersection::Exact {point,..}] if point.is_pole()));
    }

    #[test]
    fn endpoint_defined_arcs_use_true_unrounded_unique_branch_intersections() {
        let mut context = MetricContext::wgs84().expect("environment");
        let a = shortest((-10, 0), (10, 0), &mut context);
        let b = shortest((0, -10), (0, 10), &mut context);
        let result = intersections(&a, &b, &mut context).expect("certified transverse geodesics");
        let half = Rat::one().div(&Rat::from_i64(2)).expect("half");
        assert!(
            matches!(&result[..], [CurveIntersection::Isolated {left,right,
            longitude_radians,latitude_radians}] if left.0<=half && half<=left.1
            && right.0<=half && half<=right.1 && longitude_radians.0<=Rat::zero()
            && Rat::zero()<=longitude_radians.1 && latitude_radians.0<=Rat::zero()
            && Rat::zero()<=latitude_radians.1)
        );
        let reversed = shortest((10, 0), (-10, 0), &mut context);
        assert_eq!(
            intersections(&a, &reversed, &mut context).expect("exact unique-path reversal"),
            vec![CurveIntersection::Coincident { reversed: true }]
        );
        let c = shortest((10, 0), (10, 10), &mut context);
        let contact = intersections(&a, &c, &mut context).expect("closed exact endpoint root");
        assert!(
            matches!(&contact[..], [CurveIntersection::Endpoint {left,right,..}]
            if left==&Rat::one() && right==&Rat::zero())
        );
        let north_a = shortest((0, 80), (5, 90), &mut context);
        let north_b = shortest((90, 80), (-73, 90), &mut context);
        let pole =
            intersections(&north_a, &north_b, &mut context).expect("smooth normal pole chart");
        assert!(
            matches!(&pole[..],[CurveIntersection::Endpoint {left,right,point}]
            if left==&Rat::one() && right==&Rat::one() && point.is_pole())
        );
    }
    #[test]
    fn selected_branch_prefix_overlap_retains_exact_law_parameters() {
        let mut context = MetricContext::wgs84().expect("context");
        let coordinate = |lon, lat| {
            PreparedCoordinate::new(
                Coord::xy(Rat::from_i64(lon), Rat::from_i64(lat)),
                &GeographicReference::wgs84(),
            )
            .expect("source")
        };
        let selected = |azimuth, length, context: &mut MetricContext| {
            PreparedEdge::AzimuthLength(Box::new(
                crate::AzimuthLengthArc::new(
                    coordinate(10, 20),
                    Rat::from_i64(azimuth),
                    crate::Metres::new(Rat::from_i64(length)),
                    context,
                )
                .expect("selected arc"),
            ))
        };
        let left = selected(90, 1000, &mut context);
        let right = selected(450, 3000, &mut context);
        assert_eq!(
            intersections(&left, &right, &mut context).expect("exact prefix law"),
            vec![CurveIntersection::BranchOverlap {
                left: (Rat::zero(), Rat::one()),
                right: (
                    Rat::zero(),
                    Rat::one().div(&Rat::from_i64(3)).expect("one third")
                ),
            }]
        );
    }

    #[test]
    fn strict_contact_proofs_tighten_without_changing_the_original_branches() {
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 4_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let first = shortest((-10, 0), (10, 0), &mut context);
        let transverse = shortest((0, -10), (0, 10), &mut context);
        let proof = intersections_refined(&first, &transverse, 64, &mut context)
            .expect("complete strict transverse contact");
        let [CurveIntersection::Isolated { left, right, .. }] = proof.as_slice() else {
            panic!("isolated transverse root");
        };
        let quantum = Rat::new(crate::Int::one(), crate::Int::one().shl(64)).unwrap();
        let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
        for (lower, upper) in [left, right] {
            assert!(lower <= &half && &half <= upper);
            assert!(upper.sub(lower) <= quantum);
        }
        let subarc = shortest((-5, 0), (5, 0), &mut context);
        let proof = intersections_refined(&first, &subarc, 64, &mut context)
            .expect("complete strict overlap endpoints");
        let [
            CurveIntersection::GeodesicOverlap {
                left,
                right,
                start,
                end,
            },
        ] = proof.as_slice()
        else {
            panic!("exact-source shared subcurve");
        };
        assert!(start.same_location(subarc.start().unwrap().point()));
        let PreparedEdge::ShortestGeodesic(subarc) = &subarc else {
            panic!("original shortest branch");
        };
        assert!(end.same_location(subarc.end().point()));
        for (lower, upper) in left.iter().chain(right.iter()) {
            assert!(upper.sub(lower) <= quantum);
        }
        assert!(matches!(
            intersections_refined(&first, &transverse, 513, &mut context),
            Err(GeoError::PrecisionExhausted { bits: 512 })
        ));
    }
}
