#![allow(
    clippy::bool_to_int_with_if,
    clippy::empty_line_after_doc_comments,
    clippy::map_unwrap_or,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Per-level compression tuning: the matcher config structs, the level
//! parameter table, and the level → params resolution chain.
//!
//! Moved verbatim from `match_generator.rs` (no behaviour change): the
//! `HcConfig` / `RowConfig` / `DfastConfig` / `FastConfig` knobs, `LevelParams`
//! and `LEVEL_TABLE`, the public-parameter overrides, the source-size tiering,
//! and the workspace estimators. `match_generator` imports this resolution API
//! instead of carrying it inline. Encoding-level paths are written absolute
//! (`crate::encoding::…`) so the module can live under `levels/` unchanged.

use crate::encoding::CompressionLevel;
use crate::encoding::match_generator::{HC_SEARCH_DEPTH, HC_TARGET_LEN, ROW_MIN_MATCH_LEN};

/// Bundled tuning knobs for the hash-chain matcher. Using a typed config
/// instead of positional `usize` args eliminates parameter-order hazards.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct HcConfig {
    pub(crate) hash_log: usize,
    pub(crate) chain_log: usize,
    pub(crate) search_depth: usize,
    pub(crate) target_len: usize,
    /// Binary-tree finder hash width. Upstream uses `mls = BOUNDED(3, minMatch, 6)`
    /// (`ZSTD_selectBtGetAllMatches`, zstd_opt.c:896) — i.e. mls=3 on the
    /// btultra/btultra2 levels (L18-22, minMatch=3) — and surfaces 3-byte matches
    /// through a fallback-only HC3 finder (zstd_opt.c:691-720: distance < 256 KiB,
    /// taken only when no longer/repcode match exists). Our optimal parser does
    /// NOT yet replicate that 3-byte handling — it emits short matches C prices
    /// out, breaking level-22 sequence parity — so the BT hash width is clamped UP
    /// to 4 (`cp.min_match.clamp(4, 6)`) to keep the finder from surfacing those
    /// 3-byte matches and so match C's output. This is a deliberate workaround,
    /// NOT C's finder width; drop the clamp once the optimal parser is C-faithful
    /// at minMatch 3 (tracked in #337). Carried explicitly per level so a
    /// `target_length` override can't silently change the finder's hashing width.
    /// Only the BT body reads it; HC / lazy levels keep it at 4.
    pub(crate) search_mls: usize,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct RowConfig {
    pub(crate) hash_bits: usize,
    pub(crate) row_log: usize,
    pub(crate) search_depth: usize,
    pub(crate) target_len: usize,
    /// Upstream zstd `cParams.minMatch` for the row matcher: the regular-search
    /// acceptance floor (a row candidate must extend to >= `mls` bytes).
    /// The C-like advanced API surfaces this as the row min-match knob.
    /// `ROW_MIN_MATCH_LEN` (5) is the default; the row hash key width stays
    /// 4 bytes (an internal detail), so this only tunes the acceptance
    /// floor, not the candidate hash distribution.
    pub(crate) mls: usize,
}

// Only used as the default HashChain config when the test-only parse×search
// override pairs a level with a backend its native row doesn't populate.

/// Base HashChain config synthesized when a public-parameter strategy
/// override ([`crate::encoding::parameters`]) routes a level to the HC / BT
/// backend whose native level row didn't populate `hc` (e.g. forcing
/// `Strategy::Lazy2` onto a level the table resolves to Fast). Mirrors
/// the mid-band lazy defaults; the per-knob overrides then refine it.
pub(crate) const HC_OVERRIDE_DEFAULT: HcConfig = HcConfig {
    hash_log: crate::encoding::match_table::storage::HC_HASH_LOG,
    chain_log: crate::encoding::match_table::storage::HC_CHAIN_LOG,
    search_depth: HC_SEARCH_DEPTH,
    target_len: HC_TARGET_LEN,
    search_mls: 4,
};

// Default Row config: only used by tests and the test-only parse×search
// override (production greedy L5 carries its own `ROW_L5`).

// Level-5 greedy is the ONLY strategy routed to the Row backend
// (`StrategyTag::backend`: greedy -> Row; lazy / btopt / btultra* ->
// HashChain), so it is the only level whose `row:` field is read. The upstream zstd
// `clevels.h` default row (srcSize > 256 KB) for level 5 is searchLog=3,
// targetLength=2, from which the row matcher derives:
//   rowLog       = clamp(searchLog, 4, 6) = 4
//   search_depth = 1 << min(searchLog, rowLog) = 8   (= nbAttempts)
//   target_len   = targetLength = 2                  (nice-match early-out)
// The shared `ROW_CONFIG` (row_log=5, search_depth=16, target_len=48) ran a
// level-12-grade search here: 16 slots per row, never early-exiting until a
// 48-byte match. That exhaustive walk was the dominant cost in greedy L5's
// encode-speed regression vs FFI. `hash_bits` matches upstream zstd's
// `ZSTD_getCParams(5, .., 0).hashLog` = 19 (verified via
// `cparams_check 5`), so the row table is the same width as upstream's
// (2^19 slots); the previous `ROW_HASH_BITS` (20) doubled both row tables vs
// upstream, the dominant peak-memory excess on the greedy band.
pub(crate) const ROW_L5: RowConfig = RowConfig {
    hash_bits: 19,
    row_log: 4,
    search_depth: 8,
    target_len: 2,
    mls: ROW_MIN_MATCH_LEN,
};

