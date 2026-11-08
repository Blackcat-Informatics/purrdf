// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Existing dataset/buffer controls around the shared native YAML source law.

use crate::{
    buffer::PurrdfBuffer,
    cstr_to_str,
    error::PurrdfError,
    handles::{PurrdfDataset, publish_handle},
    status::PurrdfStatus,
};
use purrdf_core::DatasetHandle;
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_validate::yaml_source::{self, Profile, YamlError};
use std::ffi::c_char;

fn native(error: yaml_source::Error, status: PurrdfStatus) -> PurrdfError {
    PurrdfError::from_owned_diagnostic(status, error.into_diagnostic())
}
fn storage(error: StorageError) -> PurrdfError {
    native(YamlError::Storage(error).into(), PurrdfStatus::StorageError)
}

unsafe fn profile(bytes: *const u8, length: usize) -> Result<Profile, PurrdfError> {
    if bytes.is_null() {
        return Err(PurrdfError::new(
            PurrdfStatus::NullPointer,
            "profile_json is required",
        ));
    }
    if length > yaml_source::MAX_PROFILE_BYTES || length > isize::MAX as usize {
        return Err(PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            "YAML source profile exceeds 16384 bytes",
        ));
    }
    yaml_source::profile(unsafe { std::slice::from_raw_parts(bytes, length) })
        .map_err(|error| native(error, PurrdfStatus::InvalidArgument))
}

/// Export original YAML bytes to a frozen dataset and its computed stream root.
///
/// `profile_json` is mandatory version-one profile JSON. Source and profile are
/// borrowed only during this synchronous call. On success both results are owned
/// independently: release the dataset with `purrdf_dataset_free`, and the root
/// with `purrdf_buffer_free`. The root buffer contains exact UTF-8 IRI bytes,
/// without a trailing NUL or newline. Both handles are prepared before either
/// output is published. Valid output slots are cleared before input processing;
/// failure publishes neither result. Host handle/buffer controls and the native
/// profile, parser, partition and projection accounts remain separate.
///
/// # Safety
/// Non-null source/profile pointers must be readable for their admitted lengths.
/// Null inputs are rejected even at zero length; oversized lengths are rejected
/// before constructing a slice. Non-null `source_iri` points to a live
/// NUL-terminated C string whose UTF-8 is checked. Non-null output/error slots
/// are writable and initially empty. Aliasing error slots are rejected by status alone,
/// without writing an error through the aliased pointer. No allocator abort can
/// be converted to a returned status by the existing FFI panic guard.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_yaml_source_export(
    source: *const u8,
    source_len: usize,
    source_iri: *const c_char,
    profile_json: *const u8,
    profile_len: usize,
    out_dataset: *mut *mut PurrdfDataset,
    out_root: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    if !out_error.is_null()
        && (out_dataset.cast::<*mut PurrdfError>() == out_error
            || out_root.cast::<*mut PurrdfError>() == out_error)
    {
        return PurrdfStatus::InvalidArgument as i32;
    }
    unsafe {
        ffi_try!(out_error, {
            if out_dataset.is_null() || out_root.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "YAML source export requires both output slots",
                ));
            }
            if out_dataset.cast::<*mut PurrdfBuffer>() == out_root {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "YAML source output slots must be distinct",
                ));
            }
            *out_dataset = std::ptr::null_mut();
            *out_root = std::ptr::null_mut();
            if source.is_null() || source_iri.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "YAML source and source_iri are required",
                ));
            }
            let profile = profile(profile_json, profile_len)?;
            if source_len > isize::MAX as usize {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "YAML source length exceeds the addressable slice range",
                ));
            }
            let maximum = profile.bounds().syntax.syntax.max_input_bytes;
            if source_len > maximum {
                return Err(native(
                    YamlError::Limit {
                        resource: "source bytes",
                        limit: maximum,
                    }
                    .into(),
                    PurrdfStatus::ParseError,
                ));
            }
            let source_iri = cstr_to_str(source_iri)?;
            let exported = yaml_source::export(
                source_iri,
                std::slice::from_raw_parts(source, source_len),
                &profile,
            )
            .map_err(|error| native(error, PurrdfStatus::ParseError))?;
            let mut resident = Resident;
            Memory::new(&mut resident)
                .add_bytes(DatasetHandle::allocation_layout::<Resident>().size())
                .map_err(storage)?;
            let dataset = DatasetHandle::try_from_admitted(exported.dataset, resident)
                .map_err(|_| storage(StorageError::AllocationFailed))?;
            let mut controls = Resident;
            let mut memory = Memory::new(&mut controls);
            let dataset = memory.boxed(PurrdfDataset::new(dataset)).map_err(storage)?;
            let root = memory
                .boxed(PurrdfBuffer::from(exported.root))
                .map_err(storage)?;
            // No allocation, native work or other fallible operation remains.
            *out_dataset = publish_handle(dataset);
            *out_root = publish_handle(root);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Reconstruct exact verified YAML source bytes from an existing frozen dataset.
///
/// Supply the exact expected root and matching explicit profile. The returned
/// source has no added newline or NUL and lives independently of the input
/// dataset/profile/root. Release it with `purrdf_buffer_free`. Caller RDF storage,
/// native decode/reparse/reprojection, and host buffer control are separate.
/// Valid output slots are cleared before work; no source is published on refusal.
///
/// # Safety
/// Non-null `dataset` is a live dataset handle; non-null `root` points to a live
/// NUL-terminated C string whose UTF-8 is checked. Non-null profile bytes are
/// readable for their admitted length. Non-null output/error slots are writable
/// and initially empty. Null required inputs and outputs are rejected; an
/// aliased error slot is rejected without writing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_yaml_source_reconstruct(
    dataset: *const PurrdfDataset,
    root: *const c_char,
    profile_json: *const u8,
    profile_len: usize,
    out_source: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    if !out_error.is_null() && out_source.cast::<*mut PurrdfError>() == out_error {
        return PurrdfStatus::InvalidArgument as i32;
    }
    unsafe {
        ffi_try!(out_error, {
            if out_source.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "YAML source reconstruction requires an output slot",
                ));
            }
            *out_source = std::ptr::null_mut();
            if dataset.is_null() || root.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "YAML source dataset and root are required",
                ));
            }
            let profile = profile(profile_json, profile_len)?;
            let root = cstr_to_str(root)?;
            let text = yaml_source::reconstruct(PurrdfDataset::dataset(dataset), root, &profile)
                .map_err(|error| native(error, PurrdfStatus::ParseError))?;
            let mut resident = Resident;
            let buffer = Memory::new(&mut resident)
                .boxed(PurrdfBuffer::from(text))
                .map_err(storage)?;
            *out_source = publish_handle(buffer);
            Ok(PurrdfStatus::Ok)
        })
    }
}
