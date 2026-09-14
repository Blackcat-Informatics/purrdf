// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::sync::Arc;

use purrdf_core::{
    RdfLiteral,
    ir::{RdfDataset, RdfDatasetBuilder, TermId},
};

use crate::profile::{RDF_TYPE, TERMS, XSD_INTEGER};
use crate::{Document, JsonError, Profile, SourceDocument, analyze};

/// Analyze and project a source directly to the shared RDF 1.2 IR.
pub fn encode(source: SourceDocument<'_>, profile: &Profile) -> Result<Arc<RdfDataset>, JsonError> {
    project(&analyze(source, profile)?, profile)
}

/// Project a validated model. The supplied profile must match the profile used
/// to analyze it. No Turtle intermediate, filesystem or ambient configuration.
pub fn project(document: &Document<'_>, profile: &Profile) -> Result<Arc<RdfDataset>, JsonError> {
    if document.profile_id != profile.identity() {
        return Err(JsonError::ProfileMismatch);
    }
    let mut writer = Writer::new(profile);
    let root = writer.builder.intern_iri(document.id());
    writer.class(root, "Document");
    writer.iri(root, "source", document.source.id);
    writer.typed(
        root,
        "sourceDigest",
        &format!("sha256:{}", document.digest.to_hex()),
        "digest",
    );
    writer.number(root, "byteLength", document.text.len());
    writer.string(root, "profile", &profile.identity().to_hex());
    let value_ids: Vec<_> = (0..document.values.len())
        .map(|index| writer.builder.intern_iri(&value_id(document.id(), index)))
        .collect();
    for (index, value) in document.values.iter().enumerate() {
        let node = value_ids[index];
        writer.class(node, "Value");
        writer.link(node, "document", root);
        writer.number(node, "byteStart", value.span.start);
        writer.number(node, "byteEnd", value.span.end);
        writer.number(node, "occurrence", index);
        writer.number(node, "ordinal", value.ordinal);
        writer.link(
            node,
            "parent",
            value.parent.map_or(root, |parent| value_ids[parent]),
        );
        writer.string(node, "path", &value.path);
        writer.string(node, "kind", value.kind.name());
        if value.kind.is_scalar() {
            writer.string(node, "text", &document.text[value.span.clone()]);
        } else {
            writer.number(node, "size", value.size as usize);
        }
    }
    for (index, run) in document.runs.iter().enumerate() {
        let node = writer
            .builder
            .intern_iri(&structure_id(document.id(), index));
        writer.class(node, "Structure");
        writer.link(node, "document", root);
        writer.number(node, "byteStart", run.start);
        writer.number(node, "byteEnd", run.end);
        writer.typed(
            node,
            "verbatim",
            &document.text[run.clone()],
            "verbatimText",
        );
    }
    writer.builder.freeze().map_err(JsonError::Rdf)
}

pub(crate) fn value_id(document: &str, index: usize) -> String {
    format!("{document}#v{index}")
}
pub(crate) fn structure_id(document: &str, index: usize) -> String {
    format!("{document}#s{index}")
}

struct Writer<'a> {
    builder: RdfDatasetBuilder,
    terms: [(&'static str, TermId); TERMS.len()],
    rdf_type: TermId,
    vocabulary: &'a crate::Vocabulary,
}

impl<'a> Writer<'a> {
    fn new(profile: &'a Profile) -> Self {
        let mut builder = RdfDatasetBuilder::new();
        let terms = TERMS.map(|name| (name, builder.intern_iri(profile.vocabulary().iri(name))));
        let rdf_type = builder.intern_iri(RDF_TYPE);
        Self {
            builder,
            terms,
            rdf_type,
            vocabulary: profile.vocabulary(),
        }
    }

    fn term(&self, local: &str) -> TermId {
        self.terms
            .iter()
            .find(|(name, _)| *name == local)
            .expect("codec term belongs to the vocabulary")
            .1
    }

    fn class(&mut self, subject: TermId, class: &str) {
        self.builder
            .push_quad(subject, self.rdf_type, self.term(class), None);
    }

    fn link(&mut self, subject: TermId, predicate: &str, object: TermId) {
        self.builder
            .push_quad(subject, self.term(predicate), object, None);
    }

    fn iri(&mut self, subject: TermId, predicate: &str, object: &str) {
        let object = self.builder.intern_iri(object);
        self.link(subject, predicate, object);
    }

    fn string(&mut self, subject: TermId, predicate: &str, value: &str) {
        let object = self.builder.intern_literal(RdfLiteral::simple(value));
        self.link(subject, predicate, object);
    }

    fn typed(&mut self, subject: TermId, predicate: &str, value: &str, datatype: &str) {
        let object = self
            .builder
            .intern_literal(RdfLiteral::typed(value, self.vocabulary.iri(datatype)));
        self.link(subject, predicate, object);
    }

    fn number(&mut self, subject: TermId, predicate: &str, value: usize) {
        let object = self
            .builder
            .intern_literal(RdfLiteral::typed(value.to_string(), XSD_INTEGER));
        self.link(subject, predicate, object);
    }
}
