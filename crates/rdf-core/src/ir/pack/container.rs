// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The on-disk pack container (Task 5 of the succinct-pack-codec feature):
//! frames [`super::dict`]/[`super::triples`]/[`super::side`]'s independently
//! serialized byte blocks into ONE fixed-layout, mmap-friendly file, written
//! deterministically by [`PackBuilder`] and opened zero-copy by [`PackView`].
//!
//! # On-disk layout
//!
//! Every multi-byte integer is little-endian; there is no pointer-patching —
//! every offset is an absolute byte offset from the start of the file, and
//! every raw section body starts on an 8-byte boundary (so a caller may `mmap`
//! the file and hand a section slice straight to an aligned reader without a
//! copy, even though this codec itself never assumes alignment — see
//! [`super::bits`]'s doc comment on alignment-agnostic parsing).
//!
//! ## Header (fixed, `HEADER_LEN` = 64 bytes, offset `0`)
//!
//! | Field           | Type      | Bytes | Offset | Meaning                                            |
//! |-----------------|-----------|-------|--------|-----------------------------------------------------|
//! | `magic`         | `[u8; 8]` | 8     | 0      | `MAGIC` = `b"PURRPCK1"`                            |
//! | `version`       | `u32`     | 4     | 8      | `FORMAT_VERSION` = `1`                             |
//! | `flags`         | `u32`     | 4     | 12     | capability bitmask — see "Capability flags" below    |
//! | `n_terms`       | `u64`     | 8     | 16     | the dictionary's total unified-id count              |
//! | `section_count` | `u32`     | 4     | 24     | always `SECTION_COUNT` = `3` in this format version |
//! | `reserved`      | `u32`     | 4     | 28     | always `0`; reserved for a future format revision    |
//! | `rdfc_digest`   | `[u8;32]` | 32    | 32     | SHA-256 of the dataset's RDFC-1.0 canonical N-Quads  |
//!
//! ## Capability flags (the header's `flags` field)
//!
//! Bit `i` set iff the correspondingly named [`RdfStoreCapabilities`] field is
//! `true`, in field-declaration order: bit 0 `named_graphs`, bit 1
//! `quoted_triples`, bit 2 `reifiers`, bit 3 `annotations`, bit 4
//! `source_locations`, bit 5 `loss_records`, bit 6 `lookaside`. Every other bit
//! is always `0` in this format version. [`PackView::from_bytes`] recomputes
//! capabilities independently (via [`super::side::capabilities`], never by
//! trusting this field) and fails closed with [`PackError::Malformed`] if the
//! stored flags disagree — the field exists for a caller that wants a
//! capability probe without decoding the dictionary/side-tables, not as a
//! trusted source.
//!
//! ## Section directory (fixed, `SECTION_COUNT` × `ENTRY_LEN` = 156 bytes,
//! immediately after the header at offset `HEADER_LEN`)
//!
//! Three fixed-order entries — DICT, then TRIPLES, then SIDE — each:
//!
//! | Field    | Type      | Bytes | Meaning                                          |
//! |----------|-----------|-------|---------------------------------------------------|
//! | `kind`   | `u32`     | 4     | `SECTION_DICT`/`SECTION_TRIPLES`/`SECTION_SIDE` |
//! | `offset` | `u64`     | 8     | absolute file offset of this section's raw bytes  |
//! | `len`    | `u64`     | 8     | this section's raw byte length                    |
//! | `sha256` | `[u8;32]` | 32    | SHA-256 of this section's raw bytes                |
//!
//! ## Section bytes
//!
//! Starting at `align_up(HEADER_LEN + SECTION_COUNT * ENTRY_LEN, 8)` (offset
//! 224 in this format version), each section's raw bytes — respectively
//! [`super::dict::EncodedDict::to_bytes`], [`super::triples::Triples::to_bytes`],
//! [`super::side::SideTables::to_bytes`] — follow in fixed order, each
//! zero-padded up to the next 8-byte boundary before the next section starts
//! (no padding after the LAST section — the file ends exactly at its end).
//!
//! # Determinism
//!
//! [`PackBuilder::build_bytes`] is a pure function of `dataset`'s VALUE content
//! (no hash-iteration order, no wall-clock, no RNG reaches the output — see the
//! byte-determinism discipline each of [`super::dict`]/[`super::triples`]/
//! [`super::side`] already upholds and this module inherits by construction):
//! two calls on the same dataset produce byte-identical output.
//!
//! # Verification on open
//!
//! [`PackView::from_bytes`] fails closed at every step, in order: magic,
//! version, section count, EACH section's SHA-256 (before the section's bytes
//! are handed to its own reader), each submodule's own internal structural
//! validation ([`super::dict::PackDict::open`]/
//! [`super::triples::TriplesRef::from_bytes`]/
//! [`super::side::SideTablesRef::from_bytes`]), then the header's `flags`/
//! `n_terms` fields against the values recomputed from the decoded sections.
//! A successfully-opened [`PackView`] therefore never panics on a later query.

