// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source/request byte pairs through the shared structural document boundary.

use crate::{
    buffer::PurrdfBuffer, error::PurrdfError, handles::publish_handle, status::PurrdfStatus,
};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_validate::{
    document::{self, Operation},
    rdf_input::MAX_OPTIONS_BYTES,
};

#[derive(Clone, Copy)]
struct InputBytes {
    data: *const u8,
    len: usize,
}

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
    unsafe {
        invoke(
            Operation::Expand,
            InputBytes {
                data: source,
                len: source_len,
            },
            None,
            InputBytes {
                data: request_json,
                len: request_len,
            },
            out_buffer,
            out_error,
        )
    }
}

/// Compact source bytes under the explicit output context in the request.
/// Returns one owned UTF-8 JSON buffer; release buffer/error through their free functions.
/// # Safety
/// Inputs must be readable for their addressable lengths, and non-null. Output
/// slots must be writable, initially empty and independent of the input storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_document_compact(
    source: *const u8,
    source_len: usize,
    request_json: *const u8,
    request_len: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        invoke(
            Operation::Compact,
            InputBytes {
                data: source,
                len: source_len,
            },
            None,
            InputBytes {
                data: request_json,
                len: request_len,
            },
            out_buffer,
            out_error,
        )
    }
}

/// Flatten source bytes, compacting only with a non-null request output context.
/// Returns one owned UTF-8 JSON buffer; release buffer/error through their free functions.
/// # Safety
/// Inputs must be readable for their addressable lengths, and non-null. Output
/// slots must be writable, initially empty and independent of the input storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_document_flatten(
    source: *const u8,
    source_len: usize,
    request_json: *const u8,
    request_len: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        invoke(
            Operation::Flatten,
            InputBytes {
                data: source,
                len: source_len,
            },
            None,
            InputBytes {
                data: request_json,
                len: request_len,
            },
            out_buffer,
            out_error,
        )
    }
}

/// Frame separate source/frame bytes through the native two-document operation.
/// Success publishes one owned UTF-8 JSON buffer without a newline or NUL.
/// Release the buffer with `purrdf_buffer_free` and errors with `purrdf_error_free`.
/// # Safety
/// All three inputs must be non-null and readable for their addressable lengths.
/// Output/error slots must be writable, initially empty and independent of input
/// storage. Aliased slots are refused without writing through either.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_document_frame(
    source: *const u8,
    source_len: usize,
    frame: *const u8,
    frame_len: usize,
    request_json: *const u8,
    request_len: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        invoke(
            Operation::Frame,
            InputBytes {
                data: source,
                len: source_len,
            },
            Some(InputBytes {
                data: frame,
                len: frame_len,
            }),
            InputBytes {
                data: request_json,
                len: request_len,
            },
            out_buffer,
            out_error,
        )
    }
}

unsafe fn invoke(
    operation: Operation,
    source: InputBytes,
    frame: Option<InputBytes>,
    request_json: InputBytes,
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
                    format!(
                        "document {} requires an output buffer slot",
                        operation.name()
                    ),
                ));
            }
            *out_buffer = std::ptr::null_mut();
            let inputs = [Some(source), frame, Some(request_json)];
            if inputs.iter().flatten().any(|input| input.data.is_null()) {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    if frame.is_some() {
                        "document source, frame and request bytes are required"
                    } else {
                        "document source and request bytes are required"
                    },
                ));
            }
            if inputs
                .iter()
                .flatten()
                .any(|input| input.len > isize::MAX as usize)
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    if frame.is_some() {
                        "document source/frame/request length exceeds the addressable slice range"
                    } else {
                        "document source/request length exceeds the addressable slice range"
                    },
                ));
            }
            if request_json.len > MAX_OPTIONS_BYTES {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    format!(
                        "document {} request exceeds its native options byte limit",
                        operation.name()
                    ),
                ));
            }
            let request = document::request(
                std::slice::from_raw_parts(request_json.data, request_json.len),
                operation,
            )
            .map_err(|diagnostic| {
                PurrdfError::from_owned_diagnostic(PurrdfStatus::InvalidArgument, diagnostic)
            })?;
            if source.len > request.max_input_bytes() {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "document source exceeds its native input byte limit",
                ));
            }
            if frame.is_some_and(|frame| frame.len > request.max_input_bytes()) {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "document frame exceeds its native input byte limit",
                ));
            }
            let source = std::slice::from_raw_parts(source.data, source.len);
            let text = match frame {
                Some(frame) => document::frame_decoded(
                    source,
                    std::slice::from_raw_parts(frame.data, frame.len),
                    &request,
                ),
                None => document::execute_decoded(source, &request),
            }
            .map_err(|diagnostic| {
                let status = if diagnostic.code == operation.output_code() {
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
