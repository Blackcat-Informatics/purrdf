// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native immutable preparation publication under its original physical owner.

use core::alloc::Layout;
use core::fmt;
use core::ops::{Deref, DerefMut};

use purrdf_lex::allocation::{Admission, Memory, Shared, SharedOwned, StorageError};

use super::WorkProgress;
use crate::{Coord, GeoError, LonLat, MetricContext, Rat};

pub(crate) trait NativeDetached: Sized {
    fn layout_bytes(&self) -> Result<usize, StorageError>;
    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError>;
    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError>;
}

// These native value shapes contain only intrinsically retained scalar limbs.
// The adapter shares their actual typed factories without a second copy body.
macro_rules! detached_value {
    ($($value:ty),+ $(,)?) => {$(
        impl NativeDetached for $value {
            fn layout_bytes(&self) -> Result<usize, StorageError> {
                self.detached_required_bytes()
            }
            fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
                self.detached_with_memory(memory)
            }
            fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
                self.publish_detached(original)
            }
        }
    )+};
}
detached_value!(Coord, LonLat, Rat, purrdf_xsd::math::FixedInterval);

/// Native typed fields use their existing scalar/array factories. Discriminants
/// stay inline; no field list is rebuilt by a separate allocator algorithm.
macro_rules! native_detached_fields {
    ($value:ty {
        native: [$($native:tt),* $(,)?], copy: [$($copy:tt),* $(,)?]
        $(, shared: [$($shared:tt),* $(,)?])?
    }) => {
        impl $crate::context::retained::NativeDetached for $value {
            fn layout_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
                let bytes = 0_usize;
                $(let bytes = bytes.checked_add(
                    $crate::context::retained::NativeDetached::layout_bytes(&self.$native)?
                ).ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;)*
                Ok(bytes)
            }
            fn prepare(&self, memory: &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>)
                -> Result<Self, purrdf_lex::allocation::StorageError> {
                Ok(Self {
                    $($native: $crate::context::retained::NativeDetached::prepare(&self.$native, memory)?,)*
                    $($copy: self.$copy,)*
                    $($($shared: self.$shared.clone(),)*)?
                })
            }
            fn publish(&mut self, original: &purrdf_lex::allocation::SharedOwned<()>)
                -> Result<(), purrdf_lex::allocation::StorageError> {
                $($crate::context::retained::NativeDetached::publish(&mut self.$native, original)?;)*
                Ok(())
            }
        }
    };
}
pub(crate) use native_detached_fields;

macro_rules! native_detached_enum {
    ($value:ty {
        units: [$($unit:ident),* $(,)?],
        tuples: [$($tuple:ident),* $(,)?],
        fields: [$($variant:ident { native: [$($native:ident),* $(,)?], copy: [$($copy:ident),* $(,)?] }),* $(,)?]
    }) => {
        impl $crate::context::retained::NativeDetached for $value {
            fn layout_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
                match self {
                    $(Self::$unit => Ok(0),)*
                    $(Self::$tuple(value) => $crate::context::retained::NativeDetached::layout_bytes(value),)*
                    $(Self::$variant { $($native,)* $($copy: _,)* } => {
                        let bytes = 0_usize;
                        $(let bytes = bytes.checked_add($crate::context::retained::NativeDetached::layout_bytes($native)?)
                            .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;)*
                        Ok(bytes)
                    },)*
                }
            }
            fn prepare(&self, memory: &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>)
                -> Result<Self, purrdf_lex::allocation::StorageError> {
                Ok(match self {
                    $(Self::$unit => Self::$unit,)*
                    $(Self::$tuple(value) => Self::$tuple($crate::context::retained::NativeDetached::prepare(value, memory)?),)*
                    $(Self::$variant { $($native,)* $($copy,)* } => Self::$variant {
                        $($native: $crate::context::retained::NativeDetached::prepare($native, memory)?,)* $($copy: *$copy,)*
                    },)*
                })
            }
            fn publish(&mut self, original: &purrdf_lex::allocation::SharedOwned<()>)
                -> Result<(), purrdf_lex::allocation::StorageError> {
                match self {
                    $(Self::$unit => {},)*
                    $(Self::$tuple(value) => $crate::context::retained::NativeDetached::publish(value, original)?,)*
                    $(Self::$variant { $($native,)* $($copy: _,)* } => {
                        $($crate::context::retained::NativeDetached::publish($native, original)?;)*
                    },)*
                }
                Ok(())
            }
        }
    };
}
pub(crate) use native_detached_enum;

impl<T: NativeDetached> NativeDetached for Option<T> {
    fn layout_bytes(&self) -> Result<usize, StorageError> {
        self.as_ref().map_or(Ok(0), NativeDetached::layout_bytes)
    }

    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
        self.as_ref().map(|value| value.prepare(memory)).transpose()
    }

    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
        if let Some(value) = self {
            value.publish(original)?;
        }
        Ok(())
    }
}

impl<A: NativeDetached, B: NativeDetached> NativeDetached for (A, B) {
    fn layout_bytes(&self) -> Result<usize, StorageError> {
        self.0
            .layout_bytes()?
            .checked_add(self.1.layout_bytes()?)
            .ok_or(StorageError::SizeOverflow)
    }
    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
        Ok((self.0.prepare(memory)?, self.1.prepare(memory)?))
    }
    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
        self.0.publish(original)?;
        self.1.publish(original)
    }
}

impl<A: NativeDetached, B: NativeDetached, C: NativeDetached> NativeDetached for (A, B, C) {
    fn layout_bytes(&self) -> Result<usize, StorageError> {
        let first_two = self
            .0
            .layout_bytes()?
            .checked_add(self.1.layout_bytes()?)
            .ok_or(StorageError::SizeOverflow)?;
        first_two
            .checked_add(self.2.layout_bytes()?)
            .ok_or(StorageError::SizeOverflow)
    }
    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
        Ok((
            self.0.prepare(memory)?,
            self.1.prepare(memory)?,
            self.2.prepare(memory)?,
        ))
    }
    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
        self.0.publish(original)?;
        self.1.publish(original)?;
        self.2.publish(original)
    }
}

impl<T: NativeDetached> NativeDetached for Box<T> {
    fn layout_bytes(&self) -> Result<usize, StorageError> {
        Layout::new::<T>()
            .size()
            .checked_add((**self).layout_bytes()?)
            .ok_or(StorageError::SizeOverflow)
    }

    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
        let value = (**self).prepare(memory)?;
        memory.boxed(value)
    }

    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
        (**self).publish(original)
    }
}

impl<T: NativeDetached, const N: usize> NativeDetached for [T; N] {
    fn layout_bytes(&self) -> Result<usize, StorageError> {
        self.iter().try_fold(0_usize, |bytes, value| {
            bytes
                .checked_add(value.layout_bytes()?)
                .ok_or(StorageError::SizeOverflow)
        })
    }

    fn prepare(&self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
        let mut prepared = core::array::from_fn::<_, N, _>(|_| None);
        for (destination, source) in prepared.iter_mut().zip(self) {
            *destination = Some(source.prepare(memory)?);
        }
        Ok(prepared.map(|value| value.expect("every original array element was prepared")))
    }

    fn publish(&mut self, original: &SharedOwned<()>) -> Result<(), StorageError> {
        for value in self {
            value.publish(original)?;
        }
        Ok(())
    }
}

/// Preserve a numerical result outside its reusable arena. Every detached limb
/// owns the original lease intrinsically before this stack carrier is consumed;
/// the admitted native value shape contains no separate unowned container.
pub(crate) fn persistent_numeric<T: NativeDetached>(
    value: T,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<T, GeoError> {
    persistent_numeric_owned(value, context, progress, retained).map(|value| value.value)
}

/// A native immutable number and the exact policy allowance of the original
/// detached publication. The value itself retains every physical limb owner.
pub(crate) struct PersistentNumeric<T> {
    value: T,
    workspace_bytes: core::cell::Cell<u64>,
}

impl<T> PersistentNumeric<T> {
    pub(crate) fn workspace_bytes(&self) -> u64 {
        self.workspace_bytes.get()
    }

    /// A returned value may share this original immutable publication. Move
    /// its already admitted allowance once; neither the live charge nor the
    /// original physical owner changes while temporary aliases still exist.
    pub(crate) fn transfer_allowance(
        &self,
        temporary: &mut u64,
        retained: &mut u64,
    ) -> Result<u64, GeoError> {
        let bytes = self.workspace_bytes.get();
        let remaining = temporary
            .checked_sub(bytes)
            .ok_or(GeoError::ArithmeticOverflow("numeric allowance transfer"))?;
        let output = retained
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("numeric allowance transfer"))?;
        *temporary = remaining;
        *retained = output;
        self.workspace_bytes.set(0);
        Ok(bytes)
    }

    pub(crate) fn into_value(self) -> T {
        self.value
    }
}

impl<T> Deref for PersistentNumeric<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

