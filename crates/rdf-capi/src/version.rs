// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ABI version reporting and dataset capability introspection.

use purrdf_core::RdfStoreCapabilities;

use crate::handles::PurrdfDataset;
use crate::status::{PurrdfAbiVersion, PurrdfCapabilities, PurrdfStatus};

/// ABI major version. `0` signals the surface is still **beta** — the freeze
/// discipline (append-only status enum, drift-gated header) is in place, but the
/// version stays pre-1.0 until a real C consumer + the rdflib shim exercise it.
///
/// # The bump rule this triple obeys
///
/// The project's pre-1.0 policy — `docs/book/src/project/releases.md`, the
/// "Pre-1.0 semver policy" section — is: while the version is `0.x`, a **minor**
/// bump (`0.x` → `0.(x+1)`) may include breaking changes, and a **patch** bump
/// (`0.x.y` → `0.x.(y+1)`) is bugfix-only and API-compatible. So while MAJOR is
/// `0`, **an incompatible C-ABI change rides the MINOR component**; MAJOR does
/// not move, because moving it would declare the 1.0 stability this surface has
/// explicitly not earned yet (the paragraph above).
///
/// A change is incompatible — and therefore MUST bump MINOR — when a host built
/// against the previous header would mis-execute against the new library:
/// removing an exported function, renaming one, retyping or reordering its
/// parameters, inserting a parameter anywhere but the end, changing its return
/// type, or renumbering a status discriminant. `tests/abi_signatures.rs` pins
/// the complete exported prototype list to this triple, so such a change cannot
/// reach a release without an author deliberately touching these constants.
pub const PURRDF_ABI_MAJOR: u32 = 0;
/// ABI minor version.
///
/// `0.6.0` → `0.7.0`: `purrdf_shacl_validate_to_sarif` and
/// `purrdf_shacl_entail_to_ntriples` each gained a `shapes_base_iri` parameter
/// **in the middle** of their existing parameter list, between `shapes_ttl` and
/// `data_nt`. For a host compiled against `0.6.0` that is a silent, unguardable
/// break: it passes `data_nt` into the `shapes_base_iri` slot and its
/// `PurrdfBuffer **` out-pointer into `data_nt`, which the boundary then reads
/// as a NUL-terminated C string. The parameter is deliberately positional rather
/// than appended — it belongs immediately beside the document it qualifies, and
/// one declared break beats a permanently confusing argument order — so the
/// version, not the signature, absorbs the incompatibility.
///
/// The same bump also carries a second mid-list insertion, deliberately bundled
/// rather than deferred: `purrdf_serialize_jsonld_configured` gained `base_iri`
/// after `media_type`, the slot it occupies on `purrdf_serialize`. `0.7.0` is
/// unreleased, so a consumer recompiles against this header exactly once for
/// both; splitting them across two versions would break the same consumer twice
/// for one reason. A THIRD incompatible change made after `0.7.0` ships must
/// bump again — the ledger here is what makes that judgement possible.
pub const PURRDF_ABI_MINOR: u32 = 7;
/// ABI patch version. Reset to `0` by the MINOR bump documented above.
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
