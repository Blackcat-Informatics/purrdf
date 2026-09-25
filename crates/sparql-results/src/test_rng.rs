// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of this crate's deterministic SplitMix64 test
//! generator. Every `#[cfg(test)]` module in this crate that needs a
//! fixed-seed pseudo-random stream draws from here rather than hand-copying
//! the mixing step, so there is exactly one place that defines what "the
//! stream" means.
//!
//! `#[cfg(test)]`-only: nothing here ships, and nothing outside this crate's
//! own test modules can see it.

/// A deterministic SplitMix64 step: `state` is advanced by the golden-ratio
/// increment and the advanced value is returned through the SplitMix64
/// finalizer.
pub(crate) const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
