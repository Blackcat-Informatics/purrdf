#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Hash / chain / BT tables and shared match-finder helpers.
//!
//! Will host the shared `MatchTable` abstraction plus the concrete
//! storage layouts:
//!
//! - hash table (main, indexed by `ZSTD_hashPtr`-equivalent)
//! - chain table (dual-purpose: HC chain links or BT pointer pairs)
//! - hash3 side table (HC3 short-match lookup)
//! - arena allocator (cwksp parity — single `Box<[u8]>` bumped per
//!   frame so per-frame `Vec` reallocation disappears from the hot
//!   path)
//!
//! Phase 1b (this commit) populates [`helpers`] with the shared
//! match-finder primitives (back-extend, rep-code probe, lazy
//! decision) so subsequent per-matcher extractions don't have to drag
//! those helpers along. Storage abstractions are still scoped for the
//! Phase 2 arena allocator work.

#![allow(dead_code)]

pub(crate) mod helpers;
pub(crate) mod storage;
