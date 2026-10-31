#![allow(
    clippy::cast_lossless,
    clippy::inline_always,
    clippy::manual_ilog2,
    clippy::struct_excessive_bools,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Optimal-parser cost model.
//!
//! Hosts the price tables (`HcOptState`), the `bit_weight` /
//! `frac_weight` functions that translate symbol counts to bit-costs, and
//! the per-symbol price helpers (`literal_price`, `lit_length_price`,
//! `offset_price_for`, `match_length_price`) consumed by the optimal
//! parser DP body in [`crate::encoding::opt`].
//!
//! Upstream zstd parity: mirrors `zstd_opt.c` price functions and stat
//! handling. The per-DP-iteration hot path (`bit_weight` /
//! `frac_weight`, `*_price` helpers, `update_stats`, `set_base_prices`,
//! `downscale_stats`, `scale_stats`) already runs on raw `+`/`-`/`*`
//! arithmetic guarded by `debug_assert!`. The
//! init paths (`rescale_freqs` literal histogram build,
//! `HC_BLOCKSIZE_MAX.saturating_sub(1)` fallback in `lit_length_price`)
//! keep the original `saturating_*` defensiveness; folding that
//! cleanup into the same patch would conflate a behaviour-preserving
//! split with a performance tweak.
//!
//! Shared with `match_generator.rs`
//! (structural split). No behaviour change — types, constants, and
//! method names are identical to the pre-extraction monolith.

pub(crate) const HC_MAX_LIT: usize = 255;
pub(crate) const HC_MAX_LL: usize = 35;
pub(crate) const HC_MAX_ML: usize = 52;
pub(crate) const HC_MAX_OFF: usize = 31;
pub(crate) const HC_LITFREQ_ADD: u32 = 2;
pub(crate) const HC_PREDEF_THRESHOLD: usize = 8;
pub(crate) const HC_BITCOST_MULTIPLIER: u32 = 1 << 8;
pub(crate) const HC_BLOCKSIZE_MAX: usize = crate::common::MAX_BLOCK_SIZE as usize;
pub(crate) const HC_OPT_NUM: usize = 1 << 12;
/// Fixed stride of each price/generation region in the optimal-parser price
/// arena. The DP frontier never exceeds `HC_OPT_NUM`, so `HC_OPT_NUM + 1`
/// indices (positions `0..=frontier_limit`) always fit.
pub(crate) const HC_OPT_PRICE_STRIDE: usize = HC_OPT_NUM + 1;
/// Backing length (in `[price, generation]` pairs) of the single price
/// arena: two fixed-stride regions (LL pairs, ML pairs). Each `[u32; 2]`
/// cell co-locates a code's price and its generation stamp so the optimal
/// parser's cache probe touches one line instead of two strided regions.
pub(crate) const HC_OPT_PRICE_ARENA_LEN: usize = 2 * HC_OPT_PRICE_STRIDE;
/// Node-frontier buffer length (`nodes` / `store`): positions
/// `0..=frontier_limit` plus the `+2` lookahead slack the DP body uses.
pub(crate) const HC_OPT_NODE_LEN: usize = HC_OPT_NUM + 2;
pub(crate) const HC_FORMAT_MINMATCH: usize = 3;

#[derive(Copy, Clone)]
pub(crate) enum HcOptPriceType {
    Dynamic,
    Predefined,
}

#[derive(Clone)]
pub(crate) struct HcDictEntropySeed {
    pub(crate) has_lit: bool,
    pub(crate) has_ll: bool,
    pub(crate) has_ml: bool,
    pub(crate) has_of: bool,
    pub(crate) lit_bits: [u8; HC_MAX_LIT + 1],
    pub(crate) ll_bits: [u8; HC_MAX_LL + 1],
    pub(crate) ml_bits: [u8; HC_MAX_ML + 1],
    pub(crate) of_bits: [u8; HC_MAX_OFF + 1],
}

