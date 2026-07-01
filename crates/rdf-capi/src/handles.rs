// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The frozen-dataset handle and its read-only accessors.

use std::sync::Arc;

use purrdf_core::RdfDataset;

use crate::status::PurrdfStatus;

/// A frozen, immutable RDF-1.2 dataset. Wraps `Arc<RdfDataset>`, so it is
/// `Send + Sync`: it may be read concurrently from multiple threads. Release
/// with `purrdf_dataset_free`.
#[derive(Debug)]
pub struct PurrdfDataset(pub(crate) Arc<RdfDataset>);

/// Compile-time proof of the `Send + Sync` guarantee documented on
/// [`PurrdfDataset`] (and published in the README thread-safety table). If a
/// future change made `RdfDataset` non-`Sync`, this would fail to compile rather
/// than silently breaking the frozen ABI contract.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PurrdfDataset>();
};

impl PurrdfDataset {
    /// Wrap a frozen dataset as a heap-owned handle pointer.
    pub(crate) fn into_raw(dataset: Arc<RdfDataset>) -> *mut Self {
        Box::into_raw(Box::new(Self(dataset)))
    }

    /// Borrow the inner `Arc` from a non-null handle pointer (used where an
    /// owned clone of the `Arc` is needed, e.g. pinning a cursor).
    ///
    /// # Safety
    /// `ptr` must be a live `PurrdfDataset` handle.
    pub(crate) unsafe fn arc<'a>(ptr: *const Self) -> &'a Arc<RdfDataset> {
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

/// Release a dataset handle. No-op on null.
///
/// # Safety
/// `dataset` must be null or a live dataset handle not already freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_dataset_free(dataset: *mut PurrdfDataset) {
    unsafe {
        ffi_guard!((), {
            if !dataset.is_null() {
                drop(Box::from_raw(dataset));
            }
        });
    }
}

/// Write the number of quads in the dataset to `*out`.
///
/// # Safety
/// `dataset` must be a live handle; `out` must be writable.
#[no_mangle]
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
#[no_mangle]
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
