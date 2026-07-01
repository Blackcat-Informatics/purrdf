// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The column-addressed SELECT row cursor returned by `purrdf_query`.

use std::ffi::{c_char, CString};

use purrdf_core::TermValue;

use crate::status::PurrdfStatus;
use crate::term::{render_value, PurrdfTermView};

/// A SPARQL SELECT result cursor. Owns its variable names and solution rows
/// (dataset-independent `TermValue`s).
///
/// **Lifetime invariant (load-bearing):** `rows` is IMMUTABLE for the cursor's
/// whole life — `purrdf_rowcursor_next` only advances `current`, never mutates,
/// reorders, or reallocates the `Vec`. A `PurrdfStr` returned by
/// `purrdf_rowcursor_term` borrows directly into `rows[current]`'s owned
/// `TermValue`; those pointers stay valid for the cursor's life (documented
/// conservatively as "until the next `_next`/`_free`"). The C side never frees a
/// `PurrdfStr.ptr`. Single-threaded.
pub struct PurrdfRowCursor {
    variables: Vec<CString>,
    rows: Vec<Vec<Option<TermValue>>>,
    /// The current row index once `_next` has been called, else `None`.
    current: Option<usize>,
    /// The next row index `_next` will yield.
    next: usize,
}

impl PurrdfRowCursor {
    /// Build a row cursor from a materialized solution set. Variable names with
    /// interior NUL bytes (impossible for SPARQL vars) are sanitized defensively.
    pub(crate) fn new(variables: Vec<String>, rows: Vec<Vec<Option<TermValue>>>) -> Self {
        let variables = variables
            .into_iter()
            .map(|name| {
                CString::new(name.replace('\0', "_"))
                    .unwrap_or_else(|_| CString::new("var").expect("static"))
            })
            .collect();
        Self {
            variables,
            rows,
            current: None,
            next: 0,
        }
    }

    /// Heap-allocate as a handle pointer.
    pub(crate) fn into_raw(self) -> *mut PurrdfRowCursor {
        Box::into_raw(Box::new(self))
    }
}

/// Write the number of result variables (columns) to `*out`.
///
/// # Safety
/// `rc` must be a live row cursor; `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_rowcursor_variable_count(
    rc: *const PurrdfRowCursor,
    out: *mut usize,
) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if rc.is_null() || out.is_null() {
            return PurrdfStatus::NullPointer as i32;
        }
        let rc = &*rc;
        *out = rc.variables.len();
        PurrdfStatus::Ok as i32
    })
}

/// Write a borrowed, NUL-terminated variable name for column `i` to `*out`.
/// Valid until `purrdf_rowcursor_free`. `InvalidArgument` if `i` is out of range.
///
/// # Safety
/// `rc` must be a live row cursor; `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_rowcursor_variable_name(
    rc: *const PurrdfRowCursor,
    i: usize,
    out: *mut *const c_char,
) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if rc.is_null() || out.is_null() {
            return PurrdfStatus::NullPointer as i32;
        }
        let rc = &*rc;
        match rc.variables.get(i) {
            Some(name) => {
                *out = name.as_ptr();
                PurrdfStatus::Ok as i32
            }
            None => PurrdfStatus::InvalidArgument as i32,
        }
    })
}

/// Advance to the next solution row. Returns `PurrdfStatus::CursorExhausted`
/// (a non-error terminal signal, `> 0`) when no rows remain.
///
/// # Safety
/// `rc` must be a live row cursor.
#[no_mangle]
pub unsafe extern "C" fn purrdf_rowcursor_next(rc: *mut PurrdfRowCursor) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if rc.is_null() {
            return PurrdfStatus::NullPointer as i32;
        }
        let rc = &mut *rc;
        if rc.next >= rc.rows.len() {
            return PurrdfStatus::CursorExhausted as i32;
        }
        rc.current = Some(rc.next);
        rc.next += 1;
        PurrdfStatus::Ok as i32
    })
}

/// Read the term in `column` of the CURRENT row. `out_bound` receives `0` when
/// the variable is UNBOUND in this row (the view is then a zeroed placeholder),
/// else `1`. The view's `PurrdfStr` pointers are valid until the next
/// `purrdf_rowcursor_next` or `purrdf_rowcursor_free`. `InvalidArgument` if no
/// row is current (call `_next` first) or `column` is out of range.
///
/// # Safety
/// `rc` must be a live row cursor; the out-params must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_rowcursor_term(
    rc: *const PurrdfRowCursor,
    column: usize,
    out_view: *mut PurrdfTermView,
    out_bound: *mut u8,
) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if rc.is_null() || out_view.is_null() || out_bound.is_null() {
            return PurrdfStatus::NullPointer as i32;
        }
        let rc = &*rc;
        let Some(row_index) = rc.current else {
            return PurrdfStatus::InvalidArgument as i32;
        };
        let row = &rc.rows[row_index];
        let Some(cell) = row.get(column) else {
            return PurrdfStatus::InvalidArgument as i32;
        };
        match cell {
            Some(value) => {
                render_value(value, &mut *out_view);
                *out_bound = 1;
            }
            None => {
                *out_view = PurrdfTermView::empty();
                *out_bound = 0;
            }
        }
        PurrdfStatus::Ok as i32
    })
}

/// Release a row cursor handle. No-op on null.
///
/// # Safety
/// `rc` must be null or a live row cursor not already freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_rowcursor_free(rc: *mut PurrdfRowCursor) {
    ffi_guard!((), {
        if !rc.is_null() {
            drop(Box::from_raw(rc));
        }
    })
}
