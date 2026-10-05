// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! # libpurrdf — the PurRDF purrdf semantic RDF-1.2 C-ABI
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
//!   no-backwards-compat exception. The current ABI is **0.9.0 (beta)**; the minor
//!   number tracks the exported signatures (see [`version::PURRDF_ABI_MINOR`]).
//!   Pre-1.0, an incompatible change rides a MINOR bump — see
//!   [`version::PURRDF_ABI_MAJOR`] for the rule and
//!   `tests/abi_signatures.rs` for the snapshot that enforces it.
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
pub mod entail;
pub mod error;
pub mod governor;
pub mod graph;
pub mod gts;
pub mod handles;
pub mod parse;
pub mod projection;
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

/// Borrow the `count` C strings at `array`. `count == 0` is accepted with a null
/// `array`, since there is nothing to dereference. A null `array` with a non-zero
/// count is refused as `NullPointer`, naming the `param` and the `entry` point, and
/// each element goes through [`cstr_to_str`], so a null or non-UTF-8 element is
/// refused before it is read.
///
/// The one reader of a C `(const char *const *, size_t)` string-array argument:
/// every entry point taking such a pair shares this refusal contract.
///
/// # Safety
/// When `count` is non-zero, `array` must address at least `count` readable
/// `*const c_char`, each null (refused here) or a NUL-terminated C string that
/// outlives the returned borrows.
pub(crate) unsafe fn cstr_array<'a>(
    array: *const *const c_char,
    count: usize,
    param: &str,
    entry: &str,
) -> Result<Vec<&'a str>, PurrdfError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if array.is_null() {
        return Err(PurrdfError::new(
            PurrdfStatus::NullPointer,
            format!("null {param} array with a non-zero count ({count}) passed to {entry}"),
        ));
    }
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: the caller's contract above: the array is non-null (checked) and
        // holds at least `count` readable elements, so `index < count` is in bounds.
        // `cstr_to_str` refuses a null element rather than dereferencing it.
        out.push(unsafe { cstr_to_str(*array.add(index))? });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    #[test]
    fn an_empty_string_array_may_be_null() {
        let strings = unsafe { cstr_array(std::ptr::null(), 0, "names", "entry") };
        let strings = strings.expect("a null array of zero strings is empty");
        assert_eq!(strings, Vec::<&str>::new());
    }

    #[test]
    fn a_null_string_array_with_a_count_is_refused_by_name() {
        let error = unsafe { cstr_array(std::ptr::null(), 2, "names", "entry") }
            .expect_err("a null array cannot hold two strings");
        assert_eq!(error.code, PurrdfStatus::NullPointer);
        assert_eq!(
            error.message.to_str().expect("UTF-8"),
            "null names array with a non-zero count (2) passed to entry"
        );
    }

    #[test]
    fn a_null_element_is_refused_and_a_full_array_is_borrowed() {
        let first = CString::new("alpha").expect("no NUL");
        let second = CString::new("beta").expect("no NUL");
        let full = [first.as_ptr(), second.as_ptr()];
        let strings = unsafe { cstr_array(full.as_ptr(), 2, "names", "entry") };
        assert_eq!(
            strings.expect("both elements are strings"),
            ["alpha", "beta"]
        );
        let holed = [first.as_ptr(), std::ptr::null()];
        let error = unsafe { cstr_array(holed.as_ptr(), 2, "names", "entry") }
            .expect_err("a null element is refused");
        assert_eq!(error.code, PurrdfStatus::NullPointer);
    }
}
