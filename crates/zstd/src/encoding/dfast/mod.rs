#![allow(
    clippy::cast_lossless,
    clippy::empty_line_after_doc_comments,
    clippy::inline_always,
    clippy::manual_let_else,
    clippy::ptr_as_ptr,
    clippy::semicolon_if_nothing_returned,
    clippy::unreadable_literal,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Double-fast match finder (default-level backend, upstream zstd parity for
//! `ZSTD_dfast.c`). Two parallel hash chains — a 4-byte short hash and
//! an 8-byte long hash — feed an adaptive sparse search that bails out
//! when consecutive misses suggest an incompressible region.
//!
//! Shared with `match_generator.rs`
//! (structural split). Mechanical move — names, fields, method bodies,
//! constants, and the `#[inline]` annotations are preserved; the
//! visibility on the relocated items was opened to `pub(crate)` so
//! `match_generator` can keep dispatching to `DfastMatchGenerator`
//! through the `dfast::` import path.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::Sequence;
use super::blocks::encode_offset_with_history;
use super::dict_attach::DictAttach;
use super::fastpath::{FastpathKernel, select_kernel};
use super::incompressible::block_looks_incompressible;
use super::levels::config::MIN_WINDOW_LOG;
use super::match_generator::{
    DFAST_EMPTY_SLOT, DFAST_HASH_BITS, DFAST_INCOMPRESSIBLE_SKIP_STEP, DFAST_MIN_MATCH_LEN,
    DFAST_REBASE_GUARD_BAND, DFAST_SHORT_HASH_BITS_DELTA, DFAST_SHORT_HASH_LOOKAHEAD,
    DFAST_SKIP_STEP_GROWTH_INTERVAL,
};
use super::match_table::helpers::{common_prefix_len_with_kernel, extend_backwards_shared};
use super::match_table::storage::{REBASE_RESET_FLOOR_CEILING, check_stream_abs_headroom};
use super::opt::types::MatchCandidate;

/// Upstream zstd `HASH_READ_SIZE` (`zstd_compress_internal.h`): the largest probe
/// width any hash / equality check in the dfast hot path reads at once.
/// Loop guards must stop scanning when fewer than `HASH_READ_SIZE` bytes
/// remain ahead of the probe cursor, matching upstream zstd `ilimit = iend -
/// HASH_READ_SIZE`. The `DFAST_MIN_MATCH_LEN = 5` floor is the match
/// acceptance threshold, NOT a safe loop bound — using it as the loop
/// guard reads up to 3 bytes past the live history end and is UB on a
/// raw pointer load.
const HASH_READ_SIZE: usize = 8;

/// Rep-extension minimum match length. The upstream zstd 1.5.7
/// reference (`zstd_double_fast.c:191`, rep1 emit
/// `ZSTD_count(ip+1+4, ip+1+4-offset_1, iend) + 4`) accepts 4-byte
/// rep hits even though hash-search matches require 5 bytes
/// (`DFAST_MIN_MATCH_LEN`) — rep coding has no on-wire offset cost,
/// so a 4-byte rep is a net win over re-running the hash probe. Both
/// the fast-loop inline rep1 peek and the post-match
/// `extend_with_repcode_after_match` chain must gate on this floor;
/// using the hash-search floor on either site silently drops 4-byte
/// rep emissions that upstream produces.
const DFAST_REP_MIN_MATCH_LEN: usize = 4;

/// Cached `DFTRACE` env flag for the dfast commit-path diagnostic (read once;
/// see the `DFTRACE` gate in the fast-loop commit handler).

static DFTRACE_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

#[derive(Clone)]
pub(crate) struct DfastMatchGenerator {
    pub(crate) max_window_size: usize,
    /// Per-block length queue. Previously held the raw input
    /// `VecDeque<Vec<u8>>` for each appended block — that duplicated
    /// every byte in `history`, doubling the input footprint relative
    /// to upstream zstd on the hot path. Now stores only the lengths so the
    /// matcher can still pop blocks block-by-block on eviction
    /// (advancing `history_start` by each pop) while the actual byte
    /// storage lives once, in `history`.
    pub(crate) window_blocks: VecDeque<usize>,
    pub(crate) window_size: usize,
    // We keep a contiguous searchable history to avoid rebuilding and reseeding
    // the matcher state from disjoint block buffers on every block.
    pub(crate) history: Vec<u8>,
    pub(crate) history_start: usize,
    pub(crate) history_abs_start: usize,
    pub(crate) offset_hist: [u32; 3],
    // Storage: single `u32` per bucket — upstream zstd-parity overwrite-on-
    // collision. Each slot holds a +1-biased relative position
    // (`(abs_pos - position_base + 1) as u32`); `DFAST_EMPTY_SLOT = 0`
    // is therefore never a real value. The two tables are sized
    // independently: `long_hash` (8-byte hash) uses `long_hash_bits`
    // (upstream zstd `hashTable`); `short_hash` (4-byte hash) uses
    // `short_hash_bits` = `long - 1` (upstream zstd `chainTable` for dfast).
    // Upstream zstd parity at Level 3: `2^17 × 4 + 2^16 × 4 = 768 KiB`. The
    // ratio loss from single-slot is compensated by upstream zstd's
    // `_search_next_long` retry — after a short-hash hit, the search
    // probes long_hash at `ip + 1` and picks the longer of the two
    // (see `hash_candidate`, invoked via `best_match`, for the retry).
    /// Single backing allocation for both hash tables (cwksp analog): the long
    /// table occupies `[0, long_len)` and the short table `[long_len,
    /// long_len + short_len)`. One allocation instead of two halves the
    /// per-fresh-frame large-table allocator churn (the page-fault / calloc
    /// storm that dominated dfast on medium inputs). Access through the
    /// `long_*` / `short_*` helpers, which apply the short-region offset.
    pub(crate) tables: Vec<u32>,
    /// Absolute position whose `(abs_pos - position_base + 1)` slot
    /// encoding evaluates to `1`. Advances only via [`Self::reduce`]
    /// when an insert is about to overflow the u32 window — the
    /// frame-level `STREAM_ABS_HEADROOM` gate already bounds
    /// `history_abs_start` against `usize::MAX`, so a rebase trigger
    /// here only fires on encoder sessions that span more than
    /// `u32::MAX - DFAST_REBASE_GUARD_BAND ≈ 3 GiB` of input through
    /// a single matcher instance. Upstream zstd parity: `ZSTD_window_reduce`
    /// (`zstd_compress_internal.h`).
    pub(crate) position_base: usize,
    /// Long-hash table bit-width — `long_hash.len() == 1 <<
    /// long_hash_bits`. Upstream zstd parity with `cParams.hashLog` (17 for
    /// Level 3 large input, 16 for Level 2; see `clevels.h`).
    pub(crate) long_hash_bits: usize,
    /// Short-hash table bit-width — `short_hash.len() == 1 <<
    /// short_hash_bits`. Default is `long_hash_bits -
    /// DFAST_SHORT_HASH_BITS_DELTA`, upstream zstd parity with
    /// `cParams.chainLog` for dfast levels (one bit smaller than the
    /// long hash). Halves the short-table footprint without losing
    /// measurable ratio — the 4-byte short hash overwrites less
    /// frequently than the 8-byte long hash on average, so the
    /// smaller bucket count is the upstream zstd-correct sizing.
    pub(crate) short_hash_bits: usize,
    /// Immutable dictionary long+short hash tables (upstream zstd `dictMatchState`)
    /// plus the CDict cache lifecycle, via the shared [`DictAttach`] level-1
    /// scaffolding. Built once over the dictionary region at the front of the
    /// contiguous history (flat `[dict][input]` model like the Fast backend:
    /// a dict match is `offset = ip - dict_pos` and counts cross the
    /// dict→input boundary, no `dictBase`/`dictIndexDelta`). Slots store the
    /// dict position as a +1-biased CONCAT index (offset into
    /// `history[history_start..]`), NOT the live tables' `position_base`-packed
    /// encoding — the dict table is never rebased, so it keys off the stable
    /// concat coordinate and is invalidated on any history eviction (which
    /// would slide the dict out of the concat) so positions never go stale.
    /// `is_attached()` activates the dual-probe kernel; `region_len()` is the
    /// dict/input boundary (`dict_end`, a concat index).
    pub(crate) dict: DictAttach<DfastDictTables>,
    /// CPU kernel for the `common_prefix_len` / repcode byte-compares,
    /// resolved once at construction so the per-byte match-finder scans skip
    /// the `select_kernel()` `OnceLock` atomic on every call.
    pub(crate) kernel: FastpathKernel,
    /// Borrowed one-shot input window, registered once per frame:
    /// `(input_base_ptr, total_len)`. When `Some`, the fast loop sources
    /// bytes from the caller's in-place input slice with absolute input
    /// positions (`abs_start = position_base = start_offset = 0`) instead of
    /// copying each block into the owned `history` concat — the dfast analog
    /// of the Fast backend's borrowed window. `None` = owned (history-copying)
    /// path. Cleared on `reset()`.
    pub(crate) borrowed_input: Option<(*const u8, usize)>,
    /// Active borrowed block range `[block_start, block_end)` (absolute input
    /// offsets), re-staged before each block scan so `scan_source` bounds the
    /// readable length to `block_end` (byte-identical match window to the
    /// owned evicting path) and `get_last_space` reports the borrowed block
    /// to the emit pipeline. Only meaningful while `borrowed_input` is `Some`.
    pub(crate) borrowed_block: Option<(usize, usize)>,
    /// Set by [`Self::reset`] when it re-borrowed a resident attach-mode
    /// dictionary (kept the dict bytes at the front of history + the cached dict
    /// tables in place instead of clearing + re-committing them). Signals the
    /// frame compressor to SKIP `prime_with_dictionary` this frame.
    pub(crate) dict_resident: bool,
}

/// The dfast backend's immutable dictionary tables — a long+short pair mirroring
/// the live [`DfastMatchGenerator::long_hash`] / [`DfastMatchGenerator::short_hash`]
/// shapes, sized to the same `(long_hash_bits, short_hash_bits)`. Held by the
/// shared [`DictAttach`] level-1 lifecycle; the per-tier dual-probe LOOKUP is
/// level-2 in this backend's kernel. Slots hold a +1-biased concat index
/// (`DFAST_EMPTY_SLOT = 0` is "no entry").
#[derive(Debug, Default, Clone)]
pub(crate) struct DfastDictTables {
    pub(crate) long: Vec<u32>,
    pub(crate) short: Vec<u32>,
}

impl DfastMatchGenerator {
    // Keep a short dense tail at block boundaries for two related reasons:
    // 1) insert_position() needs short (4-byte) and long (8-byte) lookahead,
    //    so appending a new block can make starts from the previous block newly
    //    hashable and require backfill;
    // 2) we also need enough trailing bytes from the previous block to preserve
    //    cross-block matching for the minimum match length.
    pub(crate) const BOUNDARY_DENSE_TAIL_LEN: usize = DFAST_MIN_MATCH_LEN + 3;

    pub(crate) fn new(max_window_size: usize) -> Self {
        Self {
            max_window_size,
            window_blocks: VecDeque::new(),
            window_size: 0,
            history: Vec::new(),
            history_start: 0,
            history_abs_start: 0,
            offset_hist: [1, 4, 8],
            tables: Vec::new(),
            position_base: 0,
            long_hash_bits: DFAST_HASH_BITS,
            short_hash_bits: DFAST_HASH_BITS - DFAST_SHORT_HASH_BITS_DELTA,
            dict: DictAttach::new(),
            kernel: select_kernel(),
            borrowed_input: None,
            borrowed_block: None,
            dict_resident: false,
        }
    }

    /// Whether the last [`Self::reset`] re-borrowed a resident dictionary (kept
    /// the dict bytes + cached dict tables in place). The driver reports this up
    /// so the frame compressor skips `prime_with_dictionary` for the frame.
    pub(crate) fn dict_resident(&self) -> bool {
        self.dict_resident
    }

    /// Number of slots in the long hash table (`[0, long_len)` of `tables`).
    #[inline(always)]
    pub(crate) fn long_len(&self) -> usize {
        1usize << self.long_hash_bits
    }

    /// Number of slots in the short hash table (`[long_len, +short_len)`).
    #[inline(always)]
    pub(crate) fn short_len(&self) -> usize {
        1usize << self.short_hash_bits
    }

    /// Base const-pointer to the long table region.
    #[inline(always)]
    fn long_ptr(&self) -> *const u32 {
        self.tables.as_ptr()
    }

    /// Base const-pointer to the short table region (offset past the long).
    #[inline(always)]
    fn short_ptr(&self) -> *const u32 {
        // SAFETY: `tables.len() == long_len + short_len`, so `long_len` is in
        // bounds (one past the long region = start of the short region).
        unsafe { self.tables.as_ptr().add(self.long_len()) }
    }

    /// Base mut-pointer to the long table region.
    #[inline(always)]
    fn long_mut_ptr(&mut self) -> *mut u32 {
        self.tables.as_mut_ptr()
    }

    /// Base mut-pointer to the short table region (offset past the long).
    #[inline(always)]
    fn short_mut_ptr(&mut self) -> *mut u32 {
        let off = self.long_len();
        // SAFETY: as `short_ptr`; `off == long_len` is in bounds.
        unsafe { self.tables.as_mut_ptr().add(off) }
    }

    /// Set both hash table sizes from the per-level [`DfastConfig`]:
    /// `long_bits` = upstream zstd `cParams.hashLog`, `short_bits` = upstream zstd
    /// `cParams.chainLog`. Both clamps stay above `MIN_WINDOW_LOG` so very
    /// small windows don't underflow. The caller already caps `long_bits` by
    /// the source-size window when hinted, so no upper clamp is applied here.
    pub(crate) fn set_hash_bits(&mut self, long_bits: usize, short_bits: usize) {
        let min_bits = MIN_WINDOW_LOG as usize;
        let long_clamped = long_bits.max(min_bits);
        let short_clamped = short_bits.max(min_bits);
        let resized = self.long_hash_bits != long_clamped || self.short_hash_bits != short_clamped;
        if resized {
            self.long_hash_bits = long_clamped;
            self.short_hash_bits = short_clamped;
            // Drop the combined backing so `ensure_hash_tables` reallocates at
            // the new (long_len + short_len).
            self.tables = Vec::new();
        }
        if resized {
            // A table-size change makes the cached dict tables (sized to the old
            // bits) and their hash indices invalid — drop the attach so the next
            // prime rebuilds at the new shape.
            self.dict.invalidate();
        }
    }

    /// Encode an absolute position into a u32 slot value
    /// (`(abs_pos - position_base + 1) as u32`). Caller must have
    /// invoked [`Self::ensure_room_for`] earlier in the same frame so
    /// the relative offset is guaranteed to fit in `u32`.
    ///
    /// # Panics
    ///
    /// Panics if `abs_pos < position_base` (producer bug — a position
    /// before the current rebase base should have been filtered out
    /// before reaching the table) or if the relative offset exceeds
    /// `u32::MAX`. Runtime `assert!` rather than `debug_assert!`: a
    /// silent wrap would store a garbage relative offset and corrupt
    /// the bucket far from the bug's source.
    #[inline]
    pub(crate) fn pack_slot(&self, abs_pos: usize) -> u32 {
        let rel = abs_pos.checked_sub(self.position_base).unwrap_or_else(|| {
            panic!(
                "DfastMatchGenerator::pack_slot: abs_pos {abs_pos} below \
                 position_base {} — caller must filter pre-rebase positions",
                self.position_base
            )
        });
        assert!(
            rel < u32::MAX as usize,
            "DfastMatchGenerator::pack_slot: rel {rel} >= u32::MAX — \
             caller must invoke ensure_room_for before insert"
        );
        (rel as u32) + 1
    }

    /// Ensure that an absolute position `abs_pos` fits in the `u32`
    /// slot encoding when packed. If the relative offset would
    /// exceed `u32::MAX - DFAST_REBASE_GUARD_BAND`, advance the base
    /// by `DFAST_REBASE_GUARD_BAND` (in a loop, in case the caller
    /// jumped past multiple guard bands at once) and shift every
    /// stored slot down by the same amount. Mirrors
    /// `LdmHashTable::ensure_room_for` and the upstream zstd's
    /// `ZSTD_window_reduce` semantics.
    pub(crate) fn ensure_room_for(&mut self, abs_pos: usize) {
        if abs_pos < self.position_base {
            // Pre-base positions can't push us past the u32 ceiling.
            return;
        }
        let max_rel = u32::MAX as usize - DFAST_REBASE_GUARD_BAND as usize;
        while abs_pos - self.position_base > max_rel {
            self.reduce(DFAST_REBASE_GUARD_BAND);
        }
    }

    /// Subtract `reducer` from every stored slot value. Slots whose
    /// pre-shift value was `<= reducer` become the empty sentinel.
    /// Advance `position_base` by the same amount so future inserts
    /// continue from the rebased origin.
    fn reduce(&mut self, reducer: u32) {
        let shift_slots = |slots: &mut [u32]| {
            for slot in slots.iter_mut() {
                *slot = if *slot <= reducer {
                    DFAST_EMPTY_SLOT
                } else {
                    *slot - reducer
                };
            }
        };
        let long_len = self.long_len();
        let (long, short) = self.tables.split_at_mut(long_len);
        shift_slots(long);
        shift_slots(short);
        self.position_base += reducer as usize;
    }

    /// Heap bytes this matcher owns: history, the long/short hash tables, the
    /// window-block deque, and any attached dictionary tables.
    pub(crate) fn heap_size(&self) -> usize {
        let u32_sz = size_of::<u32>();
        self.window_blocks.capacity() * size_of::<usize>()
            + self.history.capacity()
            + self.tables.capacity() * u32_sz
            + self
                .dict
                .table()
                .map_or(0, |t| (t.long.capacity() + t.short.capacity()) * u32_sz)
    }

    pub(crate) fn reset(&mut self) {
        // Floor-advance reset, also used by `MatchTable`. Instead of
        // zeroing the long/short tables every frame (a memset proportional
        // to table size, ~25% of a small reused-context dfast frame),
        // advance the absolute-position floor past the previous frame's
        // end. Every slot-decode site rejects a candidate whose absolute
        // position is below `history_abs_start` before it dereferences a
        // history byte (audited: fast-loop long/short/long+1/repcode,
        // `probe_tail_ip0_only`, `probe_slot_match`), so the previous
        // frame's entries become unreachable without being cleared.
        // `position_base` is left untouched so stale slots still decode to
        // their (now sub-floor) absolute positions; `ensure_room_for` /
        // `reduce` keep the `u32` packing bounded as the cursor climbs.
        let next_floor = self.history_abs_start + (self.history.len() - self.history_start);
        self.offset_hist = [1, 4, 8];
        // Re-borrow: an attach-mode reused dict frame keeps its bytes resident at
        // the front of history (`[0, region)`) + the cached concat-keyed dict
        // tables, so the per-frame dict re-commit (the dominant ~37% memmove on
        // a profiled small dfast frame) is skipped — the frame compressor then
        // skips `prime_with_dictionary`. The floor-advance still rejects the
        // previous frame's INPUT (its abs falls below `next_floor`); the dict
        // matches come from the separate dict tables, which bypass the floor.
        // Gated on the dict being fully resident at `history_start == 0` and the
        // floor-advance staying bounded.
        let reborrow_region = if self.dict.is_primed()
            && self.history_start == 0
            && next_floor <= REBASE_RESET_FLOOR_CEILING
        {
            let r = self.dict.region_len();
            (r > 0 && self.history.len() >= r).then_some(r)
        } else {
            None
        };
        if let Some(region) = reborrow_region {
            // Keep `[0, region)` (the dict); drop the previous frame's input.
            self.history.truncate(region);
            self.window_size = region;
            self.window_blocks.clear();
            self.window_blocks.push_back(region);
            // Bump the eviction window by the dict size (clamped to
            // MAX_PRIMED_WINDOW_SIZE) so the resident dict + the next input both
            // stay — base is `1 << window_log` (<= 2^30 < the ceiling), so the
            // headroom subtraction can't underflow and the sum can't overflow.
            let headroom = crate::encoding::match_table::storage::MAX_PRIMED_WINDOW_SIZE
                - self.max_window_size;
            self.max_window_size += region.min(headroom);
            self.history_abs_start = next_floor;
            self.dict_resident = true;
        } else {
            self.window_size = 0;
            self.history.clear();
            self.history_start = 0;
            // Non-reborrow reset starts with an empty window; clear the block
            // ledger to match. (The reborrow branch above intentionally keeps a
            // single `[region]` entry for the resident dict — see below.)
            self.window_blocks.clear();
            self.dict_resident = false;
            if next_floor <= REBASE_RESET_FLOOR_CEILING {
                // Fast path: advance the floor; tables keep their contents (a
                // later `ensure_tables`/level change still reallocs them clean
                // if the dimensions changed). The dict tables key off stable
                // concat indices and are untouched here.
                self.history_abs_start = next_floor;
            } else {
                // Bounded fallback: rewind the cursor and zero the tables so
                // `history_abs_start` cannot climb toward `usize::MAX` (keeps
                // `check_stream_abs_headroom` satisfiable on 32-bit targets;
                // fires ~once per 2 GiB cumulative input there).
                self.history_abs_start = 0;
                self.position_base = 0;
                if !self.tables.is_empty() {
                    self.tables.fill(DFAST_EMPTY_SLOT);
                }
            }
        }
        // No Vec<u8> blocks to recycle: `add_data` returns each input
        // Vec to the caller eagerly via its own `reuse_space`, and the
        // history Vec is owned solely by the matcher. There is nothing
        // for an outer pool helper to do at reset time, so the dfast
        // signature does not take one (HC / Row do because they hold
        // per-block input Vecs internally; the dispatcher in
        // `match_generator.rs` resolves the per-backend shape).
        // NOTE: `window_blocks` is cleared per-branch above (the reborrow branch
        // keeps its `[region]` dict entry; the non-reborrow branch clears). It
        // must NOT be cleared unconditionally here — doing so dropped the
        // resident dict block while `window_size`/`history` still counted it,
        // desyncing the ledger so the next eviction popped the wrong block.
        // Drop any borrowed window: the input slice it pointed at does not
        // outlive the frame, and the next frame re-stages its own (or runs
        // the owned path). A stale pointer must never survive a reset.
        self.borrowed_input = None;
        self.borrowed_block = None;
    }

    /// Slice of bytes from the most recently appended block. Returns
    /// the trailing `last_block_len` bytes of `history`, or an empty
    /// slice if no block has been ingested yet.
    ///
    /// Mirrors the inline gate pattern used by `skip_matching` /
    /// `skip_matching_dense` / `start_matching` / `emit_candidate` /
    /// `emit_trailing_literals`: read
    /// `window_blocks.back().copied().unwrap_or(0)` and slice the
    /// trailing `last_len` bytes (which is empty when `last_len == 0`).
    /// All current external callers — streaming encoder, block
    /// compressor, per-level helpers — invoke this only after at
    /// least one `add_data`, but returning an empty slice on the
    /// empty case keeps the trait surface aligned with the internal
    /// usage and avoids a panic-vs-gate divergence that would
    /// surprise a future refactor consolidating the call sites.
    pub(crate) fn get_last_space(&self) -> &[u8] {
        if let (Some((ptr, _total)), Some((block_start, block_end))) =
            (self.borrowed_input, self.borrowed_block)
        {
            // Borrowed window: the active block is the in-place input range
            // `[block_start, block_end)`, staged before the scan so the emit
            // pipeline's pre-scan `get_last_space().len()` reserve is correct.
            // SAFETY: borrowed liveness contract; `block_start <= block_end <=
            // buffer len` (validated when staged).
            return unsafe {
                core::slice::from_raw_parts(ptr.add(block_start), block_end - block_start)
            };
        }
        let last_len = self.window_blocks.back().copied().unwrap_or(0);
        &self.history[self.history.len() - last_len..]
    }

    pub(crate) fn add_data(&mut self, data: Vec<u8>, mut reuse_space: impl FnMut(Vec<u8>)) {
        assert!(data.len() <= self.max_window_size);
        // Run the headroom check first so the safety invariant
        // (`history_abs_start + window_size + len + STREAM_ABS_HEADROOM
        // <= usize::MAX`) is enforced at the function boundary, not
        // hidden behind the empty-chunk short-circuit below. With
        // `data.len() == 0` the check is a cheap no-op on the cumulative
        // state today, but keeping the call here means the invariant
        // doesn't depend on "empty data implies nothing changes"
        // reasoning if a future change ever attaches side effects to
        // the empty path.
        check_stream_abs_headroom(self.history_abs_start, self.window_size, data.len());
        // Empty chunks have nothing to record: pushing a `0` into
        // `window_blocks` would let a streaming caller that flushes
        // empty chunks grow the deque without bound (`window_size`
        // stays unchanged so `trim_to_window` never has cause to
        // evict the zero-length entries). Hand the Vec straight back
        // to the pool and short-circuit.
        //
        // Side effect: this short-circuits BEFORE the eviction `while`
        // loop below. If a caller shrinks `max_window_size` and then
        // calls `add_data(vec![])` hoping to trigger trim, the trim
        // won't fire here. Use `trim_to_window` directly for that
        // case — it's the dedicated path for shedding retained bytes
        // and now actually frees the prefix (via `split_off`) instead
        // of leaving it pinned in the `history` allocation.
        //
        // In-tree caller audit (`grep -rn '\.commit_space(' src/encoding/`):
        // every production path that reaches the driver's
        // `commit_space` → `add_data` chain originates in
        // `levels/fastest.rs`'s block emitter, which produces
        // non-empty blocks gated by `should_emit_raw_fast_path` /
        // RLE-detect on the source bytes — none of them pass
        // `Vec::new()`. The streaming encoder's block-sourcing loop
        // also filters empty reads before forwarding to the matcher.
        // Tests are the only callers that exercise the empty path
        // explicitly, and the regression covers eviction-driven trim
        // semantics through `trim_to_window` directly. So this
        // behaviour change is observable only by a hypothetical
        // future caller that relies on `add_data(empty)` as a
        // side-effecting trim trigger — and we deliberately want
        // such a caller to use `trim_to_window` instead.
        if data.is_empty() {
            reuse_space(data);
            return;
        }
        if self.window_size + data.len() > self.max_window_size {
            // Eviction advances `history_start`, so the dict tables' concat
            // indices (primed at `history_start == 0`) no longer address the
            // dict bytes — drop the attach (dict ratio benefit lost once the
            // dict slides out of the window, like the Fast backend).
            self.dict.invalidate();
            // Cap the history buffer near the live window: reserve exactly
            // (window + window/4 + one block) once eviction starts so the Vec
            // grows linearly to that ceiling instead of power-of-two doubling
            // to ~2x window; `compact_history`'s quarter-window drain keeps len
            // under it, so the Vec never reallocates again. Small frames that
            // never fill the window keep their tight data-sized buffer.
            let target = self.max_window_size
                + (self.max_window_size >> 2)
                + crate::common::MAX_BLOCK_SIZE as usize;
            if target > self.history.len() && self.history.capacity() < target {
                self.history.reserve_exact(target - self.history.len());
            }
        }
        while self.window_size + data.len() > self.max_window_size {
            let removed_len = self.window_blocks.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
        self.compact_history();
        self.history.extend_from_slice(&data);
        self.window_size += data.len();
        self.window_blocks.push_back(data.len());
        // Eager Vec recycle: the only purpose of holding the input Vec
        // was to return it to the caller's pool on eviction. Now that
        // `history` owns the bytes, hand the Vec back immediately so
        // the pool grows on first add instead of waiting for window
        // overflow.
        reuse_space(data);
    }

    /// Trim retained blocks until the window fits `max_window_size`.
    ///
    /// Unlike `MatchGenerator::trim_to_window`,
    /// `RowMatchGenerator::trim_to_window`, and
    /// `HcMatchGenerator::trim_to_window`, this backend does NOT take a
    /// `reuse_space` callback because it doesn't retain per-block
    /// `Vec<u8>` storage to recycle (history is the sole byte buffer
    /// and `add_data` returns each input Vec eagerly). The dispatcher
    /// in `match_generator.rs` knows the variant and threads the right
    /// signature; callers needing the eviction byte count derive it
    /// from the `window_size` delta before/after this call.
    ///
    /// The explicit-trim-before-idle path is the reason this helper
    /// exists: a caller that trims to shed memory before a long
    /// quiescent period must see the resident size drop immediately,
    /// not "eventually, on the next ingest".
    ///
    /// `compact_history` is NOT the right tool for that — it uses
    /// `Vec::drain(..history_start)`, which moves elements down in
    /// the existing allocation and leaves capacity untouched (per
    /// `Vec` docs, only `shrink_to_fit` releases capacity). So even
    /// when compact ran, the original buffer stayed alive. Instead,
    /// rebuild `history` via `split_off`: it allocates a fresh
    /// buffer sized to the retained suffix, and the assignment
    /// drops the original (full-capacity) buffer. On a normal block
    /// loop `add_data` will compact again the next iter — the cost
    /// is one extra realloc on the trim boundary in exchange for
    /// actually shedding the prefix to the system allocator.
    ///
    /// Unlike `HashChainTable::trim_to_window` /
    /// `RowMatcher::trim_to_window` this signature deliberately takes
    /// no `reuse_space` callback — Dfast stores its raw bytes in the
    /// single contiguous `history` buffer (no per-block `Vec<u8>` to
    /// recycle), releases the dead prefix via `split_off`, and
    /// surfaces eviction byte count to the dispatcher via the
    /// `window_size` delta rather than a callback. A uniform-signature
    /// trim helper would force every call site to monomorphize an
    /// `impl FnMut(Vec<u8>)` that is documented to never fire, paying
    /// codegen + inlining cost on the cold path for zero behaviour
    /// difference; the dispatcher in `match_generator.rs` already
    /// branches per backend (matchers diverge on `reset` and a few
    /// other lifecycle calls) so adding one more per-backend arm is
    /// free.
    pub(crate) fn trim_to_window(&mut self) {
        if self.window_size > self.max_window_size || self.history_start != 0 {
            // Any history shift slides the dictionary out of (or within) the
            // concat, staling the dict tables' concat indices — drop the attach.
            self.dict.invalidate();
        }
        while self.window_size > self.max_window_size {
            let removed_len = self.window_blocks.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
        if self.history_start != 0 {
            // `split_off` returns the suffix in a fresh allocation;
            // the original Vec (still owning [..history_start]) is
            // dropped on the assignment below, releasing the dead
            // prefix back to the allocator.
            self.history = self.history.split_off(self.history_start);
            self.history_start = 0;
        }
    }

    pub(crate) fn skip_matching(&mut self, incompressible_hint: Option<bool>) {
        self.ensure_hash_tables();
        let current_len = self.window_blocks.back().copied().unwrap_or(0);
        if current_len == 0 {
            // `add_data` short-circuits on empty input and does NOT push
            // a zero-length entry onto `window_blocks`. A caller that
            // invokes skip-matching after a streaming flush of an empty
            // chunk would otherwise re-seed the previous block's
            // retained tail on every empty write. Mirror the gate that
            // `start_matching` already uses.
            return;
        }
        let current_abs_start = self.history_abs_start + self.window_size - current_len;
        let current_abs_end = current_abs_start + current_len;
        let tail_start = current_abs_start.saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN);
        if tail_start < current_abs_start {
            self.insert_positions(tail_start, current_abs_start);
        }

        let used_sparse = incompressible_hint
            .unwrap_or_else(|| self.block_looks_incompressible(current_abs_start, current_abs_end));
        if used_sparse {
            self.insert_positions_with_step(
                current_abs_start,
                current_abs_end,
                DFAST_INCOMPRESSIBLE_SKIP_STEP,
            );
        } else {
            self.insert_positions(current_abs_start, current_abs_end);
        }

        // Seed the tail densely only after sparse insertion so the next block
        // can match across the boundary without rehashing the full block twice.
        if used_sparse {
            let tail_start = current_abs_end
                .saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN)
                .max(current_abs_start);
            if tail_start < current_abs_end {
                self.insert_positions(tail_start, current_abs_end);
            }
        }
    }

    pub(crate) fn skip_matching_dense(&mut self) {
        self.ensure_hash_tables();
        let current_len = self.window_blocks.back().copied().unwrap_or(0);
        if current_len == 0 {
            // Same gate as `skip_matching` and `start_matching`: empty
            // chunks fed through `add_data` no longer push a block
            // entry, so a streaming caller that flushes empty chunks
            // would otherwise re-seed the retained tail on every
            // empty write.
            return;
        }
        let current_abs_start = self.history_abs_start + self.window_size - current_len;
        let current_abs_end = current_abs_start + current_len;
        let backfill_start = current_abs_start
            .saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN)
            .max(self.history_abs_start);
        if backfill_start < current_abs_start {
            self.insert_positions(backfill_start, current_abs_start);
        }
        self.insert_positions(current_abs_start, current_abs_end);
    }

    /// Upstream zstd `ZSTD_dictMatchState` attach (dfast): hash the dictionary block at
    /// the front of history into the SEPARATE immutable [`Self::dict`] tables
    /// (long+short), once, instead of re-priming the live tables every frame
    /// (`skip_matching_dense`). The dual-probe kernel then searches live + dict
    /// without per-frame dict cost. Cached via the `DictAttach` primed flag.
    pub(crate) fn skip_matching_for_dict_attach(&mut self) {
        self.ensure_hash_tables();
        let current_len = self.window_blocks.back().copied().unwrap_or(0);
        if current_len == 0 {
            return;
        }
        let current_abs_start = self.history_abs_start + self.window_size - current_len;
        let current_abs_end = current_abs_start + current_len;
        // Convert absolute → concat (history-relative) coordinates: the dict
        // table keys off the stable concat index, not `position_base`.
        let start_concat = current_abs_start - self.history_abs_start;
        let end_concat = current_abs_end - self.history_abs_start;
        self.prime_dict_tables_for_range(start_concat, end_concat);
    }

    /// Mark the dict tables fully built (CDict cache). The driver calls this
    /// after the final dictionary chunk so the next frame skips the re-hash.
    ///
    pub(crate) fn mark_dict_primed(&mut self) {
        self.dict.mark_primed();
    }

    /// Drop the cached dict tables (next frame carries no dict, or eviction /
    /// param change staled the concat positions).
    pub(crate) fn invalidate_dict_cache(&mut self) {
        self.dict.invalidate();
    }

    /// Build the immutable dict long+short tables over the contiguous-history
    /// concat range `[start_concat, end_concat)` (the dictionary bytes at the
    /// front of history). Mirrors [`Self::insert_positions`]' hash + lookahead
    /// gating, but writes a +1-biased CONCAT index (`idx + 1`, stable across
    /// `position_base` rebases) into `self.dict` rather than the live,
    /// `position_base`-packed tables. `DFAST_EMPTY_SLOT = 0` means "no entry".
    /// Skips the rehash when the CDict cache is already primed.
    fn prime_dict_tables_for_range(&mut self, start_concat: usize, end_concat: usize) {
        const PRIME: u64 = 0xCF1BBCDCB7A56463_u64;
        // Record the dict/input boundary (concat index) regardless of whether
        // any position is hashable (a sub-min-match dict still bounds dict_end).
        self.dict.set_region_len(end_concat);
        if self.dict.is_primed() {
            return;
        }
        let history_start = self.history_start;
        let concat_len = self.history.len() - history_start;
        let long_bits = self.long_hash_bits;
        let short_bits = self.short_hash_bits;
        // Lookahead-safe cutoffs within the concat: long needs 8 readable
        // bytes, short needs 5 (upstream zstd `mls = 5` for the short hash).
        let long_safe_end = concat_len.saturating_sub(7).min(end_concat);
        let short_safe_end = concat_len.saturating_sub(4).min(end_concat);
        // The seam window `[backfill_floor, start_concat)` holds positions that
        // only became hashable when THIS chunk extended history (e.g. a `4+1`
        // or `7+1` dict chunking), so gate the early return on `backfill_floor`,
        // not `start_concat` — otherwise those seam inserts are dropped and the
        // attached dict tables stay incomplete.
        let backfill_floor = start_concat.saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN);
        if backfill_floor >= short_safe_end {
            return;
        }
        let dict = self.dict.table_mut_or_init(|| DfastDictTables {
            long: alloc::vec![DFAST_EMPTY_SLOT; 1usize << long_bits],
            short: alloc::vec![DFAST_EMPTY_SLOT; 1usize << short_bits],
        });
        let short_shift = 64 - short_bits;
        let long_shift = 64 - long_bits;
        let base = self.history.as_ptr();
        let long_ptr = dict.long.as_mut_ptr();
        let short_ptr = dict.short.as_mut_ptr();
        // SAFETY: `base.add(history_start + pos)` is in-bounds for
        // `pos + 8 <= concat_len` (long) / `pos + 5 <= concat_len` (short, the
        // upstream zstd 5-byte key), enforced by the `*_safe_end` cutoffs (the short
        // loop reads a 4-byte word + 1 byte, never past `concat_len`).
        // `*_idx = mixed >> (64 - bits)`
        // has at most `bits` bits set, in-bounds for the `1 << bits` tables.
        // `pos + 1` fits u32: concat indices are bounded by the u32 history
        // gate upstream (`check_stream_abs_headroom`).
        //
        // Backfill the previous chunk's last 7/3 bytes (the seam), which only
        // became hashable now that this chunk extended history — mirrors the
        // dense priming paths' backfill so multi-chunk dictionary priming
        // doesn't drop seam-spanning candidates.
        //
        // `saturating_sub` is a deliberate FLOOR clamp to concat position 0
        // (the dict start), NOT overflow-masking: `start_concat` is a valid
        // concat index, and the seam window `[start_concat - tail, start_concat)`
        // is clamped at 0 because there are no dict bytes before the front.
        // The first chunk (`start_concat == 0`) clamps to 0 → no seam, no-op.
        let mut pos = backfill_floor;
        while pos < long_safe_end {
            unsafe {
                let load_ptr = base.add(history_start + pos);
                let v8 = (load_ptr as *const u64).read_unaligned();
                // Upstream zstd 5-byte short hash (ZSTD_hash5 shape): low 5 bytes in
                // the high 40 bits (`v8 << 24`), matching `short_hash_index`.
                let short_idx = ((v8 << 24).wrapping_mul(PRIME) >> short_shift) as usize;
                let long_idx = (v8.wrapping_mul(PRIME) >> long_shift) as usize;
                let packed = (pos as u32) + 1;
                *short_ptr.add(short_idx) = packed;
                *long_ptr.add(long_idx) = packed;
            }
            pos += 1;
        }
        while pos < short_safe_end {
            unsafe {
                let load_ptr = base.add(history_start + pos);
                // 5-byte short key (upstream zstd `mls = 5`), assembled from a 4-byte
                // load + 1 byte so it never over-reads the <8-byte tail; the
                // low 5 bytes land in bits 24..63, matching `v8 << 24`.
                let lo4 = (load_ptr as *const u32).read_unaligned() as u64;
                let b5 = *load_ptr.add(4) as u64;
                let v5 = (lo4 | (b5 << 32)) << 24;
                let short_idx = (v5.wrapping_mul(PRIME) >> short_shift) as usize;
                *short_ptr.add(short_idx) = (pos as u32) + 1;
            }
            pos += 1;
        }
    }

    pub(crate) fn start_matching(&mut self, mut handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        self.ensure_hash_tables();

        let current_len = self.window_blocks.back().copied().unwrap_or(0);
        if current_len == 0 {
            return;
        }

        let current_abs_start = self.history_abs_start + self.window_size - current_len;
        // Re-seed the previous block's seam. With the upstream zstd 5-byte short hash,
        // a position within `mls - 1` bytes of the prior block end could not
        // form its full key when that block was processed (the trailing bytes
        // arrived with THIS block); re-hash that tail now that history spans
        // it, so cross-block matches anchored in the seam are found. Mirrors
        // `skip_matching_dense`'s backfill.
        let backfill_start = current_abs_start
            .saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN)
            .max(self.history_abs_start);
        if backfill_start < current_abs_start {
            self.insert_positions(backfill_start, current_abs_start);
        }
        // dfast is the upstream zstd's greedy double-fast at every level (no lazy
        // variant exists — lazy parsing is the separate `ZSTD_lazy`/`lazy2`
        // strategy), so there is a single match path.
        self.start_matching_fast_loop(current_abs_start, current_len, &mut handle_sequence);
    }

    /// Register the caller's in-place input buffer for the borrowed one-shot
    /// path (upstream zstd `ZSTD_CCtx` in-place input). The dfast analog of
    /// [`super::simple::fast_matcher::FastKernelMatcher::set_borrowed_window`]:
    /// subsequent blocks scan ranges of `buffer` directly instead of copying
    /// each into the owned `history` concat.
    ///
    /// # Safety
    /// `buffer` must stay live and unmodified until [`Self::clear_borrowed_window`]
    /// (or [`Self::reset`]) — the matcher stores a raw pointer into it and
    /// dereferences it during every staged block scan.
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        self.borrowed_input = Some((buffer.as_ptr(), buffer.len()));
        self.borrowed_block = None;
    }

    /// Drop the borrowed input window (the caller's slice no longer lives).
    pub(crate) fn clear_borrowed_window(&mut self) {
        self.borrowed_input = None;
        self.borrowed_block = None;
    }

    /// Make `[block_start, block_end)` the active borrowed block BEFORE the
    /// scan, so the emit pipeline's pre-scan `get_last_space().len()` reserve
    /// reports this block (not a stale or whole-input range).
    pub(crate) fn stage_borrowed_block(&mut self, block_start: usize, block_end: usize) {
        super::history::stage_borrowed_block(
            self.borrowed_input,
            &mut self.borrowed_block,
            block_start,
            block_end,
        );
    }

    /// Borrowed one-shot equivalent of [`Self::start_matching`]: scan
    /// `[block_start, block_end)` of the registered borrowed window in place
    /// (no `commit_space` copy). Produces a byte-identical sequence stream to
    /// the owned path for in-window inputs — positions are absolute input
    /// offsets, candidate reads land in the same buffer, and the seam re-seed
    /// re-hashes the prior block's short-key tail exactly as the owned loop
    /// does once `history` spans it.
    pub(crate) fn start_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        self.stage_borrowed_block(block_start, block_end);
        self.ensure_hash_tables();
        let current_len = block_end - block_start;
        if current_len == 0 {
            return;
        }
        let current_abs_start = block_start;
        // Seam re-seed (mirror `start_matching`): re-hash the prior block's
        // trailing `BOUNDARY_DENSE_TAIL_LEN` bytes now that the readable
        // length spans them so a position whose 5-byte key could not form at
        // the prior block end becomes matchable. The borrowed floor is the
        // input start (0), so the first block (block_start == 0) backfills
        // nothing.
        let backfill_start = current_abs_start.saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN);
        if backfill_start < current_abs_start {
            self.insert_positions(backfill_start, current_abs_start);
        }
        self.start_matching_fast_loop(current_abs_start, current_len, &mut handle_sequence);
    }

    /// Borrowed one-shot equivalent of [`Self::skip_matching`]: stage the
    /// block (so `get_last_space` reports it for the RLE/Raw emit) without
    /// scanning. The block's bytes already sit in the borrowed buffer at
    /// their absolute offsets, so a future block reaches them by offset just
    /// as the owned skip's `history` append makes them reachable; the
    /// `Some(false)` dictionary-priming case hashes every position so future
    /// blocks can MATCH them, mirroring the owned skip's prime path.
    pub(crate) fn skip_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        incompressible_hint: Option<bool>,
    ) {
        self.stage_borrowed_block(block_start, block_end);
        if incompressible_hint == Some(false) {
            self.ensure_hash_tables();
            self.insert_positions(block_start, block_end);
        }
    }

    /// Single-cursor probe at the last hashable position in the
    /// current block. Called from `start_matching_fast_loop` only when
    /// the outer iteration sees `ip0` still hashable but `ip1` past
    /// the end — the upstream reference's single-cursor loop probes
    /// every position with `p + HASH_READ_SIZE <= iend`, and skipping
    /// `ip0` here would leave a real match unsearched.
    ///
    /// Mirror the inner loop's long+short probe and the table update
    /// at `ip0`, but skip the rep1 peek (reads at `ip1`) and the
    /// `_search_next_long` retry (reads at `ip1`) — both depend on a
    /// hashable `ip1`. Upstream accepts this exact tradeoff at the
    /// `iend` boundary: the rep peek and retry only ever fire when a
    /// second hashable position exists.
    ///
    /// Returns `Some(MatchCandidate)` if a long or short hit at `ip0`
    /// meets the `DFAST_MIN_MATCH_LEN` floor, `None` otherwise (caller
    /// then breaks the outer loop). On a hit, the hash tables are NOT
    /// updated by this helper — the caller routes through
    /// `emit_candidate` which inserts via `insert_positions` over the
    /// emitted range, exactly like the inner loop's hit path.
    fn probe_tail_ip0_only(
        &self,
        current_abs_start: usize,
        current_len: usize,
        ip0: usize,
        literals_start: usize,
        borrowed: bool,
    ) -> Option<MatchCandidate> {
        debug_assert!(ip0 + HASH_READ_SIZE <= current_len);
        const PRIME: u64 = 0xCF1BBCDCB7A56463_u64;
        let short_shift = 64 - self.short_hash_bits;
        let long_shift = 64 - self.long_hash_bits;

        let abs_ip0 = current_abs_start + ip0;
        let lit_len_ip0 = ip0 - literals_start;
        // Byte source through `scan_source()` so the tail probe reads the
        // borrowed input in place when a borrowed window is active.
        let (history_base_ptr, history_start_offset, history_abs_start, position_base, concat_len) =
            self.scan_source();

        // Per-position window-low bound, identical to the fast loop's `wlow0`
        // (see `advertised_window` there). In borrowed mode `history_abs_start`
        // is 0, so a bare `cand_pos >= history_abs_start` floor admits
        // candidates older than the advertised window and could emit an
        // unresolvable offset for over-window inputs; bound by `abs_ip0 -
        // advertised_window` instead. Owned mode keeps the eviction-floor
        // `history_abs_start`.
        let wlow = if borrowed {
            abs_ip0.saturating_sub(self.max_window_size)
        } else {
            history_abs_start
        };

        let concat_idx0 = abs_ip0 - history_abs_start;

        // SAFETY: `concat_idx0 + 8 <= concat_len` follows from the
        // caller's `ip0 + HASH_READ_SIZE <= current_len` precondition
        // (the live history contains the full current block).
        let v8_0 = unsafe {
            (history_base_ptr.add(history_start_offset + concat_idx0) as *const u64)
                .read_unaligned()
        };
        // `v4_0` (low 4 bytes) is the 4-byte equality-gate key below; the short
        // HASH keys on the upstream zstd 5-byte window (`v8_0 << 24`, ZSTD_hash5 shape).
        let v4_0 = v8_0 & 0xFFFF_FFFF;
        let hl0_idx = (v8_0.wrapping_mul(PRIME) >> long_shift) as usize;
        let hs0_idx = ((v8_0 << 24).wrapping_mul(PRIME) >> short_shift) as usize;

        // Read-only on the hash tables here — unlike the inner loop's
        // "update-before-check" pattern, the writes at `hl0_idx` /
        // `hs0_idx` would be dead in this helper:
        //
        //   * On a hit, the caller routes through `emit_candidate`
        //     which insert-positions the entire emitted range, then
        //     either advances `pos` past `current_len - HASH_READ_SIZE`
        //     and the outer guard breaks (so a future iter never
        //     re-uses these slots), or `continue 'outer` re-enters
        //     with `pos = start + match_len ≥ current_len - 3`, which
        //     fails the outer-entry `pos + HASH_READ_SIZE > current_len`
        //     guard immediately. Either way, no second probe sees the
        //     write.
        //   * On no hit, the caller `break 'outer`s directly. Same
        //     conclusion.
        //   * `seed_remaining_hashable_starts` inserts `ip0` itself
        //     during the post-loop tail seed pass, so even the "fresh
        //     entry for next block" rationale doesn't justify writing
        //     here — the seeder does that.
        //
        // Skipping the writes also removes a small amount of cache
        // dirtying on the tail boundary and keeps `probe_tail_ip0_only`
        // strictly cheaper than a full inner-loop iter.
        let idxl0 = unsafe { *self.long_ptr().add(hl0_idx) };
        let idxs0 = unsafe { *self.short_ptr().add(hs0_idx) };

        // Live tables only — no attached-dict probe here, by design. This
        // helper runs for exactly ONE position per block (the last hashable
        // `ip0` when `ip1` has fallen off the tail), so a missed dict match
        // costs at most one sequence per ~128 KiB block (negligible ratio).
        // `seed_remaining_hashable_starts` inserts this position so it is
        // dict-searchable in the next block; replicating the full dict
        // long+short dual-probe here would duplicate ~40 lines for that single
        // boundary position and defeat the helper's "strictly cheaper than a
        // full inner-loop iter" purpose.

        // Long-hash probe first (upstream priority: an 8-byte hit
        // beats a 4-byte hit even before extension).
        if idxl0 != DFAST_EMPTY_SLOT {
            let cand_pos = position_base + (idxl0 as usize) - 1;
            if cand_pos >= wlow && cand_pos < abs_ip0 {
                let cand_idx = cand_pos - history_abs_start;
                let cand_v8 = unsafe {
                    (history_base_ptr.add(history_start_offset + cand_idx) as *const u64)
                        .read_unaligned()
                };
                if cand_v8 == v8_0 {
                    let mut match_len = 8usize;
                    let max_fwd = concat_len.saturating_sub(concat_idx0 + 8);
                    unsafe {
                        let lhs = history_base_ptr.add(history_start_offset + cand_idx + 8);
                        let rhs = history_base_ptr.add(history_start_offset + concat_idx0 + 8);
                        let ext = crate::encoding::fastpath::dispatch_common_prefix_len_ptr(
                            lhs, rhs, max_fwd,
                        );
                        match_len += ext;
                    }
                    let concat = unsafe {
                        core::slice::from_raw_parts(
                            history_base_ptr.add(history_start_offset),
                            concat_len,
                        )
                    };
                    return Some(extend_backwards_shared(
                        concat,
                        history_abs_start,
                        cand_pos,
                        abs_ip0,
                        match_len,
                        lit_len_ip0,
                    ));
                }
            }
        }

        // Short-hash probe (4-byte gate, forward extension, same
        // floor-enforcement as the inner loop's short path — see
        // comment there).
        if idxs0 != DFAST_EMPTY_SLOT {
            let cand_pos_s = position_base + (idxs0 as usize) - 1;
            if cand_pos_s >= wlow && cand_pos_s < abs_ip0 {
                let cand_idx_s = cand_pos_s - history_abs_start;
                let cand4 = unsafe {
                    (history_base_ptr.add(history_start_offset + cand_idx_s) as *const u32)
                        .read_unaligned()
                };
                if cand4 == v4_0 as u32 {
                    let mut s_match_len = 4usize;
                    let max_fwd = concat_len.saturating_sub(concat_idx0 + 4);
                    unsafe {
                        let lhs = history_base_ptr.add(history_start_offset + cand_idx_s + 4);
                        let rhs = history_base_ptr.add(history_start_offset + concat_idx0 + 4);
                        let ext = crate::encoding::fastpath::dispatch_common_prefix_len_ptr(
                            lhs, rhs, max_fwd,
                        );
                        s_match_len += ext;
                    }
                    let concat = unsafe {
                        core::slice::from_raw_parts(
                            history_base_ptr.add(history_start_offset),
                            concat_len,
                        )
                    };
                    let short_cand = extend_backwards_shared(
                        concat,
                        history_abs_start,
                        cand_pos_s,
                        abs_ip0,
                        s_match_len,
                        lit_len_ip0,
                    );
                    if short_cand.match_len >= DFAST_MIN_MATCH_LEN {
                        return Some(short_cand);
                    }
                }
            }
        }

        None
    }

    /// Upstream zstd `zstd_double_fast.c` post-match rep-0 extension. After the
    /// primary match has been emitted and `pos` advanced past it, upstream zstd
    /// opportunistically chains additional `rep_2`-coded matches at the
    /// new cursor as long as 4 bytes at `ip` keep matching the bytes at
    /// `ip - offset_2` (in upstream zstd naming; in Rust offset terms this is
    /// `offset_hist[1]` once `lit_len == 0` after the just-emitted
    /// primary). Each iteration:
    ///
    ///   * emits one zero-literal sequence with the old `offset_hist[1]`,
    ///   * swaps `offset_hist[0]` ↔ `offset_hist[1]` via
    ///     [`encode_offset_with_history`] (the upstream zstd `offset_2 = offset_1;
    ///     offset_1 = old_offset_2;` swap),
    ///   * skips the hash-table probe entirely on every extra match.
    ///
    /// Critically uses upstream zstd's `MINMATCH = 4` here rather than the
    /// `DFAST_MIN_MATCH_LEN = 5` enforced on the main search
    /// loop. The upstream zstd accepts any 4-byte rep extension; we mirror that
    /// because the rep emission carries no offset cost — even a 4-byte
    /// rep is a net win over re-running the full hash search. Returns
    /// the new value of `pos` and updates `literals_start` in place to
    /// the post-rep-chain anchor.
    fn extend_with_repcode_after_match(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        mut pos: usize,
        literals_start: &mut usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) -> usize {
        loop {
            // Need at least DFAST_REP_MIN_MATCH_LEN bytes of room past `pos`.
            if pos + DFAST_REP_MIN_MATCH_LEN > current_len {
                break;
            }
            // After a primary emit `literals_start == pos`, so `lit_len`
            // on the next sequence is zero — upstream zstd's rep probe uses
            // `offset_2` (== `offset_hist[1]` under our encoding).
            let rep = self.offset_hist[1] as usize;
            if rep == 0 {
                break;
            }
            // Source bytes + rebase coordinate through `scan_source()` so a
            // borrowed window's rep-extension reads the in-place input.
            let (base_ptr, start_offset, abs_start, _position_base, concat_len) =
                self.scan_source();
            let abs_pos = current_abs_start + pos;
            let cur_idx = abs_pos - abs_start;
            // `checked_sub` is the authoritative bound here: a valid rep
            // can reach beyond the current block into retained history
            // (the contiguous `live_history()` buffer covers
            // `history_abs_start..history_abs_end`), so the only hard
            // constraint is `cur_idx >= rep` (i.e. the candidate is in
            // the addressable history range). A previous draft also
            // gated on `rep > pos`, which over-rejected valid offsets
            // that point into retained history near block boundaries —
            // exactly the upstream zstd-style chain win this helper is meant to
            // recover.
            let cand_idx = match cur_idx.checked_sub(rep) {
                Some(idx) => idx,
                None => break,
            };
            // SAFETY: `base_ptr + start_offset` is the live source start and
            // `concat_len` its readable length (owned history or borrowed
            // input); the `cur_idx` / `cand_idx` reads below are gated `<
            // concat.len()`. Raw-ptr backed, no borrow on `self`.
            let concat =
                unsafe { core::slice::from_raw_parts(base_ptr.add(start_offset), concat_len) };
            if cur_idx + DFAST_REP_MIN_MATCH_LEN > concat.len() {
                break;
            }
            // Cheap 4-byte gate before the SIMD `common_prefix_len`. Read it as
            // one unaligned u32 rather than a slice `!=` (which lowers to a libc
            // `memcmp` CALL). Bounds: `cur_idx + 4 <= len` from the
            // `DFAST_REP_MIN_MATCH_LEN` (= 4) check above, and
            // `cand_idx = cur_idx - rep < cur_idx` so `cand_idx + 4 <= len` too.
            let gate_eq = unsafe {
                concat.as_ptr().add(cur_idx).cast::<u32>().read_unaligned()
                    == concat.as_ptr().add(cand_idx).cast::<u32>().read_unaligned()
            };
            if !gate_eq {
                break;
            }
            let match_len =
                common_prefix_len_with_kernel(self.kernel, &concat[cand_idx..], &concat[cur_idx..]);
            if match_len < DFAST_REP_MIN_MATCH_LEN {
                break;
            }
            // Upstream zstd immediate-repcode insertion (zstd_double_fast.c:314-315):
            // INSIDE the rep chain, upstream writes BOTH hash tables at the rep
            // position itself (`hashSmall[hash(ip)] = ip; hashLong[hash(ip)] = ip`)
            // before advancing — NOT the `curr+2 / ip-2 / ip-1` primary-match
            // complementary set (that pattern belongs to `_match_found`, lines
            // 300-304, reached only by the non-rep store path). Inserting the
            // wrong set here leaves the rep-position keys stale, so a later
            // position re-resolves the long hash to an older far candidate
            // instead of the stable rep offset — the dfast ratio gap vs C.
            self.insert_position(abs_pos);
            // Emit zero-literal rep sequence.
            handle_sequence(Sequence::Triple {
                literals: &[],
                offset: rep,
                match_len,
            });
            let _ = encode_offset_with_history(rep as u32, 0, &mut self.offset_hist);
            pos += match_len;
            *literals_start = pos;
        }
        pos
    }

    pub(crate) fn seed_remaining_hashable_starts(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        pos: usize,
    ) {
        let boundary_tail_start = current_len.saturating_sub(Self::BOUNDARY_DENSE_TAIL_LEN);
        let mut seed_pos = pos.min(current_len).min(boundary_tail_start);
        while seed_pos + DFAST_SHORT_HASH_LOOKAHEAD <= current_len {
            self.insert_position(current_abs_start + seed_pos);
            seed_pos += 1;
        }
    }

    /// Per-outer-iteration byte-source descriptor for the fast loop:
    /// `(base_ptr, start_offset, abs_start, position_base, concat_len)`.
    ///
    /// Read once at the top of every outer iteration so the kernel body
    /// is agnostic to where its bytes live: the owned path returns the
    /// `history` concat's (rebased) fields, and a future borrowed
    /// one-shot window will return its own constant descriptor without
    /// the kernel body changing. Returns raw pointer + offsets (no borrow
    /// held), so the subsequent `&mut self` hash-table pointer snapshot in
    /// the loop stays sound.
    #[inline(always)]
    fn scan_source(&self) -> (*const u8, usize, usize, usize, usize) {
        if let (Some((ptr, _total)), Some((_block_start, block_end))) =
            (self.borrowed_input, self.borrowed_block)
        {
            // Borrowed one-shot window: positions are absolute input
            // offsets, so the rebase coordinates collapse to zero
            // (`start_offset = abs_start = position_base = 0`) and the
            // readable length is the active block's end. No history concat,
            // no `commit_space` copy. Candidate reads from earlier blocks
            // land at `ptr + earlier_abs_pos` (< block_end), in range.
            return (ptr, 0, 0, 0, block_end);
        }
        let start_offset = self.history_start;
        (
            self.history.as_ptr(),
            start_offset,
            self.history_abs_start,
            self.position_base,
            self.history.len() - start_offset,
        )
    }

    /// Byte-source descriptor for the BORROWED kernel: `(input_ptr,
    /// block_end)`. The borrowed window packs absolute input offsets, so the
    /// owned rebase coordinates (`start_offset`, `abs_start`, `position_base`)
    /// are constant `0` and the hot loop supplies them as literals — folding
    /// every per-position `abs - abs_start` / `>= abs_start` term to the bare
    /// absolute position (upstream zstd `base + index` shape). Split out from
    /// [`Self::scan_source`] so the `BORROWED` const kernel never materialises
    /// the owned-path arithmetic.
    #[inline(always)]
    fn borrowed_scan_descriptor(&self) -> (*const u8, usize) {
        let (ptr, _total) = self
            .borrowed_input
            .expect("BORROWED kernel dispatched without a borrowed window");
        let (_block_start, block_end) = self
            .borrowed_block
            .expect("BORROWED kernel dispatched without a staged block");
        (ptr, block_end)
    }

    /// Byte-source descriptor for the owned (history-concat) kernel. Mirrors
    /// the owned arm of [`Self::scan_source`] without the borrowed branch, so
    /// the `!BORROWED` const kernel reads the rebased fields directly.
    #[inline(always)]
    fn owned_scan_descriptor(&self) -> (*const u8, usize, usize, usize, usize) {
        let start_offset = self.history_start;
        (
            self.history.as_ptr(),
            start_offset,
            self.history_abs_start,
            self.position_base,
            self.history.len() - start_offset,
        )
    }

    /// `(ptr, len)` of the block currently being emitted, for the literal
    /// slices in `emit_candidate` / `emit_trailing_literals`. Owned: the
    /// `history` concat tail (`last_len` bytes). Borrowed: the in-place
    /// input sub-slice `[current_abs_start, block_end)`. Returns a raw
    /// pointer (not a `&[u8]` tied to `&self`) so the caller can rebuild
    /// the slice locally and still take `&mut self` for `offset_hist`.
    #[inline]
    fn current_block_ptr_len(&self, current_abs_start: usize) -> (*const u8, usize) {
        if let (Some((ptr, _total)), Some((_block_start, block_end))) =
            (self.borrowed_input, self.borrowed_block)
        {
            // SAFETY: borrowed liveness contract; `current_abs_start` =
            // block_start <= block_end <= buffer len (entry-validated).
            (
                unsafe { ptr.add(current_abs_start) },
                block_end - current_abs_start,
            )
        } else {
            let last_len = self.window_blocks.back().copied().unwrap_or(0);
            let off = self.history.len() - last_len;
            // SAFETY: `off + last_len == history.len()`, in bounds.
            (unsafe { self.history.as_ptr().add(off) }, last_len)
        }
    }

    // Force-inline on native (the dfast monolithization speedup) but NOT on
    // wasm32, where inlining these per-match helpers into every call site
    // bloats the module past the .wasm size budget; wasm is size-, not
    // speed-sensitive, so let LLVM keep them out-of-line there.
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn emit_candidate(
        &mut self,
        current_abs_start: usize,
        literals_start: &mut usize,
        candidate: MatchCandidate,
        scan_pos: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) -> usize {
        // Upstream zstd `zstd_double_fast.c` parity: the inner search loop already
        // inserts every position it VISITS (step-accelerated), so the literal
        // run is hashed exactly as densely as the upstream zstd's cursor swept it —
        // stepped-over and block-anchor (position 0) positions are NOT
        // re-inserted here (the upstream zstd skips them too via `ip += (ip ==
        // prefixStart)` + the growing `step`). Match interior: upstream zstd fills only
        // the sparse 3-target set (`curr+2`, `ip-2`, `ip-1`), each clamped to
        // the open match interval.
        // Upstream zstd complementary insertion (zstd_double_fast.c:300-304) is
        // ASYMMETRIC across the two tables:
        //   hashLong:  curr+2, ip-2
        //   hashSmall: curr+2, ip-1
        // The previous form inserted curr+2/ip-2/ip-1 into BOTH tables, putting
        // ip-1 into long and ip-2 into short that upstream never writes —
        // polluting the long table so later positions resolve to the wrong
        // (far, non-rep) candidate. Mirror the exact per-table target set.
        //
        // `curr` is the iteration's SCAN position (`scan_pos`, upstream `curr`
        // = `(U32)(ip-base)` fixed at zstd_double_fast.c:184), NOT the match
        // start: upstream advances `ip` for a rep1 (`ip++`, the match begins at
        // `scan+1`) and rewinds it during backward catch-up, but `curr` stays
        // pinned to the scan cursor. Anchoring `curr+2` on `match_start` instead
        // shifts the insert by +1 on every rep1 (and by the catch-up length on
        // extended matches), seeding the short hash with the wrong positions —
        // a divergence that compounds across a block.
        let post_match_end = candidate.start + candidate.match_len;
        // `match_len >= DFAST_REP_MIN_MATCH_LEN` (4) for every committed match,
        // so `post_match_end >= 4`: the `- 2` / `- 1` cannot underflow (plain
        // arithmetic, no `saturating_*` masking).
        let curr_plus_2 = scan_pos + 2;
        let ip_minus_2 = post_match_end - 2;
        let ip_minus_1 = post_match_end - 1;
        self.insert_long(curr_plus_2);
        self.insert_long(ip_minus_2);
        self.insert_short(curr_plus_2);
        self.insert_short(ip_minus_1);
        // Inline the trailing-block slice rather than calling
        // `get_last_space()` so this matches the gate pattern used by
        // `skip_matching` / `start_matching` (read `window_blocks.back()`
        // with `unwrap_or(0)`). `emit_candidate` runs only after a
        // successful match was found in the active block, so
        // `last_len > 0` is a structural precondition — the
        // `debug_assert!` makes that precondition fail at the source
        // in tests rather than silently produce an empty slice and
        // panic on the literals subslice below.
        let (cur_ptr, cur_len) = self.current_block_ptr_len(current_abs_start);
        debug_assert!(
            cur_len > 0,
            "emit_candidate precondition: active block must be non-empty"
        );
        // SAFETY: raw-ptr backed (no borrow on `self`), so the
        // `&mut self.offset_hist` below coexists. Bytes are the active block
        // (owned history tail or borrowed input sub-slice).
        let current = unsafe { core::slice::from_raw_parts(cur_ptr, cur_len) };
        let start = candidate.start - current_abs_start;
        let literals = &current[*literals_start..start];
        handle_sequence(Sequence::Triple {
            literals,
            offset: candidate.offset,
            match_len: candidate.match_len,
        });
        let _ = encode_offset_with_history(
            candidate.offset as u32,
            literals.len() as u32,
            &mut self.offset_hist,
        );
        *literals_start = start + candidate.match_len;
        start
    }

    fn emit_trailing_literals(
        &self,
        current_abs_start: usize,
        literals_start: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        let (cur_ptr, cur_len) = self.current_block_ptr_len(current_abs_start);
        if literals_start < cur_len {
            // SAFETY: raw-ptr backed active-block slice (owned history tail
            // or borrowed input sub-slice); `literals_start < cur_len` gated
            // above keeps the subslice in range.
            let current = unsafe { core::slice::from_raw_parts(cur_ptr, cur_len) };
            handle_sequence(Sequence::Literals {
                literals: &current[literals_start..],
            });
        }
    }

    pub(crate) fn ensure_hash_tables(&mut self) {
        // Independent sizing per upstream zstd `clevels.h`: long-hash =
        // `hashLog`, short-hash = `chainLog`. Lazy allocation so
        // Fastest/Uncompressed never pay the dfast-level memory cost.
        let total = self.long_len() + self.short_len();
        if self.tables.len() != total {
            // Single zeroed allocation for both regions (`vec![0; n]` lowers to
            // `alloc_zeroed`). One buffer instead of two cuts the large-table
            // allocator churn on fresh-per-frame compressors.
            self.tables = alloc::vec![DFAST_EMPTY_SLOT; total];
        }
    }

    fn compact_history(&mut self) {
        super::history::compact_history(
            &mut self.history,
            &mut self.history_start,
            self.max_window_size,
        );
    }

    pub(crate) fn live_history(&self) -> &[u8] {
        &self.history[self.history_start..]
    }

    pub(crate) fn history_abs_end(&self) -> usize {
        self.history_abs_start + self.live_history().len()
    }

    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    pub(crate) fn insert_positions(&mut self, start: usize, end: usize) {
        // Source the byte buffer + rebase coordinates through `scan_source()`
        // so a borrowed window's batch re-seed hashes the in-place input
        // (owned returns its `history` fields, byte-identical).
        let (history_base_ptr, history_start, history_abs_start, _pb0, concat_len) =
            self.scan_source();
        let start = start.max(history_abs_start);
        let end = end.min(history_abs_start + concat_len);
        if start >= end {
            return;
        }
        // Owned: hoist the rebase trigger out of the inner loop (a single
        // `ensure_room_for(end - 1)` covers every `pack_slot` in the range);
        // it may advance `position_base`, so read that AFTER the call. The
        // borrowed window packs absolute input offsets with `position_base ==
        // 0` and never rebases (the eligibility gate caps `input_len <=
        // u32::MAX`), so a `reduce()` here would corrupt the absolute slots.
        let position_base = if self.borrowed_block.is_none() {
            self.ensure_room_for(end - 1);
            self.position_base
        } else {
            0
        };

        // Snapshot the remaining per-call invariants. `&mut self` blocks the
        // optimiser from hoisting these loads across the inner loop — the
        // bodies of `short_hash` / `long_hash` writes mutate through `self`,
        // so each iteration would otherwise reload `*_hash_bits`. With ~1
        // input byte per call on the dfast hot path that re-load shape was
        // the dominant cost in the per-position cluster. `history_base_ptr`
        // stays valid across `ensure_room_for` (it never reallocs `history`).
        let short_hash_bits = self.short_hash_bits;
        let long_hash_bits = self.long_hash_bits;
        let short_hash_ptr = self.short_mut_ptr();
        let long_hash_ptr = self.long_mut_ptr();
        let short_shift = 64 - short_hash_bits;
        let long_shift = 64 - long_hash_bits;

        // Two contiguous regions in the input range:
        // * `[start .. long_safe_end)` — every position has at least 8
        //   bytes of lookahead, so both short and long hashes get
        //   inserted from a single 8-byte unaligned load.
        // * `[long_safe_end .. short_safe_end)` — only 4..7 bytes
        //   remain, so only the short hash gets inserted (4-byte
        //   load).
        // Past `short_safe_end` neither hash has enough lookahead and
        // upstream zstd parity is "no insert" — skip entirely.
        let abs_concat_end = history_abs_start + concat_len;
        let long_safe_end = abs_concat_end.saturating_sub(7).min(end);
        let short_safe_end = abs_concat_end.saturating_sub(4).min(end);

        // SAFETY: `history_base_ptr.add(history_start + idx)` is
        // in-bounds for `idx + 8 <= concat_len`, which the two
        // `*_safe_end` cutoffs enforce. `short_hash_ptr.add(k)` /
        // `long_hash_ptr.add(k)` are in-bounds because
        // `ensure_hash_tables` sizes the two tables to `1 <<
        // *_hash_bits` and `k = mixed >> (64 - bits)` has at most
        // `bits` bits set. `position_base` and `history_abs_start`
        // are constant across the loop after the single `ensure_room_for`
        // call above. `packed` fits in `u32` by that same gate.
        let mut pos = start;
        while pos < long_safe_end {
            unsafe {
                let idx = pos - history_abs_start;
                let packed = ((pos - position_base) as u32) + 1;
                let load_ptr = history_base_ptr.add(history_start + idx);
                let v8 = (load_ptr as *const u64).read_unaligned();
                // Upstream zstd parity (`zstd_compress_internal.h:923-924`):
                // scalar `* prime8bytes` then shift to high bits. Drops
                // the CRC32d-based kernel dispatch (3-4 instructions) for
                // a single mul on the per-byte insert path. Short hash keys
                // on the upstream zstd 5-byte window (`v8 << 24`, ZSTD_hash5 shape).
                let mixed_short = (v8 << 24).wrapping_mul(0xCF1BBCDCB7A56463_u64);
                let mixed_long = v8.wrapping_mul(0xCF1BBCDCB7A56463_u64);
                let short_idx = (mixed_short >> short_shift) as usize;
                let long_idx = (mixed_long >> long_shift) as usize;
                *short_hash_ptr.add(short_idx) = packed;
                *long_hash_ptr.add(long_idx) = packed;
            }
            pos += 1;
        }
        while pos < short_safe_end {
            unsafe {
                let idx = pos - history_abs_start;
                let packed = ((pos - position_base) as u32) + 1;
                let load_ptr = history_base_ptr.add(history_start + idx);
                // 5-byte short key (upstream zstd `mls = 5`): 4-byte load + 1 byte so
                // the <8-byte tail is never over-read; low 5 bytes in bits
                // 24..63 to match `v8 << 24`.
                let lo4 = (load_ptr as *const u32).read_unaligned() as u64;
                let b5 = *load_ptr.add(4) as u64;
                let mixed_short = ((lo4 | (b5 << 32)) << 24).wrapping_mul(0xCF1BBCDCB7A56463_u64);
                let short_idx = (mixed_short >> short_shift) as usize;
                *short_hash_ptr.add(short_idx) = packed;
            }
            pos += 1;
        }
    }

    pub(crate) fn insert_positions_with_step(&mut self, start: usize, end: usize, step: usize) {
        // The raw `pos += step` below is correct only while `step` is
        // bounded by `DFAST_INCOMPRESSIBLE_SKIP_STEP` (the only value
        // any in-tree caller passes here). Asserting it locally keeps
        // a future caller from quietly reintroducing the overflow risk
        // that the upstream `check_stream_abs_headroom` gate is sized
        // for.
        assert!(
            step <= DFAST_INCOMPRESSIBLE_SKIP_STEP,
            "insert_positions_with_step: step ({step}) exceeds \
             DFAST_INCOMPRESSIBLE_SKIP_STEP — raw `pos += step` would \
             eat into the STREAM_ABS_HEADROOM reserve"
        );
        let start = start.max(self.history_abs_start);
        let end = end.min(self.history_abs_end());
        if step <= 1 {
            self.insert_positions(start, end);
            return;
        }
        let mut pos = start;
        while pos < end {
            self.insert_position(pos);
            // `pos + step` is safe: `pos < end <= history_abs_end()` and
            // `history_abs_end <= usize::MAX - STREAM_ABS_HEADROOM` by
            // the upstream `check_stream_abs_headroom` gate, while
            // `step` is bounded above by the assertion at function
            // entry.
            pos += step;
        }
    }

    #[inline]
    pub(crate) fn insert_position(&mut self, pos: usize) {
        self.insert_masked::<true, true>(pos);
    }

    /// Insert `pos` into ONLY the long hash table (upstream zstd asymmetric
    /// complementary insertion: `hashLong` at `curr+2` and `ip-2`).
    fn insert_long(&mut self, pos: usize) {
        self.insert_masked::<false, true>(pos);
    }

    /// Insert `pos` into ONLY the short hash table (upstream zstd: `hashSmall`
    /// at `curr+2` and `ip-1`).
    fn insert_short(&mut self, pos: usize) {
        self.insert_masked::<true, false>(pos);
    }

    /// Const-generic insertion core. `SHORT` / `LONG` select which tables to
    /// write; both `true` is the symmetric `insert_position`. The const flags
    /// fold at compile time, so `insert_position` stays byte-identical to the
    /// previous single-function form while the asymmetric variants emit only
    /// their one write.
    fn insert_masked<const SHORT: bool, const LONG: bool>(&mut self, pos: usize) {
        // Source the bytes + rebase coordinates through `scan_source()` so a
        // borrowed window's seam / tail re-seeds hash the in-place input
        // exactly as the owned path hashes its `history` concat.
        let (base_ptr, start_offset, abs_start, _position_base, concat_len) = self.scan_source();
        let idx = pos.wrapping_sub(abs_start);
        // Pre-rebase guard (owned only). The producer that walks
        // `insert_positions*` can sweep an arbitrary number of positions
        // per block; running `pack_slot` per-position would call
        // `ensure_room_for` from a tight inner loop. Hoisting the rebase
        // trigger here keeps the per-byte hot path branch-free when the
        // relative window has headroom (the common case) while still
        // guaranteeing the slot value below fits in `u32`. The borrowed
        // window never rebases (`position_base == 0`, no eviction), so it
        // packs the absolute position directly with the same +1 bias.
        let packed = if self.borrowed_block.is_some() {
            (pos as u32).wrapping_add(1)
        } else {
            self.ensure_room_for(pos);
            self.pack_slot(pos)
        };
        // SAFETY: `base_ptr + start_offset` is the live source start (owned
        // `history[history_start..]` or the borrowed input slice) and
        // `concat_len` its readable byte count; the `idx + 5` / `idx + 8`
        // gates keep both keyed reads in range. The slice is raw-pointer
        // backed (holds no borrow on `self`), so the `&mut self.short_hash`
        // / `&mut self.long_hash` writes below stay sound. The `*_hash_index`
        // helpers mask to `long_hash_bits` / `short_hash_bits` and
        // `ensure_hash_tables` sizes both tables to `1 << bits`, so every
        // index is below the table length — eliding the bounds check on this
        // per-byte hot path saves ~4 instructions per call.
        //
        // Single-slot overwrite (upstream parity): upstream
        // `ZSTD_compressBlock_doubleFast_*` writes a single `U32` per hash
        // position and relies on the dense `_search_next_long` retry in
        // `hash_candidate` (via `best_match`) to preserve compression ratio.
        // Short key needs 5 readable bytes (upstream zstd `mls = 5`). A position
        // within 4 bytes of the source end is not inserted here; the
        // `start_matching` seam re-seed picks it up once the next block
        // extends the source far enough to form its full 5-byte key.
        let concat = unsafe { core::slice::from_raw_parts(base_ptr.add(start_offset), concat_len) };
        if SHORT && idx + 5 <= concat_len {
            let short = self.short_hash_index(&concat[idx..]);
            debug_assert!(short < self.short_len());
            // Short region starts at `long_len`.
            let slot = self.long_len() + short;
            unsafe { *self.tables.get_unchecked_mut(slot) = packed };
        }

        if LONG && idx + 8 <= concat_len {
            let long = self.long_hash_index(&concat[idx..]);
            debug_assert!(long < self.long_len());
            unsafe { *self.tables.get_unchecked_mut(long) = packed };
        }
    }

    /// 5-byte short-hash index (upstream zstd `ZSTD_hashPtr(ip, hBitsS, mls=5)`). A
    /// 4-byte key collides more on repetitive / log-stream data, so the
    /// single-slot table overwrites useful positions the upstream zstd's 5-byte key
    /// keeps. `data` MUST hold at least 5 bytes — every call site gates on a
    /// 5-byte lookahead, so no zero-padded synthetic key is ever formed (a
    /// padded short key would populate buckets for starts the upstream zstd skips).
    #[inline(always)]
    pub(crate) fn short_hash_index(&self, data: &[u8]) -> usize {
        debug_assert!(data.len() >= 5, "short hash needs a full 5-byte key");
        // Low 5 bytes (ZSTD_hash5 shape) shifted into bits 24..63, matching the
        // raw `v8 << 24` form used by the fast-loop probe / insert sites.
        let lo4 = u32::from_le_bytes(*data.first_chunk::<4>().unwrap()) as u64;
        let b5 = data[4] as u64;
        let value = (lo4 | (b5 << 32)) << 24;
        self.hash_index_with_bits(value, self.short_hash_bits)
    }

    #[inline(always)]
    pub(crate) fn long_hash_index(&self, data: &[u8]) -> usize {
        let value = u64::from_le_bytes(*data.first_chunk::<8>().unwrap());
        self.hash_index_with_bits(value, self.long_hash_bits)
    }

    fn block_looks_incompressible(&self, start: usize, end: usize) -> bool {
        let live = self.live_history();
        if start >= end || start < self.history_abs_start {
            return false;
        }
        let start_idx = start - self.history_abs_start;
        let end_idx = end - self.history_abs_start;
        if end_idx > live.len() {
            return false;
        }
        let block = &live[start_idx..end_idx];
        block_looks_incompressible(block)
    }

    #[inline(always)]
    fn hash_index_with_bits(&self, value: u64, bits: usize) -> usize {
        // Upstream zstd parity (`zstd_compress_internal.h:923-924`, `ZSTD_hash8`):
        // a single 64-bit multiply by `prime8bytes` followed by a high-bits
        // shift. Drops the CRC32d + rotate + mul kernel dispatch the rest
        // of the crate uses — for dfast the upstream zstd's scalar hash is
        // distribution-equivalent and one instruction shorter on the hot
        // path.
        let mixed = value.wrapping_mul(0xCF1BBCDCB7A56463_u64);
        (mixed >> (64 - bits)) as usize
    }
}

