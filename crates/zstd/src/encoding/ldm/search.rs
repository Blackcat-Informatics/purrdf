#![allow(
    clippy::needless_pass_by_value,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Bucket lookup + forward / backward extension for the LDM
//! producer.
//!
//! Implements the per-split candidate selection from upstream zstd
//! `ZSTD_ldm_generateSequences_internal` (`zstd_ldm.c:405-466`)
//! v1.5.7, **prefix-only path**. The two-segment `extDict`
//! variant (upstream zstd `ZSTD_count_2segments` +
//! `ZSTD_ldm_countBackwardsMatch_2segments`) is deferred — the
//! current Rust encoder does not surface a separate `extDict`
//! buffer, so every byte the producer can reach lives in a
//! single contiguous `history` slice and the prefix-only path
//! is bit-for-bit equivalent to upstream zstd on those inputs.
//!
//! Upstream zstd parity anchors:
//! * `ZSTD_count`                        → [`super::super::match_table::helpers::common_prefix_len`]
//! * `ZSTD_ldm_countBackwardsMatch`      → [`count_backwards_match`]
//! * Per-bucket best-match selection     → [`find_best_match`]

use super::super::match_table::helpers::common_prefix_len;
use super::table::LdmHashTable;

/// Result of [`find_best_match`]: a verified LDM candidate.
///
/// Holds the *resolved* forward and backward lengths separately
/// because the caller needs both to derive the emitted raw-seq
/// `lit_length` (`split - backward - anchor`) and the wire-format
/// match length (`forward + backward`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct LdmMatch {
    /// **Absolute** byte position of the matching window's start
    /// in the back reference (upstream zstd `bestEntry->offset`). Stored
    /// as `usize` so streams larger than `u32::MAX` (4 GiB) stay
    /// representable end-to-end; the table rebases its internal
    /// `u32` storage transparently via
    /// [`LdmHashTable::ensure_room_for`].
    pub(crate) match_pos: usize,
    /// Bytes that matched forward from `split`.
    pub(crate) forward_len: usize,
    /// Bytes that matched backward from `split` (capped by
    /// `split - anchor` and by `match_pos > lowest_index`).
    pub(crate) backward_len: usize,
}

impl LdmMatch {
    /// Total match length emitted on the wire. Upstream zstd
    /// `mLength = forwardMatchLength + backwardMatchLength`
    /// (`zstd_ldm.c:477`).
    pub(crate) const fn total_len(&self) -> usize {
        self.forward_len + self.backward_len
    }
}

/// Upstream zstd `ZSTD_ldm_countBackwardsMatch` (`zstd_ldm.c:214-225`):
/// walk left from `(p_in, p_match)` while bytes still match and
/// both pointers stay above their respective lower bounds.
///
/// Bounds expressed as **slice indices** into `history`: the
/// caller is expected to translate absolute stream positions to
/// slice indices via `abs - history_abs_start` before invoking.
/// `p_in_idx` is the candidate's "split" index, `p_match_idx`
/// is the back-ref index; `anchor_idx` / `match_base_idx` are
/// the corresponding lower bounds. The walk stops when either
/// pointer reaches its bound or the bytes diverge.
///
/// Returns the number of matched backward bytes (capped at the
/// tighter of `min(p_in_idx - anchor_idx, p_match_idx -
/// match_base_idx)`).
pub(crate) fn count_backwards_match(
    history: &[u8],
    p_in_idx: usize,
    anchor_idx: usize,
    p_match_idx: usize,
    match_base_idx: usize,
) -> usize {
    debug_assert!(p_in_idx <= history.len());
    debug_assert!(p_match_idx <= history.len());
    debug_assert!(anchor_idx <= p_in_idx);
    debug_assert!(match_base_idx <= p_match_idx);

    let mut p_in = p_in_idx;
    let mut p_match = p_match_idx;
    let mut len = 0usize;
    while p_in > anchor_idx && p_match > match_base_idx && history[p_in - 1] == history[p_match - 1]
    {
        p_in -= 1;
        p_match -= 1;
        len += 1;
    }
    len
}

