// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Why a document could not be sliced.
//!
//! Every refusal the crate can state lives here, in one enum, and every
//! one of them is answered for at the seam — [`analyze`](crate::analyze)
//! — before a claim exists. That is what lets the projection
//! ([`render`](crate::render)) be infallible: a [`Document`](crate::Document)
//! can only be obtained by passing every check this enum names, and it
//! keeps the profile it passed them under
//! ([`Document::profile`](crate::Document::profile)), which is the only
//! law the projection applies. The checks here are therefore checks of
//! the law that will actually be emitted under, and not of a law some
//! later call might substitute — `render` takes the document alone and
//! has no second argument to substitute one with.
//!
//! One refusal is stated after a document exists rather than before:
//! [`MarkdownError::TamperedUnit`], which
//! [`verify_unit`](crate::verify_unit) states about bytes handed to it
//! *later*. It is not an exception to the rule above — nothing it
//! refuses could have been known at slicing time.
//!
//! # A refusal borrowed from another law states that law's finding
//!
//! Four variants carry a `cause` rather than a sentence of their own:
//! [`TamperedUnit`](MarkdownError::TamperedUnit), whose cause is the
//! kernel's verification law, and the three malformed-IRI refusals
//! ([`MalformedSourceId`](MarkdownError::MalformedSourceId),
//! [`MalformedVocabulary`](MarkdownError::MalformedVocabulary),
//! [`MalformedCanonBase`](MarkdownError::MalformedCanonBase)), whose
//! cause is the workspace IRI law's. This crate does not own those
//! laws, so it does not get to word their findings: it names the seam
//! that asked and hands the finding through. A truncated
//! percent-encoding is reported as a truncated percent-encoding, and at
//! the byte it is at, because the law that found it says so.

use purrdf_core::IriError;
use purrdf_core::embedding::EmbeddingError;

/// Why a document could not be sliced.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownError {
    /// The bytes are not UTF-8; the offset is where decoding stopped.
    InvalidUtf8 {
        /// The byte offset of the first invalid sequence.
        valid_up_to: usize,
    },
    /// The source id is empty. It would be written `<>`, a relative IRI
    /// reference that names whatever document happens to hold the
    /// claims, so every node the slicer minted would point at nothing
    /// fixed.
    EmptySourceId,
    /// The source id cannot be written as an IRI reference.
    InvalidSourceId {
        /// The offending character.
        found: char,
    },
    /// The source id is writable and is an IRI reference, but is not
    /// absolute: it carries no scheme, so it names a document only
    /// relative to whatever holds the claims, and every node minted
    /// over it inherits that.
    RelativeSourceId {
        /// The id as declared.
        id: String,
    },
    /// The source id is no IRI reference at all — the workspace IRI law
    /// cannot read it — so the question of a scheme never arises.
    ///
    /// It is a different finding from [`Self::RelativeSourceId`], and
    /// deliberately so: `https://example.org/%` ends in a truncated
    /// percent-encoding and plainly carries a scheme, and an id told it
    /// "has no scheme" would send its author looking for a defect that
    /// is not there.
    MalformedSourceId {
        /// The id as declared.
        id: String,
        /// What the workspace IRI law found, carried rather than
        /// restated.
        cause: IriError,
    },
    /// A vocabulary IRI is empty, cannot be written as an IRI
    /// reference, or is an IRI reference carrying no scheme.
    InvalidVocabulary {
        /// The vocabulary field.
        field: &'static str,
        /// The offending value.
        iri: String,
    },
    /// A vocabulary IRI is writable but is no IRI reference at all, so
    /// it states no term — the same distinction
    /// [`Self::MalformedSourceId`] draws, drawn at the field that
    /// carried it.
    MalformedVocabulary {
        /// The vocabulary field.
        field: &'static str,
        /// The offending value.
        iri: String,
        /// What the workspace IRI law found, carried rather than
        /// restated.
        cause: IriError,
    },
    /// The profile's byte bound is under [`MIN_MAX_BYTES`](crate::MIN_MAX_BYTES),
    /// so no piece could be both within the bound and outside a scalar.
    InvalidMaxBytes {
        /// The declared bound.
        max_bytes: usize,
        /// The least bound the split law can honour.
        least: usize,
    },
    /// The profile's overlap is at or over its byte bound, so a
    /// continuation could reach back over the whole piece it continues
    /// and advance by as little as a byte.
    InvalidOverlap {
        /// The declared overlap.
        overlap: usize,
        /// The declared bound it must sit strictly under.
        max_bytes: usize,
    },
    /// The profile's name carries a control character, which the
    /// line-oriented stage description cannot frame.
    InvalidProfileName {
        /// The offending character.
        found: char,
    },
    /// The profile declares a canon base that is an IRI reference
    /// carrying no scheme, so nothing concatenated onto it could carry
    /// one either.
    InvalidCanonBase {
        /// The base as declared.
        base: String,
    },
    /// The profile declares a canon base that is no IRI reference at
    /// all — the empty base among them, which is no string to mint
    /// under — so there is nothing to concatenate an anchor onto.
    MalformedCanonBase {
        /// The base as declared.
        base: String,
        /// What the workspace IRI law found, carried rather than
        /// restated.
        cause: IriError,
    },
    /// A concordance anchor does not mint a lawful IRI under the
    /// declared canon base: `base ++ anchor` is not an absolute IRI, or
    /// it is one that lies outside the base. The verse range is the
    /// row's, so a long concordance names the row to fix rather than
    /// leaving it to be searched for.
    InvalidAnchor {
        /// The anchor as the concordance wrote it, between backticks.
        anchor: String,
        /// The first and last verse of the row that named it.
        verses: (u64, u64),
    },
    /// A unit does not answer for the bytes it was checked against: the
    /// span lies outside them, or it cuts a scalar in half, or the bytes
    /// at it no longer digest to the digest the unit carries.
    ///
    /// The finding is the kernel's own, carried through rather than
    /// restated: [`verify_unit`](crate::verify_unit) applies
    /// [`TextChunkTarget::verify_document`](purrdf_core::embedding::TextChunkTarget::verify_document),
    /// which is the same law a PURREMB consumer applies to the same
    /// target, so the two refuse the same bytes for the same reason.
    TamperedUnit {
        /// The unit's byte span, as the model states it.
        span: (u64, u64),
        /// What the kernel's verification law found.
        cause: EmbeddingError,
    },
    /// The assembled model's spans would not decode back to the very
    /// bytes they cover: the codec's own emission would be unlawful
    /// under the decode law, and the document is refused rather than
    /// sliced into a graph nothing can rebuild.
    ///
    /// Every admitted document is held to this before a model exists
    /// to render — in every build, not behind a debug assertion —
    /// because a cover defect discovered by a *consumer's* decode is a
    /// silent drop that already shipped. Seeing this refusal means a
    /// defect in this crate's slicing law, not in the document; it is
    /// stated as a refusal rather than a panic so it fails loudly, by
    /// name, and carries the cover law's own finding.
    CoverDefect {
        /// What the kernel's cover law found.
        cause: purrdf_core::cover::ReconstructError,
    },
}

