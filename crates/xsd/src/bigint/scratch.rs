// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit reusable integer limb storage. No thread-local or global allocator.

use core::alloc::Layout;
use core::fmt;
use purrdf_lex::allocation::{
    Admission, Memory, Resident, SharedOwned, StorageError, shared::SharedBuffer,
};
use std::sync::Mutex;

use super::storage::Mag;

/// A bounded integer destination could not be checked out. No fallback heap
/// allocation is made after a scratch arena has been constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimbScratchError {
    /// An upcoming complete working set needs more simultaneous destinations.
    /// This is detected before constructing that working set.
    Destinations {
        /// Total destinations required, including values already live.
        required: usize,
        /// Total destinations in the installed arena.
        admitted: usize,
    },
    /// The requested destination exceeds every admitted buffer's capacity.
    Capacity {
        /// Complete limbs needed by this exact destination.
        required_limbs: usize,
        /// Maximum limbs admitted per reusable destination.
        admitted_limbs: usize,
    },
    /// All admitted buffers remain borrowed by live integer values.
    Exhausted {
        /// Number of admitted destinations, all currently held by live values.
        destinations: usize,
    },
    /// Replacing an arena would discard the receipt for live immutable values.
    Retained {
        /// Destinations still held by one or more immutable values.
        held_destinations: usize,
        /// Total destinations in the arena whose storage must remain admitted.
        destinations: usize,
    },
    /// Arena sizing overflows the platform's addressable storage.
    SizeOverflow,
    /// Physical construction failed before any arena payload was published.
    Storage(StorageError),
    /// The allocator refused an explicitly fallible native destination.
    AllocationFailed,
}

impl purrdf_hash::error::StructuredError for LimbScratchError {
    purrdf_hash::error_visit! {
        Destinations { required, admitted } => "limb-scratch-destinations" {
            "required" => Unsigned(*required as u128), "admitted" => Unsigned(*admitted as u128),
        },
        Capacity { required_limbs, admitted_limbs } => "limb-scratch-capacity" {
            "required_limbs" => Unsigned(*required_limbs as u128),
            "admitted_limbs" => Unsigned(*admitted_limbs as u128),
        },
        Exhausted { destinations } => "limb-scratch-exhausted" { "destinations" => Unsigned(*destinations as u128) },
        Retained { held_destinations, destinations } => "limb-scratch-retained" {
            "held_destinations" => Unsigned(*held_destinations as u128),
            "destinations" => Unsigned(*destinations as u128),
        },
        SizeOverflow => "limb-scratch-size-overflow" {},
        Storage(error) => "limb-scratch-storage" { "cause" => Cause(error) },
        AllocationFailed => "limb-scratch-allocation-failed" {},
    }
}

impl fmt::Display for LimbScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Destinations { required, admitted } => write!(
                f,
                "integer working set requires {required} destinations; admitted {admitted}"
            ),
            Self::Capacity {
                required_limbs,
                admitted_limbs,
            } => write!(
                f,
                "integer scratch requires {required_limbs} limbs; admitted {admitted_limbs}"
            ),
            Self::Exhausted { destinations } => write!(
                f,
                "all {destinations} integer scratch destinations are held"
            ),
            Self::Retained {
                held_destinations,
                destinations,
            } => write!(
                f,
                "cannot replace integer scratch while {held_destinations} of {destinations} destinations are held"
            ),
            Self::SizeOverflow => f.write_str("integer scratch sizing overflow"),
            Self::Storage(error) => error.fmt(f),
            Self::AllocationFailed => f.write_str("integer destination allocation failed"),
        }
    }
}
impl std::error::Error for LimbScratchError {}

#[derive(Debug)]
struct FreeBuffers {
    buffers: Vec<SharedBuffer<u64>>,
    limbs: usize,
    bytes: usize,
    handles: usize,
}

