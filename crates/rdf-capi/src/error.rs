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
    execution_receipt: Option<CString>,
    details: Option<Box<FailureDetails>>,
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

/// Original native causes and optional canonicalization evidence. Neutral to the
/// operation that produced the refusal.
#[derive(Debug, Default)]
struct FailureDetails {
    canonicalization_json: Option<CString>,
    native_cause_json: Option<CString>,
    original_cause: Option<OriginalCause>,
    inspection_failure: Option<purrdf_validate::native_error::InspectionError>,
}

#[derive(Debug)]
enum OriginalCause {
    Canonical(purrdf_core::CanonError),
    Diagnostic(RdfDiagnostic),
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
    /// Retain a reasoning request's complete native cause and existing plain text.
    pub(crate) fn reasoning(error: purrdf_validate::regime::ReasoningError) -> Self {
        let code = if purrdf_validate::canonical::native_canonical_cause(&error).is_some() {
            PurrdfStatus::CanonicalizationError
        } else {
            PurrdfStatus::ParseError
        };
        let diagnostic = error.into_diagnostic();
        let message = diagnostic.message.clone();
        Self::from_owned_diagnostic_with_message(code, diagnostic, &message)
    }

    /// Preserve the original native canonicalization cause while mapping host text.
    pub(crate) fn service(error: impl std::error::Error + 'static) -> Self {
        Self::new(PurrdfStatus::ParseError, error.to_string()).retaining_cause(&error)
    }