#[derive(Clone)]
pub(crate) struct HcOptState {
    pub(crate) lit_freq: [u32; HC_MAX_LIT + 1],
    pub(crate) lit_length_freq: [u32; HC_MAX_LL + 1],
    pub(crate) match_length_freq: [u32; HC_MAX_ML + 1],
    pub(crate) off_code_freq: [u32; HC_MAX_OFF + 1],
    pub(crate) lit_sum: u32,
    pub(crate) lit_length_sum: u32,
    pub(crate) match_length_sum: u32,
    pub(crate) off_code_sum: u32,
    pub(crate) lit_sum_base_price: u32,
    pub(crate) lit_length_sum_base_price: u32,
    pub(crate) match_length_sum_base_price: u32,
    pub(crate) off_code_sum_base_price: u32,
    pub(crate) price_type: HcOptPriceType,
    literals_compressed: bool,
    pub(crate) dictionary_seed: Option<HcDictEntropySeed>,
}

impl HcOptState {
    pub(crate) fn new() -> Self {
        Self {
            lit_freq: [0; HC_MAX_LIT + 1],
            lit_length_freq: [0; HC_MAX_LL + 1],
            match_length_freq: [0; HC_MAX_ML + 1],
            off_code_freq: [0; HC_MAX_OFF + 1],
            lit_sum: 0,
            lit_length_sum: 0,
            match_length_sum: 0,
            off_code_sum: 0,
            lit_sum_base_price: 0,
            lit_length_sum_base_price: 0,
            match_length_sum_base_price: 0,
            off_code_sum_base_price: 0,
            price_type: HcOptPriceType::Dynamic,
            literals_compressed: true,
            dictionary_seed: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(crate) fn bit_weight(stat: u32) -> u32 {
        // Upstream zstd parity: stat+1 ≥ 1 ⇒ leading_zeros ≤ 31 ⇒ 31-lz ≥ 0.
        // hb ≤ 31, MULTIPLIER = 256 ⇒ product ≤ 7936, no overflow.
        let hb = 31 - (stat + 1).leading_zeros();
        hb * HC_BITCOST_MULTIPLIER
    }

    pub(crate) fn frac_weight(raw_stat: u32) -> u32 {
        // Upstream zstd parity (ZSTD_fracWeight). All operands bounded: stat fits u32,
        // hb ∈ 0..=31, BWeight ≤ 7936, FWeight ≤ 65535 ⇒ sum no overflow.
        let stat = raw_stat + 1;
        let hb = 31 - stat.leading_zeros();
        let b_weight = hb * HC_BITCOST_MULTIPLIER;
        let f_weight = (stat << 8) >> hb;
        b_weight + f_weight
    }

    pub(crate) fn weight(stat: u32, accurate: bool) -> u32 {
        if accurate {
            Self::frac_weight(stat)
        } else {
            Self::bit_weight(stat)
        }
    }

    pub(crate) fn downscale_stats(table: &mut [u32], shift: u32, base1: bool) -> u32 {
        // Upstream zstd parity: table sums bounded by block size (≤ 256K) ⇒ no overflow.
        let mut sum = 0u32;
        for stat in table {
            let base = if base1 { 1 } else { u32::from(*stat > 0) };
            let new_stat = base + (*stat >> shift);
            *stat = new_stat;
            sum += new_stat;
        }
        sum
    }

    pub(crate) fn scale_stats(table: &mut [u32], log_target: u32) -> u32 {
        let prev_sum = table.iter().copied().sum::<u32>();
        let factor = prev_sum >> log_target;
        if factor <= 1 {
            return prev_sum;
        }
        // factor ≥ 2 ⇒ leading_zeros ≤ 30 ⇒ 31-lz ≥ 1.
        let shift = 31 - factor.leading_zeros();
        Self::downscale_stats(table, shift, true)
    }

    pub(crate) fn set_base_prices(&mut self, accurate: bool) {
        self.lit_sum_base_price = if self.literals_compressed() {
            Self::weight(self.lit_sum, accurate)
        } else {
            0
        };
        self.lit_length_sum_base_price = Self::weight(self.lit_length_sum, accurate);
        self.match_length_sum_base_price = Self::weight(self.match_length_sum, accurate);
        self.off_code_sum_base_price = Self::weight(self.off_code_sum, accurate);
    }

    pub(crate) fn literals_compressed(&self) -> bool {
        self.literals_compressed
    }

    pub(crate) fn seed_dictionary_entropy(
        &mut self,
        huff: Option<&crate::huff0::huff0_encoder::HuffmanTable>,
        ll: Option<&crate::fse::fse_encoder::FSETable>,
        ml: Option<&crate::fse::fse_encoder::FSETable>,
        of: Option<&crate::fse::fse_encoder::FSETable>,
    ) {
        if huff.is_none() && ll.is_none() && ml.is_none() && of.is_none() {
            self.dictionary_seed = None;
            return;
        }
        let mut lit_bits = [0u8; HC_MAX_LIT + 1];
        if let Some(huff) = huff {
            for (sym, slot) in lit_bits.iter_mut().enumerate() {
                *slot = huff.num_bits_for_symbol(sym as u8).unwrap_or(0);
            }
        }
        let mut ll_bits = [0u8; HC_MAX_LL + 1];
        if let Some(ll) = ll {
            for (sym, slot) in ll_bits.iter_mut().enumerate() {
                *slot = ll.max_num_bits_for_symbol(sym as u8).unwrap_or(0);
            }
        }
        let mut ml_bits = [0u8; HC_MAX_ML + 1];
        if let Some(ml) = ml {
            for (sym, slot) in ml_bits.iter_mut().enumerate() {
                *slot = ml.max_num_bits_for_symbol(sym as u8).unwrap_or(0);
            }
        }
        let mut of_bits = [0u8; HC_MAX_OFF + 1];
        if let Some(of) = of {
            for (sym, slot) in of_bits.iter_mut().enumerate() {
                *slot = of.max_num_bits_for_symbol(sym as u8).unwrap_or(0);
            }
        }
        self.dictionary_seed = Some(HcDictEntropySeed {
            has_lit: huff.is_some(),
            has_ll: ll.is_some(),
            has_ml: ml.is_some(),
            has_of: of.is_some(),
            lit_bits,
            ll_bits,
            ml_bits,
            of_bits,
        });
    }

    fn apply_seeded_table<const N: usize>(
        table: &mut [u32; N],
        bits: &[u8; N],
        scale_log: u8,
    ) -> u32 {
        let mut sum = 0u32;
        for (slot, &bit_cost) in table.iter_mut().zip(bits.iter()) {
            let value = if bit_cost == 0 || bit_cost >= scale_log {
                1
            } else {
                1u32 << (scale_log - bit_cost)
            };
            *slot = value;
            sum += value;
        }
        sum
    }

    #[inline(always)]
    pub(crate) fn lit_code_and_bits(lit_len: usize) -> (usize, u32) {
        // Upstream zstd parity (`ZSTD_LLcode` + `LL_bits`, zstd_compress_internal.h):
        // a direct table lookup for ll < 64 and `highbit32(ll) + delta` above,
        // replacing the former 20-arm range cascade (a `jae` comparison ladder
        // that was ~8% of L16-btopt-dict). Same (code, nbBits) values — only
        // the derivation changes.
        const LL_CODE: [u8; 64] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 16, 17, 17, 18, 18, 19, 19,
            20, 20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 23, 23, 23, 23, 23, 23,
            23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
        ];
        // Extra bits per LL code (index = code, 0..=35).
        const LL_BITS: [u8; 36] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9,
            10, 11, 12, 13, 14, 15, 16,
        ];
        let ll = lit_len.min(131_071) as u32;
        if ll < 64 {
            let code = LL_CODE[ll as usize] as usize;
            (code, LL_BITS[code] as u32)
        } else {
            // ll in 64..=131_071 ⇒ highbit32 in 6..=16; code = hb + 19
            // (25..=35), nbBits = hb. Matches the cascade's upper arms.
            let hb = 31 - ll.leading_zeros();
            ((hb + 19) as usize, hb)
        }
    }

