// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Occurrence-preserving projection directly into the production RDF IR.

use std::{ops::Range, sync::Arc};

use purrdf_core::{
    datatype::XSD_HEX_BINARY,
    ir::{RdfDataset, RdfDatasetBuilder, TermValue},
};

use crate::{Document, MimeError, Profile, Vocabulary};

struct Writer<'a> {
    builder: RdfDatasetBuilder,
    vocabulary: &'a Vocabulary,
    document: &'a str,
}

impl Writer<'_> {
    fn fact(&mut self, subject: &str, predicate: &str, object: TermValue) {
        let subject = self.builder.intern_iri(subject);
        let predicate = self.builder.intern_iri(predicate);
        let object = self.builder.intern_owned_term(
            &object
                .into_rdf_term()
                .expect("projection creates no triple-term predicate"),
        );
        self.builder.push_quad(subject, predicate, object, None);
    }

    fn field(&mut self, subject: &str, field: &str, object: TermValue) {
        self.fact(subject, self.vocabulary.iri(field), object);
    }

    fn number(&mut self, subject: &str, field: &str, value: usize) {
        self.field(subject, field, TermValue::integer(value as i128));
    }

    fn bytes(&mut self, subject: &str, field: &str, value: &[u8]) {
        self.field(
            subject,
            field,
            TermValue::typed_literal(purrdf_hash::hex::encode(value), XSD_HEX_BINARY),
        );
    }

    fn range(&mut self, subject: &str, range: &Range<usize>) {
        self.number(subject, "byteStart", range.start);
        self.number(subject, "byteEnd", range.end);
    }

    fn occurrence(&mut self, subject: &str, class: &str, ordinal: usize) {
        self.fact(
            subject,
            purrdf_core::vocab::rdf::TYPE,
            TermValue::iri(self.vocabulary.iri(class)),
        );
        self.field(subject, "document", TermValue::iri(self.document));
        self.number(subject, "ordinal", ordinal);
        self.field(self.document, "occurrence", TermValue::iri(subject));
    }
}

pub(crate) fn node(document: &str, kind: &str, ordinal: usize) -> String {
    format!("{document}/{kind}/{ordinal}")
}

