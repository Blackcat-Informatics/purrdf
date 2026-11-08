#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
pub(crate) mod config;
mod fastest;
pub(crate) use fastest::{compress_block_encoded, compress_block_encoded_borrowed};
