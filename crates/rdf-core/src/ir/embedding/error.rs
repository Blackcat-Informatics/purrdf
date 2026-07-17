// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structured failures for PURREMB encoding, validation, and attachment.

use core::fmt;

/// A digest or typed identity that failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DigestKind {
    /// Plain per-section SHA-256.
    Section,
    /// Whole-artifact integrity root.
    ArtifactRoot,
    /// Exact source artifact SHA-256.
    SourceExact,
    /// Independently certified RDFC digest.
    CertifiedRdf,
    /// Canonical contract or typed identity.
    Contract,
    /// Target identity.
    Target,
    /// Target-set identity.
    TargetSet,
    /// Stored matrix identity.
    Matrix,
    /// Effective Matryoshka projection identity.
    Projection,
    /// External-artifact binding identity.
    ExternalBinding,
    /// Derived-index identity or payload.
    Index,
}

/// A fail-closed PURREMB format, identity, or verification error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmbeddingError {
    /// The input is shorter than the declared fixed structure or span.
    Truncated,
    /// The file magic is not `PURREMB1`.
    BadMagic,
    /// The trailer magic is not `PURREND1`.
    BadTrailerMagic,
    /// The format version is not supported.
    UnsupportedVersion(u32),
    /// A fixed-size field contains an unsupported code.
    UnsupportedCode {
        /// Field being interpreted.
        field: &'static str,
        /// Unsupported numeric value.
        value: u32,
    },
    /// A declared count exceeds the v1 bound.
    CountLimit {
        /// Counted structure.
        field: &'static str,
        /// Claimed count.
        value: u64,
    },
    /// Checked integer arithmetic or host conversion failed.
    ArithmeticOverflow(&'static str),
    /// A byte span is out of bounds or otherwise invalid.
    InvalidSpan {
        /// Structure containing the span.
        context: &'static str,
        /// Start offset.
        offset: u64,
        /// Span length.
        length: u64,
    },
    /// A required alignment is not satisfied.
    Misaligned {
        /// Structure containing the offset.
        context: &'static str,
        /// Observed offset.
        offset: u64,
        /// Required power-of-two alignment.
        alignment: u64,
    },
    /// Canonical padding contains a nonzero byte or has a nonminimal extent.
    InvalidPadding {
        /// Offset of the first invalid padding byte or gap.
        offset: u64,
    },
    /// Records that must be strictly increasing are unordered or duplicated.
    NonCanonicalOrder(&'static str),
    /// A required record or field is absent.
    Missing(&'static str),
    /// A record or field occurs more than once.
    Duplicate(&'static str),
    /// Reserved bytes or unknown flag bits are nonzero.
    ReservedNonzero(&'static str),
    /// A canonical TLV block is malformed.
    MalformedTlv(&'static str),
    /// UTF-8 bytes are malformed or violate a field-specific rule.
    InvalidUtf8(&'static str),
    /// A typed ID, content digest, or integrity digest does not match.
    DigestMismatch {
        /// Digest domain that failed.
        kind: DigestKind,
        /// Expected bytes carried by the artifact.
        expected: [u8; 32],
        /// Independently computed bytes.
        actual: [u8; 32],
    },
    /// An exact source artifact has a different byte length.
    SourceLengthMismatch {
        /// Length recorded by PURREMB.
        expected: u64,
        /// Supplied source length.
        actual: u64,
    },
    /// An exact external artifact has a different byte length.
    ExternalLengthMismatch {
        /// Length recorded by the binding.
        expected: u64,
        /// Supplied artifact length.
        actual: u64,
    },
    /// The attached exact source bytes are not a structurally valid pack.
    InvalidSourcePack(String),
    /// A certified external binding does not contain a structurally valid pack.
    InvalidExternalPack(String),
    /// A cross-reference names no compatible record.
    MissingReference(&'static str),
    /// Two vector spaces are not semantically comparable.
    IncompatibleVectorSpaces,
    /// A requested Matryoshka prefix is not declared or stored.
    UnavailablePrefix(u32),
    /// A scalar is NaN or infinite.
    NonFiniteScalar {
        /// Matrix row.
        row: u64,
        /// Matrix column.
        column: u32,
    },
    /// Deterministic L2 normalization encountered an invalid zero norm.
    ZeroNorm {
        /// Matrix row.
        row: u64,
        /// Effective prefix dimension.
        dimension: u32,
    },
    /// A document or chunk content digest, length, or scalar boundary failed.
    ContentMismatch(&'static str),
    /// A source-local ordinal resolves to another canonical target.
    OrdinalMismatch {
        /// Target kind carrying the ordinal.
        target_kind: u32,
        /// Source-local ordinal.
        ordinal: u64,
    },
    /// A resident validation certificate was applied to another byte slice.
    CertificateMismatch,
    /// The file is structurally inconsistent.
    Malformed(&'static str),
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated PURREMB artifact"),
            Self::BadMagic => f.write_str("invalid PURREMB file magic"),
            Self::BadTrailerMagic => f.write_str("invalid PURREMB trailer magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported PURREMB version {version}")
            }
            Self::UnsupportedCode { field, value } => {
                write!(f, "unsupported {field} code {value}")
            }
            Self::CountLimit { field, value } => {
                write!(f, "{field} count {value} exceeds the v1 limit")
            }
            Self::ArithmeticOverflow(context) => {
                write!(f, "integer arithmetic overflow in {context}")
            }
            Self::InvalidSpan {
                context,
                offset,
                length,
            } => write!(f, "invalid {context} span at {offset} with length {length}"),
            Self::Misaligned {
                context,
                offset,
                alignment,
            } => write!(f, "{context} offset {offset} is not aligned to {alignment}"),
            Self::InvalidPadding { offset } => {
                write!(f, "noncanonical padding at byte {offset}")
            }
            Self::NonCanonicalOrder(context) => {
                write!(f, "noncanonical or duplicate {context} ordering")
            }
            Self::Missing(field) => write!(f, "missing required {field}"),
            Self::Duplicate(field) => write!(f, "duplicate {field}"),
            Self::ReservedNonzero(field) => write!(f, "nonzero reserved {field}"),
            Self::MalformedTlv(context) => write!(f, "malformed TLV block: {context}"),
            Self::InvalidUtf8(context) => write!(f, "invalid UTF-8 in {context}"),
            Self::DigestMismatch { kind, .. } => write!(f, "{kind:?} digest mismatch"),
            Self::SourceLengthMismatch { expected, actual } => write!(
                f,
                "source length mismatch: expected {expected} bytes, received {actual}"
            ),
            Self::ExternalLengthMismatch { expected, actual } => write!(
                f,
                "external artifact length mismatch: expected {expected} bytes, received {actual}"
            ),
            Self::InvalidSourcePack(reason) => {
                write!(f, "invalid attached source pack: {reason}")
            }
            Self::InvalidExternalPack(reason) => {
                write!(f, "invalid certified external pack: {reason}")
            }
            Self::MissingReference(context) => write!(f, "missing {context} reference"),
            Self::IncompatibleVectorSpaces => f.write_str("incompatible vector spaces"),
            Self::UnavailablePrefix(dimension) => {
                write!(f, "Matryoshka prefix dimension {dimension} is unavailable")
            }
            Self::NonFiniteScalar { row, column } => {
                write!(f, "non-finite matrix scalar at row {row}, column {column}")
            }
            Self::ZeroNorm { row, dimension } => {
                write!(f, "zero norm at row {row} for prefix dimension {dimension}")
            }
            Self::ContentMismatch(context) => write!(f, "content mismatch: {context}"),
            Self::OrdinalMismatch {
                target_kind,
                ordinal,
            } => write!(
                f,
                "source ordinal {ordinal} disagrees with target kind {target_kind}"
            ),
            Self::CertificateMismatch => {
                f.write_str("resident certificate does not match this byte slice")
            }
            Self::Malformed(context) => write!(f, "malformed PURREMB artifact: {context}"),
        }
    }
}

impl std::error::Error for EmbeddingError {}

/// A PURREMB streaming-write failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum EmbeddingWriteError {
    /// Canonical input or numerical validation failed.
    Format(EmbeddingError),
    /// The output writer failed.
    Io(std::io::Error),
}

impl fmt::Display for EmbeddingWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(f),
            Self::Io(error) => write!(f, "PURREMB output I/O failed: {error}"),
        }
    }
}

impl std::error::Error for EmbeddingWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<EmbeddingError> for EmbeddingWriteError {
    fn from(error: EmbeddingError) -> Self {
        Self::Format(error)
    }
}

impl From<std::io::Error> for EmbeddingWriteError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
