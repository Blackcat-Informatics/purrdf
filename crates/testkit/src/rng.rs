// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one deterministic pseudo-random stream for tests and
//! benches: a SplitMix64 finaliser and a xoshiro256** generator seeded
//! through it.
//!
//! Every crate that needs a fixed-seed stream for generated test inputs draws
//! from here rather than hand-copying the mixing step, so there is exactly
//! one place that defines what "the stream" means. [`prop`](crate::prop)
//! itself is built on this module.
//!
//! Nothing here is cryptographically secure, and nothing here ships in a
//! release artifact's data path: it exists only behind `[dev-dependencies]`.

/// The SplitMix64 finalizer, without the golden-ratio increment: two
/// xor-shift-multiply rounds and a final xor-shift.
const fn mix_rounds(z: u64) -> u64 {
    let mut z = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A deterministic SplitMix64 step (Steele, Lea and Flood) over a plain
/// counter: `state` is advanced by the golden-ratio increment, then the
/// advanced value is mixed and returned. The next call advances the SAME raw
/// counter again, not the mixed output.
#[must_use]
pub const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mix_rounds(*state)
}

/// A self-composed SplitMix64 step: the golden-ratio increment is added to
/// `state` and the result mixed, and the caller is expected to feed the
/// returned value back in as the next `state` — so the counter itself is the
/// fully mixed value, not a raw increment. This is a DIFFERENT stream from
/// [`splitmix64_next`]'s (that one re-mixes a plain incrementing counter from
/// scratch on every call); the two are not interchangeable.
#[must_use]
pub const fn splitmix64_step(state: u64) -> u64 {
    mix_rounds(state.wrapping_add(0x9E37_79B9_7F4A_7C15))
}

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
    use super::{SplitMix64, Xoshiro256, signed_unit_stream, splitmix64_next, splitmix64_step};

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

    /// Every `#[cfg(test)]` SplitMix64 copy that used to live in
    /// `purrdf-iri`, `purrdf-columnar`, `purrdf-entail` and
    /// `purrdf-sparql-results` had this exact body — one golden-ratio
    /// increment and the SplitMix64 finalizer — under different names
    /// (`splitmix64_next` in three crates, `mix` in the fourth). This test
    /// pins the first 16 outputs from seed 0 and from
    /// `0x9E3779B97F4A7C15`, recorded from those copies before they were
    /// deleted, so a future edit to this module cannot silently change what
    /// any of those crates' existing tests generate.
    #[test]
    fn splitmix64_next_matches_the_deleted_crate_local_copies() {
        const SEED_ZERO: [u64; 16] = [
            0xE220_A839_7B1D_CDAF,
            0x6E78_9E6A_A1B9_65F4,
            0x06C4_5D18_8009_454F,
            0xF88B_B8A8_724C_81EC,
            0x1B39_896A_51A8_749B,
            0x53CB_9F0C_747E_A2EA,
            0x2C82_9ABE_1F45_32E1,
            0xC584_133A_C916_AB3C,
            0x3EE5_7890_41C9_8AC3,
            0xF3B8_488C_368C_B0A6,
            0x657E_ECDD_3CB1_3D09,
            0xC2D3_26E0_055B_DEF6,
            0x8621_A03F_E0BB_DB7B,
            0x8E1F_7555_983A_A92F,
            0xB54E_0F16_00CC_4D19,
            0x84BB_3F97_971D_80AB,
        ];
        const SEED_GOLDEN_RATIO: [u64; 16] = [
            0x6E78_9E6A_A1B9_65F4,
            0x06C4_5D18_8009_454F,
            0xF88B_B8A8_724C_81EC,
            0x1B39_896A_51A8_749B,
            0x53CB_9F0C_747E_A2EA,
            0x2C82_9ABE_1F45_32E1,
            0xC584_133A_C916_AB3C,
            0x3EE5_7890_41C9_8AC3,
            0xF3B8_488C_368C_B0A6,
            0x657E_ECDD_3CB1_3D09,
            0xC2D3_26E0_055B_DEF6,
            0x8621_A03F_E0BB_DB7B,
            0x8E1F_7555_983A_A92F,
            0xB54E_0F16_00CC_4D19,
            0x84BB_3F97_971D_80AB,
            0x7D29_825C_7552_1255,
        ];
        let mut state = 0u64;
        let seed_zero: [u64; 16] = std::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(seed_zero, SEED_ZERO);

        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let seed_golden_ratio: [u64; 16] = std::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(seed_golden_ratio, SEED_GOLDEN_RATIO);
    }

    /// The `[-1, 1)` stream `purrdf-sparql-eval` kept in its own
    /// `#[doc(hidden)]` module for its kNN metric tests, its kNN relation
    /// bench and its wasm32 reassociated-distance test, pinned as bit patterns
    /// from seed 0 and from `0x9E3779B97F4A7C15`, recorded from that module
    /// before it was deleted.
    #[test]
    fn signed_unit_stream_matches_the_deleted_crate_local_copy() {
        const SEED_ZERO: [u64; 16] = [
            0x3FE8_882A_0E5E_C772,
            0xBFC1_8761_955E_46A0,
            0xBFEE_4EE8_B9DF_FDB0,
            0x3FEE_22EE_2A1C_9320,
            0xBFE9_319D_A56B_95E4,
            0xBFD6_1A30_79C5_C0B0,
            0xBFE4_DF59_5078_2EB4,
            0x3FE1_6104_CEB2_45AA,
            0xBFE0_46A1_DBEF_8D9E,
            0x3FEC_EE12_230D_A32C,
            0xBFCA_8113_22C3_4EC8,
            0x3FE0_B4C9_B801_56F6,
            0x3FA8_8680_FF82_EF60,
            0x3FBC_3EEA_AB30_7550,
            0x3FDA_A707_8B00_6624,
            0x3FA2_ECFE_5E5C_7600,
        ];
        const SEED_GOLDEN_RATIO: [u64; 16] = [
            0xBFC1_8761_955E_46A0,
            0xBFEE_4EE8_B9DF_FDB0,
            0x3FEE_22EE_2A1C_9320,
            0xBFE9_319D_A56B_95E4,
            0xBFD6_1A30_79C5_C0B0,
            0xBFE4_DF59_5078_2EB4,
            0x3FE1_6104_CEB2_45AA,
            0xBFE0_46A1_DBEF_8D9E,
            0x3FEC_EE12_230D_A32C,
            0xBFCA_8113_22C3_4EC8,
            0x3FE0_B4C9_B801_56F6,
            0x3FA8_8680_FF82_EF60,
            0x3FBC_3EEA_AB30_7550,
            0x3FDA_A707_8B00_6624,
            0x3FA2_ECFE_5E5C_7600,
            0xBF96_B3ED_1C55_6F80,
        ];
        let bits = |seed| -> Vec<u64> {
            signed_unit_stream(16, seed)
                .into_iter()
                .map(f64::to_bits)
                .collect()
        };
        assert_eq!(bits(0), SEED_ZERO);
        assert_eq!(bits(0x9E37_79B9_7F4A_7C15), SEED_GOLDEN_RATIO);
    }

    /// The self-composed stream `purrdf-core` and `purrdf-sparql-eval` each
    /// copied under the name `splitmix64_step`, pinned from state
    /// `0xC057` before their copies were deleted.
    #[test]
    fn splitmix64_step_matches_the_deleted_crate_local_copies() {
        const FROM_0XC057: [u64; 16] = [
            0x1C22_A3B7_31BC_110E,
            0x5973_9F6F_16CD_4B42,
            0x6F5F_6C53_8C96_CFA8,
            0xAE1A_77B8_EB2F_2665,
            0x7E4E_CF20_F8E6_7C2F,
            0xFE83_248D_BB90_6BF0,
            0x2B62_FEC6_6B02_87AD,
            0x1D53_503C_ADFA_1E99,
            0x1BB6_C297_3F03_A2BA,
            0xC0A4_B544_205C_859B,
            0xF2D0_474A_D47E_8818,
            0xCF18_FF27_6E5E_0796,
            0x7713_D23D_9B4D_FF82,
            0x9C99_5235_29DC_7B13,
            0x311B_A5D7_578A_F63A,
            0x106C_1FC2_2115_832A,
        ];
        let mut state = 0xC057_u64;
        let observed: [u64; 16] = std::array::from_fn(|_| {
            state = splitmix64_step(state);
            state
        });
        assert_eq!(observed, FROM_0XC057);
    }
}
