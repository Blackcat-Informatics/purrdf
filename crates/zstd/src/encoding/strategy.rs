#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Encoder strategy types — Phase 3 of #111.
//!
//! Every per-position branch the encoder used to dispatch at runtime
//! (lazy / optimal split, BT walker on/off, hash3 short-match probe,
//! refined / coarse cost model) now reads from a compile-time
//! `S: Strategy` parameter. The compiler monomorphises the inner
//! loops per concrete `S` and drops the dead arms during codegen.
//!
//! ## Dispatch flow
//!
//! ```text
//! Matcher::start_matching                       // 7-arm match on StrategyTag (per block)
//!  └─ compress_block::<S>                       // S::BACKEND const match
//!      ├─ Simple/Dfast/Row                      // backends without parse_mode
//!      └─ HcMatchGenerator::start_matching_strategy::<S>
//!          ├─ S::USE_BT == false → start_matching_lazy
//!          └─ S::USE_BT == true  → start_matching_optimal::<S>
//!              ├─ HcOptimalCostProfile::const_for_strategy::<S>()
//!              ├─ should_run_btultra2_seed_pass::<S>          // const false unless S = BtUltra2
//!              └─ select_kernel() once, then per-segment loop:
//!                  └─ build_optimal_plan_impl_<kernel>::<S, ACC, FAV, LDM>
//!                      └─ build_optimal_plan_impl_body!(S)
//!                              ├─ S::OPT_LEVEL == 0  → abort_on_worse_match
//!                              ├─ S::OPT_LEVEL >= 2  → opt_level (refined)
//!                              └─ $collect::<S, true>
//!                                  └─ collect_optimal_candidates_initialized_body!(S)
//!                                      └─ S::USE_HASH3 → hash3 lookup (const-gated)
//! ```
//!
//! Upstream zstd parity reference: `ZSTD_compressionParameters` in
//! `lib/compress/zstd_compress_internal.h` and the per-level table in
//! `lib/compress/clevels.h`.

#![allow(dead_code)]

/// Upstream zstd `ZSTD_compressionParameters.strategy` equivalent — names the
/// concrete match-finder backend a [`Strategy`] runs on top of. The
/// runtime [`StrategyTag`] dispatcher and the [`Strategy::BACKEND`]
/// associated const both produce values of this type, so the
/// per-block driver dispatch and the per-strategy backend selection
/// stay in lock-step.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum BackendTag {
    /// `SimpleMatchGenerator` — level 1.
    Simple,
    /// `DfastMatchGenerator` — levels 2-3.
    Dfast,
    /// `RowMatchGenerator` — level 4.
    Row,
    /// `HcMatchGenerator` — levels 5-22.
    HashChain,
}

/// Parse strategy — what the outer match loop does with the candidates
/// a search method produces. Orthogonal to [`SearchMethod`]: the same
/// parse can run on top of any search backend (upstream zstd decouples these as
/// `cParams.strategy`'s parse half vs the `useRowMatchFinder` switch).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ParseMode {
    /// Commit the first acceptable match (upstream zstd `ZSTD_greedy`, depth 0).
    Greedy,
    /// One-position lazy lookahead (upstream zstd `ZSTD_lazy`, depth 1).
    Lazy,
    /// Two-position lazy lookahead (upstream zstd `ZSTD_lazy2`, depth 2).
    Lazy2,
    /// Optimal cost-model parse (upstream zstd `ZSTD_btopt`/`btultra`/`btultra2`).
    Optimal,
}

impl ParseMode {
    /// Lazy lookahead depth for the greedy/lazy band (0/1/2). `Optimal`
    /// has no lazy depth and returns 0.
    pub(crate) const fn lazy_depth(self) -> u8 {
        match self {
            Self::Greedy => 0,
            Self::Lazy => 1,
            Self::Lazy2 => 2,
            Self::Optimal => 0,
        }
    }

    /// Derive the greedy/lazy parse from a lazy-lookahead depth. Depth >= 2
    /// saturates to `Lazy2`. Used to read the existing `LevelParams.lazy_depth`
    /// into the decoupled parse axis.
    pub(crate) const fn from_lazy_depth(depth: u8) -> Self {
        match depth {
            0 => Self::Greedy,
            1 => Self::Lazy,
            _ => Self::Lazy2,
        }
    }
}