use sha2::{Digest, Sha256};

use crate::{CanonHash, RdfDataset, RdfStoreCapabilities, try_canonicalize_with};

use super::dict::{PackDict, PackDictError};
use super::side::{self, PackSideError, SideTables, SideTablesRef};
use super::triples::{PackTriplesError, Triples, TriplesRef};

// ---------------------------------------------------------------------------
// Fixed layout constants.
// ---------------------------------------------------------------------------

/// The 8-byte magic every pack file starts with. Chosen to be a stable,
/// human-legible ASCII tag (`"PURRPCK1"`) rather than an arbitrary byte
/// pattern, so a hex dump or `file`-style sniff immediately identifies the
/// format; the trailing `1` is NOT the format version (that is the separate
/// `version` header field) — it is fixed for the life of this magic string,
/// bumped only if the magic itself is ever retired.
const MAGIC: [u8; 8] = *b"PURRPCK1";

/// The on-disk format version [`PackBuilder::build_bytes`] writes and
/// [`PackView::from_bytes`] requires.
const FORMAT_VERSION: u32 = 1;

/// The number of fixed-order sections this format version frames.
const SECTION_COUNT: usize = 3;

/// The directory tag for the [`super::dict`] section.
const SECTION_DICT: u32 = 1;
/// The directory tag for the [`super::triples`] section.
const SECTION_TRIPLES: u32 = 2;
/// The directory tag for the [`super::side`] section.
const SECTION_SIDE: u32 = 3;

/// The fixed-order tags every pack file's directory carries, in the exact
/// on-disk order (DICT, TRIPLES, SIDE).
const SECTION_KINDS: [u32; SECTION_COUNT] = [SECTION_DICT, SECTION_TRIPLES, SECTION_SIDE];

/// The header's total byte length (see the [module docs](self) table).
const HEADER_LEN: usize = 64;

/// One section directory entry's fixed byte length (`kind` + `offset` + `len`
/// + `sha256`).
const ENTRY_LEN: usize = 4 + 8 + 8 + 32;

/// Capability bit positions within the header's `flags` field, in
/// [`RdfStoreCapabilities`]'s field-declaration order.
const FLAG_NAMED_GRAPHS: u32 = 1 << 0;
const FLAG_QUOTED_TRIPLES: u32 = 1 << 1;
const FLAG_REIFIERS: u32 = 1 << 2;
const FLAG_ANNOTATIONS: u32 = 1 << 3;
const FLAG_SOURCE_LOCATIONS: u32 = 1 << 4;
const FLAG_LOSS_RECORDS: u32 = 1 << 5;
const FLAG_LOOKASIDE: u32 = 1 << 6;

/// Round `value` up to the next multiple of `align` (`align` a power of two).
const fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

/// Encode `caps` as the header's `flags` bitmask — see "Capability flags" in
/// the [module docs](self).
fn capabilities_to_flags(caps: RdfStoreCapabilities) -> u32 {
    let mut flags = 0u32;
    if caps.named_graphs {
        flags |= FLAG_NAMED_GRAPHS;
    }
    if caps.quoted_triples {
        flags |= FLAG_QUOTED_TRIPLES;
    }
    if caps.reifiers {
        flags |= FLAG_REIFIERS;
    }
    if caps.annotations {
        flags |= FLAG_ANNOTATIONS;
    }
    if caps.source_locations {
        flags |= FLAG_SOURCE_LOCATIONS;
    }
    if caps.loss_records {
        flags |= FLAG_LOSS_RECORDS;
    }
    if caps.lookaside {
        flags |= FLAG_LOOKASIDE;
    }
    flags
}

