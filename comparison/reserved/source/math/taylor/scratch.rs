// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit reusable coefficient storage. A checkout holds no mutex guard.

use core::{alloc::Layout, fmt};
use purrdf_lex::allocation::{Admission, Memory, Resident, SharedOwned, StorageError};
use std::sync::Mutex;

use super::super::{FixedInterval, MathError};

/// A reusable Taylor pool cannot be checked out simultaneously.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaylorScratchError {
    /// Another branded workspace already owns this pool's buffers.
    InUse,
    /// The native pool initializer must have two exact zero endpoints.
    InvalidInitializer,
    /// Checked native layout, physical allocator or original admission refusal.
    Storage(StorageError),
}
impl purrdf_hash::error::StructuredError for TaylorScratchError {
    purrdf_hash::error_visit! {
        InUse => "taylor-scratch-in-use" {},
        InvalidInitializer => "taylor-scratch-invalid-initializer" {},
        Storage(error) => "taylor-scratch-storage" { "cause" => Cause(error) },
    }
}

impl fmt::Display for TaylorScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InUse => f.write_str("Taylor scratch already checked out"),
            Self::InvalidInitializer => f.write_str("Taylor initializer must be zero"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for TaylorScratchError {}

pub(super) struct ScratchBuffers {
    pub(super) coefficients: Box<[FixedInterval]>,
    pub(super) generations: Box<[u64]>,
}

struct ScratchInner {
    coefficients: usize,
    jets: usize,
    buffers: Mutex<Option<ScratchBuffers>>,
    bytes: usize,
}

/// Immutable-capacity worker pool for branded Taylor coefficient workspaces.
/// Its buffers are returned and cleared on success, refusal and unwinding.
#[derive(Clone)]
pub struct TaylorScratch(SharedOwned<ScratchInner>);

impl fmt::Debug for TaylorScratch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaylorScratch")
            .field("coefficients", &self.0.coefficients)
            .field("jets", &self.0.jets)
            .finish_non_exhaustive()
    }
}

impl TaylorScratch {
    pub(super) fn new(
        coefficients: usize,
        jets: usize,
        zero: FixedInterval,
    ) -> Result<Self, MathError> {
        Self::new_with_storage(coefficients, jets, zero, Resident)
            .map_err(|(error, _)| MathError::TaylorScratch(error))
    }

    /// Prepare immutable-capacity coefficient buffers with their original owner.
    /// The initializer must be the exact zero at the workspace's precision;
    /// zero endpoints clone without allocating limbs. Checkout and return use
    /// these same buffers without holding a lock during arithmetic or callbacks.
    ///
    /// # Errors
    /// Returns the original storage with a domain or physical refusal, after
    /// destroying every partial buffer. No resident replacement is allocated.
    pub fn new_with_storage<S: Admission + Send + Sync + 'static>(
        coefficients: usize,
        jets: usize,
        zero: FixedInterval,
        mut storage: S,
    ) -> Result<Self, (TaylorScratchError, S)> {
        if !zero.lower().is_zero() || !zero.upper().is_zero() {
            return Err((TaylorScratchError::InvalidInitializer, storage));
        }
        let Some(required) = Self::required_bytes_with_storage::<S>(coefficients, jets) else {
            return Err((
                TaylorScratchError::Storage(StorageError::SizeOverflow),
                storage,
            ));
        };
        let mut memory = Memory::new(&mut storage);
        let built = memory.scope(|memory| {
            let mut values = Vec::new();
            memory.reserve(&mut values, coefficients)?;
            values.resize(coefficients, zero);
            let mut generations = Vec::new();
            memory.reserve(&mut generations, jets)?;
            generations.resize(jets, 0);
            let values = memory.boxed_slice(values)?;
            let generations = memory.boxed_slice(generations)?;
            memory.add_bytes(SharedOwned::<ScratchInner>::allocation_layout::<S>().size())?;
            debug_assert_eq!(memory.admitted_bytes(), required);
            Ok(ScratchInner {
                coefficients,
                jets,
                buffers: Mutex::new(Some(ScratchBuffers {
                    coefficients: values,
                    generations,
                })),
                bytes: required,
            })
        });
        let value = match built {
            Ok(value) => value,
            Err(error) => return Err((TaylorScratchError::Storage(error), storage)),
        };
        let mut pending = Some((value, storage));
        match SharedOwned::try_from_admitted_with(|| {
            pending.take().expect("one admitted Taylor publication")
        }) {
            Ok(value) => Ok(Self(value)),
            Err(_) => {
                let (value, storage) = pending.take().expect("refused Taylor remains owned");
                drop(value);
                Err((
                    TaylorScratchError::Storage(StorageError::AllocationFailed),
                    storage,
                ))
            }
        }
    }

    pub(super) fn supports(&self, coefficients: usize, jets: usize) -> bool {
        self.0.coefficients >= coefficients && self.0.jets >= jets
    }

    /// Actual reusable heap, including buffer and shared ownership headers.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.0.bytes
    }

    /// Whether two worker handles borrow the same retained allocation.
    #[must_use]
    pub fn shares_storage(&self, other: &Self) -> bool {
        core::ptr::eq(self.0.as_ref(), other.0.as_ref())
    }

    pub(super) fn required_bytes(coefficients: usize, jets: usize) -> Option<usize> {
        Self::required_bytes_with_storage::<Resident>(coefficients, jets)
    }

    /// Complete actual buffer/control layout including the original owner.
    ///
    /// Returns None when an array or sum cannot be represented by the platform.
    #[must_use]
    pub fn required_bytes_with_storage<S>(coefficients: usize, jets: usize) -> Option<usize> {
        Layout::array::<FixedInterval>(coefficients)
            .ok()?
            .size()
            .checked_add(Layout::array::<u64>(jets).ok()?.size())?
            .checked_add(SharedOwned::<ScratchInner>::allocation_layout::<S>().size())
    }

    pub(super) fn checkout(&self) -> Result<ScratchBuffers, MathError> {
        let mut buffers = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = buffers
            .take()
            .ok_or(MathError::TaylorScratch(TaylorScratchError::InUse));
        drop(buffers);
        result
    }

    pub(super) fn return_buffers(&self, storage: ScratchBuffers) {
        let previous = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(storage);
        debug_assert!(previous.is_none(), "one exclusive Taylor checkout");
    }

    pub(super) fn available(&self) -> Result<(), MathError> {
        let buffers = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = if buffers.is_some() {
            Ok(())
        } else {
            Err(MathError::TaylorScratch(TaylorScratchError::InUse))
        };
        drop(buffers);
        result
    }
}

