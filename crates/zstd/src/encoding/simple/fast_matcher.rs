#![allow(
    clippy::checked_conversions,
    clippy::doc_link_code,
    clippy::inline_always,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned,
    clippy::suspicious_operation_groupings,
    reason = "Block and dictionary offsets intentionally differ; preserve the pinned upstream match extension and kernel spelling"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Upstream zstd-shape Fast strategy matcher backend — selected for every
//! Fast-strategy level (Uncompressed, Fastest, Level(1), and the
//! negative Level(-7..=-1) variants). Per-level dispatch on
//! `(fast_hash_log, fast_mls, fast_step_size)` is wired through
//! `LevelParams` → `with_params` / `reset` (step_size is part of
//! the construction/reset signature, not a separate setter).
//! Level(1) uses `(hash_log=14, mls=7, step_size=2)`; Fastest /
//! Uncompressed / Level(-1..=-7) use `(hash_log=14, mls=6)` with
//! `step_size` 2..8 driving upstream zstd's acceleration gradient on
//! negative levels.
//!
//! `use_cmov` is derived directly from `window_log` inside the
//! matcher (upstream zstd heuristic `windowLog < 19`) — NOT a
//! `LevelParams` field.
//!
//! Wraps the kernel from
//! [`super::fast_kernel::kernel::compress_block_fast`] and presents the
//! `Matcher` API expected by [`crate::encoding::match_generator::MatchGeneratorDriver`].
//! Replaces the SuffixStore-based `MatchGenerator` for the Fast strategy
//! path with a upstream zstd-parity hash table and tight per-block loop.
//!
//! Wired into production: [`crate::encoding::match_generator::MatcherStorage::Simple`]
//! holds `FastKernelMatcher` directly; the driver's Matcher trait
//! methods (`commit_space` / `start_matching` / `skip_matching_with_hint`
//! / `reset` / `prime_with_dictionary` / `trim_after_budget_retire`)
//! all route through this module's inherent API.
//!
//! # Invariants this module guarantees
//!
//! - `prefix_start_index >= INITIAL_PREFIX_START_INDEX = 1` at all
//!   times. `history` holds real input bytes from position 0
//!   onward (no dummy region — M8 dropped the seeded sentinel byte
//!   for upstream zstd byte-range parity). Sentinel-0 protection comes from
//!   the kernel's `match_idx >= prefix_start_index` filter rejecting
//!   the hash table's empty-slot value `0`. After eviction / drain
//!   the buffer is rebased to position 0 and `prefix_start_index`
//!   resets to 1, making the first retained byte (`history[0]`)
//!   unmatchable — small ratio cost, accepted for sentinel safety.
//! - `history.len()` is bounded by `2 × max_window_size` post-append.
//!   See [`FastKernelMatcher::extend_history_with_pending`].
//! - `rep[0..2]` is the functional repcode state: the kernel's
//!   two-deep stack, overwritten from `FastBlockResult.rep` after every
//!   `start_matching`, and what the NEXT block's kernel probes against.
//!   `offset_hist[0..3]` is NOT mutated by matching — the Fast backend
//!   drives repcodes off `rep`, and the wire-offset repcode coding is
//!   done downstream by `encode_raw_sequences_into` against the encode
//!   pipeline's OWN offset history, so a per-match
//!   `encode_offset_with_history` on the matcher would be redundant.
//!   `offset_hist` is therefore only seeded by `prime_offset_history`
//!   (which also sets `rep`) and otherwise stays at its `reset` default.
//!   Do NOT reintroduce per-match `offset_hist` rotation here: it is
//!   pure overhead on this backend (it was removed because the coded
//!   offset it produced was discarded).

use alloc::vec::Vec;

use crate::encoding::Sequence;
use crate::encoding::dict_attach::DictAttach;

use super::fast_kernel::hash_table::{FastHashTable, hash_ptr_raw};
use super::fast_kernel::kernel::compress_block_fast;
use super::fast_kernel::kernel::{DICT_TAG_BITS, DICT_TAG_MASK};

/// Upstream zstd `ZSTD_defaultCParameters[level=1][srcSize > 256 KiB][Fast]`
/// constants. Kept for `MatchGeneratorDriver::new`'s initial-state
/// matcher (which runs BEFORE any `reset` from a resolved
/// `LevelParams`). Production calls thread per-level values
/// (`fast_hash_log`, `fast_mls`, `fast_step_size`) through
/// `LevelParams` instead.
pub(crate) const FAST_LEVEL_1_HASH_LOG: u32 = 14;
pub(crate) const FAST_LEVEL_1_MLS: u32 = 7;

/// Upstream zstd's initial repcode state — `(rep_offset1 = 1, rep_offset2 = 4)`
/// matches `ZSTD_initCCtx`'s reset of `rep` at the start of every
/// frame. Used both as a struct-init constant and as a recovery point
/// in `reset`.
pub(crate) const FAST_INITIAL_REP: [u32; 2] = [1, 4];

/// Initial offset-history seed for the encoder's repcode-coded
/// offsets — matches upstream zstd's `repToConfirm[] = { 1, 4, 8 }` at frame
/// start and mirrors the value the old [`super::MatchGenerator`] used.
pub(crate) const FAST_INITIAL_OFFSET_HIST: [u32; 3] = [1, 4, 8];

/// Drain start offset used by eviction / drain paths. Set to 0:
/// `history` holds real input bytes from position 0 onward,
/// upstream zstd-parity layout, no dummy region. Sentinel-0 protection
/// (hash table's empty-slot value `0` would otherwise be
/// indistinguishable from a real match at position 0) is provided
/// by [`INITIAL_PREFIX_START_INDEX`] = 1 via the kernel's
/// `match_idx >= prefix_start_index` filter.
///
/// Kept as a named constant so the drain math reads consistently
/// against future changes.
pub(crate) const HISTORY_DRAIN_BASE: usize = 0;

/// Upstream zstd's `prefixStartIndex` floor on fresh frames. Set to 1 (not 0)
/// so the kernel's `match_idx >= prefix_start_index` filter rejects
/// stale empty-slot lookups (value 0 in FastHashTable's all-zero
/// initial state). Upstream zstd relies on its `ip0 += (ip0 == prefixStart)`
/// bump to skip position 0 instead — both approaches match the same
/// 0..N-1 byte ranges for the hash table.
///
/// Tradeoff: this rejects legitimate position-0 matches upstream zstd would
/// emit (rare — requires `read32(ip0)` to coincidentally equal
/// `read32(base)`), but cross-block isolation under
/// `skip_matching_with_hint(None)` depends on the sentinel — the
/// `skip_matching_with_none_hint_skips_hash_population` test
/// exercises that contract. Lowering to 0 breaks the test; the
/// position-0 emit rate is too small to be worth that breakage.
const INITIAL_PREFIX_START_INDEX: u32 = 1;

/// Upstream zstd-shape Fast-strategy matcher state.
///
/// State layout mirrors the upstream zstd's `ZSTD_compressBlock_fast_*` entry
/// frame:
///
/// - `history` holds the flat byte buffer that the kernel reads from.
///   Both already-matched prior-block bytes (the prefix) and the
///   current block live in this single contiguous buffer; the kernel's
///   `block_start` parameter separates the two.
/// - `prefix_start_index` is upstream zstd's `prefixStartIndex` — the lowest
///   position any match may reference. Pinned to
///   `INITIAL_PREFIX_START_INDEX` (= 1) at construction and after every
///   drain (drain re-indexes the retained tail; the `1` floor rejects
///   the hash table's all-zero empty-slot value from being read as a
///   valid match at position 0).
/// - `rep` carries the two-deep repcode state across blocks.
/// - `offset_hist` is the encoder-side 3-deep offset history used by
///   the wire encoder's repcode coding (separate from `rep`, which is
///   the matcher's own two-deep stack for the kernel).
/// - `hash_table` is the upstream zstd's flat `u32` hash table, persistent
///   across blocks (cleared only on full `reset`).
/// - `pending` holds the most recently `commit_space`'d block before
///   `start_matching` appends it onto `history` and runs the kernel.
pub(crate) struct FastKernelMatcher {
    /// Concatenated input history: prior-block bytes followed by the
    /// most-recently-committed (still pending-matching) tail.
    history: Vec<u8>,
    /// Upstream zstd `prefixStartIndex` — earliest position any match may
    /// reference.
    prefix_start_index: u32,
    /// Upstream zstd `rep_offset1, rep_offset2`. Threaded into the kernel as
    /// the `rep` array and updated from the kernel's `FastBlockResult`
    /// after every block.
    rep: [u32; 2],
    /// Encoder-side 3-deep offset history for repcode wire coding.
    /// `pub(crate)` so the driver's `prime_with_dictionary` can
    /// inject a seeded history without going through a setter —
    /// matches the legacy `MatchGenerator` field-visibility pattern
    /// the driver was written against.
    pub(crate) offset_hist: [u32; 3],
    /// Flat hash table indexed by upstream zstd `hash_ptr<MLS>`. Persistent
    /// across blocks; only `reset` (or a `(hash_log, mls)` parameter
    /// change) reallocates it.
    hash_table: FastHashTable,
    /// `1 << window_log`. Soft upper bound on `history.len()` — once
    /// the buffer grows past this point the prefix is dropped and
    /// `prefix_start_index` advances. `pub(crate)` for the same
    /// reason as `offset_hist`: the driver's `prime_with_dictionary`
    /// path widens this to accommodate retained dictionary bytes,
    /// matching the legacy MatchGenerator pattern.
    pub(crate) max_window_size: usize,
    /// Decoder-side window size (in `log` bits). Reported to the
    /// frame header via the `Matcher::window_size` trait method.
    window_log: u8,
    /// Upstream zstd heuristic: prefer cmov match-found when
    /// `windowLog < 19` (`ZSTD_compressBlock_fast` line 449). Small-
    /// window encoders have less predictable in-range filtering, so
    /// the branchless variant beats the branchful one on those
    /// levels. Set during `reset` / `with_params` from `window_log`.
    /// Reachable in production via source-size hints (when the
    /// caller passes a small `source_size` to a streaming encoder,
    /// `adjust_params_for_source_size` clamps `window_log` below
    /// the upstream zstd default of 19, flipping `use_cmov` on).
    use_cmov: bool,
    /// Cached per-tier SIMD kernel selection (resolved once via
    /// [`crate::encoding::fastpath::select_kernel`] at construction / reset),
    /// mirroring the Dfast/Row backends. Drives the `#[target_feature]`
    /// umbrella dispatch in the borrowed dual-base dict scan so the
    /// match-length `common_prefix_len_ptr` is the tier's 32-byte AVX2 /
    /// 16-byte SSE4.2 / NEON / wasm-simd128 compare instead of the generic
    /// word-at-a-time `count_forward`.
    kernel: crate::encoding::fastpath::FastpathKernel,
    /// Initial step the kernel uses for the 4-cursor body's skip
    /// schedule. Upstream zstd `stepSize = targetLength + !(targetLength) +
    /// 1` (min 2). Negative-level frames set this to 2..8 to
    /// recreate upstream zstd's acceleration gradient; Level(1) and other
    /// Fast levels keep step_size=2.
    step_size: usize,
    /// Holds a `commit_space`'d block until `start_matching` consumes
    /// it. `None` between frames and immediately after `start_matching`
    /// returns. The driver guarantees at most one outstanding pending
    /// space at a time (single-block-per-cycle protocol).
    pending: Option<Vec<u8>>,
    /// Absolute history position where the MOST RECENTLY appended
    /// block starts — `extend_history_with_pending` updates this so
    /// [`Self::last_committed_space`] can return that block's bytes
    /// AFTER processing (upstream zstd / legacy MatchGenerator parity: the
    /// driver's frame compressor reads `get_last_space` after
    /// `start_matching` to fetch the raw bytes for raw-block
    /// emission). Initialised to 0 — overwritten by every
    /// extend_history_with_pending call.
    last_block_start: usize,
    /// Per-block input buffer recycle slot. After
    /// `extend_history_with_pending` copies bytes from the pending
    /// buffer into `history`, the now-spent `Vec<u8>` allocation is
    /// stashed here (cleared, capacity retained). The driver pulls
    /// it via [`Self::take_recycled_space`] after every
    /// `start_matching` / `skip_matching_with_hint` and returns it
    /// to its `vec_pool` — avoiding a fresh allocation per block on
    /// the hot path.
    recycled_space: Option<Vec<u8>>,
    /// One-shot borrowed match window: `(ptr, len)` into a caller-owned
    /// input buffer that holds the entire frame. When `Some`, all window
    /// *reads* ([`Self::history_bytes`] and the kernel match-slice) view
    /// this range instead of the owned `history` buffer, so the matcher
    /// never copies the input into `history`. `None` selects the owned
    /// streaming path (the default; the borrowed path is opt-in via
    /// [`Self::set_borrowed_window`]).
    ///
    /// A raw pointer (not a borrow) is required because this matcher is
    /// a persistent field of the driver / frame compressor; a borrowed
    /// lifetime would tie those structs to the input buffer. SAFETY
    /// contract (enforced by the caller, see `set_borrowed_window`): the
    /// pointed-to buffer must stay live and unmodified for as long as
    /// the window is set, and the window must be cleared before the
    /// buffer is dropped or the matcher is reused for another frame.
    borrowed: Option<(*const u8, usize)>,
    /// Absolute `[start, end)` range of the block most recently scanned
    /// by [`Self::start_matching_borrowed`], in the borrowed window's
    /// coordinate space. `Some` only while a borrowed window is active
    /// and at least one borrowed block has been scanned; lets
    /// [`Self::last_committed_space`] return that block's bytes
    /// (`borrowed[start..end]`, zero-copy) for the emit path — the
    /// borrowed analogue of `last_block_start` on the owned path.
    last_borrowed_block: Option<(usize, usize)>,
    /// Immutable dictionary hash table (upstream zstd `dictMatchState` Fast path) plus
    /// its CDict cache lifecycle, via the shared [`DictAttach`] level-1
    /// scaffolding. The table is built once during `prime_with_dictionary` over
    /// the dictionary region at the front of `history` (positions
    /// `[1, region_len)`), using the same `(hash_log, mls)` as
    /// [`Self::hash_table`] so a single hash keys both. Attached
    /// (`is_attached()`) activates the dual-probe [`compress_block_fast_dict`]
    /// kernel; invalidated on any history eviction (absolute dict positions
    /// would otherwise go stale) so the no-dict kernel takes over —
    /// correctness-safe, only the dict ratio benefit is lost when the input is
    /// large enough to slide the dictionary out of the window. `region_len()`
    /// is the dict/input boundary (`dict_end`).
    dict: DictAttach<FastHashTable>,
    /// High-water mark of any position storable into [`Self::hash_table`]
    /// since the last table clear / epoch advance: the largest history
    /// length seen by [`Self::extend_history_with_pending`] and the largest
    /// borrowed `block_end` scanned. `reset` feeds it to
    /// [`FastHashTable::advance_epoch`] as the span that makes every
    /// previously-stored entry stale, then rearms it at 0.
    table_pos_high_water: usize,
    /// Set by [`Self::reset`] when it re-borrowed a resident attach-mode
    /// dictionary (kept the dict bytes at the front of history + the cached
    /// dict table in place instead of clearing + re-committing them). Signals
    /// the frame compressor to SKIP `prime_with_dictionary` this frame.
    dict_resident: bool,
    /// Upstream zstd `ms->loadedDictEnd` for the COPY-mode dict path (inputs
    /// over the Fast attach cutoff): the history position one past the last
    /// dictionary byte committed at the front of `history`. The copy path
    /// primes the dict into the live hash table as window prefix; this records
    /// the dict/input boundary so `start_matching` can floor the block prefix
    /// at the dict start (upstream zstd `ZSTD_getLowestPrefixIndex` with
    /// `isDictionary`), keeping the whole dict reachable while it is still
    /// within `maxDist` of the block end. `0` when no copy-mode dict is
    /// resident (attach mode tracks its own boundary via `dict.region_len()`,
    /// and a plain frame has none). Cleared on reset / eviction (the dict has
    /// slid out of the window, so the windowed floor takes over).
    loaded_dict_end: usize,
}

