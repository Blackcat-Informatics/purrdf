#![allow(
    clippy::inline_always,
    clippy::manual_let_else,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Hash-chain match finder used by `Lazy2`.
//!
//! Hosts the runtime knobs of the lazy parser — the lookahead depth
//! (`lazy_depth`), the chain-walk search budget (`search_depth`), and
//! the "sufficient match length" threshold (`target_len`). Method
//! bodies (chain walk, `insert_position`, `pick_lazy_match`,
//! `start_matching_lazy`) still live on `HcMatchGenerator` and will
//! move onto `impl HcMatcher` in Stage 2b alongside the
//! `&mut MatchTable` thread-through; this stage establishes the
//! ownership boundary so the BT-side extraction in Stage 3 has a
//! clean counterpart type to mirror.
//!
//! Upstream zstd parity reference: `lib/compress/zstd_lazy.c`,
//! `ZSTD_HcFindBestMatch` / `ZSTD_compressBlock_lazy2_generic`.

#![allow(dead_code)]

pub(crate) mod generator;
pub(crate) mod optimal;
pub(crate) mod priceset;

use super::cost_model::{HC_FORMAT_MINMATCH, HC_OPT_NUM, HcOptimalCostProfile};
use super::match_table::helpers::common_prefix_len;
use super::match_table::storage::{HC_EMPTY, MatchTable};
use super::opt::types::MatchCandidate;

/// Minimum match length emitted by the lazy / lazy2 chain walker.
/// Upstream zstd parity: `MIN_MATCH` in `lib/compress/zstd_lazy.c`.
pub(crate) const HC_MIN_MATCH_LEN: usize = 4;

/// Forward-only match found by the lazy chain walk: the `(offset, length)` pair
/// upstream's `ZSTD_HcFindBestMatch` returns (`size_t matchLength` in `rax` +
/// the offset through `offsetPtr`). 16 bytes, so the find result + the lazy
/// lookahead arg travel in registers (`rax:rdx`) instead of the 24-byte
/// `MatchCandidate` (`start` included) the old find returned by value, which
/// the System V ABI spills to the stack and the lazy loop then copies. The
/// backward extension that produces `start` is deferred to the commit site
/// (upstream does it once in the lazy loop after rep-vs-chain selection), so it
/// never touches the per-position find/compare path.
#[derive(Clone, Copy)]
pub(crate) struct HcMatch {
    /// Match offset (`abs_pos - candidate_pos`). Meaningful only when
    /// [`HcMatch::is_match`] is true.
    pub(crate) offset: usize,
    /// Forward match length, or `< HC_MIN_MATCH_LEN` for "no match".
    pub(crate) match_len: usize,
}

impl HcMatch {
    /// The "no match found" sentinel (`match_len = 0`).
    pub(crate) const NONE: Self = Self {
        offset: 0,
        match_len: 0,
    };

    /// A real match reaches at least `HC_MIN_MATCH_LEN`; shorter is the seed /
    /// sentinel, not an emittable match.
    #[inline]
    pub(crate) fn is_match(self) -> bool {
        self.match_len >= HC_MIN_MATCH_LEN
    }
}

/// Hard cap on chain-walk depth. Used to size the fixed-length
/// candidate buffer returned by [`HcMatcher::chain_candidates`].
pub(crate) const MAX_HC_SEARCH_DEPTH: usize = 512;

/// Hash-chain matcher state used by the `Lazy2` parse mode (and the
/// short-history fast path of the BT cascade's initial pass).
///
/// Owns only the per-frame *configuration* — the actual chain / hash
/// tables live on the shared
/// [`super::match_table::storage::MatchTable`] that this matcher
/// borrows when it runs.
#[derive(Clone)]
pub(crate) struct HcMatcher {
    /// Lookahead depth (1 = lazy, 2 = lazy2). Upstream zstd parity:
    /// `params->cParams.strategy >= ZSTD_lazy2`.
    pub(crate) lazy_depth: u8,
    /// Maximum number of chain entries inspected per `find_best_match`
    /// call. Upstream zstd parity: `params->cParams.searchLog` (clamped to
    /// [`MAX_HC_SEARCH_DEPTH`](super::match_generator::MAX_HC_SEARCH_DEPTH)
    /// for HC mode; BT modes use the unclamped value as their walk
    /// budget).
    pub(crate) search_depth: usize,
    /// "Sufficient" match length — once a candidate reaches this
    /// length, the lazy decision short-circuits without checking the
    /// next position. Upstream zstd parity:
    /// `params->cParams.targetLength`.
    pub(crate) target_len: usize,
}

impl HcMatcher {
    pub(crate) fn new(lazy_depth: u8, search_depth: usize, target_len: usize) -> Self {
        Self {
            lazy_depth,
            search_depth,
            target_len,
        }
    }

