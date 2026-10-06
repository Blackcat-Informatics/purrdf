// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified oriented Jordan regions through the smooth geodetic-normal map.
//!
//! For a query normal q and oriented boundary n(t), the one-form
//! (−q×n)·dn/(1−q·n) integrates to the Left-region normal-sphere area minus
//! 4π when the query belongs to that side. A proper Jordan region's area lies
//! strictly between zero and 4π, so its complete integral sign decides the side.
//! Complete panel images enclose the integral; sampling and successive numerical
//! quadrature differences never establish membership.

use super::arcs::{self, ChartLine, CurveIntersection};
use crate::context::WorkProgress;
use crate::geodesic::normal::{FixedArithmetic, NormalArithmetic, TaylorArithmetic};
use crate::numerical::{fixed_from_rat, geo_math_error};
use crate::{
    GeoError, LonLat, MetricContext, MetricWorkObserver, PreparedCurve, PreparedEdge, Rat, Set,
};
use core::cmp::Ordering;
use purrdf_core::SmallVec;
use std::collections::BinaryHeap;

use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, FixedIntervalSum, MathError},
};

/// A source-defined closed Jordan ring with its left side explicitly selected.
/// The ring retains exact endpoints, chosen geodesic branches and continuous
/// operation images. No smaller-area convention or sampled carrier is used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrientedRing {
    curve: PreparedCurve,
}
impl OrientedRing {
    /// Consume the shared complete source-cell proof. The caller constructs
    /// this ring directly from that convex cell's original operation edges,
    /// reversing its traversal exactly when its certified orientation is
    /// negative. Whole-cell injectivity in the certified nonpolar chart proves
    /// closure and the Jordan property without a second contact solve.
    pub(crate) fn from_operation_cell(
        curve: PreparedCurve,
        certificate: &crate::operation::AreaCellCertificate,
        reversed: bool,
    ) -> Result<Self, GeoError> {
        if !certificate.physical_chart()
            || reversed != (certificate.orientation() < 0)
            || curve.edges().is_empty()
        {
            return Err(GeoError::config(
                "operation cell Jordan certificate mismatch",
            ));
        }
        Ok(Self { curve })
    }
    /// Certify exact closure and absence of self contacts before selecting Left.
    ///
    /// # Errors
    /// Refuses empty/constant rings, non-Jordan contacts, reference mismatch,
    /// uncertain closure and incomplete numerical or governor admission.
    pub fn new(curve: PreparedCurve, context: &mut MetricContext) -> Result<Self, GeoError> {
        Self::prepare(curve, context, None)
    }
    /// Certify one original ring with bounded external governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::new`].
    pub fn new_metered(
        curve: PreparedCurve,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(curve, context, Some(observer))
    }
    fn prepare(
        curve: PreparedCurve,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        let bytes = arcs::source_workspace_bound(curve.edges().iter());
        context.admit_workspace(bytes)?;
        let result = progress
            .context_poll(context)
            .and_then(|()| validate_ring(&curve, context, &mut progress));
        context.release_workspace(bytes)?;
        result?;
        Ok(Self { curve })
    }
    /// Exact modeled ring, in its declared traversal order.
    #[must_use]
    pub const fn curve(&self) -> &PreparedCurve {
        &self.curve
    }
    /// Consume the certified witness while retaining the exact modeled curve.
    #[must_use]
    pub fn into_curve(self) -> PreparedCurve {
        self.curve
    }
    /// Classify the proper Left side, identifying every physical seam and pole.
    ///
    /// # Errors
    /// Refuses an unresolved boundary/sign decision or incomplete admission.
    pub fn locate_left(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
    ) -> Result<Set, GeoError> {
        self.locate(point, context, None)
    }
    /// Classify Left with bounded external governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::locate_left`].
    pub fn locate_left_metered(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Set, GeoError> {
        self.locate(point, context, Some(observer))
    }
    fn locate(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Set, GeoError> {
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        check_curve_binding(&self.curve, context, &mut progress)?;
        let source = arcs::source_workspace_bound(self.curve.edges().iter());
        context.admit_workspace(source)?;
        let result = progress
            .context_poll(context)
            .and_then(|()| locate_validated(&self.curve, point, context, &mut progress));
        context.release_workspace(source)?;
        result
    }
}

pub(crate) fn validate_ring(
    curve: &PreparedCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    check_curve_binding(curve, context, progress)?;
    let edges = curve.edges();
    if edges.is_empty() {
        return Err(GeoError::domain("empty oriented ring"));
    }
    for (index, edge) in edges.iter().enumerate() {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        let next = &edges[(index + 1) % edges.len()];
        let same_mapping = match (edge, next) {
            (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) => {
                a.same_mapping_admitted(b, context, progress)?
            }
            _ => false,
        };
        let operands = edge
            .original_coordinates()
            .chain(next.original_coordinates())
            .flat_map(|coordinate| {
                [coordinate.x(), coordinate.y()]
                    .into_iter()
                    .chain(coordinate.z())
                    .chain(coordinate.m())
            })
            .collect::<SmallVec<[&Rat; 16]>>();
        let closed = crate::numerical::ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &operands,
            16,
            || Ok(junction_equal(edge, next, same_mapping)),
        )?;
        if !closed {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        match edge {
            PreparedEdge::SourceLinear(line) => {
                if line.start().point() == line.end().point()
                    || (line.start().point().is_pole()
                        && line.end().point().same_location(line.start().point()))
                {
                    return Err(GeoError::domain("constant edge in oriented ring"));
                }
                // Original coordinates vary affinely, with longitude span at
                // most one turn. Identified endpoints are the only possible
                // alias of a nonconstant edge; a complete parallel is simple.
            }
            PreparedEdge::ShortestGeodesic(arc) => {
                if arc.start().point().same_location(arc.end().point()) {
                    return Err(GeoError::domain("constant edge in oriented ring"));
                }
                // A unique shortest positive path cannot self-intersect: its
                // loop could otherwise be removed to obtain a shorter path.
            }
            PreparedEdge::AzimuthLength(_) | PreparedEdge::Transformed(_) => {
                certify_numeric_simple(edge, context, progress)?;
            }
        }
    }
    for (i, left) in edges.iter().enumerate() {
        for (j, right) in edges.iter().enumerate().skip(i + 1) {
            let mut child = context.remaining_child()?;
            let contacts = {
                let mut nested = progress.nested(
                    context.work_items(),
                    context
                        .policy()
                        .limits()
                        .max_workspace_bytes
                        .saturating_sub(context.remaining_workspace()),
                    context.workspace_peak(),
                );
                arcs::intersections_metered(left, right, &mut child, &mut nested)
            };
            for contact in progress.absorb_child_result(context, &child, contacts)? {
                let (CurveIntersection::Endpoint { left, right, .. }
                | CurveIntersection::Exact { left, right, .. }
                | CurveIntersection::SymbolicEndpoint { left, right }
                | CurveIntersection::SymbolicSourceContact { left, right }) = contact
                else {
                    return Err(GeoError::domain("non-Jordan oriented ring contact"));
                };
                let adjacent = j == i + 1 && left == Rat::one() && right == Rat::zero();
                let closure =
                    i == 0 && j == edges.len() - 1 && left == Rat::zero() && right == Rat::one();
                if !adjacent && !closure {
                    return Err(GeoError::domain("self contact in oriented ring"));
                }
            }
        }
    }
    Ok(())
}

fn junction_equal(left: &PreparedEdge, right: &PreparedEdge, same_mapping: bool) -> bool {
    if let (PreparedEdge::Transformed(a), PreparedEdge::Transformed(b)) = (left, right) {
        let end = a.source_endpoints().1;
        let start = b.source_endpoints().0;
        return same_mapping && end.same_planar(start) && end.z() == start.z();
    }
    let Some(start) = right.start().map(crate::PreparedCoordinate::point) else {
        return false;
    };
    match left {
        PreparedEdge::SourceLinear(edge) => edge.end().point().same_location(start),
        PreparedEdge::ShortestGeodesic(arc) => arc.end().point().same_location(start),
        PreparedEdge::AzimuthLength(arc) if arc.length().exact().is_zero() => {
            arc.start().point().same_location(start)
        }
        PreparedEdge::AzimuthLength(_) | PreparedEdge::Transformed(_) => false,
    }
}

fn check_curve_binding(
    curve: &PreparedCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let binding = crate::numerical::reference_identity(context, progress, None)?;
    for edge in curve.edges() {
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

/// Classify an original Jordan ring whose source and immutable context binding
/// have already been checked by its enclosing public invocation. Repeated
/// sector/face queries share that admission rather than render the same ID.
pub(super) fn locate_validated(
    curve: &PreparedCurve,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    if let Some(location) = parallel_location(curve, point, context, progress)? {
        return Ok(location);
    }
    if let Some(location) = linear_chart_location(curve, point, context, progress)? {
        return Ok(location);
    }
    for edge in curve.edges() {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if super::point::exact_source(edge, point, context, progress)? == Some(true) {
            return Ok(Set::Boundary);
        }
    }
    let policy = context.policy();
    let mut bits = 80.min(policy.limits().max_precision_bits);
    loop {
        let result = flux_attempt(curve, point, bits, context, progress);
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

/// A proper source-linear Jordan ring of longitude span below one full turn
/// has the same bounded component as its exact coordinate-chart ring. Adjacent
/// writings of the same exact pole are joined only along that pole latitude;
/// this connector has a constant physical image. The normal map preserves
/// orientation on every nonpolar chart interior, so signed planar orientation
/// selects Left without selecting the smaller surface region.
fn linear_chart_location(
    curve: &PreparedCurve,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    use crate::numerical::{ExactAdmission, exact_rational};
    use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalCompare};
    let edges = curve.edges();
    if edges.is_empty()
        || edges
            .iter()
            .any(|edge| !matches!(edge, PreparedEdge::SourceLinear(_)))
    {
        return Ok(None);
    }
    context.charge_work(edges.len() as u64 + 1)?;
    progress.context_poll(context)?;
    let storage = edges
        .iter()
        .flat_map(PreparedEdge::original_coordinates)
        .fold(
            (edges.len() as u64 + 1).saturating_mul(size_of::<crate::Coord>() as u64),
            |bytes, coordinate| {
                bytes
                    .saturating_add(coordinate.x().allocated_bytes() as u64)
                    .saturating_add(coordinate.y().allocated_bytes() as u64)
            },
        )
        .saturating_mul(2);
    context.admit_workspace(storage)?;
    let result = (|| {
        let mut ring = Vec::with_capacity(edges.len().saturating_mul(2) + 1);
        let mut minimum: Option<&Rat> = None;
        let mut maximum: Option<&Rat> = None;
        let mut touches_query_pole = false;
        for (index, edge) in edges.iter().enumerate() {
            let PreparedEdge::SourceLinear(line) = edge else {
                unreachable!("source-linear chart");
            };
            let next = edges[(index + 1) % edges.len()]
                .start()
                .expect("source-linear endpoint")
                .point();
            let start = line.start().point();
            let end = line.end().point();
            let (closes, connector, touches) = ExactAdmission::new(context, progress).rational(
                Linear,
                &[
                    end.longitude(),
                    end.latitude(),
                    next.longitude(),
                    next.latitude(),
                    start.latitude(),
                    point.latitude(),
                ],
                16,
                || {
                    let pole_join =
                        end.is_pole() && next.is_pole() && end.latitude() == next.latitude();
                    Ok((
                        end == next || pole_join,
                        end != next && pole_join,
                        point.is_pole()
                            && (start.latitude() == point.latitude()
                                || end.latitude() == point.latitude()),
                    ))
                },
            )?;
            if !closes {
                return Ok(None);
            }
            touches_query_pole |= touches;
            for longitude in [start.longitude(), end.longitude()] {
                if let (Some(low), Some(high)) = (minimum, maximum) {
                    let (lower, higher) = ExactAdmission::new(context, progress).rational(
                        RationalCompare,
                        &[longitude, low, high],
                        2,
                        || Ok((longitude < low, longitude > high)),
                    )?;
                    if lower {
                        minimum = Some(longitude);
                    }
                    if higher {
                        maximum = Some(longitude);
                    }
                } else {
                    minimum = Some(longitude);
                    maximum = Some(longitude);
                }
            }
            if index == 0 {
                ring.push(super::chart_point(start, context, progress)?);
            }
            ring.push(super::chart_point(end, context, progress)?);
            if connector {
                ring.push(super::chart_point(next, context, progress)?);
            }
        }
        let low = minimum.expect("nonempty original ring");
        let high = maximum.expect("nonempty original ring");
        let mut admission = ExactAdmission::new(context, progress);
        let span = exact_rational(Some(&mut admission), RationalAdd, &[low, high], || {
            high.sub(low)
        })?;
        if !admission.rational(RationalCompare, &[&span], 1, || {
            Ok(span < Rat::from_i64(360))
        })? {
            return Ok(None);
        }
        let orientation = crate::measure::signed_ring_orientation_admitted(&ring, &mut admission)?;
        if orientation == 0 {
            return Ok(None);
        }
        let polar = admission.rational(Linear, &[point.latitude()], 1, || Ok(point.is_pole()))?;
        let status = if polar {
            if touches_query_pole {
                Set::Boundary
            } else {
                Set::Exterior
            }
        } else {
            let mut query = super::chart_point(point, context, progress)?;
            if let Some(alias) =
                super::cut_alias(&query, &mut ExactAdmission::new(context, progress))?
            {
                let inside = ExactAdmission::new(context, progress).rational(
                    RationalCompare,
                    &[alias.x(), low, high],
                    2,
                    || Ok(alias.x() >= low && alias.x() <= high),
                )?;
                if inside {
                    query = alias;
                }
            }
            crate::topology::locate_surface_admitted(
                &query,
                &vec![ring],
                &mut ExactAdmission::new(context, progress),
            )?
        };
        Ok(Some(if orientation > 0 {
            status
        } else {
            super::complement(status)
        }))
    })();
    context.release_workspace(storage)?;
    result
}

/// A complete written parallel is an exact Jordan separator: positive longitude
/// traversal has north on its Left, negative traversal has south on its Left.
/// This tests the original complete curve law before any flux preparation.
fn parallel_location(
    curve: &PreparedCurve,
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Set>, GeoError> {
    let [PreparedEdge::SourceLinear(line)] = curve.edges() else {
        return Ok(None);
    };
    let a = line.start().point();
    let b = line.end().point();
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    let parallel = admission.rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[a.latitude(), b.latitude()],
        2,
        || Ok(a.latitude() == b.latitude()),
    )?;
    if !parallel {
        return Ok(None);
    }
    let delta = crate::numerical::exact_rational(
        Some(&mut admission),
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[a.longitude(), b.longitude()],
        || b.longitude().sub(a.longitude()),
    )?;
    let complete = admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[&delta],
        2,
        || Ok(delta.abs() == Rat::from_i64(360)),
    )?;
    if !complete {
        return Ok(None);
    }
    admission.rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.latitude(), a.latitude()],
        1,
        || {
            Ok(Some(match point.latitude().cmp(a.latitude()) {
                Ordering::Equal => Set::Boundary,
                side if (side == Ordering::Greater) == (delta.signum() > 0) => Set::Interior,
                _ => Set::Exterior,
            }))
        },
    )
}

#[derive(PartialEq, Eq)]
struct FluxPanel {
    edge: usize,
    parameter: FixedInterval,
    depth: u32,
    integral: FixedInterval,
    width: BigInt,
}
impl Ord for FluxPanel {
    fn cmp(&self, other: &Self) -> Ordering {
        self.width
            .cmp(&other.width)
            .then_with(|| other.edge.cmp(&self.edge))
            .then_with(|| other.parameter.lower().cmp(self.parameter.lower()))
    }
}
impl PartialOrd for FluxPanel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
fn flux_attempt(
    curve: &PreparedCurve,
    point: &LonLat,
    bits: u32,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let policy = context.policy();
    arcs::with_lines(
        curve.edges(),
        bits,
        context,
        progress,
        |lines, math, progress| {
            let q = query_normal(point, math).map_err(|e| geo_math_error(&e, policy))?;
            let parameter = FixedInterval::from_bounds(
                BigInt::zero(),
                BigInt::from_i128(1).mul_pow2(bits),
                math,
            )
            .map_err(|e| geo_math_error(&e, policy))?;
            let sum = FixedIntervalSum::new(math).map_err(|e| geo_math_error(&e, policy))?;
            let mut reserved = 0;
            FluxIntegrator {
                lines,
                query: &q,
                iterations: policy.limits().max_iterations,
                max_depth: policy.limits().max_subdivision_levels,
                math,
                progress,
                reserved: &mut reserved,
                panels: BinaryHeap::new(),
                sum,
            }
            .integrate(&parameter)
            .map_err(|e| geo_math_error(&e, policy))
        },
    )
}

struct FluxIntegrator<'math, 'observer> {
    lines: &'math [ChartLine],
    query: &'math [FixedInterval; 3],
    iterations: u32,
    max_depth: u32,
    math: &'math mut CoordinateMath,
    progress: &'math mut WorkProgress<'observer>,
    reserved: &'math mut usize,
    panels: BinaryHeap<FluxPanel>,
    sum: FixedIntervalSum,
}
impl FluxIntegrator<'_, '_> {
    fn retain(&mut self, bytes: usize) -> Result<(), MathError> {
        self.math.reserve_workspace(bytes)?;
        *self.reserved = self
            .reserved
            .checked_add(bytes)
            .ok_or(MathError::WorkspaceExhausted)?;
        Ok(())
    }
    fn integrate(mut self, parameter: &FixedInterval) -> Result<Set, MathError> {
        for edge in 0..self.lines.len() {
            self.append(edge, parameter.clone(), 0)?;
        }
        loop {
            self.progress.math_poll(self.math)?;
            let total = self.sum.enclosure(self.math)?;
            if total.upper().is_negative() {
                return Ok(Set::Interior);
            }
            if total.lower() > &BigInt::zero() {
                return Ok(Set::Exterior);
            }
            let panel = self
                .panels
                .pop()
                .ok_or(MathError::Domain("empty oriented integral"))?;
            if panel.depth >= self.max_depth {
                return Err(MathError::PrecisionExhausted);
            }
            self.sum.remove(&panel.integral, self.math)?;
            for child in split_parameter(&panel.parameter, self.math)? {
                self.append(panel.edge, child, panel.depth + 1)?;
            }
        }
    }
    fn append(
        &mut self,
        edge: usize,
        parameter: FixedInterval,
        depth: u32,
    ) -> Result<(), MathError> {
        let mut pending =
            purrdf_lex::walk::WorkList::<SimplePanel, 8>::with(SimplePanel { parameter, depth });
        while let Some(panel) = pending.pop() {
            self.progress.math_poll(self.math)?;
            let result = panel_flux(
                &self.lines[edge],
                &panel.parameter,
                self.query,
                self.iterations,
                self.math,
                self.progress,
            );
            match result {
                Ok(integral) => {
                    let width = integral.width(self.math)?.lower().clone();
                    let bytes = panel
                        .parameter
                        .workspace_bytes()
                        .saturating_add(integral.workspace_bytes())
                        .saturating_add(width.allocated_bytes())
                        .saturating_add(size_of::<FluxPanel>())
                        .saturating_mul(2);
                    self.retain(bytes)?;
                    self.sum.add(&integral, self.math)?;
                    self.panels.push(FluxPanel {
                        edge,
                        parameter: panel.parameter,
                        depth: panel.depth,
                        integral,
                        width,
                    });
                }
                Err(MathError::PrecisionExhausted) if panel.depth < self.max_depth => {
                    self.retain(
                        panel
                            .parameter
                            .workspace_bytes()
                            .saturating_add(size_of::<SimplePanel>())
                            .saturating_mul(4),
                    )?;
                    let [left, right] = split_parameter(&panel.parameter, self.math)?;
                    pending.push(SimplePanel {
                        parameter: right,
                        depth: panel.depth + 1,
                    });
                    pending.push(SimplePanel {
                        parameter: left,
                        depth: panel.depth + 1,
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
pub(super) fn split_parameter(
    parameter: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 2], MathError> {
    let midpoint = parameter.midpoint(math)?;
    if midpoint.lower() <= parameter.lower() || midpoint.upper() >= parameter.upper() {
        return Err(MathError::PrecisionExhausted);
    }
    Ok([
        FixedInterval::from_bounds(parameter.lower().clone(), midpoint.upper().clone(), math)?,
        FixedInterval::from_bounds(midpoint.lower().clone(), parameter.upper().clone(), math)?,
    ])
}
pub(super) fn query_normal(
    point: &LonLat,
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 3], MathError> {
    if point.is_pole() {
        let zero = FixedInterval::from_i64(0, math)?;
        return Ok([
            zero.clone(),
            zero,
            FixedInterval::from_i64(
                if point.latitude().numerator().is_negative() {
                    -1
                } else {
                    1
                },
                math,
            )?,
        ]);
    }
    let radians = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
    let longitude = fixed_from_rat(point.longitude(), math)?.mul(&radians, math)?;
    let latitude = fixed_from_rat(point.latitude(), math)?.mul(&radians, math)?;
    Ok(arcs::normal_from_angles(&longitude, &latitude, None, math)?.0)
}
fn panel_flux(
    line: &ChartLine,
    parameter: &FixedInterval,
    q: &[FixedInterval; 3],
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    if !matches!(line, ChartLine::Transformed { .. }) {
        let middle = parameter.midpoint(math)?;
        let half = parameter
            .width(math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?;
        let panel = purrdf_xsd::math::SymmetricTaylorPanel::new(&middle, parameter, &half, math)?;
        let (integral, remainder) = purrdf_xsd::math::integrate_taylor_panel_observed(
            4,
            192,
            panel,
            math,
            progress,
            |math, progress| progress.math_poll(math),
            |parameter, workspace, math, progress| {
                let (normal, derivative) = line
                    .normal_taylor_in(parameter, iterations, workspace, math, progress)?
                    .ok_or(MathError::PrecisionExhausted)?;
                let q0 = workspace.constant(q[0].clone(), math)?;
                let q1 = workspace.constant(q[1].clone(), math)?;
                let q2 = workspace.constant(q[2].clone(), math)?;
                flux_density(
                    &normal,
                    &derivative,
                    &[q0, q1, q2],
                    &mut TaylorArithmetic { workspace, math },
                )
            },
        )?;
        return integral.add(&remainder.neg(math)?.hull(&remainder, math)?, math);
    }
    let (normal, derivative) = line.normal(parameter, iterations, math, progress)?;
    let derivative = derivative.ok_or(MathError::PrecisionExhausted)?;
    flux_density(&normal, &derivative, q, &mut FixedArithmetic(math))?
        .mul(&parameter.width(math)?, math)
}
fn flux_density<A: NormalArithmetic>(
    normal: &[A::Value; 3],
    derivative: &[A::Value; 3],
    q: &[A::Value; 3],
    arithmetic: &mut A,
) -> Result<A::Value, MathError> {
    let dot0 = arithmetic.mul(&q[0], &normal[0])?;
    let dot1 = arithmetic.mul(&q[1], &normal[1])?;
    let dot2 = arithmetic.mul(&q[2], &normal[2])?;
    let dot01 = arithmetic.add(&dot0, &dot1)?;
    let dot = arithmetic.add(&dot01, &dot2)?;
    let one = arithmetic.one()?;
    let denominator = arithmetic.sub(&one, &dot)?;
    let mut cross = |a: usize, b: usize| {
        let first = arithmetic.mul(&q[a], &normal[b])?;
        let second = arithmetic.mul(&q[b], &normal[a])?;
        arithmetic.sub(&first, &second)
    };
    let crosses = [cross(1, 2)?, cross(2, 0)?, cross(0, 1)?];
    let product0 = arithmetic.mul(&crosses[0], &derivative[0])?;
    let product1 = arithmetic.mul(&crosses[1], &derivative[1])?;
    let product2 = arithmetic.mul(&crosses[2], &derivative[2])?;
    let sum = arithmetic.add(&product0, &product1)?;
    let sum = arithmetic.add(&sum, &product2)?;
    let numerator = arithmetic.neg(&sum)?;
    arithmetic.div(&numerator, &denominator)
}

fn certify_numeric_simple(
    edge: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let bits = 80.min(context.policy().limits().max_precision_bits);
    let line_context = context.remaining_child()?;
    let policy = context.policy();
    arcs::with_lines(
        core::slice::from_ref(edge),
        bits,
        context,
        progress,
        |lines, math, progress| {
            let parameter = FixedInterval::from_bounds(
                BigInt::zero(),
                BigInt::from_i128(1).mul_pow2(bits),
                math,
            )
            .map_err(|e| geo_math_error(&e, policy))?;
            certify_simple_line(&lines[0], parameter, &line_context, math, progress)
        },
    )
}

struct SimplePanel {
    parameter: FixedInterval,
    depth: u32,
}
fn certify_simple_line(
    line: &ChartLine,
    parameter: FixedInterval,
    context: &MetricContext,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let policy = context.policy();
    let mut panels = vec![SimplePanel {
        parameter,
        depth: 0,
    }];
    let mut reserved = 0usize;
    let result = (|| {
        'partition: loop {
            for i in 0..panels.len() {
                progress
                    .math_poll(math)
                    .map_err(|e| geo_math_error(&e, policy))?;
                if !monotone(
                    line,
                    &panels[i].parameter,
                    policy.limits().max_iterations,
                    math,
                    progress,
                )
                .map_err(|e| geo_math_error(&e, policy))?
                {
                    split_simple(
                        i,
                        &mut panels,
                        policy.limits().max_subdivision_levels,
                        math,
                        &mut reserved,
                    )
                    .map_err(|e| geo_math_error(&e, policy))?;
                    continue 'partition;
                }
            }
            for i in 0..panels.len() {
                for j in i + 1..panels.len() {
                    let hull = panels[i]
                        .parameter
                        .hull(&panels[j].parameter, math)
                        .map_err(|e| geo_math_error(&e, policy))?;
                    if monotone(line, &hull, policy.limits().max_iterations, math, progress)
                        .map_err(|e| geo_math_error(&e, policy))?
                    {
                        continue;
                    }
                    if j == i + 1 {
                        split_simple(
                            i,
                            &mut panels,
                            policy.limits().max_subdivision_levels,
                            math,
                            &mut reserved,
                        )
                        .map_err(|e| geo_math_error(&e, policy))?;
                        continue 'partition;
                    }
                    let domain = [panels[i].parameter.clone(), panels[j].parameter.clone()];
                    if !arcs::search_contacts(
                        line,
                        line,
                        &domain,
                        &[],
                        context.policy(),
                        math,
                        progress,
                    )?
                    .is_empty()
                    {
                        return Err(GeoError::domain("self contact on an oriented edge"));
                    }
                }
            }
            return Ok(());
        }
    })();
    math.release_workspace(reserved)
        .map_err(|e| geo_math_error(&e, policy))?;
    result
}
fn monotone(
    line: &ChartLine,
    parameter: &FixedInterval,
    iterations: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, MathError> {
    Ok(line
        .normal(parameter, iterations, math, progress)?
        .1
        .is_some_and(|d| {
            d.iter()
                .any(|d| d.lower() > &BigInt::zero() || d.upper().is_negative())
        }))
}
fn split_simple(
    index: usize,
    panels: &mut Vec<SimplePanel>,
    depth: u32,
    math: &mut CoordinateMath,
    reserved: &mut usize,
) -> Result<(), MathError> {
    let panel = &panels[index];
    if panel.depth >= depth {
        return Err(MathError::PrecisionExhausted);
    }
    let bytes = panel
        .parameter
        .workspace_bytes()
        .saturating_add(size_of::<SimplePanel>())
        .saturating_mul(4);
    math.reserve_workspace(bytes)?;
    *reserved = reserved
        .checked_add(bytes)
        .ok_or(MathError::WorkspaceExhausted)?;
    let [lo, hi] = split_parameter(&panel.parameter, math)?;
    let depth = panel.depth + 1;
    panels[index] = SimplePanel {
        parameter: lo,
        depth,
    };
    panels.insert(
        index + 1,
        SimplePanel {
            parameter: hi,
            depth,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Coord, GeographicReference, PreparedCoordinate, ShortestGeodesicArc, SourceLinearEdge,
    };

    #[derive(Default)]
    struct RingReceipt {
        work: u64,
        peak: u64,
    }
    impl MetricWorkObserver for RingReceipt {
        fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
            self.work += work;
            self.peak += workspace;
            Ok(())
        }
    }

    #[test]
    fn cold_wide_oriented_entries_refuse_before_identity_rendering_and_report_peak() {
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 1_000_000_000,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut budget = crate::PreparationBudget::new(policy);
        let reference = GeographicReference::new(
            crate::PreparedEllipsoid::new_in_budget(
                Rat::from_int(crate::Int::one().shl(4096).add(&crate::Int::one())),
                Rat::from_i64(300),
                &mut budget,
            )
            .unwrap(),
            purrdf_hash::hex::Digest32::new([82; 32]),
            crate::AxisOrder::LonLat,
        );
        let vertices = [(-1, -1), (1, -1), (1, 1), (-1, 1), (-1, -1)]
            .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let curve = PreparedCurve::from_source(&vertices, &reference).unwrap();
        let mut admitted = MetricContext::new(reference.clone(), policy).unwrap();
        let ring = OrientedRing::new(curve, &mut admitted).unwrap();
        let region = crate::PreparedRegion::polygons(vec![
            crate::PreparedPolygon::from_oriented_rings(
                vec![ring.clone()],
                crate::OrientedInterior::Left,
                &reference,
            )
            .unwrap(),
        ]);
        let query = point(0, 0);
        for limits in [
            crate::ExecutionLimits {
                max_work_items: 1,
                ..crate::ExecutionLimits::GEOMETRY
            },
            crate::ExecutionLimits {
                max_workspace_bytes: 1,
                ..crate::ExecutionLimits::GEOMETRY
            },
        ] {
            for entry in 0..5 {
                let cold = GeographicReference::new(
                    reference.ellipsoid().clone(),
                    reference.datum(),
                    reference.axes(),
                );
                let mut context =
                    MetricContext::new(cold, crate::ExecutionPolicy::new(limits).unwrap()).unwrap();
                let owned = ring.curve().clone();
                let mut receipt = RingReceipt::default();
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let result = match entry {
                    0 => OrientedRing::new_metered(owned, &mut context, &mut receipt).map(|_| ()),
                    1 => ring
                        .locate_left_metered(&query, &mut context, &mut receipt)
                        .map(|_| ()),
                    2 => crate::atlas::point::contact_metered(
                        &ring.curve().edges()[0],
                        &query,
                        &mut context,
                        &mut receipt,
                    )
                    .map(|_| ()),
                    3 => arcs::intersections_metered(
                        &ring.curve().edges()[0],
                        &ring.curve().edges()[1],
                        &mut context,
                        &mut receipt,
                    )
                    .map(|_| ()),
                    _ => arcs::arrangement::native_boundary_metered(
                        &region,
                        &mut context,
                        &mut receipt,
                    )
                    .map(|_| ()),
                };
                let allocations = window.close();
                if limits.max_work_items == 1 {
                    assert!(matches!(result, Err(GeoError::WorkExhausted { .. })));
                } else {
                    assert!(matches!(result, Err(GeoError::MemoryExhausted { .. })));
                }
                assert_eq!(allocations.allocations, 0);
                assert_eq!(allocations.requested_bytes, 0);
                // The entry cancellation latch observes one work item even
                // when resource preflight refuses before native work starts.
                assert_eq!(receipt.work, context.work_items().max(1));
                assert_eq!(receipt.peak, context.workspace_peak());
                assert_eq!(context.current_workspace_bytes(), 0);
            }
        }
    }

    fn point(longitude: i64, latitude: i64) -> LonLat {
        LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).expect("geographic point")
    }
    fn coordinate(longitude: i64, latitude: i64) -> PreparedCoordinate {
        PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
            &GeographicReference::wgs84(),
        )
        .expect("source")
    }
    fn linear_ring(vertices: &[(i64, i64)]) -> PreparedCurve {
        PreparedCurve::new(
            vertices
                .windows(2)
                .map(|p| {
                    PreparedEdge::SourceLinear(Box::new(
                        SourceLinearEdge::new(
                            coordinate(p[0].0, p[0].1),
                            coordinate(p[1].0, p[1].1),
                        )
                        .expect("edge"),
                    ))
                })
                .collect(),
        )
    }
    #[test]
    fn oriented_left_keeps_large_complement_and_pole_seam_identity() {
        let mut context = MetricContext::wgs84().expect("context");
        let vertices = [(-40, -30), (40, -30), (40, 30), (-40, 30), (-40, -30)];
        let ring = OrientedRing::new(linear_ring(&vertices), &mut context).expect("Jordan ring");
        assert_eq!(
            ring.locate_left(&point(0, 0), &mut context)
                .expect("interior"),
            Set::Interior
        );
        assert_eq!(
            ring.locate_left(&point(0, 90), &mut context)
                .expect("outside pole"),
            Set::Exterior
        );
        assert_eq!(
            ring.locate_left(&point(40, 0), &mut context)
                .expect("exact boundary"),
            Set::Boundary
        );
        let reversed = OrientedRing::new(
            linear_ring(&vertices.into_iter().rev().collect::<Vec<_>>()),
            &mut context,
        )
        .expect("reverse ring");
        assert_eq!(
            reversed
                .locate_left(&point(0, 0), &mut context)
                .expect("large Left exterior"),
            Set::Exterior
        );
        assert_eq!(
            reversed
                .locate_left(&point(-73, 90), &mut context)
                .expect("large Left interior"),
            Set::Interior
        );
        let parallel = OrientedRing::new(linear_ring(&[(-180, 30), (180, 30)]), &mut context)
            .expect("one simple closed parallel");
        assert_eq!(
            parallel
                .locate_left(&point(90, 90), &mut context)
                .expect("north cap"),
            Set::Interior
        );
        assert_eq!(
            parallel
                .locate_left(&point(-90, -90), &mut context)
                .expect("south outside"),
            Set::Exterior
        );
        assert_eq!(
            parallel
                .locate_left(&point(-180, 30), &mut context)
                .expect("seam boundary"),
            Set::Boundary
        );
    }
    #[test]
    fn true_endpoint_geodesic_ring_classifies_without_carrier_chords() {
        let mut limits = *crate::ExecutionPolicy::geometry().limits();
        limits.max_work_items = 2_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).expect("raised admitted work"),
        )
        .expect("context");
        let vertices = [(-50, -25), (50, -25), (0, 65), (-50, -25)];
        let mut edges = Vec::new();
        for pair in vertices.windows(2) {
            edges.push(PreparedEdge::ShortestGeodesic(Box::new(
                ShortestGeodesicArc::new(
                    coordinate(pair[0].0, pair[0].1),
                    coordinate(pair[1].0, pair[1].1),
                    &mut context,
                )
                .expect("unique arc"),
            )));
        }
        let ring =
            OrientedRing::new(PreparedCurve::new(edges), &mut context).expect("unique Jordan ring");
        assert_eq!(
            ring.locate_left(&point(0, 0), &mut context)
                .expect("inside true arcs"),
            Set::Interior
        );
        assert_eq!(
            ring.locate_left(&point(0, -80), &mut context)
                .expect("outside true arcs"),
            Set::Exterior
        );
        assert_eq!(
            ring.locate_left(&point(0, 65), &mut context)
                .expect("exact branch endpoint"),
            Set::Boundary
        );
    }
    #[test]
    fn source_pole_junctions_keep_oriented_wide_wedges_and_seam_aliases() {
        let mut context = MetricContext::wgs84().expect("context");
        for (west, east, pole, middle) in [
            (0, 60, -90, 30),
            (170, 180, -90, 175),
            (-170, 170, -90, 0),
            (0, 60, 90, 30),
        ] {
            let vertices = if pole < 0 {
                [(east, pole), (east, 0), (west, 0), (west, pole)]
            } else {
                [(west, pole), (west, 0), (east, 0), (east, pole)]
            };
            let latitude = pole / 2;
            let ring = OrientedRing::new(linear_ring(&vertices), &mut context)
                .expect("pole-closed Jordan");
            assert_eq!(
                ring.locate_left(&point(middle, latitude), &mut context)
                    .unwrap(),
                Set::Interior
            );
            assert_eq!(
                ring.locate_left(&point(middle, -latitude), &mut context)
                    .unwrap(),
                Set::Exterior
            );
            for longitude in [-180, -73, 0, 119, 180] {
                assert_eq!(
                    ring.locate_left(&point(longitude, pole), &mut context)
                        .unwrap(),
                    Set::Boundary
                );
                assert_eq!(
                    ring.locate_left(&point(longitude, -pole), &mut context)
                        .unwrap(),
                    Set::Exterior
                );
            }
            let reversed = OrientedRing::new(
                linear_ring(&vertices.into_iter().rev().collect::<Vec<_>>()),
                &mut context,
            )
            .unwrap();
            assert_eq!(
                reversed
                    .locate_left(&point(middle, latitude), &mut context)
                    .unwrap(),
                Set::Exterior
            );
            assert_eq!(
                reversed
                    .locate_left(&point(middle, -pole), &mut context)
                    .unwrap(),
                Set::Interior
            );
            if east == 180 {
                for longitude in [-180, 180] {
                    assert_eq!(
                        ring.locate_left(&point(longitude, latitude), &mut context)
                            .unwrap(),
                        Set::Boundary
                    );
                }
            }
            if west == -170 {
                for longitude in [-180, 180] {
                    assert_eq!(
                        ring.locate_left(&point(longitude, latitude), &mut context)
                            .unwrap(),
                        Set::Exterior
                    );
                }
            }
        }
        let meridians = [
            linear_ring(&[(50, -90), (50, 90)]),
            linear_ring(&[(-50, 90), (-50, -90)]),
        ];
        let lune = OrientedRing::new(
            PreparedCurve::new(
                meridians
                    .iter()
                    .flat_map(|curve| curve.edges().iter().cloned())
                    .collect(),
            ),
            &mut context,
        )
        .unwrap();
        assert_eq!(
            lune.locate_left(&point(0, 0), &mut context).unwrap(),
            Set::Interior
        );
        assert_eq!(
            lune.locate_left(&point(120, 0), &mut context).unwrap(),
            Set::Exterior
        );
        for pole in [-90, 90] {
            assert_eq!(
                lune.locate_left(&point(177, pole), &mut context).unwrap(),
                Set::Boundary
            );
        }
    }

    #[test]
    fn non_jordan_and_incomplete_rings_refuse() {
        let mut context = MetricContext::wgs84().expect("context");
        assert!(
            OrientedRing::new(
                linear_ring(&[(-20, -20), (20, 20), (-20, 20), (20, -20), (-20, -20)]),
                &mut context
            )
            .is_err()
        );
        assert!(OrientedRing::new(linear_ring(&[(0, 0), (10, 0)]), &mut context).is_err());
        assert!(OrientedRing::new(linear_ring(&[(0, 90), (50, 90)]), &mut context).is_err());
    }
}