/// Decode a header `flags` bitmask back to [`RdfStoreCapabilities`] — the
/// inverse of [`capabilities_to_flags`], used only to CROSS-CHECK the stored
/// field against the independently recomputed capabilities at open time (see
/// "Capability flags" in the [module docs](self)); never trusted on its own.
fn flags_to_capabilities(flags: u32) -> RdfStoreCapabilities {
    RdfStoreCapabilities {
        named_graphs: flags & FLAG_NAMED_GRAPHS != 0,
        quoted_triples: flags & FLAG_QUOTED_TRIPLES != 0,
        reifiers: flags & FLAG_REIFIERS != 0,
        annotations: flags & FLAG_ANNOTATIONS != 0,
        source_locations: flags & FLAG_SOURCE_LOCATIONS != 0,
        loss_records: flags & FLAG_LOSS_RECORDS != 0,
        lookaside: flags & FLAG_LOOKASIDE != 0,
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why building or opening a pack container failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackError {
    /// The buffer's leading 8 bytes are not `MAGIC`.
    BadMagic,
    /// The header's `version` field is not `FORMAT_VERSION`.
    UnsupportedVersion(u32),
    /// The buffer ended before all the bytes the header/directory/a section
    /// span promised were present.
    Truncated,
    /// The buffer's header or directory was internally inconsistent (a bad
    /// section count, an out-of-fixed-order section tag, an offset/length
    /// that overflows, or the header's `flags`/`n_terms` fields disagreeing
    /// with the values recomputed from the decoded sections).
    Malformed(&'static str),
    /// A section's recomputed SHA-256 disagreed with its stored directory
    /// digest — the section's bytes were altered after
    /// [`PackBuilder::build_bytes`] wrote them. `kind` is the section's
    /// directory tag (`SECTION_DICT`/`SECTION_TRIPLES`/`SECTION_SIDE`).
    SectionDigestMismatch {
        /// The mismatched section's directory tag.
        kind: u32,
    },
    /// [`crate::try_canonicalize_with`] exceeded its call budget computing
    /// the dataset's RDFC-1.0 digest (a pathologically symmetric blank
    /// graph — see [`crate::BudgetExceeded`]).
    CanonBudgetExceeded,
    /// [`super::certify::verify_pack`]'s independent RDFC-1.0 recompute over the
    /// pack's own decoded contents disagreed with the header's stored
    /// `rdfc_digest` field. Unlike [`Self::SectionDigestMismatch`], this is NOT a
    /// byte-corruption signal `from_bytes` can see on its own: the `rdfc_digest`
    /// header field sits outside the section directory's SHA-256 coverage (see
    /// [module docs](self)), so a pack whose digest field was tampered with still
    /// opens cleanly — this variant is the certified-projection defense that
    /// catches exactly that case.
    RdfcDigestMismatch {
        /// The digest recorded in the pack's header.
        expected: [u8; 32],
        /// The digest independently recomputed from the pack's own decoded
        /// contents.
        computed: [u8; 32],
    },
    /// The DICT section failed to decode.
    Dict(PackDictError),
    /// The TRIPLES section failed to decode.
    Triples(PackTriplesError),
    /// The SIDE section failed to decode.
    Side(PackSideError),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "pack-container: bad magic"),
            Self::UnsupportedVersion(v) => {
                write!(f, "pack-container: unsupported format version {v}")
            }
            Self::Truncated => write!(f, "pack-container: truncated input"),
            Self::Malformed(reason) => write!(f, "pack-container: malformed input: {reason}"),
            Self::SectionDigestMismatch { kind } => write!(
                f,
                "pack-container: section {kind} failed its SHA-256 integrity check"
            ),
            Self::CanonBudgetExceeded => write!(
                f,
                "pack-container: RDFC-1.0 canonicalization exceeded its call budget"
            ),
            Self::RdfcDigestMismatch { expected, computed } => {
                write!(
                    f,
                    "pack-container: RDFC-1.0 digest mismatch: header claims {}, recomputed {}",
                    hex32(expected),
                    hex32(computed)
                )
            }
            Self::Dict(e) => write!(f, "pack-container: dict section: {e}"),
            Self::Triples(e) => write!(f, "pack-container: triples section: {e}"),
            Self::Side(e) => write!(f, "pack-container: side section: {e}"),
        }
    }
}

