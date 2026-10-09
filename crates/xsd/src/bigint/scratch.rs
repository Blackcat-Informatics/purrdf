// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit reusable integer limb storage. No thread-local or global allocator.

use core::fmt;
use std::sync::{Arc, Mutex, Weak};

use super::storage::Mag;

/// A bounded integer destination could not be checked out. No fallback heap
/// allocation is made after a scratch arena has been constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimbScratchError {
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
}

impl fmt::Display for LimbScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}
impl std::error::Error for LimbScratchError {}

#[derive(Debug)]
struct FreeBuffers {
    buffers: Vec<Arc<Vec<u64>>>,
    limbs: usize,
    bytes: usize,
}

/// Preallocated reusable destinations for one worker's exact arithmetic.
///
/// Every destination has a preallocated limb vector and shared ownership
/// header. Cloning a returned integer shares immutable limbs; its last drop
/// returns the destination. Integers may outlive this handle and remain
/// ordinary thread-safe values. Checkout and return hold a private mutex only
/// while moving a buffer; no arithmetic, caller callback, or suspension occurs
/// under that lock.
#[derive(Clone, Debug)]
pub struct LimbScratch {
    owner: Arc<Mutex<FreeBuffers>>,
}

impl LimbScratch {
    /// Checked heap bound including every vector, ownership header and free
    /// list. The two-word ownership header is the `Arc` allocation layout.
    /// # Errors
    /// Refuses overflowing arena sizes before allocation.
    pub fn required_bytes(buffers: usize, limbs: usize) -> Result<usize, LimbScratchError> {
        let words = limbs
            .checked_mul(size_of::<u64>())
            .ok_or(LimbScratchError::SizeOverflow)?;
        let slot = words
            .checked_add(
                size_of::<Vec<u64>>() + 2 * size_of::<usize>() + size_of::<Arc<Vec<u64>>>(),
            )
            .ok_or(LimbScratchError::SizeOverflow)?;
        buffers
            .checked_mul(slot)
            .and_then(|bytes| {
                bytes.checked_add(size_of::<Mutex<FreeBuffers>>() + 2 * size_of::<usize>())
            })
            .ok_or(LimbScratchError::SizeOverflow)
    }

    /// Allocate all admitted destinations once. Subsequent integer `_in`
    /// operations neither grow vectors nor allocate ownership headers.
    /// # Errors
    /// Refuses overflowing arena sizes before allocation.
    pub fn new(buffers: usize, limbs: usize) -> Result<Self, LimbScratchError> {
        let bytes = Self::required_bytes(buffers, limbs)?;
        let mut free = Vec::with_capacity(buffers);
        for _ in 0..buffers {
            free.push(Arc::new(Vec::with_capacity(limbs)));
        }
        Ok(Self {
            owner: Arc::new(Mutex::new(FreeBuffers {
                buffers: free,
                limbs,
                bytes,
            })),
        })
    }

    /// Heap retained by the whole arena, including checked-out destinations.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .bytes
    }

    /// Maximum admitted limbs in one reusable destination.
    #[must_use]
    pub fn limb_capacity(&self) -> usize {
        self.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .limbs
    }

    /// Total admitted reusable destinations, including checked-out buffers.
    #[must_use]
    pub fn destination_capacity(&self) -> usize {
        self.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .buffers
            .capacity()
    }

    /// Number of destinations currently free. Immutable integer clones do not
    /// consume a second destination.
    #[must_use]
    pub fn available(&self) -> usize {
        self.owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .buffers
            .len()
    }

    pub(super) fn destination(
        &self,
        capacity: usize,
        length: usize,
    ) -> Result<Mag, LimbScratchError> {
        if capacity <= 3 {
            return Ok(Mag::zeroed(length));
        }
        let mut free = self
            .owner
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
        let unique = Arc::get_mut(&mut words).expect("free destinations have one owner");
        unique.clear();
        unique.resize(length, 0);
        drop(free);
        Ok(Mag::Pooled(Pooled {
            words: Some(words),
            owner: Arc::downgrade(&self.owner),
        }))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Pooled {
    words: Option<Arc<Vec<u64>>>,
    owner: Weak<Mutex<FreeBuffers>>,
}

impl Pooled {
    pub(super) fn words(&self) -> &[u64] {
        self.words.as_deref().expect("live destination").as_slice()
    }
    pub(super) fn words_mut(&mut self) -> &mut Vec<u64> {
        Arc::get_mut(self.words.as_mut().expect("live destination"))
            .expect("only a unique arithmetic destination is mutated")
    }
    pub(super) fn allocated_bytes(&self) -> usize {
        self.words.as_deref().expect("live destination").capacity() * size_of::<u64>()
            + size_of::<Vec<u64>>()
            + 2 * size_of::<usize>()
            + size_of::<Mutex<FreeBuffers>>()
            + 2 * size_of::<usize>()
    }
}

impl Drop for Pooled {
    fn drop(&mut self) {
        let Some(mut words) = self.words.take() else {
            return;
        };
        // Serialize the reference-count check AND decrement. Two simultaneous
        // final drops must not both see count=2 and discard the reusable slot.
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let mut free = owner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if Arc::strong_count(&words) == 1 {
            Arc::get_mut(&mut words)
                .expect("last destination owner")
                .clear();
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

    fn copy(&self, words: &[u64]) -> Result<Mag, LimbScratchError> {
        let mut result = self.destination(words.len(), words.len())?;
        result.copy_from_slice(words);
        Ok(result)
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

#[cfg(test)]
mod tests {
    use super::{LimbScratch, LimbScratchError};
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
