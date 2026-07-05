// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # libpurrdf — the PurRDF purrdf semantic RDF-1.2 C-ABI (purrdf P8)
//!
//! A stable, SemVer-disciplined `extern "C"` surface over the native
//! `purrdf` semantic stack. It is the rich companion to the permissive
//! `libgts` C-ABI: where `libgts` is transport/format only, `libpurrdf` exposes
//! parse / serialize / pattern iteration / copy-on-write mutation / SPARQL /
//! GTS round-trip. A language shim links **`libpurrdf` alone** — it statically
//! reuses the permissive `purrdf-gts` crate, so no second `.so` is needed.
//!
//! ## ABI contract (every entry point)
//! - **No unwinding across the boundary.** Every `extern "C"` function body runs
//!   inside [`std::panic::catch_unwind`] via the `ffi_try!` / `ffi_guard!`
//!   macros. A caught panic becomes [`status::PurrdfStatus::Panic`].
//! - **`int32` status + out-params.** Fallible functions return a
//!   [`status::PurrdfStatus`] as `i32` and write results through out-pointers;
//!   on error they set `*out_error` to an owned [`error::PurrdfError`].
//! - **Explicit ownership.** Every handle / buffer / error / cursor the library
//!   hands out has exactly one matching `*_free`. Borrowed UTF-8 slices
//!   (`PurrdfStr`) point into library-owned memory — the C side
//!   **never** `free()`s a `PurrdfStr.ptr`.
//! - **SemVer-frozen ABI.** The status enum is append-only; the committed
//!   `include/purrdf.h` is the contract. This is the project's one sanctioned
//!   no-backwards-compat exception. The current ABI is **0.1.0 (beta)**.
//!
//! ## Thread-safety (per handle)
//! - [`handles::PurrdfDataset`] wraps `Arc<RdfDataset>` — `Send + Sync`; it may
//!   be read concurrently from multiple threads.
//! - `PurrdfGraph` (COW delta), `PurrdfCursor`, `PurrdfRowCursor` are
//!   single-threaded mutable; do not touch one from two threads without external
//!   locking.

#![deny(improper_ctypes_definitions)]

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::PurrdfError;
use crate::status::PurrdfStatus;

/// Wrap a fallible entry point: catch panics, route `Err` to `*out_error`, and
/// return the `i32` status. The body must evaluate to
/// `Result<PurrdfStatus, PurrdfError>`; the error carries its own status code.
macro_rules! ffi_try {
    ($err_out:expr, $body:block) => {{
        let err_out: *mut *mut $crate::error::PurrdfError = $err_out;
        let outcome = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(
            || -> ::core::result::Result<$crate::status::PurrdfStatus, $crate::error::PurrdfError> { $body },
        ));
        match outcome {
            ::core::result::Result::Ok(::core::result::Result::Ok(status)) => status as i32,
            ::core::result::Result::Ok(::core::result::Result::Err(err)) => {
                let code = err.code;
                $crate::error::store_error(err_out, err);
                code as i32
            }
            ::core::result::Result::Err(panic) => {
                let err = $crate::error::PurrdfError::new(
                    $crate::status::PurrdfStatus::Panic,
                    $crate::panic_message(panic.as_ref()),
                );
                $crate::error::store_error(err_out, err);
                $crate::status::PurrdfStatus::Panic as i32
            }
        }
    }};
}

/// Wrap an infallible entry point (a `*_free` or a simple getter) that has no
/// error channel: catch panics and return `$default` instead of unwinding.
macro_rules! ffi_guard {
    ($default:expr, $body:block) => {{
        match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| $body)) {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(_) => $default,
        }
    }};
}

pub mod buffer;
pub mod cursor;
pub mod error;
pub mod graph;
pub mod gts;
pub mod handles;
pub mod parse;
pub mod query;
pub mod rowcursor;
pub mod serialize;
pub mod shacl;
pub mod status;
pub mod term;
pub mod version;

/// Render a caught panic payload as a human-readable message.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic in libpurrdf (non-string payload)".to_string()
    }
}

/// Borrow a non-null C string as `&str`. `NullPointer` if null, `InvalidUtf8` if
/// the bytes are not valid UTF-8.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated C string valid for the
/// returned reference's lifetime.
pub(crate) unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, PurrdfError> {
    unsafe {
        if ptr.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null C string pointer",
            ));
        }
        CStr::from_ptr(ptr)
            .to_str()
            .map_err(|_| PurrdfError::new(PurrdfStatus::InvalidUtf8, "C string is not valid UTF-8"))
    }
}

/// Borrow an optional C string: null → `None`, otherwise `Some(&str)`.
///
/// # Safety
/// Same contract as [`cstr_to_str`].
pub(crate) unsafe fn opt_cstr_to_str<'a>(
    ptr: *const c_char,
) -> Result<Option<&'a str>, PurrdfError> {
    unsafe {
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(cstr_to_str(ptr)?))
        }
    }
}
