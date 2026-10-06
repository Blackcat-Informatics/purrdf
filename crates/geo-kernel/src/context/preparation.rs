// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One admission for sequential ingestion, preparation and numerical phases.

use super::MetricContext;
use crate::{ExecutionPolicy, GeoError};

/// An immutable prepared source's original identity and complete admission.
/// Only a successfully prepared geometry or geographic index constructs this receipt. It carries
/// resource evidence separately from the mathematical source content identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedSourceReceipt {
    source: purrdf_hash::hex::Digest32,
    work: u64,
    workspace: u64,
}

impl PreparedSourceReceipt {
    pub(crate) const fn new(source: purrdf_hash::hex::Digest32, work: u64, workspace: u64) -> Self {
        Self {
            source,
            work,
            workspace,
        }
    }

    /// The exact original prepared-source identity.
    #[must_use]
    pub const fn source_id(self) -> purrdf_hash::hex::Digest32 {
        self.source
    }

    /// Complete actual source preparation work, excluding subsequent metrics.
    #[must_use]
    pub const fn work_items(self) -> u64 {
        self.work
    }

    /// Complete retained prepared-source storage allowance.
    #[must_use]
    pub const fn workspace_bytes(self) -> u64 {
        self.workspace
    }

    /// Check that a following context already admits the complete source.
    ///
    /// # Errors
    /// Refuses cancellation and a missing persistent preparation/storage receipt.
    /// The consumer also verifies the source identity and exact storage amount
    /// against the immutable source it evaluates.
    pub fn validate_context(self, context: &MetricContext) -> Result<(), GeoError> {
        context.checkpoint()?;
        if context.preparation_work_items() < self.work
            || context.retained_workspace_bytes() < self.workspace
        {
            return Err(GeoError::config(
                "prepared source is absent from the context admission baseline",
            ));
        }
        Ok(())
    }
}

