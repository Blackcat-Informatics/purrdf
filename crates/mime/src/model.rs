// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::ops::Range;

use purrdf_core::{ContentDigest, cover::ByteCover};

use crate::{MimeError, transfer};

/// Caller-owned original message bytes and absolute source identifier.
#[derive(Clone, Copy, Debug)]
pub struct SourceDocument<'a> {
    /// Absolute caller source identity.
    pub id: &'a str,
    /// Exact original bytes, irrespective of encoding or conformance.
    pub bytes: &'a [u8],
}

/// One physical header occurrence, including malformed field lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// Original containing part occurrence.
    pub part: usize,
    /// Header order within its part, including repeats and invalid fields.
    pub ordinal: usize,
    /// Entire physical range, folding and line endings included.
    pub span: Range<usize>,
    /// Exact field name, absent for an invalid field line.
    pub name: Option<Range<usize>>,
    /// Value segments in order, retaining original horizontal whitespace.
    pub segments: Vec<Range<usize>>,
}

impl Header {
    /// Original bytes of this occurrence, without unfolding or charset guessing.
    pub fn raw<'a>(&self, source: &'a [u8]) -> &'a [u8] {
        &source[self.span.clone()]
    }

    /// RFC5322 unfolding removes physical line endings only. Every retained
    /// segment is a source slice; original folding remains in span/segments.
    pub fn unfolded(&self, source: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for segment in &self.segments {
            bytes.extend_from_slice(&source[segment.clone()]);
        }
        bytes
    }

    pub(crate) fn named(&self, source: &[u8], name: &[u8]) -> bool {
        self.name
            .as_ref()
            .is_some_and(|range| source[range.clone()].eq_ignore_ascii_case(name))
    }
}

/// Declared media type and ordered parameter occurrences, including duplicates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaType {
    /// ASCII identity fold of the declared type token.
    pub main: Vec<u8>,
    /// ASCII identity fold of the declared subtype token.
    pub sub: Vec<u8>,
    /// Parameters in source order; names are ASCII-folded, values unchanged.
    pub parameters: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Declared transfer encoding. Unknown or ambiguous declarations are never
/// silently substituted with a known decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransferEncoding {
    /// RFC2045 default, or explicit 7bit.
    SevenBit,
    /// Explicit 8bit.
    EightBit,
    /// Explicit binary.
    Binary,
    /// Base64, decoded through the existing native binary home.
    Base64,
    /// RFC2045 quoted-printable.
    QuotedPrintable,
    /// Unsupported token, retained unchanged.
    Unknown(Vec<u8>),
    /// Conflicting singleton declarations.
    Ambiguous,
}

impl TransferEncoding {
    /// Stable typed spelling independent of source token casing.
    pub fn name(&self) -> &str {
        match self {
            Self::SevenBit => "7bit",
            Self::EightBit => "8bit",
            Self::Binary => "binary",
            Self::Base64 => "base64",
            Self::QuotedPrintable => "quoted-printable",
            Self::Unknown(_) => "unknown",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// A flat part occurrence: children name indices, so depth never controls
/// native stack use during parsing, projection, cloning or destruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// Parent occurrence, absent only for the root message.
    pub parent: Option<usize>,
    /// Child order under its parent.
    pub ordinal: usize,
    /// Nesting depth, with zero at the root.
    pub depth: usize,
    /// Exact part bytes, excluding its parent's delimiter-owned CRLF.
    pub span: Range<usize>,
    /// Original body bytes under this part's header separator.
    pub body: Range<usize>,
    /// Original ordered header occurrence indices.
    pub headers: Vec<usize>,
    /// Original ordered child occurrence indices.
    pub children: Vec<usize>,
    /// Parsed explicit or RFC-default media type; None denotes invalid/ambiguous declarations.
    pub media_type: Option<MediaType>,
    /// Exact declared transfer policy or a typed unsupported/ambiguous state.
    pub transfer_encoding: TransferEncoding,
}

/// Non-payload structure inside a multipart body, still present in the byte cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureKind {
    /// Original bytes preceding the first declared delimiter.
    Preamble,
    /// A delimiter introducing a child part, including parent-owned CRLF.
    Boundary,
    /// The declared closing delimiter ending the multipart body.
    ClosingBoundary,
    /// Original bytes following the closing delimiter.
    Epilogue,
}

impl StructureKind {
    /// Stable occurrence kind for RDF/query projection.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Preamble => "preamble",
            Self::Boundary => "boundary",
            Self::ClosingBoundary => "closingBoundary",
            Self::Epilogue => "epilogue",
        }
    }
}

/// An original structural occurrence under a part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Structure {
    /// Containing multipart part.
    pub part: usize,
    /// Original source range.
    pub span: Range<usize>,
    /// Typed structural role.
    pub kind: StructureKind,
}