/// Per-level Double-Fast hash sizing, mirroring the upstream zstd `clevels.h` columns
/// (config-driven, not a hardcoded constant): `long_hash_log` =
/// `cParams.hashLog` (the long 8-byte hash table), `short_hash_log` =
/// `cParams.chainLog` (the short hash table dfast repurposes as its
/// secondary index). Only the Dfast backend reads it, so non-dfast level
/// rows carry `dfast: None`. `minMatch` stays the upstream zstd-fixed `5`
/// (`DFAST_MIN_MATCH_LEN`, used in const contexts).
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct DfastConfig {
    pub(crate) long_hash_log: u8,
    pub(crate) short_hash_log: u8,
}

// Upstream zstd clevels.h default row (srcSize > 256 KB): L3 {hashLog 17, chainLog 16}.
pub(crate) const DFAST_L3: DfastConfig = DfastConfig {
    long_hash_log: 17,
    short_hash_log: 16,
};

/// Per-level Fast-strategy tuning, only consumed by the `FastKernelMatcher`
/// (Simple backend): `hash_log` = upstream zstd `cParams.hashLog`, `mls` = upstream zstd
/// `cParams.minMatch` (4..=8), `step_size` = upstream zstd `stepSize`. Carried as
/// `LevelParams.fast` (`Some` only on Fast level rows; `None` elsewhere).
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct FastConfig {
    pub(crate) hash_log: u32,
    pub(crate) mls: u32,
    pub(crate) step_size: usize,
}

pub(crate) const FAST_L1: FastConfig = FastConfig {
    hash_log: 14,
    // Tier-0 (srcSize > 256 KiB) `cParams.minMatch`. Upstream zstd selects the
    // Level-1 row from a 4-way srcSize-tiered table (`ZSTD_getCParams_internal`
    // → `ZSTD_defaultCParameters[tableID][1]`), and minMatch shrinks for
    // smaller inputs: 7 (>256 KiB) / 6 (16..256 KiB) / 5 (<=16 KiB). The base
    // here is the tier-0 value; `fast_l1_mls_for_source_size` lowers it per the
    // tier in `adjust_params_for_source_size`.
    mls: 7,
    step_size: 2,
};

/// Resolved tuning parameters for a compression level. The
/// [`StrategyTag`] is the single source of truth for the backend
/// family and the compile-time strategy consts; the runtime
/// [`BackendTag`] used by the driver dispatcher is derived via
/// [`StrategyTag::backend`] so the two cannot drift.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct LevelParams {
    pub(crate) strategy_tag: crate::encoding::strategy::StrategyTag,
    /// Decoupled search-method axis. Independent of `strategy_tag`'s
    /// parse half: a level can pair any parse (greedy / lazy depth via
    /// `lazy_depth`) with any search backend here. Defaults to the
    /// historical pairing (`strategy_tag.search()`) but is overridable
    /// per level so the parse×search matrix can be swept and tuned.
    pub(crate) search: crate::encoding::strategy::SearchMethod,
    pub(crate) window_log: u8,
    pub(crate) lazy_depth: u8,
    /// Per-strategy tuning. Exactly one is `Some` on each level row, matching
    /// `strategy_tag`'s backend, so the table self-documents which knobs a
    /// level actually consumes (the others are `None`, not dead placeholders):
    /// `fast` for the Fast/Simple backend, `dfast` for Double-Fast, `hc` for
    /// the HashChain (lazy / btopt / btultra*) backend, `row` for the Row
    /// (greedy L5) backend.
    pub(crate) fast: Option<FastConfig>,
    pub(crate) dfast: Option<DfastConfig>,
    pub(crate) hc: Option<HcConfig>,
    pub(crate) row: Option<RowConfig>,
}

impl LevelParams {
    /// Backend family (storage variant) for the driver dispatcher.
    /// Derived from the decoupled `search` axis so a level can route to
    /// a different search backend than its `strategy_tag` historically
    /// implied.
    pub(crate) fn backend(&self) -> crate::encoding::strategy::BackendTag {
        self.search.backend()
    }

    /// Parse mode derived from the decoupled `search` axis: the binary-tree
    /// search path carries `ParseMode::Optimal`; every other search backend
    /// derives greedy/lazy/lazy2 from `lazy_depth`. Reading `search` (not the
    /// strategy tag) keeps the parse×search decoupling complete even when a
    /// level whose tag is `Bt*` is overridden to a non-BT search backend.
    pub(crate) fn parse(&self) -> crate::encoding::strategy::ParseMode {
        match self.search {
            crate::encoding::strategy::SearchMethod::BinaryTree => {
                crate::encoding::strategy::ParseMode::Optimal
            }
            _ => crate::encoding::strategy::ParseMode::from_lazy_depth(self.lazy_depth),
        }
    }

