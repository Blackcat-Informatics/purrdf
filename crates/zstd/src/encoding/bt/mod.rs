#![allow(
    clippy::inline_always,
    clippy::redundant_closure_for_method_calls,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Binary-tree match finder used by `BtOpt` / `BtUltra` / `BtUltra2`.
//!
//! Hosts the BT-side per-frame state: the upstream zstd `optStatePtr_t` cost
//! model (`opt_state`), the optimal-parser scratch buffers
//! (`opt_*_scratch` / `opt_*_generation` / `opt_*_stamp`), and the
//! LDM long-distance match buffer (`ldm_sequences`). Method bodies
//! (BT walk, `bt_insert_step_no_rebase`, `bt_update_tree_until`,
//! `build_optimal_plan*`, `collect_optimal_candidates*`,
//! `emit_optimal_plan`, …) still live on `HcMatchGenerator` and will
//! move onto `impl BtMatcher` once Stage 3b threads
//! `&mut MatchTable` through them — this stage establishes the
//! ownership boundary mirror to Stage 1 (`MatchTable`) and Stage 2
//! (`HcMatcher`).
//!
//! Upstream zstd parity reference: `lib/compress/zstd_opt.c`,
//! `ZSTD_compressBlock_opt_generic` and friends.

#![allow(dead_code)]

use alloc::vec::Vec;

use super::cost_model::{HC_MAX_LIT, HcOptState, HcOptimalCostProfile};

use super::ldm::LdmProducer;
use super::opt::ldm::{HcOptLdmState, HcRawSeq, HcRawSeqStore};
use super::opt::types::{HcOptimalNode, HcOptimalPlanBuffers, HcOptimalSequence, MatchCandidate};

/// Maximum offset reachable by the HC3 short-match probe. Upstream zstd
/// parity: keeps the 3-byte side table from emitting offsets that
/// the main BT/HC paths would address more efficiently. Used inside
/// the `hash3_candidate_body!` macro that the kernel-specific
/// variants below expand.
pub(crate) const HC3_MAX_OFFSET: usize = 1 << 18;

