// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The native fallible box allocation shared by grammar and RDF payload owners.

/// Shrink an owned vector fallibly, restoring its original allocation on refusal.
/// Accounting and progress callbacks belong outside this ownership transition.
pub(super) fn try_shrink_to_fit<T>(values: &mut Vec<T>) -> Result<(), super::StorageError> {
    if size_of::<T>() == 0 || values.capacity() == values.len() {
        return Ok(());
    }
    if values.is_empty() {
        *values = Vec::new();
        return Ok(());
    }
    let old = core::alloc::Layout::array::<T>(values.capacity())
        .map_err(|_| super::StorageError::SizeOverflow)?;
    let new = core::alloc::Layout::array::<T>(values.len())
        .map_err(|_| super::StorageError::SizeOverflow)?;
    let length = values.len();
    let capacity = values.capacity();
    let mut original = core::mem::ManuallyDrop::new(core::mem::take(values));
    let pointer = original.as_mut_ptr();
    // SAFETY: this nonempty, non-ZST Vec owns the global allocation described
    // by its original capacity. Both layouts are checked and have T's alignment;
    // the new nonzero extent retains every initialized element. No callback or
    // fallible operation intervenes before exactly one Vec owner is restored.
    let resized = unsafe { std::alloc::realloc(pointer.cast(), old, new.size()) }.cast::<T>();
    let refused = resized.is_null();
    // SAFETY: null preserves the original allocation, length and capacity.
    // Success transfers that allocation to the returned pointer with exactly
    // the requested layout. ManuallyDrop never destroys the superseded owner.
    *values = unsafe {
        Vec::from_raw_parts(
            if refused { pointer } else { resized },
            length,
            if refused { capacity } else { length },
        )
    };
    if refused {
        Err(super::StorageError::AllocationFailed)
    } else {
        Ok(())
    }
}

/// Relocate into an empty, preallocated buffer, retaining the original owner
/// across every refused or panicking progress callback.
pub(super) fn relocate<T>(
    source: &mut Vec<T>,
    destination: &mut Vec<T>,
    mut checkpoint: impl FnMut(u64) -> Result<(), super::StorageError>,
) -> Result<(), super::StorageError> {
    assert!(destination.is_empty());
    assert!(destination.capacity() >= source.len());
    let length = source.len();
    let bytes = length
        .checked_mul(size_of::<T>())
        .ok_or(super::StorageError::SizeOverflow)?;
    let mut offset = 0;
    while offset < bytes {
        let count = (bytes - offset).min(256);
        checkpoint(count as u64)?;
        // SAFETY: the two Vecs own disjoint allocations. The checked byte
        // extent covers only initialized source elements and fits destination
        // capacity. Copying their representation through u8 permits padding
        // and arbitrary byte boundaries. Destination length stays zero, so
        // refusal or panic cannot drop duplicate element owners.
        unsafe {
            core::ptr::copy_nonoverlapping(
                source.as_ptr().cast::<u8>().add(offset),
                destination.as_mut_ptr().cast::<u8>().add(offset),
                count,
            );
        }
        offset += count;
    }
    // SAFETY: all representations were copied, or T is zero-sized. No
    // callback or fallible operation intervenes in this ownership transfer.
    // Source relinquishes every element before destination takes ownership.
    unsafe {
        source.set_len(0);
        destination.set_len(length);
    }
    Ok(())
}

/// Allocate one concrete owner fallibly, retaining the single-element array
/// type so conversion never requires a second allocation.
///
/// # Errors
/// Returns the allocator's original reservation failure.
pub fn try_boxed_one<T>(value: T) -> Result<Box<[T; 1]>, std::collections::TryReserveError> {
    try_boxed_one_retaining(value).map_err(|(_, cause)| cause)
}

fn try_boxed_one_retaining<T>(
    value: T,
) -> Result<Box<[T; 1]>, (T, std::collections::TryReserveError)> {
    let mut storage = Vec::new();
    if let Err(cause) = storage.try_reserve_exact(1) {
        return Err((value, cause));
    }
    storage.push(value);
    Ok(storage
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!("one element was inserted")))
}

