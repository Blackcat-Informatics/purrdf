// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value as YamlValue;

use super::reader::{extract_markdown_links, resolve_link_path};
use super::{OkfBundle, OkfConfig, OkfError, decimal_lexical_from_f64, minted_document_iri};
use crate::{
    BlankScope, LossEntry, LossLedger, QuadIds, RdfDataset, RdfDatasetVisitor, RdfLocation,
    RdfTextDirection, TermId, TermRef,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// Result of projecting an RDF 1.2 dataset into an OKF Markdown bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OkfWriteOutcome {
    /// Deterministic in-memory Markdown bundle.
    pub bundle: OkfBundle,
    /// Always-computed runtime loss ledger.
    pub losses: LossLedger,
    /// Number of concept documents written.
    pub documents: usize,
}

#[derive(Clone, Debug)]
enum OwnedTerm {
    Iri(String),
    Blank {
        label: String,
        scope: BlankScope,
    },
    Literal {
        lexical: String,
        datatype: TermId,
        language: Option<String>,
        direction: Option<RdfTextDirection>,
    },
    Triple {
        s: TermId,
        p: TermId,
        o: TermId,
    },
}

impl OwnedTerm {
    fn from_ref(term: TermRef<'_>) -> Self {
        match term {
            TermRef::Iri(iri) => Self::Iri(iri.to_owned()),
            TermRef::Blank { label, scope } => Self::Blank {
                label: label.to_owned(),
                scope,
            },
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => Self::Literal {
                lexical: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            },
            TermRef::Triple { s, p, o } => Self::Triple { s, p, o },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ReifierEvent {
    ordinal: usize,
    reifier: TermId,
    triple: TermId,
    graph: Option<TermId>,
}

#[derive(Clone, Copy, Debug)]
struct AnnotationEvent {
    ordinal: usize,
    reifier: TermId,
    predicate: TermId,
    object: TermId,
    graph: Option<TermId>,
}

/// Event receiver for the RDF-dataset → OKF projection.
///
/// Drive it with [`RdfDataset::emit`], then call [`finish`](Self::finish). Term
/// declarations must be dense and ascending, exactly as the frozen-dataset driver
/// guarantees. The writer retains owned term values because frontmatter grouping and
/// link-layer verification happen after the visitor has seen the complete dataset.
#[derive(Debug)]
pub struct OkfWriter<'a> {
    config: &'a OkfConfig,
    terms: Vec<OwnedTerm>,
    quads: Vec<(usize, QuadIds)>,
    reifiers: Vec<ReifierEvent>,
    annotations: Vec<AnnotationEvent>,
    stream_error: Option<OkfError>,
}

impl<'a> OkfWriter<'a> {
    /// Construct a writer using mandatory caller-owned configuration.
    pub fn new(config: &'a OkfConfig) -> Self {
        Self {
            config,
            terms: Vec::new(),
            quads: Vec::new(),
            reifiers: Vec::new(),
            annotations: Vec::new(),
            stream_error: None,
        }
    }

    /// Finish grouping, verify the exact OKF link layer, render the bundle, and
    /// return its runtime loss ledger.
    ///
    /// # Errors
    ///
    /// Returns [`OkfError`] when the visitor stream is malformed, configured OKF
    /// profile data is ambiguous/inconsistent, a link target/layer is incomplete,
    /// a YAML value cannot round-trip without value loss, or bundle limits are hit.
    pub fn finish(mut self) -> Result<OkfWriteOutcome, OkfError> {
        if let Some(error) = self.stream_error.take() {
            return Err(error);
        }
        Projector::new(
            self.config,
            self.terms,
            self.quads,
            self.reifiers,
            self.annotations,
        )
        .project()
    }

    fn remember_error(&mut self, detail: impl Into<String>) {
        if self.stream_error.is_none() {
            self.stream_error = Some(OkfError::new(detail));
        }
    }

    fn push_reifier(&mut self, reifier: TermId, triple: TermId, graph: Option<TermId>) {
        let ordinal = self.reifiers.len();
        self.reifiers.push(ReifierEvent {
            ordinal,
            reifier,
            triple,
            graph,
        });
    }

    fn push_annotation(
        &mut self,
        reifier: TermId,
        predicate: TermId,
        object: TermId,
        graph: Option<TermId>,
    ) {
        let ordinal = self.annotations.len();
        self.annotations.push(AnnotationEvent {
            ordinal,
            reifier,
            predicate,
            object,
            graph,
        });
    }
}

impl RdfDatasetVisitor for OkfWriter<'_> {
    fn term(&mut self, id: TermId, term: TermRef<'_>) {
        if self.stream_error.is_some() {
            return;
        }
        if id.index() != self.terms.len() {
            self.remember_error(format!(
                "OKF writer requires dense ascending term declarations: expected index {}, got {}",
                self.terms.len(),
                id.index()
            ));
            return;
        }
        self.terms.push(OwnedTerm::from_ref(term));
    }

    fn quad(&mut self, quad: QuadIds) {
        if self.stream_error.is_none() {
            let ordinal = self.quads.len();
            self.quads.push((ordinal, quad));
        }
    }

    fn reifier(&mut self, reifier: TermId, triple: TermId) {
        if self.stream_error.is_none() {
            self.push_reifier(reifier, triple, None);
        }
    }

    fn reifier_in_graph(&mut self, reifier: TermId, triple: TermId, graph: Option<TermId>) {
        if self.stream_error.is_none() {
            self.push_reifier(reifier, triple, graph);
        }
    }

    fn annotation(&mut self, reifier: TermId, predicate: TermId, object: TermId) {
        if self.stream_error.is_none() {
            self.push_annotation(reifier, predicate, object, None);
        }
    }

    fn annotation_in_graph(
        &mut self,
        reifier: TermId,
        predicate: TermId,
        object: TermId,
        graph: Option<TermId>,
    ) {
        if self.stream_error.is_none() {
            self.push_annotation(reifier, predicate, object, graph);
        }
    }
}

/// Project a frozen RDF 1.2 dataset through [`OkfWriter`].
///
/// # Errors
///
/// Returns the same [`OkfError`] conditions as [`OkfWriter::finish`].
pub fn write_okf_bundle(
    dataset: &RdfDataset,
    config: &OkfConfig,
) -> Result<OkfWriteOutcome, OkfError> {
    let mut writer = OkfWriter::new(config);
    dataset.emit(&mut writer);
    writer.finish()
}

#[derive(Debug)]
struct DocumentAccumulator {
    subject: TermId,
    path: Option<String>,
    fields: BTreeMap<String, YamlValue>,
    tags: BTreeSet<String>,
    body: Option<String>,
}

impl DocumentAccumulator {
    fn new(subject: TermId) -> Self {
        Self {
            subject,
            path: None,
            fields: BTreeMap::new(),
            tags: BTreeSet::new(),
            body: None,
        }
    }

