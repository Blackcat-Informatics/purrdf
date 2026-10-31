#![allow(
    clippy::inline_always,
    clippy::manual_assert_eq,
    clippy::manual_bit_width,
    clippy::ptr_as_ptr,
    clippy::semicolon_if_nothing_returned,
    clippy::struct_excessive_bools,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Shared match-finder storage.
//!
//! `MatchTable` owns every byte of state that both the hash-chain (HC /
//! lazy / lazy2) and binary-tree (BT / optimal parser) backends touch:
//! the rolling window, the contiguous `history` mirror, the absolute
//! position cursors, the hash / hash3 / chain (or BT pointer-pair)
//! tables, and the dictionary-priming flags. Both backends operate on
//! the same physical buffers; the only difference is the semantics of
//! `chain_table` entries — HC mode threads single-link chain pointers
//! through it, BT mode lays out pairs of pointers per node — and that
//! interpretation is the matcher's concern, not the table's.
//!
//! Shared by the HC and BT matchers so their
//! modules do not need to duplicate this storage as a forest of
//! `&mut Vec<u32>` arguments.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::super::Sequence;
use super::super::blocks::encode_offset_with_history;
use super::super::cost_model::{HC_OPT_NUM, HcOptimalCostProfile};
use super::super::dict_attach::DictAttach;
use super::super::hc::HC_MIN_MATCH_LEN;
use super::super::opt::types::{HcOptimalSequence, MatchCandidate};
use super::helpers::INCOMPRESSIBLE_SKIP_STEP;

/// Lookahead bytes the BT walker / optimal parser will read past the
/// per-position absolute cursor (`abs_pos + 9` for match-end sentinels,
/// `abs_pos + HC_OPT_NUM` for the optimal-parser match-length cap, etc.).
/// `MatchTable::add_data` rejects input that would advance
/// `history_abs_end` past `usize::MAX - STREAM_ABS_HEADROOM`, which lets
/// every downstream raw `abs_pos + N` arithmetic stay within `usize` on
/// 32-bit targets without per-call saturating guards.
pub(crate) const STREAM_ABS_HEADROOM: usize = HC_OPT_NUM + 16;

/// Ceiling on the absolute-position floor (`history_abs_start`) that the
/// rebase-style `reset` is allowed to advance to before it falls back to a
/// full table zeroing.
///
/// `reset` invalidates the previous frame's table entries by advancing
/// `history_abs_start` past the previous frame's end (a per-position
/// `window_low` reject then drops every stale candidate before it is ever
/// dereferenced), instead of memset-ing the hash / chain / hash3 tables.
/// That keeps the absolute cursor monotonic across independent frames in a
/// reused compressor, so the floor grows by one frame per `reset`. Once it
/// crosses this ceiling, `reset` does the original full zeroing and rewinds
/// the floor to `0`, which bounds `history_abs_start` so
/// [`check_stream_abs_headroom`] stays satisfiable on 32-bit targets (where
/// `usize::MAX` is ~4.29e9): the fallback fires roughly once per 2 GiB of
/// cumulative input on a 32-bit build and is astronomically far on 64-bit.
pub(crate) const REBASE_RESET_FLOOR_CEILING: usize = usize::MAX >> 1;

/// Ceiling for `max_window_size` after a dictionary bump: the eviction band is
/// `2 * max_window_size` and the kernel asserts `data.len() <= u32::MAX`, so a
/// large dictionary must not push the doubled band (plus one pending block)
/// past `u32::MAX`. Both `prime_with_dictionary` and the re-borrow reset clamp
/// to this so the doubled-band-plus-block invariant survives.
pub(crate) const MAX_PRIMED_WINDOW_SIZE: usize =
    (u32::MAX as usize - crate::common::MAX_BLOCK_SIZE as usize) / 2;

/// Frame-level overflow gate shared by `MatchTable`,
/// `DfastMatchGenerator`, and `RowMatchGenerator`.
///
/// Each backend owns its own rolling window and `history_abs_start`
/// cursor but they all hand absolute positions to inner loops that add
/// small constants without per-iteration overflow checks. This helper
/// enforces a single contract: the next `data.len()` bytes — together
/// with the still-resident `window_size` — must leave at least
/// `STREAM_ABS_HEADROOM` slack below `usize::MAX`. Failing fast here
/// lets every downstream `abs_pos + N` site stay raw and keeps i686
/// streams correct. New match-finder backends with their own
/// `add_data` path must route through this helper.
#[inline]
pub(crate) fn check_stream_abs_headroom(
    history_abs_start: usize,
    window_size: usize,
    data_len: usize,
) {
    let _future_abs_end = history_abs_start
        .checked_add(window_size)
        .and_then(|p| p.checked_add(data_len))
        .and_then(|p| p.checked_add(STREAM_ABS_HEADROOM))
        .expect(
            "structured-zstd: cumulative input would leave less than \
             STREAM_ABS_HEADROOM (HC_OPT_NUM + 16) slack below \
             `usize::MAX` for the encoder's absolute-position \
             lookahead; on 32-bit targets, split the input into \
             smaller frames or use a 64-bit build",
        );
}

/// Knuth-style 3-byte hash multiplier. Upstream zstd parity:
/// `ZSTD_HASH3PRIME` in `lib/compress/zstd_compress_internal.h`. Used
/// by the HC3 short-match side table and by the 3-byte branch of the
/// generic `hash_value_with_mls`.
pub(crate) const HC_PRIME3BYTES: u32 = 506_832_829;
/// Knuth-style 4-byte hash multiplier. Upstream zstd parity: `ZSTD_HASHPRIME`.
pub(crate) const HC_PRIME4BYTES: u32 = 2_654_435_761;
/// 5-byte hash multiplier. Upstream zstd parity: `prime5bytes` in
/// `lib/compress/zstd_compress_internal.h`.
pub(crate) const HC_PRIME5BYTES: u64 = 889_523_592_379;
/// 6-byte hash multiplier. Upstream zstd parity: `prime6bytes`.
pub(crate) const HC_PRIME6BYTES: u64 = 227_718_039_650_203;

/// Hash / chain / hash3 sentinel marking an empty slot.
///
/// The upstream zstd uses position `0` as the sentinel because absolute
/// positions are stored as `relative_position + 1`, so a stored zero
/// never collides with a real position. Kept here so storage helpers
/// don't have to pull it from the matcher modules.
pub(crate) const HC_EMPTY: u32 = 0;

// Default table-log constants — the canonical (and only) definitions.
// `match_generator.rs` re-imports the names so existing macros / configs
// can keep referring to them unqualified; do NOT shadow these values
// there with a second `const HC_*_LOG = ...;` declaration. Drift between
// the two copies caused Phase 1d review feedback that this comment
// guards against re-introducing.

/// Default `hash_log` for the level-7 hash-chain matcher. Real values
/// are written directly into [`MatchTable::hash_log`] by the matcher's
/// `configure()` call once the driver resolves the compression level;
/// this constant only seeds the field for matchers that haven't been
/// configured yet.
pub(crate) const HC_HASH_LOG: usize = 20;
/// Default `chain_log` for HC mode (also the pointer-pair log for BT
/// mode — same table reused).
pub(crate) const HC_CHAIN_LOG: usize = 19;
/// Default `hash3_log` for the HC3 short-match side table. Only
/// allocated when the `btultra2` / `btopt` cascade asks for it; HC
/// modes leave it sized to zero.
pub(crate) const HC3_HASH_LOG: usize = 17;

/// Shared storage backing every match finder. Holds the contiguous
/// Immutable dictionary match structure (upstream zstd `ZSTD_dictMatchState`) for
/// the binary-tree / optimal path. A hash + single-link chain over the
/// dictionary content concat range, searched in addition to the live BT
/// with its OWN compare budget so dictionary candidates are reachable even
/// when the live tree's budget is spent on recent positions. Slots store
/// `dict_rel + 1` (dictionary-relative concat index); `0` is empty.
/// Which matcher built a [`DmsDictTables`]: the hash-chain (`prime_dms_hc`) and
/// the binary-tree (`prime_dms_bt`) populate `hash_table` / `chain_table` with
/// incompatible meanings, so a cached dms must only be reused by the SAME
/// builder. Part of the cache-reuse key alongside `mls` / `hash_log` / region so
/// a reused compressor that switches level across the HC↔BT boundary (same
/// `mls` / `hash_log`) rebuilds instead of reinterpreting the other layout.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum DmsDictLayout {
    /// Unbuilt / invalidated — never matches a reuse probe.
    #[default]
    None,
    /// Hash-chain dms (`prime_dms_hc`): single-link chain for the lazy backend.
    Hc,
    /// Binary-tree dms (`prime_dms_bt`): unsorted DUBT for the optimal backend.
    Bt,
}

#[derive(Clone, Default)]
pub(crate) struct DmsDictTables {
    /// Builder that populated the tables (HC vs BT), part of the reuse key.
    pub(crate) layout: DmsDictLayout,
    /// Per-hash-bucket binary-tree root (`1 << hash_log` entries), packed
    /// `dict_rel + 1` (`0` = empty).
    pub(crate) hash_table: Vec<u32>,
    /// Binary-tree children, 2 per dict position (`2 * region_len` entries):
    /// `[2*p] = smaller child, [2*p+1] = larger child` of dict position `p`,
    /// packed `dict_rel + 1` (`0` = empty). An unsorted-DUBT (upstream zstd dms-BT).
    pub(crate) chain_table: Vec<u32>,
    /// Hash width (bits) the dict chain was built at, so probe hashing
    /// reproduces the same buckets.
    pub(crate) hash_log: usize,
    /// Match-length search width (`mls`) the dict chain was built/probed at —
    /// the live cParams `minMatch` (upstream zstd `ZSTD_dictMatchState` uses the same
    /// `mls` as the live search), so probe hashing matches the build.
    pub(crate) mls: usize,
}

/// history buffer, the rolling window, and the hash / chain / hash3
/// tables. Methods on this struct contain only logic that's identical
/// between HC and BT modes — backend-specific table interpretation
/// lives in the matcher modules.
pub(crate) struct MatchTable {
    pub(crate) max_window_size: usize,
    /// Per-chunk lengths of the live window, in add order. The bytes
    /// themselves live only in `history` (the contiguous mirror); this
    /// deque tracks chunk boundaries for eviction and locates the
    /// current block (the back chunk) as a tail slice of `history`,
    /// so the live region is never duplicated.
    pub(crate) chunk_lens: VecDeque<usize>,
    pub(crate) window_size: usize,
    pub(crate) history: Vec<u8>,
    pub(crate) history_start: usize,
    pub(crate) history_abs_start: usize,
    pub(crate) position_base: usize,
    pub(crate) index_shift: usize,
    pub(crate) offset_hist: [u32; 3],
    pub(crate) hash_table: Vec<u32>,
    pub(crate) hash3_table: Vec<u32>,
    pub(crate) chain_table: Vec<u32>,
    pub(crate) hash_log: usize,
    pub(crate) chain_log: usize,
    pub(crate) hash3_log: usize,
    pub(crate) next_to_update3: usize,
    pub(crate) skip_insert_until_abs: usize,
    pub(crate) dictionary_limit_abs: Option<usize>,
    pub(crate) dictionary_primed_for_frame: bool,
    /// Sticky across resets: set once a dictionary is primed into this table
    /// (`set_dictionary_limit_from_primed_bytes` with a non-zero length),
    /// cleared when the dictionary is detached. While set, `reset` takes the
    /// full-reset path (rewind `history_abs_start` to 0 + clear the tables)
    /// instead of the floor-advance fast path: a reused dictionary frame
    /// re-primes the dict at the live `history_abs_start`, and letting that
    /// floor climb frame-over-frame eventually pushes the freshly-primed dict
    /// region below `window_low`, silently dropping every dict match (the
    /// reused-CDict path then degrades to no-dict output after a few frames).
    pub(crate) dictionary_active: bool,
    /// Set by `reset` when it re-borrows a resident attach-mode dictionary
    /// (kept the dict bytes + cached dms in place instead of clear + re-prime).
    /// Signals the frame compressor to SKIP `prime_with_dictionary` this frame.
    pub(crate) dict_resident: bool,
    pub(crate) allow_zero_relative_position: bool,
    /// HC chain-walk depth, mirrored from `HcMatcher::search_depth` during
    /// `configure()`. Stage D moves the BT walker onto this struct, and the
    /// walker macros read the depth from `$table.search_depth` directly so
    /// the call sites don't have to plumb it through.
    pub(crate) search_depth: usize,
    /// Whether the active parser is `btultra2`. Mirrored from the
    /// current [`super::super::strategy::StrategyTag`] during
    /// `configure()` so the BT walker and rebase machinery can stay
    /// on `MatchTable` without consulting the outer generator.
    pub(crate) is_btultra2: bool,
    /// Whether the active backend is one of the BT parsers (`btopt`,
    /// `btultra`, `btultra2`). Mirrored from the active strategy for
    /// the same reason as `is_btultra2`.
    pub(crate) uses_bt: bool,
    /// Binary-tree finder hash width (`mls`), upstream zstd `ZSTD_hashPtr` mls =
    /// `BOUNDED(4, cParams.minMatch, 6)`. clevels.h gives the btlazy2 / btopt
    /// band (levels 13-16) `minMatch = 5` → a 5-byte (8-byte read) hash that
    /// shortens the chains the BT walks (speed), so it is set per level in
    /// `configure`. Only the BT body reads it (the BT hash sites already
    /// guarantee 8 readable bytes); the HC `hash_position` stays 4-byte.
    /// Defaults to `4`.
    pub(crate) search_mls: usize,
    /// Immutable dictionary match chain (upstream zstd `ZSTD_dictMatchState`),
    /// searched by the BT/optimal collect alongside the live tree. `Some`
    /// once primed from a non-empty dictionary on a BT level.
    pub(crate) dms: DictAttach<DmsDictTables>,
    /// Borrowed (no-copy) one-shot input window `(ptr, len)`. When set, the
    /// scan reads candidate/cursor bytes straight from the caller's slice
    /// instead of the owned `history` mirror, so an over-window one-shot
    /// input is matched in place (no input->mirror memmove). Raw pointer:
    /// live until `clear_borrowed_window` / `reset`.
    pub(crate) borrowed_input: Option<(*const u8, usize)>,
    /// Active borrowed block range `[start, end)` within `borrowed_input`,
    /// staged before each borrowed scan.
    pub(crate) borrowed_block: Option<(usize, usize)>,
    /// SIMD kernel tier, resolved ONCE at construction (upstream-style
    /// once-per-encoder-invoke detection: the CPU does not change mid-run, so
    /// `select_kernel()`'s CPUID/`OnceLock` read is paid a single time here).
    /// The per-block strategy entries and the BT walker dispatch off this
    /// cached value instead of re-detecting in the hot path. Mirrors the
    /// `kernel` field the Fast / Row / Dfast matchers already cache.
    pub(crate) kernel: crate::encoding::fastpath::FastpathKernel,
}