impl std::fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "source bytes are not UTF-8 after byte {valid_up_to}")
            }
            Self::EmptySourceId => {
                write!(f, "source id is empty: <> names no document")
            }
            Self::InvalidSourceId { found } => {
                write!(
                    f,
                    "source id cannot be an IRI reference: contains {found:?}"
                )
            }
            Self::RelativeSourceId { id } => {
                write!(f, "source id is not an absolute IRI: {id:?} has no scheme")
            }
            Self::MalformedSourceId { id, cause } => {
                write!(f, "source id {id:?} is not an IRI at all: {cause}")
            }
            Self::InvalidVocabulary { field, iri } => {
                write!(
                    f,
                    "vocabulary field {field} is not an absolute IRI: {iri:?}"
                )
            }
            Self::MalformedVocabulary { field, iri, cause } => {
                write!(
                    f,
                    "vocabulary field {field} {iri:?} is not an IRI at all: {cause}"
                )
            }
            Self::InvalidMaxBytes { max_bytes, least } => {
                write!(
                    f,
                    "profile max_bytes {max_bytes} is under {least}, the widest UTF-8 scalar"
                )
            }
            Self::InvalidOverlap { overlap, max_bytes } => {
                write!(
                    f,
                    "profile overlap {overlap} is not under max_bytes {max_bytes}"
                )
            }
            Self::InvalidProfileName { found } => {
                write!(
                    f,
                    "profile name cannot hold a control character: contains {found:?}"
                )
            }
            Self::InvalidCanonBase { base } => {
                write!(f, "canon base is not an absolute IRI: {base:?}")
            }
            Self::MalformedCanonBase { base, cause } => {
                write!(f, "canon base {base:?} is not an IRI at all: {cause}")
            }
            Self::InvalidAnchor { anchor, verses } => {
                write!(
                    f,
                    "concordance anchor {anchor:?} for verses {}\u{2013}{} mints no lawful IRI under the canon base",
                    verses.0, verses.1
                )
            }
            Self::CoverDefect { cause } => {
                write!(
                    f,
                    "the emitted spans would not decode back to the source: {cause}"
                )
            }
            Self::TamperedUnit { span, cause } => {
                write!(
                    f,
                    "unit at bytes {}\u{2013}{} does not answer for these bytes: {cause}",
                    span.0, span.1
                )
            }
        }
    }
}

impl std::error::Error for MarkdownError {}