/// Binary-tree matcher state used by the `BtOpt` / `BtUltra` /
/// `BtUltra2` parse modes. Owns the cost model and the per-frame
/// scratch arenas; the actual BT pointer-pair table lives on the
/// shared [`super::match_table::storage::MatchTable`].
#[derive(Clone)]
pub(crate) struct BtMatcher {
    /// Upstream zstd `optStatePtr_t` — Huffman / FSE-derived literal and
    /// sequence-symbol cost tables that drive the optimal parser.
    pub(crate) opt_state: HcOptState,
    /// Per-frame scratch for the optimal-parse node stream. Fixed-size
    /// boxed slice (no `cap` field, no in-parse `resize`/realloc) sized to
    /// `HC_OPT_NODE_LEN`, mirroring upstream zstd's fixed `opt[ZSTD_OPT_NUM]`.
    pub(crate) opt_nodes_scratch: alloc::boxed::Box<[HcOptimalNode]>,
    /// SoA companion to `opt_nodes_scratch`: the running DP price for each
    /// node, split out of `HcOptimalNode` into its own contiguous `u32`
    /// array so the optimal-parser inner price-set loop can SIMD-compare a
    /// run of consecutive node prices with a single vector load (the 28-byte
    /// AoS node stride would otherwise force a strided gather). Same length
    /// as `opt_nodes_scratch`; index `i` is the price of node `i`.
    pub(crate) opt_node_prices_scratch: alloc::boxed::Box<[u32]>,
    /// Per-frame scratch for collected match candidates.
    pub(crate) opt_candidates_scratch: Vec<MatchCandidate>,
    /// Per-frame scratch for the final emitted node stream.
    pub(crate) opt_store_scratch: Vec<HcOptimalNode>,
    /// Per-segment plan buffer (parse → encode hand-off).
    pub(crate) opt_segment_plan_scratch: Vec<HcOptimalSequence>,
    /// `btultra2` seed-pass plan buffer.
    pub(crate) opt_seed_plan_scratch: Vec<HcOptimalSequence>,
    /// Single backing allocation for the LL/ML price caches as `[price,
    /// generation]` pairs (see [`HcOptimalPlanBuffers::price_arena`]).
    /// Replaces four separate `Vec<u32>` with one boxed slice of two
    /// fixed-stride pair regions: one base pointer + offsets, like upstream
    /// zstd's single opt workspace. `stamp` counters are the per-pass
    /// generation tags that let the parser skip re-zeroing the price cells.
    pub(crate) opt_price_arena: alloc::boxed::Box<[[u32; 2]]>,
    pub(crate) opt_ll_price_stamp: u32,
    /// Cached literal-symbol cost lookup (per-symbol fixed array).
    pub(crate) opt_lit_price_scratch: [u32; HC_MAX_LIT + 1],
    pub(crate) opt_lit_price_generation: [u32; HC_MAX_LIT + 1],
    pub(crate) opt_lit_price_stamp: u32,
    pub(crate) opt_ml_price_stamp: u32,
    /// Long-distance match (LDM) candidates seeded into the optimal
    /// parser. Built per-block during `start_matching_optimal` and
    /// drained as the parser advances.
    pub(crate) ldm_sequences: Vec<HcRawSeq>,
    /// LDM producer — `None` while LDM is opt-out (current
    /// default) so the table allocation only happens for callers
    /// that opt in. The producer is **never auto-constructed**:
    /// every `CompressionLevel` preset leaves this field as
    /// `None`, matching upstream `libzstd.so.1` where
    /// `ZSTD_compress(..., level)` never enables LDM. The
    /// opt-in surface (Rust parameter API) builds an
    /// `LdmProducer` from caller-supplied params and assigns it
    /// here at frame setup time; `prepare_ldm_candidates` only
    /// consumes the field, it does not construct it. See
    /// [`super::ldm::LdmProducer`].
    ///
    /// Gated behind the `hash` feature because the producer's
    /// per-window XXH64 hashing depends on the optional
    /// `twox-hash` dependency; under `default-features = false`
    /// the field disappears and the `prepare_ldm_candidates`
    /// body shrinks to the legacy `ldm_sequences.clear()` stub.
    pub(crate) ldm_producer: Option<LdmProducer>,
}

impl BtMatcher {
    /// BT/HC hash MLS (minimum-length-segment) parameter. Upstream zstd
    /// parity: even when `minMatch == 3` (btultra2), the main BT/HC
    /// hash still goes through `ZSTD_hashPtr(…, mls)` which falls
    /// back to the default `case 4` in
    /// `zstd_compress_internal.h`. The 3-byte path is a separate HC3
    /// side table only.
    pub(crate) const HASH_MLS: usize = 4;

    /// Steady-state workspace budget for one boxed matcher: the inline
    /// payload (`HcOptState` cost tables, lit-price arrays) plus the
    /// retained scratch arenas at their growth bounds — node frontier and
    /// emitted store (`HC_OPT_NODE_LEN` nodes each, including the `+2`
    /// lookahead slack), the SoA node-price companion (`HC_OPT_NODE_LEN`
    /// `u32`s), the consolidated price arena (two frontier-sized
    /// `[price, generation]` pair regions, LL and ML), the per-segment plan
    /// buffers, and the candidate ladder (`MAX_HC_SEARCH_DEPTH`). LDM is
    /// opt-in and excluded (`ldm_sequences` stays empty on every level
    /// preset). Kept next to the struct so the estimator and the real
    /// retained layout evolve together.
    pub(crate) fn estimated_workspace_bytes() -> usize {
        use super::cost_model::{HC_OPT_NODE_LEN, HC_OPT_NUM};
        use super::hc::MAX_HC_SEARCH_DEPTH;
        let frontier = HC_OPT_NUM + 1;
        size_of::<Self>()
            + 2 * HC_OPT_NODE_LEN * size_of::<HcOptimalNode>()
            + HC_OPT_NODE_LEN * size_of::<u32>()
            + 2 * frontier * size_of::<[u32; 2]>()
            + 2 * frontier * size_of::<HcOptimalSequence>()
            + MAX_HC_SEARCH_DEPTH * size_of::<MatchCandidate>()
    }

