// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-worker numerical admission and scratch ownership.
//!
//! A context is thread-bound. Immutable references and prepared geometry can be
//! shared; each worker owns its context. No floating control-state guard is kept
//! in a context or across a host callback, suspension or await.

use core::marker::PhantomData;
use purrdf_hash::Backend;
use purrdf_xsd::integer::{LimbScratch, LimbScratchError};
use purrdf_xsd::math::{
    CoordinateMath, FloatProductBackend, MathError, MathLimits, PreparedBinary64, TaylorScratch,
};
use std::rc::Rc;
use std::sync::Arc;

use crate::{ExecutionPolicy, GeoError, GeographicReference};

mod output;
mod preparation;
pub use output::MaterializedOutputReceipt;
pub use preparation::{PreparationBudget, PreparedSourceReceipt};

/// Restricted external work/cancellation capability, without geometry or evaluator reentry.
///
/// A numerical operation invokes this only between bounded chunks with every
/// floating control guard dropped, then revalidates the thread environment.
/// Integer-only assignment, encoding and source-admission chunks use the same
/// charging capability without inspecting floating-point control state.
/// `workspace_growth` counts additional admitted retained/scratch peak bytes.
pub trait MetricWorkObserver {
    /// Charge completed bounded work and newly admitted workspace growth.
    ///
    /// # Errors
    ///
    /// A refusal stops the operation before another bounded chunk runs.
    fn charge_chunk(&mut self, work_items: u64, workspace_growth: u64) -> Result<(), GeoError>;
}

/// Continue one governed request after preparation has already charged the
/// same observer. Context limits still include that work and retained storage;
/// this adapter suppresses only duplicate external charges for the receipt.
pub struct MetricWorkContinuation<'a> {
    observer: &'a mut dyn MetricWorkObserver,
    remaining_work: u64,
    remaining_workspace: u64,
}
purrdf_hash::debug_non_exhaustive!(MetricWorkContinuation<'_> { remaining_work, remaining_workspace });

impl<'a> MetricWorkContinuation<'a> {
    /// Carry the actual already observed cumulative work and workspace peak
    /// from preparation into the following numerical phase.
    #[must_use]
    pub fn new(observer: &'a mut dyn MetricWorkObserver, work: u64, workspace_peak: u64) -> Self {
        Self {
            observer,
            remaining_work: work,
            remaining_workspace: workspace_peak,
        }
    }
}

impl MetricWorkObserver for MetricWorkContinuation<'_> {
    fn charge_chunk(&mut self, work_items: u64, workspace_growth: u64) -> Result<(), GeoError> {
        let work = work_items.min(self.remaining_work);
        let workspace = workspace_growth.min(self.remaining_workspace);
        self.remaining_work -= work;
        self.remaining_workspace -= workspace;
        self.observer
            .charge_chunk(work_items - work, workspace_growth - workspace)
    }
}

#[derive(Clone, Copy)]
enum ObserverArithmetic {
    Floating,
    Integer,
}
impl ObserverArithmetic {
    fn validate(self) -> Result<(), GeoError> {
        match self {
            Self::Floating => validate_environment(),
            Self::Integer => Ok(()),
        }
    }
}

/// One home for cumulative external work charging and post-callback validation.
/// Callers drop every floating guard before entering this adapter.
pub(crate) struct WorkProgress<'a> {
    observer: Option<&'a mut dyn MetricWorkObserver>,
    arithmetic: ObserverArithmetic,
    work: u64,
    peak: u64,
    math_work_base: u64,
    math_workspace_base: u64,
    math_peak_base: u64,
    prepared_coefficients: Option<Arc<crate::geodesic::WorkerCoefficientCache>>,
    prepared_coefficients_added_bytes: u64,
    error: Option<GeoError>,
}

impl<'a> WorkProgress<'a> {
    pub(crate) fn new(observer: Option<&'a mut dyn MetricWorkObserver>) -> Self {
        Self::with_arithmetic(observer, ObserverArithmetic::Floating)
    }

    /// Pure discrete/integer laws share charging and cancellation while
    /// remaining independent of the thread's floating-point control state.
    pub(crate) fn integer(observer: Option<&'a mut dyn MetricWorkObserver>) -> Self {
        Self::with_arithmetic(observer, ObserverArithmetic::Integer)
    }

    fn with_arithmetic(
        observer: Option<&'a mut dyn MetricWorkObserver>,
        arithmetic: ObserverArithmetic,
    ) -> Self {
        Self {
            observer,
            arithmetic,
            work: 0,
            peak: 0,
            math_work_base: 0,
            math_workspace_base: 0,
            math_peak_base: 0,
            prepared_coefficients: None,
            prepared_coefficients_added_bytes: 0,
            error: None,
        }
    }

    pub(crate) fn initial(&mut self) -> Result<(), GeoError> {
        self.charge_counts(1, 0)
    }

    pub(crate) fn reset(&mut self) {
        self.work = 0;
        // Sequential numerical precision attempts reuse scratch. Keep the
        // largest admitted peak so callbacks receive only its new growth.
        self.math_work_base = 0;
        self.math_workspace_base = 0;
        self.math_peak_base = 0;
    }

    pub(crate) fn charge_counts(&mut self, work: u64, peak: u64) -> Result<(), GeoError> {
        self.charge_counts_with(work, peak, self.arithmetic)
    }

    fn charge_counts_with(
        &mut self,
        work: u64,
        peak: u64,
        arithmetic: ObserverArithmetic,
    ) -> Result<(), GeoError> {
        // A refusal ends the invocation's external observation. Cleanup and
        // enclosing numerical phases may still account native work, but must
        // neither call the observer again nor replace its original failure.
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let count = work.saturating_sub(self.work);
        let growth = peak.saturating_sub(self.peak);
        self.work = self.work.max(work);
        self.peak = self.peak.max(peak);
        if let Some(observer) = self.observer.as_deref_mut() {
            let result = observer
                .charge_chunk(count, growth)
                .and_then(|()| arithmetic.validate());
            if let Err(error) = result {
                self.error = Some(error.clone());
                return Err(error);
            }
        }
        Ok(())
    }

