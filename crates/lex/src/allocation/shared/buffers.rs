// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded shared destinations packed into one admitted native array.

use core::alloc::Layout;
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

use super::{Shared, SharedCloneError, clone_count};
use crate::allocation::{Admission, Memory, StorageError};

struct Slot {
    strong: AtomicUsize,
    length: UnsafeCell<usize>,
}

struct Buffers<T: Copy> {
    slots: Vec<Slot>,
    words: Vec<UnsafeCell<T>>,
    capacity: usize,
}

// SAFETY: each initialized range has its own atomic strong count. Only its
// sole handle may mutate its length or elements, through an exclusive borrow.
// Different ranges are disjoint and never move or grow after construction.
unsafe impl<T: Copy + Send + Sync> Sync for Buffers<T> {}

/// One bounded destination in a shared array. Immutable clones do not allocate.
/// Its elements and control slots stay initialized for the array's lifetime;
/// mutation is possible only through the sole handle to this particular range.
pub struct SharedBuffer<T: Copy> {
    array: Shared<Buffers<T>>,
    index: usize,
}

impl<T: Copy> SharedBuffer<T> {
    /// Complete native arrays, handle table and shared control, before birth.
    ///
    /// # Errors
    /// Refuses checked array or address-space overflow.
    pub fn required_bytes(buffers: usize, capacity: usize) -> Result<usize, StorageError> {
        if buffers == 0 {
            return Ok(0);
        }
        let words = buffers
            .checked_mul(capacity)
            .ok_or(StorageError::SizeOverflow)?;
        let arrays = [
            Layout::array::<Self>(buffers),
            Layout::array::<Slot>(buffers),
            Layout::array::<UnsafeCell<T>>(words),
        ];
        arrays.into_iter().try_fold(
            Shared::<Buffers<T>>::allocation_layout().size(),
            |bytes, layout| {
                bytes
                    .checked_add(layout.map_err(|_| StorageError::SizeOverflow)?.size())
                    .ok_or(StorageError::SizeOverflow)
            },
        )
    }

    /// Construct all destinations in four allocations through the original
    /// admission account. No per-destination limb or ownership allocation occurs.
    /// The enclosing native owner retains this account through final destruction.
    ///
    /// # Errors
    /// Preserves original admission, layout or allocation refusal. Partial
    /// arrays are destroyed before the scope refunds their admitted capacity.
    pub fn new_array_with_memory<S: Admission + ?Sized>(
        buffers: usize,
        capacity: usize,
        zero: T,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<Self>, StorageError> {
        Self::required_bytes(buffers, capacity)?;
        if buffers == 0 {
            return Ok(Vec::new());
        }
        let words = buffers
            .checked_mul(capacity)
            .ok_or(StorageError::SizeOverflow)?;
        memory.scope(|memory| {
            let mut handles = Vec::new();
            memory.reserve(&mut handles, buffers)?;
            let mut slots = Vec::new();
            memory.reserve(&mut slots, buffers)?;
            let mut elements = Vec::new();
            memory.reserve(&mut elements, words)?;
            for _ in 0..buffers {
                slots.push(Slot {
                    strong: AtomicUsize::new(1),
                    length: UnsafeCell::new(0),
                });
            }
            for _ in 0..words {
                elements.push(UnsafeCell::new(zero));
            }
            memory.add_bytes(Shared::<Buffers<T>>::allocation_layout().size())?;
            let array = Shared::try_new(Buffers {
                slots,
                words: elements,
                capacity,
            })
            .map_err(|_| StorageError::AllocationFailed)?;
            for index in 0..buffers {
                handles.push(Self {
                    array: array.clone(),
                    index,
                });
            }
            Ok(handles)
        })
    }

    /// This destination's admitted element capacity. It never grows.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.array.capacity
    }

    /// Borrow this destination for mutation only when no immutable clone exists.
    pub fn get_mut(&mut self) -> Option<SharedBufferMut<'_, T>> {
        (self.array.slots[self.index].strong.load(Ordering::Acquire) == 1)
            .then_some(SharedBufferMut { buffer: self })
    }

    /// Clone this range without allocating or copying its elements.
    ///
    /// # Errors
    /// Refuses a shared-owner count overflow without changing the source.
    pub fn try_clone(&self) -> Result<Self, SharedCloneError> {
        let array = self.array.try_clone()?;
        clone_count(&array.slots[self.index].strong)?;
        Ok(Self {
            array,
            index: self.index,
        })
    }

    fn length(&self) -> usize {
        // SAFETY: mutation requires the sole range handle and its exclusive
        // borrow, excluding this shared borrow and every immutable clone.
        unsafe { *self.array.slots[self.index].length.get() }
    }

    fn pointer(&self) -> *mut T {
        // UnsafeCell<T> is representation-transparent. All elements were
        // initialized at construction and the array never reallocates.
        self.array
            .words
            .as_ptr()
            .wrapping_add(self.index * self.capacity())
            .cast::<T>()
            .cast_mut()
    }
}

impl<T: Copy> Clone for SharedBuffer<T> {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("shared buffer reference count overflow")
    }
}

impl<T: Copy> Deref for SharedBuffer<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        // SAFETY: this live handle keeps the entire initialized array alive;
        // the checked construction bounds this range and its initialized length.
        // Any immutable clone prevents mutation through get_mut.
        unsafe { core::slice::from_raw_parts(self.pointer(), self.length()) }
    }
}

impl<T: Copy + fmt::Debug> fmt::Debug for SharedBuffer<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<T: Copy> Drop for SharedBuffer<T> {
    fn drop(&mut self) {
        self.array.slots[self.index]
            .strong
            .fetch_sub(1, Ordering::Release);
        // The containing array owns every initialized Copy element. Its one
        // normal final drop destroys storage; an individual range never frees it.
    }
}

