#![allow(
    clippy::cast_lossless,
    clippy::manual_is_variant_and,
    clippy::semicolon_if_nothing_returned,
    clippy::wildcard_imports,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Matching algorithm used find repeated parts in the original data
//!
//! The Zstd format relies on finden repeated sequences of data and compressing these sequences as instructions to the decoder.
//! A sequence basically tells the decoder "Go back X bytes and copy Y bytes to the end of your decode buffer".
//!
//! The task here is to efficiently find matches in the already encoded data for the current suffix of the not yet encoded data.

use alloc::vec::Vec;
// SIMD/CRC intrinsics now live in `crate::encoding::fastpath::*` where they
// sit under per-CPU `#[target_feature]` umbrellas; no architecture-specific
// intrinsic imports remain in this file.
use super::CompressionLevel;
use super::Matcher;
use super::Sequence;
use super::cost_model::HC_FORMAT_MINMATCH;

use super::dfast::DfastMatchGenerator;

// FAST_HASH_FILL_STEP test-only re-export was tied to the legacy
// SuffixStore MatchGenerator's interleaved hash-fill stride. The
// upstream zstd-shape Fast kernel walks ip0 with kSearchStrength step-skip
// acceleration instead, so the constant has no consumer in the
// remaining live test set today.

use super::match_table::helpers::MIN_MATCH_LEN;

use super::row::RowMatchGenerator;
use super::simple::fast_matcher::{FAST_LEVEL_1_HASH_LOG, FAST_LEVEL_1_MLS, FastKernelMatcher};

pub(crate) const DFAST_MIN_MATCH_LEN: usize = 5;
// Bytes the dfast short hash reads (upstream zstd `mls = 5`). Seeding / lookahead
// guards use it so a position is only short-hashed once its full 5-byte key
// is in range.
pub(crate) const DFAST_SHORT_HASH_LOOKAHEAD: usize = 5;
pub(crate) const ROW_MIN_MATCH_LEN: usize = 5;
// Upstream zstd `clevels.h:31` at level 3 large-input bucket sets
// `hashLog = 17` (the long-hash table) and `chainLog = 16` (the
// short-hash table — upstream zstd names this `chainTable` even though for
// dfast it's used as a plain single-slot hash). Each table holds one
// `U32` per slot; the upstream zstd overwrites on collision and recovers
// compression quality via the inline `_search_next_long` retry
// (after a short-hash hit, probes `hashLong[hl1]` at `ip + 1` and
// keeps the longer match).
//
// We mirror that storage layout: single `u32` per bucket (no
// `[u32; N]` array), `long_hash` sized `1 << DFAST_HASH_BITS` and
// `short_hash` one bit smaller via `DFAST_SHORT_HASH_BITS_DELTA`.
// Two-table footprint at Level 3: `2^17 × 4 + 2^16 × 4 = 768 KiB`,
// exact upstream parity. The `_search_next_long` retry lives in
// `DfastMatchGenerator::hash_candidate` (called via
// `best_match`). Earlier revisions kept a
// 4-slot bucket per hash position; that paid 4× the upstream zstd memory
// without measurable ratio gain once the retry was in place.
//
// `dfast_hash_bits_for_window` still clamps the runtime long-hash
// value to `[MIN_WINDOW_LOG, DFAST_HASH_BITS]`, so this const is the
// upper bound rather than a fixed default.
pub(crate) const DFAST_HASH_BITS: usize = 17;
/// Difference between `long_hash_bits` and `short_hash_bits` —
/// upstream zstd `hashLog - chainLog` is 1 at every dfast level (`clevels.h`
/// level 2: 16-15=1; level 3: 17-16=1). The short hash is one bit
/// smaller than the long hash so the per-bucket footprint matches
/// upstream zstd sizing exactly.
pub(crate) const DFAST_SHORT_HASH_BITS_DELTA: usize = 1;
/// Sentinel value for an empty slot in the dfast hash tables. Real
/// positions are stored as `(abs_pos - position_base + 1) as u32`, so
/// `0` is reserved as the "empty" marker and a true relative offset
/// of `0` never appears in the table. Mirrors the LDM table's
/// `LdmEntry.offset == 0` convention (see `encoding/ldm/table.rs`)
/// so both rebasing structures share
/// one sentinel scheme.
pub(crate) const DFAST_EMPTY_SLOT: u32 = 0;

/// Guard band reserved above the high-water mark before triggering a
/// rebase on the Dfast hash tables. When the next insert would push a
/// relative offset above `u32::MAX - DFAST_REBASE_GUARD_BAND`, the
/// table calls `reduce(GUARD_BAND)` to shift every slot down and
/// advance `position_base` so future inserts stay inside the `u32`
/// window. Same scheme as `encoding/ldm/table.rs`.
pub(crate) const DFAST_REBASE_GUARD_BAND: u32 = 1u32 << 30;
// `kSearchStrength` (upstream `zstd_compress_internal.h:32`). The dfast step
// ramp grows one position every `1 << kSearchStrength` = 256 bytes travelled
// (upstream `kStepIncr`, zstd_double_fast.c:131). A smaller value accelerates
// the scan faster and skips source positions upstream still inserts, which
// drops the short matches upstream finds at a block start — so the
// `#167`-disabled path must use the upstream 8 to stay byte-identical.
pub(crate) const DFAST_SKIP_SEARCH_STRENGTH: usize = 8;
pub(crate) const DFAST_SKIP_STEP_GROWTH_INTERVAL: usize = 1 << DFAST_SKIP_SEARCH_STRENGTH;
pub(crate) const DFAST_INCOMPRESSIBLE_SKIP_STEP: usize = 16;
pub(crate) const ROW_HASH_BITS: usize = 20;
pub(crate) const ROW_LOG: usize = 5;
pub(crate) const ROW_SEARCH_DEPTH: usize = 16;
pub(crate) const ROW_TARGET_LEN: usize = 48;
pub(crate) const ROW_TAG_BITS: usize = 8;
pub(crate) const ROW_EMPTY_SLOT: u32 = u32::MAX;
pub(crate) const ROW_HASH_KEY_LEN: usize = 4;
// HASH_MIX_PRIME now lives in `crate::encoding::fastpath::scalar`; the four
// per-CPU `hash_mix_u64` variants share it via that module.
// HC_PRIME3BYTES / HC_PRIME4BYTES moved to match_table::storage
// alongside the hash helpers in Phase 1e Stage A. Only the test
// module references the constants directly (production code goes
// through `MatchTable::hash_value_with_mls`).

// HC_HASH_LOG / HC_CHAIN_LOG / HC3_HASH_LOG / HC_EMPTY live on the
// shared storage module so MatchTable methods can reference them
// without pulling in this module. Re-imported here so existing
// macros / configs / tests keep their unqualified names.

// HC3_MAX_OFFSET moved to encoding::bt alongside the hash3 candidate
// probe macro that consumes it; the macro references it via the
// fully-qualified `$crate::encoding::bt::HC3_MAX_OFFSET` path so this
// module no longer needs a local import.
pub(crate) const HC_SEARCH_DEPTH: usize = 16;
// HC_MIN_MATCH_LEN moved to encoding::hc; re-imported here so
// existing references compile unchanged.
pub(crate) const HC_OPT_MIN_MATCH_LEN: usize = HC_FORMAT_MINMATCH;
pub(crate) const HC_TARGET_LEN: usize = 48;

// MAX_HC_SEARCH_DEPTH moved to encoding::hc alongside chain_candidates.
// Per-level tuning config (the config structs + `LEVEL_TABLE` + the
// level→params resolution chain) lives in `levels::config`; the driver imports
// that resolution API here.
use super::levels::config::*;
// The HashChain / BT match generator + its optimal-parse machinery lives in
// `hc::generator`; the driver stores it in the `HashChain` storage variant.
use super::hc::generator::HcMatchGenerator;

// Dictionary prime + CDict-equivalent snapshot lifecycle. A child module so it
// can reach the driver's private `primed` / `reset_shape` state directly; the
// `Matcher` trait's dict entry points forward to its inherent `*_impl` helpers.
mod dict_prime;

// `Strategy` and `StrategyTag` live in `crate::encoding::strategy`.
// The driver carries a `StrategyTag` field set at `reset()` and
// dispatches each block into a monomorphised `compress_block::<S>`
// per concrete strategy.

/// Backend storage for [`MatchGeneratorDriver`]. Exactly one match-finder
/// state lives in the driver at a time — the active variant. Backend
/// transitions in [`Matcher::reset`] drain the current variant's allocations
/// into the shared `vec_pool` and then replace `storage` with a freshly
/// constructed variant for the new backend.
///
/// Replaces the prior pattern of four parallel fields (`match_generator`,
/// `dfast_match_generator: Option<…>`, `row_match_generator: Option<…>`,
/// `hc_match_generator: Option<…>`) + an `active_backend: BackendTag`
/// discriminator: the parallel layout kept drained inner structures
/// allocated across backend switches, and every per-frame/per-slice
/// driver operation had to dispatch on `active_backend` to pick the
/// right field. A single enum collapses the storage and makes the
/// dispatcher pattern-match on the storage variant directly — same
/// number of arms, but `storage.backend()` is now the canonical source
/// of truth and dead variants are dropped when the active backend
/// changes.
#[derive(Clone)]
enum MatcherStorage {
    /// Upstream zstd `ZSTD_fast` family. Constructed by
    /// [`MatchGeneratorDriver::new`] as the initial variant and
    /// re-selected by [`Matcher::reset`] for any [`CompressionLevel`]
    /// that `resolve_level_params` maps to [`StrategyTag::Fast`]
    /// (`Uncompressed`, `Fastest`, `Level(1)`, and any non-positive
    /// `Level(n)` not equal to `0`).
    Simple(FastKernelMatcher),
    /// Upstream zstd `ZSTD_dfast` family — two-table hash chain. Selected for
    /// any level that resolves to [`StrategyTag::Dfast`] in
    /// `resolve_level_params` (`Default`, `Level(0)`, `Level(2)`,
    /// `Level(3)`).
    Dfast(DfastMatchGenerator),
    /// Upstream zstd `ZSTD_greedy` family with row hashing. Selected for any
    /// level that resolves to [`StrategyTag::Greedy`] (currently
    /// `Level(4)` only).
    Row(RowMatchGenerator),
    /// Upstream zstd `ZSTD_lazy2` and the BT-based optimal modes
    /// (`btopt` / `btultra` / `btultra2`). Selected for any level that
    /// resolves to [`StrategyTag::Lazy`], [`StrategyTag::BtOpt`],
    /// [`StrategyTag::BtUltra`], or [`StrategyTag::BtUltra2`]
    /// (`Better`, `Best`, `Level(5..=22)`, and any `Level(n)` with
    /// `n > MAX_LEVEL` — `resolve_level_params` clamps positive
    /// numeric levels at `MAX_LEVEL = 22` via
    /// `Level(n).clamp(1, MAX_LEVEL)`, so `Level(23..=i32::MAX)` all
    /// land on `BtUltra2` here). The [`HcMatchGenerator`]'s internal
    /// [`HcBackend`] discriminator decides whether BT scratch is
    /// allocated.
    HashChain(HcMatchGenerator),
}

impl MatcherStorage {
    /// Heap bytes the active backend variant holds (tables, history, scratch).
    fn heap_size(&self) -> usize {
        match self {
            Self::Simple(m) => m.heap_size(),
            Self::Dfast(m) => m.heap_size(),
            Self::Row(m) => m.heap_size(),
            Self::HashChain(m) => m.heap_size(),
        }
    }

    /// [`super::strategy::BackendTag`] family of the active variant.
    fn backend(&self) -> super::strategy::BackendTag {
        use super::strategy::BackendTag;
        match self {
            Self::Simple(_) => BackendTag::Simple,
            Self::Dfast(_) => BackendTag::Dfast,
            Self::Row(_) => BackendTag::Row,
            Self::HashChain(_) => BackendTag::HashChain,
        }
    }
}

