// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_serialize`: a frozen dataset → bytes in a requested media type, with
//! the RDF-1.2 statement-layer loss count surfaced (MAXIMAL INFORMATION FLOW).

use std::os::raw::c_char;

use purrdf_rs::{classify, serialize_dataset_to_format};

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// Serialize the frozen dataset to `media_type` (e.g. `"text/turtle"`,
/// `"application/n-quads"`). `base_iri` may be null. The output bytes go to
/// `*out_buffer` (free with `purrdf_buffer_free`). When `out_statement_rows_dropped`
/// is non-null it receives the number of RDF-1.2 statement-layer rows dropped
/// because the target format cannot represent quoted triples (`0` for
/// star-capable formats) — so the caller can detect lossy projection.
///
/// # Safety
/// `dataset` must be a live handle; the `c_char` pointers must be null or
/// NUL-terminated; the out-params must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_serialize(
    dataset: *const PurrdfDataset,
    media_type: *const c_char,
    base_iri: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_statement_rows_dropped: *mut usize,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || media_type.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_serialize",
                ));
            }
            let media = cstr_to_str(media_type)?;
            let base_iri = opt_cstr_to_str(base_iri)?;

            // The media-type registry is the single source of truth (no duplicated map).
            let format = classify(media).map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::UnsupportedFormat, &diagnostic)
            })?;

            let outcome =
                serialize_dataset_to_format(PurrdfDataset::dataset(dataset), format, base_iri)
                    .map_err(|diagnostic| {
                        PurrdfError::from_diagnostic(PurrdfStatus::SerializeError, &diagnostic)
                    })?;

            if !out_statement_rows_dropped.is_null() {
                *out_statement_rows_dropped = outcome.statement_rows_dropped;
            }
            *out_buffer = PurrdfBuffer::into_raw(outcome.bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}
