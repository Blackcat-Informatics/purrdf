// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One admission for sequential ingestion, preparation and numerical phases.

use super::MetricContext;
use crate::{ExecutionPolicy, GeoError};

/// An immutable prepared source's original identity and complete admission.
/// Only a successfully prepared geometry or geographic index constructs this receipt. It carries
/// resource evidence separately from the mathematical source content identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparedSourceReceipt {
    source: purrdf_hash::hex::Digest32,
    work: u64,
    workspace: u64,
}

impl purrdf_hash::error::StructuredError for PreparedSourceReceipt {
    purrdf_hash::error_visit! {
        @record self { source, work, workspace } => "prepared-source-receipt" {
            "source" => Bytes(self.source.as_bytes()), "work" => Unsigned(u128::from(self.work)),
            "workspace" => ByteCount(u128::from(self.workspace))
        }
    }
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
        let actual_work = context.preparation_work_items();
        let actual_workspace = context.retained_workspace_bytes();
        let reason = if actual_work < self.work {
            Some(crate::PreparationBaselineReason::Work)
        } else if actual_workspace < self.workspace {
            Some(crate::PreparationBaselineReason::Workspace)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(GeoError::resident_failure(
                crate::SourcePreparationError::Baseline {
                    receipt: self,
                    reason,
                    actual_work,
                    actual_workspace,
                },
                GeoError::SourcePreparation,
            ));
        }
        Ok(())
    }
}

/// Cumulative actual preparation receipts under one original invocation policy.
/// It does not charge an observer again; each metered phase charges its own work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreparationBudget {
    policy: ExecutionPolicy,
    work: u64,
    workspace: u64,
    peak: u64,
}
impl purrdf_hash::error::StructuredError for PreparationBudget {
    purrdf_hash::error_visit! {
        @record self { policy, work, workspace, peak } => "preparation-budget" {
            "policy" => Record(&self.policy), "work" => Unsigned(u128::from(self.work)),
            "workspace" => ByteCount(u128::from(self.workspace)), "peak" => ByteCount(u128::from(self.peak))
        }
    }
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

    /// Retain one context's cumulative receipt against this original baseline.
    /// No receipt is changed when the context predates the baseline or exceeds
    /// the invocation's limits. A diagnostic uses the supplied native observer.
    ///
    /// # Errors
    /// Preserves original inconsistent counters or native admission refusal.
    pub fn retain_context_phase(
        &mut self,
        context: &MetricContext,
        workspace: u64,
        phase: crate::PreparationPhase,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let actual_work = context.work_items();
        let complete_peak = context.workspace_peak();
        let work = actual_work.checked_sub(self.work).ok_or_else(|| {
            GeoError::retained_failure(
                crate::ContextConfigError::PreparationWork {
                    phase,
                    baseline_work: self.work,
                    actual_work,
                    retained_workspace: workspace,
                    complete_peak,
                },
                GeoError::ContextConfiguration,
                observer,
            )
        })?;
        self.retain_phase(work, workspace, complete_peak)
    }

    /// Admit the next phase against the same cumulative original policy.
    ///
    /// # Errors
    /// Refuses a policy whose remaining resource counters cannot be admitted.
    pub fn remaining(self) -> Result<ExecutionPolicy, GeoError> {
        self.policy.remaining_after(self.work, self.workspace)
    }
    /// Admit a phase that borrows original buffers already in this receipt.
    /// The phase describes those same buffers in its own source admission, so
    /// they are included once rather than removed from its allowance twice.
    /// This transfers no allocation or owner and licenses no replacement growth.
    ///
    /// # Errors
    /// Refuses an original-buffer receipt larger than the retained baseline.
    pub fn remaining_with_owned_sources(
        self,
        original_buffers: u64,
    ) -> Result<ExecutionPolicy, GeoError> {
        let other_sources =
            self.workspace
                .checked_sub(original_buffers)
                .ok_or(GeoError::InvalidNativeStorage(
                    "owned source buffers are absent from preparation",
                ))?;
        self.policy.remaining_after(self.work, other_sources)
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
        self.exact_observed(cost, |_| evaluate(), None)
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
        self.exact_observed(cost, |_| evaluate(), Some(observer))
    }

