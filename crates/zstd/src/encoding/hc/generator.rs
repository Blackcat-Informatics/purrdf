#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::inline_always,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! HashChain / binary-tree match generator (`HcMatchGenerator`).
//!
//! The HC chain matcher plus the BT-backed optimal-parse machinery the lazy /
//! btopt / btultra strategies run on: the `HcBackend` discriminator, the
//! `HcMatchGenerator` storage + methods, the `btlazy2` cost helpers, the
//! optimal-plan body macros, and the `build_optimal_plan` driver. Moved
//! verbatim from `match_generator.rs` (no behaviour change); encoding-level
//! paths are absolute (`crate::encoding::…`) for the deeper module.

use alloc::vec::Vec;

use crate::encoding::Sequence;
use crate::encoding::blocks::encode_offset_with_history;
use crate::encoding::cost_model::HC_PREDEF_THRESHOLD;
use crate::encoding::hc::{HC_MIN_MATCH_LEN, HcMatch, MAX_HC_SEARCH_DEPTH};
use crate::encoding::levels::config::HcConfig;
use crate::encoding::match_generator::{HC_SEARCH_DEPTH, HC_TARGET_LEN, HcBackend};
use crate::encoding::match_table::storage::HC3_HASH_LOG;

impl HcBackend {
    /// Heap bytes held by the backend. `Hc` is zero-sized; `Bt` boxes a
    /// `BtMatcher`, so count the boxed payload plus its own scratch heap.
    pub(crate) fn heap_size(&self) -> usize {
        match self {
            Self::Hc => 0,
            Self::Bt(bt) => size_of::<crate::encoding::bt::BtMatcher>() + bt.heap_size(),
        }
    }