impl std::error::Error for PackError {}

impl From<PackDictError> for PackError {
    fn from(e: PackDictError) -> Self {
        Self::Dict(e)
    }
}

impl From<PackTriplesError> for PackError {
    fn from(e: PackTriplesError) -> Self {
        Self::Triples(e)
    }
}

impl From<PackSideError> for PackError {
    fn from(e: PackSideError) -> Self {
        Self::Side(e)
    }
}

// ---------------------------------------------------------------------------
// Small byte-header write/read helpers (explicit LE, no pointer casts).
// ---------------------------------------------------------------------------

fn write_u32_le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_u64_le(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Read a `u32` at `*pos`, advancing `*pos` past it. The caller must already
/// have checked `bytes.len() >= *pos + 4` (every call site here reads from
/// within the fixed-length header/directory, whose total length is checked
/// up front by [`PackView::from_bytes`]).
fn read_u32_le(bytes: &[u8], pos: &mut usize) -> u32 {
    let v = u32::from_le_bytes(
        bytes[*pos..*pos + 4]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    );
    *pos += 4;
    v
}

/// Read a `u64` at `*pos`, advancing `*pos` past it. See [`read_u32_le`]'s
/// bounds-checking note.
fn read_u64_le(bytes: &[u8], pos: &mut usize) -> u64 {
    let v = u64::from_le_bytes(
        bytes[*pos..*pos + 8]
            .try_into()
            .expect("slice is exactly 8 bytes"),
    );
    *pos += 8;
    v
}

/// Read a 32-byte digest at `*pos`, advancing `*pos` past it. See
/// [`read_u32_le`]'s bounds-checking note.
fn read_digest(bytes: &[u8], pos: &mut usize) -> [u8; 32] {
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[*pos..*pos + 32]);
    *pos += 32;
    digest
}