    pub(crate) fn retaining_cause(mut self, error: &(dyn std::error::Error + 'static)) -> Self {
        if let Some(cause) = purrdf_validate::canonical::native_canonical_cause(error) {
            self.code = PurrdfStatus::CanonicalizationError;
            self.details.get_or_insert_with(Box::default).original_cause =
                Some(OriginalCause::Canonical(cause.clone()));
        }
        self
    }
    /// Build an error from a status and a message. Interior NUL bytes in the
    /// message are replaced with spaces so the `CString` construction never
    /// fails.
    pub(crate) fn new(code: PurrdfStatus, message: impl Into<String>) -> Self {
        Self {
            execution_receipt: None,
            details: None,
            code,
            message: sanitized(&message.into()),
            dimension: None,
            import: None,
            diagnostic: None,
        }
    }

    /// Map a shapes-graph entry point's error onto the C error channel: the
    /// `owl:imports` refusal as [`PurrdfStatus::ShapesImportError`] carrying its kind and
    /// IRIs, native storage refusal with its diagnostic record, and other failures
    /// as a `ParseError` with the engine's message.
    pub(crate) fn shapes(error: purrdf_validate::ShapesError) -> Self {
        match error {
            purrdf_validate::ShapesError::ImportArrays(cause) => {
                let diagnostic = purrdf_validate::native_error::diagnostic(
                    "import-array-refused",
                    cause,
                    cause.storage_error(),
                );
                let message = diagnostic.message.clone();
                Self::from_owned_diagnostic_with_message(
                    PurrdfStatus::ParseError,
                    diagnostic,
                    &message,
                )
            }
            purrdf_validate::ShapesError::ScopeBinding(cause) => {
                let diagnostic =
                    purrdf_validate::native_error::diagnostic("scope-binding-refused", cause, None);
                let message = diagnostic.message.clone();
                Self::from_owned_diagnostic_with_message(
                    PurrdfStatus::ParseError,
                    diagnostic,
                    &message,
                )
            }
            purrdf_validate::ShapesError::Canonical(cause) => Self::service(cause),
            purrdf_validate::ShapesError::RuleExecution(error) => {
                let receipt =
                    purrdf_validate::query::rule_execution_receipt_json(&error.coordinates);
                let cause = error.into_cause();
                let error = match cause {
                    purrdf_rs::shapes::rules::RulesError::Shapes(error) => Self::shapes(error),
                    other => Self::native_shacl(other),
                };
                error.with_execution_receipt(&receipt)
            }
            purrdf_validate::ShapesError::Storage(error) => {
                Self::from_diagnostic(PurrdfStatus::StorageError, &error.into())
            }
            purrdf_validate::ShapesError::Imports(error) => Self::shapes_import(&error),
            other => Self::native_shacl(other),
        }
    }

    fn native_shacl(error: impl purrdf_lex::error::NativeError + 'static) -> Self {
        let diagnostic = purrdf_validate::native_error::diagnostic("shapes-refused", error, None);
        let message = diagnostic.message.clone();
        Self::from_owned_diagnostic_with_message(PurrdfStatus::ParseError, diagnostic, &message)
    }

    pub(crate) fn complete(error: purrdf_rs::shapes::report::CompleteValidationError) -> Self {
        match error {
            purrdf_rs::shapes::report::CompleteValidationError::Shapes(error) => {
                Self::shapes(error)
            }
            purrdf_rs::shapes::report::CompleteValidationError::RuleExecution {
                cause,
                coordinates,
            } => Self::complete(*cause).with_execution_receipt(
                &purrdf_validate::query::rule_execution_receipt_json(&coordinates),
            ),
            other => Self::native_shacl(other),
        }
    }

    /// The [`PurrdfStatus::ShapesImportError`] for `error`.
    pub(crate) fn shapes_import(error: &purrdf_validate::ShapesImportError) -> Self {
        let diagnostic =
            purrdf_validate::native_error::diagnostic("shapes-import-refused", error.clone(), None);
        let message = diagnostic.message.clone();
        let mut result = Self::from_owned_diagnostic_with_message(
            PurrdfStatus::ShapesImportError,
            diagnostic,
            &message,
        );
        result.import = Some(ImportRefusal {
            kind: sanitized(error.kind()),
            iris: error.iris().into_iter().map(sanitized).collect(),
        });
        result
    }

    /// Build a prepared-shapes-product refusal, carrying the pinned kebab-case
    /// dimension label alongside the message.
    ///
    /// The label is taken from the codec's own `ProductDimension::label`, never
    /// re-spelled here: a second transcription of a twenty-entry pinned contract is
    /// exactly how a renamed dimension would reach C hosts under its old name.
    pub(crate) fn product(dimension: Option<&'static str>, message: impl Into<String>) -> Self {
        Self {
            execution_receipt: None,
            details: None,
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

    /// Own the diagnostic and its original native cause through error_free.
    /// A broken inspection protocol becomes an explicit failure while its
    /// original diagnostic remains owned; it cannot become partial success.
    pub(crate) fn from_owned_diagnostic(code: PurrdfStatus, diagnostic: RdfDiagnostic) -> Self {
        Self::from_owned_diagnostic_text(code, diagnostic, None)
    }

    /// Retain the original diagnostic with a boundary's existing plain message.
    pub(crate) fn from_owned_diagnostic_with_message(
        code: PurrdfStatus,
        diagnostic: RdfDiagnostic,
        message: &str,
    ) -> Self {
        Self::from_owned_diagnostic_text(code, diagnostic, Some(message))
    }

    fn from_owned_diagnostic_text(
        code: PurrdfStatus,
        diagnostic: RdfDiagnostic,
        message: Option<&str>,
    ) -> Self {
        let code = if diagnostic.storage_error().is_some() {
            PurrdfStatus::StorageError
        } else {
            code
        };
        let inspected = diagnostic
            .native_cause()
            .map(|cause| purrdf_validate::native_error::to_json(cause))
            .transpose();
        let mut error = Self::from_diagnostic(code, &diagnostic);
        if let Some(message) = message {
            error.message = sanitized(message);
        }
        match inspected {
            Ok(record) => {
                let details = error.details.get_or_insert_with(Box::default);
                details.native_cause_json = record.map(|record| sanitized(&record.to_string()));
                details.original_cause = Some(OriginalCause::Diagnostic(diagnostic));
            }
            Err(failure) => {
                let message = format!("{failure}; original refusal: {diagnostic}");
                error = Self::from_diagnostic(
                    PurrdfStatus::ParseError,
                    &RdfDiagnostic::error("native-cause-inspection-failed", message),
                );
                let details = error.details.get_or_insert_with(Box::default);
                details.original_cause = Some(OriginalCause::Diagnostic(diagnostic));
                details.inspection_failure = Some(failure);
            }
        }
        error
    }

    pub(crate) fn with_canonicalization(
        mut self,
        receipts: &str,
        cause: Option<(purrdf_core::CanonError, String)>,
    ) -> Self {
        let canonical = self.details.get_or_insert_with(Box::default);
        canonical.canonicalization_json = Some(sanitized(receipts));
        if let Some((original, json)) = cause {
            if canonical.original_cause.is_none() {
                canonical.original_cause = Some(OriginalCause::Canonical(original));
            }
            if canonical.native_cause_json.is_none() && canonical.inspection_failure.is_none() {
                canonical.native_cause_json = Some(sanitized(&json));
            }
        }
        self
    }

    /// Attach the actual native receipt to the original classified failure.
    pub(crate) fn with_execution_receipt(mut self, receipt: &str) -> Self {
        self.execution_receipt = Some(sanitized(receipt));
        self
    }
}

impl std::fmt::Display for PurrdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message.to_string_lossy())
    }
}

