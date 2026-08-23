// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `pack` subcommands: standalone pack-container utilities.
//!
//! `pack verify` is the explicit surface for the full pack integrity check — every
//! section digest AND the RDFC-1.0 canonical-identity digest — that the ordinary
//! read/reason paths already run on every open. It is additive: nothing enters the
//! pipeline unverified regardless, so this verb never substitutes for that. It
//! acquires the pack through the same memory-safe immutable-input authority, verifies
//! it, and prints the verified canonical digest.

use purrdf_core::verify_pack;

use crate::error::CliError;
use crate::source;

/// Verify the pack at `input` (or stdin when `input` is `-`) and print its verified
/// RDFC-1.0 canonical digest to stdout.
///
/// # Errors
///
/// Returns a [`CliError`] if the input cannot be read, or if pack verification fails
/// — a bad magic/format, a section-digest mismatch, or an RDFC canonical-digest
/// mismatch. The bytes are acquired through the immutable-input authority, so a
/// hostile concurrent pathname writer cannot fault the check.
pub(crate) fn verify(input: &str) -> Result<(), CliError> {
    let owner = source::acquire_pack_input(input)?;
    let digest = verify_pack(owner.as_bytes())?;
    println!("{digest}");
    Ok(())
}
