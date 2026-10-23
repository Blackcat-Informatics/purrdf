// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::{IriError, RdfDiagnostic, cover::ReconstructError};

/// Typed refusal at the source, profile, RDF metadata or byte-cover boundary.
#[derive(Debug)]
#[non_exhaustive]
pub enum JsonError {
    /// The original reader progress observer refused work at this byte offset.
    Interrupted {
        /// Byte where the original progress observer stopped the read.
        at: usize,
    },
    /// The shared reader refused physical allocation or admission.
    Storage(purrdf_lex::allocation::StorageError),
    /// A source or vocabulary IRI is not absolute and well formed.
    Iri(IriError),
    /// The declared profile has an invalid identifier, namespace or bound.
    Profile(&'static str),
    /// Source bytes are not UTF-8.
    InvalidUtf8 {
        /// Prefix length before the first invalid sequence.
        valid_up_to: usize,
    },
    /// JSON grammar refusal at a byte position.
    Syntax {
        /// First byte where the grammar fails.
        at: usize,
        /// The expected grammar element.
        expected: &'static str,
    },
    /// A resource bound in the profile was exceeded.
    Limit {
        /// The bounded resource's name.
        resource: &'static str,
        /// Declared maximum.
        limit: u64,
    },
    /// A JSON member name escape cannot form a Unicode scalar path component.
    LoneSurrogate {
        /// Start of the string's lexical span.
        at: usize,
    },
    /// Trailing bytes after the complete JSON value and whitespace.
    Trailing {
        /// First trailing byte.
        at: usize,
    },
    /// A selected RDF node has missing, repeated, mistyped or inconsistent metadata.
    Metadata {
        /// Node whose metadata failed validation.
        subject: String,
        /// Predicate or structural rule being checked.
        field: &'static str,
    },
    /// A model or encoded document belongs to a different profile.
    ProfileMismatch,
    /// The shared IR refused a projected term or statement.
    Rdf(RdfDiagnostic),
    /// The kernel's cover validator refused reconstruction.
    Cover(ReconstructError),
}

impl purrdf_hash::error::StructuredError for JsonError {
    purrdf_hash::error_visit! {
        Interrupted { at } => "json-cover-interrupted" { "at" => ByteCount(*at as u128) },
        Storage(cause) => "json-cover-storage" { "cause" => Cause(cause) },
        Iri(cause) => "json-cover-iri" { "cause" => Cause(cause) },
        Profile(reason) => "json-cover-profile" { "reason" => Text(reason) },
        InvalidUtf8 { valid_up_to } => "json-cover-invalid-utf8" { "valid_up_to" => ByteCount(*valid_up_to as u128) },
        Syntax { at, expected } => "json-cover-syntax" { "at" => ByteCount(*at as u128), "expected" => Text(expected) },
        Limit { resource, limit } => "json-cover-limit" { "resource" => Code(resource), "limit" => Unsigned(u128::from(*limit)) },
        LoneSurrogate { at } => "json-cover-lone-surrogate" { "at" => ByteCount(*at as u128) },
        Trailing { at } => "json-cover-trailing" { "at" => ByteCount(*at as u128) },
        Metadata { subject, field } => "json-cover-metadata" { "subject" => Text(subject), "field" => Code(field) },
        ProfileMismatch => "json-cover-profile-mismatch" {},
        Rdf(cause) => "json-cover-rdf" { "cause" => Cause(cause) },
        Cover(cause) => "json-cover-reconstruction" { "cause" => Cause(cause) },
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interrupted { at } => write!(f, "JSON input progress interrupted at byte {at}"),
            Self::Storage(error) => write!(f, "JSON storage: {error}"),
            Self::Iri(error) => write!(f, "invalid JSON codec IRI: {error}"),
            Self::Profile(message) => write!(f, "invalid JSON profile: {message}"),
            Self::InvalidUtf8 { valid_up_to } => write!(f, "invalid UTF-8 at byte {valid_up_to}"),
            Self::Syntax { at, expected } => write!(f, "JSON byte {at}: expected {expected}"),
            Self::Limit { resource, limit } => {
                write!(f, "JSON {resource} exceeds profile limit {limit}")
            }
            Self::LoneSurrogate { at } => {
                write!(f, "JSON string at byte {at} contains an unpaired surrogate")
            }
            Self::Trailing { at } => write!(f, "trailing JSON data at byte {at}"),
            Self::Metadata { subject, field } => {
                write!(f, "JSON metadata <{subject}> violates {field}")
            }
            Self::ProfileMismatch => f.write_str("JSON profile identity does not match"),
            Self::Rdf(error) => write!(f, "JSON RDF projection: {error}"),
            Self::Cover(error) => write!(f, "JSON cover: {error}"),
        }
    }
}

impl std::error::Error for JsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Iri(error) => Some(error),
            Self::Rdf(error) => Some(error),
            Self::Cover(error) => Some(error),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(JsonError { Storage(purrdf_lex::allocation::StorageError) });

pub(crate) fn metadata(subject: &str, field: &'static str) -> JsonError {
    JsonError::Metadata {
        subject: subject.to_owned(),
        field,
    }
}
