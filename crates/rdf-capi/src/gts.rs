// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS container read and write through the shared Rust RDF engine.

use std::os::raw::c_char;

use purrdf_core::RdfLookaside;
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_rs::gts::{
    GtsDatasetReadError, has_gts_v2_opener, read_gts_dataset as read_dataset_by_generation,
};
use purrdf_rs::gts::{read_graph, read_graph_with_limits, reader::GtsReadLimits};
use purrdf_rs::gts_write::{to_gts, to_gts_v2};
use purrdf_rs::import_gts_graph;

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::handles::{into_handle, publish_handle};
use crate::status::PurrdfStatus;

/// C-ABI whole-read limits for hostile GTS v1 input.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfGtsReadLimits {
    /// Complete encoded input bytes.
    pub encoded_input_bytes: u64,
    /// CBOR sequence item count.
    pub items: u64,
    /// Encoded bytes in one header or frame.
    pub encoded_frame_bytes: u64,
    /// Decoded bytes in one non-blob payload.
    pub decoded_frame_bytes: u64,
    /// Decoded bytes in one blob payload.
    pub decoded_blob_bytes: u64,
    /// Aggregate decoded payload bytes.
    pub total_decoded_bytes: u64,
    /// Accepted term rows.
    pub terms: u64,
    /// Accepted RDF and suppression rows.
    pub rows: u64,
    /// Retained reader metadata entries.
    pub metadata_entries: u64,
    /// Reader diagnostics.
    pub diagnostics: u64,
    /// Measurable reader work events.
    pub work: u64,
}

impl PurrdfGtsReadLimits {
    const DEFAULT: Self = Self {
        encoded_input_bytes: GtsReadLimits::DEFAULT.encoded_input_bytes as u64,
        items: GtsReadLimits::DEFAULT.items as u64,
        encoded_frame_bytes: GtsReadLimits::DEFAULT.encoded_frame_bytes as u64,
        decoded_frame_bytes: GtsReadLimits::DEFAULT.decoded_frame_bytes as u64,
        decoded_blob_bytes: GtsReadLimits::DEFAULT.decoded_blob_bytes as u64,
        total_decoded_bytes: GtsReadLimits::DEFAULT.total_decoded_bytes as u64,
        terms: GtsReadLimits::DEFAULT.terms as u64,
        rows: GtsReadLimits::DEFAULT.rows as u64,
        metadata_entries: GtsReadLimits::DEFAULT.metadata_entries as u64,
        diagnostics: GtsReadLimits::DEFAULT.diagnostics as u64,
        work: GtsReadLimits::DEFAULT.work as u64,
    };
}

/// Initialize `*out` with the finite limits for hostile input.
///
/// # Safety
/// `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_gts_read_limits_init(out: *mut PurrdfGtsReadLimits) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = PurrdfGtsReadLimits::DEFAULT;
            PurrdfStatus::Ok as i32
        })
    }
}

/// Read a GTS v1 or v2 container into a fresh frozen dataset.
/// Release `*out_dataset` with `purrdf_dataset_free`.
///
/// This returns the complete RDF 1.2 dataset with native RDF set semantics.
/// Unreadable v2 content or framing fails with diagnostic evidence. Transport
/// metadata and language tag spelling are outside the dataset projection.
///
/// # Safety
/// `bytes` must be valid for `len` bytes; the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_from_gts(
    bytes: *const u8,
    len: usize,
    out_dataset: *mut *mut PurrdfDataset,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if bytes.is_null() || out_dataset.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_from_gts",
                ));
            }
            let slice = std::slice::from_raw_parts(bytes, len);
            *out_dataset = into_handle(read_gts_dataset(slice, None)?);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Read a GTS v1 container under caller-selected whole-read limits.
///
/// `limits == NULL` selects finite hostile-input defaults. The legacy
/// [`purrdf_from_gts`] entry point also reads GTS v2 with its finite limits.
/// Zero is a valid inclusive ceiling in every field.
///
/// # Safety
/// `bytes` must be valid for `len` bytes; non-null `limits` and the out-params
/// must be readable/writable respectively.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_from_gts_with_limits(
    bytes: *const u8,
    len: usize,
    limits: *const PurrdfGtsReadLimits,
    out_dataset: *mut *mut PurrdfDataset,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if bytes.is_null() || out_dataset.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_from_gts_with_limits",
                ));
            }
            let slice = std::slice::from_raw_parts(bytes, len);
            let limits = limits
                .as_ref()
                .map(native_limits)
                .transpose()?
                .unwrap_or(GtsReadLimits::DEFAULT);
            *out_dataset = into_handle(read_gts_dataset(slice, Some(limits))?);
            Ok(PurrdfStatus::Ok)
        })
    }
}

