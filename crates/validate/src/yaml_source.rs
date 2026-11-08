// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit YAML source profiles, native RDF export and verified reconstruction.
//!
//! Profile JSON requires `version: 1`, `name`, nonzero `profile_version` and
//! `vocabulary`. Optional `bounds` contains `syntax` and the six native phase
//! bounds. Every numeric bound is an unsigned integer through `u32::MAX`;
//! `syntax.aliases` is boolean. No namespace or standard profile is implicit.
//!
//! Omitted bounds use [`Bounds::standard`]: 16 MiB source, 128 MiB Lex storage,
//! 1,048,576 occurrences/documents, depth 128 and aliases enabled; 2,097,153
//! cover spans, 64 MiB partition, 2,097,152 statements, 512 MiB payload and
//! projection each, and 256 MiB decode storage. Zero ceilings are valid policies.
//!
//! JSON decoding has its own 128 KiB account, 16 KiB input, four open containers
//! and 64 values. Native profile construction is separate. Lex, partition,
//! projection, caller RDF/parser storage and decode/reparse/reprojection retain
//! their native separate accounts. Export additionally owns a fallible copy of
//! the root IRI, at most 4,096 bytes; this does not share or reset the dataset's
//! projection account. None of these limits is a combined process-memory claim.

#[cfg(test)]
mod tests;

use crate::json_config::{field, portable};
use purrdf_core::{RdfDataset, RdfDiagnostic};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_lex::json::{
    Value,
    record::{DecodeError, Record, RecordAllocation, Within},
};
pub use purrdf_yaml::{Bounds, Profile, SourceDocument, Vocabulary, YamlError};

/// Maximum profile JSON input bytes.
pub const MAX_PROFILE_BYTES: usize = 16 << 10;
/// Separate JSON decoder/record storage ceiling.
pub const MAX_PROFILE_ALLOCATION_BYTES: usize = 128 << 10;

/// Original configuration diagnostic or native source-carrier refusal.
#[derive(Debug)]
pub enum Error {
    /// Strict JSON/record refusal, retaining its original typed cause.
    Configuration(RdfDiagnostic),
    /// Original syntax, storage, cover, IRI, RDF, profile or metadata refusal.
    Native(YamlError),
}
impl core::fmt::Display for Error {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Configuration(cause) => cause.fmt(out),
            Self::Native(cause) => cause.fmt(out),
        }
    }
}
purrdf_hash::error_sources!(Error, Configuration, Native);
purrdf_lex::variant_from!(Error { Configuration(RdfDiagnostic), Native(YamlError) });
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        Configuration(cause) => "yaml-source-configuration" { "cause" => Cause(cause) },
        Native(cause) => "yaml-source-native" { "cause" => Cause(cause) },
    }
}

