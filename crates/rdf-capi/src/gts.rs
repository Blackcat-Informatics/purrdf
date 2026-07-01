// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_from_gts` / `purrdf_to_gts`: lossless GTS container read/write.
//!
//! libpurrdf statically reuses the permissive `purrdf-gts` Rust crate (via the
//! oxigraph-free `gts_write` / `import_gts_events` core), so a language shim
//! links `libpurrdf` ALONE and still reads/writes `.gts` containers — the spec's
//! "one shared library, not two" clause (PurRDF-PLAN P8).
//!
//! GTS is a lossless container: both the plain-graph data AND the full RDF-1.2
//! statement layer (quoted triples + reifier bindings) survive the round-trip.
//! The earlier `gts-missing-reifier-binding` gap was closed by the native
//! text-codec work (#909); see [`purrdf_from_gts`].

use std::os::raw::c_char;

use purrdf_core::RdfLookaside;
use purrdf_rs::gts::read_graph;
use purrdf_rs::gts_write::to_gts;
use purrdf_rs::import_gts_graph;

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;

/// Read a GTS container into a fresh frozen dataset. `*out_dataset` is a
/// caller-owned handle (free with `purrdf_dataset_free`).
///
/// **Lossless**, including the RDF-1.2 statement layer: a reifier-bound quoted
/// triple written by `to_gts` is read back intact via the canonical fold-back
/// (`read_graph` → `import_gts_graph`). The earlier `gts-missing-reifier-binding`
/// gap was closed by the native text-codec work (#909).
///
/// # Safety
/// `bytes` must be valid for `len` bytes; the out-params must be writable.
#[no_mangle]
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
            // The canonical fold-back: read the GTS container into a graph, then
            // import the graph into the IR. `import_gts_graph` resolves `to_gts`'s
            // reifier bindings (including forward references), so the RDF-1.2
            // statement layer survives the round-trip.
            let graph = read_graph(slice, true).map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic)
            })?;
            let bundle = import_gts_graph(graph).map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic)
            })?;
            *out_dataset = PurrdfDataset::into_raw(bundle.dataset);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Write a frozen dataset to canonical GTS container bytes under `profile`
/// (e.g. `"dist"`). The output goes to `*out_buffer` (free with
/// `purrdf_buffer_free`).
///
/// # Safety
/// `dataset` must be a live handle; `profile` must be a NUL-terminated C string;
/// the out-params must be writable.
#[no_mangle]
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
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}