fn read_gts_dataset(
    bytes: &[u8],
    limits: Option<GtsReadLimits>,
) -> Result<PurrdfDataset, PurrdfError> {
    if has_gts_v2_opener(bytes) {
        if limits.is_some() {
            return Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                "The requested limits apply to GTS v1. Use purrdf_from_gts for GTS v2 with its finite limits.",
            ));
        }
        let dataset = read_dataset_by_generation(bytes).map_err(|error| match error {
            GtsDatasetReadError::Legacy(diagnostic)
            | GtsDatasetReadError::GenerationTwo(diagnostic) => {
                PurrdfError::from_owned_diagnostic(PurrdfStatus::GtsError, diagnostic)
            }
            error => PurrdfError::new(PurrdfStatus::GtsError, error.to_string()),
        })?;
        return Ok(PurrdfDataset::new(dataset));
    }
    // The canonical fold-back: read the GTS container into a graph, then import
    // it into the IR. `import_gts_graph` resolves forward reifier bindings.
    let graph = match limits {
        Some(limits) => read_graph_with_limits(bytes, true, limits),
        None => read_graph(bytes, true),
    }
    .map_err(|diagnostic| PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic))?;
    let bundle = import_gts_graph(graph)
        .map_err(|diagnostic| PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic))?;
    Ok(PurrdfDataset::new(bundle.dataset))
}

fn native_limits(limits: &PurrdfGtsReadLimits) -> Result<GtsReadLimits, PurrdfError> {
    let convert = |name: &str, value: u64| {
        usize::try_from(value).map_err(|_| {
            PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                format!("GTS read limit {name} does not fit this host"),
            )
        })
    };
    Ok(GtsReadLimits {
        encoded_input_bytes: convert("encoded_input_bytes", limits.encoded_input_bytes)?,
        items: convert("items", limits.items)?,
        encoded_frame_bytes: convert("encoded_frame_bytes", limits.encoded_frame_bytes)?,
        decoded_frame_bytes: convert("decoded_frame_bytes", limits.decoded_frame_bytes)?,
        decoded_blob_bytes: convert("decoded_blob_bytes", limits.decoded_blob_bytes)?,
        total_decoded_bytes: convert("total_decoded_bytes", limits.total_decoded_bytes)?,
        terms: convert("terms", limits.terms)?,
        rows: convert("rows", limits.rows)?,
        metadata_entries: convert("metadata_entries", limits.metadata_entries)?,
        diagnostics: convert("diagnostics", limits.diagnostics)?,
        work: convert("work", limits.work)?,
    })
}

/// Write a frozen dataset to canonical GTS v1 bytes under `profile`
/// (e.g. `"dist"`). The output goes to `*out_buffer` (free with
/// `purrdf_buffer_free`).
///
/// # Safety
/// `dataset` must be a live handle; `profile` must be a NUL-terminated C string;
/// the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_to_gts(
    dataset: *const PurrdfDataset,
    profile: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || profile.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_to_gts",
                ));
            }
            let profile = cstr_to_str(profile)?;
            let bytes = to_gts(
                PurrdfDataset::dataset(dataset),
                &RdfLookaside::default(),
                profile,
            )
            .map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic)
            })?;
            *out_buffer = into_handle(PurrdfBuffer::from(bytes));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Write a frozen dataset as one committed GTS v2 segment in arrival order.
/// Release the returned `*out_buffer` with `purrdf_buffer_free`.
///
/// Preserves RDF 1.2 statements, scoped blank identities and empty named graphs.
/// Used composite literals require valid lexical forms and blank-free contents.
/// # Safety
/// `dataset` must be a live handle and the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_to_gts_v2(
    dataset: *const PurrdfDataset,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_to_gts_v2",
                ));
            }
            let bytes = to_gts_v2(PurrdfDataset::dataset(dataset)).map_err(|diagnostic| {
                PurrdfError::from_owned_diagnostic(PurrdfStatus::GtsError, diagnostic)
            })?;
            let buffer = Memory::new(&mut Resident)
                .boxed(PurrdfBuffer::from(bytes))
                .map_err(|cause| {
                    PurrdfError::from_owned_diagnostic(PurrdfStatus::StorageError, cause.into())
                })?;
            *out_buffer = publish_handle(buffer);
            Ok(PurrdfStatus::Ok)
        })
    }
}