impl Clone for FastKernelMatcher {
    fn clone(&self) -> Self {
        Self {
            history: self.history.clone(),
            prefix_start_index: self.prefix_start_index,
            rep: self.rep,
            offset_hist: self.offset_hist,
            hash_table: self.hash_table.clone(),
            max_window_size: self.max_window_size,
            window_log: self.window_log,
            use_cmov: self.use_cmov,
            kernel: self.kernel,
            step_size: self.step_size,
            pending: self.pending.clone(),
            last_block_start: self.last_block_start,
            recycled_space: self.recycled_space.clone(),
            borrowed: self.borrowed,
            last_borrowed_block: self.last_borrowed_block,
            dict: self.dict.clone(),
            table_pos_high_water: self.table_pos_high_water,
            dict_resident: self.dict_resident,
            loaded_dict_end: self.loaded_dict_end,
        }
    }

    // The per-frame dictionary snapshot restore `clone_from`s this whole
    // matcher; reusing the retained `history` / hash-table / dict-table
    // buffers turns that restore into pure copies (no allocations), which
    // is what the upstream zstd's CDict table-copy regime pays.
    fn clone_from(&mut self, source: &Self) {
        self.history.clone_from(&source.history);
        self.prefix_start_index = source.prefix_start_index;
        self.rep = source.rep;
        self.offset_hist = source.offset_hist;
        self.hash_table.clone_from(&source.hash_table);
        self.max_window_size = source.max_window_size;
        self.window_log = source.window_log;
        self.use_cmov = source.use_cmov;
        self.kernel = source.kernel;
        self.step_size = source.step_size;
        self.pending.clone_from(&source.pending);
        self.last_block_start = source.last_block_start;
        self.recycled_space.clone_from(&source.recycled_space);
        self.borrowed = source.borrowed;
        self.last_borrowed_block = source.last_borrowed_block;
        self.dict.clone_from(&source.dict);
        self.table_pos_high_water = source.table_pos_high_water;
        self.dict_resident = source.dict_resident;
        self.loaded_dict_end = source.loaded_dict_end;
    }
}

impl FastKernelMatcher {
    /// Whether a dictionary table is attached (drives the dual-probe dispatch:
    /// the borrowed scan must consult the dict when set). Mirrors the owned
    /// path's `self.dict.is_attached()` gate.
    pub(crate) fn dict_is_attached(&self) -> bool {
        self.dict.is_attached()
    }

    /// Cheap dict-relevance probe for the raw-fast-path. A high-entropy-LOOKING
    /// block can still compress against an EXTERNAL dictionary: the block's own
    /// content gives no hint (no internal repeat), so only probing the dict
    /// table reveals it. Sample ~32 evenly-spread positions of `block`, hash
    /// each into the attached dict table, and return `true` on the first 4-byte
    /// dict match. The raw-fast-path uses this to keep a dict-matchable block off
    /// the scan-skip (which would ignore the dictionary). Returns `false` when no
    /// dict is attached or `block` is too short to hash — the no-dict path never
    /// calls this, so its incompressibility verdict is unaffected.
    pub(crate) fn block_samples_match_dict(&self, block: &[u8]) -> bool {
        use super::fast_kernel::kernel::dict_lookup;
        const HASH_READ_SIZE: usize = 8;
        if !self.dict.is_attached() || block.len() < HASH_READ_SIZE {
            return false;
        }
        let Some(dict_tab) = self.dict.table() else {
            return false;
        };
        let dict_end = self.dict.region_len();
        if dict_end > self.history.len() || dict_end < HASH_READ_SIZE {
            return false;
        }
        let dict_bytes = &self.history[..dict_end];
        let dhl = dict_tab.hash_log();
        let mls = self.hash_table.mls();
        let last = block.len() - HASH_READ_SIZE;
        // ~32 evenly-spread probes (every position for tiny blocks); a dict that
        // covers any sampled record trips on the first hit.
        let step = (block.len() / 32).max(1);
        let base = block.as_ptr();
        let mut pos = 0;
        while pos <= last {
            // SAFETY: pos <= block.len() - 8 ⇒ `base.add(pos)` has ≥ 8 readable
            // bytes, the context `dict_lookup`/`hash_ptr_raw` require.
            let dpos = unsafe {
                match mls {
                    4 => dict_lookup::<4>(dict_tab, base.add(pos), dhl),
                    5 => dict_lookup::<5>(dict_tab, base.add(pos), dhl),
                    6 => dict_lookup::<6>(dict_tab, base.add(pos), dhl),
                    7 => dict_lookup::<7>(dict_tab, base.add(pos), dhl),
                    _ => dict_lookup::<8>(dict_tab, base.add(pos), dhl),
                }
            } as usize;
            // `dpos >= 1` rejects the empty sentinel; verify a real 4-byte match
            // before declaring the dict relevant, and EXTEND it: only a match
            // long enough to be worth committing signals a dict that helps. A
            // short (4-byte) coincidental hit — common when the dict was trained
            // on data statistically like this block, yet contributes no net
            // compression — must NOT force a scan, or an incompressible block
            // with a self-trained dict pays a full wasted scan instead of the
            // fast raw path. `USEFUL_DICT_MATCH` mirrors the magnitude at which a
            // dict match starts paying for its (far) offset coding.
            const USEFUL_DICT_MATCH: usize = 16;
            if dpos >= 1
                && dpos + 4 <= dict_end
                && block[pos..pos + 4] == dict_bytes[dpos..dpos + 4]
            {
                let cap = (block.len() - pos).min(dict_end - dpos);
                let mut len = 4;
                while len < cap && block[pos + len] == dict_bytes[dpos + len] {
                    len += 1;
                }
                if len >= USEFUL_DICT_MATCH {
                    return true;
                }
            }
            pos += step;
        }
        false
    }

    /// Explicit-parameter constructor used by the wiring commit when
    /// the level resolution produced a non-default `(window_log,
    /// hash_log, mls, step_size)` tuple (typically because a small
    /// source-size hint clamped the window). Tests can also call this
    /// directly.
    /// Construct with the hash table allocated up front at `hash_log`.
    pub(crate) fn with_params(window_log: u8, hash_log: u32, mls: u32, step_size: usize) -> Self {
        Self::with_params_table(
            window_log,
            hash_log,
            mls,
            step_size,
            FastHashTable::new(hash_log, mls),
        )
    }

    /// Construct with the hash table allocation deferred to the first
    /// [`Self::reset`]. Used by `MatchGeneratorDriver::new`, which runs before
    /// any source size is known and would otherwise allocate the table at the
    /// level-default `hash_log` only to realloc it the moment the first frame
    /// clamps the window to a smaller input — a wasted malloc + zero-fill on
    /// every fresh compressor (the `compare_ffi` bench shape). The reset path
    /// allocates the table once at the resolved size before the kernel runs.
    pub(crate) fn with_params_deferred(
        window_log: u8,
        hash_log: u32,
        mls: u32,
        step_size: usize,
    ) -> Self {
        Self::with_params_table(
            window_log,
            hash_log,
            mls,
            step_size,
            FastHashTable::new_deferred(hash_log, mls),
        )
    }