// Manual `Clone` (not derived) so the per-frame dictionary-snapshot restore can
// `clone_from` the retained `history` / hash / chain / hash3 / dms buffers
// in place — the derived `clone_from` is `*self = source.clone()`, a full
// per-frame allocate+copy+drop that dominated small `compress-dict` HC/lazy
// frames (upstream zstd reuses the CDict tables in place). `clone()` is the obvious
// field-wise copy; `clone_from()` reuses every heap buffer.
impl Clone for MatchTable {
    fn clone(&self) -> Self {
        Self {
            max_window_size: self.max_window_size,
            chunk_lens: self.chunk_lens.clone(),
            window_size: self.window_size,
            history: self.history.clone(),
            history_start: self.history_start,
            history_abs_start: self.history_abs_start,
            position_base: self.position_base,
            index_shift: self.index_shift,
            offset_hist: self.offset_hist,
            hash_table: self.hash_table.clone(),
            hash3_table: self.hash3_table.clone(),
            chain_table: self.chain_table.clone(),
            hash_log: self.hash_log,
            chain_log: self.chain_log,
            hash3_log: self.hash3_log,
            next_to_update3: self.next_to_update3,
            skip_insert_until_abs: self.skip_insert_until_abs,
            dictionary_limit_abs: self.dictionary_limit_abs,
            dictionary_primed_for_frame: self.dictionary_primed_for_frame,
            dictionary_active: self.dictionary_active,
            dict_resident: self.dict_resident,
            allow_zero_relative_position: self.allow_zero_relative_position,
            search_depth: self.search_depth,
            is_btultra2: self.is_btultra2,
            uses_bt: self.uses_bt,
            search_mls: self.search_mls,
            dms: self.dms.clone(),
            borrowed_input: self.borrowed_input,
            borrowed_block: self.borrowed_block,
            kernel: self.kernel,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        // Heap buffers: reuse the existing allocation (Vec/VecDeque/DictAttach
        // `clone_from` keep capacity and overwrite in place).
        self.chunk_lens.clone_from(&source.chunk_lens);
        self.history.clone_from(&source.history);
        self.hash_table.clone_from(&source.hash_table);
        self.hash3_table.clone_from(&source.hash3_table);
        self.chain_table.clone_from(&source.chain_table);
        self.dms.clone_from(&source.dms);
        // Scalars / Copy fields.
        self.max_window_size = source.max_window_size;
        self.window_size = source.window_size;
        self.history_start = source.history_start;
        self.history_abs_start = source.history_abs_start;
        self.position_base = source.position_base;
        self.index_shift = source.index_shift;
        self.offset_hist = source.offset_hist;
        self.hash_log = source.hash_log;
        self.chain_log = source.chain_log;
        self.hash3_log = source.hash3_log;
        self.next_to_update3 = source.next_to_update3;
        self.skip_insert_until_abs = source.skip_insert_until_abs;
        self.dictionary_limit_abs = source.dictionary_limit_abs;
        self.dictionary_primed_for_frame = source.dictionary_primed_for_frame;
        self.dictionary_active = source.dictionary_active;
        self.dict_resident = source.dict_resident;
        self.allow_zero_relative_position = source.allow_zero_relative_position;
        self.search_depth = source.search_depth;
        self.is_btultra2 = source.is_btultra2;
        self.uses_bt = source.uses_bt;
        self.search_mls = source.search_mls;
        self.borrowed_input = source.borrowed_input;
        self.borrowed_block = source.borrowed_block;
        self.kernel = source.kernel;
    }
}

/// Hash-log for a dictionary-match-state table: `ceil_log2(region)` (a table
/// sized to the dict region), with a 10-bit minimum but never wider than the
/// matcher's own `hash_log` (the dms table need not exceed the live table).
///
/// A tiny source makes `ZSTD_adjustCParams` legitimately shrink the matcher's
/// `hash_log` below 10 (a BT strategy, level >= 13, with a dictionary). The
/// minimum is therefore `min(10, hash_log)`, not a fixed 10: when `hash_log`
/// is already below 10 the 10-bit floor would invert the clamp bounds
/// (`min > max`) and panic. Collapsing the floor to `hash_log` keeps the clamp
/// valid and sizes the dms table to the (small) live-table width — matching
/// what upstream's dict cParams `hashLog` yields for the same small dictionary.
pub(crate) fn dms_hash_log(region: usize, hash_log: usize) -> usize {
    debug_assert!(region >= 1, "dms_hash_log called with empty region");
    let hash_log = hash_log as u32;
    (usize::BITS - (region - 1).leading_zeros()).clamp(10.min(hash_log), hash_log) as usize
}

/// Shared scaffold for the two `prime_dms_*` dictionary-match-state builders.
///
/// Both resolve the dict `region` (capped to the live history), bail on an
/// undersized dict, derive the dict-sized hash log, short-circuit when a
/// same-shape table is already primed, recycle the prior tables' capacity, run
/// the layout-specific `$build` over the dict bytes at the front of the live
/// history, then publish the tables and mark the cache primed. Only the build
/// pass (HC single-link chain vs BT binary tree), the minimum readable region,
/// and the chain entries per dict position differ — those are the macro
/// parameters. `$build` sees `concat` (the dict bytes), `&mut` `hash_table` /
/// `chain_table`, `dms_hash_log`, and `region` in scope; `mls` and any
/// build-only widths come from the caller's surrounding locals.
macro_rules! build_dms {
    (
        $self:ident, $region_len:expr, $layout:expr, $mls:expr, $min_region:expr, $per_pos:expr,
        |$concat:ident, $ht:ident, $ct:ident, $hlog:ident, $region:ident| $build:block
    ) => {{
        let region = $region_len.min($self.live_history().len());
        if region < $min_region {
            $self.dms.invalidate();
            return;
        }
        // Dict-sized hash log: ceil-log2(region), bounded to the matcher's
        // `hash_log`. See [`dms_hash_log`] for the bound rationale.
        let dms_hash_log =
            $crate::encoding::match_table::storage::dms_hash_log(region, $self.hash_log);
        // CDict cache: the tables depend only on the dict bytes (re-committed
        // identically to the front of history every frame) and the
        // (region, mls, hash_log) shape, so a primed same-shape table is valid
        // as-is. A dictionary swap drops the cache via the invalidate path; a
        // level change lands here with a different shape and rebuilds.
        if $self.dms.is_primed()
            && $self.dms.region_len() == region
            && $self
                .dms
                .table()
                .is_some_and(|t| t.layout == $layout && t.mls == $mls && t.hash_log == dms_hash_log)
        {
            return;
        }
        // Reuse the previous tables' capacity on a genuine rebuild (shape
        // change): move the Vecs out before `live_history` re-borrows `self`.
        let (mut hash_table, mut chain_table) = match $self.dms.table_mut() {
            Some(t) => (
                core::mem::take(&mut t.hash_table),
                core::mem::take(&mut t.chain_table),
            ),
            None => (alloc::vec::Vec::new(), alloc::vec::Vec::new()),
        };
        hash_table.clear();
        hash_table.resize(1usize << dms_hash_log, 0u32);
        chain_table.clear();
        chain_table.resize($per_pos * region, 0u32);
        {
            let $concat = $self.live_history();
            let $ht = &mut hash_table;
            let $ct = &mut chain_table;
            let $hlog = dms_hash_log;
            let $region = region;
            $build
        }
        // `concat`'s borrow of `self` ends above; now mutate `self.dms`.
        let tables = $self.dms.table_mut_or_init(DmsDictTables::default);
        tables.layout = $layout;
        tables.hash_table = hash_table;
        tables.chain_table = chain_table;
        tables.hash_log = dms_hash_log;
        tables.mls = $mls;
        $self.dms.set_region_len(region);
        $self.dms.mark_primed();
    }};
}

impl MatchTable {
    pub(crate) fn new(max_window_size: usize) -> Self {
        Self {
            max_window_size,
            chunk_lens: VecDeque::new(),
            window_size: 0,
            history: Vec::new(),
            history_start: 0,
            history_abs_start: 0,
            position_base: 0,
            index_shift: 0,
            offset_hist: [1, 4, 8],
            hash_table: Vec::new(),
            hash3_table: Vec::new(),
            chain_table: Vec::new(),
            hash_log: HC_HASH_LOG,
            chain_log: HC_CHAIN_LOG,
            hash3_log: HC3_HASH_LOG,
            next_to_update3: 0,
            skip_insert_until_abs: 0,
            dictionary_limit_abs: None,
            dictionary_primed_for_frame: false,
            dictionary_active: false,
            dict_resident: false,
            allow_zero_relative_position: false,
            search_depth: 0,
            is_btultra2: false,
            uses_bt: false,
            search_mls: 4,
            dms: DictAttach::new(),
            borrowed_input: None,
            borrowed_block: None,
            kernel: crate::encoding::fastpath::select_kernel(),
        }
    }

    /// Cheap precondition check: can the rebase guard for `abs_pos`
    /// (against the eventual `max_abs_pos`) be skipped because every
    /// involved position is already trivially representable as a
    /// `(rel + 1)` u32? The `is_btultra2` flag tweaks the boundary
    /// rule: BtUltra2 allows `abs_pos == history_abs_start` even when
    /// `allow_zero_relative_position` is `false`, matching the upstream zstd
    /// btultra2 seed-pass behaviour.
    #[inline(always)]
    pub(crate) fn can_skip_rebase_check_at(
        &self,
        abs_pos: usize,
        max_abs_pos: usize,
        is_btultra2: bool,
    ) -> bool {
        let max_rel_no_rebase = (u32::MAX as usize).saturating_sub(2);
        self.position_base == 0
            && self.index_shift == 0
            && max_abs_pos <= max_rel_no_rebase
            && (self.allow_zero_relative_position
                || abs_pos > self.history_abs_start
                || (is_btultra2 && abs_pos == self.history_abs_start))
    }

    /// Decide whether the table needs a cold rebase before `abs_pos`
    /// can be inserted. Pure predicate — does **not** perform the
    /// rebase. The caller (whichever backend owns the BT walk path)
    /// is responsible for invoking `rebase_positions_cold` when this
    /// returns `true`. Hot path: ~once per byte, so the function is
    /// kept tight and `#[inline]`.
    #[inline]
    pub(crate) fn needs_rebase(&self, abs_pos: usize, is_btultra2: bool) -> bool {
        if is_btultra2
            && !self.allow_zero_relative_position
            && self.position_base == 0
            && abs_pos == 0
        {
            return false;
        }
        self.relative_position(abs_pos)
            .is_none_or(|relative| relative >= u32::MAX - 1)
    }

