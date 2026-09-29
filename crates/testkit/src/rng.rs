// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one deterministic pseudo-random stream for tests and
//! benches: SplitMix64 and a xoshiro256** generator seeded through it.
//!
//! Every crate that needs a fixed-seed stream for generated test inputs draws
//! from here, and the SplitMix64 function itself is [`purrdf_hash::mix`], so
//! there is exactly one place that defines what "the stream" means.
//! [`prop`](crate::prop) itself is built on this module.
//!
//! Nothing here is cryptographically secure, and nothing here ships in a
//! release artifact's data path: it exists only behind `[dev-dependencies]`.

/// The SplitMix64 generator and its self-composed step, from
/// [`purrdf_hash::mix`], where the function and its pinned outputs live.
pub use purrdf_hash::mix::{splitmix64_next, splitmix64_step};

/// One value in `[-1, 1)` from the [`splitmix64_next`] counter stream: the
/// top 53 bits of the next output as a fraction of 2^53, doubled and shifted
/// down by one. Every step is exact in binary64, so the value is the same on
/// every target.
#[must_use]
pub fn signed_unit_next(state: &mut u64) -> f64 {
    let z = splitmix64_next(state);
    ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
}

/// `len` values in `[-1, 1)` from the [`splitmix64_next`] counter stream
/// started at `seed`, drawn with [`signed_unit_next`].
#[must_use]
pub fn signed_unit_stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len).map(|_| signed_unit_next(&mut state)).collect()
}

/// SplitMix64 (Steele, Lea and Flood): a seed expander and a strong 64-bit
/// finaliser.
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator seeded at `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64-bit output.
    #[must_use]
    pub const fn next_u64(&mut self) -> u64 {
        splitmix64_next(&mut self.0)
    }
}

/// xoshiro256** (Blackman and Vigna), seeded through [`SplitMix64`] as its
/// authors recommend so that no seed yields the all-zero state.
#[derive(Debug, Clone)]
pub struct Xoshiro256 {
    s: [u64; 4],
}

impl Xoshiro256 {
    /// A generator seeded at `seed`, expanded into the four-word state
    /// through [`SplitMix64`].
    #[must_use]
    pub const fn from_seed(seed: u64) -> Self {
        let mut expander = SplitMix64::new(seed);
        Self {
            s: [
                expander.next_u64(),
                expander.next_u64(),
                expander.next_u64(),
                expander.next_u64(),
            ],
        }
    }

    /// A generator over an explicit state, for testing against the reference
    /// implementation's published vectors.
    #[cfg(test)]
    pub(crate) const fn from_state(s: [u64; 4]) -> Self {
        Self { s }
    }

    /// The next 64-bit output.
    #[must_use]
    pub const fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// A uniform value in `0..=max`, without modulo bias: draws below
    /// `2^64 mod (max + 1)` are redrawn.
    #[must_use]
    pub const fn up_to(&mut self, max: u64) -> u64 {
        if max == u64::MAX {
            return self.next_u64();
        }
        let range = max + 1;
        let threshold = range.wrapping_neg() % range;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % range;
            }
        }
    }

    /// A uniform `f64` in `[0, 1)` from the top 53 bits.
    #[must_use]
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::{SplitMix64, Xoshiro256, signed_unit_stream};

    #[test]
    fn splitmix64_matches_the_reference_outputs() {
        // The first outputs of the reference implementation from seed 0.
        let mut generator = SplitMix64::new(0);
        assert_eq!(generator.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(generator.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(generator.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn xoshiro256_starstar_matches_the_reference_output() {
        // The reference implementation's first output from state [1, 2, 3, 4]
        // is rotl(2 * 5, 7) * 9.
        let mut generator = Xoshiro256::from_state([1, 2, 3, 4]);
        assert_eq!(generator.next_u64(), 11_520);
        assert_eq!(generator.next_u64(), 0);
        assert_eq!(generator.next_u64(), 1_509_978_240);
    }

    #[test]
    fn up_to_stays_in_bounds_and_reaches_both_ends() {
        let mut generator = Xoshiro256::from_seed(7);
        let mut seen = [false; 5];
        for _ in 0..10_000 {
            let value = generator.up_to(4);
            seen[value as usize] = true;
        }
        assert_eq!(seen, [true; 5]);
        assert_eq!(generator.up_to(0), 0);
    }

    /// `purrdf-sparql-eval`'s kNN tests draw their fixture
    /// vectors from [`signed_unit_stream`]. This test pins the first eight
    /// values from seed 0 and from seed `0xC0FFEE`, as binary64 bit patterns,
    /// so that a change to the stream cannot silently move what those
    /// fixtures contain. Each value is exact: 53 bits over 2^53, doubled,
    /// minus one.
    #[test]
    fn signed_unit_stream_matches_the_pinned_reference_outputs() {
        const SEED_ZERO: [u64; 8] = [
            0x3FE8_882A_0E5E_C772,
            0xBFC1_8761_955E_46A0,
            0xBFEE_4EE8_B9DF_FDB0,
            0x3FEE_22EE_2A1C_9320,
            0xBFE9_319D_A56B_95E4,
            0xBFD6_1A30_79C5_C0B0,
            0xBFE4_DF59_5078_2EB4,
            0x3FE1_6104_CEB2_45AA,
        ];
        const SEED_C0FFEE: [u64; 8] = [
            0x3FE2_A085_BEA4_1634,
            0x3FEB_3916_EAF3_A1C0,
            0x3FAE_FA4E_9285_A9C0,
            0xBFD2_C71F_BB54_2D7C,
            0x3FE0_D175_B85A_2B4A,
            0x3FE9_1F7C_CA8E_9890,
            0xBFED_CD52_36CD_7C00,
            0x3FE7_E914_A508_AA64,
        ];
        for (seed, expected) in [(0, SEED_ZERO), (0x00C0_FFEE, SEED_C0FFEE)] {
            let observed: Vec<u64> = signed_unit_stream(8, seed)
                .into_iter()
                .map(f64::to_bits)
                .collect();
            assert_eq!(observed, expected, "seed {seed:#x}");
        }
        // The first value from seed 0 is the first SplitMix64 output
        // (pinned above), scaled: 0xE220_A839_7B1D_CDAF >> 11 over 2^53,
        // doubled, minus one.
        assert_eq!(
            signed_unit_stream(1, 0)[0].to_bits(),
            0.766_621_616_427_285_2_f64.to_bits(),
            "the first value from seed 0"
        );
    }
}