    pub(crate) fn context_poll(&mut self, context: &MetricContext) -> Result<(), GeoError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if let Err(error) = context.checkpoint_with(self.arithmetic) {
            self.error = Some(error.clone());
            return Err(error);
        }
        self.charge_counts(context.work_items(), context.workspace_peak())
    }

    /// Admit the arithmetic home's complete exact-operation bound before any
    /// products, reductions or temporary allocations. The caller separately
    /// retains result storage; this reservation covers the bounded computation.
    pub(crate) fn exact<T>(
        &mut self,
        context: &mut MetricContext,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.exact_context(context, cost, |_| evaluate())
    }

    /// The same scoped admission when the body borrows an immutable source
    /// owned by the worker itself. No borrowed data escapes this closure.
    pub(crate) fn exact_context<T>(
        &mut self,
        context: &mut MetricContext,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&mut MetricContext) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        // Reject an inadmissible operation before opening scratch that its body
        // cannot use. The original checked counters supply both preflights;
        // neither changes the live or observed receipt.
        self.preflight_exact(context, cost, 0)?;
        context.admit_workspace(cost.workspace_bytes)?;
        let result = (|| {
            context.charge_work(cost.work_items)?;
            self.context_poll(context)?;
            let result = evaluate(context);
            self.context_poll(context)?;
            result
        })();
        context.release_workspace(cost.workspace_bytes)?;
        result
    }

    /// Check the original exact operation together with separately retained
    /// output storage before either reservation changes a receipt. This does
    /// not charge work or allocate; `exact_context` still admits the body.
    pub(crate) fn preflight_exact(
        &self,
        context: &MetricContext,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        retained_bytes: u64,
    ) -> Result<(), GeoError> {
        if let Some(error) = self.latched_error() {
            return Err(error);
        }
        let bytes =
            cost.workspace_bytes
                .checked_add(retained_bytes)
                .ok_or(GeoError::MemoryExhausted {
                    limit: context.policy.limits().max_workspace_bytes,
                })?;
        context.next_workspace(bytes)?;
        context.next_work(cost.work_items)?;
        Ok(())
    }

    pub(crate) fn math_phase(&mut self, context: &MetricContext) {
        self.math_work_base = context.work_items;
        self.math_workspace_base = context.workspace_bytes;
        self.math_peak_base = context.workspace_peak;
        self.prepared_coefficients
            .clone_from(&context.geodesic_coefficients);
        self.prepared_coefficients_added_bytes = 0;
    }

    pub(crate) fn prepared_coefficients(
        &self,
    ) -> Option<&Arc<crate::geodesic::WorkerCoefficientCache>> {
        self.prepared_coefficients.as_ref()
    }

    pub(crate) fn retain_prepared_coefficients(
        &mut self,
        cache: Arc<crate::geodesic::WorkerCoefficientCache>,
        bytes: u64,
    ) -> Result<(), MathError> {
        let added = self
            .prepared_coefficients_added_bytes
            .checked_add(bytes)
            .ok_or(MathError::WorkspaceExhausted)?;
        self.prepared_coefficients = Some(cache);
        self.prepared_coefficients_added_bytes = added;
        Ok(())
    }

    pub(crate) fn take_prepared_coefficients(
        &mut self,
    ) -> (Option<Arc<crate::geodesic::WorkerCoefficientCache>>, u64) {
        (
            self.prepared_coefficients.take(),
            core::mem::take(&mut self.prepared_coefficients_added_bytes),
        )
    }

    pub(crate) fn math_poll(&mut self, math: &CoordinateMath) -> Result<(), MathError> {
        if self.error.is_some() {
            return Err(MathError::Cancelled);
        }
        let result = self.charge_counts_with(
            self.math_work_base.saturating_add(math.work_used()),
            self.math_peak_base.max(
                self.math_workspace_base
                    .saturating_add(math.workspace_peak() as u64),
            ),
            ObserverArithmetic::Floating,
        );
        if let Err(error) = result {
            self.error = Some(error);
            Err(MathError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub(crate) fn latched_error(&self) -> Option<GeoError> {
        self.error.clone()
    }

    pub(crate) fn nested(
        &mut self,
        work_base: u64,
        workspace_base: u64,
        peak_base: u64,
    ) -> impl MetricWorkObserver + '_ {
        NestedProgress {
            work_base: self.work.max(work_base),
            workspace_base,
            peak_base: self.peak.max(peak_base),
            work: 0,
            workspace: 0,
            parent: self,
        }
    }

    /// Adopt a nested phase's already observed work and transient peak. The
    /// child admitted these counts before allocation under the remaining
    /// policy; this records them in the parent's native receipt without another
    /// callback or changing its live retained/transient storage. The caller
    /// separately retains any returned owner.
    pub(crate) fn absorb_nested<T>(
        &self,
        context: &mut MetricContext,
        result: Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let accounting = (|| {
            context.charge_work(self.work.saturating_sub(context.work_items()))?;
            let growth = self.peak.saturating_sub(context.current_workspace_bytes());
            context.admit_workspace(growth)?;
            context.release_workspace(growth)
        })();
        self.settle_result(result, accounting)
    }

    /// Settle a real sequential child's work, cache lineage and retained peak
    /// through the original accounting body, then reconcile the already
    /// observed receipt. A prior observer refusal remains the first cause even
    /// if settling the child's later lineage or storage also refuses.
    pub(crate) fn absorb_child_result<T>(
        &self,
        context: &mut MetricContext,
        child: &MetricContext,
        result: Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.absorb_child_result_with_baseline(context, child, 0, 0, result)
    }

    /// Settle a child whose original source receipt was already included in
    /// the parent invocation. Only the remaining original work/storage enters
    /// the same accounting and first-refusal reconciliation body.
    pub(crate) fn absorb_child_result_with_baseline<T>(
        &self,
        context: &mut MetricContext,
        child: &MetricContext,
        original_work: u64,
        original_storage: u64,
        result: Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let accounting = context.absorb_child_with_baseline(child, original_work, original_storage);
        let result = self.settle_result(result, accounting);
        self.absorb_nested(context, result)
    }

    fn settle_result<T>(
        &self,
        result: Result<T, GeoError>,
        accounting: Result<(), GeoError>,
    ) -> Result<T, GeoError> {
        if let Some(error) = self.latched_error() {
            return Err(error);
        }
        // A row-scoped argument error cannot conceal a fatal accounting
        // refusal from FILTER/BIND. An earlier native fatal error keeps its
        // original cause. The shared kernel classification owns that split.
        if result
            .as_ref()
            .err()
            .is_none_or(GeoError::is_expression_error)
        {
            accounting?;
        }
        result
    }
}

struct NestedProgress<'a, 'observer> {
    parent: &'a mut WorkProgress<'observer>,
    work_base: u64,
    workspace_base: u64,
    peak_base: u64,
    work: u64,
    workspace: u64,
}

impl MetricWorkObserver for NestedProgress<'_, '_> {
    fn charge_chunk(&mut self, work_items: u64, workspace_growth: u64) -> Result<(), GeoError> {
        self.work = self
            .work
            .checked_add(work_items)
            .ok_or(GeoError::ArithmeticOverflow("nested work accounting"))?;
        self.workspace = self
            .workspace
            .checked_add(workspace_growth)
            .ok_or(GeoError::ArithmeticOverflow("nested workspace accounting"))?;
        self.parent.charge_counts(
            self.work_base.saturating_add(self.work),
            self.peak_base
                .max(self.workspace_base.saturating_add(self.workspace)),
        )
    }
}

/// Per-thread geographic numerical context and checked resource counters.
///
/// ```compile_fail
/// use purrdf_geo_kernel::MetricContext;
/// fn assert_send<T: Send>() {}
/// assert_send::<MetricContext>();
/// ```
///
/// ```compile_fail
/// use purrdf_geo_kernel::MetricContext;
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<MetricContext>();
/// ```
#[derive(Debug)]
pub struct MetricContext {
    reference: GeographicReference,
    policy: ExecutionPolicy,
    work_items: u64,
    preparation_work_items: u64,
    workspace_bytes: u64,
    retained_workspace_bytes: u64,
    materialized_output_bytes: u64,
    materialized_receipt_storage_bytes: u64,
    materialized_outputs: purrdf_core::SmallVec<[MaterializedOutputReceipt; 4]>,
    workspace_peak: u64,
    cancelled: bool,
    arithmetic: Option<PreparedBinary64>,
    integer_scratch: Option<LimbScratch>,
    integer_scratch_owned: bool,
    taylor_scratch: Option<TaylorScratch>,
    taylor_scratch_owned_bytes: u64,
    geodesic_coefficients: Option<Arc<crate::geodesic::WorkerCoefficientCache>>,
    geodesic_coefficients_owned_bytes: u64,
    zero_conversion_bound: Option<crate::Rat>,
    zero_conversion_bound_owned_bytes: u64,
    binary64_backend: FloatProductBackend,
    thread_bound: PhantomData<Rc<()>>,
}

impl MetricContext {
    /// Validate the current thread's numerical environment and prepare a context.
    ///
    /// # Errors
    ///
    /// Refuses an incompatible floating-point environment before arithmetic.
    pub fn new(reference: GeographicReference, policy: ExecutionPolicy) -> Result<Self, GeoError> {
        validate_environment()?;
        Ok(Self {
            reference,
            policy,
            work_items: 0,
            preparation_work_items: 0,
            workspace_bytes: 0,
            retained_workspace_bytes: 0,
            materialized_output_bytes: 0,
            materialized_receipt_storage_bytes: 0,
            materialized_outputs: purrdf_core::SmallVec::new(),
            workspace_peak: 0,
            cancelled: false,
            arithmetic: None,
            integer_scratch: None,
            integer_scratch_owned: false,
            taylor_scratch: None,
            taylor_scratch_owned_bytes: 0,
            geodesic_coefficients: None,
            geodesic_coefficients_owned_bytes: 0,
            zero_conversion_bound: None,
            zero_conversion_bound_owned_bytes: 0,
            binary64_backend: FloatProductBackend::selected(),
            thread_bound: PhantomData,
        })
    }

    /// Construct a worker from a borrowed immutable reference, admitting the
    /// complete original-parameter clone before copying any integer limbs.
    ///
    /// # Errors
    /// Adds original reference work/storage refusal to [`Self::new`].
    pub fn from_reference(
        reference: &GeographicReference,
        policy: ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::from_reference_observed(reference, policy, None)
    }

    /// Construct the same borrowed worker with bounded external work charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_reference`].
    pub fn from_reference_metered(
        reference: &GeographicReference,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::from_reference_observed(reference, policy, Some(observer))
    }

    fn from_reference_observed(
        reference: &GeographicReference,
        policy: ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        validate_environment()?;
        let cost = crate::numerical::reference_copy_cost(reference)?;
        let retained = reference.ellipsoid().retained_limb_bytes();
        let peak = retained
            .checked_add(cost.workspace_bytes)
            .ok_or(GeoError::ArithmeticOverflow("worker reference preparation"))?;
        if cost.work_items > policy.limits().max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: policy.limits().max_work_items,
            });
        }
        if peak > policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            });
        }
        if let Some(observer) = observer {
            observer.charge_chunk(cost.work_items, peak)?;
            validate_environment()?;
        }
        let mut context = Self::new(reference.clone(), policy)?;
        context.set_preparation_work(cost.work_items)?;
        context.set_retained_workspace(retained)?;
        context.workspace_peak = peak;
        Ok(context)
    }

    /// Copy the original reference for immutable preparation under this worker's
    /// cumulative admission. The returned parameter limbs stay in the retained
    /// preparation baseline, independently of subsequent coefficient tables.
    ///
    /// # Errors
    /// Refuses original copy work/storage before allocating the returned limbs.
    pub fn clone_reference_for_preparation(&mut self) -> Result<GeographicReference, GeoError> {
        self.clone_reference_observed(None)
    }

    /// Copy the same immutable reference with bounded governor/cancellation
    /// charging before the original parameter limbs are cloned.
    /// # Errors
    /// Adds observer refusal to [`Self::clone_reference_for_preparation`].
    pub fn clone_reference_for_preparation_metered(
        &mut self,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<GeographicReference, GeoError> {
        self.clone_reference_observed(Some(observer))
    }

    fn clone_reference_observed(
        &mut self,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<GeographicReference, GeoError> {
        let bytes = self.reference.ellipsoid().retained_limb_bytes();
        let retained = self
            .retained_workspace_bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("prepared reference storage"))?;
        self.admit_workspace(bytes)?;
        let result = crate::numerical::reference_clone(self, &mut WorkProgress::new(observer));
        match result {
            Ok(reference) => {
                self.set_retained_workspace(retained)?;
                self.set_preparation_work(self.work_items)?;
                Ok(reference)
            }
            Err(error) => {
                self.release_workspace(bytes)?;
                Err(error)
            }
        }
    }

    /// Prepare WGS84 with the frozen default resource admission.
    ///
    /// # Errors
    ///
    /// Refuses an incompatible floating-point environment.
    pub fn wgs84() -> Result<Self, GeoError> {
        Self::new(GeographicReference::wgs84(), ExecutionPolicy::geometry())
    }

    /// Borrow the remaining admission into an independently counted numerical
    /// child, sharing only immutable arithmetic preparation.
    pub(crate) fn remaining_child(&self) -> Result<Self, GeoError> {
        self.remaining_child_with_baseline(0, 0)
    }

    pub(crate) fn remaining_child_with_baseline(
        &self,
        work: u64,
        storage: u64,
    ) -> Result<Self, GeoError> {
        if work > self.preparation_work_items || storage > self.retained_workspace_bytes {
            return Err(GeoError::config(
                "child source receipt exceeds its parent preparation",
            ));
        }
        let retained = self
            .policy
            .limits()
            .max_workspace_bytes
            .saturating_sub(self.remaining_workspace());
        let policy = self.policy.remaining_after(
            self.work_items.saturating_sub(work),
            retained.saturating_sub(storage),
        )?;
        let mut child = Self::new(self.reference.clone(), policy)?;
        child.binary64_backend = self.binary64_backend;
        // These immutable coefficients are already charged to their owning
        // worker. A sequential child borrows the same allocation without
        // claiming or charging its heap again.
        child
            .geodesic_coefficients
            .clone_from(&self.geodesic_coefficients);
        child
            .zero_conversion_bound
            .clone_from(&self.zero_conversion_bound);
        if let Some(scratch) = &self.integer_scratch {
            child.set_borrowed_integer_scratch(scratch.clone())?;
        }
        child.taylor_scratch.clone_from(&self.taylor_scratch);
        if let Some(arithmetic) = self.prepared_arithmetic() {
            child.set_prepared_arithmetic(arithmetic);
        }
        child.set_preparation_work(work)?;
        child.set_retained_workspace(storage)?;
        Ok(child)
    }

    // Raw settlement is retained only for independent receipt-control tests.
    // Production settles the result together with its original observer cause.
    #[cfg(test)]
    pub(crate) fn absorb_child(&mut self, child: &Self) -> Result<(), GeoError> {
        self.absorb_child_with_baseline(child, 0, 0)
    }

    pub(crate) fn absorb_child_with_baseline(
        &mut self,
        child: &Self,
        original_work: u64,
        original_storage: u64,
    ) -> Result<(), GeoError> {
        if original_work > child.preparation_work_items()
            || original_storage > child.retained_workspace_bytes()
        {
            return Err(GeoError::config(
                "absorbed child receipt exceeds its original baseline",
            ));
        }
        let work = child
            .work_items()
            .checked_sub(original_work)
            .ok_or(GeoError::ArithmeticOverflow("child work receipt"))?;
        let workspace = child
            .workspace_peak()
            .checked_sub(original_storage)
            .ok_or(GeoError::ArithmeticOverflow("child workspace receipt"))?;
        self.charge_work(work)?;
        let mut retained_coefficients = None;
        if child.geodesic_coefficients_owned_bytes != 0
            && let Some(coefficients) = &child.geodesic_coefficients
            && !self
                .geodesic_coefficients
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, coefficients))
        {
            let lineage_work = coefficients
                .lineage_work()
                .ok_or(GeoError::ArithmeticOverflow("child coefficient lineage"))?;
            self.charge_work(lineage_work)?;
            if let Some(bytes) = crate::geodesic::WorkerCoefficientCache::lineage_bytes(
                coefficients,
                self.geodesic_coefficients.as_ref(),
            ) && bytes != 0
                && bytes <= child.geodesic_coefficients_owned_bytes
            {
                if bytes > workspace {
                    return Err(GeoError::ArithmeticOverflow(
                        "child coefficient receipt exceeds its peak",
                    ));
                }
                let owned = self
                    .geodesic_coefficients_owned_bytes
                    .checked_add(bytes)
                    .ok_or(GeoError::ArithmeticOverflow("absorbed coefficient storage"))?;
                retained_coefficients = Some((coefficients.clone(), bytes, owned));
            }
        }
        self.admit_workspace(workspace)?;
        let retained = if let Some((coefficients, bytes, owned)) = retained_coefficients {
            // The child's complete peak was observed and admitted before the
            // immutable allocation was made. Retain only its new heads from
            // that same peak; inherited tables remain charged to their owner.
            self.promote_admitted_workspace(bytes)?;
            self.geodesic_coefficients = Some(coefficients);
            self.geodesic_coefficients_owned_bytes = owned;
            bytes
        } else {
            0
        };
        self.release_workspace(workspace - retained)
    }

    /// The immutable exact reference for this context.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }

    /// The validated resource admission, independent of mathematical identity.
    #[must_use]
    pub const fn policy(&self) -> ExecutionPolicy {
        self.policy
    }

    /// Borrow this worker's explicitly admitted reusable exact-arithmetic arena.
    /// Values retain valid immutable ownership after the worker is dropped.
    #[must_use]
    pub fn integer_scratch(&self) -> Option<&LimbScratch> {
        self.integer_scratch.as_ref()
    }

    /// Borrow reusable branded Taylor coefficient storage owned by this worker
    /// or its sequential parent. No numerical lock is retained by this handle.
    #[must_use]
    pub fn taylor_scratch(&self) -> Option<&TaylorScratch> {
        self.taylor_scratch.as_ref()
    }

    pub(crate) fn adopt_taylor_scratch(&mut self, scratch: TaylorScratch) -> Result<(), GeoError> {
        if self
            .taylor_scratch
            .as_ref()
            .is_some_and(|current| current.shares_storage(&scratch))
        {
            return Ok(());
        }
        let bytes =
            u64::try_from(scratch.retained_bytes()).map_err(|_| GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            })?;
        // The numerical phase has already exposed allocation admission to the
        // observer. Promotion counts the overlap with the old retained pool
        // before dropping that owner; borrowed parent storage stays charged to
        // the parent and is never subtracted from this child's receipt.
        self.admit_retained_workspace(bytes)?;
        let old_bytes = self.taylor_scratch_owned_bytes;
        self.taylor_scratch = Some(scratch);
        self.taylor_scratch_owned_bytes = bytes;
        self.retained_workspace_bytes = self
            .retained_workspace_bytes
            .checked_sub(old_bytes)
            .ok_or(GeoError::ArithmeticOverflow("replaced Taylor scratch"))?;
        self.release_workspace(old_bytes)
    }

    pub(crate) fn geodesic_coefficients(
        &self,
    ) -> Option<&Arc<crate::geodesic::WorkerCoefficientCache>> {
        self.geodesic_coefficients.as_ref()
    }

    pub(crate) fn zero_conversion_bound(&self) -> Option<&crate::Rat> {
        self.zero_conversion_bound.as_ref()
    }

    pub(crate) fn retain_zero_conversion_bound(
        &mut self,
        value: crate::Rat,
    ) -> Result<(), GeoError> {
        let bytes = value.allocated_bytes() as u64;
        self.admit_retained_workspace(bytes)?;
        self.zero_conversion_bound = Some(value);
        self.zero_conversion_bound_owned_bytes = bytes;
        Ok(())
    }

    pub(crate) fn retain_geodesic_coefficients(
        &mut self,
        coefficients: Arc<crate::geodesic::WorkerCoefficientCache>,
        bytes: u64,
    ) -> Result<(), GeoError> {
        let owned_bytes = self
            .geodesic_coefficients_owned_bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "owned geodesic coefficient storage",
            ))?;
        self.admit_retained_workspace(bytes)?;
        self.geodesic_coefficients = Some(coefficients);
        self.geodesic_coefficients_owned_bytes = owned_bytes;
        Ok(())
    }

    /// Install an arena whose complete heap is retained and admitted by an
    /// enclosing preparation owner. This context counts only a later local
    /// replacement; borrowing does not charge the same shared heap twice.
    pub(crate) fn set_borrowed_integer_scratch(
        &mut self,
        scratch: LimbScratch,
    ) -> Result<(), GeoError> {
        self.checkpoint()?;
        if self.integer_scratch.is_some() {
            return Err(GeoError::config("integer scratch already installed"));
        }
        if scratch.destination_capacity() > self.policy.limits().max_scratch_destinations as usize {
            return Err(GeoError::config(
                "borrowed scratch exceeds admitted destination count",
            ));
        }
        self.integer_scratch = Some(scratch);
        self.integer_scratch_owned = false;
        Ok(())
    }

    /// Prepare bounded destinations once for every admitted numerical precision.
    /// This worker counts the entire heap once; nested children borrow that
    /// arena and their reduced budgets exclude its parent-owned heap. Checkout
    /// locks never span arithmetic, callbacks, or suspension.
    /// # Errors
    /// Refuses cancellation, sizing overflow, or workspace exhaustion before allocation.
    pub fn prepare_integer_scratch(&mut self) -> Result<(), GeoError> {
        self.prepare_integer_scratch_for(0)
    }

    /// Prepare or grow the arena before a kernel using actual original source
    /// operand bits. The new and old heaps overlap during checked preparation;
    /// no destination grows or falls back to an allocation inside arithmetic.
    /// # Errors
    /// Refuses overflow, cancellation, or workspace exhaustion before allocation.
    pub fn prepare_integer_scratch_for(&mut self, source_bits: u64) -> Result<(), GeoError> {
        self.prepare_integer_scratch_with_capacity(
            source_bits,
            self.policy.limits().max_scratch_destinations as usize,
        )
    }

    /// Explicitly admit a larger reusable destination count before a retained
    /// panel/jet computation. Existing equal-or-larger arenas are reused.
    /// # Errors
    /// Refuses cancellation, sizing overflow, or the actual overlap heap before allocation.
    pub fn prepare_integer_scratch_with_capacity(
        &mut self,
        source_bits: u64,
        buffers: usize,
    ) -> Result<(), GeoError> {
        self.prepare_integer_scratch_observed(source_bits, buffers, None)
    }

    pub(crate) fn prepare_integer_scratch_for_observed(
        &mut self,
        source_bits: u64,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        self.prepare_integer_scratch_observed(
            source_bits,
            self.policy.limits().max_scratch_destinations as usize,
            Some(progress),
        )
    }

    pub(crate) fn prepare_integer_scratch_for_cost_observed(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        // A final exact proof already has a complete intermediate-width bound.
        // Reapplying original-source growth to its precision-grid denominator
        // would unnecessarily replace an adequate arena while that proof is live.
        if self
            .integer_scratch
            .as_ref()
            .is_some_and(|scratch| scratch.limb_capacity() as u64 >= cost.output_bits.div_ceil(64))
        {
            return Ok(());
        }
        self.prepare_integer_scratch_for_observed(cost.output_bits, progress)
    }

    fn prepare_integer_scratch_observed(
        &mut self,
        source_bits: u64,
        buffers: usize,
        progress: Option<&mut WorkProgress<'_>>,
    ) -> Result<(), GeoError> {
        self.checkpoint()?;
        if buffers > self.policy.limits().max_scratch_destinations as usize {
            return Err(GeoError::config(
                "scratch destination request exceeds admitted execution policy",
            ));
        }
        let limbs = usize::try_from(
            u64::from(self.policy.limits().max_precision_bits)
                .checked_mul(4)
                .and_then(|bits| bits.checked_add(2048))
                .and_then(|bits| {
                    source_bits
                        .checked_mul(2)
                        .and_then(|source| bits.checked_add(source))
                })
                .ok_or(GeoError::ArithmeticOverflow("integer scratch source width"))?
                .div_ceil(64),
        )
        .map_err(|_| GeoError::MemoryExhausted {
            limit: self.policy.limits().max_workspace_bytes,
        })?;
        if self.integer_scratch.as_ref().is_some_and(|scratch| {
            scratch.limb_capacity() >= limbs && scratch.destination_capacity() >= buffers
        }) {
            return Ok(());
        }
        // A returned proof may still own one of the old arena's destinations.
        // Replacing that arena would otherwise erase its live heap receipt.
        if let Some(scratch) = &self.integer_scratch
            && scratch.available() != scratch.destination_capacity()
        {
            return Err(GeoError::NumericalScratch(LimbScratchError::Retained {
                held_destinations: scratch.destination_capacity() - scratch.available(),
                destinations: scratch.destination_capacity(),
            }));
        }
        let old_bytes = if self.integer_scratch_owned {
            self.integer_scratch
                .as_ref()
                .map_or(0, |scratch| scratch.retained_bytes() as u64)
        } else {
            0
        };
        // Up to 128 integral coefficients retain two endpoints each, along
        // with generated constants and bounded simultaneous solver scratch.
        let bytes =
            LimbScratch::required_bytes(buffers, limbs).map_err(|_| GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            })?;
        self.admit_workspace(bytes as u64)?;
        // The outer governor observes the complete native admission before
        // the arena's vectors or ownership headers are allocated.
        let prepared = (|| {
            if let Some(progress) = progress {
                progress.context_poll(self)?;
            }
            LimbScratch::new(buffers, limbs).map_err(|_| GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            })
        })();
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.release_workspace(bytes as u64)?;
                return Err(error);
            }
        };
        // The entire new heap is already admitted above, before allocation.
        self.promote_admitted_workspace(bytes as u64)?;
        self.integer_scratch = Some(prepared);
        self.integer_scratch_owned = true;
        self.retained_workspace_bytes = self
            .retained_workspace_bytes
            .checked_sub(old_bytes)
            .ok_or(GeoError::ArithmeticOverflow("replaced integer scratch"))?;
        self.release_workspace(old_bytes)?;
        Ok(())
    }

    pub(crate) fn coordinate_math(&self, limits: MathLimits) -> Result<CoordinateMath, MathError> {
        let mut math = match &self.integer_scratch {
            Some(scratch) => {
                CoordinateMath::new_with_borrowed_limb_scratch(limits, scratch.clone())
            }
            None => CoordinateMath::new(limits),
        }?;
        if let Some(scratch) = &self.taylor_scratch {
            math.set_borrowed_taylor_scratch(scratch.clone())?;
        }
        Ok(math)
    }

    /// Share immutable generated arithmetic tables with another worker context.
    /// This contains no floating guard, mutable scratch or semantic parameters.
    #[must_use]
    pub fn prepared_arithmetic(&self) -> Option<PreparedBinary64> {
        self.arithmetic.clone()
    }

    /// Install equivalent immutable arithmetic tables prepared by another worker.
    /// Work and retained storage are admitted when the next numerical batch begins.
    pub fn set_prepared_arithmetic(&mut self, prepared: PreparedBinary64) {
        self.arithmetic = Some(prepared);
    }

    /// Select an equivalent admitted arithmetic backend for this worker and its
    /// numerical children. This choice does not change any semantic identity.
    ///
    /// # Errors
    /// Refuses a backend unavailable on the executing target.
    pub fn set_binary64_backend(&mut self, backend: FloatProductBackend) -> Result<(), GeoError> {
        if !backend.is_available() {
            return Err(GeoError::config("unavailable binary64 product backend"));
        }
        self.binary64_backend = backend;
        Ok(())
    }

    /// Equivalent arithmetic execution path selected for this worker.
    #[must_use]
    pub const fn binary64_backend(&self) -> FloatProductBackend {
        self.binary64_backend
    }

    /// Prepare immutable arithmetic once before a worker batch and return a
    /// shareable handle. Preparation has its own measured work/workspace receipt.
    ///
    /// # Errors
    /// Refuses an invalid environment, cancellation and exhausted admission.
    pub fn prepare_arithmetic(&mut self) -> Result<PreparedBinary64, GeoError> {
        self.begin(0)?;
        let policy = self.policy;
        let mut math = CoordinateMath::new(MathLimits {
            precision_bits: policy.limits().max_precision_bits.min(96),
            max_work: policy.limits().max_work_items,
            max_workspace_bytes: usize::try_from(policy.limits().max_workspace_bytes).map_err(
                |_| GeoError::MemoryExhausted {
                    limit: policy.limits().max_workspace_bytes,
                },
            )?,
        })
        .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
        self.install_arithmetic(&mut math)
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
        self.charge_work(math.work_used())?;
        self.admit_workspace(math.workspace_peak() as u64)?;
        self.release_workspace(math.workspace_peak() as u64)?;
        self.arithmetic.clone().ok_or(GeoError::ArithmeticOverflow(
            "prepared arithmetic invariant",
        ))
    }

    pub(crate) fn install_arithmetic(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        math.set_binary64_backend(self.binary64_backend)?;
        if let Some(prepared) = &self.arithmetic {
            math.install_binary64(prepared.clone())
        } else {
            let prepared = math.prepare_binary64()?;
            self.arithmetic = Some(prepared);
            Ok(())
        }
    }

    /// Work consumed by the current or most recent operation/batch.
    #[must_use]
    pub const fn work_items(&self) -> u64 {
        self.work_items
    }

    /// Bind already performed preparation to the following complete operation.
    /// The same original policy admits preparation and numerical evaluation.
    /// Each numerical entry retains this charge while resetting transient work.
    /// # Errors
    /// Refuses cancellation, insufficient work, or changing an active work charge.
    pub fn set_preparation_work(&mut self, items: u64) -> Result<(), GeoError> {
        if self.cancelled {
            return Err(GeoError::Cancelled);
        }
        if self.work_items != self.preparation_work_items && self.work_items != items {
            return Err(GeoError::ArithmeticOverflow(
                "preparation work changed during invocation",
            ));
        }
        let limit = self.policy.limits().max_work_items;
        if items > limit {
            return Err(GeoError::WorkExhausted { limit });
        }
        self.preparation_work_items = items;
        self.work_items = items;
        Ok(())
    }

    /// Original preparation work retained across numerical invocation entries.
    #[must_use]
    pub const fn preparation_work_items(&self) -> u64 {
        self.preparation_work_items
    }

    /// Immutable source/preparation bytes retained separately from active scratch.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_workspace_bytes
    }

    /// Current retained and temporary storage admitted by this context.
    pub(crate) const fn current_workspace_bytes(&self) -> u64 {
        self.workspace_bytes
    }

    /// Admit completed worker-owned storage without disturbing an enclosing
    /// source reservation. The numerical phase observes its allocation before
    /// publication; this promotion preserves the live scratch and peak count.
    fn admit_retained_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
        self.admit_workspace(bytes)?;
        self.promote_admitted_workspace(bytes)
    }

    fn promote_admitted_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
        let retained = self
            .retained_workspace_bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("retained numerical storage"))?;
        if retained > self.workspace_bytes {
            return Err(GeoError::ArithmeticOverflow(
                "retained numerical storage was not admitted",
            ));
        }
        self.retained_workspace_bytes = retained;
        Ok(())
    }

    /// Retain completed preparation's actual work and admitted owned storage.
    ///
    /// # Errors
    /// Refuses cancellation or an inconsistent transient admission. Callers
    /// release temporary scratch before promoting their completed owned data.
    pub fn retain_current_preparation(&mut self) -> Result<(), GeoError> {
        self.checkpoint()?;
        let total = self.workspace_bytes;
        self.release_workspace(total.saturating_sub(self.retained_workspace_bytes))?;
        self.set_retained_workspace(total)?;
        self.set_preparation_work(self.work_items)
    }

    pub(crate) fn release_transient_workspace(&mut self) -> Result<(), GeoError> {
        self.release_workspace(
            self.workspace_bytes
                .checked_sub(self.retained_workspace_bytes)
                .ok_or(GeoError::ArithmeticOverflow("transient workspace baseline"))?,
        )
    }

    /// End one checked immutable-source admission without erasing its work or
    /// peak receipt. Numerical preparation retained during the call remains live.
    pub(crate) fn release_preparation_admission(
        &mut self,
        work: u64,
        bytes: u64,
    ) -> Result<(), GeoError> {
        if self.workspace_bytes != self.retained_workspace_bytes {
            return Err(GeoError::ArithmeticOverflow(
                "releasing preparation with active scratch",
            ));
        }
        let remaining_work =
            self.preparation_work_items
                .checked_sub(work)
                .ok_or(GeoError::ArithmeticOverflow(
                    "source preparation work release",
                ))?;
        let remaining_bytes = self.retained_workspace_bytes.checked_sub(bytes).ok_or(
            GeoError::ArithmeticOverflow("source preparation storage release"),
        )?;
        self.check_owned_workspace(remaining_bytes)?;
        self.preparation_work_items = remaining_work;
        self.retained_workspace_bytes = remaining_bytes;
        self.workspace_bytes = remaining_bytes;
        Ok(())
    }

    /// Storage still available while all current sources, scratch and outputs
    /// remain live. Historical temporary peaks do not consume this allowance.
    #[must_use]
    pub fn remaining_workspace(&self) -> u64 {
        self.policy
            .limits()
            .max_workspace_bytes
            .saturating_sub(self.workspace_bytes)
    }

    /// Greatest simultaneously retained workspace in the current operation.
    #[must_use]
    pub const fn workspace_peak(&self) -> u64 {
        self.workspace_peak
    }

    /// Admit immutable caller-owned preparations or index buckets across batches.
    /// This baseline consumes the same policy as transient numerical scratch and
    /// remains present when a new invocation resets its work counters.
    ///
    /// # Errors
    /// Refuses cancellation, a live transient reservation and retained storage
    /// beyond this context's unchanged execution-policy workspace limit.
    pub fn set_retained_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
        if self.cancelled {
            return Err(GeoError::Cancelled);
        }
        if self.workspace_bytes != self.retained_workspace_bytes {
            return Err(GeoError::ArithmeticOverflow(
                "changing retained baseline with active scratch",
            ));
        }
        let limit = self.policy.limits().max_workspace_bytes;
        if bytes > limit {
            return Err(GeoError::MemoryExhausted { limit });
        }
        self.check_owned_workspace(bytes)?;
        self.retained_workspace_bytes = bytes;
        self.workspace_bytes = bytes;
        self.workspace_peak = self.workspace_peak.max(bytes);
        Ok(())
    }

    fn check_owned_workspace(&self, bytes: u64) -> Result<(), GeoError> {
        let arena_bytes = if self.integer_scratch_owned {
            self.integer_scratch
                .as_ref()
                .map_or(0, |arena| arena.retained_bytes() as u64)
        } else {
            0
        };
        let owned = arena_bytes
            .checked_add(self.geodesic_coefficients_owned_bytes)
            .and_then(|value| value.checked_add(self.zero_conversion_bound_owned_bytes))
            .and_then(|value| value.checked_add(self.taylor_scratch_owned_bytes))
            .and_then(|value| value.checked_add(self.materialized_output_bytes))
            .and_then(|value| value.checked_add(self.materialized_receipt_storage_bytes))
            .ok_or(GeoError::ArithmeticOverflow("owned numerical workspace"))?;
        if bytes < owned {
            return Err(GeoError::config(
                "retained workspace cannot erase live owned numerical storage",
            ));
        }
        Ok(())
    }

    /// Cancel this context. Cancellation stays latched until explicitly cleared.
    pub const fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Admit a new invocation after a caller has handled a cancellation.
    pub const fn clear_cancellation(&mut self) {
        self.cancelled = false;
    }

    /// Revalidate after returning from an external callback or suspension.
    ///
    /// # Errors
    ///
    /// Refuses cancellation and a changed floating-point environment.
    pub fn checkpoint(&self) -> Result<(), GeoError> {
        self.checkpoint_with(ObserverArithmetic::Floating)
    }

    fn checkpoint_with(&self, arithmetic: ObserverArithmetic) -> Result<(), GeoError> {
        if self.cancelled {
            return Err(GeoError::Cancelled);
        }
        arithmetic.validate()
    }

    /// Charge a bounded chunk before executing it.
    ///
    /// # Errors
    ///
    /// Refuses cancellation, overflow and work beyond the admitted total.
    pub fn charge_work(&mut self, count: u64) -> Result<(), GeoError> {
        self.work_items = self.next_work(count)?;
        Ok(())
    }

    // The same checked counter can preflight a numerical constructor's
    // nonempty work allowance before allocating its reusable arena.
    pub(crate) fn next_work(&self, count: u64) -> Result<u64, GeoError> {
        if self.cancelled {
            return Err(GeoError::Cancelled);
        }
        let limit = self.policy.limits().max_work_items;
        let next = self
            .work_items
            .checked_add(count)
            .ok_or(GeoError::WorkExhausted { limit })?;
        if next > limit {
            return Err(GeoError::WorkExhausted { limit });
        }
        Ok(next)
    }

    /// Produce a bounded pure host result after admitting its work and storage.
    /// The body cannot call a numerical kernel or an external observer. This
    /// numerical-context seam revalidates FENV after observer callbacks, before
    /// a host formatter inspects floating values. Scratch is released on every
    /// exit. Successful output storage remains admitted
    /// until the caller drops or transfers the result and releases `output_bytes`,
    /// or drops this context at the end of the invocation.
    /// Only new work and peak growth are reported; an aggregate observer must
    /// already have charged this context's entering work and workspace peak.
    ///
    /// # Errors
    /// Refuses output, aggregate work/storage and cancellation before the body.
    /// A failed body or post-body observer releases its output allowance.
    pub fn produce_output_metered<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        output_bytes: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<T, GeoError> {
        self.produce_output_observed(cost, output_bytes, evaluate, Some(observer))
    }

    /// Produce the same bounded pure output without an external observer.
    /// The caller keeps its successful output charged until dropping or
    /// transferring that output and releasing `output_bytes`, or dropping the
    /// invocation context. Scratch and refused output are released on all exits.
    ///
    /// # Errors
    /// Refuses cumulative work/storage, cancellation and the numerical
    /// environment before entering the pure output body.
    pub fn produce_output<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        output_bytes: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.produce_output_observed(cost, output_bytes, evaluate, None)
    }

    fn produce_output_observed<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        output_bytes: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<T, GeoError> {
        if self.policy.limits().max_output_elements == 0 {
            return Err(GeoError::OutputExhausted { limit: 0 });
        }
        let mut continuation = observer.map(|observer| {
            MetricWorkContinuation::new(observer, self.work_items(), self.workspace_peak())
        });
        let mut progress = WorkProgress::new(
            continuation
                .as_mut()
                .map(|value| value as &mut dyn MetricWorkObserver),
        );
        self.admit_workspace(output_bytes)?;
        let result = progress.exact(self, cost, evaluate);
        if result.is_err() {
            self.release_workspace(output_bytes)?;
        }
        result
    }

    /// Admit retained workspace before allocating it.
    ///
    /// # Errors
    ///
    /// Refuses cancellation and aggregate workspace beyond the admitted total.
    pub fn admit_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
        self.workspace_bytes = self.next_workspace(bytes)?;
        self.workspace_peak = self.workspace_peak.max(self.workspace_bytes);
        Ok(())
    }

    fn next_workspace(&self, bytes: u64) -> Result<u64, GeoError> {
        if self.cancelled {
            return Err(GeoError::Cancelled);
        }
        let limit = self.policy.limits().max_workspace_bytes;
        let next = self
            .workspace_bytes
            .checked_add(bytes)
            .ok_or(GeoError::MemoryExhausted { limit })?;
        if next > limit {
            return Err(GeoError::MemoryExhausted { limit });
        }
        Ok(next)
    }

    /// Admit storage and add it to the owner's checked retained allowance.
    pub(crate) fn retain_workspace(
        &mut self,
        bytes: u64,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let next = retained
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("retained workspace allowance"))?;
        self.admit_workspace(bytes)?;
        *retained = next;
        Ok(())
    }

    /// Release a dropped phase and its owner's checked retained allowance.
    pub(crate) fn release_retained_workspace(
        &mut self,
        bytes: u64,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let next = retained
            .checked_sub(bytes)
            .ok_or(GeoError::ArithmeticOverflow("retained workspace allowance"))?;
        self.release_workspace(bytes)?;
        *retained = next;
        Ok(())
    }

    /// Release admitted retained workspace after its allocation is dropped.
    ///
    /// # Errors
    ///
    /// Refuses an accounting underflow without changing the current charge.
    pub fn release_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
        let remaining = self
            .workspace_bytes
            .checked_sub(bytes)
            .filter(|remaining| *remaining >= self.retained_workspace_bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "workspace accounting underflow",
            ))?;
        self.workspace_bytes = remaining;
        Ok(())
    }

    pub(crate) fn begin(&mut self, output_elements: usize) -> Result<(), GeoError> {
        self.begin_with(output_elements, ObserverArithmetic::Floating)
    }

    /// Admit an integer-only phase under the same resource/cancellation law.
    pub(crate) fn begin_integer(&mut self, output_elements: usize) -> Result<(), GeoError> {
        self.begin_with(output_elements, ObserverArithmetic::Integer)
    }

    fn begin_with(
        &mut self,
        output_elements: usize,
        arithmetic: ObserverArithmetic,
    ) -> Result<(), GeoError> {
        self.checkpoint_with(arithmetic)?;
        let limit = self.policy.limits().max_output_elements;
        let count =
            u64::try_from(output_elements).map_err(|_| GeoError::OutputExhausted { limit })?;
        if count > limit {
            return Err(GeoError::OutputExhausted { limit });
        }
        if self.workspace_bytes != self.retained_workspace_bytes {
            return Err(GeoError::ArithmeticOverflow(
                "workspace retained across invocation",
            ));
        }
        self.work_items = self.preparation_work_items;
        self.workspace_peak = self.retained_workspace_bytes;
        Ok(())
    }
}