/// Per-kernel body of the dfast fast match loop. Mirrors the BT/opt
/// `*_body!` macros: the wrapper carries the `#[target_feature]` umbrella and
/// passes its tier `common_prefix_len_ptr` as `$cpl`, so the 7 match-extension
/// cpl calls inline under one umbrella and `select_kernel()` is resolved ONCE
/// per block in the bare dispatcher, never per cpl call.
macro_rules! start_matching_fast_loop_body {
    ($self:ident, $current_abs_start:ident, $current_len:ident, $handle_sequence:ident, $cpl:path, $use_dict:expr, $borrowed:expr) => {{
        // Behaviour change vs the pre-refactor `start_matching_general`:
        // this fast loop deliberately drops the strict-incompressible
        // early-skip path (the `block_looks_incompressible_strict` short
        // circuit + `miss_run` / `DFAST_LOCAL_SKIP_TRIGGER` thresholding).
        // The step ramp is now driven purely by distance traveled
        // (`DFAST_SKIP_STEP_GROWTH_INTERVAL = 256`, matching upstream zstd's
        // `kStepIncr = 1 << kSearchStrength`), so blocks the strict gate used to bail out of early now scan
        // through the standard ramp. `block_looks_incompressible_strict`
        // is still used by `levels/fastest.rs` for the Fastest preset
        // and by `incompressible.rs` unit tests, so the helper itself
        // stays.
        //
        // Upstream zstd outer/inner structure (`zstd_double_fast.c:167-322`):
        //   * outer `while(1)` runs once per match-found-and-stored;
        //   * inner `do { ... } while (ip1 <= ilimit)` carries `hl0`,
        //     `idxl0` between iterations, and precomputes `hl1` so the
        //     next iter's long hash is reused (one long-hash compute per
        //     two positions scanned, not per position).
        // We mirror that here. Per-frame invariants are hoisted before
        // the outer loop; mutable state (`position_base`, `history_abs_start`)
        // is re-snapshotted inside the outer loop because `emit_candidate`
        // → `insert_positions` → `ensure_room_for` can advance the rebase
        // base mid-frame.
        //
        // Rebase BEFORE any inner-loop slot pack. The hot loop computes
        // `packed_curr = (abs_ip0 - position_base) as u32 + 1` and writes
        // it straight into the hash tables, bypassing `insert_position`
        // (which is where `ensure_room_for` normally fires). On a long
        // stream of all-miss / non-hashable blocks the matcher can advance
        // `$current_abs_start` arbitrarily far without any per-byte insert,
        // so `position_base` may be stale by `> u32::MAX`. The first
        // fast-loop block after that would silently truncate `packed_curr`
        // and poison both hash tables. The guard band in `ensure_room_for`
        // (`DFAST_REBASE_GUARD_BAND`) covers the entire current block's
        // worth of positions, so a single call at function entry suffices.
        //
        // Raw-pointer aliasing invariant. The inner loop caches
        // `short_hash_ptr = $self.short_hash.as_mut_ptr()` and
        // `long_hash_ptr = $self.long_hash.as_mut_ptr()` (and a
        // `history_base_ptr` for the byte-buffer reads), then over
        // the rest of the function does:
        //
        //   * raw writes / reads via `*short_hash_ptr.add(...) =
        //     packed_curr`, `*long_hash_ptr.add(...) = packed_curr`,
        //     and `*long_hash_ptr.add(hl1_idx)`;
        //   * `&$self`-shared reads of `$self.offset_hist[0]` for the
        //     rep1 peek;
        //   * `&$self`-shared slice reads of `$self.history` (`let
        //     concat = &$self.history[history_start_offset..]`) for
        //     `extend_backwards_shared` invocations.
        //
        // This is sound under both Stacked Borrows and Tree Borrows
        // because the three fields touched (`short_hash`, `long_hash`,
        // `history`, `offset_hist`) are physically disjoint
        // allocations — `Vec<u32>`/`Vec<u8>` each own their own heap
        // buffer, and `offset_hist` is an inline `[u32; 3]` inside
        // the struct. A raw pointer derived from one field's Vec data
        // has provenance over that field only, so the subsequent
        // `&$self` reads of sibling fields don't reborrow through the
        // raw pointer's provenance tree.
        //
        // CRITICAL: this invariant must be preserved by any future
        // refactor that adds method calls inside the loop body. A
        // call that takes `&mut $self` (e.g.,
        // `$self.ensure_hash_tables()`, `$self.insert_position(...)`)
        // would reborrow the tables and invalidate the cached raw
        // pointers — every such call must happen OUTSIDE the
        // `'outer: loop` (as `ensure_room_for` does, hoisted to the
        // function preamble above) or be followed by a fresh
        // `as_mut_ptr()` reload of both `short_hash_ptr` /
        // `long_hash_ptr`. The outer-loop body already re-snapshots
        // `history_base_ptr` per iteration for exactly this reason
        // (`emit_candidate` → `insert_positions` may grow `history`
        // and trigger a realloc), so the same re-snapshot discipline
        // applies to the hash-table pointers.
        //
        // `$current_len > 0` is a precondition of this helper: every
        // caller (`start_matching`, `skip_matching`, `skip_matching_dense`)
        // returns early at `$current_len == 0`. Encode it as a hard
        // assert so a future caller that forgets the gate fails loudly
        // in debug rather than wrap-underflowing the `- 1` below; the
        // rebase is unconditional in release builds since the
        // precondition holds.
        debug_assert!($current_len > 0, "fast_loop precondition: $current_len > 0");
        // Owned only: advance the u32 packing rebase past this block. A
        // borrowed window packs absolute input offsets with `position_base ==
        // 0` (the eligibility gate caps `input_len <= u32::MAX`, so every
        // `abs + 1` slot fits without a reduce), and a `reduce()` here would
        // subtract from every live slot and corrupt that absolute encoding.
        if !$borrowed {
            $self.ensure_room_for($current_abs_start + $current_len - 1);
        }
        const PRIME: u64 = 0xCF1BBCDCB7A56463_u64;
        let short_shift = 64 - $self.short_hash_bits;
        let long_shift = 64 - $self.long_hash_bits;
        let mut pos = 1usize;
        let mut literals_start = 0usize;

        // Dict dual-probe (upstream zstd `ZSTD_dictMatchState`): snapshot the immutable
        // dict tables ONCE. Unlike the live tables (which `emit_candidate` may
        // grow / rebase), the dict tables are never mutated during matching, so
        // one snapshot before the outer loop stays valid every iteration. The
        // raw pointers hold no borrow, so the per-iter `&$self`/`&mut $self`
        // accesses below coexist. `use_dict` gates every probe so the no-dict
        // hot path keeps its exact instruction shape. `dict_end` is the
        // dict/input boundary as a CONCAT index (history_start-relative); the
        // dict is invalidated on any history eviction, so concat indices stay
        // valid for the snapshot's lifetime.
        // `use_dict` MUST track table presence, NOT `is_attached()`:
        // `prime_dict_tables_for_range` records the dict region (so
        // `is_attached()` is true) but returns before allocating the
        // tables when the hashable region is shorter than the short-hash
        // lookahead. Gating on `table().is_some()` keeps the null dict
        // pointers out of the probe below, which dereferences them before
        // the `dict_end` bound is consulted.
        // Dict probe pointers are materialised ONLY on the `USE_DICT` kernel.
        // The dispatcher monomorphises a separate no-dict kernel
        // (`$use_dict == false`, a compile-time const) in which this block and
        // every `if $use_dict` probe below const-fold away — so the hot no-dict
        // loop carries zero dict code and zero per-position dict check, instead
        // of branching on a loop-invariant flag every position (upstream zstd keeps the
        // no-dict and dictMatchState loops as separate functions for the same
        // reason). `$use_dict == true` is dispatched only when the table is
        // present, so the `expect` never fires.
        let (dict_long_ptr, dict_short_ptr, dict_end): (*const u32, *const u32, usize) =
            if $use_dict {
                let d = $self
                    .dict
                    .table()
                    .expect("USE_DICT kernel dispatched without a dict table");
                (d.long.as_ptr(), d.short.as_ptr(), $self.dict.region_len())
            } else {
                (core::ptr::null(), core::ptr::null(), 0)
            };

        // Advertised window cap = `1 << window_log`. Owned mode evicts, so
        // `history_abs_start` already bounds candidates to the live window;
        // borrowed mode keeps the whole input in place (no eviction), so an
        // OVER-window borrowed scan must explicitly reject candidates whose
        // offset would exceed the advertised window — otherwise it would emit
        // an offset the decoder cannot resolve. Hoisted (loop-invariant).
        let advertised_window = $self.max_window_size;

        'outer: loop {
            // Outer-iter precondition: at least `HASH_READ_SIZE = 8` bytes
            // ahead of `pos` so the unconditional 8-byte `u64` load below
            // is in-bounds for the live history buffer. `DFAST_MIN_MATCH_LEN
            // = 5` is the match acceptance threshold and is NOT a safe
            // load bound — using it here read up to 3 bytes past
            // `history.len()` on tiny blocks (CI fuzz `interop` crash,
            // 7-byte input).
            // NOTE: when this guard fires on the very first outer-iter
            // (tiny block, `$current_len < 9`), we `break 'outer` BEFORE
            // `tail_seed_anchor` is even declared. The post-loop
            // `seed_remaining_hashable_starts(.., pos)` then runs with
            // `pos` at its initial value (1 on first frame, or the
            // post-match cursor from a previous outer iter); that's the
            // correct tail seed for tiny blocks — `seed_pos = pos.min(
            // $current_len).min(boundary_tail_start)` plus the
            // `seed_pos + DFAST_SHORT_HASH_LOOKAHEAD <= $current_len`
            // guard inside the seeder keeps every insert in-bounds.
            if pos + HASH_READ_SIZE > $current_len {
                break 'outer;
            }
            let mut step = 1usize;
            let mut next_step_pos = pos + DFAST_SKIP_STEP_GROWTH_INTERVAL;
            let mut ip0 = pos;
            let mut ip1 = ip0 + step;
            // Same `HASH_READ_SIZE` rationale for `ip1`: the inner loop
            // pre-loads 8 bytes at `concat_idx1` for the `hl1` precompute
            // and the `_search_next_long` retry, so `ip1 + 8 <= $current_len`
            // must hold before we enter. If `ip0` is still hashable but
            // `ip1` is not (boundary case `pos == $current_len - 8`), we
            // still want to probe `ip0` — skipping it would leave the
            // last hashable position to `seed_remaining_hashable_starts`,
            // which inserts but does NOT search, and that drops real
            // matches in the tail window vs the upstream reference
            // (whose single-cursor loop probes every position p with
            // `p + HASH_READ_SIZE <= iend`). Handle the boundary inline
            // before exiting: a single-cursor probe at `ip0` (rep peek
            // and `_search_next_long` retry both depend on `ip1` so
            // they're skipped — upstream zstd accepts that exact tradeoff at
            // the iend boundary).
            if ip1 + HASH_READ_SIZE > $current_len {
                if let Some(committed) = $self.probe_tail_ip0_only(
                    $current_abs_start,
                    $current_len,
                    ip0,
                    literals_start,
                    $borrowed,
                ) {
                    let start = $self.emit_candidate(
                        $current_abs_start,
                        &mut literals_start,
                        committed,
                        $current_abs_start + ip0,
                        $handle_sequence,
                    );
                    pos = start + committed.match_len;
                    pos = $self.extend_with_repcode_after_match(
                        $current_abs_start,
                        $current_len,
                        pos,
                        &mut literals_start,
                        $handle_sequence,
                    );
                    continue 'outer;
                }
                break 'outer;
            }

            // Re-read every per-frame-mutable cursor here — `emit_candidate`
            // in the previous outer iteration may have triggered a rebase.
            // The byte-source quintet comes through `scan_source()` so a
            // borrowed one-shot window can substitute the owned `history`
            // concat without touching this kernel body: the owned path
            // returns its rebased fields, a borrowed window its constant
            // descriptor. The hash-table pointers are mode-invariant (the
            // tables persist across owned/borrowed) so they stay direct.
            // `$borrowed` is a compile-time const (the kernel is monomorphised
            // per borrowed/owned), so only one arm survives. On the borrowed
            // kernel `history_start_offset / history_abs_start / position_base`
            // are LITERAL `0`, collapsing every per-position `abs - abs_start`
            // term and the always-true `cand_pos >= abs_start` lower-bound
            // check to the bare absolute position (upstream zstd `base + index` shape).
            let (history_base_ptr, history_start_offset, history_abs_start, position_base, concat_len) =
                if $borrowed {
                    let (ptr, block_end) = $self.borrowed_scan_descriptor();
                    (ptr, 0usize, 0usize, 0usize, block_end)
                } else {
                    $self.owned_scan_descriptor()
                };
            let short_hash_ptr = $self.short_mut_ptr();
            let long_hash_ptr = $self.long_mut_ptr();

            // Pre-compute long hash at ip0 ONCE per outer iter.
            // `concat_idx = ($current_abs_start + ip0) - history_abs_start`
            // is the byte offset within `live_history`.
            let mut hl0_idx;
            let mut idxl0;
            // SAFETY: `$current_abs_start + ip0 >= history_abs_start`
            // (`ip` is inside the current block, which is part of live
            // history). The 8-byte unaligned load on
            // `concat[concat_idx..]` is safe because the outer loop
            // guards above enforce `ip0 + HASH_READ_SIZE <= $current_len`,
            // and `$current_abs_start + $current_len - history_abs_start
            // <= concat_len` (live history contains the full current
            // block plus any retained earlier blocks), giving
            // `concat_idx + 8 <= concat_len`. The `debug_assert!` below
            // makes that invariant explicit so a future refactor that
            // touches eviction / `compact_history` / `trim_to_window`
            // semantics catches a violation in tests instead of leaking
            // through to ASan UB.
            unsafe {
                let concat_idx = ($current_abs_start + ip0) - history_abs_start;
                debug_assert!(
                    concat_idx + HASH_READ_SIZE <= concat_len,
                    "fast-loop 8-byte load OOB: concat_idx={} HASH_READ_SIZE={} concat_len={}",
                    concat_idx,
                    HASH_READ_SIZE,
                    concat_len,
                );
                let v8 = (history_base_ptr.add(history_start_offset + concat_idx) as *const u64)
                    .read_unaligned();
                hl0_idx = (v8.wrapping_mul(PRIME) >> long_shift) as usize;
                idxl0 = *long_hash_ptr.add(hl0_idx);
            }

            // Inner-loop exit shape. Every `break 'inner` produces a
            // value of this type, so the type system enforces the
            // previously implicit pairing between "did we commit a
            // match?" and "where does the tail seeder pick up?":
            //
            //   * `Committed(c)` — rep1 / long / short(+next_long retry)
            //     paths chose `c`; outer arm runs emit + rep-extension.
            //   * `Tail(seed)` — inner ran out of safe scan room at
            //     `seed = ip0` (first un-scanned position); outer arm
            //     hands `seed` to `seed_remaining_hashable_starts` so
            //     the tail seeder does not redundantly re-pack
            //     positions the fast loop already wrote (which is what
            //     restarting from outer-entry `pos` would do on a
            //     miss-only block — throwing away the skip-step win on
            //     incompressible data).
            //
            // Adding a new `break 'inner` variant now forces the
            // author to pick a `InnerExit::…` variant explicitly; the
            // previous `Option<MatchCandidate>` + sibling `usize`
            // pairing relied on a comment-block to flag the coupling.
            enum InnerExit {
                // `u8` is a debug path tag (0=rep1, 1=long@ip0, 2=short/_search_next_long,
                // 3=dict-long, 4=dict-snl), surfaced by the `DFTRACE` env gate in the
                // commit handler to diagnose match-path / offset divergence vs C ffi.
                // `usize` is the iteration's scan position `abs_ip0` (upstream `curr`,
                // zstd_double_fast.c:184) — the complementary insertion anchors on it,
                // NOT on the (rep1 `+1` / catch-up adjusted) match start.
                Committed(MatchCandidate, u8, usize),
                Tail(usize),
            }

            let inner_exit: InnerExit = 'inner: loop {
                let abs_ip0 = $current_abs_start + ip0;
                let abs_ip1 = $current_abs_start + ip1;
                // Per-position candidate window-low bound (see `advertised_window`
                // above). `$borrowed` is const, so owned collapses to
                // `history_abs_start` (byte-identical) and borrowed-in-window
                // saturates to 0 (== history_abs_start, also byte-identical);
                // only borrowed-over-window gains the `abs_ip - window` cap.
                let wlow0 = if $borrowed {
                    abs_ip0.saturating_sub(advertised_window)
                } else {
                    history_abs_start
                };
                let wlow1 = if $borrowed {
                    abs_ip1.saturating_sub(advertised_window)
                } else {
                    history_abs_start
                };
                let lit_len_ip0 = ip0 - literals_start;
                let lit_len_ip1 = ip1 - literals_start;
                let packed_curr = ((abs_ip0 - position_base) as u32) + 1;

                let concat_idx0 = abs_ip0 - history_abs_start;
                let concat_idx1 = abs_ip1 - history_abs_start;

                // Load 8 bytes at ip0 for both short (low 4) and long
                // probe equality checks. We already used `v8_at_ip0` to
                // compute hl0/idxl0 in the outer init / previous iter's
                // carry; reload now (cheap unaligned read) so the
                // `read_unaligned` is from the same offset the
                // probe-eq below will use.
                let v8_0 = unsafe {
                    (history_base_ptr.add(history_start_offset + concat_idx0) as *const u64)
                        .read_unaligned()
                };
                // `v4_0` (low 4 bytes) is the cheap 4-byte equality-gate key
                // below; the short HASH keys on the upstream zstd 5-byte window
                // (`v8_0 << 24`, ZSTD_hash5 shape) to match `short_hash_index`.
                let v4_0 = v8_0 & 0xFFFF_FFFF;
                let hs0_idx = ((v8_0 << 24).wrapping_mul(PRIME) >> short_shift) as usize;
                let idxs0 = unsafe { *short_hash_ptr.add(hs0_idx) };

                // Upstream zstd parity (`zstd_double_fast.c:187`): update BOTH
                // tables at curr BEFORE checking matches. The benefit is
                // for hash-table consumers, NOT the rep peek (rep at ip+1
                // reads `offset_hist[0]`, never the hash tables). The
                // upstream zstd rationale is the long-hash retry path: the next
                // inner iter's `idxl1` lookup (`hashLong[hl1_idx]`) can
                // collide with this iter's `hl0_idx`, and writing curr
                // first means a $self-collision still resolves to a real
                // match instead of the previous occupant. The short
                // probe of the same iter and the `_search_next_long`
                // retry at ip+1 are the other consumers that see the
                // fresh write.
                unsafe {
                    *long_hash_ptr.add(hl0_idx) = packed_curr;
                    *short_hash_ptr.add(hs0_idx) = packed_curr;
                }

                // Upstream zstd parity (`zstd_double_fast.c:190`): inline rep1
                // peek at ip+1, 4-byte gate. Upstream zstd's hot path checks ONLY
                // `offset_1` here (full 3-rep walk lives in lazy/btopt).
                // Since the peek is at `ip+1` with `pos >= literals_start`,
                // `lit_len_ip1 >= 1`, so `offset_hist[0]` is the upstream zstd's
                // `offset_1`. The `repcode_candidate_shared` helper we used
                // before walked all three offsets + did a full SIMD
                // `common_prefix_len` per probe, paying ~3× the work for
                // rep2/rep3 hits that the dfast fast path never benefits
                // from (those wins live in the lazy/btopt strategies).
                let rep1 = $self.offset_hist[0] as usize;
                if rep1 != 0 && rep1 <= abs_ip1 {
                    let cand_pos_r = abs_ip1 - rep1;
                    // Window-low bound ONLY (upstream zstd `zstd_double_fast.c:190`
                    // gates the rep on nothing but `offset_1 > 0` and the 4-byte
                    // equality at `ip+1-offset_1`, i.e. the candidate is in the
                    // prefix). A prior extra `cand_pos_r >= literals_start` clause
                    // rejected essentially EVERY rep — after a match
                    // `literals_start ≈ ip`, so any back-reference (cand before
                    // the literal cursor) failed it — collapsing the rep path and
                    // forcing the long-hash to mint creeping offsets.
                    if cand_pos_r >= wlow1 {
                        let cand_idx_r = cand_pos_r - history_abs_start;
                        // 4-byte gate; full forward count only if it passes.
                        let cand4 = unsafe {
                            (history_base_ptr.add(history_start_offset + cand_idx_r) as *const u32)
                                .read_unaligned()
                        };
                        let cur4 = unsafe {
                            (history_base_ptr.add(history_start_offset + concat_idx1) as *const u32)
                                .read_unaligned()
                        };
                        if cand4 == cur4 {
                            let mut match_len = 4usize;
                            let max_fwd = concat_len.saturating_sub(concat_idx1 + 4);
                            unsafe {
                                let lhs =
                                    history_base_ptr.add(history_start_offset + cand_idx_r + 4);
                                let rhs =
                                    history_base_ptr.add(history_start_offset + concat_idx1 + 4);
                                let ext = $cpl(
                                    lhs, rhs, max_fwd,
                                );
                                match_len += ext;
                            }
                            // Rep extensions use the 4-byte
                            // `DFAST_REP_MIN_MATCH_LEN` floor, NOT the
                            // hash-search `DFAST_MIN_MATCH_LEN = 5`. Rep
                            // coding has no on-wire offset cost, so the
                            // upstream reference accepts 4-byte rep
                            // hits; gating at 5 here would silently drop
                            // every 4-byte rep1 the upstream encoder
                            // produces, and leave the post-match
                            // `extend_with_repcode_after_match` chain
                            // (which uses 4) inconsistent with the peek.
                            if match_len >= DFAST_REP_MIN_MATCH_LEN {
                                // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                                let rep_cand = extend_backwards_shared(
                                    concat,
                                    history_abs_start,
                                    cand_pos_r,
                                    abs_ip1,
                                    match_len,
                                    lit_len_ip1,
                                );
                                break 'inner InnerExit::Committed(rep_cand, 0, abs_ip0);
                            }
                        }
                    }
                }

                // Precompute hl1 (upstream zstd `_search_next_long` carry, line 197).
                let v8_1 = unsafe {
                    (history_base_ptr.add(history_start_offset + concat_idx1) as *const u64)
                        .read_unaligned()
                };
                let hl1_idx = (v8_1.wrapping_mul(PRIME) >> long_shift) as usize;

                // Prefetch the RANDOM-ACCESS hash-table slots the loop is about
                // to read, while there is work to hide the latency behind. The
                // match-find loop is memory-latency bound on these table loads
                // (perf --call-graph dwarf annotate: ~21% self-time across the
                // long/short slot reads), and the hardware prefetcher cannot
                // predict a hash-indexed address. `hl1_idx`'s slot is loaded
                // ~100 instructions below (the `_search_next_long` retry at
                // `ip1`); the next inner iteration's short-hash slot is keyed on
                // these same `v8_1` bytes (this `ip1` becomes the next `ip0`).
                // Unlike the upstream zstd's input `PREFETCH_L1(ip + 256)` — which only
                // warms the sequential input the HW prefetcher already covers —
                // these target the loads that actually stall.
                unsafe {
                    let hs1_idx = ((v8_1 << 24).wrapping_mul(PRIME) >> short_shift) as usize;
                    crate::decoding::prefetch::prefetch_l1_at(long_hash_ptr.add(hl1_idx) as *const u8);
                    crate::decoding::prefetch::prefetch_l1_at(
                        short_hash_ptr.add(hs1_idx) as *const u8,
                    );
                }

                // Long match check at ip0 with idxl0. 8-byte equality
                // gate (`MEM_read64`) — if it passes, candidate is real.
                //
                // Branchy validity rather than a branchless `in_long` mask: the
                // slot-populated / in-window / before-cursor checks are highly
                // predictable once the table is warm, so the branch predictor
                // speculates the hot 8-byte candidate load past them. A
                // branchless mask instead makes the load ADDRESS depend on the
                // mask (`cand_idx &= -(in_long)`), serialising the loop's single
                // hottest instruction (~13% self-time on z000033) behind the
                // mask — a stall the predictor cannot hide.
                if idxl0 != DFAST_EMPTY_SLOT {
                    let cand_pos = position_base + ((idxl0 as usize) - 1);
                    if cand_pos >= wlow0 && cand_pos < abs_ip0 {
                        let cand_idx = cand_pos - history_abs_start;
                        // SAFETY: the bounds above make `cand_idx` a valid
                        // in-window concat index, so the 8-byte load stays in
                        // live history (same buffer/length bounds as `v8_0`).
                        let cand_v8 = unsafe {
                            (history_base_ptr.add(history_start_offset + cand_idx) as *const u64)
                                .read_unaligned()
                        };
                        if cand_v8 == v8_0 {
                            {
                            // 8 bytes match; count forward + extend back.
                            let mut match_len = 8usize;
                            let max_fwd = concat_len.saturating_sub(concat_idx0 + 8);
                            // SAFETY: both ptrs at the same buffer; offsets
                            // verified above. `max_fwd` caps the scan to
                            // the live region.
                            unsafe {
                                let lhs = history_base_ptr.add(history_start_offset + cand_idx + 8);
                                let rhs =
                                    history_base_ptr.add(history_start_offset + concat_idx0 + 8);
                                let ext = $cpl(
                                    lhs, rhs, max_fwd,
                                );
                                match_len += ext;
                            }
                            // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                            let cand = extend_backwards_shared(
                                concat,
                                history_abs_start,
                                cand_pos,
                                abs_ip0,
                                match_len,
                                lit_len_ip0,
                            );
                            // Upstream zstd `_match_found` (zstd_double_fast.c:287):
                            // `if (step < 4) hashLong[hl1] = ip1`. Insert the
                            // lookahead position's long hash before storing the
                            // match — safe only while `step < 4` (then `ip1` is
                            // below the post-match cursor `ip + mLength`).
                            if step < 4 {
                                let packed_ip1 = ((abs_ip1 - position_base) as u32) + 1;
                                unsafe {
                                    *long_hash_ptr.add(hl1_idx) = packed_ip1;
                                }
                            }
                            break 'inner InnerExit::Committed(cand, 1, abs_ip0);
                        }
                    }
                    }
                }

                // Dict long fallback (upstream zstd `dictMatchState`): the live long
                // missed (empty / out-of-window / 8-byte mismatch). Probe the
                // immutable dict long table at the SAME `hl0_idx`. Flat model:
                // the dict sits in the contiguous history before the input, so
                // a dict match is `offset = abs_ip0 - dict_abs` and the forward
                // count crosses the dict→input boundary like any in-window
                // match (no `dictBase`/`count_2segments`).
                if $use_dict {
                    // SAFETY: when `use_dict`, `dict_long_ptr` is non-null and
                    // sized `1 << long_hash_bits`; `hl0_idx < 1 << long_hash_bits`.
                    let dl = unsafe { *dict_long_ptr.add(hl0_idx) };
                    if dl != DFAST_EMPTY_SLOT {
                        let dp = (dl as usize) - 1;
                        // Dict long slots were only written for positions with
                        // 8-byte lookahead, so `dp + 8 <= dict_len <= concat_len`;
                        // `dp < dict_end` keeps the match inside the dict region.
                        if dp < dict_end {
                            debug_assert!(
                                dp + HASH_READ_SIZE <= concat_len,
                                "dict long load OOB: dp={dp} concat_len={concat_len}",
                            );
                            // SAFETY: `dp + 8 <= concat_len` (above) ⇒ the 8-byte
                            // load at concat `dp` is in-bounds for live history.
                            let dcand_v8 = unsafe {
                                (history_base_ptr.add(history_start_offset + dp) as *const u64)
                                    .read_unaligned()
                            };
                            if dcand_v8 == v8_0 {
                                let mut match_len = 8usize;
                                let max_fwd = concat_len.saturating_sub(concat_idx0 + 8);
                                // SAFETY: both ptrs in the same buffer; `max_fwd`
                                // caps the scan to the live region.
                                unsafe {
                                    let lhs = history_base_ptr.add(history_start_offset + dp + 8);
                                    let rhs = history_base_ptr
                                        .add(history_start_offset + concat_idx0 + 8);
                                    let ext =
                                        $cpl(
                                            lhs, rhs, max_fwd,
                                        );
                                    match_len += ext;
                                }
                                let cand_pos = history_abs_start + dp;
                                // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                                let cand = extend_backwards_shared(
                                    concat,
                                    history_abs_start,
                                    cand_pos,
                                    abs_ip0,
                                    match_len,
                                    lit_len_ip0,
                                );
                                break 'inner InnerExit::Committed(cand, 3, abs_ip0);
                            }
                        }
                    }
                }

                let idxl1 = unsafe { *long_hash_ptr.add(hl1_idx) };

                // Short match check at ip0 with idxs0 — 4-byte gate
                // ONLY (upstream zstd `zstd_double_fast.c:220`). Forward count
                // and `_search_next_long` retry happen ONLY on hit.
                // Branchy validity, same shape as the ip1 long retry below:
                // the slot-populated / in-window / before-cursor checks are
                // predictable after warmup, so the predictor speculates the
                // 4-byte candidate load past them. A branchless mask would tie
                // the load address to the mask, serialising it.
                if idxs0 != DFAST_EMPTY_SLOT {
                    let cand_pos_s = position_base + ((idxs0 as usize) - 1);
                    if cand_pos_s >= wlow0 && cand_pos_s < abs_ip0 {
                        let cand_idx_s = cand_pos_s - history_abs_start;
                        let cand4 = unsafe {
                            (history_base_ptr.add(history_start_offset + cand_idx_s) as *const u32)
                                .read_unaligned()
                        };
                        if cand4 == v4_0 as u32 {
                            {
                            // Short hit: count forward from byte 4 onwards.
                            let mut s_match_len = 4usize;
                            let max_fwd = concat_len.saturating_sub(concat_idx0 + 4);
                            unsafe {
                                let lhs =
                                    history_base_ptr.add(history_start_offset + cand_idx_s + 4);
                                let rhs =
                                    history_base_ptr.add(history_start_offset + concat_idx0 + 4);
                                let ext = $cpl(
                                    lhs, rhs, max_fwd,
                                );
                                s_match_len += ext;
                            }
                            // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                            let short_cand = extend_backwards_shared(
                                concat,
                                history_abs_start,
                                cand_pos_s,
                                abs_ip0,
                                s_match_len,
                                lit_len_ip0,
                            );

                            // Enforce the hash-search floor BEFORE
                            // committing this short hit. The 4-byte
                            // equality gate plus forward-count extension
                            // can produce `short_cand.match_len < 5`
                            // (the literal 4-byte hit, no extension).
                            // `probe_slot_match` rejects below-floor
                            // long-hash hits at the same gate; the
                            // short-hash path must do the same so the
                            // fast loop never emits a sub-floor non-rep
                            // match. The retry below can still upgrade
                            // to a long hit (which has its own 8-byte
                            // floor, comfortably above `DFAST_MIN_MATCH_LEN`).
                            let short_hit_valid = short_cand.match_len >= DFAST_MIN_MATCH_LEN;

                            // Upstream zstd `_search_next_long` retry (line 260):
                            // try long match at ip1 with precomputed idxl1.
                            // If it produces a strictly longer match, use it.
                            let mut chosen = short_cand;
                            let mut retry_upgraded = false;
                            let mut live_l1_hit = false;
                            if idxl1 != DFAST_EMPTY_SLOT {
                                let cand_pos_l1 = position_base + (idxl1 as usize) - 1;
                                if cand_pos_l1 >= wlow1 && cand_pos_l1 < abs_ip1 {
                                    let cand_idx_l1 = cand_pos_l1 - history_abs_start;
                                    let cand_v8_l1 = unsafe {
                                        (history_base_ptr.add(history_start_offset + cand_idx_l1)
                                            as *const u64)
                                            .read_unaligned()
                                    };
                                    if cand_v8_l1 == v8_1 {
                                        live_l1_hit = true;
                                        let mut l1_match_len = 8usize;
                                        let max_fwd_l1 = concat_len.saturating_sub(concat_idx1 + 8);
                                        unsafe {
                                            let lhs = history_base_ptr
                                                .add(history_start_offset + cand_idx_l1 + 8);
                                            let rhs = history_base_ptr
                                                .add(history_start_offset + concat_idx1 + 8);
                                            let ext = $cpl(
                                                lhs, rhs, max_fwd_l1,
                                            );
                                            l1_match_len += ext;
                                        }
                                        if l1_match_len > short_cand.match_len {
                                            // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                                            chosen = extend_backwards_shared(
                                                concat,
                                                history_abs_start,
                                                cand_pos_l1,
                                                abs_ip1,
                                                l1_match_len,
                                                lit_len_ip1,
                                            );
                                            // Long-hash hits start at 8
                                            // bytes (`MEM_read64` gate
                                            // above), well above
                                            // `DFAST_MIN_MATCH_LEN = 5`,
                                            // so the retry upgrade is
                                            // always valid even when
                                            // the raw short hit was
                                            // below the floor.
                                            retry_upgraded = true;
                                        }
                                    }
                                }
                            }
                            // Dict long match at ip1 (upstream zstd `_search_next_long`
                            // dict arm, zstd_double_fast.c:472-483): probed ONLY
                            // when the live long+1 missed, mirroring upstream zstd's
                            // `else if dictTagsMatchL3`. Attach-mode keeps the
                            // dict in a SEPARATE table, so the live long+1 probe
                            // above never sees dict positions; without this the
                            // dict-long upgrade the old dense-reprime path got
                            // for free (dict positions lived in the live table)
                            // is lost and the loop emits the shorter short match.
                            if !live_l1_hit && $use_dict {
                                // SAFETY: `use_dict` ⇒ `dict_long_ptr` non-null,
                                // sized `1 << long_hash_bits`; `hl1_idx` is in range.
                                let dl1 = unsafe { *dict_long_ptr.add(hl1_idx) };
                                if dl1 != DFAST_EMPTY_SLOT {
                                    let dp1 = (dl1 as usize) - 1;
                                    if dp1 < dict_end {
                                        debug_assert!(
                                            dp1 + HASH_READ_SIZE <= concat_len,
                                            "dict long+1 load OOB: dp1={dp1} concat_len={concat_len}",
                                        );
                                        // SAFETY: `dp1 + 8 <= concat_len` ⇒ the
                                        // 8-byte load at concat `dp1` is in-bounds.
                                        let dcand_v8_l1 = unsafe {
                                            (history_base_ptr.add(history_start_offset + dp1)
                                                as *const u64)
                                                .read_unaligned()
                                        };
                                        if dcand_v8_l1 == v8_1 {
                                            let mut dl1_match_len = 8usize;
                                            let max_fwd =
                                                concat_len.saturating_sub(concat_idx1 + 8);
                                            // SAFETY: same buffer; `max_fwd` caps
                                            // the scan to the live region.
                                            unsafe {
                                                let lhs = history_base_ptr
                                                    .add(history_start_offset + dp1 + 8);
                                                let rhs = history_base_ptr
                                                    .add(history_start_offset + concat_idx1 + 8);
                                                let ext = $cpl(
                                                    lhs, rhs, max_fwd,
                                                );
                                                dl1_match_len += ext;
                                            }
                                            if dl1_match_len > short_cand.match_len {
                                                let cand_pos = history_abs_start + dp1;
                                                // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                                                chosen = extend_backwards_shared(
                                                    concat,
                                                    history_abs_start,
                                                    cand_pos,
                                                    abs_ip1,
                                                    dl1_match_len,
                                                    lit_len_ip1,
                                                );
                                                retry_upgraded = true;
                                            }
                                        }
                                    }
                                }
                            }
                            if short_hit_valid || retry_upgraded {
                                // Upstream zstd `_match_found` (zstd_double_fast.c:287):
                                // `if (step < 4) hashLong[hl1] = ip1`.
                                if step < 4 {
                                    let packed_ip1 = ((abs_ip1 - position_base) as u32) + 1;
                                    unsafe {
                                        *long_hash_ptr.add(hl1_idx) = packed_ip1;
                                    }
                                }
                                break 'inner InnerExit::Committed(chosen, 2, abs_ip0);
                            }
                            // Below-floor short hit with no retry
                            // upgrade — fall through to the step bump
                            // and keep scanning. Discarding here is
                            // correctness-relevant: emitting a 4-byte
                            // non-rep match would mint an offset on
                            // wire that costs more than the 4-byte
                            // payload buys (offsets at level 2/3 dfast
                            // are 13–17 bits depending on offset class
                            // and prior offset_hist state — strictly
                            // worse than emitting the 4 bytes as
                            // literals).
                        }
                    }
                    }
                }

                // Dict short fallback (upstream zstd `dictMatchState`): the live short
                // missed / was below floor. Probe the immutable dict short
                // table at the SAME `hs0_idx`, 4-byte gate, forward count, then
                // enforce the same `DFAST_MIN_MATCH_LEN` floor the live short
                // path uses (a sub-floor non-rep match mints a wire offset that
                // costs more than emitting the bytes as literals). No
                // `_search_next_long` retry: the dict long fallback already
                // covers the long-upgrade case at `ip0`.
                if $use_dict {
                    // SAFETY: `use_dict` ⇒ `dict_short_ptr` non-null, sized
                    // `1 << short_hash_bits`; `hs0_idx < 1 << short_hash_bits`.
                    let ds = unsafe { *dict_short_ptr.add(hs0_idx) };
                    if ds != DFAST_EMPTY_SLOT {
                        let dp = (ds as usize) - 1;
                        if dp < dict_end {
                            debug_assert!(
                                dp + 4 <= concat_len,
                                "dict short load OOB: dp={dp} concat_len={concat_len}",
                            );
                            // SAFETY: short slots were only written with 4 bytes
                            // of lookahead ⇒ `dp + 4 <= dict_len <= concat_len`.
                            let dcand4 = unsafe {
                                (history_base_ptr.add(history_start_offset + dp) as *const u32)
                                    .read_unaligned()
                            };
                            if dcand4 == v4_0 as u32 {
                                let mut s_match_len = 4usize;
                                let max_fwd = concat_len.saturating_sub(concat_idx0 + 4);
                                unsafe {
                                    let lhs = history_base_ptr.add(history_start_offset + dp + 4);
                                    let rhs = history_base_ptr
                                        .add(history_start_offset + concat_idx0 + 4);
                                    let ext =
                                        $cpl(
                                            lhs, rhs, max_fwd,
                                        );
                                    s_match_len += ext;
                                }
                                let cand_pos = history_abs_start + dp;
                                // SAFETY: `history_base_ptr + history_start_offset` is the live
                                // source start (owned `history[history_start..]` or the
                                // borrowed input slice) and `concat_len` its readable byte
                                // count, both from `scan_source()` at the top of this outer
                                // iter; `extend_backwards_shared` only indexes within the
                                // candidate/cursor range it is handed, all `< concat_len`.
                                let concat = unsafe {
                                    core::slice::from_raw_parts(
                                        history_base_ptr.add(history_start_offset),
                                        concat_len,
                                    )
                                };
                                let dcand = extend_backwards_shared(
                                    concat,
                                    history_abs_start,
                                    cand_pos,
                                    abs_ip0,
                                    s_match_len,
                                    lit_len_ip0,
                                );
                                if dcand.match_len >= DFAST_MIN_MATCH_LEN {
                                    break 'inner InnerExit::Committed(dcand, 4, abs_ip0);
                                }
                            }
                        }
                    }
                }

                // Step bump on distance (upstream zstd `zstd_double_fast.c:224-228`).
                // Upstream grows the step unbounded (one per `kStepIncr` travelled);
                // no cap, so the scan stride matches byte-for-byte.
                if ip1 >= next_step_pos {
                    step += 1;
                    next_step_pos += DFAST_SKIP_STEP_GROWTH_INTERVAL;
                }

                // Advance: ip0 = ip1; ip1 += step; carry hl1 → hl0 / idxl1 → idxl0.
                ip0 = ip1;
                ip1 += step;
                hl0_idx = hl1_idx;
                idxl0 = idxl1;
                if ip1 + HASH_READ_SIZE > $current_len {
                    // First position the fast loop did NOT pack into the
                    // hash tables. `seed_remaining_hashable_starts` will
                    // pick up from `ip0` instead of restarting at the
                    // outer-entry `pos`.
                    break 'inner InnerExit::Tail(ip0);
                }
            };

            match inner_exit {
                InnerExit::Committed(candidate, _path_tag, scan_pos) => {
                    // `DFTRACE` env gate: dump each committed match's path tag +
                    // (offset, match_len, literal_len) so the match-path / offset
                    // stream can be diffed against C ffi when chasing a dfast
                    // ratio divergence. The env is read ONCE into a cached flag
                    // (a per-commit `getenv` showed up at ~3% of small-frame
                    // encode); off by default, an atomic load in production.

                    if *DFTRACE_ENABLED.get_or_init(|| std::env::var_os("DFTRACE").is_some()) {
                        std::eprintln!(
                            "DFT path={} off={} ml={} ll={}",
                            _path_tag,
                            candidate.offset,
                            candidate.match_len,
                            candidate.start - $current_abs_start - literals_start,
                        );
                    }
                    let start = $self.emit_candidate(
                        $current_abs_start,
                        &mut literals_start,
                        candidate,
                        scan_pos,
                        $handle_sequence,
                    );
                    pos = start + candidate.match_len;
                    pos = $self.extend_with_repcode_after_match(
                        $current_abs_start,
                        $current_len,
                        pos,
                        &mut literals_start,
                        $handle_sequence,
                    );
                }
                InnerExit::Tail(seed) => {
                    // Inner loop ran out of safe scan room without
                    // committing. Hand the tail seeder the first
                    // un-scanned position so it does not redundantly
                    // re-pack everything the fast loop already wrote.
                    pos = seed;
                    break 'outer;
                }
            }
        }

        $self.seed_remaining_hashable_starts($current_abs_start, $current_len, pos);
        $self.emit_trailing_literals($current_abs_start, literals_start, $handle_sequence);
    }};
}