    fn with_params_table(
        window_log: u8,
        // Redundant here (`hash_table` is already built at this shape), kept in
        // the signature to mirror `with_params`'s call shape at both sites.
        _hash_log: u32,
        _mls: u32,
        step_size: usize,
        hash_table: FastHashTable,
    ) -> Self {
        assert!(
            step_size >= 2,
            "FastKernelMatcher requires step_size >= 2 (got {step_size})"
        );
        // Kernel indices are `u32`. `accept_data` lets history grow
        // up to `2 * max_window_size` before draining (upstream zstd parity
        // for the eager-eviction band), so `max_window_size` is
        // capped at 2^30 to keep that band ≤ 2^31 < `u32::MAX` and
        // prevent any `history.len()` from tripping the kernel's
        // `data.len() > u32::MAX` panic. Upstream zstd's
        // `ZSTD_WINDOWLOG_MAX_64` is 30 for the same reason — we
        // mirror it.
        assert!(
            window_log <= 30,
            "FastKernelMatcher requires window_log <= 30 (got {window_log}); \
             2 * (1 << 30) is the eviction-band ceiling that keeps history \
             length below the kernel's u32::MAX input bound"
        );
        // M8: history starts empty (HISTORY_DRAIN_BASE = 0).
        // Sentinel-0 protection comes from prefix_start_index =
        // INITIAL_PREFIX_START_INDEX = 1, which filters hash table
        // lookups returning the empty-slot value 0.
        let history = alloc::vec![0u8; HISTORY_DRAIN_BASE];
        Self {
            last_block_start: HISTORY_DRAIN_BASE,
            recycled_space: None,
            history,
            // Filter `match_idx >= prefix_start_index` rejects the
            // hash table's empty-slot value 0. Eviction in
            // `extend_history_with_pending` rebases the retained
            // tail and resets prefix_start_index back to 1.
            prefix_start_index: INITIAL_PREFIX_START_INDEX,
            rep: FAST_INITIAL_REP,
            offset_hist: FAST_INITIAL_OFFSET_HIST,
            hash_table,
            max_window_size: 1usize << window_log,
            window_log,
            use_cmov: window_log < 19,
            kernel: crate::encoding::fastpath::select_kernel(),
            step_size,
            pending: None,
            borrowed: None,
            last_borrowed_block: None,
            dict: DictAttach::new(),
            table_pos_high_water: 0,
            dict_resident: false,
            loaded_dict_end: 0,
        }
    }

    /// Reset for the next frame.
    ///
    /// Drops all history, clears the repcode and offset stacks, and
    /// either clears the existing hash table (if `(hash_log, mls)` are
    /// unchanged) or reallocates it. The window_log update redirects
    /// the soft-eviction bound and the decoder-side reported window.
    ///
    /// `dict_attach_epoch`: the upcoming frame re-primes the SAME
    /// dictionary in attach mode (separate cached dict table, dual-probe
    /// kernel). When the cached dict table is still primed, the main
    /// table is then invalidated via an epoch advance (upstream zstd
    /// `ZSTD_continueCCtx` cadence — stale entries filtered by the bias,
    /// no full-table memset); every other shape keeps the historical
    /// `clear()` so the raw-slice no-dict kernels always see a bias-0
    /// table.
    pub(crate) fn reset(
        &mut self,
        window_log: u8,
        hash_log: u32,
        mls: u32,
        step_size: usize,
        dict_attach_epoch: bool,
        // The caller (driver) has a primed-snapshot whose key matches this
        // exact reset shape and WILL `clone_from` it over this matcher
        // right after the reset (the copy-mode dictionary restore). The
        // table contents and epoch bias are about to be replaced
        // wholesale, so the full-table memset here would be pure waste.
        table_overwritten_by_restore: bool,
    ) {
        assert!(
            step_size >= 2,
            "FastKernelMatcher requires step_size >= 2 (got {step_size})"
        );
        // Same window_log cap as `with_params` — see there for why
        // the ceiling is 30, not 31.
        assert!(
            window_log <= 30,
            "FastKernelMatcher requires window_log <= 30 (got {window_log})"
        );
        // Re-borrow detection: set to the resident dict region when the
        // epoch-reuse branch below keeps the dict bytes in place (see there).
        let mut reborrow_region: Option<usize> = None;
        if !self.hash_table.is_allocated() {
            // Deferred table from `with_params`: this first reset is where the
            // source-size-clamped (hash_log, mls) is finally known, so allocate
            // once at the resolved size. Subsequent frames take the
            // same-shape `clear()` / epoch branches below.
            self.hash_table = FastHashTable::new(hash_log, mls);
            self.dict.invalidate();
        } else if table_overwritten_by_restore
            && self.hash_table.hash_log() == hash_log
            && self.hash_table.mls() == mls
        {
            // Leave the table untouched: the snapshot restore copies the
            // primed contents (and bias) over it immediately after.
        } else if self.hash_table.hash_log() != hash_log || self.hash_table.mls() != mls {
            // Parameters changed — rebuild the table at the new size.
            // Cannot reuse the old allocation because the hash table
            // dimensions are baked in at construction. A reshape also
            // invalidates the cached dict table: its absolute positions
            // index a table whose shape no longer matches.
            self.hash_table = FastHashTable::new(hash_log, mls);
            self.dict.invalidate();
        } else if dict_attach_epoch && self.dict.is_primed() {
            // Dict-attach frame over the same primed dictionary: advance
            // the epoch bias past every position the previous frames could
            // have stored instead of memsetting the whole table (upstream zstd
            // `ZSTD_continueCCtx`). The dual-probe dict kernel reads the
            // main table only through `FastHashTable::get`, which maps
            // pre-advance entries to the empty sentinel. The cached dict
            // table is untouched (its own instance, bias 0): the dictionary
            // lands at the same absolute history positions every frame, so
            // its hashes stay valid and the per-frame re-hash is skipped
            // (CDict-equivalent).
            let span = u32::try_from(self.table_pos_high_water).unwrap_or(u32::MAX);
            self.hash_table.advance_epoch(span);
            // Re-borrow: the dictionary bytes from the previous frame are still
            // resident at the front of history (`[0, region)`), so keep them in
            // place instead of clearing + re-committing — the per-frame dict
            // memmove was the dominant Fast-dict cost (~39% on a profiled small
            // frame). The cached dict table already covers them; the frame
            // compressor skips `prime_with_dictionary`. Gated on the dict still
            // being fully resident (no eviction drained it off the front).
            let region = self.dict.region_len();
            if region > 0 && self.history.len() >= region {
                reborrow_region = Some(region);
            }
        } else {
            // Same shape — keep the allocation, zero the entries via
            // `memset` (ZSTD_window_clear cadence). A primed dict table
            // is retained (see the epoch branch above for why that is
            // sound).
            self.hash_table.clear();
        }
        self.table_pos_high_water = 0;
        // No copy-mode dict is resident across a reset: attach mode re-borrows
        // its separate dict table (handled above), and the copy path re-primes
        // (and re-records `loaded_dict_end`) during this frame's dict prime.
        self.loaded_dict_end = 0;
        if let Some(region) = reborrow_region {
            // Keep `[0, region)` (the resident dict); drop the previous input.
            self.history.truncate(region);
            self.dict_resident = true;
        } else {
            // M8: history starts empty (HISTORY_DRAIN_BASE = 0).
            self.history.clear();
            self.history.resize(HISTORY_DRAIN_BASE, 0);
            self.dict_resident = false;
        }
        // Sentinel-0 protection via prefix_start_index >= 1 filter
        // — see `with_params` for the full rationale. Unchanged for re-borrow:
        // the dict sits at `[0, region)`, position 0 is the sentinel, so the
        // dict's `[1, region)` stay reachable to the kernel and the decoder.
        self.prefix_start_index = INITIAL_PREFIX_START_INDEX;
        self.rep = FAST_INITIAL_REP;
        self.offset_hist = FAST_INITIAL_OFFSET_HIST;
        self.window_log = window_log;
        self.use_cmov = window_log < 19;
        self.step_size = step_size;
        self.max_window_size = 1usize << window_log;
        if let Some(region) = reborrow_region {
            // Bump the eviction window by the dict size (exactly as
            // `prime_with_dictionary`, clamped to MAX_PRIMED_WINDOW_SIZE) so the
            // resident dict + the next input both stay in the window. Base is
            // `1 << window_log` (<= 2^30 < MAX_PRIMED_WINDOW_SIZE), so headroom
            // cannot underflow and the sum cannot overflow — no `saturating_*`.
            let headroom = crate::encoding::match_table::storage::MAX_PRIMED_WINDOW_SIZE
                - self.max_window_size;
            self.max_window_size += region.min(headroom);
        }
        self.pending = None;
        // Input starts after the resident dict on a re-borrow frame; otherwise
        // at the drain base. (The first `extend_history` re-derives this, but
        // keep it consistent for any pre-append reads.)
        self.last_block_start = reborrow_region.unwrap_or(HISTORY_DRAIN_BASE);
        self.recycled_space = None;
        // Drop any borrowed window: the next frame's input buffer is a
        // different allocation, so a stale (ptr, len) would dangle.
        self.borrowed = None;
        self.last_borrowed_block = None;
    }

    /// Heap bytes this matcher owns: the history buffer, the hash table, the
    /// recycle/pending slots, and any attached dictionary hash table.
    pub(crate) fn heap_size(&self) -> usize {
        self.history.capacity()
            + self.hash_table.heap_size()
            + self.pending.as_ref().map_or(0, |v| v.capacity())
            + self.recycled_space.as_ref().map_or(0, |v| v.capacity())
            + self.dict.table().map_or(0, |t| t.heap_size())
    }

    /// Flat byte view of the match window the kernel scans against.
    ///
    /// Single read accessor for the window storage so the storage
    /// representation (owned buffer or borrowed one-shot view) is
    /// resolved in one place. The owned `window_low` length math and the
    /// last-committed-block peek (`last_committed_space`) call this.
    /// Tail-literal emission is NOT a caller — it happens inside
    /// `run_fast_kernel_block` against the `history` slice handed to it.
    /// The kernel match-slice in `start_matching` does NOT call this — it
    /// inlines the identical owned/borrowed selection so the immutable
    /// window borrow stays a disjoint field projection alongside the
    /// `&mut self.hash_table` borrow (a `&self` accessor call would
    /// borrow all of `self` and collide). Owned-only mutation paths
    /// (append, drain, rehash) keep accessing the backing buffer
    /// directly.
    #[inline(always)]
    fn history_bytes(&self) -> &[u8] {
        match self.borrowed {
            // SAFETY: the (ptr, len) pair was registered via
            // `set_borrowed_window`, whose contract guarantees the
            // buffer stays live and unmodified until the window is
            // cleared (and the window is cleared on `reset` / before the
            // buffer drops). `len` is the exact length passed in, so the
            // reconstructed slice never exceeds the original allocation.
            Some((ptr, len)) => unsafe { core::slice::from_raw_parts(ptr, len) },
            None => &self.history,
        }
    }

