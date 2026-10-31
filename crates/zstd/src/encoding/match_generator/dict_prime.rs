#![allow(
    clippy::match_wildcard_for_single_variants,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Dictionary priming and CDict-equivalent snapshot lifecycle for the
//! [`MatchGeneratorDriver`].
//!
//! Holds the matcher's dictionary entry points (prime / restore / capture /
//! invalidate / resident reapply / entropy seed) as inherent `*_impl` helpers;
//! the parent module's trait surface forwards to them. Kept as a child module
//! so the snapshot path can touch the driver's private invariant-bearing
//! `primed` / `reset_shape` fields directly without widening them to the rest
//! of the encoding module.

use super::{MIN_MATCH_LEN, MatchGeneratorDriver, Matcher, MatcherStorage, PrimedKey};

impl MatchGeneratorDriver {
    pub(super) fn dictionary_is_resident_impl(&self) -> bool {
        match &self.storage {
            MatcherStorage::HashChain(hc) => hc.table.dict_resident,
            MatcherStorage::Simple(s) => s.dict_resident(),
            MatcherStorage::Dfast(d) => d.dict_resident(),
            _ => false,
        }
    }

    pub(super) fn reapply_resident_dictionary_impl(&mut self, offset_hist: [u32; 3]) {
        // Same offset-history head as `prime_with_dictionary`, without the dict
        // commit / re-index (resident dict bytes + cached dms already in place).
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => {
                self.simple_mut().prime_offset_history(offset_hist)
            }
            super::super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().offset_hist = offset_hist
            }
            super::super::strategy::BackendTag::Row => {
                self.row_matcher_mut().offset_hist = offset_hist
            }
            super::super::strategy::BackendTag::HashChain => {
                let matcher = self.hc_matcher_mut();
                matcher.table.offset_hist = offset_hist;
                matcher.table.mark_dictionary_primed();
            }
        }
        // Restore the retained-dictionary budget the per-frame `reset` cleared.
        // The matcher's `reset` re-inflated `max_window_size` by the resident
        // dict region (so the dict + next input both stay in the eviction band),
        // exactly as `prime_with_dictionary` does — but the resident path skips
        // that prime, so without this the driver-level budget stays 0 and
        // `retire_dictionary_budget` never shrinks the inflated window as input
        // evicts the dict. For HashChain (whose `window_low` is measured against
        // `max_window_size`), a stuck-inflated window would let a post-eviction
        // match exceed the frame header's base window and emit an over-window
        // offset. The inflation equals `max_window_size - base`, and
        // `reported_window_size` is the base `1 << window_log` set by `reset`.
        let base = self.reported_window_size;
        let inflated = match self.active_backend() {
            super::super::strategy::BackendTag::Simple => self.simple_mut().max_window_size,
            super::super::strategy::BackendTag::Dfast => self.dfast_matcher_mut().max_window_size,
            super::super::strategy::BackendTag::Row => self.row_matcher_mut().max_window_size,
            super::super::strategy::BackendTag::HashChain => {
                self.hc_matcher_mut().table.max_window_size
            }
        };
        self.dictionary_retained_budget = inflated.saturating_sub(base);
    }

    pub(super) fn prime_with_dictionary_impl(
        &mut self,
        dict_content: &[u8],
        offset_hist: [u32; 3],
    ) {
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => {
                // Routes through prime_offset_history so BOTH
                // offset_hist (wire encoder) and rep[0..2] (kernel)
                // are updated atomically. Without this, the two
                // tracks drift after dict priming — kernel emits
                // repcode matches against stale FAST_INITIAL_REP
                // while the wire encoder uses the primed history,
                // producing divergent wire encoding.
                self.simple_mut().prime_offset_history(offset_hist);
            }
            super::super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().offset_hist = offset_hist
            }
            super::super::strategy::BackendTag::Row => {
                self.row_matcher_mut().offset_hist = offset_hist
            }
            super::super::strategy::BackendTag::HashChain => {
                let matcher = self.hc_matcher_mut();
                // Clear the chain/hash tables (deferred from the dict-active
                // `reset`): prime rebuilds them from the dict, so they must start
                // empty. The reuse hot path skips prime and `clone_from`s a clean
                // snapshot instead, so only the first-prime / key-mismatch frames
                // pay the fill -- not every reused-CDict frame.
                matcher.table.clear_chain_hash_tables();
                matcher.table.offset_hist = offset_hist;
                matcher.table.mark_dictionary_primed();
            }
        }

        if dict_content.is_empty() {
            return;
        }

        // Dictionary bytes should stay addressable until produced frame output
        // itself exceeds the live window size. We bump `max_window_size`
        // by the dictionary length so the eviction band keeps the
        // primed bytes in `history`.
        //
        // Cap: `with_params`/`reset` enforce `window_log <= 30` so the
        // eviction band `2 * max_window_size` stays below `u32::MAX`
        // with headroom for one MAX_BLOCK_SIZE pending block — the
        // kernel asserts `data.len() <= u32::MAX`. A large enough
        // dictionary could otherwise push `max_window_size` past
        // that ceiling via the `saturating_add` below and silently
        // re-introduce the same overflow the `window_log` cap was
        // designed to prevent. Clamp the post-priming size so the
        // doubled-band-plus-block invariant survives.
        use super::super::match_table::storage::MAX_PRIMED_WINDOW_SIZE;

        // `requested_dict_budget` is what the caller asked for;
        // `base_max_window_size` snapshots the pre-priming cap so we
        // can compute how much window the cap actually GRANTED below.
        // The cap may clip the requested growth, in which case the
        // bookkeeping (`dictionary_retained_budget` retire path) must
        // track only the granted portion — otherwise
        // `retire_dictionary_budget()` would later reclaim more than
        // was actually added and shrink the matcher below its real
        // base window (and `cap = 2 * max_window_size` would shrink
        // with it, risking under-allocation on subsequent commits).
        // The `granted_retained_budget` calculation further below is
        // the load-bearing piece — see its block-level comment for
        // the post-clip / post-uncommitted-tail math.
        let requested_dict_budget = dict_content.len();
        let base_max_window_size = match self.active_backend() {
            super::super::strategy::BackendTag::Simple => self.simple_mut().max_window_size,
            super::super::strategy::BackendTag::Dfast => self.dfast_matcher_mut().max_window_size,
            super::super::strategy::BackendTag::Row => self.row_matcher_mut().max_window_size,
            super::super::strategy::BackendTag::HashChain => {
                self.hc_matcher_mut().table.max_window_size
            }
        };
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => {
                let matcher = self.simple_mut();
                matcher.max_window_size = matcher
                    .max_window_size
                    .saturating_add(requested_dict_budget)
                    .min(MAX_PRIMED_WINDOW_SIZE);
            }
            super::super::strategy::BackendTag::Dfast => {
                let matcher = self.dfast_matcher_mut();
                matcher.max_window_size = matcher
                    .max_window_size
                    .saturating_add(requested_dict_budget)
                    .min(MAX_PRIMED_WINDOW_SIZE);
            }
            super::super::strategy::BackendTag::Row => {
                let matcher = self.row_matcher_mut();
                matcher.max_window_size = matcher
                    .max_window_size
                    .saturating_add(requested_dict_budget)
                    .min(MAX_PRIMED_WINDOW_SIZE);
            }
            super::super::strategy::BackendTag::HashChain => {
                let matcher = self.hc_matcher_mut();
                matcher.table.max_window_size = matcher
                    .table
                    .max_window_size
                    .saturating_add(requested_dict_budget)
                    .min(MAX_PRIMED_WINDOW_SIZE);
            }
        }

        let mut start = 0usize;
        let mut committed_dict_budget = 0usize;
        // insert_position needs 4 bytes of lookahead for hashing;
        // backfill_boundary_positions re-visits tail positions once the
        // next slice extends history, but cannot hash <4 byte fragments.
        let min_primed_tail = match self.active_backend() {
            super::super::strategy::BackendTag::Simple => MIN_MATCH_LEN,
            super::super::strategy::BackendTag::Dfast
            | super::super::strategy::BackendTag::Row
            | super::super::strategy::BackendTag::HashChain => 4,
        };
        while start < dict_content.len() {
            let end = (start + self.slice_size).min(dict_content.len());
            if end - start < min_primed_tail {
                break;
            }
            // Stage the dict chunk WITHOUT `get_next_space`'s
            // `resize(slice_size, 0)` zero-fill: that memsets a full
            // block-sized buffer (up to ~128 KiB) every frame only to have it
            // `clear()`-ed and overwritten by the dict bytes on the very next
            // lines — pure waste (measured ~10% of the small dict encode).
            // Reuse a pooled buffer's capacity if one is free (the prime/skip
            // cycle recycles them back), else allocate exactly the chunk.
            // Mirrors upstream zstd, which references the CDict content rather
            // than zero-filling a fresh window per frame.
            let mut space = self.vec_pool.pop().unwrap_or_default();
            space.clear();
            space.extend_from_slice(&dict_content[start..end]);
            self.commit_space(space);
            self.skip_matching_for_dictionary_priming();
            committed_dict_budget += end - start;
            start = end;
        }

        // Derive `granted_retained_budget` directly from the two real
        // bounds — bytes actually committed and bytes the cap allows
        // — instead of doing a cap-clip pass followed by an
        // uncommitted-tail subtract. Previous shape double-discounted
        // when the cap clipped: clip lost `(requested - allowed)`,
        // then tail-subtract lost ANOTHER `(requested - committed)`,
        // leaving `max_window_size` shy of the dictionary that was
        // actually retained (e.g. cap=900, committed=998, uncommitted=2
        // landed at granted=898 instead of the correct 900).
        let capped_retained_budget = MAX_PRIMED_WINDOW_SIZE.saturating_sub(base_max_window_size);
        let granted_retained_budget = committed_dict_budget.min(capped_retained_budget);
        let final_max_window_size = base_max_window_size.saturating_add(granted_retained_budget);
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => {
                self.simple_mut().max_window_size = final_max_window_size;
            }
            super::super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().max_window_size = final_max_window_size;
            }
            super::super::strategy::BackendTag::Row => {
                self.row_matcher_mut().max_window_size = final_max_window_size;
            }
            super::super::strategy::BackendTag::HashChain => {
                self.hc_matcher_mut().table.max_window_size = final_max_window_size;
            }
        }
        if granted_retained_budget > 0 {
            self.dictionary_retained_budget = self
                .dictionary_retained_budget
                .saturating_add(granted_retained_budget);
        }
        if self.active_backend() == super::super::strategy::BackendTag::HashChain {
            // Recompute the lazy-HC attach decision made per-chunk in
            // `skip_matching_for_dictionary_priming` (stable across the prime —
            // `reset_size_log` does not change here).
            //
            // The HC attach/copy mode is deliberately NOT folded into `PrimedKey`
            // (unlike Fast `fast_attach`). Fast attach builds a separate dict
            // table whose dimensions differ from the copy-mode live table, so a
            // cross-mode restore would install mismatched table geometry and the
            // encoder could search past the frame window (undecodable). The two
            // HC modes share identical window geometry: `max_window_size` and the
            // dictionary limit are both set ABOVE this branch (the same value in
            // either mode), and the live chain table dimensions come from the
            // resolved `params` the key already pins. The modes differ only in
            // WHERE the committed dict lives — a single-link `dms` (attach) vs
            // merged into the live chain (copy) — both producing valid matches at
            // in-window offsets. Upstream zstd makes the same observation: attach
            // (`ZSTD_resetCCtx_byAttachingCDict`) and copy
            // (`ZSTD_resetCCtx_byCopyingCDict`) both keep the caller's
            // `windowLog`; the choice is a memory/speed trade-off, not a wire
            // contract. So restoring an attach snapshot where this frame would
            // have copied (or vice versa) yields a decodable frame that may only
            // differ in which matches are found (ratio) — algorithmic freedom, not
            // a defect. Keying on the mode would instead force a re-prime across
            // the cutoff, re-adding the per-frame cost this snapshot path removes.
            //
            // In practice the public reuse path (`compress_independent_frame`)
            // only ever captures AND restores the COPY-mode snapshot — capture is
            // gated on the above-cutoff source size, so a restored frame always
            // matches the captured mode. `hc_dict_snapshot_reuse_roundtrips` pins
            // that same-mode reuse decodes; the driver-level cross-mode restore is
            // accepted (not refused) per
            // `primed_snapshot_fast_attach_does_not_over_key_non_simple_backends`.
            let attach = self.hc_dict_attach_mode();
            let table = &mut self.hc_matcher_mut().table;
            table.set_dictionary_limit_from_primed_bytes(committed_dict_budget);
            // Build the dictMatchState over the committed dict (front of history)
            // so `find_best_match` dual-probes it with its own compare budget —
            // but ONLY in ATTACH mode. BT/optimal attach → DUBT dms; lazy-HC
            // attach → single-link hash-chain dms. COPY mode (large known source,
            // both BT and lazy-HC) already merged the dict into the live tree /
            // chain in `skip_matching_for_dictionary_priming`, so it carries no
            // separate dms — drop any stale one.
            if !attach {
                table.dms.invalidate();
            } else if table.uses_bt {
                table.prime_dms_bt(committed_dict_budget);
            } else {
                table.prime_dms_hc(committed_dict_budget);
            }
        }
        // CDict-equivalent: now that every dict chunk is indexed, mark the
        // Fast-backend dict table primed so the next frame's re-prime reuses
        // it (skips the re-hash) while still re-committing the dict bytes to
        // history. No-op when the attach path built no table (copy mode or a
        // sub-8-byte dict) — `mark_dict_primed` self-guards on table presence.
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => self.simple_mut().mark_dict_primed(),
            super::super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().mark_dict_primed()
            }
            super::super::strategy::BackendTag::Row => self.row_matcher_mut().mark_dict_primed(),
            _ => {}
        }
    }

    pub(super) fn restore_primed_dictionary_impl(
        &mut self,
        level: super::super::CompressionLevel,
    ) -> bool {
        // Only the (storage, dictionary_retained_budget) pair is what
        // `prime_with_dictionary` writes; restoring them reproduces the
        // post-prime state exactly. Gated on the FULL resolved key (level + the
        // resolved `LevelParams` + the active backend's table width), not just
        // the level: `reset` resolves the hint into a window/table geometry, so a
        // same-level snapshot taken at a hint that resolved to a different shape
        // carries a `storage.max_window_size` / table dimensions that no longer
        // match this reset. Restoring it would let the encoder search past the
        // frame header's window (an undecodable match), so on a key mismatch we
        // refuse and the caller re-primes.
        let Some((params, table_bits, fast_attach, ldm)) = self.reset_shape else {
            return false;
        };
        let key = PrimedKey {
            level,
            params,
            table_bits,
            fast_attach,
            ldm,
        };
        let Some((snapshot, budget, captured_key)) = &self.primed else {
            return false;
        };
        if *captured_key != key {
            return false;
        }
        let budget = *budget;
        match (&mut self.storage, snapshot) {
            // Same-variant Fast restore: copy the snapshot into the retained
            // live storage. `clone_from` reuses the history / hash-table /
            // dict-table buffers, so this is the upstream zstd CDict table-copy
            // regime's cost (pure copies) instead of a full per-frame
            // allocation + copy + drop cycle.
            (MatcherStorage::Simple(live), MatcherStorage::Simple(snap)) => {
                live.clone_from(snap);
            }
            // Same-variant HC lazy/greedy restore (non-BT): the snapshot keeps
            // the full primed hash/chain tables (capture's non-BT full clone),
            // so `clone_from` reuses the live history/hash/chain/dms buffers in
            // place — upstream zstd reuses the CDict tables rather than reallocating
            // them. This is the per-frame allocate+copy+drop that dominated
            // small `compress-dict` HC frames (5-7x vs C). BT (`uses_bt`)
            // snapshots drop their live tables, so they stay on the realloc
            // path below.
            (MatcherStorage::HashChain(live), MatcherStorage::HashChain(snap))
                if !snap.table.uses_bt =>
            {
                live.table.clone_from(&snap.table);
                live.hc.clone_from(&snap.hc);
                live.strategy_tag = snap.strategy_tag;
                // backend is `HcBackend::Hc` (zero-sized) for non-BT levels;
                // the live one is already correct for this resolved key.
            }
            (live, snapshot_storage) => {
                let mut storage = snapshot_storage.clone();
                // This arm handles the binary-tree backend. In ATTACH mode the
                // snapshot was stored WITHOUT its live hash / chain / hash3
                // tables (they hold no dictionary entries — the dict lives in
                // `dms` + history; see `capture_primed_dictionary`), so
                // `ensure_tables` re-allocates them zeroed to the snapshot's
                // geometry, exactly reproducing the post-prime state (all
                // `HC_EMPTY`). In COPY mode the snapshot retained its FULL live
                // tree (the dict was merged into it, no `dms`), so the tables are
                // already present at the right length and `ensure_tables` — which
                // only allocates on a length mismatch — leaves them untouched.
                // Either way this is a full storage replace, so no stale
                // live-table entry from a prior frame can survive.
                if let MatcherStorage::HashChain(hc) = &mut storage {
                    hc.table.ensure_tables();
                }
                // The snapshot does not retain the LDM producer (it holds no
                // dict state; see `capture_primed_dictionary`). Carry over the
                // frame's freshly-reset producer — built this frame by `reset`
                // with the same params the snapshot key pins, and empty (no
                // input processed yet), so it is equivalent to the producer
                // the snapshot was captured with.

                {
                    let fresh_ldm = if let MatcherStorage::HashChain(hc) = live {
                        hc.take_ldm_producer()
                    } else {
                        None
                    };
                    if let MatcherStorage::HashChain(hc) = &mut storage {
                        hc.set_ldm_producer(fresh_ldm);
                    }
                }
                *live = storage;
            }
        }
        self.dictionary_retained_budget = budget;
        true
    }

    pub(super) fn capture_primed_dictionary_impl(&mut self, level: super::super::CompressionLevel) {
        // No resolved shape means `reset` has not run for this frame — nothing
        // valid to key a snapshot on, so skip the capture.
        let Some((params, table_bits, fast_attach, ldm)) = self.reset_shape else {
            return;
        };
        let key = PrimedKey {
            level,
            params,
            table_bits,
            fast_attach,
            ldm,
        };
        // CDict-equivalent retained state. A binary-tree level in ATTACH mode
        // decouples the dictionary into `dms` (the upstream zstd `dictMatchState`); its
        // live hash / chain / hash3 tables carry NO dict entries
        // (`skip_matching_dict_bt` keeps the dict out of the live tree), so they
        // are pure zeros. Storing them in the snapshot wastes the full table
        // footprint (a second window-tier table set resident for the whole
        // compress). Instead, move the live tables OUT of the working storage,
        // clone only the dict-state (history + `dms` + window/offset/dict-limit),
        // then move the live tables back — the snapshot keeps just what upstream zstd's
        // CDict keeps, and `restore_primed_dictionary` re-allocates the zeroed
        // live tables. Every other case keeps the dict reachable through the live
        // structure, so the snapshot must retain the full tables (full clone):
        // lazy-HC attach (it DOES prime a hash-chain `dms`, but the live chain is
        // still the search structure, so the tables must travel) and COPY mode for
        // BOTH BT and lazy-HC (`dms` invalidated, dict merged into the live tree /
        // chain). `uses_bt && dms.is_primed()` is therefore the exact "decoupled"
        // signal — true only for the BT attach prime; lazy-HC attach primes `dms`
        // too but is intentionally NOT decoupled.
        let bt_decoupled = matches!(
            &self.storage,
            MatcherStorage::HashChain(hc) if hc.table.uses_bt && hc.table.dms.is_primed()
        );
        if bt_decoupled {
            let MatcherStorage::HashChain(hc) = &mut self.storage else {
                unreachable!("bt_decoupled implies HashChain storage");
            };
            let hash_table = core::mem::take(&mut hc.table.hash_table);
            let chain_table = core::mem::take(&mut hc.table.chain_table);
            let hash3_table = core::mem::take(&mut hc.table.hash3_table);
            // The LDM producer carries no dictionary state (LDM is not
            // dict-primed; its hash table is empty at capture), so it is not
            // retained either — `restore` reinstates the frame's freshly-reset
            // producer. Take it out so the clone does not duplicate its table.

            let ldm_producer = hc.take_ldm_producer();
            // Clone the dict-state-only storage (live tables now empty Vecs,
            // LDM producer detached).
            let snapshot = self.storage.clone();
            // Move the live tables (and LDM producer) back into the working storage.
            let MatcherStorage::HashChain(hc) = &mut self.storage else {
                unreachable!("storage variant is stable across the take/put");
            };
            hc.table.hash_table = hash_table;
            hc.table.chain_table = chain_table;
            hc.table.hash3_table = hash3_table;

            hc.set_ldm_producer(ldm_producer);
            self.primed = Some((snapshot, self.dictionary_retained_budget, key));
        } else {
            self.primed = Some((self.storage.clone(), self.dictionary_retained_budget, key));
        }
    }

    pub(super) fn invalidate_primed_dictionary_impl(&mut self) {
        self.primed = None;
        // Drop the Fast-backend CDict-equivalent table cache too: it is keyed
        // to the dictionary being removed / replaced. Left in place, the next
        // same-params `reset` would retain it and the kernel would probe a
        // dict region whose bytes are no longer re-committed to history.
        match self.active_backend() {
            super::super::strategy::BackendTag::Simple => self.simple_mut().invalidate_dict_cache(),
            super::super::strategy::BackendTag::Dfast => {
                self.dfast_matcher_mut().invalidate_dict_cache()
            }
            // Row keeps its attach index across frames (like Simple/Dfast),
            // so a dictionary swap must drop its cached dict rows too;
            // otherwise the next small/unknown-size frame reuses stale
            // attach state through `prime_dict_attach_current_block`.
            super::super::strategy::BackendTag::Row => {
                self.row_matcher_mut().invalidate_dict_cache()
            }
            // The BT dms tree is keyed to the dict bytes; `prime_dms_bt`
            // skips the rebuild while its shape matches, so a swapped
            // dictionary of the same length would otherwise keep serving the
            // OLD dictionary's tree.
            super::super::strategy::BackendTag::HashChain => {
                let table = &mut self.hc_matcher_mut().table;
                table.dms.invalidate();
                // Deactivate the dictionary state so the next `reset` does not
                // take the dict-active defer-the-table-clear branch. That branch
                // rewinds the tables to the origin and hands the clear off to a
                // following `prime_with_dictionary` / `restore_primed_dictionary`.
                // After a dictionary is removed (or replaced), the very next
                // frame may carry no dictionary, in which case neither hand-off
                // runs and the deferred clear would never execute — leaving stale
                // dict-region entries at the rewound base. Clearing the flag
                // routes that reset down the no-dictionary path instead; a
                // replacement dictionary re-arms the flag when it re-primes.
                table.dictionary_active = false;
            }
        }
    }

    pub(super) fn seed_dictionary_entropy_impl(
        &mut self,
        huff: Option<&crate::huff0::huff0_encoder::HuffmanTable>,
        ll: Option<&crate::fse::fse_encoder::FSETable>,
        ml: Option<&crate::fse::fse_encoder::FSETable>,
        of: Option<&crate::fse::fse_encoder::FSETable>,
    ) {
        if self.active_backend() == super::super::strategy::BackendTag::HashChain {
            self.hc_matcher_mut()
                .seed_dictionary_entropy(huff, ll, ml, of);
        }
    }
}
