// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native failures while certifying or verifying an attachment.

use purrdf_columnar::ColumnarError;
use purrdf_core::{CanonError, EmbeddingError, EmbeddingSourceFormat};
use std::fmt;
use std::sync::Arc;

/// An attachment refusal with its original native owner and structured fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SourceAttachmentError {
    /// The supplied carrier differs from the declared source format.
    FormatMismatch {
        /// The carrier named by the embedding.
        expected: EmbeddingSourceFormat,
        /// The actual supplied carrier.
        actual: EmbeddingSourceFormat,
    },
    /// Shared embedding laws or native pack certification refused the attachment.
    Embedding(EmbeddingError),
    /// The strict Columnar reader refused the original files.
    Columnar(Arc<ColumnarError>),
    /// Independent native RDF canonicalization refused the decoded dataset.
    Canonical(Arc<CanonError>),
}

purrdf_lex::variant_from!(SourceAttachmentError {
    Embedding(EmbeddingError),
});
purrdf_lex::variant_from!(SourceAttachmentError { Columnar(ColumnarError) via Arc::new });
purrdf_lex::variant_from!(SourceAttachmentError { Canonical(CanonError) via Arc::new });

impl purrdf_hash::error::StructuredError for SourceAttachmentError {
    purrdf_hash::error_visit! {
        FormatMismatch { expected, actual } => "purremb-source-format-mismatch" {
            "expected" => Unsigned(u128::from(expected.code())), "actual" => Unsigned(u128::from(actual.code())),
        },
        Embedding(cause) => "purremb-attachment-embedding" { "cause" => Cause(cause) },
        Columnar(cause) => "purremb-attachment-columnar" { "cause" => Cause(&**cause) },
        Canonical(cause) => "purremb-attachment-canonical" { "cause" => Cause(&**cause) },
    }
}

impl fmt::Display for SourceAttachmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormatMismatch { expected, actual } => write!(
                f,
                "embedding requires source format {}, received {}",
                expected.code(),
                actual.code()
            ),
            Self::Embedding(cause) => cause.fmt(f),
            Self::Columnar(cause) => cause.fmt(f),
            Self::Canonical(cause) => cause.fmt(f),
        }
    }
}

purrdf_lex::error_source!(SourceAttachmentError => {
    Self::Embedding(cause) => cause,
    Self::Columnar(cause) => cause.as_ref(),
    Self::Canonical(cause) => cause.as_ref(),
});