/// This is the default implementation of the `Matcher` trait. It allocates and reuses the buffers when possible.
pub struct MatchGeneratorDriver {
    vec_pool: Vec<Vec<u8>>,
    /// Active match-finder state. Exactly one backend lives here at a
    /// time; [`Matcher::reset`] drains the previous variant into
    /// `vec_pool` before swapping in a freshly constructed variant for
    /// the new backend. `storage.backend()` is the canonical source of
    /// truth for the parse family; `strategy_tag` carries the
    /// compile-time strategy chosen at the last `reset()`.
    storage: MatcherStorage,
    // Compile-time strategy tag resolved at `reset()` from the
    // requested `CompressionLevel`'s `LevelParams`. The driver's
    // hot-block dispatcher in `blocks/compressed.rs` matches on
    // this tag to enter the corresponding `Strategy`
    // monomorphisation (`compress_block::<S>`).
    strategy_tag: super::strategy::StrategyTag,
    // Decoupled search-method axis resolved at `reset()` from
    // `LevelParams.search`. The per-block dispatcher routes on this
    // (not on `strategy_tag`) so a level's parse and search backend can
    // be chosen independently. The `BinaryTree` arm still consults
    // `strategy_tag` to pick the opt `Strategy` ZST.
    search: super::strategy::SearchMethod,
    // Decoupled parse-mode axis resolved at `reset()` from
    // `LevelParams::parse()`. Independent of `search`: greedy / lazy /
    // lazy2 can run on any non-opt search backend. The backends still
    // read their own `lazy_depth` (kept in sync at `reset()`); this is
    // the authoritative parse selector for the dispatcher.
    pub(crate) parse: super::strategy::ParseMode,

    /// Fine-grained per-knob overrides from the public
    /// [`super::parameters::CompressionParameters`] surface (#27).
    /// `None` (or an all-`None` [`super::parameters::ParamOverrides`])
    /// keeps the resolved level geometry byte-identical to plain
    /// level-based compression. Applied in [`Matcher::reset`] after the
    /// level params are resolved, before backend selection. Persists
    /// across resets (it is frame configuration, not a one-shot) until
    /// the caller changes it.
    param_overrides: Option<super::parameters::ParamOverrides>,
    slice_size: usize,
    base_slice_size: usize,
    // Frame header window size must stay at the configured live-window budget.
    // Dictionary retention expands internal matcher capacity only.
    reported_window_size: usize,
    // Tracks currently retained bytes that originated from primed dictionary
    // history and have not been evicted yet.
    dictionary_retained_budget: usize,
    // Source size hint for next frame (set via set_source_size_hint, cleared on reset).
    source_size_hint: Option<u64>,
    // Dictionary content size for the next frame (set via set_dictionary_size_hint,
    // consumed on reset). When present on a binary-tree / hash-chain backend, the
    // match-finder hash/chain tables are sized from the DICTIONARY (upstream zstd CDict
    // economics: a loaded dictionary supplies the long matches, so the live tables
    // can shrink to the dict's size tier) while the eviction window stays
    // source-sized. Mirrors upstream zstd `ZSTD_getCParamRowSize`, which picks the cParams
    // table column from `dictSize` for a dictionary-bearing compress.
    dictionary_size_hint: Option<usize>,
    // Normalized `ceil_log2` bucket of the frame's source-size hint, captured at
    // `reset` (where `source_size_hint` is consumed) via [`source_size_ceil_log`].
    // `None` means the frame was unhinted. Drives `prime_with_dictionary`'s upstream zstd
    // `ZSTD_shouldAttachDict` mode for the Simple/Fast backend: `None` (unknown)
    // or `<= FAST_ATTACH_DICT_CUTOFF_LOG` → attach (separate dict table, 2-cursor
    // `compress_block_fast_dict`); larger → copy (dictionary primed into the live
    // table, 4-cursor `compress_block_fast`). The primed-snapshot key is the
    // resolved shape ([`reset_shape`](Self::reset_shape)), not this bucket.
    reset_size_log: Option<u8>,
    // Whether the loaded dictionary fits the Fast attach path's tagged position
    // field (`<= MAX_FAST_ATTACH_DICT_REGION`). Captured at `reset` from the
    // dict-size hint (which equals the actual dict length on load) so the Fast
    // attach decision, the attach-epoch reset bit, and the primed-snapshot
    // `fast_attach` bit all gate on it consistently. `true` when there is no
    // dictionary (the attach path is then unused). A dict too large to tag falls
    // back to copy mode instead of overflowing the packed position.
    reset_dict_attach_ok: bool,
    // Hint-resolved matcher shape from the last `reset`: the [`LevelParams`], the
    // active backend's applied Dfast/Row hash-table width (`0` for HC/Fast), the
    // Fast attach-vs-copy mode, and the active LDM override (#27). Combined with
    // the frame's level into the [`PrimedKey`] that keys the primed snapshot, so
    // it is only restored into a reset that resolved the identical matcher AND
    // LDM configuration. `None` before the first `reset`.
    reset_shape: Option<(
        LevelParams,
        usize,
        bool,
        Option<super::parameters::LdmOverride>,
    )>,
    // One-shot borrowed block range `[start, end)` staged by the borrowed
    // Fast frame path (`set_borrowed_block`) for the NEXT
    // `start_matching` / `skip_matching_with_hint`. `Some` routes that
    // call to the Simple backend's borrowed scan instead of the owned
    // committed-block path; consumed (reset to `None`) by the routed
    // call. Always `None` on the owned streaming path.
    borrowed_pending: Option<(usize, usize)>,
    /// CDict-equivalent: snapshot of the post-prime matcher state taken
    /// once after the first dictionary prime — the backend `storage`
    /// (hash tables + dictionary history + offset history + window) plus
    /// the driver-level `dictionary_retained_budget`, the only two pieces
    /// `prime_with_dictionary` writes. Subsequent frames restore this
    /// (a table memcpy) instead of re-hashing every dictionary position,
    /// mirroring upstream zstd `ZSTD_compressBegin_usingCDict` copying the
    /// precomputed `cdict->matchState`. Invalidated when the dictionary
    /// changes; keyed by the [`PrimedKey`] resolved matcher shape so a snapshot
    /// is only restored into a reset that produces the same matcher — see
    /// `restore_primed_dictionary`.
    primed: Option<(MatcherStorage, usize, PrimedKey)>,
}

/// Identity of the matcher configuration a primed snapshot was captured under:
/// the FULLY RESOLVED matcher shape, not the raw source-size hint.
///
/// `reset()` resolves the hint into a [`LevelParams`] (window_log cap, the
/// HC/Fast table and search geometry, the parse depth/target-length that get
/// baked into the restored `storage`) plus, for the Dfast/Row backends, a
/// table-width derived from the hint's ceil-log bucket. The mapping from hint
/// to resolved shape is many-to-one: the source-size adjustment is monotone in
/// `ceil_log2(hint)`, and Level 22 additionally collapses several buckets onto
/// one upstream zstd tier (its `<= 16/128/256 KiB` thresholds). Keying on the raw hint
/// (or even its ceil-log bucket) therefore over-keys — two hints that resolve
/// to the identical matcher would each force a full re-prime. Keying on the
/// resolved (`params`, `table_bits`) pair restores across them.
///
/// `table_bits` is the hint-dependent hash-table width the ACTIVE backend
/// applied (`set_hash_bits` value for Dfast/Row; `0` for HC/Fast, whose widths
/// already live in `params`). The snapshot is only ever captured on the COPY
/// path (a hinted, above-cutoff frame), so `table_bits` is always the resolved
/// Dfast/Row value there, never the unhinted default.
///
/// `level` is kept alongside the resolved `params` because some stored matcher
/// state is derived from the level DIRECTLY, not through `params`: e.g. Dfast's
/// `use_fast_loop` is true for L3 but false for L4, yet L3 and L4 resolve to
/// byte-identical `params`. Without `level` a snapshot captured at L3 could be
/// restored into an L4 reset, installing the wrong `use_fast_loop`.
///
/// `fast_attach` records the Fast backend's attach-vs-copy mode
/// ([`FAST_ATTACH_DICT_CUTOFF_LOG`]) because that cutoff (8 KiB) falls INSIDE a
/// single resolved shape: an 8192- and an 8193-byte Level 1 hint both clamp to
/// window_log 14 with identical `params`/`table_bits`, yet 8192 attaches (a
/// separate dict table) while 8193 copies into the live table — two different
/// `storage` shapes. The frame compressor only captures/restores snapshots on
/// the copy path today, but keying on the mode keeps the snapshot identity
/// self-sufficient rather than relying on that external gate.
///
/// Restoring a snapshot whose key differs would reinstate the old `storage`
/// (and its `max_window_size` / table dimensions / parse params / dict-table
/// shape) under a reset that resolved a different shape — the encoder could
/// then search past the frame header's window and emit an undecodable match.
/// All fields must match before a restore is allowed.
#[derive(Clone, Copy, PartialEq, Eq)]
struct PrimedKey {
    level: CompressionLevel,
    params: LevelParams,
    table_bits: usize,
    fast_attach: bool,
    /// Fine-grained LDM override (#27) active at capture time. The
    /// snapshot's cloned `storage` carries `BtMatcher::ldm_producer`,
    /// which is configured from this override; restoring a snapshot
    /// captured under a different LDM configuration (enable flip or
    /// changed knobs) would reinstate a stale producer. `params` already
    /// pins `window_log` / `strategy_tag` (the rest of the producer's
    /// identity), so folding the override completes the LDM identity.
    /// `None` = LDM off, matching `ParamOverrides::ldm`.
    ldm: Option<super::parameters::LdmOverride>,
}

impl MatchGeneratorDriver {
    /// `slice_size` sets the base block allocation size used for matcher input chunks.
    /// `max_slices_in_window` determines the initial window capacity at construction
    /// time. Effective window sizing is recalculated on every [`reset`](Self::reset)
    /// from the resolved compression level and optional source-size hint.
    pub(crate) fn new(slice_size: usize, max_slices_in_window: usize) -> Self {
        // Validate inputs before deriving window_log_init. Three
        // failure modes need explicit guards:
        //
        // 1. Zero args → `max_window_size = 0` → silent 1-byte
        //    degenerate window (useless).
        // 2. Multiplication overflow on `slice_size *
        //    max_slices_in_window` → wraps silently in release.
        // 3. `next_power_of_two` overflow when the product is
        //    above `1 << (usize::BITS - 1)` → modern Rust PANICS
        //    on overflow (older Rust returned 0).
        //
        // Catch all three at construction with a clear domain-
        // specific message via `assert!` + `checked_mul` +
        // `checked_next_power_of_two`, rather than letting either
        // mode produce a silent degenerate matcher OR a generic
        // panic deep in `FastKernelMatcher::with_params`.
        assert!(
            slice_size > 0,
            "MatchGeneratorDriver::new requires slice_size > 0 (got 0)",
        );
        assert!(
            max_slices_in_window > 0,
            "MatchGeneratorDriver::new requires max_slices_in_window > 0 (got 0)",
        );
        let max_window_size = max_slices_in_window
            .checked_mul(slice_size)
            .expect("MatchGeneratorDriver::new: slice_size * max_slices_in_window overflows usize");
        // Derive an effective window_log for the initial-state matcher.
        // `MatchGeneratorDriver::new` runs BEFORE any reset, so it has
        // no LevelParams to consult — we initialise to whatever
        // window_log fits the caller's requested max_window_size
        // (round up to the next power of two via `next_power_of_two`'s
        // log). Reset() overwrites all three params from the resolved
        // LevelParams.
        //
        // `checked_next_power_of_two` returns `None` if the next power
        // of two would overflow `usize`. Modern Rust's
        // `next_power_of_two` PANICS on overflow rather than returning
        // 0 (the panic message is generic and unhelpful), so use the
        // checked variant to surface the failure with a clear,
        // domain-specific error.
        let next_pow2 = max_window_size.checked_next_power_of_two().expect(
            "MatchGeneratorDriver::new: max_window_size too large for \
             next_power_of_two without overflow",
        );
        let window_log_init = next_pow2.trailing_zeros() as u8;
        Self {
            vec_pool: Vec::new(),
            // Deferred table: `new` runs before any source size or resolved
            // LevelParams exist, so allocating at the level-default hash_log
            // here would be thrown away by the first frame's reset (which
            // clamps the window to the input and reallocs at the resolved
            // size). The deferral lets that first reset allocate exactly once.
            storage: MatcherStorage::Simple(FastKernelMatcher::with_params_deferred(
                window_log_init,
                FAST_LEVEL_1_HASH_LOG,
                FAST_LEVEL_1_MLS,
                2, // upstream zstd default step_size (targetLength=0 → step=2)
            )),
            strategy_tag: super::strategy::StrategyTag::Fast,
            search: super::strategy::SearchMethod::Fast,
            parse: super::strategy::ParseMode::Greedy,

            param_overrides: None,
            slice_size,
            base_slice_size: slice_size,
            // Report the ROUNDED-UP window size that the matcher
            // actually carries (via `window_log_init = log2(next_pow2)`
            // → matcher's `max_window_size = 1 << window_log_init =
            // next_pow2`). For non-power-of-two `slice_size *
            // max_slices_in_window` inputs, the unrounded value
            // would under-report the active backend's window until
            // the first `reset()` overwrites both sides from the
            // resolved LevelParams.
            reported_window_size: next_pow2,
            reset_size_log: None,
            reset_dict_attach_ok: true,
            reset_shape: None,
            dictionary_retained_budget: 0,
            source_size_hint: None,
            dictionary_size_hint: None,
            borrowed_pending: None,
            primed: None,
        }
    }

