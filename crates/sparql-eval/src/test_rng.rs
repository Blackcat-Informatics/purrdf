// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of this crate's deterministic SplitMix64 test/bench
//! generators, following the same `#[doc(hidden)]`-module pattern other
//! crates in this workspace use to share code between a library and its
//! `tests/` and `benches/` targets: those are separate compilation units
//! that can only reach shared code through the library.
//!
//! `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. No
//! shipping code path calls it.
//!
//! Two call patterns live here side by side because they are two DIFFERENT
//! streams from the same seed, and collapsing them into one would change
//! what every caller draws:
//!
//! * [`splitmix64_next`] — `state` is a plain incrementing counter; each
//!   draw advances it by the golden-ratio increment and re-mixes it from
//!   scratch. Used by [`stream`] and by the knn metric differential tests.
//! * [`splitmix64_step`] — self-composed: `state` itself becomes the fully
//!   mixed value, so the next draw mixes the previous draw's output. Used by
//!   the `knn_relation` bench fixture.
//!
//! Both share the same [`mix_rounds`] finalizer; they differ only in what
//! they feed it.

/// The SplitMix64 finalizer, without the golden-ratio increment: two
/// xor-shift-multiply rounds and a final xor-shift.
const fn mix_rounds(z: u64) -> u64 {
    let mut z = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A SplitMix64 step over a plain counter: `state` is advanced by the
/// golden-ratio increment, then the advanced value is mixed and returned.
/// The next call advances the SAME raw counter again, not the mixed output.
#[doc(hidden)]
pub const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mix_rounds(*state)
}

/// A self-composed SplitMix64 step: the golden-ratio increment is added to
/// `state` and the result mixed, and the caller is expected to feed the
/// returned value back in as the next `state` — so the counter itself is
/// the fully mixed value, not a raw increment.
#[doc(hidden)]
#[must_use]
pub const fn splitmix64_step(state: u64) -> u64 {
    mix_rounds(state.wrapping_add(0x9E37_79B9_7F4A_7C15))
}

/// One value in `[-1, 1)`, drawn from the [`splitmix64_next`] counter stream.
#[doc(hidden)]
#[must_use]
pub fn unit_f64_next(state: &mut u64) -> f64 {
    let z = splitmix64_next(state);
    ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
}

/// `len` values in `[-1, 1)` from the fixed [`splitmix64_next`] counter
/// stream at `seed`.
#[doc(hidden)]
#[must_use]
pub fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len).map(|_| unit_f64_next(&mut state)).collect()
}
