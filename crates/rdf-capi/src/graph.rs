// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_graph_*`: a copy-on-write mutable graph branched off a frozen
//! dataset, then re-frozen.

use purrdf_core::{DatasetMut, MutableDataset, QuadValues};

use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;
use crate::term::{view_to_value, PurrdfTermView};

/// A copy-on-write mutable graph: a suppression-delta over a frozen base
/// dataset. Single-threaded mutable — NOT `Sync`; do not touch one from two
/// threads. Release with `purrdf_graph_free`.
pub struct PurrdfGraph(pub(crate) MutableDataset);

/// Build an owned value-quad from the three required term views and the optional
/// graph view.
unsafe fn quad_from_views(
    s: *const PurrdfTermView,
    p: *const PurrdfTermView,
    o: *const PurrdfTermView,
    g: *const PurrdfTermView,
) -> Result<QuadValues, PurrdfError> {
    let s = view_to_value(&*s)?;
    let p = view_to_value(&*p)?;
    let o = view_to_value(&*o)?;
    let g = if g.is_null() {
        None
    } else {
        Some(view_to_value(&*g)?)
    };
    Ok(QuadValues { s, p, o, g })
}

/// Branch a single-threaded mutable COW graph off a frozen dataset. `*out_graph`
/// is a caller-owned handle (free with `purrdf_graph_free`); the source dataset
/// is unaffected and may be freed independently.
///
/// # Safety
/// `dataset` must be a live handle; `out_graph` must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_graph_from_dataset(
    dataset: *const PurrdfDataset,
    out_graph: *mut *mut PurrdfGraph,
) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if dataset.is_null() || out_graph.is_null() {
            return PurrdfStatus::NullPointer as i32;
        }
        let base = PurrdfDataset::arc(dataset).clone();
        *out_graph = Box::into_raw(Box::new(PurrdfGraph(MutableDataset::new(base))));
        PurrdfStatus::Ok as i32
    })
}

/// Insert a value-quad into the effective set. `s`/`p`/`o` are required input
/// term views; `g` may be null (default graph). `out_changed` (nullable)
/// receives `1` if the effective set changed, else `0` (an already-present quad
/// is a no-op; inserting a previously-removed base quad un-suppresses it).
///
/// # Safety
/// `graph` must be a live handle; the view pointers must be valid where
/// non-null; the out-params must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_graph_insert(
    graph: *mut PurrdfGraph,
    s: *const PurrdfTermView,
    p: *const PurrdfTermView,
    o: *const PurrdfTermView,
    g: *const PurrdfTermView,
    out_changed: *mut u8,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    ffi_try!(out_error, {
        if graph.is_null() || s.is_null() || p.is_null() || o.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null pointer argument to purrdf_graph_insert",
            ));
        }
        let quad = quad_from_views(s, p, o, g)?;
        let changed = (*graph).0.insert(quad);
        if !out_changed.is_null() {
            *out_changed = changed as u8;
        }
        Ok(PurrdfStatus::Ok)
    })
}

/// Remove a value-quad from the effective set. Removing a delta-added quad drops
/// it; removing a base quad creates a suppression. `out_changed` (nullable)
/// receives `1` if the effective set changed.
///
/// # Safety
/// Same contract as [`purrdf_graph_insert`].
#[no_mangle]
pub unsafe extern "C" fn purrdf_graph_remove(
    graph: *mut PurrdfGraph,
    s: *const PurrdfTermView,
    p: *const PurrdfTermView,
    o: *const PurrdfTermView,
    g: *const PurrdfTermView,
    out_changed: *mut u8,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    ffi_try!(out_error, {
        if graph.is_null() || s.is_null() || p.is_null() || o.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null pointer argument to purrdf_graph_remove",
            ));
        }
        let quad = quad_from_views(s, p, o, g)?;
        let changed = (*graph).0.remove(&quad);
        if !out_changed.is_null() {
            *out_changed = changed as u8;
        }
        Ok(PurrdfStatus::Ok)
    })
}

/// Compact the COW delta into a fresh frozen dataset. The graph remains valid
/// (free it separately with `purrdf_graph_free`). `*out_dataset` is a
/// caller-owned handle.
///
/// # Safety
/// `graph` must be a live handle; `out_dataset` must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_graph_freeze(
    graph: *const PurrdfGraph,
    out_dataset: *mut *mut PurrdfDataset,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    ffi_try!(out_error, {
        if graph.is_null() || out_dataset.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null pointer argument to purrdf_graph_freeze",
            ));
        }
        let frozen = (*graph).0.freeze().map_err(|diagnostic| {
            PurrdfError::from_diagnostic(PurrdfStatus::FreezeError, &diagnostic)
        })?;
        *out_dataset = PurrdfDataset::into_raw(frozen);
        Ok(PurrdfStatus::Ok)
    })
}

/// Release a graph handle. No-op on null.
///
/// # Safety
/// `graph` must be null or a live graph handle not already freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_graph_free(graph: *mut PurrdfGraph) {
    ffi_guard!((), {
        if !graph.is_null() {
            drop(Box::from_raw(graph));
        }
    })
}