/// Search method — how match candidates are produced for a position.
/// Orthogonal to [`ParseMode`] (upstream zstd `cParams.strategy` search half plus
/// the `useRowMatchFinder` cParam that swaps `HashChain` for `RowHash` in
/// the greedy/lazy band).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum SearchMethod {
    /// Single-table fast finder (upstream zstd `ZSTD_fast`).
    Fast,
    /// Two parallel hash tables (upstream zstd `ZSTD_dfast`).
    DoubleFast,
    /// Row-hash SIMD-tag finder (upstream zstd row matchfinder).
    RowHash,
    /// Hash-chain finder (upstream zstd `ZSTD_HcFindBestMatch`).
    HashChain,
    /// Binary-tree finder for the optimal parser (upstream zstd `ZSTD_BtGetAllMatches`).
    BinaryTree,
}

impl SearchMethod {
    /// Storage backend (matcher variant) this search method runs in.
    /// `BinaryTree` shares the `HashChain` storage (the BT scratch lives
    /// inside the hash-chain matcher, gated by `uses_bt`).
    pub(crate) const fn backend(self) -> BackendTag {
        match self {
            Self::Fast => BackendTag::Simple,
            Self::DoubleFast => BackendTag::Dfast,
            Self::RowHash => BackendTag::Row,
            Self::HashChain | Self::BinaryTree => BackendTag::HashChain,
        }
    }
}

/// Compile-time encoder strategy. Each concrete implementor is a ZST
/// whose associated `const`s tell the optimal parser / match finder
/// which upstream zstd-equivalent path to execute. Hot entry points are
/// generic over `S: Strategy`, so monomorphisation strips every
/// dead `if S::FOO` arm at codegen time.
pub(crate) trait Strategy: Copy + 'static {
    /// Match-finder backend this strategy runs on.
    const BACKEND: BackendTag;

    /// Nominal minimum match length for this strategy, mirroring the
    /// upstream `clevels.h` `searchLength` column (3 for btultra/btultra2,
    /// where the mls=3 hash3 probe is active). Descriptive: the optimal
    /// parser's actual acceptance floor is the crate-wide
    /// `HC_OPT_MIN_MATCH_LEN` (= 3) and the row matcher uses
    /// `ROW_MIN_MATCH_LEN`; this const documents the strategy's intent and
    /// is not read on the hot path.
    const MIN_MATCH: usize;

    /// `accurate` flag for [`crate::encoding::cost_model::HcOptimalCostProfile`]
    /// — enables refined statistics weighting (upstream zstd `ZSTD_btultra` and
    /// above).
    const ACCURATE_PRICE: bool;

    /// Upstream zstd "small offset bonus" toggle. Enabled for Lazy2 / BtOpt to
    /// favour decompression speed; disabled for BtUltra / BtUltra2.
    const FAVOR_SMALL_OFFSETS: bool;

    /// Compile-time gate for the `static (mls==3)` short-match probe.
    /// Both btultra and btultra2 search the hash3 table (their
    /// `clevels.h` minMatch is 3); btopt does not (minMatch >= 4).
    const USE_HASH3: bool;

    /// Whether the optimal parser runs the in-block two-pass dynamic
    /// statistics seed (upstream `ZSTD_initStats_ultra`). Only btultra2
    /// does this; btultra is single-pass even though it now shares the
    /// hash3 short-match probe. Defaults to `false` so every other
    /// strategy drops the seed-pass body at codegen time.
    const TWO_PASS_SEED: bool = false;

    /// Whether the optimal parser walks the BT — `false` for Lazy2,
    /// `true` for BtOpt / BtUltra / BtUltra2.
    const USE_BT: bool;

    /// Upstream zstd `optLevel` (0 = btopt, 2 = btultra / btultra2). Drives the
    /// `opt_level >= 2` price-table refinement in
    /// `build_optimal_plan_impl_body!`.
    const OPT_LEVEL: u8;

    /// Upstream zstd `max_chain_depth` for the optimal-parser cost profile.
    /// Used by `HcOptimalCostProfile::const_for_strategy::<S>()`.
    const MAX_CHAIN_DEPTH: usize;

    /// Upstream zstd `sufficient_match_len` — the BT walker bails out as soon
    /// as a candidate at or above this length is seen. `usize::MAX`
    /// means "never bail early".
    const SUFFICIENT_MATCH_LEN: usize;
}

/// Level 1 — upstream zstd `ZSTD_fast`. Single-table Simple matcher.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Fast;

