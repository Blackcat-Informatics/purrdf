// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SplitMix64 (Steele, Lea and Flood): the 64-bit finalizer and the two
//! counter streams built on it.
//!
//! Every crate that needs a fixed-seed stream for generated inputs, or a
//! cheap bit-mixing step, draws from here, so there is exactly one place
//! that defines what the stream is. Nothing here is cryptographically
//! secure and nothing here is a table hash: use [`fixed`](crate::fixed) for
//! tables and a digest for identities.
//!
//! The two streams are **different** and are not interchangeable:
//!
//! * [`splitmix64_next`] advances a plain counter by the golden-ratio
//!   increment and mixes the *counter* — the reference SplitMix64 generator.
//! * [`splitmix64_step`] mixes `state + increment` and expects the caller to
//!   feed the *mixed output* back in as the next state.
//!
//! ```
//! use purrdf_hash::mix::{splitmix64_next, splitmix64_step};
//!
//! let mut counter = 0;
//! assert_eq!(splitmix64_next(&mut counter), 0xE220_A839_7B1D_CDAF);
//! assert_eq!(splitmix64_step(0), 0xE220_A839_7B1D_CDAF);
//! // From here the streams part: the counter is now 0x9E37_79B9_7F4A_7C15,
//! // whereas the self-composed state is the mixed value.
//! assert_ne!(splitmix64_next(&mut counter), splitmix64_step(0xE220_A839_7B1D_CDAF));
//! ```

/// The golden-ratio increment `⌊2^64/φ⌋`, SplitMix64's "gamma".
pub const SPLITMIX64_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;

/// The SplitMix64 finalizer, without the increment: two xor-shift-multiply
/// rounds and a final xor-shift. A bijection on `u64`, so distinct inputs
/// give distinct outputs.
#[must_use]
#[inline]
pub const fn splitmix64_finalize(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One draw of the reference SplitMix64 generator: `state` is advanced by
/// [`SPLITMIX64_INCREMENT`], and the advanced counter is finalized and
/// returned. The next call advances the same raw counter again, not the
/// mixed output.
#[must_use]
#[inline]
pub const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX64_INCREMENT);
    splitmix64_finalize(*state)
}

/// One self-composed SplitMix64 step: `state + increment`, finalized. The
/// caller feeds the result back in as the next `state`, so the state is the
/// mixed value rather than a raw counter — a different stream from
/// [`splitmix64_next`]'s after the first draw.
#[must_use]
#[inline]
pub const fn splitmix64_step(state: u64) -> u64 {
    splitmix64_finalize(state.wrapping_add(SPLITMIX64_INCREMENT))
}

#[cfg(test)]
mod tests {
    use super::{SPLITMIX64_INCREMENT, splitmix64_finalize, splitmix64_next, splitmix64_step};

    /// The reference implementation's first outputs from seed 0.
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

    /// From seed `0x9E37_79B9_7F4A_7C15`: the seed-0 stream, one draw on.
    const SEED_INCREMENT: [u64; 16] = [
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

    /// The self-composed stream from state `0xC057`.
    const STEP_FROM_0XC057: [u64; 16] = [
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

    #[test]
    fn next_matches_the_reference_outputs_from_seed_zero() {
        let mut state = 0u64;
        assert_eq!(splitmix64_next(&mut state), 0xE220_A839_7B1D_CDAF);
        assert_eq!(state, SPLITMIX64_INCREMENT, "the counter, not the output");
        let mut state = 0u64;
        let observed: [u64; 16] = core::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(observed, SEED_ZERO);
        let mut state = SPLITMIX64_INCREMENT;
        let observed: [u64; 16] = core::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(observed, SEED_INCREMENT);
    }

    #[test]
    fn step_matches_the_self_composed_reference_outputs() {
        assert_eq!(splitmix64_step(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64_step(0), SEED_ZERO[0]);
        let mut state = 0xC057_u64;
        let observed: [u64; 16] = core::array::from_fn(|_| {
            state = splitmix64_step(state);
            state
        });
        assert_eq!(observed, STEP_FROM_0XC057);
        // The streams part after the first draw.
        assert_ne!(splitmix64_step(SEED_ZERO[0]), SEED_ZERO[1]);
    }

    #[test]
    fn finalize_is_the_mixing_step_both_streams_share() {
        assert_eq!(splitmix64_finalize(0), 0);
        assert_eq!(splitmix64_finalize(SPLITMIX64_INCREMENT), SEED_ZERO[0]);
        assert_eq!(
            splitmix64_finalize(SPLITMIX64_INCREMENT.wrapping_mul(2)),
            SEED_ZERO[1]
        );
        assert_eq!(
            splitmix64_finalize(0xC057_u64.wrapping_add(SPLITMIX64_INCREMENT)),
            STEP_FROM_0XC057[0]
        );
        // Evaluated at compile time too.
        const FIRST: u64 = splitmix64_step(0);
        assert_eq!(FIRST, 0xE220_A839_7B1D_CDAF);
    }
}