    fn level_params(level: CompressionLevel, source_size: Option<u64>) -> LevelParams {
        resolve_level_params(level, source_size)
    }

    /// Install the public-parameter per-knob overrides (#27) applied at
    /// the next [`Matcher::reset`]. `None` (or an all-`None` set) restores
    /// plain level-based geometry. Persists across resets until changed.
    pub(crate) fn set_param_overrides(
        &mut self,
        overrides: Option<super::parameters::ParamOverrides>,
    ) {
        self.param_overrides = overrides;
    }

    /// Active backend family derived from the storage variant. Single
    /// source of truth — no separate runtime tag to drift against.
    pub(crate) fn active_backend(&self) -> super::strategy::BackendTag {
        self.storage.backend()
    }

    /// Whether the borrowed (no-copy, in-place over-window) scan is
    /// implemented for the current backend + search configuration. The
    /// HashChain backend serves both the lazy CHAIN parser
    /// (`SearchMethod::HashChain`) and the BT/optimal parsers
    /// (`SearchMethod::BinaryTree`); only the lazy chain has a borrowed scan
    /// so far, so BT/optimal stay on the owned path.
    pub(crate) fn borrowed_supported(&self) -> bool {
        use super::strategy::{BackendTag, SearchMethod, StrategyTag};
        match self.active_backend() {
            BackendTag::Simple | BackendTag::Dfast | BackendTag::Row => true,
            // The HashChain backend covers two searches: the lazy CHAIN parser
            // (borrowed-capable) and the BINARY-TREE search (btlazy2 L13-15 +
            // optimal BtOpt/BtUltra/BtUltra2 L16-22). btlazy2's BT-tree borrowed
            // scan is byte-identical to owned (reads via live_history()), so it
            // takes the in-place path. The OPTIMAL parsers stay owned: their
            // cost-based DP is sensitive to candidate quality, and the borrowed
            // continuous-index scan yields slightly different (ratio-worse)
            // candidates than the owned evict+rehash scan — borrowed optimal
            // both diverged from owned and fell outside the ffi ratio bound.
            // Search-aware (not just strategy_tag) so optimal BT can never be
            // staged on the borrowed path even via an internal caller.
            BackendTag::HashChain => match self.search {
                SearchMethod::HashChain => true,
                SearchMethod::BinaryTree => matches!(self.strategy_tag, StrategyTag::Btlazy2),
                _ => false,
            },
        }
    }

    /// Whether a DICTIONARY frame can take the borrowed (no input copy) path.
    /// Only the Simple (Fast) backend with the dictionary ATTACHED (not the
    /// copy/merge regime) has a borrowed dict scan — `start_matching_borrowed_dict`
    /// reads live matches from the borrowed input in place and dict matches
    /// from the committed dict prefix via the 2-segment counter. Every other
    /// backend, and copy-mode (large-input) dict frames, stay on the owned
    /// path. Checked AFTER priming, so `is_attached()` reflects the resolved
    /// attach-vs-copy decision.
    pub(crate) fn borrowed_dict_supported(&self) -> bool {
        matches!(
            &self.storage,
            MatcherStorage::Simple(m) if m.dict_is_attached()
        )
    }

    fn simple_mut(&mut self) -> &mut FastKernelMatcher {
        match &mut self.storage {
            MatcherStorage::Simple(m) => m,
            _ => panic!("simple backend must be initialized by reset() before use"),
        }
    }

    /// Reclaim the per-block input buffer that the Simple backend
    /// just spent inside `start_matching` / `skip_matching_with_hint`.
    ///
    /// `FastKernelMatcher::take_recycled_space` returns the cleared
    /// (capacity-retained) `Vec<u8>` from the last
    /// `extend_history_with_pending`. We push it onto `vec_pool`
    /// as-is (with `len = 0`); `get_next_space()` is responsible for
    /// resizing the buffer back to `slice_size` on its next pop. The
    /// pushed length is irrelevant — only the capacity matters, and
    /// `extend_history_with_pending` preserves it. Without this
    /// recycle path, the Simple backend would allocate a new
    /// `Vec<u8>` per block — a measurable hot-path cost when blocks
    /// are small (~128 KiB) and processed at hundreds of MiB/s.
    fn recycle_simple_space(&mut self) {
        if let Some(space) = self.simple_mut().take_recycled_space() {
            // `space` is already cleared (len = 0) by
            // `extend_history_with_pending`; capacity is retained.
            // Leaving `len = 0` here avoids the cost of zero-filling
            // the entire allocation — `get_next_space()` resizes the
            // popped buffer up to `slice_size` on demand, so the
            // length the pool holds is irrelevant. This matters most
            // after a small-source-size hint has shrunk `slice_size`
            // mid-frame: the recycled buffer can be much larger than
            // the current `slice_size`, and zero-filling 128 KiB+ on
            // every block would erase the perf win the recycle path
            // is meant to deliver.
            self.vec_pool.push(space);
        }
    }