pub(crate) fn validate_environment() -> Result<(), GeoError> {
    purrdf_xsd::ieee::environment::check().map_err(GeoError::FloatEnvironment)
}

#[cfg(all(
    test,
    any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")
))]
mod integer_observer_tests;

#[cfg(test)]
mod tests {
    use super::{MetricContext, MetricWorkObserver};
    use crate::{ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference};

    #[test]
    fn sequential_children_share_wide_immutable_reference_operands_without_allocating() {
        let original = crate::Rat::from_int(crate::Int::one().shl(4096).add(&crate::Int::one()));
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let copied = original.clone();
        let control = window.close();
        assert!(control.allocations > 0);
        assert!(control.requested_bytes > 0);
        assert_eq!(original, copied);
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1_000_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut budget = super::PreparationBudget::new(policy);
        let ellipsoid = crate::PreparedEllipsoid::new_in_budget(
            original,
            crate::Rat::from_i64(300),
            &mut budget,
        )
        .unwrap();
        let reference = GeographicReference::new(
            ellipsoid,
            purrdf_hash::hex::Digest32::new([71; 32]),
            crate::AxisOrder::LonLat,
        );
        let baseline = reference.ellipsoid().retained_limb_bytes();
        assert!(baseline > 4096 / 8);
        let mut parent = MetricContext::new(reference, policy).unwrap();
        parent.set_retained_workspace(baseline).unwrap();
        parent.admit_workspace(17).unwrap();
        let mut surviving_reference = None;
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..64 {
            let child = parent.remaining_child().unwrap();
            assert_eq!(child.reference(), parent.reference());
            assert_eq!(child.retained_workspace_bytes(), 0);
            surviving_reference = Some(child.reference().clone());
            drop(child);
        }
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        assert_eq!(measured.retained_bytes, 0);
        assert_eq!(parent.current_workspace_bytes(), baseline + 17);
        let expected = parent.reference().ellipsoid().semimajor().clone();
        drop(parent);
        let surviving_reference = surviving_reference.unwrap();
        assert_eq!(surviving_reference.ellipsoid().semimajor(), &expected);
        // Arithmetic on a shared original still creates a fresh destination;
        // the immutable reference and its derived factors do not change.
        let changed = expected.add(&crate::Rat::one());
        assert_ne!(&changed, surviving_reference.ellipsoid().semimajor());
        assert_eq!(surviving_reference.ellipsoid().semimajor(), &expected);
    }