impl Strategy for Fast {
    const BACKEND: BackendTag = BackendTag::Simple;
    const MIN_MATCH: usize = 4;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = false;
    const OPT_LEVEL: u8 = 0;
    // `MAX_CHAIN_DEPTH` / `SUFFICIENT_MATCH_LEN` are placeholder
    // values for non-BT strategies — the trait associated consts
    // must be total, but only the optimal parser reads them and
    // it runs exclusively under `S::USE_BT == true`. The
    // `debug_assert!(<S>::USE_BT, …)` guard in
    // `HcOptimalCostProfile::const_for_strategy` plus the
    // `debug_assert!(<S>::USE_BT, …)` at the top of
    // `build_optimal_plan_impl_body!` make this unreachable in
    // debug builds. Future refactors that introduce a non-BT
    // reader must add a fresh guard or replace these placeholders
    // with real Fast values.
    const MAX_CHAIN_DEPTH: usize = 8;
    const SUFFICIENT_MATCH_LEN: usize = 32;
}

/// Levels 2-3 — upstream zstd `ZSTD_dfast`. Two parallel hash chains.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Dfast;

impl Strategy for Dfast {
    const BACKEND: BackendTag = BackendTag::Dfast;
    const MIN_MATCH: usize = 4;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = false;
    const OPT_LEVEL: u8 = 0;
    // Placeholder optimal-parser consts; see `Fast` for the
    // unreachable-by-design contract.
    const MAX_CHAIN_DEPTH: usize = 8;
    const SUFFICIENT_MATCH_LEN: usize = 32;
}

/// Level 4 — upstream zstd `ZSTD_greedy` with row hashing.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Greedy;

impl Strategy for Greedy {
    const BACKEND: BackendTag = BackendTag::Row;
    const MIN_MATCH: usize = 4;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = false;
    const OPT_LEVEL: u8 = 0;
    // Placeholder optimal-parser consts; see `Fast` for the
    // unreachable-by-design contract.
    const MAX_CHAIN_DEPTH: usize = 8;
    const SUFFICIENT_MATCH_LEN: usize = 32;
}

/// Levels 6-12 — upstream zstd `ZSTD_lazy`/`ZSTD_lazy2` on the row finder
/// (upstream zstd row mode is the greedy..lazy2 default). Levels inside the
/// band differ only by runtime `RowConfig` fields (`search_depth`,
/// `hash_bits`, `row_log`, `target_len`, `lazy_depth`), not by
/// compile-time `Strategy` consts, so they share a single type.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Lazy;

impl Strategy for Lazy {
    const BACKEND: BackendTag = BackendTag::Row;
    const MIN_MATCH: usize = 4;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = false;
    const OPT_LEVEL: u8 = 0;
    // Lazy runs on the Row backend with `USE_BT == false`, so the
    // optimal parser entry point is unreachable for this strategy.
    // These values mirror the upstream zstd `lazy2` cost profile (would be
    // the right defaults if a future caller did build a profile for
    // the lazy path), but with no current reader the same
    // unreachable-by-design contract from `Fast` applies.
    const MAX_CHAIN_DEPTH: usize = 8;
    const SUFFICIENT_MATCH_LEN: usize = 32;
}

/// Levels 13-15 — upstream zstd `ZSTD_btlazy2`. Binary-tree match finder driving
/// a greedy/lazy parse (NOT the optimal DP). Reuses the BT candidate
/// collector to surface the longest match per position, then commits it
/// greedily. minMatch = 5 (`search_mls`); the BT find runs to full depth
/// (no `sufficient_match_len` early bail — upstream zstd `ZSTD_BtFindBestMatch`
/// does not cap by `targetLength`), so `MAX_CHAIN_DEPTH` must be large
/// enough that the runtime `HcConfig::search_depth` (16/32/64 for
/// L13/14/15) governs the BT walk, not this const.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Btlazy2;

impl Strategy for Btlazy2 {
    const BACKEND: BackendTag = BackendTag::HashChain;
    const MIN_MATCH: usize = 5;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = true;
    const OPT_LEVEL: u8 = 0;
    // L15 configures search_depth = 64; keep the cap at 64 so
    // `max_chain_depth.min(search_depth)` lets the level's search_depth
    // govern (BtOpt's 32 would silently halve L15's BT walk).
    const MAX_CHAIN_DEPTH: usize = 64;
    // Full BT find, no early bail (upstream zstd `ZSTD_BtFindBestMatch`).
    const SUFFICIENT_MATCH_LEN: usize = usize::MAX;
}

