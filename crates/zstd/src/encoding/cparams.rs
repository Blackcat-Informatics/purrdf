#![allow(
    clippy::cast_lossless,
    clippy::manual_ilog2,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Faithful port of upstream zstd's compression-parameter selection
//! (`ZSTD_getCParams` and its helpers in `lib/compress/zstd_compress.c` +
//! the `ZSTD_defaultCParameters[4][23]` table in `lib/compress/clevels.h`,
//! pinned to zstd 1.5.7). This drives per-(level, srcSize, dictSize)
//! `(windowLog, hashLog, minMatch, strategy, ...)` selection so the encoder
//! sizes its match-finder tables and picks its strategy the same way the
//! reference does — including the source-size tier table and the
//! `ZSTD_adjustCParams` down-sizing. Verified byte-for-byte against the C
//! `ZSTD_getCParams` over a grid in `ffi-bench`.
//!
//! Pure `core` (const table + arithmetic); no allocation, no `std`.

/// One row of the upstream `ZSTD_compressionParameters` struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CParams {
    pub window_log: u32,
    pub chain_log: u32,
    pub hash_log: u32,
    pub search_log: u32,
    /// Upstream `minMatch` (a.k.a. `searchLength`): the match-finder hash width.
    pub min_match: u32,
    pub target_length: u32,
    /// Upstream `ZSTD_strategy` numeric value: fast=1, dfast=2, greedy=3,
    /// lazy=4, lazy2=5, btlazy2=6, btopt=7, btultra=8, btultra2=9.
    pub strategy: u32,
}

// Upstream constants (zstd.h / zstd_internal.h / clevels.h, 1.5.7).
const HASHLOG_MIN: u32 = 6;
const WINDOWLOG_ABSOLUTEMIN: u32 = 10;
// 64-bit host value; structured-zstd's encoder runs hosted. The 32-bit value
// is 30, only relevant on 32-bit targets where windowLog never reaches 31 here.
const WINDOWLOG_MAX: u32 = 31;
const SHORT_CACHE_TAG_BITS: u32 = 8;
/// `ZSTD_CONTENTSIZE_UNKNOWN` == `(0ULL - 1)`.
pub(crate) const CONTENTSIZE_UNKNOWN: u64 = u64::MAX;

// Level-row selection constants — only the public `get_cparams` entry (the
// C-reference comparison surface) uses these; the encoder indexes the table via
// `default_cparams` and never re-selects a level row here.
const ZSTD_MAX_CLEVEL: i32 = 22;
const ZSTD_CLEVEL_DEFAULT: i32 = 3;
const TARGETLENGTH_MAX: i32 = 1 << 17;
/// `ZSTD_minCLevel()` == `-ZSTD_TARGETLENGTH_MAX`.
const MIN_CLEVEL: i32 = -TARGETLENGTH_MAX;
const KB_256: u64 = 256 * 1024;
const KB_128: u64 = 128 * 1024;
const KB_16: u64 = 16 * 1024;