    /// Insert a position into the HC3 short-match side table without
    /// running the rebase check. Caller is responsible for ensuring
    /// the position is already representable (or that the rebase
    /// guard upstream already cleared it). Upstream zstd parity: the inner
    /// `ZSTD_insertAndFindFirstIndexHash3` body.
    pub(crate) fn insert_hash3_only_no_rebase(&mut self, abs_pos: usize) {
        if self.hash3_log == 0 {
            return;
        }
        let idx = abs_pos - self.history_abs_start;
        // Borrowed-aware: hash the SAME bytes the finder reads via
        // `live_history()` (borrowed window in no-copy mode, owned mirror
        // otherwise). Reading `self.history[history_start..]` directly hashed
        // the EMPTY owned mirror on the borrowed one-shot path, so every hash3
        // insert early-returned and the HC3 side table stayed empty — btultra2
        // short-match probing then found nothing. This is the identical bug
        // already fixed for `insert_position_no_rebase`; the hash3 inserter was
        // missed. `hash3` is the only value needed past the borrow, so it ends
        // before the table write.
        let hash3 = {
            let concat = self.live_history();
            if idx + 4 > concat.len() {
                return;
            }
            Self::hash_position_at(concat, idx, self.hash3_log, 3)
        };
        let Some(relative_pos) = self.relative_position(abs_pos) else {
            return;
        };
        self.hash3_table[hash3] = relative_pos + 1;
    }

    /// Insert a position into the main hash / chain table without
    /// running the rebase check. Caller pre-validates that the
    /// position is representable as a `(rel + 1)` u32, either via
    /// `maybe_rebase_positions` (HC) or `bt_update_tree_until` (BT).
    /// Upstream zstd parity: `ZSTD_insertAndFindFirstIndex` inner body.
    #[inline]
    pub(crate) fn insert_position_no_rebase(&mut self, abs_pos: usize) {
        let idx = abs_pos.wrapping_sub(self.history_abs_start);
        // Borrowed-aware: read the SAME bytes the finder hashes via
        // `live_history()` (borrowed window in no-copy mode, owned mirror
        // otherwise). Reading `self.history[history_start..]` directly would
        // hash the empty owned mirror on the borrowed HC lazy/greedy path
        // (live=input, mirror=0) — the insert would skip or hash garbage while
        // the finder hashes the real input, so the chain stays empty and no
        // matches are ever found. `hash` is the only value needed past this
        // borrow, so it ends before the table mutation below.
        let hash = {
            let concat = self.live_history();
            if idx + 4 > concat.len() {
                return;
            }
            Self::hash_position_at(concat, idx, self.hash_log, 4)
        };
        let Some(relative_pos) = self.relative_position(abs_pos) else {
            return;
        };
        let stored = relative_pos + 1;
        let chain_mask = (1usize << self.chain_log) - 1;
        let chain_idx = relative_pos as usize & chain_mask;
        // SAFETY: `hash` is produced by `hash_value_with_mls` which masks
        // the result down to `hash_log` bits, and `hash_table.len() == 1 <<
        // hash_log` (`ensure_tables`). `chain_idx` is `& chain_mask` so
        // `< chain_table.len() == 1 << chain_log`. Both indices are
        // provably in bounds, so the elided bounds checks save ~4
        // instructions per call on this per-byte-of-input hot path.
        debug_assert!(hash < self.hash_table.len());
        debug_assert!(chain_idx < self.chain_table.len());
        unsafe {
            let prev = *self.hash_table.get_unchecked(hash);
            *self.chain_table.get_unchecked_mut(chain_idx) = prev;
            *self.hash_table.get_unchecked_mut(hash) = stored;
        }
    }

    /// Allocate the hash / chain / hash3 tables sized to the current
    /// `hash_log` / `chain_log` / `hash3_log` configuration. Each table
    /// is reallocated whenever its current length doesn't match the
    /// width implied by its `*_log` field — `insert_position_no_rebase`
    /// reaches via `get_unchecked`, so a stale-width table after a
    /// level change would index out of bounds (UB) on the next encode.
    pub(crate) fn ensure_tables(&mut self) {
        let hash_size = 1 << self.hash_log;
        if self.hash_table.len() != hash_size {
            self.hash_table = alloc::vec![HC_EMPTY; hash_size];
        }

        let chain_size = 1 << self.chain_log;
        if self.chain_table.len() != chain_size {
            self.chain_table = alloc::vec![HC_EMPTY; chain_size];
        }

        let hash3_size = if self.hash3_log == 0 {
            0
        } else {
            1 << self.hash3_log
        };
        if self.hash3_table.len() != hash3_size {
            self.hash3_table = alloc::vec![HC_EMPTY; hash3_size];
        }
    }

    /// Unaligned little-endian `u32` load. Hot helper for every
    /// `hash_position*` site. Upstream zstd parity: `MEM_readLE32`.
    #[inline(always)]
    pub(crate) fn read_le_u32(data: &[u8]) -> u32 {
        debug_assert!(data.len() >= 4);
        unsafe { Self::read_le_u32_ptr(data.as_ptr()) }
    }

    /// Pointer variant of [`read_le_u32`]. Used from macros that
    /// already hold a raw pointer.
    ///
    /// # Safety
    /// `ptr` must be valid for a `u32` read.
    #[inline(always)]
    pub(crate) unsafe fn read_le_u32_ptr(ptr: *const u8) -> u32 {
        unsafe { u32::from_le(core::ptr::read_unaligned(ptr as *const u32)) }
    }

    /// 8-byte little-endian read for the `mls` 5/6 hash. Upstream zstd parity:
    /// `MEM_readLE64`.
    ///
    /// # Safety
    /// `ptr` must be valid for a `u64` read (caller guarantees 8 in-bounds
    /// bytes — every `mls >= 5` hash site gates on `idx + 8 <= len`).
    #[inline(always)]
    pub(crate) unsafe fn read_le_u64_ptr(ptr: *const u8) -> u64 {
        unsafe { u64::from_le(core::ptr::read_unaligned(ptr as *const u64)) }
    }

    /// MLS-parameterised hash of a 32-bit value into a `hash_log`-bit
    /// index. Upstream zstd parity: the `mls`-switch in `ZSTD_hashPtr`.
    #[inline(always)]
    pub(crate) fn hash_value_with_mls(value: u32, hash_log: usize, mls: usize) -> usize {
        match mls {
            3 => (((value << 8).wrapping_mul(HC_PRIME3BYTES)) >> (32 - hash_log)) as usize,
            _ => ((value.wrapping_mul(HC_PRIME4BYTES)) >> (32 - hash_log)) as usize,
        }
    }

    /// 8-byte (`mls` 5/6) hash of a 64-bit value into a `hash_log`-bit index.
    /// Upstream zstd parity: `ZSTD_hash5` / `ZSTD_hash6` in
    /// `lib/compress/zstd_compress_internal.h` (`(readLE64 << (64 - 8*mls)) *
    /// primeNbytes >> (64 - hBits)`). Used by the `minMatch = 5` lazy / BT
    /// band (clevels.h levels 6-16).
    #[inline(always)]
    pub(crate) fn hash_value8_with_mls(value: u64, hash_log: usize, mls: usize) -> usize {
        match mls {
            6 => (((value << 16).wrapping_mul(HC_PRIME6BYTES)) >> (64 - hash_log)) as usize,
            // mls == 5 (and any other 8-byte caller defaults here).
            _ => (((value << 24).wrapping_mul(HC_PRIME5BYTES)) >> (64 - hash_log)) as usize,
        }
    }

    /// Hash a 4-byte window at the head of `data`.
    #[inline(always)]
    pub(crate) fn hash_position_with_mls(data: &[u8], hash_log: usize, mls: usize) -> usize {
        if mls >= 5 {
            debug_assert!(data.len() >= 8);
            let value = unsafe { Self::read_le_u64_ptr(data.as_ptr()) };
            return Self::hash_value8_with_mls(value, hash_log, mls);
        }
        let value = Self::read_le_u32(data);
        Self::hash_value_with_mls(value, hash_log, mls)
    }

    /// Hash a 4-byte window starting at `idx` inside `data`. Skips the
    /// slice subrange to keep the bounds check off the per-byte hot
    /// path.
    #[inline(always)]
    pub(crate) fn hash_position_at(data: &[u8], idx: usize, hash_log: usize, mls: usize) -> usize {
        if mls >= 5 {
            // 8-byte (mls 5/6) hash. Caller guarantees `idx + 8 <= len`.
            debug_assert!(idx + 8 <= data.len());
            let value = unsafe { Self::read_le_u64_ptr(data.as_ptr().add(idx)) };
            return Self::hash_value8_with_mls(value, hash_log, mls);
        }
        debug_assert!(idx + 4 <= data.len());
        let value = unsafe { Self::read_le_u32_ptr(data.as_ptr().add(idx)) };
        Self::hash_value_with_mls(value, hash_log, mls)
    }

    /// Main hash for the current matcher (4-byte MLS, `hash_log` from
    /// the table's configuration).
    #[inline(always)]
    pub(crate) fn hash_position(&self, data: &[u8]) -> usize {
        Self::hash_position_with_mls(data, self.hash_log, 4)
    }

    /// Mark this frame as dictionary-primed so the HC / BT seed paths
    /// know to honour the dictionary boundary.
    pub(crate) fn mark_dictionary_primed(&mut self) {
        self.dictionary_primed_for_frame = true;
    }

    /// Set the per-frame dictionary boundary in absolute coordinates.
    /// `primed_len == 0` clears the limit.
    pub(crate) fn set_dictionary_limit_from_primed_bytes(&mut self, primed_len: usize) {
        self.dictionary_limit_abs = if primed_len == 0 {
            None
        } else {
            Some(self.history_abs_start.saturating_add(primed_len))
        };
        // Sticky: once primed, every following reset must rewind to a clean
        // base so the re-prime lands at a stable low `history_abs_start`.
        self.dictionary_active = primed_len != 0;
    }

    /// Build the immutable dictionary match **binary tree** (upstream zstd
    /// `ZSTD_dictMatchState`, the dms-BT walked in `ZSTD_insertBtAndGetAllMatches`
    /// `zstd_opt.c:777-813`) over the first `region_len` bytes of the live
    /// history (the dictionary content at the front). A hash-chain dms surfaces
    /// only the few candidates that share a hash bucket; a DUBT descends to the
    /// LONGEST dict match efficiently, which is where the upstream zstd extracts the
    /// bulk of its dict-match value at btlazy2 / btopt (a chain there left
    /// 80-90% of the dict savings on the table). Hashed at the BT `search_mls`
    /// width into a dict-sized hash log; `chain_table` holds 2 entries per dict
    /// position (`[smaller_child, larger_child]`), `hash_table` the per-bucket
    /// tree roots. `dict_rel + 1` packing, `0` sentinel. Dict positions are
    /// concat indices at the front of the shared buffer, so the walk's offset is
    /// `idx - dict_idx` directly (no upstream zstd `dmsIndexDelta`). Called on BT levels
    /// (`uses_bt`); cached across frames via `DictAttach::is_primed`.
    pub(crate) fn prime_dms_bt(&mut self, region_len: usize) {
        // Upstream zstd `ZSTD_dictMatchState` searches the dict at the live cParams
        // `minMatch` (= our `search_mls`), NOT a fixed 3 — a fixed 3 surfaces
        // huge-offset ml=3 dict matches that the greedy btlazy2 parser commits
        // at a loss. The dms's value is being a SEPARATE structure (the dict is
        // not in the live tree), not a lower minMatch.
        let mls = self.search_mls;
        let read = if mls >= 5 { 8 } else { 4 };
        // Build-pass compare budget: the dict is bounded (<= window), so a
        // generous fixed depth keeps the tree well-ordered without the live
        // searchLog cap. Mirrors upstream zstd `ZSTD_insertBt1` nbCompares.
        const DMS_BUILD_DEPTH: usize = 1 << 9;
        // 2 children per dict position: [smaller, larger].
        build_dms!(
            self,
            region_len,
            DmsDictLayout::Bt,
            mls,
            read,
            2,
            |concat, hash_table, chain_table, dms_hash_log, region| {
                let mut current = 0usize;
                while current + read <= region {
                    let h = Self::hash_position_at(concat, current, dms_hash_log, mls);
                    // Insert `current` as the new root; splay the old tree into
                    // `current`'s smaller/larger subtrees (upstream zstd `ZSTD_insertBt1`).
                    let mut match_packed = hash_table[h];
                    hash_table[h] = (current + 1) as u32;
                    let mut smaller_slot = 2 * current;
                    let mut larger_slot = 2 * current + 1;
                    let mut common_smaller = 0usize;
                    let mut common_larger = 0usize;
                    let mut compares = DMS_BUILD_DEPTH;
                    while compares > 0 && match_packed != 0 {
                        let cand = (match_packed - 1) as usize;
                        // Tree holds only earlier positions; `cand < current` always.
                        if cand >= current {
                            break;
                        }
                        compares -= 1;
                        let next_pair = 2 * cand;
                        let mut ml = common_smaller.min(common_larger);
                        // Common prefix bounded by the dict tail from `current`
                        // (`cand < current`, so `current` is the binding limit).
                        let limit = region - current;
                        while ml < limit && concat[cand + ml] == concat[current + ml] {
                            ml += 1;
                        }
                        if current + ml >= region {
                            // Reached the dict end: can't order this pair, stop
                            // (upstream zstd `ip+matchLength == iend` break).
                            break;
                        }
                        if concat[cand + ml] < concat[current + ml] {
                            // `cand` (and its smaller subtree) sorts below `current`.
                            chain_table[smaller_slot] = match_packed;
                            common_smaller = ml;
                            smaller_slot = next_pair + 1;
                            match_packed = chain_table[next_pair + 1];
                        } else {
                            chain_table[larger_slot] = match_packed;
                            common_larger = ml;
                            larger_slot = next_pair;
                            match_packed = chain_table[next_pair];
                        }
                    }
                    chain_table[smaller_slot] = 0;
                    chain_table[larger_slot] = 0;
                    current += 1;
                }
            }
        );
    }