    /// Upstream zstd "match gain" heuristic: `match_len * 4 - offset_bits`.
    /// The lazy lookahead uses this to compare a candidate at the
    /// Pick the better of two candidate matches by LENGTH (ties to the smaller
    /// offset), the same selector the chain walk and the shared Row/Dfast
    /// repcode probe use. Upstream `ZSTD_compressBlock_lazy_generic` compares
    /// the repcode against the searched match by length at depth 0 (`ml2 >
    /// matchLength`, the repcode keeps ties); a longer or equal-length-closer
    /// match wins. `None` arms pass the surviving `Some` through.
    pub(crate) fn better_candidate(lhs: HcMatch, rhs: HcMatch) -> HcMatch {
        if !lhs.is_match() {
            return rhs;
        }
        if !rhs.is_match() {
            return lhs;
        }
        if rhs.match_len > lhs.match_len
            || (rhs.match_len == lhs.match_len && rhs.offset < lhs.offset)
        {
            rhs
        } else {
            lhs
        }
    }

    /// Probe the 3 rep-code offsets (with the upstream zstd `ll0 ↦ rep[0] − 1`
    /// fallback) and return the best in-range match. Pure helper —
    /// only reads from `MatchTable`, no HcMatcher state needed.
    pub(crate) fn repcode_candidate(table: &MatchTable, abs_pos: usize, lit_len: usize) -> HcMatch {
        let reps = if lit_len == 0 {
            [
                Some(table.offset_hist[1] as usize),
                Some(table.offset_hist[2] as usize),
                (table.offset_hist[0] > 1).then_some((table.offset_hist[0] - 1) as usize),
            ]
        } else {
            [
                Some(table.offset_hist[0] as usize),
                Some(table.offset_hist[1] as usize),
                Some(table.offset_hist[2] as usize),
            ]
        };

        let concat = table.live_history();
        let current_idx = abs_pos - table.history_abs_start;
        if current_idx + HC_MIN_MATCH_LEN > concat.len() {
            return HcMatch::NONE;
        }
        // Raw base pointer for the upstream zstd-style `MEM_read32` 4-byte rep
        // gate below (single unaligned load each, no per-rep slice bounds check).
        let base_ptr = concat.as_ptr();

        let mut best = HcMatch::NONE;
        for rep in reps.into_iter().flatten() {
            if rep == 0 || rep > abs_pos {
                continue;
            }
            let candidate_pos = abs_pos - rep;
            if candidate_pos
                < table
                    .history_abs_start
                    .max(abs_pos.saturating_sub(table.max_window_size))
            {
                continue;
            }
            let candidate_idx = candidate_pos - table.history_abs_start;
            // Cheap 4-byte equality gate before the wider SIMD count (upstream
            // zstd `MEM_read32` repcode gate in `ZSTD_compressBlock_lazy`).
            // `HC_MIN_MATCH_LEN == 4` and `current_idx + 4 <= len` (the early
            // return above), so a first-4-byte mismatch can never reach the
            // match floor — reject without the vector load+count. Falls through
            // to the full count only when the candidate lacks a 4-byte lookahead
            // (short tail), so the accepted set is byte-identical to the
            // unconditional count. Mirrors the chain-walk gate.
            if candidate_idx + 4 <= concat.len()
                && unsafe {
                    MatchTable::read_le_u32_ptr(base_ptr.add(candidate_idx))
                        != MatchTable::read_le_u32_ptr(base_ptr.add(current_idx))
                }
            {
                continue;
            }
            let match_len = common_prefix_len(&concat[candidate_idx..], &concat[current_idx..]);
            if match_len >= HC_MIN_MATCH_LEN {
                best = Self::better_candidate(
                    best,
                    HcMatch {
                        offset: rep,
                        match_len,
                    },
                );
            }
        }
        best
    }