    #[inline(always)]
    pub(crate) fn ml_code_and_bits(match_len: usize) -> (usize, u32) {
        // Upstream zstd parity (`ZSTD_MLcode` + `ML_bits`, zstd_compress_internal.h):
        // direct table lookup on `mlBase = ml - MINMATCH` for mlBase < 128 and
        // `highbit32(mlBase) + 36` above — same (code, nbBits) values as the
        // former 22-arm range cascade, replacing its jae ladder. Cold on
        // random data but hot on the compressible btopt/btultra band, where
        // the optimal parser calls match_length_price per candidate.
        const ML_CODE: [u8; 128] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 32, 33, 33, 34, 34, 35, 35, 36, 36, 36, 36, 37, 37,
            37, 37, 38, 38, 38, 38, 38, 38, 38, 38, 39, 39, 39, 39, 39, 39, 39, 39, 40, 40, 40, 40,
            40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41,
            41, 41, 41, 41, 41, 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
            42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        ];
        // Extra bits per ML code (index = code, 0..=52).
        const ML_BITS: [u8; 53] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ];
        let ml = match_len.clamp(3, 131_074);
        let ml_base = (ml - 3) as u32;
        if ml_base < 128 {
            let code = ML_CODE[ml_base as usize] as usize;
            (code, ML_BITS[code] as u32)
        } else {
            // mlBase in 128..=131_071 ⇒ highbit32 in 7..=16; code = hb + 36.
            let hb = 31 - ml_base.leading_zeros();
            ((hb + 36) as usize, hb)
        }
    }

    pub(crate) fn rescale_freqs(&mut self, src: &[u8], profile: HcOptimalCostProfile) {
        self.price_type = HcOptPriceType::Dynamic;
        if self.lit_length_sum == 0 {
            if src.len() <= HC_PREDEF_THRESHOLD {
                self.price_type = HcOptPriceType::Predefined;
            }
            if let Some(seed) = self.dictionary_seed.take() {
                if seed.has_lit || seed.has_ll || seed.has_ml || seed.has_of {
                    self.price_type = HcOptPriceType::Dynamic;
                }
                if seed.has_lit && self.literals_compressed() {
                    self.lit_sum = Self::apply_seeded_table(&mut self.lit_freq, &seed.lit_bits, 11);
                } else if self.literals_compressed() {
                    self.lit_freq.fill(0);
                    // Plain `+`: lit_freq is u32 and a byte's count is bounded
                    // by the block size (<= MAX_BLOCK_SIZE), far under u32::MAX.
                    for &byte in src {
                        self.lit_freq[byte as usize] += 1;
                    }
                    self.lit_sum = Self::downscale_stats(&mut self.lit_freq, 8, false);
                    if self.lit_sum == 0 {
                        self.lit_freq[0] = 1;
                        self.lit_sum = 1;
                    }
                } else {
                    self.lit_freq.fill(0);
                    self.lit_sum = 0;
                }

                if seed.has_ll {
                    self.lit_length_sum =
                        Self::apply_seeded_table(&mut self.lit_length_freq, &seed.ll_bits, 10);
                } else {
                    let base_ll_freqs: [u32; HC_MAX_LL + 1] = [
                        4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    ];
                    self.lit_length_freq = base_ll_freqs;
                    self.lit_length_sum = base_ll_freqs.iter().copied().sum();
                }

                if seed.has_ml {
                    self.match_length_sum =
                        Self::apply_seeded_table(&mut self.match_length_freq, &seed.ml_bits, 10);
                } else {
                    self.match_length_freq.fill(1);
                    self.match_length_sum = (HC_MAX_ML + 1) as u32;
                }

                if seed.has_of {
                    self.off_code_sum =
                        Self::apply_seeded_table(&mut self.off_code_freq, &seed.of_bits, 10);
                } else {
                    let base_off_freqs: [u32; HC_MAX_OFF + 1] = [
                        6, 2, 1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1,
                    ];
                    self.off_code_freq = base_off_freqs;
                    self.off_code_sum = base_off_freqs.iter().copied().sum();
                }
            } else {
                if self.literals_compressed() {
                    self.lit_freq.fill(0);
                    // Plain `+`: lit_freq is u32 and a byte's count is bounded
                    // by the block size (<= MAX_BLOCK_SIZE), far under u32::MAX.
                    for &byte in src {
                        self.lit_freq[byte as usize] += 1;
                    }
                    self.lit_sum = Self::downscale_stats(&mut self.lit_freq, 8, false);
                    if self.lit_sum == 0 {
                        self.lit_freq[0] = 1;
                        self.lit_sum = 1;
                    }
                } else {
                    self.lit_freq.fill(0);
                    self.lit_sum = 0;
                }

                let base_ll_freqs: [u32; HC_MAX_LL + 1] = [
                    4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                ];
                self.lit_length_freq = base_ll_freqs;
                self.lit_length_sum = base_ll_freqs.iter().copied().sum();

                self.match_length_freq.fill(1);
                self.match_length_sum = (HC_MAX_ML + 1) as u32;

                let base_off_freqs: [u32; HC_MAX_OFF + 1] = [
                    6, 2, 1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1,
                ];
                self.off_code_freq = base_off_freqs;
                self.off_code_sum = base_off_freqs.iter().copied().sum();
            }
        } else {
            if self.literals_compressed() {
                self.lit_sum = Self::scale_stats(&mut self.lit_freq, 12);
            }
            self.lit_length_sum = Self::scale_stats(&mut self.lit_length_freq, 11);
            self.match_length_sum = Self::scale_stats(&mut self.match_length_freq, 11);
            self.off_code_sum = Self::scale_stats(&mut self.off_code_freq, 11);
        }
        self.set_base_prices(profile.accurate);
    }

    pub(crate) fn update_stats(
        &mut self,
        lit_len: usize,
        literals: &[u8],
        off_base: u32,
        match_len: usize,
    ) {
        // Upstream zstd parity (ZSTD_updateStats). All freq sums bounded by
        // block_size * LITFREQ_ADD ≤ 256K * 2 = 512K ≪ u32::MAX.
        if self.literals_compressed() {
            for &byte in literals.iter().take(lit_len) {
                self.lit_freq[byte as usize] += HC_LITFREQ_ADD;
            }
            self.lit_sum += (lit_len as u32) * HC_LITFREQ_ADD;
        }

        let (ll_code, _) = Self::lit_code_and_bits(lit_len);
        self.lit_length_freq[ll_code] += 1;
        self.lit_length_sum += 1;

        // off_base ≥ 1 ⇒ leading_zeros ≤ 31 ⇒ 31-lz ≥ 0.
        let off_code = ((31 - off_base.max(1).leading_zeros()) as usize).min(HC_MAX_OFF);
        self.off_code_freq[off_code] += 1;
        self.off_code_sum += 1;

        let (ml_code, _) = Self::ml_code_and_bits(match_len);
        self.match_length_freq[ml_code] += 1;
        self.match_length_sum += 1;
    }
}

