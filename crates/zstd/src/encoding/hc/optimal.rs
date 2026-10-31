#![allow(
    clippy::cast_lossless,
    clippy::manual_ilog2,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Optimal-parse machinery for the binary-tree strategies (btopt / btultra /
//! btultra2): the `build_optimal_plan` DP + its per-CPU-tier kernels, the
//! candidate-collection + price-set body macros, and the `btlazy2` cost
//! helpers. Split out of `hc/generator.rs` (no behaviour change) as a second
//! `impl HcMatchGenerator` block over the same matcher, so the HC chain matcher
//! and the optimal parser live in separate files.

// The DP body macros reference many opt-parser types / cost-model constants
// UNqualified inside their `macro_rules!` bodies; rustc's `unused_imports` lint
// does not count macro-body references, so it false-positives on every such
// import (gating or removing any one breaks the lib build — the expansions
// need them). This module is macro-dense enough that per-import suppression is
// noise, so the lint is disabled module-wide here.
#![allow(unused_imports)]

use super::generator::HcMatchGenerator;
use crate::encoding::Sequence;
use crate::encoding::blocks::encode_offset_with_history;
use crate::encoding::hc::MAX_HC_SEARCH_DEPTH;
use alloc::vec::Vec;

// The DP body macros reference these opt-parser types and cost-model constants
// UNqualified inside their `macro_rules!` bodies. rustc's `unused_imports` lint
// does not count macro-body references, so it reports every one of these as
// "unused" even though each macro expansion requires them (gating or removing
// any one breaks the lib build). Suppress the false positive on the group.
#[allow(unused_imports)]
use crate::encoding::{
    bt::BtMatcher,
    cost_model::{
        HC_BITCOST_MULTIPLIER, HC_OPT_NODE_LEN, HC_OPT_NUM, HC_OPT_PRICE_ARENA_LEN,
        HC_OPT_PRICE_STRIDE, HC_PREDEF_THRESHOLD, HcOptState, HcOptimalCostProfile,
    },
    hc::HC_MIN_MATCH_LEN,
    levels::config::HcConfig,
    match_generator::{HC_OPT_MIN_MATCH_LEN, HC_SEARCH_DEPTH, HC_TARGET_LEN, HcBackend},
    match_table::storage::HC3_HASH_LOG,
    opt::ldm::{HcOptLdmState, HcRawSeqStore},
    opt::types::{
        HcCandidateQuery, HcOptimalNode, HcOptimalPlanBuffers, HcOptimalPlanState,
        HcOptimalSequence, MatchCandidate,
    },
};

/// Upstream zstd `offBase` for the btlazy2 lazy gain heuristic: a match whose
/// offset equals one of the three active repeat offsets prices as the cheap
/// repcode code (1/2/3); any other offset prices as `offset + 3`. So an
/// equal-length repeat-offset match always out-gains an explicit-offset one
/// (`zstd_lazy.c` `ZSTD_storeSeq` offBase convention).
#[inline]
fn btlazy2_offbase(offset: usize, reps: [u32; 3], ll0: bool) -> u32 {
    let o = offset as u32;
    // Upstream zstd repcode mapping shifts by `ll0` (zero-literal position): the cheap
    // codes become rep1 / rep2 / (rep0 - 1) instead of rep0 / rep1 / rep2,
    // because at ll0 an offset equal to rep0 is the special rep0-1 case, not
    // repcode 1. Scoring offsets against the wrong code at ll0 over-rewards a
    // rep0-distance match that does not actually encode as the cheapest code.
    if ll0 {
        if o == reps[1] {
            1
        } else if o == reps[2] {
            2
        } else if reps[0] > 1 && o == reps[0] - 1 {
            3
        } else {
            // Offsets are < window (<= 2^27), so `+ 3` never overflows u32.
            o + 3
        }
    } else if o == reps[0] {
        1
    } else if o == reps[1] {
        2
    } else if o == reps[2] {
        3
    } else {
        // Offsets are < window (<= 2^27), so `+ 3` never overflows u32.
        o + 3
    }
}

/// Upstream zstd lazy match gain (`matchLength * 4 - ZSTD_highbit32(offBase)`): the
/// selection metric that lets a shorter repeat-offset match beat a longer
/// explicit-offset one. `offBase >= 1`, so `highbit` is well-defined.
#[inline]
fn btlazy2_gain(match_len: usize, offset: usize, reps: [u32; 3], ll0: bool) -> i64 {
    let offbase = btlazy2_offbase(offset, reps, ll0);
    (match_len as i64) * 4 - (31 - offbase.leading_zeros()) as i64
}

/// Per-kernel body of the `btlazy2` (levels 13-15) greedy/lazy parse over
/// the binary-tree match finder. Mirrors `build_optimal_plan_impl_body!`'s
/// kernel-dispatch discipline: the wrapper carries the `#[target_feature]`
/// umbrella and passes its tier-specific `collect_optimal_candidates_initialized_<kernel>`
/// as `$collect`, so the per-position BT collect (and its inlined cpl)
/// stays under one umbrella — the runtime `select_kernel()` dispatch happens
/// ONCE per block in the bare `start_matching_btlazy2`, never per position.
macro_rules! start_matching_btlazy2_body {
    ($self:ident, $handle_sequence:ident, $collect:ident, $cmf:path $(,)?) => {{
        $self.table.ensure_tables();
        // Borrowed-aware: owned → last committed chunk; borrowed → staged block.
        let (current_abs_start, current_len) = $self.table.current_block_range();
        if current_len == 0 {
            return;
        }
        let current_ptr = $self.table.get_last_space().as_ptr();
        // Mutates tables but never reallocates `history`, so this tail slice
        // stays valid for the routine's duration (same as the other parsers).
        let current: &[u8] = unsafe { core::slice::from_raw_parts(current_ptr, current_len) };
        // Full contiguous live region (owned: dict + prior blocks + current
        // block in `history`; borrowed: `[0, block_end)` of the in-place
        // input) as a raw slice, for the explicit repcode probe: a rep offset
        // can point before the current block, which `current` can't reach.
        // `live_history()` is borrowed-aware; reborrow-then-raw-ptr so the
        // slice holds NO borrow and coexists with the `&mut self` collector
        // calls below. Same no-realloc validity contract as `current`.
        let history_abs_start = $self.table.history_abs_start;
        let concat_full: &[u8] = unsafe {
            let lh = $self.table.live_history();
            core::slice::from_raw_parts(lh.as_ptr(), lh.len())
        };
        let current_abs_end = current_abs_start + current_len;
        $self
            .table
            .apply_limited_update_after_long_match(current_abs_start);
        $self
            .table
            .backfill_boundary_positions(current_abs_start, current_abs_end);

        let profile =
            HcOptimalCostProfile::const_for_strategy::<crate::encoding::strategy::Btlazy2>();
        let mut candidates = core::mem::take(&mut $self.backend.bt_mut().opt_candidates_scratch);

        let depth = $self.hc.lazy_depth as usize;
        let mut pos = 0usize;
        let mut literals_start = 0usize;

        // Collect + select the highest-GAIN match at a position (upstream zstd
        // `ZSTD_searchMax` plus the explicit offset_1 repcode check): scan the
        // length-sorted BT/dms ladder by gain, then probe rep0 directly since
        // the ladder's strictly-increasing-length filter drops short cheap
        // reps. Expands to `(match_len, offset)`; `match_len == 0` = no match.
        macro_rules! bt_select {
            ($p:expr) => {{
                let sel_pos: usize = $p;
                // `ll0` (upstream zstd): zero literals pending before this position, so
                // the repcode set is shifted (see `btlazy2_offbase`).
                let ll0 = sel_pos == literals_start;
                let sel_abs = current_abs_start + sel_pos;
                candidates.clear();
                let query = HcCandidateQuery {
                    reps: $self.table.offset_hist,
                    lit_len: sel_pos - literals_start,
                    // No LDM seed: L13-15 run at windowLog 22, below upstream zstd's
                    // LDM auto-enable threshold (windowLog >= 27).
                    ldm_candidate: None,
                };
                // SAFETY: called inside the wrapper's `#[target_feature]`
                // umbrella (the scalar wrapper's `$collect` is a safe fn).
                unsafe {
                    $self.$collect::<crate::encoding::strategy::Btlazy2>(
                        sel_abs,
                        current_abs_end,
                        profile,
                        query,
                        &mut candidates,
                    );
                }
                let reps = $self.table.offset_hist;
                let mut sel_ml = 0usize;
                let mut sel_off = 0usize;
                let mut sel_gain = i64::MIN;
                for c in candidates.iter() {
                    let ml = c.match_len.min(current_len - sel_pos);
                    if ml < HC_OPT_MIN_MATCH_LEN {
                        continue;
                    }
                    let g = btlazy2_gain(ml, c.offset, reps, ll0);
                    if g > sel_gain {
                        sel_gain = g;
                        sel_ml = ml;
                        sel_off = c.offset;
                    }
                }
                let sel_idx = sel_abs - history_abs_start;
                // Upstream zstd probes `rep[0 + ll0]` directly (the length-sorted ladder
                // drops short cheap reps): rep0 normally, rep1 at a zero-literal
                // position where rep0 is not the cheapest code.
                let probe_rep = if ll0 {
                    reps[1] as usize
                } else {
                    reps[0] as usize
                };
                if probe_rep != 0 && sel_idx >= probe_rep {
                    let tail = current_len - sel_pos;
                    // SAFETY: `sel_idx - probe_rep < sel_idx`, `sel_idx + tail <=
                    // concat_full.len()`; same overshoot slack the collector
                    // relies on for this block.
                    let rep_ml =
                        unsafe { $cmf(concat_full, sel_idx, sel_idx - probe_rep, tail, 0) };
                    if rep_ml >= HC_OPT_MIN_MATCH_LEN
                        && btlazy2_gain(rep_ml, probe_rep, reps, ll0) > sel_gain
                    {
                        sel_ml = rep_ml;
                        sel_off = probe_rep;
                    }
                }
                (sel_ml, sel_off)
            }};
        }

        while pos + HC_OPT_MIN_MATCH_LEN <= current_len {
            let (mut best_ml, mut best_off) = bt_select!(pos);
            if best_ml < HC_OPT_MIN_MATCH_LEN {
                pos += 1;
                continue;
            }
            // Lazy lookahead (upstream zstd depth 1/2): advance one byte and accept the
            // later match only if it out-gains the current one by the upstream zstd
            // margin (deferring costs an extra literal — `+4` at depth 1, `+7`
            // at depth 2). `start` tracks where the chosen match begins.
            let mut start = pos;
            let mut d = 0usize;
            while d < depth && start + 1 + HC_OPT_MIN_MATCH_LEN <= current_len {
                let look = start + 1;
                let (ml2, off2) = bt_select!(look);
                if ml2 < HC_OPT_MIN_MATCH_LEN {
                    break;
                }
                let reps = $self.table.offset_hist;
                let margin = if d == 0 { 4 } else { 7 };
                // `best` sits at `start` (ll0 iff no literals precede it); the
                // lookahead match at `start + 1` always has a pending literal.
                let gain1 = btlazy2_gain(best_ml, best_off, reps, start == literals_start) + margin;
                let gain2 = btlazy2_gain(ml2, off2, reps, false);
                if gain2 > gain1 {
                    best_ml = ml2;
                    best_off = off2;
                    start = look;
                    d += 1;
                } else {
                    break;
                }
            }
            // Commit the chosen match at `start`; [literals_start, start) is
            // emitted as literals. `best_ml` was bounded to `current_len -
            // start` at selection, so `start + best_ml <= current_len`.
            let lit_len = start - literals_start;
            let literals = &current[literals_start..start];
            $handle_sequence(Sequence::Triple {
                literals,
                offset: best_off,
                match_len: best_ml,
            });
            let _ = encode_offset_with_history(
                best_off as u32,
                lit_len as u32,
                &mut $self.table.offset_hist,
            );
            pos = start + best_ml;
            literals_start = pos;
        }

        if literals_start < current_len {
            $handle_sequence(Sequence::Literals {
                literals: &current[literals_start..],
            });
        }
        $self.backend.bt_mut().opt_candidates_scratch = candidates;
    }};
}

macro_rules! build_optimal_plan_impl_body {
    (
        $self:expr,
        $strategy_ty:ty,
        $current:ident,
        $current_abs_start:ident,
        $current_len:ident,
        $initial_state:ident,
        $stats:ident,
        $out:ident,
        $buffers:expr,
        $collect:ident,
        $priceset:path $(,)?
    ) => {{
        let current_abs_end = $current_abs_start + $current_len;
        let min_match_len = HC_OPT_MIN_MATCH_LEN;
        // `HC_OPT_NUM > 0` by const definition, so `HC_OPT_NUM - 1` is safe.
        let frontier_limit = $current_len.min(HC_OPT_NUM - 1);
        let initial_reps = $initial_state.reps;
        let initial_litlen = $initial_state.litlen;
        let ldm_block_offset = $initial_state.block_offset;
        let mut profile = $initial_state.profile;
        profile.sufficient_match_len = $self.hc.sufficient_match_len_for_pass(profile);
        // Const-fold from the strategy's associated `OPT_LEVEL`
        // (upstream zstd `optLevel`): BtOpt = 0, BtUltra / BtUltra2 = 2.
        // The two flags below are the only places the inner DP loop
        // used to consult `parse_mode`; lifting them into const
        // expressions drops one indirect read + one branch on every
        // candidate insertion and every traceback step.
        // `let` (not `const`) — nested `const` items inside a
        // generic fn cannot project through the outer fn's type
        // parameter, but a `let` binding from a const expression
        // does get folded by the optimiser per monomorphisation,
        // which is what we actually want here.
        debug_assert!(
            <$strategy_ty as crate::encoding::strategy::Strategy>::USE_BT,
            "build_optimal_plan_impl_body called on non-BT strategy"
        );
        let abort_on_worse_match: bool =
            <$strategy_ty as crate::encoding::strategy::Strategy>::OPT_LEVEL == 0;
        let opt_level: bool = <$strategy_ty as crate::encoding::strategy::Strategy>::OPT_LEVEL >= 2;
        // `frontier_limit + 2 <= HC_OPT_NODE_LEN` — bounded by const.
        let frontier_buffer_size = frontier_limit + 2;
        // The five DP scratch buffers are owned one level up
        // (`start_matching_optimal` / `run_btultra2_seed_pass` take them once per
        // block via `take_optimal_plan_buffers`, size them, and reuse them across
        // every per-segment call, then restore them once). Borrow the fields here
        // and reset the per-call ones. Hoisting the take/restore out of this
        // per-segment body removes five `mem::take`s plus five restores per call,
        // which on near-random input runs once per literal.
        let HcOptimalPlanBuffers {
            nodes,
            node_prices,
            candidates,
            store,
            price_arena,
        } = &mut *$buffers;
        candidates.clear();
        store.clear();
        // Price-cache slices + monotonic stamps feed ONLY the priced paths (the
        // matched-seed block and the forward DP loop), both of which run only
        // when the seed found a candidate. On a no-match seed (the per-literal
        // case that dominates near-random input, where the forward loop is
        // skipped) they are never touched, so their setup (the arena slice
        // creation and the three stamp bumps) is deferred into the
        // `!candidates.is_empty()` block below. Declared here so they stay in
        // scope for the forward DP loop; the empty slices / zero stamps are
        // never read on the no-match path.
        let mut ll_cache: &mut [[u32; 2]] = &mut [];
        let mut ml_cache: &mut [[u32; 2]] = &mut [];
        let mut ll_price_stamp = 0u32;
        let mut lit_price_stamp = 0u32;
        let mut ml_price_stamp = 0u32;
        let sufficient_len = profile.sufficient_match_len;
        // `node0_price` / `ll0_price` / `ll1_price` (and the `nodes[0]` base node)
        // feed ONLY the forward-DP and seed paths below, which run only when the
        // seed found a candidate. On a no-match seed — the per-literal case that
        // dominates incompressible / near-random input — they are never read, so
        // defer the three cached price lookups + base-node init until a candidate
        // is confirmed (mirrors upstream zstd doing the literal `ip++` without
        // touching the DP price state). Initialised to 0; assigned in the
        // `!candidates.is_empty()` block before any reader.
        let mut ll0_price = 0u32;
        let mut ll1_price = 0u32;
        let mut pos = 1usize;
        let mut last_pos = 0usize;
        let mut forced_end: Option<usize> = None;
        let mut forced_end_state: Option<HcOptimalNode> = None;
        // Price companion of `forced_end_state` (price no longer lives in the
        // node struct; tracked alongside the forced-seed node).
        let mut forced_end_price: Option<u32> = None;
        let mut seed_forced_shortest_path = false;
        // LDM presence is a frame constant (decided before entering the DP), so
        // it is a const-generic monomorph here, not a hot-loop runtime check:
        // with HAS_LDM == false every LDM branch below const-folds away and the
        // opt_ldm workspace is never built.
        let mut opt_ldm = if HAS_LDM {
            HcOptLdmState {
                seq_store: HcRawSeqStore {
                    pos: 0,
                    pos_in_sequence: 0,
                    size: $self.backend.bt_mut().ldm_sequences.len(),
                },
                ..HcOptLdmState::default()
            }
        } else {
            HcOptLdmState::default()
        };
        if HAS_LDM {
            // `ldm_sequences` are emitted in BLOCK-relative coordinates,
            // but this optimal-parser pass runs over a SEGMENT of the
            // block starting at block-offset `$block_offset` and uses
            // segment-relative positions throughout. Fast-forward the raw
            // seq-store cursor past the bytes covered by earlier segments
            // so the (segment-relative) LDM windows below land at the
            // correct positions. Idempotent: `ldm_skip_raw_seq_store_bytes`
            // recomputes from `pos = 0`, so re-running it per segment is
            // safe. Without this, every segment after the first re-applied
            // the block's leading LDM windows at the wrong offset, emitting
            // matches that copy the wrong bytes (undecodable frame).
            if ldm_block_offset > 0 {
                $self
                    .backend
                    .bt_mut()
                    .ldm_skip_raw_seq_store_bytes(&mut opt_ldm.seq_store, ldm_block_offset);
            }
            $self
                .backend
                .bt_mut()
                .ldm_get_next_match_and_update_seq_store(&mut opt_ldm, 0, $current_len);
        }

        // Upstream zstd-like seed at rPos=0: initialize frontier with matches starting
        // at current position before entering the generic forward DP loop.
        if $current_len >= min_match_len {
            let seed_ldm = if HAS_LDM {
                $self.backend.bt_mut().ldm_process_match_candidate(
                    &mut opt_ldm,
                    0,
                    $current_len,
                    min_match_len,
                )
            } else {
                None
            };
            candidates.clear();
            // SAFETY: wrapper is in the same target_feature umbrella as the
            // `$collect` kernel variant; the runtime kernel detector already
            // gated entry into the wrapper.
            unsafe {
                $self.$collect::<$strategy_ty>(
                    $current_abs_start,
                    current_abs_end,
                    profile,
                    HcCandidateQuery {
                        reps: initial_reps,
                        lit_len: initial_litlen,
                        ldm_candidate: seed_ldm,
                    },
                    &mut *candidates,
                )
            };
            if !candidates.is_empty() {
                // Deferred price-cache setup: the arena slices are two disjoint
                // STRIDE-wide regions of `price_arena` (LL, ML); the fixed STRIDE
                // keeps each code's cell at a constant offset so the monotonic
                // stamps stay valid across calls. price_arena is exactly
                // HC_OPT_PRICE_ARENA_LEN = 2 * HC_OPT_PRICE_STRIDE pairs (sized by
                // take_optimal_plan_buffers).
                // SAFETY: the two STRIDE regions are in bounds and disjoint; the
                // raw slices are confined to this priced path and never outlive
                // `price_arena` (owned by the caller for the whole call).
                let arena_base = price_arena.as_mut_ptr();
                ll_cache =
                    unsafe { core::slice::from_raw_parts_mut(arena_base, HC_OPT_PRICE_STRIDE) };
                ml_cache = unsafe {
                    core::slice::from_raw_parts_mut(
                        arena_base.add(HC_OPT_PRICE_STRIDE),
                        HC_OPT_PRICE_STRIDE,
                    )
                };
                $self.backend.bt_mut().opt_ll_price_stamp =
                    $self.backend.bt_mut().opt_ll_price_stamp.wrapping_add(1).max(1);
                ll_price_stamp = $self.backend.bt_mut().opt_ll_price_stamp;
                $self.backend.bt_mut().opt_lit_price_stamp =
                    $self.backend.bt_mut().opt_lit_price_stamp.wrapping_add(1).max(1);
                lit_price_stamp = $self.backend.bt_mut().opt_lit_price_stamp;
                $self.backend.bt_mut().opt_ml_price_stamp =
                    $self.backend.bt_mut().opt_ml_price_stamp.wrapping_add(1).max(1);
                ml_price_stamp = $self.backend.bt_mut().opt_ml_price_stamp;
                // Deferred base/seed prices: only reached on a matched seed (see
                // the declarations above). Assign before the forward DP / seed
                // paths below read them.
                node_prices[0] = BtMatcher::cached_lit_length_price(
                    profile,
                    $stats,
                    initial_litlen,
                    &mut ll_cache,
                    ll_price_stamp,
                );
                nodes[0] = HcOptimalNode {
                    litlen: initial_litlen as u32,
                    reps: initial_reps,
                    ..HcOptimalNode::default()
                };
                ll0_price = BtMatcher::cached_lit_length_price(
                    profile,
                    $stats,
                    0,
                    &mut ll_cache,
                    ll_price_stamp,
                );
                ll1_price = BtMatcher::cached_lit_length_price(
                    profile,
                    $stats,
                    1,
                    &mut ll_cache,
                    ll_price_stamp,
                );
                // `min_match_len >= HC_FORMAT_MINMATCH (3)` by invariant.
                last_pos = (min_match_len - 1).min(frontier_limit);
                for p in 1..min_match_len.min(frontier_buffer_size) {
                    BtMatcher::reset_opt_node(&mut nodes[p]);
                    // Reset the price (sole home; the node carries none).
                    node_prices[p] = u32::MAX;
                    // `initial_litlen` is the litlen carried from prior
                    // optimal-plan segments — its real bound is the
                    // current block length (the frame compressor caps
                    // block scan at `HC_BLOCKSIZE_MAX`), not the segment
                    // `current_len`. `p < min_match_len` (small constant),
                    // so the sum stays well within `u32::MAX`. Use
                    // `checked_add` FIRST so the `usize` addition itself
                    // cannot overflow on i686 (where `usize` is 32-bit
                    // and a wrapping `+` would slip past `try_from`).
                    let seed_litlen = initial_litlen
                        .checked_add(p)
                        .and_then(|s| u32::try_from(s).ok())
                        .expect("optimal parser seed litlen out of u32 range");
                    nodes[p].litlen = seed_litlen;
                }
            }

            if let Some(candidate) = candidates.last() {
                let longest_len = candidate.match_len.min($current_len);
                if longest_len > sufficient_len {
                    let off_base = BtMatcher::encode_offset_base_with_reps(
                        candidate.offset as u32,
                        initial_litlen,
                        initial_reps,
                    );
                    let off_price = profile
                        .offset_price_for::<ACCURATE_PRICE, FAVOR_SMALL_OFFSETS>($stats, off_base);
                    let ml_price = BtMatcher::cached_match_length_price(
                        profile,
                        $stats,
                        longest_len,
                        &mut ml_cache,
                        ml_price_stamp,
                    );
                    let seq_cost = BtMatcher::add_prices(
                        ll0_price,
                        profile.match_price_from_parts(off_price, ml_price, $stats),
                    );
                    let forced_price = BtMatcher::add_prices(node_prices[0], seq_cost);
                    let forced_state = HcOptimalNode {
                        off: candidate.offset as u32,
                        mlen: longest_len as u32,
                        litlen: 0,
                        reps: initial_reps,
                    };
                    if longest_len < frontier_buffer_size && forced_price < node_prices[longest_len] {
                        nodes[longest_len] = forced_state;
                        node_prices[longest_len] = forced_price;
                    }
                    forced_end = Some(longest_len);
                    forced_end_state = Some(forced_state);
                    forced_end_price = Some(forced_price);
                    seed_forced_shortest_path = true;
                }
            }
            if !seed_forced_shortest_path {
                let mut prev_max_len = min_match_len - 1;
                for candidate in candidates.iter() {
                    let max_match_len = candidate.match_len.min(frontier_limit);
                    if max_match_len < min_match_len {
                        continue;
                    }
                    let start_len = (prev_max_len + 1).max(min_match_len);
                    if start_len > max_match_len {
                        prev_max_len = prev_max_len.max(max_match_len);
                        continue;
                    }
                    if max_match_len > last_pos {
                        BtMatcher::reset_opt_nodes(
                            &mut *nodes,
                            &mut *node_prices,
                            last_pos + 1,
                            max_match_len,
                        );
                    }
                    let off_base = BtMatcher::encode_offset_base_with_reps(
                        candidate.offset as u32,
                        initial_litlen,
                        initial_reps,
                    );
                    let off_price = profile
                        .offset_price_for::<ACCURATE_PRICE, FAVOR_SMALL_OFFSETS>($stats, off_base);
                    debug_assert!(max_match_len < frontier_buffer_size);
                    let nodes0_price = node_prices[0];
                    for match_len in (start_len..=max_match_len).rev() {
                        let ml_price = BtMatcher::cached_match_length_price(
                            profile,
                            $stats,
                            match_len,
                            &mut ml_cache,
                            ml_price_stamp,
                        );
                        let seq_cost = BtMatcher::add_prices(
                            ll0_price,
                            profile.match_price_from_parts(off_price, ml_price, $stats),
                        );
                        let next_cost = BtMatcher::add_prices(nodes0_price, seq_cost);
                        let node_price = unsafe { *node_prices.get_unchecked(match_len) };
                        if match_len > last_pos || next_cost < node_price {
                            let slot = unsafe { nodes.get_unchecked_mut(match_len) };
                            *slot = HcOptimalNode {
                                off: candidate.offset as u32,
                                mlen: match_len as u32,
                                litlen: 0,
                                reps: initial_reps,
                            };
                            unsafe { *node_prices.get_unchecked_mut(match_len) = next_cost };
                            if match_len > last_pos {
                                last_pos = match_len;
                            }
                        } else if abort_on_worse_match {
                            break;
                        }
                    }
                    prev_max_len = prev_max_len.max(max_match_len);
                }
                if last_pos + 1 < frontier_buffer_size {
                    node_prices[last_pos + 1] = u32::MAX;
                }
            }
        }
        while !seed_forced_shortest_path && pos <= last_pos && pos <= frontier_limit {
            debug_assert!(pos + 1 < frontier_buffer_size);
            let prev_node = unsafe { *nodes.get_unchecked(pos - 1) };
            let prev_node_price = unsafe { *node_prices.get_unchecked(pos - 1) };
            if prev_node_price != u32::MAX {
                let lit_len = prev_node.litlen as usize + 1;
                let lit_price = {
                    let bt = $self.backend.bt_mut();
                    BtMatcher::cached_literal_price(
                        profile,
                        $stats,
                        $current[pos - 1],
                        &mut bt.opt_lit_price_scratch,
                        &mut bt.opt_lit_price_generation,
                        lit_price_stamp,
                    )
                };
                let ll_delta = BtMatcher::cached_lit_length_delta_price(
                    profile,
                    $stats,
                    lit_len,
                    &mut ll_cache,
                    ll_price_stamp,
                );
                let lit_cost = BtMatcher::add_price_delta(prev_node_price, lit_price, ll_delta);
                // `node_pos_price` is the OLD price at `pos` (before the write
                // below) — also the price of `prev_match`, the pre-overwrite copy.
                let node_pos_price = unsafe { *node_prices.get_unchecked(pos) };
                if lit_cost <= node_pos_price {
                    let prev_match = unsafe { *nodes.get_unchecked(pos) };
                    let slot = unsafe { nodes.get_unchecked_mut(pos) };
                    *slot = prev_node;
                    slot.litlen = lit_len as u32;
                    node_prices[pos] = lit_cost;
                    #[allow(clippy::collapsible_if)]
                    if opt_level
                        && prev_match.mlen > 0
                        && prev_match.litlen == 0
                        && pos < $current_len
                    {
                        if ll1_price < ll0_price {
                            let next_lit_price = {
                                let bt = $self.backend.bt_mut();
                                BtMatcher::cached_literal_price(
                                    profile,
                                    $stats,
                                    $current[pos],
                                    &mut bt.opt_lit_price_scratch,
                                    &mut bt.opt_lit_price_generation,
                                    lit_price_stamp,
                                )
                            };
                            let with1literal = BtMatcher::add_price_delta(
                                node_pos_price,
                                next_lit_price,
                                ll1_price as i32 - ll0_price as i32,
                            );
                            let ll_delta_next = BtMatcher::cached_lit_length_delta_price(
                                profile,
                                $stats,
                                lit_len + 1,
                                &mut ll_cache,
                                ll_price_stamp,
                            );
                            let with_more_literals =
                                BtMatcher::add_price_delta(lit_cost, next_lit_price, ll_delta_next);
                            let next = pos + 1;
                            let next_price = unsafe { *node_prices.get_unchecked(next) };
                            if with1literal < with_more_literals && with1literal < next_price {
                                // Upstream zstd parity (zstd_opt.c:1232): `cur >= prevMatch.mlen`.
                                debug_assert!(pos >= prev_match.mlen as usize);
                                let prev_pos = pos - prev_match.mlen as usize;
                                {
                                    let prev_state = unsafe { *nodes.get_unchecked(prev_pos) };
                                    let (_, reps_after_match) = BtMatcher::encode_offset_with_reps(
                                        prev_match.off,
                                        prev_state.litlen as usize,
                                        prev_state.reps,
                                    );
                                    let slot = unsafe { nodes.get_unchecked_mut(next) };
                                    *slot = prev_match;
                                    slot.reps = reps_after_match;
                                    slot.litlen = 1;
                                    node_prices[next] = with1literal;
                                    if next > last_pos {
                                        last_pos = next;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Memory-resident DP (upstream zstd parity): read opt[cur] fields on
            // demand instead of holding a 28-byte node copy live across the
            // per-position `$collect` call below. The held copy forced LLVM
            // to spill reps[3] + litlen around the (non-inlinable) call;
            // reading the fields fresh on each side keeps them out of the
            // cross-call live set. `nodes[pos]` is stable across `$collect`
            // (it only fills `candidates`), so post-call reads are identical.
            let base_cost = unsafe { *node_prices.get_unchecked(pos) };
            if base_cost == u32::MAX {
                pos += 1;
                continue;
            }
            {
                let base_node = unsafe { *nodes.get_unchecked(pos) };
                if base_node.mlen > 0 && base_node.litlen == 0 {
                    // Upstream zstd parity (zstd_opt.c:1255): `cur >= opt[cur].mlen`.
                    debug_assert!(pos >= base_node.mlen as usize);
                    let prev_pos = pos - base_node.mlen as usize;
                    let prev_state = unsafe { *nodes.get_unchecked(prev_pos) };
                    let (_, reps_after_match) = BtMatcher::encode_offset_with_reps(
                        base_node.off,
                        prev_state.litlen as usize,
                        prev_state.reps,
                    );
                    unsafe { nodes.get_unchecked_mut(pos).reps = reps_after_match };
                }
            }

            if pos + 8 > $current_len {
                pos += 1;
                continue;
            }

            if pos == last_pos {
                break;
            }

            let next_price = unsafe { *node_prices.get_unchecked(pos + 1) };
            // `saturating_add` is REQUIRED here, not a masked bug: `base_cost`
            // is a node price that can be the `u32::MAX` "unreachable" sentinel,
            // and saturating keeps `base_cost + margin` pinned at MAX so the
            // comparison stays correct. Plain `+` would wrap the sentinel and
            // flip the abort decision (a ratio bug / debug overflow panic).
            if abort_on_worse_match
                && next_price <= base_cost.saturating_add(HC_BITCOST_MULTIPLIER / 2)
            {
                pos += 1;
                continue;
            }

            let abs_pos = $current_abs_start + pos;
            let ldm_candidate = if HAS_LDM {
                $self.backend.bt_mut().ldm_process_match_candidate(
                    &mut opt_ldm,
                    pos,
                    $current_len - pos,
                    min_match_len,
                )
            } else {
                None
            };
            candidates.clear();
            // SAFETY: same umbrella as `$collect`. Query fields are read
            // fresh here (consumed into the call's argument) so they do not
            // stay live across the call; the post-call reads below are a
            // separate, fresh load of the same stable `nodes[pos]`.
            unsafe {
                $self.$collect::<$strategy_ty>(
                    abs_pos,
                    current_abs_end,
                    profile,
                    HcCandidateQuery {
                        reps: nodes.get_unchecked(pos).reps,
                        lit_len: nodes.get_unchecked(pos).litlen as usize,
                        ldm_candidate,
                    },
                    &mut *candidates,
                )
            };
            // Post-call reads of opt[cur]: fresh, born after `$collect`, so
            // never part of the cross-call live set (see memory-resident note
            // above). `nodes[pos]` is untouched by `$collect`.
            let base_reps = unsafe { nodes.get_unchecked(pos).reps };
            let base_litlen = unsafe { nodes.get_unchecked(pos).litlen as usize };
            if let Some(candidate) = candidates.last() {
                let longest_len = candidate.match_len.min($current_len - pos);
                if longest_len > sufficient_len
                    || pos + longest_len >= HC_OPT_NUM
                    || pos + longest_len >= $current_len
                {
                    let lit_len = base_litlen;
                    let off_base = BtMatcher::encode_offset_base_with_reps(
                        candidate.offset as u32,
                        lit_len,
                        base_reps,
                    );
                    let off_price = profile
                        .offset_price_for::<ACCURATE_PRICE, FAVOR_SMALL_OFFSETS>($stats, off_base);
                    let ml_price = BtMatcher::cached_match_length_price(
                        profile,
                        $stats,
                        longest_len,
                        &mut ml_cache,
                        ml_price_stamp,
                    );
                    let seq_cost = BtMatcher::add_prices(
                        ll0_price,
                        profile.match_price_from_parts(off_price, ml_price, $stats),
                    );
                    let forced_price = BtMatcher::add_prices(base_cost, seq_cost);
                    let end_pos = (pos + longest_len).min($current_len);
                    forced_end = Some(end_pos);
                    forced_end_state = Some(HcOptimalNode {
                        off: candidate.offset as u32,
                        mlen: longest_len as u32,
                        litlen: 0,
                        reps: base_reps,
                    });
                    forced_end_price = Some(forced_price);
                    break;
                }
            }
            let mut prev_max_len = min_match_len - 1;
            for candidate in candidates.iter() {
                // Outer loop guards `pos <= frontier_limit` (see the
                // `while ... pos <= frontier_limit` condition); the
                // subtraction below is therefore safe.
                debug_assert!(pos <= frontier_limit);
                let max_match_len = candidate
                    .match_len
                    .min($current_len - pos)
                    .min(frontier_limit - pos);
                let min_len = min_match_len;
                if max_match_len < min_len {
                    continue;
                }
                let start_len = (prev_max_len + 1).max(min_len);
                if start_len > max_match_len {
                    prev_max_len = prev_max_len.max(max_match_len);
                    continue;
                }
                let max_next = pos + max_match_len;
                if max_next > last_pos {
                    BtMatcher::reset_opt_nodes(
                        &mut *nodes,
                        &mut *node_prices,
                        last_pos + 1,
                        max_next,
                    );
                }
                let lit_len = base_litlen;
                let off_base = BtMatcher::encode_offset_base_with_reps(
                    candidate.offset as u32,
                    lit_len,
                    base_reps,
                );
                let off_price = profile
                    .offset_price_for::<ACCURATE_PRICE, FAVOR_SMALL_OFFSETS>($stats, off_base);
                debug_assert!(pos + max_match_len < frontier_buffer_size);
                if abort_on_worse_match {
                    // btopt (OPT_LEVEL == 0): reverse-iterate with early break —
                    // once a longer match stops improving, shorter ones are
                    // skipped. Order-dependent, stays scalar.
                    for match_len in (start_len..=max_match_len).rev() {
                        let next = pos + match_len;
                        let ml_price = BtMatcher::cached_match_length_price(
                            profile,
                            $stats,
                            match_len,
                            &mut ml_cache,
                            ml_price_stamp,
                        );
                        let seq_cost = BtMatcher::add_prices(
                            ll0_price,
                            profile.match_price_from_parts(off_price, ml_price, $stats),
                        );
                        let next_cost = BtMatcher::add_prices(base_cost, seq_cost);
                        let node_next_price = unsafe { *node_prices.get_unchecked(next) };
                        if next > last_pos || next_cost < node_next_price {
                            let slot = unsafe { nodes.get_unchecked_mut(next) };
                            *slot = HcOptimalNode {
                                off: candidate.offset as u32,
                                mlen: match_len as u32,
                                litlen: 0,
                                reps: base_reps,
                            };
                            unsafe { *node_prices.get_unchecked_mut(next) = next_cost };
                            if next > last_pos {
                                last_pos = next;
                            }
                        } else {
                            break;
                        }
                    }
                } else {
                    // btultra / btultra2 (OPT_LEVEL >= 2): no abort, each
                    // match_len writes a distinct node => order-independent.
                    // Dispatch to the per-tier price-set ($priceset is the
                    // tier's fn: AVX2 SoA-vector compare for the avx2 wrapper,
                    // inline scalar otherwise) — it folds into this wrapper's
                    // monomorphisation, so no call ABI / runtime feature check.
                    #[allow(unused_unsafe)]
                    {
                        last_pos = last_pos.max(unsafe {
                            $priceset(
                                &mut *node_prices,
                                &mut *nodes,
                                ml_cache,
                                ml_price_stamp,
                                profile,
                                $stats,
                                pos,
                                start_len,
                                max_match_len,
                                ll0_price,
                                off_price,
                                base_cost,
                                candidate.offset as u32,
                                base_reps,
                                last_pos,
                            )
                        });
                    }
                }
                prev_max_len = prev_max_len.max(max_match_len);
            }

            if last_pos + 1 < frontier_buffer_size {
                unsafe {
                    *node_prices.get_unchecked_mut(last_pos + 1) = u32::MAX;
                }
            }
            pos += 1;
        }

        if last_pos == 0 {
            if $current_len == 0 {
                let price = 0u32; // deferred: node_prices[0] unset on no-match; caller discards price
                return (price, initial_reps, initial_litlen, 0);
            }
            // No match at this position: it is a single literal (upstream zstd
            // `ZSTD_compressBlock_opt_generic` `if (!nbMatches) { ip++; }`). The
            // previous code recomputed a literal+length price here purely to fill
            // the returned price slot — which the per-segment caller discards
            // (`let (_, ..)`). Skip that recompute: on near-random input this
            // no-match return runs once per literal, so the two cached price
            // lookups were pure waste. Byte-identical — the tree, the carried
            // litlen (+1), and the reps are unchanged; only the discarded price
            // value differs (now the base-literal node price).
            let next_litlen = initial_litlen
                .checked_add(1)
                .expect("optimal parser next litlen out of usize range");
            // node_prices[0] is unset on a no-match seed (deferred); the caller
            // discards this price anyway.
            let price = 0u32;
            return (price, initial_reps, next_litlen, 1);
        }

        let target_pos = forced_end.unwrap_or(last_pos.min(frontier_limit));
        // Price lives in `node_prices`, not the node struct, so carry the
        // final-stretch price alongside its node (forced-seed companion or the
        // frontier price at `target_pos`).
        let (last_stretch, last_stretch_price) = if let Some(forced_state) = forced_end_state {
            (forced_state, forced_end_price.expect("forced state has a price"))
        } else {
            (nodes[target_pos], node_prices[target_pos])
        };
        if last_stretch_price == u32::MAX {
            return (u32::MAX, initial_reps, initial_litlen, $current_len);
        }

        if last_stretch.mlen == 0 {
            return (
                last_stretch_price,
                last_stretch.reps,
                last_stretch.litlen as usize,
                target_pos.min($current_len),
            );
        }

        let mut cur = target_pos.saturating_sub(last_stretch.mlen as usize);
        let end_reps = if last_stretch.litlen == 0 {
            let prev_state = nodes[cur];
            let (_, reps_after_match) = BtMatcher::encode_offset_with_reps(
                last_stretch.off,
                prev_state.litlen as usize,
                prev_state.reps,
            );
            reps_after_match
        } else {
            let tail_literals = last_stretch.litlen as usize;
            if cur < tail_literals {
                return (
                    last_stretch_price,
                    last_stretch.reps,
                    tail_literals,
                    target_pos.min($current_len),
                );
            }
            cur -= tail_literals;
            last_stretch.reps
        };
        let store_end = cur + 2;
        if store.len() <= store_end {
            store.resize(store_end + 1, HcOptimalNode::default());
        }
        let mut store_start;
        let mut stretch_pos = cur;

        if last_stretch.litlen > 0 {
            store[store_end] = HcOptimalNode {
                litlen: last_stretch.litlen,
                mlen: 0,
                ..HcOptimalNode::default()
            };
            store_start = store_end.saturating_sub(1);
            store[store_start] = last_stretch;
        }
        store[store_end] = last_stretch;
        store_start = store_end;

        loop {
            let next_stretch = nodes[stretch_pos];
            store[store_start].litlen = next_stretch.litlen;
            if next_stretch.mlen == 0 {
                break;
            }
            if store_start == 0 {
                break;
            }
            store_start -= 1;
            store[store_start] = next_stretch;
            // Parser invariant: every emitted stretch is bounded by the
            // current block, so `litlen + mlen <= current_len <=
            // HC_BLOCKSIZE_MAX (128 KiB)`. The `as usize` widening + raw
            // `+` is safe on 32-bit targets — two u32 values do NOT
            // automatically fit in `usize` on i686, the block bound is
            // what makes this addition safe.
            let litlen = next_stretch.litlen as usize;
            let mlen = next_stretch.mlen as usize;
            debug_assert!(litlen + mlen <= $current_len);
            let step = litlen + mlen;
            if step == 0 || stretch_pos < step {
                break;
            }
            stretch_pos -= step;
        }

        let mut tail_literals = initial_litlen;
        let mut store_pos = store_start;
        while store_pos <= store_end {
            let stretch = store[store_pos];
            let llen = stretch.litlen as usize;
            let mlen = stretch.mlen as usize;
            if mlen == 0 {
                tail_literals = llen;
                store_pos += 1;
                continue;
            }
            $out.push(HcOptimalSequence {
                offset: stretch.off,
                match_len: mlen as u32,
                lit_len: llen as u32,
            });
            tail_literals = 0;
            store_pos += 1;
        }
        let result = (
            last_stretch_price,
            end_reps,
            if last_stretch.litlen > 0 {
                last_stretch.litlen as usize
            } else {
                tail_literals
            },
            target_pos.min($current_len),
        );
        result
    }};
}

/// `collect_optimal_candidates_initialized` body parameterized over the per-CPU
/// kernel: the `$cpl` path is the kernel's `common_prefix_len_ptr` (used in
/// the HC chain walk fallback), and the four method-name substitutions
/// (`$bt_update`, `$bt_insert`, `$for_each_rep`, `$hash3`) route to the
/// kernel-specific wrappers of the inner helpers. With every helper under
/// the same `target_feature` umbrella, the entire per-position pipeline
/// (BT-tree fill + rep probing + hash3 probing + BT match collection /
/// HC chain walk) inlines without ABI barriers on the level22 hot path.
macro_rules! collect_optimal_candidates_initialized_body {
    (
        $self:expr,
        $strategy_ty:ty,
        $abs_pos:ident,
        $current_abs_end:ident,
        $profile:ident,
        $query:ident,
        $out:ident,
        $bt_insert_step:ident,
        $cpl:path,
        $cmf:path $(,)?
    ) => {{
        // Per-strategy compile-time const: only BtUltra2 drives the
        // hash3 short-match table. All other monomorphisations drop
        // the entire hash3 lookup block at codegen time. The relaxed
        // implication enforces only the direction we depend on:
        // if the strategy declares hash3, the table must be live.
        // The reverse (`hash3_log != 0` without `USE_HASH3`) is OK —
        // a future caller may pre-allocate hash3 storage without
        // wiring the BtUltra2 path through.
        let use_hash3: bool = <$strategy_ty as crate::encoding::strategy::Strategy>::USE_HASH3;
        debug_assert!(!$self.table.hash_table.is_empty());
        debug_assert!($self.table.hash3_log == 0 || !$self.table.hash3_table.is_empty());
        debug_assert!(
            !use_hash3 || $self.table.hash3_log != 0,
            "Strategy::USE_HASH3 = true but runtime hash3_log is 0 — call configure() first",
        );
        debug_assert!(!$self.table.chain_table.is_empty());
        let min_match_len = HC_OPT_MIN_MATCH_LEN;
        let reps = $query.reps;
        let lit_len = $query.lit_len;
        let ldm_candidate = $query.ldm_candidate;
        $out.clear();
        if $abs_pos < $self.table.skip_insert_until_abs {
            if let Some(ldm) = ldm_candidate {
                let mut best_len_for_skip = 0usize;
                let _ = crate::encoding::bt::BtMatcher::push_candidate_ladder(
                    $out,
                    &mut best_len_for_skip,
                    ldm,
                    min_match_len,
                );
            }
            return;
        }
        {
            // BT tree catch-up folded inline (was a per-position call to
            // bt_update_tree_until): insert the positions the parser skipped into
            // the binary tree before this position's search. Upstream zstd
            // ZSTD_updateTree shape; `$bt_insert_step` overshoots the target with
            // no clamp (forward skips match-covered positions), exactly as C.
            // SAFETY: caller is in the same target_feature umbrella as
            // `$bt_insert_step`; the runtime kernel detector already gated entry.
            if $self.table.skip_insert_until_abs < $self.table.history_abs_start {
                $self.table.skip_insert_until_abs = $self.table.history_abs_start;
            }
            let mut update_abs = $self.table.skip_insert_until_abs;
            let is_btultra2 = $self.table.is_btultra2;
            while update_abs < $abs_pos {
                if !$self
                    .table
                    .can_skip_rebase_check_at(update_abs, $abs_pos, is_btultra2)
                {
                    $self.table.maybe_rebase_positions(update_abs);
                }
                let forward = unsafe {
                    $self
                        .table
                        .$bt_insert_step(update_abs, $current_abs_end, $abs_pos)
                };
                update_abs += forward.max(1);
            }
            $self.table.skip_insert_until_abs = $abs_pos;
        }
        let current_idx = $abs_pos - $self.table.history_abs_start;
        if current_idx + 4 > $self.table.live_history().len() {
            if let Some(ldm) = ldm_candidate {
                let mut best_len_for_skip = 0usize;
                let _ = crate::encoding::bt::BtMatcher::push_candidate_ladder(
                    $out,
                    &mut best_len_for_skip,
                    ldm,
                    min_match_len,
                );
            }
            return;
        }
        let mut best_len_for_skip = 0usize;
        {
            // Per-position match-finder: the rep-code probe, the hash3
            // short-match probe and the binary-tree walk folded into one body
            // (upstream ZSTD_insertBtAndGetAllMatches, FORCE_INLINE_TEMPLATE into
            // the opt loop). Seeds `best_len_for_skip` and handles the
            // sufficient-match early-out internally.
            let bt_search_depth = $self.table.search_depth;
            let best_len_ref = &mut best_len_for_skip;
            crate::encoding::hc::generator::bt_insert_and_collect_matches_body!(
                $self.table,
                bt_search_depth,
                $abs_pos,
                $current_abs_end,
                $profile,
                min_match_len,
                best_len_ref,
                $out,
                reps,
                lit_len,
                use_hash3,
                $cpl,
                $cmf,
            );
        }
        if let Some(ldm) = ldm_candidate {
            let _ = crate::encoding::bt::BtMatcher::push_candidate_ladder(
                $out,
                &mut best_len_for_skip,
                ldm,
                min_match_len,
            );
        }
    }};
}
impl HcMatchGenerator {
    /// Upstream zstd `ZSTD_btlazy2` (levels 13-15): binary-tree match finder with a
    /// greedy/lazy parse. Bare dispatcher — resolves the runtime tier ONCE
    /// per block via `select_kernel()` and calls the matching
    /// `start_matching_btlazy2_<kernel>` wrapper, so the per-position BT
    /// collect runs under a single `#[target_feature]` umbrella (mirrors
    /// `build_optimal_plan_impl`). See `start_matching_btlazy2_body!` for the
    /// shared loop.
    pub(crate) fn start_matching_btlazy2(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.start_matching_btlazy2_neon(&mut handle_sequence)
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.table.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.start_matching_btlazy2_avx2_bmi2(&mut handle_sequence)
                },
                FastpathKernel::Sse42 => unsafe {
                    self.start_matching_btlazy2_sse42(&mut handle_sequence)
                },
                FastpathKernel::Scalar => self.start_matching_btlazy2_scalar(&mut handle_sequence),
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.start_matching_btlazy2_scalar(&mut handle_sequence)
        }
    }

    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    unsafe fn start_matching_btlazy2_neon(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_btlazy2_body!(
            self,
            handle_sequence,
            collect_optimal_candidates_initialized_neon,
            crate::encoding::fastpath::neon::count_match_from_indices
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    unsafe fn start_matching_btlazy2_sse42(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_btlazy2_body!(
            self,
            handle_sequence,
            collect_optimal_candidates_initialized_sse42,
            crate::encoding::fastpath::sse42::count_match_from_indices
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    unsafe fn start_matching_btlazy2_avx2_bmi2(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_btlazy2_body!(
            self,
            handle_sequence,
            collect_optimal_candidates_initialized_avx2_bmi2,
            crate::encoding::fastpath::avx2_bmi2::count_match_from_indices
        )
    }

    // Scalar wrapper: no `#[target_feature]`; `$collect` (the scalar collect)
    // is a safe fn, so the body macro's `unsafe` block is inert here. Same cfg
    // as `collect_optimal_candidates_initialized_scalar` (absent on
    // aarch64-little, where NEON is the baseline tier).
    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    #[allow(unused_unsafe)]
    pub(crate) fn start_matching_btlazy2_scalar(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        start_matching_btlazy2_body!(
            self,
            handle_sequence,
            collect_optimal_candidates_initialized_scalar,
            crate::encoding::fastpath::scalar::count_match_from_indices
        )
    }

    // The scalar arm of `run_main_loop!` wraps a safe `*_scalar` call in
    // `unsafe` (the SIMD arms need it); allow the redundant block here.
    #[allow(unused_unsafe)]
    pub(crate) fn start_matching_optimal<S: crate::encoding::strategy::Strategy>(
        &mut self,
        mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        self.table.ensure_tables();
        // Borrowed-aware: owned → last committed chunk; borrowed → staged
        // in-place block range.
        let (current_abs_start, current_len) = self.table.current_block_range();
        if current_len == 0 {
            return;
        }
        let current_ptr = self.table.get_last_space().as_ptr();
        // `start_matching_optimal()` mutates tables/state but never mutates or
        // reallocates `self.table.history`, so this tail slice remains valid for
        // the duration of the routine and avoids cloning the full block.
        let current = unsafe { core::slice::from_raw_parts(current_ptr, current_len) };

        let current_abs_end = current_abs_start + current_len;
        self.table
            .apply_limited_update_after_long_match(current_abs_start);
        let hash3_start_cursor = self
            .table
            .skip_insert_until_abs
            .max(self.table.history_abs_start);
        self.table
            .backfill_boundary_positions(current_abs_start, current_abs_end);
        self.table.next_to_update3 = hash3_start_cursor;
        // Borrow split: `prepare_ldm_candidates` needs immutable
        // access to the live history (the post-`history_start`
        // slice of `self.table.history`) while it mutates the LDM
        // bucket table owned by `self.backend.bt_mut()`. Both live
        // in disjoint fields of `Self`, so we capture the slice +
        // its base before reaching for `bt_mut()`.
        //
        // The producer operates in absolute stream coordinates
        // throughout; `live_history[0]` corresponds to absolute
        // `history_abs_start` (upstream zstd `base + dictLimit`), and the
        // abs→slice translation happens inside the producer at
        // each `live_history[..]` access. Passing the full
        // `history` Vec would index into the dead prefix (the
        // bytes already retired past `history_start`).
        let live_history = self.table.live_history();
        let history_abs_start = self.table.history_abs_start;
        self.backend.bt_mut().prepare_ldm_candidates(
            live_history,
            history_abs_start,
            current_abs_start,
            current_len,
        );

        // Take the five DP scratch buffers ONCE for both the seed pass and the
        // main pass (upstream shares one workspace across btultra2's two
        // passes). Threading the same `&mut` into the seed pass removes its
        // separate take + restore round-trip.
        let mut plan_buffers = self.take_optimal_plan_buffers();
        if self.should_run_btultra2_seed_pass::<S>(current_len) {
            self.run_btultra2_seed_pass(current, current_abs_start, current_len, &mut plan_buffers);
        }

        // Const-generic profile selection: every field is folded from
        // S's associated consts (MAX_CHAIN_DEPTH /
        // SUFFICIENT_MATCH_LEN / ACCURATE_PRICE / FAVOR_SMALL_OFFSETS),
        // so the optimiser produces the literal at codegen time
        // without a runtime match.
        let profile = HcOptimalCostProfile::const_for_strategy::<S>();
        // The DP bodies read the strategy's `FAVOR_SMALL_OFFSETS` const directly;
        // verify the runtime profile (built from the same strategy) agrees.
        debug_assert_eq!(profile.favor_small_offsets, S::FAVOR_SMALL_OFFSETS);
        let mut opt_state =
            core::mem::replace(&mut self.backend.bt_mut().opt_state, HcOptState::new());
        opt_state.rescale_freqs(current, profile);
        let mut best_plan = core::mem::take(&mut self.backend.bt_mut().opt_segment_plan_scratch);
        best_plan.clear();
        let mut plan_reps = self.table.offset_hist;
        let (mut cursor, mut plan_litlen) =
            self.table.opt_start_cursor_and_litlen(current_abs_start);
        let mut plan_literals_cursor = 0usize;
        let match_loop_limit = current_len.saturating_sub(8);
        // Frame-constant LDM presence, resolved once per block (not per segment
        // and not in the DP hot loop): drives the HAS_LDM const-generic dispatch.
        let has_ldm = !self.backend.bt_mut().ldm_sequences.is_empty();
        // Resolve the SIMD tier ONCE here, never per segment. The per-literal
        // hot loop then runs under a single kernel-monomorphized expansion
        // (calling build_optimal_plan_impl_<kernel> directly) instead of
        // hitting select_kernel()'s OnceLock atomic + a CPU-tier match on every
        // build_optimal_plan call. Mirrors the Fast matcher dispatch shape.
        macro_rules! run_main_loop {
            ($impl_wrapper:ident) => {{
                while cursor < match_loop_limit {
                    let remaining_len = current_len - cursor;
                    let segment_abs_start = current_abs_start + cursor;
                    let segment_start = best_plan.len();
                    let state = HcOptimalPlanState {
                        block_offset: cursor,
                        reps: plan_reps,
                        litlen: plan_litlen,
                        profile,
                    };
                    let (_, end_reps, end_litlen, consumed_len) =
                        match (S::ACCURATE_PRICE, S::FAVOR_SMALL_OFFSETS, has_ldm) {
                            (true, false, false) => unsafe {
                                self.$impl_wrapper::<S, true, false, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (true, false, true) => unsafe {
                                self.$impl_wrapper::<S, true, false, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (true, true, false) => unsafe {
                                self.$impl_wrapper::<S, true, true, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (true, true, true) => unsafe {
                                self.$impl_wrapper::<S, true, true, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (false, false, false) => unsafe {
                                self.$impl_wrapper::<S, false, false, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (false, false, true) => unsafe {
                                self.$impl_wrapper::<S, false, false, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (false, true, false) => unsafe {
                                self.$impl_wrapper::<S, false, true, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                            (false, true, true) => unsafe {
                                self.$impl_wrapper::<S, false, true, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut best_plan,
                                    &mut plan_buffers,
                                )
                            },
                        };
                    // On a no-match segment (the per-literal case that dominates
                    // near-random input) nothing was emitted, so the stats
                    // update is a guaranteed no-op (it early-returns on an empty
                    // plan slice). Skip the per-literal call + its marshalling.
                    if best_plan.len() > segment_start {
                        BtMatcher::update_plan_stats_segment(
                            current,
                            current_len,
                            &best_plan[segment_start..],
                            &mut plan_literals_cursor,
                            &mut plan_reps,
                            &mut opt_state,
                            profile.accurate,
                        );
                    }
                    plan_reps = end_reps;
                    plan_litlen = end_litlen;
                    cursor += consumed_len;
                }
            }};
        }
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            run_main_loop!(build_optimal_plan_impl_neon);
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.table.kernel {
                FastpathKernel::Avx2Bmi2 => run_main_loop!(build_optimal_plan_impl_avx2_bmi2),
                FastpathKernel::Sse42 => run_main_loop!(build_optimal_plan_impl_sse42),
                FastpathKernel::Scalar => run_main_loop!(build_optimal_plan_impl_scalar),
            }
        }
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        unsafe {
            run_main_loop!(build_optimal_plan_impl_simd128);
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64",
            all(target_arch = "wasm32", target_feature = "simd128")
        )))]
        {
            run_main_loop!(build_optimal_plan_impl_scalar);
        }

        self.table
            .emit_optimal_plan(current_len, &best_plan, &mut handle_sequence);
        best_plan.clear();
        self.backend.bt_mut().opt_segment_plan_scratch = best_plan;
        let _ = self
            .backend
            .bt_mut()
            .finish_optimal_plan(plan_buffers, (0, [0; 3], 0, 0));
        self.backend.bt_mut().opt_state = opt_state;
    }

    // The scalar arm of `run_seed_loop!` wraps a safe call in `unsafe`.
    #[allow(unused_unsafe)]
    fn run_btultra2_seed_pass(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        plan_buffers: &mut HcOptimalPlanBuffers,
    ) {
        // The seed pass is BtUltra2-exclusive by name (the only
        // caller is `should_run_btultra2_seed_pass`), so pin `S` to
        // `BtUltra2` for both the cost-profile lookup and the
        // kernel-dispatched segment loop below.
        type S = crate::encoding::strategy::BtUltra2;
        // `S` is a concrete type here (not a generic bound), so the `Strategy`
        // trait must be in scope to read its associated consts in
        // `run_seed_loop!`.
        use crate::encoding::strategy::Strategy;
        let seed_profile = HcOptimalCostProfile::const_for_strategy::<S>();
        debug_assert_eq!(seed_profile.favor_small_offsets, S::FAVOR_SMALL_OFFSETS);
        let mut opt_state =
            core::mem::replace(&mut self.backend.bt_mut().opt_state, HcOptState::new());
        opt_state.rescale_freqs(current, seed_profile);
        let mut seed_reps = self.table.offset_hist;
        let (mut cursor, mut seed_litlen) =
            self.table.opt_start_cursor_and_litlen(current_abs_start);
        let mut seed_literals_cursor = 0usize;
        let mut seed_plan = core::mem::take(&mut self.backend.bt_mut().opt_seed_plan_scratch);
        seed_plan.clear();
        let match_loop_limit = current_len.saturating_sub(8);
        let has_ldm = !self.backend.bt_mut().ldm_sequences.is_empty();
        // SIMD tier resolved ONCE (see start_matching_optimal): the per-literal
        // seed loop runs under a single kernel-monomorphized expansion, never
        // re-entering select_kernel() per segment.
        macro_rules! run_seed_loop {
            ($impl_wrapper:ident) => {{
                while cursor < match_loop_limit {
                    let remaining_len = current_len - cursor;
                    let segment_abs_start = current_abs_start + cursor;
                    let segment_start = seed_plan.len();
                    let state = HcOptimalPlanState {
                        block_offset: cursor,
                        reps: seed_reps,
                        litlen: seed_litlen,
                        profile: seed_profile,
                    };
                    let (_, end_reps, end_litlen, consumed_len) =
                        match (S::ACCURATE_PRICE, S::FAVOR_SMALL_OFFSETS, has_ldm) {
                            (true, false, false) => unsafe {
                                self.$impl_wrapper::<S, true, false, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (true, false, true) => unsafe {
                                self.$impl_wrapper::<S, true, false, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (true, true, false) => unsafe {
                                self.$impl_wrapper::<S, true, true, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (true, true, true) => unsafe {
                                self.$impl_wrapper::<S, true, true, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (false, false, false) => unsafe {
                                self.$impl_wrapper::<S, false, false, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (false, false, true) => unsafe {
                                self.$impl_wrapper::<S, false, false, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (false, true, false) => unsafe {
                                self.$impl_wrapper::<S, false, true, false>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                            (false, true, true) => unsafe {
                                self.$impl_wrapper::<S, false, true, true>(
                                    &current[cursor..],
                                    segment_abs_start,
                                    remaining_len,
                                    state,
                                    &opt_state,
                                    &mut seed_plan,
                                    &mut *plan_buffers,
                                )
                            },
                        };
                    // No-match segment: stats update no-ops on the empty slice
                    // and the truncate has nothing to drop; skip both.
                    if seed_plan.len() > segment_start {
                        BtMatcher::update_plan_stats_segment(
                            current,
                            current_len,
                            &seed_plan[segment_start..],
                            &mut seed_literals_cursor,
                            &mut seed_reps,
                            &mut opt_state,
                            seed_profile.accurate,
                        );
                        seed_plan.truncate(segment_start);
                    }
                    seed_reps = end_reps;
                    seed_litlen = end_litlen;
                    cursor += consumed_len;
                }
            }};
        }
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            run_seed_loop!(build_optimal_plan_impl_neon);
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.table.kernel {
                FastpathKernel::Avx2Bmi2 => run_seed_loop!(build_optimal_plan_impl_avx2_bmi2),
                FastpathKernel::Sse42 => run_seed_loop!(build_optimal_plan_impl_sse42),
                FastpathKernel::Scalar => run_seed_loop!(build_optimal_plan_impl_scalar),
            }
        }
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        unsafe {
            run_seed_loop!(build_optimal_plan_impl_simd128);
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64",
            all(target_arch = "wasm32", target_feature = "simd128")
        )))]
        {
            run_seed_loop!(build_optimal_plan_impl_scalar);
        }
        seed_plan.clear();
        self.backend.bt_mut().opt_seed_plan_scratch = seed_plan;
        // The DP scratch buffers are owned by the caller (start_matching_optimal)
        // and shared across both passes, so the seed pass leaves them in place
        // (`plan_buffers` is a borrow) instead of restoring them here.
        self.backend.bt_mut().opt_state = opt_state;

        // Upstream zstd initStats_ultra keeps the collected entropy statistics but
        // invalidates the first-pass matchfinder history before the real pass.
        self.table.position_base = self.table.history_abs_start;
        self.table.index_shift = current_len;
        self.table.next_to_update3 = current_abs_start;
        self.table.skip_insert_until_abs = current_abs_start;
        // Upstream zstd `ZSTD_initStats_ultra()` invalidates the first scan by moving
        // `window.base` back by `srcSize`, making the real pass start at
        // `curr == srcSize` instead of 0. Position 0 is therefore a valid
        // table entry in the second pass even though raw C tables reserve
        // value 0 as empty during an unshifted first pass.
        self.table.allow_zero_relative_position = true;
    }

    /// Take the five DP scratch buffers out of the backend ONCE per block and
    /// size them for the optimal-parser frontier, so the per-segment
    /// `build_optimal_plan` calls borrow and reuse them instead of taking +
    /// restoring all five every call (which on near-random input runs once per
    /// literal). Restore with [`BtMatcher::finish_optimal_plan`] after the
    /// per-segment loop.
    fn take_optimal_plan_buffers(&mut self) -> HcOptimalPlanBuffers {
        let bt = self.backend.bt_mut();
        let mut nodes = core::mem::take(&mut bt.opt_nodes_scratch);
        let mut node_prices = core::mem::take(&mut bt.opt_node_prices_scratch);
        let mut candidates = core::mem::take(&mut bt.opt_candidates_scratch);
        let store = core::mem::take(&mut bt.opt_store_scratch);
        let mut price_arena = core::mem::take(&mut bt.opt_price_arena);
        if nodes.len() < HC_OPT_NODE_LEN {
            nodes = alloc::vec![HcOptimalNode::default(); HC_OPT_NODE_LEN].into_boxed_slice();
        }
        if node_prices.len() < HC_OPT_NODE_LEN {
            node_prices = alloc::vec![u32::MAX; HC_OPT_NODE_LEN].into_boxed_slice();
        }
        if candidates.capacity() < MAX_HC_SEARCH_DEPTH {
            candidates.reserve_exact(MAX_HC_SEARCH_DEPTH - candidates.capacity());
        }
        if price_arena.len() < HC_OPT_PRICE_ARENA_LEN {
            price_arena = alloc::vec![[0u32; 2]; HC_OPT_PRICE_ARENA_LEN].into_boxed_slice();
        }
        HcOptimalPlanBuffers {
            nodes,
            node_prices,
            candidates,
            store,
            price_arena,
        }
    }

    /// NEON-umbrella DP body. Inlines
    /// `collect_optimal_candidates_initialized_neon` (and its entire
    /// per-position pipeline) directly into the DP loop.
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    #[allow(clippy::too_many_arguments)]
    unsafe fn build_optimal_plan_impl_neon<
        S: crate::encoding::strategy::Strategy,
        const ACCURATE_PRICE: bool,
        const FAVOR_SMALL_OFFSETS: bool,
        const HAS_LDM: bool,
    >(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        initial_state: HcOptimalPlanState,
        stats: &HcOptState,
        out: &mut Vec<HcOptimalSequence>,
        buffers: &mut HcOptimalPlanBuffers,
    ) -> (u32, [u32; 3], usize, usize) {
        build_optimal_plan_impl_body!(
            self,
            S,
            current,
            current_abs_start,
            current_len,
            initial_state,
            stats,
            out,
            buffers,
            collect_optimal_candidates_initialized_neon,
            crate::encoding::hc::priceset::priceset_range_nonabort_neon,
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    #[allow(clippy::too_many_arguments)]
    unsafe fn build_optimal_plan_impl_sse42<
        S: crate::encoding::strategy::Strategy,
        const ACCURATE_PRICE: bool,
        const FAVOR_SMALL_OFFSETS: bool,
        const HAS_LDM: bool,
    >(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        initial_state: HcOptimalPlanState,
        stats: &HcOptState,
        out: &mut Vec<HcOptimalSequence>,
        buffers: &mut HcOptimalPlanBuffers,
    ) -> (u32, [u32; 3], usize, usize) {
        build_optimal_plan_impl_body!(
            self,
            S,
            current,
            current_abs_start,
            current_len,
            initial_state,
            stats,
            out,
            buffers,
            collect_optimal_candidates_initialized_sse42,
            crate::encoding::hc::priceset::priceset_range_nonabort_sse41,
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    #[allow(clippy::too_many_arguments)]
    unsafe fn build_optimal_plan_impl_avx2_bmi2<
        S: crate::encoding::strategy::Strategy,
        const ACCURATE_PRICE: bool,
        const FAVOR_SMALL_OFFSETS: bool,
        const HAS_LDM: bool,
    >(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        initial_state: HcOptimalPlanState,
        stats: &HcOptState,
        out: &mut Vec<HcOptimalSequence>,
        buffers: &mut HcOptimalPlanBuffers,
    ) -> (u32, [u32; 3], usize, usize) {
        build_optimal_plan_impl_body!(
            self,
            S,
            current,
            current_abs_start,
            current_len,
            initial_state,
            stats,
            out,
            buffers,
            collect_optimal_candidates_initialized_avx2_bmi2,
            crate::encoding::hc::priceset::priceset_range_nonabort_avx2,
        )
    }

    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    // Body macros wrap callees in `unsafe { }` for the NEON/AVX/SSE
    // variants where callees are `unsafe fn`. The scalar wrappers route
    // through safe fns, so those blocks are redundant on this path.
    #[allow(unused_unsafe)]
    // The dispatch reaches this only on non-SIMD x86 (Scalar tier) and the
    // portable fallback; on wasm+simd128 the simd128 wrapper is selected, so
    // this is cfg-dead there.
    #[cfg_attr(
        all(target_arch = "wasm32", target_feature = "simd128"),
        allow(dead_code)
    )]
    #[allow(clippy::too_many_arguments)]
    fn build_optimal_plan_impl_scalar<
        S: crate::encoding::strategy::Strategy,
        const ACCURATE_PRICE: bool,
        const FAVOR_SMALL_OFFSETS: bool,
        const HAS_LDM: bool,
    >(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        initial_state: HcOptimalPlanState,
        stats: &HcOptState,
        out: &mut Vec<HcOptimalSequence>,
        buffers: &mut HcOptimalPlanBuffers,
    ) -> (u32, [u32; 3], usize, usize) {
        build_optimal_plan_impl_body!(
            self,
            S,
            current,
            current_abs_start,
            current_len,
            initial_state,
            stats,
            out,
            buffers,
            collect_optimal_candidates_initialized_scalar,
            crate::encoding::hc::priceset::priceset_range_nonabort_scalar,
        )
    }

    /// wasm `simd128`-umbrella DP body: scalar candidate collection (no wasm
    /// collect kernel) but the simd128 4-lane price-set.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    #[target_feature(enable = "simd128")]
    // With `+simd128` in the wasm baseline the shared body macro's `unsafe`
    // blocks (needed by the safe scalar wrapper) are redundant inside this
    // target_feature fn.
    #[allow(unused_unsafe)]
    #[allow(clippy::too_many_arguments)]
    unsafe fn build_optimal_plan_impl_simd128<
        S: crate::encoding::strategy::Strategy,
        const ACCURATE_PRICE: bool,
        const FAVOR_SMALL_OFFSETS: bool,
        const HAS_LDM: bool,
    >(
        &mut self,
        current: &[u8],
        current_abs_start: usize,
        current_len: usize,
        initial_state: HcOptimalPlanState,
        stats: &HcOptState,
        out: &mut Vec<HcOptimalSequence>,
        buffers: &mut HcOptimalPlanBuffers,
    ) -> (u32, [u32; 3], usize, usize) {
        build_optimal_plan_impl_body!(
            self,
            S,
            current,
            current_abs_start,
            current_len,
            initial_state,
            stats,
            out,
            buffers,
            collect_optimal_candidates_initialized_scalar,
            crate::encoding::hc::priceset::priceset_range_nonabort_simd128,
        )
    }

    /// Cross-platform entry. Picks the kernel-specific variant so the per-
    /// position pipeline (BT-tree fill, rep probing, hash3 probing, BT
    /// collect / HC chain walk) runs inside a single `target_feature`
    /// umbrella — all inner SIMD probes inline without ABI barriers.
    ///
    /// The on-encode hot path bypasses this dispatcher: `build_optimal_plan_impl_<kernel>`
    /// calls the matching `_<kernel>` variant directly. This entry is kept
    /// for the cfg(test)-only `collect_optimal_candidates` shim and any
    /// future caller that isn't already inside a kernel umbrella.
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn collect_optimal_candidates_initialized<S: crate::encoding::strategy::Strategy>(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: HcOptimalCostProfile,
        query: HcCandidateQuery,
        out: &mut Vec<MatchCandidate>,
    ) {
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        unsafe {
            self.collect_optimal_candidates_initialized_neon::<S>(
                abs_pos,
                current_abs_end,
                profile,
                query,
                out,
            )
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use crate::encoding::fastpath::FastpathKernel;
            match self.table.kernel {
                FastpathKernel::Avx2Bmi2 => unsafe {
                    self.collect_optimal_candidates_initialized_avx2_bmi2::<S>(
                        abs_pos,
                        current_abs_end,
                        profile,
                        query,
                        out,
                    )
                },
                FastpathKernel::Sse42 => unsafe {
                    self.collect_optimal_candidates_initialized_sse42::<S>(
                        abs_pos,
                        current_abs_end,
                        profile,
                        query,
                        out,
                    )
                },
                FastpathKernel::Scalar => self.collect_optimal_candidates_initialized_scalar::<S>(
                    abs_pos,
                    current_abs_end,
                    profile,
                    query,
                    out,
                ),
            }
        }
        #[cfg(not(any(
            all(target_arch = "aarch64", target_endian = "little"),
            target_arch = "x86",
            target_arch = "x86_64"
        )))]
        {
            self.collect_optimal_candidates_initialized_scalar::<S>(
                abs_pos,
                current_abs_end,
                profile,
                query,
                out,
            )
        }
    }

    /// NEON-umbrella variant. Every inner helper (`bt_update_tree_until_neon`,
    /// `for_each_repcode_candidate_with_reps_neon`, `hash3_candidate_neon`,
    /// `bt_insert_and_collect_matches_neon`, `fastpath::neon::
    /// common_prefix_len_ptr`) shares the NEON umbrella so the per-position
    /// pipeline executes as a single straight-line inline sequence.
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    unsafe fn collect_optimal_candidates_initialized_neon<
        S: crate::encoding::strategy::Strategy,
    >(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: HcOptimalCostProfile,
        query: HcCandidateQuery,
        out: &mut Vec<MatchCandidate>,
    ) {
        collect_optimal_candidates_initialized_body!(
            self,
            S,
            abs_pos,
            current_abs_end,
            profile,
            query,
            out,
            bt_insert_step_no_rebase_neon,
            crate::encoding::fastpath::neon::common_prefix_len_ptr,
            crate::encoding::fastpath::neon::count_match_from_indices,
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    unsafe fn collect_optimal_candidates_initialized_sse42<
        S: crate::encoding::strategy::Strategy,
    >(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: HcOptimalCostProfile,
        query: HcCandidateQuery,
        out: &mut Vec<MatchCandidate>,
    ) {
        collect_optimal_candidates_initialized_body!(
            self,
            S,
            abs_pos,
            current_abs_end,
            profile,
            query,
            out,
            bt_insert_step_no_rebase_sse42,
            crate::encoding::fastpath::sse42::common_prefix_len_ptr,
            crate::encoding::fastpath::sse42::count_match_from_indices,
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    unsafe fn collect_optimal_candidates_initialized_avx2_bmi2<
        S: crate::encoding::strategy::Strategy,
    >(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: HcOptimalCostProfile,
        query: HcCandidateQuery,
        out: &mut Vec<MatchCandidate>,
    ) {
        collect_optimal_candidates_initialized_body!(
            self,
            S,
            abs_pos,
            current_abs_end,
            profile,
            query,
            out,
            bt_insert_step_no_rebase_avx2_bmi2,
            crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
            crate::encoding::fastpath::avx2_bmi2::count_match_from_indices,
        )
    }

    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    // Macro emits `unsafe { }` wrappers for NEON/AVX/SSE variants; scalar
    // callees are safe so the blocks are redundant here only.
    #[allow(unused_unsafe)]
    pub(crate) fn collect_optimal_candidates_initialized_scalar<
        S: crate::encoding::strategy::Strategy,
    >(
        &mut self,
        abs_pos: usize,
        current_abs_end: usize,
        profile: HcOptimalCostProfile,
        query: HcCandidateQuery,
        out: &mut Vec<MatchCandidate>,
    ) {
        collect_optimal_candidates_initialized_body!(
            self,
            S,
            abs_pos,
            current_abs_end,
            profile,
            query,
            out,
            bt_insert_step_no_rebase_scalar,
            crate::encoding::fastpath::scalar::common_prefix_len_ptr,
            crate::encoding::fastpath::scalar::count_match_from_indices,
        )
    }
}
