// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's deterministic pseudo-random streams for tests and
//! benches: a SplitMix64 finaliser and a xoshiro256** generator seeded
//! through it, and — because fixtures were frozen on them — the Fisher–Yates
//! shuffle over SplitMix64, a xorshift64 byte stream and Knuth's MMIX linear
//! congruential generator, each kept bit for bit as the tests that built on
//! it wrote it.
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
    signed_unit_of(splitmix64_next(state))
}

/// `len` values in `[-1, 1)` from the [`splitmix64_next`] counter stream
/// started at `seed`, drawn with [`signed_unit_next`].
#[must_use]
pub fn signed_unit_stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len).map(|_| signed_unit_next(&mut state)).collect()
}

/// [`signed_unit_next`], with a draw of exactly `0.0` replaced by
/// `zero_substitute`: the coordinate draw of a fixture whose vectors must
/// have a positive norm. The stream is consumed exactly as
/// [`signed_unit_next`] consumes it, so a fixture that never draws a zero is
/// bit-identical under both.
///
/// The substitutes in use, so a migration keeps its fixture. On this
/// linear-counter stream every site substitutes `0.125`: `purrdf`'s facade
/// tests and benches and `purrdf-hnsw`'s `linear_step`. `purrdf-sparql-eval`'s
/// kNN metric tests take [`signed_unit_next`] raw, with no substitute. The
/// `0.25` and `0.5` substitutes belong to fixtures on the self-composed
/// stream, which is [`signed_unit_step_nonzero`]'s — not this one's.
#[must_use]
pub fn signed_unit_next_nonzero(state: &mut u64, zero_substitute: f64) -> f64 {
    substitute_zero(signed_unit_next(state), zero_substitute)
}

/// One value in `[-1, 1)` from the self-composed [`splitmix64_step`] stream:
/// `state` is replaced by its next step, and the top 53 bits of that step
/// are mapped exactly as [`signed_unit_next`] maps them. This is a DIFFERENT
/// stream from [`signed_unit_next`]'s, for the same reason the two integer
/// steps differ; the two are not interchangeable.
#[must_use]
pub fn signed_unit_step(state: &mut u64) -> f64 {
    *state = splitmix64_step(*state);
    signed_unit_of(*state)
}

/// [`signed_unit_step`], with a draw of exactly `0.0` replaced by
/// `zero_substitute`. The substitutes in use on this self-composed stream:
/// `purrdf-hnsw`'s `Stream::unit` and its invariants tests substitute
/// `0.125`; its index, determinism, relation, conformance, oracle, exclusion,
/// adversarial and recall fixtures substitute `0.25`, as do the kNN benches of
/// `purrdf-core` and `purrdf-sparql-eval` (which round the draw through
/// binary32 before substituting); its builder fixture substitutes `0.5`.
#[must_use]
pub fn signed_unit_step_nonzero(state: &mut u64, zero_substitute: f64) -> f64 {
    substitute_zero(signed_unit_step(state), zero_substitute)
}

