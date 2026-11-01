#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! In a Zstandard frame, there's a frame header, followed by one or more *blocks*.
//!
//! A block contains data, and a header describing how that data is encoded, as well
//! as other misc metadata.
//!
//! <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#blocks>

pub(crate) mod block;
pub(crate) mod literals_section;
pub(crate) mod sequence_section;
