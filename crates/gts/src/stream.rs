// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The spec-owned `stream` vocabulary (GTS-SPEC §13.3) — mirror of
//! `packages/gts/src/gts/stream.py`.
//!
//! Constants only — the streaming-index terms a streamable segment leads with
//! (§3.3) and the compaction-provenance terms a streamable rewrite records
//! (§10.1). Like the `files` profile vocabulary (§13.2), the terms are
//! authored in the spec and carried as literal IRIs; no external ontology is
//! required.

pub const STREAM_NS: &str = "https://w3id.org/gts/stream#";

// Streaming-index terms (§3.3): one Manifestation per promised blob.
pub const MANIFESTATION: &str = "https://w3id.org/gts/stream#Manifestation";
pub const DIGEST: &str = "https://w3id.org/gts/stream#digest";
pub const MEDIA_TYPE: &str = "https://w3id.org/gts/stream#mediaType";
pub const SIZE: &str = "https://w3id.org/gts/stream#size";
pub const ROLE: &str = "https://w3id.org/gts/stream#role";
pub const ORDER: &str = "https://w3id.org/gts/stream#order";

// Compaction-provenance terms (§10.1).
pub const COMPACTION: &str = "https://w3id.org/gts/stream#Compaction";
pub const AGENT: &str = "https://w3id.org/gts/stream#agent";
pub const TIMESTAMP: &str = "https://w3id.org/gts/stream#timestamp";
pub const SOURCE_HEAD: &str = "https://w3id.org/gts/stream#sourceHead";
pub const SEALED_SOURCE: &str = "https://w3id.org/gts/stream#sealedSource";
pub const DETACHED_SIGNATURE: &str = "https://w3id.org/gts/stream#DetachedSignature";
pub const SOURCE_FRAME: &str = "https://w3id.org/gts/stream#sourceFrame";
pub const COSE: &str = "https://w3id.org/gts/stream#cose";

/// The fixed compactor identity recorded as `stream:agent` — a constant so
/// the rewrite is byte-reproducible across engines (§14.1 determinism).
pub const COMPACT_AGENT: &str = "gts-compact";
