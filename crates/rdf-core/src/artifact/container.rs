// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The envelope itself: [`ArtifactSpec`] declares a format, [`ArtifactBuilder`]
//! writes one deterministically, and [`ArtifactView`] opens one fail-closed.
//!
//! See the [module docs](super) for the byte layout table, the totality law that
//! makes every declared section always present, the determinism guarantee, and
//! the reason the refusal register here is a hard refusal rather than a
//! diagnostic fold.
//!
//! # What lives here and what does not
//!
//! This module knows about magics, offsets, alignment, digests and refusals. It
//! knows NOTHING about what a section means — a section is a `u32` tag and a
//! byte string, and the codec that instantiates the spec owns both. That
//! division is the point: the envelope's laws are proven once here, and a codec
//! adding a new section kind cannot weaken them, because it has no code in this
//! file to weaken.
//!
//! The rejected alternative was to let each codec pass a validation callback the
//! envelope would run over each section body before admitting it. It would have
//! saved the codec one pass, and it would have put caller code inside the
//! admission boundary, where the exact question being answered is "is this
//! buffer trustworthy yet?". The answer, until every digest in this file has
//! matched, is no — so the codec decodes AFTER [`ArtifactView::from_bytes`]
//! returns, never during it.

use std::fmt;

use sha2::{Digest, Sha256};

use super::identity::Identity;

// ---------------------------------------------------------------------------
// Fixed layout constants.
// ---------------------------------------------------------------------------

/// The 8-byte magic every artifact envelope ENDS with. Unlike the header's
/// magic — which the instantiating codec supplies, so that two products are
/// never mistaken for each other — the trailer magic is fixed for every
/// artifact in this workspace: a human-legible ASCII tag that says "a PurRDF
/// artifact envelope ended exactly here" regardless of which codec wrote it.
const TRAILER_MAGIC: [u8; 8] = *b"PURRAEND";

/// The header's total byte length (see the [layout table](super)).
const HEADER_LEN: usize = 64;

/// One section directory entry's fixed byte length (`kind` + `offset` + `len` +
/// `sha256`).
const ENTRY_LEN: usize = 4 + 8 + 8 + 32;

/// The trailer's total byte length (see the [layout table](super)).
const TRAILER_LEN: usize = 64;

/// Every region (the identity, each section body, the trailer) starts on a
/// multiple of this many bytes, with zero padding in between.
const ALIGNMENT: usize = 8;

/// Round `value` up to the next multiple of [`ALIGNMENT`].
fn align_up(value: usize) -> Result<usize, ArtifactError> {
    value
        .checked_add(ALIGNMENT - 1)
        .map(|biased| biased & !(ALIGNMENT - 1))
        .ok_or(ArtifactError::Malformed(
            "artifact: alignment overflows the address space",
        ))
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why building or opening an artifact envelope failed.
///
/// Every variant is a HARD refusal: [`ArtifactView::from_bytes`] returns on the
/// first inconsistency and never produces a partially-verified view. See the
/// [module docs](super) for why this register, and not a diagnostic fold, is the
/// right law at an admission boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArtifactError {
    /// The buffer's leading 8 bytes are not the spec's [`ArtifactSpec::magic`].
    BadMagic,
    /// The header's `version` field is not the spec's
    /// [`ArtifactSpec::format_version`]. Carries the version the BUFFER claimed,
    /// so a caller can say which format they were handed.
    UnsupportedVersion(u32),
    /// The buffer ended before all the bytes the header, the directory, the
    /// identity region, a section span or the trailer promised were present.
    Truncated,
    /// The buffer was internally inconsistent in a way the other variants do not
    /// name: a region at a non-canonical offset, non-zero padding, a directory
    /// out of ascending kind order, a span that overflows, bytes trailing the
    /// trailer, or an identity region that fails its own framing rules.
    Malformed(&'static str),
    /// A section's recomputed SHA-256 disagreed with its stored directory
    /// digest — that section's bytes were altered after
    /// [`ArtifactBuilder::build_bytes`] wrote them.
    SectionDigestMismatch {
        /// The mismatched section's directory tag.
        kind: u32,
    },
    /// The trailer's whole-container SHA-256 disagreed with the digest
    /// recomputed over every byte ahead of the trailer.
    ///
    /// Distinct from [`Self::SectionDigestMismatch`]: that one says a section
    /// body moved, this one says something OUTSIDE the section bodies did — the
    /// header, the directory, the identity region, or the padding between them.
    ContainerDigestMismatch {
        /// The digest recorded in the trailer.
        expected: [u8; 32],
        /// The digest independently recomputed from the buffer's own bytes.
        computed: [u8; 32],
    },
    /// The fixed trailer's magic, version, reserved fields or recorded file
    /// length disagreed with what the buffer actually is.
    TrailerMismatch,
    /// The same section kind was supplied twice to [`ArtifactBuilder`], or
    /// appears twice in an opened buffer's directory.
    DuplicateSection {
        /// The repeated section tag.
        kind: u32,
    },
    /// [`ArtifactView::section`] was asked for a kind this artifact's directory
    /// does not carry.
    ///
    /// The section directory is TOTAL — see the [module docs](super). A kind the
    /// codec declared is always written, so an absent kind means the buffer was
    /// produced by a different format, not that the section was "left out".
    MissingSection {
        /// The requested tag.
        kind: u32,
    },
    /// The number of sections supplied to [`ArtifactBuilder`], or recorded in an
    /// opened buffer's header, is not the spec's
    /// [`ArtifactSpec::section_count`].
    SectionCountMismatch {
        /// The count the spec requires.
        expected: usize,
        /// The count actually supplied or found.
        found: usize,
    },
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "artifact: bad magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "artifact: unsupported format version {version}")
            }
            Self::Truncated => write!(f, "artifact: truncated input"),
            Self::Malformed(reason) => write!(f, "artifact: malformed input: {reason}"),
            Self::SectionDigestMismatch { kind } => write!(
                f,
                "artifact: section {kind} failed its SHA-256 integrity check"
            ),
            Self::ContainerDigestMismatch { expected, computed } => write!(
                f,
                "artifact: container digest mismatch: trailer claims {}, recomputed {}",
                hex32(expected),
                hex32(computed)
            ),
            Self::TrailerMismatch => write!(f, "artifact: trailer does not describe this buffer"),
            Self::DuplicateSection { kind } => {
                write!(f, "artifact: section {kind} appears more than once")
            }
            Self::MissingSection { kind } => {
                write!(f, "artifact: section {kind} is not in this artifact")
            }
            Self::SectionCountMismatch { expected, found } => write!(
                f,
                "artifact: expected exactly {expected} sections, found {found}"
            ),
        }
    }
}