    /// Best hash-chain match at `abs_pos`, in upstream zstd's
    /// `ZSTD_HcFindBestMatch` forward-length model: walk the live chain (and
    /// the dms) tracking the longest FORWARD match (`currentMl > ml`, ties
    /// keep the closest), gate each candidate on the self-tightening 4-byte
    /// tail probe, and extend the single winner backwards over the literal
    /// run ONCE at the end. No per-candidate `Option` / gain merge / backward
    /// extension — that per-attempt overhead is what the old gain-based walk
    /// paid over C.
    ///
    /// `#[inline(always)]`: folded INTO the out-of-line `find_best_match` so the
    /// whole find (rep + chain + dms + selection) is ONE function — upstream's
    /// `ZSTD_HcFindBestMatch` shape — reached by a single call per position, not
    /// the find->chain->dms call chain that an out-of-line chain walk adds.
    #[inline(always)]
    pub(crate) fn hash_chain_candidate<const DICT: bool>(
        &self,
        concat: &[u8],
        dms_primed: bool,
        table: &MatchTable,
        abs_pos: usize,
    ) -> HcMatch {
        let current_idx = abs_pos - table.history_abs_start;
        let history_tail = concat.len();
        if current_idx + HC_MIN_MATCH_LEN > history_tail {
            return HcMatch::NONE;
        }

        // Chain walk is inlined below — avoids the per-call 4 KiB
        // `[usize; MAX_HC_SEARCH_DEPTH]` array that
        // [`Self::chain_candidates`] materializes (zero-init on entry,
        // memcpy on return). At lazy_depth=2 (L7+) `pick_lazy_match`
        // triggers up to three chain walks per committed position, so
        // the array form costs ~12 KiB of stack traffic per accepted
        // match. Upstream zstd (`zstd_lazy.c` `ZSTD_HcFindBestMatch`) runs a
        // single fused loop with no intermediate buffer; mirror that.
        //
        // `chain_candidates` itself is still alive — the chain-walk
        // unit tests drive it directly, and the BT-optimal HC
        // candidate collector in `match_generator.rs` consumes it
        // through a macro pipeline that inherits the array form.
        // Inlining the array out of that BT-optimal callsite is a
        // separate, larger refactor; this commit only addresses the
        // lazy hot path.
        let hash = table.hash_position(&concat[current_idx..]);
        let chain_mask = (1usize << table.chain_log) - 1;
        let mut cur = table.hash_table[hash];
        // Cap loop at MAX_HC_SEARCH_DEPTH so a misconfigured
        // `search_depth > MAX_HC_SEARCH_DEPTH` (BT modes set it from the
        // upstream zstd config, which can exceed our cap) cannot run forever.
        let max_chain_steps = self.search_depth.min(MAX_HC_SEARCH_DEPTH);
        let mut steps = 0usize;
        let history_abs_start = table.history_abs_start;

        // Forward-length match state (upstream `ZSTD_HcFindBestMatch`): `ml` is
        // the FORWARD match length, seeded at `MIN_MATCH - 1` so the first
        // candidate's self-tightening gate (`match + ml - 3`) degenerates to a
        // first-4-byte compare. `best_idx` is the winning candidate's concat
        // index.
        //
        // `found` is tracked ONLY in the `DICT = false` monomorph. `ml >=
        // MIN_MATCH` is always exactly equivalent to "a match was found" (ml
        // only updates via `current_ml > ml`, whose first hit is >= MIN_MATCH),
        // so the `DICT = true` path uses that equivalence and drops the bool —
        // the dict monomorph is register-tight (it also runs the dms walk), and
        // eliding `found` there frees a stack slot (measured: -5% cycles on the
        // dict path). The `DICT = false` monomorph is NOT register-tight, and
        // there the equivalent `ml < MIN_MATCH` final check changed codegen
        // layout enough to cost ~+4% (same instruction count, worse IPC), so it
        // keeps the original bool. `DICT` is const, so each monomorph compiles
        // exactly one arm with no runtime branch; in the dict monomorph `found`
        // is written/read only in dead `!DICT` arms and is fully eliminated.
        let mut ml = HC_MIN_MATCH_LEN - 1;
        let mut best_idx = 0usize;
        let mut found = false;
        // Raw base pointer for the upstream zstd-style `MEM_read32` gate
        // (single unaligned load each, no per-candidate slice bounds check).
        let base_ptr = concat.as_ptr();
        // Loop-invariant precompute, hoisted out of the chain walk (upstream zstd:
        // `lowLimit` + base pointers are set once before the loop). Candidates
        // are tracked in history-RELATIVE index space so each step is a single
        // `add` + range check + 4-byte gate, mirroring `ZSTD_HcFindBestMatch`'s
        // tight body (`matchIndex >= lowLimit`; `NEXT_IN_CHAIN`) instead of an
        // `Option`-returning absolute-position decode plus a per-candidate
        // `.max()/.saturating_sub()` window-floor recompute.
        let floor_idx = current_idx.saturating_sub(table.max_window_size);
        // `candidate_idx = position_base + (cur-1) - index_shift -
        // history_abs_start`, folded into one wrapping bias so the per-step cost
        // is `cur + idx_bias`. Stale / sub-floor chain entries wrap to a huge
        // index and are rejected by the `< current_idx` upper bound — the
        // accepted set is identical to the `stored_abs_position_fast` decode
        // (which returned `None` for the same sub-floor entries).
        let idx_bias = table
            .position_base
            .wrapping_sub(1)
            .wrapping_sub(table.index_shift)
            .wrapping_sub(history_abs_start);
        // Hoist the chain-table base pointer ONCE: through `&MatchTable` the
        // optimizer reloads the `Vec` (ptr, len) header — and re-checks bounds —
        // on every `table.chain_table[..]`, a measured ~1.7x of upstream's
        // per-find load count. `candidate_rel & chain_mask` is always
        // `< chain_table.len()` (the table is `1 << chain_log` and
        // `chain_mask == len - 1`), so the raw read is in-bounds.
        let chain_ptr: *const u32 = table.chain_table.as_ptr();
        while steps < max_chain_steps {
            if cur == HC_EMPTY {
                break;
            }
            let candidate_rel = cur.wrapping_sub(1) as usize;
            // SAFETY: `candidate_rel & chain_mask < chain_table.len()`.
            let next = unsafe { *chain_ptr.add(candidate_rel & chain_mask) };
            steps += 1;

            let candidate_idx = (cur as usize).wrapping_add(idx_bias);
            // A wrapped index (`>= current_idx`) means `abs < history_abs_start`:
            // a stale entry from a previous frame. The chain is head-insert
            // monotonic, so the rest of the tail is older and also stale -- stop.
            // This is the early-exit the no-memset floor-advance reset needs; in
            // the memset reset the chain has no wrapped entries, so it never
            // fires. INVARIANT: the memset reset zeroes every slot to `HC_EMPTY`
            // (caught by the `cur == HC_EMPTY` break above) and uses `idx_bias`
            // that maps stored positions back to indices `< current_idx`, so no
            // surviving entry can wrap -- only the floor-advance path retains
            // cross-frame entries that decode past `current_idx`. If a future
            // change let a memset-reset chain produce a wrap, this break would
            // silently cut the walk short (a miss-rate regression, not a
            // correctness bug). Below-floor (`< floor_idx`) entries are NOT a
            // break: a chain-slot collision can put an in-window entry after one.
            if candidate_idx >= current_idx {
                break;
            }
            if candidate_idx >= floor_idx {
                // Upstream zstd's single self-tightening gate (`zstd_lazy.c:714`):
                //   MEM_read32(match + ml - 3) == MEM_read32(ip + ml - 3)
                // proves the candidate can possibly reach `ml + 1` before paying
                // for the full `common_prefix_len`. `ml >= MIN_MATCH - 1 = 3` so
                // `gate_off >= 0`. The iLimit break below keeps
                // `current_idx + ml < history_tail` on entry, so
                // `current_idx + gate_off + 4 = current_idx + ml + 1 <= history_tail`,
                // and `candidate_idx < current_idx` keeps the match-side read in
                // range too -- no per-iteration bounds check (matches C's
                // branchless gate).
                let gate_off = ml - 3;
                // SAFETY: both reads are `<= history_tail` per the bound above.
                let gate_ok = unsafe {
                    MatchTable::read_le_u32_ptr(base_ptr.add(candidate_idx + gate_off))
                        == MatchTable::read_le_u32_ptr(base_ptr.add(current_idx + gate_off))
                };
                if gate_ok {
                    let current_ml =
                        common_prefix_len(&concat[candidate_idx..], &concat[current_idx..]);
                    // `currentMl > ml`: strictly longer forward match wins; ties
                    // keep the first (closest, the walk is newest-first).
                    if current_ml > ml {
                        ml = current_ml;
                        best_idx = candidate_idx;
                        // Dict path reads `ml >= MIN_MATCH`; only the no-dict
                        // monomorph keeps the `found` flag (see seed comment).
                        if !DICT {
                            found = true;
                        }
                        // Upstream `if (ip + currentMl == iLimit) break`: reached
                        // the input end (best possible); also keeps the next gate
                        // read in bounds.
                        if current_idx + ml >= history_tail {
                            break;
                        }
                    }
                }
            }

            // Self-loop (two positions share `candidate_rel & chain_mask`):
            // checked here where `next` / `cur` are already live, not carried as
            // a bool across the body (a register in this saturated walk).
            if next == cur {
                break;
            }
            cur = next;
        }

        // Separate dictionary match state (upstream `ZSTD_dictMatchState`). The
        // walk is OUT-OF-LINE so the dict-only code never bloats this hot
        // function on no-dict frames. It shares `ml` / `steps` (upstream's
        // single `nbAttempts` across the live + dms loops). Skip it when the
        // live walk already reached iLimit (`ml` maximal, and the dms
        // self-tightening gate would read past `history_tail`).
        // `dms_primed` is hoisted by the caller (block-invariant — the dict is
        // primed before the block scan), replacing a per-find `dms.is_primed()`
        // load from the cold `dms` cacheline.
        if DICT && dms_primed && current_idx + ml < history_tail {
            // dms runs only in the `DICT = true` monomorph, which uses the
            // `ml >= MIN_MATCH` match test, so the walk tracks `(ml, best_idx)`
            // and never the `found` flag.
            let walk = self.dms_chain_walk(
                table,
                concat,
                current_idx,
                history_tail,
                base_ptr,
                max_chain_steps,
                steps,
                ml,
                best_idx,
            );
            ml = walk.0;
            best_idx = walk.1;
        }

        // Dict monomorph: `ml >= MIN_MATCH` is the match test (drops `found`).
        // No-dict monomorph: keep the original `found` flag (see seed comment).
        let matched = if DICT { ml >= HC_MIN_MATCH_LEN } else { found };
        if !matched {
            return HcMatch::NONE;
        }
        // Forward `(offset, length)` only — the backward extension that produces
        // `start` runs once at the commit site (upstream's lazy loop), off the
        // per-position find/compare path. `offset = abs_pos - (best_idx +
        // history_abs_start) = current_idx - best_idx`.
        HcMatch {
            offset: current_idx - best_idx,
            match_len: ml,
        }
    }

