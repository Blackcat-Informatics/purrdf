// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic global minimum over complete prepared point/edge domains.
//!
//! Triangle inequality and certified complete path-length bounds enclose every
//! parameter box. A local sample improves only the upper bound; no local minimum
//! excludes any other part of an edge. Exact atlas intersections are checked first.

use core::cmp::Ordering;
use std::collections::BinaryHeap;

mod selected;

use super::{GeometryMetricEstimate, GeometryMetricLaw};
use crate::context::WorkProgress;
use crate::{
    Coord, GeoError, LonLat, Metres, MetricContext, MetricWorkObserver, PreparedEdge,
    PreparedGeodesic, PreparedGeometry, PreparedRegion, Rat, SegmentIntersection, Set,
};
use selected::{PointSample, SelectedDomain};

/// Globally certified geographic distance of complete prepared geometries.
///
/// # Errors
/// Refuses empty inputs, missing reference operations and incomplete admission.
pub fn distance(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    distance_inner(a, b, context, None)
}

/// Global distance with borrowed work/cancellation access between bounded chunks.
///
/// # Errors
/// Also propagates the observer's original refusal without a partial answer.
pub fn distance_metered(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<GeometryMetricEstimate, GeoError> {
    distance_inner(a, b, context, Some(observer))
}

/// Compare the true global physical distance to an exact metre threshold.
///
/// # Errors
/// Refuses empty inputs, reference mismatch and unresolved or unadmitted proof.
pub fn within_physical(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    threshold: &Metres,
    context: &mut MetricContext,
) -> Result<bool, GeoError> {
    within_inner(a, b, threshold, context, None)
}
/// Compare physical distance with bounded cancellation/work callbacks.
///
/// # Errors
/// Also preserves the observer's original refusal.
pub fn within_physical_metered(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    threshold: &Metres,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<bool, GeoError> {
    within_inner(a, b, threshold, context, Some(observer))
}
fn within_inner(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    threshold: &Metres,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<bool, GeoError> {
    let answer = compute_inner(
        a,
        b,
        context,
        observer,
        MinimumGoal::Physical(threshold.exact()),
    )?;
    let MinimumAnswer::Physical(answer) = answer else {
        unreachable!("physical comparison goal")
    };
    Ok(answer)
}
#[derive(Clone, Copy)]
enum MinimumGoal<'a> {
    Estimate,
    Physical(&'a Rat),
}
enum MinimumAnswer {
    Estimate(Rat),
    Physical(bool),
}
impl MinimumGoal<'_> {
    fn zero(self) -> MinimumAnswer {
        match self {
            Self::Estimate => MinimumAnswer::Estimate(Rat::zero()),
            Self::Physical(threshold) => MinimumAnswer::Physical(threshold >= &Rat::zero()),
        }
    }
    fn physical(self) -> bool {
        matches!(self, Self::Physical(_))
    }
}

#[derive(Clone, Copy)]
enum Primitive<'a> {
    Point(&'a LonLat),
    ImagePoint(&'a crate::OperationImagePoint),
    Edge(&'a PreparedEdge),
    ChartEdge(&'a LonLat, &'a LonLat),
    Selected(SelectedDomain<'a>),
}

impl<'a> Primitive<'a> {
    fn length(
        self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Rat, GeoError> {
        match self {
            Self::Selected(domain) => domain.length(context, progress),
            Self::Point(_) | Self::ImagePoint(_) => Ok(Rat::zero()),
            Self::ChartEdge(start, end) => {
                crate::numerical::quantized_source_linear_bound(start, end, None, context, progress)
            }
            Self::Edge(PreparedEdge::SourceLinear(edge)) => {
                crate::numerical::quantized_source_linear_bound(
                    edge.start().point(),
                    edge.end().point(),
                    None,
                    context,
                    progress,
                )
            }
            Self::Edge(PreparedEdge::AzimuthLength(arc)) => {
                crate::numerical::ExactAdmission::new(context, progress).rational(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[arc.length().exact()],
                    1,
                    || Ok(arc.length().exact().clone()),
                )
            }
            Self::Edge(PreparedEdge::ShortestGeodesic(arc)) => {
                let length = arc.proof().distance.upper.exact();
                crate::numerical::ExactAdmission::new(context, progress).rational(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[length],
                    1,
                    || Ok(length.clone()),
                )
            }
            Self::Edge(PreparedEdge::Transformed(image)) => {
                let mut child = context.remaining_child()?;
                let panel = {
                    let mut observer = progress.nested(
                        context.work_items(),
                        context
                            .policy()
                            .limits()
                            .max_workspace_bytes
                            .saturating_sub(context.remaining_workspace()),
                        context.workspace_peak(),
                    );
                    image.enclosure_metered(
                        &Rat::zero(),
                        &Rat::one(),
                        96,
                        &mut child,
                        &mut observer,
                    )
                };
                let derivative = progress
                    .absorb_child_result(context, &child, panel)?
                    .derivative
                    .ok_or_else(|| GeoError::PrecisionExhausted {
                        bits: context.policy().limits().max_precision_bits,
                    })?;
                let rate = derivative.iter().fold(Rat::zero(), |sum, value| {
                    let (lower, upper) = crate::numerical::exact_bounds(value);
                    sum.add(&lower.abs().max(upper.abs()))
                });
                let factor = crate::numerical::quantized_source_linear_bound(
                    &LonLat::new(Rat::zero(), Rat::zero()).expect("origin"),
                    &LonLat::new(Rat::one(), Rat::zero()).expect("one degree"),
                    None,
                    context,
                    progress,
                )?;
                crate::numerical::ExactAdmission::new(context, progress).rational(
                    purrdf_xsd::integer::ExactOperation::RationalMultiply,
                    &[&rate, &factor],
                    1,
                    || Ok(rate.mul(&factor)),
                )
            }
        }
    }
    fn start(self) -> Option<&'a LonLat> {
        match self {
            Self::Point(point) => Some(point),
            Self::ImagePoint(_) => None,
            Self::Selected(_) => None,
            Self::ChartEdge(start, _) => Some(start),
            Self::Edge(edge) => edge.start().map(crate::PreparedCoordinate::point),
        }
    }
    fn point(
        self,
        parameter: &Rat,
        child: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
        physical: bool,
        precision_bits: u32,
    ) -> Result<PointSample, GeoError> {
        if let Self::Selected(domain) = self {
            return domain.point(parameter, child, observer, physical, precision_bits);
        }
        let result = match self {
            Self::Point(point) => Ok((point.clone(), Rat::zero())),
            Self::ChartEdge(start, end) => {
                crate::SourceLinearEdge::interpolate(start, end, parameter)
                    .map(|point| (point, Rat::zero()))
            }
            Self::Edge(PreparedEdge::SourceLinear(edge)) => {
                edge.at(parameter).map(|point| (point, Rat::zero()))
            }
            Self::Edge(
                edge @ (PreparedEdge::AzimuthLength(_) | PreparedEdge::ShortestGeodesic(_)),
            ) if physical => {
                let view = edge
                    .geodesic_view_metered(precision_bits.max(128), child, observer)?
                    .expect("geodesic source");
                let already_observed = child.work_items();
                let already_peak = child.workspace_peak();
                child.set_preparation_work(already_observed)?;
                let mut continuation =
                    crate::MetricWorkContinuation::new(observer, already_observed, already_peak);
                let image =
                    view.parameter_enclosure(parameter, parameter, child, Some(&mut continuation))?;
                let policy = child.policy();
                let previous_peak = child.workspace_peak();
                let mut math =
                    purrdf_xsd::math::CoordinateMath::new(purrdf_xsd::math::MathLimits {
                        precision_bits: image.longitude.precision_bits(),
                        max_work: policy
                            .limits()
                            .max_work_items
                            .saturating_sub(child.work_items()),
                        max_workspace_bytes: usize::try_from(child.remaining_workspace()).map_err(
                            |_| GeoError::MemoryExhausted {
                                limit: policy.limits().max_workspace_bytes,
                            },
                        )?,
                    })
                    .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
                math.reserve_workspace(8192)
                    .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
                let factor =
                    purrdf_xsd::math::FixedInterval::from_i64(180, &mut math).and_then(|degrees| {
                        degrees.div(&purrdf_xsd::math::FixedInterval::pi(&mut math)?, &mut math)
                    });
                let result = factor.and_then(|factor| {
                    Ok((
                        image.longitude.mul(&factor, &mut math)?,
                        image.latitude.mul(&factor, &mut math)?,
                    ))
                });
                math.release_workspace(8192)
                    .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
                child.charge_work(math.work_used())?;
                child.admit_workspace(math.workspace_peak() as u64)?;
                child.release_workspace(math.workspace_peak() as u64)?;
                continuation.charge_chunk(
                    math.work_used(),
                    child.workspace_peak().saturating_sub(previous_peak),
                )?;
                child.checkpoint()?;
                let (longitude, latitude) =
                    result.map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
                certified_point(&longitude, &latitude, child.reference())
            }
            Self::Edge(PreparedEdge::AzimuthLength(arc)) => Ok((
                PreparedGeodesic::new(child.reference().clone())
                    .direct_metered(
                        arc.start().point(),
                        arc.azimuth(),
                        &Metres::new(arc.length().exact().mul(parameter)),
                        child,
                        observer,
                    )?
                    .endpoint()
                    .clone(),
                decimal("0.000001"),
            )),
            Self::Edge(PreparedEdge::ShortestGeodesic(arc)) => Ok((
                arc.at_metered(parameter, child, observer)?
                    .endpoint()
                    .clone(),
                decimal("0.000001"),
            )),
            Self::Edge(PreparedEdge::Transformed(_)) | Self::ImagePoint(_) => {
                let mut progress = WorkProgress::new(Some(observer));
                progress.context_poll(child)?;
                let maximum = child.policy().limits().max_precision_bits;
                let mut bits = precision_bits.max(96).min(maximum);
                loop {
                    let mut attempt = child.remaining_child()?;
                    let result = {
                        let mut nested = progress.nested(
                            child.work_items(),
                            child.current_workspace_bytes(),
                            child.workspace_peak(),
                        );
                        self.image_enclosure(parameter, bits, &mut attempt, &mut nested)
                    };
                    let panel = progress.absorb_child_result(child, &attempt, result)?;
                    progress.context_poll(child)?;
                    let point =
                        certified_point(&panel.longitude, &panel.latitude, child.reference())?;
                    if point.1 <= decimal("0.000001") {
                        return Ok(PointSample::ordinary(point));
                    }
                    if bits >= maximum {
                        return Err(GeoError::PrecisionExhausted { bits: maximum });
                    }
                    bits = bits.saturating_mul(2).min(maximum);
                }
            }
            Self::Selected(_) => unreachable!("selected source domain handled above"),
        };
        result.map(PointSample::ordinary)
    }
    fn image_enclosure(
        self,
        parameter: &Rat,
        bits: u32,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<crate::operation::OperationImageEnclosure, GeoError> {
        match self {
            Self::ImagePoint(point) => point.enclosure_metered(bits, context, observer),
            Self::Edge(PreparedEdge::Transformed(curve)) => {
                curve.enclosure_metered(parameter, parameter, bits, context, observer)
            }
            _ => unreachable!("original operation image inventory"),
        }
    }
}

fn certified_point(
    longitude: &purrdf_xsd::math::FixedInterval,
    latitude: &purrdf_xsd::math::FixedInterval,
    reference: &crate::GeographicReference,
) -> Result<(LonLat, Rat), GeoError> {
    let (west, east) = crate::numerical::exact_bounds(longitude);
    let (south, north) = crate::numerical::exact_bounds(latitude);
    let half = Rat::one().div(&Rat::from_i64(2)).expect("positive divisor");
    let raw_longitude = west.add(&east).mul(&half);
    let normalized = crate::geographic::normalize_degrees(&raw_longitude, -180);
    let south = south.max(Rat::from_i64(-90));
    let north = north.min(Rat::from_i64(90));
    if south > north {
        return Err(GeoError::PrecisionExhausted {
            bits: latitude.precision_bits(),
        });
    }
    let point = LonLat::new(normalized, south.add(&north).mul(&half))?;
    let span = east.sub(&west).add(&north.sub(&south)).mul(&half);
    let factor = crate::SourceLinearEdge::upper_bound(
        &LonLat::new(Rat::zero(), Rat::zero()).expect("origin"),
        &LonLat::new(Rat::one(), Rat::zero()).expect("one degree"),
        reference,
    );
    Ok((point, span.mul(factor.exact())))
}

fn primitives<'a>(
    geometry: &'a PreparedGeometry,
    arrangement: Option<&'a crate::atlas::boundary::RegionBoundary>,
    selected: Option<(
        &'a [super::native::Parameters],
        &'a [super::native::Parameters],
    )>,
) -> Vec<Primitive<'a>> {
    geometry
        .points()
        .iter()
        .map(|point| Primitive::Point(point.point()))
        .chain(geometry.symbolic_points().iter().map(Primitive::ImagePoint))
        .chain(
            geometry
                .curves()
                .iter()
                .flat_map(crate::PreparedCurve::edges)
                .map(Primitive::Edge),
        )
        .chain(match geometry.region() {
            PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => arrangement
                .expect("areal region has an exact physical boundary")
                .edges
                .iter()
                .map(|edge| Primitive::ChartEdge(&edge[0], &edge[1]))
                .collect(),
            PreparedRegion::Empty | PreparedRegion::Whole => Vec::new(),
        })
        .chain(
            arrangement
                .into_iter()
                .flat_map(|boundary| &boundary.curves)
                .flat_map(crate::PreparedCurve::edges)
                .map(Primitive::Edge),
        )
        .chain(arrangement.into_iter().flat_map(|boundary| {
            boundary.native.iter().flat_map(|native| {
                let (fragments, points) = selected.expect("selected domain samples");
                native
                    .fragments()
                    .iter()
                    .zip(fragments)
                    .map(|(fragment, samples)| {
                        Primitive::Selected(SelectedDomain::fragment(fragment, samples))
                    })
                    .chain(
                        native
                            .isolated_points()
                            .iter()
                            .zip(points)
                            .map(|(point, samples)| {
                                Primitive::Selected(SelectedDomain::point_domain(point, samples))
                            }),
                    )
            })
        }))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    level: u32,
    index: u64,
}

impl Range {
    const ROOT: Self = Self { level: 0, index: 0 };
    fn middle(self) -> Rat {
        Rat::new(
            crate::Int::from_u64(self.index)
                .shl(1)
                .add(&crate::Int::one()),
            crate::Int::one().shl(self.level + 1),
        )
        .expect("positive dyadic denominator")
    }
    fn radius(self, length: &Rat) -> Rat {
        length
            .div(&Rat::from_int(crate::Int::one().shl(self.level + 1)))
            .expect("positive dyadic denominator")
    }
    fn children(self, limit: u32) -> Result<[Self; 2], GeoError> {
        if self.level >= limit || self.level >= 63 {
            return Err(GeoError::ConvergenceExhausted { iterations: limit });
        }
        let index = self
            .index
            .checked_mul(2)
            .ok_or(GeoError::ArithmeticOverflow("distance subdivision index"))?;
        Ok([
            Self {
                level: self.level + 1,
                index,
            },
            Self {
                level: self.level + 1,
                index: index + 1,
            },
        ])
    }
}

struct Candidate {
    lower: Rat,
    value: Rat,
    upper: Rat,
    // A threshold witness is qualitative. Do not invent a numerical distance
    // gap from a strict comparison whose certified endpoints were not returned.
    physical_classification: Option<bool>,
    a: usize,
    b: usize,
    a_range: Range,
    b_range: Range,
    serial: u64,
    precision_bits: u32,
    sample_error: Rat,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial
    }
}
impl Eq for Candidate {}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .cmp(&self.lower)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn distance_inner(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    mut observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<GeometryMetricEstimate, GeoError> {
    let answer = match observer.as_mut() {
        Some(observer) => {
            compute_inner(a, b, context, Some(&mut **observer), MinimumGoal::Estimate)
        }
        None => compute_inner(a, b, context, None, MinimumGoal::Estimate),
    }?;
    let MinimumAnswer::Estimate(value) = answer else {
        unreachable!("reported estimate goal")
    };
    let mut continuation = observer.map(|observer| {
        crate::MetricWorkContinuation::new(observer, context.work_items(), context.workspace_peak())
    });
    let mut progress = WorkProgress::new(
        continuation
            .as_mut()
            .map(|observer| observer as &mut dyn MetricWorkObserver),
    );
    let reported = crate::metric::reported_double_with_progress(&value, context, &mut progress)?;
    if crate::metric::binary64_half_ulp(reported.to_bits()) > decimal("0.000125") {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    let binding = crate::numerical::reference_identity(context, &mut progress, None)?;
    Ok(GeometryMetricEstimate {
        value,
        reported: reported.to_bits(),
        law: GeometryMetricLaw::Distance,
        binding,
    })
}
fn compute_inner(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
    goal: MinimumGoal<'_>,
) -> Result<MinimumAnswer, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    for geometry in [a, b] {
        if geometry.reference() != context.reference() {
            return Err(crate::numerical::missing_operation(
                geometry.reference(),
                context,
                &mut progress,
            )?);
        }
    }
    progress.context_poll(context)?;
    let count = |geometry: &PreparedGeometry| {
        geometry
            .points()
            .len()
            .saturating_add(geometry.symbolic_points().len())
            .saturating_add(
                geometry
                    .curves()
                    .iter()
                    .map(|curve| curve.edges().len())
                    .sum::<usize>(),
            )
            .saturating_add(match geometry.region() {
                PreparedRegion::Polygons(polygons)
                | PreparedRegion::ComplementOfPolygons(polygons) => polygons
                    .iter()
                    .flat_map(crate::PreparedPolygon::rings)
                    .map(|ring| ring.edges().len())
                    .sum(),
                PreparedRegion::Empty | PreparedRegion::Whole => 0,
            })
    };
    let scratch = (count(a) as u64)
        .saturating_add(count(b) as u64)
        .saturating_mul(4096)
        .saturating_add(65_536);
    context.admit_workspace(scratch)?;
    let mut retained = scratch;
    let result = (|| {
        let mut arrangement = |geometry: &PreparedGeometry| -> Result<_, GeoError> {
            if matches!(
                geometry.region(),
                PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_)
            ) {
                let result = crate::atlas::boundary::region_boundary(
                    geometry.region(),
                    context,
                    &mut progress,
                )?;
                retained = retained.checked_add(result.workspace_bytes).ok_or(
                    GeoError::ArithmeticOverflow("distance arrangement workspace"),
                )?;
                Ok(Some(result))
            } else {
                Ok(None)
            }
        };
        let a_arrangement = arrangement(a)?;
        let b_arrangement = arrangement(b)?;
        // True zero distance is exact set contact, including irrational isolated
        // nodes and selected subcurve cuts. Reuse the complete original graph;
        // a numerical sample is never an existence/absence proof.
        if a_arrangement
            .as_ref()
            .is_some_and(|value| value.native.is_some())
            || b_arrangement
                .as_ref()
                .is_some_and(|value| value.native.is_some())
        {
            let matrix =
                crate::atlas::arcs::arrangement::relation_matrix(a, b, context, &mut progress)?;
            if crate::SpatialRelation::SfIntersects.holds(&matrix, 0, 0) {
                return Ok(goal.zero());
            }
        }
        let mut samples =
            |boundary: Option<&crate::atlas::boundary::RegionBoundary>| -> Result<_, GeoError> {
                boundary
                    .and_then(|value| value.native.as_ref())
                    .map(|native| {
                        Ok((
                            super::native::prepare(native, context, &mut progress, &mut retained)?,
                            super::native::prepare_parameters(
                                native
                                    .isolated_points()
                                    .iter()
                                    .map(|point| [point.parameter(); 2]),
                                context,
                                &mut progress,
                                &mut retained,
                            )?,
                        ))
                    })
                    .transpose()
            };
        let a_samples = samples(a_arrangement.as_ref())?;
        let b_samples = samples(b_arrangement.as_ref())?;
        let aa = primitives(
            a,
            a_arrangement.as_ref(),
            a_samples
                .as_ref()
                .map(|(a, b)| (a.as_slice(), b.as_slice())),
        );
        let bb = primitives(
            b,
            b_arrangement.as_ref(),
            b_samples
                .as_ref()
                .map(|(a, b)| (a.as_slice(), b.as_slice())),
        );
        minimize(
            (a.region(), &aa),
            (b.region(), &bb),
            context,
            &mut progress,
            &mut retained,
            goal,
        )
    })();
    context.release_workspace(retained)?;
    result
}

fn minimize(
    (a_region, aa): (&PreparedRegion, &[Primitive<'_>]),
    (b_region, bb): (&PreparedRegion, &[Primitive<'_>]),
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    goal: MinimumGoal<'_>,
) -> Result<MinimumAnswer, GeoError> {
    if let MinimumGoal::Physical(threshold) = goal
        && threshold < &Rat::zero()
    {
        return Ok(MinimumAnswer::Physical(false));
    }
    if matches!(a_region, PreparedRegion::Empty) && aa.is_empty()
        || matches!(b_region, PreparedRegion::Empty) && bb.is_empty()
    {
        return Err(GeoError::domain(
            "distance requires two nonempty geometries",
        ));
    }
    if matches!(a_region, PreparedRegion::Whole) || matches!(b_region, PreparedRegion::Whole) {
        return Ok(goal.zero());
    }
    if intersection(a_region, b_region, aa, bb, context, progress)? {
        return Ok(goal.zero());
    }
    if aa.is_empty() || bb.is_empty() {
        // No physical boundary and no component witnessed by the other input:
        // this region is empty. Whole chart regions were recognized by membership.
        return Err(GeoError::domain(
            "distance requires two nonempty geometries",
        ));
    }
    let a_lengths: Result<Vec<_>, _> = aa
        .iter()
        .map(|item| item.length(context, progress))
        .collect();
    let b_lengths: Result<Vec<_>, _> = bb
        .iter()
        .map(|item| item.length(context, progress))
        .collect();
    let a_lengths = a_lengths?;
    let b_lengths = b_lengths?;

    let reference = crate::numerical::reference_clone(context, progress)?;
    let mut preparation = context.remaining_child()?;
    let prepared = {
        let mut nested = progress.nested(
            context.work_items(),
            context.current_workspace_bytes(),
            context.workspace_peak(),
        );
        PreparedGeodesic::prepare_metered(reference, &mut preparation, &mut nested)
    };
    let prepared = progress.absorb_child_result(context, &preparation, prepared)?;
    if let Some(arithmetic) = preparation.prepared_arithmetic() {
        context.set_prepared_arithmetic(arithmetic);
    }
    let bytes = prepared.retained_workspace_bytes();
    context.admit_workspace(bytes)?;
    *retained = retained
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow(
            "prepared distance coefficients",
        ))?;
    progress.context_poll(context)?;

    if let MinimumGoal::Physical(threshold) = goal
        && aa
            .iter()
            .chain(bb)
            .all(|primitive| matches!(primitive, Primitive::Point(_)))
    {
        for a in aa {
            for b in bb {
                let (Primitive::Point(a), Primitive::Point(b)) = (a, b) else {
                    unreachable!("point inventory")
                };
                let mut child = context.remaining_child()?;
                let answer = {
                    let mut observer = progress.nested(
                        context.work_items(),
                        context
                            .policy()
                            .limits()
                            .max_workspace_bytes
                            .saturating_sub(context.remaining_workspace()),
                        context.workspace_peak(),
                    );
                    prepared.within_physical_metered(
                        a,
                        b,
                        &Metres::new(threshold.clone()),
                        &mut child,
                        &mut observer,
                    )
                };
                if progress.absorb_child_result(context, &child, answer)? {
                    return Ok(MinimumAnswer::Physical(true));
                }
            }
        }
        return Ok(MinimumAnswer::Physical(false));
    }
    let mut queue = BinaryHeap::new();
    let mut serial = 0_u64;
    let mut best: Option<Rat> = None;
    for (a, _) in aa.iter().enumerate() {
        for (b, _) in bb.iter().enumerate() {
            let node = sample(
                &prepared,
                aa,
                bb,
                &a_lengths,
                &b_lengths,
                Candidate {
                    lower: Rat::zero(),
                    value: Rat::zero(),
                    upper: Rat::zero(),
                    physical_classification: None,
                    a,
                    b,
                    a_range: Range::ROOT,
                    b_range: Range::ROOT,
                    serial,
                    precision_bits: 96,
                    sample_error: Rat::zero(),
                },
                context,
                progress,
                goal,
            )?;

            match node.physical_classification {
                Some(true) => return Ok(MinimumAnswer::Physical(true)),
                Some(false) => continue,
                None => {}
            }
            best = Some(best.map_or_else(
                || {
                    if goal.physical() {
                        node.upper.clone()
                    } else {
                        node.value.clone()
                    }
                },
                |value| {
                    value.min(if goal.physical() {
                        node.upper.clone()
                    } else {
                        node.value.clone()
                    })
                },
            ));
            push(&mut queue, node, context, retained)?;
            serial = serial
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow("distance traversal serial"))?;
        }
    }
    let Some(mut best) = best else {
        if goal.physical() {
            return Ok(MinimumAnswer::Physical(false));
        }
        return Err(GeoError::domain(
            "distance requires two nonempty geometries",
        ));
    };
    let epsilon = decimal("0.000003");
    while let Some(node) = queue.pop() {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        match goal {
            MinimumGoal::Estimate if best.add(&epsilon).sub(&node.lower) <= decimal("0.000125") => {
                return Ok(MinimumAnswer::Estimate(best));
            }
            MinimumGoal::Physical(threshold) if &best <= threshold => {
                return Ok(MinimumAnswer::Physical(true));
            }
            MinimumGoal::Physical(threshold) if &node.lower > threshold => {
                return Ok(MinimumAnswer::Physical(false));
            }
            _ => {}
        }
        let a_radius = node.a_range.radius(&a_lengths[node.a]);
        let b_radius = node.b_range.radius(&b_lengths[node.b]);
        if goal.physical()
            && (matches!(aa[node.a], Primitive::Selected(_))
                || matches!(bb[node.b], Primitive::Selected(_)))
            && a_radius.add(&b_radius) <= node.sample_error
        {
            let maximum = context.policy().limits().max_precision_bits;
            if node.precision_bits >= maximum {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            let mut refined = node;
            refined.precision_bits = refined.precision_bits.saturating_mul(2).min(maximum);
            let refined = sample(
                &prepared, aa, bb, &a_lengths, &b_lengths, refined, context, progress, goal,
            )?;
            match refined.physical_classification {
                Some(true) => return Ok(MinimumAnswer::Physical(true)),
                Some(false) => continue,
                None => {}
            }
            best = best.min(refined.upper.clone());
            push(&mut queue, refined, context, retained)?;
            continue;
        }
        if a_radius.is_zero() && b_radius.is_zero() {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        let split_a = a_radius >= b_radius;
        let children = if split_a {
            node.a_range
                .children(context.policy().limits().max_subdivision_levels)?
        } else {
            node.b_range
                .children(context.policy().limits().max_subdivision_levels)?
        };
        for range in children {
            let child = Candidate {
                lower: Rat::zero(),
                value: Rat::zero(),
                upper: Rat::zero(),
                physical_classification: None,
                a: node.a,
                b: node.b,
                a_range: if split_a { range } else { node.a_range },
                b_range: if split_a { node.b_range } else { range },
                serial,
                precision_bits: node.precision_bits,
                sample_error: Rat::zero(),
            };
            let child = sample(
                &prepared, aa, bb, &a_lengths, &b_lengths, child, context, progress, goal,
            )?;
            match child.physical_classification {
                Some(true) => return Ok(MinimumAnswer::Physical(true)),
                Some(false) => continue,
                None => {}
            }
            best = best.min(if goal.physical() {
                child.upper.clone()
            } else {
                child.value.clone()
            });
            if child.lower <= best.add(&epsilon) {
                push(&mut queue, child, context, retained)?;
            }
            serial = serial
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow("distance traversal serial"))?;
        }
    }
    Ok(match goal {
        MinimumGoal::Estimate => MinimumAnswer::Estimate(best),
        MinimumGoal::Physical(threshold) => MinimumAnswer::Physical(&best <= threshold),
    })
}

fn push(
    queue: &mut BinaryHeap<Candidate>,
    node: Candidate,
    context: &mut MetricContext,
    retained: &mut u64,
) -> Result<(), GeoError> {
    if queue.len() == queue.capacity() {
        let capacity = queue.capacity().saturating_mul(2).max(4);
        let growth = capacity.saturating_sub(queue.capacity());
        let bytes = (growth as u64)
            .checked_mul(2048)
            .ok_or_else(|| GeoError::MemoryExhausted {
                limit: context.policy().limits().max_workspace_bytes,
            })?;
        context.admit_workspace(bytes)?;
        *retained = retained
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("distance retained workspace"))?;
        queue
            .try_reserve_exact(capacity.saturating_sub(queue.len()))
            .map_err(|_| GeoError::MemoryExhausted {
                limit: context.policy().limits().max_workspace_bytes,
            })?;
    }
    queue.push(node);
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Complete global box operands and borrowed admission remain explicit.
fn sample(
    prepared: &PreparedGeodesic,
    aa: &[Primitive<'_>],
    bb: &[Primitive<'_>],
    a_lengths: &[Rat],
    b_lengths: &[Rat],
    node: Candidate,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    goal: MinimumGoal<'_>,
) -> Result<Candidate, GeoError> {
    let mut storage = 0;
    let result = sample_inner(
        prepared,
        aa,
        bb,
        a_lengths,
        b_lengths,
        node,
        context,
        progress,
        goal,
        &mut storage,
    );
    // Every sampled point/error owner is dropped by sample_inner on either
    // exit. Its child-preadmitted output allowance remains live until here.
    context.release_workspace(storage)?;
    result
}

#[expect(
    clippy::too_many_arguments,
    reason = "one global sample keeps original domains, proof goal and scoped output admission explicit"
)]
fn sample_inner(
    prepared: &PreparedGeodesic,
    aa: &[Primitive<'_>],
    bb: &[Primitive<'_>],
    a_lengths: &[Rat],
    b_lengths: &[Rat],
    mut node: Candidate,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    goal: MinimumGoal<'_>,
    storage: &mut u64,
) -> Result<Candidate, GeoError> {
    let mut child = context.remaining_child()?;
    let parallel = parallel_foot(aa[node.a], bb[node.b], context.reference().ellipsoid())?;
    let has_parallel_minimum = parallel.is_some();
    let reflection = parallel
        .as_ref()
        .and_then(|(_, _, reflection)| reflection.clone());
    let ((ap, a_error), (bp, b_error)) = if let Some((point, foot, _)) = parallel {
        ((point.clone(), Rat::zero()), (foot, Rat::zero()))
    } else {
        let ap = {
            let mut observer = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            aa[node.a].point(
                &node.a_range.middle(),
                &mut child,
                &mut observer,
                goal.physical(),
                node.precision_bits,
            )
        };
        let ap = progress.absorb_child_result(context, &child, ap)?;
        context.retain_workspace(ap.storage, storage)?;
        progress.context_poll(context)?;
        child = context.remaining_child()?;
        let bp = {
            let mut observer = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            bb[node.b].point(
                &node.b_range.middle(),
                &mut child,
                &mut observer,
                goal.physical(),
                node.precision_bits,
            )
        };
        let bp = progress.absorb_child_result(context, &child, bp)?;
        context.retain_workspace(bp.storage, storage)?;
        progress.context_poll(context)?;
        ((ap.point, ap.error), (bp.point, bp.error))
    };
    child = context.remaining_child()?;
    let result = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        if goal.physical() {
            prepared.distance_with_proof_metered(&ap, &bp, &mut child, &mut observer)
        } else {
            prepared
                .distance_metered(&ap, &bp, &mut child, &mut observer)
                .map(|answer| {
                    let proof = crate::MetricProofReceipt {
                        lower: Metres::new(
                            answer
                                .value()
                                .exact()
                                .sub(&decimal("0.0000005"))
                                .max(Rat::zero()),
                        ),
                        upper: Metres::new(answer.value().exact().add(&decimal("0.0000005"))),
                        precision_bits: 0,
                        work_items: child.work_items(),
                    };
                    (answer, proof)
                })
        }
    };
    let (answer, proof) = progress.absorb_child_result(context, &child, result)?;
    if let Some(arithmetic) = child.prepared_arithmetic() {
        context.set_prepared_arithmetic(arithmetic);
    }
    node.value = answer.value().exact().clone();
    let arc_error = a_error.add(&b_error);
    node.sample_error = arc_error.clone();
    node.upper = proof.upper.exact().add(&arc_error);
    let sample_lower = proof.lower.exact().sub(&arc_error).max(Rat::zero());
    node.lower = if let Some(reflection) = reflection {
        child = context.remaining_child()?;
        let result = {
            let mut observer = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            prepared.distance_with_proof_metered(&ap, &reflection, &mut child, &mut observer)
        };
        let (answer, proof) = progress.absorb_child_result(context, &child, result)?;
        if goal.physical() {
            proof
                .lower
                .exact()
                .div(&Rat::from_i64(2))
                .expect("positive divisor")
                .max(Rat::zero())
        } else {
            answer
                .value()
                .exact()
                .div(&Rat::from_i64(2))
                .expect("positive divisor")
                .sub(&decimal("0.00000025"))
                .max(Rat::zero())
        }
    } else if has_parallel_minimum {
        if goal.physical() {
            sample_lower
        } else {
            node.value.sub(&decimal("0.0000005")).max(Rat::zero())
        }
    } else {
        (if goal.physical() {
            sample_lower
        } else {
            node.value.sub(&decimal("0.000003"))
        })
        .sub(&node.a_range.radius(&a_lengths[node.a]))
        .sub(&node.b_range.radius(&b_lengths[node.b]))
        .max(Rat::zero())
    };
    if let MinimumGoal::Physical(threshold) = goal
        && node.lower <= *threshold
        && node.upper > *threshold
    {
        // Correctly rounded distance completion is not the threshold proof.
        // If its invocation enclosure straddles an adjusted comparison, ask
        // the true physical point comparator to refine that comparison itself.
        let witness_threshold = threshold.sub(&arc_error);
        if witness_threshold >= *proof.lower.exact()
            && witness_threshold <= *proof.upper.exact()
            && point_within_threshold(prepared, &ap, &bp, &witness_threshold, context, progress)?
        {
            node.physical_classification = Some(true);
        } else if has_parallel_minimum && arc_error.is_zero() {
            node.physical_classification = Some(point_within_threshold(
                prepared, &ap, &bp, threshold, context, progress,
            )?);
        } else {
            let exclusion_threshold = threshold
                .add(&arc_error)
                .add(&node.a_range.radius(&a_lengths[node.a]))
                .add(&node.b_range.radius(&b_lengths[node.b]));
            if exclusion_threshold >= *proof.lower.exact()
                && exclusion_threshold <= *proof.upper.exact()
                && !point_within_threshold(
                    prepared,
                    &ap,
                    &bp,
                    &exclusion_threshold,
                    context,
                    progress,
                )?
            {
                node.physical_classification = Some(false);
            }
        }
    }
    Ok(node)
}

fn point_within_threshold(
    prepared: &PreparedGeodesic,
    a: &LonLat,
    b: &LonLat,
    threshold: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let mut child = context.remaining_child()?;
    let result = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        prepared.within_physical_metered(
            a,
            b,
            &Metres::new(threshold.clone()),
            &mut child,
            &mut observer,
        )
    };
    progress.absorb_child_result(context, &child, result)
}

// Every surface path has length >= |integral M(phi)dphi|. When a
// constant-latitude source edge contains the point's meridian, that bound is
// attained there. This is a global proof, including written long paths/poles.
type ProvenFoot<'a> = (&'a LonLat, LonLat, Option<LonLat>);

fn parallel_foot<'a>(
    a: Primitive<'a>,
    b: Primitive<'a>,
    ellipsoid: &crate::PreparedEllipsoid,
) -> Result<Option<ProvenFoot<'a>>, GeoError> {
    let (point, start, end) = match (a, b) {
        (Primitive::Point(point), Primitive::Edge(PreparedEdge::SourceLinear(edge)))
        | (Primitive::Edge(PreparedEdge::SourceLinear(edge)), Primitive::Point(point)) => {
            (point, edge.start().point(), edge.end().point())
        }
        (Primitive::Point(point), Primitive::ChartEdge(start, end))
        | (Primitive::ChartEdge(start, end), Primitive::Point(point)) => (point, start, end),
        _ => return Ok(None),
    };
    // Meridian reflection is an isometry. Any path from P to the complete
    // meridian, followed by its reflection, joins P to reflected P with twice
    // the path length. Half the independently certified reflected distance is
    // therefore a global lower bound, attained at the equator in this chart.
    if start.longitude() == end.longitude()
        && point.latitude().is_zero()
        && start.latitude().min(end.latitude()) <= &Rat::zero()
        && start.latitude().max(end.latitude()) >= &Rat::zero()
    {
        let delta = crate::geographic::longitude_difference(point.longitude(), start.longitude());
        let limit = Rat::from_i64(90).mul(
            &ellipsoid
                .semiminor()
                .div(ellipsoid.semimajor())
                .expect("positive axis"),
        );
        if delta.abs() <= limit {
            let mut reflected = point.longitude().add(&delta.mul(&Rat::from_i64(2)));
            if reflected > Rat::from_i64(180) {
                reflected = reflected.sub(&Rat::from_i64(360));
            }
            if reflected <= Rat::from_i64(-180) {
                reflected = reflected.add(&Rat::from_i64(360));
            }
            return Ok(Some((
                point,
                LonLat::new(start.longitude().clone(), Rat::zero())?,
                Some(LonLat::new(reflected, Rat::zero())?),
            )));
        }
    }
    if start.latitude() != end.latitude() {
        return Ok(None);
    }
    let west = start.longitude().min(end.longitude());
    let east = start.longitude().max(end.longitude());
    for shift in [-360, 0, 360] {
        let longitude = point.longitude().add(&Rat::from_i64(shift));
        if point.is_pole() || (longitude >= *west && longitude <= *east) {
            let longitude = if point.is_pole() {
                start.longitude().clone()
            } else {
                longitude
            };
            return Ok(Some((
                point,
                LonLat::new(longitude, start.latitude().clone())?,
                None,
            )));
        }
    }
    Ok(None)
}

fn endpoints(primitive: Primitive<'_>) -> Option<(&LonLat, &LonLat)> {
    match primitive {
        Primitive::Point(point) => Some((point, point)),
        Primitive::ImagePoint(_) => None,
        Primitive::Selected(_) => None,
        Primitive::ChartEdge(start, end) => Some((start, end)),
        Primitive::Edge(PreparedEdge::SourceLinear(edge)) => {
            Some((edge.start().point(), edge.end().point()))
        }
        Primitive::Edge(
            PreparedEdge::AzimuthLength(_)
            | PreparedEdge::ShortestGeodesic(_)
            | PreparedEdge::Transformed(_),
        ) => None,
    }
}

fn intersection(
    a_region: &PreparedRegion,
    b_region: &PreparedRegion,
    aa: &[Primitive<'_>],
    bb: &[Primitive<'_>],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    for (points, region) in [(aa, b_region), (bb, a_region)] {
        for point in points {
            let Some(start) = point.start() else {
                if matches!(
                    point,
                    Primitive::Edge(PreparedEdge::Transformed(_)) | Primitive::ImagePoint(_)
                ) && !matches!(region, PreparedRegion::Empty)
                {
                    let mut bits = 96;
                    loop {
                        let mut child = context.remaining_child()?;
                        let enclosure = {
                            let mut observer = progress.nested(
                                context.work_items(),
                                context
                                    .policy()
                                    .limits()
                                    .max_workspace_bytes
                                    .saturating_sub(context.remaining_workspace()),
                                context.workspace_peak(),
                            );
                            point.image_enclosure(&Rat::zero(), bits, &mut child, &mut observer)
                        };
                        let enclosure = progress.absorb_child_result(context, &child, enclosure)?;
                        if let Some(location) = crate::atlas::locate_enclosure(
                            &enclosure.longitude,
                            &enclosure.latitude,
                            region,
                            context,
                            progress,
                        )? {
                            if location != Set::Exterior {
                                return Ok(true);
                            }
                            break;
                        }
                        if bits >= context.policy().limits().max_precision_bits {
                            return Err(GeoError::PrecisionExhausted { bits });
                        }
                        bits = bits
                            .saturating_mul(2)
                            .min(context.policy().limits().max_precision_bits);
                    }
                }
                continue;
            };
            let located = crate::atlas::locate_with_progress(start, region, context, progress)?;
            progress.context_poll(context)?;
            if located != Set::Exterior {
                return Ok(true);
            }
        }
    }
    for a in aa {
        for b in bb {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if let (Primitive::ImagePoint(a), Primitive::ImagePoint(b)) = (a, b)
                && a.id() == b.id()
            {
                return Ok(true);
            }
            if a.start()
                .zip(b.start())
                .is_some_and(|(a, b)| a.same_location(b))
            {
                return Ok(true);
            }
            if let (Some((a0, a1)), Some((b0, b1))) = (endpoints(*a), endpoints(*b)) {
                for shift in [-360, 0, 360] {
                    let coord = |point: &LonLat, shift: i64| {
                        Coord::xy(
                            point.longitude().add(&Rat::from_i64(shift)),
                            point.latitude().clone(),
                        )
                    };
                    if !matches!(
                        crate::topology::intersect(
                            &coord(a0, 0),
                            &coord(a1, 0),
                            &coord(b0, shift),
                            &coord(b1, shift)
                        ),
                        SegmentIntersection::None
                    ) {
                        return Ok(true);
                    }
                }
                for endpoint_a in [a0, a1] {
                    for endpoint_b in [b0, b1] {
                        if endpoint_a.same_location(endpoint_b) {
                            return Ok(true);
                        }
                    }
                }
            } else if let (Primitive::Point(point), Primitive::Edge(edge))
            | (Primitive::Edge(edge), Primitive::Point(point)) = (a, b)
            {
                if crate::atlas::point::contact_with_progress(edge, point, context, progress)? {
                    return Ok(true);
                }
            } else if let (Primitive::Edge(left), Primitive::Edge(right)) = (a, b) {
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
                    crate::atlas::arcs::intersections_metered(
                        left,
                        right,
                        &mut child,
                        &mut observer,
                    )
                };
                if !progress
                    .absorb_child_result(context, &child, contacts)?
                    .is_empty()
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn decimal(value: &str) -> Rat {
    Rat::parse_decimal(value).expect("frozen distance decimal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbolic_point_inventory_keeps_dimension_zero_and_original_physical_thresholds() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationModel, OperationReference,
        };
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let reference = crate::GeographicReference::wgs84();
        let chain = Arc::new(
            OperationChain::compile(vec![
                CoordinateOperation::compile(
                    OperationReference {
                        realization: Digest32::new([17; 32]),
                        unit: CoordinateUnit::Metres,
                        swapped_axes: false,
                    },
                    OperationReference {
                        realization: reference.id().digest(),
                        unit: CoordinateUnit::Degrees,
                        swapped_axes: false,
                    },
                    OperationModel::MercatorToGeographic {
                        radius: Rat::from_i64(6_378_137),
                        eccentricity_squared: Rat::zero(),
                        square_domain: true,
                    },
                )
                .unwrap(),
            ])
            .unwrap(),
        );
        let policy = crate::ExecutionPolicy::geometry();
        let image = |x: i64| {
            let point = crate::OperationImagePoint::new(
                Coord::xy(Rat::from_i64(x), Rat::zero()),
                Arc::clone(&chain),
                reference.clone(),
                None,
                policy,
            )
            .unwrap();
            PreparedGeometry::from_parts_with_symbolic(
                reference.clone(),
                Vec::new(),
                vec![point],
                Vec::new(),
                PreparedRegion::Empty,
                policy,
            )
            .unwrap()
        };
        let a = image(0);
        let b = image(1);
        assert!(a.points().is_empty() && a.curves().is_empty());
        let mut context = MetricContext::wgs84().unwrap();
        assert_eq!(distance(&a, &b, &mut context).unwrap().exact(), &Rat::one());
        assert!(within_physical(&a, &a, &Metres::new(Rat::zero()), &mut context).unwrap());
        let below = Metres::new(decimal("0.999999999"));
        let above = Metres::new(decimal("1.000000001"));
        assert!(!within_physical(&a, &b, &below, &mut context).unwrap());
        assert!(within_physical(&a, &b, &above, &mut context).unwrap());
        let source = Arc::new(b);
        let origin = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        for (radius, expected) in [(below, false), (above, true)] {
            let offset = crate::OffsetRegion::new(Arc::clone(&source), radius, policy).unwrap();
            assert_eq!(offset.contains(&origin, &mut context).unwrap(), expected);
        }
    }
    fn geometry(text: &str) -> PreparedGeometry {
        let crs = crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        PreparedGeometry::from_literal(
            &crate::wkt::parse(text, &crs).unwrap(),
            &crate::GeoProfile::standard(),
        )
        .unwrap()
    }

    #[test]
    fn general_points_preserve_the_correctly_rounded_point_answer() {
        let a = geometry("POINT(0 0)");
        let b = geometry("POINT(1 0)");
        let mut context = MetricContext::wgs84().unwrap();
        let result = distance(&a, &b, &mut context).unwrap();
        assert_eq!(result.exact(), &decimal("111319.490793"));
        assert_eq!(result.error_bound(), decimal("0.00025"));
    }

    #[test]
    fn complete_edge_interior_and_written_long_path_use_global_meridian_bound() {
        let point = geometry("POINT(0 1)");
        let line = geometry("LINESTRING(-2 0,2 0)");
        let long = geometry("LINESTRING(170 0,-170 0)");
        let mut context = MetricContext::wgs84().unwrap();
        let first = distance(&point, &line, &mut context).unwrap();
        assert_eq!(first.exact(), &decimal("110574.388558"));
        assert_eq!(
            distance(&point, &long, &mut context).unwrap().exact(),
            first.exact()
        );
    }

    #[test]
    fn physical_edge_threshold_refines_beyond_point_rounding_uncertainty() {
        let point = geometry("POINT(0.000001 0)");
        let meridian = geometry("LINESTRING(0 -1,0 1)");
        // The equatorial reflection argument proves this is the global minimum.
        // An independent 50-place decimal pi bounds the analytic distance much
        // more tightly than the two one-nanometre threshold offsets below.
        let truth = decimal("3.14159265358979323846264338327950288419716939937510")
            .mul(&Rat::from_i64(6_378_137))
            .div(&Rat::from_i64(180_000_000))
            .unwrap();
        let delta = decimal("0.000000001");
        let mut context = MetricContext::wgs84().unwrap();
        assert!(
            !within_physical(
                &point,
                &meridian,
                &Metres::new(truth.sub(&delta)),
                &mut context,
            )
            .unwrap()
        );
        assert!(
            within_physical(
                &point,
                &meridian,
                &Metres::new(truth.add(&delta)),
                &mut context,
            )
            .unwrap()
        );
        let reported = distance(&point, &meridian, &mut context).unwrap();
        assert!(reported.exact() < &truth.sub(&delta));
    }

    #[test]
    fn union_complement_excludes_internal_original_ring_edges() {
        let source =
            geometry("MULTIPOLYGON(((-2 -2,1 -2,1 2,-2 2,-2 -2)),((-1 -2,2 -2,2 2,-1 2,-1 -2)))");
        let complementary = PreparedGeometry::from_parts(
            source.reference().clone(),
            Vec::new(),
            Vec::new(),
            source.region().clone().complement(),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let point = geometry("POINT(0 0)");
        let foot = LonLat::new(Rat::zero(), Rat::from_i64(2)).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let expected = crate::geodesic::distance(point.points()[0].point().clone(), foot).unwrap();
        assert_eq!(
            distance(&point, &complementary, &mut context)
                .unwrap()
                .exact(),
            expected.value().exact()
        );
        assert!(
            distance(&geometry("POINT(0 3)"), &complementary, &mut context)
                .unwrap()
                .exact()
                .is_zero()
        );
    }

    #[test]
    fn exact_atlas_contacts_holes_and_longitude_aliases_are_shared() {
        let crossing = geometry("LINESTRING(-1 -1,1 1)");
        let other = geometry("LINESTRING(-1 1,1 -1)");
        let point = geometry("POINT(0 0)");
        let polygon = geometry("POLYGON((-2 -2,2 -2,2 2,-2 2,-2 -2))");
        let seam = geometry("LINESTRING(180 -1,180 1)");
        let alias = geometry("POINT(-180 0)");
        let mut context = MetricContext::wgs84().unwrap();
        for (a, b) in [
            (&crossing, &other),
            (&point, &crossing),
            (&point, &polygon),
            (&seam, &alias),
        ] {
            assert!(distance(a, b, &mut context).unwrap().exact().is_zero());
        }
    }

    #[test]
    fn metered_refusal_is_original_and_no_estimate_is_published() {
        struct Refuse;
        impl MetricWorkObserver for Refuse {
            fn charge_chunk(&mut self, _work: u64, _bytes: u64) -> Result<(), GeoError> {
                Err(GeoError::WorkExhausted { limit: 17 })
            }
        }
        let a = geometry("POINT(0 0)");
        let b = geometry("POINT(1 0)");
        let mut context = MetricContext::wgs84().unwrap();
        assert_eq!(
            distance_metered(&a, &b, &mut context, &mut Refuse),
            Err(GeoError::WorkExhausted { limit: 17 })
        );
    }
}
