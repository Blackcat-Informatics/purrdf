// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical metric offsets, retaining the complete prepared source set.

mod materialize;
pub use materialize::BufferMaterialization;

use crate::context::WorkProgress;
use crate::{
    Coord, GeoError, LonLat, Metres, MetricContext, MetricWorkObserver, PreparedCoordinate,
    PreparedEdge, PreparedGeodesic, PreparedGeometry, PreparedRegion, Rat, Set,
};
use purrdf_hash::{Domain, hex::Digest32};
use std::borrow::Cow;
use std::sync::Arc;

const OFFSET: Domain = Domain::new(b"purrdf-geo-kernel/physical-offset/v1");

/// Closed physical metric offset of an immutable complete source set. A
/// negative radius is empty; zero is the source closure. Membership succeeds
/// only when the shared exact predicates or distance enclosures prove it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsetRegion {
    source: Arc<PreparedGeometry>,
    radius: Metres,
    identity: Digest32,
    preparation_work: u64,
    retained_workspace: u64,
}

impl OffsetRegion {
    /// Retain the original source and exact physical radius without materializing
    /// polygon vertices or changing its curve laws.
    ///
    /// # Errors
    /// Refuses incomplete work/workspace admission before parameter rendering.
    pub fn new(
        source: Arc<PreparedGeometry>,
        radius: Metres,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare(source, Cow::Owned(radius), policy, None)
    }

    /// Borrow and admit the exact radius before copying or identity rendering.
    ///
    /// # Errors
    /// Preserves all constructor refusals and the observer's original failure.
    pub fn new_metered(
        source: Arc<PreparedGeometry>,
        radius: &Metres,
        policy: crate::ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(source, Cow::Borrowed(radius), policy, Some(observer))
    }

