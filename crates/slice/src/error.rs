// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Error type for the purrdf-slice crate.

mod path;
pub use path::PathRefusal;
mod json;
pub use json::{JsonMemberKind, JsonRefusal};
mod manifest;
pub use manifest::{ManifestRefusal, ParseRefusal};

/// All errors that can arise from slice discovery, manifest parsing, artifact
/// inventory, and content-addressed digest operations.
/// Large native causes stay in cold boxes; storage refusals remain inline and
/// are selected before allocating those reporting payloads.
#[derive(Debug)]
#[non_exhaustive]
pub enum SliceError {
    /// Original absolute-IRI constructor refusal.
    Iri {
        /// Original rejected spelling.
        iri: String,
        /// Original grammar or term-position refusal.
        cause: Box<purrdf_sparql_algebra::ParseError>,
    },
    /// Original canonicalization refusal at its source artifact.
    Canonicalize {
        /// Original filesystem path.
        path: std::path::PathBuf,
        /// Original canonicalization error.
        cause: Box<purrdf_rdf::CanonError>,
    },
    /// Logical capacity or physical storage refused RDF construction.
    Storage(purrdf_lex::allocation::StorageError),
    /// An I/O error reading from the filesystem.
    Io(purrdf_hash::error::IoFailure),
    /// Original RDF parser or dataset refusal with its operation context.
    Rdf {
        /// Operation that was refused.
        context: String,
        /// Complete original diagnostic, including native attachments.
        cause: Box<purrdf_rdf::RdfDiagnostic>,
    },
    /// An RDF parse error (Turtle or other format).
    Parse(ParseRefusal),
    /// The manifest.ttl is structurally invalid (missing required field, etc.).
    InvalidManifest(ManifestRefusal),
    /// A path within a slice violates the safety rules (absolute, `..`, etc.).
    InvalidPath(PathRefusal),
    /// A digest computed at discovery time does not match a stored expectation.
    DigestMismatch {
        /// The stored (expected) digest.
        expected: String,
        /// The digest actually computed.
        actual: String,
    },
    /// A structurally malformed RDF Collection encountered while walking an
    /// `rdf:first`/`rdf:rest` chain.
    RdfList(purrdf_rdf::RdfListError),
    /// A JSON value does not have the shape a slice record decodes from
    /// (`from_json` on [`crate::ManifestView`], [`crate::ArtifactRecord`] and
    /// their parts).
    Json(JsonRefusal),
}

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iri { iri, cause } => write!(f, "RDF parse error: invalid IRI {iri}: {cause}"),
            Self::Canonicalize { path, cause } => write!(
                f,
                "RDF parse error: canonicalize {} for its semantic digest: {cause}",
                path.display()
            ),
            Self::Storage(error) => write!(f, "RDF storage error: {error}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Rdf { context, cause } => write!(f, "RDF parse error: {context}: {cause}"),
            Self::Parse(msg) => write!(f, "RDF parse error: {msg}"),
            Self::InvalidManifest(msg) => write!(f, "invalid manifest: {msg}"),
            Self::InvalidPath(msg) => write!(f, "invalid path: {msg}"),
            Self::DigestMismatch { expected, actual } => {
                write!(f, "digest mismatch: expected {expected}, got {actual}")
            }
            Self::RdfList(e) => write!(f, "malformed RDF collection: {e}"),
            Self::Json(msg) => write!(f, "invalid slice JSON: {msg}"),
        }
    }
}

impl std::error::Error for SliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Iri { cause, .. } => Some(cause.as_ref()),
            Self::Canonicalize { cause, .. } => Some(cause.as_ref()),
            Self::InvalidPath(cause) => Some(cause),
            Self::Json(cause) => Some(cause),
            Self::InvalidManifest(cause) => Some(cause),
            Self::Parse(cause) => Some(cause),
            Self::Storage(error) => Some(error),
            Self::Io(e) => Some(e),
            Self::Rdf { cause, .. } => Some(cause.as_ref()),
            Self::RdfList(e) => Some(e),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(SliceError {
    Storage(purrdf_lex::allocation::StorageError),
    Io(purrdf_hash::error::IoFailure),
});

purrdf_lex::variant_from!(SliceError { Io(std::io::Error) via Into::into });

impl purrdf_hash::error::StructuredError for SliceError {
    purrdf_hash::error_visit! {
        Iri { iri, cause } => "slice-iri" { "iri" => Iri(iri), "cause" => Cause(cause.as_ref()) },
        Canonicalize { path, cause } => "slice-canonicalize" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "cause" => Cause(cause.as_ref()) },
        Storage(cause) => "slice-storage" { "cause" => Cause(cause) },
        Io(cause) => "slice-io" { "cause" => Cause(cause) },
        Rdf { context, cause } => "slice-rdf" { "context" => Text(context), "cause" => Cause(cause.as_ref()) },
        Parse(cause) => "slice-parse" { "cause" => Cause(cause) },
        InvalidManifest(reason) => "slice-invalid-manifest" { "reason" => Cause(reason) },
        InvalidPath(reason) => "slice-invalid-path" { "reason" => Cause(reason) },
        DigestMismatch { expected, actual } => "slice-digest-mismatch" { "expected" => Text(expected), "actual" => Text(actual) },
        RdfList(cause) => "slice-rdf-list" { "cause" => Cause(cause) },
        Json(reason) => "slice-json" { "reason" => Cause(reason) },
    }
}

impl SliceError {
    pub(crate) fn rdf(error: purrdf_rdf::RdfDiagnostic, context: &str) -> Self {
        error.storage_error().map_or_else(
            || Self::Rdf {
                context: context.to_owned(),
                cause: Box::new(error),
            },
            Self::Storage,
        )
    }
}