#[derive(Copy, Clone)]
pub(crate) struct HcOptimalCostProfile {
    pub(crate) max_chain_depth: usize,
    pub(crate) sufficient_match_len: usize,
    pub(crate) accurate: bool,
    pub(crate) favor_small_offsets: bool,
}

impl HcOptimalCostProfile {
    /// Build the optimal-parser cost profile for a compile-time
    /// strategy. Every field is read from `S`'s associated consts so
    /// the optimiser materialises the literal at codegen time
    /// (no runtime branch). Every production call site goes through
    /// this entry — there is no runtime peer.
    ///
    /// The `debug_assert!(S::USE_BT, …)` enforces that
    /// `MAX_CHAIN_DEPTH` / `SUFFICIENT_MATCH_LEN` are only consulted
    /// for BT-walking strategies, since non-BT strategies
    /// (`Fast` / `Dfast` / `Greedy` / `Lazy`) carry placeholder
    /// values for those consts — see the `MAX_CHAIN_DEPTH` doc
    /// comment on each of those strategy types.
    #[inline]
    pub(crate) fn const_for_strategy<S: super::strategy::Strategy>() -> Self {
        debug_assert!(
            S::USE_BT,
            "HcOptimalCostProfile::const_for_strategy::<S>() called with a \
             non-BT strategy (S::USE_BT == false). The optimal-parser cost \
             profile is only meaningful when the BT walker is active.",
        );
        Self {
            max_chain_depth: S::MAX_CHAIN_DEPTH,
            sufficient_match_len: S::SUFFICIENT_MATCH_LEN,
            accurate: S::ACCURATE_PRICE,
            favor_small_offsets: S::FAVOR_SMALL_OFFSETS,
        }
    }

