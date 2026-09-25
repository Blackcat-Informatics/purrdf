// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of this crate's deterministic SplitMix64 test/bench
//! generator, exactly as [`plain_bench`](crate::plain_bench) shares code
//! between this library and its `tests/` and `benches/` targets: those are
//! separate compilation units that can only reach shared code through the
//! library.
//!
//! `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. No shipping
//! code path calls it.

/// A deterministic SplitMix64 step: the tests' and benches' only source of
/// variety. `state` is advanced by the golden-ratio increment and the
/// advanced value is returned through the SplitMix64 finalizer.
#[doc(hidden)]
#[must_use]
pub const fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
