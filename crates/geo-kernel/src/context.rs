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

use crate::{ExecutionPolicy, GeoError, GeographicReference};

/// The initial numerical working set, capped by the caller's slot ceiling.
/// Coefficient producers separately request their complete actual order,
/// including live outer coefficients when a centered expansion is needed.
pub(crate) fn scratch_destination_count(limits: &crate::ExecutionLimits) -> usize {
    limits
        .max_scratch_destinations
        .min(crate::ExecutionLimits::GEOMETRY.max_scratch_destinations) as usize
}

pub(crate) fn scratch_limb_capacity(
    source_bits: u64,
    max_precision_bits: u32,
) -> Result<usize, GeoError> {
    let bits = u64::from(max_precision_bits)
        .checked_mul(4)
        .and_then(|bits| bits.checked_add(2048))
        .and_then(|bits| {
            source_bits
                .checked_mul(2)
                .and_then(|source| bits.checked_add(source))
        })
        .ok_or(GeoError::ArithmeticOverflow("integer scratch source width"))?;
    usize::try_from(bits.div_ceil(64))
        .map_err(|_| GeoError::ArithmeticOverflow("integer scratch target width"))
}

mod native_memory;
mod output;
mod preparation;
pub(crate) mod retained;
pub(crate) use native_memory::{native_memory, with_policy_memory};
pub use output::MaterializedOutputReceipt;
pub use preparation::{PreparationBudget, PreparedSourceReceipt};
pub(crate) use retained::{
    NativeDetached, PersistentNumeric, Produced, RetainedBuilder, persistent_numeric,
    persistent_numeric_bytes, persistent_numeric_from, persistent_numeric_owned,
    produce_persistent_numeric, produce_persistent_numeric_from,
};
pub use retained::{
    Retained, RetainedAllocationProducer, RetainedProductionFailure, RetainedValues,
    retain_failure_with_storage, retain_produced_with_storage,
};

/// Restricted external work/cancellation capability, without geometry or evaluator reentry.
///
/// A numerical operation invokes this only between bounded chunks with every
/// floating control guard dropped, then revalidates the thread environment.
/// Integer-only assignment, encoding and source-admission chunks use the same
/// charging capability without inspecting floating-point control state.
/// `workspace_growth` counts additional admitted retained/scratch peak bytes.
pub trait MetricWorkObserver {
    /// Publish a failed producer's original cause after destroying its working
    /// locals. Dynamic cause fields retain their original account; a failed
    /// bounded producer retains no reusable working buffers.
    #[must_use]
    fn retain_failure(&mut self, error: GeoError) -> GeoError {
        if self.allocation().is_some() {
            drop(error);
            return GeoError::InvalidNativeStorage(
                "native observer cannot retain its original failure cause",
            );
        }
        let admitted = match error.native_text_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return GeoError::Storage(error),
        };
        retain_failure_with_storage(
            error,
            &mut purrdf_lex::allocation::Resident,
            purrdf_lex::allocation::Resident,
            admitted,
        )
    }
    /// Run an original native producer while lending its live storage separately
    /// from its work/numeric factory capability. These are two stack borrows of
    /// the same request, not separate parser or evaluator implementations.
    ///
    /// # Errors
    /// A bounded observer must implement the split at its concrete owner home.
    fn native_storage(
        mut self: &mut Self,
        producer: &mut dyn NativeStorageProducer,
    ) -> Result<(), GeoError> {
        if self.allocation().is_some() {
            return Err(GeoError::InvalidNativeStorage(
                "native allocation observer cannot lend its original storage and work capability",
            ));
        }
        let mut resident = purrdf_lex::allocation::Resident;
        producer.produce(
            &mut purrdf_lex::allocation::Memory::new(&mut resident),
            &mut self,
        )
    }
    /// Exact control layout of this observer's fresh retained allocation owner.
    /// This is the same concrete storage type used by `retain_produced`.
    fn retained_owner_layout(&self) -> core::alloc::Layout {
        purrdf_lex::allocation::SharedOwned::<()>::allocation_layout::<
            purrdf_lex::allocation::Resident,
        >()
    }

    /// Produce an immutable preparation under a fresh original allocation owner.
    /// The concrete outer control and lease are admitted before production;
    /// every failed production destroys its pending payload before the owner.
    ///
    /// # Errors
    /// Preserves exact native physical or numerical refusal.
    fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        retain_produced_with_storage(control, producer, purrdf_lex::allocation::Resident)
            .map_err(RetainedProductionFailure::into_error)
    }

    /// Borrow this native producer's original physical allocation account.
    /// The total is the already admitted requested layout of its live payloads.
    /// Returned source/result payloads must remain under this account until
    /// destruction or ownership-carrying publication. `None` denotes the
    /// existing caller-owned resident path, never a bounded-query substitute.
    fn allocation(&mut self) -> Option<(&mut dyn purrdf_lex::allocation::Admission, usize)> {
        None
    }

    /// Retain this producer's already admitted account with an immutable
    /// result. Control allocation precedes transfer; no existing payload is
    /// measured, copied, or moved out before its destination is allocated.
    ///
    /// # Errors
    /// A bounded observer must support transfer of its original live account;
    /// the resident default never substitutes for such a bounded account.
    fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        if self.allocation().is_some() {
            return Err(GeoError::InvalidNativeStorage(
                "native allocation observer cannot retain its original account",
            ));
        }
        purrdf_lex::allocation::SharedOwned::retain_current(
            &mut purrdf_lex::allocation::Resident,
            purrdf_lex::allocation::Resident,
            0,
            control,
        )
        .map_err(GeoError::Storage)
    }

    /// Publish immutable text under the original native allocation owner.
    /// Shallow clones retain that owner rather than copying or shedding it.
    /// # Errors
    /// Preserves exact physical allocator or admission refusal.
    fn retain_text(&mut self, text: &str) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        crate::carrier::resident_shared_text(text)
    }

    /// Charge completed bounded work and newly admitted workspace growth.
    ///
    /// # Errors
    ///
    /// A refusal stops the operation before another bounded chunk runs.
    fn charge_chunk(&mut self, work_items: u64, workspace_growth: u64) -> Result<(), GeoError>;

    /// Exact reusable integer-arena layout including this producer's original
    /// owner/control type, before either its policy or physical allocation.
    /// # Errors
    /// Refuses a layout that cannot be represented by the target.
    fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
        LimbScratch::required_bytes(buffers, limbs).map_err(GeoError::NumericalScratch)
    }

    /// Produce the quoted arena through its original fallible native home.
    /// Implementations retain the original physical grant with the arena.
    /// # Errors
    /// Preserves exact capacity, physical allocator or admission refusal.
    fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<LimbScratch, GeoError> {
        LimbScratch::new(buffers, limbs).map_err(GeoError::NumericalScratch)
    }

    /// Exact immutable binary64 table layout for this producer's owner type.
    #[must_use]
    fn binary64_layout(&self) -> usize {
        PreparedBinary64::required_bytes_with_storage::<purrdf_lex::allocation::Resident>()
    }

    /// Produce the original generated table while retaining its physical owner.
    /// Arithmetic uses the context's already admitted integer arena.
    /// # Errors
    /// Preserves arithmetic, physical allocator, and original admission refusal.
    fn prepare_binary64(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<PreparedBinary64, MathError> {
        math.prepare_binary64()
    }

    /// Exact coefficient/generation/control layout for the paired native pool.
    /// # Errors
    /// Refuses a layout that cannot be represented by the target.
    fn taylor_scratch_layout(&self, coefficients: usize, jets: usize) -> Result<usize, GeoError> {
        TaylorScratch::required_bytes_with_storage::<purrdf_lex::allocation::Resident>(
            coefficients,
            jets,
        )
        .ok_or(GeoError::Storage(
            purrdf_lex::allocation::StorageError::SizeOverflow,
        ))
    }

    /// Produce the quoted Taylor pool without borrowing storage beyond this call.
    /// The returned immutable pool retains its concrete original owner.
    /// # Errors
    /// Preserves exact native physical or initializer refusal.
    fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<TaylorScratch, GeoError> {
        TaylorScratch::new_with_storage(coefficients, jets, zero, purrdf_lex::allocation::Resident)
            .map_err(|(error, _)| GeoError::NumericalTaylorScratch(error))
    }
}

