// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A deterministic SplitMix64 fixture generator shared by this crate's
//! facade tests, so two of them are not two definitions of what "the
//! stream" means.
//!
//! This is pure, self-contained arithmetic with no dependency on any
//! `purrdf` internal: including it costs a facade test nothing of its
//! reason for existing, which is to exercise only the public umbrella
//! surface.

/// One step of the "linear counter" SplitMix64 stream: `state` is a plain
/// incrementing counter, re-mixed from scratch on every draw, mapped into
/// `[-1, 1)` and never exactly zero.
fn linear_step(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let value = ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
    if value == 0.0 { 0.125 } else { value }
}

/// `count` values from the linear-counter SplitMix64 stream seeded by `seed`.
pub(crate) fn linear_unit_values(seed: u64, count: usize) -> Vec<f64> {
    let mut state = seed;
    (0..count).map(|_| linear_step(&mut state)).collect()
}