/// Cumulative actual preparation receipts under one original invocation policy.
/// It does not charge an observer again; each metered phase charges its own work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationBudget {
    policy: ExecutionPolicy,
    work: u64,
    workspace: u64,
    peak: u64,
}
impl PreparationBudget {
    /// Begin an invocation before any source/prepared allocation.
    #[must_use]
    pub const fn new(policy: ExecutionPolicy) -> Self {
        Self {
            policy,
            work: 0,
            workspace: 0,
            peak: 0,
        }
    }
    /// Retain an actual completed phase receipt, checking the complete sum.
    ///
    /// # Errors
    /// Refuses work/storage overflow and exhaustion without changing the receipt.
    pub fn retain(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
        let total_work = self
            .work
            .checked_add(work)
            .ok_or_else(|| GeoError::WorkExhausted {
                limit: self.policy.limits().max_work_items,
            })?;
        let total_workspace =
            self.workspace
                .checked_add(workspace)
                .ok_or_else(|| GeoError::MemoryExhausted {
                    limit: self.policy.limits().max_workspace_bytes,
                })?;
        if total_work > self.policy.limits().max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: self.policy.limits().max_work_items,
            });
        }
        if total_workspace > self.policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            });
        }
        self.work = total_work;
        self.workspace = total_workspace;
        self.peak = self.peak.max(total_workspace);
        Ok(())
    }
    /// Retain a completed phase's work, live output and complete observed peak.
    /// The peak includes this invocation's earlier retained sources; it is not
    /// an additional allocation and does not change remaining live storage.
    /// # Errors
    /// Refuses the complete policy limits without changing any receipt.
    pub fn retain_phase(
        &mut self,
        work: u64,
        workspace: u64,
        complete_peak: u64,
    ) -> Result<(), GeoError> {
        if complete_peak > self.policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            });
        }
        self.retain(work, workspace)?;
        self.peak = self.peak.max(complete_peak);
        Ok(())
    }

    /// Admit the next phase against the same cumulative original policy.
    ///
    /// # Errors
    /// Refuses a policy whose remaining resource counters cannot be admitted.
    pub fn remaining(self) -> Result<ExecutionPolicy, GeoError> {
        self.policy.remaining_after(self.work, self.workspace)
    }
    /// Original admitted policy shared by every invocation phase.
    #[must_use]
    pub const fn policy(self) -> ExecutionPolicy {
        self.policy
    }
    /// Actual already performed work across all retained phases.
    #[must_use]
    pub const fn work_items(self) -> u64 {
        self.work
    }
    /// Complete retained source/preparation allowance across these phases.
    #[must_use]
    pub const fn workspace_bytes(self) -> u64 {
        self.workspace
    }
    /// Greatest admitted simultaneous source, retained preparation and scratch.
    /// Completed temporary phases preserve this observer receipt without
    /// consuming retained storage in the following phase.
    #[must_use]
    pub const fn workspace_peak(self) -> u64 {
        self.peak
    }
    /// Admit a bounded exact body before it validates, clones, renders or hashes
    /// original configuration. Scratch is temporary; performed work remains in
    /// the cumulative receipt even if the body refuses its parameters.
    ///
    /// # Errors
    /// Refuses complete work/scratch exhaustion before calling the body.
    pub fn exact<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.exact_observed(cost, evaluate, None)
    }

    /// Admit the same bounded pure body before an external governor or
    /// cancellation callback. Integer configuration does not depend on FENV.
    /// # Errors
    /// Adds observer refusal to [`Self::exact`] before evaluating the body.
    pub fn exact_metered<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
        observer: &mut dyn super::MetricWorkObserver,
    ) -> Result<T, GeoError> {
        self.exact_observed(cost, evaluate, Some(observer))
    }

    fn exact_observed<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<T, GeoError> {
        let mut admitted = *self;
        admitted.retain(cost.work_items, cost.workspace_bytes)?;
        let mut continuation = observer
            .map(|observer| super::MetricWorkContinuation::new(observer, self.work, self.peak));
        let mut progress = super::WorkProgress::integer(
            continuation
                .as_mut()
                .map(|value| value as &mut dyn super::MetricWorkObserver),
        );
        progress.charge_counts(admitted.work, admitted.peak)?;
        self.retain(cost.work_items, 0)?;
        self.peak = admitted.peak;
        let result = evaluate();
        progress.charge_counts(self.work, self.peak)?;
        result
    }

    /// Admit the original rational bodies through the shared limb-cost home.
    ///
    /// # Errors
    /// Refuses cost overflow and cumulative limits before evaluating operands.
    pub fn exact_rationals<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&crate::Rat],
        count: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.exact(
            crate::numerical::rational_cost(operation, operands, count)
                .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?,
            evaluate,
        )
    }

    /// Publish immutable original limbs once under configuration admission.
    /// Heap limbs move into admitted ownership headers; reusable limbs detach
    /// once. Later clones share those canonical operands without allocating.
    pub(crate) fn share_rational(&mut self, value: crate::Rat) -> Result<crate::Rat, GeoError> {
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[&value],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "immutable rational sharing work",
        ))?;
        let bytes = u64::try_from(value.shared_owned_bytes())
            .map_err(|_| GeoError::ArithmeticOverflow("immutable rational sharing storage"))?;
        self.retain(0, bytes)?;
        self.exact(cost, || Ok(value.into_shared()))
    }

    /// Evaluate a preparation phase while original source and intermediate
    /// storage remain live. Its exact work survives success or refusal; the
    /// caller's persistent storage receipt excludes these scoped allocations.
    ///
    /// # Errors
    /// Refuses the complete source allowance before entering the phase.
    pub fn with_workspace<T>(
        &mut self,
        bytes: u64,
        evaluate: impl FnOnce(&mut Self) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let mut phase = *self;
        phase.retain(0, bytes)?;
        let result = evaluate(&mut phase);
        self.retain(phase.work - self.work, 0)?;
        self.peak = self.peak.max(phase.peak);
        result
    }

    /// Construct an independent worker from a borrowed reference after admitting
    /// its parameter copy alongside every already retained invocation source.
    ///
    /// # Errors
    /// Refuses cumulative work/storage, cancellation and the numerical environment
    /// before allocating original reference limbs. The completed constructor
    /// receipt remains part of the same invocation's preparation baseline.
    pub fn worker_context(
        &mut self,
        reference: &crate::GeographicReference,
    ) -> Result<MetricContext, GeoError> {
        self.worker_context_observed(reference, None)
    }

    /// Construct the same worker after charging original reference-copy work
    /// and its simultaneous source/scratch storage to the external governor.
    /// # Errors
    /// Adds observer refusal to [`Self::worker_context`], before any copy.
    pub fn worker_context_metered(
        &mut self,
        reference: &crate::GeographicReference,
        observer: &mut dyn super::MetricWorkObserver,
    ) -> Result<MetricContext, GeoError> {
        self.worker_context_observed(reference, Some(observer))
    }

    fn worker_context_observed(
        &mut self,
        reference: &crate::GeographicReference,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<MetricContext, GeoError> {
        let cost = crate::numerical::reference_copy_cost(reference)?;
        let retained = reference.ellipsoid().retained_limb_bytes();
        let peak = retained
            .checked_add(cost.workspace_bytes)
            .ok_or(GeoError::ArithmeticOverflow("worker reference preparation"))?;
        let mut admitted = *self;
        admitted.retain(cost.work_items, peak)?;
        let mut context = if let Some(observer) = observer {
            let mut continuation =
                super::MetricWorkContinuation::new(observer, self.work, self.peak);
            let mut progress = super::WorkProgress::new(Some(&mut continuation));
            let mut nested = progress.nested(self.work, self.workspace, self.peak);
            MetricContext::from_reference_metered(reference, self.policy, &mut nested)?
        } else {
            MetricContext::from_reference(reference, self.policy)?
        };
        self.retain(
            context.preparation_work_items(),
            context.retained_workspace_bytes(),
        )?;
        self.peak = admitted.peak;
        self.install(&mut context)?;
        // Constructor scratch coexisted with every earlier retained source.
        // Preserve that complete peak even though its copy already finished.
        context.admit_workspace(cost.workspace_bytes)?;
        context.release_workspace(cost.workspace_bytes)?;
        Ok(context)
    }

    /// Install the actual cumulative receipt before numerical entry.
    ///
    /// # Errors
    /// Refuses a different original policy or changing an active context phase.
    pub fn install(self, context: &mut MetricContext) -> Result<(), GeoError> {
        if context.policy() != self.policy {
            return Err(GeoError::config(
                "preparation receipt belongs to a different execution policy",
            ));
        }
        context.set_preparation_work(self.work)?;
        context.set_retained_workspace(self.workspace)?;
        // Earlier scratch has been released, but its already observed peak
        // must remain visible to a fresh numerical-phase continuation.
        let historical = self.peak - self.workspace;
        context.admit_workspace(historical)?;
        context.release_workspace(historical)
    }

    /// Retain a metric offset while counting its already prepared source once.
    /// Original constructor work and the new radius/reference storage remain
    /// in this invocation's unchanged policy and external governor.
    /// # Errors
    /// Refuses missing source receipts, constructor limits or observer failure.
    pub fn prepare_offset(
        &mut self,
        geometry: crate::PreparedGeometry,
        radius: &crate::Metres,
        observer: &mut dyn super::MetricWorkObserver,
    ) -> Result<crate::OffsetRegion, GeoError> {
        if self.work < geometry.preparation_work_items() {
            return Err(GeoError::config(
                "offset source preparation work receipt is missing",
            ));
        }
        let source_bytes = geometry.retained_workspace_bytes();
        let other_bytes = self
            .workspace
            .checked_sub(source_bytes)
            .ok_or_else(|| GeoError::config("offset source preparation receipt is missing"))?;
        let policy = self.policy.remaining_after(self.work, other_bytes)?;
        let mut continuation = super::MetricWorkContinuation::new(observer, 0, source_bytes);
        let offset = crate::OffsetRegion::new_metered(
            std::sync::Arc::new(geometry),
            radius,
            policy,
            &mut continuation,
        )?;
        let additional = offset
            .retained_workspace_bytes()
            .checked_sub(source_bytes)
            .ok_or_else(|| GeoError::config("offset constructor source receipt is inconsistent"))?;
        self.retain(offset.preparation_work_items(), additional)?;
        Ok(offset)
    }
}