/// Per-call inputs to [`find_best_match`]. Bundled into a struct
/// so the public function avoids the `clippy::too_many_arguments`
/// trip-wire while keeping each input clearly named (every field
/// has a distinct semantic role; merging would obscure the upstream zstd
/// citations).
///
/// All positional fields are **absolute stream coordinates** —
/// stable across window evictions. `live_history` carries the
/// concrete byte slice corresponding to the absolute range
/// `[history_abs_start, history_abs_start + live_history.len())`;
/// the function performs the abs→slice translation internally so
/// the bucket entries (which `LdmProducer` stores in absolute
/// coordinates by design) remain valid after a window slide.
pub(crate) struct FindBestMatchInputs<'a> {
    /// Live history slice (upstream zstd: `base + dictLimit .. iend`).
    /// `live_history[0]` is the byte at absolute position
    /// `history_abs_start`.
    pub(crate) live_history: &'a [u8],
    /// Absolute stream position of `live_history[0]`. Subtracted
    /// from every absolute position before indexing into the
    /// slice.
    pub(crate) history_abs_start: usize,
    /// Absolute stream position of the candidate window's start.
    /// Upstream zstd `split` (as an absolute index relative to `base`).
    pub(crate) split_abs: usize,
    /// Absolute stream position of the leftmost byte the producer
    /// is still allowed to emit as literal — the previous emitted
    /// match's post-match boundary or the block start at frame
    /// entry. Upstream zstd `anchor`.
    pub(crate) anchor_abs: usize,
    /// **Inclusive** lower bound: entries with absolute `offset <
    /// lowest_index_abs` are stale and rejected, entries with
    /// `offset >= lowest_index_abs` survive. Conceptually the
    /// "lowest still-live absolute position" — the caller passes
    /// `history_abs_start` so a candidate at the very left edge
    /// of the live window (`live_history[0]`, absolute
    /// `history_abs_start`) remains matchable. Stored as `usize`
    /// so the comparison stays valid above the `u32` boundary
    /// (the table itself rebases internally).
    ///
    /// Semantic deviation from upstream zstd `zstd_ldm.c:431` — upstream zstd's
    /// `cur->offset <= lowestIndex` is an exclusive lower bound
    /// where `lowestIndex` itself is rejected. We use the inclusive
    /// form because our internal coordinate space (absolute
    /// stream position, +1-biased relative offset in the
    /// rebase-aware table) already differs from upstream zstd; flipping
    /// the comparison removes the need for a
    /// `history_abs_start.saturating_sub(1)` adjustment at every
    /// callsite.
    pub(crate) lowest_index_abs: usize,
    /// Absolute stream position one past the last byte the
    /// forward match is allowed to reach. Upstream zstd `iend` (the
    /// caller-supplied `block_end_abs`); the forward count is
    /// clamped to `iend_abs - split_abs` bytes so a match cannot
    /// extend past the current block's scan boundary even if
    /// `live_history` happens to contain matching bytes beyond
    /// it. Must satisfy `iend_abs >= split_abs` and
    /// `iend_abs <= history_abs_start + live_history.len()`.
    pub(crate) iend_abs: usize,
    /// Upstream zstd `params->minMatchLength` — forward matches shorter
    /// than this floor are filtered out.
    pub(crate) min_match_length: usize,
}