    /// Register a caller-owned input buffer as the Simple backend's
    /// borrowed one-shot match window. Only valid on the Simple (Fast)
    /// backend; the one-shot frame path gates on that before calling.
    ///
    /// # Safety
    /// Same contract as [`FastKernelMatcher::set_borrowed_window`]: the
    /// buffer must stay live and unmodified until the window is cleared,
    /// and must be cleared before the buffer is dropped or the matcher is
    /// reused for another frame.
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        // SAFETY: forwarded contract — caller upholds liveness/clear.
        match self.active_backend() {
            super::strategy::BackendTag::Simple => unsafe {
                self.simple_mut().set_borrowed_window(buffer)
            },
            super::strategy::BackendTag::Dfast => unsafe {
                self.dfast_matcher_mut().set_borrowed_window(buffer)
            },
            super::strategy::BackendTag::Row => unsafe {
                self.row_matcher_mut().set_borrowed_window(buffer)
            },
            super::strategy::BackendTag::HashChain => unsafe {
                self.hc_matcher_mut().set_borrowed_window(buffer)
            },
        }
    }

    /// Clear the borrowed one-shot window, returning the active backend
    /// to the owned `history` path.
    pub(crate) fn clear_borrowed_window(&mut self) {
        match self.active_backend() {
            super::strategy::BackendTag::Simple => self.simple_mut().clear_borrowed_window(),
            super::strategy::BackendTag::Dfast => self.dfast_matcher_mut().clear_borrowed_window(),
            super::strategy::BackendTag::Row => self.row_matcher_mut().clear_borrowed_window(),
            super::strategy::BackendTag::HashChain => self.hc_matcher_mut().clear_borrowed_window(),
            #[allow(unreachable_patterns)]
            _ => {}
        }
        self.borrowed_pending = None;
    }

    /// Stage the borrowed block range `[block_start, block_end)` for the
    /// NEXT `start_matching` / `skip_matching_with_hint`, which the
    /// borrowed Fast frame path uses in place of `commit_space`. While
    /// staged, those trait calls route to the Simple backend's borrowed
    /// scan/skip (consuming the stage) instead of the owned committed
    /// block. See [`Matcher::start_matching`] /
    /// [`Matcher::skip_matching_with_hint`] on this type.
    pub(crate) fn set_borrowed_block(&mut self, block_start: usize, block_end: usize) {
        assert!(
            self.borrowed_supported(),
            "borrowed block staging is not supported for the active backend/search config",
        );
        assert!(
            block_start <= block_end,
            "borrowed block range must satisfy start <= end (start={block_start} end={block_end})",
        );
        self.borrowed_pending = Some((block_start, block_end));
        // Make the range visible to `get_last_space()` immediately: the
        // emit pipeline reads `get_last_space().len()` in
        // `collect_block_parts` BEFORE `start_matching` consumes the
        // stage, so the staged block (not the whole borrowed window) must
        // be reported now to keep the literal-buffer reservation right.
        match self.active_backend() {
            super::strategy::BackendTag::Simple => self
                .simple_mut()
                .stage_borrowed_block(block_start, block_end),
            super::strategy::BackendTag::Dfast => self
                .dfast_matcher_mut()
                .stage_borrowed_block(block_start, block_end),
            super::strategy::BackendTag::Row => self
                .row_matcher_mut()
                .stage_borrowed_block(block_start, block_end),
            super::strategy::BackendTag::HashChain => self
                .hc_matcher_mut()
                .table
                .stage_borrowed_block(block_start, block_end),
        }
    }

    fn dfast_matcher_mut(&mut self) -> &mut DfastMatchGenerator {
        match &mut self.storage {
            MatcherStorage::Dfast(m) => m,
            _ => panic!("dfast backend must be initialized by reset() before use"),
        }
    }

    pub(crate) fn row_matcher_mut(&mut self) -> &mut RowMatchGenerator {
        match &mut self.storage {
            MatcherStorage::Row(m) => m,
            _ => panic!("row backend must be initialized by reset() before use"),
        }
    }

    fn hc_matcher_mut(&mut self) -> &mut HcMatchGenerator {
        match &mut self.storage {
            MatcherStorage::HashChain(m) => m,
            _ => panic!("hash chain backend must be initialized by reset() before use"),
        }
    }

    /// Shrink the active backend's `max_window_size` by the bytes
    /// reclaimed from the dictionary-retention budget. Returns `true`
    /// iff any reclamation happened — the caller uses that as the
    /// gate for [`Self::trim_after_budget_retire`] (which is a no-op
    /// otherwise: with `max_window_size` unchanged the backend's
    /// `trim_to_window` cannot find anything to evict, so calling it
    /// just runs an extra `match` ladder + a single early-out check
    /// per slice commit).
    #[must_use]
    fn retire_dictionary_budget(&mut self, evicted_bytes: usize) -> bool {
        let reclaimed = evicted_bytes.min(self.dictionary_retained_budget);
        if reclaimed == 0 {
            return false;
        }
        self.dictionary_retained_budget -= reclaimed;
        match self.active_backend() {
            super::strategy::BackendTag::Simple => {
                let matcher = self.simple_mut();
                // `reclaimed` can exceed the CURRENT `max_window_size`: the
                // retained dict budget is tracked independently and the
                // window may already have been shrunk by a prior eviction,
                // so the floor at 0 is the correct clamp, not a masked bug.
                matcher.max_window_size = matcher.max_window_size.saturating_sub(reclaimed);
            }
            super::strategy::BackendTag::Dfast => {
                let matcher = self.dfast_matcher_mut();
                // `reclaimed` can exceed the CURRENT `max_window_size`: the
                // retained dict budget is tracked independently and the
                // window may already have been shrunk by a prior eviction,
                // so the floor at 0 is the correct clamp, not a masked bug.
                matcher.max_window_size = matcher.max_window_size.saturating_sub(reclaimed);
            }
            super::strategy::BackendTag::Row => {
                let matcher = self.row_matcher_mut();
                // `reclaimed` can exceed the CURRENT `max_window_size`: the
                // retained dict budget is tracked independently and the
                // window may already have been shrunk by a prior eviction,
                // so the floor at 0 is the correct clamp, not a masked bug.
                matcher.max_window_size = matcher.max_window_size.saturating_sub(reclaimed);
            }
            super::strategy::BackendTag::HashChain => {
                let matcher = self.hc_matcher_mut();
                // See the Simple arm: `reclaimed` may exceed the current
                // window, so saturating to 0 is the correct clamp.
                matcher.table.max_window_size =
                    matcher.table.max_window_size.saturating_sub(reclaimed);
            }
        }
        true
    }

    fn trim_after_budget_retire(&mut self) {
        loop {
            let mut evicted_bytes = 0usize;
            match self.active_backend() {
                super::strategy::BackendTag::Simple => {
                    // FastKernelMatcher owns its history as a single
                    // flat `Vec<u8>` (upstream zstd's flat-buffer layout)
                    // rather than the legacy per-block `WindowEntry`
                    // stack. There are no per-block Vec allocations
                    // to recycle into `vec_pool` — `trim_to_window`
                    // drains the oldest bytes in-place and returns
                    // the count for the dictionary-budget loop's
                    // termination check.
                    let MatcherStorage::Simple(m) = &mut self.storage else {
                        unreachable!("active_backend() == Simple proven above");
                    };
                    evicted_bytes += m.trim_to_window();
                }
                super::strategy::BackendTag::Dfast => {
                    // Dfast doesn't retain input Vecs — `history` is the
                    // only byte store, so there is no per-block buffer
                    // to push back through a callback. Eviction byte
                    // count is derived from the `window_size` delta
                    // before/after; the Dfast variant of
                    // `trim_to_window` takes no closure, sidestepping
                    // an unused-`impl FnMut` monomorphization that
                    // would otherwise contractually never fire.
                    let dfast = self.dfast_matcher_mut();
                    let pre = dfast.window_size;
                    dfast.trim_to_window();
                    evicted_bytes += pre - dfast.window_size;
                }
                super::strategy::BackendTag::Row => {
                    // Row keeps bytes only in the contiguous `history` mirror
                    // (block buffers are returned to the pool per block in
                    // `add_data`), so derive the eviction count from the
                    // `window_size` delta, mirroring the Dfast / HashChain arms.
                    let row = self.row_matcher_mut();
                    let pre = row.window_size;
                    row.trim_to_window();
                    evicted_bytes += pre - row.window_size;
                }
                super::strategy::BackendTag::HashChain => {
                    // HC keeps bytes only in the contiguous `history` mirror
                    // (no per-block Vecs to recycle since the window<->history
                    // dedup), so derive the eviction count from the
                    // `window_size` delta, mirroring the Dfast arm above.
                    let table = &mut self.hc_matcher_mut().table;
                    let pre = table.window_size;
                    table.trim_to_window();
                    evicted_bytes += pre - table.window_size;
                }
            }
            if evicted_bytes == 0 {
                break;
            }
            // The loop's invariant is "the backend's previous
            // `max_window_size` shrink had downstream bytes left to
            // evict" — that's what `evicted_bytes != 0` proves at
            // this point. `dictionary_retained_budget` is NOT
            // guaranteed to be positive here: the outer
            // `retire_dictionary_budget` call may have already
            // drained it to zero by reclaiming the last retained
            // bytes, while the backend still has bytes above the
            // freshly-shrunk window cap waiting for this loop to
            // evict. The return value of the retire call below is
            // therefore intentionally discarded — the loop's
            // termination is driven by `evicted_bytes == 0`, not by
            // whether the budget has more bytes left to reclaim.
            let _ = self.retire_dictionary_budget(evicted_bytes);
        }
    }

    /// ATTACH (`true`) vs COPY (`false`) decision for the dms-bearing HashChain
    /// backend (lazy hash-chain AND binary-tree/optimal levels), mirroring
    /// upstream `ZSTD_shouldAttachDict` and its per-strategy `attachDictSizeCutoffs`:
    /// a small / unknown source ATTACHES the dict as a separate dms (hash-chain
    /// dms for lazy, DUBT dms for BT); a large known source COPIES it into the
    /// live chain / tree. The cutoff is the lazy/lazy2 value for HC, the
    /// btlazy2/btopt value for Bt{Opt}, and the smaller btultra/btultra2 value for
    /// the deepest parses. Both `skip_matching_for_dictionary_priming` (which
    /// stages the dict) and `prime_with_dictionary` (which builds-or-drops the
    /// dms) read this so the two stay in lock-step.
    fn hc_dict_attach_mode(&self) -> bool {
        // Only the HashChain backend (lazy hash-chain + BT/optimal) routes here;
        // a non-HashChain storage has no dms decision, so default to attach.
        let MatcherStorage::HashChain(hc) = &self.storage else {
            return true;
        };
        let cutoff = if hc.table.uses_bt {
            match hc.strategy_tag {
                super::strategy::StrategyTag::BtUltra | super::strategy::StrategyTag::BtUltra2 => {
                    BT_ULTRA_ATTACH_DICT_CUTOFF_LOG
                }
                _ => BT_OPT_ATTACH_DICT_CUTOFF_LOG,
            }
        } else {
            HC_ATTACH_DICT_CUTOFF_LOG
        };
        self.reset_size_log.is_none_or(|log| log <= cutoff)
    }

    fn skip_matching_for_dictionary_priming(&mut self) {
        match self.active_backend() {
            super::strategy::BackendTag::Simple => {
                // Upstream zstd `ZSTD_shouldAttachDict` mode selection for the Fast
                // strategy (cutoff 8 KB): small / unknown-size inputs ATTACH
                // (index dict positions into a SEPARATE immutable table; the
                // dual-probe 2-cursor `compress_block_fast_dict` then prefers
                // recent-input matches and falls back to the dict — the path
                // that wins small/unknown). Large known-size inputs COPY (prime
                // dict into the live table; the 4-cursor `compress_block_fast`
                // matches against it as window history — the path that already
                // matches/beats the upstream zstd on large corpora). The dispatch in
                // `start_matching` keys off `dict_table.is_some()`, which only
                // the attach path populates. See [`FAST_ATTACH_DICT_CUTOFF_LOG`].
                let attach = self.reset_dict_attach_ok
                    && self
                        .reset_size_log
                        .is_none_or(|log| log <= FAST_ATTACH_DICT_CUTOFF_LOG);
                if attach {
                    self.simple_mut().skip_matching_for_dict_prime();
                } else {
                    self.simple_mut().skip_matching_with_hint(Some(false));
                }
                self.recycle_simple_space();
            }
            super::strategy::BackendTag::Dfast => {
                // Upstream zstd `ZSTD_dictMatchState` mode selection for dfast (cutoff
                // 16 KiB): small / unknown-size inputs ATTACH (build the
                // separate immutable dict long+short tables; the dual-probe
                // `start_matching_fast_loop` searches live + dict, the path that
                // avoids the per-frame dict re-prime that dominates small
                // `compress-dict`). Larger known-size inputs COPY (re-prime the
                // dict into the live tables via `skip_matching_dense`, where the
                // dense scan matches it as window history). `skip_matching_for_dict_attach`
                // self-gates on `use_fast_loop` (only fast-loop levels carry the
                // dual-probe; general-path levels fall back to the dense copy).
                let attach = self
                    .reset_size_log
                    .is_none_or(|log| log <= DFAST_ATTACH_DICT_CUTOFF_LOG);
                if attach {
                    self.dfast_matcher_mut().skip_matching_for_dict_attach();
                } else {
                    self.dfast_matcher_mut().invalidate_dict_cache();
                    self.dfast_matcher_mut().skip_matching_dense();
                }
            }
            super::strategy::BackendTag::Row => {
                // Upstream zstd `ZSTD_RowFindBestMatch` `dictMatchState`: small /
                // unknown-size inputs ATTACH (build the separate immutable dict
                // row index; the bounded dual-probe in `row_candidate_rl`
                // searches live + dict, avoiding the per-frame dict re-index),
                // larger known-size inputs COPY (dense re-prime into the live
                // rows).
                let attach = self
                    .reset_size_log
                    .is_none_or(|log| log <= ROW_ATTACH_DICT_CUTOFF_LOG);
                if attach {
                    self.row_matcher_mut().prime_dict_attach_current_block();
                } else {
                    self.row_matcher_mut().invalidate_dict_cache();
                    self.row_matcher_mut().skip_matching_with_hint(Some(false));
                }
            }
            super::strategy::BackendTag::HashChain => {
                // Lazy-HC AND BT/optimal both follow upstream zstd `ZSTD_shouldAttachDict`
                // per-strategy: ATTACH (a separate dms — hash-chain dms for lazy,
                // DUBT dms for BT) for small / unknown inputs, COPY (merge the dict
                // into the live chain/tree) for large known inputs. ATTACH keeps
                // the dict in history but out of the live structure via
                // `skip_matching_dict_bt` (the cursor advance is shared by both
                // arms); COPY routes through the normal `skip_matching` (its
                // `uses_bt` branch fills the live tree, the lazy branch the live
                // chain). The dms is built-or-dropped to match in
                // `prime_with_dictionary`.
                if self.hc_dict_attach_mode() {
                    self.hc_matcher_mut().table.skip_matching_dict_bt();
                } else {
                    self.hc_matcher_mut().skip_matching(Some(false));
                }
            }
        }
    }
}

impl Matcher for MatchGeneratorDriver {
    fn supports_dictionary_priming(&self) -> bool {
        true
    }

    fn set_source_size_hint(&mut self, size: u64) {
        self.source_size_hint = Some(size);
    }

    fn set_dictionary_size_hint(&mut self, size: usize) {
        self.dictionary_size_hint = Some(size);
    }

    /// Dict-relevance gate for the raw-fast-path. Reached only when a dictionary
    /// is active (the caller short-circuits on `dict_active`), so this answers
    /// "could the dict compress this otherwise-incompressible-looking block?".
    /// The Simple (Fast) backend samples its dict table precisely
    /// ([`FastKernelMatcher::block_samples_match_dict`]); the other backends
    /// (Dfast / Row / HashChain / BT) have their own dict structures and no cheap
    /// probe here, so they answer CONSERVATIVELY `true`: without a probe they
    /// cannot tell whether the dict compresses an incompressible-LOOKING block,
    /// and answering `false` would let the raw-fast-path emit such a block raw
    /// and miss an embedded dict segment. `dictionary_segment_in_incompressible_input_is_matched`
    /// pins this for Dfast/Row/BT — the 512-byte dict run inside high-entropy
    /// filler is matched only because these backends stay on the scan. So they
    /// keep the blanket scan the old `!dict_active` gate gave them; only the
    /// Simple/Fast backend trades it for the precise probe.
    fn block_samples_match_dict(&self, block: &[u8]) -> bool {
        match &self.storage {
            MatcherStorage::Simple(m) => m.block_samples_match_dict(block),
            _ => true,
        }
    }

