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

/// A failure raised while building a profile or fusing ranked streams.
///
/// Every variant is a refusal, never a repaired or partial answer: an invalid
/// profile is rejected where it is supplied, a malformed stream is returned to
/// its producer, and an arithmetic intermediate that does not fit is reported
/// rather than wrapped.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FusionError {
    /// A fixed-point addition left the representable range. A wrapped sum would
    /// be a wrong order presented as a right one, so it is refused.
    #[error("fusion overflowed the fixed-point range")]
    Overflow,

    /// A ranked stream violated the input protocol.
    #[error("ranked-stream protocol violation: {0}")]
    Protocol(Box<crate::ranked_stream::ProtocolError>),

    /// The reciprocal-rank smoothing constant `K` was zero. `K >= 1` keeps a
    /// rank-one item's contribution finite and below one.
    #[error("fusion profile K must be at least 1, got {k}")]
    InvalidK {
        /// The rejected value.
        k: u32,
    },

    /// A contribution was asked for at rank zero. Ranks are 1-based.
    #[error("rank must be at least 1, got {rank}")]
    InvalidRank {
        /// The rejected rank.
        rank: u64,
    },

    /// A fusion profile carried no stratum weights.
    #[error("fusion profile must declare at least one stratum weight")]
    EmptyWeights,

    /// A stratum weight was not strictly positive.
    #[error("stratum {stratum} has non-positive weight {weight:?}")]
    NonPositiveWeight {
        /// The stratum whose weight was rejected, as its canonical IRI text.
        stratum: String,
        /// The rejected weight.
        weight: purrdf_text::Fixed,
    },

    /// The declared maximum contribution count was zero.
    #[error("fusion profile maximum contributions must be at least 1, got {max}")]
    InvalidMaxContributions {
        /// The rejected value.
        max: u32,
    },

    /// A stream emitted under a stratum the profile declares no weight for.
    #[error("fusion profile declares no weight for stratum {stratum}")]
    UnknownStratum {
        /// The undeclared stratum, as its canonical IRI text.
        stratum: String,
    },

    /// Two streams were tagged with the same stratum.
    #[error("two streams are tagged with stratum {stratum}")]
    DuplicateStratum {
        /// The repeated stratum, as its canonical IRI text.
        stratum: String,
    },

    /// A fusion profile's canonical bytes could not be decoded.
    #[error("malformed fusion profile: {0}")]
    MalformedProfile(String),
}

impl From<crate::ranked_stream::ProtocolError> for FusionError {
    /// Wrap a producer's protocol violation without growing `FusionError` to the
    /// producer error's own size.
    fn from(error: crate::ranked_stream::ProtocolError) -> Self {
        Self::Protocol(Box::new(error))
    }
}
