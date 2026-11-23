// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS container read and write through the shared Rust RDF engine.

use std::os::raw::c_char;

use purrdf_core::RdfLookaside;
use purrdf_rs::gts::{GtsDatasetReadError, read_gts_dataset};
use purrdf_rs::gts_write::to_gts;

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::handles::into_handle;
use crate::status::PurrdfStatus;

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
            let dataset = read_gts_dataset(slice).map_err(|error| match error {
                GtsDatasetReadError::Legacy(diagnostic) => {
                    PurrdfError::from_diagnostic(PurrdfStatus::GtsError, &diagnostic)
                }
                GtsDatasetReadError::GenerationTwo(diagnostic) => {
                    PurrdfError::from_owned_diagnostic(PurrdfStatus::GtsError, diagnostic)
                }
                error => PurrdfError::new(PurrdfStatus::GtsError, error.to_string()),
            })?;
            *out_dataset = into_handle(PurrdfDataset::new(dataset));
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