    /// Append `candidate` to `out` if it's strictly longer than the
    /// best length seen so far (and at least `min_match_len`). Maintains
    /// `best_len_for_skip` so subsequent calls only keep strictly
    /// improving candidates. Pure associated function — no BtMatcher
    /// state needed, just the candidate ladder bookkeeping.
    pub(crate) fn push_candidate_ladder(
        out: &mut Vec<MatchCandidate>,
        best_len_for_skip: &mut usize,
        candidate: MatchCandidate,
        min_match_len: usize,
    ) -> bool {
        if candidate.match_len < min_match_len {
            return false;
        }
        if candidate.match_len > *best_len_for_skip {
            out.push(candidate);
            *best_len_for_skip = candidate.match_len;
            return true;
        }
        false
    }

    pub(crate) fn new() -> Self {
        Self {
            opt_state: HcOptState::new(),
            // Empty boxed slices: no allocation until the optimal parser
            // first runs (non-BT strategies never touch these), matching
            // the prior lazy `Vec::new()` + grow behaviour.
            opt_nodes_scratch: alloc::boxed::Box::default(),
            opt_node_prices_scratch: alloc::boxed::Box::default(),
            opt_candidates_scratch: Vec::new(),
            opt_store_scratch: Vec::new(),
            opt_segment_plan_scratch: Vec::new(),
            opt_seed_plan_scratch: Vec::new(),
            opt_price_arena: alloc::boxed::Box::default(),
            opt_ll_price_stamp: 0,
            opt_lit_price_scratch: [0; HC_MAX_LIT + 1],
            opt_lit_price_generation: [0; HC_MAX_LIT + 1],
            opt_lit_price_stamp: 0,
            opt_ml_price_stamp: 0,
            ldm_sequences: Vec::new(),

            ldm_producer: None,
        }
    }

    /// Heap bytes the optimal-parser scratch buffers and the optional LDM
    /// producer hold. The fixed-size price arrays and `opt_state` are inline
    /// (counted by the owner's `size_of`), so only the `Vec` fields contribute.
    pub(crate) fn heap_size(&self) -> usize {
        let scratch = self.opt_nodes_scratch.len() * size_of::<HcOptimalNode>()
            + self.opt_node_prices_scratch.len() * size_of::<u32>()
            + self.opt_candidates_scratch.capacity() * size_of::<MatchCandidate>()
            + self.opt_store_scratch.capacity() * size_of::<HcOptimalNode>()
            + (self.opt_segment_plan_scratch.capacity() + self.opt_seed_plan_scratch.capacity())
                * size_of::<HcOptimalSequence>()
            + self.opt_price_arena.len() * size_of::<[u32; 2]>()
            + self.ldm_sequences.capacity() * size_of::<HcRawSeq>();
        // The LDM producer is only present under the `hash` feature.

        let ldm = self.ldm_producer.as_ref().map_or(0, |p| p.heap_size());

        scratch + ldm
    }

    /// Per-frame reset — clears scratch buffers, resets cost model,
    /// drops cached price stamps.
    pub(crate) fn reset(&mut self) {
        self.opt_state.reset();
        // The fixed-size `opt_nodes_scratch` / `opt_price_arena` boxed
        // slices persist across resets (no realloc churn). Per-block
        // correctness comes from the DP re-initialising the node frontier
        // it reads and from the generation stamps marking stale price
        // cells. The LL/ML stamps stay MONOTONIC across resets (never
        // zeroed): stale generation cells in the persistent arena carry
        // older, smaller stamps and so can never falsely match the next
        // pass — zeroing the stamp would risk a stale cell aliasing the
        // fresh value `1`. The inline lit price/generation arrays are
        // small and stay zeroed (self-consistent stamp reset).
        self.opt_candidates_scratch.clear();
        self.opt_store_scratch.clear();
        self.opt_segment_plan_scratch.clear();
        self.opt_seed_plan_scratch.clear();
        self.opt_lit_price_scratch = [0; HC_MAX_LIT + 1];
        self.opt_lit_price_generation = [0; HC_MAX_LIT + 1];
        self.opt_lit_price_stamp = 0;
        self.ldm_sequences.clear();

        if let Some(producer) = self.ldm_producer.as_mut() {
            producer.clear();
        }
    }