/// A defect observed in the original bytes; it is not a guessed repair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemKind {
    /// A physical field line has no valid field-name/colon production.
    InvalidHeader,
    /// A folded continuation has no preceding valid field occurrence.
    OrphanFold,
    /// A physical line ends with LF without the required preceding CR.
    BareLf,
    /// A physical line uses CR without the following LF.
    BareCr,
    /// Original header bytes contain a non-ASCII octet.
    NonAsciiHeader,
    /// No empty physical line separates a part's fields and body.
    MissingHeaderSeparator,
    /// A MIME structural singleton field occurs more than once.
    DuplicateStructuralHeader,
    /// Repeated structural declarations disagree; no first-value repair is chosen.
    ConflictingStructuralHeaders,
    /// A media-type or boundary parameter does not define an unambiguous structure.
    InvalidContentType,
    /// A declared multipart boundary is absent from the declaration or body.
    MissingBoundary,
    /// A closing delimiter appears before any opening delimiter.
    MissingOpeningBoundary,
    /// A started multipart body has no matching closing delimiter.
    MissingClosingBoundary,
    /// An original delimiter candidate does not match the declared boundary.
    UnexpectedBoundary,
    /// Transfer spelling or payload does not admit a defined exact decoder.
    InvalidTransferEncoding,
    /// An encoded container cannot define child windows in original source bytes.
    UnsupportedContainerEncoding,
    /// A header encoded-word has broken delimiters, encoding token or transfer syntax.
    InvalidEncodedWord,
}

impl ProblemKind {
    /// Stable typed spelling for lossless RDF occurrences.
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidHeader => "invalidHeader",
            Self::OrphanFold => "orphanFold",
            Self::BareLf => "bareLf",
            Self::BareCr => "bareCr",
            Self::NonAsciiHeader => "nonAsciiHeader",
            Self::MissingHeaderSeparator => "missingHeaderSeparator",
            Self::DuplicateStructuralHeader => "duplicateStructuralHeader",
            Self::ConflictingStructuralHeaders => "conflictingStructuralHeaders",
            Self::InvalidContentType => "invalidContentType",
            Self::MissingBoundary => "missingBoundary",
            Self::MissingOpeningBoundary => "missingOpeningBoundary",
            Self::MissingClosingBoundary => "missingClosingBoundary",
            Self::UnexpectedBoundary => "unexpectedBoundary",
            Self::InvalidTransferEncoding => "invalidTransferEncoding",
            Self::UnsupportedContainerEncoding => "unsupportedContainerEncoding",
            Self::InvalidEncodedWord => "invalidEncodedWord",
        }
    }
}

/// One typed problem at an unchanged original byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Containing part occurrence.
    pub part: usize,
    /// Original source range, possibly empty for a missing separator/boundary.
    pub span: Range<usize>,
    /// Closed typed observation.
    pub kind: ProblemKind,
}

/// Exact deterministic decoded attachment bytes and their replicable identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedAttachment {
    /// Decoded octets, with no newline or charset conversion.
    pub bytes: Vec<u8>,
    /// The shared content digest of those decoded octets.
    pub digest: ContentDigest,
}

/// Lossless flat MIME model borrowing original source, with one verified cover.
#[derive(Debug)]
pub struct Document<'a> {
    /// Exact caller source descriptor.
    pub source: SourceDocument<'a>,
    /// Complete byte partition emitted under the shared cover law.
    pub cover: ByteCover<'a>,
    /// Part occurrences in canonical source traversal order.
    pub parts: Vec<Part>,
    /// Header occurrences including identical repeats and malformed lines.
    pub headers: Vec<Header>,
    /// Preamble/delimiter/epilogue occurrences.
    pub structures: Vec<Structure>,
    /// Typed original defects, without normalization or repair.
    pub problems: Vec<Problem>,
    pub(crate) id: String,
    pub(crate) profile_id: ContentDigest,
    pub(crate) profile: crate::Profile,
}

impl Document<'_> {
    /// Deterministic document identity bound to source/profile/content.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Reproduce a part's decoded attachment directly from covered bytes,
    /// including an attached message whose internal structure has children.
    /// Filename and containing-message identity do not enter its content digest.
    ///
    /// # Errors
    /// Refuses unknown parts, unknown/conflicting encodings or broken
    /// transfer syntax. No partial decoded payload is returned.
    pub fn decoded_attachment(&self, part: usize) -> Result<DecodedAttachment, MimeError> {
        self.checked_part(part)?;
        self.decode_original_part(part)
    }

    /// Cover a nested part's exact original octets under the shared identity
    /// emitter. Its model coordinates remain absolute in the containing source;
    /// this independent cover uses coordinates relative to the nested bytes.
    ///
    /// # Errors
    /// Refuses an unknown part or changed source/part declaration.
    pub fn part_cover(&self, part: usize) -> Result<ByteCover<'_>, MimeError> {
        let entry = self.checked_part(part)?;
        Ok(ByteCover::identity(&self.source.bytes[entry.span.clone()]))
    }

    fn checked_part(&self, part: usize) -> Result<&Part, MimeError> {
        let entry = self.parts.get(part).ok_or(MimeError::UnknownPart(part))?;
        let original = crate::analyze(self.source, &self.profile)?;
        if original.id != self.id
            || original.cover != self.cover
            || original.parts.get(part) != Some(entry)
        {
            return Err(crate::error::metadata(self.id(), "source part"));
        }
        Ok(entry)
    }

    // Projection already verified every model occurrence against source. It
    // calls this exact decoder without repeating whole-message parsing per part.
    pub(crate) fn decode_original_part(&self, part: usize) -> Result<DecodedAttachment, MimeError> {
        let entry = self.parts.get(part).ok_or(MimeError::UnknownPart(part))?;
        let bytes = transfer::decode(
            &self.source.bytes[entry.body.clone()],
            &entry.transfer_encoding,
        )
        .map_err(|error| MimeError::Transfer {
            part,
            at: entry.body.start + error.at,
            reason: error.reason,
        })?;
        let digest = ContentDigest::of(&bytes);
        Ok(DecodedAttachment { bytes, digest })
    }
}
