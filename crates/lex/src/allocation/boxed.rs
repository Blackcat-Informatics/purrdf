// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The native fallible box allocation shared by grammar and RDF payload owners.

/// Allocate one concrete owner fallibly, retaining the single-element array
/// type so conversion never requires a second allocation.
///
/// # Errors
/// Returns the allocator's original reservation failure.
pub fn try_boxed_one<T>(value: T) -> Result<Box<[T; 1]>, std::collections::TryReserveError> {
    let mut storage = Vec::new();
    storage.try_reserve_exact(1)?;
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
    let storage = try_boxed_one(value)?;
    // SAFETY: a one-element array and its element have identical layout and
    // alignment. The returned Box owns the same allocation exactly once.
    Ok(unsafe { Box::from_raw(Box::into_raw(storage).cast::<T>()) })
}