    /// Upstream zstd parity: `ZSTD_optLdm_skipRawSeqStoreBytes`. Fast-forward the
    /// raw LDM seq store cursor by `nb_bytes`, consuming whole stored
    /// sequences and leaving a partial-sequence offset in `pos_in_sequence`.
    pub(crate) fn ldm_skip_raw_seq_store_bytes(
        &self,
        seq_store: &mut HcRawSeqStore,
        nb_bytes: usize,
    ) {
        let mut curr_pos = seq_store.pos_in_sequence.saturating_add(nb_bytes);
        while curr_pos > 0 && seq_store.pos < seq_store.size {
            let curr_seq = self.ldm_sequences[seq_store.pos];
            let seq_len = curr_seq.lit_length.saturating_add(curr_seq.match_length);
            if curr_pos >= seq_len {
                curr_pos -= seq_len;
                seq_store.pos += 1;
            } else {
                seq_store.pos_in_sequence = curr_pos;
                break;
            }
        }
        if curr_pos == 0 || seq_store.pos == seq_store.size {
            seq_store.pos_in_sequence = 0;
        }
    }

    /// Upstream zstd parity: `ZSTD_optLdm_maybeAddMatch` / its preamble in
    /// `ZSTD_optLdm_getNextMatch`. Advance the per-block LDM window
    /// markers to the next raw LDM sequence and skip its literals.
    pub(crate) fn ldm_get_next_match_and_update_seq_store(
        &self,
        opt_ldm: &mut HcOptLdmState,
        curr_pos_in_block: usize,
        block_bytes_remaining: usize,
    ) {
        if opt_ldm.seq_store.size == 0 || opt_ldm.seq_store.pos >= opt_ldm.seq_store.size {
            opt_ldm.start_pos_in_block = usize::MAX;
            opt_ldm.end_pos_in_block = usize::MAX;
            return;
        }
        let curr_seq = self.ldm_sequences[opt_ldm.seq_store.pos];
        let curr_block_end_pos = curr_pos_in_block.saturating_add(block_bytes_remaining);
        let literals_bytes_remaining = curr_seq
            .lit_length
            .saturating_sub(opt_ldm.seq_store.pos_in_sequence);
        let match_bytes_remaining = if literals_bytes_remaining == 0 {
            curr_seq.match_length.saturating_sub(
                opt_ldm
                    .seq_store
                    .pos_in_sequence
                    .saturating_sub(curr_seq.lit_length),
            )
        } else {
            curr_seq.match_length
        };
        if literals_bytes_remaining >= block_bytes_remaining {
            opt_ldm.start_pos_in_block = usize::MAX;
            opt_ldm.end_pos_in_block = usize::MAX;
            self.ldm_skip_raw_seq_store_bytes(&mut opt_ldm.seq_store, block_bytes_remaining);
            return;
        }
        opt_ldm.start_pos_in_block = curr_pos_in_block.saturating_add(literals_bytes_remaining);
        opt_ldm.end_pos_in_block = opt_ldm
            .start_pos_in_block
            .saturating_add(match_bytes_remaining);
        opt_ldm.offset = curr_seq.offset;
        if opt_ldm.end_pos_in_block > curr_block_end_pos {
            opt_ldm.end_pos_in_block = curr_block_end_pos;
            self.ldm_skip_raw_seq_store_bytes(
                &mut opt_ldm.seq_store,
                curr_block_end_pos.saturating_sub(curr_pos_in_block),
            );
        } else {
            self.ldm_skip_raw_seq_store_bytes(
                &mut opt_ldm.seq_store,
                literals_bytes_remaining.saturating_add(match_bytes_remaining),
            );
        }
    }