impl std::error::Error for ArtifactError {}

/// Lowercase-hex a 32-byte digest for [`ArtifactError`]'s `Display`. A local
/// helper rather than a shared utility: this module owes nothing to any layer
/// above it, and a digest renderer is four lines.
fn hex32(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// ---------------------------------------------------------------------------
// Small byte-header write/read helpers (explicit LE, no pointer casts).
// ---------------------------------------------------------------------------
//
// `ir::pack::bits` keeps its equivalents module-private and that file's byte
// layouts are frozen by goldens, so these are transcribed rather than shared.
// They follow the same alignment-agnostic law that module documents: every
// multi-byte field is decoded through `from_le_bytes` over an explicit
// byte-slice copy, never a pointer cast, so the caller's buffer may sit at any
// address.

fn write_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64_le(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Read a `u32` at `*pos`, advancing `*pos` past it. Every call site has already
/// proven the buffer holds `*pos + 4` bytes.
fn read_u32_le(bytes: &[u8], pos: &mut usize) -> u32 {
    let value = u32::from_le_bytes(
        bytes[*pos..*pos + 4]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    );
    *pos += 4;
    value
}

/// Read a `u64` at `*pos`, advancing `*pos` past it. See [`read_u32_le`]'s
/// bounds note.
fn read_u64_le(bytes: &[u8], pos: &mut usize) -> u64 {
    let value = u64::from_le_bytes(
        bytes[*pos..*pos + 8]
            .try_into()
            .expect("slice is exactly 8 bytes"),
    );
    *pos += 8;
    value
}

/// Read a 32-byte digest at `*pos`, advancing `*pos` past it. See
/// [`read_u32_le`]'s bounds note.
fn read_digest(bytes: &[u8], pos: &mut usize) -> [u8; 32] {
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[*pos..*pos + 32]);
    *pos += 32;
    digest
}

/// Convert a `usize` length into the `u64` the format stores.
fn to_u64(value: usize) -> Result<u64, ArtifactError> {
    u64::try_from(value)
        .map_err(|_| ArtifactError::Malformed("artifact: length exceeds the 64-bit field"))
}

/// Convert a stored `u64` offset or length into a `usize` this process can
/// index with.
fn to_usize(value: u64, what: &'static str) -> Result<usize, ArtifactError> {
    usize::try_from(value).map_err(|_| ArtifactError::Malformed(what))
}

// ---------------------------------------------------------------------------
// ArtifactSpec
// ---------------------------------------------------------------------------

/// The complete declaration of one artifact format: everything an instantiating
/// codec supplies, and nothing more.
///
/// The spec fixes the CARDINALITY of the section directory, not the kind tags
/// themselves — the codec owns those as its own constants. What this type makes
/// enforceable is the totality law: whatever kinds the codec chose, exactly
/// [`section_count`](Self::section_count) of them are written on every build and
/// required on every open (see the [module docs](super)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactSpec {
    /// The 8-byte header magic identifying this artifact format. Two codecs must
    /// never share one, or each will happily open the other's bytes.
    pub magic: [u8; 8],
    /// The format version this codec writes and requires. A buffer carrying any
    /// other value is refused with [`ArtifactError::UnsupportedVersion`] rather
    /// than best-effort parsed.
    pub format_version: u32,
    /// How many sections every artifact of this format carries — always all of
    /// them, some possibly zero-length.
    pub section_count: usize,
}

impl ArtifactSpec {
    /// Declare an artifact format.
    #[must_use]
    pub const fn new(magic: [u8; 8], format_version: u32, section_count: usize) -> Self {
        Self {
            magic,
            format_version,
            section_count,
        }
    }
}

// ---------------------------------------------------------------------------
// ArtifactBuilder
// ---------------------------------------------------------------------------

/// The deterministic writer: collect an identity and every declared section,
/// then emit the one byte layout that content admits.
///
/// Insertion order is NOT an input — sections are written in ascending `kind`
/// order, so two builds that supply the same sections in different orders emit
/// identical bytes. See "Determinism" in the [module docs](super).
#[derive(Debug, Clone)]
pub struct ArtifactBuilder {
    spec: ArtifactSpec,
    identity: Identity,
    sections: Vec<(u32, Vec<u8>)>,
}

