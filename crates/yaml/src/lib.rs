// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-preserving YAML streams under an explicit profile and vocabulary.
//! Presentation metadata does not execute tags, merge keys or YAML-LD semantics.

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod decode;
mod model;
mod profile;
mod project;

pub use decode::decode_stream;
pub use model::{Document, SourceDocument, analyze};
pub use profile::{Bounds, Profile, STANDARD_NAMESPACE, Vocabulary};
pub use project::{encode, project};
pub use purrdf_lex::yaml as syntax;

/// Original typed source, storage, cover, profile or metadata refusal.
#[derive(Debug)]
#[non_exhaustive]
pub enum YamlError {
    /// The shared lexical parser's positioned error.
    Syntax(syntax::Error),
    /// Physical storage or admission refusal.
    Storage(purrdf_lex::allocation::StorageError),
    /// The shared cover law's original refusal.
    Cover(purrdf_core::cover::ReconstructError),
    /// Absolute-IRI parsing or physical IRI storage refusal.
    Iri(purrdf_core::IriReadError),
    /// The native RDF builder's typed refusal.
    Rdf(purrdf_core::NativeBuildError),
    /// Invalid UTF-8 source at its first unrecognized byte.
    Utf8 {
        /// Original byte offset.
        at: usize,
    },
    /// Invalid explicit configuration.
    Profile(&'static str),
    /// Missing, conflicting, wrongly typed or forged selected metadata.
    Metadata(&'static str),
    /// A caller resource bound was exhausted before further retention.
    Limit {
        /// Resource admitted.
        resource: &'static str,
        /// Caller ceiling.
        limit: usize,
    },
}
impl core::fmt::Display for YamlError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Syntax(cause) => cause.fmt(out),
            Self::Storage(cause) => cause.fmt(out),
            Self::Cover(cause) => cause.fmt(out),
            Self::Iri(cause) => cause.fmt(out),
            Self::Rdf(cause) => cause.fmt(out),
            Self::Utf8 { at } => write!(out, "YAML source is not UTF-8 at byte {at}"),
            Self::Profile(reason) => write!(out, "YAML source profile: {reason}"),
            Self::Metadata(reason) => write!(out, "YAML source metadata: {reason}"),
            Self::Limit { resource, limit } => {
                write!(out, "YAML source {resource} exceeds {limit}")
            }
        }
    }
}
purrdf_hash::error_sources!(YamlError, Syntax, Storage, Cover, Iri, Rdf);
purrdf_lex::variant_from!(YamlError {
    Syntax(syntax::Error), Storage(purrdf_lex::allocation::StorageError),
    Cover(purrdf_core::cover::ReconstructError), Iri(purrdf_core::IriReadError),
    Rdf(purrdf_core::NativeBuildError)
});
impl purrdf_hash::error::StructuredError for YamlError {
    purrdf_hash::error_visit! {
        Syntax(cause) => "yaml-source-syntax" { "cause" => Cause(cause) },
        Storage(cause) => "yaml-source-storage" { "cause" => Cause(cause) },
        Cover(cause) => "yaml-source-cover" { "cause" => Cause(cause) },
        Iri(cause) => "yaml-source-iri" { "cause" => Cause(cause) },
        Rdf(cause) => "yaml-source-rdf" { "cause" => Cause(cause) },
        Utf8 { at } => "yaml-source-utf8" { "at" => ByteCount(*at as u128) },
        Profile(reason) => "yaml-source-profile" { "reason" => Text(reason) },
        Metadata(reason) => "yaml-source-metadata" { "reason" => Text(reason) },
        Limit { resource, limit } => "yaml-source-limit" { "resource" => Code(resource), "limit" => Unsigned(*limit as u128) },
    }
}

pub(crate) fn admit_count(
    current: usize,
    maximum: usize,
    resource: &'static str,
) -> Result<(), YamlError> {
    if current > maximum {
        return Err(YamlError::Limit {
            resource,
            limit: maximum,
        });
    }
    Ok(())
}