/// Levels 16-17 — upstream zstd `ZSTD_btopt`. BT + opt without the ultra
/// price-table refinements.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BtOpt;

impl Strategy for BtOpt {
    const BACKEND: BackendTag = BackendTag::HashChain;
    const MIN_MATCH: usize = 4;
    const ACCURATE_PRICE: bool = false;
    const FAVOR_SMALL_OFFSETS: bool = true;
    const USE_HASH3: bool = false;
    const USE_BT: bool = true;
    const OPT_LEVEL: u8 = 0;
    const MAX_CHAIN_DEPTH: usize = 32;
    const SUFFICIENT_MATCH_LEN: usize = usize::MAX;
}

/// Level 18 — upstream `ZSTD_btultra`. BT + opt with refined price
/// tables and no small-offset bias; shares the mls=3 hash3 short-match
/// probe but stays single-pass (no two-pass dynamic-stats seed).
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BtUltra;

impl Strategy for BtUltra {
    const BACKEND: BackendTag = BackendTag::HashChain;
    const MIN_MATCH: usize = 3;
    const ACCURATE_PRICE: bool = true;
    const FAVOR_SMALL_OFFSETS: bool = false;
    // clevels.h level 18 minMatch = 3 → the hash3 short-match probe is
    // active, same as btultra2. The two-pass seed stays btultra2-only
    // (TWO_PASS_SEED defaults to false here).
    const USE_HASH3: bool = true;
    const USE_BT: bool = true;
    const OPT_LEVEL: u8 = 2;
    // 1 << searchLog for level 18 (clevels.h searchLog = 6). The BT walk
    // caps compares at min(MAX_CHAIN_DEPTH, hc.search_depth); both must be
    // >= 64 for level 18 to reach upstream search depth.
    const MAX_CHAIN_DEPTH: usize = 64;
    const SUFFICIENT_MATCH_LEN: usize = usize::MAX;
}

/// Levels 19-22 — upstream `ZSTD_btultra2`. BT + opt with the two-pass
/// dynamic-statistics seed and the hash3 short-match table.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BtUltra2;

impl Strategy for BtUltra2 {
    const BACKEND: BackendTag = BackendTag::HashChain;
    const MIN_MATCH: usize = 3;
    const ACCURATE_PRICE: bool = true;
    const FAVOR_SMALL_OFFSETS: bool = false;
    const USE_HASH3: bool = true;
    const TWO_PASS_SEED: bool = true;
    const USE_BT: bool = true;
    const OPT_LEVEL: u8 = 2;
    const MAX_CHAIN_DEPTH: usize = 512;
    const SUFFICIENT_MATCH_LEN: usize = usize::MAX;
}

/// Runtime strategy tag for the per-level dispatcher. Each variant
/// maps to exactly one [`Strategy`] implementor; the dispatcher
/// itself stays runtime-tagged because it only fires once per frame
/// on `reset()`, so the cost of a 7-arm match is invisible compared
/// to the per-block hot-loop work it dispatches into.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StrategyTag {
    Fast,
    Dfast,
    Greedy,
    Lazy,
    /// Upstream zstd `ZSTD_btlazy2` (levels 13-15): binary-tree match finder with a
    /// lazy parse (not the optimal DP). Shares the BT table layout / finder
    /// with the opt strategies (`uses_bt`) but selects greedily/lazily.
    Btlazy2,
    BtOpt,
    BtUltra,
    BtUltra2,
}

impl StrategyTag {
    /// Map a compression level (1..=22) to its [`StrategyTag`].
    ///
    /// Matches `LEVEL_TABLE` in `match_generator.rs` and the upstream zstd
    /// `clevels.h` table:
    /// Mirrors upstream zstd `ZSTD_defaultCParameters[0]` (srcSize > 256 KiB
    /// tier) strategy column at `zstd/lib/compress/clevels.h:25-50`:
    ///
    /// * 1-2 → `Fast`
    /// * 3-4 → `Dfast`
    /// * 5 → `Greedy`
    /// * 6-12 → `Lazy` (upstream zstd 6/7=lazy, 8-12=lazy2; we collapse lazy and
    ///   lazy2 onto our `Lazy` tag and carry the lazy/lazy2 split via
    ///   `LevelParams.lazy_depth` — 1 for lazy, 2 for lazy2)
    /// * 13-15 → `Btlazy2` (upstream zstd btlazy2: BinaryTree finder + lazy parse)
    /// * 16-17 → `BtOpt`
    /// * 18 → `BtUltra`
    /// * 19-22 → `BtUltra2`
    pub(crate) const fn for_level(level: u8) -> Self {
        match level {
            1 | 2 => Self::Fast,
            3 | 4 => Self::Dfast,
            5 => Self::Greedy,
            6..=12 => Self::Lazy,
            // 13-15 are btlazy2 in `LEVEL_TABLE` (BinaryTree finder on the
            // HashChain storage). Folding them onto `Lazy` was harmless
            // while `Lazy` also meant HashChain; with `Lazy` resolving to
            // the Row finder the distinction is load-bearing.
            13..=15 => Self::Btlazy2,
            16 | 17 => Self::BtOpt,
            18 => Self::BtUltra,
            19 => Self::BtUltra2,
            _ => Self::BtUltra2,
        }
    }

