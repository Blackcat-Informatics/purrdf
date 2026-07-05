// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_parse`: bytes + media type → a fresh frozen dataset.

use std::os::raw::c_char;

use purrdf_core::RdfDiagnostic;
use purrdf_rs::{DatasetSink, GtsCodecBackend, RdfParseRequest, RdfParserBackend};

use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// Map a parser diagnostic to the most specific C status.
fn parse_status(diagnostic: &RdfDiagnostic) -> PurrdfStatus {
    if diagnostic.code.contains("unsupported-format") {
        PurrdfStatus::UnsupportedFormat
    } else {
        PurrdfStatus::ParseError
    }
}

/// Parse `len` bytes of `media_type` (e.g. `"text/turtle"`, `"application/n-quads"`)
/// into a fresh frozen dataset. `base_iri` and `source_name` may be null. On
/// success `*out_dataset` is a caller-owned handle (free with
/// `purrdf_dataset_free`); on failure `*out_error` is set.
///
/// # Safety
/// `bytes` must be valid for `len` bytes; the `c_char` pointers must be null or
/// NUL-terminated; the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_parse(
    bytes: *const u8,
    len: usize,
    media_type: *const c_char,
    base_iri: *const c_char,
    source_name: *const c_char,
    out_dataset: *mut *mut PurrdfDataset,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if bytes.is_null() || media_type.is_null() || out_dataset.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_parse",
                ));
            }
            let slice = std::slice::from_raw_parts(bytes, len);
            let media = cstr_to_str(media_type)?;
            let base_iri = opt_cstr_to_str(base_iri)?;
            let source_name = opt_cstr_to_str(source_name)?;

            let mut sink = DatasetSink::new();
            GtsCodecBackend
                .parse_into(
                    RdfParseRequest {
                        bytes: slice,
                        media_type: media,
                        base_iri,
                        source_name,
                    },
                    &mut sink,
                )
                .map_err(|diagnostic| {
                    PurrdfError::from_diagnostic(parse_status(&diagnostic), &diagnostic)
                })?;

            let dataset = sink.into_dataset().ok_or_else(|| {
                PurrdfError::new(PurrdfStatus::ParseError, "parse produced no dataset")
            })?;
            *out_dataset = PurrdfDataset::into_raw(dataset);
            Ok(PurrdfStatus::Ok)
        })
    }
}
