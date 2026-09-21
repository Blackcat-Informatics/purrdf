// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `wasm32-unknown-unknown` half of `purrdf-hnsw`'s cross-target determinism
//! check.
//!
//! `purrdf-hnsw` claims that a build is a pure function of its input, so a native
//! answer and a wasm answer are byte-identical. `crates/hnsw/tests/determinism.rs`
//! pins the native value of [`purrdf_hnsw::determinism::digest`] against a golden
//! constant. This crate exports the same function to WebAssembly so that
//! `scripts/check-hnsw-determinism.sh` can run it under Node and assert the SAME
//! golden — which turns the claim from an argument into an observation.
//!
//! It is deliberately as thin as an export can be. Anything computed here rather
//! than in `purrdf-hnsw` would be code the native side never runs, and the whole
//! value of the harness is that both sides run the same code.
//!
//! Excluded from the workspace (root `Cargo.toml`), `publish = false`, and
//! depended on by nothing.

/// `purrdf-hnsw`'s determinism digest, exported to a WebAssembly host.
///
/// # Safety
///
/// `#[unsafe(no_mangle)]` is required for a WebAssembly host to find the export
/// by name; the attribute is unsafe only because it can collide with another
/// symbol, and this `cdylib` has exactly one export.
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_hnsw_determinism_digest() -> u64 {
    purrdf_hnsw::determinism::digest()
}

/// The number of corpus members the digest folds, so the harness can prove the
/// digest is not vacuous on the wasm side too.
///
/// # Safety
///
/// As above.
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_hnsw_determinism_corpus_len() -> u32 {
    u32::try_from(purrdf_hnsw::determinism::corpus_len()).expect("the corpus length fits u32")
}
