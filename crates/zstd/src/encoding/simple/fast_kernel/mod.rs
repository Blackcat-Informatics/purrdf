#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Upstream zstd-shape Fast strategy block compressor — flat hash-table +
//! tight per-block loop, ported from `lib/compress/zstd_fast.c`. See
//! [`kernel::compress_block_fast`] for the entry point.
//!
//! The Fast-strategy matcher encodes blocks with these table, prefix,
//! and match-search kernels.

pub(crate) mod count;
pub(crate) mod hash_table;
pub(crate) mod kernel;