    /// Cheap fingerprint pre-splitter level (the C-like `blockSplitterLevel`):
    /// the EFFECTIVE upstream `ZSTD_splitBlock` level that
    /// `ZSTD_optimalBlockSize` dispatches, i.e. `splitLevels[strategy] - 2`
    /// (clamped at 0), NOT the raw `splitLevels[]` value. `split_level == 0`
    /// routes to the cheap from-borders heuristic; `1..=4` to byChunks with
    /// internal sampling level `split_level - 1`. See the body for the
    /// per-strategy tier table and why the raw-table mapping was wrong.
    pub(crate) fn pre_split(&self) -> Option<u8> {
        use crate::encoding::strategy::StrategyTag;
        // Effective upstream `ZSTD_splitBlock` level = `splitLevels[strat] - 2`
        // (clamped at 0). Upstream `splitLevels[] = {0,0,1,2,2,3,3,4,4,4}` then
        // subtracts 2 before dispatch, so the byChunks sampling tier is two
        // steps coarser than the raw table: greedy/lazy(d1)=0 (from-borders),
        // lazy2/btlazy2=1 (byChunks rate 43), btopt+=2 (byChunks rate 11).
        // An earlier version mirrored the RAW table AND bumped lazy2 to the
        // rate-1 full scan (split 4) to dodge a periodic-input phantom-split —
        // that ran the pre-splitter at up to 43x upstream's sampling cost
        // (~87% of L9 encode time on the decode corpus). Per the drop-in
        // contract ratio only needs to stay <= upstream, so matching upstream's
        // sampling tier (and accepting upstream's identical over-split on
        // periodic input) is the dominant large-input encode-speed win.
        Some(match self.strategy_tag {
            // splitLevels 0/1 -> 0: upstream does not pre-split fast/dfast at
            // all; from-borders is the cheapest stand-in and rarely splits.
            StrategyTag::Fast | StrategyTag::Dfast => 0,
            // greedy / lazy(depth 1): splitLevels 2 -> 0 (from-borders).
            StrategyTag::Greedy => 0,
            StrategyTag::Lazy => {
                if self.lazy_depth >= 2 {
                    1 // lazy2: splitLevels 3 -> 1 (byChunks rate 43)
                } else {
                    0 // lazy depth 1: splitLevels 2 -> 0 (from-borders)
                }
            }
            StrategyTag::Btlazy2 => 1, // splitLevels 3 -> 1 (byChunks rate 43)
            StrategyTag::BtOpt | StrategyTag::BtUltra | StrategyTag::BtUltra2 => 2,
        })
    }
}

/// Apply the public-parameter per-knob overrides (#27) onto the
/// level-resolved [`LevelParams`], in place. Runs in [`Matcher::reset`]
/// after the level params are computed and before backend selection, so
/// a strategy override re-routes the backend uniformly. An all-`None`
/// override is a no-op the caller skips via
/// [`crate::encoding::parameters::ParamOverrides::is_empty`], keeping the default
/// level geometry byte-identical.
pub(crate) fn apply_param_overrides(
    params: &mut LevelParams,
    ov: &crate::encoding::parameters::ParamOverrides,
) {
    use crate::encoding::strategy::SearchMethod;

    // 1. Strategy override re-derives tag / search / lazy depth.
    if let Some(strategy) = ov.strategy {
        let tag = strategy.tag();
        params.strategy_tag = tag;
        params.search = tag.search();
        params.lazy_depth = strategy.lazy_depth();
    }

    // 2. Ensure the active backend's config row exists (synthesize a
    //    default when a strategy override moved off the native row).
    match params.search {
        SearchMethod::Fast => {
            params.fast.get_or_insert(FAST_L1);
        }
        SearchMethod::DoubleFast => {
            params.dfast.get_or_insert(DFAST_L3);
        }
        SearchMethod::RowHash => {
            params.row.get_or_insert(ROW_L5);
        }
        SearchMethod::HashChain | SearchMethod::BinaryTree => {
            // A `Btlazy2` strategy override moved off a non-HC row needs the
            // BT 5-byte finder hash (upstream zstd minMatch 5); other synthesized HC
            // rows keep the 4-byte default. An explicit `min_match` override
            // below refines this further.
            params.hc.get_or_insert(HcConfig {
                search_mls: if matches!(
                    params.strategy_tag,
                    crate::encoding::strategy::StrategyTag::Btlazy2
                ) {
                    5
                } else {
                    HC_OVERRIDE_DEFAULT.search_mls
                },
                ..HC_OVERRIDE_DEFAULT
            });
        }
    }

    // 3. window_log (bounds-checked at <= 30 by the builder).
    if let Some(window_log) = ov.window_log {
        params.window_log = window_log;
    }

    // 4. Per-backend numeric knobs map into the active config, mirroring
    //    the upstream zstd `cParams` -> matcher translation documented on each
    //    config struct.
    match params.search {
        SearchMethod::Fast => {
            if let Some(fast) = params.fast.as_mut() {
                if let Some(hash_log) = ov.hash_log {
                    fast.hash_log = hash_log;
                }
                if let Some(min_match) = ov.min_match {
                    fast.mls = min_match;
                }
            }
        }
        SearchMethod::DoubleFast => {
            if let Some(dfast) = params.dfast.as_mut() {
                // hashLog -> long table, chainLog -> short table (the
                // dfast secondary index). Both bounds-checked <= 30, so
                // the `u8` casts are lossless.
                if let Some(hash_log) = ov.hash_log {
                    dfast.long_hash_log = hash_log as u8;
                }
                if let Some(chain_log) = ov.chain_log {
                    dfast.short_hash_log = chain_log as u8;
                }
            }
        }
        SearchMethod::RowHash => {
            if let Some(row) = params.row.as_mut() {
                // Row hash-table width override (mirrors dfast `long_hash_log`
                // / hc `hash_log`). Row has no separate chain table — the
                // per-row depth comes from `search_log` below — so only
                // `hash_log` maps here; `chain_log` has no Row analogue.
                if let Some(hash_log) = ov.hash_log {
                    row.hash_bits = hash_log as usize;
                }
                if let Some(search_log) = ov.search_log {
                    // Upstream zstd: rowLog = clamp(searchLog, 4, 6);
                    //        nbAttempts = 1 << min(searchLog, rowLog).
                    let row_log = (search_log as usize).clamp(4, 6);
                    row.row_log = row_log;
                    row.search_depth = 1usize << (search_log as usize).min(row_log);
                }
                if let Some(target_length) = ov.target_length {
                    row.target_len = target_length as usize;
                }
                if let Some(min_match) = ov.min_match {
                    row.mls = min_match as usize;
                }
            }
        }
        SearchMethod::HashChain | SearchMethod::BinaryTree => {
            if let Some(hc) = params.hc.as_mut() {
                if let Some(hash_log) = ov.hash_log {
                    hc.hash_log = hash_log as usize;
                }
                if let Some(chain_log) = ov.chain_log {
                    hc.chain_log = chain_log as usize;
                }
                if let Some(search_log) = ov.search_log {
                    hc.search_depth = 1usize << search_log;
                }
                if let Some(target_length) = ov.target_length {
                    hc.target_len = target_length as usize;
                }
                if let Some(min_match) = ov.min_match {
                    // BT finder hash width, derived from cParams.minMatch exactly
                    // as upstream zstd: `mls = BOUNDED(3, cParams.minMatch, 6)`
                    // (zstd_opt.c:896 ZSTD_selectBtGetAllMatches). minMatch=3
                    // tiers hash on 3 bytes (btultra/btultra2 path). Only the BT
                    // body reads `search_mls`; HC/lazy hash on 4 bytes regardless.
                    hc.search_mls = (min_match as usize).clamp(3, 6);
                }
            }
        }
    }
}

