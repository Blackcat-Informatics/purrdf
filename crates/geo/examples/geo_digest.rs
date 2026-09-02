// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Print `purrdf-geo`'s cross-target determinism digest, natively.
//!
//! The native half of `scripts/check-geo-determinism.sh`, which compares this
//! number against the one the same function produces under
//! `wasm32-unknown-unknown`. See `purrdf_geo::determinism` for what the digest
//! covers and why it is folded over serialized bytes.
//!
//! Deliberately an example rather than a test, so the harness can read one clean
//! line of stdout without parsing a test runner's output format.

fn main() {
    println!("digest={:016x}", purrdf_geo::determinism::digest());
    println!("corpus_len={}", purrdf_geo::determinism::corpus_len());
}
