// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GENERIC authenticated artifact envelope: one fixed-layout, fully
//! self-verifying byte container that any prepared-product codec in this
//! workspace instantiates by supplying a magic, a format version and a section
//! count — and nothing else.
//!
//! # Why this exists
//!
//! A *prepared product* is a byte blob a caller stores, ships and later hands
//! back to us claiming "this is the compiled form of those inputs". Accepting
//! one is an ADMISSION decision, not a parse: the moment a blob is opened, every
//! downstream layer treats its contents as though they had been derived here.
//! [`crate::ir::pack::container`] already got this right for the dataset pack —
//! fixed magic, fixed version, fixed-order section directory with a per-section
//! SHA-256, alignment-padded bodies, and a `from_bytes` that fails closed on
//! every step. The problem is that each *new* prepared product needed that same
//! discipline again, and integrity discipline transcribed N times drifts N ways.
//! The copy that drifts is always the one nobody re-reads.
//!
//! So the envelope moved here, once, with the pack container's layout laws
//! intact and the pack's domain knowledge (dictionaries, triples, side tables)
//! removed. A codec instantiating it declares an [`ArtifactSpec`] and gets the
//! whole admission boundary — including the [`Identity`] binding that says WHICH
//! inputs the artifact was compiled from.
//!
//! # The rejected alternative
//!
//! The obvious alternative was a general serialization framework: derive the
//! envelope, let each codec name optional fields, version them independently.
//! It was rejected because the envelope's entire value is that there is exactly
//! ONE byte layout for a given `(spec, identity, sections)` tuple and the reader
//! accepts only that one. A framework that tolerates several encodings of the
//! same content cannot make that statement, and an envelope that cannot make it
//! is not an admission boundary — it is a parser with a checksum bolted on.
//!
//! The failure this prevents, concretely: a corrupted or deliberately edited
//! prepared product being opened and used as if it had been compiled from the
//! inputs its identity claims, with nothing in the pipeline saying otherwise.
//!
//! # The section directory is TOTAL
//!
//! **Every declared section kind is ALWAYS present in every artifact, possibly
//! zero-length. There are NO optional sections.** [`ArtifactSpec::section_count`]
//! is a cardinality the builder and the reader both enforce exactly:
//! [`ArtifactBuilder::build_bytes`] refuses a build that supplied a different
//! number of sections ([`ArtifactError::SectionCountMismatch`]), and
//! [`ArtifactView::from_bytes`] refuses a buffer whose header disagrees with the
//! spec it was opened against. A kind the caller never wrote is an error
//! ([`ArtifactError::MissingSection`]), and a kind written twice is an error
//! ([`ArtifactError::DuplicateSection`]). **A zero-length section body is
//! perfectly VALID** and is how "this artifact has nothing to say under that
//! kind" is spelled.
//!
//! This is load-bearing rather than tidy. A genuinely optional section turns the
//! format into a modal one with `2^n` admissible shapes and a combinatorial
//! refusal matrix behind it; the project's standing rule forbids optional
//! components for exactly that reason. An unexercised branch inside an admission
//! boundary is precisely where a forgery hides, because it is the branch whose
//! refusals nobody ever ran.
//!
//! # On-disk layout
//!
//! Every multi-byte integer is little-endian. There is no pointer patching:
//! every offset is an absolute byte offset from the start of the buffer. The
//! layout is fully DETERMINED by the spec, the identity length and the section
//! lengths — the reader recomputes each offset and refuses any buffer whose
//! stored offsets disagree, so there is no second admissible spelling of the
//! same content. Regions are `ALIGNMENT`-padded (8 bytes) with zero bytes, and
//! the padding is verified to be zero on open so it cannot carry a covert
//! payload.
//!
//! ## Header (fixed, `HEADER_LEN` = 64 bytes, offset `0`)
//!
//! | Field             | Type      | Bytes | Offset | Meaning                                                    |
//! |-------------------|-----------|-------|--------|-------------------------------------------------------------|
//! | `magic`           | `[u8; 8]` | 8     | 0      | [`ArtifactSpec::magic`], supplied by the instantiating codec |
//! | `version`         | `u32`     | 4     | 8      | [`ArtifactSpec::format_version`]                            |
//! | `section_count`   | `u32`     | 4     | 12     | [`ArtifactSpec::section_count`]                             |
//! | `identity_offset` | `u64`     | 8     | 16     | absolute offset of the identity region                       |
//! | `identity_len`    | `u64`     | 8     | 24     | the identity region's exact byte length                      |
//! | `identity_digest` | `[u8;32]` | 32    | 32     | SHA-256 of the identity region's bytes                       |
//!
//! ## Section directory (`section_count` × `ENTRY_LEN` = 52 bytes each, at offset `HEADER_LEN`)
//!
//! Entries are in **strictly ascending `kind` order** — the canonical order, so
//! the order a caller happened to call [`ArtifactBuilder::section`] in never
//! reaches the bytes:
//!
//! | Field    | Type      | Bytes | Meaning                                       |
//! |----------|-----------|-------|------------------------------------------------|
//! | `kind`   | `u32`     | 4     | the caller-defined section kind tag             |
//! | `offset` | `u64`     | 8     | absolute offset of this section's raw bytes     |
//! | `len`    | `u64`     | 8     | this section's raw byte length (may be `0`)     |
//! | `sha256` | `[u8;32]` | 32    | SHA-256 of this section's raw bytes             |
//!
//! ## Identity region (at `align_up(HEADER_LEN + section_count * ENTRY_LEN)`)
//!
//! [`Identity::to_bytes`] — the ordered, length-prefixed, labelled components
//! that say which inputs this artifact was compiled from. Zero-length when the
//! identity is empty. See [`identity`] for the framing and why it is injective.
//!
//! ## Section bodies
//!
//! Starting at `align_up(identity_offset + identity_len)`, each section's raw
//! bytes in ascending `kind` order, each zero-padded up to the next 8-byte
//! boundary — including after the LAST one, so the trailer is aligned too.
//!
//! ## Trailer (fixed, `TRAILER_LEN` = 64 bytes, at the end of the buffer)
//!
//! | Field              | Type      | Bytes | Offset | Meaning                                      |
//! |--------------------|-----------|-------|--------|-----------------------------------------------|
//! | `magic`            | `[u8; 8]` | 8     | 0      | `TRAILER_MAGIC` = `b"PURRAEND"`               |
//! | `version`          | `u32`     | 4     | 8      | [`ArtifactSpec::format_version`]              |
//! | `reserved`         | `u32`     | 4     | 12     | always `0`                                    |
//! | `file_length`      | `u64`     | 8     | 16     | the buffer's exact total length               |
//! | `container_digest` | `[u8;32]` | 32    | 24     | SHA-256 of every byte BEFORE the trailer      |
//! | `reserved`         | `[u8; 8]` | 8     | 56     | always `0`                                    |
//!
//! The fixed trailer follows the PURREMB embedding wire format's precedent: a header
//! alone cannot certify that the bytes after it are all the bytes that were
//! written, so the trailer restates the total length and seals everything ahead
//! of it under one digest. A buffer truncated anywhere fails the length check;
//! a buffer with bytes appended fails it too.
//!
//! # Determinism
//!
//! [`ArtifactBuilder::build_bytes`] is a pure function of its inputs: the
//! section kinds and bodies, the identity components, and the spec. Nothing else
//! reaches the output — no hash-iteration order (sections are sorted by kind, the
//! identity is an ordered list), no wall-clock, no RNG. Two builds from equal
//! inputs are byte-identical, and insertion order is not an input.
//!
//! # Verification on open
//!
//! [`ArtifactView::from_bytes`] fails closed at every step, in order: buffer
//! length, magic, version, section count, canonical region offsets, zero
//! padding, EACH section's SHA-256, the identity region's digest and canonical
//! framing, the trailer's structure, and finally the whole-container digest.
//! This follows the pack register — a hard refusal on the first inconsistency —
//! and deliberately NOT the GTS reader's "record a diagnostic and keep folding".
//! That totality is the right law for a transport log, where the goal is to
//! report everything wrong with a document someone else wrote. It is the wrong
//! law for an admission boundary, where continuing past an inconsistency means
//! handing partially-verified bytes to a layer that will treat them as certified.
//!
//! # Portability
//!
//! `&[u8]` in, [`Vec<u8>`] out. No `std::fs`, no threads, no wall-clock, no RNG,
//! no new dependencies — `wasm32-unknown-unknown`-clean by construction. Every
//! multi-byte field is decoded via `from_le_bytes` on an explicit byte-slice
//! copy rather than a pointer cast, so the caller's buffer need not be aligned
//! to any field's native alignment (see [`crate::ir::pack::bits`]'s module doc
//! for the house statement of this requirement).

pub mod container;
pub mod identity;

pub use container::{ArtifactBuilder, ArtifactError, ArtifactSpec, ArtifactView};
pub use identity::{Identity, IdentityComponent, IdentityMismatch};
