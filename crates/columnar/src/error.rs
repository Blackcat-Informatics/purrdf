// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed failures from the narrow columnar codec.

use std::fmt;

/// A fail-closed Parquet encoding, decoding, or RDF reconstruction error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnarError {
    /// Logical capacity or physical storage refused RDF reconstruction.
    Storage(purrdf_lex::allocation::StorageError),
    /// Original content-store integrity refusal during projection.
    ContentStore(purrdf_core::ContentStoreError),
    /// Original blob integrity refusal at its decoded row.
    BlobContent {
        /// Zero-based blob table row.
        row: usize,
        /// The original content-store cause, including both digest identities.
        cause: purrdf_core::ContentStoreError,
    },
    /// Original reconstruction diagnostic, retaining its native attachments.
    Reconstruction(purrdf_core::RdfDiagnostic),
    /// Input ended before a required byte range was available.
    Truncated {
        /// Structure being decoded.
        context: &'static str,
        /// Exclusive byte position or length required.
        needed: usize,
        /// Bytes available in the enclosing buffer.
        available: usize,
    },
    /// Bytes were present but violated the fixed schema or encoding contract.
    Malformed {
        /// Structure being decoded or constructed.
        context: &'static str,
        /// Stable diagnostic detail.
        detail: String,
    },
    /// A valid Parquet construct falls outside PurRDF's deliberately narrow profile.
    Unsupported {
        /// Unsupported field or construct.
        context: &'static str,
        /// Observed numeric discriminator.
        value: i64,
    },
    /// A count or byte length exceeds a representable or safety bound.
    LimitExceeded {
        /// Bounded structure.
        context: &'static str,
        /// Observed value.
        value: u64,
        /// Maximum accepted value.
        maximum: u64,
    },
}

impl purrdf_hash::error::StructuredError for ColumnarError {
    purrdf_hash::error_visit! {
        Storage(cause) => "columnar-storage" { "cause" => Cause(cause) },
        ContentStore(cause) => "columnar-content-store" { "cause" => Cause(cause) },
        BlobContent { row, cause } => "columnar-blob-content" {
            "row" => Unsigned(*row as u128), "cause" => Cause(cause),
        },
        Reconstruction(cause) => "columnar-reconstruction" { "cause" => Cause(cause) },
        Truncated { context, needed, available } => "columnar-truncated" {
            "context" => Code(context), "needed" => ByteCount(*needed as u128), "available" => ByteCount(*available as u128),
        },
        Malformed { context, detail } => "columnar-malformed" { "context" => Code(context), "detail" => Text(detail) },
        Unsupported { context, value } => "columnar-unsupported" { "context" => Code(context), "value" => Signed(i128::from(*value)) },
        LimitExceeded { context, value, maximum } => "columnar-limit-exceeded" {
            "context" => Code(context), "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(u128::from(*maximum)),
        },
    }
}

impl ColumnarError {
    pub(crate) fn malformed(context: &'static str, detail: impl Into<String>) -> Self {
        Self::Malformed {
            context,
            detail: detail.into(),
        }
    }

    pub(crate) fn truncated(context: &'static str, needed: usize, available: usize) -> Self {
        Self::Truncated {
            context,
            needed,
            available,
        }
    }

    pub(crate) fn limit(context: &'static str, value: usize, maximum: usize) -> Self {
        Self::LimitExceeded {
            context,
            value: value as u64,
            maximum: maximum as u64,
        }
    }
}

impl fmt::Display for ColumnarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "columnar storage: {error}"),
            Self::ContentStore(cause) => {
                write!(f, "columnar content store is malformed: {cause}")
            }
            Self::BlobContent { row, cause } => {
                write!(f, "columnar blobs.bytes is malformed: row {row}: {cause}")
            }
            Self::Reconstruction(cause) => {
                if let Some(storage) = cause.storage_error() {
                    write!(f, "columnar storage: {storage}")
                } else {
                    write!(f, "columnar RDF reconstruction is malformed: {cause}")
                }
            }
            Self::Truncated {
                context,
                needed,
                available,
            } => write!(
                f,
                "columnar {context} is truncated: needs {needed} bytes, has {available}"
            ),
            Self::Malformed { context, detail } => {
                write!(f, "columnar {context} is malformed: {detail}")
            }
            Self::Unsupported { context, value } => {
                write!(f, "columnar {context} value {value} is unsupported")
            }
            Self::LimitExceeded {
                context,
                value,
                maximum,
            } => write!(
                f,
                "columnar {context} value {value} exceeds maximum {maximum}"
            ),
        }
    }
}

purrdf_lex::error_source!(ColumnarError => {
    Self::Storage(cause) => cause,
    Self::ContentStore(cause) => cause,
    Self::BlobContent { cause, .. } => cause,
    Self::Reconstruction(cause) => cause,
});

purrdf_lex::variant_from!(ColumnarError { Storage(purrdf_lex::allocation::StorageError) });

/// An RDF projection or encoding refusal, retaining its original source cause.
///
/// The source error remains owned and typed until the caller selects its human
/// presentation. Inspection borrows it without rendering or cloning it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnarWriteError<E> {
    /// The native fixed-schema codec refused the input or output.
    Codec(ColumnarError),
    /// The original operational read session refused projection or term access.
    SourceRead(E),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for ColumnarWriteError<E>
{
    purrdf_hash::error_visit! {
        Codec(cause) => "columnar-write-codec" { "cause" => Cause(cause) },
        SourceRead(cause) => "columnar-source-read" { "cause" => Cause(cause) },
    }
}

impl<E: fmt::Display> fmt::Display for ColumnarWriteError<E> {
    /// Render the original cause only when the caller requests human text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Codec(cause) => cause.fmt(f),
            Self::SourceRead(cause) => write!(f, "columnar source read failed: {cause}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ColumnarWriteError<E> {
    /// Borrow the original cause directly, preserving standard downcasts.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(cause) => Some(cause),
            Self::SourceRead(cause) => Some(cause),
        }
    }
}

purrdf_lex::variant_from!(impl<E> ColumnarWriteError<E> { Codec(ColumnarError) });
