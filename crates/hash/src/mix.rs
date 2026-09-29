// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SplitMix64 (Steele, Lea and Flood, "Fast Splittable Pseudorandom Number
//! Generators", OOPSLA 2014): the golden-ratio increment `0x9E3779B97F4A7C15`
//! and the finaliser that mixes a 64-bit word through two
//! xor-shift-multiply rounds (multipliers `0xBF58476D1CE4E5B9` and
//! `0x94D049BB133111EB`, shifts 30, 27 and 31) and a final xor-shift.
//!
//! Two streams are built from them, and they are not interchangeable:
//!
//! * [`splitmix64_next`] is the published generator. The state is a plain
//!   counter advanced by the increment, and each output is the finaliser of
//!   the advanced counter.
//! * [`splitmix64_step`] is self-composed. The caller feeds each output back
//!   in as the next state, so the state is the fully mixed value.
//!
//! Every function is `const`, total and target-independent: wrapping integer
//! arithmetic only, so a value is the same on every target, wasm32 included.
//! Nothing here is cryptographically secure; it is a seed expander, a
//! deterministic test-input stream and a strong 64-bit finaliser.

/// The golden-ratio increment: `⌊2^64 / φ⌋`, which is odd.
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// The SplitMix64 finaliser of `z`, without the golden-ratio increment: two
/// xor-shift-multiply rounds and a final xor-shift. A bijection on `u64`.
#[must_use]
pub const fn splitmix64_finalize(z: u64) -> u64 {
    let mut z = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The next output of the SplitMix64 generator over the counter `state`:
/// `state` is advanced by the golden-ratio increment, then the advanced value
/// is finalised and returned. The next call advances the same raw counter
/// again, not the output.
#[must_use]
pub const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(GOLDEN_GAMMA);
    splitmix64_finalize(*state)
}

/// One self-composed SplitMix64 step: the finaliser of `state` plus the
/// golden-ratio increment. The caller feeds the returned value back in as
/// the next `state`, so the stream's state is the fully mixed value; that is
/// a different stream from [`splitmix64_next`]'s, which re-mixes a plain
/// incrementing counter on every call.
#[must_use]
pub const fn splitmix64_step(state: u64) -> u64 {
    splitmix64_finalize(state.wrapping_add(GOLDEN_GAMMA))
}

#[cfg(test)]
mod tests {
    use super::{splitmix64_finalize, splitmix64_next, splitmix64_step};

    /// The first sixteen outputs of the published generator from seed 0 and
    /// from seed `0x9E3779B97F4A7C15` (the second stream is the first shifted
    /// by one, since that seed is the counter after one step from 0).
    #[test]
    fn splitmix64_next_matches_the_pinned_reference_outputs() {
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
        assert_eq!(state, 16u64.wrapping_mul(0x9E37_79B9_7F4A_7C15));

        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let seed_golden_ratio: [u64; 16] = std::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(seed_golden_ratio, SEED_GOLDEN_RATIO);
    }

    /// The self-composed stream from state `0xC057`.
    #[test]
    fn splitmix64_step_matches_the_pinned_reference_outputs() {
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

    /// The finaliser is the generator's output function: the first output
    /// from seed 0 is the finaliser of one increment, and 0 is its fixed point.
    #[test]
    fn splitmix64_finalize_is_the_output_function() {
        assert_eq!(
            splitmix64_finalize(0x9E37_79B9_7F4A_7C15),
            0xE220_A839_7B1D_CDAF
        );
        assert_eq!(splitmix64_finalize(0), 0);
        const FINALIZED_AT_COMPILE_TIME: u64 = splitmix64_finalize(0x9E37_79B9_7F4A_7C15);
        assert_eq!(FINALIZED_AT_COMPILE_TIME, 0xE220_A839_7B1D_CDAF);
    }
}
