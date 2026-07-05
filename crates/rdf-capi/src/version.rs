// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ABI version reporting and dataset capability introspection.

use purrdf_core::RdfStoreCapabilities;

use crate::handles::PurrdfDataset;
use crate::status::{PurrdfAbiVersion, PurrdfCapabilities, PurrdfStatus};

/// ABI major version. `0` signals the surface is still **beta** — the freeze
/// discipline (append-only status enum, drift-gated header) is in place, but the
/// version stays pre-1.0 until a real C consumer + the rdflib shim exercise it.
pub const PURRDF_ABI_MAJOR: u32 = 0;
/// ABI minor version.
pub const PURRDF_ABI_MINOR: u32 = 1;
/// ABI patch version.
pub const PURRDF_ABI_PATCH: u32 = 0;

/// Write the libpurrdf ABI version to `*out`.
///
/// # Safety
/// `out` must be null-checked-writable for one `PurrdfAbiVersion`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_abi_version(out: *mut PurrdfAbiVersion) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = PurrdfAbiVersion {
                major: PURRDF_ABI_MAJOR,
                minor: PURRDF_ABI_MINOR,
                patch: PURRDF_ABI_PATCH,
            };
            PurrdfStatus::Ok as i32
        })
    }
}

/// Convert kernel capabilities to the `#[repr(C)]` flag struct.
fn capabilities_to_c(caps: RdfStoreCapabilities) -> PurrdfCapabilities {
    PurrdfCapabilities {
        named_graphs: u8::from(caps.named_graphs),
        quoted_triples: u8::from(caps.quoted_triples),
        reifiers: u8::from(caps.reifiers),
        annotations: u8::from(caps.annotations),
        source_locations: u8::from(caps.source_locations),
        loss_records: u8::from(caps.loss_records),
        lookaside: u8::from(caps.lookaside),
    }
}

/// Write the dataset's capability flags to `*out`.
///
/// # Safety
/// `dataset` must be a live handle; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_capabilities(
    dataset: *const PurrdfDataset,
    out: *mut PurrdfCapabilities,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if dataset.is_null() || out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = capabilities_to_c(PurrdfDataset::arc(dataset).capabilities());
            PurrdfStatus::Ok as i32
        })
    }
}
