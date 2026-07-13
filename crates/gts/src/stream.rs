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

/// Namespace prefix of the spec-owned `stream` vocabulary (§13.3).
pub const STREAM_NS: &str = "https://w3id.org/gts/stream#";

// Streaming-index terms (§3.3): one Manifestation per promised blob.
/// Class of a streaming-index entry: one `stream:Manifestation` per promised blob (§3.3).
pub const MANIFESTATION: &str = "https://w3id.org/gts/stream#Manifestation";
/// `stream:digest` — BLAKE3 content digest of the promised blob.
pub const DIGEST: &str = "https://w3id.org/gts/stream#digest";
/// `stream:mediaType` — media type of the promised blob.
pub const MEDIA_TYPE: &str = "https://w3id.org/gts/stream#mediaType";
/// `stream:size` — decoded byte size of the promised blob.
pub const SIZE: &str = "https://w3id.org/gts/stream#size";
/// `stream:role` — the blob's role within the streamable layout.
pub const ROLE: &str = "https://w3id.org/gts/stream#role";
/// `stream:order` — delivery position of the blob in the streamable layout.
pub const ORDER: &str = "https://w3id.org/gts/stream#order";

// Compaction-provenance terms (§10.1).
/// Class of a compaction-provenance record (`stream:Compaction`, §10.1).
pub const COMPACTION: &str = "https://w3id.org/gts/stream#Compaction";
/// `stream:agent` — the tool identity that performed the compaction.
pub const AGENT: &str = "https://w3id.org/gts/stream#agent";
/// `stream:timestamp` — when the compaction rewrite was performed.
pub const TIMESTAMP: &str = "https://w3id.org/gts/stream#timestamp";
/// `stream:sourceHead` — head id of the source file the rewrite consumed.
pub const SOURCE_HEAD: &str = "https://w3id.org/gts/stream#sourceHead";
/// `stream:sealedSource` — whether the compacted source was sealed.
pub const SEALED_SOURCE: &str = "https://w3id.org/gts/stream#sealedSource";
/// Class of a carried detached frame signature (`stream:DetachedSignature`, §10.1).
pub const DETACHED_SIGNATURE: &str = "https://w3id.org/gts/stream#DetachedSignature";
/// `stream:sourceFrame` — original frame id a detached signature verifies against.
pub const SOURCE_FRAME: &str = "https://w3id.org/gts/stream#sourceFrame";
/// `stream:cose` — raw COSE_Sign1 bytes of a detached frame signature.
pub const COSE: &str = "https://w3id.org/gts/stream#cose";
/// `stream:contentRefoldDigest` — the RDFC-1.0 digest of the compacted content
/// projection, embedded so a repack certifies without the pre-compaction bytes
/// (proof-carrying pack, §10.1 refold equivalence).
pub const CONTENT_REFOLD_DIGEST: &str = "https://w3id.org/gts/stream#contentRefoldDigest";
/// `stream:detachedSignatureRoot` — MMR root binding the set of carried detached
/// frame signatures under one commitment (§10.1 signature preservation).
pub const DETACHED_SIGNATURE_ROOT: &str = "https://w3id.org/gts/stream#detachedSignatureRoot";

/// The fixed compactor identity recorded as `stream:agent` — a constant so
/// the rewrite is byte-reproducible across engines (§14.1 determinism).
pub const COMPACT_AGENT: &str = "gts-compact";
