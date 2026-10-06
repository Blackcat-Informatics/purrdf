// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered JSON as a queryable RDF 1.2 byte cover.
//!
//! [`analyze`] validates JSON and borrows its lexical spans; [`project`] writes
//! the model directly to the shared IR. [`decode_document`] reads an explicitly
//! selected document, reconstructs its bytes through the kernel cover law, then
//! reparses them to verify every asserted occurrence, path, kind and identity.
//! Object order, duplicate keys, whitespace, escapes and number spellings survive.
//! A profile and vocabulary are always explicit.

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

mod decode;
mod error;
mod model;
mod parse;
mod profile;
mod project;

pub use decode::decode_document;
pub use error::JsonError;
pub use model::{Document, Kind, SourceDocument, Value};
pub use profile::{Bounds, Profile, STANDARD_NAMESPACE, Vocabulary};
pub use project::{encode, project};

use purrdf_core::{BaseIri, ContentDigest};

/// Validate a source and describe its occurrences without copying source spans.
pub fn analyze<'a>(
    source: SourceDocument<'a>,
    profile: &Profile,
) -> Result<Document<'a>, JsonError> {
    BaseIri::parse(source.id).map_err(JsonError::Iri)?;
    if source.bytes.len() as u64 > profile.bounds().max_source_bytes {
        return Err(JsonError::Limit {
            resource: "source bytes",
            limit: profile.bounds().max_source_bytes,
        });
    }
    let text = core::str::from_utf8(source.bytes).map_err(|error| JsonError::InvalidUtf8 {
        valid_up_to: error.valid_up_to(),
    })?;
    let (values, runs) = parse::parse(text, profile.bounds())?;
    let digest = ContentDigest::of(source.bytes);
    let id = profile.document_id(source.id, &digest);
    Ok(Document {
        source,
        text,
        values,
        runs,
        digest,
        id,
        profile_id: profile.identity(),
    })
}