    /// Out-of-line dms HC4 walk (upstream `ZSTD_HcFindBestMatch` dms loop,
    /// `zstd_lazy.c:751-769`). Split from [`Self::hash_chain_candidate`] so the
    /// no-dict hot path keeps its small body / register budget. Shares the
    /// caller's forward-length state (`ml` / `best_idx`) and `steps`
    /// budget -- the live + dms loops are one bounded operation (upstream's
    /// single `nbAttempts`). The dict sits at the front of `concat`
    /// (`[0, region)`); a dms candidate is a concat index `< current_idx`, so
    /// the offset / gate / count logic matches the live walk. Only the
    /// `DICT = true` monomorph calls this, and it uses the `ml >= MIN_MATCH`
    /// match test, so the walk returns `(ml, best_idx)` and tracks no `found`
    /// flag.
    ///
    /// `#[inline]`: the `DICT = false` monomorph never references this (the
    /// `if DICT &&` gate is a compile-time false), so it is dropped from the
    /// no-dict path; the `DICT = true` monomorph inlines it into the dict hot
    /// path.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn dms_chain_walk(
        &self,
        table: &MatchTable,
        concat: &[u8],
        current_idx: usize,
        history_tail: usize,
        base_ptr: *const u8,
        max_chain_steps: usize,
        mut steps: usize,
        mut ml: usize,
        mut best_idx: usize,
    ) -> (usize, usize) {
        let dms = match table.dms.table() {
            Some(d) => d,
            None => return (ml, best_idx),
        };
        // Match upstream's effective dict search depth: the CDict path runs the
        // dedicated dict search deeper than the bare level searchLog (measured:
        // upstream needs nbAttempts >= 16 to surface the long dict match on the
        // per-label-dict fixtures). Floor the shared budget at 16.
        //
        // NOTE on the per-find step budget: `steps` is SHARED with the live
        // walk. The floor below is `max(max_chain_steps, 16)`, so the live and
        // dms walks TOGETHER are bounded by it (= 16 at every standard HC level,
        // where `max_chain_steps < 16`: L6 `searchLog = 3` -> 8); the dms
        // consumes whatever the live walk left. Giving the dms a fresh 16
        // attempts on top of the live walk instead (a per-walk, non-shared
        // budget) was measured +3% cycles on the small-10k-random dict path
        // with zero ratio change -- the shared floor already surfaces every
        // dict match that reaches the output, so sharing it is intentional: the
        // floor (not `max_chain_steps`) is the binding dict-search constraint.
        let dms_budget = max_chain_steps.max(16);
        let dms_hash = MatchTable::hash_position_at(concat, current_idx, dms.hash_log, dms.mls);
        let mut dcur = dms.hash_table[dms_hash];
        // Hoist the dms chain-table base pointer (same Vec-header-reload removal
        // as the live walk above).
        let dms_chain_ptr: *const u32 = dms.chain_table.as_ptr();
        // Reachability floor, hoisted out of the loop. A dict position is
        // decoder-reachable iff its offset (`current_idx - dict_idx`) does not
        // exceed `max_window_size`, i.e. `dict_idx >= current_idx -
        // max_window_size`. Comparing `dict_idx` against this precomputed floor
        // drops the per-iteration `current_idx - dict_idx` subtract from the hot
        // path (the offset is only needed when a candidate WINS, which is rare on
        // the walk) and frees the `max_window_size` register inside the loop.
        // `saturating_sub` is the intended business logic, not an overflow guard:
        // when `current_idx <= max_window_size` the whole live history fits the
        // window, so every dict position is reachable and the floor is 0. Mirrors
        // C's `ZSTD_HcFindBestMatch` dms loop, which bounds reachability by chain
        // construction rather than re-checking each candidate.
        let dict_reachable_floor = current_idx.saturating_sub(table.max_window_size);
        while steps < dms_budget {
            if dcur == 0 {
                break;
            }
            // Dict position is a concat index in `[0, region)`; the dict is at
            // the front so `dict_idx < current_idx` always (and
            // `< dms.chain_table.len()`).
            let dict_idx = (dcur - 1) as usize;
            // SAFETY: `dict_idx` is a stored dict position `< chain_table.len()`.
            let dnext = unsafe { *dms_chain_ptr.add(dict_idx) };
            steps += 1;
            // Out-of-window dict positions are unreachable to the decoder.
            if dict_idx >= dict_reachable_floor {
                // Same self-tightening gate as the live walk; the caller's guard
                // (`current_idx + ml < history_tail`) + the iLimit break keep
                // `current_idx + (ml - 3) + 4 <= history_tail`, and
                // `dict_idx < current_idx` keeps the dict-side read in range.
                let gate_off = ml - 3;
                // SAFETY: both reads are `<= history_tail` per the bound above.
                let gate_ok = unsafe {
                    MatchTable::read_le_u32_ptr(base_ptr.add(dict_idx + gate_off))
                        == MatchTable::read_le_u32_ptr(base_ptr.add(current_idx + gate_off))
                };
                if gate_ok {
                    let current_ml = common_prefix_len(&concat[dict_idx..], &concat[current_idx..]);
                    if current_ml > ml {
                        ml = current_ml;
                        best_idx = dict_idx;
                        if current_idx + ml >= history_tail {
                            break;
                        }
                    }
                }
            }
            // Chain links are strictly decreasing (head insert); `dnext == dcur`
            // (or the `dcur == 0` at the top) ends the walk.
            if dnext == dcur {
                break;
            }
            dcur = dnext;
        }
        (ml, best_idx)
    }

    /// Single-rep (offset_1) probe — the lazy parser's inline rep check
    /// (upstream `ZSTD_compressBlock_lazy_generic`: only `rep[0]` is tested per
    /// position, the 3-rep rotation is the optimal parser's). Caller guarantees
    /// `lit_len > 0`, so `rep[0]` is repcode index 0 with no ll0 rotation, and
    /// extending it backwards over the literal run is sound. Strips
    /// [`Self::repcode_candidate`] to its first rep: one window check, one
    /// 4-byte gate, one count — a third of the per-position rep cost the 3-rep
    /// probe paid (and which the lazy lookahead paid again at pos+1 / pos+2).
    ///
    /// `#[inline(always)]`: upstream checks the rep INLINE in the lazy loop;
    /// folded into the (out-of-line) `find_best_match` monolith so the rep is
    /// part of the single per-position find call, not a nested out-of-line probe.
    #[inline(always)]
    pub(crate) fn repcode_candidate_offset1(
        concat: &[u8],
        table: &MatchTable,
        abs_pos: usize,
    ) -> HcMatch {
        let rep = table.offset_hist[0] as usize;
        if rep == 0 || rep > abs_pos {
            return HcMatch::NONE;
        }
        let current_idx = abs_pos - table.history_abs_start;
        if current_idx + HC_MIN_MATCH_LEN > concat.len() {
            return HcMatch::NONE;
        }
        let candidate_pos = abs_pos - rep;
        if candidate_pos
            < table
                .history_abs_start
                .max(abs_pos.saturating_sub(table.max_window_size))
        {
            return HcMatch::NONE;
        }
        let candidate_idx = candidate_pos - table.history_abs_start;
        // Cheap 4-byte equality gate before the SIMD count (upstream `MEM_read32`
        // repcode gate); identical to the per-rep gate in `repcode_candidate`.
        let base_ptr = concat.as_ptr();
        if candidate_idx + 4 <= concat.len()
            && unsafe {
                MatchTable::read_le_u32_ptr(base_ptr.add(candidate_idx))
                    != MatchTable::read_le_u32_ptr(base_ptr.add(current_idx))
            }
        {
            return HcMatch::NONE;
        }
        let match_len = common_prefix_len(&concat[candidate_idx..], &concat[current_idx..]);
        if match_len >= HC_MIN_MATCH_LEN {
            HcMatch {
                offset: rep,
                match_len,
            }
        } else {
            HcMatch::NONE
        }
    }

    /// Combine the rep-code and chain-walk candidates and pick the
    /// better of the two. Monomorphised over `DICT` (upstream's compile-time
    /// `dictMode` template param): the `DICT = false` instance compiles WITHOUT
    /// any dms code so the no-dict hot path keeps its tight body, while the
    /// `DICT = true` instance dual-probes the separate dictMatchState. The
    /// dispatcher in `start_matching_lazy` picks the instance from whether a dms
    /// is primed.
    ///
    /// `#[inline(never)]`: kept out-of-line (upstream's `ZSTD_HcFindBestMatch`
    /// is its own function, with the lazy loop thin) so the find's register
    /// pressure is contained in its own frame instead of competing with the
    /// per-position loop state; the 16-byte `HcMatch` return travels in
    /// `rax:rdx`, so the find->loop boundary stays a register pair, not a stack
    /// spill.
    #[inline(never)]
    pub(crate) fn find_best_match<const DICT: bool>(
        &self,
        concat: &[u8],
        dms_primed: bool,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
    ) -> HcMatch {
        // `concat` is the full live history (dict + committed blocks + current
        // block), hoisted ONCE per block by the caller — `live_history()` is
        // loop-invariant for the position scan, so fetching it per find (in
        // `hash_chain_candidate` + the rep probe) was pure per-position
        // overhead. See `start_matching_lazy`'s block-level hoist.
        //
        // Upstream `ZSTD_compressBlock_lazy_generic` checks only offset_1
        // (`rep[0]`) inline per position; the 3-rep rotation lives in the
        // optimal parser, not the lazy loop. For `lit_len > 0` that is exactly
        // `rep[0]` (repcode index 0, no ll0 rotation), so probe the single rep.
        // The `lit_len == 0` case keeps the full rotated probe for encode
        // correctness (the ll0 `rep[0]-1` rule).
        let rep = if lit_len == 0 {
            Self::repcode_candidate(table, abs_pos, lit_len)
        } else {
            Self::repcode_candidate_offset1(concat, table, abs_pos)
        };
        let hash = self.hash_chain_candidate::<DICT>(concat, dms_primed, table, abs_pos);
        Self::better_candidate(rep, hash)
    }

    /// Upstream zstd `lazy` / `lazy2` lookahead: evaluate the match a byte
    /// (and optionally two) ahead before committing the current one.
    /// Returns `true` if the current match `best` wins (commit it), `false`
    /// if the caller should defer to a later position.
    ///
    /// Takes the already-unwrapped `best` BY VALUE and returns the lazy
    /// commit/defer decision: `None` = COMMIT `best`; `Some(carry)` = DEFER and
    /// advance one byte. The caller reuses `carry` as the next `best` when it is
    /// a real match (`carry.is_match()`), or searches the next position fresh
    /// when it is the `NONE` sentinel — the depth-2 defer-WITHOUT-carry case
    /// (the `abs_pos + 2` probe wins but `abs_pos + 1` had no match). That case
    /// must DEFER, not commit, or lazy2 misses the two-ahead match.
    ///
    /// The caller inserts `abs_pos` into the hash tables BEFORE calling this, so
    /// the `pos + 1` / `pos + 2` lookahead sees `abs_pos` at offset 1 — matching
    /// upstream, where the searched position is inserted during its own search,
    /// before the depth loop probes the next one.
    pub(crate) fn lazy_decide_carry<const DICT: bool>(
        &self,
        concat: &[u8],
        dms_primed: bool,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        best: HcMatch,
    ) -> Option<HcMatch> {
        let history_end = table.history_abs_start + concat.len();
        // HC's finder is out-of-line by design (`#[inline(never)]`), so the
        // shared decision macro expands here with no register cost. Map the
        // macro's `Option<Option<HcMatch>>` to one `Option` level: `None`
        // (commit) stays `None`; `Some(Some(carry))` (defer with a reusable
        // lookahead) -> `Some(carry)`; `Some(None)` (depth-2 defer with no
        // carryable match one byte ahead) -> `Some(NONE)` so the caller still
        // DEFERS but searches the next position fresh.
        match crate::encoding::lazy_parse::lazy_decide!(
            best_len = best.match_len,
            best_off = best.offset,
            target_len = self.target_len,
            lazy_depth = self.lazy_depth,
            abs_pos = abs_pos,
            lit_len = lit_len,
            history_end = history_end,
            min_match = HC_MIN_MATCH_LEN,
            search = |pos, ll| {
                let m = self.find_best_match::<DICT>(concat, dms_primed, table, pos, ll);
                m.is_match().then_some(m)
            },
        ) {
            ::core::option::Option::None => ::core::option::Option::None,
            ::core::option::Option::Some(::core::option::Option::Some(carry)) => Some(carry),
            ::core::option::Option::Some(::core::option::Option::None) => Some(HcMatch::NONE),
        }
    }

    /// Cross-platform dispatcher for the rep-code probe used by the
    /// optimal-parser pipeline. Routes to the kernel-specific variant
    /// so the per-rep `common_prefix_len_ptr` call inlines under the
    /// callee's `target_feature` umbrella. Test / external callers
    /// only — the on-encode hot path bypasses this dispatcher via the
    /// kernel-specific variants invoked from inside
    /// `collect_optimal_candidates_initialized_<kernel>`.
    #[allow(dead_code)]
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_each_repcode_candidate_with_reps(
        &self,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        reps: [u32; 3],
        current_abs_end: usize,
        min_match_len: usize,
        f: impl FnMut(MatchCandidate),
    ) {
        // SAFETY: each branch verifies the target_feature requirement of
        // the callee (same shape as the BT walk dispatchers).
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.for_each_repcode_candidate_with_reps_neon(
                table,
                abs_pos,
                lit_len,
                reps,
                current_abs_end,
                min_match_len,
                f,
            )
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match table.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.for_each_repcode_candidate_with_reps_avx2_bmi2(
                        table,
                        abs_pos,
                        lit_len,
                        reps,
                        current_abs_end,
                        min_match_len,
                        f,
                    )
                },
                FastpathKernel::Sse42 => unsafe {
                    self.for_each_repcode_candidate_with_reps_sse42(
                        table,
                        abs_pos,
                        lit_len,
                        reps,
                        current_abs_end,
                        min_match_len,
                        f,
                    )
                },
                FastpathKernel::Scalar => self.for_each_repcode_candidate_with_reps_scalar(
                    table,
                    abs_pos,
                    lit_len,
                    reps,
                    current_abs_end,
                    min_match_len,
                    f,
                ),
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.for_each_repcode_candidate_with_reps_scalar(
                table,
                abs_pos,
                lit_len,
                reps,
                current_abs_end,
                min_match_len,
                f,
            )
        }
    }

    /// NEON umbrella variant of the rep-code probe.
    ///
    /// # Safety
    /// Caller must be running on an AArch64 target with NEON
    /// available (baseline on AArch64).
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn for_each_repcode_candidate_with_reps_neon(
        &self,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        reps: [u32; 3],
        current_abs_end: usize,
        min_match_len: usize,
        mut f: impl FnMut(MatchCandidate),
    ) {
        let _ = self;
        super::hc::generator::for_each_repcode_candidate_body!(
            table,
            abs_pos,
            lit_len,
            reps,
            current_abs_end,
            min_match_len,
            f,
            crate::encoding::fastpath::neon::common_prefix_len_ptr,
        )
    }

    /// SSE4.2 umbrella variant.
    ///
    /// # Safety
    /// Caller must be running on x86/x86_64 with SSE4.2 available.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn for_each_repcode_candidate_with_reps_sse42(
        &self,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        reps: [u32; 3],
        current_abs_end: usize,
        min_match_len: usize,
        mut f: impl FnMut(MatchCandidate),
    ) {
        let _ = self;
        super::hc::generator::for_each_repcode_candidate_body!(
            table,
            abs_pos,
            lit_len,
            reps,
            current_abs_end,
            min_match_len,
            f,
            crate::encoding::fastpath::sse42::common_prefix_len_ptr,
        )
    }

    /// AVX2+BMI2 umbrella variant.
    ///
    /// # Safety
    /// Caller must be running on x86/x86_64 with AVX2 + BMI2 available.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn for_each_repcode_candidate_with_reps_avx2_bmi2(
        &self,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        reps: [u32; 3],
        current_abs_end: usize,
        min_match_len: usize,
        mut f: impl FnMut(MatchCandidate),
    ) {
        let _ = self;
        super::hc::generator::for_each_repcode_candidate_body!(
            table,
            abs_pos,
            lit_len,
            reps,
            current_abs_end,
            min_match_len,
            f,
            crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
        )
    }

    /// Scalar fallback used on non-AArch64 targets.
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_each_repcode_candidate_with_reps_scalar(
        &self,
        table: &MatchTable,
        abs_pos: usize,
        lit_len: usize,
        reps: [u32; 3],
        current_abs_end: usize,
        min_match_len: usize,
        mut f: impl FnMut(MatchCandidate),
    ) {
        let _ = self;
        super::hc::generator::for_each_repcode_candidate_body!(
            table,
            abs_pos,
            lit_len,
            reps,
            current_abs_end,
            min_match_len,
            f,
            crate::encoding::fastpath::scalar::common_prefix_len_ptr,
        )
    }

    /// Walk a candidate match backwards over the literal run so the
    /// matcher can absorb literal bytes that happen to match the byte
    /// preceding the candidate. Upstream zstd parity: equivalent to the back
    /// extend inside `ZSTD_HcFindBestMatch` before committing a
    /// sequence.
    ///
    /// Takes `&MatchTable` because the only thing it needs is read
    /// access to the contiguous history mirror — kept off `&self` so
    /// callers don't have to hand it an HcMatcher reference they
    /// don't otherwise use.
    pub(crate) fn extend_backwards(
        table: &MatchTable,
        candidate_pos: usize,
        abs_pos: usize,
        match_len: usize,
        lit_len: usize,
    ) -> MatchCandidate {
        crate::encoding::match_table::helpers::extend_backwards_shared(
            table.live_history(),
            table.history_abs_start,
            candidate_pos,
            abs_pos,
            match_len,
            lit_len,
        )
    }

    /// Upstream zstd parity: per-pass clamp of the "good enough — stop probing"
    /// threshold that the optimal parser passes to the BT/HC walkers.
    /// Reflects upstream zstd `ZSTD_compressBlock_opt_generic` which caps the
    /// profile's `sufficient_match_len` by the user-configured
    /// `targetLength` and the `HC_OPT_NUM` ceiling.
    pub(crate) fn sufficient_match_len_for_pass(&self, profile: HcOptimalCostProfile) -> usize {
        profile
            .sufficient_match_len
            .min(self.target_len)
            .clamp(HC_FORMAT_MINMATCH, HC_OPT_NUM - 1)
    }
}