/// Lowercase-hex a 32-byte digest, for [`PackError::RdfcDigestMismatch`]'s
/// `Display`. A tiny local helper rather than a crate-wide hex utility: the
/// only other digest-hex renderer ([`super::certify::PackDigest::to_hex`])
/// lives one layer up and this module has no reason to depend on it.
fn hex32(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(64);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

// ---------------------------------------------------------------------------
// PackBuilder — the offline factory writer.
// ---------------------------------------------------------------------------

/// The offline factory writer: assembles a self-contained, byte-deterministic
/// pack file from an [`RdfDataset`]. See the [module docs](self) for the exact
/// on-disk layout.
#[derive(Debug, Clone, Copy)]
pub struct PackBuilder;

impl PackBuilder {
    /// Build the complete pack file for `dataset`: encode the dictionary,
    /// bitmap-triples, and RDF 1.2 side-tables (in that fixed order), compute
    /// the dataset's RDFC-1.0 digest, and frame everything into the on-disk
    /// container format (see the [module docs](self)).
    ///
    /// Deterministic: byte-identical output for the same dataset across calls
    /// (no hash-iteration order, wall-clock, or RNG reaches the output).
    ///
    /// # Errors
    ///
    /// [`PackError::CanonBudgetExceeded`] if RDFC-1.0 canonicalization's call
    /// budget is exhausted (a pathologically symmetric blank graph). The
    /// dict/triples/side encode steps are infallible; the ONLY way this
    /// method otherwise fails is if one of THIS module's own just-written
    /// sections fails to re-open — a broken-invariant bug in this module or
    /// an upstream submodule, not a data-dependent error.
    pub fn build_bytes(dataset: &RdfDataset) -> Result<Vec<u8>, PackError> {
        let encoded_dict = PackDict::encode(dataset);
        let dict_bytes = encoded_dict.to_bytes();
        let n_terms = encoded_dict.n_terms();
        let dict = PackDict::open(&dict_bytes)?;

        let triples_bytes = Triples::encode(&dict, dataset).to_bytes();
        let triples_ref = TriplesRef::from_bytes(&triples_bytes)?;

        let side_bytes = SideTables::encode(&dict, dataset).to_bytes();
        let side_ref = SideTablesRef::from_bytes(&side_bytes)?;

        let base_named_graphs = triples_ref.named_graph_ids().next().is_some();
        let capabilities = side::capabilities(&dict, &side_ref, base_named_graphs);

        let canonicalized = try_canonicalize_with(dataset, CanonHash::Sha256)
            .map_err(|_| PackError::CanonBudgetExceeded)?;
        let rdfc_digest: [u8; 32] = Sha256::digest(canonicalized.nquads.as_bytes()).into();

        Ok(assemble(
            n_terms,
            capabilities,
            rdfc_digest,
            &dict_bytes,
            &triples_bytes,
            &side_bytes,
        ))
    }

    /// A public convenience alias for [`build_bytes`](Self::build_bytes) —
    /// the entry point Task 6's `DatasetView`-over-`PackView` seam re-exports.
    ///
    /// # Errors
    ///
    /// Identical to [`build_bytes`](Self::build_bytes).
    pub fn from_dataset(dataset: &RdfDataset) -> Result<Vec<u8>, PackError> {
        Self::build_bytes(dataset)
    }
}

/// Assemble the header + directory + 8-byte-aligned section bytes, in the
/// fixed `[DICT, TRIPLES, SIDE]` order — the single place [`build_bytes`]'s
/// layout decisions live, so the [module docs](self) table and this function
/// are the only two places that need to agree.
fn assemble(
    n_terms: u64,
    capabilities: RdfStoreCapabilities,
    rdfc_digest: [u8; 32],
    dict_bytes: &[u8],
    triples_bytes: &[u8],
    side_bytes: &[u8],
) -> Vec<u8> {
    let sections: [(u32, &[u8]); SECTION_COUNT] = [
        (SECTION_DICT, dict_bytes),
        (SECTION_TRIPLES, triples_bytes),
        (SECTION_SIDE, side_bytes),
    ];
    let digests: [[u8; 32]; SECTION_COUNT] =
        std::array::from_fn(|i| Sha256::digest(sections[i].1).into());

    // First pass: compute every section's absolute, 8-byte-aligned offset.
    let mut offsets = [0u64; SECTION_COUNT];
    let mut cursor = align_up((HEADER_LEN + SECTION_COUNT * ENTRY_LEN) as u64, 8);
    for (i, (_, bytes)) in sections.iter().enumerate() {
        offsets[i] = cursor;
        cursor += bytes.len() as u64;
        if i + 1 < SECTION_COUNT {
            cursor = align_up(cursor, 8);
        }
    }

    let mut out = Vec::with_capacity(cursor as usize);

    // -- Header ---------------------------------------------------------
    out.extend_from_slice(&MAGIC);
    write_u32_le(&mut out, FORMAT_VERSION);
    write_u32_le(&mut out, capabilities_to_flags(capabilities));
    write_u64_le(&mut out, n_terms);
    write_u32_le(&mut out, SECTION_COUNT as u32);
    write_u32_le(&mut out, 0); // reserved
    out.extend_from_slice(&rdfc_digest);
    debug_assert_eq!(out.len(), HEADER_LEN);

    // -- Section directory ------------------------------------------------
    for (i, (kind, bytes)) in sections.iter().enumerate() {
        write_u32_le(&mut out, *kind);
        write_u64_le(&mut out, offsets[i]);
        write_u64_le(&mut out, bytes.len() as u64);
        out.extend_from_slice(&digests[i]);
    }
    debug_assert_eq!(out.len(), HEADER_LEN + SECTION_COUNT * ENTRY_LEN);

    // -- Section bytes, 8-byte aligned, zero-padded between sections ------
    for (i, (_, bytes)) in sections.iter().enumerate() {
        while (out.len() as u64) < offsets[i] {
            out.push(0);
        }
        out.extend_from_slice(bytes);
    }
    debug_assert_eq!(out.len() as u64, cursor);

    out
}

// ---------------------------------------------------------------------------
// PackView — the zero-copy reader.
// ---------------------------------------------------------------------------

/// The zero-copy, borrowed reader over [`PackBuilder::build_bytes`]'s output.
/// Owns the decoded [`PackDict`] (an arena the dictionary's PFC sections are
/// decompressed into once, at open time) and borrows the bitmap-triples and
/// side-table sections directly from the input buffer — see the
/// [module docs](self) for the exact on-disk layout and the fail-closed
/// verification [`from_bytes`](Self::from_bytes) performs.
#[derive(Debug)]
pub struct PackView<'a> {
    dict: PackDict,
    triples: TriplesRef<'a>,
    side: SideTablesRef<'a>,
    capabilities: RdfStoreCapabilities,
    rdfc_digest: [u8; 32],
}