    /// Upstream zstd parity: `ZSTD_optLdm_maybeAddMatch`. Convert the active LDM
    /// window (open/close cursors set by
    /// [`ldm_get_next_match_and_update_seq_store`]) into a usable
    /// `MatchCandidate` when the current position falls inside it.
    pub(crate) fn ldm_maybe_add_match(
        &self,
        opt_ldm: &HcOptLdmState,
        curr_pos_in_block: usize,
        min_match: usize,
    ) -> Option<MatchCandidate> {
        let _ = self;
        let pos_diff = curr_pos_in_block.saturating_sub(opt_ldm.start_pos_in_block);
        let candidate_match_length = opt_ldm
            .end_pos_in_block
            .saturating_sub(opt_ldm.start_pos_in_block)
            .saturating_sub(pos_diff);
        if curr_pos_in_block < opt_ldm.start_pos_in_block
            || curr_pos_in_block >= opt_ldm.end_pos_in_block
            || candidate_match_length < min_match
        {
            return None;
        }
        Some(MatchCandidate {
            start: curr_pos_in_block,
            offset: opt_ldm.offset,
            match_len: candidate_match_length,
        })
    }

    /// Upstream zstd parity: `ZSTD_optLdm_processMatchCandidate`. Wraps
    /// [`ldm_maybe_add_match`] with a re-seed step when the parser has
    /// stepped past the current LDM window.
    pub(crate) fn ldm_process_match_candidate(
        &self,
        opt_ldm: &mut HcOptLdmState,
        curr_pos_in_block: usize,
        remaining_bytes: usize,
        min_match: usize,
    ) -> Option<MatchCandidate> {
        if opt_ldm.seq_store.size == 0 || opt_ldm.seq_store.pos >= opt_ldm.seq_store.size {
            return None;
        }
        if curr_pos_in_block >= opt_ldm.end_pos_in_block {
            if curr_pos_in_block > opt_ldm.end_pos_in_block {
                let pos_overshoot = curr_pos_in_block.saturating_sub(opt_ldm.end_pos_in_block);
                self.ldm_skip_raw_seq_store_bytes(&mut opt_ldm.seq_store, pos_overshoot);
            }
            self.ldm_get_next_match_and_update_seq_store(
                opt_ldm,
                curr_pos_in_block,
                remaining_bytes,
            );
        }
        self.ldm_maybe_add_match(opt_ldm, curr_pos_in_block, min_match)
    }

    /// Upstream zstd parity: restore the seven per-frame scratch buffers that
    /// `build_optimal_plan_impl!` borrowed via `core::mem::take`. The
    /// passed `result` tuple is the parser's `(offset, reps, litlen,
    /// match_len)` return value — kept untouched and returned so the
    /// macro chains the move-out in a single expression.
    pub(crate) fn finish_optimal_plan(
        &mut self,
        buffers: HcOptimalPlanBuffers,
        result: (u32, [u32; 3], usize, usize),
    ) -> (u32, [u32; 3], usize, usize) {
        let HcOptimalPlanBuffers {
            nodes,
            node_prices,
            mut candidates,
            store,
            price_arena,
        } = buffers;
        candidates.clear();
        self.opt_nodes_scratch = nodes;
        self.opt_node_prices_scratch = node_prices;
        self.opt_candidates_scratch = candidates;
        self.opt_store_scratch = store;
        self.opt_price_arena = price_arena;
        result
    }