pub(crate) fn persistent_numeric_owned<T: NativeDetached>(
    value: T,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<PersistentNumeric<T>, GeoError> {
    let result = persistent_numeric_from(&value, context, progress, retained);
    drop(value);
    result
}

/// Copy a borrowed native value inside the original detached factory, rather
/// than creating an unowned intermediate with a resident deep clone.
pub(crate) fn persistent_numeric_from<T: NativeDetached>(
    value: &T,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<PersistentNumeric<T>, GeoError> {
    let bytes = persistent_numeric_bytes(value, progress)?;
    context.retain_workspace(bytes, retained)?;
    let produced = progress
        .context_poll(context)
        .and_then(|()| produce_persistent_numeric_from(value, progress));
    match produced {
        Ok(value) => Ok(value),
        Err(error) => {
            let _ = context.release_retained_workspace(bytes, retained);
            Err(error)
        }
    }
}

pub(crate) fn persistent_numeric_bytes<T: NativeDetached>(
    value: &T,
    progress: &WorkProgress<'_>,
) -> Result<u64, GeoError> {
    numeric_fields_bytes(value.layout_bytes().map_err(GeoError::Storage)?, progress)
}

/// The shared physical factory; metric and cell budgets keep their own policy
/// counters around this one original immutable publication body.
pub(crate) fn produce_persistent_numeric<T: NativeDetached>(
    value: T,
    progress: &mut WorkProgress<'_>,
) -> Result<PersistentNumeric<T>, GeoError> {
    let result = produce_persistent_numeric_from(&value, progress);
    drop(value);
    result
}

/// Copy a borrowed original operand only inside its actual detached factory.
/// A failure diagnostic must not first deep-clone caller-owned magnitude limbs.
pub(crate) fn produce_persistent_numeric_from<T: NativeDetached>(
    value: &T,
    progress: &mut WorkProgress<'_>,
) -> Result<PersistentNumeric<T>, GeoError> {
    produce_persistent_numeric_fields(
        value.layout_bytes().map_err(GeoError::Storage)?,
        progress,
        |memory| value.prepare(memory).map_err(GeoError::Storage),
    )
}

/// Construct native scalar fields directly from borrowed original inputs.
/// The caller certifies the sum of those fields' native layouts before this
/// same factory runs; no resident intermediate clone precedes admission.
pub(crate) fn persistent_numeric_fields<T: NativeDetached>(
    payload_bytes: usize,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    make: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let bytes = numeric_fields_bytes(payload_bytes, progress)?;
    context.retain_workspace(bytes, retained)?;
    let result = progress.context_poll(context).and_then(|()| {
        produce_persistent_numeric_fields(payload_bytes, progress, make)
            .map(PersistentNumeric::into_value)
    });
    if result.is_err() {
        let _ = context.release_retained_workspace(bytes, retained);
    }
    result
}

fn numeric_fields_bytes(
    payload_bytes: usize,
    progress: &WorkProgress<'_>,
) -> Result<u64, GeoError> {
    if payload_bytes == 0 {
        return Ok(0);
    }
    payload_bytes
        .checked_add(progress.retained_owner_layout().size())
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::Storage(StorageError::SizeOverflow))
}

pub(crate) fn produce_persistent_numeric_fields<T: NativeDetached>(
    payload_bytes: usize,
    progress: &mut WorkProgress<'_>,
    make: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
) -> Result<PersistentNumeric<T>, GeoError> {
    let bytes = numeric_fields_bytes(payload_bytes, progress)?;
    if payload_bytes == 0 {
        let mut resident = purrdf_lex::allocation::Resident;
        return Ok(PersistentNumeric {
            value: make(&mut Memory::new(&mut resident as &mut dyn Admission))?,
            workspace_bytes: core::cell::Cell::new(0),
        });
    }
    let Produced {
        value,
        allocation: original,
        ..
    } = Produced::produce(progress, Layout::new::<()>(), make, |value, original| {
        value.publish(original).map_err(GeoError::Storage)
    })?;
    // Every surviving Mag::Owned header already holds the original lease.
    drop(original);
    Ok(PersistentNumeric {
        value,
        workspace_bytes: core::cell::Cell::new(bytes),
    })
}

/// One preparation factory borrowed by the allocation-owning observer.
///
/// The observer calls `produce` once under a fresh original account. It calls
/// `abort` before releasing that account on any failed production or publication.
/// A successful producer keeps its pending payload until its shared destination
/// exists. Neither operation allocates a factory object.
pub trait RetainedAllocationProducer {
    /// Build through the original native allocation homes under this account.
    ///
    /// # Errors
    /// Preserves the original numerical or physical failure.
    fn produce(&mut self, memory: &mut Memory<'_, dyn Admission + '_>) -> Result<(), GeoError>;

    /// Destroy every pending payload before the original account is released.
    fn abort(&mut self);
}

/// Failure of an original retained producer or its immutable publication.
/// A published cause already owns its original storage; a refused publication
/// returns that storage so the caller can preserve its first physical refusal.
pub enum RetainedProductionFailure<S> {
    /// The typed cause retains the account under which its payload was born.
    Published(GeoError),
    /// No failure payload escaped the original account.
    Unpublished {
        /// The original producer or physical publication refusal.
        error: GeoError,
        /// The original storage capability, still available to its caller.
        storage: S,
    },
}

impl<S> RetainedProductionFailure<S> {
    /// Preserve the typed cause at a resident producer boundary.
    /// Published payloads carry their original owner intrinsically.
    #[must_use]
    pub fn into_error(self) -> GeoError {
        match self {
            Self::Published(error) | Self::Unpublished { error, .. } => error,
        }
    }
}

impl<S> fmt::Debug for RetainedProductionFailure<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, error) = match self {
            Self::Published(error) => ("Published", error),
            Self::Unpublished { error, .. } => ("Unpublished", error),
        };
        formatter.debug_tuple(kind).field(error).finish()
    }
}

/// Run a retained preparation using its concrete original storage owner.
///
/// `control` is the actual outer shared allocation. Both controls are admitted
/// before the factory runs. The returned immutable lease covers every native
/// buffer produced in that account and lives through the outer allocation's
/// deallocation. Partial payloads are destroyed before an error returns storage.
///
/// # Errors
/// Returns an ownership-carrying cause, or the original refusal together with
/// the unpublished storage capability. Storage is never duplicated or replaced.
pub fn retain_produced_with_storage<S: Admission + Send + Sync + 'static>(
    control: Layout,
    producer: &mut dyn RetainedAllocationProducer,
    mut storage: S,
) -> Result<SharedOwned<()>, RetainedProductionFailure<S>> {
    let result = {
        let mut memory = Memory::new(&mut storage as &mut dyn Admission);
        (|| {
            memory
                .add_bytes(control.size())
                .map_err(GeoError::Storage)?;
            memory
                .add_bytes(SharedOwned::<()>::allocation_layout::<S>().size())
                .map_err(GeoError::Storage)?;
            producer.produce(&mut memory)
        })()
    };
    if let Err(error) = result {
        producer.abort();
        let retained = match error.native_text_bytes() {
            Ok(bytes) => bytes,
            Err(original) => {
                drop(error);
                return Err(RetainedProductionFailure::Unpublished {
                    error: GeoError::Storage(original),
                    storage,
                });
            }
        };
        if retained != 0 {
            // The failure's text was already priced before it was constructed.
            // All other producer locals are now dead: settle to that surviving
            // payload plus its exact immutable control before publication.
            let Some(bytes) =
                retained.checked_add(SharedOwned::<GeoError>::allocation_layout::<S>().size())
            else {
                drop(error);
                return Err(RetainedProductionFailure::Unpublished {
                    error: GeoError::Storage(StorageError::SizeOverflow),
                    storage,
                });
            };
            if let Err(original) = storage.resize(bytes) {
                drop(error);
                return Err(RetainedProductionFailure::Unpublished {
                    error: GeoError::Storage(original),
                    storage,
                });
            }
            let mut pending = Some((error, storage));
            return match SharedOwned::try_from_admitted_with(|| {
                pending.take().expect("one original failure publication")
            }) {
                Ok(original) => Err(RetainedProductionFailure::Published(GeoError::Retained(
                    crate::RetainedGeoError(original),
                ))),
                Err(_) => {
                    let (error, storage) = pending
                        .take()
                        .expect("refused failure retains its payload and account");
                    drop(error);
                    Err(RetainedProductionFailure::Unpublished {
                        error: GeoError::Storage(StorageError::AllocationFailed),
                        storage,
                    })
                }
            };
        }
        return Err(RetainedProductionFailure::Unpublished { error, storage });
    }
    let mut pending = Some(storage);
    match SharedOwned::try_from_admitted_with(|| {
        ((), pending.take().expect("one original preparation owner"))
    }) {
        Ok(owner) => Ok(owner),
        Err(_) => {
            producer.abort();
            Err(RetainedProductionFailure::Unpublished {
                error: GeoError::Storage(StorageError::AllocationFailed),
                storage: pending.take().expect("refused owner remains available"),
            })
        }
    }
}

pub(crate) struct Produced<T> {
    value: T,
    admitted_bytes: usize,
    // Heap production keeps its original lease until the payload dies. Checked
    // inline arrays have no container allocation requiring a lease.
    allocation: Option<SharedOwned<()>>,
}

/// Settle an original failed producer after its working locals have died.
/// The cause's fields were admitted when constructed; inspection here refunds
/// dead scratch and prices only the new immutable control before allocation.
///
/// # Errors
/// Returns the original cause or the first physical refusal. An unpriced cause
/// is a hard ownership error, never a resident-copy fallback.
pub fn retain_failure_with_storage<S: Admission + Send + Sync + 'static>(
    error: GeoError,
    storage: &mut S,
    replacement: S,
    admitted: usize,
) -> GeoError {
    let bytes = match error.native_text_bytes() {
        Ok(bytes) => bytes,
        Err(error) => return GeoError::Storage(error),
    };
    if bytes == 0 {
        let _ = storage.resize(0);
        return error;
    }
    if bytes > admitted {
        drop(error);
        return GeoError::InvalidNativeStorage(
            "failure fields were not admitted by their original producer",
        );
    }
    let Some(total) = bytes.checked_add(SharedOwned::<GeoError>::allocation_layout::<S>().size())
    else {
        drop(error);
        return GeoError::Storage(StorageError::SizeOverflow);
    };
    if let Err(original) = storage.resize(total) {
        drop(error);
        return GeoError::Storage(original);
    }
    let mut pending = Some(error);
    match SharedOwned::try_from_admitted_with(|| {
        (
            pending.take().expect("one original failure cause"),
            core::mem::replace(storage, replacement),
        )
    }) {
        Ok(original) => GeoError::Retained(crate::RetainedGeoError(original)),
        Err(_) => {
            drop(pending);
            GeoError::Storage(StorageError::AllocationFailed)
        }
    }
}

/// Immutable native payload retaining its original physical allocation owner.
///
/// Cloning this value shares the payload and its owner. Only an immutable borrow
/// is exposed; publication cannot detach the payload from its original lease.
pub struct Retained<T>(Shared<Produced<T>>);

impl<T> Retained<T> {
    /// Publish a private payload already covered by the current producer's
    /// original account. Both controls are admitted before transfer; failure
    /// destroys the payload while that original account remains live.
    pub(crate) fn publish_original(
        value: T,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let (allocation, admitted_bytes) = progress.retain_current(Self::allocation_layout())?;
        let mut pending = Some(Produced {
            value,
            admitted_bytes,
            allocation: Some(allocation),
        });
        match Shared::try_new_with(|| pending.take().expect("one original publication")) {
            Ok(value) => Ok(Self(value)),
            Err(_) => {
                drop(pending);
                Err(GeoError::Storage(StorageError::AllocationFailed))
            }
        }
    }

    /// Publish a caller-owned resident value through the same native control factory.
    ///
    /// # Errors
    /// Returns a physical allocator refusal without invoking an alternate factory.
    pub fn try_new(value: T) -> Result<Self, GeoError> {
        Self::produce(
            &mut WorkProgress::integer(None),
            |_| Ok(value),
            |_, _| Ok(()),
        )
    }

    /// Publish a resident payload under the resident allocation contract.
    ///
    /// Bounded producers use their original admission factory instead.
    /// # Panics
    /// Panics if physical allocation of the resident control fails.
    #[must_use]
    pub fn resident(value: T) -> Self {
        Self::try_new(value).expect("resident prepared value allocation")
    }

    pub(crate) fn required_bytes(progress: &WorkProgress<'_>) -> Result<u64, GeoError> {
        Self::allocation_layout()
            .size()
            .checked_add(progress.retained_owner_layout().size())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))
    }

    pub(crate) fn new_admitted(
        value: T,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Self, GeoError> {
        let bytes = Self::required_bytes(progress)?;
        context.retain_workspace(bytes, retained)?;
        let result = progress
            .context_poll(context)
            .and_then(|()| Self::produce(progress, |_| Ok(value), |_, _| Ok(())));
        if result.is_err() {
            let _ = context.release_retained_workspace(bytes, retained);
        }
        result
    }

    /// Publish a numerical phase's already constructed payload while retaining
    /// its exact control layout. Payload buffers retain their own original
    /// scalar owners; the enclosing phase destroys this control before refund.
    pub(crate) fn new_in_math(
        value: T,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
        retained: &mut usize,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        let bytes = usize::try_from(Self::required_bytes(progress)?)
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let total = retained
            .checked_add(bytes)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        math.reserve_workspace(bytes)
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))?;
        let result = progress
            .math_poll(math)
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))
            .and_then(|()| Self::produce(progress, |_| Ok(value), |_, _| Ok(())));
        if result.is_err() {
            let _ = math.release_workspace(bytes);
        } else {
            *retained = total;
        }
        result
    }

    pub(crate) const fn allocation_layout() -> Layout {
        Shared::<Produced<T>>::allocation_layout()
    }

    pub(crate) fn ptr_eq(a: &Self, b: &Self) -> bool {
        core::ptr::eq(core::ptr::from_ref(&**a), core::ptr::from_ref(&**b))
    }

    pub(crate) fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    pub(crate) fn get_mut(value: &mut Self) -> Option<&mut T> {
        Shared::get_mut(&mut value.0).map(|payload| &mut payload.value)
    }

    pub(crate) fn admitted_bytes(&self) -> usize {
        self.0.admitted_bytes
    }
}