impl ArtifactBuilder {
    /// Start an artifact for `spec`, with an empty identity and no sections yet.
    #[must_use]
    pub fn new(spec: ArtifactSpec) -> Self {
        Self {
            spec,
            identity: Identity::new(),
            sections: Vec::with_capacity(spec.section_count),
        }
    }

    /// Bind the inputs this artifact was compiled from.
    ///
    /// Optional at the API level only: the empty identity an unset builder
    /// carries is itself a well-formed binding that says "compiled from
    /// nothing", and it occupies a zero-length identity region rather than an
    /// absent one (see [`Identity::new`]).
    pub fn identity(&mut self, identity: Identity) -> &mut Self {
        self.identity = identity;
        self
    }

    /// Supply one section's raw bytes.
    ///
    /// `bytes` may be empty — a zero-length section is VALID and is how a
    /// declared kind with nothing to say is spelled. Supplying the same `kind`
    /// twice is an error, reported by [`build_bytes`](Self::build_bytes) as
    /// [`ArtifactError::DuplicateSection`]; this method is infallible so that a
    /// codec can declare its whole directory in one chained statement.
    pub fn section(&mut self, kind: u32, bytes: &[u8]) -> &mut Self {
        self.sections.push((kind, bytes.to_vec()));
        self
    }

    /// Emit the complete artifact.
    ///
    /// A pure function of the spec, the identity and the sections: no
    /// hash-iteration order, no wall-clock and no RNG reach the output, so two
    /// calls over equal inputs produce byte-identical buffers.
    ///
    /// # Errors
    ///
    /// [`ArtifactError::SectionCountMismatch`] if the number of sections
    /// supplied is not [`ArtifactSpec::section_count`] — the totality law;
    /// [`ArtifactError::DuplicateSection`] if one kind was supplied twice;
    /// [`ArtifactError::Malformed`] if a length or offset overflows the format's
    /// fields (only reachable for inputs larger than this process can address).
    pub fn build_bytes(self) -> Result<Vec<u8>, ArtifactError> {
        let Self {
            spec,
            identity,
            mut sections,
        } = self;

        if sections.len() != spec.section_count {
            return Err(ArtifactError::SectionCountMismatch {
                expected: spec.section_count,
                found: sections.len(),
            });
        }

        // Canonical order. Duplicates are rejected immediately afterwards, so
        // an unstable sort's tie-breaking can never reach the output.
        sections.sort_unstable_by_key(|(kind, _)| *kind);
        for pair in sections.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(ArtifactError::DuplicateSection { kind: pair[0].0 });
            }
        }

        let identity_bytes = identity.to_bytes();

        // -- Plan every offset. The reader recomputes these identically and
        //    refuses any buffer that stores different ones, so this is the ONE
        //    admissible layout for this content.
        let directory_len =
            spec.section_count
                .checked_mul(ENTRY_LEN)
                .ok_or(ArtifactError::Malformed(
                    "artifact: section directory length overflows",
                ))?;
        let directory_end =
            HEADER_LEN
                .checked_add(directory_len)
                .ok_or(ArtifactError::Malformed(
                    "artifact: section directory overflows",
                ))?;
        let identity_offset = align_up(directory_end)?;
        let identity_end =
            identity_offset
                .checked_add(identity_bytes.len())
                .ok_or(ArtifactError::Malformed(
                    "artifact: identity span overflows",
                ))?;

        let mut cursor = align_up(identity_end)?;
        let mut offsets = Vec::with_capacity(sections.len());
        for (_, body) in &sections {
            offsets.push(cursor);
            let end = cursor
                .checked_add(body.len())
                .ok_or(ArtifactError::Malformed("artifact: section span overflows"))?;
            cursor = align_up(end)?;
        }
        let trailer_offset = cursor;
        let file_len = trailer_offset
            .checked_add(TRAILER_LEN)
            .ok_or(ArtifactError::Malformed("artifact: file length overflows"))?;

        let digests: Vec<[u8; 32]> = sections
            .iter()
            .map(|(_, body)| Sha256::digest(body).into())
            .collect();

        let mut out = Vec::with_capacity(file_len);

        // -- Header -----------------------------------------------------------
        out.extend_from_slice(&spec.magic);
        write_u32_le(&mut out, spec.format_version);
        write_u32_le(
            &mut out,
            u32::try_from(spec.section_count).map_err(|_| {
                ArtifactError::Malformed("artifact: section count exceeds the 32-bit field")
            })?,
        );
        write_u64_le(&mut out, to_u64(identity_offset)?);
        write_u64_le(&mut out, to_u64(identity_bytes.len())?);
        out.extend_from_slice(identity.digest());
        debug_assert_eq!(out.len(), HEADER_LEN);

        // -- Section directory -------------------------------------------------
        for (index, (kind, body)) in sections.iter().enumerate() {
            write_u32_le(&mut out, *kind);
            write_u64_le(&mut out, to_u64(offsets[index])?);
            write_u64_le(&mut out, to_u64(body.len())?);
            out.extend_from_slice(&digests[index]);
        }
        debug_assert_eq!(out.len(), directory_end);

        // -- Identity region, then section bodies, each zero-padded up ---------
        pad_to(&mut out, identity_offset);
        out.extend_from_slice(&identity_bytes);
        for (index, (_, body)) in sections.iter().enumerate() {
            pad_to(&mut out, offsets[index]);
            out.extend_from_slice(body);
        }
        pad_to(&mut out, trailer_offset);
        debug_assert_eq!(out.len(), trailer_offset);

        // -- Trailer, sealing every byte ahead of it ---------------------------
        let container_digest: [u8; 32] = Sha256::digest(&out).into();
        out.extend_from_slice(&TRAILER_MAGIC);
        write_u32_le(&mut out, spec.format_version);
        write_u32_le(&mut out, 0); // reserved
        write_u64_le(&mut out, to_u64(file_len)?);
        out.extend_from_slice(&container_digest);
        out.extend_from_slice(&[0u8; 8]); // reserved
        debug_assert_eq!(out.len(), file_len);

        Ok(out)
    }
}

