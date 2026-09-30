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
//! Two older recurrences live here too, because fixtures and frozen vectors
//! were generated from them and must keep regenerating bit for bit: Marsaglia's
//! xorshift64 ([`xorshift64_next`]) and the 64-bit linear congruential
//! generator with Knuth's MMIX multiplier ([`lcg64_next`]). New tests draw
//! from SplitMix64; these exist so an existing fixture's bytes do not move.
//!
//! Nothing here is cryptographically secure, and nothing here ships in a
//! release artifact's data path: it exists only behind `[dev-dependencies]`.

/// The SplitMix64 generator and its self-composed step, from
/// [`purrdf_hash::mix`], where the function and its pinned outputs live.
pub use purrdf_hash::mix::{splitmix64_next, splitmix64_step};

/// The `[-1, 1)` draws over both SplitMix64 streams, from
/// [`purrdf_hash::mix`], where the mapping and its pinned outputs live: the
/// HNSW determinism corpus is published code built on them.
pub use purrdf_hash::mix::{
    signed_unit, signed_unit_next, signed_unit_next_nonzero, signed_unit_nonzero, signed_unit_step,
    signed_unit_step_nonzero,
};

/// One step of Marsaglia's xorshift64 with the shift triple `(13, 7, 17)`:
/// `state` is advanced and the new state is the output.
///
/// Bit-exact with the recurrence the frozen BLAKE3 random vectors and several
/// fixed-seed corpora were generated from, so they regenerate unchanged. A zero
/// state stays zero; seed with a non-zero value.
#[must_use]
pub const fn xorshift64_next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// The multiplier of the 64-bit linear congruential generator: Knuth's MMIX
/// constant, which passes the spectral test for modulus 2^64.
pub const LCG64_MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// The increment MMIX pairs with [`LCG64_MULTIPLIER`].
pub const LCG64_MMIX_INCREMENT: u64 = 1_442_695_040_888_963_407;