    /// Upstream zstd parity: `ZSTD_ldm_blockCompress` seeds external
    /// long-distance match candidates here when `enableLdm ==
    /// ZSTD_ps_enable`. The default Rust encoder still keeps LDM
    /// disabled (`ldm_producer = None`); when an external caller
    /// opts in through the parameter surface, the producer is delegated to via
    /// [`LdmProducer::generate_into`].
    ///
    /// While `ldm_producer.is_none()` the behaviour matches the
    /// pre-Phase-5 stub: a defensive clear of [`Self::ldm_sequences`]
    /// so cross-frame carry-over is impossible if a producer is
    /// activated mid-session.
    ///
    /// # Coordinate convention
    ///
    /// The producer operates entirely in **absolute stream
    /// coordinates** (so its bucket-table entries remain valid
    /// across window evictions — entries inserted by an earlier
    /// view of the frame are filtered by the staleness check
    /// `entry.offset < history_abs_start`, i.e. inclusive lower
    /// bound: entries at exactly `history_abs_start` survive,
    /// see `ldm::search::FindBestMatchInputs::lowest_index_abs`).
    /// The caller is expected to pass:
    ///
    /// * `live_history` — the contiguous *live* slice of the
    ///   per-frame `MatchTable::history` (`&history[history_start..]`).
    ///   `live_history[0]` corresponds to absolute position
    ///   `history_abs_start`.
    /// * `history_abs_start` — absolute stream position of
    ///   `live_history[0]`.
    /// * `current_abs_start` / `current_len` — absolute span of
    ///   the block to scan.
    ///
    /// `prepare_ldm_candidates` forwards these absolute
    /// coordinates straight to [`LdmProducer::generate_into`]; the
    /// abs→slice translation happens inside the producer only at
    /// the moment of `live_history[..]` indexing.
    pub(crate) fn prepare_ldm_candidates(
        &mut self,
        live_history: &[u8],
        history_abs_start: usize,
        current_abs_start: usize,
        current_len: usize,
    ) {
        self.ldm_sequences.clear();

        if let Some(producer) = self.ldm_producer.as_mut() {
            debug_assert!(current_abs_start >= history_abs_start);
            // MatchTable invariant: `live_history.len() ==
            // window_size`, and `current_abs_start =
            // history_abs_start + window_size − current_len`
            // (match_generator.rs around line 1330). The two
            // sides below must coincide; using `min(...)` would
            // silently truncate the scanned range and mask an
            // invariant violation. Raw `+` is safe — the frame-
            // level `check_stream_abs_headroom`
            // (`match_table/storage.rs:50`) guarantees
            // `history_abs_start + window_size +
            // STREAM_ABS_HEADROOM ≤ usize::MAX`.
            debug_assert_eq!(
                current_abs_start + current_len,
                history_abs_start + live_history.len(),
                "MatchTable invariant violation: current block range \
                 `[current_abs_start, +current_len)` must coincide with the \
                 live-history end (window_size == live_history.len())"
            );
            let block_end_abs = current_abs_start + current_len;
            producer.generate_into(
                live_history,
                history_abs_start,
                current_abs_start,
                block_end_abs,
                &mut self.ldm_sequences,
            );
        }
    }

    /// Upstream zstd parity: `ZSTD_storeSeq` — encode `actual_offset` into the
    /// upstream zstd's compact offset base (1/2/3 for rep slots, otherwise
    /// `actual_offset + 3`) and update the rolling `reps` window in
    /// lock-step. Returns `(off_base, next_reps)`. The non-rep branch
    /// uses `saturating_add` so a `u32` near `u32::MAX` (only possible
    /// via malformed external input) clamps to `u32::MAX` rather than
    /// wrapping into a small rep-code value that would silently corrupt
    /// the encoded stream.
    pub(crate) fn encode_offset_with_reps(
        actual_offset: u32,
        lit_len: usize,
        reps: [u32; 3],
    ) -> (u32, [u32; 3]) {
        let mut next_reps = reps;
        let encoded = if lit_len > 0 {
            if actual_offset == reps[0] {
                1
            } else if actual_offset == reps[1] {
                2
            } else if actual_offset == reps[2] {
                3
            } else {
                actual_offset.saturating_add(3)
            }
        } else if actual_offset == reps[1] {
            1
        } else if actual_offset == reps[2] {
            2
        } else if reps[0] > 1 && actual_offset == reps[0] - 1 {
            3
        } else {
            actual_offset.saturating_add(3)
        };

        if lit_len > 0 {
            match encoded {
                1 => {}
                2 => {
                    next_reps[1] = next_reps[0];
                    next_reps[0] = actual_offset;
                }
                _ => {
                    next_reps[2] = next_reps[1];
                    next_reps[1] = next_reps[0];
                    next_reps[0] = actual_offset;
                }
            }
        } else {
            match encoded {
                1 => {
                    next_reps[1] = next_reps[0];
                    next_reps[0] = actual_offset;
                }
                _ => {
                    next_reps[2] = next_reps[1];
                    next_reps[1] = next_reps[0];
                    next_reps[0] = actual_offset;
                }
            }
        }

        (encoded, next_reps)
    }