    /// Heap bytes this driver owns: the active backend's tables/history, the
    /// recycled input-buffer pool, and the primed-dictionary snapshot (a cloned
    /// backend kept for CDict-equivalent reuse). The inline struct itself is
    /// accounted by the owner's `size_of`.
    fn heap_size(&self) -> usize {
        let pool: usize = self.vec_pool.capacity() * size_of::<Vec<u8>>()
            + self.vec_pool.iter().map(Vec::capacity).sum::<usize>();
        let snapshot = self
            .primed
            .as_ref()
            .map_or(0, |(storage, _, _)| storage.heap_size());
        pool + self.storage.heap_size() + snapshot
    }

    fn clear_param_overrides(&mut self) {
        self.param_overrides = None;
    }

    fn reset(&mut self, level: CompressionLevel) {
        let hint = self.source_size_hint.take();
        let dict_hint = self.dictionary_size_hint.take();
        // Snapshot the hint's normalized ceil-log bucket for the primed-snapshot
        // key and prime_with_dictionary's attach/copy mode decision (the hint is
        // consumed here, but priming happens just after reset). Storing the
        // bucket rather than the raw bytes means two hints that resolve to the
        // same matcher shape share one snapshot instead of each re-priming.
        self.reset_size_log = hint.map(source_size_ceil_log);
        // A dictionary too large for the tagged attach position field falls back
        // to copy mode. Captured here (from the load-set size hint = actual dict
        // length) so the prime decision and the snapshot-key / epoch bits agree.
        self.reset_dict_attach_ok =
            dict_hint.is_none_or(|size| size <= MAX_FAST_ATTACH_DICT_REGION);
        let hinted = hint.is_some();
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut params = Self::level_params(level, hint);
        // Test-only: apply a parse×search override so the matrix can be
        // exercised without editing `LEVEL_TABLE`. Mutating `params` here
        // (before `next_backend`) flows the override through storage
        // selection, `configure`, and the `self.search`/`self.parse`
        // writes uniformly. Consumed with `take()` so it is one-shot: the
        // synthetic pairing applies to exactly this `reset()`, and a later
        // reset on the same driver falls back to the level's real config.

        // Public-parameter overrides (#27): apply the per-knob set on top
        // of the level-resolved params. A strategy override re-routes the
        // backend, so this must precede `next_backend` selection. The
        // all-`None` case is skipped so default level geometry stays
        // byte-identical to plain level-based compression.
        if let Some(ov) = self.param_overrides
            && !ov.is_empty()
        {
            apply_param_overrides(&mut params, &ov);
            // `Self::level_params(level, hint)` applied the source-size cap
            // for the LEVEL's native backend. If a strategy override moved
            // the frame onto a different backend, `apply_param_overrides`
            // synthesized that backend's DEFAULT config (FAST_L1 /
            // HC_OVERRIDE_DEFAULT) with full-size table logs AFTER that cap
            // ran. Re-apply the hint cap so a tiny hinted frame doesn't
            // allocate the new backend's full-size tables. An explicit
            // `window_log` override is the user's hard request and must
            // survive the re-cap, so restore it afterwards.
            if let Some(hint_size) = hint {
                params = adjust_params_for_source_size(params, hint_size);
                if let Some(window_log) = ov.window_log {
                    params.window_log = window_log;
                }
            }
        }
        // Dictionary-driven table sizing — parity with upstream zstd `ZSTD_createCDict`
        // (`ZSTD_getCParams_internal(level, UNKNOWN, dictSize, ZSTD_cpm_createCDict)`
        // → `ZSTD_adjustCParams_internal`). A loaded dictionary supplies the
        // long-distance matches, so upstream zstd sizes the prepared match-finder tables
        // to the DICTIONARY (assuming a `minSrcSize` source), not the live
        // window: it downsizes `hashLog`/`chainLog` toward the dict-and-window
        // log while leaving the frame's eviction `window_log` source-derived so
        // the dictionary bytes stay referenceable (`ZSTD_resetCCtx_byCopyingCDict`
        // copies the small CDict tables but keeps the source window). We apply
        // the same downsizing to the level's own hc geometry and cap (min) so a
        // dict never inflates the level tables. Only the binary-tree / hash-chain
        // backend reads `hc.{hash,chain}_log`; Simple/Dfast/Row derive their
        // widths from the source window in their `reset` arms.
        // A zero-length dictionary is "no dictionary": running the CDict sizing
        // path for `Some(0)` is not a no-op — `cdict_table_logs(.., 0)` still
        // collapses the HC/BT tables toward the 513-byte upstream zstd tier via
        // `DICT_MIN_SRC_SIZE`, tanking ratio/perf on the next frame. Priming
        // already treats empty content as empty, so skip the downsizing here too.
        if let Some(dict_size) = dict_hint.filter(|&size| size > 0) {
            // Derive the dict-tier geometry from the level's FULL (un-source-capped)
            // hc widths. `Self::level_params(level, hint)` already source-capped
            // `params.hc`; feeding those capped widths into `cdict_table_logs` and
            // then `.min()`-ing would double-cap, so on a small hinted source with a
            // large dictionary the prepared tables collapse below what the dict needs
            // — defeating the `ZSTD_createCDict` geometry this mirrors. Take the
            // un-hinted base widths instead and assign the result directly:
            // `cdict_table_logs` only ever downsizes, so it never exceeds the base
            // level geometry, while the eviction `window_log` stays source-derived so
            // the dictionary bytes remain referenceable. Active public-parameter
            // overrides (#27) are applied to the base too, so a strategy override
            // that routes onto HashChain/BinaryTree still gets dict-tier sizing and
            // explicit hash/chain overrides feed through as the geometry ceiling.
            let mut base_params = Self::level_params(level, None);
            if let Some(ov) = self.param_overrides
                && !ov.is_empty()
            {
                apply_param_overrides(&mut base_params, &ov);
            }
            if let (Some(hc), Some(base_hc)) = (params.hc.as_mut(), base_params.hc) {
                let uses_bt = matches!(
                    params.strategy_tag,
                    super::strategy::StrategyTag::Btlazy2
                        | super::strategy::StrategyTag::BtOpt
                        | super::strategy::StrategyTag::BtUltra
                        | super::strategy::StrategyTag::BtUltra2
                );
                let (dict_hash_log, dict_chain_log) = cdict_table_logs(
                    params.window_log,
                    base_hc.hash_log,
                    base_hc.chain_log,
                    uses_bt,
                    dict_size,
                );
                hc.hash_log = dict_hash_log;
                hc.chain_log = dict_chain_log;
            }
        }
        // upstream zstd `ZSTD_resolveRowMatchFinderMode` (zstd_compress.c:238-245):
        // the row matchfinder is used for greedy/lazy/lazy2 ONLY when
        // `windowLog > 14`; at or below that upstream runs the hash-chain
        // matcher (`ZSTD_HcFindBestMatch`). We previously hardcoded the Row
        // backend for these strategies regardless of window, sending every
        // small-window frame (hinted floor = windowLog 14, e.g. the small-4k/10k
        // fixtures) through Row where upstream uses HC. Match it: fall back to
        // the hash-chain matcher (lazy/greedy parse via `lazy_depth`) when the
        // resolved window is <= 14. The HC config is synthesised from the
        // level's RowConfig (HC and Row share the same cParams; only the
        // matchfinder differs) — `hash_log` / `chain_log` are
        // clamped to the (<= 14) window inside the HashChain reset arm, so the
        // nominal width here only sets the clamp ceiling.
        if params.search == super::strategy::SearchMethod::RowHash && params.window_log <= 14 {
            let row = params
                .row
                .expect("a RowHash level row must carry a RowConfig");
            params.search = super::strategy::SearchMethod::HashChain;
            // For a dict-bearing frame, downsize the synthesised HC logs to the
            // dictionary's content tier via `cdict_table_logs` (the same
            // correction the native HC dict-prime path applies above), so a dict
            // much smaller than the window doesn't prime a needlessly sparse
            // table. Row-finder levels are never BinaryTree, so `uses_bt = false`.
            //
            // Feed `cdict_table_logs` the UN-hinted base Row width, not the
            // resolved `row.hash_bits`: the latter is already source-capped on a
            // hinted reset (the `row_cap = table_log + 1` clamp), so passing it
            // here would double-cap exactly as the native HC dict path warns
            // above — a small hinted source with a large dictionary would
            // collapse the prepared table below what the dict needs.
            // `cdict_table_logs` only ever downsizes, so deriving the ceiling
            // from the un-hinted base (plus active public overrides) keeps the
            // dict-tier geometry intact. No source hint => `row.hash_bits` is
            // already the level's full width, so reuse it directly.
            let row_cdict_hash_bits = match dict_hint.filter(|&size| size > 0) {
                Some(_) => {
                    let mut base_params = Self::level_params(level, None);
                    if let Some(ov) = self.param_overrides
                        && !ov.is_empty()
                    {
                        apply_param_overrides(&mut base_params, &ov);
                    }
                    base_params
                        .row
                        .map_or(row.hash_bits, |base_row| base_row.hash_bits)
                }
                None => row.hash_bits,
            };
            // Row-backed levels carry only `hash_bits`; the HC chain table they
            // fall back to follows the upstream zstd cParams relationship `chainLog =
            // hashLog - 1` for every Row level (L6 c18 h19 .. L12 c22 h23, see
            // the ROW_L* tables). Synthesise the chain width as `hash_bits - 1`
            // so the dict path doesn't leave the chain table one bit too wide
            // (cdict_table_logs only downsizes, so passing the full hash width
            // for both would keep a 2x-too-large chain table on dict frames).
            // Raw `- 1` is underflow-safe: `hash_bits` is either a predefined
            // ROW_L* width (>= 19) or a public `hash_log` override, and the
            // override is range-validated to `ZSTD_HASHLOG_MIN = 6` at the
            // parameter API, so the value is always >= 6 here.
            //
            // A public `chain_log` override (#27) is dropped by the RowHash
            // override arm (Row has no chain table), but once this frame falls
            // back to HC the chain table is live and must honour it — mirror
            // the native HC dict path, which feeds the override-applied
            // `base_hc.chain_log` into `cdict_table_logs`. Use the explicit
            // override (also API-validated to ZSTD_CHAINLOG_MIN = 6) when set,
            // else the upstream zstd `hashLog - 1` relationship.
            let explicit_chain_log = self
                .param_overrides
                .filter(|ov| !ov.is_empty())
                .and_then(|ov| ov.chain_log)
                .map(|chain_log| chain_log as usize);
            let row_cdict_chain_bits = explicit_chain_log.unwrap_or(row_cdict_hash_bits - 1);
            let (mut hash_log, mut chain_log) = match dict_hint.filter(|&size| size > 0) {
                Some(dict_size) => cdict_table_logs(
                    params.window_log,
                    row_cdict_hash_bits,
                    row_cdict_chain_bits,
                    false,
                    dict_size,
                ),
                None => (
                    row.hash_bits,
                    explicit_chain_log.unwrap_or(row.hash_bits - 1),
                ),
            };
            // No-dict path: the HashChain reset arm only clamps the logs to the
            // window when `hinted`, but a public `window_log` override can lower
            // this level to <= 14 with no source hint — clamp the level's full
            // Row `hash_bits` to the window here too (upstream zstd `ZSTD_adjustCParams`:
            // hashLog <= windowLog + 1, chainLog <= windowLog) so a 16 KiB window
            // doesn't allocate Row-sized HC tables.
            if dict_hint.filter(|&size| size > 0).is_none() {
                let wlog = params.window_log as usize;
                hash_log = hash_log.min(wlog + 1);
                chain_log = chain_log.min(wlog);
            }
            params.hc = Some(HcConfig {
                hash_log,
                chain_log,
                search_depth: row.search_depth,
                target_len: row.target_len,
                search_mls: 4,
            });
            params.row = None;
        }
        let next_backend = params.backend();
        let max_window_size = 1usize << params.window_log;
        self.dictionary_retained_budget = 0;
        // Drop any frame-local borrowed staging so it can't leak across a
        // reset and misroute the next start/skip into borrowed dispatch.
        self.borrowed_pending = None;
        if self.active_backend() != next_backend {
            // Drain the outgoing backend's allocations into the shared
            // pool. The `match &mut self.storage { ... }` block runs to
            // completion before the assignment below replaces the
            // variant, so the inner state we just drained is dropped
            // with the old variant.
            match &mut self.storage {
                MatcherStorage::Simple(_m) => {
                    // FastKernelMatcher owns a flat Vec<u8> history
                    // and a Vec<u32> hash table — both drop with the
                    // variant assignment below, no per-block buffers
                    // to recycle into the driver pools. The
                    // assignment-replace path collapses to a noop
                    // pre-pass for this backend.
                }
                MatcherStorage::Dfast(m) => {
                    // Drop the long / short hash table allocations
                    // before calling `m.reset`. Without this prepass,
                    // `DfastMatchGenerator::reset` would `fill` both
                    // tables with `DFAST_EMPTY_SLOT` sentinels — wasted
                    // work given the next assignment to `self.storage`
                    // is about to drop `m` entirely. `reset` itself
                    // short-circuits on `if !self.tables.is_empty()`, so
                    // handing it an empty `Vec` skips the fill loop.
                    // Mirrors the pre-drain pattern in the HashChain
                    // arm below (and serves the same peak-memory
                    // purpose: release the table-allocation footprint
                    // before constructing the replacement variant).
                    m.tables = Vec::new();
                    m.reset();
                }
                MatcherStorage::Row(m) => {
                    m.row_heads = Vec::new();
                    m.row_positions = Vec::new();
                    m.row_tags = Vec::new();
                    m.reset();
                }
                MatcherStorage::HashChain(m) => {
                    // Release oversized tables when switching away from
                    // HashChain so Best's larger allocations don't persist.
                    // hash3_table must be released alongside the other
                    // two: BtUltra2's `1 << HC3_HASH_LOG` entries would
                    // otherwise stay pinned across the backend switch,
                    // even though no future caller of this backend will
                    // touch them.
                    m.table.hash_table = Vec::new();
                    m.table.chain_table = Vec::new();
                    m.table.hash3_table = Vec::new();
                    let vec_pool = &mut self.vec_pool;
                    m.reset(|mut data| {
                        data.resize(data.capacity(), 0);
                        vec_pool.push(data);
                    });
                }
            }
            // Swap in a fresh variant for the new backend. The previous
            // `storage` is dropped here.
            self.storage = match next_backend {
                super::strategy::BackendTag::Simple => {
                    // Per-level Fast cParams from resolve_level_params:
                    // Level(1) gets (hash_log=14, mls=7); Level(-7..=-1)
                    // get upstream zstd row-0 (hash_log=13, mls=7); Fastest /
                    // Uncompressed keep (hash_log=14, mls=6). See
                    // resolve_level_params for rationale.
                    let fast = params.fast.expect("Fast level row carries a FastConfig");
                    MatcherStorage::Simple(FastKernelMatcher::with_params(
                        params.window_log,
                        fast.hash_log,
                        fast.mls,
                        fast.step_size,
                    ))
                }
                super::strategy::BackendTag::Dfast => {
                    MatcherStorage::Dfast(DfastMatchGenerator::new(max_window_size))
                }
                super::strategy::BackendTag::Row => {
                    MatcherStorage::Row(RowMatchGenerator::new(max_window_size))
                }
                super::strategy::BackendTag::HashChain => {
                    MatcherStorage::HashChain(HcMatchGenerator::new(max_window_size))
                }
            };
        }

        // Single source of truth: `LevelParams::strategy_tag` is the
        // authoritative mapping from `CompressionLevel` to strategy.
        // `storage.backend()` derives the parse family from the variant,
        // so there is no separate runtime tag that could drift against
        // `LEVEL_TABLE`.
        self.strategy_tag = params.strategy_tag;
        self.search = params.search;
        self.parse = params.parse();
        self.slice_size = self.base_slice_size.min(max_window_size);
        self.reported_window_size = max_window_size;
        let strategy_tag = self.strategy_tag;
        // Source-proportional table window for the backends whose hash-table
        // widths are recomputed here (Dfast / Row). Like the HC / Fast caps
        // in `adjust_params_for_source_size`, this sizes the internal tables
        // from the RAW source log (not the wire `window_log` floor) so a
        // small frame zeroes a small table; it never exceeds the real window.
        let table_window_size = match hint {
            Some(h) => {
                let raw_log = source_size_ceil_log(h);
                // Clamp the shift below the pointer width before `1usize <<`:
                // an oversized hint (>= 2^63 + 1, and on 32-bit usize any hint
                // >= 2^32) drives `raw_log` to 64 / >= 32, and the shift would
                // overflow (panic in debug, wrap to 0 in release) before the
                // `.min(max_window_size)` cap below could bound it. The min cap
                // still provides the real semantic window bound.
                let shift = raw_log.max(MIN_WINDOW_LOG).min(usize::BITS as u8 - 1);
                (1usize << shift).min(max_window_size)
            }
            None => max_window_size,
        };
        // The hint-dependent hash-table width the active backend applies, for
        // the primed-snapshot key. Dfast/Row compute it from `table_window_size`
        // below; HC/Fast leave it `0` because their widths live in `params`
        // (`hc.{hash,chain}_log` / `fast_hash_log`) — already part of the key.
        let mut resolved_table_bits: usize = 0;
        match &mut self.storage {
            MatcherStorage::Simple(m) => {
                // Per-level Fast cParams threaded from
                // resolve_level_params (see Simple-backend swap
                // arm above for the (level → params) mapping).
                let fast = params.fast.expect("Fast level row carries a FastConfig");
                // Same attach/copy split the dict-prime dispatch applies
                // below (`prime_with_dictionary`): only attach-mode dict
                // frames may keep the main table across the reset via an
                // epoch advance — copy-mode and no-dict frames must memset
                // it back to bias 0 for the raw-slice kernels.
                // `Some(0)` is "no dictionary" (the dict-sizing path above
                // filters it the same way): an empty dict primes nothing, so
                // an epoch-advance reset would preserve stale attach state
                // instead of clearing it.
                let dict_attach_epoch = matches!(dict_hint, Some(size) if size > 0)
                    && self.reset_dict_attach_ok
                    && self
                        .reset_size_log
                        .is_none_or(|log| log <= FAST_ATTACH_DICT_CUTOFF_LOG);
                // Copy-mode dictionary frame whose primed snapshot matches
                // this exact resolved shape: `restore_primed_dictionary`
                // (called right after this reset; the caller gates the
                // restore on the same size bucket and the restore re-checks
                // the same key) will `clone_from` the snapshot over this
                // matcher, replacing the table contents and bias wholesale —
                // the reset's full-table memset would be thrown away. The
                // key components mirror `reset_shape` below: Simple leaves
                // `resolved_table_bits` 0, never carries an LDM override,
                // and `fast_attach` is false in copy mode by construction.
                let table_overwritten_by_restore = matches!(dict_hint, Some(size) if size > 0)
                    && !dict_attach_epoch
                    && self.primed.as_ref().is_some_and(|(_, _, captured)| {
                        *captured
                            == PrimedKey {
                                level,
                                params,
                                table_bits: 0,
                                fast_attach: false,
                                ldm: None,
                            }
                    });
                // Cap `hash_log <= window_log + 1` (upstream zstd
                // `ZSTD_adjustCParams_internal`): once `window_log` is resized
                // down for a small source, a level-default `1 << hash_log`
                // table is mostly wasted address space whose per-frame memset
                // dominates the compress cost on tiny frames (a 4 KB frame at
                // window_log 12 still zero-fills the 64 KiB hash_log-14 table).
                // Gated to no-dict frames: the dict-attach path shares one
                // hash_log between the main and dict tables (so one hash keys
                // both), and shrinking only the main table would break that
                // invariant and the small-frame dict ratio.
                let hash_log = if dict_hint.is_some_and(|s| s > 0) {
                    fast.hash_log
                } else {
                    fast.hash_log.min(params.window_log as u32 + 1)
                };
                m.reset(
                    params.window_log,
                    hash_log,
                    fast.mls,
                    fast.step_size,
                    dict_attach_epoch,
                    table_overwritten_by_restore,
                );
            }
            MatcherStorage::Dfast(dfast) => {
                dfast.max_window_size = max_window_size;
                let dcfg = params
                    .dfast
                    .expect("Dfast level row must carry a DfastConfig");
                // Upstream zstd `cParams.hashLog`/`chainLog`, capped by the
                // source-size window when hinted so tiny inputs don't
                // over-allocate.
                let long_bits = if hinted {
                    dfast_hash_bits_for_window(table_window_size).min(dcfg.long_hash_log as usize)
                } else {
                    dcfg.long_hash_log as usize
                };
                let short_bits = if hinted {
                    dfast_hash_bits_for_window(table_window_size).min(dcfg.short_hash_log as usize)
                } else {
                    dcfg.short_hash_log as usize
                };
                resolved_table_bits = long_bits;
                dfast.set_hash_bits(long_bits, short_bits);
                // Dfast holds no per-block input Vecs (history owns the
                // bytes and `add_data` returns each Vec eagerly), so
                // `reset` takes no `reuse_space` callback.
                dfast.reset();
            }
            MatcherStorage::Row(row) => {
                row.max_window_size = max_window_size;
                row.lazy_depth = params.lazy_depth;
                let mut row_cfg = params.row.expect("Row level row carries a RowConfig");
                if hinted {
                    // Clamp the configured hash width by the hinted window
                    // (upstream zstd `ZSTD_adjustCParams` caps hashLog by windowLog) —
                    // `min`, not replace, so an explicit `hash_log` param
                    // override (`row_cfg.hash_bits`) survives the hinted path
                    // instead of being overwritten by the window value.
                    //
                    // Clamp BEFORE `configure` so the backend sees ONE width
                    // per frame. Configuring with the unclamped level width
                    // and then re-clamping made `row_hash_log` oscillate on
                    // every hinted frame, and each width change clears the
                    // row tables — `ensure_tables` then re-filled all three
                    // every frame in a reused compressor.
                    row_cfg.hash_bits = row_cfg
                        .hash_bits
                        .min(row_hash_bits_for_window(table_window_size));
                }
                row.configure(row_cfg);
                // Key the primed snapshot on the width the backend ACTUALLY
                // applied (`set_hash_bits` clamps the request): recording the
                // request — or the 0 default on the unhinted path — keys
                // identical table geometries apart and forces needless
                // dictionary re-primes.
                resolved_table_bits = row.hash_bits();
                row.reset();
            }
            MatcherStorage::HashChain(hc) => {
                hc.table.max_window_size = max_window_size;
                hc.hc.lazy_depth = params.lazy_depth;
                let mut hc_cfg = params.hc.expect("HashChain level row carries an HcConfig");
                // Cap the hash / chain table logs by the hinted window so a small
                // input doesn't allocate the full level's tables (the upstream zstd
                // `ZSTD_adjustCParams_internal` clamp: `hashLog <= windowLog + 1`,
                // and `cycleLog <= windowLog` — `cycleLog == chainLog` for the HC
                // finder, `chainLog - 1` for the BT pair table, so `chainLog <=
                // windowLog` (+1 for BT)). Ratio-neutral: a hinted window of
                // `2^wlog` bytes holds at most `2^wlog` positions, so the slots
                // beyond that are never populated — capping only sheds unused
                // allocation. Was the source of L10-lazy peak-alloc ~2.15x the
                // upstream zstd on a 1 MiB input. Only applied when hinted; an
                // unknown-size stream keeps the full level tables.
                // Skip for dict-bearing frames: their `hc_cfg.{hash,chain}_log`
                // were already sized to the dictionary content tier via
                // `cdict_table_logs` (the dict supplies the long-distance
                // matches, so upstream `ZSTD_createCDict` sizes the prepared
                // tables to the dict, not the source window). Re-applying the
                // source-window cap here would collapse those dict-tier logs
                // back to the small hinted source — the same double-cap the
                // synthesis sites avoid by using the un-hinted base width.
                if hinted && !matches!(dict_hint, Some(size) if size > 0) {
                    let wlog = hc_hash_bits_for_window(table_window_size);
                    let uses_bt = matches!(
                        strategy_tag,
                        super::strategy::StrategyTag::Btlazy2
                            | super::strategy::StrategyTag::BtOpt
                            | super::strategy::StrategyTag::BtUltra
                            | super::strategy::StrategyTag::BtUltra2
                    );
                    hc_cfg.hash_log = hc_cfg.hash_log.min(wlog + 1);
                    hc_cfg.chain_log = hc_cfg.chain_log.min(if uses_bt { wlog + 1 } else { wlog });
                }
                hc.configure(hc_cfg, strategy_tag, params.window_log);
                let vec_pool = &mut self.vec_pool;
                hc.reset(|mut data| {
                    data.resize(data.capacity(), 0);
                    vec_pool.push(data);
                });
                // When the source size is known, pre-size the history mirror to
                // the expected total (dictionary + payload) so per-block growth
                // does not overshoot via Vec capacity doubling (upstream zstd sizes its
                // window buffer exactly). Dominates peak once the match-finder
                // tables are dictionary-tier-small. Unhinted streams skip this
                // and keep doubling growth.
                if let Some(src) = hint {
                    // `src` is a u64 hint and may be the u64::MAX "unknown
                    // size" sentinel, which truncates under `as usize` on
                    // 32-bit targets and overflows when the dict hint is
                    // added. Saturate the source size, then saturate the
                    // dict-hint addition; `reserve_history` applies the
                    // tighter window ceiling to the result.
                    let src_hint = usize::try_from(src).unwrap_or(usize::MAX);
                    let expected = src_hint.saturating_add(dict_hint.unwrap_or(0));
                    hc.table.reserve_history(expected);
                }
            }
        }
        // LDM wiring (#27): attach (or clear) the long-distance-match
        // producer on the optimal (BT) backend. LDM is the only
        // back-reference path that crosses the regular window, so it
        // only has a home on the `BtMatcher`; non-BT strategies drop the
        // producer. Built AFTER `hc.reset()` because `BtMatcher::reset`
        // clears an existing producer's table but does not null the
        // slot — installing here gives the new frame a fresh producer.

        {
            // Resolve the derived LDM params first (immutable borrow of the
            // overrides), then reuse the existing producer's allocation below.
            let derived_ldm = self
                .param_overrides
                .as_ref()
                .and_then(|ov| ov.ldm)
                .map(|ldm_ov| {
                    let strategy_ord = ldm_strategy_ordinal(params.strategy_tag, params.lazy_depth);
                    // Seed the caller-pinned knobs, then run the upstream zstd
                    // derivation over the seed so the remaining (zero)
                    // fields are filled with cross-field consistency
                    // (e.g. `hash_rate_log = window_log - hash_log`).
                    // Clobbering after `adjust_for` would break that and
                    // hand the producer an inconsistent set.
                    let seed = super::ldm::params::LdmParams {
                        window_log: params.window_log as u32,
                        hash_log: ldm_ov.hash_log.unwrap_or(0),
                        hash_rate_log: ldm_ov.hash_rate_log.unwrap_or(0),
                        min_match_length: ldm_ov.min_match.unwrap_or(0),
                        bucket_size_log: ldm_ov.bucket_size_log.unwrap_or(0),
                    };
                    seed.derive(strategy_ord)
                });
            if let MatcherStorage::HashChain(hc) = &mut self.storage {
                // Reuse the existing producer's hash-table allocation when the
                // derived params are unchanged: only `clear()` (re-zero the
                // table + re-seed the rolling hash, no allocation) is needed for
                // the new frame. A params change (or the first frame) forces a
                // fresh `LdmProducer::new`. On the reused-encoder compress-dict
                // path this avoids re-allocating the LDM hash table (large at
                // btultra2) every frame — upstream zstd reuses its `ldmState_t`
                // the same way. `clear()` is mandatory here for correctness
                // regardless of what `BtMatcher::reset` did to the old table.
                let producer = derived_ldm.map(|p| match hc.take_ldm_producer() {
                    Some(mut existing) if existing.params() == p => {
                        existing.clear();
                        existing
                    }
                    _ => super::ldm::LdmProducer::new(p),
                });
                hc.set_ldm_producer(producer);
            }
        }
        // Record the resolved matcher shape for the primed-snapshot key. Captured
        // here (post-resolution, after the test-only param override) so the key
        // reflects exactly the geometry the restored `storage` must match. The
        // Fast attach-vs-copy mode is part of the shape ONLY for the Simple
        // backend (it decides the distinct dict-table shape that backend builds).
        // Dfast/Row/HashChain have their OWN attach/copy regimes, but this bit
        // models only the Fast table split; those backends are keyed by the
        // resolved matcher geometry instead, so folding the Fast bit into their
        // key would over-key identical resolved shapes. When it applies it
        // matches the decision `prime_with_dictionary` makes from the same
        // `reset_size_log`.
        let fast_attach = matches!(next_backend, super::strategy::BackendTag::Simple)
            && self.reset_dict_attach_ok
            && self
                .reset_size_log
                .is_none_or(|log| log <= FAST_ATTACH_DICT_CUTOFF_LOG);
        // The LDM override is part of the snapshot identity ONLY on the
        // optimal (BinaryTree) path: that is the only backend whose cloned
        // `storage` carries a `BtMatcher::ldm_producer`. On Fast / Dfast /
        // Row and lazy-HashChain resets the producer slot does not exist,
        // so folding the override there would over-key the snapshot and
        // force needless re-primes when LDM is toggled. Gated like
        // `fast_attach` (a key bit only participates where it changes the
        // cloned matcher shape).
        let active_ldm = if matches!(params.search, super::strategy::SearchMethod::BinaryTree) {
            self.param_overrides.and_then(|ov| ov.ldm)
        } else {
            None
        };
        self.reset_shape = Some((params, resolved_table_bits, fast_attach, active_ldm));
    }