    pub(crate) fn literal_price(&self, stats: &HcOptState, byte: u8) -> u32 {
        if !stats.literals_compressed() {
            return 8 * HC_BITCOST_MULTIPLIER;
        }
        if matches!(stats.price_type, HcOptPriceType::Predefined) {
            return 6 * HC_BITCOST_MULTIPLIER;
        }
        // Upstream zstd parity: ZSTD_rawLiteralsCost asserts lit_sum_base_price ≥
        // BITCOST_MULTIPLIER, then clamps lit_weight to that range, so the
        // final subtract never underflows.
        debug_assert!(stats.lit_sum_base_price >= HC_BITCOST_MULTIPLIER);
        let lit_max = stats.lit_sum_base_price - HC_BITCOST_MULTIPLIER;
        let mut lit_weight = HcOptState::weight(stats.lit_freq[byte as usize], self.accurate);
        if lit_weight > lit_max {
            lit_weight = lit_max;
        }
        stats.lit_sum_base_price - lit_weight
    }

    pub(crate) fn lit_length_price(&self, stats: &HcOptState, lit_len: usize) -> u32 {
        if lit_len == HC_BLOCKSIZE_MAX {
            // Upstream zstd parity: ZSTD_litLengthPrice() handles the non-representable
            // BLOCKSIZE_MAX literal-length by charging one extra bit over the
            // largest encodable litLength symbol.
            return HC_BITCOST_MULTIPLIER
                + self.lit_length_price(stats, HC_BLOCKSIZE_MAX.saturating_sub(1));
        }
        if matches!(stats.price_type, HcOptPriceType::Predefined) {
            return HcOptState::weight(lit_len as u32, self.accurate);
        }
        // ll_bits ≤ 16 ⇒ ll_bits * 256 ≤ 4096, sum no overflow.
        let (ll_code, ll_bits) = HcOptState::lit_code_and_bits(lit_len);
        ll_bits * HC_BITCOST_MULTIPLIER + stats.lit_length_sum_base_price
            - HcOptState::weight(stats.lit_length_freq[ll_code], self.accurate)
    }

