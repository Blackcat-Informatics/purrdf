// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed, structured failures of the retrieval-plan layer.
//!
//! Every failure carries the dimension that failed, so admission and decode
//! paths can name an exact reason rather than report a generic parse problem.
//! A version mismatch is its own variant precisely because a plan written by a
//! different build must be refused loudly, never silently reinterpreted under
//! the current layout.

/// A failure to validate an IRI or to encode/decode a canonical [`Plan`].
///
/// [`Plan`]: crate::Plan
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlanError {
    /// A plan's canonical encoding carries a version this build does not write.
    ///
    /// This is a refusal, not a warning: a plan whose layout a newer or older
    /// build defined cannot be decoded under this build's field order.
    #[error("unsupported plan version {found}; this build writes version {expected}")]
    VersionMismatch {
        /// The version found in the encoded plan.
        found: u16,
        /// The version this build writes and understands.
        expected: u16,
    },

    /// The canonical encoding ended before a complete value was read.
    #[error("plan canonical encoding is truncated at byte {offset}")]
    Truncated {
        /// The byte offset at which more data was needed.
        offset: usize,
    },

    /// A canonical discriminator byte named no known variant.
    #[error("plan canonical encoding carries invalid tag {tag} for {what}")]
    InvalidTag {
        /// The field being decoded when the tag was read.
        what: &'static str,
        /// The tag byte that named no variant.
        tag: u8,
    },

    /// A canonical variable-length field was not valid UTF-8.
    #[error("plan canonical encoding carries invalid UTF-8 in {what}")]
    InvalidUtf8 {
        /// The field being decoded.
        what: &'static str,
    },

    /// A canonical field did not parse as an IRI.
    #[error("invalid IRI {text:?}: {source}")]
    InvalidIri {
        /// The offending IRI text.
        text: String,
        /// The kernel parser's refusal.
        #[source]
        source: purrdf_core::IriError,
    },

    /// The canonical encoding held bytes after the last field.
    #[error("plan canonical encoding has {extra} trailing byte(s)")]
    TrailingBytes {
        /// The number of unconsumed bytes.
        extra: usize,
    },
}