impl DfastMatchGenerator {
    /// Dispatcher for the per-kernel dfast fast loop: resolve the tier ONCE
    /// per block via `select_kernel()` and call the matching
    /// `start_matching_fast_loop_<kernel>` wrapper, so every per-position
    /// `common_prefix_len_ptr` inlines under one `#[target_feature]` umbrella
    /// (mirrors `build_optimal_plan_impl`). Replaces the prior per-cpl
    /// `dispatch_common_prefix_len_ptr` runtime dispatch.
    fn start_matching_fast_loop(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        // Resolve dict presence ONCE here, off the hot path, and select a
        // const-monomorphised kernel (`USE_DICT` true/false) so the per-position
        // dict probe is compiled in or out at the call shape — never a
        // loop-invariant runtime check inside the scan. The no-dict kernel
        // carries zero dict code (upstream zstd keeps the noDict / dictMatchState loops
        // as separate functions for exactly this reason).
        // Two orthogonal axes resolved ONCE here, off the hot path, into a
        // const-monomorphised kernel:
        //   * `USE_DICT` — dict probe compiled in or out (upstream zstd keeps noDict /
        //     dictMatchState as separate functions for the same reason).
        //   * `BORROWED` — borrowed-window scan vs owned history concat. The
        //     borrowed kernel folds the rebase coordinates to literal `0`,
        //     erasing the per-position abstraction arithmetic the owned path
        //     needs (upstream zstd `base + index` shape).
        // A borrowed block never carries a dict (`borrowed_eligible` rejects
        // `use_dictionary_state`), so only three of the four combinations are
        // ever instantiated; the `<true, true>` arm is unreachable.
        let use_dict = self.dict.table().is_some();
        let borrowed = self.borrowed_block.is_some();
        macro_rules! dispatch_dict {
            ($kernel:ident) => {
                if borrowed {
                    self.$kernel::<false, true>(current_abs_start, current_len, handle_sequence)
                } else if use_dict {
                    self.$kernel::<true, false>(current_abs_start, current_len, handle_sequence)
                } else {
                    self.$kernel::<false, false>(current_abs_start, current_len, handle_sequence)
                }
            };
        }
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            dispatch_dict!(start_matching_fast_loop_neon)
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            // Use the matcher-level cached kernel (resolved once in `new()`),
            // not a per-block `select_kernel()` — the cache only helps if the
            // hot path actually reads it. Mirrors the Row backend.
            match self.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    dispatch_dict!(start_matching_fast_loop_avx2_bmi2)
                },
                FastpathKernel::Sse42 => unsafe { dispatch_dict!(start_matching_fast_loop_sse42) },
                FastpathKernel::Scalar => dispatch_dict!(start_matching_fast_loop_scalar),
            }
        }
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        unsafe {
            dispatch_dict!(start_matching_fast_loop_simd128)
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64",
            all(target_arch = "wasm32", target_feature = "simd128")
        )))]
        {
            dispatch_dict!(start_matching_fast_loop_scalar)
        }
    }

    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    unsafe fn start_matching_fast_loop_neon<const USE_DICT: bool, const BORROWED: bool>(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_fast_loop_body!(
            self,
            current_abs_start,
            current_len,
            handle_sequence,
            crate::encoding::fastpath::neon::common_prefix_len_ptr,
            USE_DICT,
            BORROWED
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    unsafe fn start_matching_fast_loop_sse42<const USE_DICT: bool, const BORROWED: bool>(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_fast_loop_body!(
            self,
            current_abs_start,
            current_len,
            handle_sequence,
            crate::encoding::fastpath::sse42::common_prefix_len_ptr,
            USE_DICT,
            BORROWED
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    unsafe fn start_matching_fast_loop_avx2_bmi2<const USE_DICT: bool, const BORROWED: bool>(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_fast_loop_body!(
            self,
            current_abs_start,
            current_len,
            handle_sequence,
            crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
            USE_DICT,
            BORROWED
        )
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    #[target_feature(enable = "simd128")]
    unsafe fn start_matching_fast_loop_simd128<const USE_DICT: bool, const BORROWED: bool>(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_fast_loop_body!(
            self,
            current_abs_start,
            current_len,
            handle_sequence,
            crate::encoding::fastpath::simd128::common_prefix_len_ptr,
            USE_DICT,
            BORROWED
        )
    }

    #[cfg(not(any(
        all(target_arch = "aarch64", target_endian = "little"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    #[allow(unused_unsafe)]
    fn start_matching_fast_loop_scalar<const USE_DICT: bool, const BORROWED: bool>(
        &mut self,
        current_abs_start: usize,
        current_len: usize,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_fast_loop_body!(
            self,
            current_abs_start,
            current_len,
            handle_sequence,
            crate::encoding::fastpath::scalar::common_prefix_len_ptr,
            USE_DICT,
            BORROWED
        )
    }
}