/// Preallocated reusable destinations for one worker's exact arithmetic.
///
/// Destinations share one preallocated limb array and one control-slot array.
/// Cloning a returned integer shares its immutable range; its last drop
/// returns the destination. The original arena owner remains live even when
/// integers outlive this handle. Checkout and return hold a private mutex only
/// while moving a buffer; no arithmetic, caller callback, or suspension occurs
/// under that lock.
#[derive(Debug)]
pub struct LimbScratch {
    owner: Option<SharedOwned<Mutex<FreeBuffers>>>,
}

impl LimbScratch {
    /// Use the native three-limb inline destinations without a heap owner.
    /// A larger destination is refused; this capability never allocates a
    /// fallback buffer or changes the original arithmetic implementation.
    #[must_use]
    pub const fn inline() -> Self {
        Self { owner: None }
    }

    /// Checked complete native layout for a caller-owned resident arena.
    ///
    /// # Errors
    /// Refuses overflowing arrays and live-byte sums before allocation.
    pub fn required_bytes(buffers: usize, limbs: usize) -> Result<usize, LimbScratchError> {
        Self::required_bytes_with_storage::<Resident>(buffers, limbs)
            .map_err(|_| LimbScratchError::SizeOverflow)
    }

    /// Checked complete arena layout including the concrete original owner.
    /// No guessed ownership header or allocator multiplier participates.
    ///
    /// # Errors
    /// Refuses overflowing arrays and live-byte sums before allocation.
    pub fn required_bytes_with_storage<S>(
        buffers: usize,
        limbs: usize,
    ) -> Result<usize, StorageError> {
        SharedBuffer::<u64>::required_bytes(buffers, limbs)?
            .checked_add(SharedOwned::<Mutex<FreeBuffers>>::allocation_layout::<S>().size())
            .ok_or(StorageError::SizeOverflow)
    }

    /// Allocate all destinations once through the same native physical body.
    /// Subsequent integer `_in` operations allocate neither vectors nor headers.
    ///
    /// # Errors
    /// Refuses overflowing layouts or a physical allocator refusal.
    pub fn new(buffers: usize, limbs: usize) -> Result<Self, LimbScratchError> {
        Self::required_bytes(buffers, limbs)?;
        Self::new_with_storage(buffers, limbs, Resident)
            .map_err(|(error, _)| LimbScratchError::Storage(error))
    }