/// Map the resolved runtime strategy to the upstream zstd LDM strategy ordinal
/// (1..=9) that [`crate::encoding::ldm::params::LdmParams::adjust_for`] expects.
/// The collapsed `Lazy` tag splits on `lazy_depth` (lazy = 4, lazy2 = 5).

pub(crate) fn ldm_strategy_ordinal(
    tag: crate::encoding::strategy::StrategyTag,
    lazy_depth: u8,
) -> u32 {
    use crate::encoding::strategy::StrategyTag;
    match tag {
        StrategyTag::Fast => 1,
        StrategyTag::Dfast => 2,
        StrategyTag::Greedy => 3,
        StrategyTag::Lazy => {
            if lazy_depth >= 2 {
                5
            } else {
                4
            }
        }
        // Upstream zstd `ZSTD_btlazy2` ordinal.
        StrategyTag::Btlazy2 => 6,
        StrategyTag::BtOpt => 7,
        StrategyTag::BtUltra => 8,
        StrategyTag::BtUltra2 => 9,
    }
}

/// `ceil(log2(size))` of a source-size hint, with a zero hint floored to
/// [`MIN_WINDOW_LOG`]. This is the single quantization every hint-dependent
/// matcher parameter is derived from: the window-log cap, the HC / Fast hash
/// and chain widths, the Dfast / Row table widths, the L22 config buckets, and
/// the Fast attach-vs-copy cutoff. Two hints sharing this value resolve to the
/// identical matcher shape, which is why it (not the raw byte count) keys the
/// primed-dictionary snapshot — see [`PrimedKey`]. Operates on the full `u64`
/// so callers comparing a hint against a cutoff get the same bucketed decision
/// here and at the driver, with no `as usize` truncation on 32-bit targets.
pub(crate) fn source_size_ceil_log(size: u64) -> u8 {
    if size == 0 {
        MIN_WINDOW_LOG
    } else {
        (64 - (size - 1).leading_zeros()) as u8
    }
}

/// Attach-vs-copy cutoff for the Fast strategy, as a ceil-log bucket: a hint at
/// or below `2^this` (or unknown, `None`) ATTACHES the dictionary (a separate
/// immutable table scanned in place via the borrowed dual-base kernel); a larger
/// hint would COPY it into the live table.
///
/// We set this to `31` so every dictionary source up to 2 GiB attaches,
/// diverging from upstream zstd's 8 KiB `ZSTD_shouldAttachDict` cutoff ON
/// PURPOSE: upstream copy mode copies the small CDict TABLES into the cctx and
/// still scans the input in place, but our flat-history copy path memmoves the
/// whole INPUT into history every frame (profiled at 30% `__memmove` + 14%
/// `__memset` on a reused 1 MiB dict encode). Attach mode scans the caller's
/// input in place with the dict as a separate prefix base, so it is strictly
/// faster for every frame size here (measured: 1 MiB dict frame 167 us -> 52 us,
/// 0.42x of C; 10 KiB 20.4 us -> 4.4 us, 0.17x of C). The dual-base kernel
/// carries `window_low`, so over-window inputs stay in-window and C-decodable.
///
/// `31` is also the largest bucket the borrowed kernel can attach: it stores
/// virtual positions as `u32` (`cur_abs as u32`), so the maximum attached source
/// `1 << 31` (plus the dict prefix) stays below `u32::MAX`; the next bucket `32`
/// (4 GiB) would wrap that arithmetic. Sources past 2 GiB therefore fall back to
/// copy mode — rare in practice, and the relative copy cost shrinks as the
/// source grows. Per the drop-in-not-binary-parity contract, we make this match
/// decision ourselves.
/// Shared by `reset` (records the mode in the primed-snapshot key) and
/// `prime_with_dictionary` (acts on it).
pub(crate) const FAST_ATTACH_DICT_CUTOFF_LOG: u8 = 31;