    /// Point the match window at a caller-owned input buffer instead of
    /// the owned `history` mirror, so the matcher reads the input in
    /// place without copying it block-by-block.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `buffer` stays live and is not
    /// moved or mutated for as long as the window is set, and that the
    /// window is cleared (via [`Self::clear_borrowed_window`] or
    /// [`Self::reset`]) before `buffer` is dropped or the matcher is
    /// reused for a different frame. The owned-buffer mutation paths
    /// (`accept_data`, `extend_history_with_pending`, drain, prime) must
    /// not run while a borrowed window is active.
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        // A staged owned `pending` block would make `last_committed_space`
        // return the pending buffer (it checks `pending` first) instead of
        // the borrowed range, breaking the borrowed/owned equivalence the
        // emit path relies on. The borrowed one-shot caller resets before
        // registering (so `pending` is None), but this is an unsafe mode
        // switch — make the precondition explicit and loud.
        assert!(
            self.pending.is_none(),
            "set_borrowed_window requires no staged owned block; reset before switching to a borrowed window",
        );
        // A live borrowed window at entry means a prior frame's window was never
        // cleared by a `reset()` — and since `clear_borrowed_window` no longer
        // memsets the table (it relies on the next reset to do so), the table
        // would still hold the prior scan's virtual `dict_end + input_off`
        // positions. The borrowed-dict kernel would then read those as current
        // offsets (`main_idx - dict_end` underflowing to a huge pointer). Catch
        // that missing-reset regression here rather than letting it silently
        // corrupt memory; the production caller always resets first.
        debug_assert!(
            self.borrowed.is_none(),
            "set_borrowed_window called without a preceding reset()/clear_borrowed_window",
        );
        self.borrowed = Some((buffer.as_ptr(), buffer.len()));
        self.last_borrowed_block = None;
        // Stale hash-table entries from a prior window are invalidated by the
        // `reset()` the caller (`run_borrowed_block_loop` via
        // `compress_oneshot_borrowed`) ALWAYS runs immediately before this
        // call — `reset` either memsets the table (no-dict / copy frames) or
        // epoch-advances the bias past every prior position (dict-attach
        // frames, upstream zstd `ZSTD_continueCCtx`). Re-clearing here would memset the
        // whole table a SECOND time per frame and, on the dict-attach path,
        // throw away the epoch advance the reset just performed (measured: the
        // redundant clear was ~12% of the borrowed-dict encode). The
        // mode-switch precondition (no stale owned `pending`) is asserted
        // above. Re-flooring `prefix_start_index` is a single store (not a
        // memset), so keep it for self-containment.
        self.prefix_start_index = INITIAL_PREFIX_START_INDEX;
    }

    /// Clear a borrowed window set by [`Self::set_borrowed_window`],
    /// returning the matcher to the owned `history` path.
    pub(crate) fn clear_borrowed_window(&mut self) {
        self.borrowed = None;
        self.last_borrowed_block = None;
        // Detach the window pointer only — do NOT memset the table. The hash
        // table still holds absolute positions into the now-detached borrowed
        // buffer, but every frame begins with `reset()` (which memsets the
        // table for no-dict / copy frames, or epoch-advances the bias for
        // dict-attach frames) BEFORE any subsequent scan reads it, so an
        // owned- or borrowed-path scan never observes the stale entries. The
        // prior unconditional `hash_table.clear()` here was a second
        // full-table memset per borrowed frame (this runs on the
        // `ClearBorrowedOnDrop` guard at every frame exit) on top of the next
        // frame's reset — measured at ~12% of the borrowed-dict encode — with
        // no correctness role the reset does not already cover. `start_matching`
        // (the owned path) additionally asserts `borrowed.is_none()`, which the
        // `self.borrowed = None` above satisfies. Re-floor the sentinel-0
        // baseline (a single store, not a memset).
        self.prefix_start_index = INITIAL_PREFIX_START_INDEX;
    }

    /// Read-only view of the most recently committed block — upstream zstd /
    /// legacy MatchGenerator's `window.last().data` equivalent.
    ///
    /// Three states:
    /// - Pre-`accept_data`: empty slice — `history` is empty and
    ///   `last_block_start` is 0, so `history[last_block_start..]`
    ///   degenerates to a zero-length slice.
    /// - Between `accept_data` and processing: the pending buffer.
    /// - Post-processing: `history` slice of the just-processed
    ///   block — frame compressor's raw-block emission reads this.
    pub(crate) fn last_committed_space(&self) -> &[u8] {
        if let Some(slice) = self.pending.as_deref() {
            return slice;
        }
        // Borrowed one-shot path: the just-scanned block lives at
        // `[start, end)` of the borrowed window, which `history_bytes()`
        // views in place — return it zero-copy for the emit path.
        if let Some((start, end)) = self.last_borrowed_block {
            return &self.history_bytes()[start..end];
        }
        &self.history_bytes()[self.last_block_start..]
    }

    /// Accept a freshly-committed block from the driver.
    ///
    /// Upstream zstd's `ZSTD_window_update`: the new bytes are stashed for
    /// the next [`Self::start_matching`] / [`Self::skip_matching`]
    /// call but NOT yet appended to `history` — that delay lets the
    /// driver-side `get_last_space` peek at the still-pending buffer
    /// without committing it to the matcher's hot path.
    ///
    /// History budget is enforced EAGERLY in this function (not lazily
    /// inside [`Self::extend_history_with_pending`]) so the driver's
    /// `commit_space` can observe the eviction delta via a pre/post
    /// `history.len()` comparison. That delta feeds
    /// `retire_dictionary_budget`, which shrinks `max_window_size`
    /// back to the frame's contracted window after dictionary priming
    /// inflated it. Without commit-time visibility the dict-budget
    /// retire never runs and the matcher can emit offsets exceeding
    /// the frame header's reported window size (format-correctness
    /// risk).
    pub(crate) fn accept_data(&mut self, space: Vec<u8>) {
        assert!(
            self.pending.is_none(),
            "FastKernelMatcher: accept_data called with a still-pending buffer; \
             the driver must run start_matching / skip_matching between commits",
        );

        // Eager window eviction: drop oldest history bytes NOW if
        // accepting this block would push the total past upstream zstd's
        // `2 × max_window_size` soft cap. This fires at commit time
        // (not at append time inside `extend_history_with_pending`)
        // so the driver's `commit_space` can observe the byte delta
        // via a `pre/post history.len()` comparison — that delta
        // feeds `retire_dictionary_budget` which shrinks
        // `max_window_size` back to the frame's contracted window
        // after dictionary priming inflated it. Without commit-time
        // visibility the dict-budget retire never runs and the
        // matcher can emit offsets exceeding the frame header's
        // reported window size (format-correctness risk).
        // Eviction operates on REAL data length. Post-M8 there is
        // no dummy prefix at the head of `history`, so `real_len` is
        // just `history.len()` minus the `HISTORY_DRAIN_BASE`
        // sentinel-slot offset — not a placeholder block subtraction.
        let real_len = self.history.len().saturating_sub(HISTORY_DRAIN_BASE);
        // Plain `*`: `max_window_size` starts at `1 << window_log` (window_log
        // <= 30 from `with_params`/`reset`) but dictionary priming widens it,
        // always capped via `.min(MAX_PRIMED_WINDOW_SIZE)` where
        // `MAX_PRIMED_WINDOW_SIZE = (u32::MAX - MAX_BLOCK_SIZE) / 2`. So
        // `cap = max_window_size * 2 <= u32::MAX - MAX_BLOCK_SIZE < u32::MAX`
        // by construction — fits usize on 32-bit targets. (The `saturating_sub`
        // above stays: it is a real floor at the sentinel slot.)
        let cap = self.max_window_size * 2;
        // Hard precondition: caller must split blocks into pieces no
        // larger than `2 × max_window_size`. Without this, the
        // eviction math below can't keep post-append history under
        // the advertised cap (retain_real saturates to 0 but the
        // full block still appends, violating the invariant).
        assert!(
            space.len() <= cap,
            "FastKernelMatcher requires block_size <= 2 × max_window_size \
             (block={}, cap={})",
            space.len(),
            cap,
        );
        // Subtraction, not `real_len + space.len() > cap`: the assert above
        // guarantees `space.len() <= cap`, so `cap - space.len()` cannot
        // underflow. With a primed `cap` approaching `u32::MAX - MAX_BLOCK_SIZE`,
        // both `real_len` and `space.len()` can each be large enough that the
        // addition would overflow usize on 32-bit targets before the comparison.
        if real_len > cap - space.len() {
            // Compute how many real bytes to KEEP, then drop the
            // delta. Pre-fix code naively kept `max_window_size`
            // regardless of incoming block size — for a committed
            // block larger than `max_window_size` that left
            // `real_len + space.len() > 2 × max_window_size`,
            // violating the docstring invariant.
            //
            // Post-fix: retained = (cap - space.len()) clamped to
            // [0, max_window_size]. When the incoming block alone
            // exceeds cap, retained = 0 (no historical context kept,
            // but the cap is still as close as we can get without
            // truncating the caller's block).
            let retain_real = cap.saturating_sub(space.len()).min(self.max_window_size);
            let drop_n = real_len.saturating_sub(retain_real);
            if drop_n > 0 {
                self.drain_real_prefix(drop_n);
            }
        }

        self.pending = Some(space);
    }

    /// Drop the OLDEST `drop_n` real bytes from history and rebase
    /// the retained tail to start at position 0 (M8 layout: no
    /// dummy region). Used by both the eager commit-time eviction
    /// in [`Self::accept_data`] and the dictionary-budget retire
    /// loop's [`Self::trim_to_window`].
    ///
    /// Side effects:
    ///
    /// 1. Drain `history[0..drop_n)`.
    /// 2. Reset `prefix_start_index` to `INITIAL_PREFIX_START_INDEX = 1`
    ///    — drain re-indexes the retained tail; the sentinel-0
    ///    filter restores via this fixed baseline.
    /// 3. Clear the hash table — entries hold pre-drain absolute
    ///    positions that no longer reference live bytes.
    /// 4. `saturating_sub` `last_block_start` by `drop_n`.
    /// 5. Rehash retained tail starting at the sentinel-0 floor
    ///    ([`INITIAL_PREFIX_START_INDEX`] = 1) so block N+1 can find
    ///    matches against the kept bytes (without this they'd be
    ///    "dead history" — visible in the Vec but unlookupable).
    ///    Starting from index 1 instead of 0 avoids hashing a position
    ///    that the kernel's `match_idx >= prefix_start_index` filter
    ///    would reject anyway.
    fn drain_real_prefix(&mut self, drop_n: usize) {
        let drain_end = HISTORY_DRAIN_BASE + drop_n;
        self.history.drain(HISTORY_DRAIN_BASE..drain_end);
        self.prefix_start_index = INITIAL_PREFIX_START_INDEX;
        // Any drain rebases the retained tail to position 0, invalidating
        // the immutable dict table's absolute positions (and likely
        // sliding the dictionary out of the live window entirely). Drop it
        // so subsequent blocks fall back to the no-dict kernel — the only
        // cost is the dict ratio benefit on inputs large enough to evict
        // the dictionary, which is exactly when the dict is no longer
        // reachable anyway.
        self.dict.invalidate();
        // Same reasoning for the COPY-mode dict boundary: the rebased tail
        // moves the dict positions, and the drain only fires when history
        // outgrew the window, i.e. the dict has slid out. Clear it so the
        // prefix floor reverts to the windowed value (upstream zstd
        // `ZSTD_window_enforceMaxDist` zeroing `loadedDictEnd`).
        self.loaded_dict_end = 0;
        self.hash_table.clear();
        self.last_block_start = self.last_block_start.saturating_sub(drop_n);
        // Skip position 0 — `prefix_start_index = 1` means the kernel
        // rejects any match resolving to index 0, so populating that
        // slot would just pollute the table with an unreachable entry.
        self.prime_hash_table_for_range(INITIAL_PREFIX_START_INDEX as usize);
    }

    /// Internal: drain `self.pending` into `self.history`, applying
    /// the window-budget eviction first. Returns the absolute position
    /// at which the newly-appended block starts (upstream zstd's
    /// `currentBlockStart` — what the kernel receives as
    /// `block_start`).
    ///
    /// Eviction rule mirrors upstream zstd's `ZSTD_window_correctOverflow`:
    /// when total retained bytes would exceed `2 × max_window_size`,
    /// drop the oldest bytes back down to a `max_window_size` tail
    /// and clear the hash table. The clear is forced because absolute
    /// positions stored in the table would otherwise reference
    /// evicted bytes; upstream zstd avoids the clear via a base-pointer trick
    /// (`base += correction`) that the flat-`Vec<u8>` history can't
    /// reuse, but pays for it with a one-time eviction every
    /// `max_window_size` worth of input — amortised constant.
    fn extend_history_with_pending(&mut self) -> usize {
        let mut space = self
            .pending
            .take()
            .expect("extend_history_with_pending without a pending buffer");

        // Eviction was already applied during `accept_data` (eager
        // pre-commit drain so the driver's `commit_space` accounting
        // sees the byte delta). At this point the matcher's
        // invariant `history.len() + space.len() <= 2 *
        // max_window_size` already holds — just append.
        let block_start = self.history.len();
        self.history.extend_from_slice(&space);
        // Track the largest position any kernel scan over this history
        // could store into the hash table (consumed by `reset`'s epoch
        // advance).
        self.table_pos_high_water = self.table_pos_high_water.max(self.history.len());
        // Record where this newly-appended block starts so
        // `last_committed_space` can return its bytes AFTER the
        // kernel call consumes pending.
        self.last_block_start = block_start;
        // Stash the now-spent space buffer (cleared, capacity
        // retained) for the driver to pull via
        // `take_recycled_space()` and return to its vec_pool. Avoids
        // a fresh per-block allocation on the hot path. If a previous
        // recycled buffer was never taken (e.g. driver crashed mid-
        // cycle) we drop it here — only ONE buffer is recycled per
        // cycle, matching the single-pending-block protocol.
        space.clear();
        self.recycled_space = Some(space);
        block_start
    }

    /// Reclaim the most recently spent input buffer (the `Vec<u8>`
    /// passed in via `accept_data` after its bytes were copied into
    /// `history`). The buffer is empty but retains its capacity —
    /// the driver can resize it back to `slice_size` and push onto
    /// `vec_pool` to amortise per-block allocation cost.
    ///
    /// Returns `None` if no block has been processed since the last
    /// `take_recycled_space` (or since construction / reset).
    pub(crate) fn take_recycled_space(&mut self) -> Option<Vec<u8>> {
        self.recycled_space.take()
    }

    /// Process the pending block with the upstream zstd-shape kernel,
    /// streaming `Sequence::Triple` emissions to `handle_sequence`
    /// and emitting a terminal `Sequence::Literals` if any tail
    /// remained after the last match.
    ///
    /// The MLS const-generic is dispatched at runtime against the
    /// hash table's `mls` (4..=8). Each arm monomorphises a separate
    /// `compress_block_fast<MLS>` body so the inner-loop hash formula
    /// and shift widths compile to constants per supported mls. The
    /// `_ =>` arm is unreachable because `validate_params` in
    /// [`FastHashTable::new`] rejects mls outside 4..=8 at
    /// construction.
    pub(crate) fn start_matching(&mut self, handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        // Owned scan path. A borrowed one-shot window (set via
        // `set_borrowed_window`) is mutually exclusive with this path:
        // `extend_history_with_pending` appends into `self.history` and
        // `block_start` indexes that owned buffer, so matching against a
        // borrowed window here would index it with an owned-history
        // offset, and the kernel would read `self.history` at hash-table
        // indices that were populated against the (possibly larger)
        // borrowed window — out-of-bounds / UB. Always-on (not
        // debug_assert): the guard must hold in release / `cargo test
        // --release` too, since the failure mode is memory-unsafe, not
        // merely wrong output. The borrowed window is scanned by
        // `start_matching_borrowed` instead.
        assert!(
            self.borrowed.is_none(),
            "start_matching is the owned path; clear the borrowed window first (use start_matching_borrowed)",
        );
        let block_start = self.extend_history_with_pending();
        // Compute the EFFECTIVE prefix floor for this scan against
        // the ADVERTISED frame window (`1 << window_log`), NOT
        // `max_window_size` — the driver may temporarily inflate
        // `max_window_size` by the retained dictionary budget
        // during `prime_with_dictionary`. The frame header still
        // reports `1 << window_log`, so any emitted offset older
        // than `history.len() - (1 << window_log)` would exceed
        // the decoder's reserved window and produce a format-
        // invalid sequence. Upstream zstd's
        // `ZSTD_getLowestPrefixIndex(ms, endIndex, windowLog)`
        // uses `windowLog` for the same reason.
        let advertised_window = 1usize << self.window_log;
        // Upstream zstd's `windowLow` analogue: the absolute floor of in-window
        // positions. Equals 0 at block 0 (no prior input retained) and
        // advances as the window slides. Drives the prologue's
        // `max_rep = ip0 - window_low` computation AND the backward-
        // extension `match_pos > window_low` bound — both paths that
        // upstream zstd expresses against `prefixStart` directly (NOT against
        // a sentinel-1 floor).
        let block_end = self.history_bytes().len();
        let windowed_low = block_end.saturating_sub(advertised_window) as u32;
        // Upstream zstd `ZSTD_getLowestPrefixIndex` with `isDictionary`: when a
        // COPY-mode dict is resident at the front of history AND still within
        // `maxDist` of the block end (`block_end <= advertised_window +
        // loadedDictEnd`), the prefix floor is the DICT START, not the
        // window-clamped floor — so the whole dict stays reachable for the
        // block. A window-clamped floor computed from the history END would
        // reject every dict position once the input fills the window, ignoring
        // the dictionary entirely. Once the block end passes `maxDist +
        // loadedDictEnd` the dict has slid out of the window and the windowed
        // floor takes over (eviction also clears `loaded_dict_end`). Offsets
        // reaching the dict (bounded by `advertised_window + loaded_dict_end`)
        // stay decodable because the decoder loads the same dictionary.
        // `loaded_dict_end` is set only for COPY-mode dicts; an ATTACHED
        // (in-place) dict tracks its boundary via `dict.region_len()` instead. Use
        // whichever mode is active so an owned attach-mode scan keeps the WHOLE
        // dict reachable once the input fills the advertised window — mirrors the
        // borrowed dict floor. Without it the `dpos >= window_low` gate rejects
        // every attached-dict slot past `advertised_window`, silently dropping the
        // dictionary for over-window owned frames.
        let effective_dict_end = if self.loaded_dict_end != 0 {
            self.loaded_dict_end
        } else if self.dict.is_attached() {
            self.dict.region_len()
        } else {
            0
        };
        let dict_in_window =
            effective_dict_end != 0 && block_end <= advertised_window + effective_dict_end;
        let window_low = if dict_in_window { 0 } else { windowed_low };
        // Sentinel-aware prefix for the hash-table filter — match_idx
        // == 0 (an uninitialized FastHashTable slot) must be rejected
        // by `match_found`, so we floor at `INITIAL_PREFIX_START_INDEX
        // = 1` when window_low is 0 (block 0 / pre-eviction blocks).
        // For later blocks (window_low >= 1) the two values coincide.
        //
        // This SPLIT preserves upstream zstd parity: using
        // `prefix_start_index = 1` for the prologue's max_rep gave
        // `max_rep = 0` at ip0=1, zeroing upstream zstd's default
        // `rep_offset1 = 1` and disabling rep-at-ip2 for the entire
        // first block. With `window_low = 0` we match upstream zstd exactly
        // (`max_rep = 1`, rep_offset1 survives).
        let prefix_start_index = window_low.max(self.prefix_start_index);
        let rep_in = self.rep;
        let mls = self.hash_table.mls();
        let step_size = self.step_size;
        let use_cmov = self.use_cmov;

        // Upstream zstd `dictMatchState` Fast path, active whenever a dictionary is
        // primed (and not yet evicted — `drain_real_prefix` drops the table).
        // The 4-cursor search and its `prefix_start_index` / `window_low`
        // bounds are shared with the no-dict kernel (main matches are input
        // positions, all `>= dict_end >= prefix_start_index`); the dict probe
        // additionally requires `pos >= window_low` and `pos < dict_end` so
        // emitted offsets (`ip0 - pos`) stay within the advertised window,
        // including the pre-drain 1x..2x-window band where `window_low > 0`.
        let use_dict = self.dict.is_attached();
        let history: &[u8] = &self.history;
        let rep_out = if use_dict {
            use super::fast_kernel::kernel::PrefixBounds;
            let dict_end = self.dict.region_len() as u32;
            let bounds = PrefixBounds {
                prefix_start_index,
                window_low,
            };
            let main_table = &mut self.hash_table;
            let dict_table = self
                .dict
                .table()
                .expect("use_dict implies dict_table is Some");
            run_fast_kernel_block_dict(
                history,
                block_start,
                bounds,
                dict_end,
                main_table,
                dict_table,
                rep_in,
                step_size,
                mls,
                use_cmov,
                handle_sequence,
            )
        } else {
            // Owned scan reads `self.history` directly (the guard above
            // guarantees no borrowed window is active). Borrowing only the
            // `history` field keeps it a disjoint projection alongside the
            // `&mut self.hash_table` borrow handed to `run_fast_kernel_block`
            // (a `&self` accessor would borrow all of `self` and collide).
            run_fast_kernel_block(
                history,
                block_start,
                prefix_start_index,
                window_low,
                &mut self.hash_table,
                rep_in,
                step_size,
                mls,
                use_cmov,
                handle_sequence,
            )
        };
        // Persist the kernel's rep state for the next block.
        self.rep = rep_out;
    }

    /// Borrowed-window equivalent of [`Self::start_matching`]: scan the
    /// block spanning `[block_start, block_end)` of the registered
    /// borrowed window in place, without appending to or evicting from
    /// the owned `history`. Requires [`Self::set_borrowed_window`] to
    /// have registered the buffer; the caller supplies absolute block
    /// bounds (the one-shot frame path tracks them as it walks the
    /// input). `block_start <= block_end` and `block_end` <= the
    /// registered buffer length.
    ///
    /// Produces a byte-identical sequence stream to the owned path: a
    /// one-shot frame's blocks lie back-to-back in the input buffer
    /// exactly as the owned `history` accumulates them. Over-window inputs
    /// are supported: matches are bounded by `window_low = block_end -
    /// advertised_window` (the same bound the owned evicting path applies),
    /// and the per-position `put` during the scan keeps in-window hash
    /// slots current — so an out-of-window stale slot is rejected exactly
    /// where the owned rehash would have left the slot empty, giving
    /// identical match decisions with or without eviction.
    pub(crate) fn start_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        let (ptr, total_len) = self
            .borrowed
            .expect("start_matching_borrowed requires a registered borrowed window");
        // Always-on (not debug_assert): the bounds feed the unsafe
        // `from_raw_parts` below, so they must be validated even in
        // release / `cargo test --release` where debug_assert is
        // compiled out — otherwise an out-of-range block_end would build
        // a slice past the borrowed allocation (immediate UB).
        assert!(
            block_start <= block_end && block_end <= total_len,
            "borrowed block bounds out of range: start={block_start} end={block_end} total={total_len}",
        );
        // Borrowed scans store raw (bias-0) positions up to `block_end`
        // through the table's hot-state slice; record them for `reset`'s
        // epoch-advance span. (A borrowed window never coexists with a
        // primed dict, so the table bias is 0 here — see `hot_state` —
        // but the high-water must still cover these positions in case a
        // dictionary is attached on a later frame.)
        self.table_pos_high_water = self.table_pos_high_water.max(block_end);
        // Same window math as the owned path, but against the absolute
        // block end in the borrowed buffer rather than the accumulated
        // history length.
        let advertised_window = 1usize << self.window_log;
        let window_low = block_end.saturating_sub(advertised_window) as u32;
        let prefix_start_index = window_low.max(self.prefix_start_index);
        let rep_in = self.rep;
        let mls = self.hash_table.mls();
        // SAFETY: `block_end <= total_len` (the registered buffer length)
        // by the caller contract + the always-on `assert!` above, so the slice stays
        // within the borrowed allocation; `set_borrowed_window`'s
        // liveness contract guarantees the buffer is still live. `ptr` is
        // copied out of the `Copy` `borrowed` field, so `history` is not
        // tied to `&self` and stays disjoint from the `&mut` field
        // borrows passed to the kernel runner.
        let history: &[u8] = unsafe { core::slice::from_raw_parts(ptr, block_end) };
        let rep_out = run_fast_kernel_block(
            history,
            block_start,
            prefix_start_index,
            window_low,
            &mut self.hash_table,
            rep_in,
            self.step_size,
            mls,
            self.use_cmov,
            handle_sequence,
        );
        self.rep = rep_out;
        // Record the scanned range so `last_committed_space` can return
        // this block's bytes (`borrowed[start..end]`) for the emit path.
        self.last_borrowed_block = Some((block_start, block_end));
    }

    /// Borrowed in-place scan WITH an attached dictionary (the dict-attach
    /// counterpart of [`Self::start_matching_borrowed`]). The dictionary
    /// content sits in `history[0..dict_end]`; the frame input is read in
    /// place from the borrowed window, never copied after it. Positions live
    /// in the logical `[dict][input]` window: a dict byte `d` has absolute
    /// position `d`; an input byte `i` has absolute position `dict_end + i`
    /// (the dict precedes the input). A match offset is `cur_abs - cand_abs`.
    ///
    /// Live (input) candidates read from the borrowed window and count flat;
    /// dictionary candidates read from `history[0..dict_end]` and extend across
    /// the dict→input boundary via the 2-segment counter — the 2-segment count
    /// C performs with `dictBase` / `ZSTD_count_2segments`, which the flat
    /// single-base path cannot. Dispatches the active `(mls, use_cmov)` pair to
    /// the monomorphised dual-base kernel
    /// [`compress_block_fast_dict_borrowed`], which carries the owned dict
    /// kernel's full machinery (repcode probe, step-ramp two-position
    /// lookahead, dense fills, backward extension, immediate repcode-2 loop) —
    /// the prior scalar greedy scan had none of these and was +68% slower.
    /// Validated by roundtrip + cross-validation + the FFI ratio gate.
    pub(crate) fn start_matching_borrowed_dict(
        &mut self,
        block_start: usize,
        block_end: usize,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        use super::fast_kernel::kernel::{PrefixBounds, compress_block_fast_dict_borrowed};
        let (ptr, total_len) = self
            .borrowed
            .expect("start_matching_borrowed_dict requires a registered borrowed window");
        assert!(
            block_start <= block_end && block_end <= total_len,
            "borrowed block bounds out of range: start={block_start} end={block_end} total={total_len}",
        );
        self.last_borrowed_block = Some((block_start, block_end));

        let dict_end = self.dict.region_len();
        // Single checked virtual length of the [dict][input] window. The dual-base
        // kernel stores VIRTUAL positions `dict_end + input_off` (up to
        // `virtual_len`) into the main table, so the next frame's epoch advance
        // must span past it — NOT just `block_end` — or stale entries in
        // `[block_end, virtual_len)` would survive the bias advance and alias as
        // bogus low positions. Checked so the `as u32` casts below cannot
        // truncate (the kernel asserts the same bound).
        let virtual_len = dict_end
            .checked_add(block_end)
            .filter(|&v| v <= u32::MAX as usize)
            .expect("dict_end + block_end exceeds the u32 FastKernel position space");
        self.table_pos_high_water = self.table_pos_high_water.max(virtual_len);
        let advertised_window = 1usize << self.window_log;
        let mls = self.hash_table.mls();
        let use_cmov = self.use_cmov;
        let step_size = self.step_size;
        let rep_in = self.rep;
        let kernel = self.kernel;

        // Window bounds in VIRTUAL `[dict][input]` coords, so the kernel's gates
        // match the owned flat dict kernel: `window_low` is the absolute floor;
        // `prefix_start_index` the sentinel-aware floor (`>= 1`) for the
        // hash-slot filter. `virtual_len` was checked above.
        let windowed_low = virtual_len.saturating_sub(advertised_window);
        // Upstream zstd `ZSTD_getLowestPrefixIndex` with `isDictionary`: while the
        // dict (committed at virtual `[0, dict_end)`) is still within
        // `maxDist + loadedDictEnd` of the block end, the floor is the DICT START
        // (0), NOT the window-clamped `virtual_len - window`. `dict_end` is the
        // loaded-dict-end here. Without this, a block over the Fast attach cutoff
        // (now every Fast dict frame) computes `window_low = virtual_len - window`
        // which lands at or above `dict_end`, so the kernel's `dpos >= window_low`
        // gate rejects EVERY dict position and the dictionary is ignored for the
        // whole block (the same prefix-floor bug the owned copy path fixed). Once
        // the block grows past `window + dict_end` the dict has slid out of the
        // window and the windowed floor takes over; offsets reaching the dict stay
        // bounded by `window + dict_end`, so they remain decodable.
        let dict_in_window = dict_end != 0 && virtual_len <= advertised_window + dict_end;
        let window_low = if dict_in_window { 0 } else { windowed_low };
        let prefix_start_index = window_low.max(self.prefix_start_index as usize) as u32;
        let bounds = PrefixBounds {
            prefix_start_index,
            window_low: window_low as u32,
        };

        // SAFETY: `block_end <= total_len` (asserted) and the borrowed window is
        // live for the call; `ptr` is `Copy`d out so `inp` holds no `&self`
        // borrow. The dict slice reborrows `history`'s base through a raw ptr so
        // it coexists with the `&mut self.hash_table` writes inside the kernel —
        // disjoint memory (the dict content is committed at `history[0..
        // dict_end]` and is not mutated during the scan). The dict hash table
        // likewise reborrows through a raw ptr (immutable, built once in
        // `prime_dict_table_*`).
        let inp: &[u8] = unsafe { core::slice::from_raw_parts(ptr, block_end) };
        debug_assert!(
            dict_end <= self.history.len(),
            "dict region_len ({dict_end}) exceeds history.len() ({}) — \
             dictionary bytes must be committed before borrowed-dict scan",
            self.history.len(),
        );
        let dict: &[u8] = unsafe { core::slice::from_raw_parts(self.history.as_ptr(), dict_end) };
        let dict_tab_ptr: *const FastHashTable = self
            .dict
            .table()
            .expect("start_matching_borrowed_dict requires an attached dict table");
        // SAFETY: reborrow the immutable dict table (built once in
        // `prime_dict_table_*`, not mutated during the scan) detached from
        // `&self` so it coexists with the `&mut self.hash_table` borrow below.
        let dict_tab: &FastHashTable = unsafe { &*dict_tab_ptr };
        let main_table = &mut self.hash_table;

        macro_rules! run {
            ($mls:literal, $cmov:literal) => {
                compress_block_fast_dict_borrowed::<$mls, $cmov>(
                    inp,
                    dict,
                    block_start,
                    block_end,
                    main_table,
                    dict_tab,
                    bounds,
                    rep_in,
                    step_size,
                    &mut handle_sequence,
                    kernel,
                )
            };
        }
        let result = match (mls, use_cmov) {
            (4, false) => run!(4, false),
            (4, true) => run!(4, true),
            (5, false) => run!(5, false),
            (5, true) => run!(5, true),
            (6, false) => run!(6, false),
            (6, true) => run!(6, true),
            (7, false) => run!(7, false),
            (7, true) => run!(7, true),
            (8, false) => run!(8, false),
            (8, true) => run!(8, true),
            _ => {
                unreachable!("FastHashTable construction rejects mls outside 4..=8 — got mls={mls}")
            }
        };
        self.rep = result.rep;

        if result.tail_literals_len > 0 {
            let tail_start = block_end - result.tail_literals_len;
            handle_sequence(Sequence::Literals {
                literals: &inp[tail_start..block_end],
            });
        }
    }

    /// Make `[block_start, block_end)` the block `last_committed_space`
    /// reports BEFORE the scan runs. The emit pipeline reads
    /// `get_last_space().len()` in `collect_block_parts` *before* calling
    /// `start_matching`, so without this the first borrowed block would
    /// report the whole borrowed window (`last_borrowed_block` still
    /// `None` → `history_bytes()[0..]`), over-reserving the literal buffer
    /// and undercutting the peak-alloc win. Called by the driver when it
    /// stages the range; `start_matching_borrowed` / `skip_matching_borrowed`
    /// re-record the same value idempotently.
    pub(crate) fn stage_borrowed_block(&mut self, block_start: usize, block_end: usize) {
        let (_ptr, total_len) = self
            .borrowed
            .expect("stage_borrowed_block requires a registered borrowed window");
        // Always-on (not debug_assert): the staged range is later sliced by
        // `last_committed_space` as `history_bytes()[start..end]`, so an
        // out-of-range or inverted range would panic deep in the emit path
        // instead of at the staging call site. Validate here to match
        // `start_matching_borrowed` / `skip_matching_borrowed`.
        assert!(
            block_start <= block_end && block_end <= total_len,
            "staged borrowed block bounds out of range: start={block_start} end={block_end} total={total_len}",
        );
        self.last_borrowed_block = Some((block_start, block_end));
    }

    /// Upstream zstd's `skipMatching` equivalent: append the pending block to
    /// history without running the kernel.
    ///
    /// The block's bytes are NOT hashed into the table, so block N+1's
    /// matcher cannot find matches against the skipped region. This
    /// trades compression on the skipped bytes for CPU — the driver
    /// calls this when an upstream incompressibility hint marks the
    /// block as not worth scanning. Upstream zstd's
    /// `ZSTD_compressBlock_targetCBlockSize_body` makes the same
    /// trade.
    ///
    /// The `incompressible_hint` parameter accepts the upstream zstd's
    /// `Matcher::skip_matching_with_hint` semantics:
    ///
    /// - `Some(true)` or `None` — incompressible / no opinion: append
    ///   only, no hash entries (cheapest path).
    /// - `Some(false)` — explicitly "this block IS compressible, but
    ///   the driver is skipping it for dictionary-priming reasons":
    ///   the block's bytes need to be matchable in future blocks, so
    ///   pre-populate the hash table for every position in the newly
    ///   appended range. This matches the
    ///   `skip_matching_for_dictionary_priming` flow on the driver.
    pub(crate) fn skip_matching_with_hint(&mut self, incompressible_hint: Option<bool>) {
        let block_start = self.extend_history_with_pending();
        // Rep state survives unchanged: skip should look idempotent
        // to the next block's matcher (no fake match implies no rep
        // promotion). offset_hist likewise unchanged.

        // Dictionary-priming path: explicit `Some(false)` means the
        // upstream knows the block is compressible material that the
        // future matcher should be able to reach. Populate hash
        // entries for every position in the appended range that has
        // at least `HASH_READ_SIZE` bytes of forward context — under
        // that threshold the kernel itself can't read the position
        // either, so a hash entry there would be unreachable.
        //
        // Iteration runs while `pos + HASH_READ_SIZE <= history.len()`;
        // a saturating subtract gives the loop bound without ever
        // wrapping for short blocks (history shorter than HASH_READ_SIZE
        // is a legal post-prime state when the dictionary itself is
        // very small).
        if incompressible_hint == Some(false) {
            self.prime_hash_table_for_range(block_start);
            // Copy-mode dict prime: the dict now occupies `[0, history.len())`
            // at the front of history (this is the only caller of the
            // `Some(false)` hint — see the doc above). Record the dict/input
            // boundary so `start_matching` floors the block prefix at the dict
            // start while the dict stays within the window (upstream zstd
            // `ms->loadedDictEnd`). A multi-slice dict advances this to the
            // running end on each slice; the final slice leaves the full
            // dict size.
            self.loaded_dict_end = self.history.len();
        }
    }

    /// Borrowed-window equivalent of [`Self::skip_matching_with_hint`]:
    /// the block `[block_start, block_end)` is emitted as RLE / raw
    /// without running the kernel, but its bytes are already resident in
    /// the borrowed window (no `history` append needed). Records the
    /// range for [`Self::last_committed_space`]; on the dict-priming hint
    /// (`Some(false)`) populates the hash table for the range so a later
    /// block can match into this skipped-but-compressible region, exactly
    /// as the owned path does.
    pub(crate) fn skip_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        incompressible_hint: Option<bool>,
    ) {
        let (_ptr, total_len) = self
            .borrowed
            .expect("skip_matching_borrowed requires a registered borrowed window");
        // Always-on (not debug_assert): the recorded range is later sliced
        // by `last_committed_space` for the emit path, and the priming
        // path below does unsafe pointer reads up to `block_end - 8`. An
        // out-of-range `block_end` would be immediate UB even in release,
        // so validate it before storing the range or touching the table —
        // mirrors `start_matching_borrowed`.
        assert!(
            block_start <= block_end && block_end <= total_len,
            "borrowed block bounds out of range: start={block_start} end={block_end} total={total_len}",
        );
        self.last_borrowed_block = Some((block_start, block_end));
        if incompressible_hint == Some(false) {
            self.prime_hash_table_for_range_borrowed(block_start, block_end);
        }
    }

    /// Borrowed-window analogue of [`Self::prime_hash_table_for_range`].
    /// Hashes every position in `[range_start, block_end - HASH_READ_SIZE]`
    /// reading from the borrowed input buffer. Bounded by `block_end`
    /// (not the full buffer) so only the just-committed block's positions
    /// are indexed — future blocks aren't matchable yet, mirroring the
    /// owned path where `history` ends at `block_end`.
    fn prime_hash_table_for_range_borrowed(&mut self, range_start: usize, block_end: usize) {
        const HASH_READ_SIZE: usize = 8;
        if block_end < HASH_READ_SIZE {
            return;
        }
        let last_hashable = block_end - HASH_READ_SIZE;
        // Backfill the (HASH_READ_SIZE - 1) seam positions below
        // `range_start` (see `prime_hash_table_for_range`): the prior
        // block's trailing positions read across the block boundary, so
        // skipping them drops seam-spanning matches between blocks.
        let backfill_start = range_start.saturating_sub(HASH_READ_SIZE - 1);
        if backfill_start > last_hashable {
            return;
        }
        let (base, _len) = self
            .borrowed
            .expect("prime_hash_table_for_range_borrowed requires a registered borrowed window");
        // Store primed input positions in VIRTUAL `[dict][input]` coords so they
        // match what the dual-base dict kernel reads from the main table; 0 when
        // no dict is attached.
        let base_offset = self.dict.region_len();
        debug_assert!(
            base_offset
                .checked_add(block_end)
                .is_some_and(|v| v <= u32::MAX as usize),
            "virtual position overflow: dict.region_len()={base_offset} + block_end={block_end} exceeds u32",
        );
        match self.hash_table.mls() {
            4 => self.prime_hash_table_impl::<4>(base, backfill_start, last_hashable, base_offset),
            5 => self.prime_hash_table_impl::<5>(base, backfill_start, last_hashable, base_offset),
            6 => self.prime_hash_table_impl::<6>(base, backfill_start, last_hashable, base_offset),
            7 => self.prime_hash_table_impl::<7>(base, backfill_start, last_hashable, base_offset),
            8 => self.prime_hash_table_impl::<8>(base, backfill_start, last_hashable, base_offset),
            _ => unreachable!("FastHashTable construction rejects mls outside 4..=8"),
        }
    }

    /// Seed both the wire encoder's offset history AND the kernel's
    /// repcode state from a primed dictionary load. Upstream zstd's
    /// `ZSTD_dictAndWindowLoad` restores `rep[0..2]` to the
    /// dictionary's stored `repToConfirm[0..2]`; the wire encoder
    /// uses the same triple as its 3-deep offset history. Setting
    /// only one side leaves the kernel making repcode decisions
    /// against stale FAST_INITIAL_REP while the wire encoder uses
    /// the primed values — divergent wire encoding.
    ///
    /// This setter writes both fields atomically. `rep[0..2]`
    /// mirrors `offset_hist[0..2]`; `offset_hist[2]` (the
    /// rep3 slot) lives only on the wire encoder side since the
    /// kernel's `rep` is two-deep.
    pub(crate) fn prime_offset_history(&mut self, offset_hist: [u32; 3]) {
        self.offset_hist = offset_hist;
        self.rep = [offset_hist[0], offset_hist[1]];
    }

    /// Read-only view of history's real-data length for the driver's
    /// eviction accounting (`commit_space` →
    /// `retire_dictionary_budget` flow). The driver compares pre/post
    /// values to derive a byte-delta; under M8 history holds only
    /// real bytes from position 0 onward (HISTORY_DRAIN_BASE is 0),
    /// so this is just the history length — the `saturating_sub` is
    /// kept symmetric with `trim_to_window` below in case the drain
    /// base ever moves off 0.
    pub(crate) fn history_len_for_eviction_accounting(&self) -> usize {
        self.history.len().saturating_sub(HISTORY_DRAIN_BASE)
    }

    /// Drop history bytes past `max_window_size` via
    /// [`Self::drain_real_prefix`] (resets `prefix_start_index` to
    /// `INITIAL_PREFIX_START_INDEX` = 1 — the sentinel-0 floor — and
    /// clears + rehashes the table). Returns evicted byte count;
    /// idempotent when `real_len <= max_window_size`.
    pub(crate) fn trim_to_window(&mut self) -> usize {
        let real_len = self.history.len().saturating_sub(HISTORY_DRAIN_BASE);
        if real_len <= self.max_window_size {
            return 0;
        }
        let drop_n = real_len - self.max_window_size;
        // Front-drain bookkeeping shared with `accept_data`'s
        // eager-eviction branch — see `drain_real_prefix` for the
        // full invariant list. Keeping the two sites in lockstep
        // (rather than inlined-and-duplicated) prevents the next
        // drain-related fix from landing in only one of them.
        self.drain_real_prefix(drop_n);
        drop_n
    }

    /// Pre-populate the hash table with entries for every position in
    /// `history[range_start..end_of_history]` that has at least
    /// `HASH_READ_SIZE` bytes of forward context. Used by the
    /// dictionary-priming skip path (`skip_matching` with
    /// `incompressible_hint = Some(false)`).
    ///
    /// `mls` dispatch is hoisted OUTSIDE the per-position loop so
    /// the inner body is monomorphised per matcher instance (no
    /// branch / mispredict in the hot path).
    fn prime_hash_table_for_range(&mut self, range_start: usize) {
        let history_len = self.history.len();
        // HASH_READ_SIZE = 8 is the kernel's load-width invariant
        // (upstream zstd `MEM_readST` cadence). Hashing a position with fewer
        // forward bytes would compute a hash over uninitialised /
        // out-of-range memory.
        const HASH_READ_SIZE: usize = 8;
        if history_len < HASH_READ_SIZE {
            return;
        }
        let last_hashable = history_len - HASH_READ_SIZE;
        // Backfill the (HASH_READ_SIZE - 1) positions below `range_start`:
        // their 8-byte hash read straddles the seam into this slice, so
        // without re-hashing them a multi-slice history drops every
        // seam-spanning match. `saturating_sub` floors at HISTORY_DRAIN_BASE
        // (0); re-hashing already-indexed tail positions is idempotent.
        let backfill_start = range_start.saturating_sub(HASH_READ_SIZE - 1);
        if backfill_start > last_hashable {
            return;
        }

        let base = self.history.as_ptr();
        // Owned path: history offsets are already the flat `[dict][input]`
        // coordinate (input is appended after the dict in `history`), so no
        // virtual rebase is needed.
        match self.hash_table.mls() {
            4 => self.prime_hash_table_impl::<4>(base, backfill_start, last_hashable, 0),
            5 => self.prime_hash_table_impl::<5>(base, backfill_start, last_hashable, 0),
            6 => self.prime_hash_table_impl::<6>(base, backfill_start, last_hashable, 0),
            7 => self.prime_hash_table_impl::<7>(base, backfill_start, last_hashable, 0),
            8 => self.prime_hash_table_impl::<8>(base, backfill_start, last_hashable, 0),
            _ => unreachable!("FastHashTable construction rejects mls outside 4..=8"),
        }
    }

    /// Monomorphised per-MLS loop body shared by the owned
    /// [`Self::prime_hash_table_for_range`] and the borrowed
    /// [`Self::skip_matching_borrowed`] dict-priming paths. `base` is the
    /// window base pointer (owned `history` or the borrowed input
    /// buffer); positions are absolute window offsets in both.
    /// `base_offset` is added to every stored position so the main table holds
    /// the SAME coordinate space the active kernel reads. The owned path passes
    /// 0 (history offsets are already the flat `[dict][input]` coordinate). The
    /// borrowed dict path passes `dict_end` so a primed input position `pos` is
    /// stored as the VIRTUAL `dict_end + pos` the dual-base kernel expects —
    /// without this, a primed raw offset in `[1, dict_end)` would underflow the
    /// kernel's `main_idx - dict_end`. No-dict borrowed frames pass 0
    /// (`region_len() == 0`), so their raw offsets are unchanged.
    fn prime_hash_table_impl<const MLS: u32>(
        &mut self,
        base: *const u8,
        range_start: usize,
        last_hashable: usize,
        base_offset: usize,
    ) {
        for pos in range_start..=last_hashable {
            // SAFETY: pos < history_len (by loop bound), and the load
            // width HASH_READ_SIZE is the kernel's contractually
            // required minimum, so `base.add(pos)` covers
            // HASH_READ_SIZE readable bytes by `last_hashable`'s
            // definition. The MLS const-generic is bound at the
            // caller's match arm — `hash_ptr<MLS>` and `put` are
            // constant-folded per MLS.
            let ptr = unsafe { base.add(pos) };
            let hash = unsafe { self.hash_table.hash_ptr::<MLS>(ptr) };
            unsafe { self.hash_table.put(hash, (base_offset + pos) as u32) };
        }
    }

    /// Dictionary-priming entry for the upstream zstd `dictMatchState` Fast path.
    /// Appends the pending dict slice to `history` and indexes its positions
    /// into the SEPARATE immutable [`Self::dict_table`] — NOT the main hash
    /// table. Keeping dict positions out of the main table is what lets the
    /// dual-probe kernel prefer recent-input matches (main) over dictionary
    /// matches (dict fallback), matching the upstream zstd's `prefixStart`/dict split.
    /// Replaces the [`Self::skip_matching_with_hint`]`(Some(false))` call the
    /// driver used to make for Fast-backend priming.
    pub(crate) fn skip_matching_for_dict_prime(&mut self) {
        let block_start = self.extend_history_with_pending();
        self.prime_dict_table_for_range(block_start);
    }

    /// Mark the dict table as fully built (CDict-equivalent). Called by the
    /// driver after the final dictionary chunk has been primed, so the next
    /// frame's [`Self::prime_dict_table_for_range`] skips the re-hash while
    /// the dict bytes are still re-committed to history. Only marks when a
    /// table actually exists — a sub-8-byte dict builds no table and must
    /// re-run the (cheap, no-op) prime path each frame.
    pub(crate) fn mark_dict_primed(&mut self) {
        self.dict.mark_primed();
    }

    /// Whether the last [`Self::reset`] re-borrowed a resident dictionary (kept
    /// the dict bytes + cached dict table in place). The driver reports this up
    /// so the frame compressor skips `prime_with_dictionary` for the frame.
    pub(crate) fn dict_resident(&self) -> bool {
        self.dict_resident
    }

    /// Drop the cached dict table and its primed flag. Called by the driver
    /// when the next frame carries no dictionary, so the kernel never probes
    /// a stale dict region whose bytes are no longer re-committed.
    pub(crate) fn invalidate_dict_cache(&mut self) {
        self.dict.invalidate();
    }

    /// Build (or extend) [`Self::dict_table`] over `history[range_start..]`,
    /// the freshly-appended dictionary bytes. Lazily allocates the dict table
    /// at the same `(hash_log, mls)` as the main table so one hash keys both.
    fn prime_dict_table_for_range(&mut self, range_start: usize) {
        const HASH_READ_SIZE: usize = 8;
        let history_len = self.history.len();
        // Record the dict/input boundary regardless of whether any position
        // is hashable (a sub-8-byte dict still bounds the input floor).
        self.dict.set_region_len(history_len);
        if self.dict.is_primed() {
            // CDict-equivalent fast path: `dict_table` was built over the
            // identical dictionary bytes on a prior frame, and those bytes
            // sit at the same absolute history positions now (the dict is
            // re-committed before this call). Skip the re-hash entirely.
            return;
        }
        if history_len < HASH_READ_SIZE {
            return;
        }
        let last_hashable = history_len - HASH_READ_SIZE;
        // The dict fill resumes at the carried-forward fill origin
        // (`next_to_update`), NOT this slice's `range_start`: a position whose
        // wide hash read straddles a slice seam is unreachable by the prior
        // slice (its `last_hashable` stopped `HASH_READ_SIZE - 1` bytes short)
        // and sits below this slice's `range_start`, yet must still be hashed
        // here. Restarting at `range_start` instead would drop those seam
        // positions and fragment a long cross-seam dict match. `next_to_update`
        // starts at the content start (`HISTORY_DRAIN_BASE == 0`, the default).
        debug_assert!(
            self.dict.next_to_update() <= range_start,
            "dict fill origin {} ran ahead of slice start {range_start}",
            self.dict.next_to_update(),
        );
        let fill_start = self.dict.next_to_update();
        if fill_start > last_hashable {
            return;
        }
        let hash_log = self.hash_table.hash_log();
        let mls = self.hash_table.mls();
        self.dict
            .table_mut_or_init(|| FastHashTable::new(hash_log, mls));
        let base = self.history.as_ptr();
        let next = match self.hash_table.mls() {
            4 => self.prime_dict_table_impl::<4>(base, fill_start, last_hashable),
            5 => self.prime_dict_table_impl::<5>(base, fill_start, last_hashable),
            6 => self.prime_dict_table_impl::<6>(base, fill_start, last_hashable),
            7 => self.prime_dict_table_impl::<7>(base, fill_start, last_hashable),
            8 => self.prime_dict_table_impl::<8>(base, fill_start, last_hashable),
            _ => unreachable!("FastHashTable construction rejects mls outside 4..=8"),
        };
        self.dict.set_next_to_update(next);
    }

    /// Monomorphised per-MLS dict-table fill. `base` is a raw pointer into
    /// `self.history` (no borrow held), so mutating `self.dict_table` in the
    /// loop is sound — the loop never touches `history`, which stays put.
    /// Returns the carried-forward fill origin (one past the last position
    /// hashed), stored as [`DictAttach::next_to_update`] so the next
    /// `accept_data` slice resumes here without dropping the seam.
    fn prime_dict_table_impl<const MLS: u32>(
        &mut self,
        base: *const u8,
        range_start: usize,
        last_hashable: usize,
    ) -> usize {
        let dict_table = self
            .dict
            .table_mut()
            .expect("prime_dict_table_for_range creates the table before this call");
        // Every-position last-wins fill. For repetitive dictionary content one
        // hash maps to many candidate positions, and the kernel emits the match
        // offset as `ip - position`. Walking every position in increasing order
        // with last-wins keeps the HIGHEST (nearest-to-the-input) occurrence per
        // hash, so the emitted offset is the smallest reachable — and small
        // offsets cost far fewer bits in the offset-code FSE stream than far
        // ones. A strided fill (stride-overwrite plus in-between fill-if-empty)
        // instead keeps whichever occurrence the stride alignment happened to
        // land on; for variable-length records that is frequently a far one,
        // inflating the offset and erasing the dictionary's ratio benefit (the
        // dict frame ends up larger than the no-dict frame).
        //
        // Each slot stores `(position << DICT_TAG_BITS) | tag` (upstream zstd
        // `ZSTD_SHORT_CACHE`): the slot index is the plain `dict_hash_log` hash,
        // the tag is the low `DICT_TAG_BITS` of the wider hash. `0` is the empty
        // sentinel — the dict occupies positions `1..dict_end` and the kernel
        // rejects `dict_idx < 1`. The tag lets the kernel reject a colliding
        // position without the candidate load + `MEM_read32`; tagging does NOT
        // change the slot, so the last-wins nearest-occurrence guarantee holds.
        let dict_hash_log = dict_table.hash_log();
        // The tagged hash is `hash_ptr_raw(.., dict_hash_log + DICT_TAG_BITS)`,
        // well-defined only while that width fits 32 bits. Guard it here (always-on)
        // before the unsafe fill, matching the kernel-side guards — a `hash_log > 24`
        // dict table would otherwise hash past 32 bits and write a bogus slot.
        assert!(
            dict_hash_log + DICT_TAG_BITS <= 32,
            "tagged Fast dict fill requires dict hash_log <= {} (got {dict_hash_log})",
            32 - DICT_TAG_BITS,
        );
        // Every slot packs the position into the `32 - DICT_TAG_BITS`-bit field
        // (16 MiB with an 8-bit tag). `last_hashable` bounds every position the
        // loop writes, so one check here covers the whole fill; a larger dict
        // region would truncate the high position bits and alias offsets. The
        // driver routes dicts past `MAX_FAST_ATTACH_DICT_REGION` to copy mode, so
        // in production this is a backstop the attach gate already upholds.
        assert!(
            last_hashable >> (32 - DICT_TAG_BITS) == 0,
            "dict region too large for the tagged fast-table position field \
             (last_hashable={last_hashable}, max={})",
            (1usize << (32 - DICT_TAG_BITS)) - 1,
        );
        for pos in range_start..=last_hashable {
            // SAFETY: pos <= last_hashable = history_len - 8, so `base.add(pos)`
            // covers >= 8 readable bytes; MLS matches the table's mls.
            let hat = unsafe { hash_ptr_raw::<MLS>(base.add(pos), dict_hash_log + DICT_TAG_BITS) };
            unsafe {
                dict_table.put(
                    hat >> DICT_TAG_BITS,
                    ((pos as u32) << DICT_TAG_BITS) | (hat & DICT_TAG_MASK),
                )
            };
        }
        // Every position up to `last_hashable` is hashed; the next `accept_data`
        // slice resumes one past it, picking up the seam positions whose 8-byte
        // hash read straddled this slice's tail (unreachable until more bytes
        // landed).
        last_hashable + 1
    }
}