impl<T> Produced<T> {
    /// Transfer a private working payload and its original account together.
    /// The recipient reserves its owner slot before calling this door; no raw
    /// production payload may leave that enclosing owner before publication.
    pub(crate) fn into_parts(self) -> (T, usize, SharedOwned<()>) {
        (
            self.value,
            self.admitted_bytes,
            self.allocation
                .expect("physical producer has its original account"),
        )
    }

    pub(crate) fn produce(
        progress: &mut WorkProgress<'_>,
        control: Layout,
        make: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
        publish: impl FnOnce(&mut T, &SharedOwned<()>) -> Result<(), GeoError>,
    ) -> Result<Self, GeoError> {
        struct Producer<T, F> {
            value: Option<T>,
            make: Option<F>,
            admitted_bytes: usize,
        }
        impl<T, F> RetainedAllocationProducer for Producer<T, F>
        where
            F: FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
        {
            fn produce(
                &mut self,
                memory: &mut Memory<'_, dyn Admission + '_>,
            ) -> Result<(), GeoError> {
                self.value = Some(self.make.take().expect("one retained factory")(memory)?);
                self.admitted_bytes = memory.admitted_bytes();
                Ok(())
            }

            fn abort(&mut self) {
                drop(self.value.take());
            }
        }
        let mut producer = Producer {
            value: None,
            make: Some(make),
            admitted_bytes: 0,
        };
        let allocation = progress.retain_produced(control, &mut producer)?;
        if let Err(error) = publish(
            producer.value.as_mut().expect("completed preparation"),
            &allocation,
        ) {
            producer.abort();
            return Err(error);
        }
        Ok(Self {
            value: producer.value.take().expect("completed preparation"),
            admitted_bytes: producer.admitted_bytes,
            allocation: Some(allocation),
        })
    }

    pub(crate) fn admitted_bytes(&self) -> usize {
        self.admitted_bytes
    }
}

impl<A: purrdf_core::small::Array> Produced<purrdf_core::SmallVec<A>> {
    /// A checked inline array has no container allocation to retain. Its
    /// elements keep their own native owners or the enclosing numerical arena.
    pub(crate) fn inline_array(value: purrdf_core::SmallVec<A>) -> Self {
        assert!(!value.spilled(), "inline producer has no buffer allocation");
        Self {
            value,
            admitted_bytes: 0,
            allocation: None,
        }
    }
}

impl<T> Retained<T> {
    pub(crate) fn produce(
        progress: &mut WorkProgress<'_>,
        make: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
        publish: impl FnOnce(&mut T, &SharedOwned<()>) -> Result<(), GeoError>,
    ) -> Result<Self, GeoError> {
        let mut pending = Some(Produced::produce(
            progress,
            Self::allocation_layout(),
            make,
            publish,
        )?);
        match Shared::try_new_with(|| pending.take().expect("one retained publication")) {
            Ok(value) => Ok(Self(value)),
            Err(_) => {
                drop(pending);
                Err(GeoError::Storage(StorageError::AllocationFailed))
            }
        }
    }
}

impl<T> Deref for Produced<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Produced<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Produced<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, formatter)
    }
}

impl<T> Clone for Retained<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Retained<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0.value
    }
}

impl<T> AsRef<T> for Retained<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> From<T> for Retained<T> {
    /// Retain an already caller-owned resident payload through the same native
    /// publication factory. Its existing allocations retain their own owners;
    /// admitted engine producers use their original workspace factory instead.
    fn from(value: T) -> Self {
        Self::produce(
            &mut WorkProgress::integer(None),
            |_| Ok(value),
            |_, _| Ok(()),
        )
        .expect("caller-owned resident value publication")
    }
}

impl<T: fmt::Debug> fmt::Debug for Retained<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T: PartialEq> PartialEq for Retained<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: Eq> Eq for Retained<T> {}

impl<T: core::hash::Hash> core::hash::Hash for Retained<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&**self, state);
    }
}

/// Immutable ordered values retaining their original buffer and control owner.
/// Empty inventories allocate nothing. A clone shares every original value and
/// the same live buffer rather than copying it into an unowned array.
pub struct RetainedValues<T> {
    values: Option<Retained<Vec<T>>>,
}

impl<T> RetainedValues<T> {
    /// Borrow the complete ordered cells without copying or detaching an owner.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self
    }
    pub(crate) const EMPTY: Self = Self { values: None };

    pub(crate) fn from_admitted(values: RetainedBuilder<T>) -> Self {
        Self {
            values: values.finish(),
        }
    }

    pub(crate) fn from_retained(values: Retained<Vec<T>>) -> Self {
        Self {
            values: Some(values),
        }
    }

    pub(crate) fn admitted_bytes(&self) -> usize {
        self.values.as_ref().map_or(0, Retained::admitted_bytes)
    }

    /// Trusted final publication can attach allocation-free owner pins before
    /// this unique buffer escapes. Shared/published arrays remain immutable.
    pub(crate) fn unpublished_mut(&mut self) -> Option<&mut [T]> {
        match self.values.as_mut() {
            None => Some(&mut []),
            Some(values) => Retained::get_mut(values).map(Vec::as_mut_slice),
        }
    }

    /// The native immutable buffer/control layouts retained by this array.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> usize {
        self.admitted_bytes()
    }

    /// Admit existing caller-owned cells before their immutable control is born.
    /// The original Vec moves once; no slice copy or allocation replacement is
    /// used to publish it. Its original owner lives through buffer destruction.
    pub(crate) fn from_vec(
        values: Vec<T>,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        Self::from_vec_with(values, progress, |_, _| Ok(()), |_, _| Ok(()))
    }

    /// Transform a nonempty original array inside the same immutable factory.
    /// The native callback admits any sorting scratch and detached cell payload
    /// before birth; publication keeps that one original account on each cell.
    pub(crate) fn from_vec_with(
        mut values: Vec<T>,
        progress: &mut WorkProgress<'_>,
        prepare: impl FnOnce(&mut Vec<T>, &mut Memory<'_, dyn Admission + '_>) -> Result<(), GeoError>,
        publish: impl FnOnce(&mut Vec<T>, &SharedOwned<()>) -> Result<(), GeoError>,
    ) -> Result<Self, GeoError> {
        if values.capacity() == 0 {
            return Ok(Self::EMPTY);
        }
        let bytes = Layout::array::<T>(values.capacity())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            .size();
        Retained::produce(
            progress,
            |memory| {
                memory.add_bytes(bytes).map_err(GeoError::Storage)?;
                prepare(&mut values, memory)?;
                Ok(values)
            },
            publish,
        )
        .map(Self::from_retained)
    }

    /// Produce a new immutable ordered table before copying native shared rows.
    /// Each row callback must preserve intrinsic payload owners; this home
    /// admits the actual new table and control before the first row is copied.
    pub(crate) fn inserted_with(
        &self,
        index: usize,
        value: T,
        progress: &mut WorkProgress<'_>,
        mut copy: impl FnMut(&T) -> Result<T, GeoError>,
    ) -> Result<Self, GeoError> {
        let required = self
            .len()
            .checked_add(1)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        if index > self.len() {
            return Err(GeoError::ArithmeticOverflow(
                "native table insertion position",
            ));
        }
        let mut output = RetainedBuilder::default();
        output.reserve(required, progress)?;
        for row in &self[..index] {
            output.push_reserved(copy(row)?);
        }
        output.push_reserved(value);
        for row in &self[index..] {
            output.push_reserved(copy(row)?);
        }
        Ok(Self::from_admitted(output))
    }
}

impl<T> Default for RetainedValues<T> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<T> From<Vec<T>> for RetainedValues<T> {
    /// Retain a caller-owned resident buffer without copying its cells.
    fn from(values: Vec<T>) -> Self {
        Self::from_vec(values, &mut WorkProgress::integer(None))
            .expect("resident prepared array publication")
    }
}

impl<T> Clone for RetainedValues<T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}

impl<T> Deref for RetainedValues<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.values.as_ref().map_or(&[], |values| values.as_slice())
    }
}

macro_rules! borrowed_array_slice {
    ($($array:ident),+ $(,)?) => {$(
        impl<T> AsRef<[T]> for $array<T> {
            fn as_ref(&self) -> &[T] { self }
        }
    )+};
}
borrowed_array_slice!(RetainedValues, RetainedBuilder);

macro_rules! borrowed_array_iteration {
    ($($array:ident),+ $(,)?) => {$(
        impl<'a, T> IntoIterator for &'a $array<T> {
            type Item = &'a T;
            type IntoIter = core::slice::Iter<'a, T>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    )+};
}
borrowed_array_iteration!(RetainedValues, RetainedBuilder);

impl<'a, T> IntoIterator for &'a mut RetainedBuilder<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: fmt::Debug> fmt::Debug for RetainedValues<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T: PartialEq> PartialEq for RetainedValues<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<T: Eq> Eq for RetainedValues<T> {}

macro_rules! resident_array_equality {
    ($($array:ident),+ $(,)?) => {$(
        impl<T: PartialEq> PartialEq<Vec<T>> for $array<T> {
            fn eq(&self, other: &Vec<T>) -> bool { self.as_slice() == other.as_slice() }
        }
    )+};
}
resident_array_equality!(RetainedValues, RetainedBuilder);

impl<T: PartialEq> PartialEq for RetainedBuilder<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: Eq> Eq for RetainedBuilder<T> {}

impl<T: core::hash::Hash> core::hash::Hash for RetainedValues<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_ref(), state);
    }
}

macro_rules! native_retained_copy {
    ($($owner:ident),+ $(,)?) => {$(
        impl<T> NativeDetached for $owner<T> {
            fn layout_bytes(&self) -> Result<usize, StorageError> { Ok(0) }
            fn prepare(&self, _: &mut Memory<'_, dyn Admission + '_>) -> Result<Self, StorageError> {
                Ok(self.clone())
            }
            fn publish(&mut self, _: &SharedOwned<()>) -> Result<(), StorageError> { Ok(()) }
        }
    )+};
}
native_retained_copy!(Retained, RetainedValues);

/// A uniquely owned native working array. Each replacement is constructed
/// through the shared lexical buffer home under its fresh original owner;
/// both owners remain alive until the previous buffer actually dies. Freezing
/// consumes this builder and keeps the same allocation without copying cells.
pub struct RetainedBuilder<T> {
    values: Option<Retained<Vec<T>>>,
}

impl<T> Default for RetainedBuilder<T> {
    fn default() -> Self {
        Self { values: None }
    }
}

impl<T> RetainedBuilder<T> {
    /// Append a caller's already owned cell under the same configuration policy.
    /// Buffer/control replacement uses the original native retained factory;
    /// the previous owner remains alive until its actual buffer is destroyed.
    /// # Errors
    /// Preserves checked layout, cumulative work/storage and allocator refusal.
    pub fn push_in_budget(
        &mut self,
        value: T,
        budget: &mut crate::PreparationBudget,
    ) -> Result<(), GeoError> {
        budget.retain(1, 0)?;
        let required = self
            .len()
            .checked_add(1)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        let mut progress = WorkProgress::integer(None);
        if let Some(replacement) = self.replacement_bytes(required, &progress)? {
            let old = u64::try_from(self.admitted_bytes())
                .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
            let replacement = u64::try_from(replacement)
                .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
            budget.with_workspace(replacement, |_| self.reserve(required, &mut progress))?;
            budget.replace_workspace(old, replacement)?;
        }
        self.push_reserved(value);
        Ok(())
    }

