// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **fixed level formula**: a pure integer function of the stable row index.
//!
//! An HNSW graph's shape is decided almost entirely by the level each inserted node is
//! assigned, and almost every published implementation draws that level from a random
//! number generator. That is the sole reason a graph built twice from the same input is
//! not guaranteed to be the same graph: the levels differ, the entry point differs, the
//! layers differ. This module removes the RNG from the equation.
//!
//! A node's level here is **`splitmix64(row_index)` funneled through the documented
//! geometric decay** — a pure function of the row index alone. The row index is the
//! PURREMB target-set row, which is numbered by ascending sorted `TargetId` (a digest of
//! canonical content), so rebuilding the same artifact reproduces the same levels and
//! therefore the same graph. There is no seed, no entropy, and no wall clock anywhere on
//! the path.
//!
//! # The formula
//!
//! ```text
//! bits_per_level(m) = floor(log2(m))            // largest k with 2^k <= m
//! level(i)          = min(lz(splitmix64(i)) / bits_per_level(m), level_cap(n, m))
//! ```
//!
//! where `lz` is the count of leading zero bits in the 64-bit hash. `P(level >= l)` is
//! `2^(-b*l)` — exactly `m^-l` for a power-of-two `m`, and the documented `2^b`
//! approximation otherwise. The arithmetic is integer-only: no `ln`, no `exp`, no float,
//! so the answer is a property of the index and not of the target's libm.
//!
//! `level_cap` is the smallest `L` with `m^L >= n`, derived from the input rather than
//! configured. It bounds the worst-case level so a pathological hash cannot mint an
//! unbounded layer; the index identity stays exactly `M`, `M0`, `ef_construction`,
//! `ef_search`.

/// The SplitMix64 finalizer: a bijective integer mix with no state and no entropy.
///
/// This is `seed` mapped directly through the finalizer (the reference implementation's
/// `next()` with its counter folded in), not SplitMix64's stateful generator — a level is
/// a pure function of the row index, so there is no generator state to advance.
///
/// The constants are the published SplitMix64 constants; the shifts and multiplies are
/// wrapping so the function is total over `u64`.
#[must_use]
pub const fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Bits of hash consumed per level: the largest `k` with `2^k <= m`, i.e.
/// `floor(log2(m))`.
///
/// For `m = 2^b` this makes `P(level >= l) = 2^(-b*l) = m^-l`, the geometric decay the
/// HNSW construction cost analysis assumes. For any other `m` it is the documented
/// power-of-two approximation `2^b <= m`.
///
/// # Panics
///
/// Debug builds assert `m >= 2`; [`crate::Params`] rejects anything smaller before this
/// is reached, and a caller cannot construct a valid parameter set without going through
/// that validation.
#[must_use]
pub const fn bits_per_level(m: usize) -> u32 {
    debug_assert!(m >= 2, "the level formula is defined only for m >= 2");
    // `m >= 2`, so `ilog2` is total (it is only defined for a non-zero input).
    (m as u64).ilog2()
}

/// The level cap: the smallest `L` with `m^L >= n`.
///
/// `m^0 = 1` always, so an index of one row caps at level zero. The multiplication is
/// widen-and-check in `u128`: `usize` is at most 64 bits, so every `usize` power is
/// representable in `u128` up to a very large exponent, and the only way to overflow is
/// an `m` large enough that `m^2` already exceeds `u128` — in which case `m^(cap+1)` is
/// certainly at least any `usize`, and the cap is one step past the current one.
///
/// # Panics
///
/// Debug builds assert `m >= 2`.
#[must_use]
pub fn level_cap(n: usize, m: usize) -> u32 {
    debug_assert!(m >= 2, "the level cap is defined only for m >= 2");
    let target = n as u128;
    let base = m as u128;
    let mut cap: u32 = 0;
    let mut power: u128 = 1;
    loop {
        if power >= target {
            return cap;
        }
        match power.checked_mul(base) {
            Some(next) => {
                power = next;
                cap = cap.saturating_add(1);
            }
            // `m^cap < n` and `m^(cap+1)` overflowed `u128`, which can only happen when it
            // is already at least every `usize`.
            None => return cap.saturating_add(1),
        }
    }
}