/// Run the Fast kernel over `history[..]` for the block starting at
/// `block_start`, streaming emissions straight to `handle_sequence` and
/// emitting any terminal tail literals. Returns the kernel's two-deep
/// `rep` state for the caller to persist.
///
/// The Fast backend does NOT mutate the matcher's `offset_hist`: repcode
/// probes run off `rep`, and the wire-offset repcode coding is done
/// downstream by `encode_raw_sequences_into` against the encode
/// pipeline's own offset history. So emissions are forwarded verbatim,
/// with no per-match offset-history rotation here.
///
/// A free function (not a method) so the owned and borrowed
/// `start_matching` paths share one copy of the `(mls, use_cmov)`
/// dispatch: passing the disjoint `&mut` borrow of `hash_table` as an
/// explicit parameter sidesteps the `&self`-vs-`&mut self.hash_table`
/// conflict a `&self` accessor would create, the same reason the window
/// slice is selected by the caller and handed in.
#[allow(clippy::too_many_arguments)]
fn run_fast_kernel_block(
    history: &[u8],
    block_start: usize,
    prefix_start_index: u32,
    window_low: u32,
    hash_table: &mut FastHashTable,
    rep_in: [u32; 2],
    step_size: usize,
    mls: u32,
    use_cmov: bool,
    mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
) -> [u32; 2] {
    use super::fast_kernel::kernel::PrefixBounds;

    let bounds = PrefixBounds {
        prefix_start_index,
        window_low,
    };
    // Dispatch on (mls, use_cmov) — each pair monomorphises the kernel
    // hot loop independently. `_` is unreachable: `FastHashTable::new`
    // rejects mls outside 4..=8 at construction.
    let result = match (mls, use_cmov) {
        (4, false) => compress_block_fast::<4, false>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (4, true) => compress_block_fast::<4, true>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (5, false) => compress_block_fast::<5, false>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (5, true) => compress_block_fast::<5, true>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (6, false) => compress_block_fast::<6, false>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (6, true) => compress_block_fast::<6, true>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (7, false) => compress_block_fast::<7, false>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (7, true) => compress_block_fast::<7, true>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (8, false) => compress_block_fast::<8, false>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        (8, true) => compress_block_fast::<8, true>(
            history,
            block_start,
            bounds,
            hash_table,
            rep_in,
            step_size,
            &mut handle_sequence,
        ),
        _ => unreachable!(
            "FastHashTable construction rejects mls outside 4..=8 — \
             got mls={mls} which means the table was bypassed",
        ),
    };

    // Emit terminal literals if the kernel left a tail. `wrap_emit`'s
    // borrow of `handle_sequence` has ended (no use past the match), so
    // calling it directly here is allowed.
    if result.tail_literals_len > 0 {
        let tail_start = history.len() - result.tail_literals_len;
        handle_sequence(Sequence::Literals {
            literals: &history[tail_start..],
        });
    }

    result.rep
}