    #[test]
    fn nested_receipts_preserve_live_storage_peak_and_first_refusal_without_callbacks() {
        struct Receipt {
            work: u64,
            peak: u64,
            calls: u32,
            refuse: bool,
        }
        impl MetricWorkObserver for Receipt {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.work += work;
                self.peak += growth;
                self.calls += 1;
                if self.refuse && self.calls == 2 {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        for refuse in [false, true] {
            let mut context = MetricContext::wgs84().unwrap();
            context.set_preparation_work(17).unwrap();
            context.set_retained_workspace(256).unwrap();
            context.admit_workspace(11).unwrap();
            let mut receipt = Receipt {
                work: 0,
                peak: 0,
                calls: 0,
                refuse,
            };
            let mut progress = super::WorkProgress::integer(Some(&mut receipt));
            progress.context_poll(&context).unwrap();
            let child = {
                let mut nested = progress.nested(
                    context.work_items(),
                    context.current_workspace_bytes(),
                    context.workspace_peak(),
                );
                nested.charge_chunk(7, 100).map(|()| 31)
            };
            let result = progress.absorb_nested(&mut context, child);
            assert_eq!(context.work_items(), 24);
            assert_eq!(context.workspace_peak(), 367);
            assert_eq!(context.current_workspace_bytes(), 267);
            assert_eq!(context.retained_workspace_bytes(), 256);
            if refuse {
                assert_eq!(result, Err(GeoError::Cancelled));
                assert_eq!(progress.latched_error(), Some(GeoError::Cancelled));
                progress.reset();
                assert_eq!(progress.initial(), Err(GeoError::Cancelled));
            } else {
                assert_eq!(result, Ok(31));
            }
            drop(progress);
            assert_eq!(receipt.calls, 2);
            assert_eq!(receipt.work, context.work_items());
            assert_eq!(receipt.peak, context.workspace_peak());
            context.release_workspace(11).unwrap();
        }
    }

    #[test]
    fn observer_refusal_stays_latched_through_exact_cleanup_and_new_math_phases() {
        struct Refuse {
            calls: usize,
        }
        impl MetricWorkObserver for Refuse {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                self.calls += 1;
                Err(GeoError::MemoryExhausted { limit: 17 })
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            64,
            1,
        )
        .unwrap();
        let mut observer = Refuse { calls: 0 };
        let mut progress = super::WorkProgress::new(Some(&mut observer));
        let error = GeoError::MemoryExhausted { limit: 17 };
        assert_eq!(
            progress.exact(&mut context, cost, || Ok(())),
            Err(error.clone())
        );
        assert_eq!(context.current_workspace_bytes(), 0);
        assert_eq!(progress.context_poll(&context), Err(error.clone()));
        progress.reset();
        progress.math_phase(&context);
        assert_eq!(progress.initial(), Err(error.clone()));
        assert_eq!(progress.latched_error(), Some(error));
        drop(progress);
        assert_eq!(observer.calls, 1);
    }

    #[test]
    fn child_lineage_refusal_preserves_prior_observer_cancellation_and_real_peak() {
        use core::cell::Cell;

        struct Receipt<'a> {
            armed: &'a Cell<bool>,
            call_count: &'a Cell<u64>,
            work: u64,
            peak: u64,
            calls: u64,
        }
        impl MetricWorkObserver for Receipt<'_> {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.work += work;
                self.peak += growth;
                self.calls += 1;
                self.call_count.set(self.calls);
                if self.armed.get() {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }

        let reference = GeographicReference::wgs84();
        let latitude = crate::Rat::from_i64(20);
        let evaluate = |child: &mut MetricContext,
                        observer: Option<&mut dyn MetricWorkObserver>| {
            let policy = child.policy();
            let mut progress = super::WorkProgress::new(observer);
            crate::numerical::with_math(child, &mut progress, 96, 65_536, |math, progress| {
                crate::geodesic::meridian_arc_observed(
                    &latitude,
                    reference.ellipsoid(),
                    math,
                    progress,
                )
                .map_err(|error| crate::numerical::geo_math_error(&error, policy))
            })
        };
        let mut calibration = MetricContext::wgs84().unwrap();
        drop(evaluate(&mut calibration, None).unwrap());
        let child_work = calibration.work_items() + 1;
        let limit = 17 + child_work;
        let limits = ExecutionLimits {
            max_work_items: limit,
            ..ExecutionLimits::GEOMETRY
        };
        let policy = ExecutionPolicy::new(limits).unwrap();
        let make_parent = || {
            let mut context = MetricContext::new(reference.clone(), policy).unwrap();
            context.set_preparation_work(17).unwrap();
            context.set_retained_workspace(256).unwrap();
            context.admit_workspace(11).unwrap();
            context
        };
        for cancel in [false, true] {
            let mut parent = make_parent();
            let mut child = parent.remaining_child().unwrap();
            let armed = Cell::new(false);
            let call_count = Cell::new(0);
            let mut receipt = Receipt {
                armed: &armed,
                call_count: &call_count,
                work: 0,
                peak: 0,
                calls: 0,
            };
            let mut progress = super::WorkProgress::new(Some(&mut receipt));
            progress.context_poll(&parent).unwrap();
            let result = {
                let mut nested = progress.nested(
                    parent.work_items(),
                    parent.current_workspace_bytes(),
                    parent.workspace_peak(),
                );
                drop(evaluate(&mut child, Some(&mut nested)).unwrap());
                child.charge_work(1).unwrap();
                armed.set(cancel);
                nested.charge_chunk(1, 0).and_then(|()| {
                    crate::LonLat::new(crate::Rat::zero(), crate::Rat::from_i64(91)).map(|_| ())
                })
            };
            if cancel {
                assert_eq!(result, Err(GeoError::Cancelled));
            } else {
                assert!(result.as_ref().unwrap_err().is_expression_error());
            }
            assert_eq!(child.work_items(), child_work);
            assert!(child.geodesic_coefficients_owned_bytes > 0);
            assert!(
                child
                    .geodesic_coefficients
                    .as_ref()
                    .unwrap()
                    .lineage_work()
                    .unwrap()
                    > 0
            );
            // The same real child fits its numerical allowance, but settling its
            // new coefficient lineage genuinely exceeds the parent's exact cap.
            let mut control = make_parent();
            assert_eq!(
                control.absorb_child(&child),
                Err(GeoError::WorkExhausted { limit })
            );
            assert_eq!(
                progress.absorb_child_result(&mut parent, &child, result),
                Err(if cancel {
                    GeoError::Cancelled
                } else {
                    GeoError::WorkExhausted { limit }
                })
            );
            assert_eq!(parent.work_items(), limit);
            assert_eq!(parent.current_workspace_bytes(), 267);
            assert_eq!(parent.retained_workspace_bytes(), 256);
            assert!(parent.workspace_peak() > parent.current_workspace_bytes());
            let calls = call_count.get();
            if cancel {
                assert_eq!(progress.context_poll(&parent), Err(GeoError::Cancelled));
            }
            assert_eq!(call_count.get(), calls);
            drop(progress);
            assert_eq!(receipt.work, parent.work_items());
            assert_eq!(receipt.peak, parent.workspace_peak());
            assert!(calls > 2);
            drop(child);
            parent.release_workspace(11).unwrap();
            assert_eq!(parent.current_workspace_bytes(), 256);
            assert_eq!(receipt.calls, calls);
        }
    }

    #[test]
    fn live_owned_arena_receipts_cannot_be_erased_or_replaced_under_held_outputs() {
        let mut context = MetricContext::wgs84().unwrap();
        context.prepare_integer_scratch().unwrap();
        let retained = context.retained_workspace_bytes();
        assert!(context.set_retained_workspace(0).is_err());
        let value = crate::Int::one()
            .shl_in(448, context.integer_scratch().unwrap())
            .unwrap();
        assert!(matches!(
            context.prepare_integer_scratch_for(8192),
            Err(GeoError::NumericalScratch(
                purrdf_xsd::integer::LimbScratchError::Retained {
                    held_destinations: 1,
                    destinations: 768,
                }
            ))
        ));
        assert_eq!(context.retained_workspace_bytes(), retained);
        drop(value);
        context.prepare_integer_scratch_for(8192).unwrap();
        assert!(context.retained_workspace_bytes() > retained);
    }

    #[test]
    fn borrowed_child_arena_growth_charges_only_its_new_owned_heap() {
        let mut parent = MetricContext::wgs84().unwrap();
        parent.prepare_integer_scratch().unwrap();
        let parent_bytes = parent.retained_workspace_bytes();
        let parent_limbs = parent.integer_scratch().unwrap().limb_capacity();
        let mut child = parent.remaining_child().unwrap();
        assert_eq!(child.retained_workspace_bytes(), 0);
        assert_eq!(
            child.integer_scratch().unwrap().limb_capacity(),
            parent_limbs
        );
        child.prepare_integer_scratch_for(8192).unwrap();
        assert!(child.integer_scratch().unwrap().limb_capacity() > parent_limbs);
        assert_eq!(
            child.retained_workspace_bytes(),
            child.integer_scratch().unwrap().retained_bytes() as u64
        );
        assert_eq!(parent.retained_workspace_bytes(), parent_bytes);
        assert_eq!(
            parent.integer_scratch().unwrap().limb_capacity(),
            parent_limbs
        );
    }

    #[test]
    fn sequential_children_retain_only_new_coefficient_heads_from_the_observed_peak() {
        let mut parent = MetricContext::wgs84().unwrap();
        let reference = parent.reference().clone();
        let latitude = crate::Rat::from_i64(20);
        parent
            .prepare_integer_scratch_for(crate::numerical::scratch_source_bits(
                &[&latitude],
                reference.ellipsoid(),
            ))
            .unwrap();
        let original_retained = parent.retained_workspace_bytes();
        parent.admit_workspace(17).unwrap();
        let evaluate = |child: &mut MetricContext| {
            let policy = child.policy();
            let mut progress = super::WorkProgress::new(None);
            crate::numerical::with_math(child, &mut progress, 96, 65_536, |math, progress| {
                crate::geodesic::meridian_arc_observed(
                    &latitude,
                    reference.ellipsoid(),
                    math,
                    progress,
                )
                .map_err(|error| crate::numerical::geo_math_error(&error, policy))
            })
            .unwrap()
        };
        let mut first = parent.remaining_child().unwrap();
        let mut sibling = parent.remaining_child().unwrap();
        let cold = evaluate(&mut first);
        let sibling_value = evaluate(&mut sibling);
        let cold_work = first.work_items();
        let added = first.geodesic_coefficients_owned_bytes;
        assert!(added > 0);
        parent.absorb_child(&first).unwrap();
        assert_eq!(parent.retained_workspace_bytes(), original_retained + added);
        assert_eq!(
            parent.workspace_bytes,
            parent.retained_workspace_bytes() + 17
        );
        assert!(std::sync::Arc::ptr_eq(
            parent.geodesic_coefficients.as_ref().unwrap(),
            first.geodesic_coefficients.as_ref().unwrap(),
        ));
        assert!(!std::sync::Arc::ptr_eq(
            first.geodesic_coefficients.as_ref().unwrap(),
            sibling.geodesic_coefficients.as_ref().unwrap(),
        ));
        parent.absorb_child(&sibling).unwrap();
        assert_eq!(parent.retained_workspace_bytes(), original_retained + added);
        assert!(std::sync::Arc::ptr_eq(
            parent.geodesic_coefficients.as_ref().unwrap(),
            first.geodesic_coefficients.as_ref().unwrap(),
        ));
        drop(first);
        drop(sibling);
        drop(cold);
        drop(sibling_value);
        let mut second = parent.remaining_child().unwrap();
        assert_eq!(second.geodesic_coefficients_owned_bytes, 0);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let warm = evaluate(&mut second);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert!(second.work_items() < cold_work);
        let retained = parent.retained_workspace_bytes();
        parent.absorb_child(&second).unwrap();
        assert_eq!(parent.retained_workspace_bytes(), retained);
        assert_eq!(parent.workspace_bytes, retained + 17);
        drop(warm);
        drop(second);
        parent.release_workspace(17).unwrap();
        parent.begin(0).unwrap();
        assert_eq!(parent.retained_workspace_bytes(), retained);
    }

    #[test]
    fn preparation_and_evaluation_share_the_same_work_admission() {
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8;
        let policy = ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        context.set_preparation_work(5).unwrap();
        context.begin(1).unwrap();
        assert_eq!(context.work_items(), 5);
        context.charge_work(3).unwrap();
        assert!(matches!(
            context.charge_work(1),
            Err(GeoError::WorkExhausted { limit: 8 })
        ));
        assert!(context.set_preparation_work(0).is_err());
        context.begin(1).unwrap();
        assert_eq!(context.work_items(), 5);
        assert_eq!(context.policy().id(), policy.id());
    }

    #[test]
    fn external_memory_refusal_precedes_arena_allocation_and_restores_native_admission() {
        struct RejectGrowth {
            observed: u64,
        }
        impl MetricWorkObserver for RejectGrowth {
            fn charge_chunk(&mut self, _: u64, growth: u64) -> Result<(), GeoError> {
                self.observed = growth;
                if growth == 0 {
                    Ok(())
                } else {
                    Err(GeoError::MemoryExhausted { limit: 1 })
                }
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        let mut observer = RejectGrowth { observed: 0 };
        let mut progress = super::WorkProgress::new(Some(&mut observer));
        let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
        std::hint::black_box(Vec::<u8>::with_capacity(512));
        assert_eq!(instrument.close().allocations, 1);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = context.prepare_integer_scratch_for_observed(0, &mut progress);
        let measured = window.close();
        assert_eq!(result, Err(GeoError::MemoryExhausted { limit: 1 }));
        assert_eq!(measured.allocations, 0);
        assert!(observer.observed > 0);
        assert!(context.integer_scratch().is_none());
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes
        );
    }

    #[test]
    fn retained_preparation_and_transient_workspace_share_one_unchanged_policy() {
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_workspace_bytes: 8,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        context.set_retained_workspace(5).unwrap();
        context.begin(1).unwrap();
        assert_eq!(context.remaining_workspace(), 3);
        context.admit_workspace(3).unwrap();
        assert!(matches!(
            context.admit_workspace(1),
            Err(GeoError::MemoryExhausted { limit: 8 })
        ));
        assert!(context.begin(1).is_err());
        assert!(context.set_retained_workspace(0).is_err());
        context.release_workspace(3).unwrap();
        assert!(context.release_workspace(1).is_err());
        context.begin(1).unwrap();
        assert_eq!(context.workspace_peak(), 5);
        assert_eq!(context.policy().id(), policy.id());
    }

    #[test]
    fn checked_counts_never_publish_over_budget_state() {
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 2,
            max_workspace_bytes: 8,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        context.begin(1).unwrap();
        context.charge_work(2).unwrap();
        assert!(matches!(
            context.charge_work(1),
            Err(GeoError::WorkExhausted { limit: 2 })
        ));
        assert_eq!(context.work_items(), 2);
        context.admit_workspace(8).unwrap();
        assert!(matches!(
            context.admit_workspace(1),
            Err(GeoError::MemoryExhausted { limit: 8 })
        ));
        context.release_workspace(8).unwrap();
        assert_eq!(context.workspace_peak(), 8);
        context.cancel();
        assert!(matches!(context.begin(1), Err(GeoError::Cancelled)));
        context.clear_cancellation();
        context.begin(1).unwrap();
        assert_eq!(context.work_items(), 0);
    }

    #[test]
    fn borrowed_worker_admits_reference_before_copy_and_preserves_observed_receipt() {
        struct Receipt {
            work: u64,
            workspace: u64,
            cancel: bool,
        }
        impl MetricWorkObserver for Receipt {
            fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
                self.work += work;
                self.workspace += workspace;
                if self.cancel {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let reference = GeographicReference::cgcs2000();
        let policy = ExecutionPolicy::geometry();
        let mut receipt = Receipt {
            work: 0,
            workspace: 0,
            cancel: false,
        };
        let mut worker =
            MetricContext::from_reference_metered(&reference, policy, &mut receipt).unwrap();
        assert_eq!(worker.reference(), &reference);
        assert_eq!(worker.work_items(), receipt.work);
        assert_eq!(worker.workspace_peak(), receipt.workspace);
        assert!(worker.retained_workspace_bytes() <= receipt.workspace);
        let prepared = worker.preparation_work_items();
        worker.begin(1).unwrap();
        assert_eq!(worker.work_items(), prepared);
        assert_eq!(worker.policy().id(), policy.id());

        receipt.cancel = true;
        assert!(matches!(
            MetricContext::from_reference_metered(&reference, policy, &mut receipt),
            Err(GeoError::Cancelled)
        ));
        let low_work = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        receipt.work = 0;
        receipt.workspace = 0;
        assert!(matches!(
            MetricContext::from_reference_metered(&reference, low_work, &mut receipt),
            Err(GeoError::WorkExhausted { limit: 1 })
        ));
        assert_eq!((receipt.work, receipt.workspace), (0, 0));

        // Independent sequential phases carry only their new work/peak to an
        // observer that already charged the earlier source. The complete source
        // remains live while the reference constructor uses temporary scratch.
        let mut budget = super::PreparationBudget::new(policy);
        budget.retain(5, 10_000).unwrap();
        receipt.work = budget.work_items();
        receipt.workspace = budget.workspace_bytes();
        receipt.cancel = false;
        let worker = budget
            .worker_context_metered(&reference, &mut receipt)
            .unwrap();
        assert_eq!(receipt.work, worker.work_items());
        assert_eq!(receipt.workspace, worker.workspace_peak());
        assert_eq!(budget.workspace_bytes(), worker.retained_workspace_bytes());
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            64,
            3,
        )
        .unwrap();
        budget.exact_metered(cost, || Ok(()), &mut receipt).unwrap();
        assert_eq!(receipt.work, budget.work_items());
        assert_eq!(
            receipt.workspace,
            worker
                .workspace_peak()
                .max(budget.workspace_bytes() + cost.workspace_bytes)
        );
        let original = budget;
        let evaluated = std::cell::Cell::new(false);
        receipt.cancel = true;
        assert_eq!(
            budget.exact_metered(
                cost,
                || {
                    evaluated.set(true);
                    Ok(())
                },
                &mut receipt
            ),
            Err(GeoError::Cancelled)
        );
        assert!(!evaluated.get());
        assert_eq!(budget, original);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refusal = budget.worker_context_metered(&reference, &mut receipt);
        let allocations = window.close();
        assert!(matches!(refusal, Err(GeoError::Cancelled)));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(budget, original);
    }
}