/// Exclusive mutation of one unique bounded destination. Methods never allocate.
pub struct SharedBufferMut<'a, T: Copy> {
    buffer: &'a mut SharedBuffer<T>,
}

impl<T: Copy + fmt::Debug> fmt::Debug for SharedBufferMut<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl<'a, T: Copy> SharedBufferMut<'a, T> {
    /// Consume the guard into its exclusively borrowed initialized range.
    pub fn into_slice(self) -> &'a mut [T] {
        mutable_slice(self.buffer)
    }

    /// The original admitted capacity, unchanged by mutation.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    fn set_length(&mut self, length: usize) {
        // SAFETY: get_mut established the sole range owner before this exclusive
        // guard was created. No clone or slice can be borrowed during this write.
        unsafe {
            *self.buffer.array.slots[self.buffer.index].length.get() = length;
        }
    }

    /// Empty the initialized destination without releasing or reallocating it.
    pub fn clear(&mut self) {
        self.set_length(0);
    }

    /// Append within the original admitted capacity.
    ///
    /// # Panics
    /// Panics if the caller exceeds the admitted destination capacity.
    pub fn push(&mut self, value: T) {
        let length = self.len();
        assert!(length < self.capacity(), "admitted shared buffer capacity");
        // SAFETY: the unique guard owns this initialized in-range element.
        unsafe {
            self.buffer.pointer().add(length).write(value);
        }
        self.set_length(length + 1);
    }

    /// Remove the last initialized element without allocating.
    pub fn pop(&mut self) -> Option<T> {
        let length = self.len().checked_sub(1)?;
        let value = self[length];
        self.set_length(length);
        Some(value)
    }

    /// Fill or truncate within the original admitted capacity.
    ///
    /// # Panics
    /// Panics if the caller exceeds the admitted destination capacity.
    pub fn resize(&mut self, length: usize, value: T) {
        assert!(length <= self.capacity(), "admitted shared buffer capacity");
        while self.len() < length {
            self.push(value);
        }
        self.set_length(length);
    }
}

impl<T: Copy> Deref for SharedBufferMut<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.buffer
    }
}

impl<T: Copy> DerefMut for SharedBufferMut<'_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        mutable_slice(self.buffer)
    }
}

fn mutable_slice<T: Copy>(buffer: &mut SharedBuffer<T>) -> &mut [T] {
    // SAFETY: this helper is reached only through the exclusive unique-owner
    // guard. The length is bounded and every exposed element is initialized.
    unsafe { core::slice::from_raw_parts_mut(buffer.pointer(), buffer.length()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allocation::Resident;

    #[test]
    fn packed_destinations_preserve_disjoint_ranges_and_immutable_clones() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut buffers = SharedBuffer::new_array_with_memory(8, 17, 0_u64, &mut memory).unwrap();
        assert_eq!(
            memory.admitted_bytes(),
            SharedBuffer::<u64>::required_bytes(8, 17).unwrap()
        );
        for (index, buffer) in buffers.iter_mut().enumerate() {
            buffer.get_mut().unwrap().resize(17, index as u64);
        }
        let escaped = buffers[3].clone();
        assert!(buffers[3].get_mut().is_none());
        let mut separate = buffers.pop().unwrap();
        separate.get_mut().unwrap().clear();
        separate.get_mut().unwrap().push(123);
        assert_eq!(&*escaped, &[3; 17]);
        assert_eq!(&*separate, &[123]);
        drop(buffers);
        assert_eq!(&*escaped, &[3; 17]);
        assert_eq!(separate.get_mut().unwrap().pop(), Some(123));
        assert!(separate.is_empty());
    }

    #[test]
    fn overflowing_layout_and_refused_birth_leave_the_original_account_empty() {
        struct Refuse;
        impl Admission for Refuse {
            fn resize(&mut self, _: usize) -> Result<(), StorageError> {
                Err(StorageError::AdmissionFailed)
            }
        }
        let mut storage = Refuse;
        let mut memory = Memory::new(&mut storage);
        assert!(matches!(
            SharedBuffer::new_array_with_memory(2, usize::MAX, 0_u64, &mut memory),
            Err(StorageError::SizeOverflow)
        ));
        assert!(matches!(
            SharedBuffer::new_array_with_memory(2, 17, 0_u64, &mut memory),
            Err(StorageError::AdmissionFailed)
        ));
        assert_eq!(memory.admitted_bytes(), 0);
        let count = AtomicUsize::new(usize::MAX);
        assert_eq!(clone_count(&count), Err(SharedCloneError));
        assert_eq!(count.load(Ordering::Relaxed), usize::MAX);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn independent_ranges_mutate_concurrently_while_a_clone_keeps_the_array_alive() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut buffers = SharedBuffer::new_array_with_memory(4, 32, 0_u64, &mut memory).unwrap();
        for buffer in &mut buffers {
            buffer.get_mut().unwrap().resize(32, 0);
        }
        let original = buffers[0].clone();
        std::thread::scope(|scope| {
            for (index, buffer) in buffers.iter_mut().enumerate().skip(1) {
                scope.spawn(move || {
                    buffer.get_mut().unwrap().fill(index as u64);
                });
            }
            scope.spawn(|| assert_eq!(&*original, &[0; 32]));
        });
        for (index, buffer) in buffers.iter().enumerate() {
            assert!(buffer.iter().all(|word| *word == index as u64));
        }
        drop(original);
        assert!(buffers[0].get_mut().is_some());
    }
}