    /// Build a SEPARATE dictionary match state for the lazy hash-chain path
    /// (upstream `ZSTD_dictMatchState`, the dms HC4 walk in
    /// `ZSTD_HcFindBestMatch` `zstd_lazy.c:748-770`) over the first `region_len`
    /// bytes of the live history (the dictionary at the front). Unlike
    /// [`Self::prime_dms_bt`] this is a single-link hash chain (one `next` entry
    /// per dict position, like the live HC chain), not a binary tree: the lazy
    /// parser walks it with a bounded compare budget rather than descending to
    /// the longest match. `hash_table` holds per-bucket chain heads, packed
    /// `dict_rel + 1` (`0` = empty); `chain_table[dict_rel]` is the previous
    /// dict position in the same bucket, same packing. Dict positions are concat
    /// indices at the front of the shared buffer, so the offset is `idx -
    /// dict_idx` directly. Hashed at `HC_MIN_MATCH_LEN` (4), matching the live
    /// chain. Cached across frames via `DictAttach::is_primed`.
    pub(crate) fn prime_dms_hc(&mut self, region_len: usize) {
        let mls = HC_MIN_MATCH_LEN;
        // Single `next` link per dict position (HC4 chain, not the BT's 2/pos).
        build_dms!(
            self,
            region_len,
            DmsDictLayout::Hc,
            mls,
            mls,
            1,
            |concat, hash_table, chain_table, dms_hash_log, region| {
                let mut current = 0usize;
                while current + mls <= region {
                    let h = Self::hash_position_at(concat, current, dms_hash_log, mls);
                    // Prepend `current` to its bucket chain (upstream zstd
                    // `ZSTD_insertAndFindFirstIndex` head insert).
                    chain_table[current] = hash_table[h];
                    hash_table[h] = (current + 1) as u32;
                    current += 1;
                }
            }
        );
    }

    /// Append a freshly committed buffer to the rolling window. Drops
    /// chunk-length entries for the oldest slices until the new total
    /// fits inside `max_window_size`, extends the contiguous `history`
    /// mirror with the new bytes, then hands the *input* buffer back
    /// through `reuse_space` for pool reuse — `history` now owns the
    /// bytes, so the input buffer carries no live data. (Callers must
    /// therefore treat the callback as recycle-only, not as an eviction
    /// report; eviction bytes come from the `window_size` delta.)
    /// Pre-size the contiguous `history` mirror to `expected_bytes` (capped to
    /// the window eviction bound) so the per-block `add_data`
    /// `extend_from_slice` growth does not overshoot through `Vec` capacity
    /// doubling. Upstream zstd allocates its window buffer at `windowSize + blockSize`
    /// exactly; left to `Vec` doubling, a ~1 MiB history lands in a 2 MiB
    /// allocation — wasted peak that dominates once the match-finder tables are
    /// dictionary-tier-small. Correctness-neutral: the mirror still grows on
    /// demand if `expected_bytes` underestimates. Only worth calling when the
    /// total is known (source-size hinted); an unhinted stream keeps doubling.
    /// Heap bytes this table owns: history, the hash / hash3 / chain tables,
    /// the chunk-length deque, and any attached immutable dictionary tables.
    pub(crate) fn heap_size(&self) -> usize {
        let u32_sz = size_of::<u32>();
        let usize_sz = size_of::<usize>();
        self.chunk_lens.capacity() * usize_sz
            + self.history.capacity()
            + (self.hash_table.capacity()
                + self.hash3_table.capacity()
                + self.chain_table.capacity())
                * u32_sz
            + self.dms.table().map_or(0, |t| {
                (t.hash_table.capacity() + t.chain_table.capacity()) * u32_sz
            })
    }

    pub(crate) fn reserve_history(&mut self, expected_bytes: usize) {
        // Eviction keeps the live mirror within `max_window_size`; the dead
        // prefix is drained at a quarter window (see `compact_history`) and one
        // pending block can sit on top, so the steady-state ceiling is
        // `max_window_size + max_window_size/4 + MAX_BLOCK_SIZE` — matching the
        // `add_data` eviction reserve so the two never fight over capacity.
        // Plain arithmetic (not `saturating_*`): `max_window_size = 1 <<
        // window_log` with `window_log <= ZSTD_WINDOWLOG_MAX` (31), so the sum
        // is at most `2^31 + 2^29 + 2^17 < usize::MAX` even on 32-bit targets —
        // overflow is unreachable, and a silent saturation here would only mask
        // a window_log bound violation upstream.
        let cap = self.max_window_size
            + (self.max_window_size >> 2)
            + crate::common::MAX_BLOCK_SIZE as usize;
        let want = expected_bytes.min(cap);
        if want > self.history.capacity() {
            self.history.reserve_exact(want - self.history.len());
        }
    }

