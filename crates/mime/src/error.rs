// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::{
    IriError, RdfDiagnostic,
    cover::{EmitError, ReconstructError},
};

/// A declared-resource, profile, cover, RDF or deterministic decoding refusal.
/// Malformed message structure is ordinarily represented as a typed occurrence.
#[derive(Debug)]
#[non_exhaustive]
pub enum MimeError {
    /// Invalid absolute source or vocabulary IRI.
    Iri(IriError),
    /// Invalid explicitly selected profile.
    Profile(&'static str),
    /// The caller's declared maximum was exceeded.
    Limit {
        /// Resource whose observed use exceeded the explicit maximum.
        resource: &'static str,
        /// Original caller-declared maximum.
        limit: u64,
    },
    /// A projection was requested under a different profile.
    ProfileMismatch,
    /// The shared emitter refused the producer's partition.
    Emit(EmitError),
    /// The one cover decoder refused the asserted bytes.
    Cover(ReconstructError),
    /// The IR refused a term or statement.
    Rdf(RdfDiagnostic),
    /// Selected RDF document has inconsistent, missing or invented metadata.
    Metadata {
        /// Selected document or occurrence IRI.
        subject: String,
        /// Metadata field or closed representation invariant that failed.
        field: &'static str,
    },
    /// The requested part occurrence does not exist.
    UnknownPart(usize),
    /// Exact transfer decoding is undefined or malformed; no partial payload escapes.
    Transfer {
        /// Original part occurrence index.
        part: usize,
        /// Absolute source byte where decoding failed.
        at: usize,
        /// Stable explanation of the transfer-syntax refusal.
        reason: &'static str,
    },
}

impl std::fmt::Display for MimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iri(error) => write!(f, "MIME IRI: {error}"),
            Self::Profile(reason) => write!(f, "MIME profile: {reason}"),
            Self::Limit { resource, limit } => {
                write!(f, "MIME {resource} exceeds caller limit {limit}")
            }
            Self::ProfileMismatch => f.write_str("MIME profile identity differs"),
            Self::Emit(error) => write!(f, "MIME emission: {error}"),
            Self::Cover(error) => write!(f, "MIME cover: {error}"),
            Self::Rdf(error) => write!(f, "MIME RDF: {error}"),
            Self::Metadata { subject, field } => write!(f, "MIME node {subject}: invalid {field}"),
            Self::UnknownPart(part) => write!(f, "MIME part {part} does not exist"),
            Self::Transfer { part, at, reason } => {
                write!(f, "MIME part {part}, byte {at}: {reason}")
            }
        }
    }
}

impl std::error::Error for MimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Iri(error) => Some(error),
            Self::Emit(error) => Some(error),
            Self::Cover(error) => Some(error),
            Self::Rdf(error) => Some(error),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(MimeError {
    Iri(IriError), Emit(EmitError), Cover(ReconstructError), Rdf(RdfDiagnostic)
});

pub(crate) fn metadata(subject: &str, field: &'static str) -> MimeError {
    MimeError::Metadata {
        subject: subject.to_owned(),
        field,
    }
}
