// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The C host's explicit policy, original native failure and receipt boundary.

use std::ffi::c_char;

use purrdf_core::CanonicalizationControl;
use purrdf_validate::canonical::{execute, native_canonical_cause, parse_policy, receipts_json};

use crate::{cstr_to_str, error::PurrdfError, governor::PurrdfCancellation, status::PurrdfStatus};

/// Reject missing or aliased required output slots before native execution starts.
pub(crate) fn check_outputs(outputs: &[usize], error_output: usize) -> Result<(), PurrdfError> {
    for (index, &output) in outputs.iter().enumerate() {
        if output == 0 {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "canonicalization requires every output buffer slot",
            )
            .with_canonicalization("[]", None));
        }
        if output == error_output || outputs[..index].contains(&output) {
            return Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                "canonicalization output slots and error slot must not alias",
            )
            .with_canonicalization("[]", None));
        }
    }
    Ok(())
}

/// An optional output may be absent, but it may not overwrite another slot.
pub(crate) fn check_optional_output(
    output: usize,
    other_outputs: &[usize],
) -> Result<(), PurrdfError> {
    if output != 0 && other_outputs.contains(&output) {
        return Err(PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            "optional canonicalization output must not alias another output slot",
        )
        .with_canonicalization("[]", None));
    }
    Ok(())
}

/// The pointers must be null or valid C arguments according to the calling API.
pub(crate) unsafe fn run<T, E>(
    policy: *const c_char,
    cancellation: *const PurrdfCancellation,
    map_error: impl FnOnce(E) -> PurrdfError,
    operation: impl FnOnce(&mut CanonicalizationControl<'_>) -> Result<T, E>,
) -> Result<(T, String), PurrdfError>
where
    E: std::error::Error + 'static,
{
    let policy =
        unsafe { cstr_to_str(policy) }.map_err(|error| error.with_canonicalization("[]", None))?;
    let policy = parse_policy(policy).map_err(|error| {
        PurrdfError::new(PurrdfStatus::InvalidArgument, error.to_string())
            .with_canonicalization("[]", None)
    })?;
    let stop =
        unsafe { cancellation.as_ref() }.map(|signal| signal as &dyn purrdf_core::StopSignal);
    let executed = execute(policy, stop, operation);
    let receipts = receipts_json(&executed.receipts).to_string();
    match executed.result {
        Ok(value) => Ok((value, receipts)),
        Err(error) => {
            let native = native_canonical_cause(&error).map(|cause| {
                purrdf_validate::native_error::to_json(cause).map(|json| (cause.clone(), json.to_string()))
            }).transpose().map_err(|failure| PurrdfError::new(PurrdfStatus::CanonicalizationError,
                format!("native canonicalization cause inspection failed: {failure}; original refusal: {error}"))
                .retaining_cause(&error)
                .with_canonicalization(&receipts, None))?;
            let mut refused = map_error(error);
            if native.is_some()
                && matches!(
                    refused.code,
                    PurrdfStatus::ParseError
                        | PurrdfStatus::SerializeError
                        | PurrdfStatus::QueryError
                )
            {
                refused.code = PurrdfStatus::CanonicalizationError;
            }
            Err(refused.with_canonicalization(&receipts, native))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::CStr,
        ptr::{null, null_mut},
    };

    use crate::{
        entail::purrdf_entail_materialize_to_nquads,
        error::{
            purrdf_error_canonicalization_json, purrdf_error_free, purrdf_error_native_cause_json,
        },
        receipt::Output,
    };

    use super::*;

    #[test]
    fn c_search_refusal_preserves_outputs_and_the_successful_neighbor_has_separate_evidence() {
        let mut document = Output::default();
        let mut report = Output::default();
        let mut evidence = Output::default();
        let mut error = null_mut();
        let input = c"_:a <http://example.org/p> _:b .\n_:b <http://example.org/p> _:a .\n";
        unsafe {
            let status = purrdf_entail_materialize_to_nquads(
                input.as_ptr(),
                c"simple".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                &raw mut document.0,
                &raw mut report.0,
                c"0".as_ptr(),
                null(),
                &raw mut evidence.0,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::CanonicalizationError as i32);
            assert!(document.0.is_null() && report.0.is_null() && evidence.0.is_null());
            let receipts = CStr::from_ptr(purrdf_error_canonicalization_json(error))
                .to_str()
                .unwrap();
            assert!(receipts.contains("\"finite\":\"0\""), "{receipts}");
            let native = CStr::from_ptr(purrdf_error_native_cause_json(error))
                .to_str()
                .unwrap();
            assert!(native.contains("budget"), "{native}");
            purrdf_error_free(error);
            error = null_mut();
            let status = purrdf_entail_materialize_to_nquads(
                input.as_ptr(),
                c"simple".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                &raw mut document.0,
                &raw mut report.0,
                c"unlimited".as_ptr(),
                null(),
                &raw mut evidence.0,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::Ok as i32);
            assert!(error.is_null());
            assert!(!document.0.is_null() && !report.0.is_null() && !evidence.0.is_null());
            let document_buffer = document.0.as_ref().expect("successful document output");
            let document_text = std::str::from_utf8(document_buffer.as_bytes()).unwrap();
            assert!(document_text.starts_with("_:c14n"), "{document_text}");
            let evidence_buffer = evidence.0.as_ref().expect("successful receipt output");
            let receipts = std::str::from_utf8(evidence_buffer.as_bytes()).unwrap();
            assert!(
                receipts.contains("unlimited") && receipts.contains("\"completed\":true"),
                "{receipts}"
            );
        }
    }

    #[test]
    fn c_native_cause_outlives_its_source_and_aliases_refuse_before_execution() {
        let mut document = Output::default();
        let mut report = Output::default();
        let mut evidence = Output::default();
        let mut error = null_mut();
        unsafe {
            let source = std::ffi::CString::new(
                "<urn:purrdf:rdfc:private> <http://example.org/p> <http://example.org/o> .",
            )
            .unwrap();
            let status = purrdf_entail_materialize_to_nquads(
                source.as_ptr(),
                c"simple".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                &raw mut document.0,
                &raw mut report.0,
                c"unlimited".as_ptr(),
                null(),
                &raw mut evidence.0,
                &raw mut error,
            );
            drop(source);
            assert_eq!(status, PurrdfStatus::CanonicalizationError as i32);
            let native = CStr::from_ptr(purrdf_error_native_cause_json(error))
                .to_str()
                .unwrap();
            assert!(native.contains("urn:purrdf:rdfc:private"), "{native}");
            let error_ref = error.as_ref().expect("native canonical refusal");
            assert!(matches!(
                std::error::Error::source(error_ref)
                    .and_then(|cause| cause.downcast_ref::<purrdf_core::CanonError>()),
                Some(purrdf_core::CanonError::ReservedVocabulary(_))
            ));
            purrdf_error_free(error);
            error = null_mut();
            let same_slot = &raw mut document.0;
            let status = purrdf_entail_materialize_to_nquads(
                c"".as_ptr(),
                c"simple".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                same_slot,
                &raw mut report.0,
                c"unlimited".as_ptr(),
                null(),
                same_slot,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::InvalidArgument as i32);
            assert!(document.0.is_null() && report.0.is_null());
            assert_eq!(
                CStr::from_ptr(purrdf_error_canonicalization_json(error)).to_bytes(),
                b"[]"
            );
            purrdf_error_free(error);
        }
    }
}