    // Dictionary entry points forward to the `dict_prime` child module, which
    // owns the prime / snapshot lifecycle (it reaches the driver's private
    // `primed` / `reset_shape` state directly as a descendant module).

    #[inline]
    fn dictionary_is_resident(&self) -> bool {
        self.dictionary_is_resident_impl()
    }

    #[inline]
    fn reapply_resident_dictionary(&mut self, offset_hist: [u32; 3]) {
        self.reapply_resident_dictionary_impl(offset_hist)
    }

    #[inline]
    fn prime_with_dictionary(&mut self, dict_content: &[u8], offset_hist: [u32; 3]) {
        self.prime_with_dictionary_impl(dict_content, offset_hist)
    }

    #[inline]
    fn restore_primed_dictionary(&mut self, level: CompressionLevel) -> bool {
        self.restore_primed_dictionary_impl(level)
    }

    #[inline]
    fn capture_primed_dictionary(&mut self, level: CompressionLevel) {
        self.capture_primed_dictionary_impl(level)
    }

    #[inline]
    fn invalidate_primed_dictionary(&mut self) {
        self.invalidate_primed_dictionary_impl()
    }

    #[inline]
    fn seed_dictionary_entropy(
        &mut self,
        huff: Option<&crate::huff0::huff0_encoder::HuffmanTable>,
        ll: Option<&crate::fse::fse_encoder::FSETable>,
        ml: Option<&crate::fse::fse_encoder::FSETable>,
        of: Option<&crate::fse::fse_encoder::FSETable>,
    ) {
        self.seed_dictionary_entropy_impl(huff, ll, ml, of)
    }

