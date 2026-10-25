// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! TOML 1.1 authoring as a typed, ordered source model and verified RDF 1.2 cover.
//! Models borrow original UTF-8 bytes; decoded scalar values remain distinct
//! from their spelling. Tables and containers occupy a flat arena, including
//! implicit tables and individual array-of-table elements. Parsing, projection,
//! reconstruction and destruction do not recurse through nested containers.
//! A profile, vocabulary and source selection are explicit.

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod codec;
mod model;
mod parse;
mod profile;

pub use codec::{decode_document, encode, project};
pub use model::{Document, Float, Key, Kind, Node, Scalar, SourceDocument, Statement, TableOrigin};
pub use profile::{Bounds, Profile, SPECIFICATION_VERSION, STANDARD_NAMESPACE, Vocabulary};

use purrdf_core::{BaseIri, ContentDigest};

/// A source location, measured in bytes with one-based line and scalar column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    /// Zero-based byte offset.
    pub byte: usize,
    /// One-based source line.
    pub line: usize,
    /// One-based Unicode scalar column.
    pub column: usize,
}

/// Original typed failure from grammar, resource, profile or RDF reconstruction.
#[derive(Debug)]
#[non_exhaustive]
pub enum TomlError {
    /// Original physical allocation/admission refusal from the shared kernel.
    Storage(purrdf_lex::allocation::StorageError),
    /// Invalid UTF-8 at the first unrecognized byte.
    Utf8 {
        /// Byte offset.
        at: usize,
    },
    /// Grammar or table-definition conflict.
    Syntax {
        /// Original location.
        location: Location,
        /// Required production or violated rule.
        expected: &'static str,
    },
    /// An explicit caller bound was exhausted before more work was retained.
    Limit {
        /// Original location.
        location: Location,
        /// Resource being admitted.
        resource: &'static str,
        /// Caller maximum.
        limit: u64,
    },
    /// Invalid profile or vocabulary configuration.
    Profile(&'static str),
    /// A source/vocabulary identifier is not an absolute IRI.
    Iri(purrdf_core::IriError),
    /// The native RDF builder refused a term or statement.
    Rdf(purrdf_core::RdfDiagnostic),
    /// The shared byte-cover law refused reconstruction.
    Cover(purrdf_core::cover::ReconstructError),
    /// Missing, conflicting or forged document metadata.
    Metadata(&'static str),
}

impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(cause) => cause.fmt(f),
            Self::Utf8 { at } => write!(f, "TOML invalid UTF-8 at byte {at}"),
            Self::Syntax { location, expected } => write!(
                f,
                "TOML {}:{} (byte {}): {expected}",
                location.line, location.column, location.byte
            ),
            Self::Limit {
                location,
                resource,
                limit,
            } => write!(
                f,
                "TOML {}:{} (byte {}): {resource} exceeds {limit}",
                location.line, location.column, location.byte
            ),
            Self::Profile(reason) => write!(f, "TOML profile: {reason}"),
            Self::Iri(cause) => cause.fmt(f),
            Self::Rdf(cause) => cause.fmt(f),
            Self::Cover(cause) => cause.fmt(f),
            Self::Metadata(reason) => write!(f, "TOML document metadata: {reason}"),
        }
    }
}

purrdf_hash::error_sources!(TomlError, Storage, Iri, Rdf, Cover);

impl purrdf_hash::error::StructuredError for TomlError {
    purrdf_hash::error_visit! {
        Storage(cause) => "toml-storage" { "cause" => Cause(cause) },
        Utf8 { at } => "toml-utf8" { "at" => ByteCount(*at as u128) },
        Syntax { location, expected } => "toml-syntax" {
            "at" => ByteCount(location.byte as u128), "line" => Unsigned(location.line as u128),
            "column" => Unsigned(location.column as u128), "expected" => Text(expected)
        },
        Limit { location, resource, limit } => "toml-limit" {
            "at" => ByteCount(location.byte as u128), "line" => Unsigned(location.line as u128),
            "column" => Unsigned(location.column as u128), "resource" => Code(resource), "limit" => Unsigned(u128::from(*limit))
        },
        Profile(reason) => "toml-profile" { "reason" => Text(reason) },
        Iri(cause) => "toml-iri" { "cause" => Cause(cause) },
        Rdf(cause) => "toml-rdf" { "cause" => Cause(cause) },
        Cover(cause) => "toml-cover" { "cause" => Cause(cause) },
        Metadata(reason) => "toml-metadata" { "reason" => Text(reason) },
    }
}

purrdf_lex::variant_from!(TomlError {
    Storage(purrdf_lex::allocation::StorageError),
    Iri(purrdf_core::IriError), Rdf(purrdf_core::RdfDiagnostic),
    Cover(purrdf_core::cover::ReconstructError)
});

/// Validate TOML and retain its typed occurrences under an explicit profile.
pub fn analyze<'a>(
    source: SourceDocument<'a>,
    profile: &Profile,
) -> Result<Document<'a>, TomlError> {
    BaseIri::parse(source.id)?;
    if source.bytes.len() as u64 > profile.bounds().max_source_bytes {
        return Err(TomlError::Limit {
            location: Location {
                byte: 0,
                line: 1,
                column: 1,
            },
            resource: "source bytes",
            limit: profile.bounds().max_source_bytes,
        });
    }
    let text = std::str::from_utf8(source.bytes).map_err(|error| TomlError::Utf8 {
        at: error.valid_up_to(),
    })?;
    let (nodes, statements, keys) = parse::parse(text, profile.bounds())?;
    let digest = ContentDigest::of(source.bytes);
    let id = profile.document_id(source.id, digest);
    Ok(Document {
        source,
        text,
        nodes,
        statements,
        keys,
        digest,
        id,
        profile: profile.identity(),
    })
}
