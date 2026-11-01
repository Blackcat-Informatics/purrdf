#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Upstream zstd-shape Fast strategy block compressor — flat hash-table +
//! tight per-block loop, ported from `lib/compress/zstd_fast.c`. See
//! [`kernel::compress_block_fast`] for the entry point.
//!
//! This module isolates the Fast-strategy kernel: the
//! kernel is implemented and unit-tested in isolation but not yet
//! wired into [`super::MatchGenerator`] / `MatcherStorage`. The
//! follow-up commit on the same branch replaces the `SuffixStore`-
//! based Fast strategy path with a `MatcherStorage::FastKernel`
//! arm that invokes [`compress_block_fast`] per block. The narrow
//! `#![allow(dead_code)]` below is intentionally scoped: it covers
//! the kernel-internal items that the wiring commit will reach in
//! one shot, without masking the broader `unused_imports` lint
//! (which would otherwise hide real unused-import regressions in
//! sibling code). Re-exports are deferred to the wiring commit
//! since adding them now would force a wider lint relaxation.
#![allow(dead_code)]

pub(crate) mod count;
pub(crate) mod hash_table;
pub(crate) mod kernel;