    fn window_size(&self) -> u64 {
        self.reported_window_size as u64
    }

    fn get_next_space(&mut self) -> Vec<u8> {
        if let Some(mut space) = self.vec_pool.pop() {
            if space.len() > self.slice_size {
                space.truncate(self.slice_size);
            }
            if space.len() < self.slice_size {
                space.resize(self.slice_size, 0);
            }
            return space;
        }
        alloc::vec![0; self.slice_size]
    }

    fn get_last_space(&mut self) -> &[u8] {
        match &self.storage {
            MatcherStorage::Simple(m) => m.last_committed_space(),
            MatcherStorage::Dfast(m) => m.get_last_space(),
            MatcherStorage::Row(m) => m.get_last_space(),
            MatcherStorage::HashChain(m) => m.table.get_last_space(),
        }
    }

    fn commit_space(&mut self, space: Vec<u8>) {
        let mut evicted_bytes = 0usize;
        // Split borrows manually so the `add_data` closures can write
        // into `vec_pool` while the backend itself holds an exclusive
        // borrow via `storage`. (Suffix-store recycling went away
        // with the legacy `MatchGenerator`; the FastKernelMatcher
        // arm below has no pool interaction.)
        let vec_pool = &mut self.vec_pool;
        match &mut self.storage {
            MatcherStorage::Simple(m) => {
                // FastKernelMatcher owns its history as a single
                // flat Vec<u8> and the hash table as a Vec<u32> —
                // neither recycles into the driver-side pools. The
                // eager pre-commit eviction inside
                // `FastKernelMatcher::accept_data` drops bytes when
                // accepting this block would push history past 2×
                // max_window_size; that delta is what feeds
                // `evicted_bytes` here via the `pre / post`
                // history-length comparison.
                let pre = m.history_len_for_eviction_accounting();
                m.accept_data(space);
                let post = m.history_len_for_eviction_accounting();
                // `accept_data` performs eager pre-commit window
                // eviction (so this `pre - post` delta correctly
                // feeds the dictionary-budget retire flow). See
                // `FastKernelMatcher::accept_data` for the
                // commit-time-visibility rationale (closes #216
                // CodeRabbit review #5 / Copilot review #1: without
                // eager eviction, the delta was always 0 and the
                // dict budget never retired, leaving max_window_size
                // inflated post-dict-prime → matcher could emit
                // offsets exceeding the frame header's window).
                evicted_bytes += pre.saturating_sub(post);
            }
            MatcherStorage::Dfast(m) => {
                // Dfast's `add_data` callback receives the INPUT
                // `Vec<u8>` for pool recycling (Dfast stores its
                // bytes in the contiguous `history` buffer, not in
                // per-block Vecs — there is no per-block buffer to
                // pop off and hand back). Counting `data.len()` as
                // evicted bytes would conflate "new bytes ingested"
                // with "old bytes evicted from window"; the two
                // happen to coincide when the previous window was
                // saturated and the new input fills it 1:1, but
                // diverge when the eviction pop-loop drops blocks
                // of a different size than the incoming input. The
                // `dictionary_retained_budget` retire decision
                // downstream then gets driven by inflated eviction
                // counts and shrinks `max_window_size` prematurely.
                //
                // Derive the real eviction delta from `window_size`
                // before/after the call. The pop loop inside
                // `add_data` decrements `window_size` by each
                // evicted block length and then the final
                // `extend_from_slice + push_back` adds `space_len`,
                // so `evicted = pre + space_len - post`.
                let pre = m.window_size;
                let space_len = space.len();
                m.add_data(space, |data| {
                    // Same per-block recycle as the HashChain arm: push
                    // the spent input buffer back as-is rather than
                    // zero-filling to capacity. `add_data` mirrors the
                    // bytes into `history` and calls this every block, so
                    // capacity-wide zeroing would be hot-path waste;
                    // `get_next_space` zeroes at most `slice_size` bytes
                    // when it later reuses the buffer.
                    vec_pool.push(data);
                });
                // Plain `+` (the `saturating_sub` floors at 0): `pre` + one
                // block are byte counts bounded by the window, no overflow.
                evicted_bytes += (pre + space_len).saturating_sub(m.window_size);
            }
            MatcherStorage::Row(m) => {
                // RowMatchGenerator::add_data recycles the *input* buffer
                // through this callback every commit (its bytes are mirrored
                // into `history`), not the evicted chunks. Derive the eviction
                // delta from `window_size` before/after — `evicted = pre +
                // space_len - post` — exactly like the Simple / HashChain arms.
                // Counting the callback argument as evicted would charge the
                // whole committed block as evicted and prematurely retire
                // dictionary budget on a window that evicts nothing.
                let pre = m.window_size;
                let space_len = space.len();
                m.add_data(space, |data| {
                    // Recycle the spent buffer as-is; `add_data` runs this for
                    // every committed block, so zero-filling to capacity here
                    // would be hot-path waste (`get_next_space` zeroes at most
                    // `slice_size` on reuse).
                    vec_pool.push(data);
                });
                // Plain `+` (the `saturating_sub` floors at 0): `pre` + one
                // block are byte counts bounded by the window, no overflow.
                evicted_bytes += (pre + space_len).saturating_sub(m.window_size);
            }
            MatcherStorage::HashChain(m) => {
                // MatchTable::add_data now recycles the *incoming* buffer
                // through `reuse_space` (its bytes are copied into the
                // contiguous `history` mirror), so the callback no longer
                // reports evicted chunks. Derive the eviction delta from
                // `window_size` before/after, exactly like the Simple arm:
                // `evicted = pre + space_len - post`.
                let pre = m.table.window_size;
                let space_len = space.len();
                m.table.add_data(space, |data| {
                    // Recycle the spent input buffer to the pool as-is.
                    // `add_data` runs this callback for every committed
                    // block (the bytes are mirrored into `history`), so
                    // growing the buffer to its full capacity here would
                    // zero the whole allocation on the hot path.
                    // `get_next_space` resizes a popped buffer to
                    // `slice_size` on demand, touching at most
                    // `slice_size` bytes — never the larger capacity the
                    // pool retains.
                    vec_pool.push(data);
                });
                // Plain `+` (the `saturating_sub` floors at 0): byte counts
                // bounded by the window, no overflow.
                evicted_bytes += (pre + space_len).saturating_sub(m.table.window_size);
            }
        }
        // Gate the second backend trim pass on actual budget
        // reclamation. Without it, every slice commit on the
        // no-dictionary / no-eviction path (the common case) would
        // run a backend `match` ladder + `trim_to_window` early-out
        // for no reason — `trim_after_budget_retire` only does
        // meaningful work when `retire_dictionary_budget` shrank
        // `max_window_size` enough to make the backend's
        // `window_size > max_window_size` invariant trigger
        // eviction.
        if self.retire_dictionary_budget(evicted_bytes) {
            self.trim_after_budget_retire();
        }
    }