impl std::error::Error for PurrdfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let details = self.details.as_ref()?;
        if let Some(failure) = &details.inspection_failure {
            return Some(failure);
        }
        match details.original_cause.as_ref()? {
            OriginalCause::Canonical(cause) => Some(cause),
            OriginalCause::Diagnostic(diagnostic) => Some(diagnostic),
        }
    }
}

/// Borrow a failed execution's JSON receipt, or null when no execution was attempted.
///
/// # Safety
/// `error` must be null or a live error handle; the returned text lives with it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_error_execution_receipt_json(
    error: *const PurrdfError,
) -> *const c_char {
    unsafe {
        error
            .as_ref()
            .and_then(|error| error.execution_receipt.as_ref())
            .map_or(std::ptr::null(), |receipt| receipt.as_ptr())
    }
}

/// Borrow this canonicalization request's receipt JSON, including `[]` if no run
/// started. Null denotes an error from another kind of operation.
/// # Safety
/// `error` must be null or a live error handle. The text lives until that handle is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_error_canonicalization_json(
    error: *const PurrdfError,
) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            error
                .as_ref()
                .and_then(|error| error.details.as_ref()?.canonicalization_json.as_ref())
                .map_or(std::ptr::null(), |value| value.as_ptr())
        })
    }
}

/// Borrow the original native cause's closed fields as JSON.
/// The original native cause and its admission grant remain owned until error_free.
/// Null means no native cause record is available. An inspection failure is
/// reported explicitly by the error status/message and diagnostic, never as success.
/// # Safety
/// `error` must be null or a live error handle. The text lives until that handle is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_error_native_cause_json(
    error: *const PurrdfError,
) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            error
                .as_ref()
                .and_then(|error| {
                    let canonical = error.details.as_ref()?;
                    canonical.original_cause.as_ref()?;
                    canonical.native_cause_json.as_ref()
                })
                .map_or(std::ptr::null(), |value| value.as_ptr())
        })
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

    /// A canonical refusal the document path can now raise reaches a C caller as
    /// a canonicalization failure that keeps the original cause, and an ordinary
    /// service failure keeps its own status.
    #[test]
    fn source_inconsistent_maps_to_a_canonicalization_error_with_its_cause() {
        let error = PurrdfError::service(
            purrdf_core::CanonError::<std::convert::Infallible>::SourceInconsistent,
        );
        assert_eq!(error.code, PurrdfStatus::CanonicalizationError);
        assert!(matches!(
            std::error::Error::source(&error).and_then(|cause| cause.downcast_ref()),
            Some(purrdf_core::CanonError::<std::convert::Infallible>::SourceInconsistent)
        ));
        let ordinary = PurrdfError::service(std::fmt::Error);
        assert_eq!(ordinary.code, PurrdfStatus::ParseError);
    }

    #[test]
    fn shapes_capacity_refusal_preserves_native_fields() {
        let cause: purrdf_lex::allocation::StorageError = purrdf_core::TermId::from_index(u32::MAX)
            .unwrap_err()
            .into();
        let error = PurrdfError::shapes(purrdf_validate::ShapesError::Storage(cause));
        assert_eq!(error.code, PurrdfStatus::StorageError);
        let expected = RdfDiagnostic::from(cause).to_json().to_string();
        assert_eq!(error.diagnostic.unwrap().to_str().unwrap(), expected);
        assert!(expected.contains("\"capacity\""));

        let boundary = PurrdfError::from_owned_diagnostic_with_message(
            PurrdfStatus::QueryError,
            RdfDiagnostic::from(cause),
            "entailment query failed: input\0unavailable",
        );
        assert_eq!(boundary.code, PurrdfStatus::StorageError);
        assert_eq!(
            boundary.message.to_str().unwrap(),
            "entailment query failed: input unavailable"
        );
        assert_eq!(boundary.diagnostic.unwrap().to_str().unwrap(), expected);
    }

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
