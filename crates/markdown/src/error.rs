// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Why a document could not be sliced.
//!
//! Every refusal the crate can state lives here, in one enum, and every
//! one of them is answered for at the seam — [`analyze`](crate::analyze)
//! — before a claim exists. That is what lets the projection
//! ([`render`](crate::render)) be infallible: a [`Document`](crate::Document)
//! can only be obtained by passing every check this enum names.

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
    /// The source id is writable but not absolute: it carries no
    /// scheme, so it names a document only relative to whatever holds
    /// the claims, and every node minted over it inherits that.
    RelativeSourceId {
        /// The id as declared.
        id: String,
    },
    /// A vocabulary IRI is empty, cannot be written as an IRI
    /// reference, or is not absolute.
    InvalidVocabulary {
        /// The vocabulary field.
        field: &'static str,
        /// The offending value.
        iri: String,
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
    /// The profile declares a canon base that is empty or is not an
    /// absolute IRI, so nothing concatenated onto it could be one
    /// either.
    InvalidCanonBase {
        /// The base as declared.
        base: String,
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
            Self::InvalidVocabulary { field, iri } => {
                write!(
                    f,
                    "vocabulary field {field} is not an absolute IRI: {iri:?}"
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
            Self::InvalidAnchor { anchor, verses } => {
                write!(
                    f,
                    "concordance anchor {anchor:?} for verses {}\u{2013}{} mints no lawful IRI under the canon base",
                    verses.0, verses.1
                )
            }
        }
    }
}

impl std::error::Error for MarkdownError {}
