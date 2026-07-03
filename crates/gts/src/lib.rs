// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS (Graph Transport Substrate) format engine — `docs/GTS-SPEC.md` Draft v0.3.
//!
//! A GTS file is a CBOR Sequence of one or more segments (#3.1), each an
//! append-only log: a Header followed by frames chained by BLAKE3 content-id
//! (`"id"`/`"prev"`, §6/§9.1). [`reader::read`] verifies the chain and folds
//! the log into a container fold (§7.5), degrading undecodable frames to
//! opaque nodes (§7.6) instead of aborting — the reader is total.
//!
//! This crate is the Rust counterpart of the Python reference oracle
//! (`src/purrdf_tools/gts/`); both are gated against the same frozen
//! language-neutral conformance corpus in `vectors/` (§18).
//! The Python side keeps the producer; this crate owns the format engine.

pub mod codec;
pub mod compact;
pub mod cose;
// emojihash + randomart now live in the standalone `visual-hashing` crate;
// re-exported here so `purrdf_gts::emojihash::…` paths keep resolving.
pub use visual_hashing as emojihash;
pub mod examples;
pub mod files;
pub mod from_tar;
pub mod mmr;
pub mod model;
pub mod nested;
pub mod openpgp;
pub mod policy;
pub use policy::{
    evaluate_profile_policy, signature_trust, ProfileFinding, Severity, SignatureTrust, TrustPolicy,
};
pub mod reader;
mod reader_layout;
mod reader_rows;
mod reader_union;
pub mod replication;
pub mod stream;
pub mod tar;
pub mod ulid;
pub mod verify;
pub mod wire;
pub mod writer;
