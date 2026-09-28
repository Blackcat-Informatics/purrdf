// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The hasher's keys, derived at compile time from one named constant.
//!
//! [`PHI`] is `⌊2^64/φ⌋`, the golden-ratio fraction of the 64-bit circle. A
//! counter `c = 1, 2, …` is scaled by it and folded by it (rotated, so the
//! two operands differ), and each candidate, made odd, is kept when it is
//! dense and evenly spread ([`acceptable`]) and new. The first
//! [`KEY_COUNT`] kept candidates are [`KEYS`]. Nothing here is tunable after
//! the fact: the constant, the schedule and the acceptance rule are the
//! whole derivation, and a test re-derives and re-checks it.

use super::fold::fold;

/// `⌊2^64/φ⌋` where `φ = (1 + √5)/2`.
pub(crate) const PHI: u64 = 0x9E37_79B9_7F4A_7C15;

/// How many keys the schedule derives.
pub(crate) const KEY_COUNT: usize = 44;

/// Whether `key` is a good multiplier: odd, between 28 and 36 set bits, and
/// between 5 and 11 set bits in each 16-bit quarter, so every region of an
/// input reaches many partial products.
pub(crate) const fn acceptable(key: u64) -> bool {
    if key & 1 == 0 || key.count_ones() < 28 || key.count_ones() > 36 {
        return false;
    }
    let mut quarter = 0;
    while quarter < 4 {
        let ones = ((key >> (16 * quarter)) & 0xFFFF).count_ones();
        if ones < 5 || ones > 11 {
            return false;
        }
        quarter += 1;
    }
    true
}

/// The `counter`-th candidate, before acceptance.
pub(crate) const fn candidate(counter: u64) -> u64 {
    fold(counter.wrapping_mul(PHI), PHI.rotate_left(32)) | 1
}

const fn derive() -> [u64; KEY_COUNT] {
    let mut keys = [0u64; KEY_COUNT];
    let mut kept = 0;
    let mut counter = 0u64;
    while kept < KEY_COUNT {
        counter += 1;
        let key = candidate(counter);
        let mut fresh = acceptable(key);
        let mut earlier = 0;
        while fresh && earlier < kept {
            fresh = keys[earlier] != key;
            earlier += 1;
        }
        if fresh {
            keys[kept] = key;
            kept += 1;
        }
    }
    keys
}

/// Every key, in derivation order.
pub(crate) const KEYS: [u64; KEY_COUNT] = derive();

/// The accumulator's initial value.
pub(crate) const SEED: u64 = KEYS[0];
/// The multiplier of the accumulator-side word.
pub(crate) const K_A: u64 = KEYS[1];
/// The multiplier of the second word of a two-word absorb.
pub(crate) const K_B: u64 = KEYS[2];
/// The length term's offset.
pub(crate) const LEN_X: u64 = KEYS[3];
/// The length term's multiplier.
pub(crate) const LEN_M: u64 = KEYS[4];
/// The offset `finish` applies before its fold.
pub(crate) const FIN_X: u64 = KEYS[5];
/// The multiplier of `finish`.
pub(crate) const FIN_M: u64 = KEYS[6];
/// The multipliers of the four portable lanes.
pub(crate) const LANE_KEY: [u64; 4] = [KEYS[7], KEYS[8], KEYS[9], KEYS[10]];
/// The initial values of the four portable lanes.
pub(crate) const LANE_INIT: [u64; 4] = [KEYS[11], KEYS[12], KEYS[13], KEYS[14]];
/// The second word's domain offset in a portable two-word absorb.
pub(crate) const PAIR_X: u64 = KEYS[15];
/// The AES path's four lane round keys, as `(low, high)` words.
#[cfg_attr(
    not(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    )),
    allow(dead_code)
)]
pub(crate) const AES_LANE_KEY: [(u64, u64); 4] = [
    (KEYS[16], KEYS[17]),
    (KEYS[18], KEYS[19]),
    (KEYS[20], KEYS[21]),
    (KEYS[22], KEYS[23]),
];
/// The AES path's four lane initial values, as `(low, high)` words.
#[cfg_attr(
    not(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    )),
    allow(dead_code)
)]
pub(crate) const AES_LANE_INIT: [(u64, u64); 4] = [
    (KEYS[24], KEYS[25]),
    (KEYS[26], KEYS[27]),
    (KEYS[28], KEYS[29]),
    (KEYS[30], KEYS[31]),
];
/// The AES path's two merge round keys, as `(low, high)` words.
#[cfg_attr(
    not(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    )),
    allow(dead_code)
)]
pub(crate) const AES_MERGE_KEY: [(u64, u64); 2] = [(KEYS[32], KEYS[33]), (KEYS[34], KEYS[35])];
/// The AES path's four per-lane final round keys, as `(low, high)` words.
#[cfg_attr(
    not(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    )),
    allow(dead_code)
)]
pub(crate) const AES_FINAL_KEY: [(u64, u64); 4] = [
    (KEYS[36], KEYS[37]),
    (KEYS[38], KEYS[39]),
    (KEYS[40], KEYS[41]),
    (KEYS[42], KEYS[43]),
];

#[cfg(test)]
mod tests {
    use super::{KEY_COUNT, KEYS, PHI, acceptable, candidate};

    #[test]
    fn phi_is_the_floor_of_two_to_the_64_over_the_golden_ratio() {
        // 1/φ is the positive root of x² + x = 1, and x² + x increases on
        // x > 0, so g = ⌊2^64/φ⌋ is the one g with
        // g² + g·2^64 < 2^128 ≤ (g + 1)² + (g + 1)·2^64.
        let g = u128::from(PHI);
        let below = (g * g).checked_add(g << 64);
        assert!(below.is_some(), "g² + g·2^64 must stay below 2^128");
        let above = ((g + 1) * (g + 1)).checked_add((g + 1) << 64);
        assert!(above.is_none(), "(g+1)² + (g+1)·2^64 must reach 2^128");
    }

    #[test]
    fn keys_are_the_first_acceptable_fresh_candidates() {
        let mut expected = Vec::with_capacity(KEY_COUNT);
        let mut counter = 0;
        while expected.len() < KEY_COUNT {
            counter += 1;
            let key = candidate(counter);
            if acceptable(key) && !expected.contains(&key) {
                expected.push(key);
            }
        }
        assert_eq!(KEYS.as_slice(), expected.as_slice());
        for key in KEYS {
            assert_eq!(key & 1, 1);
            assert!((28..=36).contains(&key.count_ones()), "{key:#x}");
        }
    }
}