    /// Map a [`CompressionLevel`] to its [`StrategyTag`]. Mirrors the
    /// per-level dispatch in `match_generator::resolve_level_params`.
    pub(crate) fn for_compression_level(level: crate::encoding::CompressionLevel) -> Self {
        use crate::encoding::CompressionLevel;
        match level {
            CompressionLevel::Uncompressed => Self::Fast,
            CompressionLevel::Fastest => Self::Fast,
            CompressionLevel::Default => Self::Dfast,
            CompressionLevel::Better => Self::Lazy,
            CompressionLevel::Best => Self::Lazy,
            CompressionLevel::Level(n) => {
                if n <= 0 {
                    if n == 0 { Self::Dfast } else { Self::Fast }
                } else {
                    // Clamp in `i32` BEFORE casting to `u8`: a bare
                    // `n as u8` truncates values ≥ 256 (e.g.
                    // `Level(256)` wraps to `0`, `Level(257)` to
                    // `1`) and silently routes them to the wrong
                    // strategy. `MAX_LEVEL` (22) fits a u8 by
                    // definition, so the cast after the i32 clamp
                    // is lossless.
                    let clamped_i32 = n.clamp(1, CompressionLevel::MAX_LEVEL);
                    Self::for_level(clamped_i32 as u8)
                }
            }
        }
    }

    /// Bridge to [`BackendTag`] for the dispatcher entry point.
    /// Greedy AND lazy run on the Row finder (upstream zstd
    /// `ZSTD_resolveRowMatchFinderMode`: row mode is the default for
    /// greedy..lazy2); the BT strategies keep the HashChain storage
    /// (their tree scratch lives inside it).
    pub(crate) const fn backend(self) -> BackendTag {
        match self {
            Self::Fast => BackendTag::Simple,
            Self::Dfast => BackendTag::Dfast,
            Self::Greedy | Self::Lazy => BackendTag::Row,
            Self::Btlazy2 | Self::BtOpt | Self::BtUltra | Self::BtUltra2 => BackendTag::HashChain,
        }
    }

    /// Default search method this tag historically ran on. Used to seed
    /// the decoupled [`SearchMethod`] axis from the legacy tag during
    /// migration; once `LEVEL_TABLE` carries explicit configs this is the
    /// fallback for tag-only call sites.
    pub(crate) const fn search(self) -> SearchMethod {
        match self {
            Self::Fast => SearchMethod::Fast,
            Self::Dfast => SearchMethod::DoubleFast,
            Self::Greedy | Self::Lazy => SearchMethod::RowHash,
            Self::Btlazy2 | Self::BtOpt | Self::BtUltra | Self::BtUltra2 => {
                SearchMethod::BinaryTree
            }
        }
    }

    /// Default parse mode for this tag, ignoring per-level lazy depth (the
    /// lazy band's real depth comes from `LevelParams.lazy_depth` via
    /// [`ParseMode::from_lazy_depth`]). The opt tags map to `Optimal`.
    pub(crate) const fn parse_mode(self) -> ParseMode {
        match self {
            Self::Fast | Self::Dfast | Self::Greedy => ParseMode::Greedy,
            Self::Lazy => ParseMode::Lazy,
            // Upstream zstd btlazy2 = BinaryTree finder + depth-2 lazy parse
            // (`Strategy::lazy_depth()` reports 2 for it as well).
            Self::Btlazy2 => ParseMode::Lazy2,
            Self::BtOpt | Self::BtUltra | Self::BtUltra2 => ParseMode::Optimal,
        }
    }
}
