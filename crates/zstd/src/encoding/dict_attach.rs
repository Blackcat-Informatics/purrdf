#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Shared CDict-style dictionary-attach lifecycle, parameterized by the
//! matcher's immutable dictionary table type `T`.
//!
//! This is the level-1 scaffolding common to every match-finder backend that
//! supports upstream zstd `ZSTD_dictMatchState` attach-by-reference: instead of
//! re-priming the whole dictionary into the live hash table(s) on every frame
//! (O(dict) per frame, the dominant cost on small-payload `compress-dict`), the
//! dictionary is hashed ONCE into a SEPARATE immutable table `T` held here, and
//! the kernel dual-probes the live table(s) plus this dict table. A reused
//! compressor keeps `T` across the per-frame `reset` (the dict re-commits to the
//! same absolute history positions) via the `primed` cache flag — mirroring
//! C copying / referencing `cdict->matchState` rather than rebuilding it.
//!
//! `T` is backend-specific (Fast = single `FastHashTable`; Dfast = a long+short
//! pair; Row/HC/BT = their own structures); the per-backend BUILD of `T` and the
//! per-kernel dual-probe LOOKUP are level-2 (kept in each backend, expanded per
//! SIMD tier). This type only owns the shared lifecycle: presence, the
//! dict/input boundary, the build-once cache flag, and invalidation.

/// Lifecycle holder for an attached, immutable dictionary table of type `T`.
#[derive(Debug, Default)]
pub(crate) struct DictAttach<T> {
    /// The immutable dictionary table, built once over the dictionary region.
    /// `Some` activates the backend's dual-probe kernel; `None` means no dict
    /// is attached (or it was invalidated, e.g. on history eviction that would
    /// stale the absolute dict positions) and the plain kernel runs.
    table: Option<T>,
    /// Number of dictionary bytes at the front of history — one past the last
    /// valid dict position. The boundary the dual-probe kernel uses to separate
    /// live-table (input) matches from dict-table matches. `0` when unattached.
    region_len: usize,
    /// CDict-equivalent cache flag: `true` once `table` is fully built for the
    /// attached dictionary. A reused compressor keeps the built table across
    /// per-frame `reset` and skips the re-hash. Cleared on parameter change,
    /// history eviction, or dictionary attach/clear via [`Self::invalidate`].
    primed: bool,
    /// Next history position a multi-slice dict fill should process (upstream
    /// zstd `ms->nextToUpdate`). A dictionary loaded across several
    /// `accept_data` slices is hashed incrementally; without a persistent
    /// high-water the second slice's fill would restart at its own slice
    /// origin and drop the `HASH_READ_SIZE - 1` seam positions just below it
    /// (their wide hash read straddles the slice boundary, so the prior slice
    /// could not reach them). Carrying the fill origin forward keeps the
    /// stride phase continuous and closes the seam gap. `0` until the first
    /// fill; reset by [`Self::invalidate`].
    next_to_update: usize,
}

impl<T: Clone> Clone for DictAttach<T> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            region_len: self.region_len,
            primed: self.primed,
            next_to_update: self.next_to_update,
        }
    }

    // Recurse into the table's `clone_from` (via `Option::clone_from`) so
    // snapshot restores reuse the retained table buffers.
    fn clone_from(&mut self, source: &Self) {
        self.table.clone_from(&source.table);
        self.region_len = source.region_len;
        self.primed = source.primed;
        self.next_to_update = source.next_to_update;
    }
}

impl<T> DictAttach<T> {
    pub(crate) const fn new() -> Self {
        Self {
            table: None,
            region_len: 0,
            primed: false,
            next_to_update: 0,
        }
    }

    /// Next history position a multi-slice dict fill should process (upstream
    /// zstd `ms->nextToUpdate`). Backends carry the fill origin forward across
    /// slices via this so the cross-slice seam positions are not dropped.
    #[inline]
    pub(crate) fn next_to_update(&self) -> usize {
        self.next_to_update
    }

    /// Record how far the dict fill has advanced (the first position not yet
    /// hashed). The next slice's fill resumes here, keeping the stride phase
    /// continuous across the slice seam.
    #[inline]
    pub(crate) fn set_next_to_update(&mut self, pos: usize) {
        self.next_to_update = pos;
    }

    /// Whether a dict table is attached (drives the dual-probe dispatch).
    #[inline]
    pub(crate) fn is_attached(&self) -> bool {
        self.table.is_some()
    }

    /// Shared reference to the dict table, if attached.
    #[inline]
    pub(crate) fn table(&self) -> Option<&T> {
        self.table.as_ref()
    }

    /// The dict/input boundary (`dict_end`) for kernel bounds checks.
    #[inline]
    pub(crate) fn region_len(&self) -> usize {
        self.region_len
    }

    /// Record the dict/input boundary. Set every prime call regardless of
    /// whether any position was hashable (a sub-min-match dict still bounds the
    /// input floor).
    #[inline]
    pub(crate) fn set_region_len(&mut self, region_len: usize) {
        self.region_len = region_len;
    }

    /// CDict cache flag: `true` once the table is fully built. The prime path
    /// checks this to skip the re-hash on reused frames.
    #[inline]
    pub(crate) fn is_primed(&self) -> bool {
        self.primed
    }

    /// Mark the table fully built (CDict cache). Only marks when a table
    /// actually exists — a sub-min-match dict builds no table and must re-run
    /// the (cheap, no-op) prime path each frame.
    #[inline]
    pub(crate) fn mark_primed(&mut self) {
        if self.table.is_some() {
            self.primed = true;
        }
    }

    /// Get the table for building, initializing it with `init` if absent.
    /// Backends call this lazily inside their per-backend prime once they know
    /// at least one position is hashable.
    #[inline]
    pub(crate) fn table_mut_or_init(&mut self, init: impl FnOnce() -> T) -> &mut T {
        self.table.get_or_insert_with(init)
    }

    /// Mutable reference to the table, if attached (for the build/fill pass).
    #[inline]
    pub(crate) fn table_mut(&mut self) -> Option<&mut T> {
        self.table.as_mut()
    }

    /// Drop the cached dict table, boundary, and primed flag. Called when the
    /// next frame carries no dictionary (or on eviction/param change) so the
    /// kernel never probes a stale dict region.
    #[inline]
    pub(crate) fn invalidate(&mut self) {
        self.table = None;
        self.region_len = 0;
        self.primed = false;
        self.next_to_update = 0;
    }
}