/// Fallibly allocate one sized payload at the shared boxing home.
///
/// # Errors
/// Returns the allocator refusal before ownership is published.
pub fn try_boxed<T>(value: T) -> Result<Box<T>, std::collections::TryReserveError> {
    try_boxed_retaining(value).map_err(|(_, cause)| cause)
}

pub(super) fn try_boxed_retaining<T>(
    value: T,
) -> Result<Box<T>, (T, std::collections::TryReserveError)> {
    let storage = try_boxed_one_retaining(value)?;
    // SAFETY: a one-element array and its element have identical layout and
    // alignment. The returned Box owns the same allocation exactly once.
    Ok(unsafe { Box::from_raw(Box::into_raw(storage).cast::<T>()) })
}

#[cfg(test)]
mod tests {
    /// Cancellation and unwinding during relocation retain one payload owner;
    /// successful relocation transfers that owner without duplicating drops.
    #[test]
    fn relocation_preserves_owning_payloads_across_refusal_and_unwind() {
        use std::cell::Cell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use super::super::{Memory, Resident};

        struct Payload(Rc<Cell<usize>>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let drops = Rc::new(Cell::new(0));
        let mut source: Vec<_> = (0..100).map(|_| Payload(Rc::clone(&drops))).collect();
        let mut destination = Vec::with_capacity(source.len() + 17);
        let pointer = source.as_ptr();
        let mut calls = 0;
        let refused = super::relocate(&mut source, &mut destination, |_| {
            calls += 1;
            if calls == 2 {
                Err(super::super::StorageError::AdmissionFailed)
            } else {
                Ok(())
            }
        });
        assert!(refused.is_err());
        assert_eq!(source.as_ptr(), pointer);
        assert_eq!(source.len(), 100);
        assert!(destination.is_empty());
        assert_eq!(drops.get(), 0);
        let mut calls = 0;
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::relocate(&mut source, &mut destination, |_| {
                calls += 1;
                assert!(calls != 2, "injected relocation callback panic");
                Ok(())
            })
        }));
        assert!(unwind.is_err());
        assert_eq!(source.len(), 100);
        assert!(destination.is_empty());
        super::relocate(&mut source, &mut destination, |_| Ok(())).unwrap();
        assert!(source.is_empty());
        assert_eq!(destination.len(), 100);
        drop(source);
        assert_eq!(drops.get(), 0);
        let mut resident = Resident;
        let mut memory =
            Memory::resume(&mut resident, destination.capacity() * size_of::<Payload>());
        let frozen = memory.boxed_slice(destination).unwrap();
        assert_eq!(frozen.len(), 100);
        assert_eq!(memory.admitted_bytes(), 100 * size_of::<Payload>());
        assert_eq!(drops.get(), 0);
        drop(frozen);
        memory.release_bytes(100 * size_of::<Payload>()).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(drops.get(), 100);

        let empty = Vec::<Payload>::with_capacity(17);
        let mut memory = Memory::resume(&mut resident, empty.capacity() * size_of::<Payload>());
        let frozen = memory.boxed_slice(empty).unwrap();
        assert!(frozen.is_empty());
        assert_eq!(memory.admitted_bytes(), 0);
        drop(frozen);
        assert_eq!(drops.get(), 100);

        static ZST_DROPS: AtomicUsize = AtomicUsize::new(0);
        struct ZeroSized;
        impl Drop for ZeroSized {
            fn drop(&mut self) {
                ZST_DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
        let mut zero_sized = Vec::new();
        zero_sized.resize_with(3, || ZeroSized);
        let mut memory = Memory::new(&mut resident);
        let frozen = memory.boxed_slice(zero_sized).unwrap();
        assert_eq!(frozen.len(), 3);
        assert_eq!(memory.admitted_bytes(), 0);
        drop(frozen);
        assert_eq!(ZST_DROPS.load(Ordering::Relaxed), 3);
    }
}