/// Zero-pad `out` up to `offset`.
fn pad_to(out: &mut Vec<u8>, offset: usize) {
    while out.len() < offset {
        out.push(0);
    }
}

/// Whether every byte in `bytes` is zero — the padding law.
fn all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

// ---------------------------------------------------------------------------
// ArtifactView
// ---------------------------------------------------------------------------

/// The borrowed, zero-copy reader over [`ArtifactBuilder::build_bytes`]'s
/// output: section bodies alias the caller's buffer directly.
///
/// Holding one is a statement that every check in
/// [`from_bytes`](Self::from_bytes) passed, so a codec reading from it can treat
/// the bytes as exactly what a builder in this workspace wrote.
#[derive(Debug, Clone)]
pub struct ArtifactView<'a> {
    spec: ArtifactSpec,
    identity: Identity,
    identity_digest: [u8; 32],
    container_digest: [u8; 32],
    sections: Vec<(u32, &'a [u8])>,
}

impl<'a> ArtifactView<'a> {
    /// Parse and fully verify an artifact against `spec`.
    ///
    /// Fails closed, in this order: total length, magic, format version, section
    /// count, the directory's ascending-kind order and canonical offsets, the
    /// zero padding between regions, EACH section's SHA-256, the identity
    /// region's digest and canonical framing, the trailer's structure and
    /// recorded length, and finally the whole-container digest. The order runs
    /// most-specific first so the error names the smallest thing that is wrong;
    /// every check runs before any `Ok` is produced, so the order changes which
    /// refusal a caller sees and never whether one happens.
    ///
    /// # Errors
    ///
    /// See [`ArtifactError`] for every refusal this can return.
    pub fn from_bytes(spec: ArtifactSpec, bytes: &'a [u8]) -> Result<Self, ArtifactError> {
        if bytes.len() < HEADER_LEN {
            return Err(ArtifactError::Truncated);
        }
        if bytes[0..8] != spec.magic {
            return Err(ArtifactError::BadMagic);
        }

        let mut pos = 8usize;
        let version = read_u32_le(bytes, &mut pos);
        if version != spec.format_version {
            return Err(ArtifactError::UnsupportedVersion(version));
        }
        let stored_count = to_usize(
            u64::from(read_u32_le(bytes, &mut pos)),
            "artifact: section count exceeds usize",
        )?;
        if stored_count != spec.section_count {
            return Err(ArtifactError::SectionCountMismatch {
                expected: spec.section_count,
                found: stored_count,
            });
        }
        let identity_offset = to_usize(
            read_u64_le(bytes, &mut pos),
            "artifact: identity offset exceeds usize",
        )?;
        let identity_len = to_usize(
            read_u64_le(bytes, &mut pos),
            "artifact: identity length exceeds usize",
        )?;
        let identity_digest = read_digest(bytes, &mut pos);
        debug_assert_eq!(pos, HEADER_LEN);

        // -- Directory bounds --------------------------------------------------
        let directory_len = stored_count
            .checked_mul(ENTRY_LEN)
            .ok_or(ArtifactError::Malformed(
                "artifact: section directory length overflows",
            ))?;
        let directory_end =
            HEADER_LEN
                .checked_add(directory_len)
                .ok_or(ArtifactError::Malformed(
                    "artifact: section directory overflows",
                ))?;
        if bytes.len() < directory_end {
            return Err(ArtifactError::Truncated);
        }

        // -- Identity region, at its one canonical offset ----------------------
        if identity_offset != align_up(directory_end)? {
            return Err(ArtifactError::Malformed(
                "artifact: identity region is not at its canonical offset",
            ));
        }
        let identity_end =
            identity_offset
                .checked_add(identity_len)
                .ok_or(ArtifactError::Malformed(
                    "artifact: identity span overflows",
                ))?;
        let identity_bytes = bytes
            .get(identity_offset..identity_end)
            .ok_or(ArtifactError::Truncated)?;
        check_padding(bytes, directory_end, identity_offset)?;

        // -- Section directory -------------------------------------------------
        let mut cursor = align_up(identity_end)?;
        let mut unpadded_end = identity_end;
        let mut sections: Vec<(u32, &'a [u8])> = Vec::with_capacity(stored_count);
        let mut previous: Option<u32> = None;
        for _ in 0..stored_count {
            let kind = read_u32_le(bytes, &mut pos);
            let offset = read_u64_le(bytes, &mut pos);
            let len = read_u64_le(bytes, &mut pos);
            let stored_digest = read_digest(bytes, &mut pos);

            if let Some(prev) = previous {
                if kind == prev {
                    return Err(ArtifactError::DuplicateSection { kind });
                }
                if kind < prev {
                    return Err(ArtifactError::Malformed(
                        "artifact: section directory is not in ascending kind order",
                    ));
                }
            }
            previous = Some(kind);

            let offset = to_usize(offset, "artifact: section offset exceeds usize")?;
            let len = to_usize(len, "artifact: section length exceeds usize")?;
            if offset != cursor {
                return Err(ArtifactError::Malformed(
                    "artifact: section is not at its canonical offset",
                ));
            }
            let end = offset
                .checked_add(len)
                .ok_or(ArtifactError::Malformed("artifact: section span overflows"))?;
            let body = bytes.get(offset..end).ok_or(ArtifactError::Truncated)?;
            check_padding(bytes, unpadded_end, offset)?;

            let computed: [u8; 32] = Sha256::digest(body).into();
            if computed != stored_digest {
                return Err(ArtifactError::SectionDigestMismatch { kind });
            }

            sections.push((kind, body));
            unpadded_end = end;
            cursor = align_up(end)?;
        }
        debug_assert_eq!(pos, directory_end);

        let trailer_offset = cursor;
        let file_len = trailer_offset
            .checked_add(TRAILER_LEN)
            .ok_or(ArtifactError::Malformed("artifact: file length overflows"))?;
        if bytes.len() < file_len {
            return Err(ArtifactError::Truncated);
        }
        if bytes.len() > file_len {
            return Err(ArtifactError::Malformed(
                "artifact: trailing bytes after the trailer",
            ));
        }
        check_padding(bytes, unpadded_end, trailer_offset)?;

        // -- Identity: digest first, then its own framing ----------------------
        let computed_identity: [u8; 32] = Sha256::digest(identity_bytes).into();
        if computed_identity != identity_digest {
            return Err(ArtifactError::Malformed(
                "artifact: header identity digest disagrees with the identity region",
            ));
        }
        let identity = Identity::from_bytes(identity_bytes)?;

        // -- Trailer ------------------------------------------------------------
        let trailer = &bytes[trailer_offset..file_len];
        if trailer[0..8] != TRAILER_MAGIC {
            return Err(ArtifactError::TrailerMismatch);
        }
        let mut tpos = 8usize;
        let trailer_version = read_u32_le(trailer, &mut tpos);
        let trailer_reserved = read_u32_le(trailer, &mut tpos);
        let trailer_file_len = read_u64_le(trailer, &mut tpos);
        let trailer_digest = read_digest(trailer, &mut tpos);
        debug_assert_eq!(tpos, 56);
        if trailer_version != spec.format_version
            || trailer_reserved != 0
            || trailer_file_len != to_u64(file_len)?
            || !all_zero(&trailer[56..TRAILER_LEN])
        {
            return Err(ArtifactError::TrailerMismatch);
        }

        // -- The outermost seal --------------------------------------------------
        let container_digest: [u8; 32] = Sha256::digest(&bytes[..trailer_offset]).into();
        if container_digest != trailer_digest {
            return Err(ArtifactError::ContainerDigestMismatch {
                expected: trailer_digest,
                computed: container_digest,
            });
        }

        Ok(Self {
            spec,
            identity,
            identity_digest,
            container_digest,
            sections,
        })
    }

    /// One section's raw bytes, borrowed from the opened buffer.
    ///
    /// A zero-length result is a normal, valid answer. A kind this artifact does
    /// not carry is [`ArtifactError::MissingSection`] — never an empty slice,
    /// because the directory is total and "absent" and "empty" are different
    /// facts (see the [module docs](super)).
    ///
    /// # Errors
    ///
    /// [`ArtifactError::MissingSection`] if `kind` is not in the directory.
    pub fn section(&self, kind: u32) -> Result<&'a [u8], ArtifactError> {
        self.sections
            .binary_search_by_key(&kind, |(section_kind, _)| *section_kind)
            .map(|index| self.sections[index].1)
            .map_err(|_| ArtifactError::MissingSection { kind })
    }