/// One native producer borrowing the original physical account independently
/// of work callbacks. Its returned private payload remains covered by that
/// account until destruction or immutable ownership-carrying publication.
pub trait NativeStorageProducer {
    /// Produce through original shared allocation and numerical homes.
    /// # Errors
    /// Preserves the first original source, numerical or physical refusal.
    fn produce(
        &mut self,
        memory: &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError>;
}

impl<T: MetricWorkObserver + ?Sized> MetricWorkObserver for &mut T {
    fn retain_failure(&mut self, error: GeoError) -> GeoError {
        (**self).retain_failure(error)
    }
    fn native_storage(&mut self, producer: &mut dyn NativeStorageProducer) -> Result<(), GeoError> {
        (**self).native_storage(producer)
    }
    fn allocation(&mut self) -> Option<(&mut dyn purrdf_lex::allocation::Admission, usize)> {
        (**self).allocation()
    }
    fn retained_owner_layout(&self) -> core::alloc::Layout {
        (**self).retained_owner_layout()
    }
    fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        (**self).retain_current(control)
    }
    fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        (**self).retain_produced(control, producer)
    }
    fn retain_text(&mut self, text: &str) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        (**self).retain_text(text)
    }
    fn charge_chunk(&mut self, work: u64, storage: u64) -> Result<(), GeoError> {
        (**self).charge_chunk(work, storage)
    }
    fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
        (**self).integer_scratch_layout(buffers, limbs)
    }
    fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<LimbScratch, GeoError> {
        (**self).prepare_integer_scratch(buffers, limbs)
    }
    fn binary64_layout(&self) -> usize {
        (**self).binary64_layout()
    }
    fn prepare_binary64(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<PreparedBinary64, MathError> {
        (**self).prepare_binary64(math)
    }
    fn taylor_scratch_layout(&self, coefficients: usize, jets: usize) -> Result<usize, GeoError> {
        (**self).taylor_scratch_layout(coefficients, jets)
    }
    fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<TaylorScratch, GeoError> {
        (**self).prepare_taylor_scratch(coefficients, jets, zero)
    }
}

/// Continue one governed request after preparation has already charged the
/// same observer. Context limits still include that work and retained storage;
/// this adapter suppresses only duplicate external charges for the receipt.
pub struct MetricWorkContinuation<'a> {
    observer: &'a mut dyn MetricWorkObserver,
    remaining_work: u64,
    remaining_workspace: u64,
    received_work: u64,
    received_workspace: u64,
    failure: Option<GeoError>,
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
            received_work: 0,
            received_workspace: 0,
            failure: None,
        }
    }
}

