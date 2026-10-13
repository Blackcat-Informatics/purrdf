// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed metadata validation before any reconstructed bytes escape.

use std::collections::BTreeSet;

use purrdf_core::{
    ContentDigest,
    cover::{ByteSpan, reconstruct_bytes},
    datatype::{XSD_HEX_BINARY, XSD_INTEGER, XSD_STRING},
    ir::{RdfDataset, TermValue},
};

use crate::{MimeError, Profile, SourceDocument, analyze, error::metadata, project};

fn one(
    dataset: &RdfDataset,
    subject: &str,
    predicate: &str,
    field: &'static str,
) -> Result<TermValue, MimeError> {
    let mut values = dataset
        .quads()
        .filter(|quad| {
            quad.g.is_none()
                && dataset.term_value(quad.s).as_iri() == Some(subject)
                && dataset.term_value(quad.p).as_iri() == Some(predicate)
        })
        .map(|quad| dataset.term_value(quad.o));
    let value = values.next().ok_or_else(|| metadata(subject, field))?;
    if values.next().is_some() {
        return Err(metadata(subject, field));
    }
    Ok(value)
}

fn literal(
    value: TermValue,
    datatype: &str,
    subject: &str,
    field: &'static str,
) -> Result<String, MimeError> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype: actual,
            language: None,
            direction: None,
        } if actual == datatype => Ok(lexical_form),
        _ => Err(metadata(subject, field)),
    }
}

fn number(
    dataset: &RdfDataset,
    subject: &str,
    profile: &Profile,
    field: &'static str,
) -> Result<u64, MimeError> {
    let spelling = literal(
        one(dataset, subject, profile.vocabulary().iri(field), field)?,
        XSD_INTEGER,
        subject,
        field,
    )?;
    let value = spelling
        .parse::<u64>()
        .map_err(|_| metadata(subject, field))?;
    if value.to_string() != spelling {
        return Err(metadata(subject, field));
    }
    Ok(value)
}

fn selected(
    dataset: &RdfDataset,
    document: &str,
    profile: &Profile,
) -> Result<BTreeSet<Vec<u8>>, MimeError> {
    let prefix = format!("{document}/");
    let mut statements = BTreeSet::new();
    for quad in dataset.quads() {
        let subject = dataset.term_value(quad.s);
        let predicate = dataset.term_value(quad.p);
        let object = dataset.term_value(quad.o);
        let owned = subject
            .as_iri()
            .is_some_and(|iri| iri == document || iri.starts_with(&prefix));
        let declares_membership = predicate.as_iri() == Some(profile.vocabulary().iri("document"))
            && object.as_iri() == Some(document);
        if !owned && !declares_membership {
            continue;
        }
        if quad.g.is_some() || !owned {
            return Err(metadata(document, "document membership"));
        }
        let mut bytes = Vec::new();
        subject.canonical_bytes(&mut bytes);
        predicate.canonical_bytes(&mut bytes);
        object.canonical_bytes(&mut bytes);
        statements.insert(bytes);
    }
    // Reification/annotation is outside the closed MIME representation, so an
    // asserted extension cannot silently stand in for its occurrence metadata.
    for (reifier, _) in dataset.reifiers() {
        if dataset
            .term_value(reifier)
            .as_iri()
            .is_some_and(|iri| iri == document || iri.starts_with(&prefix))
        {
            return Err(metadata(document, "reifier"));
        }
    }
    for (subject, _, _) in dataset.annotations() {
        if dataset
            .term_value(subject)
            .as_iri()
            .is_some_and(|iri| iri == document || iri.starts_with(&prefix))
        {
            return Err(metadata(document, "annotation"));
        }
    }
    Ok(statements)
}

/// Rebuild a selected message from RDF, prove its shared cover identity, then
/// reparse and compare every asserted occurrence and metadata fact. Other
/// document namespaces may coexist; invented, omitted or conflicting selected
/// metadata refuses the entire result. No partial source bytes are returned.
///
/// # Errors
/// Returns typed metadata, profile, cover, caller-resource or RDF refusals.
pub fn decode_document(
    dataset: &RdfDataset,
    document: &str,
    profile: &Profile,
) -> Result<Vec<u8>, MimeError> {
    let vocabulary = profile.vocabulary();
    let TermValue::Iri(source) = one(dataset, document, vocabulary.iri("source"), "source")? else {
        return Err(metadata(document, "source"));
    };
    let declared_profile = literal(
        one(dataset, document, vocabulary.iri("profile"), "profile")?,
        XSD_STRING,
        document,
        "profile",
    )?;
    if declared_profile != profile.identity().to_hex() {
        return Err(MimeError::ProfileMismatch);
    }
    let digest = literal(
        one(
            dataset,
            document,
            vocabulary.iri("sourceDigest"),
            "sourceDigest",
        )?,
        XSD_STRING,
        document,
        "sourceDigest",
    )?;
    let digest =
        ContentDigest::from_hex(&digest).ok_or_else(|| metadata(document, "sourceDigest"))?;
    let length = number(dataset, document, profile, "byteLength")?;
    crate::Limits::check(profile.limits().source_bytes, length, "source bytes")?;
    let prefix = format!("{document}/span/");
    let mut subjects = BTreeSet::new();
    for quad in dataset.quads() {
        if let Some(subject) = dataset
            .term_value(quad.s)
            .as_iri()
            .filter(|iri| iri.starts_with(&prefix))
        {
            subjects.insert(subject.to_owned());
        }
    }
    let mut payloads = Vec::new();
    let mut ranges = Vec::new();
    for subject in subjects {
        let start = number(dataset, &subject, profile, "byteStart")?;
        let end = number(dataset, &subject, profile, "byteEnd")?;
        let spelling = literal(
            one(dataset, &subject, vocabulary.iri("verbatim"), "verbatim")?,
            XSD_HEX_BINARY,
            &subject,
            "verbatim",
        )?;
        let bytes = purrdf_xsd::parse_hex(&spelling).map_err(|_| metadata(&subject, "verbatim"))?;
        if purrdf_hash::hex::encode(&bytes) != spelling {
            return Err(metadata(&subject, "verbatim"));
        }
        payloads.push(bytes);
        ranges.push((start, end));
    }
    let spans: Vec<_> = ranges
        .iter()
        .zip(&payloads)
        .map(|(&(start, end), bytes)| ByteSpan {
            byte_start: start,
            byte_end: end,
            bytes,
            continues: None,
        })
        .collect();
    let bytes = reconstruct_bytes(length, &digest, &spans)?;
    let model = analyze(
        SourceDocument {
            id: &source,
            bytes: &bytes,
        },
        profile,
    )?;
    if model.id() != document {
        return Err(metadata(document, "document identity"));
    }
    let expected = project(&model, profile)?;
    if selected(dataset, document, profile)? != selected(&expected, document, profile)? {
        return Err(metadata(document, "occurrence metadata"));
    }
    Ok(bytes)
}