    /// Mutable accessor on the BT matcher; panics if the active
    /// backend is `Hc`. The HC-or-Bt branches in orchestrator code use
    /// `let HcBackend::Bt(bt) = &self.backend` directly for readonly
    /// access — this helper exists so macro bodies that already drive
    /// a mutable BT update through the optimal parser can write
    /// `$self.backend.bt_mut().X` without an outer `match` ladder.
    #[inline(always)]
    pub(crate) fn bt_mut(&mut self) -> &mut crate::encoding::bt::BtMatcher {
        match self {
            Self::Bt(bt) => bt,
            Self::Hc => unreachable!("BT-only accessor called in HC mode"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct HcMatchGenerator {
    /// Shared match-finder storage (window, history, hash / chain /
    /// hash3 tables, dictionary-priming flags). Used identically by HC
    /// and BT modes; backend-specific table interpretation lives in the
    /// matcher methods on this struct.
    pub(crate) table: crate::encoding::match_table::storage::MatchTable,
    /// HC runtime knobs (lazy_depth, search_depth, target_len). Always
    /// present — BT modes still consult `hc.search_depth` for repcode
    /// probing and chain candidate enumeration.
    pub(crate) hc: crate::encoding::hc::HcMatcher,
    /// Backend discriminator. [`HcBackend::Hc`] is zero-sized for the
    /// lazy / lazy2 path so HC-only generators don't carry the BT
    /// optimal-parser scratch buffers. [`HcBackend::Bt`] holds the
    /// `BtMatcher` when an optimal mode is configured.
    pub(crate) backend: HcBackend,
    /// Compile-time strategy tag mirrored from
    /// [`MatchGeneratorDriver::strategy_tag`] during `configure()`.
    /// The driver hot path never reads this — it dispatches to
    /// `compress_block::<S>` from its own tag — but the
    /// `#[cfg(test)] start_matching` helper consumes it so artificial
    /// test setups still pick the correct concrete `S` for the
    /// const-generic optimal parser (BtOpt vs BtUltra vs BtUltra2).
    /// Without this field the test path would have to collapse
    /// `BtOpt` and `BtUltra` onto the same monomorphisation since
    /// `table.uses_bt` / `table.is_btultra2` alone can't tell them
    /// apart.
    pub(crate) strategy_tag: crate::encoding::strategy::StrategyTag,
}

// Plain-data types relocated to [`crate::encoding::opt::types`] and
// [`crate::encoding::opt::ldm`]. The use statements at
// the top of this file bring them back into scope so the existing
// methods on `HcMatchGenerator` compile unchanged.

/// `bt_insert_step_no_rebase` body parameterized over the per-CPU
/// `count_match_from_indices` symbol. Each kernel-specific wrapper invokes
/// the macro with its own `fastpath::<kernel>::count_match_from_indices`
/// path so the call resolves inside the wrapper's `#[target_feature]`
/// umbrella and inlines instead of paying the function-call ABI per BT walk
/// iteration. Used only by `HcMatchGenerator` BT walk wrappers below.
///
/// Crate-private: the macro body references private `encoding::*`
/// modules via `$crate::...`, so it is unusable downstream and is
/// re-exported only inside this crate via `pub(crate) use` below.
macro_rules! bt_insert_step_no_rebase_body {
    ($table:expr, $search_depth:expr, $abs_pos:ident, $current_abs_end:ident, $target_abs:ident, $cmf:path) => {{
        let idx = $abs_pos - $table.history_abs_start;
        // Borrowed-aware live region (owned: `history[history_start..]`;
        // borrowed: the in-place input `[0, block_end)`). Reborrow-then-raw-ptr
        // so the slice holds NO borrow and coexists with the `&mut $table`
        // binary-tree writes below. Owned is byte-identical (same bytes).
        let concat: &[u8] = unsafe {
            let lh = $table.live_history();
            core::slice::from_raw_parts(lh.as_ptr(), lh.len())
        };
        if idx + 8 > concat.len() {
            return 1;
        }
        debug_assert!(
            $abs_pos <= $current_abs_end,
            "BT walker called past current block end"
        );
        let tail_limit = $current_abs_end - $abs_pos;
        let hash = $crate::encoding::match_table::storage::MatchTable::hash_position_at(
            concat,
            idx,
            $table.hash_log,
            $table.search_mls,
        );
        // Prefetch the hash bucket now. For the large L16+ hash table over
        // high-entropy input the bucket is L3/DRAM-cold, and unlike upstream's
        // monolithic ZSTD_btGetAllMatches (which overlaps this miss with its
        // inline rep/hash3 prologue) the read+write of `hash_table[hash]`
        // below is reached with nothing to hide it behind — it stalled a large
        // share of this function's cycles. Issuing the hint here lets the miss
        // overlap the address setup that follows.
        #[cfg(all(
            target_feature = "sse",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        {
            #[cfg(target_arch = "x86")]
            use core::arch::x86::{_MM_HINT_T0, _mm_prefetch};
            #[cfg(target_arch = "x86_64")]
            use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            // SAFETY: prefetch is a hint that never faults; `hash` indexes
            // `hash_table` directly below, so it is in bounds.
            unsafe {
                _mm_prefetch($table.hash_table.as_ptr().add(hash).cast(), _MM_HINT_T0);
            }
            // Prefetch the NEXT position's bucket too. The optimal-parser DP
            // advances one position per iteration, so this miss is issued a
            // full BT walk plus the next iteration's pre-collect work ahead of
            // the collect that will read it — far more lead than the same-call
            // hint above, enough to hide the full DRAM latency.
            if idx + 1 + 8 <= concat.len() {
                let hash_next =
                    $crate::encoding::match_table::storage::MatchTable::hash_position_at(
                        concat,
                        idx + 1,
                        $table.hash_log,
                        $table.search_mls,
                    );
                // SAFETY: prefetch never faults; an out-of-range index is a
                // harmless no-op hint.
                unsafe {
                    _mm_prefetch(
                        $table.hash_table.as_ptr().add(hash_next).cast(),
                        _MM_HINT_T0,
                    );
                }
            }
        }
        let Some(relative_pos) = $table.relative_position($abs_pos) else {
            return 1;
        };
        let stored = relative_pos + 1;
        let bt_mask = $table.bt_mask();
        // `abs_pos < bt_mask` legitimately happens for the first BT walk of
        // a fresh frame (bt_low effectively "no floor"). Saturating keeps
        // the floor at 0 so the `candidate_abs <= bt_low` check never
        // triggers early; raw subtraction would underflow into a huge
        // sentinel that ALWAYS triggers.
        let bt_low = $abs_pos.saturating_sub(bt_mask);
        // Hoist the BT pointer-pair base out of `self` once — see the
        // collect-matches body for the full rationale (per-step Vec reload +
        // bounds check through `&mut self` vs the upstream zstd's raw `U32*` walk).
        let chain_ptr = $table.chain_table.as_mut_ptr();
        debug_assert_eq!($table.chain_table.len(), 2 << $table.bt_log());
        let window_low = $table.window_low_abs_for_target($target_abs);
        // `abs_pos + 9` is safe in raw form: `MatchTable::add_data` caps
        // total input at `usize::MAX - STREAM_ABS_HEADROOM` (where
        // `STREAM_ABS_HEADROOM = HC_OPT_NUM + 16`), so every
        // frame-lifetime absolute cursor passed to the BT walker stays
        // below `usize::MAX - 9` regardless of stream length or
        // pointer width. The guard is hoisted to the data-ingest
        // boundary so this per-position site pays zero arithmetic
        // overhead in the hot loop.
        let mut match_end_abs = $abs_pos + 9;
        let mut best_len = 8usize;
        let mut compares_left = $search_depth;
        let mut common_length_smaller = 0usize;
        let mut common_length_larger = 0usize;
        let pair_idx = $table.bt_pair_index_for_abs($abs_pos);
        let mut smaller_slot = pair_idx;
        let mut larger_slot = pair_idx + 1;
        let mut match_stored = $table.hash_table[hash];
        $table.hash_table[hash] = stored;

        while compares_left > 0 {
            if match_stored == $crate::encoding::match_table::storage::HC_EMPTY {
                break;
            }
            // Reject stale post-rebase slots whose pre-shift position is below
            // `index_shift` explicitly. A `wrapping_sub` maps such a slot to a
            // near-`usize::MAX` value that the `>= abs_pos` test only rejects
            // while `abs_pos` is far from the integer ceiling; on a
            // long-running rebased stream (reachable on 32-bit) `abs_pos` can
            // approach the ceiling and the wrapped value can land back inside
            // `[window_low, abs_pos)`. `checked_sub` ends the walk on the
            // underflow instead. `match_stored != HC_EMPTY` here, so the `- 1`
            // cannot underflow.
            let Some(candidate_abs) = ($table.position_base + (match_stored as usize - 1))
                .checked_sub($table.index_shift)
            else {
                break;
            };
            if candidate_abs < window_low || candidate_abs >= $abs_pos {
                break;
            }
            compares_left -= 1;

            let next_pair_idx = $table.bt_pair_index_for_abs(candidate_abs);
            // SAFETY: `next_pair_idx (+1)` = `2*(candidate_abs & bt_mask) (+1)`
            // ≤ `chain_table.len()-1`; `chain_ptr` is the hoisted live base,
            // table not realloc'd during the walk.
            let next_smaller = unsafe { *chain_ptr.add(next_pair_idx) };
            let next_larger = unsafe { *chain_ptr.add(next_pair_idx + 1) };
            let seed_len = common_length_smaller.min(common_length_larger);
            let candidate_idx = candidate_abs - $table.history_abs_start;
            // SAFETY: BT walk invariant — `candidate_idx + tail_limit ≤
            // concat.len()` since the candidate is within
            // `[history_abs_start, abs_pos)` and `tail_limit ≤
            // current_abs_end - abs_pos`.
            let match_len = unsafe { $cmf(concat, idx, candidate_idx, tail_limit, seed_len) };

            if match_len > best_len {
                best_len = match_len;
                // `candidate_abs + match_len <= current_abs_end` by BT walk
                // invariant — `match_len <= tail_limit = current_abs_end -
                // abs_pos` and `candidate_abs < abs_pos`.
                let candidate_end = candidate_abs + match_len;
                if candidate_end > match_end_abs {
                    match_end_abs = candidate_end;
                }
            }

            if match_len >= tail_limit {
                break;
            }

            let candidate_next = candidate_idx + match_len;
            let current_next = idx + match_len;
            // SAFETY: first-differing positions after a match_len-long prefix;
            // match_len < tail_limit (break above) + BT-walk bound
            // idx/candidate_idx + tail_limit <= concat.len() keep both in range.
            if unsafe {
                *concat.get_unchecked(candidate_next) < *concat.get_unchecked(current_next)
            } {
                // SAFETY: `smaller_slot` holds a valid pair index (init
                // `pair_idx`, updated to `next_pair_idx + 1`); the `usize::MAX`
                // sentinel is set only just before `break`, never written here.
                unsafe { *chain_ptr.add(smaller_slot) = match_stored };
                common_length_smaller = match_len;
                if candidate_abs <= bt_low {
                    smaller_slot = usize::MAX;
                    break;
                }
                smaller_slot = next_pair_idx + 1;
                match_stored = next_larger;
            } else {
                // SAFETY: as above for `larger_slot`.
                unsafe { *chain_ptr.add(larger_slot) = match_stored };
                common_length_larger = match_len;
                if candidate_abs <= bt_low {
                    larger_slot = usize::MAX;
                    break;
                }
                larger_slot = next_pair_idx;
                match_stored = next_smaller;
            }
        }

        // SAFETY: both slots, when not the `usize::MAX` sentinel, hold valid
        // pair indices into the hoisted `chain_table` base.
        if smaller_slot != usize::MAX {
            unsafe {
                *chain_ptr.add(smaller_slot) = $crate::encoding::match_table::storage::HC_EMPTY
            };
        }
        if larger_slot != usize::MAX {
            unsafe {
                *chain_ptr.add(larger_slot) = $crate::encoding::match_table::storage::HC_EMPTY
            };
        }

        let speed_positions = if best_len > 384 {
            (best_len - 384).min(192)
        } else {
            0
        };
        // `match_end_abs` is initialized to `abs_pos + 9` and is only
        // reassigned inside the `candidate_end > match_end_abs` branch
        // above. So even though an individual `candidate_end =
        // candidate_abs + match_len` can land below `abs_pos` (the
        // candidate sits earlier in history and the match runs short),
        // the variable itself never drops below its initial value.
        // That gives `match_end_abs ≥ abs_pos + 9 > abs_pos + 8` as a
        // loop-wide invariant, so the raw subtraction below cannot
        // underflow.
        speed_positions.max(match_end_abs - ($abs_pos + 8))
    }};
}
pub(crate) use bt_insert_step_no_rebase_body;

/// `hash3_candidate` body parameterized over the per-CPU
/// `common_prefix_len_ptr` symbol. The hash3 probe checks one candidate per
/// position when invoked, so the per-call ABI savings compound across the
/// segment. Crate-private (see `bt_insert_step_no_rebase_body!`).
macro_rules! hash3_candidate_body {
    (
        $table:expr,
        $abs_pos:ident,
        $current_abs_end:ident,
        $min_match_len:ident,
        $cpl:path $(,)?
    ) => {{
        if $table.hash3_log == 0 {
            return None;
        }
        let idx = $abs_pos.checked_sub($table.history_abs_start)?;
        let concat = $table.live_history();
        if idx + 4 > concat.len() {
            return None;
        }
        let hash3 = $crate::encoding::match_table::storage::MatchTable::hash_position_at(
            concat,
            idx,
            $table.hash3_log,
            3,
        );
        let entry = $table
            .hash3_table
            .get(hash3)
            .copied()
            .unwrap_or($crate::encoding::match_table::storage::HC_EMPTY);
        let candidate_abs =
            $crate::encoding::match_table::storage::MatchTable::stored_abs_position_fast(
                entry,
                $table.position_base,
                $table.index_shift,
            )?;
        if candidate_abs < $table.history_abs_start || candidate_abs >= $abs_pos {
            return None;
        }
        let offset = $abs_pos - candidate_abs;
        if offset >= $crate::encoding::bt::HC3_MAX_OFFSET {
            return None;
        }
        let candidate_idx = candidate_abs - $table.history_abs_start;
        let tail_limit = $current_abs_end.saturating_sub($abs_pos);
        let base = concat.as_ptr();
        // SAFETY: candidate/idx are within history range; tail_limit
        // bounds the scan within `concat`.
        let match_len = unsafe { $cpl(base.add(candidate_idx), base.add(idx), tail_limit) };
        (match_len >= $min_match_len).then_some($crate::encoding::opt::types::MatchCandidate {
            start: $abs_pos,
            offset,
            match_len,
        })
    }};
}
pub(crate) use hash3_candidate_body;

/// `for_each_repcode_candidate_with_reps` body parameterized over the per-CPU
/// `common_prefix_len_ptr` symbol so the per-rep prefix probe inlines under
/// the wrapper's `target_feature` umbrella instead of crossing the ABI
/// boundary through the dispatcher. Three rep probes per encoded position →
/// thousands per segment, so the per-call barrier was non-trivial.
///
/// The callback `f` runs in the wrapper's umbrella context too, so closures
/// that capture mutable state still work (FnMut). Crate-private
/// (see `bt_insert_step_no_rebase_body!`).
macro_rules! for_each_repcode_candidate_body {
    (
        $table:expr,
        $abs_pos:ident,
        $lit_len:ident,
        $reps:ident,
        $current_abs_end:ident,
        $min_match_len:ident,
        $f:ident,
        $cpl:path $(,)?
    ) => {{
        let rep_offsets: [Option<usize>; 3] = if $lit_len == 0 {
            [
                Some($reps[1] as usize),
                Some($reps[2] as usize),
                ($reps[0] > 1).then_some(($reps[0] - 1) as usize),
            ]
        } else {
            [
                Some($reps[0] as usize),
                Some($reps[1] as usize),
                Some($reps[2] as usize),
            ]
        };
        let concat = $table.live_history();
        let current_idx = $abs_pos - $table.history_abs_start;
        if current_idx + 4 > concat.len() {
            return;
        }
        let tail_limit = $current_abs_end.saturating_sub($abs_pos);
        let base = concat.as_ptr();
        let concat_len = concat.len();
        for rep in rep_offsets.into_iter().flatten() {
            if rep == 0 || rep > $abs_pos {
                continue;
            }
            let candidate_pos = $abs_pos - rep;
            if candidate_pos < $table.history_abs_start {
                continue;
            }
            let candidate_idx = candidate_pos - $table.history_abs_start;
            // Upstream zstd `ZSTD_readMINMATCH` gate (zstd_opt.c:657-674): a
            // 4-byte (3-byte when min_match_len == 3) equality probe
            // before the full prefix scan. Equivalent filtering — a
            // mismatch here means `match_len < min_match_len`, which
            // the post-scan check rejects anyway — but it skips the
            // prefix-kernel call for the common no-match case (rep
            // offsets rarely hit on low-redundancy input).
            //
            // SAFETY: `current_idx + 4 <= concat_len` (early return
            // above) and `candidate_idx < current_idx` (rep >= 1), so
            // both 4-byte reads stay inside `concat`.
            let gate_matches = unsafe {
                let cand = base.add(candidate_idx).cast::<u32>().read_unaligned();
                let cur = base.add(current_idx).cast::<u32>().read_unaligned();
                if $min_match_len == 3 {
                    // Compare the low-address 3 bytes regardless of
                    // endianness: byte-shift on LE, mask via to_le.
                    (cand.to_le() & 0x00FF_FFFF) == (cur.to_le() & 0x00FF_FFFF)
                } else {
                    cand == cur
                }
            };
            if !gate_matches {
                continue;
            }
            // SAFETY: `candidate_idx ≤ current_idx < concat_len` (since
            // candidate_pos ≤ abs_pos and we early-returned on
            // `current_idx + 4 > concat_len`). `max` clamps to the shorter
            // remaining run so neither pointer overruns `concat`.
            let max = (concat_len - candidate_idx)
                .min(concat_len - current_idx)
                .min(tail_limit);
            let match_len = unsafe { $cpl(base.add(candidate_idx), base.add(current_idx), max) };
            if match_len < $min_match_len {
                continue;
            }
            $f(MatchCandidate {
                start: $abs_pos,
                offset: rep,
                match_len,
            });
        }
    }};
}
pub(crate) use for_each_repcode_candidate_body;

/// `bt_insert_and_collect_matches` body parameterized over the per-CPU
/// `count_match_from_indices` symbol. Same shape as
/// [`bt_insert_step_no_rebase_body`] — picks up the matching kernel through
/// `$cmf` so the per-iteration vector probe inlines under the wrapper's
/// `target_feature` umbrella. Returns nothing (matches the original method).
/// Crate-private (see `bt_insert_step_no_rebase_body!`).
macro_rules! bt_insert_and_collect_matches_body {
    (
        $table:expr,
        $search_depth:expr,
        $abs_pos:ident,
        $current_abs_end:ident,
        $profile:ident,
        $min_match_len:ident,
        $best_len_for_skip:ident,
        $out:ident,
        $reps:ident,
        $lit_len:ident,
        $use_hash3:expr,
        $cpl:path,
        $cmf:path $(,)?
    ) => {{
        let idx = $abs_pos - $table.history_abs_start;
        // Borrowed-aware live region (owned: `history[history_start..]`;
        // borrowed: the in-place input `[0, block_end)`). Reborrow-then-raw-ptr
        // so the slice holds NO borrow and coexists with the `&mut $table`
        // binary-tree writes below. Owned is byte-identical (same bytes).
        let concat: &[u8] = unsafe {
            let lh = $table.live_history();
            core::slice::from_raw_parts(lh.as_ptr(), lh.len())
        };
        debug_assert!(
            $abs_pos <= $current_abs_end,
            "BT collect called past current block end"
        );
        let tail_limit = $current_abs_end - $abs_pos;
        // ===== Merged rep-code + hash3 probes (reshape toward upstream
        // ZSTD_btGetAllMatches: rep + hash3 + BT walk in ONE out-of-line frame).
        // Folded here from the prior separate per-position calls in
        // collect_optimal_candidates_initialized_body!. Probe order
        // (rep -> hash3 -> BT walk) + best_len_for_skip seeding are byte-identical
        // to the previous orchestration. Out-of-line keeps register pressure in
        // this frame (inlining the same probes into the DP caller regressed). =====
        let mut skip_further_match_search = false;
        let mut rep_len_candidate_found = false;
        if idx + 4 <= concat.len() {
            let rep_offsets: [Option<usize>; 3] = if $lit_len == 0 {
                [
                    Some($reps[1] as usize),
                    Some($reps[2] as usize),
                    ($reps[0] > 1).then_some(($reps[0] - 1) as usize),
                ]
            } else {
                [
                    Some($reps[0] as usize),
                    Some($reps[1] as usize),
                    Some($reps[2] as usize),
                ]
            };
            let rbase = concat.as_ptr();
            let rlen = concat.len();
            for rep in rep_offsets.into_iter().flatten() {
                if rep == 0 || rep > $abs_pos {
                    continue;
                }
                let candidate_pos = $abs_pos - rep;
                if candidate_pos < $table.history_abs_start {
                    continue;
                }
                let candidate_idx = candidate_pos - $table.history_abs_start;
                // SAFETY: `idx + 4 <= rlen` (guard above) and `candidate_idx < idx`
                // (rep >= 1), so both 4-byte reads stay inside `concat`.
                let gate_matches = unsafe {
                    let cand = rbase.add(candidate_idx).cast::<u32>().read_unaligned();
                    let cur = rbase.add(idx).cast::<u32>().read_unaligned();
                    if $min_match_len == 3 {
                        (cand.to_le() & 0x00FF_FFFF) == (cur.to_le() & 0x00FF_FFFF)
                    } else {
                        cand == cur
                    }
                };
                if !gate_matches {
                    continue;
                }
                let rmax = (rlen - candidate_idx).min(rlen - idx).min(tail_limit);
                // SAFETY: same umbrella; both pointers + `rmax` stay in `concat`.
                let match_len = unsafe { $cpl(rbase.add(candidate_idx), rbase.add(idx), rmax) };
                if match_len < $min_match_len {
                    continue;
                }
                rep_len_candidate_found = true;
                let _ = $crate::encoding::bt::BtMatcher::push_candidate_ladder(
                    $out,
                    $best_len_for_skip,
                    $crate::encoding::opt::types::MatchCandidate {
                        start: $abs_pos,
                        offset: rep,
                        match_len,
                    },
                    $min_match_len,
                );
                if match_len > $profile.sufficient_match_len
                    || $abs_pos + match_len >= $current_abs_end
                {
                    skip_further_match_search = true;
                }
            }
        }
        if $use_hash3 && !skip_further_match_search && *$best_len_for_skip < $min_match_len {
            $table.update_hash3_until($abs_pos);
            // hash3 short-match probe folded inline (was a separate per-kernel
            // call): table lookup + one common-prefix scan via `$cpl`, reusing
            // the BT collect's `concat` / `idx` / `tail_limit`. Labeled block so
            // the probe's early-outs yield None without returning from the walk.
            let h3_candidate: Option<$crate::encoding::opt::types::MatchCandidate> =
                if $table.hash3_log == 0 || idx + 4 > concat.len() {
                    None
                } else {
                    'h3: {
                        let hh =
                            $crate::encoding::match_table::storage::MatchTable::hash_position_at(
                                concat,
                                idx,
                                $table.hash3_log,
                                3,
                            );
                        let entry = $table
                            .hash3_table
                            .get(hh)
                            .copied()
                            .unwrap_or($crate::encoding::match_table::storage::HC_EMPTY);
                        let Some(cand_abs) =
                            $crate::encoding::match_table::storage::MatchTable::stored_abs_position_fast(
                                entry,
                                $table.position_base,
                                $table.index_shift,
                            )
                        else {
                            break 'h3 None;
                        };
                        if cand_abs < $table.history_abs_start || cand_abs >= $abs_pos {
                            break 'h3 None;
                        }
                        let off = $abs_pos - cand_abs;
                        if off >= $crate::encoding::bt::HC3_MAX_OFFSET {
                            break 'h3 None;
                        }
                        let cand_idx = cand_abs - $table.history_abs_start;
                        let hbase = concat.as_ptr();
                        // SAFETY: cand_idx/idx within history; tail_limit bounds the scan.
                        let ml = unsafe { $cpl(hbase.add(cand_idx), hbase.add(idx), tail_limit) };
                        (ml >= $min_match_len).then_some(
                            $crate::encoding::opt::types::MatchCandidate {
                                start: $abs_pos,
                                offset: off,
                                match_len: ml,
                            },
                        )
                    }
                };
            if let Some(h3) = h3_candidate {
                let _ = $crate::encoding::bt::BtMatcher::push_candidate_ladder(
                    $out,
                    $best_len_for_skip,
                    h3,
                    $min_match_len,
                );
                if !rep_len_candidate_found
                    && (h3.match_len > $profile.sufficient_match_len
                        || $abs_pos + h3.match_len >= $current_abs_end)
                {
                    $table.skip_insert_until_abs = $abs_pos + 1;
                    skip_further_match_search = true;
                }
            }
        }
        if skip_further_match_search {
            return;
        }
        if idx + 8 > concat.len() {
            return;
        }
        let hash = $crate::encoding::match_table::storage::MatchTable::hash_position_at(
            concat,
            idx,
            $table.hash_log,
            $table.search_mls,
        );
        // Prefetch the hash bucket now. For the large L16+ hash table over
        // high-entropy input the bucket is L3/DRAM-cold, and unlike upstream's
        // monolithic ZSTD_btGetAllMatches (which overlaps this miss with its
        // inline rep/hash3 prologue) the read+write of `hash_table[hash]`
        // below is reached with nothing to hide it behind — it stalled a large
        // share of this function's cycles. Issuing the hint here lets the miss
        // overlap the address setup that follows.
        #[cfg(all(target_feature = "sse", any(target_arch = "x86", target_arch = "x86_64")))]
        {
            #[cfg(target_arch = "x86")]
            use core::arch::x86::{_MM_HINT_T0, _mm_prefetch};
            #[cfg(target_arch = "x86_64")]
            use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            // SAFETY: prefetch is a hint that never faults; `hash` indexes
            // `hash_table` directly below, so it is in bounds.
            unsafe {
                _mm_prefetch($table.hash_table.as_ptr().add(hash).cast(), _MM_HINT_T0);
            }
            // Prefetch the NEXT position's bucket too. The optimal-parser DP
            // advances one position per iteration, so this miss is issued a
            // full BT walk plus the next iteration's pre-collect work ahead of
            // the collect that will read it — far more lead than the same-call
            // hint above, enough to hide the full DRAM latency.
            if idx + 1 + 8 <= concat.len() {
                let hash_next =
                    $crate::encoding::match_table::storage::MatchTable::hash_position_at(
                        concat,
                        idx + 1,
                        $table.hash_log,
                        $table.search_mls,
                    );
                // SAFETY: prefetch never faults; an out-of-range index is a
                // harmless no-op hint.
                unsafe {
                    _mm_prefetch(
                        $table.hash_table.as_ptr().add(hash_next).cast(),
                        _MM_HINT_T0,
                    );
                }
            }
        }
        let Some(relative_pos) = $table.relative_position($abs_pos) else {
            return;
        };
        let stored = relative_pos + 1;
        let bt_mask = $table.bt_mask();
        // Hoist the BT pointer-pair table's base out of `self` once: every
        // access below is `chain_table[computed_index]` through `&mut self`,
        // which the optimizer cannot prove loop-invariant, so it reloads the
        // Vec's (ptr,len) from the struct AND bounds-checks on every tree
        // step (the upstream zstd walks a raw `U32* btable`, zstd_opt.c). The raw
        // base carries no borrow, so the `&self` helper calls in the loop
        // (`bt_pair_index_for_abs`, `window_low_abs_for_target`,
        // `relative_position`) coexist — they read other fields, never
        // `chain_table`. Indices are in bounds by the BT invariants:
        // `bt_pair_index_for_abs` returns `2*(abs & bt_mask) (+1)` ≤
        // `chain_table.len()-1`, and the slots only ever hold those values.
        let chain_ptr = $table.chain_table.as_mut_ptr();
        debug_assert_eq!($table.chain_table.len(), 2 << $table.bt_log());
        // See `bt_insert_step_no_rebase_body!`: saturating is needed for the
        // first BT walk of a fresh frame where `abs_pos < bt_mask`.
        let bt_low = $abs_pos.saturating_sub(bt_mask);
        let window_low = $table.window_low_abs_for_target($abs_pos);
        // Upstream zstd-style window bound in stored space so the BT-walk loop
        // condition rejects out-of-window / HC_EMPTY candidates WITHOUT
        // decoding them (mirrors upstream `while ... matchIndex >= matchLow`):
        // one range check on `match_stored` instead of decode-then-break,
        // dropping the wasted candidate_abs decode on every walk's terminating
        // step. candidate_abs(s) = (position_base + s - 1) - index_shift =
        // base + s (wrapping); in-window ⟺ candidate_abs - window_low <
        // abs_pos - window_low ⟺ s.wrapping_add(win_off) < win_range.
        // HC_EMPTY (s = 0) maps to base = (lowest representable abs) - 1 <
        // window_low, so it falls out of range and ends the walk.
        let win_off = $table
            .position_base
            .wrapping_sub(1)
            .wrapping_sub($table.index_shift)
            .wrapping_sub(window_low);
        let win_range = $abs_pos - window_low;
        // Decode biases: fold the per-node coordinate conversions into
        // loop-invariant additions. The gate-validated chain entry
        // `match_stored` maps to its absolute position, its history-relative
        // index and its BT pair slot by a single add each, instead of
        // re-reading `position_base` / `index_shift` / `history_abs_start` /
        // `bt_mask` from `self` and round-tripping `index_shift` through the
        // slot computation on every node. Upstream zstd walks one
        // window-relative `matchIndex` directly (`match = base + matchIndex`,
        // slot `2*(matchIndex & btMask)`); these biases are the
        // single-coordinate equivalent. Wrapping throughout: the window gate
        // already proved `match_stored ∈ [window_low, abs_pos)` before decode,
        // mirroring the `win_off` form above.
        let abs_bias = $table
            .position_base
            .wrapping_sub(1)
            .wrapping_sub($table.index_shift);
        let idx_bias = abs_bias.wrapping_sub($table.history_abs_start);
        let bt_bias = $table.position_base.wrapping_sub(1);
        // Raw `+ 9` is safe here — see `bt_insert_step_no_rebase_body!`
        // for the full discussion of the upstream `STREAM_ABS_HEADROOM`
        // cap in `MatchTable::add_data`.
        let mut match_end_abs = $abs_pos + 9;
        let mut compares_left = $profile.max_chain_depth.min($search_depth);
        let mut common_length_smaller = 0usize;
        let mut common_length_larger = 0usize;
        let pair_idx = $table.bt_pair_index_for_abs($abs_pos);
        let mut smaller_slot = pair_idx;
        let mut larger_slot = pair_idx + 1;
        let mut match_stored = $table.hash_table[hash];
        $table.hash_table[hash] = stored;
        // Upstream zstd semantics: `bestLength` starts at `lengthToBeat - 1`; rep/hash3
        // probing may raise it; BT then only reports strictly longer matches.
        // `min_match_len >= HC_FORMAT_MINMATCH (3)` by configure invariant,
        // so `min_match_len - 1 >= 2` cannot underflow.
        debug_assert!(
            $min_match_len >= $crate::encoding::cost_model::HC_FORMAT_MINMATCH,
            "min_match_len must be at least HC_FORMAT_MINMATCH"
        );
        let mut best_len = (*$best_len_for_skip).max($min_match_len - 1);

        // Upstream zstd-form loop condition: the stored-space window range check
        // (`s.wrapping_add(win_off) < win_range`) rejects out-of-window and
        // HC_EMPTY candidates here, so the terminating step never enters the
        // body — no wasted candidate_abs decode, matching upstream's
        // `while ... matchIndex >= matchLow`.
        while compares_left > 0 && (match_stored as usize).wrapping_add(win_off) < win_range {
            compares_left -= 1;
            // The condition proved this candidate is in `[window_low,
            // abs_pos)`, so `match_stored >= 1` (HC_EMPTY is out of range).
            // Decode via the precomputed biases (single add each), the
            // single-coordinate form of upstream's `matchIndex`.
            let stored = match_stored as usize;
            let candidate_abs = stored.wrapping_add(abs_bias);
            // `2*(candidate_abs + index_shift & bt_mask)` with `index_shift`
            // folded away: `candidate_abs + index_shift == stored + bt_bias`.
            let next_pair_idx = 2 * (stored.wrapping_add(bt_bias) & bt_mask);
            // SAFETY: `next_pair_idx (+1)` = `2*(candidate_abs & bt_mask) (+1)`
            // ≤ `chain_table.len()-1`; `chain_ptr` is the hoisted live base,
            // table not realloc'd during the walk.
            let next_smaller = unsafe { *chain_ptr.add(next_pair_idx) };
            let next_larger = unsafe { *chain_ptr.add(next_pair_idx + 1) };
            let seed_len = common_length_smaller.min(common_length_larger);
            let candidate_idx = stored.wrapping_add(idx_bias);
            // SAFETY: BT walk invariant — `candidate_idx + tail_limit ≤
            // concat.len()`.
            let match_len = unsafe { $cmf(concat, idx, candidate_idx, tail_limit, seed_len) };

            if match_len > best_len {
                let offset = $abs_pos - candidate_abs;
                let accepted = $crate::encoding::bt::BtMatcher::push_candidate_ladder(
                    $out,
                    $best_len_for_skip,
                    $crate::encoding::opt::types::MatchCandidate {
                        start: $abs_pos,
                        offset,
                        match_len,
                    },
                    $min_match_len,
                );
                if accepted {
                    best_len = match_len;
                    // BT walker invariants: `candidate_abs < abs_pos`
                    // and `match_len <= tail_limit = current_abs_end -
                    // abs_pos`. So `candidate_abs + match_len <
                    // abs_pos + tail_limit = current_abs_end`, which
                    // fits in `usize` on every supported target (32-bit
                    // i686 included) — the addition stays within the
                    // current block.
                    let candidate_end = candidate_abs + match_len;
                    if candidate_end > match_end_abs {
                        match_end_abs = candidate_end;
                    }
                    if match_len >= tail_limit
                        || match_len > $crate::encoding::cost_model::HC_OPT_NUM
                    {
                        break;
                    }
                }
            }

            if match_len >= tail_limit {
                break;
            }

            let candidate_next = candidate_idx + match_len;
            let current_next = idx + match_len;
            // SAFETY: first-differing positions after a match_len-long prefix;
            // match_len < tail_limit (break above) + BT-walk bound
            // idx/candidate_idx + tail_limit <= concat.len() keep both in range.
            if unsafe {
                *concat.get_unchecked(candidate_next) < *concat.get_unchecked(current_next)
            } {
                // SAFETY: `smaller_slot` holds a valid pair index (init
                // `pair_idx`, updated to `next_pair_idx + 1`); the `usize::MAX`
                // sentinel is set only just before `break`, never written here.
                unsafe { *chain_ptr.add(smaller_slot) = match_stored };
                common_length_smaller = match_len;
                if candidate_abs <= bt_low {
                    smaller_slot = usize::MAX;
                    break;
                }
                smaller_slot = next_pair_idx + 1;
                match_stored = next_larger;
            } else {
                // SAFETY: as above for `larger_slot`.
                unsafe { *chain_ptr.add(larger_slot) = match_stored };
                common_length_larger = match_len;
                if candidate_abs <= bt_low {
                    larger_slot = usize::MAX;
                    break;
                }
                larger_slot = next_pair_idx;
                match_stored = next_smaller;
            }
        }

        // SAFETY: both slots, when not the `usize::MAX` sentinel, hold valid
        // pair indices into the hoisted `chain_table` base.
        if smaller_slot != usize::MAX {
            unsafe {
                *chain_ptr.add(smaller_slot) = $crate::encoding::match_table::storage::HC_EMPTY
            };
        }
        if larger_slot != usize::MAX {
            unsafe {
                *chain_ptr.add(larger_slot) = $crate::encoding::match_table::storage::HC_EMPTY
            };
        }

        // Dict dual-probe (upstream zstd `ZSTD_dictMatchState`, zstd_opt.c:777-813):
        // after the live tree, descend the immutable dictionary BINARY TREE
        // (built in `prime_dms_bt`) with its OWN compare budget and push any
        // dict match longer than the live best into the ladder. The DUBT
        // descent reaches the longest dict match efficiently (a hash-chain
        // surfaced only the few same-bucket candidates and left most of the
        // dict savings unrealised at btlazy2 / btopt). Dict positions are
        // dictionary-relative concat indices in `[0, region)`, pinned at the
        // front of history, so a dict candidate at `dict_idx` sits at offset
        // `idx - dict_idx` (no upstream zstd `dmsIndexDelta`). The optimal parser
        // prices these (its DP lookahead values the repcode chain a dict match
        // seeds); the greedy/lazy parser commits the longest.
        // `is_primed()` is the canonical "DMS table is valid to walk" predicate
        // (set only after a full build, cleared by `invalidate`): gate on it so
        // a present-but-stale table is never descended. `is_primed()` implies
        // the table exists (`mark_primed` requires `Some`), so `expect` holds.
        if compares_left > 0 && $table.dms.is_primed() {
            let dms = $table.dms.table().expect("is_primed() guarantees a DMS table");
            let region = $table.dms.region_len();
            let dh = $crate::encoding::match_table::storage::MatchTable::hash_position_at(
                concat,
                idx,
                dms.hash_log,
                dms.mls,
            );
            let mut dcur = dms.hash_table[dh];
            // DUBT seed lengths: bytes already known common on each side, so
            // `$cmf` resumes from there (upstream zstd commonLengthSmaller/Larger).
            let mut common_smaller = 0usize;
            let mut common_larger = 0usize;
            // Shared compare budget (upstream zstd zstd_opt.c:723/781/786): the
            // dict DUBT descent consumes the SAME `compares_left` the live BT
            // walk left over (and is skipped entirely once it is zero, via the
            // `.filter(|_| compares_left > 0)` on the dict table above), instead
            // of getting a fresh full budget. On a match-dense / populated live
            // tree this cuts the cold dict-region descent the way C does.
            while compares_left > 0 && dcur != $crate::encoding::match_table::storage::HC_EMPTY {
                let dict_idx = (dcur - 1) as usize;
                // The dict tree holds only dict positions (`< region <= idx`).
                if dict_idx >= region || dict_idx >= idx {
                    break;
                }
                compares_left -= 1;
                let pair = 2 * dict_idx;
                let seed = common_smaller.min(common_larger);
                // SAFETY: `dict_idx < idx` and `idx + tail_limit <=
                // concat.len()` (checked at entry); same umbrella as the live
                // walk's `$cmf`. `seed <= prior match_len <= tail_limit`.
                let match_len = unsafe { $cmf(concat, idx, dict_idx, tail_limit, seed) };
                if match_len > best_len {
                    let offset = idx - dict_idx;
                    let accepted = $crate::encoding::bt::BtMatcher::push_candidate_ladder(
                        $out,
                        $best_len_for_skip,
                        $crate::encoding::opt::types::MatchCandidate {
                            start: $abs_pos,
                            offset,
                            match_len,
                        },
                        $min_match_len,
                    );
                    if accepted {
                        best_len = match_len;
                        let candidate_end = $abs_pos + match_len;
                        if candidate_end > match_end_abs {
                            match_end_abs = candidate_end;
                        }
                        if match_len > $crate::encoding::cost_model::HC_OPT_NUM {
                            break;
                        }
                    }
                }
                // Match reached the block tail: can't order the pair (upstream zstd
                // `ip+matchLength == iLimit`), and indexing `concat[idx +
                // match_len]` below would step past the searchable region.
                if match_len >= tail_limit {
                    break;
                }
                // Descend the DUBT (upstream zstd zstd_opt.c:806-811): dict candidate
                // smaller than input → its larger child is closer to `idx`.
                if concat[dict_idx + match_len] < concat[idx + match_len] {
                    common_smaller = match_len;
                    dcur = dms.chain_table[pair + 1];
                } else {
                    common_larger = match_len;
                    dcur = dms.chain_table[pair];
                }
            }
        }

        // `match_end_abs >= abs_pos + 9 >= 9` (initialized and monotonic),
        // so `match_end_abs - 8 >= 1` cannot underflow.
        $table.skip_insert_until_abs = match_end_abs - 8;
    }};
}
pub(crate) use bt_insert_and_collect_matches_body;

impl HcMatchGenerator {
    /// Heap bytes this generator owns: the shared match table plus the BT
    /// backend's optimal-parser / LDM scratch (the HC knobs are inline).
    pub(crate) fn heap_size(&self) -> usize {
        self.table.heap_size() + self.backend.heap_size()
    }

    pub(crate) fn should_run_btultra2_seed_pass<S: crate::encoding::strategy::Strategy>(
        &self,
        current_len: usize,
    ) -> bool {
        // The in-block two-pass dynamic-stats seed (`initStats_ultra`)
        // is btultra2-only. `TWO_PASS_SEED` is `false` for every other
        // strategy — including btultra, which now shares the hash3
        // short-match probe but stays single-pass — so the seed call and
        // its body drop at codegen time for all non-btultra2 kernels.
        if !S::TWO_PASS_SEED {
            return false;
        }
        let HcBackend::Bt(bt) = &self.backend else {
            return false;
        };
        bt.opt_state.lit_length_sum == 0
            && bt.opt_state.dictionary_seed.is_none()
            && !self.table.dictionary_primed_for_frame
            && bt.ldm_sequences.is_empty()
            && self.table.window_size == current_len
            && self.table.history_abs_start == 0
            && self.table.chunk_lens.len() == 1
            && current_len > HC_PREDEF_THRESHOLD
    }

    pub(crate) fn new(max_window_size: usize) -> Self {
        Self {
            table: crate::encoding::match_table::storage::MatchTable::new(max_window_size),
            hc: crate::encoding::hc::HcMatcher::new(2, HC_SEARCH_DEPTH, HC_TARGET_LEN),
            // Default to the zero-sized HC backend; `configure()` swaps
            // in a `BtMatcher` only when an optimal strategy lands.
            backend: HcBackend::Hc,
            // Lazy is the per-construct default — every production
            // caller calls `configure()` before the first encode and
            // overwrites this. Tests that drive `HcMatchGenerator`
            // without calling `configure()` end up in the
            // `start_matching_lazy` arm of the test dispatcher, which
            // matches the previous default behaviour.
            strategy_tag: crate::encoding::strategy::StrategyTag::Lazy,
        }
    }

    pub(crate) fn configure(
        &mut self,
        config: HcConfig,
        tag: crate::encoding::strategy::StrategyTag,
        window_log: u8,
    ) {
        use crate::encoding::strategy::StrategyTag;
        // Mirror the driver-resolved strategy tag so the
        // `#[cfg(test)] start_matching` dispatcher can route
        // BtOpt / BtUltra / BtUltra2 to distinct monomorphisations.
        self.strategy_tag = tag;
        let is_btultra2 = tag == StrategyTag::BtUltra2;
        let uses_bt = matches!(
            tag,
            StrategyTag::Btlazy2
                | StrategyTag::BtOpt
                | StrategyTag::BtUltra
                | StrategyTag::BtUltra2
        );
        // btultra and btultra2 both run the mls=3 hash3 short-match probe
        // (clevels.h minMatch 3). The `is_btultra2` flag below stays
        // exclusive to btultra2 because it tweaks the BT rebase boundary,
        // not match finding.
        let wants_hash3 = matches!(tag, StrategyTag::BtUltra | StrategyTag::BtUltra2);
        let next_hash3_log = if wants_hash3 {
            HC3_HASH_LOG.min(window_log as usize)
        } else {
            0
        };
        let resize = self.table.hash_log != config.hash_log
            || self.table.chain_log != config.chain_log
            || self.table.hash3_log != next_hash3_log;
        // Capture the layout flip BEFORE `uses_bt` is overwritten below — it
        // feeds the dms invalidation (the dms is keyed by layout too).
        let uses_bt_changed = self.table.uses_bt != uses_bt;
        self.table.hash_log = config.hash_log;
        self.table.chain_log = config.chain_log;
        self.table.hash3_log = next_hash3_log;
        self.hc.search_depth = if uses_bt {
            config.search_depth
        } else {
            config.search_depth.min(MAX_HC_SEARCH_DEPTH)
        };
        self.hc.target_len = config.target_len;
        // Mirror strategy-derived flags + HC search depth onto MatchTable
        // so the BT walker and rebase machinery can read them directly
        // without dispatching back through HcMatchGenerator.
        self.table.search_depth = self.hc.search_depth;
        self.table.is_btultra2 = is_btultra2;
        self.table.uses_bt = uses_bt;
        // BT finder hash width, upstream zstd `mls = BOUNDED(4, cParams.minMatch, 6)`,
        // carried explicitly in the level config so a `target_length` override
        // cannot silently flip the finder between 5- and 4-byte hashing. Only
        // the BT body reads it; HC/lazy levels leave it at 4. clevels.h
        // (srcSize > 256 KiB tier): btlazy2 L13-15 + btopt L16 are minMatch=5,
        // btopt L17 is minMatch=4, btultra/btultra2 are minMatch=3 (4-byte main
        // hash + the hash3 short-match probe).
        // The cached dms is keyed by the full (region, layout, mls, hash_log)
        // shape that `build_dms!` validates on the normal prime path, but the
        // reborrow fast path in `MatchTable::reset` reuses it on `dms.is_primed()`
        // ALONE. A reused-compressor level switch can change the search mls (e.g.
        // btlazy2 -> lazy), the table geometry (hash_log / chain_log / hash3,
        // captured in `resize`), OR the HC<->BT layout (`uses_bt_changed`)
        // independently of each other, and any of them leaves the dms hashed for
        // a different shape. Invalidate on ANY so the next dict frame re-primes at
        // the new shape (configure runs before reset) instead of probing a
        // mismatched dms and silently degrading match quality. Over-invalidation
        // only costs a re-prime, which a real shape change needs anyway.
        let mls_changed = self.table.search_mls != config.search_mls;
        if resize || mls_changed || uses_bt_changed {
            self.table.dms.invalidate();
        }
        self.table.search_mls = config.search_mls;
        // Stage D: promote the backend discriminator. HC modes drop the
        // BT scratch buffers entirely; switching back into a BT mode
        // allocates a fresh `BtMatcher` on demand.
        match (&self.backend, self.table.uses_bt) {
            (HcBackend::Hc, true) => {
                self.backend =
                    HcBackend::Bt(alloc::boxed::Box::new(crate::encoding::bt::BtMatcher::new()));
            }
            (HcBackend::Bt(_), false) => {
                self.backend = HcBackend::Hc;
            }
            _ => {}
        }
        if resize && !self.table.hash_table.is_empty() {
            // Force reallocation on next ensure_tables() call.
            self.table.hash_table.clear();
            self.table.hash3_table.clear();
            self.table.chain_table.clear();
        }
    }

    pub(crate) fn seed_dictionary_entropy(
        &mut self,
        huff: Option<&crate::huff0::huff0_encoder::HuffmanTable>,
        ll: Option<&crate::fse::fse_encoder::FSETable>,
        ml: Option<&crate::fse::fse_encoder::FSETable>,
        of: Option<&crate::fse::fse_encoder::FSETable>,
    ) {
        if let HcBackend::Bt(bt) = &mut self.backend {
            bt.opt_state.seed_dictionary_entropy(huff, ll, ml, of);
        }
    }

    /// Install (or clear) the long-distance-match producer. Only
    /// the BT backend owns an `ldm_producer` slot; on the HC (lazy)
    /// backend the producer is dropped because there is no optimal-parser
    /// candidate buffer to seed. Call after [`Self::reset`].

    pub(crate) fn set_ldm_producer(&mut self, producer: Option<crate::encoding::ldm::LdmProducer>) {
        if let HcBackend::Bt(bt) = &mut self.backend {
            bt.ldm_producer = producer;
        }
    }

    /// Move the LDM producer out of the BT backend, leaving `None`. Used by the
    /// dictionary snapshot path: the producer carries no dictionary state (LDM
    /// is not dict-primed; its hash table is empty at capture), so it is not
    /// retained in the snapshot — the working frame's freshly-reset producer is
    /// reinstated on restore instead.

    pub(crate) fn take_ldm_producer(&mut self) -> Option<crate::encoding::ldm::LdmProducer> {
        if let HcBackend::Bt(bt) = &mut self.backend {
            bt.ldm_producer.take()
        } else {
            None
        }
    }

    pub(crate) fn reset(&mut self, reuse_space: impl FnMut(Vec<u8>)) {
        self.table.reset(reuse_space);
        if let HcBackend::Bt(bt) = &mut self.backend {
            bt.reset();
        }
    }

    /// Backfill positions from the tail of the previous slice that couldn't be
    /// hashed at the time (insert_position needs 4 bytes of lookahead).
    pub(crate) fn skip_matching(&mut self, incompressible_hint: Option<bool>) {
        self.table.skip_matching(incompressible_hint);
    }

    /// Strategy-aware entry point used by
    /// [`MatchGeneratorDriver::compress_block`]. Branches on
    /// `S::USE_BT` — a compile-time `const` — so each
    /// monomorphisation keeps exactly one arm: `Lazy` /
    /// `Fast` / `Dfast` / `Greedy` see only `start_matching_lazy`,
    /// `BtOpt` / `BtUltra` / `BtUltra2` see only
    /// `start_matching_optimal`. The inherent test-only
    /// [`HcMatchGenerator::start_matching`] reaches the same arms by
    /// runtime-matching on `self.strategy_tag` (the parse-mode field
    /// has been removed); production never invokes that path.
    pub(crate) fn start_matching_strategy<S: crate::encoding::strategy::Strategy>(
        &mut self,
        handle_sequence: &mut impl for<'a> FnMut(Sequence<'a>),
    ) {
        debug_assert_eq!(
            self.table.uses_bt,
            S::USE_BT,
            "Strategy::USE_BT disagrees with runtime table.uses_bt at HC dispatch"
        );
        if S::USE_BT {
            self.start_matching_optimal::<S>(handle_sequence)
        } else {
            self.start_matching_lazy(handle_sequence)
        }
    }

    /// Dispatcher: pick the dict-aware monomorph when a separate dms is primed
    /// (attach-mode dictionary), else the no-dict monomorph. Mirrors upstream's
    /// compile-time `dictMode` split — the `DICT = false` body carries no dms
    /// code at all, so the no-dict hot path is unaffected by the dict search.
    pub(crate) fn start_matching_lazy(
        &mut self,
        handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        if self.table.dms.is_primed() {
            self.start_matching_lazy_impl::<true>(handle_sequence);
        } else {
            self.start_matching_lazy_impl::<false>(handle_sequence);
        }
    }

    pub(crate) fn start_matching_lazy_impl<const DICT: bool>(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        self.table.ensure_tables();

        // `current_block_range()` is borrowed-aware: owned → last committed
        // chunk; borrowed → the staged in-place block range.
        let (current_abs_start, current_len) = self.table.current_block_range();
        if current_len == 0 {
            return;
        }
        // The current block is the tail of `history` (owned) or the staged
        // borrowed range (`get_last_space()` resolves both). Hoist it as a raw
        // slice: the routine mutates the hash/chain tables + `offset_hist` but
        // never reallocates `history`, so the slice stays valid and we avoid
        // re-borrowing `self.table` (which would conflict with the
        // `offset_hist` write).
        let current_ptr = self.table.get_last_space().as_ptr();
        let current: &[u8] = unsafe { core::slice::from_raw_parts(current_ptr, current_len) };

        // Full live history (dict + committed blocks + current block), hoisted
        // ONCE for the whole position scan and threaded into every
        // `find_best_match` / `pick_lazy_match` call. `live_history()` is
        // loop-invariant here (the scan mutates the hash/chain tables +
        // `offset_hist` but never the history bytes or length), so re-fetching
        // it per find — inside `hash_chain_candidate` + the rep probe, plus
        // again for each lazy lookahead at pos+1 / pos+2 — was pure
        // per-position overhead. Same raw-slice detach as `current` so the
        // loop's `&mut self.table` inserts coexist with this `&[u8]`.
        let concat: &[u8] = {
            let lh = self.table.live_history();
            unsafe { core::slice::from_raw_parts(lh.as_ptr(), lh.len()) }
        };
        // Dict-match-state primed flag, hoisted ONCE for the scan: it is
        // block-invariant (the dict is primed before the block) and lives on the
        // cold `dms` cacheline, so the per-find `dms.is_primed()` load was a
        // measurable hot-path cost (~8% of `hash_chain_candidate` on the
        // dict-over-random fixture). The `DICT = false` monomorph ignores it.
        let dms_primed = self.table.dms.is_primed();

        let current_abs_end = current_abs_start + current_len;
        self.table
            .backfill_boundary_positions(current_abs_start, current_abs_end);

        let mut pos = 0usize;
        let mut literals_start = 0usize;
        // Lookahead carry (upstream's lazy depth loop searches each position
        // ONCE): when the lookahead defers to `abs_pos + 1`, its already-computed
        // match is carried here and reused next iteration instead of re-searched.
        // `None` means search this position fresh.
        let mut carried: Option<HcMatch> = None;
        // Set when the previous position deferred WITHOUT a carry (depth-2:
        // `abs_pos + 2` won but `abs_pos + 1` was cold). If this position then
        // misses, the no-match skip below must advance by exactly ONE so it
        // cannot hop over the already-proven `abs_pos + 2` winner.
        let mut deferred_without_carry = false;
        while pos + HC_MIN_MATCH_LEN <= current_len {
            let abs_pos = current_abs_start + pos;
            let lit_len = pos - literals_start;
            let forced_single_step = core::mem::take(&mut deferred_without_carry);

            // Reuse the carried lookahead match (searched WITH the previous
            // position already inserted, so its offset-1 candidate is visible)
            // or search this position fresh. `find_best_match` returns the
            // forward `(offset, length)` in registers (`HcMatch`, 16 bytes).
            let best = match carried.take() {
                Some(m) => m,
                None => self.hc.find_best_match::<DICT>(
                    concat,
                    dms_primed,
                    &self.table,
                    abs_pos,
                    lit_len,
                ),
            };
            if best.is_match() {
                // Insert `abs_pos` BEFORE the lookahead so the `abs_pos + 1`
                // probe sees it at offset 1 — upstream inserts the searched
                // position during its own search, before the depth loop probes
                // the next one.
                self.table.insert_position(abs_pos);
                if let Some(carry) = self.hc.lazy_decide_carry::<DICT>(
                    concat,
                    dms_primed,
                    &self.table,
                    abs_pos,
                    lit_len,
                    best,
                ) {
                    // DEFER: the lazy lookahead found a better match one (or two)
                    // bytes ahead. Advance exactly ONE byte. Reuse the lookahead
                    // match when it is real (so it is not re-searched — upstream
                    // searches each position once); the `NONE` sentinel is the
                    // depth-2 defer-without-carry case (`abs_pos + 2` won but
                    // `abs_pos + 1` had no match), so search the next fresh and
                    // pin the following advance to one byte (below) so the skip
                    // heuristic cannot hop over that `abs_pos + 2` winner.
                    carried = carry.is_match().then_some(carry);
                    deferred_without_carry = carried.is_none();
                    pos += 1;
                    continue;
                }
                // COMMIT `best`. Backward-extend over the literal run (upstream
                // `zstd_lazy.c` after rep-vs-chain selection) through the shared
                // raw-pointer helper the Row / Dfast probes use — one
                // back-extension source, no per-step slice bounds checks.
                // `abs_pos` is already inserted above, so the forward span fill
                // starts at `abs_pos + 1`.
                let ext = crate::encoding::match_table::helpers::extend_backwards_shared(
                    concat,
                    self.table.history_abs_start,
                    abs_pos - best.offset,
                    abs_pos,
                    best.match_len,
                    lit_len,
                );
                let match_len = ext.match_len;
                self.table
                    .insert_match_span(abs_pos + 1, ext.start + match_len);
                let start = ext.start - current_abs_start;
                let literals = &current[literals_start..start];
                handle_sequence(Sequence::Triple {
                    literals,
                    offset: best.offset,
                    match_len,
                });
                let _ = encode_offset_with_history(
                    best.offset as u32,
                    literals.len() as u32,
                    &mut self.table.offset_hist,
                );
                pos = start + match_len;
                literals_start = pos;
                continue;
            }
            // No match found.
            self.table.insert_position(abs_pos);
            // Lazy skipping (upstream zstd `ZSTD_compressBlock_lazy_generic`,
            // zstd_lazy.c:1614): advance faster over runs with no match.
            // `step = ((ip - anchor) >> kSearchStrength) + 1` with
            // kSearchStrength = 8, where `ip - anchor` is the current
            // literal-run length. On compressible input the run stays short
            // (step == 1, identical to a 1-byte advance); on incompressible
            // / dict-over-random input the run grows so the parser skips
            // ahead (one search per `step` positions) instead of searching
            // every byte. Skipped positions are not inserted, mirroring
            // upstream (it inserts only searched positions during a no-match
            // run). Ratio follows upstream (not byte-identical).
            // A miss one byte after a depth-2 defer-without-carry must advance by
            // exactly one so the skip cannot pass the proven `abs_pos + 2` match.
            let step = if forced_single_step {
                1
            } else {
                ((pos - literals_start) >> 8) + 1
            };
            pos += step;
            // No clamp needed before the tail loop: the search bound and the
            // hashable bound are both `pos + HC_MIN_MATCH_LEN <= current_len`
            // (HC_MIN_MATCH_LEN == 4 == the insert width), so there is no
            // non-searchable-but-hashable anchor to miss. Positions the skip
            // jumps over inside the searchable region are intentionally not
            // inserted — same as upstream zstd, which advances past them via
            // the identical `ip += step` and never hashes them either.
        }

        // Insert remaining hashable positions in the tail (the matching loop
        // stops at HC_MIN_MATCH_LEN but insert_position only needs 4 bytes).
        while pos + 4 <= current_len {
            self.table.insert_position(current_abs_start + pos);
            pos += 1;
        }

        if literals_start < current_len {
            handle_sequence(Sequence::Literals {
                literals: &current[literals_start..],
            });
        }
    }

    /// Register the borrowed input window for the no-copy one-shot path.
    /// # Safety
    /// `buffer` must outlive the borrowed scans (see `MatchTable`).
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        // SAFETY: forwarded liveness contract.
        unsafe { self.table.set_borrowed_window(buffer) };
    }

    pub(crate) fn clear_borrowed_window(&mut self) {
        self.table.clear_borrowed_window();
    }

    /// Borrowed (no-copy) equivalent of [`Self::start_matching_lazy`]: stage
    /// the in-place block range, then run the same lazy chain parse. The
    /// parse reads its range via `current_block_range()` and its bytes via
    /// `get_last_space()` / `live_history()`, all borrowed-aware, so the block
    /// is scanned in place with the per-position window_low offset cap.
    pub(crate) fn start_matching_lazy_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        self.table.stage_borrowed_block(block_start, block_end);
        self.start_matching_lazy(handle_sequence);
    }

    /// Borrowed (no-copy) equivalent of the lazy `skip_matching`: stage the
    /// in-place block, then seed positions without an owned-history append.
    pub(crate) fn skip_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        incompressible_hint: Option<bool>,
    ) {
        self.table.stage_borrowed_block(block_start, block_end);
        self.table.skip_matching(incompressible_hint);
    }
}