/// `ZSTD_defaultCParameters[4][ZSTD_MAX_CLEVEL+1]` (clevels.h, zstd 1.5.7),
/// verbatim. Outer index = source-size tier (0 = `>256 KB`, 1 = `<=256 KB`,
/// 2 = `<=128 KB`, 3 = `<=16 KB`); inner index = level row (0 = base for
/// negative levels, 1..=22 = that level). Fields: (W, C, H, S, minMatch,
/// targetLength, strategy).
#[rustfmt::skip]
const DEFAULT_CPARAMS: [[CParams; 23]; 4] = {
    macro_rules! r {
        ($w:expr,$c:expr,$h:expr,$s:expr,$l:expr,$t:expr,$strat:expr) => {
            CParams { window_log: $w, chain_log: $c, hash_log: $h, search_log: $s,
                      min_match: $l, target_length: $t, strategy: $strat }
        };
    }
    [
    // tier 0: srcSize > 256 KB
    [
        r!(19,12,13,1,6,1,1), r!(19,13,14,1,7,0,1), r!(20,15,16,1,6,0,1), r!(21,16,17,1,5,0,2),
        r!(21,18,18,1,5,0,2), r!(21,18,19,3,5,2,3), r!(21,18,19,3,5,4,4), r!(21,19,20,4,5,8,4),
        r!(21,19,20,4,5,16,5), r!(22,20,21,4,5,16,5), r!(22,21,22,5,5,16,5), r!(22,21,22,6,5,16,5),
        r!(22,22,23,6,5,32,5), r!(22,22,22,4,5,32,6), r!(22,22,23,5,5,32,6), r!(22,23,23,6,5,32,6),
        r!(22,22,22,5,5,48,7), r!(23,23,22,5,4,64,7), r!(23,23,22,6,3,64,8), r!(23,24,22,7,3,256,9),
        r!(25,25,23,7,3,256,9), r!(26,26,24,7,3,512,9), r!(27,27,25,9,3,999,9),
    ],
    // tier 1: srcSize <= 256 KB
    [
        r!(18,12,13,1,5,1,1), r!(18,13,14,1,6,0,1), r!(18,14,14,1,5,0,2), r!(18,16,16,1,4,0,2),
        r!(18,16,17,3,5,2,3), r!(18,17,18,5,5,2,3), r!(18,18,19,3,5,4,4), r!(18,18,19,4,4,4,4),
        r!(18,18,19,4,4,8,5), r!(18,18,19,5,4,8,5), r!(18,18,19,6,4,8,5), r!(18,18,19,5,4,12,6),
        r!(18,19,19,7,4,12,6), r!(18,18,19,4,4,16,7), r!(18,18,19,4,3,32,7), r!(18,18,19,6,3,128,7),
        r!(18,19,19,6,3,128,8), r!(18,19,19,8,3,256,8), r!(18,19,19,6,3,128,9), r!(18,19,19,8,3,256,9),
        r!(18,19,19,10,3,512,9), r!(18,19,19,12,3,512,9), r!(18,19,19,13,3,999,9),
    ],
    // tier 2: srcSize <= 128 KB
    [
        r!(17,12,12,1,5,1,1), r!(17,12,13,1,6,0,1), r!(17,13,15,1,5,0,1), r!(17,15,16,2,5,0,2),
        r!(17,17,17,2,4,0,2), r!(17,16,17,3,4,2,3), r!(17,16,17,3,4,4,4), r!(17,16,17,3,4,8,5),
        r!(17,16,17,4,4,8,5), r!(17,16,17,5,4,8,5), r!(17,16,17,6,4,8,5), r!(17,17,17,5,4,8,6),
        r!(17,18,17,7,4,12,6), r!(17,18,17,3,4,12,7), r!(17,18,17,4,3,32,7), r!(17,18,17,6,3,256,7),
        r!(17,18,17,6,3,128,8), r!(17,18,17,8,3,256,8), r!(17,18,17,10,3,512,8), r!(17,18,17,5,3,256,9),
        r!(17,18,17,7,3,512,9), r!(17,18,17,9,3,512,9), r!(17,18,17,11,3,999,9),
    ],
    // tier 3: srcSize <= 16 KB
    [
        r!(14,12,13,1,5,1,1), r!(14,14,15,1,5,0,1), r!(14,14,15,1,4,0,1), r!(14,14,15,2,4,0,2),
        r!(14,14,14,4,4,2,3), r!(14,14,14,3,4,4,4), r!(14,14,14,4,4,8,5), r!(14,14,14,6,4,8,5),
        r!(14,14,14,8,4,8,5), r!(14,15,14,5,4,8,6), r!(14,15,14,9,4,8,6), r!(14,15,14,3,4,12,7),
        r!(14,15,14,4,3,24,7), r!(14,15,14,5,3,32,8), r!(14,15,15,6,3,64,8), r!(14,15,15,7,3,256,8),
        r!(14,15,15,5,3,48,9), r!(14,15,15,6,3,128,9), r!(14,15,15,7,3,256,9), r!(14,15,15,8,3,256,9),
        r!(14,15,15,8,3,512,9), r!(14,15,15,9,3,512,9), r!(14,15,15,10,3,999,9),
    ],
    ]
};

/// `ZSTD_highbit32(val)` — index of the most-significant set bit. Caller
/// guarantees `val != 0` (matches the upstream `assert(val != 0)`).
#[inline]
fn highbit32(val: u32) -> u32 {
    debug_assert!(val != 0);
    31 - val.leading_zeros()
}

