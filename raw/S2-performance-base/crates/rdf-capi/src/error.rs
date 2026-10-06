// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The opaque error handle and its accessors.
//!
//! Fallible entry points set `*out_error` to a heap-owned `PurrdfError` when
//! they fail. The caller reads `purrdf_error_code` / `purrdf_error_message` and
//! must release it with `purrdf_error_free`.

use std::ffi::{CString, c_char};

use purrdf_core::RdfDiagnostic;

use crate::handles::{free_handle, into_handle};
use crate::status::PurrdfStatus;

/// An owned error: a status code plus a NUL-terminated message, and — for the one
/// boundary that has one — a named DIMENSION. Opaque to C.
#[derive(Debug)]
pub struct PurrdfError {
    pub(crate) code: PurrdfStatus,
    pub(crate) message: CString,
    /// The prepared-shapes-product admission dimension this refusal names, when it
    /// names one.
    ///
    /// A slot on the shared error rather than a second error type, because the C ABI
    /// has exactly one error channel and every entry point routes through it. It is
    /// `None` for every error that is not a product refusal, and
    /// `purrdf_shapes_product_error_dimension` returns NULL for those — an honest
    /// "this refusal names no dimension" rather than an empty string a caller could
    /// mistake for a label.
    pub(crate) dimension: Option<CString>,
    /// The shapes-graph `owl:imports` refusal this error is, when it is one: its kind
    /// label and the IRIs it names. `None` for every other error, and the
    /// `purrdf_shapes_import_error_*` accessors answer NULL / 0 for those.
    pub(crate) import: Option<ImportRefusal>,
    /// Original structured RDF presentation, borrowed through the error handle.
    diagnostic: Option<CString>,
}

/// The typed half of a [`PurrdfStatus::ShapesImportError`], kept as C strings so the
/// accessors can hand out borrows valid until `purrdf_error_free`.
#[derive(Debug)]
pub(crate) struct ImportRefusal {
    /// `unresolved-import`, `unreached-import`, `incompatible-import-versions`,
    /// `invalid-import`, `unresolved-shapes-graph-link`, `unheld-shapes-graph-link` or
    /// `invalid-shapes-graph-link`.
    pub(crate) kind: CString,
    /// The IRIs the refusal names, in the engine's order.
    pub(crate) iris: Vec<CString>,
}

impl PurrdfError {
    /// Build an error from a status and a message. Interior NUL bytes in the
    /// message are replaced with spaces so the `CString` construction never
    /// fails.
    pub(crate) fn new(code: PurrdfStatus, message: impl Into<String>) -> Self {
        Self {
            code,
            message: sanitized(&message.into()),
            dimension: None,
            import: None,
            diagnostic: None,
        }
    }

    /// Map a shapes-graph entry point's error onto the C error channel: the
    /// `owl:imports` refusal as [`PurrdfStatus::ShapesImportError`] carrying its kind and
    /// IRIs, and anything else as a `ParseError` with the engine's message.
    pub(crate) fn shapes(error: purrdf_validate::ShapesError) -> Self {
        match error {
            purrdf_validate::ShapesError::Imports(error) => Self::shapes_import(&error),
            purrdf_validate::ShapesError::Invalid(message) => {
                Self::new(PurrdfStatus::ParseError, message)
            }
            purrdf_validate::ShapesError::ShaclJs(refusal) => {
                Self::new(PurrdfStatus::ParseError, refusal.to_string())
            }
            purrdf_validate::ShapesError::IllFormed(refusal) => {
                Self::new(PurrdfStatus::ParseError, refusal.to_string())
            }
            purrdf_validate::ShapesError::Prebinding(violation) => {
                Self::new(PurrdfStatus::ParseError, violation.to_string())
            }
            purrdf_validate::ShapesError::UnsupportedTarget(refusal) => {
                Self::new(PurrdfStatus::ParseError, refusal.to_string())
            }
            purrdf_validate::ShapesError::SparqlTargetDisagreement(refusal) => {
                Self::new(PurrdfStatus::ParseError, refusal.to_string())
            }
        }
    }

    /// The [`PurrdfStatus::ShapesImportError`] for `error`.
    pub(crate) fn shapes_import(error: &purrdf_validate::ShapesImportError) -> Self {
        Self {
            code: PurrdfStatus::ShapesImportError,
            message: sanitized(&error.to_string()),
            dimension: None,
            diagnostic: None,
            import: Some(ImportRefusal {
                kind: sanitized(error.kind()),
                iris: error.iris().into_iter().map(sanitized).collect(),
            }),
        }
    }