/// One step of the 64-bit linear congruential generator modulo 2^64:
/// `state = state * LCG64_MULTIPLIER + increment`, returning the new state.
///
/// The fixtures built on it use two increments — [`LCG64_MMIX_INCREMENT`] and
/// `1` — and take different bits of the state (the high 31 above bit 33, the
/// high byte, the whole word), so the increment is the caller's and the
/// projection is too. The low bits of an LCG modulo a power of two are weak;
/// a caller drawing a small range takes the high bits.
#[must_use]
pub const fn lcg64_next(state: &mut u64, increment: u64) -> u64 {
    *state = state.wrapping_mul(LCG64_MULTIPLIER).wrapping_add(increment);
    *state
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

    /// The next output reduced modulo `bound`: a value in `0..bound`.
    ///
    /// Plain modulo reduction, bias and all, because the fixed-seed corpora
    /// built on it are pinned to exactly these values; a caller that needs an
    /// unbiased draw uses [`Xoshiro256::up_to`].
    ///
    /// # Panics
    ///
    /// When `bound` is zero: there is no value below it.
    #[must_use]
    pub const fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    /// [`Self::below`] for an index into a collection of `len` elements.
    ///
    /// # Panics
    ///
    /// When `len` is zero.
    #[must_use]
    pub const fn below_usize(&mut self, len: usize) -> usize {
        // A `usize` widens losslessly to `u64` on every supported target, and a
        // value below `len` narrows back.
        (self.below(len as u64)) as usize
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
    use super::{
        LCG64_MMIX_INCREMENT, SplitMix64, Xoshiro256, lcg64_next, signed_unit_stream,
        xorshift64_next,
    };

    #[test]
    fn below_reduces_the_splitmix64_stream_modulo_the_bound() {
        // The first outputs from seed 0 (pinned below) modulo 10.
        let mut generator = SplitMix64::new(0);
        let drawn: Vec<u64> = (0..6).map(|_| generator.below(10)).collect();
        assert_eq!(drawn, [5, 0, 9, 4, 7, 0]);
        let mut generator = SplitMix64::new(0);
        // The first output is below `u64::MAX`, so the widest bound returns it whole.
        assert_eq!(generator.below(u64::MAX), 0xE220_A839_7B1D_CDAF);
        assert_eq!(generator.below(1), 0);
        let mut by_index = SplitMix64::new(0);
        let indices: Vec<usize> = (0..6).map(|_| by_index.below_usize(10)).collect();
        assert_eq!(indices, [5, 0, 9, 4, 7, 0]);
    }

    #[test]
    #[should_panic(expected = "remainder with a divisor of zero")]
    fn below_zero_has_no_value() {
        let _ = SplitMix64::new(0).below(0);
    }

    /// Marsaglia's xorshift64 `(13, 7, 17)` from the seeds the existing
    /// fixtures use, and from 1, whose first output is the published one.
    #[test]
    fn xorshift64_matches_the_pinned_reference_outputs() {
        for (seed, expected) in [
            (
                0x2545_F491_4F6C_DD1D_u64,
                [
                    0x7F6C_280B_EAA8_E3E7,
                    0xE471_1987_1CF9_ABE0,
                    0x3517_4A41_58B8_A0B7,
                    0x62CE_1FFA_D85B_1C36,
                ],
            ),
            (
                0x9E37_79B9_7F4A_7C15,
                [
                    0xDC1B_77AE_0BF3_4DAD,
                    0x64F0_EEB9_026E_6076,
                    0x7B07_CE91_E590_6136,
                    0x305F_050C_368D_CC74,
                ],
            ),
            (
                1,
                [
                    0x0000_0000_4082_2041,
                    0x1000_4106_0C01_1441,
                    0x9B1E_842F_6E86_2629,
                    0xF554_F503_555D_8025,
                ],
            ),
        ] {
            let mut state = seed;
            let observed: Vec<u64> = (0..4).map(|_| xorshift64_next(&mut state)).collect();
            assert_eq!(observed, expected, "seed {seed:#x}");
            assert_eq!(state, expected[3], "the output is the state");
        }
        let mut zero = 0;
        assert_eq!(xorshift64_next(&mut zero), 0, "zero is a fixed point");
    }

    /// The 64-bit LCG under both increments the existing fixtures use.
    #[test]
    fn lcg64_matches_the_pinned_reference_outputs() {
        for (seed, increment, expected) in [
            (
                0x2545_F491_4F6C_DD1D_u64,
                LCG64_MMIX_INCREMENT,
                [
                    0x78DC_9D8B_3D1C_C268,
                    0x3765_A806_006F_4597,
                    0xE188_6FBB_935F_A5DA,
                    0x9C3A_48CE_9260_CEA1,
                ],
            ),
            (
                0x9E37_79B9_7F4A_7C15,
                LCG64_MMIX_INCREMENT,
                [
                    0x2CEA_EE21_BF46_BC00,
                    0xAA80_754D_1A1A_8D4F,
                    0xB3C4_904A_6D27_8932,
                    0xBC69_CF42_7684_6D19,
                ],
            ),
            (
                0x2545_F491_4F6C_DD1D,
                1,
                [
                    0x64D7_220C_45B5_411A,
                    0x75AE_AE21_C84A_5793,
                    0x0743_8468_B312_51D8,
                    0x013A_242B_538A_8AF9,
                ],
            ),
            (
                0x9E37_79B9_7F4A_7C15,
                1,
                [
                    0x18E5_72A2_C7DF_3AB2,
                    0xE8C9_7B68_E1F5_9F4B,
                    0xD97F_A4F7_8CDA_3530,
                    0x2169_AA9F_37AE_2971,
                ],
            ),
        ] {
            let mut state = seed;
            let observed: Vec<u64> = (0..4).map(|_| lcg64_next(&mut state, increment)).collect();
            assert_eq!(observed, expected, "seed {seed:#x}, increment {increment}");
            assert_eq!(state, expected[3], "the output is the state");
        }
    }

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