/// Largest dictionary region (bytes) the Fast attach path can index. The tagged
/// dict table packs each position into `32 - DICT_TAG_BITS` (= 24) bits, so a
/// region past `2^24` (16 MiB) would overflow the packed position. Dictionaries
/// this large fall back to COPY mode, whose live table stores full `u32`
/// positions and handles them. The size hint set on dict load equals the actual
/// dict content length, so the attach-vs-copy decision (and the matching
/// snapshot-key / epoch bits) can gate on it consistently at reset time.
pub(crate) const MAX_FAST_ATTACH_DICT_REGION: usize = 1 << 24;

/// Dfast counterpart of [`FAST_ATTACH_DICT_CUTOFF_LOG`]: upstream zstd
/// `ZSTD_dictMatchState` attach cutoff for the double-fast strategy is 16 KiB
/// (`2^14`), so small / unknown-size inputs ATTACH (separate immutable dict
/// long+short tables + dual-probe in `start_matching_fast_loop`) and larger
/// known-size inputs COPY (re-prime the dict into the live tables, where the
/// dense scan matches it as window history). The attach build also self-gates
/// on `use_fast_loop` inside `skip_matching_for_dict_attach` — only the
/// fast-loop levels (L3 / Default / L0) carry the dual-probe.
pub(crate) const DFAST_ATTACH_DICT_CUTOFF_LOG: u8 = 14;

/// `ZSTD_dictMatchState` attach cutoff for the Row (greedy/lazy) strategy is
/// 32 KiB (`2^15`, upstream zstd `attachDictSizeCutoffs`): small / unknown-size inputs
/// ATTACH the dict into the separate immutable row index (bounded dual-probe in
/// `row_candidate_rl`), larger known-size inputs dense-COPY into the live rows.
pub(crate) const ROW_ATTACH_DICT_CUTOFF_LOG: u8 = 15;

/// 32 KiB (`2^15`, upstream zstd `attachDictSizeCutoffs[ZSTD_lazy2]`): small /
/// unknown-size inputs ATTACH the dict as a separate hash-chain dms (the dual
/// search in `find_best_match` walks the live input chain + the dms), larger
/// known-size inputs dense-COPY (merge the dict into the live chain and search
/// the one combined chain).
pub(crate) const HC_ATTACH_DICT_CUTOFF_LOG: u8 = 15;

/// BT/optimal attach cutoff for `btlazy2` + `btopt`: 32 KiB (`2^15`, upstream
/// zstd `attachDictSizeCutoffs[ZSTD_btlazy2]` == `[ZSTD_btopt]`). Small /
/// unknown-size inputs ATTACH the dict as a separate DUBT dms; larger known-size
/// inputs COPY the dict into the LIVE binary tree (upstream zstd
/// `ZSTD_resetCCtx_byCopyingCDict`).
pub(crate) const BT_OPT_ATTACH_DICT_CUTOFF_LOG: u8 = 15;

/// BT/optimal attach cutoff for `btultra` + `btultra2`: 8 KiB (`2^13`, upstream
/// zstd `attachDictSizeCutoffs[ZSTD_btultra]` == `[ZSTD_btultra2]`). The deepest
/// parses copy the dict into the live tree past a much smaller source than the
/// `btopt` tier, matching upstream's per-strategy cutoff table.
pub(crate) const BT_ULTRA_ATTACH_DICT_CUTOFF_LOG: u8 = 13;

// Source-size cap for the dfast hash bits when a size hint is present: a tiny
// input needs no larger hash than its window. The upstream zstd `cParams.hashLog` /
// `chainLog` (from `DfastConfig`) caps it from above at the call site.
pub(crate) fn dfast_hash_bits_for_window(max_window_size: usize) -> usize {
    let window_log = (usize::BITS - 1 - max_window_size.leading_zeros()) as usize;
    window_log.max(MIN_WINDOW_LOG as usize)
}

pub(crate) fn row_hash_bits_for_window(max_window_size: usize) -> usize {
    // Upstream zstd `ZSTD_adjustCParams_internal` cap: `hashLog <= windowLog + 1`.
    // The `+ 1` is load-bearing for L12, whose upstream zstd hashLog (23) exceeds
    // its windowLog (22) — a plain `windowLog` cap would shrink the L12
    // table on EVERY hinted reset and split primed snapshots between
    // hinted and unhinted frames that resolve to the identical geometry.
    // No constant upper clamp: the old `ROW_HASH_BITS` (20) ceiling
    // predates the lazy band moving onto Row (L9-12 carry upstream zstd hashLog
    // 21-23).
    let window_log = (usize::BITS - 1 - max_window_size.leading_zeros()) as usize;
    (window_log + 1).max(MIN_WINDOW_LOG as usize)
}

/// `floor(log2(window))` for the HashChain table-log cap (upstream zstd
/// `ZSTD_adjustCParams_internal`). The caller clamps the level's `hash_log` /
/// `chain_log` from above with this so a small hinted input doesn't allocate the
/// full level's tables.
pub(crate) fn hc_hash_bits_for_window(max_window_size: usize) -> usize {
    let window_log = (usize::BITS - 1 - max_window_size.leading_zeros()) as usize;
    window_log.max(MIN_WINDOW_LOG as usize)
}

/// Upstream `ZSTD_createCDict` table geometry: the `(hash_log, chain_log)` a
/// dictionary's prepared match-finder tables get. Thin adapter over the single
/// cParams source [`crate::encoding::cparams::create_cdict_table_logs`], which mirrors
/// `ZSTD_adjustCParams_internal` under `ZSTD_cpm_createCDict`. `window_log` is
/// the resolved compress window; `hash_log` / `chain_log` are the level's own
/// widths; `uses_bt` selects the binary-tree `cycleLog` (`chainLog - 1`).
pub(crate) fn cdict_table_logs(
    window_log: u8,
    hash_log: usize,
    chain_log: usize,
    uses_bt: bool,
    dict_size: usize,
) -> (usize, usize) {
    let (h, c) = crate::encoding::cparams::create_cdict_table_logs(
        window_log,
        hash_log as u32,
        chain_log as u32,
        uses_bt,
        dict_size,
    );
    (h as usize, c as usize)
}

