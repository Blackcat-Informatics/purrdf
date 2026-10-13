// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen-dataset handle and its read-only accessors.

use purrdf_core::{DatasetHandle, RdfDataset};
use purrdf_sparql_eval::DivisionPolicy;

use crate::status::PurrdfStatus;

/// A frozen, immutable RDF-1.2 dataset. Retains the frozen dataset storage owner, so it is
/// `Send + Sync`: it may be read concurrently from multiple threads. Release
/// with `purrdf_dataset_free`.
///
/// The handle also carries the precision every SPARQL query and UPDATE run over it
/// forms an `xsd:integer`/`xsd:decimal` quotient at (`/` and `AVG`), set with
/// `purrdf_dataset_set_division_policy`. Every handle the library returns starts at
/// [`DivisionPolicy::xsd_default`]: eighteen fractional digits, truncated toward zero.
#[derive(Debug)]
pub struct PurrdfDataset(pub(crate) DatasetHandle, pub(crate) DivisionPolicy);

/// Compile-time proof of the `Send + Sync` guarantee documented on
/// [`PurrdfDataset`] (and published in the README thread-safety table). If a
/// future change made `RdfDataset` non-`Sync`, this would fail to compile rather
/// than silently breaking the frozen ABI contract.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PurrdfDataset>();
};

impl PurrdfDataset {
    /// A handle over `dataset` at the default division policy.
    pub(crate) fn new(dataset: impl Into<DatasetHandle>) -> Self {
        Self(dataset.into(), DivisionPolicy::xsd_default())
    }

    /// The division policy every query and UPDATE over this handle runs under.
    ///
    /// # Safety
    /// `ptr` must be a live `PurrdfDataset` handle.
    pub(crate) const unsafe fn division(ptr: *const Self) -> DivisionPolicy {
        unsafe { (*ptr).1 }
    }

    /// Borrow the frozen storage handle from a non-null handle pointer (used where an
    /// owned clone of the handle is needed, e.g. pinning a cursor).
    ///
    /// # Safety
    /// `ptr` must be a live `PurrdfDataset` handle.
    pub(crate) unsafe fn handle<'a>(ptr: *const Self) -> &'a DatasetHandle {
        unsafe { &(*ptr).0 }
    }

    /// Borrow the frozen dataset from a non-null handle pointer.
    ///
    /// # Safety
    /// `ptr` must be a live `PurrdfDataset` handle.
    pub(crate) unsafe fn dataset<'a>(ptr: *const Self) -> &'a RdfDataset {
        unsafe { &(*ptr).0 }
    }
}

/// Move `value` to the heap and hand its ownership across the ABI as an opaque
/// handle pointer. Every handle libpurrdf returns is minted here, so every handle
/// is a `Box` allocation that [`free_handle`] can reclaim with the matching layout.
pub(crate) fn into_handle<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Reclaim and drop a handle minted by [`into_handle`]; a null pointer is a no-op.
///
/// The single destructor behind every `purrdf_*_free` entry point: each handle is a
/// `Box<T>` allocation, so releasing one is the same `Box::from_raw` for every `T`. A
/// panic in `T`'s destructor is caught here rather than unwinding into C.
///
/// # Safety
/// `handle` must be null or a pointer returned by [`into_handle`] for this `T` and
/// not already freed.
pub(crate) unsafe fn free_handle<T>(handle: *mut T) {
    ffi_guard!((), {
        if !handle.is_null() {
            // SAFETY: the caller's contract — a live `into_handle` allocation of `T`.
            drop(unsafe { Box::from_raw(handle) });
        }
    });
}

/// Release a dataset handle. No-op on null.
///
/// # Safety
/// `dataset` must be null or a live dataset handle not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_dataset_free(dataset: *mut PurrdfDataset) {
    unsafe { free_handle::<PurrdfDataset>(dataset) }
}

/// Write the number of quads in the dataset to `*out`.
///
/// # Safety
/// `dataset` must be a live handle; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_dataset_quad_count(
    dataset: *const PurrdfDataset,
    out: *mut usize,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if dataset.is_null() || out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = (*dataset).0.quad_count();
            PurrdfStatus::Ok as i32
        })
    }
}

/// Write the number of interned terms in the dataset to `*out`.
///
/// # Safety
/// `dataset` must be a live handle; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_dataset_term_count(
    dataset: *const PurrdfDataset,
    out: *mut usize,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if dataset.is_null() || out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = (*dataset).0.term_count();
            PurrdfStatus::Ok as i32
        })
    }
}