/// `ZSTD_getCParamRowSize` (zstd_compress.c): the effective size used to pick
/// the tier row, for the public `ZSTD_getCParams` (`ZSTD_cpm_unknown`) entry.
fn cparam_row_size(src_size_hint: u64, dict_size: usize) -> u64 {
    let dict_size = dict_size as u64;
    let unknown = src_size_hint == CONTENTSIZE_UNKNOWN;
    let added_size = if unknown && dict_size > 0 { 500 } else { 0 };
    if unknown && dict_size == 0 {
        CONTENTSIZE_UNKNOWN
    } else {
        src_size_hint
            .wrapping_add(dict_size)
            .wrapping_add(added_size)
    }
}

/// `ZSTD_cycleLog` (zstd_compress.c): `hashLog - (strategy >= btlazy2)`.
#[inline]
fn cycle_log(hash_log: u32, strategy: u32) -> u32 {
    // ZSTD_btlazy2 == 6.
    hash_log - u32::from(strategy >= 6)
}

/// `ZSTD_dictAndWindowLog` (zstd_compress.c). `src_size` must not be
/// `CONTENTSIZE_UNKNOWN` (the caller gates this).
fn dict_and_window_log(window_log: u32, src_size: u64, dict_size: u64) -> u32 {
    let max_window_size: u64 = 1u64 << WINDOWLOG_MAX;
    if dict_size == 0 {
        return window_log;
    }
    let window_size: u64 = 1u64 << window_log;
    let dict_and_window_size = dict_size + window_size;
    if window_size >= dict_size + src_size {
        window_log
    } else if dict_and_window_size >= max_window_size {
        WINDOWLOG_MAX
    } else {
        highbit32((dict_and_window_size as u32).wrapping_sub(1)) + 1
    }
}

/// Whether a CDict built with these params uses tagged short-cache indices
/// (`ZSTD_CDictIndicesAreTagged`): true for the dictMatchState strategies
/// (fast / dfast), where the dict hash table packs a tag. Upstream gates the
/// short-cache hashLog cap on this.
#[inline]
fn cdict_indices_are_tagged(cp: &CParams) -> bool {
    // ZSTD_dfast == 2: tagged for fast(1) and dfast(2).
    cp.strategy <= 2
}

/// `ZSTD_adjustCParams_internal` (zstd_compress.c): down-size `cp` for the
/// given `(src_size, dict_size, mode)`. `src_size == CONTENTSIZE_UNKNOWN`
/// means unknown; `src_size == 0` means literally zero. The row-match-finder
/// hashLog cap is omitted here because this port only consumes the Fast /
/// Dfast cparams (no row hash); add it before relying on row-strategy output.
pub(crate) fn adjust_cparams(
    mut cp: CParams,
    mut src_size: u64,
    dict_size: usize,
    create_cdict: bool,
) -> CParams {
    const MIN_SRC_SIZE: u64 = 513; // (1<<9) + 1
    let max_window_resize: u64 = 1u64 << (WINDOWLOG_MAX - 1);

    // `ZSTD_cpm_createCDict`: an unknown source assumes a `minSrcSize` source so
    // the dictionary's prepared tables down-size deterministically. The other
    // modes (`unknown` / `noAttachDict` / `attachDict`) leave `src_size` as-is;
    // only `unknown` (the public entry) and `createCDict` have live callers here.
    if create_cdict && dict_size != 0 && src_size == CONTENTSIZE_UNKNOWN {
        src_size = MIN_SRC_SIZE;
    }

    // resize windowLog if input is small enough, to use less memory.
    if src_size <= max_window_resize && (dict_size as u64) <= max_window_resize {
        let t_size = (src_size as u32).wrapping_add(dict_size as u32);
        let hash_size_min: u32 = 1 << HASHLOG_MIN;
        let src_log = if t_size < hash_size_min {
            HASHLOG_MIN
        } else {
            highbit32(t_size.wrapping_sub(1)) + 1
        };
        if cp.window_log > src_log {
            cp.window_log = src_log;
        }
    }

    if src_size != CONTENTSIZE_UNKNOWN {
        let daw = dict_and_window_log(cp.window_log, src_size, dict_size as u64);
        let cl = cycle_log(cp.chain_log, cp.strategy);
        if cp.hash_log > daw + 1 {
            cp.hash_log = daw + 1;
        }
        if cl > daw {
            cp.chain_log -= cl - daw;
        }
    }

    if cp.window_log < WINDOWLOG_ABSOLUTEMIN {
        cp.window_log = WINDOWLOG_ABSOLUTEMIN;
    }

    if create_cdict && cdict_indices_are_tagged(&cp) {
        let max_short_cache_hash_log = 32 - SHORT_CACHE_TAG_BITS;
        if cp.hash_log > max_short_cache_hash_log {
            cp.hash_log = max_short_cache_hash_log;
        }
        if cp.chain_log > max_short_cache_hash_log {
            cp.chain_log = max_short_cache_hash_log;
        }
    }

    cp
}