/// Smallest window_log the encoder will use regardless of source size.
pub(crate) const MIN_WINDOW_LOG: u8 = 10;

/// Translate a verbatim upstream `ZSTD_defaultCParameters[tier][level]` row
/// (`cparams::CParams`) into our resolved [`LevelParams`], reproducing
/// upstream's cParams -> matcher-config derivation so the encoder follows C's
/// source-size-tiered STRATEGY + table widths rather than a single hand-tuned
/// `LEVEL_TABLE`. Derivation (verified against L6/L16/L22 vs clevels.h):
/// `search_depth = 1 << searchLog`; row `row_log = clamp(searchLog, 4, 6)`; the
/// per-strategy sub-config carries the verbatim `hashLog` / `chainLog` /
/// `targetLength` / `minMatch`. Strategy numbers are upstream `ZSTD_strategy`
/// (fast=1, dfast=2, greedy=3, lazy=4, lazy2=5, btlazy2=6, btopt=7, btultra=8,
/// btultra2=9).
fn level_params_from_cparams(cp: crate::encoding::cparams::CParams) -> LevelParams {
    use crate::encoding::strategy::{SearchMethod, StrategyTag};
    let window_log = cp.window_log as u8;
    let search_depth = 1usize << cp.search_log;
    let target_len = cp.target_length as usize;
    let hc = HcConfig {
        hash_log: cp.hash_log as usize,
        chain_log: cp.chain_log as usize,
        search_depth,
        target_len,
        // Clamp UP to 4: C's BT finder uses mls=3 on L18-22, but our optimal
        // parser diverges on the resulting 3-byte matches (breaks level-22
        // sequence parity), so we keep the finder at >=4 as a workaround until
        // the parser is C-faithful at minMatch 3. See `HcConfig::search_mls` (#337).
        search_mls: cp.min_match.clamp(4, 6) as usize,
    };
    let row = RowConfig {
        hash_bits: cp.hash_log as usize,
        row_log: cp.search_log.clamp(4, 6) as usize,
        search_depth,
        target_len,
        mls: cp.min_match as usize,
    };
    let bt = |tag| LevelParams {
        strategy_tag: tag,
        search: SearchMethod::BinaryTree,
        window_log,
        lazy_depth: 2,
        fast: None,
        dfast: None,
        hc: Some(hc),
        row: None,
    };
    let row_lvl = |tag, lazy_depth| LevelParams {
        strategy_tag: tag,
        search: SearchMethod::RowHash,
        window_log,
        lazy_depth,
        fast: None,
        dfast: None,
        hc: None,
        row: Some(row),
    };
    match cp.strategy {
        1 => LevelParams {
            strategy_tag: StrategyTag::Fast,
            search: SearchMethod::Fast,
            window_log,
            lazy_depth: 0,
            // Upstream fast `stepSize`: `targetLength + 1` (0 -> 1, so step 2).
            fast: Some(FastConfig {
                hash_log: cp.hash_log,
                mls: cp.min_match,
                step_size: target_len.max(1) + 1,
            }),
            dfast: None,
            hc: None,
            row: None,
        },
        2 => LevelParams {
            strategy_tag: StrategyTag::Dfast,
            search: SearchMethod::DoubleFast,
            window_log,
            lazy_depth: 1,
            fast: None,
            dfast: Some(DfastConfig {
                long_hash_log: cp.hash_log as u8,
                short_hash_log: cp.chain_log as u8,
            }),
            hc: None,
            row: None,
        },
        3 => row_lvl(StrategyTag::Greedy, 0),
        4 => row_lvl(StrategyTag::Lazy, 1),
        5 => row_lvl(StrategyTag::Lazy, 2),
        6 => bt(StrategyTag::Btlazy2),
        7 => bt(StrategyTag::BtOpt),
        8 => bt(StrategyTag::BtUltra),
        _ => bt(StrategyTag::BtUltra2),
    }
}

