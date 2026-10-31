#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! "Simple" (upstream zstd `ZSTD_fast`) match-finder backend selected for
//! every Fast-strategy level — `CompressionLevel::Uncompressed`,
//! `CompressionLevel::Fastest`, `CompressionLevel::Level(1)`, and
//! the negative `CompressionLevel::Level(-7..=-1)` "faster"
//! variants. All Fast levels currently resolve to the same
//! `FastKernelMatcher` with upstream zstd level-1 `hash_log = 14, mls = 7`;
//! per-level acceleration knobs (kSearchStrength dispatch,
//! 4-cursor pipelining) land in the phase 3 follow-up.
//!
//! Upstream zstd parity: the active matcher is the upstream zstd-shape
//! [`fast_kernel`] + [`fast_matcher`] pair — a single-pass kernel
//! reading a flat `Vec<u8>` history with a `Vec<u32>` hash table
//! indexed by `hash_ptr<MLS>` (multiply-shift over the first MLS
//! bytes). Replaces the legacy SuffixStore-based MatchGenerator
//! previously used here.

pub(crate) mod fast_kernel;
pub(crate) mod fast_matcher;
