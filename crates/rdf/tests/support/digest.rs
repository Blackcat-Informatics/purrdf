// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SHA-256 digest the conformance and projection tests pin their inputs by.

use sha2::{Digest, Sha256};

/// The SHA-256 of `bytes` as lowercase hex. The digest renders itself — one
/// idiom for "SHA-256 as lowercase hex" across the suite rather than a per-file
/// accumulate loop.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{}", purrdf_hash::hex::Lower(&Sha256::digest(bytes)))
}