/// Dictionary-primed counterpart of [`run_fast_kernel_block`]: dispatches the
/// `(mls, use_cmov)` pair to [`compress_block_fast_dict`], threading the
/// immutable `dict_table` alongside the main table. Emits any terminal tail
/// literals exactly as the no-dict helper does.
#[allow(clippy::too_many_arguments)]
fn run_fast_kernel_block_dict(
    history: &[u8],
    block_start: usize,
    bounds: super::fast_kernel::kernel::PrefixBounds,
    dict_end: u32,
    main_table: &mut FastHashTable,
    dict_table: &FastHashTable,
    rep_in: [u32; 2],
    step_size: usize,
    mls: u32,
    use_cmov: bool,
    mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
) -> [u32; 2] {
    use super::fast_kernel::kernel::compress_block_fast_dict;

    macro_rules! run {
        ($mls:literal, $cmov:literal) => {
            compress_block_fast_dict::<$mls, $cmov>(
                history,
                block_start,
                bounds,
                main_table,
                dict_table,
                dict_end,
                rep_in,
                step_size,
                &mut handle_sequence,
            )
        };
    }
    let result = match (mls, use_cmov) {
        (4, false) => run!(4, false),
        (4, true) => run!(4, true),
        (5, false) => run!(5, false),
        (5, true) => run!(5, true),
        (6, false) => run!(6, false),
        (6, true) => run!(6, true),
        (7, false) => run!(7, false),
        (7, true) => run!(7, true),
        (8, false) => run!(8, false),
        (8, true) => run!(8, true),
        _ => unreachable!("FastHashTable construction rejects mls outside 4..=8 — got mls={mls}",),
    };

    if result.tail_literals_len > 0 {
        let tail_start = history.len() - result.tail_literals_len;
        handle_sequence(Sequence::Literals {
            literals: &history[tail_start..],
        });
    }

    result.rep
}
