// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_serialize`: a frozen dataset → bytes in a requested media type, with
//! the RDF-1.2 statement-layer loss count surfaced (MAXIMAL INFORMATION FLOW).

use std::os::raw::c_char;
use std::sync::Arc;

use purrdf_rs::{
    CompiledJsonLdContext, JsonLdSerializeMode, JsonLdSerializeOptions, classify,
    serialize_dataset_to_format, serialize_dataset_to_format_with_jsonld_options,
};

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// An immutable compiled JSON-LD context. Release with
/// `purrdf_jsonld_context_free`; the handle is safe for concurrent reads.
#[derive(Debug)]
pub struct PurrdfJsonLdContext(Arc<CompiledJsonLdContext>);

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PurrdfJsonLdContext>();
};

/// Compile a reusable context from a versioned JSON-LD options document.
///
/// # Safety
/// `options_json` must point to `options_len` readable bytes; the output pointers
/// must be writable. On success, free `*out_context` with
/// `purrdf_jsonld_context_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_jsonld_context_compile(
    options_json: *const u8,
    options_len: usize,
    out_context: *mut *mut PurrdfJsonLdContext,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if options_json.is_null() || out_context.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_jsonld_context_compile",
                ));
            }
            let json = std::slice::from_raw_parts(options_json, options_len);
            let options = decode_options(json)?;
            let JsonLdSerializeMode::Context(context) = options.mode() else {
                return Err(PurrdfError::new(
                    PurrdfStatus::SerializeError,
                    "compiled JSON-LD context requires options mode `context`",
                ));
            };
            *out_context = Box::into_raw(Box::new(PurrdfJsonLdContext(Arc::clone(context))));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Release a compiled JSON-LD context handle. No-op on null.
///
/// # Safety
/// `context` must be null or a live handle not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_jsonld_context_free(context: *mut PurrdfJsonLdContext) {
    unsafe {
        ffi_guard!((), {
            if !context.is_null() {
                drop(Box::from_raw(context));
            }
        });
    }
}

/// Serialize JSON-LD or YAML-LD with exactly one versioned options document or
/// reusable compiled context. `yaml_schema_url` may be null and overrides the
/// options document for YAML-LD when supplied.
///
/// # Safety
/// `dataset` and `media_type` must be live/non-null. If `options_json` is not
/// null it points to `options_len` readable bytes and `context` must be null; if
/// `options_json` is null, `options_len` must be zero and `context` must be live.
/// Output pointers must be writable.
#[unsafe(no_mangle)]
#[allow(
    clippy::too_many_arguments,
    reason = "the C ABI names each pointer, length, configuration, and output explicitly"
)]
pub unsafe extern "C" fn purrdf_serialize_jsonld_configured(
    dataset: *const PurrdfDataset,
    media_type: *const c_char,
    options_json: *const u8,
    options_len: usize,
    context: *const PurrdfJsonLdContext,
    yaml_schema_url: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || media_type.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null required pointer argument to purrdf_serialize_jsonld_configured",
                ));
            }
            let mut options = match (options_json.is_null(), context.is_null()) {
                (false, true) => {
                    decode_options(std::slice::from_raw_parts(options_json, options_len))?
                }
                (true, false) if options_len == 0 => {
                    JsonLdSerializeOptions::compiled(Arc::clone(&(*context).0))
                }
                _ => {
                    return Err(PurrdfError::new(
                        PurrdfStatus::SerializeError,
                        "provide exactly one of options_json or context",
                    ));
                }
            };
            if let Some(url) = opt_cstr_to_str(yaml_schema_url)? {
                options = options.with_yaml_schema_url(url).map_err(|diagnostic| {
                    PurrdfError::from_diagnostic(PurrdfStatus::SerializeError, &diagnostic)
                })?;
            }
            let media = cstr_to_str(media_type)?;
            let format = classify(media).map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::UnsupportedFormat, &diagnostic)
            })?;
            let outcome = serialize_dataset_to_format_with_jsonld_options(
                PurrdfDataset::dataset(dataset),
                format,
                None,
                &options,
            )
            .map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::SerializeError, &diagnostic)
            })?;
            *out_buffer = PurrdfBuffer::into_raw(outcome.bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

fn decode_options(json: &[u8]) -> Result<JsonLdSerializeOptions, PurrdfError> {
    JsonLdSerializeOptions::from_json(json).map_err(|diagnostic| {
        PurrdfError::from_diagnostic(PurrdfStatus::SerializeError, &diagnostic)
    })
}

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
#[unsafe(no_mangle)]
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
