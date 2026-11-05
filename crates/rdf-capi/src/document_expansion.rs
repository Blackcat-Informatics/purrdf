// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source/request byte pairs through the shared structural expansion boundary.

use crate::{
    buffer::PurrdfBuffer, error::PurrdfError, handles::publish_handle, status::PurrdfStatus,
};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_validate::{document_expansion, rdf_input::MAX_OPTIONS_BYTES};

/// Expand original JSON-LD/YAML-LD source bytes into owned expanded JSON bytes.
/// Source/request pointers are borrowed only during this synchronous call.
/// Success publishes one buffer, without an added newline or NUL. Release it
/// with `purrdf_buffer_free`; diagnostic ownership ends at `purrdf_error_free`.
/// Request/native phases and the final host buffer control are separate accounts.
/// # Safety
/// Both input pointers must be non-null and readable for their addressable lengths.
/// Output/error slots must be writable, initially empty, and independent of input
/// storage. Aliased output/error slots are refused without writing through either.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_document_expand(
    source: *const u8,
    source_len: usize,
    request_json: *const u8,
    request_len: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    if !out_error.is_null() && out_buffer.cast::<*mut PurrdfError>() == out_error {
        return PurrdfStatus::InvalidArgument as i32;
    }
    unsafe {
        ffi_try!(out_error, {
            if out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "document expansion requires an output buffer slot",
                ));
            }
            *out_buffer = std::ptr::null_mut();
            if source.is_null() || request_json.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "document source and request bytes are required",
                ));
            }
            if source_len > isize::MAX as usize || request_len > isize::MAX as usize {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "document source/request length exceeds the addressable slice range",
                ));
            }
            if request_len > MAX_OPTIONS_BYTES {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "document expansion request exceeds its native options byte limit",
                ));
            }
            let request =
                document_expansion::request(std::slice::from_raw_parts(request_json, request_len))
                    .map_err(|diagnostic| {
                        PurrdfError::from_owned_diagnostic(
                            PurrdfStatus::InvalidArgument,
                            diagnostic,
                        )
                    })?;
            if source_len > request.max_input_bytes() {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "document source exceeds its native input byte limit",
                ));
            }
            let text = document_expansion::expand_decoded(
                std::slice::from_raw_parts(source, source_len),
                &request,
            )
            .map_err(|diagnostic| {
                let status = if diagnostic.code == "document-expansion-output" {
                    PurrdfStatus::SerializeError
                } else {
                    PurrdfStatus::ParseError
                };
                PurrdfError::from_owned_diagnostic(status, diagnostic)
            })?;
            let mut resident = Resident;
            let buffer = Memory::new(&mut resident)
                .boxed(PurrdfBuffer::from(text))
                .map_err(|cause| {
                    PurrdfError::from_owned_diagnostic(PurrdfStatus::StorageError, cause.into())
                })?;
            // All native work and fallible host-control admission is complete.
            *out_buffer = publish_handle(buffer);
            Ok(PurrdfStatus::Ok)
        })
    }
}