    pub(crate) fn add_data(&mut self, data: Vec<u8>, mut reuse_space: impl FnMut(Vec<u8>)) {
        assert!(data.len() <= self.max_window_size);
        check_stream_abs_headroom(self.history_abs_start, self.window_size, data.len());
        if self.window_size + data.len() > self.max_window_size {
            // Cap the history mirror near the live window once eviction starts:
            // reserve exactly (window + window/4 + one block) so the Vec grows
            // linearly to that ceiling instead of power-of-two doubling to ~2x
            // window; `compact_history`'s quarter-window drain keeps len under
            // it, so the Vec never reallocates again. Small frames that never
            // fill the window keep their tight data-sized buffer.
            let target = self.max_window_size
                + (self.max_window_size >> 2)
                + crate::common::MAX_BLOCK_SIZE as usize;
            if target > self.history.len() && self.history.capacity() < target {
                self.history.reserve_exact(target - self.history.len());
            }
        }
        while self.window_size + data.len() > self.max_window_size {
            let removed_len = self.chunk_lens.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
        self.compact_history();
        let added = data.len();
        self.history.extend_from_slice(&data);
        self.next_to_update3 = self.next_to_update3.max(self.history_abs_start);
        self.window_size += added;
        self.chunk_lens.push_back(added);
        // The input buffer's bytes now live in `history`; hand the buffer
        // straight back to the caller's pool instead of holding a second
        // copy in the window (the source of the per-compress duplicate).
        reuse_space(data);
    }

    /// Drop window slices that have rolled past `max_window_size`.
    /// Used after `max_window_size` shrinks (dictionary release path).
    pub(crate) fn trim_to_window(&mut self) {
        while self.window_size > self.max_window_size {
            let removed_len = self.chunk_lens.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
    }

    /// Drain the dead prefix of `history` (already-rolled-out bytes)
    /// once it reaches a quarter window, or when it accounts for at
    /// least half of the mirror. Keeps the contiguous mirror compact
    /// so reallocation costs stay amortised.
    pub(crate) fn compact_history(&mut self) {
        if self.history_start == 0 {
            return;
        }
        // Drain the dead prefix at a quarter window (paired with the eviction
        // reserve in `add_data`) so the mirror stays near `window + window/4`
        // rather than doubling to ~2x window on long streams.
        if self.history_start >= (self.max_window_size >> 2)
            || self.history_start * 2 >= self.history.len()
        {
            self.history.drain(..self.history_start);
            self.history_start = 0;
        }
    }

    /// The live (post-`history_start`) slice of the contiguous history
    /// mirror. Match finders operate on this slice rather than the raw
    /// `history` Vec.
    #[inline]
    pub(crate) fn live_history(&self) -> &[u8] {
        // Borrowed one-shot: expose `[0, block_end)` of the caller's input so
        // candidate/cursor reads land in place (the owned `history` mirror is
        // empty under the no-copy path). Loop-invariant branch, inlines.
        if let Some((_start, end)) = self.borrowed_block {
            let (ptr, total) = self
                .borrowed_input
                .expect("borrowed_block set without a registered borrowed window");
            debug_assert!(
                end <= total,
                "borrowed block end {end} exceeds window {total}"
            );
            // SAFETY: `ptr` is the registered window start (live by contract);
            // `end <= total` in bounds.
            return unsafe { core::slice::from_raw_parts(ptr, end) };
        }
        &self.history[self.history_start..]
    }

    /// Absolute position one past the end of the live history.
    pub(crate) fn history_abs_end(&self) -> usize {
        self.history_abs_start + self.live_history().len()
    }

    /// Get a reference to the last committed window slice. Returns
    /// the most recent buffer in the rolling window — panics if no
    /// data has been committed yet.
    pub(crate) fn get_last_space(&self) -> &[u8] {
        if let (Some((ptr, _total)), Some((block_start, block_end))) =
            (self.borrowed_input, self.borrowed_block)
        {
            // SAFETY: borrowed liveness contract; range validated when staged.
            return unsafe {
                core::slice::from_raw_parts(ptr.add(block_start), block_end - block_start)
            };
        }
        let last = *self.chunk_lens.back().unwrap();
        &self.history[self.history.len() - last..]
    }

    /// Register the borrowed input window. Zeroes `history_abs_start` so
    /// borrowed positions are absolute input offsets (the owned floor-advance
    /// reset leaves it non-zero across frames; borrowed never `add_data`s, so
    /// it stays 0 for the frame). Every probed candidate is byte-verified, so
    /// stale table entries from a prior frame are rejected (or, if their bytes
    /// coincidentally match, are genuine in-window matches).
    ///
    /// # Safety
    /// `buffer` must stay live and unmodified until `clear_borrowed_window`
    /// or `reset`.
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        self.borrowed_input = Some((buffer.as_ptr(), buffer.len()));
        self.borrowed_block = None;
        self.history_abs_start = 0;
    }

    pub(crate) fn clear_borrowed_window(&mut self) {
        self.borrowed_input = None;
        self.borrowed_block = None;
    }

    /// Stage `[block_start, block_end)` as the active borrowed block.
    pub(crate) fn stage_borrowed_block(&mut self, block_start: usize, block_end: usize) {
        let (_ptr, total) = self
            .borrowed_input
            .expect("stage_borrowed_block requires a registered borrowed window");
        assert!(
            block_start <= block_end && block_end <= total,
            "borrowed block bounds out of range: start={block_start} end={block_end} total={total}",
        );
        self.borrowed_block = Some((block_start, block_end));
    }

    /// `(current_abs_start, current_len)` for the active scan. Borrowed: the
    /// staged block range. Owned: the last committed chunk in the live window.
    pub(crate) fn current_block_range(&self) -> (usize, usize) {
        if let Some((start, end)) = self.borrowed_block {
            (start, end - start)
        } else {
            let current_len = *self.chunk_lens.back().unwrap();
            (
                self.history_abs_start + self.window_size - current_len,
                current_len,
            )
        }
    }

    /// Convert an absolute position into the (relative_pos + 1) form
    /// stored in the hash / chain tables. Returns `None` for positions
    /// outside the current window's representable range. Upstream zstd parity:
    /// matches the `relIdx` arithmetic in `ZSTD_HcFindBestMatch`.
    pub(crate) fn relative_position(&self, abs_pos: usize) -> Option<u32> {
        let shifted_abs = abs_pos.checked_add(self.index_shift)?;
        let rel = shifted_abs.checked_sub(self.position_base)?;
        let rel_u32 = u32::try_from(rel).ok()?;
        // Upstream zstd parity: raw BT/HC tables use 0 as the empty sentinel, so
        // the very first absolute position in the first block
        // (curr == 0) is not a representable candidate index.
        if !self.allow_zero_relative_position && self.position_base == 0 && rel_u32 == 0 {
            return None;
        }
        // Positions are stored as (relative_pos + 1), with 0 reserved
        // as the empty sentinel. So the raw relative position itself
        // must stay strictly below u32::MAX.
        (rel_u32 < u32::MAX).then_some(rel_u32)
    }

    /// Lower bound (in absolute positions) of the window that's still
    /// reachable from `target_abs`. Upstream zstd parity: `windowLow` in
    /// `ZSTD_compressBlock_*`.
    pub(crate) fn window_low_abs_for_target(&self, target_abs: usize) -> usize {
        let history_low = self.history_abs_start;
        let window_low = target_abs.saturating_sub(self.max_window_size);
        history_low.max(window_low)
    }

    /// BT pointer-pair log: chain_log minus one because the table
    /// stores pairs of pointers (smaller / larger) per node.
    #[inline(always)]
    pub(crate) fn bt_log(&self) -> usize {
        self.chain_log.saturating_sub(1)
    }

    /// BT pointer-pair address mask. Upstream zstd parity: `(1 << btLog) - 1`.
    #[inline(always)]
    pub(crate) fn bt_mask(&self) -> usize {
        (1usize << self.bt_log()) - 1
    }

    /// Convert an absolute position into a BT pair index in
    /// `chain_table`. Each node occupies two consecutive slots
    /// (smaller, larger) so the result is doubled. Upstream zstd parity:
    /// `2 * (curr & btMask)` from `ZSTD_insertBt1`.
    #[inline(always)]
    pub(crate) fn bt_pair_index_for_abs(&self, abs_pos: usize) -> usize {
        // Hot per-iteration BT walker entry. `abs_pos` is a
        // frame-lifetime absolute stream cursor (capped by
        // `check_stream_abs_headroom`); `index_shift` is block-local
        // (`current_len` during the btultra2 seed pass, `0` otherwise).
        // The result is immediately masked down to the BT ring width
        // by `& bt_mask()`, so what matters here is only the modulo-
        // ring identity `(a + b) mod m == ((a mod 2^bits) + (b mod
        // 2^bits)) mod m` for `m | 2^bits`: `wrapping_add` preserves
        // that identity even on the rare i686 streams where the raw
        // `usize` sum overflows. Using `wrapping_add` instead of `+`
        // (or `saturating_add`) also keeps the release-mode overflow
        // branch and the debug-mode overflow panic off this hot path.
        let bt_pos = abs_pos.wrapping_add(self.index_shift);
        2 * (bt_pos & self.bt_mask())
    }

    /// Decode a stored hash / chain table entry back into its absolute
    /// position. Returns `None` for the `HC_EMPTY` sentinel or for
    /// entries that underflowed after `index_shift` was applied. Pure
    /// associated function — kept off `&self` so macros can pass the
    /// constituent fields directly when partial-borrow shenanigans
    /// would block a `&self` call.
    #[inline(always)]
    pub(crate) fn stored_abs_position_fast(
        stored: u32,
        position_base: usize,
        index_shift: usize,
    ) -> Option<usize> {
        if stored == HC_EMPTY {
            return None;
        }
        let shifted = position_base + (stored as usize - 1);
        if shifted < index_shift {
            return None;
        }
        Some(shifted - index_shift)
    }

    /// Reset the per-frame portion of the storage for the next
    /// independent frame.
    ///
    /// Rather than memset-ing the hash / chain / hash3 tables (the cost
    /// of which is proportional to table size, paid once per frame in a
    /// reused compressor), this advances the absolute-position floor
    /// `history_abs_start` past the previous frame's end. Every table
    /// walker rejects a candidate whose absolute position is below the
    /// floor (`window_low >= history_abs_start`) before it dereferences
    /// any history byte or trusts a binary-tree seed length, so the
    /// previous frame's stale entries become unreachable without being
    /// cleared. `position_base` / `index_shift` are left untouched so the
    /// stale entries still decode to their (now sub-floor) absolute
    /// positions; the per-position rebase guard
    /// ([`Self::maybe_rebase_positions`]) handles the rare `u32`
    /// representability rollover as the cursor keeps climbing.
    ///
    /// When advancing the floor would push it past
    /// [`REBASE_RESET_FLOOR_CEILING`], the original full zeroing runs and
    /// the floor rewinds to `0`, bounding the absolute cursor so
    /// [`check_stream_abs_headroom`] stays satisfiable on 32-bit targets.
    /// The window is just `chunk_lens` (the bytes live in `history`,
    /// cleared below), so there are no per-block buffers to drain;
    /// `_reuse_space` is retained only for caller signature compatibility
    /// and is intentionally unused.
    pub(crate) fn reset(&mut self, _reuse_space: impl FnMut(Vec<u8>)) {
        // Snapshot the previous frame's one-past-the-end absolute
        // position before clearing the history that `history_abs_end`
        // reads. Every stale table entry points strictly below this.
        let next_floor = self.history_abs_end();
        // Re-borrow: an ATTACH-mode reused dict frame keeps the dict in the
        // separate cached dms (not the live tables), so the resident dict bytes
        // at the front of history + the cached dms can stay in place — the frame
        // compressor then SKIPs `prime_with_dictionary`. Only when the dict is
        // fully resident at concat `[0, region)` with no drain (`history_start
        // == 0`). Pinned at abs `[0, region)` (concat-keyed dms is abs-invariant).
        //
        // mls consistency: `dms.is_primed()` here is sufficient because the dms is
        // keyed by `(region, layout, mls, hash_log)` and `configure()` invalidates
        // it whenever `search_mls` changes (a level switch). So a primed dms is
        // always one built with the CURRENT `search_mls`; the reborrow can never
        // reuse a dms whose tables were built under a stale mls.
        let reborrow_region =
            if self.dictionary_active && self.dms.is_primed() && self.history_start == 0 {
                // `checked_sub`, not `-`: after the dict chunk is evicted,
                // `compact_history` can reset `history_start` to 0 while
                // `history_abs_start` has already advanced PAST
                // `dictionary_limit_abs`. The plain subtraction would underflow
                // (panic under overflow checks) before the `r > 0` filter rejects
                // the stale region; `checked_sub` -> `None` lets the filter run.
                self.dictionary_limit_abs
                    .and_then(|limit| limit.checked_sub(self.history_abs_start))
                    .filter(|&r| r > 0 && r <= self.history.len())
            } else {
                None
            };
        self.window_size = 0;
        self.offset_hist = [1, 4, 8];
        self.skip_insert_until_abs = 0;
        // Clear borrowed-window state so a following OWNED frame reads the
        // owned mirror; a borrowed frame re-arms via `set_borrowed_window`.
        self.borrowed_input = None;
        self.borrowed_block = None;
        self.dictionary_primed_for_frame = false;
        self.allow_zero_relative_position = false;
        if let Some(region) = reborrow_region {
            // Keep `[0, region)` (the dict); drop the previous frame's input.
            self.history.truncate(region);
            self.chunk_lens.clear();
            self.chunk_lens.push_back(region);
            self.window_size = region;
            // Bump the eviction window by the dict size exactly as
            // `prime_with_dictionary` does (clamped to MAX_PRIMED_WINDOW_SIZE so
            // the doubled-band invariant survives), so the resident dict + the
            // next frame's input both stay in the window — otherwise adding the
            // input drains the dict prefix and `history_abs_start` climbs off
            // it, turning every dms candidate into an out-of-range underflow.
            //
            // The driver re-establishes `self.max_window_size = 1 << window_log`
            // (the un-bumped base) on EVERY reset, before this `reset` runs (see
            // `MatchGeneratorDriver::reset`'s HashChain arm), so this `+=` yields
            // `base + region` each frame, never an accumulating `base + N*region`.
            // The driver also restores `dictionary_retained_budget = region` on
            // the resident path (`reapply_resident_dictionary`) so the retire
            // path shrinks this back to base as the dict evicts. The headroom
            // subtraction is underflow-safe regardless of the pre-bump value: the
            // `MAX_PRIMED_WINDOW_SIZE >= max_window_size` invariant always holds
            // (every bump clamps to it), so `headroom >= 0` — no `saturating_*`.
            let headroom = MAX_PRIMED_WINDOW_SIZE - self.max_window_size;
            self.max_window_size += region.min(headroom);
            // The dict bytes stay at the buffer front `[0, region)`; the dms is
            // concat-keyed (abs-invariant), so its candidates + extension are
            // independent of the abs base. The live tables carry only input.
            // Retire the previous frame's input entries one of two ways:
            if next_floor <= REBASE_RESET_FLOOR_CEILING {
                // Floor-advance (no memset): keep the climbing abs base + the
                // position encoding. The dict sits AT the floor — its abs
                // `[next_floor, next_floor+region)` is `>= window_low`, so it
                // stays in-window via the bump above; the previous frame's input
                // entries are `< next_floor` and wrap out of range on read (the
                // non-dict fast path's reject-by-wrap, reused for the resident
                // dict). Skip cursors move into the climbed abs space.
                self.history_abs_start = next_floor;
                self.skip_insert_until_abs = next_floor + region;
                self.next_to_update3 = next_floor + region;
                self.dictionary_limit_abs = Some(next_floor + region);
            } else {
                // Bounded fallback: the floor would climb past the 32-bit
                // headroom ceiling, so rewind to the origin and zero the live
                // tables (the dict region is re-pinned at abs `[0, region)`).
                self.history_abs_start = 0;
                self.position_base = 0;
                self.index_shift = 0;
                self.hash_table.fill(HC_EMPTY);
                self.hash3_table.fill(HC_EMPTY);
                self.chain_table.fill(HC_EMPTY);
                self.skip_insert_until_abs = region;
                self.next_to_update3 = region;
                self.dictionary_limit_abs = Some(region);
            }
            self.dict_resident = true;
            return;
        }
        self.chunk_lens.clear();
        self.history.clear();
        self.history_start = 0;
        self.dictionary_limit_abs = None;
        self.dict_resident = false;
        if self.dictionary_active {
            // A reused dictionary frame re-primes the dict at the live
            // `history_abs_start` every frame. The floor-advance fast path lets
            // that base climb frame-over-frame, and once it climbs past the
            // window the freshly-primed dict region falls below `window_low`
            // and every dict match is silently dropped (reused-CDict output
            // degrades to no-dict after a few frames). Rewind to the origin and
            // clear the tables so the next prime lands at a stable low base.
            // Table CLEARS are deferred to whichever cleanup runs next: a
            // dict-active reset is always followed by either
            // `restore_primed_dictionary` (its `clone_from` overwrites every
            // table wholesale -- the reuse hot path) or `prime_with_dictionary`
            // (which now clears + rebuilds). Filling here would be thrown away on
            // the clone path, and that fill dominated tiny dict frames (a 4 KB
            // frame zero-fills the whole dict-tier chain table -- ~26% of
            // compress). The rewound `position_base` / `index_shift` make the
            // un-cleared slots decode out-of-range, and no match-find runs before
            // the clone / prime cleans them, so deferring is safe (skipping the
            // fills WITHOUT this hand-off reproduces the reused-CDict decay).
            // The "always followed by prime/restore" hand-off holds because
            // removing or replacing a dictionary clears `dictionary_active`
            // (in `invalidate_primed_dictionary`): this branch is only reached
            // while a dictionary is attached and will re-prime/restore, so a
            // subsequent no-dictionary frame can never strand the deferred clear.
            self.history_abs_start = 0;
            self.position_base = 0;
            self.index_shift = 0;
            self.next_to_update3 = 0;
        } else if next_floor <= REBASE_RESET_FLOOR_CEILING {
            // Fast path: advance the floor so the previous frame's
            // entries fall below `window_low` and are rejected on read.
            // The tables keep their contents (and their sizing — a later
            // `ensure_tables` call still reallocates them clean if the
            // next level's config changed their dimensions).
            self.history_abs_start = next_floor;
            self.next_to_update3 = next_floor;
        } else {
            // Bounded fallback: rewind the cursor to the origin and zero
            // the tables so `history_abs_start` cannot climb toward
            // `usize::MAX`. Clear each table independently — `Vec::fill`
            // on an empty Vec is a no-op, so unconditional fills are safe
            // even when a table hasn't been allocated yet (HC mode keeps
            // hash3_table empty, and the backend-switch path swaps every
            // table for Vec::new() to release oversized allocations).
            self.history_abs_start = 0;
            self.position_base = 0;
            self.index_shift = 0;
            self.next_to_update3 = 0;
            self.hash_table.fill(HC_EMPTY);
            self.hash3_table.fill(HC_EMPTY);
            self.chain_table.fill(HC_EMPTY);
        }
    }

    /// Zero the hash / hash3 / chain tables to `HC_EMPTY`. The dict-active
    /// `reset` defers this fill to here (called from `prime_with_dictionary`)
    /// so the reuse hot path -- which restores a clean snapshot via `clone_from`
    /// instead of re-priming -- never pays the dict-tier-table memset that
    /// dominated tiny dict frames. `Vec::fill` on an unallocated (empty) table
    /// is a no-op, so the unconditional fills are safe before `ensure_tables`.
    pub(crate) fn clear_chain_hash_tables(&mut self) {
        self.hash_table.fill(HC_EMPTY);
        self.hash3_table.fill(HC_EMPTY);
        self.chain_table.fill(HC_EMPTY);
    }

    /// Upstream zstd parity: `ZSTD_compressBlock_btopt_generic` starts its main
    /// match loop at cursor `1` (not `0`) whenever the current block sits
    /// at the absolute history origin — the byte at offset `0` is
    /// reserved for the seed literal so the parser never reports a
    /// zero-offset match. The same flag governs the initial `litlen`
    /// because the seed literal counts as one pending literal byte.
    pub(crate) fn opt_start_cursor_and_litlen(&self, current_abs_start: usize) -> (usize, usize) {
        let start_cursor = usize::from(current_abs_start == self.history_abs_start);
        (start_cursor, start_cursor)
    }

    /// Stage D: BT walker step. Cross-platform dispatcher that picks
    /// the per-kernel variant so the per-iteration
    /// `count_match_from_indices` symbol inlines under the kernel's
    /// `target_feature` umbrella. Previously lived on `BtMatcher`
    /// but the body uses only table state plus `self.search_depth`,
    /// so it migrates onto `MatchTable` and clears the cross-struct
    /// borrow that blocked the rest of the BT update chain.
    #[inline(always)]
    pub(crate) fn bt_insert_step_no_rebase(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        target_abs: usize,
    ) -> usize {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.bt_insert_step_no_rebase_neon(abs_pos, current_abs_end, target_abs)
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.bt_insert_step_no_rebase_avx2_bmi2(abs_pos, current_abs_end, target_abs)
                },
                FastpathKernel::Sse42 => unsafe {
                    self.bt_insert_step_no_rebase_sse42(abs_pos, current_abs_end, target_abs)
                },
                FastpathKernel::Scalar => {
                    self.bt_insert_step_no_rebase_scalar(abs_pos, current_abs_end, target_abs)
                }
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.bt_insert_step_no_rebase_scalar(abs_pos, current_abs_end, target_abs)
        }
    }

