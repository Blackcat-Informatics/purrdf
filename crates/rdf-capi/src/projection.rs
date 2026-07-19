// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic graph, tabular, and research-object carrier entry points.

use std::os::raw::c_char;

use purrdf_rs::{
    LiftProfile, ProjectionConfig, ProjectionError, ProjectionProfile, RoCrateAssets, lift_archive,
    project_archive, project_archive_with_assets,
};

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;

fn projection_error(error: &ProjectionError) -> PurrdfError {
    PurrdfError::new(PurrdfStatus::InvalidArgument, error.to_string())
}

/// Project a frozen RDF dataset into a canonical deterministic USTAR carrier.
///
/// `profile` is a NUL-terminated projection profile name. `config_json` is the
/// mandatory profile-tagged configuration and must be valid for `config_len`
/// bytes. On success, `*out_archive` and `*out_loss_ledger_json` are independent
/// caller-owned buffers released with `purrdf_buffer_free`. The loss ledger is
/// always computed and uses PurRDF's versioned canonical JSON schema.
/// Research-object profiles are `croissant-1.1`, `ro-crate-1.3`,
/// `datacite-4.6`, `dcat-3`, and `frictionless-data-package-1`.
///
/// # Safety
/// `dataset` must be a live handle; `profile` must be a valid C string;
/// `config_json` must be readable for `config_len` bytes; the two output pointers
/// must be non-null, distinct, and writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_project(
    dataset: *const PurrdfDataset,
    profile: *const c_char,
    config_json: *const u8,
    config_len: usize,
    out_archive: *mut *mut PurrdfBuffer,
    out_loss_ledger_json: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null()
                || profile.is_null()
                || config_json.is_null()
                || out_archive.is_null()
                || out_loss_ledger_json.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_project",
                ));
            }
            if out_archive == out_loss_ledger_json {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "purrdf_project output pointers must be distinct",
                ));
            }
            *out_archive = std::ptr::null_mut();
            *out_loss_ledger_json = std::ptr::null_mut();

            let profile = cstr_to_str(profile)?
                .parse::<ProjectionProfile>()
                .map_err(|error| projection_error(&error))?;
            let config_bytes = std::slice::from_raw_parts(config_json, config_len);
            let config = ProjectionConfig::from_json(config_bytes)
                .map_err(|error| projection_error(&error))?;
            let outcome = project_archive(PurrdfDataset::dataset(dataset), profile, &config)
                .map_err(|error| projection_error(&error))?;

            let archive = Box::new(PurrdfBuffer(outcome.archive));
            let ledger = Box::new(PurrdfBuffer(outcome.loss_ledger.render_json().into_bytes()));
            *out_archive = Box::into_raw(archive);
            *out_loss_ledger_json = Box::into_raw(ledger);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Project a frozen RDF dataset and payload-only USTAR into an attached RO-Crate.
///
/// `assets_archive` is a canonical PurRDF USTAR containing payload members only;
/// metadata and preview names are reserved to the engine. The profile and tagged
/// configuration must select attached `ro-crate-1.3` packaging. Output ownership
/// matches [`purrdf_project`].
///
/// # Safety
/// `dataset` must be a live handle; `profile` must be a valid C string;
/// `config_json` and `assets_archive` must be readable for their respective lengths;
/// the two output pointers must be non-null, distinct, and writable. `out_error` may
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_project_with_assets(
    dataset: *const PurrdfDataset,
    profile: *const c_char,
    config_json: *const u8,
    config_len: usize,
    assets_archive: *const u8,
    assets_len: usize,
    out_archive: *mut *mut PurrdfBuffer,
    out_loss_ledger_json: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null()
                || profile.is_null()
                || config_json.is_null()
                || assets_archive.is_null()
                || out_archive.is_null()
                || out_loss_ledger_json.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_project_with_assets",
                ));
            }
            if out_archive == out_loss_ledger_json {
                return Err(PurrdfError::new(
                    PurrdfStatus::InvalidArgument,
                    "purrdf_project_with_assets output pointers must be distinct",
                ));
            }
            *out_archive = std::ptr::null_mut();
            *out_loss_ledger_json = std::ptr::null_mut();

            let profile = cstr_to_str(profile)?
                .parse::<ProjectionProfile>()
                .map_err(|error| projection_error(&error))?;
            let config_bytes = std::slice::from_raw_parts(config_json, config_len);
            let config = ProjectionConfig::from_json(config_bytes)
                .map_err(|error| projection_error(&error))?;
            let assets_bytes = std::slice::from_raw_parts(assets_archive, assets_len);
            let assets = RoCrateAssets::from_ustar(assets_bytes, config.limits())
                .map_err(|error| projection_error(&error))?;
            let outcome = project_archive_with_assets(
                PurrdfDataset::dataset(dataset),
                profile,
                &config,
                &assets,
            )
            .map_err(|error| projection_error(&error))?;

            let archive = Box::new(PurrdfBuffer(outcome.archive));
            let ledger = Box::new(PurrdfBuffer(outcome.loss_ledger.render_json().into_bytes()));
            *out_archive = Box::into_raw(archive);
            *out_loss_ledger_json = Box::into_raw(ledger);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Lift a canonical USTAR carrier into a fresh frozen RDF dataset.
///
/// Only the closed bidirectional profiles are accepted; curated CSVW terms, OBO
/// Graphs, and SKOS fail as invalid arguments instead of pretending to round-trip.
/// All five research-object profiles are bidirectional. On success,
/// `*out_dataset` is released with `purrdf_dataset_free` and
/// `*out_loss_ledger_json` with `purrdf_buffer_free`.
///
/// # Safety
/// `archive` and `config_json` must be readable for their respective lengths;
/// `profile` must be a valid C string; output pointers must be non-null and
/// writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_lift(
    archive: *const u8,
    archive_len: usize,
    profile: *const c_char,
    config_json: *const u8,
    config_len: usize,
    out_dataset: *mut *mut PurrdfDataset,
    out_loss_ledger_json: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if archive.is_null()
                || profile.is_null()
                || config_json.is_null()
                || out_dataset.is_null()
                || out_loss_ledger_json.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_lift",
                ));
            }
            *out_dataset = std::ptr::null_mut();
            *out_loss_ledger_json = std::ptr::null_mut();

            let profile = cstr_to_str(profile)?
                .parse::<LiftProfile>()
                .map_err(|error| projection_error(&error))?;
            let config_bytes = std::slice::from_raw_parts(config_json, config_len);
            let config = ProjectionConfig::from_json(config_bytes)
                .map_err(|error| projection_error(&error))?;
            let archive = std::slice::from_raw_parts(archive, archive_len);
            let outcome = lift_archive(archive, profile, &config)
                .map_err(|error| projection_error(&error))?;

            let dataset = Box::new(PurrdfDataset(outcome.dataset));
            let ledger = Box::new(PurrdfBuffer(outcome.loss_ledger.render_json().into_bytes()));
            *out_dataset = Box::into_raw(dataset);
            *out_loss_ledger_json = Box::into_raw(ledger);
            Ok(PurrdfStatus::Ok)
        })
    }
}