impl MetricWorkObserver for MetricWorkContinuation<'_> {
    fn retain_failure(&mut self, error: GeoError) -> GeoError {
        self.observer.retain_failure(error)
    }
    fn native_storage(&mut self, producer: &mut dyn NativeStorageProducer) -> Result<(), GeoError> {
        // The physical split borrows the original account, but its producer
        // must still observe this continuation's work/cancellation boundary.
        struct Continued<'a> {
            producer: &'a mut dyn NativeStorageProducer,
            work: u64,
            workspace: u64,
            received_work: u64,
            received_workspace: u64,
            failure: Option<GeoError>,
        }
        impl NativeStorageProducer for Continued<'_> {
            fn produce(
                &mut self,
                memory: &mut purrdf_lex::allocation::Memory<
                    '_,
                    dyn purrdf_lex::allocation::Admission + '_,
                >,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<(), GeoError> {
                let mut continuation =
                    MetricWorkContinuation::new(observer, self.work, self.workspace);
                continuation.failure = self.failure.take();
                let result = self.producer.produce(memory, &mut continuation);
                self.work = continuation.remaining_work;
                self.workspace = continuation.remaining_workspace;
                self.received_work = continuation.received_work;
                self.received_workspace = continuation.received_workspace;
                self.failure = continuation.failure;
                result
            }
        }
        let mut continued = Continued {
            producer,
            work: self.remaining_work,
            workspace: self.remaining_workspace,
            received_work: 0,
            received_workspace: 0,
            failure: self.failure.clone(),
        };
        let result = self.observer.native_storage(&mut continued);
        self.remaining_work = continued.work;
        self.remaining_workspace = continued.workspace;
        self.received_work = self
            .received_work
            .checked_add(continued.received_work)
            .ok_or(GeoError::ArithmeticOverflow("continued native work"))?;
        self.received_workspace = self
            .received_workspace
            .checked_add(continued.received_workspace)
            .ok_or(GeoError::ArithmeticOverflow("continued native peak"))?;
        if self.failure.is_none() {
            self.failure = continued.failure;
        }
        result
    }
    fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        self.observer.retain_current(control)
    }
    fn retained_owner_layout(&self) -> core::alloc::Layout {
        self.observer.retained_owner_layout()
    }

    fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        self.observer.retain_produced(control, producer)
    }

    fn allocation(&mut self) -> Option<(&mut dyn purrdf_lex::allocation::Admission, usize)> {
        self.observer.allocation()
    }

    fn retain_text(&mut self, text: &str) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        self.observer.retain_text(text)
    }

    fn charge_chunk(&mut self, work_items: u64, workspace_growth: u64) -> Result<(), GeoError> {
        if let Some(original) = &self.failure {
            return Err(original.clone());
        }
        self.received_work = self
            .received_work
            .checked_add(work_items)
            .ok_or(GeoError::ArithmeticOverflow("continued work receipt"))?;
        self.received_workspace = self
            .received_workspace
            .checked_add(workspace_growth)
            .ok_or(GeoError::ArithmeticOverflow("continued workspace receipt"))?;
        let work = work_items.min(self.remaining_work);
        let workspace = workspace_growth.min(self.remaining_workspace);
        self.remaining_work -= work;
        self.remaining_workspace -= workspace;
        let result = self
            .observer
            .charge_chunk(work_items - work, workspace_growth - workspace);
        if let Err(original) = &result {
            self.failure = Some(original.clone());
        }
        result
    }

    fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
        self.observer.integer_scratch_layout(buffers, limbs)
    }
    fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<LimbScratch, GeoError> {
        self.observer.prepare_integer_scratch(buffers, limbs)
    }
    fn taylor_scratch_layout(&self, coefficients: usize, jets: usize) -> Result<usize, GeoError> {
        self.observer.taylor_scratch_layout(coefficients, jets)
    }

    fn binary64_layout(&self) -> usize {
        self.observer.binary64_layout()
    }

    fn prepare_binary64(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<PreparedBinary64, MathError> {
        self.observer.prepare_binary64(math)
    }
    fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<TaylorScratch, GeoError> {
        self.observer
            .prepare_taylor_scratch(coefficients, jets, zero)
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
    prepared_coefficients: Option<Retained<crate::geodesic::WorkerCoefficientCache>>,
    prepared_coefficients_added_bytes: u64,
    error: Option<GeoError>,
}

impl<'a> WorkProgress<'a> {
    pub(crate) fn retain_failure(&mut self, error: GeoError) -> GeoError {
        match self.observer.as_deref_mut() {
            Some(observer) => observer.retain_failure(error),
            None => {
                let admitted = match error.native_text_bytes() {
                    Ok(bytes) => bytes,
                    Err(error) => return GeoError::Storage(error),
                };
                retain_failure_with_storage(
                    error,
                    &mut purrdf_lex::allocation::Resident,
                    purrdf_lex::allocation::Resident,
                    admitted,
                )
            }
        }
    }
    pub(crate) fn with_native_storage<T>(
        &mut self,
        body: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
            &mut dyn MetricWorkObserver,
        ) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        struct Producer<F, T> {
            body: Option<F>,
            value: Option<T>,
            work: u64,
            peak: u64,
            failure: Option<GeoError>,
        }
        impl<F, T> NativeStorageProducer for Producer<F, T>
        where
            F: FnOnce(
                &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
                &mut dyn MetricWorkObserver,
            ) -> Result<T, GeoError>,
        {
            fn produce(
                &mut self,
                memory: &mut purrdf_lex::allocation::Memory<
                    '_,
                    dyn purrdf_lex::allocation::Admission + '_,
                >,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<(), GeoError> {
                let mut continuation = MetricWorkContinuation::new(observer, self.work, self.peak);
                let result =
                    self.body.take().expect("one native storage body")(memory, &mut continuation);
                self.work = self.work.max(continuation.received_work);
                self.peak = self.peak.max(continuation.received_workspace);
                self.failure = continuation.failure;
                self.value = Some(result?);
                Ok(())
            }
        }
        let mut producer = Producer {
            body: Some(body),
            value: None,
            work: self.work,
            peak: self.peak,
            failure: None,
        };
        let result = match self.observer.as_deref_mut() {
            Some(observer) => observer.native_storage(&mut producer),
            None => {
                struct Unmetered;
                impl MetricWorkObserver for Unmetered {
                    fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                        Ok(())
                    }
                }
                Unmetered.native_storage(&mut producer)
            }
        };
        self.work = producer.work;
        self.peak = producer.peak;
        if self.error.is_none() {
            self.error = producer.failure;
        }
        result?;
        Ok(producer.value.expect("completed native storage body"))
    }
    pub(crate) fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        match self.observer.as_deref_mut() {
            Some(observer) => observer.retain_current(control),
            None => purrdf_lex::allocation::SharedOwned::retain_current(
                &mut purrdf_lex::allocation::Resident,
                purrdf_lex::allocation::Resident,
                0,
                control,
            )
            .map_err(GeoError::Storage),
        }
    }
    pub(crate) fn retained_owner_layout(&self) -> core::alloc::Layout {
        self.observer.as_deref().map_or_else(
            purrdf_lex::allocation::SharedOwned::<()>::allocation_layout::<
                purrdf_lex::allocation::Resident,
            >,
            MetricWorkObserver::retained_owner_layout,
        )
    }

    pub(crate) fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        match self.observer.as_deref_mut() {
            Some(observer) => observer.retain_produced(control, producer),
            None => {
                retain_produced_with_storage(control, producer, purrdf_lex::allocation::Resident)
                    .map_err(RetainedProductionFailure::into_error)
            }
        }
    }

    /// Construct a native cause before publishing its original immutable owner.
    /// The factory's diagnostic buffers use the same original allocation homes
    /// as retained preparation payloads; no rendered copy leaves that account.
    pub(crate) fn retained_error(
        &mut self,
        make: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        ) -> Result<GeoError, purrdf_lex::allocation::StorageError>,
    ) -> GeoError {
        if let Some(error) = self.latched_error() {
            return error;
        }
        struct FailureProducer<F>(Option<F>);
        impl<F> RetainedAllocationProducer for FailureProducer<F>
        where
            F: FnOnce(
                &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
            ) -> Result<GeoError, purrdf_lex::allocation::StorageError>,
        {
            fn produce(
                &mut self,
                memory: &mut purrdf_lex::allocation::Memory<
                    '_,
                    dyn purrdf_lex::allocation::Admission + '_,
                >,
            ) -> Result<(), GeoError> {
                let make = self.0.take().expect("one native diagnostic factory");
                Err(make(memory).unwrap_or_else(GeoError::Storage))
            }

            fn abort(&mut self) {
                drop(self.0.take());
            }
        }
        match self.retain_produced(
            core::alloc::Layout::new::<()>(),
            &mut FailureProducer(Some(make)),
        ) {
            Err(error) => error,
            Ok(original) => {
                drop(original);
                GeoError::ArithmeticOverflow("native diagnostic producer returned no cause")
            }
        }
    }

    pub(crate) fn allocation(
        &mut self,
    ) -> Option<(&mut dyn purrdf_lex::allocation::Admission, usize)> {
        self.observer
            .as_deref_mut()
            .and_then(MetricWorkObserver::allocation)
    }

    /// Resume the producer's original account at its current live layout.
    /// Resident callers describe their existing buffers; that descriptor is
    /// never substituted for the bounded producer's original account total.
    pub(crate) fn with_storage<T, E>(
        &mut self,
        resident_bytes: usize,
        body: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        ) -> Result<T, E>,
    ) -> Result<T, E> {
        use purrdf_lex::allocation::{Memory, Resident};
        match self.allocation() {
            Some((storage, admitted)) => body(&mut Memory::resume(storage, admitted)),
            None => {
                let mut resident = Resident;
                body(&mut Memory::resume(&mut resident, resident_bytes))
            }
        }
    }

    pub(crate) fn retain_text(
        &mut self,
        text: &str,
    ) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        match self.observer.as_deref_mut() {
            Some(observer) => observer.retain_text(text),
            None => crate::carrier::resident_shared_text(text),
        }
    }

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

    pub(crate) fn integer_scratch_layout(
        &self,
        buffers: usize,
        limbs: usize,
    ) -> Result<usize, GeoError> {
        match &self.observer {
            Some(observer) => observer.integer_scratch_layout(buffers, limbs),
            None => LimbScratch::required_bytes(buffers, limbs).map_err(GeoError::NumericalScratch),
        }
    }

    pub(crate) fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<LimbScratch, GeoError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = match self.observer.as_deref_mut() {
            Some(observer) => observer.prepare_integer_scratch(buffers, limbs),
            None => LimbScratch::new(buffers, limbs).map_err(GeoError::NumericalScratch),
        };
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub(crate) fn binary64_layout(&self) -> usize {
        self.observer.as_ref().map_or_else(
            PreparedBinary64::required_bytes_with_storage::<purrdf_lex::allocation::Resident>,
            MetricWorkObserver::binary64_layout,
        )
    }

    pub(crate) fn prepare_binary64(
        &mut self,
        math: &mut CoordinateMath,
        policy: ExecutionPolicy,
    ) -> Result<PreparedBinary64, MathError> {
        if self.error.is_some() {
            return Err(MathError::Cancelled);
        }
        if math.prepared_binary64().is_none() {
            let bytes = self.binary64_layout();
            math.reserve_workspace(bytes)?;
            let observed = self.math_poll(math);
            math.release_workspace(bytes)?;
            observed?;
        }
        let result = self.forward_binary64(math);
        if let Err(error) = &result {
            self.error = Some(crate::numerical::geo_math_error(error, policy));
        }
        result
    }

    pub(crate) fn forward_binary64(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<PreparedBinary64, MathError> {
        if self.error.is_some() {
            return Err(MathError::Cancelled);
        }
        match self.observer.as_deref_mut() {
            Some(observer) => observer.prepare_binary64(math),
            None => math.prepare_binary64(),
        }
    }

    pub(crate) fn taylor_scratch_layout(
        &self,
        coefficients: usize,
        jets: usize,
    ) -> Result<usize, GeoError> {
        match &self.observer {
            Some(observer) => observer.taylor_scratch_layout(coefficients, jets),
            None => TaylorScratch::required_bytes_with_storage::<purrdf_lex::allocation::Resident>(
                coefficients,
                jets,
            )
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            )),
        }
    }

    pub(crate) fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<TaylorScratch, GeoError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = match self.observer.as_deref_mut() {
            Some(observer) => observer.prepare_taylor_scratch(coefficients, jets, zero),
            None => TaylorScratch::new_with_storage(
                coefficients,
                jets,
                zero,
                purrdf_lex::allocation::Resident,
            )
            .map_err(|(error, _)| GeoError::NumericalTaylorScratch(error)),
        };
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub(crate) fn taylor_factory() -> purrdf_xsd::math::TaylorScratchFactory<Self> {
        purrdf_xsd::math::TaylorScratchFactory::new(
            |coefficients, jets, progress| {
                let result = progress.taylor_scratch_layout(coefficients, jets);
                result.map_err(|error| match error {
                    GeoError::Storage(error) => MathError::Storage(error),
                    _ => MathError::WorkspaceExhausted,
                })
            },
            |coefficients, jets, zero, progress| {
                if progress.error.is_some() {
                    return Err(MathError::Cancelled);
                }
                let result = progress.prepare_taylor_scratch(coefficients, jets, zero);
                result.map_err(|error| {
                    progress.error = Some(error);
                    MathError::Cancelled
                })
            },
        )
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
        self.exact_with(context, cost, |context, _| evaluate(context))
    }

    /// The original scoped exact admission with native ownership available to
    /// its field factory. Work, callback and refusal ordering stay in one body.
    pub(crate) fn exact_with<T>(
        &mut self,
        context: &mut MetricContext,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce(&mut MetricContext, &mut Self) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        // Reject an inadmissible operation before opening scratch that its body
        // cannot use. The original checked counters supply both preflights;
        // neither changes the live or observed receipt.
        self.preflight_exact(context, cost, 0)?;
        context.admit_workspace(cost.workspace_bytes)?;
        let result = (|| {
            context.charge_work(cost.work_items)?;
            self.context_poll(context)?;
            let result = evaluate(context, self);
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
    ) -> Option<&Retained<crate::geodesic::WorkerCoefficientCache>> {
        self.prepared_coefficients.as_ref()
    }

    pub(crate) fn retain_prepared_coefficients(
        &mut self,
        cache: Retained<crate::geodesic::WorkerCoefficientCache>,
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
    ) -> (
        Option<Retained<crate::geodesic::WorkerCoefficientCache>>,
        u64,
    ) {
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

    /// Move the first original native cause through a numerical error seam.
    /// The outer phase takes this typed owner instead of rendering or copying it.
    pub(crate) fn math_failure(&mut self, error: GeoError) -> MathError {
        if self.error.is_none() {
            self.error = Some(error);
        }
        MathError::Cancelled
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

    pub(crate) fn settle_result<T>(
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
    fn retain_failure(&mut self, error: GeoError) -> GeoError {
        self.parent.retain_failure(error)
    }
    fn native_storage(&mut self, producer: &mut dyn NativeStorageProducer) -> Result<(), GeoError> {
        self.parent
            .with_native_storage(|memory, observer| producer.produce(memory, observer))
    }
    fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        self.parent.retain_current(control)
    }
    fn retained_owner_layout(&self) -> core::alloc::Layout {
        self.parent.retained_owner_layout()
    }

    fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        self.parent.retain_produced(control, producer)
    }

    fn allocation(&mut self) -> Option<(&mut dyn purrdf_lex::allocation::Admission, usize)> {
        self.parent.allocation()
    }

    fn retain_text(&mut self, text: &str) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        self.parent.retain_text(text)
    }

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

    fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
        self.parent.integer_scratch_layout(buffers, limbs)
    }

    fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<LimbScratch, GeoError> {
        self.parent.prepare_integer_scratch(buffers, limbs)
    }

    fn binary64_layout(&self) -> usize {
        self.parent.binary64_layout()
    }

    fn prepare_binary64(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<PreparedBinary64, MathError> {
        self.parent.forward_binary64(math)
    }

    fn taylor_scratch_layout(&self, coefficients: usize, jets: usize) -> Result<usize, GeoError> {
        self.parent.taylor_scratch_layout(coefficients, jets)
    }

    fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<TaylorScratch, GeoError> {
        self.parent.prepare_taylor_scratch(coefficients, jets, zero)
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
    materialized_outputs: RetainedBuilder<MaterializedOutputReceipt>,
    workspace_peak: u64,
    cancelled: bool,
    arithmetic: Option<PreparedBinary64>,
    integer_scratch: Option<LimbScratch>,
    integer_scratch_owned: bool,
    integer_scratch_reusable: bool,
    taylor_scratch: Option<TaylorScratch>,
    taylor_scratch_owned_bytes: u64,
    geodesic_coefficients: Option<Retained<crate::geodesic::WorkerCoefficientCache>>,
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
            materialized_outputs: RetainedBuilder::default(),
            workspace_peak: 0,
            cancelled: false,
            arithmetic: None,
            integer_scratch: None,
            integer_scratch_owned: false,
            integer_scratch_reusable: false,
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
        let mut progress = WorkProgress::new(observer);
        let retained = persistent_numeric_bytes(reference, &progress)?;
        let peak = retained;
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
        progress.charge_counts_with(cost.work_items, peak, ObserverArithmetic::Floating)?;
        let reference = produce_persistent_numeric_from(reference, &mut progress)?.into_value();
        let mut context = Self::new(reference, policy)?;
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

    /// Check an original carrier declaration against this worker's reference.
    /// Both borrowed exact fields and a mismatch's canonical IDs use their
    /// native admitted producers; no detached diagnostic text is constructed.
    ///
    /// # Errors
    /// Preserves work, scratch, physical allocation and reference mismatch errors.
    pub fn require_reference(&mut self, reference: &GeographicReference) -> Result<(), GeoError> {
        let mut progress = WorkProgress::integer(None);
        if crate::numerical::reference_matches(reference, self, &mut progress)? {
            Ok(())
        } else {
            Err(crate::numerical::missing_operation(
                reference,
                self,
                &mut progress,
            )?)
        }
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
        let mut progress = WorkProgress::new(observer);
        let bytes = persistent_numeric_bytes(&self.reference, &progress)?;
        let retained = self
            .retained_workspace_bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("prepared reference storage"))?;
        self.admit_workspace(bytes)?;
        let result = crate::numerical::reference_clone(self, &mut progress);
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
            return Err(GeoError::retained_failure(
                crate::SourcePreparationError::ChildBaseline {
                    absorbing: false,
                    reason: if work > self.preparation_work_items {
                        crate::PreparationBaselineReason::Work
                    } else {
                        crate::PreparationBaselineReason::Workspace
                    },
                    requested_work: work,
                    requested_workspace: storage,
                    available_work: self.preparation_work_items,
                    available_workspace: self.retained_workspace_bytes,
                },
                GeoError::SourcePreparation,
                None,
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
            return Err(GeoError::retained_failure(
                crate::SourcePreparationError::ChildBaseline {
                    absorbing: true,
                    reason: if original_work > child.preparation_work_items() {
                        crate::PreparationBaselineReason::Work
                    } else {
                        crate::PreparationBaselineReason::Workspace
                    },
                    requested_work: original_work,
                    requested_workspace: original_storage,
                    available_work: child.preparation_work_items(),
                    available_workspace: child.retained_workspace_bytes(),
                },
                GeoError::SourcePreparation,
                None,
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
                .is_some_and(|current| Retained::ptr_eq(current, coefficients))
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
    ) -> Option<&Retained<crate::geodesic::WorkerCoefficientCache>> {
        self.geodesic_coefficients.as_ref()
    }

    pub(crate) fn zero_conversion_bound(&self) -> Option<&crate::Rat> {
        self.zero_conversion_bound.as_ref()
    }

    pub(crate) fn retain_zero_conversion_bound(
        &mut self,
        value: PersistentNumeric<crate::Rat>,
    ) -> Result<(), GeoError> {
        let bytes = value.workspace_bytes();
        // The immutable factory already admitted both limbs and their original
        // owner before allocation. Move that live allowance into preparation;
        // neither allocate a resident copy nor charge the same output twice.
        self.promote_admitted_workspace(bytes)?;
        self.zero_conversion_bound = Some(value.into_value());
        self.zero_conversion_bound_owned_bytes = bytes;
        Ok(())
    }

    pub(crate) fn retain_geodesic_coefficients(
        &mut self,
        coefficients: Retained<crate::geodesic::WorkerCoefficientCache>,
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
        // A completed numerical preparation keeps its reusable worker arena
        // alongside the immutable coefficient cache. Failed attempts never
        // publish this cache and therefore do not acquire this retention law.
        if self.integer_scratch.is_some() {
            self.integer_scratch_reusable = true;
        }
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
        if let Some(existing) = &self.integer_scratch {
            return Err(GeoError::retained_failure(
                crate::ContextConfigError::ScratchInstalled {
                    existing_destinations: existing.destination_capacity(),
                    existing_limbs: existing.limb_capacity(),
                    incoming_destinations: scratch.destination_capacity(),
                    incoming_limbs: scratch.limb_capacity(),
                },
                GeoError::ContextConfiguration,
                None,
            ));
        }
        if scratch.destination_capacity() > self.policy.limits().max_scratch_destinations as usize {
            return Err(GeoError::retained_failure(
                crate::ContextConfigError::BorrowedDestinations {
                    requested: scratch.destination_capacity(),
                    limit: self.policy.limits().max_scratch_destinations,
                },
                GeoError::ContextConfiguration,
                None,
            ));
        }
        self.integer_scratch = Some(scratch);
        self.integer_scratch_owned = false;
        Ok(())
    }

    /// Prepare the initial numerical working set, within the caller's ceilings.
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
        self.prepare_integer_scratch_with_capacity(source_bits, self.initial_scratch_destinations())
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
        self.prepare_integer_scratch_observed(
            source_bits,
            buffers,
            self.initial_scratch_precision(),
            None,
        )?;
        self.integer_scratch_reusable = true;
        Ok(())
    }

    pub(crate) fn prepare_integer_scratch_for_observed(
        &mut self,
        source_bits: u64,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        self.prepare_integer_scratch_observed(
            source_bits,
            self.initial_scratch_destinations(),
            self.initial_scratch_precision(),
            Some(progress),
        )
    }

    fn initial_scratch_precision(&self) -> u32 {
        self.policy
            .limits()
            .max_precision_bits
            .min(crate::ExecutionLimits::GEOMETRY.max_precision_bits)
    }

    fn initial_scratch_destinations(&self) -> usize {
        scratch_destination_count(self.policy.limits())
    }

    /// Admit the attempted proof's working set before constructing arithmetic.
    /// A resource ceiling is not a request to allocate its maximum. Wider proof
    /// attempts grow here, with the old arena still charged until replacement.
    pub(crate) fn prepare_integer_scratch_at_precision_observed(
        &mut self,
        source_bits: u64,
        precision_bits: u32,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        if precision_bits > self.policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: self.policy.limits().max_precision_bits,
            });
        }
        let precision_bits = precision_bits.max(self.initial_scratch_precision());
        let buffers = scratch_destination_count(self.policy.limits());
        self.prepare_integer_scratch_observed(source_bits, buffers, precision_bits, Some(progress))
    }

    /// Keep an explicitly completed preparation's worker arena across calls.
    /// Transient exact operations still release their unused automatic arena.
    pub(crate) fn retain_prepared_integer_scratch(&mut self) -> Result<(), GeoError> {
        if self.integer_scratch.is_none() {
            return Err(GeoError::InvalidNativeStorage(
                "completed preparation has no admitted worker arena",
            ));
        }
        self.integer_scratch_reusable = true;
        Ok(())
    }

    pub(crate) fn prepare_integer_scratch_for_cost_observed(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let limbs = cost
            .destination_limbs()
            .ok_or(GeoError::ArithmeticOverflow(
                "exact physical destination width",
            ))?;
        if limbs <= 3 {
            return Ok(());
        }
        // A final exact proof already has a complete intermediate-width bound.
        // Reapplying original-source growth to its precision-grid denominator
        // would unnecessarily replace an adequate arena while that proof is live.
        if self
            .integer_scratch
            .as_ref()
            .is_some_and(|scratch| scratch.limb_capacity() as u64 >= limbs)
        {
            return Ok(());
        }
        self.prepare_integer_scratch_for_observed(cost.output_bits, progress)
    }

    /// Admit a producer's complete requested shape between numerical attempts.
    /// The failed attempt's values must already have died. Resource/cancellation
    /// failures and a caller slot ceiling remain original operational failures.
    pub(crate) fn grow_requested_integer_scratch(
        &mut self,
        error: &GeoError,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        let GeoError::NumericalScratch(LimbScratchError::Destinations { required, admitted }) =
            error.original()
        else {
            return Ok(false);
        };
        if required <= admitted
            || *required > self.policy.limits().max_scratch_destinations as usize
            || self
                .integer_scratch
                .as_ref()
                .is_some_and(|scratch| *required <= scratch.destination_capacity())
        {
            return Ok(false);
        }
        self.prepare_integer_scratch_observed(
            0,
            *required,
            self.initial_scratch_precision(),
            Some(progress),
        )?;
        Ok(true)
    }

    fn prepare_integer_scratch_observed(
        &mut self,
        source_bits: u64,
        buffers: usize,
        precision_bits: u32,
        progress: Option<&mut WorkProgress<'_>>,
    ) -> Result<(), GeoError> {
        self.checkpoint()?;
        if buffers > self.policy.limits().max_scratch_destinations as usize {
            let cause = crate::ContextConfigError::RequestedDestinations {
                requested: buffers,
                limit: self.policy.limits().max_scratch_destinations,
                source_bits,
                precision_bits,
            };
            return Err(match progress {
                Some(progress) => GeoError::retained_failure_with_progress(
                    cause,
                    GeoError::ContextConfiguration,
                    progress,
                ),
                None => GeoError::retained_failure(cause, GeoError::ContextConfiguration, None),
            });
        }
        let limbs = scratch_limb_capacity(source_bits, precision_bits)?.max(
            self.integer_scratch
                .as_ref()
                .map_or(0, LimbScratch::limb_capacity),
        );
        let buffers = buffers.max(
            self.integer_scratch
                .as_ref()
                .map_or(0, LimbScratch::destination_capacity),
        );
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
        // The attempted proof's coefficients and simultaneous temporaries are
        // admitted together, independently of unused caller resource ceilings.
        let bytes = match progress.as_ref() {
            Some(progress) => progress.integer_scratch_layout(buffers, limbs)?,
            None => {
                LimbScratch::required_bytes(buffers, limbs).map_err(GeoError::NumericalScratch)?
            }
        };
        self.admit_workspace(bytes as u64)?;
        // The outer governor observes the complete native admission before
        // the arena's vectors or ownership headers are allocated.
        let prepared = (|| match progress {
            Some(progress) => {
                progress.context_poll(self)?;
                progress.prepare_integer_scratch(buffers, limbs)
            }
            None => LimbScratch::new(buffers, limbs).map_err(GeoError::NumericalScratch),
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
            return Err(GeoError::retained_failure(
                crate::ContextConfigError::UnavailableBackend { backend },
                GeoError::ContextConfiguration,
                None,
            ));
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
        self.prepare_arithmetic_with_observer(None)
    }

    /// Prepare the same immutable arithmetic under the supplied original
    /// physical arena/table factory and work observer.
    ///
    /// # Errors
    /// Preserves the first original admission, allocator, numerical, or
    /// cancellation failure. Partial native producers retain their owners
    /// through destruction; no resident arena replaces a refused one.
    pub fn prepare_arithmetic_metered(
        &mut self,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<PreparedBinary64, GeoError> {
        self.prepare_arithmetic_with_observer(Some(observer))
    }

    fn prepare_arithmetic_with_observer(
        &mut self,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<PreparedBinary64, GeoError> {
        self.begin(0)?;
        let policy = self.policy;
        let mut progress = WorkProgress::new(observer);
        let source_bits = crate::numerical::scratch_source_bits(&[], self.reference.ellipsoid());
        self.prepare_integer_scratch_for_observed(source_bits, &mut progress)?;
        let mut math = self
            .coordinate_math(MathLimits {
                precision_bits: policy.limits().max_precision_bits.min(96),
                max_work: policy.limits().max_work_items,
                max_workspace_bytes: usize::try_from(policy.limits().max_workspace_bytes).map_err(
                    |_| GeoError::MemoryExhausted {
                        limit: policy.limits().max_workspace_bytes,
                    },
                )?,
            })
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
        let prepared = self.install_arithmetic_observed(&mut math, Some(&mut progress));
        if let Some(error) = progress.latched_error() {
            return Err(error);
        }
        prepared.map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
        self.charge_work(math.work_used())?;
        self.admit_workspace(math.workspace_peak() as u64)?;
        self.release_workspace(math.workspace_peak() as u64)?;
        progress.context_poll(self)?;
        self.retain_prepared_integer_scratch()?;
        self.arithmetic.clone().ok_or(GeoError::ArithmeticOverflow(
            "prepared arithmetic invariant",
        ))
    }

    pub(crate) fn install_arithmetic(
        &mut self,
        math: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        self.install_arithmetic_observed(math, None)
    }

    pub(crate) fn install_arithmetic_observed(
        &mut self,
        math: &mut CoordinateMath,
        progress: Option<&mut WorkProgress<'_>>,
    ) -> Result<(), MathError> {
        math.set_binary64_backend(self.binary64_backend)?;
        if let Some(prepared) = &self.arithmetic {
            math.install_binary64(prepared.clone())
        } else {
            let prepared = match progress {
                Some(progress) => progress.prepare_binary64(math, self.policy)?,
                None => math.prepare_binary64()?,
            };
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

    pub(crate) fn promote_admitted_workspace(&mut self, bytes: u64) -> Result<(), GeoError> {
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
            return Err(GeoError::retained_failure(
                crate::ContextConfigError::OwnedWorkspace {
                    requested: bytes,
                    arena: arena_bytes,
                    coefficients: self.geodesic_coefficients_owned_bytes,
                    zero_bound: self.zero_conversion_bound_owned_bytes,
                    taylor: self.taylor_scratch_owned_bytes,
                    output: self.materialized_output_bytes,
                    receipts: self.materialized_receipt_storage_bytes,
                    required: owned,
                },
                GeoError::ContextConfiguration,
                None,
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
        self.produce_output_observed(cost, output_bytes, |_| evaluate(), Some(observer))
    }

    /// Produce output through this invocation's original physical owner.
    ///
    /// The native factory grows its actual buffers with the supplied Memory.
    /// Work polls occur outside the factory, allowing the same account to serve
    /// the observer without a nested mutable borrow. Successful output retains
    /// that original account until destruction or ownership-carrying publication.
    ///
    /// # Errors
    /// Preserves the first policy, observer, physical, or native-body refusal.
    pub fn produce_output_with_storage_metered<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        output_bytes: u64,
        evaluate: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        ) -> Result<T, GeoError>,
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
        self.produce_output_observed(cost, output_bytes, |_| evaluate(), None)
    }

    fn produce_output_observed<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        output_bytes: u64,
        evaluate: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
        ) -> Result<T, GeoError>,
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
        progress.preflight_exact(self, cost, output_bytes)?;
        self.admit_workspace(output_bytes)?;
        self.admit_workspace(cost.workspace_bytes)?;
        let result = (|| {
            self.charge_work(cost.work_items)?;
            progress.context_poll(self)?;
            let output = progress.with_storage(0, evaluate)?;
            // On refusal output dies here while its original physical grant
            // is still alive; no detached result escapes the boundary.
            progress.context_poll(self)?;
            Ok(output)
        })();
        let scratch_refund = self.release_workspace(cost.workspace_bytes);
        if result.is_err() {
            let _ = self.release_workspace(output_bytes);
        }
        result.and_then(|output| scratch_refund.map(|()| output))
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

    /// Run native working metadata under its checked policy allowance. The
    /// callback destroys its working buffers before returning; immutable
    /// results use their separately retained publication owners. Select the
    /// original failure before cleanup, including a refused later growth.
    pub(crate) fn with_metadata<'observer, T>(
        &mut self,
        progress: &mut WorkProgress<'observer>,
        body: impl FnOnce(&mut Self, &mut WorkProgress<'observer>, &mut u64) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let mut retained = 0;
        let result = body(self, progress, &mut retained);
        let released = self.release_workspace(retained);
        match result {
            Ok(value) => released.map(|()| value),
            Err(original) => Err(original),
        }
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
        // This refunds only the allocation the caller has destroyed. An arena
        // may have just been prepared for a following operation, even when
        // every destination is currently available. Exact-operation and
        // invocation boundaries decide when an automatic arena is complete.
        Ok(())
    }

    pub(crate) fn release_unused_integer_scratch(&mut self) -> Result<(), GeoError> {
        if self.integer_scratch_reusable
            || !self.integer_scratch_owned
            || self
                .integer_scratch
                .as_ref()
                .is_none_or(|scratch| !scratch.is_unused())
        {
            return Ok(());
        }
        let bytes = self
            .integer_scratch
            .as_ref()
            .expect("owned arena")
            .retained_bytes() as u64;
        let retained = self.retained_workspace_bytes.checked_sub(bytes).ok_or(
            GeoError::ArithmeticOverflow("completed exact arena baseline"),
        )?;
        let current =
            self.workspace_bytes
                .checked_sub(bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "completed exact arena storage",
                ))?;
        // `is_unused` proves this is the sole handle and every immutable
        // destination has returned. Destruction precedes the exact refund.
        drop(self.integer_scratch.take());
        self.integer_scratch_owned = false;
        self.retained_workspace_bytes = retained;
        self.workspace_bytes = current;
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
        self.release_unused_integer_scratch()?;
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
    fn context_configuration_refusals_keep_original_capacities_and_baselines() {
        use crate::{ContextConfigError, PreparationBaselineReason, SourcePreparationError};
        use purrdf_hash::{Backend as _, error::ErrorValue};
        use purrdf_testkit::error_capture::Capture;
        use purrdf_xsd::math::FloatProductBackend;

        let mut context = MetricContext::wgs84().unwrap();
        let limit = context.policy().limits().max_scratch_destinations;
        let error = context
            .prepare_integer_scratch_observed(u64::MAX, usize::MAX, 8192, None)
            .unwrap_err();
        assert!(matches!(&error, GeoError::ContextConfiguration(cause)
            if **cause == ContextConfigError::RequestedDestinations { requested: usize::MAX, limit, source_bits: u64::MAX, precision_bits: 8192 }));
        let fields = Capture::read(&error).child("cause");
        assert!(
            matches!(fields.field("source_bits"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(context.integer_scratch().is_none());

        context.prepare_integer_scratch().unwrap();
        let scratch = context.integer_scratch().unwrap().clone();
        let destinations = scratch.destination_capacity();
        let limbs = scratch.limb_capacity();
        let mut restricted = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(ExecutionLimits {
                max_scratch_destinations: 1,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        )
        .unwrap();
        let error = restricted
            .set_borrowed_integer_scratch(scratch.clone())
            .unwrap_err();
        assert!(matches!(&error, GeoError::ContextConfiguration(cause)
            if **cause == ContextConfigError::BorrowedDestinations { requested: destinations, limit: 1 }));
        assert!(restricted.integer_scratch().is_none());
        let error = context.set_borrowed_integer_scratch(scratch).unwrap_err();
        assert!(matches!(&error, GeoError::ContextConfiguration(cause)
            if **cause == ContextConfigError::ScratchInstalled { existing_destinations: destinations, existing_limbs: limbs, incoming_destinations: destinations, incoming_limbs: limbs }));
        let retained = context.retained_workspace_bytes();
        let error = context.set_retained_workspace(0).unwrap_err();
        assert!(matches!(&error, GeoError::ContextConfiguration(cause)
            if matches!(&**cause, ContextConfigError::OwnedWorkspace { requested: 0, arena, required, .. }
                if *arena > 0 && *required == retained)));
        assert_eq!(context.retained_workspace_bytes(), retained);

        for backend in [
            FloatProductBackend::Portable,
            FloatProductBackend::Sse2,
            FloatProductBackend::Avx2,
            FloatProductBackend::Avx512,
            FloatProductBackend::Neon,
            FloatProductBackend::Simd128,
        ] {
            if !backend.is_available() {
                let original = context.binary64_backend();
                let error = context.set_binary64_backend(backend).unwrap_err();
                assert!(matches!(&error, GeoError::ContextConfiguration(cause)
                    if **cause == ContextConfigError::UnavailableBackend { backend }));
                assert_eq!(context.binary64_backend(), original);
            }
        }

        let child = MetricContext::wgs84().unwrap();
        for (work, storage, reason) in [
            (u64::MAX, u64::MAX, PreparationBaselineReason::Work),
            (0, u64::MAX, PreparationBaselineReason::Workspace),
        ] {
            for (error, absorbing, available_work, available_workspace) in [
                (
                    context
                        .remaining_child_with_baseline(work, storage)
                        .unwrap_err(),
                    false,
                    context.preparation_work_items(),
                    context.retained_workspace_bytes(),
                ),
                (
                    context
                        .absorb_child_with_baseline(&child, work, storage)
                        .unwrap_err(),
                    true,
                    child.preparation_work_items(),
                    child.retained_workspace_bytes(),
                ),
            ] {
                assert!(matches!(&error, GeoError::SourcePreparation(cause)
                    if **cause == SourcePreparationError::ChildBaseline { absorbing, reason, requested_work: work, requested_workspace: storage, available_work, available_workspace }));
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let last = error.clone();
                assert_eq!(window.close().allocations, 0);
                drop(error);
                assert_eq!(
                    Capture::read(&last).child("cause").code,
                    "source-preparation-child-baseline"
                );
            }
        }
    }

    #[test]
    fn relaxed_scratch_ceilings_admit_only_the_attempted_working_set() {
        let mut bounded = MetricContext::wgs84().unwrap();
        bounded.prepare_integer_scratch_for(128).unwrap();
        let mut relaxed =
            MetricContext::new(GeographicReference::wgs84(), ExecutionPolicy::unbounded()).unwrap();
        relaxed.prepare_integer_scratch_for(128).unwrap();
        assert_eq!(bounded.workspace_peak(), relaxed.workspace_peak());
        assert_eq!(
            bounded.integer_scratch().unwrap().limb_capacity(),
            relaxed.integer_scratch().unwrap().limb_capacity()
        );
        let initial = relaxed.workspace_peak();
        let mut progress = super::WorkProgress::new(None);
        relaxed
            .prepare_integer_scratch_at_precision_observed(128, 2048, &mut progress)
            .unwrap();
        assert!(relaxed.workspace_peak() > initial);
        let value = crate::Int::one()
            .shl_in(8192, relaxed.integer_scratch().unwrap())
            .unwrap();
        assert_eq!(value, crate::Int::one().shl(8192));
        assert_eq!(relaxed.policy(), ExecutionPolicy::unbounded());

        let original = crate::Rat::from_int(crate::Int::one().shl(1024));
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            1025,
            1,
        )
        .unwrap();
        let mut preparation = super::PreparationBudget::new(ExecutionPolicy::unbounded());
        preparation
            .exact_values_observed(
                cost,
                |values, _| {
                    assert_eq!(values.original(&original).unwrap(), original);
                    Ok(())
                },
                None,
            )
            .unwrap();
    }

    #[test]
    fn requested_scratch_growth_preserves_live_values_and_refuses_before_allocation() {
        use purrdf_alloc_probe::CurrentThreadWindow;
        use purrdf_xsd::integer::LimbScratchError;

        let mut context =
            MetricContext::new(GeographicReference::wgs84(), ExecutionPolicy::unbounded()).unwrap();
        context.prepare_integer_scratch_for(128).unwrap();
        let request = GeoError::NumericalScratch(LimbScratchError::Destinations {
            required: 900,
            admitted: 768,
        });
        let mut progress = super::WorkProgress::new(None);
        let live = crate::Int::one()
            .shl_in(400, context.integer_scratch().unwrap())
            .unwrap();
        let baseline = context.current_workspace_bytes();
        let measured = CurrentThreadWindow::open();
        assert!(matches!(
            context.grow_requested_integer_scratch(&request, &mut progress),
            Err(GeoError::NumericalScratch(LimbScratchError::Retained {
                held_destinations: 1,
                destinations: 768,
            }))
        ));
        assert_eq!(measured.close().allocations, 0);
        assert_eq!(context.current_workspace_bytes(), baseline);
        assert_eq!(live.bit_len(), 401);
        drop(live);
        assert!(
            context
                .grow_requested_integer_scratch(&request, &mut progress)
                .unwrap()
        );
        assert_eq!(
            context.integer_scratch().unwrap().destination_capacity(),
            900
        );
        assert!(
            !context
                .grow_requested_integer_scratch(&request, &mut progress)
                .unwrap()
        );

        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_workspace_bytes: baseline,
            ..ExecutionLimits::UNBOUNDED
        })
        .unwrap();
        let mut bounded = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        bounded.prepare_integer_scratch_for(128).unwrap();
        let measured = CurrentThreadWindow::open();
        assert!(matches!(
            bounded.grow_requested_integer_scratch(&request, &mut progress),
            Err(GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(measured.close().allocations, 0);
        assert_eq!(bounded.current_workspace_bytes(), baseline);
        assert_eq!(
            bounded.integer_scratch().unwrap().destination_capacity(),
            768
        );
    }

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
        assert!(crate::context::Retained::ptr_eq(
            parent.geodesic_coefficients.as_ref().unwrap(),
            first.geodesic_coefficients.as_ref().unwrap(),
        ));
        assert!(!crate::context::Retained::ptr_eq(
            first.geodesic_coefficients.as_ref().unwrap(),
            sibling.geodesic_coefficients.as_ref().unwrap(),
        ));
        parent.absorb_child(&sibling).unwrap();
        assert_eq!(parent.retained_workspace_bytes(), original_retained + added);
        assert!(crate::context::Retained::ptr_eq(
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