    /// Freeze the original buffer without copying cells or allocating control.
    #[must_use]
    pub fn into_prepared_values(self) -> crate::PreparedValues<T> {
        crate::PreparedValues::from_admitted(self)
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        self
    }
    pub(crate) fn reserve_metadata_admitted(
        &mut self,
        additional: usize,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        crate::carrier::reserve_metadata(&mut (self, retained), additional, context, progress)
    }

    pub(crate) fn capacity(&self) -> usize {
        self.values.as_ref().map_or(0, |values| values.capacity())
    }

    pub(crate) fn admitted_bytes(&self) -> usize {
        self.values.as_ref().map_or(0, Retained::admitted_bytes)
    }

    pub(crate) fn replacement_bytes(
        &self,
        required: usize,
        progress: &WorkProgress<'_>,
    ) -> Result<Option<usize>, GeoError> {
        if required <= self.capacity() {
            return Ok(None);
        }
        let capacity =
            purrdf_lex::allocation::growth_capacity::<T>(self.capacity().max(1), required)
                .map_err(GeoError::Storage)?;
        let bytes = Layout::array::<T>(capacity)
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            .size()
            .checked_add(Retained::<Vec<T>>::allocation_layout().size())
            .and_then(|total| total.checked_add(progress.retained_owner_layout().size()))
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        Ok(Some(bytes))
    }

    pub(crate) fn reserve(
        &mut self,
        required: usize,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        if required <= self.capacity() {
            return Ok(());
        }
        let capacity =
            purrdf_lex::allocation::growth_capacity::<T>(self.capacity().max(1), required)
                .map_err(GeoError::Storage)?;
        let mut replacement = Retained::produce(
            progress,
            |memory| {
                let mut values = Vec::new();
                memory
                    .reserve(&mut values, capacity)
                    .map_err(GeoError::Storage)?;
                Ok(values)
            },
            |_, _| Ok(()),
        )?;
        if let Some(previous) = self.values.as_mut() {
            let previous =
                Retained::get_mut(previous).expect("unpublished working array is unique");
            Retained::get_mut(&mut replacement)
                .expect("fresh working allocation is unique")
                .append(previous);
        }
        // Replacing the owner destroys its now-empty Vec buffer before releasing
        // the original lease; cells have already moved under the new buffer.
        drop(self.values.replace(replacement));
        Ok(())
    }

    pub(crate) fn push(
        &mut self,
        value: T,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let required = self
            .len()
            .checked_add(1)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        self.reserve(required, progress)?;
        self.push_reserved(value);
        Ok(())
    }

    /// Append only into a slot whose original allocation has already been
    /// admitted. This is the same insertion body used after ordinary growth.
    pub(crate) fn push_reserved(&mut self, value: T) {
        assert!(
            self.len() < self.capacity(),
            "working array slot is admitted"
        );
        Retained::get_mut(
            self.values
                .as_mut()
                .expect("nonempty working array has an owner"),
        )
        .expect("unpublished working array is unique")
        .push(value);
    }

    pub(crate) fn reserve_admitted(
        &mut self,
        required: usize,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let Some(bytes) = self.replacement_bytes(required, progress)? else {
            return Ok(());
        };
        let bytes =
            u64::try_from(bytes).map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let previous = u64::try_from(self.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        context.retain_workspace(bytes, retained)?;
        let result = progress
            .context_poll(context)
            .and_then(|()| self.reserve(required, progress));
        if let Err(error) = result {
            // Original failure selection precedes cleanup. Refused replacement
            // storage has already died; it cannot replace that original cause.
            let _ = context.release_retained_workspace(bytes, retained);
            return Err(error);
        }
        context.release_retained_workspace(previous, retained)
    }

    /// Replace an immutable preparation table already present in the baseline.
    /// The ordinary native replacement body destroys the old buffer before the
    /// baseline changes; both physical owners are admitted during that overlap.
    pub(crate) fn reserve_prepared(
        &mut self,
        required: usize,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let Some(bytes) = self.replacement_bytes(required, progress)? else {
            return Ok(());
        };
        let bytes =
            u64::try_from(bytes).map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let previous = u64::try_from(self.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        let baseline = context
            .retained_workspace_bytes
            .checked_add(bytes)
            .and_then(|total| total.checked_sub(previous))
            .ok_or(GeoError::ArithmeticOverflow("prepared table baseline"))?;
        let current = context
            .current_workspace_bytes()
            .checked_add(bytes)
            .and_then(|total| total.checked_sub(previous))
            .ok_or(GeoError::ArithmeticOverflow("prepared table replacement"))?;
        context.admit_workspace(bytes)?;
        if let Err(original) = progress
            .context_poll(context)
            .and_then(|()| self.reserve(required, progress))
        {
            let _ = context.release_workspace(bytes);
            return Err(original);
        }
        context.retained_workspace_bytes = baseline;
        context.workspace_bytes = current;
        Ok(())
    }

    pub(crate) fn push_admitted(
        &mut self,
        value: T,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let required = self
            .len()
            .checked_add(1)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        self.reserve_admitted(required, context, progress, retained)?;
        self.push(value, progress)
    }

    /// Use the same original buffer replacement inside a numerical phase.
    /// Both policy reservations stay live until the old buffer is destroyed.
    pub(crate) fn reserve_in_math(
        &mut self,
        required: usize,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
        retained: &mut usize,
        policy: crate::ExecutionPolicy,
    ) -> Result<(), GeoError> {
        self.reserve_numerical(required, math, progress, retained)
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))
    }

    /// Preserve the original typed numerical seam while sharing native growth.
    pub(crate) fn reserve_numerical(
        &mut self,
        required: usize,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
        retained: &mut usize,
    ) -> Result<(), purrdf_xsd::math::MathError> {
        use purrdf_xsd::math::MathError;
        let Some(bytes) = self
            .replacement_bytes(required, progress)
            .map_err(|error| progress.math_failure(error))?
        else {
            return Ok(());
        };
        let previous = self.admitted_bytes();
        let replacement = retained
            .checked_add(bytes)
            .and_then(|total| total.checked_sub(previous))
            .ok_or(MathError::Storage(StorageError::SizeOverflow))?;
        math.reserve_workspace(bytes)?;
        let result = progress.math_poll(math).and_then(|()| {
            self.reserve(required, progress)
                .map_err(|error| progress.math_failure(error))
        });
        if let Err(error) = result {
            let _ = math.release_workspace(bytes);
            return Err(error);
        }
        *retained = replacement;
        math.release_workspace(previous)
    }

    pub(crate) fn push_in_math(
        &mut self,
        value: T,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
        retained: &mut usize,
        policy: crate::ExecutionPolicy,
    ) -> Result<(), GeoError> {
        self.push_numerical(value, math, progress, retained)
            .map_err(|error| crate::numerical::geo_math_error(&error, policy))
    }

    pub(crate) fn push_numerical(
        &mut self,
        value: T,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
        retained: &mut usize,
    ) -> Result<(), purrdf_xsd::math::MathError> {
        let required = self
            .len()
            .checked_add(1)
            .ok_or(purrdf_xsd::math::MathError::Storage(
                StorageError::SizeOverflow,
            ))?;
        self.reserve_numerical(required, math, progress, retained)?;
        self.push_reserved(value);
        Ok(())
    }

    pub(crate) fn release_admitted(
        self,
        context: &mut MetricContext,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let bytes = u64::try_from(self.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        drop(self);
        context.release_retained_workspace(bytes, retained)
    }

    /// Move cells from another working array, keeping its original buffer
    /// owner until the consuming iterator actually dies. Both arrays use the
    /// enclosing producer's checked policy receipt; moved cell owners survive.
    pub(crate) fn append_admitted(
        &mut self,
        source: Self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<(), GeoError> {
        let previous = u64::try_from(source.admitted_bytes())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
        self.reserve_metadata_admitted(source.len(), context, progress, retained)?;
        for value in source {
            self.push_reserved(value);
        }
        context.release_retained_workspace(previous, retained)
    }

    pub(crate) fn truncate(&mut self, length: usize) {
        if let Some(values) = self.values.as_mut() {
            Retained::get_mut(values)
                .expect("unpublished working array is unique")
                .truncate(length);
        }
    }

    pub(crate) fn swap_remove(&mut self, index: usize) -> T {
        Retained::get_mut(
            self.values
                .as_mut()
                .expect("nonempty working array has an owner"),
        )
        .expect("unpublished working array is unique")
        .swap_remove(index)
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.values.as_mut().and_then(|values| {
            Retained::get_mut(values)
                .expect("unpublished working array is unique")
                .pop()
        })
    }

    pub(crate) fn dedup_by<E>(
        &mut self,
        same: impl FnMut(&T, &T) -> Result<bool, E>,
    ) -> Result<(), E> {
        let kept = purrdf_lex::walk::try_partition_unique_by(self, same)?;
        self.truncate(kept);
        Ok(())
    }

    pub(crate) fn finish(self) -> Option<Retained<Vec<T>>> {
        self.values
    }
}

impl<T> Deref for RetainedBuilder<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.values.as_ref().map_or(&[], |values| values.as_slice())
    }
}

impl<T: fmt::Debug> fmt::Debug for RetainedBuilder<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T> DerefMut for RetainedBuilder<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.values.as_mut().map_or(&mut [], |values| {
            Retained::get_mut(values)
                .expect("unpublished working array is unique")
                .as_mut_slice()
        })
    }
}

/// The original Vec buffer moves into this iterator; its immutable original
/// control/lease stays alive until that buffer and all unconsumed cells die.
#[derive(Debug)]
pub struct RetainedIter<T> {
    values: std::vec::IntoIter<T>,
    _original: Option<Retained<Vec<T>>>,
}

impl<T> Iterator for RetainedIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.values.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.values.size_hint()
    }
}

impl<T> ExactSizeIterator for RetainedIter<T> {}
impl<T> core::iter::FusedIterator for RetainedIter<T> {}

impl<T> IntoIterator for RetainedBuilder<T> {
    type Item = T;
    type IntoIter = RetainedIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        let values = self.values.as_mut().map_or_else(Vec::new, |values| {
            core::mem::take(Retained::get_mut(values).expect("unpublished working array is unique"))
        });
        RetainedIter {
            values: values.into_iter(),
            _original: self.values,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Retained, RetainedAllocationProducer, RetainedBuilder, retain_produced_with_storage,
    };
    use crate::{GeoError, MetricContext, MetricWorkObserver, Rat, context::WorkProgress};
    use core::alloc::Layout;
    use core::cell::Cell;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_lex::allocation::{Admission, Memory, SharedOwned, StorageError};
    use std::sync::Arc;

    struct Account {
        live: AtomicUsize,
        maximum: AtomicUsize,
        limit: usize,
    }

    struct Owner {
        account: Arc<Account>,
        live: usize,
    }