/// The `[-1, 1)` mapping every signed-unit draw shares: the top 53 bits of
/// `bits` as a fraction of 2^53, doubled and shifted down by one. Every step
/// is exact in binary64.
fn signed_unit_of(bits: u64) -> f64 {
    ((bits >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
}

/// `zero_substitute` when `value` is exactly zero, `value` otherwise.
const fn substitute_zero(value: f64, zero_substitute: f64) -> f64 {
    if value == 0.0 { zero_substitute } else { value }
}

/// Shuffle `items` in place: a Fisher–Yates shuffle driven by
/// [`splitmix64_next`] over `state` — the loop `purrdf-core`'s and
/// `purrdf-datalog`'s determinism tests wrote, kept exactly so the orders
/// they pinned survive. Positions are visited from the last down to the
/// second, and each is swapped with a position drawn as
/// `splitmix64_next(state) % (i + 1)`: the modulo form, like
/// [`SplitMix64::below`]. A slice of fewer than two items draws nothing.
/// The same `state` always yields the same order, on every target.
pub fn permute<T>(items: &mut [T], state: &mut u64) {
    for i in (1..items.len()).rev() {
        let j = (splitmix64_next(state) % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
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

    /// A value in `0..bound` by the modulo form, `next_u64() % bound`: the
    /// draw the per-crate `struct SplitMix { next, below }` wrappers made,
    /// kept as they made it so every existing expectation survives. The bias
    /// of the modulo is at most `bound / 2^64`, nothing a test input notices;
    /// [`Xoshiro256::up_to`] is the unbiased draw for a strategy.
    ///
    /// # Panics
    ///
    /// `bound` must be positive: there is no value below zero.
    #[must_use]
    pub const fn below(&mut self, bound: u64) -> u64 {
        assert!(
            bound > 0,
            "a draw below zero has no value: the bound must be positive"
        );
        self.next_u64() % bound
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

/// Xorshift64 (Marsaglia) with the shift triple 13, 7, 17: the byte-stream
/// generator behind `purrdf-hash`'s frozen random-input vectors and
/// `purrdf-core`'s PURREMB adversarial inputs. A step is exactly
/// `state ^= state << 13; state ^= state >> 7; state ^= state << 17;` and the
/// output is the new state — bit for bit what those tests compute. A frozen
/// vector file depends on it, so this stream must never change.
#[derive(Debug, Clone)]
pub struct Xorshift64(u64);

impl Xorshift64 {
    /// A generator seeded at `seed`.
    ///
    /// # Panics
    ///
    /// `seed` must be non-zero: the xorshift step maps zero to zero, so a
    /// zero seed would be a stream of zeros forever. No frozen vector uses
    /// one.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        assert!(
            seed != 0,
            "a Xorshift64 seed must be non-zero: zero is a fixed point of the step"
        );
        Self(seed)
    }

    /// The next 64-bit output: the state after one step.
    #[must_use]
    pub const fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Fill `bytes` with one step per byte, keeping the low byte of each
    /// output — how both frozen-vector sites draw their input bytes.
    pub fn fill_bytes(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

/// A 64-bit linear congruential generator with Knuth's MMIX constants:
/// `state = state * 6364136223846793005 + 1442695040888963407`, wrapping.
/// The seeded integer recurrence behind fixtures that need a spread of values
/// with no floating point anywhere — `purrdf-geo`'s arbitrary geometries,
/// `purrdf-text`'s scoring corpus, `purrdf-sparql-eval`'s generated
/// expressions and kNN crowd, `purrdf-markdown`'s heading levels. The low
/// bits of an LCG are weak, so every output is taken from the well-mixed high
/// end of the state; the sites differ only in how many bits they take, and
/// both selections are here so each keeps its sequence.
#[derive(Debug, Clone)]
pub struct Lcg(u64);

impl Lcg {
    /// Knuth's MMIX multiplier.
    pub const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    /// Knuth's MMIX increment.
    pub const INCREMENT: u64 = 1_442_695_040_888_963_407;

    /// A generator seeded at `seed`. Every seed is valid, zero included.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next state, whole: one step of the recurrence.
    #[must_use]
    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.0
    }

    /// The high 32 bits of the next state, `state >> 32` — `purrdf-geo`'s
    /// `next_u32`, whose `below(bound)` is `next_u32_high() % bound`.
    #[must_use]
    pub const fn next_u32_high(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// The high 31 bits of the next state, `state >> 33` — the selection of
    /// `purrdf-text`, `purrdf-sparql-eval` and `purrdf-markdown`.
    #[must_use]
    pub const fn next_u31_high(&mut self) -> u64 {
        self.next_u64() >> 33
    }

    /// A value in `0..bound` by the modulo form over the high 31 bits,
    /// `next_u31_high() % bound`: `purrdf-text`'s `next_below` and
    /// `purrdf-sparql-eval`'s `next(bound)`, exactly.
    ///
    /// # Panics
    ///
    /// `bound` must be positive: there is no value below zero.
    #[must_use]
    pub const fn below(&mut self, bound: u64) -> u64 {
        assert!(
            bound > 0,
            "a draw below zero has no value: the bound must be positive"
        );
        self.next_u31_high() % bound
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Lcg, SplitMix64, Xorshift64, Xoshiro256, permute, signed_unit_next,
        signed_unit_next_nonzero, signed_unit_step, signed_unit_step_nonzero, signed_unit_stream,
        splitmix64_next, splitmix64_step, substitute_zero,
    };

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

    /// `purrdf-iri`, `purrdf-columnar`, `purrdf-entail` and
    /// `purrdf-sparql-results` share this exact SplitMix64 body — one
    /// golden-ratio increment and the SplitMix64 finalizer — through this
    /// module. This test pins the first 16 outputs from seed 0 and from
    /// `0x9E3779B97F4A7C15`, so a future edit to this module cannot silently
    /// change what any of those crates' existing tests generate.
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

        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let seed_golden_ratio: [u64; 16] = std::array::from_fn(|_| splitmix64_next(&mut state));
        assert_eq!(seed_golden_ratio, SEED_GOLDEN_RATIO);
    }

    /// `purrdf-core` and `purrdf-sparql-eval` each use this self-composed
    /// stream under the name `splitmix64_step`. This test pins its output
    /// from state `0xC057`.
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

    /// `below` is the modulo form the per-crate wrappers used: the next
    /// output reduced by the bound, one output per draw. Pinned from seed 0
    /// against the outputs pinned above (`0xE220_A839_7B1D_CDAF % 10 == 5`).
    #[test]
    fn below_is_the_next_output_modulo_the_bound() {
        let mut generator = SplitMix64::new(0);
        let observed: [u64; 4] = std::array::from_fn(|_| generator.below(10));
        assert_eq!(observed, [5, 0, 9, 4]);
        let mut generator = SplitMix64::new(0);
        let observed: [u64; 4] = std::array::from_fn(|_| generator.below(7));
        assert_eq!(observed, [2, 1, 2, 4]);
        let mut generator = SplitMix64::new(0);
        assert_eq!(generator.below(1), 0);
        assert_eq!(generator.below(u64::MAX), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    #[should_panic(expected = "the bound must be positive")]
    fn a_draw_below_zero_is_refused() {
        let _ = SplitMix64::new(0).below(0);
    }

    /// The loop `purrdf-core` and `purrdf-datalog` wrote, verbatim: the
    /// oracle `permute` must agree with on every seed and length.
    fn reference_permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
        let mut out = items.to_vec();
        let mut state = seed;
        for i in (1..out.len()).rev() {
            let j = (splitmix64_next(&mut state) % (i as u64 + 1)) as usize;
            out.swap(i, j);
        }
        out
    }

    #[test]
    fn permute_is_the_workspace_fisher_yates_loop() {
        for seed in [0, 1, 0x00C0_FFEE, u64::MAX] {
            for len in [0usize, 1, 2, 3, 8, 16, 33] {
                let items: Vec<usize> = (0..len).collect();
                let mut observed = items.clone();
                let mut state = seed;
                permute(&mut observed, &mut state);
                assert_eq!(
                    observed,
                    reference_permute(&items, seed),
                    "seed {seed:#x}, len {len}"
                );
                // One draw per position from the last down to the second.
                let mut expected_state = seed;
                for _ in 1..len {
                    let _ = splitmix64_next(&mut expected_state);
                }
                assert_eq!(state, expected_state, "seed {seed:#x}, len {len}");
                let mut sorted = observed;
                sorted.sort_unstable();
                assert_eq!(sorted, items, "a permutation of the input");
            }
        }
    }

    /// Pinned orders, so a future edit cannot silently move what the
    /// determinism tests built on this loop generate.
    #[test]
    fn permute_matches_the_pinned_orders() {
        let mut items: Vec<u8> = (0..8).collect();
        let mut state = 0;
        permute(&mut items, &mut state);
        assert_eq!(items, [2, 5, 0, 3, 4, 6, 1, 7]);
        assert_eq!(
            state, 6_018_027_440_424_182_931,
            "seven golden-ratio increments from zero"
        );
        let mut items: Vec<u8> = (0..8).collect();
        let mut state = 0x00C0_FFEE;
        permute(&mut items, &mut state);
        assert_eq!(items, [6, 1, 7, 4, 0, 3, 5, 2]);
        let mut items: Vec<u8> = (0..16).collect();
        let mut state = 1;
        permute(&mut items, &mut state);
        assert_eq!(
            items,
            [2, 11, 10, 6, 7, 13, 14, 0, 12, 5, 15, 9, 3, 8, 4, 1]
        );
    }

    /// Over a stream that never draws exactly zero — as no short stream does —
    /// the nonzero form is the plain form bit for bit, with the same state
    /// after each draw; the substitution itself is checked on the value the
    /// draw could produce.
    #[test]
    fn nonzero_draws_are_the_plain_draws_and_zero_is_substituted() {
        let mut plain = 0x00C0_FFEE_u64;
        let mut nonzero = 0x00C0_FFEE_u64;
        for _ in 0..1_000 {
            let expected = signed_unit_next(&mut plain);
            let observed = signed_unit_next_nonzero(&mut nonzero, 0.125);
            assert_ne!(expected, 0.0);
            assert_eq!(observed.to_bits(), expected.to_bits());
            assert_eq!(nonzero, plain);
        }
        let mut plain = 0xC057_u64;
        let mut nonzero = 0xC057_u64;
        for _ in 0..1_000 {
            let expected = signed_unit_step(&mut plain);
            let observed = signed_unit_step_nonzero(&mut nonzero, 0.25);
            assert_ne!(expected, 0.0);
            assert_eq!(observed.to_bits(), expected.to_bits());
            assert_eq!(nonzero, plain);
        }
        for substitute in [0.125, 0.25, 0.5] {
            assert_eq!(
                substitute_zero(0.0, substitute).to_bits(),
                substitute.to_bits()
            );
            assert_eq!(
                substitute_zero(-0.0, substitute).to_bits(),
                substitute.to_bits()
            );
            assert_eq!(
                substitute_zero(0.75, substitute).to_bits(),
                0.75_f64.to_bits()
            );
            assert_eq!(
                substitute_zero(-1.0, substitute).to_bits(),
                (-1.0_f64).to_bits()
            );
        }
    }

    /// The self-composed unit stream is `splitmix64_step` mapped as
    /// `purrdf-hnsw`'s `Stream::unit` maps it — the loop written out here as
    /// the oracle — and its first values from state 0 and `0xC057` are pinned
    /// as bit patterns. From state 0 the first value is the linear stream's
    /// first value (both are one mix of the golden ratio); the second is not.
    #[test]
    fn signed_unit_step_matches_the_reference_loop_and_the_pinned_outputs() {
        const FROM_ZERO: [u64; 4] = [
            0x3FE8_882A_0E5E_C772,
            0x3FD3_836E_97A6_8CBC,
            0xBFE7_1F62_90F1_C0D2,
            0xBFE7_B3E2_DD55_4E00,
        ];
        const FROM_0XC057: [u64; 4] = [
            0xBFE8_F757_1233_90FC,
            0xBFD3_4630_4874_995C,
            0xBFC0_A093_AC73_6938,
            0x3FD7_0D3B_DC75_9790,
        ];
        for (seed, expected) in [(0, FROM_ZERO), (0xC057, FROM_0XC057)] {
            let mut state = seed;
            let observed: [u64; 4] =
                std::array::from_fn(|_| signed_unit_step(&mut state).to_bits());
            assert_eq!(observed, expected, "seed {seed:#x}");
        }
        assert_eq!(FROM_ZERO[0], signed_unit_stream(2, 0)[0].to_bits());
        assert_ne!(FROM_ZERO[1], signed_unit_stream(2, 0)[1].to_bits());
        let mut reference_state = 0x1234_5678_9ABC_DEF0_u64;
        let mut state = reference_state;
        for _ in 0..1_000 {
            reference_state = splitmix64_step(reference_state);
            let expected =
                ((reference_state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
            assert_eq!(signed_unit_step(&mut state).to_bits(), expected.to_bits());
            assert_eq!(state, reference_state);
        }
    }

    /// The three-shift loop `purrdf-hash`'s and `purrdf-core`'s tests wrote,
    /// verbatim, as the oracle.
    fn reference_xorshift_bytes(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed;
        let mut bytes = vec![0u8; len];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state as u8;
        }
        bytes
    }

    /// The first three outputs from seed 1, computed by the reference loop,
    /// and the first output and bytes from the golden ratio, which is where
    /// `purrdf-core`'s adversarial inputs start.
    #[test]
    fn xorshift64_matches_the_pinned_known_answers_and_the_reference_loop() {
        let mut generator = Xorshift64::new(1);
        assert_eq!(generator.next_u64(), 0x0000_0000_4082_2041);
        assert_eq!(generator.next_u64(), 0x1000_4106_0C01_1441);
        assert_eq!(generator.next_u64(), 0x9B1E_842F_6E86_2629);
        let mut generator = Xorshift64::new(0x9E37_79B9_7F4A_7C15);
        assert_eq!(generator.next_u64(), 0xDC1B_77AE_0BF3_4DAD);
        let mut bytes = [0u8; 64];
        let mut generator = Xorshift64::new(0x9E37_79B9_7F4A_7C15);
        generator.fill_bytes(&mut bytes);
        assert_eq!(&bytes[..3], &[0xAD, 0x76, 0x36]);
        assert_eq!(
            bytes.to_vec(),
            reference_xorshift_bytes(0x9E37_79B9_7F4A_7C15, 64)
        );
        for seed in [1, 2, 0xDEAD_BEEF, 0x6C44_888B_D9AE_AD5B, u64::MAX] {
            let mut generator = Xorshift64::new(seed);
            let mut bytes = vec![0u8; 100];
            generator.fill_bytes(&mut bytes);
            assert_eq!(bytes, reference_xorshift_bytes(seed, 100), "seed {seed:#x}");
        }
    }

    #[test]
    #[should_panic(expected = "must be non-zero")]
    fn a_zero_xorshift_seed_is_refused() {
        let _ = Xorshift64::new(0);
    }

    /// From seed 0 the first state is the increment itself; the rest are
    /// pinned, under each selection, along with `purrdf-text`'s first three
    /// document lengths from its own seed.
    #[test]
    fn lcg_matches_the_pinned_known_answers() {
        let mut generator = Lcg::new(0);
        assert_eq!(generator.next_u64(), Lcg::INCREMENT);
        assert_eq!(Lcg::INCREMENT, 0x1405_7B7E_F767_814F);
        assert_eq!(generator.next_u64(), 0x1A08_EE11_84BA_6D32);
        assert_eq!(generator.next_u64(), 0x9AF6_7822_2E72_8119);
        let mut generator = Lcg::new(0);
        let high32: [u32; 3] = std::array::from_fn(|_| generator.next_u32_high());
        assert_eq!(high32, [0x1405_7B7E, 0x1A08_EE11, 0x9AF6_7822]);
        let mut generator = Lcg::new(0);
        let high31: [u64; 3] = std::array::from_fn(|_| generator.next_u31_high());
        assert_eq!(high31, [0x0A02_BDBF, 0x0D04_7708, 0x4D7B_3C11]);
        let mut generator = Lcg::new(1);
        assert_eq!(generator.next_u64(), 0x6C57_6FAC_43FD_007C);
        assert_eq!(generator.next_u64(), 0x8268_86B3_864A_1B1B);
        assert_eq!(generator.next_u64(), 0xA5FA_E199_2097_AA0E);
        let mut generator = Lcg::new(0x5EED_1234_9ABC_DEF0);
        let lengths: [u64; 3] = std::array::from_fn(|_| generator.below(8));
        assert_eq!(lengths, [7, 1, 1]);
    }

    /// The recurrence and both selections, against the loops the sites wrote.
    #[test]
    fn lcg_selections_agree_with_the_reference_loops() {
        let seed = 0x2545_F491_4F6C_DD1D;
        let mut state = seed;
        let mut generator = Lcg::new(seed);
        for _ in 0..1_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let mut high32 = generator.clone();
            let mut high31 = generator.clone();
            let mut below = generator.clone();
            assert_eq!(high32.next_u32_high(), (state >> 32) as u32);
            assert_eq!(high31.next_u31_high(), state >> 33);
            assert_eq!(below.below(97), (state >> 33) % 97);
            assert_eq!(generator.next_u64(), state);
        }
    }

    #[test]
    #[should_panic(expected = "the bound must be positive")]
    fn an_lcg_draw_below_zero_is_refused() {
        let _ = Lcg::new(1).below(0);
    }
}
