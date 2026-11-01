// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The native fallible box allocation shared by grammar and RDF payload owners.

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

        struct Payload(Rc<Cell<usize>>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let drops = Rc::new(Cell::new(0));
        let mut source: Vec<_> = (0..100).map(|_| Payload(Rc::clone(&drops))).collect();
        let mut destination = Vec::with_capacity(source.len());
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
        drop(destination);
        assert_eq!(drops.get(), 100);
    }
}