    /// Construct an arena under the original owned admission capability.
    /// Immutable values retain that capability through their final drop, even
    /// after the worker drops this handle. The callback is invoked only before
    /// actual construction and never under the arena's private mutex.
    ///
    /// # Errors
    /// Returns the physical failure and the caller's original storage together.
    /// Every partially built payload dies before its grant is refunded or moved
    /// into this error; no ordinary heap fallback is attempted.
    pub fn new_with_storage<S: Admission + Send + Sync + 'static>(
        buffers: usize,
        limbs: usize,
        mut storage: S,
    ) -> Result<Self, (StorageError, S)> {
        let required = match Self::required_bytes_with_storage::<S>(buffers, limbs) {
            Ok(bytes) => bytes,
            Err(error) => return Err((error, storage)),
        };
        let mut memory = Memory::new(&mut storage);
        let built = memory.scope(|memory| {
            let free = SharedBuffer::new_array_with_memory(buffers, limbs, 0, memory)?;
            memory.add_bytes(SharedOwned::<Mutex<FreeBuffers>>::allocation_layout::<S>().size())?;
            debug_assert_eq!(memory.admitted_bytes(), required);
            Ok(Mutex::new(FreeBuffers {
                buffers: free,
                limbs,
                bytes: required,
                handles: 1,
            }))
        });
        let value = match built {
            Ok(value) => value,
            Err(error) => return Err((error, storage)),
        };
        // The factory borrows this option. A refused control allocation cannot
        // take or destroy the original storage before its typed cause is selected.
        let mut pending = Some((value, storage));
        match SharedOwned::try_from_admitted_with(|| {
            pending.take().expect("one admitted arena publication")
        }) {
            Ok(owner) => Ok(Self { owner: Some(owner) }),
            Err(_) => {
                let (value, storage) = pending.take().expect("refused arena remains owned");
                drop(value);
                Err((StorageError::AllocationFailed, storage))
            }
        }
    }

    /// Heap retained by the whole arena, including checked-out destinations.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.owner.as_ref().map_or(0, |owner| {
            owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .bytes
        })
    }

    /// Maximum admitted limbs in one reusable destination.
    #[must_use]
    pub fn limb_capacity(&self) -> usize {
        self.owner.as_ref().map_or(3, |owner| {
            owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .limbs
        })
    }

    /// Total admitted reusable destinations, including checked-out buffers.
    #[must_use]
    pub fn destination_capacity(&self) -> usize {
        self.owner.as_ref().map_or(0, |owner| {
            owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .buffers
                .capacity()
        })
    }

    /// Number of free destinations. Immutable clones share their destination.
    #[must_use]
    pub fn available(&self) -> usize {
        self.owner.as_ref().map_or(0, |owner| {
            owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .buffers
                .len()
        })
    }

    /// Admit a complete upcoming working set without allocating any payload.
    /// Existing immutable values remain charged to their original arena.
    ///
    /// # Errors
    /// Reports the complete required shape if available destinations do not fit.
    pub fn require_available(&self, additional: usize) -> Result<(), LimbScratchError> {
        if additional == 0 {
            return Ok(());
        }
        let Some(owner) = &self.owner else {
            return Err(LimbScratchError::Destinations {
                required: additional,
                admitted: 0,
            });
        };
        let free = owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if additional <= free.buffers.len() {
            return Ok(());
        }
        let admitted = free.buffers.capacity();
        let required = (admitted - free.buffers.len())
            .checked_add(additional)
            .ok_or(LimbScratchError::SizeOverflow)?;
        drop(free);
        Err(LimbScratchError::Destinations { required, admitted })
    }

    /// Whether dropping this handle destroys the actual complete arena.
    /// All destinations must have returned and no worker handle may remain.
    /// Inline-only capabilities own no allocation.
    #[must_use]
    pub fn is_unused(&self) -> bool {
        self.owner.as_ref().is_none_or(|owner| {
            let free = owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            free.handles == 1 && free.buffers.len() == free.buffers.capacity()
        })
    }

    pub(super) fn destination(
        &self,
        capacity: usize,
        length: usize,
    ) -> Result<Mag, LimbScratchError> {
        if length > capacity {
            return Err(LimbScratchError::SizeOverflow);
        }
        if capacity <= 3 {
            return Ok(Mag::zeroed(length));
        }
        let owner = self.owner.as_ref().ok_or(LimbScratchError::Capacity {
            required_limbs: capacity,
            admitted_limbs: 3,
        })?;
        let mut free = owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if capacity > free.limbs {
            return Err(LimbScratchError::Capacity {
                required_limbs: capacity,
                admitted_limbs: free.limbs,
            });
        }
        let mut words = free
            .buffers
            .pop()
            .ok_or_else(|| LimbScratchError::Exhausted {
                destinations: free.buffers.capacity(),
            })?;
        let mut unique = words.get_mut().expect("free destinations have one owner");
        unique.clear();
        unique.resize(length, 0);
        drop(free);
        Ok(Mag::Pooled(Pooled {
            words: Some(words),
            owner: owner.clone(),
        }))
    }
}

impl Clone for LimbScratch {
    fn clone(&self) -> Self {
        let owner = self.owner.clone();
        if let Some(owner) = &owner {
            let mut free = owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            free.handles = free
                .handles
                .checked_add(1)
                .expect("integer arena handle overflow");
        }
        Self { owner }
    }
}