impl<'a> PackView<'a> {
    /// Parse and fully verify [`PackBuilder::build_bytes`]'s output.
    ///
    /// Fails closed, in order: magic, format version, section count, then
    /// EACH section's SHA-256 against its stored directory digest (before any
    /// section's bytes are handed to its own reader), then each submodule's
    /// own structural validation, then the header's `flags`/`n_terms` fields
    /// against the values recomputed from the decoded sections. A
    /// successfully-returned `PackView` therefore never panics on a later
    /// query.
    ///
    /// # Errors
    ///
    /// See [`PackError`]'s variants for every failure this can return.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackError> {
        if bytes.len() < HEADER_LEN {
            return Err(PackError::Truncated);
        }
        if bytes[0..8] != MAGIC {
            return Err(PackError::BadMagic);
        }

        let mut pos = 8usize;
        let version = read_u32_le(bytes, &mut pos);
        if version != FORMAT_VERSION {
            return Err(PackError::UnsupportedVersion(version));
        }
        let flags = read_u32_le(bytes, &mut pos);
        let n_terms = read_u64_le(bytes, &mut pos);
        let section_count = read_u32_le(bytes, &mut pos);
        let _reserved = read_u32_le(bytes, &mut pos);
        let rdfc_digest = read_digest(bytes, &mut pos);
        debug_assert_eq!(pos, HEADER_LEN);

        if section_count as usize != SECTION_COUNT {
            return Err(PackError::Malformed(
                "container: unexpected section count in header",
            ));
        }

        let dir_len = SECTION_COUNT * ENTRY_LEN;
        if bytes.len() < HEADER_LEN + dir_len {
            return Err(PackError::Truncated);
        }

        let mut section_bytes: [&'a [u8]; SECTION_COUNT] = [&[], &[], &[]];
        for (i, &expected_kind) in SECTION_KINDS.iter().enumerate() {
            let kind = read_u32_le(bytes, &mut pos);
            let offset = read_u64_le(bytes, &mut pos);
            let len = read_u64_le(bytes, &mut pos);
            let stored_digest = read_digest(bytes, &mut pos);

            if kind != expected_kind {
                return Err(PackError::Malformed(
                    "container: section directory tag out of fixed order",
                ));
            }
            let offset = usize::try_from(offset)
                .map_err(|_| PackError::Malformed("container: section offset exceeds usize"))?;
            let len = usize::try_from(len)
                .map_err(|_| PackError::Malformed("container: section length exceeds usize"))?;
            let end = offset
                .checked_add(len)
                .ok_or(PackError::Malformed("container: section span overflows"))?;
            let slice = bytes.get(offset..end).ok_or(PackError::Truncated)?;

            let actual_digest: [u8; 32] = Sha256::digest(slice).into();
            if actual_digest != stored_digest {
                return Err(PackError::SectionDigestMismatch { kind });
            }
            section_bytes[i] = slice;
        }
        debug_assert_eq!(pos, HEADER_LEN + dir_len);

        let dict = PackDict::open(section_bytes[0])?;
        let triples = TriplesRef::from_bytes(section_bytes[1])?;
        let side = SideTablesRef::from_bytes(section_bytes[2])?;

        let base_named_graphs = triples.named_graph_ids().next().is_some();
        let capabilities = side::capabilities(&dict, &side, base_named_graphs);

        if flags_to_capabilities(flags) != capabilities {
            return Err(PackError::Malformed(
                "container: header capability flags disagree with the recomputed capabilities",
            ));
        }
        if n_terms != dict.n_terms() {
            return Err(PackError::Malformed(
                "container: header n_terms disagrees with the decoded dictionary",
            ));
        }

        Ok(Self {
            dict,
            triples,
            side,
            capabilities,
            rdfc_digest,
        })
    }