    /// Every section kind this artifact carries, in ascending order.
    #[must_use]
    pub fn section_kinds(&self) -> Vec<u32> {
        self.sections.iter().map(|(kind, _)| *kind).collect()
    }

    /// The SHA-256 of the identity region, as recorded in the header and
    /// verified on open.
    #[must_use]
    pub fn identity_digest(&self) -> &[u8; 32] {
        &self.identity_digest
    }

    /// The decoded input binding: which inputs this artifact was compiled from.
    ///
    /// Use [`Identity::mismatches`] against the caller's own identity to report
    /// WHICH input moved rather than merely that one did.
    #[must_use]
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// The SHA-256 of every byte ahead of the trailer, verified on open — a
    /// stable name for this exact artifact.
    #[must_use]
    pub fn container_digest(&self) -> &[u8; 32] {
        &self.container_digest
    }

    /// The spec this artifact was opened against.
    #[must_use]
    pub fn spec(&self) -> ArtifactSpec {
        self.spec
    }
}

/// Verify that `bytes[from..to]` is the zero padding the layout requires.
///
/// Padding is inside the container digest's coverage, so tampering with it is
/// caught either way; checking it here names the problem precisely and denies
/// the alignment gaps their only use as a covert channel.
fn check_padding(bytes: &[u8], from: usize, to: usize) -> Result<(), ArtifactError> {
    let padding = bytes.get(from..to).ok_or(ArtifactError::Truncated)?;
    if all_zero(padding) {
        Ok(())
    } else {
        Err(ArtifactError::Malformed(
            "artifact: alignment padding is not zero",
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const MAGIC: [u8; 8] = *b"PURRTST1";
    const VERSION: u32 = 1;

    const KIND_A: u32 = 1;
    const KIND_B: u32 = 2;
    const KIND_C: u32 = 7;

    fn spec(section_count: usize) -> ArtifactSpec {
        ArtifactSpec::new(MAGIC, VERSION, section_count)
    }

    fn sample_identity() -> Identity {
        Identity::new()
            .with("base", b"https://example.org/shapes")
            .with("crate-version", b"0.0.0-test")
    }

    /// The canonical three-section fixture: one ordinary body, one zero-length
    /// body, and one body that happens to start with the header magic.
    fn sample_bytes() -> Vec<u8> {
        let mut builder = ArtifactBuilder::new(spec(3));
        builder.identity(sample_identity());
        builder
            .section(KIND_B, b"body-b")
            .section(KIND_A, &[0xaa; 40])
            .section(KIND_C, b"");
        builder.build_bytes().expect("fixture builds")
    }

    /// The absolute offset of `kind`'s directory entry within the buffer.
    fn entry_offset(index: usize) -> usize {
        HEADER_LEN + index * ENTRY_LEN
    }

    // -- Round trip ---------------------------------------------------------

    #[test]
    fn round_trip_sections_preserves_bytes() {
        let bytes = sample_bytes();
        let view = ArtifactView::from_bytes(spec(3), &bytes).expect("opens");

        assert_eq!(view.section(KIND_A).expect("A"), &[0xaa; 40][..]);
        assert_eq!(view.section(KIND_B).expect("B"), b"body-b");
        assert_eq!(view.section(KIND_C).expect("C"), b"");
        assert_eq!(view.section_kinds(), vec![KIND_A, KIND_B, KIND_C]);
        assert_eq!(view.identity(), &sample_identity());
        assert_eq!(view.identity_digest(), sample_identity().digest());
        assert_eq!(view.spec(), spec(3));
        assert_eq!(&bytes[..8], &MAGIC);
        assert_eq!(&bytes[bytes.len() - TRAILER_LEN..][..8], &TRAILER_MAGIC);
        assert_eq!(bytes.len() % ALIGNMENT, 0);
    }

    #[test]
    fn build_bytes_is_deterministic() {
        assert_eq!(sample_bytes(), sample_bytes());
    }

    #[test]
    fn insertion_order_does_not_reach_the_bytes() {
        let mut reversed = ArtifactBuilder::new(spec(3));
        reversed.identity(sample_identity());
        reversed
            .section(KIND_C, b"")
            .section(KIND_A, &[0xaa; 40])
            .section(KIND_B, b"body-b");
        assert_eq!(reversed.build_bytes().expect("builds"), sample_bytes());
    }

    #[test]
    fn container_digest_changes_with_any_byte_of_content() {
        let base = *ArtifactView::from_bytes(spec(3), &sample_bytes())
            .expect("opens")
            .container_digest();

        let mut changed = ArtifactBuilder::new(spec(3));
        changed.identity(sample_identity());
        changed
            .section(KIND_A, &[0xaa; 40])
            .section(KIND_B, b"body-B")
            .section(KIND_C, b"");
        let changed = changed.build_bytes().expect("builds");
        let other = *ArtifactView::from_bytes(spec(3), &changed)
            .expect("opens")
            .container_digest();
        assert_ne!(base, other);

        // The identity is inside the seal too.
        let mut reidentified = ArtifactBuilder::new(spec(3));
        reidentified.identity(Identity::new().with("base", b"https://example.org/other"));
        reidentified
            .section(KIND_A, &[0xaa; 40])
            .section(KIND_B, b"body-b")
            .section(KIND_C, b"");
        let reidentified = reidentified.build_bytes().expect("builds");
        assert_ne!(
            base,
            *ArtifactView::from_bytes(spec(3), &reidentified)
                .expect("opens")
                .container_digest()
        );
    }

    // -- Refusals -----------------------------------------------------------

    #[test]
    fn refuses_bad_magic() {
        let mut bytes = sample_bytes();
        bytes[0] ^= 0xff;
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::BadMagic
        );
    }

    #[test]
    fn refuses_unsupported_version() {
        let mut bytes = sample_bytes();
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::UnsupportedVersion(2)
        );

        // And the symmetric direction: a v1 buffer opened by a v2 reader.
        let v2 = ArtifactSpec::new(MAGIC, 2, 3);
        assert_eq!(
            ArtifactView::from_bytes(v2, &sample_bytes()).unwrap_err(),
            ArtifactError::UnsupportedVersion(1)
        );
    }

    #[test]
    fn refuses_truncated_body() {
        let bytes = sample_bytes();
        // Cut inside the first section body.
        let cut = &bytes[..HEADER_LEN + 3 * ENTRY_LEN + 16];
        assert_eq!(
            ArtifactView::from_bytes(spec(3), cut).unwrap_err(),
            ArtifactError::Truncated
        );
        // Cut inside the header.
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &bytes[..HEADER_LEN - 1]).unwrap_err(),
            ArtifactError::Truncated
        );
        // Cut inside the directory.
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &bytes[..HEADER_LEN + ENTRY_LEN]).unwrap_err(),
            ArtifactError::Truncated
        );
    }

    #[test]
    fn refuses_truncated_trailer() {
        let bytes = sample_bytes();
        let cut = &bytes[..bytes.len() - 8];
        assert_eq!(
            ArtifactView::from_bytes(spec(3), cut).unwrap_err(),
            ArtifactError::Truncated
        );
    }

    #[test]
    fn refuses_appended_bytes() {
        let mut bytes = sample_bytes();
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::Malformed(_)
        ));
    }

    #[test]
    fn refuses_flipped_section_byte() {
        let bytes = sample_bytes();
        // KIND_A's body is the first section; find its offset from the directory.
        let mut pos = entry_offset(0);
        let kind = read_u32_le(&bytes, &mut pos);
        assert_eq!(kind, KIND_A);
        let offset = read_u64_le(&bytes, &mut pos) as usize;

        let mut tampered = bytes;
        tampered[offset] ^= 0x01;
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &tampered).unwrap_err(),
            ArtifactError::SectionDigestMismatch { kind: KIND_A }
        );
    }

    #[test]
    fn refuses_tampered_container_digest() {
        let mut bytes = sample_bytes();
        let trailer = bytes.len() - TRAILER_LEN;
        bytes[trailer + 24] ^= 0x01;
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::ContainerDigestMismatch { .. }
        ));
    }

    #[test]
    fn refuses_tampered_trailer_fields() {
        for offset in [0usize, 8, 12, 16, 56] {
            let mut bytes = sample_bytes();
            let trailer = bytes.len() - TRAILER_LEN;
            bytes[trailer + offset] ^= 0x01;
            assert_eq!(
                ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
                ArtifactError::TrailerMismatch,
                "trailer byte {offset} must be load-bearing"
            );
        }
    }

    #[test]
    fn refuses_duplicate_section() {
        // At build time.
        let mut builder = ArtifactBuilder::new(spec(2));
        builder.section(KIND_A, b"one").section(KIND_A, b"two");
        assert_eq!(
            builder.build_bytes().unwrap_err(),
            ArtifactError::DuplicateSection { kind: KIND_A }
        );

        // And in a buffer whose directory was edited to repeat a kind.
        let mut bytes = sample_bytes();
        bytes[entry_offset(1)..entry_offset(1) + 4].copy_from_slice(&KIND_A.to_le_bytes());
        assert_eq!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::DuplicateSection { kind: KIND_A }
        );
    }

    #[test]
    fn refuses_descending_directory_order() {
        let mut bytes = sample_bytes();
        bytes[entry_offset(1)..entry_offset(1) + 4].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::Malformed(_)
        ));
    }

    #[test]
    fn refuses_missing_section() {
        let bytes = sample_bytes();
        let view = ArtifactView::from_bytes(spec(3), &bytes).expect("opens");
        assert_eq!(
            view.section(99).unwrap_err(),
            ArtifactError::MissingSection { kind: 99 }
        );
    }

    #[test]
    fn refuses_section_count_mismatch() {
        // At build time: two sections for a three-section spec.
        let mut builder = ArtifactBuilder::new(spec(3));
        builder.section(KIND_A, b"a").section(KIND_B, b"b");
        assert_eq!(
            builder.build_bytes().unwrap_err(),
            ArtifactError::SectionCountMismatch {
                expected: 3,
                found: 2
            }
        );

        // At open time: a three-section buffer read by a two-section spec.
        assert_eq!(
            ArtifactView::from_bytes(spec(2), &sample_bytes()).unwrap_err(),
            ArtifactError::SectionCountMismatch {
                expected: 2,
                found: 3
            }
        );
    }

    #[test]
    fn refuses_non_zero_padding() {
        let bytes = sample_bytes();
        // KIND_B's body is 6 bytes, so the two bytes after it are padding.
        let mut pos = entry_offset(1);
        assert_eq!(read_u32_le(&bytes, &mut pos), KIND_B);
        let offset = read_u64_le(&bytes, &mut pos) as usize;
        let len = read_u64_le(&bytes, &mut pos) as usize;
        assert_eq!(len, 6);

        let mut tampered = bytes;
        tampered[offset + len] = 0x01;
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &tampered).unwrap_err(),
            ArtifactError::Malformed(_)
        ));
    }

    #[test]
    fn refuses_relocated_section() {
        let mut bytes = sample_bytes();
        let mut pos = entry_offset(0);
        let _kind = read_u32_le(&bytes, &mut pos);
        let offset = read_u64_le(&bytes, &mut pos);
        bytes[pos - 8..pos].copy_from_slice(&(offset + ALIGNMENT as u64).to_le_bytes());
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::Malformed(_)
        ));
    }

    #[test]
    fn refuses_tampered_identity_region() {
        let mut bytes = sample_bytes();
        let mut pos = 16usize;
        let identity_offset = read_u64_le(&bytes, &mut pos) as usize;
        bytes[identity_offset] ^= 0x01;
        assert!(matches!(
            ArtifactView::from_bytes(spec(3), &bytes).unwrap_err(),
            ArtifactError::Malformed(_)
        ));
    }

    // -- Valid neighbours: one per refusal class ----------------------------

    #[test]
    fn accepts_current_version() {
        // The neighbour of `refuses_unsupported_version`: the declared version
        // opens, so the refusal is about the version and not about opening.
        assert!(ArtifactView::from_bytes(spec(3), &sample_bytes()).is_ok());
    }

    #[test]
    fn accepts_zero_length_section() {
        // The neighbour of every "missing" refusal: a declared kind with no
        // bytes is VALID and reads back as an empty slice, never as absent.
        let mut builder = ArtifactBuilder::new(spec(2));
        builder.section(KIND_A, b"").section(KIND_B, b"");
        let bytes = builder.build_bytes().expect("empty sections build");
        let view = ArtifactView::from_bytes(spec(2), &bytes).expect("opens");
        assert_eq!(view.section(KIND_A).expect("A"), b"");
        assert_eq!(view.section(KIND_B).expect("B"), b"");
        assert_eq!(view.section_kinds(), vec![KIND_A, KIND_B]);
    }

    #[test]
    fn accepts_single_section() {
        // The neighbour of `refuses_section_count_mismatch`: the smallest
        // non-degenerate directory still opens.
        let mut builder = ArtifactBuilder::new(spec(1));
        builder.section(KIND_A, b"only");
        let bytes = builder.build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(1), &bytes).expect("opens");
        assert_eq!(view.section(KIND_A).expect("A"), b"only");
    }

    #[test]
    fn accepts_zero_sections() {
        // The degenerate directory: a spec that declares no sections at all is
        // still a well-formed artifact, not an over-refused edge case.
        let bytes = ArtifactBuilder::new(spec(0)).build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(0), &bytes).expect("opens");
        assert_eq!(view.section_kinds(), Vec::<u32>::new());
        assert_eq!(
            view.section(KIND_A).unwrap_err(),
            ArtifactError::MissingSection { kind: KIND_A }
        );
    }

    #[test]
    fn accepts_payload_beginning_with_magic_bytes() {
        // The neighbour of `refuses_bad_magic`: a section body whose first eight
        // bytes ARE the header magic is ordinary content, not a nested artifact,
        // and must open exactly like any other body. Framing is by the
        // directory's offsets, never by scanning for a magic.
        let mut payload = MAGIC.to_vec();
        payload.extend_from_slice(&TRAILER_MAGIC);
        payload.extend_from_slice(b"still just bytes");

        let mut builder = ArtifactBuilder::new(spec(1));
        builder.section(KIND_A, &payload);
        let bytes = builder.build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(1), &bytes).expect("opens");
        assert_eq!(view.section(KIND_A).expect("A"), &payload[..]);
    }

    #[test]
    fn accepts_a_whole_artifact_as_a_section_body() {
        // The strongest form of the magic-scanning neighbour: nesting a complete
        // artifact inside another opens both, outer first.
        let inner = sample_bytes();
        let mut builder = ArtifactBuilder::new(spec(1));
        builder.section(KIND_A, &inner);
        let outer = builder.build_bytes().expect("builds");

        let view = ArtifactView::from_bytes(spec(1), &outer).expect("outer opens");
        let recovered = view.section(KIND_A).expect("A");
        assert_eq!(recovered, &inner[..]);
        assert!(ArtifactView::from_bytes(spec(3), recovered).is_ok());
    }

    #[test]
    fn accepts_unmodified_bytes_after_a_tampering_test() {
        // The neighbour of `refuses_flipped_section_byte` and
        // `refuses_tampered_container_digest`: flipping the bit BACK restores a
        // buffer that opens, so the refusal tracks the bit and not the test.
        let mut bytes = sample_bytes();
        let mut pos = entry_offset(0);
        let _kind = read_u32_le(&bytes, &mut pos);
        let offset = read_u64_le(&bytes, &mut pos) as usize;

        bytes[offset] ^= 0x01;
        assert!(ArtifactView::from_bytes(spec(3), &bytes).is_err());
        bytes[offset] ^= 0x01;
        assert!(ArtifactView::from_bytes(spec(3), &bytes).is_ok());
    }

    #[test]
    fn accepts_adjacent_distinct_kinds() {
        // The neighbour of `refuses_duplicate_section`: consecutive kinds are
        // not duplicates.
        let mut builder = ArtifactBuilder::new(spec(3));
        builder
            .section(5, b"five")
            .section(6, b"six")
            .section(7, b"seven");
        let bytes = builder.build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(3), &bytes).expect("opens");
        assert_eq!(view.section(6).expect("six"), b"six");
    }

    #[test]
    fn accepts_extreme_kind_tags() {
        // Kind tags are opaque `u32`s: the bounds of the space are ordinary.
        let mut builder = ArtifactBuilder::new(spec(2));
        builder.section(u32::MAX, b"max").section(0, b"zero");
        let bytes = builder.build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(2), &bytes).expect("opens");
        assert_eq!(view.section_kinds(), vec![0, u32::MAX]);
        assert_eq!(view.section(0).expect("zero"), b"zero");
        assert_eq!(view.section(u32::MAX).expect("max"), b"max");
    }

    #[test]
    fn accepts_an_empty_identity() {
        // The neighbour of `refuses_tampered_identity_region`: a zero-length
        // identity region is valid, and its digest is the empty-string digest.
        let mut builder = ArtifactBuilder::new(spec(1));
        builder.section(KIND_A, b"a");
        let bytes = builder.build_bytes().expect("builds");
        let view = ArtifactView::from_bytes(spec(1), &bytes).expect("opens");
        assert_eq!(view.identity(), &Identity::new());
        assert_eq!(view.identity_digest(), Identity::new().digest());
    }

    #[test]
    fn identity_mismatch_survives_the_round_trip() {
        // The whole reason the identity is decodable rather than an opaque fold.
        let bytes = sample_bytes();
        let view = ArtifactView::from_bytes(spec(3), &bytes).expect("opens");
        let caller = Identity::new()
            .with("base", b"https://example.org/other")
            .with("crate-version", b"0.0.0-test");
        let mismatches = view.identity().mismatches(&caller);
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].to_string().starts_with("component `base`"));
    }

    #[test]
    fn layout_constants_agree_with_the_documented_tables() {
        assert_eq!(HEADER_LEN, 64);
        assert_eq!(ENTRY_LEN, 52);
        assert_eq!(TRAILER_LEN, 64);
        assert_eq!(ALIGNMENT, 8);
        assert_eq!(align_up(0).expect("aligns"), 0);
        assert_eq!(align_up(1).expect("aligns"), 8);
        assert_eq!(align_up(8).expect("aligns"), 8);
        assert_eq!(align_up(9).expect("aligns"), 16);
        assert!(align_up(usize::MAX).is_err());
    }
}