/// Down-size a synthesized backend's window / hash / chain logs for a known
/// source size by routing through the single C-faithful adjuster
/// [`adjust_cparams`](crate::encoding::cparams::adjust_cparams)
/// (`ZSTD_adjustCParams_internal`).
///
/// Used only on the param-override re-cap path: [`apply_param_overrides`]
/// synthesizes a backend's full-size default config when a strategy override
/// moves off the native level row, and this re-applies the source-size cap
/// C-faithfully — the SAME adjuster the `get_cparams` main path uses, so the
/// override path and the level path now down-size identically (no extra hinted
/// 16 KiB window floor, no per-backend headroom that diverged from C). The
/// Dfast backend self-sizes its tables in `reset`, so only its window is capped.
pub(crate) fn adjust_params_for_source_size(mut params: LevelParams, src_size: u64) -> LevelParams {
    use crate::encoding::cparams::{CParams, adjust_cparams};
    use crate::encoding::strategy::{BackendTag, StrategyTag};

    let backend = params.backend();
    // Lift the active backend's source-cappable logs into a flat CParams. Dfast
    // contributes none (window-only); its table widths self-size in `reset`.
    let (hash_log, chain_log): (u32, u32) = match backend {
        BackendTag::Simple => (params.fast.as_ref().map_or(0, |f| f.hash_log), 0),
        BackendTag::HashChain => params
            .hc
            .as_ref()
            .map_or((0, 0), |h| (h.hash_log as u32, h.chain_log as u32)),
        BackendTag::Row => (params.row.as_ref().map_or(0, |r| r.hash_bits as u32), 0),
        BackendTag::Dfast => (0, 0),
    };
    // The chain cap (`ZSTD_cycleLog`) reads only `strategy >= btlazy2(6)`, so a
    // coarse 6/3 split is exact for it; `adjust_cparams`'s other strategy use
    // (`cdict_indices_are_tagged`) is gated on `create_cdict = false` here.
    let strategy = if matches!(
        params.strategy_tag,
        StrategyTag::Btlazy2 | StrategyTag::BtOpt | StrategyTag::BtUltra | StrategyTag::BtUltra2
    ) {
        6
    } else {
        3
    };
    let adj = adjust_cparams(
        CParams {
            window_log: u32::from(params.window_log),
            chain_log,
            hash_log,
            search_log: 1,
            min_match: 4,
            target_length: 0,
            strategy,
        },
        src_size,
        0,
        false,
    );
    params.window_log = adj.window_log as u8;
    match backend {
        BackendTag::Simple => {
            if let Some(f) = params.fast.as_mut() {
                f.hash_log = adj.hash_log;
            }
        }
        BackendTag::HashChain => {
            if let Some(h) = params.hc.as_mut() {
                h.hash_log = adj.hash_log as usize;
                h.chain_log = adj.chain_log as usize;
            }
        }
        BackendTag::Row => {
            if let Some(r) = params.row.as_mut() {
                r.hash_bits = adj.hash_log as usize;
            }
        }
        BackendTag::Dfast => {}
    }
    params
}

/// Estimated steady-state heap footprint of a one-shot compression context
/// at `level` (window history + match-finder tables + block staging), in
/// bytes. Computed from the same per-level tuning table the encoder
/// resolves at frame start, so the estimate tracks the real allocations;
/// it is an upper-bound style budget figure, not an exact accounting.
pub fn estimated_compression_workspace_bytes(level: CompressionLevel) -> usize {
    use crate::encoding::strategy::StrategyTag;
    let params = resolve_level_params(level, None);
    let window = 1usize << params.window_log;
    // Mirror `configure()`: the HC3 short-match side table exists only on
    // the btultra/btultra2 tags (minMatch 3), capped by the window log; the
    // BT pointer-pair layout fits inside the `4 << chain_log` chain term
    // (pairs over `chain_log - 1` nodes).
    let wants_hash3 = matches!(
        params.strategy_tag,
        StrategyTag::BtUltra | StrategyTag::BtUltra2
    );
    let uses_bt = matches!(
        params.strategy_tag,
        StrategyTag::Btlazy2 | StrategyTag::BtOpt | StrategyTag::BtUltra | StrategyTag::BtUltra2
    );
    let tables = params.fast.map(|f| 4usize << f.hash_log).unwrap_or(0)
        + params
            .dfast
            .map(|d| (4usize << d.long_hash_log) + (4usize << d.short_hash_log))
            .unwrap_or(0)
        + params
            .hc
            .map(|h| {
                let hash3 = if wants_hash3 {
                    4usize
                        << crate::encoding::match_table::storage::HC3_HASH_LOG
                            .min(params.window_log as usize)
                } else {
                    0
                };
                (4usize << h.hash_log) + (4usize << h.chain_log) + hash3
            })
            .unwrap_or(0)
        + params
            .row
            .map(|r| (4usize << r.hash_bits) + (2usize << r.hash_bits))
            .unwrap_or(0);
    // BT modes box a `BtMatcher`; its retained scratch layout is budgeted
    // next to the struct so estimator and allocator evolve together.
    let bt = if uses_bt {
        crate::encoding::bt::BtMatcher::estimated_workspace_bytes()
    } else {
        0
    };
    // Block staging: literal + sequence buffers plus the compressed-block
    // scratch, each bounded by the 128 KiB block size.
    let staging = 3 * (128 * 1024);
    window + tables + bt + staging
}

/// Extra steady-state workspace the binary-tree strategies (ordinals 6..=9,
/// btlazy2..btultra2) retain beyond the hash/chain tables: the boxed matcher
/// plus its scratch arenas, and the HC3 short-match side table for
/// btultra/btultra2 (capped by the window log). 0 for non-BT ordinals.
pub fn estimated_bt_strategy_extra_bytes(strategy_ordinal: u32, window_log: u32) -> usize {
    if !(6..=9).contains(&strategy_ordinal) {
        return 0;
    }
    let hash3 = if matches!(strategy_ordinal, 8 | 9) {
        4usize << crate::encoding::match_table::storage::HC3_HASH_LOG.min(window_log as usize)
    } else {
        0
    };
    crate::encoding::bt::BtMatcher::estimated_workspace_bytes() + hash3
}