    impl Admission for Owner {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            let mut previous = self.account.live.load(Ordering::Relaxed);
            loop {
                let next = previous
                    .checked_sub(self.live)
                    .and_then(|total| total.checked_add(bytes))
                    .ok_or(StorageError::SizeOverflow)?;
                if next > self.account.limit {
                    return Err(StorageError::AdmissionFailed);
                }
                match self.account.live.compare_exchange_weak(
                    previous,
                    next,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        self.live = bytes;
                        self.account.maximum.fetch_max(next, Ordering::Relaxed);
                        return Ok(());
                    }
                    Err(observed) => previous = observed,
                }
            }
        }
    }

    impl Drop for Owner {
        fn drop(&mut self) {
            self.account.live.fetch_sub(self.live, Ordering::Relaxed);
        }
    }

    #[test]
    fn native_planar_matrix_prices_actual_arrays_and_limb_owners_until_destruction() {
        let crs = crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let a = crate::wkt::parse("GEOMETRYCOLLECTION (POLYGON ((0 0,4 0,4 4,0 4,0 0),(1 1,3 1,3 3,1 3,1 1)),LINESTRING (0 2,4 2))", &crs).unwrap();
        let b = crate::wkt::parse("POLYGON ((2 -1,5 -1,5 5,2 5,2 -1))", &crs).unwrap();
        let expected = crate::topology::relate(a.geometry(), b.geometry());
        let mut context = MetricContext::wgs84().unwrap();
        let (account, mut observer) = observer(usize::MAX);
        let window = CurrentThreadWindow::open();
        assert_eq!(
            crate::topology::relate_metered(
                a.geometry(),
                b.geometry(),
                &mut context,
                &mut observer
            )
            .unwrap(),
            expected
        );
        assert!(context.integer_scratch().is_none());
        assert_eq!(
            account.live.load(Ordering::Relaxed),
            0,
            "completed arrays are destroyed and inline arithmetic owns no heap arena"
        );
        assert!(account.maximum.load(Ordering::Relaxed) != 0);
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(
            measured.peak_working_bytes
                <= i64::try_from(account.maximum.load(Ordering::Relaxed)).unwrap()
        );

        let mut context = MetricContext::wgs84().unwrap();
        let (account, mut refusing_observer) = self::observer(0);
        let window = CurrentThreadWindow::open();
        assert!(matches!(
            crate::topology::relate_metered(
                a.geometry(),
                b.geometry(),
                &mut context,
                &mut refusing_observer
            ),
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(window.close().allocations, 0);
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_heap_refuses_replacement_before_birth_and_preserves_original_buffer() {
        use purrdf_lex::allocation::Memory;
        use std::collections::BinaryHeap;
        for limit in [95, 96] {
            let (account, mut observer) = observer(limit);
            let mut heap = BinaryHeap::new();
            let mut memory = Memory::new(&mut observer.1);
            for value in 0_u64..4 {
                memory.push_heap(&mut heap, value).unwrap();
            }
            assert_eq!(heap.capacity(), 4);
            assert_eq!(account.live.load(Ordering::Relaxed), 32);
            let window = CurrentThreadWindow::open();
            let result = memory.push_heap(&mut heap, 4);
            let measured = window.close();
            if limit == 95 {
                assert_eq!(result, Err(StorageError::AdmissionFailed));
                assert_eq!(measured.allocations, 0);
                assert_eq!(heap.len(), 4);
                assert_eq!(heap.peek(), Some(&3));
                assert_eq!(account.live.load(Ordering::Relaxed), 32);
            } else {
                result.unwrap();
                assert_eq!(heap.len(), 5);
                assert_eq!(heap.peek(), Some(&4));
                assert_eq!(account.live.load(Ordering::Relaxed), 64);
                assert_eq!(account.maximum.load(Ordering::Relaxed), 96);
            }
            memory.release_heap(heap).unwrap();
            assert_eq!(account.live.load(Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn native_union_keeps_original_output_arrays_through_last_shallow_owner() {
        let profile = crate::GeoProfile::standard();
        let crs = crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse(
            "MULTIPOLYGON (((0 0,1 0,1 1,0 1,0 0)),((1 0,2 0,2 1,1 1,1 0)))",
            &crs,
        )
        .unwrap();
        let source = crate::PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.begin_integer(1).unwrap();
        let (account, mut observer) = observer(usize::MAX);
        let window = CurrentThreadWindow::open();
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let mut storage = crate::carrier::MaterializationStorage::default();
        let geometry = crate::atlas::union::materialize_with_output(
            source.region(),
            &mut context,
            &mut progress,
            |geometry, bytes, context, progress| {
                storage.adopt_produced_geometry_output(
                    geometry,
                    bytes,
                    context,
                    progress,
                    |count, _| {
                        assert_eq!(count, 7);
                        Ok(())
                    },
                )
            },
        )
        .unwrap();
        let produced = storage
            .publish_literal(
                crate::GeometryLiteral::new(crs, geometry),
                &mut context,
                &mut progress,
            )
            .unwrap();
        let (output, receipt) = storage
            .finish_result(Ok(produced), &mut context, &mut progress)
            .unwrap();
        let before = window.sample().allocations;
        let last = output.clone();
        assert_eq!(window.sample().allocations, before);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(account.live.load(Ordering::Relaxed)).unwrap()
        );
        drop(output);
        drop(context);
        drop(receipt);
        let crate::GeometryBody::MultiPolygon(polygons) = last.literal.geometry().body() else {
            panic!("native complete union must preserve its canonical polygons")
        };
        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 1);
        assert_eq!(polygons[0][0].len(), 7);
        assert!(account.live.load(Ordering::Relaxed) != 0);
        drop(last);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(
            measured.peak_working_bytes
                <= i64::try_from(account.maximum.load(Ordering::Relaxed)).unwrap()
        );
    }

    #[test]
    fn native_union_refusal_publishes_no_partial_geometry_or_retained_working_arrays() {
        let profile = crate::GeoProfile::standard();
        let crs = crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("POLYGON ((0 0,1 0,1 1,0 1,0 0))", &crs).unwrap();
        let source = crate::PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.begin_integer(1).unwrap();
        let (account, mut observer) = observer(0);
        let window = CurrentThreadWindow::open();
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let mut storage = crate::carrier::MaterializationStorage::default();
        let result = crate::atlas::union::materialize_with_output(
            source.region(),
            &mut context,
            &mut progress,
            |geometry, bytes, context, progress| {
                storage.adopt_produced_geometry_output(
                    geometry,
                    bytes,
                    context,
                    progress,
                    |_, _| Ok(()),
                )
            },
        );
        assert!(matches!(
            result.as_ref().unwrap_err().original(),
            GeoError::Storage(StorageError::AdmissionFailed)
                | GeoError::NumericalScratch(purrdf_xsd::integer::LimbScratchError::Storage(
                    StorageError::AdmissionFailed
                ))
        ));
        assert_eq!(storage.output_bytes(), 0);
        let result = result
            .map(|geometry| crate::GeometryLiteral::new(crs, geometry))
            .and_then(|literal| storage.publish_literal(literal, &mut context, &mut progress));
        assert!(
            storage
                .finish_result(result, &mut context, &mut progress)
                .is_err()
        );
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.retained_bytes, 0);
    }

    struct Observer(Arc<Account>, Owner);

    impl MetricWorkObserver for Observer {
        fn retain_failure(&mut self, error: GeoError) -> GeoError {
            let admitted = self.1.live;
            let replacement = Owner {
                account: self.0.clone(),
                live: 0,
            };
            let error =
                super::retain_failure_with_storage(error, &mut self.1, replacement, admitted);
            self.1 = Owner {
                account: self.0.clone(),
                live: 0,
            };
            error
        }
        fn native_storage(
            &mut self,
            producer: &mut dyn crate::NativeStorageProducer,
        ) -> Result<(), GeoError> {
            let admitted = self.1.live;
            let mut work = Self(
                self.0.clone(),
                Owner {
                    account: self.0.clone(),
                    live: 0,
                },
            );
            producer.produce(&mut Memory::resume(&mut self.1, admitted), &mut work)
        }
        fn retain_current(
            &mut self,
            control: Layout,
        ) -> Result<(SharedOwned<()>, usize), GeoError> {
            let admitted = self.1.live;
            let replacement = Owner {
                account: self.0.clone(),
                live: 0,
            };
            SharedOwned::retain_current(&mut self.1, replacement, admitted, control)
                .map_err(GeoError::Storage)
        }
        fn allocation(&mut self) -> Option<(&mut dyn Admission, usize)> {
            let admitted = self.1.live;
            Some((&mut self.1, admitted))
        }
        fn retained_owner_layout(&self) -> Layout {
            SharedOwned::<()>::allocation_layout::<Owner>()
        }

        fn retain_produced(
            &mut self,
            control: Layout,
            producer: &mut dyn RetainedAllocationProducer,
        ) -> Result<SharedOwned<()>, GeoError> {
            retain_produced_with_storage(
                control,
                producer,
                Owner {
                    account: self.0.clone(),
                    live: 0,
                },
            )
            .map_err(super::RetainedProductionFailure::into_error)
        }

        fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
            purrdf_xsd::integer::LimbScratch::required_bytes_with_storage::<Owner>(buffers, limbs)
                .map_err(GeoError::Storage)
        }

        fn prepare_integer_scratch(
            &mut self,
            buffers: usize,
            limbs: usize,
        ) -> Result<purrdf_xsd::integer::LimbScratch, GeoError> {
            purrdf_xsd::integer::LimbScratch::new_with_storage(
                buffers,
                limbs,
                Owner {
                    account: self.0.clone(),
                    live: 0,
                },
            )
            .map_err(|(error, original)| {
                drop(original);
                GeoError::NumericalScratch(purrdf_xsd::integer::LimbScratchError::Storage(error))
            })
        }

        fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
            Ok(())
        }
    }

    #[test]
    fn registered_profiles_share_original_tables_and_cow_keeps_each_live_owner() {
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let mut profile = crate::GeoProfile::standard();
        profile
            .register_reference_in_budget_metered(
                crate::Crs::new("urn:example:reference:first").unwrap(),
                crate::GeographicReference::wgs84(),
                &mut budget,
                &mut observer,
            )
            .unwrap();
        let first = account.live.load(Ordering::Relaxed);
        assert!(first > 0);
        let window = CurrentThreadWindow::open();
        let original = profile.clone();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), first);
        profile
            .register_reference_in_budget_metered(
                crate::Crs::new("urn:example:reference:second").unwrap(),
                crate::GeographicReference::cgcs2000(),
                &mut budget,
                &mut observer,
            )
            .unwrap();
        assert_eq!(original.references().len(), 1);
        assert_eq!(profile.references().len(), 2);
        assert!(account.live.load(Ordering::Relaxed) > first);
        drop(profile);
        assert_eq!(account.live.load(Ordering::Relaxed), first);
        drop(original);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn refused_native_registration_preserves_original_profile_and_cause_owner() {
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let name = crate::Crs::new("urn:example:reference:registered").unwrap();
        let mut profile = crate::GeoProfile::standard();
        profile
            .register_reference_in_budget_metered(
                name.clone(),
                crate::GeographicReference::wgs84(),
                &mut budget,
                &mut observer,
            )
            .unwrap();
        let previous = profile.clone();
        let failure = profile
            .register_reference_in_budget_metered(
                name,
                crate::GeographicReference::cgcs2000(),
                &mut budget,
                &mut observer,
            )
            .unwrap_err();
        assert!(matches!(failure.original(), GeoError::Config(message)
            if message == "conflicting reference registration for <urn:example:reference:registered>"));
        assert_eq!(profile, previous);
        drop(profile);
        drop(previous);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        let window = CurrentThreadWindow::open();
        let copy = failure.clone();
        assert_eq!(window.close().allocations, 0);
        drop(failure);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        drop(copy);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn zero_native_admission_refuses_registration_before_factory_birth() {
        let (account, mut observer) = observer(0);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let mut profile = crate::GeoProfile::standard();
        let reference = crate::GeographicReference::wgs84();
        let name = crate::Crs::new("urn:example:reference:denied").unwrap();
        // Immutable standard input is established before this producer's window.
        let _ = crate::binding::standard_reference();
        let window = CurrentThreadWindow::open();
        let failure = profile
            .register_reference_in_budget_metered(name, reference, &mut budget, &mut observer)
            .unwrap_err();
        assert!(
            matches!(
                failure.original(),
                GeoError::Storage(StorageError::AdmissionFailed)
            ),
            "original native refusal: {failure:?}"
        );
        assert_eq!(window.close().allocations, 0);
        assert_eq!(profile.references(), []);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    fn observer(limit: usize) -> (Arc<Account>, Observer) {
        let account = Arc::new(Account {
            live: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
            limit,
        });
        let owner = Owner {
            account: account.clone(),
            live: 0,
        };
        (account.clone(), Observer(account, owner))
    }

    fn root_record() -> purrdf_xsd::math::RootIsolation2 {
        use purrdf_xsd::math::{
            CoordinateMath, FixedInterval, MathError, MathLimits, RootBox2, RootIsolationLimits,
            RootJacobian2, RootSystem2,
        };
        struct Identity;
        impl RootSystem2 for Identity {
            fn image(
                &mut self,
                domain: &RootBox2,
                _: &mut CoordinateMath,
            ) -> Result<RootBox2, MathError> {
                Ok(domain.clone())
            }
            fn jacobian(
                &mut self,
                _: &RootBox2,
                math: &mut CoordinateMath,
            ) -> Result<RootJacobian2, MathError> {
                let zero = FixedInterval::from_i64(0, math)?;
                let one = FixedInterval::from_i64(1, math)?;
                Ok([[one.clone(), zero.clone()], [zero, one]])
            }
            fn complete(
                &mut self,
                enclosure: &RootBox2,
                _: &mut CoordinateMath,
            ) -> Result<bool, MathError> {
                Ok(enclosure.iter().all(|value| value.lower() == value.upper()))
            }
            fn poll(&mut self, _: &CoordinateMath) -> Result<(), MathError> {
                Ok(())
            }
        }
        let mut math = CoordinateMath::new(MathLimits {
            precision_bits: 16,
            max_work: 1_000_000,
            max_workspace_bytes: usize::MAX,
        })
        .unwrap();
        let low = FixedInterval::from_i64(-1, &mut math).unwrap();
        let high = FixedInterval::from_i64(1, &mut math).unwrap();
        let domain =
            FixedInterval::from_bounds(low.lower().clone(), high.upper().clone(), &mut math)
                .unwrap();
        let mut roots = purrdf_xsd::math::isolate_roots2(
            [domain.clone(), domain],
            RootIsolationLimits {
                max_depth: 32,
                max_refinements: 32,
            },
            &mut Identity,
            &mut math,
        )
        .unwrap();
        assert_eq!(roots.len(), 1);
        roots.pop().unwrap()
    }

    #[test]
    fn native_root_spill_keeps_its_original_buffer_owner_after_the_worker_dies() {
        use purrdf_xsd::math::RootStorage2;
        let root = root_record();
        let (account, mut observer) = observer(usize::MAX);
        let values = {
            let mut progress = WorkProgress::new(Some(&mut observer));
            let progress = core::cell::RefCell::new(&mut progress);
            let mut output = crate::numerical::NativeRootStorage {
                values: crate::numerical::NativeRootValues::default(),
                progress: &progress,
            };
            for _ in 0..4 {
                assert_eq!(output.insertion_layout().unwrap(), None);
                output.try_push(root.clone()).unwrap();
            }
            assert_eq!(account.live.load(Ordering::Relaxed), 0);
            let (bytes, previous) = output.insertion_layout().unwrap().unwrap();
            assert_eq!(previous, 0);
            output.try_push(root).unwrap();
            assert_eq!(output.storage_bytes().unwrap(), bytes);
            assert_eq!(account.live.load(Ordering::Relaxed), bytes);
            output.values
        };
        assert_eq!(values.len(), 5);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        let mut records = values.into_iter();
        drop(records.next());
        assert!(account.live.load(Ordering::Relaxed) > 0);
        drop(records);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn refused_native_root_spill_keeps_the_four_original_proofs_and_first_cause() {
        use purrdf_xsd::math::{MathError, RootStorage2};
        let root = root_record();
        let (account, mut observer) = observer(0);
        let mut progress = WorkProgress::new(Some(&mut observer));
        {
            let progress = core::cell::RefCell::new(&mut progress);
            let mut output = crate::numerical::NativeRootStorage {
                values: crate::numerical::NativeRootValues::default(),
                progress: &progress,
            };
            for _ in 0..4 {
                output.try_push(root.clone()).unwrap();
            }
            let window = CurrentThreadWindow::open();
            assert_eq!(output.try_push(root), Err(MathError::Cancelled));
            assert_eq!(window.close().allocations, 0);
            assert_eq!(output.roots().len(), 4);
            assert_eq!(
                progress.borrow().latched_error(),
                Some(GeoError::Storage(StorageError::AdmissionFailed))
            );
            output.reset();
        }
        drop(progress);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn already_published_numbers_share_the_original_lease_without_another_birth() {
        let (account, mut observer) = observer(usize::MAX);
        let mut context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        context.begin_integer(1).unwrap();
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let source = Rat::from_int(crate::Int::one().shl(512).add(&crate::Int::from_i64(17)));
        let mut retained = 0;
        let original =
            super::persistent_numeric(source, &mut context, &mut progress, &mut retained).unwrap();
        let live = account.live.load(Ordering::Relaxed);
        assert!(live > 0);
        let window = CurrentThreadWindow::open();
        let copied =
            super::persistent_numeric_from(&original, &mut context, &mut progress, &mut retained)
                .unwrap();
        assert_eq!(copied.workspace_bytes(), 0);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        assert_eq!(&*copied, &original);
        drop(original);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(copied);
        context.release_workspace(retained).unwrap();
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    fn boundary_source() -> (MetricContext, crate::PreparedRegion) {
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8_000_000;
        let context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let ring = [[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]
            .map(|[x, y]| crate::Coord::xy(Rat::from_i64(x), Rat::from_i64(y)))
            .to_vec();
        let polygon = crate::PreparedPolygon::from_source(
            &vec![ring],
            context.reference(),
            crate::RegionInterior::Written,
        )
        .unwrap();
        (context, crate::PreparedRegion::polygons(vec![polygon]))
    }

    #[test]
    fn boundary_arrays_and_standalone_parameter_keep_the_original_source_owner() {
        let (mut context, region) = boundary_source();
        let (account, mut observer) = observer(usize::MAX);
        let boundary =
            crate::atlas::arcs::native_boundary_metered(&region, &mut context, &mut observer)
                .unwrap();
        assert_eq!(boundary.fragments().len(), 4);
        assert!(boundary.has_areal_faces());
        let live = account.live.load(Ordering::Relaxed);
        assert!(live > 0);
        let window = CurrentThreadWindow::open();
        let copy = boundary.clone();
        let parameter = boundary.fragments()[0].parameters()[0].clone();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        assert_eq!(copy.source_id(), boundary.source_id());
        let receipt = boundary.output_receipt().unwrap();
        drop(boundary);
        drop(copy);
        let baseline = context.retained_workspace_bytes();
        context.begin(1).unwrap();
        assert_eq!(context.current_workspace_bytes(), baseline);
        assert!(matches!(
            context.release_materialized_output(receipt.clone()),
            Err(GeoError::Config(_))
        ));
        assert_eq!(context.retained_workspace_bytes(), baseline);
        drop(context);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        assert_eq!(parameter.bounds().0, parameter.bounds().1);
        drop(parameter);
        drop(receipt);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn refused_boundary_metadata_has_no_partial_result_or_unowned_allocation() {
        let (mut context, region) = boundary_source();
        let (account, mut observer) = observer(0);
        let window = CurrentThreadWindow::open();
        let failure =
            crate::atlas::arcs::native_boundary_metered(&region, &mut context, &mut observer)
                .unwrap_err();
        assert!(matches!(
            failure.original(),
            GeoError::Storage(StorageError::AdmissionFailed)
        ));
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        context.begin(1).unwrap();
    }

    #[test]
    fn native_arc_view_copy_retains_original_fields_after_its_worker_dies() {
        let (account, mut observer) = observer(usize::MAX);
        let reference = crate::GeographicReference::wgs84();
        let prepared = crate::PreparedGeodesic::new(reference.clone());
        let mut context =
            MetricContext::new(reference, crate::ExecutionPolicy::geometry()).unwrap();
        let numerator = crate::Int::one().shl(512).add(&crate::Int::from_i64(17));
        let denominator = numerator.add(&crate::Int::one());
        let longitude = Rat::new(numerator, denominator).unwrap();
        let start = crate::LonLat::new(longitude, Rat::zero()).unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(Some(&mut observer));
        let view = prepared
            .prepare_azimuth_arc_view_admitted(
                &start,
                &Rat::from_i64(90),
                &crate::Metres::new(Rat::one()),
                &mut context,
                &mut progress,
            )
            .unwrap();
        assert_eq!(view.start(), &start);
        let live = account.live.load(Ordering::Relaxed);
        assert!(live > 0);
        let window = CurrentThreadWindow::open();
        let copy = view.clone();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(progress);
        drop(context);
        drop(view);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        assert_eq!(copy.start(), &start);
        drop(copy);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn refused_arc_view_never_publishes_a_raw_copy_or_retains_failed_scratch() {
        let (account, mut observer) = observer(0);
        let reference = crate::GeographicReference::wgs84();
        let prepared = crate::PreparedGeodesic::new(reference.clone());
        let mut context =
            MetricContext::new(reference, crate::ExecutionPolicy::geometry()).unwrap();
        let numerator = crate::Int::one().shl(512).add(&crate::Int::from_i64(17));
        let denominator = numerator.add(&crate::Int::one());
        let longitude = Rat::new(numerator, denominator).unwrap();
        let start = crate::LonLat::new(longitude, Rat::zero()).unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let failure = prepared
            .prepare_azimuth_arc_view_admitted(
                &start,
                &Rat::from_i64(90),
                &crate::Metres::new(Rat::one()),
                &mut context,
                &mut progress,
            )
            .unwrap_err();
        assert!(matches!(
            failure.original(),
            GeoError::Storage(StorageError::AdmissionFailed)
                | GeoError::NumericalScratch(purrdf_xsd::integer::LimbScratchError::Storage(
                    StorageError::AdmissionFailed
                ))
        ));
        assert_eq!(window.close().allocations, 0);
        drop(progress);
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_ellipsoid_derivatives_keep_original_owners_without_runtime_arithmetic() {
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(
            crate::ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: 1 << 40,
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        );
        let inverse = Rat::from_int(crate::Int::one().shl(256).add(&crate::Int::one()));
        let flattening = inverse.recip().unwrap();
        let expected = flattening.mul(&Rat::from_i64(2).sub(&flattening));
        let ellipsoid = crate::PreparedEllipsoid::new_in_budget_metered(
            Rat::from_i64(6_378_137),
            inverse,
            &mut budget,
            &mut observer,
        )
        .unwrap();
        assert_eq!(ellipsoid.eccentricity_squared_ref(), &expected);
        let live = account.live.load(Ordering::Relaxed);
        assert!(live > 0);
        let window = CurrentThreadWindow::open();
        let copy = ellipsoid.clone();
        let eccentricity = ellipsoid.eccentricity_squared();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(ellipsoid);
        drop(copy);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        assert_eq!(eccentricity, expected);
        drop(eccentricity);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_ellipsoid_refusal_precedes_arena_and_preserves_axis_validation() {
        let (account, mut denied) = observer(0);
        let mut budget = crate::PreparationBudget::new(
            crate::ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: 1 << 40,
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        );
        let inverse = Rat::from_int(crate::Int::one().shl(256).add(&crate::Int::one()));
        let window = CurrentThreadWindow::open();
        let failure = crate::PreparedEllipsoid::new_in_budget_metered(
            Rat::from_i64(6_378_137),
            inverse,
            &mut budget,
            &mut denied,
        )
        .unwrap_err();
        assert!(matches!(
            failure.original(),
            GeoError::NumericalScratch(purrdf_xsd::integer::LimbScratchError::Storage(
                StorageError::AdmissionFailed
            ))
        ));
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let (account, mut admitted) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let failure = crate::PreparedEllipsoid::new_in_budget_metered(
            Rat::zero(),
            Rat::from_i64(300),
            &mut budget,
            &mut admitted,
        )
        .unwrap_err();
        assert!(matches!(
            failure.original(),
            GeoError::InvalidEllipsoid("semimajor axis must be positive")
        ));
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_compilation_and_inverse_share_original_box_and_scalar_owners() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, HelmertParameters, OperationModel,
            OperationReference, RotationConvention, RotationLaw,
        };
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let source = OperationReference {
            realization: purrdf_hash::hex::Digest32::new([1; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        let target = OperationReference {
            realization: purrdf_hash::hex::Digest32::new([2; 32]),
            ..source
        };
        let scale = Rat::new(
            crate::Int::one().shl(256),
            crate::Int::one().shl(257).add(&crate::Int::one()),
        )
        .unwrap();
        let operation = CoordinateOperation::compile_in_budget_metered(
            source,
            target,
            OperationModel::Helmert(Box::new(HelmertParameters {
                translation: [Rat::zero(), Rat::zero(), Rat::zero()],
                rotation_arcseconds: [Rat::zero(), Rat::zero(), Rat::zero()],
                scale_ppm: scale,
                pivot: [Rat::zero(), Rat::zero(), Rat::zero()],
                law: RotationLaw::HelmertSmallAngleV1,
                convention: RotationConvention::PositionVector,
                rates: None,
                inverse: false,
            })),
            &mut budget,
            &mut observer,
        )
        .unwrap();
        let original = account.live.load(Ordering::Relaxed);
        assert!(original > 0);
        let window = CurrentThreadWindow::open();
        let copy = operation.clone();
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), original);
        let inverse = operation
            .inverse_in_budget_metered(&mut budget, &mut observer)
            .unwrap();
        assert_eq!(inverse.source(), target);
        assert_eq!(inverse.target(), source);
        assert!(matches!(inverse.model(), OperationModel::Helmert(value) if value.inverse));
        drop(operation);
        drop(copy);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        let window = CurrentThreadWindow::open();
        let last = inverse.clone();
        assert_eq!(window.close().allocations, 0);
        drop(inverse);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        drop(last);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_declared_rectangles_share_fields_and_keep_first_validation_cause() {
        use crate::operation::{Applicability, MetricSourceDomain};
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let extent = Rat::from_int(crate::Int::one().shl(256));
        let applicability = Applicability::new_in_budget_metered(
            Rat::zero(),
            extent.clone(),
            Rat::zero(),
            extent.clone(),
            &mut budget,
            &mut observer,
        )
        .unwrap();
        let domain = MetricSourceDomain::new_in_budget_metered(
            [Rat::zero(), Rat::zero()],
            [extent.clone(), extent],
            &mut budget,
            &mut observer,
        )
        .unwrap();
        let live = account.live.load(Ordering::Relaxed);
        assert!(live > 0);
        let window = CurrentThreadWindow::open();
        let copies = (applicability.clone(), domain.clone());
        assert_eq!(window.close().allocations, 0);
        drop(applicability);
        drop(domain);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(copies);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let failure = MetricSourceDomain::new_in_budget_metered(
            [Rat::one(), Rat::zero()],
            [Rat::one(), Rat::one()],
            &mut budget,
            &mut observer,
        )
        .unwrap_err();
        assert!(matches!(failure.original(), GeoError::Config(message)
            if message == "inverse source rectangle requires positive metre extents"));
        assert!(account.live.load(Ordering::Relaxed) > 0);
        let copy = failure.clone();
        drop(failure);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        drop(copy);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_polynomial_and_grid_clones_preserve_original_tables_and_order() {
        use crate::operation::{BilinearGrid, GridNode, Polynomial2d, PolynomialTerm};
        let (account, mut observer) = observer(usize::MAX);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let coefficient = Rat::from_int(crate::Int::one().shl(256));
        let polynomial = Polynomial2d::new_in_budget_metered(
            2,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: coefficient.clone(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 0,
                    y_power: 0,
                    x_coefficient: Rat::from_i64(3),
                    y_coefficient: Rat::from_i64(4),
                },
            ],
            &mut budget,
            &mut observer,
        )
        .unwrap();
        assert_eq!(polynomial.terms()[0].x_power, 0);
        assert_eq!(polynomial.terms()[1].x_coefficient, coefficient);
        let grid = BilinearGrid::new_in_budget_metered(
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            2,
            2,
            vec![
                Some(GridNode {
                    x: coefficient,
                    y: Rat::zero(),
                }),
                None,
                Some(GridNode {
                    x: Rat::from_i64(2),
                    y: Rat::from_i64(3),
                }),
                Some(GridNode {
                    x: Rat::from_i64(4),
                    y: Rat::from_i64(5),
                }),
            ],
            &mut budget,
            &mut observer,
        )
        .unwrap();
        assert!(grid.nodes()[1].is_none());
        assert_eq!(grid.nodes()[3].as_ref().unwrap().y, Rat::from_i64(5));
        let live = account.live.load(Ordering::Relaxed);
        let window = CurrentThreadWindow::open();
        let copies = (polynomial.clone(), grid.clone());
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(polynomial);
        drop(grid);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(copies);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn native_model_refusal_precedes_parameter_or_control_allocation() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationModel, OperationReference,
        };
        let (account, mut observer) = observer(0);
        let mut budget = crate::PreparationBudget::new(crate::ExecutionPolicy::geometry());
        let source = OperationReference {
            realization: purrdf_hash::hex::Digest32::new([1; 32]),
            unit: CoordinateUnit::Degrees,
            swapped_axes: false,
        };
        let target = OperationReference {
            realization: purrdf_hash::hex::Digest32::new([2; 32]),
            ..source
        };
        let window = CurrentThreadWindow::open();
        let failure = CoordinateOperation::compile_in_budget_metered(
            source,
            target,
            OperationModel::Bd09LlV1,
            &mut budget,
            &mut observer,
        )
        .unwrap_err();
        assert!(matches!(
            failure.original(),
            GeoError::Storage(StorageError::AdmissionFailed)
        ));
        assert_eq!(window.close().allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn actual_buffer_and_controls_stay_owned_through_last_shallow_clone() {
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let value = Retained::produce(
            &mut progress,
            |memory| {
                let mut bytes = Vec::new();
                memory
                    .reserve(&mut bytes, 4096)
                    .map_err(GeoError::Storage)?;
                bytes.resize(4096, 7_u8);
                Ok(bytes)
            },
            |_, _| Ok(()),
        )
        .unwrap();
        let live = account.live.load(Ordering::Relaxed);
        assert_eq!(value.admitted_bytes(), live);
        assert_eq!(window.sample().retained_bytes, i64::try_from(live).unwrap());
        let before_clone = window.sample().allocations;
        let retained = value.clone();
        assert_eq!(window.sample().allocations, before_clone);
        drop(value);
        assert_eq!(retained[4095], 7);
        assert_eq!(account.live.load(Ordering::Relaxed), live);
        drop(retained);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(
            measured.peak_working_bytes
                <= i64::try_from(account.maximum.load(Ordering::Relaxed)).unwrap()
        );
    }

    #[test]
    fn exact_error_operands_keep_original_limb_and_control_owners_through_error_clones() {
        let source = Rat::from_int(crate::Int::one().shl(512));
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let value = GeoError::diagnostic_number(&source, &mut progress).unwrap();
        let error = GeoError::CoordinateOutOfRange {
            axis: "longitude",
            value,
        };
        let GeoError::CoordinateOutOfRange { value, .. } = &error else {
            unreachable!()
        };
        assert_eq!(value.as_ref(), &source);
        let original = account.live.load(Ordering::Relaxed);
        assert!(original > 0);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(original).unwrap()
        );
        let before_clone = window.sample().allocations;
        let copy = error.clone();
        assert_eq!(window.sample().allocations, before_clone);
        drop(error);
        assert_eq!(account.live.load(Ordering::Relaxed), original);
        let GeoError::CoordinateOutOfRange { value, .. } = &copy else {
            unreachable!()
        };
        assert_eq!(value.as_ref(), &source);
        drop(copy);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn exact_error_control_refusal_precedes_operand_copy_without_replacing_its_typed_cause() {
        let source = Rat::from_int(crate::Int::one().shl(512));
        let (account, mut observer) = observer(Retained::<Rat>::allocation_layout().size() - 1);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let failure = GeoError::diagnostic_number(&source, &mut progress).unwrap_err();
        assert!(matches!(
            failure,
            GeoError::Storage(StorageError::AdmissionFailed)
        ));
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.retained_bytes, 0);
    }

    #[test]
    fn carrier_coordinate_refusal_keeps_exact_original_range_error_after_arena_destruction() {
        let source = Rat::from_int(crate::Int::one().shl(256));
        let (account, mut observer) = observer(usize::MAX);
        let error = {
            let mut admission = crate::carrier::InputAdmission::new(
                0,
                crate::ExecutionPolicy::geometry(),
                Some(&mut observer),
            )
            .unwrap();
            admission.location(source.clone(), Rat::zero()).unwrap_err()
        };
        let original = account.live.load(Ordering::Relaxed);
        assert!(original > 0);
        let GeoError::CoordinateOutOfRange { axis, value } = &error else {
            panic!("native range validation must preserve the exact original axis and operand")
        };
        assert_eq!(*axis, "longitude");
        assert_eq!(value.as_ref(), &source);
        let window = CurrentThreadWindow::open();
        let copy = error.clone();
        assert_eq!(window.close().allocations, 0);
        drop(error);
        assert_eq!(account.live.load(Ordering::Relaxed), original);
        drop(copy);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn control_refusal_happens_before_the_native_factory_or_any_allocation() {
        let (account, mut observer) = observer(Retained::<Vec<u8>>::allocation_layout().size() - 1);
        let entered = Cell::new(false);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let result = Retained::<Vec<u8>>::produce(
            &mut progress,
            |_| {
                entered.set(true);
                Ok(Vec::new())
            },
            |_, _| Ok(()),
        );
        let measured = window.close();
        assert!(matches!(
            result,
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(!entered.get());
        assert_eq!(measured.allocations, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn failed_native_factory_destroys_its_buffer_and_releases_the_original_account() {
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let result = Retained::<Vec<u8>>::produce(
            &mut progress,
            |memory| {
                let mut bytes = Vec::<u8>::new();
                memory
                    .reserve(&mut bytes, 4096)
                    .map_err(GeoError::Storage)?;
                bytes.resize(4096, 0);
                Err(GeoError::Cancelled)
            },
            |_, _| Ok(()),
        );
        let measured = window.close();
        assert!(matches!(result, Err(GeoError::Cancelled)));
        assert!(measured.allocations > 0);
        assert_eq!(measured.retained_bytes, 0);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn lexical_failure_keeps_its_original_account_through_the_last_error_clone() {
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let error = Retained::<Vec<u8>>::produce(
            &mut progress,
            |memory| {
                let mut temporary = Vec::<u8>::new();
                memory
                    .reserve(&mut temporary, 4096)
                    .map_err(GeoError::Storage)?;
                let message = memory
                    .format(&format_args!(
                        "original malformed source at position {}",
                        37
                    ))
                    .map_err(GeoError::Storage)?;
                Err(GeoError::Literal(message))
            },
            |_, _| Ok(()),
        )
        .unwrap_err();
        let GeoError::Literal(message) = error.original() else {
            panic!("the retained cause keeps its original lexical classification");
        };
        assert_eq!(message, "original malformed source at position 37");
        let retained =
            message.capacity() + SharedOwned::<GeoError>::allocation_layout::<Owner>().size();
        assert_eq!(account.live.load(Ordering::Relaxed), retained);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(retained).unwrap()
        );
        let allocations = window.sample().allocations;
        let last = error.clone();
        assert_eq!(window.sample().allocations, allocations);
        drop(error);
        assert_eq!(last.detail(), "original malformed source at position 37");
        assert!(last.is_expression_error());
        assert_eq!(account.live.load(Ordering::Relaxed), retained);
        drop(last);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn malformed_chart_destroys_original_arrays_before_settling_its_lexical_owner() {
        let source = [(0, 0), (1, 0), (0, 0)]
            .map(|(x, y)| crate::Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let error = Retained::<crate::Geometry>::produce(
            &mut progress,
            |memory| {
                let mut ring = Vec::new();
                memory
                    .reserve(&mut ring, source.len())
                    .map_err(GeoError::Storage)?;
                ring.extend(source.iter().cloned());
                let mut rings = Vec::new();
                memory.reserve(&mut rings, 1).map_err(GeoError::Storage)?;
                rings.push(ring);
                crate::Geometry::new_with_memory(
                    crate::CoordDim::Xy,
                    crate::GeometryBody::Polygon(rings),
                    memory,
                )
            },
            |_, _| Ok(()),
        )
        .unwrap_err();
        let GeoError::Literal(message) = error.original() else {
            panic!("the original ring validation remains lexical");
        };
        assert_eq!(
            message,
            "a polygon ring needs at least four positions (three corners and the repeat that closes it); this one has 3"
        );
        let retained =
            message.capacity() + SharedOwned::<GeoError>::allocation_layout::<Owner>().size();
        assert_eq!(account.live.load(Ordering::Relaxed), retained);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(retained).unwrap()
        );
        drop(error);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn native_structural_error_and_its_last_clone_keep_the_original_diagnostic_owner() {
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let error = crate::Geometry::new_with_progress(
            crate::CoordDim::Xy,
            crate::GeometryBody::Point(Some(crate::Coord::new(
                Rat::zero(),
                Rat::zero(),
                Some(Rat::zero()),
                None,
            ))),
            &mut progress,
        )
        .unwrap_err();
        let GeoError::Literal(message) = error.original() else {
            panic!("native structural validation preserves its lexical cause");
        };
        let retained =
            message.capacity() + SharedOwned::<GeoError>::allocation_layout::<Owner>().size();
        assert_eq!(account.live.load(Ordering::Relaxed), retained);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(retained).unwrap()
        );
        let allocations = window.sample().allocations;
        let last = error.clone();
        assert_eq!(window.sample().allocations, allocations);
        drop(error);
        assert!(last.is_expression_error());
        assert_eq!(account.live.load(Ordering::Relaxed), retained);
        drop(last);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn native_structural_error_admission_refusal_precedes_diagnostic_allocation() {
        let (account, mut observer) = observer(0);
        let mut progress = WorkProgress::integer(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let result = crate::Geometry::new_with_progress(
            crate::CoordDim::Xy,
            crate::GeometryBody::Point(Some(crate::Coord::new(
                Rat::zero(),
                Rat::zero(),
                Some(Rat::zero()),
                None,
            ))),
            &mut progress,
        );
        assert!(matches!(
            result,
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.retained_bytes, 0);
    }

    #[test]
    fn detached_coefficient_clone_keeps_its_original_lease_after_preparation_drop() {
        let source = purrdf_xsd::bigint::BigInt::from_decimal_digits(
            "123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
        ).unwrap();
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let prepared = Retained::produce(
            &mut progress,
            |memory| {
                source
                    .detached_with_memory(memory)
                    .map_err(GeoError::Storage)
            },
            |value, original| value.publish_detached(original).map_err(GeoError::Storage),
        )
        .unwrap();
        let before_clone = window.sample().allocations;
        let escaped = (*prepared).clone();
        assert_eq!(window.sample().allocations, before_clone);
        drop(prepared);
        assert_eq!(escaped, source);
        assert!(account.live.load(Ordering::Relaxed) > 0);
        assert!(window.sample().retained_bytes > 0);
        drop(escaped);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn wide_native_arithmetic_keeps_original_arena_through_the_last_result_clone() {
        let source = Rat::new(
            crate::Int::from_decimal_digits(
                "123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
            )
            .unwrap(),
            crate::Int::from_i64(37),
        )
        .unwrap();
        let factor = Rat::new(crate::Int::from_i64(11), crate::Int::from_i64(17)).unwrap();
        let expected = source.mul(&factor).add(&source);
        let (account, mut observer) = observer(usize::MAX);
        let required = observer.integer_scratch_layout(16, 64).unwrap();
        let window = CurrentThreadWindow::open();
        let scratch = observer.prepare_integer_scratch(16, 64).unwrap();
        assert_eq!(account.live.load(Ordering::Relaxed), required);
        let before_arithmetic = window.sample().allocations;
        let values = crate::numerical::ExactValues::admitted(&scratch);
        let original = values.original(&source).unwrap();
        let product = values.multiply(&original, &factor).unwrap();
        let result = values.add(&product, &original).unwrap();
        assert_eq!(
            values.compare(&result, &expected).unwrap(),
            core::cmp::Ordering::Equal
        );
        assert_eq!(window.sample().allocations, before_arithmetic);
        drop(original);
        drop(product);
        let escaped = result.clone();
        assert_eq!(window.sample().allocations, before_arithmetic);
        drop(result);
        drop(scratch);
        assert_eq!(account.live.load(Ordering::Relaxed), required);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(required).unwrap()
        );
        drop(escaped);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn arena_control_refusal_destroys_every_partially_built_destination() {
        let required =
            purrdf_xsd::integer::LimbScratch::required_bytes_with_storage::<Owner>(8, 32).unwrap();
        let (account, mut observer) = observer(required - 1);
        let window = CurrentThreadWindow::open();
        let result = observer.prepare_integer_scratch(8, 32);
        assert!(matches!(
            result,
            Err(GeoError::NumericalScratch(
                purrdf_xsd::integer::LimbScratchError::Storage(StorageError::AdmissionFailed)
            ))
        ));
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn growing_array_prices_overlap_and_publishes_the_original_buffer_without_copying() {
        let (account, mut observer) = observer(usize::MAX);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let mut values = RetainedBuilder::default();
        for value in 0..4096_u32 {
            values.push(value, &mut progress).unwrap();
            assert_eq!(
                account.live.load(Ordering::Relaxed),
                values.admitted_bytes()
            );
        }
        let before_publication = window.sample().allocations;
        let value = values.finish().unwrap();
        let cloned = value.clone();
        assert_eq!(window.sample().allocations, before_publication);
        drop(value);
        assert_eq!(cloned[4095], 4095);
        assert_eq!(
            account.live.load(Ordering::Relaxed),
            cloned.admitted_bytes()
        );
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(cloned.admitted_bytes()).unwrap()
        );
        drop(cloned);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        let measured = window.close();
        assert_eq!(measured.retained_bytes, 0);
        assert!(
            measured.peak_working_bytes
                <= i64::try_from(account.maximum.load(Ordering::Relaxed)).unwrap()
        );
    }

    #[test]
    fn growing_array_refusal_preserves_original_cells_and_releases_replacement_metadata() {
        let control = Retained::<Vec<u64>>::allocation_layout().size()
            + SharedOwned::<()>::allocation_layout::<Owner>().size();
        let limit = 2 * control + 3 * size_of::<u64>() - 1;
        let (account, mut observer) = observer(limit);
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let mut values = RetainedBuilder::default();
        values.push(7_u64, &mut progress).unwrap();
        let original = values.admitted_bytes();
        assert!(matches!(
            values.push(11_u64, &mut progress),
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(&*values, &[7]);
        assert_eq!(account.live.load(Ordering::Relaxed), original);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(original).unwrap()
        );
        drop(values);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn receipt_clones_keep_original_token_after_registry_and_context_destruction() {
        let (account, mut observer) = observer(usize::MAX);
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let receipt = context
            .register_materialized_output(0, &mut progress)
            .unwrap();
        let token = usize::try_from(receipt.workspace_bytes()).unwrap();
        let total = token + usize::try_from(context.materialized_receipt_storage_bytes).unwrap();
        assert_eq!(account.live.load(Ordering::Relaxed), total);
        assert_eq!(
            window.sample().retained_bytes,
            i64::try_from(total).unwrap()
        );
        let before_clone = window.sample().allocations;
        let clone = receipt.clone();
        assert_eq!(window.sample().allocations, before_clone);
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), token);
        drop(receipt);
        assert_eq!(account.live.load(Ordering::Relaxed), token);
        drop(clone);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn initial_registry_refusal_destroys_token_and_restores_original_policy_baseline() {
        let token = Retained::<()>::allocation_layout().size()
            + SharedOwned::<()>::allocation_layout::<Owner>().size();
        let table = Retained::<Vec<crate::MaterializedOutputReceipt>>::allocation_layout().size()
            + SharedOwned::<()>::allocation_layout::<Owner>().size()
            + size_of::<crate::MaterializedOutputReceipt>();
        let (account, mut observer) = observer(token + table - 1);
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        assert!(matches!(
            context.register_materialized_output(0, &mut progress),
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(context.current_workspace_bytes(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert!(context.materialized_outputs.is_empty());
        assert_eq!(window.close().retained_bytes, 0);
    }

    #[test]
    fn grown_registry_refusal_preserves_original_receipt_and_exact_native_owners() {
        let control = SharedOwned::<()>::allocation_layout::<Owner>().size();
        let token = Retained::<()>::allocation_layout().size() + control;
        let table_control =
            Retained::<Vec<crate::MaterializedOutputReceipt>>::allocation_layout().size() + control;
        let initial = token + table_control + size_of::<crate::MaterializedOutputReceipt>();
        let replacement = table_control + 2 * size_of::<crate::MaterializedOutputReceipt>();
        let (account, mut observer) = observer(initial + token + replacement - 1);
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(Some(&mut observer));
        let window = CurrentThreadWindow::open();
        let original = context
            .register_materialized_output(0, &mut progress)
            .unwrap();
        let baseline = context.retained_workspace_bytes();
        assert_eq!(account.live.load(Ordering::Relaxed), initial);
        assert!(matches!(
            context.register_materialized_output(0, &mut progress),
            Err(GeoError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(context.materialized_outputs.len(), 1);
        assert_eq!(context.current_workspace_bytes(), baseline);
        assert_eq!(context.retained_workspace_bytes(), baseline);
        assert_eq!(account.live.load(Ordering::Relaxed), initial);
        context.release_materialized_output(original).unwrap();
        // The last receipt release also destroys the empty registry. Neither
        // its buffer nor its control is retained for hypothetical future work.
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(context.current_workspace_bytes(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        drop(context);
        assert_eq!(account.live.load(Ordering::Relaxed), 0);
        assert_eq!(window.close().retained_bytes, 0);
    }
}