impl Error {
    /// Preserve configuration diagnostics or retain the original native refusal.
    ///
    /// Native codes and exact positions remain available through the diagnostic's
    /// structured native cause. Diagnostic allocation failure reports its actual
    /// storage refusal instead of publishing an incomplete diagnostic.
    pub fn into_diagnostic(self) -> RdfDiagnostic {
        match self {
            Self::Configuration(diagnostic) => diagnostic,
            Self::Native(cause) => {
                let mut current: &(dyn std::error::Error + 'static) = &cause;
                let storage = loop {
                    if let Some(storage) =
                        current.downcast_ref::<purrdf_lex::allocation::StorageError>()
                    {
                        break Some(*storage);
                    }
                    match current.source() {
                        Some(source) => current = source,
                        None => break None,
                    }
                };
                crate::json_config::failure("yaml-source-native", cause, storage)
            }
        }
    }
}

fn syntax(
    value: &Value,
    bounds: &mut Bounds,
    allocation: &dyn RecordAllocation,
) -> Result<(), DecodeError> {
    let mut fields = Record::new_native(value, "YAML source syntax object", Some(allocation))?;
    let limits = &mut bounds.syntax.syntax;
    limits.max_input_bytes = portable(
        &mut fields,
        "max_input_bytes",
        limits.max_input_bytes,
        allocation,
    )?;
    limits.max_allocation_bytes = portable(
        &mut fields,
        "max_allocation_bytes",
        limits.max_allocation_bytes,
        allocation,
    )?;
    let default = u32::try_from(limits.max_occurrences).map_err(|_| {
        DecodeError::custom_native("native occurrence default exceeds u32", Some(allocation))
    })?;
    limits.max_occurrences = u64::from(field(&mut fields, "max_occurrences", default, allocation)?);
    limits.max_depth = portable(&mut fields, "max_depth", limits.max_depth, allocation)?;
    limits.aliases = field(&mut fields, "aliases", limits.aliases, allocation)?;
    bounds.syntax.max_documents = portable(
        &mut fields,
        "max_documents",
        bounds.syntax.max_documents,
        allocation,
    )?;
    fields.deny_unknown()
}

fn bounds(value: &Value, allocation: &dyn RecordAllocation) -> Result<Bounds, DecodeError> {
    let mut fields = Record::new_native(value, "YAML source bounds object", Some(allocation))?;
    let mut bounds = Bounds::standard();
    if let Some(value) = fields.raw("syntax")? {
        syntax(value, &mut bounds, allocation)
            .map_err(|error| error.within_native("syntax", Some(allocation)))?;
    }
    bounds.max_cover_spans = portable(
        &mut fields,
        "max_cover_spans",
        bounds.max_cover_spans,
        allocation,
    )?;
    bounds.max_partition_bytes = portable(
        &mut fields,
        "max_partition_bytes",
        bounds.max_partition_bytes,
        allocation,
    )?;
    bounds.max_statements = portable(
        &mut fields,
        "max_statements",
        bounds.max_statements,
        allocation,
    )?;
    bounds.max_payload_bytes = portable(
        &mut fields,
        "max_payload_bytes",
        bounds.max_payload_bytes,
        allocation,
    )?;
    bounds.max_projection_bytes = portable(
        &mut fields,
        "max_projection_bytes",
        bounds.max_projection_bytes,
        allocation,
    )?;
    bounds.max_decode_bytes = portable(
        &mut fields,
        "max_decode_bytes",
        bounds.max_decode_bytes,
        allocation,
    )?;
    fields.deny_unknown()?;
    Ok(bounds)
}

/// Decode one complete, closed version-one profile before document work.
///
/// # Errors
/// Preserves strict JSON/record failures and original native profile/IRI/storage
/// refusals. Null, duplicate, unknown, exponent and fractional controls fail.
pub fn profile(bytes: &[u8]) -> Result<Profile, Error> {
    let (name, version, vocabulary, bounds) = crate::json_config::decode(
        bytes,
        "yaml-source-profile",
        MAX_PROFILE_BYTES,
        MAX_PROFILE_ALLOCATION_BYTES,
        purrdf_lex::json::Limits {
            max_depth: 4,
            max_values: 64,
            max_string_bytes: MAX_PROFILE_BYTES,
            unique_members: true,
        },
        "YAML source profile document exceeds 16384 bytes",
        |value, allocation, _memory| {
            let mut fields =
                Record::new_native(value, "YAML source profile object", Some(allocation))?;
            let version: u32 = fields.required_native("version")?;
            if version != 1 {
                return Err(DecodeError::custom_native(
                    "YAML source profile schema version must be 1",
                    Some(allocation),
                ));
            }
            let name: String = fields.required_native("name")?;
            let version: u32 = fields.required_native("profile_version")?;
            let vocabulary: String = fields.required_native("vocabulary")?;
            let limits = match fields.raw("bounds")? {
                Some(value) => bounds(value, allocation)
                    .map_err(|error| error.within_native("bounds", Some(allocation)))?,
                None => Bounds::standard(),
            };
            fields.deny_unknown()?;
            Ok((name, version, vocabulary, limits))
        },
    )?;
    Ok(Profile::new(
        &name,
        version,
        Vocabulary::under(&vocabulary)?,
        bounds,
    )?)
}

/// Native projected metadata and its original computed stream root.
#[derive(Debug)]
pub struct Export {
    /// The root supplied by the analyzed native document, copied fallibly.
    pub root: String,
    /// The native carrier dataset; no envelope or alternate projection is added.
    pub dataset: RdfDataset,
}

/// Analyze exact source bytes and project through the one native carrier law.
/// # Errors
/// Returns the original native refusal, including positions and typed causes.
pub fn export(source_iri: &str, bytes: &[u8], profile: &Profile) -> Result<Export, Error> {
    let document = purrdf_yaml::analyze(
        SourceDocument {
            id: source_iri,
            bytes,
        },
        profile,
    )?;
    let dataset = purrdf_yaml::project(&document, profile)?;
    let mut resident = Resident;
    let root = Memory::new(&mut resident)
        .string(document.id())
        .map_err(YamlError::from)?;
    Ok(Export { root, dataset })
}

/// Reconstruct exact original UTF-8 bytes for an explicit root and profile.
/// # Errors
/// Native cover/digest, reparse, identity and complete reprojection checks must
/// all succeed. Caller RDF/parser storage is outside native decode admission.
pub fn reconstruct(dataset: &RdfDataset, root: &str, profile: &Profile) -> Result<String, Error> {
    Ok(purrdf_yaml::decode_stream(dataset, root, profile)?)
}