/// `ZSTD_getCParams_internal` (zstd_compress.c): tier + level row selection,
/// negative-level acceleration, then `ZSTD_adjustCParams_internal`.
pub(crate) fn get_cparams(compression_level: i32, src_size_hint: u64, dict_size: usize) -> CParams {
    let r_size = cparam_row_size(src_size_hint, dict_size);
    let table_id = (u32::from(r_size <= KB_256)
        + u32::from(r_size <= KB_128)
        + u32::from(r_size <= KB_16)) as usize;

    let row: usize = if compression_level == 0 {
        ZSTD_CLEVEL_DEFAULT as usize
    } else if compression_level < 0 {
        // Row 0 is upstream's "base for negative" row. At the >256 KB / unknown
        // tier it is hashLog=13 / minMatch=6 (smaller tiers drop minMatch to 5):
        // the 32 KiB hash table (2^13 * 4 B) stays L1d-resident on contemporary
        // cores, so every probe is an L1 hit (hashLog=14 would overflow a 32 KiB
        // L1d into L2). A short minMatch keeps the table cheap; the throughput
        // win on the negative ladder comes from the step-size acceleration
        // below, not from the match length.
        0
    } else if compression_level > ZSTD_MAX_CLEVEL {
        ZSTD_MAX_CLEVEL as usize
    } else {
        compression_level as usize
    };

    let mut cp = DEFAULT_CPARAMS[table_id][row];

    if compression_level < 0 {
        // Negative-level acceleration: upstream stores `targetLength = -level`,
        // which the fast match-finder turns into `stepSize = targetLength + 1`
        // (see the fast kernel). A larger step skips more positions per
        // iteration, so L-1..L-7 trade ratio for speed along a 2..8 step
        // gradient. Clamp to MIN_CLEVEL before negating so `i32::MIN` can't
        // overflow on `-level`.
        let clamped = compression_level.max(MIN_CLEVEL);
        cp.target_length = (-clamped) as u32;
    }

    // `ZSTD_cpm_unknown` (the public entry): no createCDict source assumption.
    adjust_cparams(cp, src_size_hint, dict_size, /* create_cdict */ false)
}

/// The `(hash_log, chain_log)` a dictionary's prepared match-finder tables get
/// under `ZSTD_cpm_createCDict` — the single source for the CDict table
/// geometry (mirrors `ZSTD_adjustCParams_internal` with an unknown source, so
/// `dictSize` drives `dict_and_window_log` down-sizing while the live window
/// stays source-sized). `uses_bt` selects the binary-tree `cycleLog`
/// (`chainLog - 1`). Returns only the table widths; the caller keeps its own
/// resolved `window_log`. Non-`dictMatchState` strategies (HC / Row / BT) skip
/// the tagged short-cache cap, matching upstream.
pub(crate) fn create_cdict_table_logs(
    window_log: u8,
    hash_log: u32,
    chain_log: u32,
    uses_bt: bool,
    dict_size: usize,
) -> (u32, u32) {
    // Strategy only steers `cycle_log` (>= btlazy2 == 6) and the tagged cap
    // (<= dfast == 2); pick lazy(4) / btopt(7) so HC/Row/BT stay untagged.
    let strategy = if uses_bt { 7 } else { 4 };
    let cp = CParams {
        window_log: window_log as u32,
        chain_log,
        hash_log,
        search_log: 0,
        min_match: 0,
        target_length: 0,
        strategy,
    };
    let adj = adjust_cparams(
        cp,
        CONTENTSIZE_UNKNOWN,
        dict_size,
        /* create_cdict */ true,
    );
    (adj.hash_log, adj.chain_log)
}