    fn start_matching(&mut self, mut handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        use super::strategy::{self, StrategyTag};
        // Borrowed one-shot Fast path: if the frame driver staged a
        // block range via `set_borrowed_block`, scan it in place against
        // the borrowed window instead of the owned committed block. Only
        // the Simple backend is instrumented (the gate guarantees it),
        // and the stage is consumed so the next block re-stages.
        if let Some((block_start, block_end)) = self.borrowed_pending.take() {
            match self.active_backend() {
                strategy::BackendTag::Simple => {
                    let m = self.simple_mut();
                    if m.dict_is_attached() {
                        // Dict-attach borrowed scan: live matches read the
                        // borrowed input in place, dict matches read the
                        // committed dict prefix via the 2-segment counter.
                        m.start_matching_borrowed_dict(
                            block_start,
                            block_end,
                            &mut handle_sequence,
                        );
                    } else {
                        m.start_matching_borrowed(block_start, block_end, &mut handle_sequence);
                    }
                }
                strategy::BackendTag::Dfast => self.dfast_matcher_mut().start_matching_borrowed(
                    block_start,
                    block_end,
                    &mut handle_sequence,
                ),
                strategy::BackendTag::Row => {
                    // Same greedy/lazy parse split as the owned RowHash arm.
                    let greedy = self.parse == strategy::ParseMode::Greedy;
                    self.row_matcher_mut().start_matching_borrowed(
                        block_start,
                        block_end,
                        greedy,
                        &mut handle_sequence,
                    );
                }
                strategy::BackendTag::HashChain => match self.search {
                    SearchMethod::HashChain => self.hc_matcher_mut().start_matching_lazy_borrowed(
                        block_start,
                        block_end,
                        &mut handle_sequence,
                    ),
                    SearchMethod::BinaryTree => {
                        // Run the SAME BT dispatch as the owned BinaryTree arm
                        // below — every BT body reads its range via
                        // current_block_range() and bytes via live_history()
                        // (borrowed-aware), so the staged block is scanned in
                        // place. The table was already staged by
                        // `set_borrowed_block` (the HashChain arm at the top of
                        // this file calls `table.stage_borrowed_block` with the
                        // same range, and `borrowed_pending` is set only there),
                        // so no re-stage is needed here.
                        // Only btlazy2 reaches the borrowed BinaryTree scan:
                        // `borrowed_supported()` keeps the optimal parsers
                        // (BtOpt/BtUltra/BtUltra2) on the owned path, and
                        // `set_borrowed_block` asserts that predicate before any
                        // range is staged, so an optimal strategy_tag can never
                        // arrive here.
                        match self.strategy_tag {
                            StrategyTag::Btlazy2 => self
                                .hc_matcher_mut()
                                .start_matching_btlazy2(&mut handle_sequence),
                            other => unreachable!(
                                "borrowed BinaryTree scan is only supported for Btlazy2, got {other:?}"
                            ),
                        }
                    }
                    other => {
                        unreachable!("HashChain backend with unexpected search {other:?}")
                    }
                },
            }
            return;
        }
        // Decoupled parse×search dispatch (fires once per block). The
        // search axis (`self.search`) picks the candidate-finding backend;
        // the parse axis (greedy vs lazy depth) is carried by the
        // backend's runtime `lazy_depth`, set per level at `reset()`.
        // The two are independent, so any parse can run on any search
        // backend. The `BinaryTree` arm still selects the opt `Strategy`
        // ZST off `strategy_tag` so `compress_block::<S>` keeps its
        // const-folded optimal-parser monomorphisation.
        use super::strategy::SearchMethod;
        match self.search {
            SearchMethod::Fast => {
                self.simple_mut().start_matching(&mut handle_sequence);
                self.recycle_simple_space();
            }
            SearchMethod::DoubleFast => {
                self.dfast_matcher_mut()
                    .start_matching(&mut handle_sequence);
            }
            SearchMethod::RowHash => {
                // Greedy parse (depth 0) = upstream zstd-greedy entry (default
                // `ip + 1` start, greedy repcode commit); lazy / lazy2 use
                // the `pick_lazy_match` lookahead entry (reads `lazy_depth`).
                // Both bare entries dispatch on `row_log` internally into the
                // const-`ROW_LOG` hot loop (upstream zstd per-rowLog variant table).
                let greedy = self.parse == strategy::ParseMode::Greedy;
                let row = self.row_matcher_mut();
                if greedy {
                    row.start_matching_greedy(&mut handle_sequence);
                } else {
                    row.start_matching(&mut handle_sequence);
                }
            }
            SearchMethod::HashChain => {
                // Greedy/lazy/lazy2 all flow through the lazy parser; it
                // reads `hc.lazy_depth` (0 = greedy commit).
                self.hc_matcher_mut()
                    .start_matching_lazy(&mut handle_sequence);
            }
            SearchMethod::BinaryTree => match self.strategy_tag {
                StrategyTag::Btlazy2 => self
                    .hc_matcher_mut()
                    .start_matching_btlazy2(&mut handle_sequence),
                StrategyTag::BtOpt => self.compress_block::<strategy::BtOpt>(&mut handle_sequence),
                StrategyTag::BtUltra => {
                    self.compress_block::<strategy::BtUltra>(&mut handle_sequence)
                }
                StrategyTag::BtUltra2 => {
                    self.compress_block::<strategy::BtUltra2>(&mut handle_sequence)
                }
                _ => unreachable!(
                    "SearchMethod::BinaryTree requires a BT strategy tag (Btlazy2/BtOpt/BtUltra/BtUltra2)"
                ),
            },
        }
    }

    fn skip_matching(&mut self) {
        self.skip_matching_with_hint(None);
    }

    fn skip_matching_with_hint(&mut self, incompressible_hint: Option<bool>) {
        // Borrowed one-shot Fast path: a staged block range routes to the
        // borrowed skip (records the range for `get_last_space`, primes
        // hashes on the dict-priming hint) with no owned-history append
        // and nothing to recycle. Stage is consumed.
        if let Some((block_start, block_end)) = self.borrowed_pending.take() {
            match self.active_backend() {
                super::strategy::BackendTag::Simple => self.simple_mut().skip_matching_borrowed(
                    block_start,
                    block_end,
                    incompressible_hint,
                ),
                super::strategy::BackendTag::Dfast => self
                    .dfast_matcher_mut()
                    .skip_matching_borrowed(block_start, block_end, incompressible_hint),
                super::strategy::BackendTag::Row => self.row_matcher_mut().skip_matching_borrowed(
                    block_start,
                    block_end,
                    incompressible_hint,
                ),
                super::strategy::BackendTag::HashChain => self
                    .hc_matcher_mut()
                    .skip_matching_borrowed(block_start, block_end, incompressible_hint),
            }
            return;
        }
        match self.active_backend() {
            super::strategy::BackendTag::Simple => {
                self.simple_mut()
                    .skip_matching_with_hint(incompressible_hint);
                self.recycle_simple_space();
            }
            super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().skip_matching(incompressible_hint)
            }
            super::strategy::BackendTag::Row => self
                .row_matcher_mut()
                .skip_matching_with_hint(incompressible_hint),
            super::strategy::BackendTag::HashChain => {
                self.hc_matcher_mut().skip_matching(incompressible_hint)
            }
        }
    }
}

impl MatchGeneratorDriver {
    /// Monomorphised optimal-parser entry point. Only the `BinaryTree`
    /// search arm of [`Matcher::start_matching`] routes here, selecting
    /// the concrete opt `S: Strategy` (BtOpt / BtUltra / BtUltra2) off
    /// `strategy_tag`, so the optimiser keeps the cost-model predicates
    /// (`S::USE_BT` / `S::USE_HASH3` / `S::ACCURATE_PRICE` /
    /// `S::TWO_PASS_SEED`) const-folded per strategy. The non-opt search
    /// backends (Fast / DoubleFast / RowHash / HashChain) are dispatched
    /// directly off the search axis and never reach this method, so all
    /// strategies arriving here are HashChain-backed.
    fn compress_block<S: super::strategy::Strategy>(
        &mut self,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        debug_assert_eq!(S::BACKEND, super::strategy::BackendTag::HashChain);
        debug_assert!(
            S::USE_BT,
            "compress_block only handles the optimal (BT) path"
        );
        self.hc_matcher_mut()
            .start_matching_strategy::<S>(handle_sequence);
    }
}

/// Stage D: backend storage discriminator.
///
/// HC (lazy / lazy2) modes carry no extra per-frame state beyond the
/// shared `MatchTable` and `HcMatcher` runtime knobs, so the
/// [`HcBackend::Hc`] variant is zero-sized — no BT scratch is
/// allocated. BT-flavoured modes (`btopt` / `btultra` / `btultra2`)
/// hold the full [`super::bt::BtMatcher`] inside the
/// [`HcBackend::Bt`] variant (cost model, optimal-parser scratch
/// arenas, LDM candidate buffer).
///
/// The discriminator lives next to `parse_mode` so `configure()` can
/// promote between the two on a level change without touching the
/// `MatchTable` storage.
#[derive(Clone)]
pub(crate) enum HcBackend {
    /// Lazy / lazy2 modes — no per-frame backend state.
    Hc,
    /// BT-driven modes — owns the optimal parser's per-frame scratch.
    /// Boxed so the enum stays pointer-sized: HC-only matchers pay
    /// just the `Box`-niche, not the 4 KiB `BtMatcher` payload.
    Bt(alloc::boxed::Box<super::bt::BtMatcher>),
}
