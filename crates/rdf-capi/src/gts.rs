// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_from_gts` / `purrdf_to_gts`: GTS container read/write.
//!
//! libpurrdf statically reuses the permissive `purrdf-gts` Rust crate (via the
//! `gts_write` / `import_gts_events` core), so a language shim
//! links `libpurrdf` ALONE and still reads/writes `.gts` containers — the spec's
//! "one shared library, not two" clause.
//!
//! Both the plain-graph data AND the full RDF-1.2 statement layer (quoted
//! triples + reifier bindings) survive the round-trip. A language tag that is not
//! lower case is read folded to lower case, and this entry point returns no loss
//! report.
//! The earlier `gts-missing-reifier-binding` gap was closed by the native
//! text-codec work; see [`purrdf_from_gts`].

use std::os::raw::c_char;

use purrdf_core::RdfLookaside;
use purrdf_rs::gts::{read_graph, read_graph_with_limits, reader::GtsReadLimits};
use purrdf_rs::gts_write::to_gts;
use purrdf_rs::import_gts_graph;

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::handles::into_handle;
use crate::status::PurrdfStatus;

/// C-ABI whole-read limits for hostile GTS input.
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

/// Read a GTS container into a fresh frozen dataset. `*out_dataset` is a
/// caller-owned handle (free with `purrdf_dataset_free`).
///
/// Keeps the RDF-1.2 statement layer: a reifier-bound quoted
/// triple written by `to_gts` is read back intact via the canonical fold-back
/// (`read_graph` → `import_gts_graph`). The earlier `gts-missing-reifier-binding`
/// gap was closed by the native text-codec work. A language tag that is not lower
/// case is read folded to lower case, and the envelope is not returned.
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

/// Read a GTS container under caller-selected whole-read limits.
///
/// `limits == NULL` selects finite hostile-input defaults. The legacy
/// [`purrdf_from_gts`] entry point retains its unbounded v4 contract.
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

/// Write a frozen dataset to canonical GTS container bytes under `profile`
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