    /// NEON umbrella BT walker step.
    ///
    /// # Safety
    /// AArch64 with NEON (baseline).
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    pub(crate) unsafe fn bt_insert_step_no_rebase_neon(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        target_abs: usize,
    ) -> usize {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_step_no_rebase_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            target_abs,
            crate::encoding::fastpath::neon::count_match_from_indices
        )
    }

    /// SSE4.2 umbrella BT walker step.
    ///
    /// # Safety
    /// x86/x86_64 with SSE4.2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    pub(crate) unsafe fn bt_insert_step_no_rebase_sse42(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        target_abs: usize,
    ) -> usize {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_step_no_rebase_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            target_abs,
            crate::encoding::fastpath::sse42::count_match_from_indices
        )
    }

    /// AVX2+BMI2 umbrella BT walker step.
    ///
    /// # Safety
    /// x86/x86_64 with AVX2 + BMI2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    pub(crate) unsafe fn bt_insert_step_no_rebase_avx2_bmi2(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        target_abs: usize,
    ) -> usize {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_step_no_rebase_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            target_abs,
            crate::encoding::fastpath::avx2_bmi2::count_match_from_indices
        )
    }

    /// Scalar fallback BT walker step (used on non-AArch64 targets).
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    pub(crate) fn bt_insert_step_no_rebase_scalar(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        target_abs: usize,
    ) -> usize {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_step_no_rebase_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            target_abs,
            crate::encoding::fastpath::scalar::count_match_from_indices
        )
    }

    /// Stage D: cross-platform dispatcher for the BT collect-matches walker.
    /// External / test entry — the hot path bypasses this and calls the
    /// per-kernel variant from inside the surrounding
    /// `collect_optimal_candidates_initialized_<kernel>` umbrella.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(crate) fn bt_insert_and_collect_matches(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: &HcOptimalCostProfile,
        min_match_len: usize,
        best_len_for_skip: &mut usize,
        out: &mut Vec<MatchCandidate>,
        reps: [u32; 3],
        lit_len: usize,
        use_hash3: bool,
    ) {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.bt_insert_and_collect_matches_neon(
                abs_pos,
                current_abs_end,
                profile,
                min_match_len,
                best_len_for_skip,
                out,
                reps,
                lit_len,
                use_hash3,
            )
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.bt_insert_and_collect_matches_avx2_bmi2(
                        abs_pos,
                        current_abs_end,
                        profile,
                        min_match_len,
                        best_len_for_skip,
                        out,
                        reps,
                        lit_len,
                        use_hash3,
                    )
                },
                FastpathKernel::Sse42 => unsafe {
                    self.bt_insert_and_collect_matches_sse42(
                        abs_pos,
                        current_abs_end,
                        profile,
                        min_match_len,
                        best_len_for_skip,
                        out,
                        reps,
                        lit_len,
                        use_hash3,
                    )
                },
                FastpathKernel::Scalar => self.bt_insert_and_collect_matches_scalar(
                    abs_pos,
                    current_abs_end,
                    profile,
                    min_match_len,
                    best_len_for_skip,
                    out,
                    reps,
                    lit_len,
                    use_hash3,
                ),
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.bt_insert_and_collect_matches_scalar(
                abs_pos,
                current_abs_end,
                profile,
                min_match_len,
                best_len_for_skip,
                out,
                reps,
                lit_len,
                use_hash3,
            )
        }
    }

    /// NEON-umbrella variant of `bt_insert_and_collect_matches`.
    ///
    /// # Safety
    /// AArch64 with NEON (baseline).
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn bt_insert_and_collect_matches_neon(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: &HcOptimalCostProfile,
        min_match_len: usize,
        best_len_for_skip: &mut usize,
        out: &mut Vec<MatchCandidate>,
        reps: [u32; 3],
        lit_len: usize,
        use_hash3: bool,
    ) {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_and_collect_matches_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            profile,
            min_match_len,
            best_len_for_skip,
            out,
            reps,
            lit_len,
            use_hash3,
            crate::encoding::fastpath::neon::common_prefix_len_ptr,
            crate::encoding::fastpath::neon::count_match_from_indices,
        )
    }

    /// SSE4.2 umbrella variant of `bt_insert_and_collect_matches`.
    ///
    /// # Safety
    /// x86/x86_64 with SSE4.2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn bt_insert_and_collect_matches_sse42(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: &HcOptimalCostProfile,
        min_match_len: usize,
        best_len_for_skip: &mut usize,
        out: &mut Vec<MatchCandidate>,
        reps: [u32; 3],
        lit_len: usize,
        use_hash3: bool,
    ) {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_and_collect_matches_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            profile,
            min_match_len,
            best_len_for_skip,
            out,
            reps,
            lit_len,
            use_hash3,
            crate::encoding::fastpath::sse42::common_prefix_len_ptr,
            crate::encoding::fastpath::sse42::count_match_from_indices,
        )
    }

    /// AVX2+BMI2 umbrella variant of `bt_insert_and_collect_matches`.
    ///
    /// # Safety
    /// x86/x86_64 with AVX2 + BMI2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn bt_insert_and_collect_matches_avx2_bmi2(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: &HcOptimalCostProfile,
        min_match_len: usize,
        best_len_for_skip: &mut usize,
        out: &mut Vec<MatchCandidate>,
        reps: [u32; 3],
        lit_len: usize,
        use_hash3: bool,
    ) {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_and_collect_matches_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            profile,
            min_match_len,
            best_len_for_skip,
            out,
            reps,
            lit_len,
            use_hash3,
            crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
            crate::encoding::fastpath::avx2_bmi2::count_match_from_indices,
        )
    }

    /// Scalar fallback BT collect-matches walker.
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bt_insert_and_collect_matches_scalar(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: &HcOptimalCostProfile,
        min_match_len: usize,
        best_len_for_skip: &mut usize,
        out: &mut Vec<MatchCandidate>,
        reps: [u32; 3],
        lit_len: usize,
        use_hash3: bool,
    ) {
        let search_depth = self.search_depth;
        super::super::hc::generator::bt_insert_and_collect_matches_body!(
            self,
            search_depth,
            abs_pos,
            current_abs_end,
            profile,
            min_match_len,
            best_len_for_skip,
            out,
            reps,
            lit_len,
            use_hash3,
            crate::encoding::fastpath::scalar::common_prefix_len_ptr,
            crate::encoding::fastpath::scalar::count_match_from_indices,
        )
    }

    /// BT-side history replay after [`Self::begin_rebase`]. Re-walks
    /// `history_start..abs_pos` through the BT step so the pointer-pair
    /// table is consistent with the freshly reset `position_base`.
    pub(crate) fn replay_history_for_rebase_bt(&mut self, history_start: usize, abs_pos: usize) {
        let rebuild_end = self.history_abs_end();
        let mut pos = history_start;
        while pos < abs_pos {
            let forward = self.bt_insert_step_no_rebase(pos, rebuild_end, abs_pos);
            // `pos` is a frame-lifetime absolute cursor that can approach
            // `usize::MAX` on long 32-bit streams. Cap the step at the
            // remaining distance to `abs_pos` so the addition stays
            // within `usize` even when the BT walker returns a large
            // `forward` near the stream end.
            let step = forward.max(1).min(abs_pos - pos);
            pos += step;
        }
    }

    /// Stage D: BT-tree update dispatcher. Picks the kernel-specific
    /// variant so the per-iteration BT walker inlines under the
    /// surrounding `target_feature` umbrella.
    #[inline(always)]
    pub(crate) fn bt_update_tree_until(&mut self, abs_pos: usize, current_abs_end: usize) {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.bt_update_tree_until_neon(abs_pos, current_abs_end)
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.bt_update_tree_until_avx2_bmi2(abs_pos, current_abs_end)
                },
                FastpathKernel::Sse42 => unsafe {
                    self.bt_update_tree_until_sse42(abs_pos, current_abs_end)
                },
                FastpathKernel::Scalar => {
                    self.bt_update_tree_until_scalar(abs_pos, current_abs_end)
                }
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.bt_update_tree_until_scalar(abs_pos, current_abs_end)
        }
    }

    /// NEON-umbrella variant: per-iteration `bt_insert_step_no_rebase_neon`
    /// inlines into the body because both share the
    /// `target_feature = "neon"` umbrella.
    ///
    /// # Safety
    /// AArch64 with NEON (baseline).
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    pub(crate) unsafe fn bt_update_tree_until_neon(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
    ) {
        if self.skip_insert_until_abs < self.history_abs_start {
            self.skip_insert_until_abs = self.history_abs_start;
        }
        let mut update_abs = self.skip_insert_until_abs;
        let is_btultra2 = self.is_btultra2;
        while update_abs < abs_pos {
            if !self.can_skip_rebase_check_at(update_abs, abs_pos, is_btultra2) {
                self.maybe_rebase_positions(update_abs);
            }
            // SAFETY: same NEON umbrella; direct call inlines the BT-walk body.
            let forward =
                unsafe { self.bt_insert_step_no_rebase_neon(update_abs, current_abs_end, abs_pos) };
            // Upstream zstd `ZSTD_updateTree`: `idx += ZSTD_insertBt1(...)` with
            // NO clamp to the target. The insert step's `forward` skips the
            // positions a long match already covers, so letting it overshoot
            // `abs_pos` leaves those positions OUT of the tree exactly as C does
            // (`nextToUpdate = target` afterwards). Clamping to `abs_pos` inserted
            // those covered positions, giving our tree extra candidates C never
            // surfaces (e.g. a longer-but-farther match the optimal parser then
            // wrongly forces over a cheaper closer one).
            update_abs += forward.max(1);
        }
        self.skip_insert_until_abs = abs_pos;
    }

    /// SSE4.2 umbrella variant.
    ///
    /// # Safety
    /// x86/x86_64 with SSE4.2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    pub(crate) unsafe fn bt_update_tree_until_sse42(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
    ) {
        if self.skip_insert_until_abs < self.history_abs_start {
            self.skip_insert_until_abs = self.history_abs_start;
        }
        let mut update_abs = self.skip_insert_until_abs;
        let is_btultra2 = self.is_btultra2;
        while update_abs < abs_pos {
            if !self.can_skip_rebase_check_at(update_abs, abs_pos, is_btultra2) {
                self.maybe_rebase_positions(update_abs);
            }
            let forward = unsafe {
                self.bt_insert_step_no_rebase_sse42(update_abs, current_abs_end, abs_pos)
            };
            // Upstream zstd `ZSTD_updateTree`: `idx += ZSTD_insertBt1(...)` with
            // NO clamp to the target. The insert step's `forward` skips the
            // positions a long match already covers, so letting it overshoot
            // `abs_pos` leaves those positions OUT of the tree exactly as C does
            // (`nextToUpdate = target` afterwards). Clamping to `abs_pos` inserted
            // those covered positions, giving our tree extra candidates C never
            // surfaces (e.g. a longer-but-farther match the optimal parser then
            // wrongly forces over a cheaper closer one).
            update_abs += forward.max(1);
        }
        self.skip_insert_until_abs = abs_pos;
    }

    /// AVX2+BMI2 umbrella variant.
    ///
    /// # Safety
    /// x86/x86_64 with AVX2 + BMI2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    pub(crate) unsafe fn bt_update_tree_until_avx2_bmi2(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
    ) {
        if self.skip_insert_until_abs < self.history_abs_start {
            self.skip_insert_until_abs = self.history_abs_start;
        }
        let mut update_abs = self.skip_insert_until_abs;
        let is_btultra2 = self.is_btultra2;
        while update_abs < abs_pos {
            if !self.can_skip_rebase_check_at(update_abs, abs_pos, is_btultra2) {
                self.maybe_rebase_positions(update_abs);
            }
            let forward = unsafe {
                self.bt_insert_step_no_rebase_avx2_bmi2(update_abs, current_abs_end, abs_pos)
            };
            // Upstream zstd `ZSTD_updateTree`: `idx += ZSTD_insertBt1(...)` with
            // NO clamp to the target. The insert step's `forward` skips the
            // positions a long match already covers, so letting it overshoot
            // `abs_pos` leaves those positions OUT of the tree exactly as C does
            // (`nextToUpdate = target` afterwards). Clamping to `abs_pos` inserted
            // those covered positions, giving our tree extra candidates C never
            // surfaces (e.g. a longer-but-farther match the optimal parser then
            // wrongly forces over a cheaper closer one).
            update_abs += forward.max(1);
        }
        self.skip_insert_until_abs = abs_pos;
    }

    /// Scalar fallback used on non-AArch64 targets.
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    pub(crate) fn bt_update_tree_until_scalar(&mut self, abs_pos: usize, current_abs_end: usize) {
        if self.skip_insert_until_abs < self.history_abs_start {
            self.skip_insert_until_abs = self.history_abs_start;
        }
        let mut update_abs = self.skip_insert_until_abs;
        let is_btultra2 = self.is_btultra2;
        while update_abs < abs_pos {
            if !self.can_skip_rebase_check_at(update_abs, abs_pos, is_btultra2) {
                self.maybe_rebase_positions(update_abs);
            }
            let forward =
                self.bt_insert_step_no_rebase_scalar(update_abs, current_abs_end, abs_pos);
            // Upstream zstd `ZSTD_updateTree`: `idx += ZSTD_insertBt1(...)` with
            // NO clamp to the target. The insert step's `forward` skips the
            // positions a long match already covers, so letting it overshoot
            // `abs_pos` leaves those positions OUT of the tree exactly as C does
            // (`nextToUpdate = target` afterwards). Clamping to `abs_pos` inserted
            // those covered positions, giving our tree extra candidates C never
            // surfaces (e.g. a longer-but-farther match the optimal parser then
            // wrongly forces over a cheaper closer one).
            update_abs += forward.max(1);
        }
        self.skip_insert_until_abs = abs_pos;
    }

    /// Hash3-only fill up to (but not including) `abs_pos`. Rebase
    /// guard fires only when `can_skip_rebase_check_at` says we can't
    /// trivially skip — the fast path is a tight loop over `hash3_table`
    /// writes.
    pub(crate) fn update_hash3_until(&mut self, abs_pos: usize) {
        let is_btultra2 = self.is_btultra2;
        if self.next_to_update3 < self.history_abs_start {
            self.next_to_update3 = self.history_abs_start;
        }
        if self.next_to_update3 >= abs_pos {
            return;
        }
        while self.next_to_update3 < abs_pos {
            if !self.can_skip_rebase_check_at(self.next_to_update3, abs_pos, is_btultra2) {
                self.maybe_rebase_positions(self.next_to_update3);
            }
            self.insert_hash3_only_no_rebase(self.next_to_update3);
            // hash3 cursor strictly less than `abs_pos` here (loop guard);
            // `+ 1` cannot overflow within encode block sizes.
            self.next_to_update3 += 1;
        }
    }

    /// Hot wrapper for the rebase guard. Fast path is a single
    /// [`Self::needs_rebase`] check; the cold rebuild is a separate
    /// `#[cold]` function so the i-cache stays warm on the common
    /// "no rebase needed" branch.
    #[inline]
    pub(crate) fn maybe_rebase_positions(&mut self, abs_pos: usize) {
        let is_btultra2 = self.is_btultra2;
        if self.needs_rebase(abs_pos, is_btultra2) {
            self.rebase_positions_cold(abs_pos);
        }
    }

    /// Cold rebase: clear the hash / hash3 / chain tables and replay
    /// the inserted history prefix through the active backend's walker
    /// so the new `position_base` is consistent. The `uses_bt` flag
    /// (mirrored from `HcParseMode`) selects between the HC and BT
    /// replay variants.
    #[cold]
    #[inline(never)]
    pub(crate) fn rebase_positions_cold(&mut self, abs_pos: usize) {
        self.begin_rebase();
        let history_start = self.history_abs_start;
        // Rebuild only the already-inserted prefix. The caller inserts abs_pos
        // immediately after this, and later positions are added in-order.
        if self.uses_bt {
            self.replay_history_for_rebase_bt(history_start, abs_pos);
        } else {
            self.replay_history_for_rebase_hc(history_start, abs_pos);
        }
        // begin_rebase() also clears `hash3_table`. Rewind `next_to_update3`
        // to the inserted prefix start so `update_hash3_until` re-fills the
        // HC3 side table — without this, every HC3 probe up to `abs_pos`
        // returns "empty" until the encoder catches up, which silently
        // changes btultra2 short-match selection on long-running streams.
        self.next_to_update3 = history_start;
        if self.hash3_log != 0 {
            self.update_hash3_until(abs_pos);
        }
    }

    /// Insert a single position into the hash / chain tables, rebasing
    /// first if required.
    #[inline]
    pub(crate) fn insert_position(&mut self, abs_pos: usize) {
        self.maybe_rebase_positions(abs_pos);
        self.insert_position_no_rebase(abs_pos);
    }

    /// Insert every position in `[start, end)` into the hash / chain
    /// tables and advance the hash3 fill cursor past `end`.
    ///
    /// The rebase guard is hoisted out of the per-position loop: a single
    /// `(rel + 1)`-representability check on both ends of the range decides
    /// whether the whole span fits without a rebase. The check is monotone
    /// in `pos` (rebase only fires as the relative position approaches
    /// `u32::MAX`, or at the reserved stream-origin `rel == 0`), so when
    /// neither end needs a rebase no interior position can either, and the
    /// fill runs as a tight loop with raw `(pos - position_base)` index
    /// arithmetic and hoisted base pointers. This mirrors the upstream zstd's
    /// once-per-block `ZSTD_window_correctOverflow` followed by an
    /// unchecked fill, and matters on highly repetitive inputs where a
    /// single long match makes this loop fill the entire block. When a
    /// rebase is required the cold per-position path runs unchanged.
    pub(crate) fn insert_positions(&mut self, start: usize, end: usize) {
        if start < end {
            let is_btultra2 = self.is_btultra2;
            if self.needs_rebase(start, is_btultra2) || self.needs_rebase(end - 1, is_btultra2) {
                // Cold path: at least one position in the range needs a
                // rebase. Defer to the guarded per-position insert, which
                // rebases exactly when each position requires it (including
                // the reserved `rel == 0` stream-origin skip).
                for pos in start..end {
                    self.insert_position(pos);
                }
            } else {
                self.fill_hash_chain_positions(start, end);
            }
        }
        self.next_to_update3 = self.next_to_update3.max(end);
    }

    /// Index a just-emitted match span with the upstream zstd skip-threshold cap
    /// (`ZSTD_row_update_internal`, `zstd_lazy.c:922-940`): when the span
    /// exceeds `SKIP_THRESHOLD` positions only the first `MAX_START` and last
    /// `MAX_END` are chained, the interior is skipped. Indexing every interior
    /// byte of a long match is O(matchlen) and dominates HashChain (lazy)
    /// encode time on periodic inputs where one match can span a whole block;
    /// upstream zstd's hash-chain finder chains nothing in the interior at all, so the
    /// 96 + 32 cap is a conservative (ratio-preserving) mirror that still keeps
    /// boundary anchors for the following search.
    pub(crate) fn insert_match_span(&mut self, start: usize, end: usize) {
        const SKIP_THRESHOLD: usize = 384;
        const MAX_START: usize = 96;
        const MAX_END: usize = 32;
        if end.saturating_sub(start) > SKIP_THRESHOLD {
            // Raw arithmetic is correct by design here, NOT masked with
            // `saturating_*`. `start` / `end` are absolute stream
            // positions, and `check_stream_abs_headroom` guarantees
            // `abs_pos + STREAM_ABS_HEADROOM (= 4112) <= usize::MAX` for
            // every position in the frame, so `start + MAX_START` (96)
            // cannot overflow even on a 32-bit target. In this branch
            // `end - start > SKIP_THRESHOLD (384)`, so `end > 384 >
            // MAX_END (32)` and `end - MAX_END` cannot underflow.
            self.insert_positions(start, start + MAX_START);
            self.insert_positions(end - MAX_END, end);
        } else {
            self.insert_positions(start, end);
        }
    }

    /// Tight hash/chain fill for `[start, end)` when the caller has already
    /// proven every position is `(rel + 1)`-representable (so no rebase and
    /// no `rel == 0` skip can occur). Equivalent to looping
    /// [`Self::insert_position_no_rebase`], but with the table base pointers
    /// and config hoisted out of the loop and the relative position derived
    /// by a raw subtraction instead of the checked `relative_position`
    /// arithmetic. Upstream zstd parity: the unchecked `ZSTD_insertAndFindFirstIndex`
    /// fill body.
    #[inline]
    fn fill_hash_chain_positions(&mut self, start: usize, end: usize) {
        let history_abs_start = self.history_abs_start;
        let position_base = self.position_base;
        let index_shift = self.index_shift;
        let hash_log = self.hash_log;
        let chain_mask = (1usize << self.chain_log) - 1;
        // Borrowed-aware source: `live_history()` is the borrowed input window
        // in no-copy mode (owned mirror empty) and `history[history_start..]`
        // otherwise. Reading `self.history` directly would hash the empty
        // mirror on the borrowed HC lazy/greedy path. The raw ptr is copied
        // out so it holds no borrow across the `hash_table` / `chain_table`
        // mutations below; the window stays valid for the loop (borrowed input
        // is live by contract, owned `history` is not realloc'd mid-fill).
        let (concat_ptr, concat_len) = {
            let live = self.live_history();
            (live.as_ptr(), live.len())
        };
        let hash_ptr = self.hash_table.as_mut_ptr();
        let chain_ptr = self.chain_table.as_mut_ptr();
        debug_assert!(self.hash_table.len() == 1usize << hash_log);
        debug_assert!(self.chain_table.len() == 1usize << self.chain_log);
        // The last `< 4` bytes of the live window can't be hashed. Compute
        // the hashable upper bound once instead of branching per position: a
        // position `pos` is hashable iff `pos - history_abs_start + 4 <=
        // concat_len`, i.e. `pos < history_abs_start + (concat_len - 3)`.
        let hashable_end = end.min(history_abs_start + concat_len.saturating_sub(3));
        if start >= hashable_end {
            return;
        }
        // Hoist the source pointer and relative index out of the loop and
        // advance both by one per iteration, mirroring the upstream zstd's
        // `ip++ / idx++` fill rather than recomputing them from `pos`.
        let mut src = unsafe { concat_ptr.add(start - history_abs_start) };
        // `rel` cannot reach `u32::MAX` because the caller proved
        // `!needs_rebase(end - 1)`; `wrapping_add` keeps the overflow branch
        // off this per-byte hot loop.
        let mut rel = (start + index_shift - position_base) as u32;
        for _ in start..hashable_end {
            // SAFETY: every `src` in `[start, hashable_end)` is at least 4
            // bytes from the end of `history`, so the unaligned 4-byte read is
            // in range. `hash` is masked to `hash_log` bits and `chain_idx` to
            // `chain_log` bits, both within the table lengths asserted above.
            unsafe {
                let value = Self::read_le_u32_ptr(src);
                let hash = Self::hash_value_with_mls(value, hash_log, 4);
                let chain_idx = (rel as usize) & chain_mask;
                let prev = *hash_ptr.add(hash);
                *chain_ptr.add(chain_idx) = prev;
                *hash_ptr.add(hash) = rel + 1;
                src = src.add(1);
            }
            rel = rel.wrapping_add(1);
        }
    }

    /// Insert every `step`-th position in `[start, end)` — the sparse
    /// counterpart to [`Self::insert_positions`]. Skipped positions are
    /// *not* advanced through `next_to_update3` (the upstream zstd's behaviour
    /// for the "incompressible block" skip path).
    pub(crate) fn insert_positions_with_step(&mut self, start: usize, end: usize, step: usize) {
        if step == 0 {
            return;
        }
        let mut pos = start;
        while pos < end {
            self.insert_position(pos);
            let next = pos.saturating_add(step);
            if next <= pos {
                break;
            }
            pos = next;
        }
    }

    /// Backfill the last `< 4` bytes of the previous slice (which couldn't
    /// be hashed at the time because `insert_position` needs 4 bytes of
    /// lookahead) before the next slice's matching pass.
    pub(crate) fn backfill_boundary_positions(
        &mut self,
        current_abs_start: usize,
        current_abs_end: usize,
    ) {
        let backfill_start = current_abs_start
            .saturating_sub(3)
            .max(self.history_abs_start);
        if backfill_start < current_abs_start {
            if self.uses_bt {
                self.bt_update_tree_until(current_abs_start, current_abs_end);
            } else {
                self.insert_positions(backfill_start, current_abs_start);
            }
        }
    }

    /// After a long BT match the optimal parser may have skipped a huge
    /// stretch of `skip_insert_until_abs`. Cap the gap at 384 to bound
    /// the worst-case BT tree update on the next match search.
    pub(crate) fn apply_limited_update_after_long_match(&mut self, current_abs_start: usize) {
        if !self.uses_bt {
            return;
        }
        let gap = current_abs_start.saturating_sub(self.skip_insert_until_abs);
        if gap > 384 {
            self.skip_insert_until_abs = current_abs_start - (gap - 384).min(192);
        }
    }

    /// BT-mode counterpart of the HC sparse skip path. Inserts every
    /// `INCOMPRESSIBLE_SKIP_STEP`-th position through the BT walker
    /// (which threads the binary tree) and then densely inserts the
    /// final `HC_MIN_MATCH_LEN + INCOMPRESSIBLE_SKIP_STEP` positions so
    /// the very last hashable starts are present for the next slice.
    pub(crate) fn bt_insert_sparse_incompressible_block(
        &mut self,
        current_abs_start: usize,
        current_abs_end: usize,
    ) {
        let mut pos = current_abs_start;
        while pos < current_abs_end {
            self.maybe_rebase_positions(pos);
            let _ = self.bt_insert_step_no_rebase(pos, current_abs_end, current_abs_end);
            self.insert_hash3_only_no_rebase(pos);
            let next = pos.saturating_add(INCOMPRESSIBLE_SKIP_STEP);
            if next <= pos {
                break;
            }
            pos = next;
        }

        let dense_tail = HC_MIN_MATCH_LEN + INCOMPRESSIBLE_SKIP_STEP;
        let tail_start = current_abs_end
            .saturating_sub(dense_tail)
            .max(self.history_abs_start)
            .max(current_abs_start);
        for pos in tail_start..current_abs_end {
            if (pos - current_abs_start).is_multiple_of(INCOMPRESSIBLE_SKIP_STEP) {
                continue;
            }
            self.maybe_rebase_positions(pos);
            let _ = self.bt_insert_step_no_rebase(pos, current_abs_end, current_abs_end);
            self.insert_hash3_only_no_rebase(pos);
        }

        self.skip_insert_until_abs = self.skip_insert_until_abs.max(current_abs_end);
        self.next_to_update3 = self.next_to_update3.max(current_abs_end);
    }

    /// `skip_matching` body — backfill the slice boundary and then walk
    /// the current slice in either dense or sparse mode (driven by the
    /// `incompressible_hint`). BT and HC modes branch via `uses_bt`.
    /// Dict-priming for BT / optimal levels (upstream zstd `ZSTD_dictMatchState`):
    /// the dictionary content stays in `history` (so the dms chain can read it
    /// and offsets are computed across the dict→input boundary) but is NOT
    /// inserted into the LIVE binary tree — the upstream zstd keeps the dictionary in a
    /// SEPARATE matchState, so the live tree searches only the input. Advancing
    /// the BT (`skip_insert_until_abs`) and hash3 cursors past the committed
    /// dict block makes the first input block's tree update start after it,
    /// leaving the live tree input-only (the dms is the sole dict source).
    pub(crate) fn skip_matching_dict_bt(&mut self) {
        self.ensure_tables();
        let committed_end = self.history_abs_start + self.window_size;
        self.skip_insert_until_abs = self.skip_insert_until_abs.max(committed_end);
        self.next_to_update3 = self.next_to_update3.max(committed_end);
    }

    pub(crate) fn skip_matching(&mut self, incompressible_hint: Option<bool>) {
        self.ensure_tables();
        let (current_abs_start, current_len) = self.current_block_range();
        let current_abs_end = current_abs_start + current_len;
        self.backfill_boundary_positions(current_abs_start, current_abs_end);
        if self.uses_bt {
            if incompressible_hint == Some(true) {
                self.bt_insert_sparse_incompressible_block(current_abs_start, current_abs_end);
                return;
            }
            self.bt_update_tree_until(current_abs_end, current_abs_end);
            return;
        }
        if incompressible_hint == Some(true) {
            self.insert_positions_with_step(
                current_abs_start,
                current_abs_end,
                INCOMPRESSIBLE_SKIP_STEP,
            );
            let dense_tail = HC_MIN_MATCH_LEN + INCOMPRESSIBLE_SKIP_STEP;
            let tail_start = current_abs_end
                .saturating_sub(dense_tail)
                .max(self.history_abs_start);
            let tail_start = tail_start.max(current_abs_start);
            for pos in tail_start..current_abs_end {
                if !(pos - current_abs_start).is_multiple_of(INCOMPRESSIBLE_SKIP_STEP) {
                    self.insert_position(pos);
                }
            }
        } else {
            self.insert_positions(current_abs_start, current_abs_end);
        }
    }

    /// Stage D: HC3 short-match probe. Cross-platform dispatcher.
    /// External / test callers only — the on-encode hot path bypasses
    /// this via the per-kernel variant from inside the surrounding
    /// `collect_optimal_candidates_initialized_<kernel>` umbrella.
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn hash3_candidate(
        &self,
        abs_pos: usize,
        current_abs_end: usize,
        min_match_len: usize,
    ) -> Option<MatchCandidate> {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.hash3_candidate_neon(abs_pos, current_abs_end, min_match_len)
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.hash3_candidate_avx2_bmi2(abs_pos, current_abs_end, min_match_len)
                },
                FastpathKernel::Sse42 => unsafe {
                    self.hash3_candidate_sse42(abs_pos, current_abs_end, min_match_len)
                },
                FastpathKernel::Scalar => {
                    self.hash3_candidate_scalar(abs_pos, current_abs_end, min_match_len)
                }
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.hash3_candidate_scalar(abs_pos, current_abs_end, min_match_len)
        }
    }

    /// NEON umbrella HC3 probe.
    ///
    /// # Safety
    /// AArch64 with NEON (baseline). Body inlines via macro.
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    pub(crate) unsafe fn hash3_candidate_neon(
        &self,
        abs_pos: usize,
        current_abs_end: usize,
        min_match_len: usize,
    ) -> Option<MatchCandidate> {
        super::super::hc::generator::hash3_candidate_body!(
            self,
            abs_pos,
            current_abs_end,
            min_match_len,
            crate::encoding::fastpath::neon::common_prefix_len_ptr,
        )
    }

    /// SSE4.2 umbrella HC3 probe.
    ///
    /// # Safety
    /// x86/x86_64 with SSE4.2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    pub(crate) unsafe fn hash3_candidate_sse42(
        &self,
        abs_pos: usize,
        current_abs_end: usize,
        min_match_len: usize,
    ) -> Option<MatchCandidate> {
        super::super::hc::generator::hash3_candidate_body!(
            self,
            abs_pos,
            current_abs_end,
            min_match_len,
            crate::encoding::fastpath::sse42::common_prefix_len_ptr,
        )
    }

    /// AVX2+BMI2 umbrella HC3 probe.
    ///
    /// # Safety
    /// x86/x86_64 with AVX2 + BMI2.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    pub(crate) unsafe fn hash3_candidate_avx2_bmi2(
        &self,
        abs_pos: usize,
        current_abs_end: usize,
        min_match_len: usize,
    ) -> Option<MatchCandidate> {
        super::super::hc::generator::hash3_candidate_body!(
            self,
            abs_pos,
            current_abs_end,
            min_match_len,
            crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
        )
    }

    /// Scalar fallback HC3 probe (used on non-AArch64 targets).
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    pub(crate) fn hash3_candidate_scalar(
        &self,
        abs_pos: usize,
        current_abs_end: usize,
        min_match_len: usize,
    ) -> Option<MatchCandidate> {
        super::super::hc::generator::hash3_candidate_body!(
            self,
            abs_pos,
            current_abs_end,
            min_match_len,
            crate::encoding::fastpath::scalar::common_prefix_len_ptr,
        )
    }

    /// Reset the rebase-derived bookkeeping (rolling `position_base` /
    /// `index_shift`) so every stored position re-encodes from
    /// `history_abs_start`, then clear the three index tables. Hot
    /// path for [`Self::rebase_positions_cold`]; the caller is
    /// responsible for re-inserting any positions the active
    /// matchfinder still needs.
    pub(crate) fn begin_rebase(&mut self) {
        self.position_base = self.history_abs_start;
        self.index_shift = 0;
        self.allow_zero_relative_position = true;
        self.hash_table.fill(HC_EMPTY);
        self.hash3_table.fill(HC_EMPTY);
        self.chain_table.fill(HC_EMPTY);
    }

    /// HC-side history replay after [`begin_rebase`]. Re-inserts every
    /// position from `history_start` (inclusive) to `abs_pos`
    /// (exclusive) into the HC chain/hash tables without re-checking
    /// the rebase guard — the caller has just rebased, so positions
    /// are by construction representable.
    pub(crate) fn replay_history_for_rebase_hc(&mut self, history_start: usize, abs_pos: usize) {
        for pos in history_start..abs_pos {
            self.insert_position_no_rebase(pos);
        }
    }

    /// Upstream zstd parity: replay an optimal-parser plan into the consumer's
    /// sequence sink. Reads the current input frame off `window` and
    /// advances `offset_hist` exactly like the upstream zstd block-store walker.
    pub(crate) fn emit_optimal_plan(
        &mut self,
        current_len: usize,
        plan: &[HcOptimalSequence],
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        // Current block bytes. `get_last_space()` is borrowed-aware (owned:
        // last committed chunk = `history[len-current_len..]`, byte-identical;
        // borrowed: the staged in-place block). Reborrow-then-raw-ptr so the
        // slice holds NO borrow and the per-sequence `&mut self.offset_hist`
        // write below stays valid (the old direct `&self.history[..]` slice
        // underflowed on the borrowed path, where `history` is empty).
        let current: &[u8] = unsafe {
            let ls = self.get_last_space();
            debug_assert!(
                current_len <= ls.len(),
                "current_len ({current_len}) exceeds block size ({})",
                ls.len()
            );
            core::slice::from_raw_parts(ls.as_ptr(), current_len)
        };
        if plan.is_empty() {
            handle_sequence(Sequence::Literals { literals: current });
            return;
        }

        let mut literals_start = 0usize;
        for item in plan {
            let lit_len = item.lit_len as usize;
            let match_len = item.match_len as usize;
            // checked_add on both edges — mirrors
            // `BtMatcher::update_plan_stats_segment`. A malformed plan
            // entry can't overflow `usize` arithmetic before the
            // `> current_len` guard fires.
            let Some(start) = literals_start.checked_add(lit_len) else {
                continue;
            };
            let Some(end) = start.checked_add(match_len) else {
                continue;
            };
            if end > current_len {
                continue;
            }
            let literals = &current[literals_start..start];
            handle_sequence(Sequence::Triple {
                literals,
                offset: item.offset as usize,
                match_len,
            });
            encode_offset_with_history(item.offset, literals.len() as u32, &mut self.offset_hist);
            literals_start = end;
        }

        if literals_start < current_len {
            handle_sequence(Sequence::Literals {
                literals: &current[literals_start..],
            });
        }
    }
}