    fn set_path(&mut self, path: String) -> Result<(), OkfError> {
        if self.path.replace(path).is_some() {
            return Err(OkfError::new(format!(
                "OKF subject term#{} has more than one path",
                self.subject.index()
            )));
        }
        Ok(())
    }

    fn set_body(&mut self, body: String) -> Result<(), OkfError> {
        if self.body.replace(body).is_some() {
            return Err(OkfError::new(format!(
                "OKF subject term#{} has more than one body",
                self.subject.index()
            )));
        }
        Ok(())
    }

    fn set_field(&mut self, key: &str, value: YamlValue) -> Result<(), OkfError> {
        if self.fields.insert(key.to_owned(), value).is_some() {
            return Err(OkfError::new(format!(
                "OKF subject term#{} has more than one `{key}` value",
                self.subject.index()
            )));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FinalDocument {
    subject: TermId,
    fields: BTreeMap<String, YamlValue>,
    body: String,
}

type CollectedDocuments = (BTreeMap<TermId, DocumentAccumulator>, Vec<(usize, QuadIds)>);

#[derive(Clone, Debug)]
struct ExpectedLink {
    source: TermId,
    target: TermId,
    document_index: usize,
    ordinal: usize,
    text: String,
}

impl ExpectedLink {
    fn reifier_label(&self) -> String {
        format!("okf_link_{}_{}", self.document_index, self.ordinal)
    }
}

struct Projector<'a> {
    config: &'a OkfConfig,
    terms: Vec<OwnedTerm>,
    quads: Vec<(usize, QuadIds)>,
    reifiers: Vec<ReifierEvent>,
    annotations: Vec<AnnotationEvent>,
    losses: LossLedger,
}

impl<'a> Projector<'a> {
    fn new(
        config: &'a OkfConfig,
        terms: Vec<OwnedTerm>,
        quads: Vec<(usize, QuadIds)>,
        reifiers: Vec<ReifierEvent>,
        annotations: Vec<AnnotationEvent>,
    ) -> Self {
        Self {
            config,
            terms,
            quads,
            reifiers,
            annotations,
            losses: LossLedger::new(),
        }
    }

    fn project(mut self) -> Result<OkfWriteOutcome, OkfError> {
        let (documents, link_quads) = self.collect_documents()?;
        let documents = self.finalize_documents(documents)?;
        let expected_links = self.expected_links(&documents)?;
        self.reconcile_link_quads(&link_quads, &expected_links)?;
        self.reconcile_statement_layer(&expected_links)?;
        let bundle = self.render_bundle(documents)?;
        let documents = bundle.len();
        Ok(OkfWriteOutcome {
            bundle,
            losses: self.losses,
            documents,
        })
    }

    fn collect_documents(&mut self) -> Result<CollectedDocuments, OkfError> {
        let mut documents = BTreeMap::new();
        let mut link_quads = Vec::new();

        for &(ordinal, quad) in &self.quads {
            self.require_term(quad.s)?;
            self.require_term(quad.p)?;
            self.require_term(quad.o)?;
            if let Some(graph) = quad.g {
                self.require_term(graph)?;
                record_quad_loss(
                    &mut self.losses,
                    &self.terms,
                    ordinal,
                    quad.s,
                    "named-graph-dropped",
                    "OKF cannot represent named-graph placement.",
                );
                continue;
            }

            let predicate = self.iri(quad.p)?.to_owned();
            if predicate == self.config.path_predicate() {
                let path = self
                    .plain_literal(quad.o, XSD_STRING, "okf:path")?
                    .to_owned();
                super::validate_relative_markdown_path(&path)?;
                documents
                    .entry(quad.s)
                    .or_insert_with(|| DocumentAccumulator::new(quad.s))
                    .set_path(path)?;
            } else if predicate == self.config.body_predicate() {
                let body = self
                    .plain_literal(quad.o, XSD_STRING, "okf:body")?
                    .to_owned();
                documents
                    .entry(quad.s)
                    .or_insert_with(|| DocumentAccumulator::new(quad.s))
                    .set_body(body)?;
            } else if predicate == self.config.links_predicate() {
                let _ = self.iri(quad.o).map_err(|_| {
                    OkfError::new(format!(
                        "OKF link object term#{} must be an IRI",
                        quad.o.index()
                    ))
                })?;
                link_quads.push((ordinal, quad));
            } else if let Some(key) = self.config.key_for_predicate(&predicate) {
                let key = key.to_owned();
                match key.as_str() {
                    "resource" => {
                        let resource = self.iri(quad.o)?.to_owned();
                        documents
                            .entry(quad.s)
                            .or_insert_with(|| DocumentAccumulator::new(quad.s))
                            .set_field(&key, YamlValue::String(resource))?;
                    }
                    "tags" => {
                        let tag = self
                            .plain_literal(quad.o, XSD_STRING, "OKF tag")?
                            .to_owned();
                        documents
                            .entry(quad.s)
                            .or_insert_with(|| DocumentAccumulator::new(quad.s))
                            .tags
                            .insert(tag);
                    }
                    "timestamp" => {
                        let timestamp = self
                            .plain_literal(quad.o, XSD_DATETIME, "OKF timestamp")?
                            .to_owned();
                        parse_known_xsd(&timestamp, XSD_DATETIME).map_err(|error| {
                            OkfError::new(format!("invalid OKF timestamp `{timestamp}`: {error}"))
                        })?;
                        documents
                            .entry(quad.s)
                            .or_insert_with(|| DocumentAccumulator::new(quad.s))
                            .set_field(&key, YamlValue::String(timestamp))?;
                    }
                    "type" | "title" | "description" => {
                        let value = self
                            .plain_literal(quad.o, XSD_STRING, &format!("OKF `{key}`"))?
                            .to_owned();
                        documents
                            .entry(quad.s)
                            .or_insert_with(|| DocumentAccumulator::new(quad.s))
                            .set_field(&key, YamlValue::String(value))?;
                    }
                    _ => {
                        let value = self.extension_value(quad.o)?;
                        documents
                            .entry(quad.s)
                            .or_insert_with(|| DocumentAccumulator::new(quad.s))
                            .set_field(&key, value)?;
                    }
                }
            } else {
                record_quad_loss(
                    &mut self.losses,
                    &self.terms,
                    ordinal,
                    quad.s,
                    "okf-non-profile-quad-dropped",
                    "RDF statement is outside the caller-configured OKF profile.",
                );
            }
        }
        Ok((documents, link_quads))
    }

    fn finalize_documents(
        &self,
        documents: BTreeMap<TermId, DocumentAccumulator>,
    ) -> Result<BTreeMap<String, FinalDocument>, OkfError> {
        let mut by_path = BTreeMap::new();
        for (_, mut document) in documents {
            let subject_iri = self.iri(document.subject)?;
            super::validate_absolute_iri("OKF document subject", subject_iri)?;
            let path = document.path.take().ok_or_else(|| {
                OkfError::new(format!(
                    "OKF subject `{subject_iri}` has profile fields but no okf:path"
                ))
            })?;
            if !document.fields.contains_key("type") {
                return Err(OkfError::new(format!(
                    "OKF document `{path}` is missing the required `type` field"
                )));
            }
            if matches!(document.fields.get("type"), Some(YamlValue::String(value)) if value.is_empty())
            {
                return Err(OkfError::new(format!(
                    "OKF document `{path}` has an empty required `type` field"
                )));
            }
            let body = document.body.take().ok_or_else(|| {
                OkfError::new(format!("OKF document `{path}` is missing okf:body"))
            })?;

            match document.fields.get("resource") {
                Some(YamlValue::String(resource)) if resource == subject_iri => {}
                Some(YamlValue::String(resource)) => {
                    return Err(OkfError::new(format!(
                        "OKF document `{path}` resource `{resource}` does not equal its subject `{subject_iri}`"
                    )));
                }
                Some(_) => unreachable!("resource fields are built as YAML strings"),
                None => {
                    let expected = minted_document_iri(self.config, &path)?;
                    if expected != subject_iri {
                        return Err(OkfError::new(format!(
                            "OKF document `{path}` has subject `{subject_iri}` but no resource; expected caller-base IRI `{expected}`"
                        )));
                    }
                }
            }

            if !document.tags.is_empty() {
                document.fields.insert(
                    "tags".to_owned(),
                    YamlValue::Sequence(document.tags.into_iter().map(YamlValue::String).collect()),
                );
            }
            if by_path
                .insert(
                    path.clone(),
                    FinalDocument {
                        subject: document.subject,
                        fields: document.fields,
                        body,
                    },
                )
                .is_some()
            {
                return Err(OkfError::new(format!(
                    "more than one OKF subject claims document path `{path}`"
                )));
            }
        }
        Ok(by_path)
    }

    fn expected_links(
        &self,
        documents: &BTreeMap<String, FinalDocument>,
    ) -> Result<Vec<ExpectedLink>, OkfError> {
        let subjects: BTreeMap<&str, TermId> = documents
            .iter()
            .map(|(path, document)| (path.as_str(), document.subject))
            .collect();
        let mut links = Vec::new();
        for (document_index, (path, document)) in documents.iter().enumerate() {
            for link in extract_markdown_links(&document.body, path)? {
                let Some(target_path) = resolve_link_path(path, &link.target)? else {
                    continue;
                };
                let Some(&target) = subjects.get(target_path.as_str()) else {
                    return Err(OkfError::new(format!(
                        "{path}: dangling Markdown link target `{}`",
                        link.target
                    )));
                };
                links.push(ExpectedLink {
                    source: document.subject,
                    target,
                    document_index,
                    ordinal: link.ordinal,
                    text: link.text,
                });
            }
        }
        Ok(links)
    }

    fn reconcile_link_quads(
        &mut self,
        link_quads: &[(usize, QuadIds)],
        expected_links: &[ExpectedLink],
    ) -> Result<(), OkfError> {
        let expected_pairs: BTreeSet<(TermId, TermId)> = expected_links
            .iter()
            .map(|link| (link.source, link.target))
            .collect();
        let mut actual_pairs = BTreeSet::new();
        for &(ordinal, quad) in link_quads {
            let pair = (quad.s, quad.o);
            if expected_pairs.contains(&pair) && actual_pairs.insert(pair) {
                continue;
            }
            record_quad_loss(
                &mut self.losses,
                &self.terms,
                ordinal,
                quad.s,
                "okf-non-profile-quad-dropped",
                "OKF link edge is not backed by exactly one relative Markdown-link target.",
            );
        }
        let missing: Vec<String> = expected_pairs
            .difference(&actual_pairs)
            .map(|(source, target)| {
                format!(
                    "{} -> {}",
                    self.term_label(*source),
                    self.term_label(*target)
                )
            })
            .collect();
        if !missing.is_empty() {
            return Err(OkfError::new(format!(
                "Markdown body link(s) lack configured OKF link edge(s): {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }

    fn reconcile_statement_layer(
        &mut self,
        expected_links: &[ExpectedLink],
    ) -> Result<(), OkfError> {
        let expected_by_label: BTreeMap<String, &ExpectedLink> = expected_links
            .iter()
            .map(|link| (link.reifier_label(), link))
            .collect();
        let mut annotations_by_reifier: BTreeMap<TermId, Vec<AnnotationEvent>> = BTreeMap::new();
        for &annotation in &self.annotations {
            annotations_by_reifier
                .entry(annotation.reifier)
                .or_default()
                .push(annotation);
        }

        let mut matched_labels = BTreeSet::new();
        let mut consumed_annotations = BTreeSet::new();
        for reifier in &self.reifiers {
            self.require_term(reifier.reifier)?;
            self.require_term(reifier.triple)?;
            if let Some(graph) = reifier.graph {
                self.require_term(graph)?;
            }
            let exact = self.match_link_reifier(
                reifier,
                &expected_by_label,
                &annotations_by_reifier,
                &matched_labels,
            )?;
            if let Some((label, annotation_ordinals)) = exact {
                matched_labels.insert(label);
                consumed_annotations.extend(annotation_ordinals);
            } else {
                record_reifier_loss(
                    &mut self.losses,
                    &self.terms,
                    reifier.ordinal,
                    reifier.reifier,
                );
            }
        }

        let missing: Vec<&str> = expected_by_label
            .keys()
            .map(String::as_str)
            .filter(|label| !matched_labels.contains(*label))
            .collect();
        if !missing.is_empty() {
            return Err(OkfError::new(format!(
                "Markdown link occurrence(s) lack exact OKF reifier metadata: {}",
                missing.join(", ")
            )));
        }

        for annotation in &self.annotations {
            self.require_term(annotation.reifier)?;
            self.require_term(annotation.predicate)?;
            self.require_term(annotation.object)?;
            if let Some(graph) = annotation.graph {
                self.require_term(graph)?;
            }
            if !consumed_annotations.contains(&annotation.ordinal) {
                record_annotation_loss(
                    &mut self.losses,
                    &self.terms,
                    annotation.ordinal,
                    annotation.reifier,
                );
            }
        }
        Ok(())
    }

    fn match_link_reifier(
        &self,
        reifier: &ReifierEvent,
        expected_by_label: &BTreeMap<String, &ExpectedLink>,
        annotations_by_reifier: &BTreeMap<TermId, Vec<AnnotationEvent>>,
        matched_labels: &BTreeSet<String>,
    ) -> Result<Option<(String, Vec<usize>)>, OkfError> {
        if reifier.graph.is_some() {
            return Ok(None);
        }
        let OwnedTerm::Blank { label, scope } = self.term(reifier.reifier)? else {
            return Ok(None);
        };
        if *scope != BlankScope::DEFAULT || matched_labels.contains(label) {
            return Ok(None);
        }
        let Some(expected) = expected_by_label.get(label) else {
            return Ok(None);
        };
        let OwnedTerm::Triple { s, p, o } = self.term(reifier.triple)? else {
            return Err(OkfError::new(format!(
                "OKF reifier {} does not bind a triple term",
                self.term_label(reifier.reifier)
            )));
        };
        if *s != expected.source
            || *o != expected.target
            || self.iri(*p)? != self.config.links_predicate()
        {
            return Ok(None);
        }
        let Some(annotations) = annotations_by_reifier.get(&reifier.reifier) else {
            return Ok(None);
        };
        let mut text_ordinal = None;
        let mut occurrence_ordinal = None;
        for annotation in annotations {
            if annotation.graph.is_some() {
                continue;
            }
            let Ok(predicate) = self.iri(annotation.predicate) else {
                continue;
            };
            if predicate == self.config.link_text_predicate() {
                if text_ordinal.is_none()
                    && self.plain_literal_opt(annotation.object, XSD_STRING)
                        == Some(expected.text.as_str())
                {
                    text_ordinal = Some(annotation.ordinal);
                }
            } else if predicate == self.config.link_occurrence_predicate() {
                let occurrence = (expected.ordinal + 1).to_string();
                if occurrence_ordinal.is_none()
                    && self.plain_literal_opt(annotation.object, XSD_INTEGER)
                        == Some(occurrence.as_str())
                {
                    occurrence_ordinal = Some(annotation.ordinal);
                }
            }
        }
        Ok(text_ordinal.zip(occurrence_ordinal).map(|ordinals| {
            let (text, occurrence) = ordinals;
            (label.clone(), vec![text, occurrence])
        }))
    }

    fn render_bundle(
        &self,
        documents: BTreeMap<String, FinalDocument>,
    ) -> Result<OkfBundle, OkfError> {
        let mut bundle = OkfBundle::new();
        for (path, document) in documents {
            validate_yaml_fields(&document.fields, &path)?;
            let yaml = serde_yaml::to_string(&document.fields).map_err(|error| {
                OkfError::new(format!("cannot serialize `{path}` frontmatter: {error}"))
            })?;
            if yaml.len() > super::MAX_OKF_FRONTMATTER_BYTES {
                return Err(OkfError::new(format!(
                    "`{path}` YAML frontmatter is {} bytes; limit is {}",
                    yaml.len(),
                    super::MAX_OKF_FRONTMATTER_BYTES
                )));
            }
            if yaml.starts_with("---") || yaml.trim_end().ends_with("...") {
                return Err(OkfError::new(format!(
                    "YAML serializer emitted a document marker for `{path}`"
                )));
            }
            bundle.insert(path, format!("---\n{yaml}---\n{}", document.body))?;
        }
        Ok(bundle)
    }

    fn extension_value(&self, id: TermId) -> Result<YamlValue, OkfError> {
        let OwnedTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = self.term(id)?
        else {
            return Err(OkfError::new(format!(
                "OKF extension object term#{} must be a literal",
                id.index()
            )));
        };
        if language.is_some() || direction.is_some() {
            return Err(OkfError::new(format!(
                "OKF extension literal term#{} cannot carry language or base direction",
                id.index()
            )));
        }
        let datatype = self.iri(*datatype)?;
        match datatype {
            XSD_STRING => Ok(YamlValue::String(lexical.clone())),
            XSD_BOOLEAN => {
                let parsed = canonical_xsd(lexical, datatype)?;
                Ok(YamlValue::Bool(parsed == "true"))
            }
            XSD_INTEGER => {
                let parsed = canonical_xsd(lexical, datatype)?;
                if let Ok(value) = parsed.parse::<i64>() {
                    serde_yaml::to_value(value).map_err(|error| yaml_value_error(&error))
                } else if let Ok(value) = parsed.parse::<u64>() {
                    serde_yaml::to_value(value).map_err(|error| yaml_value_error(&error))
                } else {
                    Err(OkfError::new(format!(
                        "OKF integer `{parsed}` exceeds YAML's exact 64-bit numeric domain"
                    )))
                }
            }
            XSD_DECIMAL => {
                let parsed = canonical_xsd(lexical, datatype)?;
                let value = parsed.parse::<f64>().map_err(|error| {
                    OkfError::new(format!(
                        "OKF decimal `{parsed}` is not YAML-representable: {error}"
                    ))
                })?;
                if !value.is_finite() {
                    return Err(OkfError::new(format!(
                        "OKF decimal `{parsed}` is not finite"
                    )));
                }
                let yaml_lexical = decimal_lexical_from_f64(value)?;
                let round_trip = parse_known_xsd(&yaml_lexical, XSD_DECIMAL).map_err(|error| {
                    OkfError::new(format!(
                        "OKF decimal `{parsed}` cannot round-trip through YAML: {error}"
                    ))
                })?;
                let original = parse_known_xsd(&parsed, XSD_DECIMAL)?;
                if !purrdf_xsd::value_eq(&original, &round_trip) {
                    return Err(OkfError::new(format!(
                        "OKF decimal `{parsed}` would lose precision in YAML"
                    )));
                }
                serde_yaml::to_value(value).map_err(|error| yaml_value_error(&error))
            }
            datatype if datatype == self.config.json_datatype() => {
                let json: serde_json::Value = serde_json::from_str(lexical).map_err(|error| {
                    OkfError::new(format!("invalid OKF JSON literal `{lexical}`: {error}"))
                })?;
                let canonical = serde_json::to_string(&json).map_err(|error| {
                    OkfError::new(format!("cannot canonicalize OKF JSON: {error}"))
                })?;
                if canonical != lexical.as_str() {
                    return Err(OkfError::new(format!(
                        "OKF JSON literal must be canonical; expected `{canonical}`"
                    )));
                }
                serde_yaml::to_value(json).map_err(|error| yaml_value_error(&error))
            }
            other => Err(OkfError::new(format!(
                "OKF extension literal datatype `{other}` is not representable"
            ))),
        }
    }

    fn plain_literal(
        &self,
        id: TermId,
        expected_datatype: &str,
        label: &str,
    ) -> Result<&str, OkfError> {
        let OwnedTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = self.term(id)?
        else {
            return Err(OkfError::new(format!(
                "{label} object term#{} must be a literal",
                id.index()
            )));
        };
        if language.is_some() || direction.is_some() || self.iri(*datatype)? != expected_datatype {
            return Err(OkfError::new(format!(
                "{label} term#{} must be an undirected, language-free `{expected_datatype}` literal",
                id.index()
            )));
        }
        Ok(lexical)
    }

    fn plain_literal_opt(&self, id: TermId, expected_datatype: &str) -> Option<&str> {
        let OwnedTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = self.term(id).ok()?
        else {
            return None;
        };
        (language.is_none()
            && direction.is_none()
            && self.iri(*datatype).ok()? == expected_datatype)
            .then_some(lexical.as_str())
    }

    fn term(&self, id: TermId) -> Result<&OwnedTerm, OkfError> {
        self.terms.get(id.index()).ok_or_else(|| {
            OkfError::new(format!(
                "OKF writer event references undeclared term index {}",
                id.index()
            ))
        })
    }

    fn require_term(&self, id: TermId) -> Result<(), OkfError> {
        self.term(id).map(|_| ())
    }

    fn iri(&self, id: TermId) -> Result<&str, OkfError> {
        let OwnedTerm::Iri(iri) = self.term(id)? else {
            return Err(OkfError::new(format!(
                "OKF writer expected term#{} to be an IRI",
                id.index()
            )));
        };
        Ok(iri)
    }

    fn term_label(&self, id: TermId) -> String {
        match self.terms.get(id.index()) {
            Some(OwnedTerm::Iri(iri)) => iri.clone(),
            Some(OwnedTerm::Blank { label, scope }) => {
                format!("_:{}@{}", label, scope.ordinal())
            }
            _ => format!("term#{}", id.index()),
        }
    }
}

fn canonical_xsd(lexical: &str, datatype: &str) -> Result<String, OkfError> {
    let value = parse_known_xsd(lexical, datatype)?;
    let canonical = value.canonical_lexical();
    if canonical != lexical {
        return Err(OkfError::new(format!(
            "OKF numeric/boolean literal `{lexical}` must use canonical `{canonical}`"
        )));
    }
    Ok(canonical)
}

fn parse_known_xsd(lexical: &str, datatype: &str) -> Result<purrdf_xsd::XsdValue, OkfError> {
    purrdf_xsd::parse_by_iri(lexical, datatype)
        .map_err(|error| {
            OkfError::new(format!("invalid `{datatype}` literal `{lexical}`: {error}"))
        })?
        .ok_or_else(|| OkfError::new(format!("unrecognized internal XSD datatype `{datatype}`")))
}

fn yaml_value_error(error: &serde_yaml::Error) -> OkfError {
    OkfError::new(format!("cannot encode OKF YAML value: {error}"))
}

fn validate_yaml_fields(fields: &BTreeMap<String, YamlValue>, path: &str) -> Result<(), OkfError> {
    fn visit(
        value: &YamlValue,
        depth: usize,
        nodes: &mut usize,
        path: &str,
    ) -> Result<(), OkfError> {
        if depth > super::MAX_OKF_YAML_DEPTH {
            return Err(OkfError::new(format!(
                "`{path}` YAML nesting exceeds depth {}",
                super::MAX_OKF_YAML_DEPTH
            )));
        }
        *nodes = nodes
            .checked_add(1)
            .ok_or_else(|| OkfError::new(format!("`{path}` YAML node count overflow")))?;
        if *nodes > super::MAX_OKF_YAML_NODES {
            return Err(OkfError::new(format!(
                "`{path}` YAML tree exceeds {} nodes",
                super::MAX_OKF_YAML_NODES
            )));
        }
        match value {
            YamlValue::Sequence(values) => {
                for child in values {
                    visit(child, depth + 1, nodes, path)?;
                }
            }
            YamlValue::Mapping(values) => {
                for child in values.values() {
                    visit(child, depth + 1, nodes, path)?;
                }
            }
            YamlValue::Tagged(_) => {
                return Err(OkfError::new(format!(
                    "`{path}` cannot represent a tagged YAML value"
                )));
            }
            YamlValue::Null | YamlValue::Bool(_) | YamlValue::Number(_) | YamlValue::String(_) => {}
        }
        Ok(())
    }

    let mut nodes = 1;
    for value in fields.values() {
        visit(value, 1, &mut nodes, path)?;
    }
    Ok(())
}

fn term_subject(terms: &[OwnedTerm], subject: TermId) -> String {
    match terms.get(subject.index()) {
        Some(OwnedTerm::Iri(iri)) => iri.clone(),
        Some(OwnedTerm::Blank { label, scope }) => format!("_:{}@{}", label, scope.ordinal()),
        _ => format!("term#{}", subject.index()),
    }
}

fn record_quad_loss(
    ledger: &mut LossLedger,
    terms: &[OwnedTerm],
    ordinal: usize,
    subject: TermId,
    code: &'static str,
    note: &'static str,
) {
    ledger.record(LossEntry {
        code: Cow::Borrowed(code),
        from: Cow::Borrowed("rdf-1.2-dataset"),
        to: Cow::Borrowed("okf"),
        note: Cow::Borrowed(note),
        location: Some(Box::new(
            RdfLocation::logical(format!("okf-writer:quad:{ordinal}"))
                .with_subject(term_subject(terms, subject)),
        )),
    });
}

fn record_reifier_loss(
    ledger: &mut LossLedger,
    terms: &[OwnedTerm],
    ordinal: usize,
    reifier: TermId,
) {
    ledger.record(LossEntry {
        code: Cow::Borrowed("okf-reifier-dropped"),
        from: Cow::Borrowed("rdf-1.2-dataset"),
        to: Cow::Borrowed("okf"),
        note: Cow::Borrowed("RDF 1.2 reifier is outside the exact OKF Markdown-link profile."),
        location: Some(Box::new(
            RdfLocation::logical(format!("okf-writer:reifier:{ordinal}"))
                .with_subject(term_subject(terms, reifier)),
        )),
    });
}

fn record_annotation_loss(
    ledger: &mut LossLedger,
    terms: &[OwnedTerm],
    ordinal: usize,
    reifier: TermId,
) {
    ledger.record(LossEntry {
        code: Cow::Borrowed("okf-annotation-dropped"),
        from: Cow::Borrowed("rdf-1.2-dataset"),
        to: Cow::Borrowed("okf"),
        note: Cow::Borrowed("RDF 1.2 annotation is outside the exact OKF Markdown-link profile."),
        location: Some(Box::new(
            RdfLocation::logical(format!("okf-writer:annotation:{ordinal}"))
                .with_subject(term_subject(terms, reifier)),
        )),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DatasetMut, DatasetSink, MutableDataset, QuadValues, RdfDatasetBuilder, RdfLiteral,
        TermValue, assert_ledger_complete, assert_ledger_sound, datasets_isomorphic,
    };

    fn config() -> OkfConfig {
        OkfConfig::new(
            "https://example.org/okf#",
            "https://example.org/doc/",
            [
                "type",
                "title",
                "description",
                "resource",
                "tags",
                "timestamp",
                "active",
                "count",
                "ratio",
                "small",
                "large",
                "producer",
            ],
        )
        .expect("valid config")
    }

    fn source_bundle() -> OkfBundle {
        OkfBundle::from_documents([
            (
                "concepts/schema.md",
                "---\ntype: Schema\ntitle: Event Schema\ndescription: Stable columns.\n---\nColumns: id.\n",
            ),
            (
                "concepts/table.md",
                "---\ntype: Table\ntitle: Events\nresource: https://example.org/data/events\ntags:\n- stable\n- analytics\ntimestamp: 2026-07-15T01:02:03Z\nactive: true\ncount: 7\nratio: 0.625\nsmall: 1e-5\nlarge: 1e20\nproducer:\n  name: fixture\n  ranks: [1, 2]\n---\nSee [Schema](schema.md) and [Schema again](schema.md#columns). External [site](https://example.net/).\n",
            ),
        ])
        .expect("valid bundle")
    }

    fn lift(bundle: &OkfBundle, config: &OkfConfig) -> std::sync::Arc<RdfDataset> {
        let mut sink = DatasetSink::new();
        let outcome = crate::native_codecs::okf::lift_okf_bundle(bundle, config, &mut sink)
            .expect("lift bundle");
        assert!(outcome.losses.is_empty());
        assert!(!outcome.cancelled);
        sink.into_dataset().expect("finished dataset sink")
    }

    #[test]
    fn writer_is_deterministic_and_write_read_write_stable() {
        let config = config();
        let source = lift(&source_bundle(), &config);

        let first = write_okf_bundle(&source, &config).expect("first write");
        let repeated = write_okf_bundle(&source, &config).expect("repeated write");
        assert_eq!(first, repeated, "same dataset must emit byte-identically");
        assert_eq!(first.documents, 2);
        assert!(first.losses.is_empty());

        let reparsed = lift(&first.bundle, &config);
        assert!(
            datasets_isomorphic(&source, &reparsed),
            "the complete OKF profile must round-trip isomorphically"
        );
        let second = write_okf_bundle(&reparsed, &config).expect("second write");
        assert_eq!(
            first.bundle, second.bundle,
            "write-read-write must stabilize"
        );
        assert!(second.losses.is_empty());

        let table = first
            .bundle
            .get("concepts/table.md")
            .expect("table document");
        assert!(table.contains("resource: https://example.org/data/events"));
        assert!(table.contains("ratio: 0.625"));
        assert!(table.contains("small: 0.00001"));
        assert!(table.contains("large: 1e20"));
        assert!(table.contains("- analytics\n- stable"));
    }

    #[test]
    fn exact_link_metadata_survives_while_extra_annotation_is_ledgered() {
        let config = config();
        let base = lift(&source_bundle(), &config);
        let mut mutable = MutableDataset::new(base.clone());
        assert!(mutable.insert(QuadValues::triple(
            TermValue::blank("okf_link_1_0"),
            TermValue::iri("https://example.org/meta#reviewer"),
            TermValue::simple_literal("Ada"),
        )));
        let extended = mutable.freeze().expect("freeze extended dataset");

        let outcome = write_okf_bundle(&extended, &config).expect("write extended dataset");
        assert_ledger_complete(&outcome.losses, &["okf-annotation-dropped"]);
        assert_ledger_sound(&outcome.losses, "rdf-1.2-dataset", "okf");

        let reparsed = lift(&outcome.bundle, &config);
        assert!(
            datasets_isomorphic(&base, &reparsed),
            "only the explicitly ledgered extra annotation may disappear"
        );
    }

    #[test]
    fn every_rdf_to_okf_loss_class_is_recorded_with_location() {
        let config = config();
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/doc/concept.md");
        let path_predicate = builder.intern_iri(config.path_predicate());
        let body_predicate = builder.intern_iri(config.body_predicate());
        let type_predicate = builder.intern_iri(config.predicate_iri("type").expect("type IRI"));
        let path = builder.intern_literal(RdfLiteral::simple("concept.md"));
        let body = builder.intern_literal(RdfLiteral::simple("Body.\n"));
        let kind = builder.intern_literal(RdfLiteral::simple("Concept"));
        builder.push_quad(subject, path_predicate, path, None);
        builder.push_quad(subject, body_predicate, body, None);
        builder.push_quad(subject, type_predicate, kind, None);

        let owl_equivalent = builder.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
        let other = builder.intern_iri("https://example.org/Other");
        let graph = builder.intern_iri("https://example.org/graph");
        builder.push_quad(subject, owl_equivalent, other, None);
        builder.push_quad(subject, owl_equivalent, other, Some(graph));

        let triple = builder.intern_triple(subject, owl_equivalent, other);
        let reifier = builder.intern_blank("unrelated", BlankScope::DEFAULT);
        builder.push_reifier(reifier, triple);
        let note_predicate = builder.intern_iri("https://example.org/note");
        let note = builder.intern_literal(RdfLiteral::simple("not OKF metadata"));
        builder.push_annotation(reifier, note_predicate, note);

        let dataset = builder.freeze().expect("valid dataset");
        let outcome = write_okf_bundle(&dataset, &config).expect("lossy write");
        assert_ledger_complete(
            &outcome.losses,
            &[
                "named-graph-dropped",
                "okf-annotation-dropped",
                "okf-non-profile-quad-dropped",
                "okf-reifier-dropped",
            ],
        );
        assert_ledger_sound(&outcome.losses, "rdf-1.2-dataset", "okf");
        assert!(
            outcome
                .losses
                .entries()
                .iter()
                .all(|entry| entry.location.is_some()),
            "every runtime loss must identify its source row"
        );
        assert_eq!(outcome.documents, 1);
        let reparsed = lift(&outcome.bundle, &config);
        assert_eq!(reparsed.quad_count(), 3);
    }

    fn profile_dataset(types: &[&str]) -> std::sync::Arc<RdfDataset> {
        let config = config();
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/doc/concept.md");
        let path_predicate = builder.intern_iri(config.path_predicate());
        let body_predicate = builder.intern_iri(config.body_predicate());
        let type_predicate = builder.intern_iri(config.predicate_iri("type").expect("type IRI"));
        let path = builder.intern_literal(RdfLiteral::simple("concept.md"));
        let body = builder.intern_literal(RdfLiteral::simple("Body.\n"));
        builder.push_quad(subject, path_predicate, path, None);
        builder.push_quad(subject, body_predicate, body, None);
        for value in types {
            let value = builder.intern_literal(RdfLiteral::simple(*value));
            builder.push_quad(subject, type_predicate, value, None);
        }
        builder.freeze().expect("valid dataset")
    }

    #[test]
    fn ambiguous_and_empty_required_profile_values_hard_fail() {
        let config = config();
        let ambiguous = write_okf_bundle(&profile_dataset(&["A", "B"]), &config)
            .expect_err("ambiguous type must fail");
        assert!(ambiguous.detail().contains("more than one `type` value"));

        let empty =
            write_okf_bundle(&profile_dataset(&[""]), &config).expect_err("empty type must fail");
        assert!(empty.detail().contains("empty required `type`"));
    }
}