    /// `encode_offset_with_reps` minus the rep-history update — used in
    /// the optimal parser's per-candidate price probe where the rep
    /// window hasn't been committed yet.
    #[inline(always)]
    pub(crate) fn encode_offset_base_with_reps(
        actual_offset: u32,
        lit_len: usize,
        reps: [u32; 3],
    ) -> u32 {
        if lit_len > 0 {
            if actual_offset == reps[0] {
                1
            } else if actual_offset == reps[1] {
                2
            } else if actual_offset == reps[2] {
                3
            } else {
                actual_offset.saturating_add(3)
            }
        } else if actual_offset == reps[1] {
            1
        } else if actual_offset == reps[2] {
            2
        } else if reps[0] > 1 && actual_offset == reps[0] - 1 {
            3
        } else {
            actual_offset.saturating_add(3)
        }
    }

    /// Upstream zstd parity: replay an already-emitted plan segment through the
    /// `optStatePtr_t` stats updater so the next parse pass sees frozen
    /// counts. Pure static helper — only mutates the caller-owned
    /// `opt_state` / `reps` / `literals_start`.
    pub(crate) fn update_plan_stats_segment(
        current: &[u8],
        current_len: usize,
        plan: &[HcOptimalSequence],
        literals_start: &mut usize,
        reps: &mut [u32; 3],
        opt_state: &mut HcOptState,
        accurate: bool,
    ) {
        if plan.is_empty() {
            return;
        }
        for item in plan {
            let lit_len = item.lit_len as usize;
            let match_len = item.match_len as usize;
            // `checked_add` on both edges so a malformed / partially-built
            // plan can't overflow `usize` arithmetic before the
            // bounds guard fires. `saturating_add` would have masked
            // overflow as "clamp to usize::MAX" which then bypasses the
            // `> current_len` check.
            let Some(start) = literals_start.checked_add(lit_len) else {
                continue;
            };
            let Some(end) = start.checked_add(match_len) else {
                continue;
            };
            if end > current_len {
                continue;
            }
            let literals = &current[*literals_start..start];
            let (off_base, next_reps) =
                Self::encode_offset_with_reps(item.offset, literals.len(), *reps);
            opt_state.update_stats(literals.len(), literals, off_base, match_len);
            *reps = next_reps;
            *literals_start = end;
        }
        opt_state.set_base_prices(accurate);
    }

    #[inline(always)]
    pub(crate) fn reset_opt_nodes(
        nodes: &mut [HcOptimalNode],
        node_prices: &mut [u32],
        start: usize,
        end: usize,
    ) {
        for node in &mut nodes[start..=end] {
            Self::reset_opt_node(node);
        }
        for price in &mut node_prices[start..=end] {
            *price = u32::MAX;
        }
    }

    #[inline(always)]
    pub(crate) fn reset_opt_node(node: &mut HcOptimalNode) {
        // Price is reset separately via `node_prices` (see `reset_opt_nodes`);
        // here we only mark the slot not end-of-match. Upstream zstd parity: stale mlen
        // is ignored while the (separately-held) price is MAX and litlen != 0.
        node.litlen = u32::MAX;
    }