    #[inline(always)]
    pub(crate) fn offset_price_for<const ACCURATE_PRICE: bool, const FAVOR_SMALL_OFFSETS: bool>(
        &self,
        stats: &HcOptState,
        off_base: u32,
    ) -> u32 {
        // Upstream zstd parity (ZSTD_getMatchPrice). off_base ≥ 1 ⇒ leading_zeros ≤ 31.
        // off_code ≤ 31, (16 + off_code) * 256 ≤ 12032, sums no overflow.
        let off_code = 31 - off_base.max(1).leading_zeros();
        if matches!(stats.price_type, HcOptPriceType::Predefined) {
            return (16 + off_code) * HC_BITCOST_MULTIPLIER;
        }
        let mut price = off_code * HC_BITCOST_MULTIPLIER
            + (stats.off_code_sum_base_price
                - HcOptState::weight(stats.off_code_freq[off_code as usize], ACCURATE_PRICE));
        if FAVOR_SMALL_OFFSETS && off_code >= 20 {
            price += (off_code - 19) * 2 * HC_BITCOST_MULTIPLIER;
        }
        price
    }

    #[inline(always)]
    pub(crate) fn match_length_price(&self, stats: &HcOptState, match_len: usize) -> u32 {
        // Upstream zstd parity: mlBase = match_len - MINMATCH; callers guarantee
        // match_len ≥ HC_FORMAT_MINMATCH. ml_bits ≤ 16, * 256 ≤ 4096.
        debug_assert!(match_len >= HC_FORMAT_MINMATCH);
        let ml_base = match_len - HC_FORMAT_MINMATCH;
        if matches!(stats.price_type, HcOptPriceType::Predefined) {
            return HcOptState::weight(ml_base as u32, self.accurate);
        }
        let (ml_code, ml_bits) = HcOptState::ml_code_and_bits(match_len);
        ml_bits * HC_BITCOST_MULTIPLIER
            + (stats.match_length_sum_base_price
                - HcOptState::weight(stats.match_length_freq[ml_code], self.accurate))
    }

    #[inline(always)]
    pub(crate) fn match_price_from_parts(
        &self,
        off_price: u32,
        ml_price: u32,
        stats: &HcOptState,
    ) -> u32 {
        let mut price = off_price + ml_price;
        debug_assert!(price >= off_price);
        if !matches!(stats.price_type, HcOptPriceType::Predefined) {
            price += HC_BITCOST_MULTIPLIER / 5;
            debug_assert!(price >= off_price);
        }
        price
    }
}