/// The level of the node at `row_index`.
///
/// A pure function of the row index and `m` (with `cap` derived from `n` and `m`; see
/// [`level_cap`]). `lz / bits` truncates downward, which is what turns the uniformly
/// distributed leading-zero count into the intended geometric decay.
#[must_use]
pub fn level_from_index(row_index: u64, m: usize, cap: u32) -> u32 {
    let bits = bits_per_level(m);
    (splitmix64(row_index).leading_zeros() / bits).min(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_matches_known_answers() {
        // Published SplitMix64 finalizer outputs; pinned by bits so a drift is a failure
        // rather than an approximation.
        assert_eq!(splitmix64(0), 0xe220_a839_7b1d_cdaf);
        assert_eq!(splitmix64(1), 0x910a_2dec_8902_5cc1);
        assert_eq!(splitmix64(2), 0x9758_35de_1c97_56ce);
        assert_eq!(splitmix64(3), 0x1d0b_14e4_db01_8fed);
    }

    #[test]
    fn splitmix64_is_a_bijection_onto_a_large_prefix() {
        // Distinct inputs must give distinct outputs over a prefix for the level mapping
        // to be meaningful at all.
        let mut seen = std::collections::HashSet::new();
        for i in 0..4096_u64 {
            assert!(seen.insert(splitmix64(i)), "collision at {i}");
        }
    }

    #[test]
    fn bits_per_level_is_floor_log2() {
        assert_eq!(bits_per_level(2), 1);
        assert_eq!(bits_per_level(3), 1);
        assert_eq!(bits_per_level(4), 2);
        assert_eq!(bits_per_level(7), 2);
        assert_eq!(bits_per_level(8), 3);
        assert_eq!(bits_per_level(15), 3);
        assert_eq!(bits_per_level(16), 4);
        assert_eq!(bits_per_level(17), 4);
        assert_eq!(bits_per_level(1024), 10);
    }

    #[test]
    fn level_cap_is_the_smallest_power_reaching_n() {
        // m = 16: 16^0 = 1, 16^1 = 16, 16^2 = 256.
        assert_eq!(level_cap(1, 16), 0);
        assert_eq!(level_cap(16, 16), 1);
        assert_eq!(level_cap(17, 16), 2);
        assert_eq!(level_cap(256, 16), 2);
        assert_eq!(level_cap(257, 16), 3);

        // m = 2: cap is the bit length of n - 1.
        assert_eq!(level_cap(1, 2), 0);
        assert_eq!(level_cap(2, 2), 1);
        assert_eq!(level_cap(3, 2), 2);
        assert_eq!(level_cap(4, 2), 2);
        assert_eq!(level_cap(5, 2), 3);
    }

    #[test]
    fn level_from_index_stays_within_the_cap_and_follows_the_formula() {
        let m = 16_usize;
        let bits = bits_per_level(m);
        for cap in [0_u32, 1, 2, 3] {
            for row in 0..10_000_u64 {
                let level = level_from_index(row, m, cap);
                let expected = (splitmix64(row).leading_zeros() / bits).min(cap);
                assert_eq!(level, expected, "row {row} cap {cap}");
                assert!(level <= cap, "the cap is a hard ceiling");
            }
        }
    }

    #[test]
    fn every_level_is_attained_within_the_cap() {
        // The cap is not so small that the level mapping collapses; over enough rows each
        // level up to the cap occurs. This is the property a too-aggressive cap would
        // silently destroy while all the boundary tests still passed.
        let m = 16_usize;
        let cap = 3_u32;
        let mut seen = [false; 4];
        for row in 0..100_000_u64 {
            seen[level_from_index(row, m, cap) as usize] = true;
        }
        assert!(seen.iter().all(|&hit| hit), "levels {seen:?}");
    }

    #[test]
    #[should_panic(expected = "m >= 2")]
    fn level_cap_rejects_m_below_two_in_debug() {
        let _ = level_cap(10, 1);
    }
}
