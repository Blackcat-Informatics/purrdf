#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Long-distance-match (LDM) integration types for the optimal parser.
//!
//! Upstream zstd parity: mirrors `optLdm_t` and `rawSeq` from `zstd_opt.c`.
//! The optimal parser consumes pre-computed LDM sequences (one per long
//! match window) and weaves them into candidate selection via
//! `ldm_process_match_candidate`.
//!
//! Extracted from `match_generator.rs` as part of #111 Phase 1
//! (structural split). Names, fields, and derives are preserved;
//! visibility was opened to `pub(crate)` so `match_generator` can
//! import the relocated items through `super::opt::ldm::*`. The
//! matcher methods that operate on these types remain on
//! `HcMatchGenerator` for now. The actual LDM *matcher* (gear hash +
//! bucket table) is scoped for #111 Phase 5 (implements #18).

/// One raw LDM sequence: `lit_length` literal bytes followed by a back
/// reference of `match_length` bytes at the given `offset`.
#[derive(Copy, Clone)]
pub(crate) struct HcRawSeq {
    pub(crate) lit_length: usize,
    pub(crate) offset: usize,
    pub(crate) match_length: usize,
}

/// Cursor into a pre-computed list of [`HcRawSeq`] entries the optimal
/// parser walks over for the current block.
#[derive(Copy, Clone, Default)]
pub(crate) struct HcRawSeqStore {
    pub(crate) pos: usize,
    pub(crate) pos_in_sequence: usize,
    pub(crate) size: usize,
}

/// Upstream zstd `optLdm_t` parity: the active LDM window relative to the
/// current block. `(start_pos_in_block, end_pos_in_block)` mark the
/// reachable slice; `UINT_MAX` (here `usize::MAX`) signals "no LDM
/// candidate currently in flight" so the default-constructed state never
/// vetoes regular candidates.
#[derive(Copy, Clone)]
pub(crate) struct HcOptLdmState {
    pub(crate) seq_store: HcRawSeqStore,
    pub(crate) start_pos_in_block: usize,
    pub(crate) end_pos_in_block: usize,
    pub(crate) offset: usize,
}

impl Default for HcOptLdmState {
    fn default() -> Self {
        Self {
            seq_store: HcRawSeqStore::default(),
            start_pos_in_block: usize::MAX,
            end_pos_in_block: usize::MAX,
            offset: 0,
        }
    }
}