/// Walk every slot of the bucket associated with `hash_id`, scoring
/// each entry by `forward + backward` match length, and return the
/// best candidate strictly above the upstream zstd's
/// `forward >= min_match_length` floor. Returns `None` when no
/// bucket entry survives the filter.
///
/// Mirrors the per-split inner loop in
/// `ZSTD_ldm_generateSequences_internal` (`zstd_ldm.c:405-466`)
/// prefix-only path. The caller must pre-resolve `hash_id` via
/// [`LdmHashTable::bucket_mask`]. All positions in
/// [`FindBestMatchInputs`] are absolute stream coordinates; the
/// returned [`LdmMatch::match_pos`] is also absolute.
pub(crate) fn find_best_match(
    table: &LdmHashTable,
    hash_id: u32,
    checksum: u32,
    inputs: FindBestMatchInputs<'_>,
) -> Option<LdmMatch> {
    let FindBestMatchInputs {
        live_history,
        history_abs_start,
        split_abs,
        anchor_abs,
        lowest_index_abs,
        iend_abs,
        min_match_length,
    } = inputs;
    debug_assert!(history_abs_start <= split_abs);
    debug_assert!(split_abs <= history_abs_start + live_history.len());
    debug_assert!(anchor_abs <= split_abs);
    debug_assert!(history_abs_start <= anchor_abs);
    debug_assert!(split_abs <= iend_abs);
    debug_assert!(iend_abs <= history_abs_start + live_history.len());

    let bucket = table.bucket(hash_id);
    let mut best: Option<LdmMatch> = None;
    let history_abs_end = history_abs_start + live_history.len();
    // Translate split_abs / iend_abs to indices into `live_history`
    // once; every forward comparison reuses them. `split_idx_end`
    // caps the forward slice so a match cannot extend past the
    // current block's scan boundary even if matching bytes happen
    // to live beyond it — upstream zstd `ZSTD_count(split, pMatch, iend)`
    // is bounded by `iend`.
    let split_idx = split_abs - history_abs_start;
    let split_idx_end = iend_abs - history_abs_start;

    for entry in bucket {
        // Upstream zstd `zstd_ldm.c:431`: skip stale or wrong-checksum
        // entries. `table.resolve` filters the empty-slot
        // sentinel (`entry.offset == 0`) and translates the
        // stored relative offset back to an absolute stream
        // position so the staleness threshold can be compared
        // directly even past the `u32` rebase boundary.
        if entry.checksum != checksum {
            continue;
        }
        let Some(match_abs) = table.resolve(entry) else {
            continue;
        };
        // Inclusive lower bound (see `FindBestMatchInputs::
        // lowest_index_abs` docs). `match_abs >=
        // lowest_index_abs` survives; everything below the live
        // window is filtered.
        if match_abs < lowest_index_abs {
            continue;
        }
        // Out-of-window guard: an entry above `history_abs_end`
        // (caller misuse or a torn write race in a future
        // concurrent caller) would index past `live_history`.
        if match_abs < history_abs_start || match_abs >= history_abs_end {
            continue;
        }
        // Back-reference ordering guard: a back-reference must
        // point at bytes that already exist (`match_abs <
        // split_abs`). In normal producer operation every entry
        // in the bucket was inserted at a position strictly less
        // than the current `split_abs` (the producer's `insert
        // _absolute` runs AFTER `find_best_match`, and the outer
        // loop's `split_abs` increases monotonically), so this
        // check is structurally redundant. Keeping it explicit
        // hardens against future direct callers (custom tables
        // injected via test fixtures, an eventual `extDict`
        // path) where `match_abs == split_abs` would emit
        // `offset = 0` (invalid back-ref) and `match_abs >
        // split_abs` would underflow the unsigned subtraction in
        // the producer's `offset = split_abs − best.match_pos`.
        if match_abs >= split_abs {
            continue;
        }
        let match_idx = match_abs - history_abs_start;

        // Forward match: bytes that compare equal starting from
        // `split` vs `match_pos`. Upstream zstd `ZSTD_count(split, pMatch,
        // iend)` — bounded by `iend`, so we cap the `split` slice
        // at `split_idx_end`. The match slice is bounded only by
        // `live_history.len()` because back-references into the
        // history before the current block are legitimate (upstream zstd
        // `pMatch < iend` is automatically true when the entry's
        // absolute offset is below `iend_abs`).
        let forward_len = common_prefix_len(
            &live_history[split_idx..split_idx_end],
            &live_history[match_idx..],
        );
        if forward_len < min_match_length {
            continue;
        }

        // Backward match: walk left as far as both `anchor`-bound
        // and `low_prefix`-bound (the absolute start of the live
        // history) permit. Upstream zstd `zstd_ldm.c:455-456`.
        let backward_len = count_backwards_match(
            live_history,
            split_idx,
            anchor_abs - history_abs_start,
            match_idx,
            0, // live_history[0] IS the low-prefix pointer in slice coords
        );

        let candidate = LdmMatch {
            match_pos: match_abs,
            forward_len,
            backward_len,
        };
        match best {
            Some(b) if candidate.total_len() <= b.total_len() => {}
            _ => best = Some(candidate),
        }
    }

    best
}