    /// Build a prepared-shapes-product refusal, carrying the pinned kebab-case
    /// dimension label alongside the message.
    ///
    /// The label is taken from the codec's own `ProductDimension::label`, never
    /// re-spelled here: a second transcription of a twenty-entry pinned contract is
    /// exactly how a renamed dimension would reach C hosts under its old name.
    pub(crate) fn product(dimension: Option<&'static str>, message: impl Into<String>) -> Self {
        Self {
            code: PurrdfStatus::ShapesProductError,
            message: sanitized(&message.into()),
            dimension: dimension.map(sanitized),
            import: None,
            diagnostic: None,
        }
    }

    /// Map a kernel [`RdfDiagnostic`] to a `PurrdfError` under the given C status,
    /// preserving the diagnostic's own code and message.
    pub(crate) fn from_diagnostic(code: PurrdfStatus, diagnostic: &RdfDiagnostic) -> Self {
        let mut error = Self::new(
            code,
            format!("[{}] {}", diagnostic.code, diagnostic.message),
        );
        error.diagnostic = Some(sanitized(&diagnostic.to_json().to_string()));
        error
    }
}

/// Render `raw` as a C string, replacing interior NUL bytes with spaces so the
/// construction never fails.
fn sanitized(raw: &str) -> CString {
    CString::new(raw.replace('\0', " ")).unwrap_or_else(|_| {
        CString::new("libpurrdf error (unprintable message)").expect("static message")
    })
}

/// Store `err` at `*out` (heap-owned), or drop it if `out` is null.
pub(crate) fn store_error(out: *mut *mut PurrdfError, err: PurrdfError) {
    if out.is_null() {
        return;
    }
    // SAFETY: `out` is non-null and, per the ABI contract, points to a writable
    // `*mut PurrdfError` out-param.
    unsafe {
        *out = into_handle(err);
    }
}

/// Return the status code carried by an error, or `Panic` if `err` is null.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not
/// yet freed.
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
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

/// Return a borrowed JSON record of the original RDF diagnostic, including its
/// stable code, optional message identity, typed parameters and logical anchors. Exact integers are decimal strings with type labels.
/// Returns null when the error carries no RDF diagnostic or `err` is null.
/// The pointer remains valid until `purrdf_error_free(err)`.
///
/// # Safety
/// Same contract as [`purrdf_error_code`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_error_presentation_json(err: *const PurrdfError) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            if err.is_null() {
                return std::ptr::null();
            }
            (*err)
                .diagnostic
                .as_ref()
                .map_or(std::ptr::null(), |record| record.as_ptr())
        })
    }
}

/// Release an error handle. No-op on null. Idempotent only in the sense that the
/// caller must not pass the same non-null pointer twice.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not
/// already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_error_free(err: *mut PurrdfError) {
    unsafe { free_handle::<PurrdfError>(err) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_machine_record_preserves_typed_fields_and_large_anchors() {
        let diagnostic = RdfDiagnostic::from_iri(&purrdf_rs::IriError::BadPercentEncoding(7))
            .with_location(
                purrdf_core::RdfLocation::file("data.nt")
                    .with_line((1 << 32) + 1)
                    .with_gts_frame((1 << 53) + 1),
            );
        let error = into_handle(PurrdfError::from_diagnostic(
            PurrdfStatus::ParseError,
            &diagnostic,
        ));
        unsafe {
            let record = std::ffi::CStr::from_ptr(purrdf_error_presentation_json(error))
                .to_str()
                .unwrap();
            assert!(record.contains("iri-bad-percent-encoding"));
            assert!(record.contains("\"offset\":{\"kind\":\"unsigned\",\"value\":\"7\"}"));
            assert!(record.contains("\"line\":\"4294967297\""));
            assert!(record.contains("\"gtsFrameIndex\":\"9007199254740993\""));
            purrdf_error_free(error);
            assert!(purrdf_error_presentation_json(std::ptr::null()).is_null());
        }
    }

    #[test]
    fn new_sanitizes_interior_nul() {
        let err = PurrdfError::new(PurrdfStatus::ParseError, "bad\0input");
        assert_eq!(err.code, PurrdfStatus::ParseError);
        assert_eq!(err.message.to_str().unwrap(), "bad input");
    }

    #[test]
    fn accessors_round_trip() {
        let err = PurrdfError::new(PurrdfStatus::QueryError, "boom");
        let boxed = into_handle(err);
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