/// Project every occurrence, physical byte-cover span and typed defect into a
/// default-graph dataset. Repeated equal headers have distinct occurrence IRIs.
/// No text encoding is guessed: all byte-valued facts are `xsd:hexBinary`.
///
/// # Errors
/// Refuses a changed profile, invalid cover, or an RDF construction failure.
pub fn project(document: &Document<'_>, profile: &Profile) -> Result<Arc<RdfDataset>, MimeError> {
    if document.profile_id != profile.identity() {
        return Err(MimeError::ProfileMismatch);
    }
    document.cover.reconstruct()?;
    let original = crate::analyze(document.source, profile)?;
    if original.id() != document.id()
        || original.parts != document.parts
        || original.headers != document.headers
        || original.structures != document.structures
        || original.problems != document.problems
        || original.cover != document.cover
    {
        return Err(crate::error::metadata(document.id(), "source model"));
    }
    let mut writer = Writer {
        builder: RdfDatasetBuilder::new(),
        vocabulary: profile.vocabulary(),
        document: document.id(),
    };
    let id = document.id();
    writer.fact(
        id,
        purrdf_core::vocab::rdf::TYPE,
        TermValue::iri(profile.vocabulary().iri("Document")),
    );
    writer.field(id, "source", TermValue::iri(document.source.id));
    writer.field(
        id,
        "sourceDigest",
        TermValue::simple_literal(document.cover.source_digest().to_hex()),
    );
    writer.field(
        id,
        "profile",
        TermValue::simple_literal(profile.identity().to_hex()),
    );
    writer.number(id, "byteLength", document.source.bytes.len());

    for (index, span) in document.cover.spans().iter().enumerate() {
        let subject = node(id, "span", index);
        writer.occurrence(&subject, "Span", index);
        writer.number(&subject, "byteStart", span.byte_start as usize);
        writer.number(&subject, "byteEnd", span.byte_end as usize);
        writer.bytes(&subject, "verbatim", span.bytes);
    }
    for (index, part) in document.parts.iter().enumerate() {
        let subject = node(id, "part", index);
        writer.occurrence(&subject, "Part", part.ordinal);
        writer.range(&subject, &part.span);
        writer.number(&subject, "bodyStart", part.body.start);
        writer.number(&subject, "bodyEnd", part.body.end);
        writer.number(&subject, "depth", part.depth);
        writer.field(
            &subject,
            "transferEncoding",
            TermValue::simple_literal(part.transfer_encoding.name()),
        );
        if let crate::TransferEncoding::Unknown(token) = &part.transfer_encoding {
            writer.bytes(&subject, "value", token);
        }
        if let Some(parent) = part.parent {
            writer.field(&subject, "parent", TermValue::iri(node(id, "part", parent)));
        }
        for child in &part.children {
            writer.field(&subject, "child", TermValue::iri(node(id, "part", *child)));
        }
        if let Some(media) = &part.media_type {
            let mut token = media.main.clone();
            token.push(b'/');
            token.extend_from_slice(&media.sub);
            writer.bytes(&subject, "mediaType", &token);
            for (ordinal, (name, value)) in media.parameters.iter().enumerate() {
                let parameter = format!("{subject}/parameter/{ordinal}");
                writer.occurrence(&parameter, "Parameter", ordinal);
                writer.field(&subject, "parameter", TermValue::iri(&parameter));
                writer.field(&parameter, "part", TermValue::iri(&subject));
                writer.bytes(&parameter, "parameterName", name);
                writer.bytes(&parameter, "parameterValue", value);
            }
        }
        // A malformed transfer has a typed source problem, never a fabricated
        // decoded-content identity. Explicit decoding returns its exact refusal.
        if let Ok(decoded) = document.decode_original_part(index) {
            writer.field(
                &subject,
                "decodedDigest",
                TermValue::simple_literal(decoded.digest.to_hex()),
            );
        }
    }
    for (index, header) in document.headers.iter().enumerate() {
        let subject = node(id, "header", index);
        writer.occurrence(&subject, "Header", header.ordinal);
        writer.range(&subject, &header.span);
        writer.field(
            &subject,
            "part",
            TermValue::iri(node(id, "part", header.part)),
        );
        if let Some(name) = &header.name {
            writer.bytes(&subject, "name", &document.source.bytes[name.clone()]);
        }
        writer.bytes(&subject, "value", &header.unfolded(document.source.bytes));
        for (ordinal, range) in header.segments.iter().enumerate() {
            let segment = format!("{subject}/segment/{ordinal}");
            writer.occurrence(&segment, "Segment", ordinal);
            writer.field(&subject, "segment", TermValue::iri(&segment));
            writer.field(&segment, "related", TermValue::iri(&subject));
            writer.range(&segment, range);
        }
    }
    for (index, structure) in document.structures.iter().enumerate() {
        let subject = node(id, "structure", index);
        writer.occurrence(&subject, "Structure", index);
        writer.range(&subject, &structure.span);
        writer.field(
            &subject,
            "part",
            TermValue::iri(node(id, "part", structure.part)),
        );
        writer.field(
            &subject,
            "kind",
            TermValue::simple_literal(structure.kind.name()),
        );
    }
    for (index, problem) in document.problems.iter().enumerate() {
        let subject = node(id, "problem", index);
        writer.occurrence(&subject, "Problem", index);
        writer.range(&subject, &problem.span);
        writer.field(
            &subject,
            "part",
            TermValue::iri(node(id, "part", problem.part)),
        );
        writer.field(
            &subject,
            "kind",
            TermValue::simple_literal(problem.kind.name()),
        );
    }
    Ok(writer.builder.freeze()?)
}