impl Drop for LimbScratch {
    fn drop(&mut self) {
        if let Some(owner) = &self.owner {
            let mut free = owner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            free.handles = free
                .handles
                .checked_sub(1)
                .expect("live integer arena handle");
        }
        // The lock is released before the shared payload and its original
        // physical grant are destroyed by the field's ordinary drop.
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Pooled {
    words: Option<SharedBuffer<u64>>,
    owner: SharedOwned<Mutex<FreeBuffers>>,
}

impl Pooled {
    pub(super) fn words(&self) -> &[u64] {
        self.words.as_deref().expect("live destination")
    }
    pub(super) fn words_mut(&mut self) -> &mut [u64] {
        self.words
            .as_mut()
            .expect("live destination")
            .get_mut()
            .expect("only a unique arithmetic destination is mutated")
            .into_slice()
    }
    pub(super) fn push(&mut self, word: u64) {
        self.words
            .as_mut()
            .expect("live destination")
            .get_mut()
            .expect("only a unique arithmetic destination is mutated")
            .push(word);
    }
    pub(super) fn pop(&mut self) -> Option<u64> {
        self.words
            .as_mut()
            .expect("live destination")
            .get_mut()
            .expect("only a unique arithmetic destination is mutated")
            .pop()
    }
    pub(super) fn allocated_bytes(&self) -> usize {
        // A surviving immutable destination retains its complete original
        // arena. This introspection is never used as a physical birth receipt.
        self.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .bytes
    }
}

impl Drop for Pooled {
    fn drop(&mut self) {
        let Some(mut words) = self.words.take() else {
            return;
        };
        // Serialize uniqueness and decrement. Concurrent final clones must
        // return exactly one slot; both keep the original arena owner live.
        let mut free = self
            .owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(mut unique) = words.get_mut() {
            unique.clear();
            debug_assert!(free.buffers.len() < free.buffers.capacity());
            free.buffers.push(words);
        } else {
            drop(words);
        }
        drop(free);
    }
}

pub(crate) trait Allocate {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError>;

    fn push(&self, destination: &mut Mag, word: u64) -> Result<(), LimbScratchError> {
        destination.push(word);
        Ok(())
    }

    fn copy(&self, words: &[u64]) -> Result<Mag, LimbScratchError> {
        let mut result = self.destination(words.len(), words.len())?;
        result.copy_from_slice(words);
        Ok(result)
    }
}

/// A comparison supplies its arena only after the original branch selects products.
/// Borrowed sign, denominator and numerator scans never construct an arena.
#[derive(Default)]
pub(crate) struct ProvidedScratch(core::cell::RefCell<Option<LimbScratch>>);

impl ProvidedScratch {
    pub(crate) fn within<T>(&self, scratch: Option<&LimbScratch>, body: impl FnOnce() -> T) -> T {
        struct Reset<'a>(&'a core::cell::RefCell<Option<LimbScratch>>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().take();
            }
        }
        self.0.replace(scratch.cloned());
        let reset = Reset(&self.0);
        let result = body();
        drop(reset);
        result
    }
}

impl Allocate for ProvidedScratch {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError> {
        if let Some(scratch) = self.0.borrow().as_ref() {
            scratch.destination(capacity, length)
        } else if capacity <= 3 && length <= capacity {
            Unbounded.destination(capacity, length)
        } else {
            Err(LimbScratchError::Capacity {
                required_limbs: capacity,
                admitted_limbs: 0,
            })
        }
    }
}

impl Allocate for LimbScratch {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError> {
        self.destination(capacity, length)
    }
}

pub(crate) struct Unbounded;
impl Allocate for Unbounded {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError> {
        let mut result = Mag::with_capacity(capacity);
        result.resize(length, 0);
        Ok(result)
    }
}