    /// The decoded, owned value dictionary.
    #[must_use]
    pub fn dict(&self) -> &PackDict {
        &self.dict
    }

    /// The borrowed, zero-copy bitmap-triples reader.
    #[must_use]
    pub fn triples(&self) -> &TriplesRef<'a> {
        &self.triples
    }

    /// The borrowed, zero-copy RDF 1.2 side-tables reader.
    #[must_use]
    pub fn side(&self) -> &SideTablesRef<'a> {
        &self.side
    }

    /// The pack's [`RdfStoreCapabilities`], computed from the dictionary and
    /// side-tables at open time (see [`super::side::capabilities`]) and
    /// cross-checked against the header's `flags` field.
    #[must_use]
    pub fn capabilities(&self) -> RdfStoreCapabilities {
        self.capabilities
    }

    /// The SHA-256 digest of the dataset's RDFC-1.0 canonical N-Quads form,
    /// as recorded in the header at build time — a caller can independently
    /// re-canonicalize and compare to certify the pack matches a claimed
    /// dataset identity.
    #[must_use]
    pub fn rdfc_digest(&self) -> [u8; 32] {
        self.rdfc_digest
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_up_rounds_to_the_next_multiple() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
        assert_eq!(align_up(220, 8), 224);
    }

    #[test]
    fn header_and_directory_constants_agree() {
        // HEADER_LEN and ENTRY_LEN are documented as fixed numbers in the
        // module docs' tables; pin them here so a future edit that changes
        // the layout is forced to update the docs in the same diff.
        assert_eq!(HEADER_LEN, 64);
        assert_eq!(ENTRY_LEN, 52);
        assert_eq!(HEADER_LEN + SECTION_COUNT * ENTRY_LEN, 220);
        assert_eq!(
            align_up((HEADER_LEN + SECTION_COUNT * ENTRY_LEN) as u64, 8),
            224
        );
    }

    #[test]
    fn capability_flags_round_trip() {
        let caps = RdfStoreCapabilities {
            named_graphs: true,
            quoted_triples: false,
            reifiers: true,
            annotations: false,
            source_locations: true,
            loss_records: false,
            lookaside: true,
        };
        let flags = capabilities_to_flags(caps);
        assert_eq!(flags_to_capabilities(flags), caps);
    }

    #[test]
    fn capability_flags_all_off_and_all_on() {
        let none = RdfStoreCapabilities::plain_rdf();
        assert_eq!(capabilities_to_flags(none), 0);
        assert_eq!(flags_to_capabilities(0), none);

        let all = RdfStoreCapabilities {
            named_graphs: true,
            quoted_triples: true,
            reifiers: true,
            annotations: true,
            source_locations: true,
            loss_records: true,
            lookaside: true,
        };
        let flags = capabilities_to_flags(all);
        assert_eq!(flags, 0b0111_1111);
        assert_eq!(flags_to_capabilities(flags), all);
    }

    #[test]
    fn assemble_places_sections_on_8_byte_boundaries() {
        let caps = RdfStoreCapabilities::plain_rdf();
        let dict_bytes = vec![1u8; 3]; // deliberately NOT a multiple of 8
        let triples_bytes = vec![2u8; 5];
        let side_bytes = vec![3u8; 1];
        let bytes = assemble(0, caps, [0u8; 32], &dict_bytes, &triples_bytes, &side_bytes);

        let mut pos = HEADER_LEN;
        for _ in 0..SECTION_COUNT {
            let mut p = pos;
            let _kind = read_u32_le(&bytes, &mut p);
            let offset = read_u64_le(&bytes, &mut p);
            let _len = read_u64_le(&bytes, &mut p);
            assert_eq!(offset % 8, 0, "section offset must be 8-byte aligned");
            pos += ENTRY_LEN;
        }
    }
}
