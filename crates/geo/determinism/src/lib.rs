// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `wasm32-unknown-unknown` half of `purrdf-geo`'s cross-target determinism
//! check.
//!
//! `purrdf-geo` claims that a native answer and a wasm answer are bit-identical.
//! `crates/geo/tests/determinism.rs` pins the native value of
//! [`purrdf_geo::determinism::digest`] against a golden constant. This crate
//! exports the same function to WebAssembly so that
//! `scripts/check-geo-determinism.sh` can run it under Node and assert the SAME
//! golden — which turns the claim from an argument into an observation.
//!
//! It is deliberately as thin as an export can be. Anything computed here rather
//! than in `purrdf-geo` would be code the native side never runs, and the whole
//! value of the harness is that both sides run the same code.
//!
//! Excluded from the workspace (root `Cargo.toml`), `publish = false`, and
//! depended on by nothing.

/// `purrdf-geo`'s determinism digest, exported to a WebAssembly host.
///
/// # Safety
///
/// `#[unsafe(no_mangle)]` is required for a WebAssembly host to find the export
/// by name; the attribute is unsafe only because it can collide with another
/// symbol, and this `cdylib` has exactly one export.
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_geo_determinism_digest() -> u64 {
    purrdf_geo::determinism::digest()
}

/// The number of corpus members the digest folds, so the harness can prove the
/// digest is not vacuous on the wasm side too.
///
/// # Safety
///
/// As above.
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_geo_determinism_corpus_len() -> u32 {
    purrdf_geo::determinism::corpus_len() as u32
}