#[cfg(test)]
mod tests {
    use crate::{GeoProfile, MetricContext, PreparedGeometry};

    #[test]
    fn immutable_limb_sharing_refuses_before_ownership_allocation() {
        for memory_failure in [false, true] {
            let value = crate::Rat::from_int(crate::Int::one().shl(256));
            let limits = crate::ExecutionLimits {
                max_work_items: if memory_failure { 262_144 } else { 1 },
                max_workspace_bytes: if memory_failure {
                    value.shared_owned_bytes() as u64 - 1
                } else {
                    65_536
                },
                ..crate::ExecutionLimits::GEOMETRY
            };
            let mut budget =
                super::PreparationBudget::new(crate::ExecutionPolicy::new(limits).unwrap());
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refusal = budget.share_rational(value);
            let allocation = window.close();
            if memory_failure {
                assert!(matches!(
                    refusal,
                    Err(crate::GeoError::MemoryExhausted { .. })
                ));
            } else {
                assert!(matches!(
                    refusal,
                    Err(crate::GeoError::WorkExhausted { .. })
                ));
            }
            assert_eq!(allocation.allocations, 0);
            assert_eq!(allocation.requested_bytes, 0);
        }
    }

    #[test]
    fn borrowed_worker_admits_complete_prior_sources_before_reference_copy() {
        let reference = crate::GeographicReference::wgs84();
        let policy = crate::ExecutionPolicy::geometry();
        let mut budget = super::PreparationBudget::new(policy);
        budget
            .retain(5, policy.limits().max_workspace_bytes)
            .unwrap();
        let original = budget;
        // Warm the platform environment probe outside the measured refusal.
        MetricContext::wgs84().unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refusal = budget.worker_context(&reference);
        let allocation = window.close();
        assert!(matches!(
            refusal,
            Err(crate::GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(budget, original);
        assert_eq!(allocation.allocations, 0);
        assert_eq!(allocation.requested_bytes, 0);

        let cost = crate::numerical::reference_copy_cost(&reference).unwrap();
        let mut budget = super::PreparationBudget::new(policy);
        budget.retain(5, 10_000).unwrap();
        let context = budget.worker_context(&reference).unwrap();
        assert_eq!(context.preparation_work_items(), 5 + cost.work_items);
        assert_eq!(context.retained_workspace_bytes(), budget.workspace_bytes());
        assert!(context.workspace_peak() >= budget.workspace_bytes() + cost.workspace_bytes);
    }

    #[test]
    fn source_receipt_requires_persistent_work_and_storage_not_temporary_counts() {
        let profile = GeoProfile::standard();
        let literal =
            crate::wkt::parse("POINT(0 0)", crate::standard_vocabulary().default_wkt_crs())
                .unwrap();
        let source = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let receipt = source.source_receipt();
        assert_eq!(receipt.source_id(), source.id());
        assert_eq!(receipt.work_items(), source.preparation_work_items());
        assert_eq!(receipt.workspace_bytes(), source.retained_workspace_bytes());
        let mut context = MetricContext::new(source.reference().clone(), profile.policy()).unwrap();
        assert!(receipt.validate_context(&context).is_err());
        context.charge_work(receipt.work_items()).unwrap();
        context.admit_workspace(receipt.workspace_bytes()).unwrap();
        assert!(receipt.validate_context(&context).is_err());
        context
            .release_workspace(receipt.workspace_bytes())
            .unwrap();
        context.set_preparation_work(receipt.work_items()).unwrap();
        context
            .set_retained_workspace(receipt.workspace_bytes())
            .unwrap();
        receipt.validate_context(&context).unwrap();
    }
}