/// Resolve a [`CompressionLevel`] (+ optional source-size hint) to the
/// concrete [`LevelParams`] the matcher runs: strategy tag, search method
/// (match-finder), window log, and per-backend config.
///
/// ## CRITICAL: input size changes the match-finder (and can change strategy)
///
/// The resolved geometry is a function of the SOURCE SIZE, not the level
/// alone. This is the easy-to-miss part (so read this before assuming a level
/// maps to one fixed match-finder). It mirrors three upstream zstd stages:
///
/// 1. [`LEVEL_TABLE`] holds the tier-0 (source > 256 KiB) base row per level
///    (upstream `ZSTD_defaultCParameters[0]`). L6-L12 carry
///    `SearchMethod::RowHash` (the Row match-finder), like upstream's
///    greedy/lazy default.
/// 2. [`apply_cparams_tier`] overrides the table-shaping widths for the
///    smaller source tiers (upstream `ZSTD_getCParams_internal` tier table).
///    NOTE: upstream ALSO switches STRATEGY in some tiers (L2 → dfast, L4 →
///    greedy on small sources); those backend switches are NOT yet replicated,
///    so those levels keep their base strategy on small inputs.
/// 3. [`adjust_params_for_source_size`] caps `window_log` to
///    ~`ceil_log2(source_size)` (upstream `ZSTD_adjustCParams_internal`).
///
/// THEN, in the matcher `reset`, the greedy/lazy band falls back from
/// `RowHash` to `SearchMethod::HashChain` when the resolved `window_log <= 14`
/// — exactly upstream's `ZSTD_resolveRowMatchFinderMode` (the Row match-finder
/// is used for greedy/lazy/lazy2 ONLY when `windowLog > 14`). Net effect for
/// the SAME level:
///
/// * small input (e.g. a 10 KiB fixture → `window_log` 14) → **HashChain**
///   (`ZSTD_HcFindBestMatch`, scalar chain walk);
/// * large input (e.g. 1 MiB → `window_log` 20) → **RowHash** (the SIMD-tag
///   row match-finder).
///
/// A dictionary does NOT change the match-finder: it only downsizes the
/// prepared tables (`cdict_table_logs`, mirroring `ZSTD_createCDict`'s
/// small-source assumption), while `window_log` stays source-derived. So
/// `(L6, 10 KiB, +dict)` is HashChain and `(L6, 1 MiB, +dict)` is RowHash,
/// both matching upstream. When comparing against C on a fixture, resolve the
/// match-finder from the fixture's size first, or you may optimise/benchmark a
/// path C does not even take for that input.
pub(crate) fn resolve_level_params(
    level: CompressionLevel,
    source_size: Option<u64>,
) -> LevelParams {
    // Uncompressed = raw blocks, no match-finder. Not a cParams level, so it is
    // the one row resolved by hand rather than through `get_cparams`.
    if matches!(level, CompressionLevel::Uncompressed) {
        return LevelParams {
            strategy_tag: crate::encoding::strategy::StrategyTag::Fast,
            search: crate::encoding::strategy::SearchMethod::Fast,
            // Raw frames emit literal blocks and never reference history;
            // advertising a wider window only inflates the decoder-side buffer
            // reservation, so clamp to 17 (128 KiB) regardless of input size.
            window_log: 17,
            lazy_depth: 0,
            // Beyond-upstream: hash_log=14 (vs upstream's row-0 13) for ~2× fewer
            // collisions on structured corpora; mls=6 / step_size=2 mirror the
            // upstream "base for negative" row (targetLength=1 -> step 2).
            fast: Some(FastConfig {
                hash_log: 14,
                mls: 6,
                step_size: 2,
            }),
            dfast: None,
            hc: None,
            row: None,
        };
    }
    // Every other level resolves through the SINGLE C-faithful cParams source,
    // `cparams::get_cparams` (the port of `ZSTD_getCParams`). One place selects
    // strategy + table widths + the negative-level acceleration per
    // (level, srcSize) + the source-size window/hash down-clamp, so the encoder
    // never re-derives parameters from a parallel hand-tuned path. Named presets
    // map to their numeric level; the cParams source clamps out-of-range levels
    // (>22 to 22, negatives to MIN_CLEVEL) itself.
    let numeric: i32 = match level {
        CompressionLevel::Uncompressed => unreachable!("handled above"),
        // Fastest = upstream level 1 (fast strategy, smallest real-compression
        // tables).
        CompressionLevel::Fastest => 1,
        // Default = upstream level 3 (the libzstd default).
        CompressionLevel::Default => CompressionLevel::DEFAULT_LEVEL,
        // Better = level 7: the lazy2 band — clearly above the fast/dfast levels
        // on ratio while still well under the binary-tree cost cliff.
        CompressionLevel::Better => 7,
        // Best = level 13: the first point of the deep binary-tree band that
        // strictly dominates every level below it on ratio (lower levels can tie
        // on window-bound corpora), so the alias sits on a config that always
        // wins rather than on a hair-thin margin.
        CompressionLevel::Best => 13,
        CompressionLevel::Level(n) => n,
    };
    let src = source_size.unwrap_or(crate::encoding::cparams::CONTENTSIZE_UNKNOWN);
    level_params_from_cparams(crate::encoding::cparams::get_cparams(numeric, src, 0))
}

/// The cheap fingerprint pre-splitter level for a compression level (the
/// C-like `blockSplitterLevel`), resolved through the same per-level
/// `LevelParams` table as every other tuning knob. `None` keeps the whole
/// 128 KiB block. The frame loop reads this instead of hardcoding the
/// level→split mapping at the call site.
pub(crate) fn level_pre_split(level: CompressionLevel) -> Option<usize> {
    // Resolve through `resolve_level_params` directly — NOT via the legacy
    // `numeric_level()` alias — so named presets read the SAME table row as
    // every other tuning knob (`Best` maps to its own row there, which is
    // not the row its numeric alias points at). `Uncompressed` (raw
    // blocks) never splits.
    if matches!(level, CompressionLevel::Uncompressed) {
        return None;
    }
    resolve_level_params(level, None)
        .pre_split()
        .map(usize::from)
}