    #[inline(always)]
    pub(crate) fn add_price_delta(price: u32, add: u32, delta: i32) -> u32 {
        #[cfg(debug_assertions)]
        {
            let sum = i64::from(price) + i64::from(add) + i64::from(delta);
            debug_assert!((0..=i64::from(u32::MAX)).contains(&sum));
        }
        price.wrapping_add(add).wrapping_add_signed(delta)
    }

    #[inline(always)]
    pub(crate) fn add_prices(lhs: u32, rhs: u32) -> u32 {
        let sum = lhs + rhs;
        debug_assert!(sum >= lhs);
        sum
    }

    #[inline(always)]
    pub(crate) fn cached_literal_price(
        profile: HcOptimalCostProfile,
        stats: &HcOptState,
        byte: u8,
        prices: &mut [u32; HC_MAX_LIT + 1],
        generations: &mut [u32; HC_MAX_LIT + 1],
        stamp: u32,
    ) -> u32 {
        // SAFETY: `byte as usize` is `0..256` and the fixed-size arrays are
        // `[u32; HC_MAX_LIT + 1 = 257]`, so the index is statically in bounds.
        // Each cached_*_price call sits inside the optimal parser per-byte
        // hot loop where these bounds checks are pure overhead.
        let idx = byte as usize;
        unsafe {
            if *generations.get_unchecked(idx) == stamp {
                return *prices.get_unchecked(idx);
            }
            let price = profile.literal_price(stats, byte);
            *prices.get_unchecked_mut(idx) = price;
            *generations.get_unchecked_mut(idx) = stamp;
            price
        }
    }

    #[inline(always)]
    pub(crate) fn cached_lit_length_price(
        profile: HcOptimalCostProfile,
        stats: &HcOptState,
        lit_len: usize,
        cache: &mut [[u32; 2]],
        stamp: u32,
    ) -> u32 {
        if lit_len >= cache.len() {
            return profile.lit_length_price(stats, lit_len);
        }
        // SAFETY: the early-return above proves `lit_len < cache.len()`.
        // Each cell pairs `[price, generation]`, so the stamp check and the
        // price read/write hit ONE cache line instead of two separate
        // strided regions 16 KiB apart.
        unsafe {
            let cell = cache.get_unchecked_mut(lit_len);
            if cell[1] == stamp {
                return cell[0];
            }
            let price = profile.lit_length_price(stats, lit_len);
            cell[0] = price;
            cell[1] = stamp;
            price
        }
    }

    #[inline(always)]
    pub(crate) fn cached_lit_length_delta_price(
        profile: HcOptimalCostProfile,
        stats: &HcOptState,
        lit_len: usize,
        cache: &mut [[u32; 2]],
        stamp: u32,
    ) -> i32 {
        if lit_len == 0 {
            // The `lit_len == 0` branch is the rare case where computing
            // `lit_len - 1` would underflow; we feed `0` to both
            // `lit_length_price` calls so the delta is 0 by construction.
            // No need to compute `0_usize - 1`.
            return 0;
        }
        let price = Self::cached_lit_length_price(profile, stats, lit_len, cache, stamp);
        let previous = Self::cached_lit_length_price(profile, stats, lit_len - 1, cache, stamp);
        price as i32 - previous as i32
    }

    #[inline(always)]
    pub(crate) fn cached_match_length_price(
        profile: HcOptimalCostProfile,
        stats: &HcOptState,
        match_len: usize,
        cache: &mut [[u32; 2]],
        stamp: u32,
    ) -> u32 {
        if match_len >= cache.len() {
            return profile.match_length_price(stats, match_len);
        }
        // SAFETY: see `cached_lit_length_price` — paired `[price, generation]`
        // cells, one cache line per probe; early return proves
        // `match_len < cache.len()`.
        unsafe {
            let cell = cache.get_unchecked_mut(match_len);
            if cell[1] == stamp {
                return cell[0];
            }
            let price = profile.match_length_price(stats, match_len);
            cell[0] = price;
            cell[1] = stamp;
            price
        }
    }
}