    /// Validate and retain one original reference name inside the same admitted
    /// exact configuration phase. Source diagnostics use their original account;
    /// immutable text is published by the observer before its control allocation.
    ///
    /// # Errors
    /// Preserves source IRI, work, cancellation and physical refusal ordering.
    pub fn crs_metered(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        iri: &str,
        storage: Option<&crate::carrier::CarrierStorage<'_>>,
        observer: &mut dyn super::MetricWorkObserver,
    ) -> Result<crate::Crs, GeoError> {
        self.exact_observed(
            cost,
            |progress| {
                crate::Crs::validate(iri, storage)?;
                progress.retain_text(iri).map(crate::Crs::from_admitted)
            },
            Some(observer),
        )
    }

    pub(crate) fn exact_observed<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&mut super::WorkProgress<'_>) -> Result<T, GeoError>,
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
        let result = evaluate(&mut progress);
        let accounting = progress.charge_counts(self.work, self.peak);
        progress.settle_result(result, accounting)
    }

    /// Prepare the caller-profiled integer arena before a native configuration
    /// callback. All arithmetic stays in the existing XSD destination bodies;
    /// the physical owner and its complete concrete layout precede allocation.
    /// Returned borrowed-arena numbers must be detached by their actual producer.
    pub(crate) fn exact_values_observed<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
            &mut super::WorkProgress<'_>,
        ) -> Result<T, GeoError>,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<T, GeoError> {
        self.exact_values_with_bits_observed(cost, cost.output_bits, evaluate, observer)
    }

    /// Admit arithmetic destinations separately from nonnumeric encoding work.
    /// A streamed preimage's total byte count is not the width of one integer.
    pub(crate) fn exact_values_with_bits_observed<T>(
        &mut self,
        mut cost: purrdf_xsd::integer::ExactArithmeticCost,
        destination_bits: u64,
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
            &mut super::WorkProgress<'_>,
        ) -> Result<T, GeoError>,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<T, GeoError> {
        let destination = purrdf_xsd::integer::ExactArithmeticCost {
            output_bits: destination_bits,
            ..cost
        };
        let limbs = usize::try_from(destination.destination_limbs().ok_or(GeoError::Storage(
            purrdf_lex::allocation::StorageError::SizeOverflow,
        ))?)
        .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
        if limbs <= 3 {
            return self.exact_observed(
                cost,
                |progress| {
                    evaluate(
                        &crate::numerical::ExactValues::admitted(
                            &purrdf_xsd::integer::LimbScratch::inline(),
                        ),
                        progress,
                    )
                },
                observer,
            );
        }
        let buffers = super::scratch_destination_count(self.policy.limits());
        let bytes = match observer.as_ref() {
            Some(observer) => observer.integer_scratch_layout(buffers, limbs)?,
            None => purrdf_xsd::integer::LimbScratch::required_bytes(buffers, limbs)
                .map_err(GeoError::NumericalScratch)?,
        };
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(u64::try_from(bytes).map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
            })?)
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
        self.exact_observed(
            cost,
            |progress| {
                let scratch = progress.prepare_integer_scratch(buffers, limbs)?;
                let result = evaluate(&crate::numerical::ExactValues::admitted(&scratch), progress);
                drop(scratch);
                result
            },
            observer,
        )
    }

    /// Validate and publish original immutable fields through their native
    /// factories. The actual detached layouts and original lease are quoted
    /// before validation; no raw arena value or unowned limb array escapes.
    pub(crate) fn detached_fields_observed<T: super::retained::NativeDetached>(
        &mut self,
        mut cost: purrdf_xsd::integer::ExactArithmeticCost,
        source: T,
        validate: impl FnOnce(
            &T,
            &crate::numerical::ExactValues<'_>,
            &mut super::WorkProgress<'_>,
        ) -> Result<(), GeoError>,
        observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<T, GeoError> {
        let payload = source.layout_bytes().map_err(GeoError::Storage)?;
        let bytes = if payload == 0 {
            0
        } else {
            payload
                .checked_add(
                    observer
                        .as_ref()
                        .map_or_else(
                            || super::WorkProgress::integer(None).retained_owner_layout(),
                            |observer| observer.retained_owner_layout(),
                        )
                        .size(),
                )
                .ok_or(GeoError::Storage(
                    purrdf_lex::allocation::StorageError::SizeOverflow,
                ))?
        };
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(u64::try_from(bytes).map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
            })?)
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
        let produced = self.exact_values_observed(
            cost,
            |values, progress| {
                validate(&source, values, progress)?;
                super::retained::produce_persistent_numeric(source, progress)
            },
            observer,
        )?;
        self.retain(0, produced.workspace_bytes())?;
        Ok(produced.into_value())
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

    /// Publish an already owned native value after admitting its exact controls.
    /// Its fields retain their original intrinsic source/numeric owners.
    /// # Errors
    /// Refuses cumulative policy or physical allocation before publication.
    pub fn retain_value<T>(&mut self, value: T) -> Result<crate::PreparedValue<T>, GeoError> {
        let mut progress = super::WorkProgress::integer(None);
        let bytes = crate::PreparedValue::<T>::required_bytes(&progress)?;
        self.retain(1, bytes)?;
        crate::PreparedValue::produce(&mut progress, |_| Ok(value), |_, _| Ok(()))
    }

    pub(crate) fn replace_workspace(
        &mut self,
        previous: u64,
        replacement: u64,
    ) -> Result<(), GeoError> {
        let baseline = self
            .workspace
            .checked_sub(previous)
            .ok_or(GeoError::ArithmeticOverflow(
                "native preparation buffer ownership",
            ))?;
        let total = baseline
            .checked_add(replacement)
            .ok_or(GeoError::ArithmeticOverflow(
                "native preparation replacement receipt",
            ))?;
        if total > self.policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            });
        }
        self.workspace = total;
        self.peak = self.peak.max(total);
        Ok(())
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
        mut observer: Option<&mut dyn super::MetricWorkObserver>,
    ) -> Result<MetricContext, GeoError> {
        let cost = crate::numerical::reference_copy_cost(reference)?;
        let peak = match observer.as_mut() {
            Some(observer) => {
                let progress = super::WorkProgress::new(Some(&mut **observer));
                super::persistent_numeric_bytes(reference, &progress)?
            }
            None => super::persistent_numeric_bytes(reference, &super::WorkProgress::new(None))?,
        };
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
        Ok(context)
    }

    /// Install the actual cumulative receipt before numerical entry.
    ///
    /// # Errors
    /// Refuses a different original policy or changing an active context phase.
    pub fn install(self, context: &mut MetricContext) -> Result<(), GeoError> {
        if context.policy() != self.policy {
            return Err(GeoError::resident_failure(
                crate::SourcePreparationError::BudgetPolicy {
                    budget: self,
                    actual: context.policy(),
                },
                GeoError::SourcePreparation,
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
        let source = geometry.source_receipt();
        if self.work < source.work_items() {
            return Err(GeoError::retained_failure(
                crate::SourcePreparationError::OffsetAdmission {
                    source,
                    budget: *self,
                    reason: crate::PreparationBaselineReason::Work,
                },
                GeoError::SourcePreparation,
                Some(observer),
            ));
        }
        let source_bytes = source.workspace_bytes();
        let other_bytes = self.workspace.checked_sub(source_bytes).ok_or_else(|| {
            GeoError::retained_failure(
                crate::SourcePreparationError::OffsetAdmission {
                    source,
                    budget: *self,
                    reason: crate::PreparationBaselineReason::Workspace,
                },
                GeoError::SourcePreparation,
                Some(&mut *observer),
            )
        })?;
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
            .ok_or_else(|| {
                GeoError::retained_failure(
                    crate::SourcePreparationError::OffsetConstruction {
                        source,
                        budget: *self,
                        result_work: offset.preparation_work_items(),
                        result_workspace: offset.retained_workspace_bytes(),
                    },
                    GeoError::SourcePreparation,
                    Some(&mut continuation),
                )
            })?;
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
            let semimajor = crate::Rat::from_int(crate::Int::one().shl(256));
            let inverse_flattening = crate::Rat::from_i64(300);
            let limits = crate::ExecutionLimits {
                max_work_items: if memory_failure { 262_144 } else { 1 },
                max_workspace_bytes: if memory_failure { 1 } else { 65_536 },
                ..crate::ExecutionLimits::GEOMETRY
            };
            let mut budget =
                super::PreparationBudget::new(crate::ExecutionPolicy::new(limits).unwrap());
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refusal =
                crate::PreparedEllipsoid::new_in_budget(semimajor, inverse_flattening, &mut budget);
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
        // The inline immutable reference needs no fresh heap allocation. A
        // completely occupied source budget may still admit this borrowed
        // worker, while any actual new byte must be refused before birth.
        let original = budget;
        MetricContext::wgs84().unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let context = budget.worker_context(&reference).unwrap();
        let allocation = window.close();
        assert_eq!(allocation.allocations, 0);
        assert_eq!(allocation.requested_bytes, 0);
        assert_eq!(
            context.retained_workspace_bytes(),
            original.workspace_bytes()
        );
        let admitted = budget;
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refusal = budget.retain(1, 1);
        assert!(matches!(
            refusal,
            Err(crate::GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(budget, admitted);
        let allocation = window.close();
        assert_eq!(allocation.allocations, 0);
        assert_eq!(allocation.requested_bytes, 0);

        let limited = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 1,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut refused = super::PreparationBudget::new(limited);
        let original = refused;
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert!(matches!(
            refused.worker_context(&reference),
            Err(crate::GeoError::WorkExhausted { .. })
        ));
        assert_eq!(refused, original);
        assert_eq!(window.close().allocations, 0);

        let cost = crate::numerical::reference_copy_cost(&reference).unwrap();
        let mut budget = super::PreparationBudget::new(policy);
        budget.retain(5, 10_000).unwrap();
        let context = budget.worker_context(&reference).unwrap();
        assert_eq!(context.preparation_work_items(), 5 + cost.work_items);
        assert_eq!(context.retained_workspace_bytes(), budget.workspace_bytes());
        assert_eq!(context.workspace_peak(), budget.workspace_bytes());
    }

    #[test]
    fn source_baseline_and_budget_policy_refusals_keep_full_original_records() {
        use crate::{GeoError, PreparationBaselineReason, SourcePreparationError};
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let source = super::PreparedSourceReceipt::new(
            purrdf_hash::hex::Digest32::new([113; 32]),
            u64::MAX,
            u64::MAX,
        );
        let mut context = MetricContext::wgs84().unwrap();
        let error = source.validate_context(&context).unwrap_err();
        let GeoError::SourcePreparation(cause) = &error else {
            panic!("original context baseline")
        };
        assert_eq!(
            &**cause,
            &SourcePreparationError::Baseline {
                receipt: source,
                reason: PreparationBaselineReason::Work,
                actual_work: 0,
                actual_workspace: 0
            }
        );
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.child("reason").code, "preparation-baseline-work");
        assert!(
            matches!(fields.child("receipt").field("work"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.child("receipt").field("workspace"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.child("receipt").field("source"), ErrorValue::Bytes(value) if value == [113; 32])
        );
        assert!(!error.is_expression_error());

        let expected = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 17,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let budget = super::PreparationBudget::new(expected);
        let error = budget.install(&mut context).unwrap_err();
        let GeoError::SourcePreparation(cause) = &error else {
            panic!("original preparation policy")
        };
        assert_eq!(
            &**cause,
            &SourcePreparationError::BudgetPolicy {
                budget,
                actual: context.policy()
            }
        );
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "source-preparation-budget-policy");
        assert_eq!(fields.child("budget").code, "preparation-budget");
        assert_eq!(context.preparation_work_items(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert_eq!(
            error.detail(),
            "preparation receipt belongs to a different execution policy"
        );
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
        let error = receipt.validate_context(&context).unwrap_err();
        assert!(
            matches!(error, crate::GeoError::SourcePreparation(cause) if matches!(&*cause, crate::SourcePreparationError::Baseline { receipt: original, reason: crate::PreparationBaselineReason::Work, actual_work: 0, actual_workspace: 0 } if *original == receipt))
        );
        context.charge_work(receipt.work_items()).unwrap();
        context.admit_workspace(receipt.workspace_bytes()).unwrap();
        assert!(receipt.validate_context(&context).is_err());
        context
            .release_workspace(receipt.workspace_bytes())
            .unwrap();
        context.set_preparation_work(receipt.work_items()).unwrap();
        let error = receipt.validate_context(&context).unwrap_err();
        assert!(
            matches!(error, crate::GeoError::SourcePreparation(cause) if matches!(&*cause, crate::SourcePreparationError::Baseline { reason: crate::PreparationBaselineReason::Workspace, .. }))
        );
        context
            .set_retained_workspace(receipt.workspace_bytes())
            .unwrap();
        receipt.validate_context(&context).unwrap();
    }
}