    fn prepare(
        source: Arc<PreparedGeometry>,
        radius: Cow<'_, Metres>,
        policy: crate::ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let bytes = source
            .retained_workspace_bytes()
            .saturating_add(source.reference().ellipsoid().retained_limb_bytes())
            .saturating_add(radius.exact().allocated_bytes() as u64)
            .saturating_add(size_of::<Self>() as u64);
        if bytes > policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            });
        }
        let mut context = MetricContext::new(source.reference().clone(), policy)?;
        context.begin(1)?;
        context.admit_workspace(bytes)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(&context)?;
        let radius = match radius {
            Cow::Owned(radius) => radius,
            Cow::Borrowed(radius) => {
                crate::numerical::ExactAdmission::new(&mut context, &mut progress).rational(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[radius.exact()],
                    1,
                    || Ok(radius.clone()),
                )?
            }
        };
        let (numerator, denominator) =
            crate::numerical::ExactAdmission::new(&mut context, &mut progress).rational(
                purrdf_xsd::integer::ExactOperation::DecimalRender,
                &[radius.exact()],
                2,
                || {
                    Ok((
                        radius.exact().numerator().to_string(),
                        radius.exact().denominator().to_string(),
                    ))
                },
            )?;
        context.charge_work(1)?;
        progress.context_poll(&context)?;
        let identity = crate::profile::hash_fields(
            OFFSET,
            [
                source.id().as_bytes().as_slice(),
                numerator.as_bytes(),
                denominator.as_bytes(),
            ],
        );
        Ok(Self {
            source,
            radius,
            identity,
            preparation_work: context.work_items(),
            retained_workspace: bytes,
        })
    }
    /// Original complete geometry and its source/binding identity.
    #[must_use]
    pub fn source(&self) -> &PreparedGeometry {
        &self.source
    }
    /// Exact selected physical radius in metres.
    #[must_use]
    pub const fn radius(&self) -> &Metres {
        &self.radius
    }
    /// Content identity independent of resource admission and tighter proofs.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.identity
    }

    /// Complete constructor work, separate from the source/radius identity.
    #[must_use]
    pub const fn preparation_work_items(&self) -> u64 {
        self.preparation_work
    }

    /// Retained complete source and exact radius storage allowance.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_workspace
    }

    /// Prove membership in the physical offset using the shared source atlas,
    /// correctly rounded point geodesy and certified global geometry distance.
    ///
    /// # Errors
    /// Refuses reference mismatch, unresolved boundary comparisons and incomplete
    /// numerical/resource admission. No approximate boolean is returned.
    pub fn contains(&self, point: &LonLat, context: &mut MetricContext) -> Result<bool, GeoError> {
        self.contains_inner(point, None, context, None)
    }
    /// Prove membership with bounded work/cancellation callbacks.
    ///
    /// # Errors
    /// Also propagates the observer's original refusal without publishing a result.
    pub fn contains_metered(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<bool, GeoError> {
        self.contains_inner(point, None, context, Some(observer))
    }
    /// Prove offset membership when the exact source and offset constructor
    /// work/storage are already retained in the original invocation baseline.
    ///
    /// # Errors
    /// Refuses a different or missing retained source and every membership error.
    pub fn contains_prepared(
        &self,
        point: &LonLat,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
    ) -> Result<bool, GeoError> {
        self.contains_inner(point, Some(receipt), context, None)
    }

    /// Continue retained-source offset membership with bounded governor polls.
    ///
    /// # Errors
    /// Also preserves the observer's original operational refusal.
    pub fn contains_prepared_metered(
        &self,
        point: &LonLat,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<bool, GeoError> {
        self.contains_inner(point, Some(receipt), context, Some(observer))
    }

    fn contains_inner(
        &self,
        point: &LonLat,
        receipt: Option<&crate::context::PreparedSourceReceipt>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<bool, GeoError> {
        if self.source.reference() != context.reference() {
            context.begin(0)?;
            return Err(crate::numerical::missing_operation(
                self.source.reference(),
                context,
                &mut WorkProgress::new(observer),
            )?);
        }
        if let Some(receipt) = receipt {
            if receipt != &self.source.source_receipt() {
                return Err(GeoError::config(
                    "offset receipt names a different original source",
                ));
            }
            receipt.validate_context(context)?;
            let work = self
                .source
                .preparation_work_items()
                .checked_add(self.preparation_work_items())
                .ok_or(GeoError::ArithmeticOverflow(
                    "retained offset preparation work",
                ))?;
            if context.preparation_work_items() < work
                || context.retained_workspace_bytes() < self.retained_workspace_bytes()
            {
                return Err(GeoError::config(
                    "offset constructor is absent from the retained source baseline",
                ));
            }
        }
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(context)?;
        let retained = if receipt.is_some() {
            0
        } else {
            self.retained_workspace_bytes()
        };
        context.admit_workspace(retained)?;
        let result = self.contains_with_progress(point, context, &mut progress);
        context.release_workspace(retained)?;
        result
    }

    fn contains_with_progress(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        self.contains_at_radius_with_progress(point, &self.radius, context, progress)
    }

    // The complete original source law under a different exact physical radius.
    // This is a borrowed numerical phase: the caller already validates the source
    // reference and admits its storage, and cumulative work is never reset.
    pub(crate) fn contains_at_radius_with_progress(
        &self,
        point: &LonLat,
        radius: &Metres,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        if radius.exact() < &Rat::zero() {
            return Ok(false);
        }
        if crate::atlas::locate_with_progress(point, self.source.region(), context, progress)?
            != Set::Exterior
        {
            return Ok(true);
        }
        for source in self.source.points() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if crate::numerical::same_location_admitted(source.point(), point, context, progress)? {
                return Ok(true);
            }
        }
        let mut has_implicit_arcs = false;
        for curve in self.source.curves() {
            for edge in curve.edges() {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                if let Some(start) = edge.start()
                    && crate::numerical::same_location_admitted(
                        start.point(),
                        point,
                        context,
                        progress,
                    )?
                {
                    return Ok(true);
                }
                match edge {
                    PreparedEdge::SourceLinear(edge) => {
                        let a = edge.start().point();
                        let b = edge.end().point();
                        let mut admission =
                            crate::numerical::ExactAdmission::new(context, progress);
                        let from = admission.rational(
                            purrdf_xsd::integer::ExactOperation::Linear,
                            &[a.longitude(), a.latitude()],
                            2,
                            || Ok(Coord::xy(a.longitude().clone(), a.latitude().clone())),
                        )?;
                        let to = admission.rational(
                            purrdf_xsd::integer::ExactOperation::Linear,
                            &[b.longitude(), b.latitude()],
                            2,
                            || Ok(Coord::xy(b.longitude().clone(), b.latitude().clone())),
                        )?;
                        for shift in [-360, 0, 360] {
                            let shift = Rat::from_i64(shift);
                            let longitude = admission.rational(
                                purrdf_xsd::integer::ExactOperation::RationalAdd,
                                &[point.longitude(), &shift],
                                1,
                                || Ok(point.longitude().add(&shift)),
                            )?;
                            let candidate = admission.rational(
                                purrdf_xsd::integer::ExactOperation::Linear,
                                &[point.latitude()],
                                1,
                                || Ok(Coord::xy(longitude, point.latitude().clone())),
                            )?;
                            if crate::topology::on_segment_admitted(
                                &candidate,
                                &from,
                                &to,
                                &mut admission,
                            )? {
                                return Ok(true);
                            }
                        }
                        if crate::numerical::same_location_admitted(b, point, context, progress)? {
                            return Ok(true);
                        }
                    }
                    PreparedEdge::ShortestGeodesic(arc) => {
                        if crate::numerical::same_location_admitted(
                            arc.end().point(),
                            point,
                            context,
                            progress,
                        )? {
                            return Ok(true);
                        }
                        has_implicit_arcs = true;
                    }
                    PreparedEdge::AzimuthLength(_) | PreparedEdge::Transformed(_) => {
                        has_implicit_arcs = true;
                    }
                }
            }
        }
        if radius.exact().is_zero()
            && !has_implicit_arcs
            && self.source.symbolic_points().is_empty()
        {
            return Ok(false);
        }
        if matches!(self.source.region(), PreparedRegion::Empty)
            && self.source.curves().is_empty()
            && self.source.symbolic_points().is_empty()
        {
            let prepared =
                PreparedGeodesic::new(crate::numerical::reference_clone(context, progress)?);
            for source in self.source.points() {
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
                    prepared.within_physical_metered(
                        source.point(),
                        point,
                        radius,
                        &mut child,
                        &mut nested,
                    )
                };
                if progress.absorb_child_result(context, &child, result)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let reference = crate::numerical::reference_clone(context, progress)?;
        let source = crate::numerical::ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[point.longitude(), point.latitude()],
            2,
            || Ok(materialize::carrier(point, reference.axes())),
        )?;
        let coordinate = crate::numerical::ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[source.x(), source.y()],
            8,
            || PreparedCoordinate::new(source.clone(), &reference),
        )?;
        let remaining = context.policy().remaining_after(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
        )?;
        let singleton = {
            let mut nested = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            PreparedGeometry::from_parts_metered(
                reference,
                vec![coordinate],
                Vec::new(),
                PreparedRegion::Empty,
                remaining,
                &mut nested,
            )
        };
        let singleton = progress.absorb_nested(context, singleton)?;
        context.admit_workspace(singleton.retained_workspace_bytes())?;
        let result = (|| {
            progress.context_poll(context)?;
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
                crate::ellipsoidal::within_physical_metered(
                    &self.source,
                    &singleton,
                    radius,
                    &mut child,
                    &mut nested,
                )
            };
            progress.absorb_child_result(context, &child, result)
        })();
        let bytes = singleton.retained_workspace_bytes();
        drop(singleton);
        context.release_workspace(bytes)?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Crs, ExecutionPolicy, GeoProfile};

    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).unwrap()
    }
    fn point(x: &str, y: &str) -> LonLat {
        LonLat::new(rat(x), rat(y)).unwrap()
    }
    fn source(text: &str) -> Arc<PreparedGeometry> {
        let literal =
            crate::wkt::parse(text, &Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap()).unwrap();
        Arc::new(PreparedGeometry::from_literal(&literal, &GeoProfile::standard()).unwrap())
    }
    #[test]
    fn physical_offsets_preserve_written_closure_and_point_threshold_law() {
        let mut context = MetricContext::wgs84().unwrap();
        let written = source("LINESTRING (170 0,-170 0)");
        let zero = OffsetRegion::new(
            written.clone(),
            Metres::new(Rat::zero()),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        assert!(zero.contains(&point("0", "0"), &mut context).unwrap());
        assert!(!zero.contains(&point("179", "0"), &mut context).unwrap());
        let negative =
            OffsetRegion::new(written, Metres::new(rat("-1")), ExecutionPolicy::geometry())
                .unwrap();
        assert!(!negative.contains(&point("0", "0"), &mut context).unwrap());
        let origin = source("POINT (0 0)");
        let offset = OffsetRegion::new(
            origin,
            Metres::new(rat("1000")),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        for candidate in [point("0", "0"), point("0.001", "0"), point("1", "0")] {
            assert_eq!(
                offset.contains(&candidate, &mut context).unwrap(),
                crate::geodesic::within_physical(
                    point("0", "0"),
                    candidate,
                    Metres::new(rat("1000"))
                )
                .unwrap()
            );
        }
        let pole = OffsetRegion::new(
            source("POINT (123 90)"),
            Metres::new(Rat::zero()),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        assert!(pole.contains(&point("-75", "90"), &mut context).unwrap());
    }
    #[test]
    fn retained_source_membership_charges_one_baseline_and_rejects_other_sources() {
        let prepared = source("POINT (123 90)");
        let offset = OffsetRegion::new(
            prepared.clone(),
            Metres::new(Rat::zero()),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        let preparation_work = prepared.preparation_work_items() + offset.preparation_work_items();
        let receipt = prepared.source_receipt();
        // Measure the real admitted overlap under the ordinary policy. Cold
        // binding admission and exact comparisons are included; a guessed
        // fixed scratch margin is not an ownership proof.
        let mut measured =
            MetricContext::new(prepared.reference().clone(), ExecutionPolicy::geometry()).unwrap();
        measured.set_preparation_work(preparation_work).unwrap();
        measured
            .set_retained_workspace(offset.retained_workspace_bytes())
            .unwrap();
        assert!(
            offset
                .contains_prepared(&point("-75", "90"), &receipt, &mut measured)
                .unwrap()
        );
        let exact_peak = measured.workspace_peak();
        assert!(exact_peak >= offset.retained_workspace_bytes());
        assert!(exact_peak < 2 * offset.retained_workspace_bytes());
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        // The actual peak admits one retained source and its complete scratch,
        // while the independently checked bound excludes two source copies.
        limits.max_workspace_bytes = exact_peak;
        let mut context = MetricContext::new(
            prepared.reference().clone(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        context.set_preparation_work(preparation_work).unwrap();
        context
            .set_retained_workspace(offset.retained_workspace_bytes())
            .unwrap();
        for _ in 0..2 {
            assert!(
                offset
                    .contains_prepared(&point("-75", "90"), &receipt, &mut context)
                    .unwrap()
            );
            assert_eq!(
                context.retained_workspace_bytes(),
                offset.retained_workspace_bytes()
            );
            assert!(context.workspace_peak() < 2 * offset.retained_workspace_bytes());
            assert_eq!(context.workspace_peak(), exact_peak);
        }
        let other = source("POINT (0 0)");
        assert!(matches!(
            offset.contains_prepared(&point("-75", "90"), &other.source_receipt(), &mut context),
            Err(GeoError::Config(_))
        ));
        assert!(matches!(
            offset.contains(&point("-75", "90"), &mut context),
            Err(GeoError::MemoryExhausted { .. })
        ));
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_workspace_bytes = offset.retained_workspace_bytes() - 1;
        let mut context = MetricContext::new(
            prepared.reference().clone(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            offset.contains(&point("-75", "90"), &mut context),
            Err(GeoError::MemoryExhausted { .. })
        ));
    }

    #[test]
    fn cancellation_refuses_before_membership_and_identity_excludes_policy() {
        let source = source("POINT (0 0)");
        let a = OffsetRegion::new(
            source.clone(),
            Metres::new(rat("1")),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items *= 2;
        let b = OffsetRegion::new(
            source,
            Metres::new(rat("1.00")),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(a.id(), b.id());
        struct Cancel;
        impl MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        assert_eq!(
            a.contains_metered(&point("0", "0"), &mut context, &mut Cancel),
            Err(GeoError::Cancelled)
        );
    }

    #[test]
    fn nested_curve_membership_refusals_keep_complete_native_receipts_and_cleanup() {
        struct Receipt {
            work: u64,
            peak: u64,
            calls: usize,
            cancel_at: usize,
            growth_calls: Vec<usize>,
        }
        impl MetricWorkObserver for Receipt {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.work += work;
                self.peak += growth;
                self.calls += 1;
                if growth != 0 {
                    self.growth_calls.push(self.calls);
                }
                if self.calls == self.cancel_at {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let offset = OffsetRegion::new(
            source("LINESTRING (0 0,0.000001 0)"),
            Metres::new(Rat::one()),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        let candidate = point("0.000001", "0.000001");
        let make_receipt = |cancel_at| Receipt {
            work: 0,
            peak: 0,
            calls: 0,
            cancel_at,
            growth_calls: Vec::new(),
        };
        let mut context = MetricContext::wgs84().unwrap();
        let mut complete = make_receipt(usize::MAX);
        assert!(
            offset
                .contains_metered(&candidate, &mut context, &mut complete)
                .unwrap()
        );
        assert_eq!(context.work_items(), complete.work);
        assert_eq!(context.workspace_peak(), complete.peak);
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
        let mut stops = complete.growth_calls;
        stops.extend(1..=complete.calls.min(16));
        stops.push(complete.calls);
        stops.sort_unstable();
        stops.dedup();
        for stop in stops {
            let mut context = MetricContext::wgs84().unwrap();
            let mut receipt = make_receipt(stop);
            assert_eq!(
                offset.contains_metered(&candidate, &mut context, &mut receipt),
                Err(GeoError::Cancelled)
            );
            assert_eq!(receipt.calls, stop);
            assert_eq!(context.work_items(), receipt.work);
            assert_eq!(
                context.workspace_peak(),
                receipt.peak,
                "workspace receipt at cancellation callback {stop}"
            );
            assert_eq!(
                context.current_workspace_bytes(),
                context.retained_workspace_bytes()
            );
        }
    }
}
