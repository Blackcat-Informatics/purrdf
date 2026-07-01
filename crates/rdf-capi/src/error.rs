// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The opaque error handle and its accessors.
//!
//! Fallible entry points set `*out_error` to a heap-owned `PurrdfError` when
//! they fail. The caller reads `purrdf_error_code` / `purrdf_error_message` and
//! must release it with `purrdf_error_free`.

use std::ffi::{c_char, CString};

use purrdf_core::RdfDiagnostic;

use crate::status::PurrdfStatus;

/// An owned error: a status code plus a NUL-terminated message. Opaque to C.
#[derive(Debug)]
pub struct PurrdfError {
    pub(crate) code: PurrdfStatus,
    pub(crate) message: CString,
}

impl PurrdfError {
    /// Build an error from a status and a message. Interior NUL bytes in the
    /// message are replaced with spaces so the `CString` construction never
    /// fails.
    pub(crate) fn new(code: PurrdfStatus, message: impl Into<String>) -> Self {
        let raw = message.into().replace('\0', " ");
        let message = CString::new(raw).unwrap_or_else(|_| {
            CString::new("libpurrdf error (unprintable message)").expect("static message")
        });
        Self { code, message }
    }

    /// Map a kernel [`RdfDiagnostic`] to a `PurrdfError` under the given C status,
    /// preserving the diagnostic's own code and message.
    pub(crate) fn from_diagnostic(code: PurrdfStatus, diagnostic: &RdfDiagnostic) -> Self {
        Self::new(
            code,
            format!("[{}] {}", diagnostic.code, diagnostic.message),
        )
    }
}

/// Store `err` at `*out` (heap-owned), or drop it if `out` is null.
pub(crate) fn store_error(out: *mut *mut PurrdfError, err: PurrdfError) {
    if out.is_null() {
        return;
    }
    // SAFETY: `out` is non-null and, per the ABI contract, points to a writable
    // `*mut PurrdfError` out-param.
    unsafe {
        *out = Box::into_raw(Box::new(err));
    }
}

/// Return the status code carried by an error, or `Panic` if `err` is null.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not
/// yet freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_error_code(err: *const PurrdfError) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if err.is_null() {
                return PurrdfStatus::Panic as i32;
            }
            (*err).code as i32
        })
    }
}

/// Return the borrowed, NUL-terminated message of an error. Valid until
/// `purrdf_error_free(err)`. Returns null if `err` is null.
///
/// # Safety
/// Same contract as [`purrdf_error_code`].
#[no_mangle]
pub unsafe extern "C" fn purrdf_error_message(err: *const PurrdfError) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            if err.is_null() {
                return std::ptr::null();
            }
            (*err).message.as_ptr()
        })
    }
}

/// Release an error handle. No-op on null. Idempotent only in the sense that the
/// caller must not pass the same non-null pointer twice.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not
/// already freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_error_free(err: *mut PurrdfError) {
    unsafe {
        ffi_guard!((), {
            if !err.is_null() {
                drop(Box::from_raw(err));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sanitizes_interior_nul() {
        let err = PurrdfError::new(PurrdfStatus::ParseError, "bad\0input");
        assert_eq!(err.code, PurrdfStatus::ParseError);
        assert_eq!(err.message.to_str().unwrap(), "bad input");
    }

    #[test]
    fn accessors_round_trip() {
        let err = PurrdfError::new(PurrdfStatus::QueryError, "boom");
        let boxed = Box::into_raw(Box::new(err));
        unsafe {
            assert_eq!(purrdf_error_code(boxed), PurrdfStatus::QueryError as i32);
            let msg = std::ffi::CStr::from_ptr(purrdf_error_message(boxed));
            assert_eq!(msg.to_str().unwrap(), "boom");
            purrdf_error_free(boxed);
        }
    }

    #[test]
    fn null_is_safe() {
        unsafe {
            assert_eq!(
                purrdf_error_code(std::ptr::null()),
                PurrdfStatus::Panic as i32
            );
            assert!(purrdf_error_message(std::ptr::null()).is_null());
            purrdf_error_free(std::ptr::null_mut());
        }
    }
}