#[cfg(test)]
mod native_owner_tests {
    use super::{TaylorScratch, TaylorScratchError};
    use crate::bigint::scratch::native_owner_tests::Storage;
    use crate::math::{CoordinateMath, FixedInterval, MathLimits};
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_lex::allocation::StorageError;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    fn zero() -> FixedInterval {
        let mut math = CoordinateMath::new(MathLimits {
            precision_bits: 32,
            max_work: 1_000_000,
            max_workspace_bytes: 8_388_608,
        })
        .unwrap();
        FixedInterval::from_i64(0, &mut math).unwrap()
    }

    #[test]
    fn original_taylor_refusal_precedes_every_buffer_birth() {
        let zero = zero();
        let live = Arc::new(AtomicUsize::new(0));
        let window = CurrentThreadWindow::open();
        let Err((error, storage)) = TaylorScratch::new_with_storage(
            16,
            3,
            zero,
            Storage {
                live: Arc::clone(&live),
                limit: 0,
                first_refused: None,
            },
        ) else {
            panic!("unadmitted coefficient pool was published")
        };
        assert_eq!(
            error,
            TaylorScratchError::Storage(StorageError::AdmissionFailed)
        );
        assert!(storage.first_refused.is_some());
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(window.close().allocations, 0);
    }

    #[test]
    fn native_taylor_original_owner_survives_all_shallow_handle_clones() {
        let live = Arc::new(AtomicUsize::new(0));
        let pool = match TaylorScratch::new_with_storage(
            16,
            3,
            zero(),
            Storage {
                live: Arc::clone(&live),
                limit: usize::MAX,
                first_refused: None,
            },
        ) {
            Ok(pool) => pool,
            Err((error, _)) => panic!("native coefficient birth refused: {error}"),
        };
        let bytes = TaylorScratch::required_bytes_with_storage::<Storage>(16, 3).unwrap();
        assert_eq!(pool.retained_bytes(), bytes);
        assert_eq!(live.load(Ordering::SeqCst), bytes);
        let window = CurrentThreadWindow::open();
        let retained = pool.clone();
        drop(pool);
        assert_eq!(retained.retained_bytes(), bytes);
        assert_eq!(live.load(Ordering::SeqCst), bytes);
        assert_eq!(window.close().allocations, 0);
        drop(retained);
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unrepresentable_taylor_layout_never_calls_the_factory_or_original_admission() {
        let zero = zero();
        let live = Arc::new(AtomicUsize::new(0));
        let window = CurrentThreadWindow::open();
        let Err((error, storage)) = TaylorScratch::new_with_storage(
            usize::MAX,
            3,
            zero,
            Storage {
                live: Arc::clone(&live),
                limit: usize::MAX,
                first_refused: None,
            },
        ) else {
            panic!("unrepresentable coefficient pool was published")
        };
        assert_eq!(
            error,
            TaylorScratchError::Storage(StorageError::SizeOverflow)
        );
        assert_eq!(storage.first_refused, None);
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(window.close().allocations, 0);
    }
}