pub(crate) struct Fallible;
impl Allocate for Fallible {
    fn destination(&self, capacity: usize, length: usize) -> Result<Mag, LimbScratchError> {
        if length > capacity {
            return Err(LimbScratchError::SizeOverflow);
        }
        Layout::array::<u64>(capacity).map_err(|_| LimbScratchError::SizeOverflow)?;
        if capacity <= 4 && length <= 3 {
            let mut result = Mag::new();
            result.resize(length, 0);
            return Ok(result);
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(capacity)
            .map_err(|_| LimbScratchError::AllocationFailed)?;
        words.resize(length, 0);
        Ok(Mag::Heap(words))
    }

    fn push(&self, destination: &mut Mag, word: u64) -> Result<(), LimbScratchError> {
        match destination {
            Mag::Inline { words, length } if length.get() == 4 => {
                let mut heap = Vec::new();
                heap.try_reserve_exact(4)
                    .map_err(|_| LimbScratchError::AllocationFailed)?;
                heap.extend_from_slice(words);
                heap.push(word);
                *destination = Mag::Heap(heap);
            }
            Mag::Heap(words) => {
                if words.len() == words.capacity() {
                    words
                        .try_reserve_exact(1)
                        .map_err(|_| LimbScratchError::AllocationFailed)?;
                }
                words.push(word);
            }
            Mag::Inline { .. } => destination.push(word),
            Mag::Shared(_) | Mag::Owned(_) | Mag::Pooled(_) => {
                unreachable!("fallible parse destinations are fresh inline or heap magnitudes");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{LimbScratch, LimbScratchError};

    #[test]
    fn working_set_preflight_counts_shared_live_values_without_allocating() {
        let scratch = LimbScratch::new(4, 16).unwrap();
        let value = BigInt::one().shl_in(400, &scratch).unwrap();
        let shared = value.clone();
        let measured = CurrentThreadWindow::open();
        assert_eq!(scratch.require_available(3), Ok(()));
        assert_eq!(
            scratch.require_available(4),
            Err(LimbScratchError::Destinations {
                required: 5,
                admitted: 4,
            })
        );
        assert_eq!(
            scratch.require_available(usize::MAX),
            Err(LimbScratchError::SizeOverflow)
        );
        drop(value);
        assert_eq!(scratch.available(), 3);
        drop(shared);
        assert_eq!(scratch.require_available(4), Ok(()));
        assert_eq!(LimbScratch::inline().require_available(0), Ok(()));
        assert_eq!(
            LimbScratch::inline().require_available(1),
            Err(LimbScratchError::Destinations {
                required: 1,
                admitted: 0,
            })
        );
        assert_eq!(measured.close().allocations, 0);
    }

    #[test]
    fn full_worker_arena_has_constant_birth_allocations_and_exact_live_layout() {
        let measured = CurrentThreadWindow::open();
        let scratch = LimbScratch::new(768, 64).unwrap();
        let bytes = scratch.retained_bytes();
        let born = measured.sample();
        assert_eq!(born.allocations, 5);
        assert_eq!(born.retained_bytes, i64::try_from(bytes).unwrap());
        assert_eq!(scratch.available(), 768);
        let value = BigInt::one().shl_in(300, &scratch).unwrap();
        let escaped = value.clone();
        assert_eq!(measured.sample().allocations, 5);
        drop(value);
        drop(scratch);
        assert_eq!(
            measured.sample().retained_bytes,
            i64::try_from(bytes).unwrap()
        );
        assert_eq!(escaped.bit_len(), 301);
        drop(escaped);
        assert_eq!(measured.close().retained_bytes, 0);
    }

    #[test]
    fn inline_capability_executes_the_native_product_and_division_without_heap_storage() {
        let scratch = LimbScratch::inline();
        let a = BigInt::from_i128(i128::from(u64::MAX));
        let b = BigInt::from_i128(17);
        let expected = a.mul(&b);
        let window = CurrentThreadWindow::open();
        let product = a.mul_in(&b, &scratch).unwrap();
        let (quotient, remainder) = product.div_rem_in(&b, &scratch).unwrap().unwrap();
        let allocations = window.close();
        assert_eq!(product, expected);
        assert_eq!(quotient, a);
        assert!(remainder.is_zero());
        assert_eq!(allocations.allocations, 0);
        assert_eq!(scratch.retained_bytes(), 0);
        assert!(matches!(
            a.shl_in(400, &scratch),
            Err(LimbScratchError::Capacity {
                admitted_limbs: 3,
                ..
            })
        ));
    }

    #[test]
    fn unused_arena_requires_the_last_worker_handle_and_last_immutable_destination() {
        let scratch = LimbScratch::new(4, 16).unwrap();
        assert!(scratch.is_unused());
        let worker = scratch.clone();
        assert!(!scratch.is_unused());
        drop(worker);
        assert!(scratch.is_unused());
        let value = BigInt::one().shl_in(400, &scratch).unwrap();
        let value_copy = value.clone();
        assert!(!scratch.is_unused());
        drop(value);
        assert!(!scratch.is_unused());
        drop(value_copy);
        assert!(scratch.is_unused());
    }
    use crate::bigint::BigInt;
    use purrdf_alloc_probe::CurrentThreadWindow;

    #[test]
    fn bounded_original_arithmetic_matches_unbounded_without_allocating() {
        let scratch = LimbScratch::new(32, 32).unwrap();
        let a = BigInt::one().shl(399).add(&BigInt::from_u64(719));
        let b = BigInt::one().shl(201).sub(&BigInt::from_u64(37));
        let expected = (
            a.add(&b),
            a.sub(&b),
            a.mul(&b),
            a.div_rem(&b),
            a.gcd(&b),
            a.sqrt_floor(),
            a.shl(73),
            a.shr(93),
            BigInt::pow10(100),
        );
        let expected_negative = expected.0.neg();
        let a = a.copy_in(&scratch).unwrap();
        let b = b.copy_in(&scratch).unwrap();
        let window = CurrentThreadWindow::open();
        for _ in 0..16 {
            let actual = (
                a.add_in(&b, &scratch).unwrap(),
                a.sub_in(&b, &scratch).unwrap(),
                a.mul_in(&b, &scratch).unwrap(),
                a.div_rem_in(&b, &scratch).unwrap(),
                a.gcd_in(&b, &scratch).unwrap(),
                a.sqrt_floor_in(&scratch).unwrap(),
                a.shl_in(73, &scratch).unwrap(),
                a.shr_in(93, &scratch).unwrap(),
                BigInt::pow10_in(100, &scratch).unwrap(),
            );
            assert_eq!(actual, expected);
            assert_eq!(
                a.neg().add_in(&b.neg(), &scratch).unwrap(),
                expected_negative
            );
        }
        assert_eq!(window.close().allocations, 0);
        assert_eq!(scratch.available(), 30);
    }

    #[test]
    fn held_immutable_values_refuse_without_fallback_and_return_last_slot() {
        let scratch = LimbScratch::new(1, 8).unwrap();
        let original = BigInt::one().shl(300);
        let value = original.copy_in(&scratch).unwrap();
        let clone = value.clone();
        assert_eq!(scratch.available(), 0);
        assert_eq!(
            original.copy_in(&scratch),
            Err(LimbScratchError::Exhausted { destinations: 1 })
        );
        drop(value);
        assert_eq!(scratch.available(), 0);
        drop(clone);
        assert_eq!(scratch.available(), 1);
        assert_eq!(
            original.shl_in(1024, &scratch),
            Err(LimbScratchError::Capacity {
                required_limbs: 21,
                admitted_limbs: 8
            })
        );
        assert_eq!(scratch.available(), 1);
        assert_eq!(
            BigInt::zero().div_rem_in(&BigInt::zero(), &scratch),
            Ok(None)
        );
    }

    #[test]
    fn values_outlive_context_and_concurrent_final_drops_recycle_destination() {
        let scratch = LimbScratch::new(1, 8).unwrap();
        let original = BigInt::one().shl(300).neg();
        let value = original.copy_in(&scratch).unwrap();
        let survivor = value.clone();
        drop(scratch.clone());
        assert_eq!(survivor, original);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let cloned_barrier = std::sync::Arc::clone(&barrier);
        let handle = std::thread::spawn(move || {
            cloned_barrier.wait();
            drop(value);
        });
        barrier.wait();
        drop(survivor);
        handle.join().unwrap();
        assert_eq!(scratch.available(), 1);
        let value = original.copy_in(&scratch).unwrap();
        drop(scratch);
        assert_eq!(value, original);
    }
}

#[cfg(test)]
pub(crate) mod native_owner_tests {
    use super::LimbScratch;
    use crate::bigint::BigInt;
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_lex::allocation::{Admission, StorageError};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    pub(crate) struct Storage {
        pub(crate) live: Arc<AtomicUsize>,
        pub(crate) limit: usize,
        pub(crate) first_refused: Option<usize>,
    }
    impl Admission for Storage {
        fn resize(&mut self, next: usize) -> Result<(), StorageError> {
            if next > self.limit {
                self.first_refused.get_or_insert(next);
                return Err(StorageError::AdmissionFailed);
            }
            self.live.store(next, Ordering::SeqCst);
            Ok(())
        }
    }
    impl Drop for Storage {
        fn drop(&mut self) {
            self.live.store(0, Ordering::SeqCst);
        }
    }

    #[test]
    fn native_arena_admits_before_birth_and_preserves_original_refusal() {
        let live = Arc::new(AtomicUsize::new(0));
        let window = CurrentThreadWindow::open();
        let Err((error, storage)) = LimbScratch::new_with_storage(
            4,
            8,
            Storage {
                live: Arc::clone(&live),
                limit: 0,
                first_refused: None,
            },
        ) else {
            panic!("unadmitted arena was published");
        };
        assert_eq!(error, StorageError::AdmissionFailed);
        assert!(storage.first_refused.is_some());
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(window.close().allocations, 0);
    }

    #[test]
    fn packed_arena_partial_birth_destroys_payload_before_refunding_owner() {
        let packed =
            purrdf_lex::allocation::shared::SharedBuffer::<u64>::required_bytes(8, 32).unwrap();
        let handles = size_of::<purrdf_lex::allocation::shared::SharedBuffer<u64>>() * 8;
        for limit in [0, handles, packed / 2, packed - 1, packed] {
            let live = Arc::new(AtomicUsize::new(0));
            let storage = Storage {
                live: Arc::clone(&live),
                limit,
                first_refused: None,
            };
            let window = CurrentThreadWindow::open();
            let Err((error, storage)) = LimbScratch::new_with_storage(8, 32, storage) else {
                panic!("partial allowance published an arena: {limit}");
            };
            assert_eq!(error, StorageError::AdmissionFailed);
            assert!(storage.first_refused.unwrap() > limit);
            assert_eq!(live.load(Ordering::SeqCst), 0);
            drop(storage);
            assert_eq!(window.close().retained_bytes, 0);
        }
    }

    #[test]
    fn original_arena_owner_survives_handle_drop_and_last_numeric_clone() {
        let live = Arc::new(AtomicUsize::new(0));
        let scratch = match LimbScratch::new_with_storage(
            4,
            8,
            Storage {
                live: Arc::clone(&live),
                limit: usize::MAX,
                first_refused: None,
            },
        ) {
            Ok(scratch) => scratch,
            Err((error, _)) => panic!("arena birth refused: {error}"),
        };
        let bytes = LimbScratch::required_bytes_with_storage::<Storage>(4, 8).unwrap();
        assert_eq!(scratch.retained_bytes(), bytes);
        assert_eq!(live.load(Ordering::SeqCst), bytes);
        let source = BigInt::one().shl(300);
        let expected = source.shl(1);
        let value = source.copy_in(&scratch).unwrap();
        let copy = value.clone();
        let window = CurrentThreadWindow::open();
        let doubled = value.add_in(&copy, &scratch).unwrap();
        assert_eq!(doubled, expected);
        drop(doubled);
        drop(scratch);
        assert_eq!(live.load(Ordering::SeqCst), bytes);
        drop(value);
        assert_eq!(copy, source);
        assert_eq!(live.load(Ordering::SeqCst), bytes);
        drop(copy);
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(window.close().allocations, 0);
    }
}
