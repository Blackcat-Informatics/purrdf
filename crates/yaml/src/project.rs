// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::{
    Ceiling, Document, Profile, SourceDocument, YamlError, admit_count, analyze,
    profile::{SPECIFICATION, TERMS},
};
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, TermId, TermRef,
    datatype::{XSD_INTEGER, XSD_STRING},
};
use purrdf_lex::{
    allocation::{Memory, OrderedTable, StorageError},
    yaml::{
        Attachment, Chomping, CollectionStyle, ExplicitTag, NodeId, NodeKind, ScalarStyle, Span,
        TriviaKind,
    },
};

#[derive(Clone, Copy)]
enum Object<'a> {
    Id(TermId),
    Iri(&'a str),
    Text(&'a str),
    Typed(&'a str, &'a str),
}
struct Emitter<'p, 'm, 's> {
    profile: &'p Profile,
    memory: &'m mut Memory<'s, Ceiling>,
    builder: RdfDatasetBuilder,
    predicates: Vec<TermId>,
    rdf_type: TermId,
    root: TermId,
    nodes: OrderedTable<NodeId, TermId>,
    statements: usize,
    payload: usize,
}
impl<'p, 'm, 's> Emitter<'p, 'm, 's> {
    fn new(
        profile: &'p Profile,
        root: &str,
        memory: &'m mut Memory<'s, Ceiling>,
    ) -> Result<Self, YamlError> {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri_with_memory(purrdf_core::vocab::rdf::TYPE, memory)?;
        let root = builder.intern_iri_with_memory(root, memory)?;
        let mut predicates = Vec::new();
        for local in TERMS {
            let id = builder.intern_iri_with_memory(profile.vocabulary().term(local)?, memory)?;
            memory.push(&mut predicates, id)?;
        }
        Ok(Self {
            profile,
            memory,
            builder,
            predicates,
            rdf_type,
            root,
            nodes: OrderedTable::default(),
            statements: 0,
            payload: 0,
        })
    }
    fn term(&self, local: &str) -> Result<TermId, YamlError> {
        TERMS
            .iter()
            .position(|&term| term == local)
            .map(|index| self.predicates[index])
            .ok_or(YamlError::Profile("unknown emitted vocabulary term"))
    }
    fn node(&self, id: NodeId) -> Result<TermId, YamlError> {
        self.nodes
            .get(&id)
            .copied()
            .ok_or(YamlError::Metadata("native node target is absent"))
    }
    fn subject(
        &mut self,
        root: &str,
        suffix: impl core::fmt::Display,
    ) -> Result<TermId, YamlError> {
        let text = self.memory.format(&format_args!("{root}#{suffix}"))?;
        let id = self.builder.intern_iri_with_memory(&text, self.memory)?;
        self.memory.release_string(text)?;
        Ok(id)
    }
    fn fact(&mut self, subject: TermId, local: &str, object: Object<'_>) -> Result<(), YamlError> {
        let bounds = self.profile.bounds();
        let statements = self
            .statements
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        admit_count(statements, bounds.max_statements, "RDF statements")?;
        let predicate = if local == "type" {
            self.rdf_type
        } else {
            self.term(local)?
        };
        let object_bytes = match object {
            Object::Id(id) => iri_bytes(self.builder.resolve(id))?,
            Object::Iri(text) => text.len(),
            Object::Text(text) => text
                .len()
                .checked_add(XSD_STRING.len())
                .ok_or(StorageError::SizeOverflow)?,
            Object::Typed(text, datatype) => text
                .len()
                .checked_add(datatype.len())
                .ok_or(StorageError::SizeOverflow)?,
        };
        let subject_bytes = iri_bytes(self.builder.resolve(subject))?;
        let predicate_bytes = iri_bytes(self.builder.resolve(predicate))?;
        let payload = self
            .payload
            .checked_add(subject_bytes)
            .and_then(|bytes| bytes.checked_add(predicate_bytes))
            .and_then(|bytes| bytes.checked_add(object_bytes))
            .ok_or(StorageError::SizeOverflow)?;
        admit_count(payload, bounds.max_payload_bytes, "metadata payload bytes")?;
        let object = match object {
            Object::Id(id) => id,
            Object::Iri(text) => self.builder.intern_iri_with_memory(text, self.memory)?,
            Object::Text(text) => self.builder.intern_literal_parts_with_memory(
                text,
                None,
                None,
                None,
                self.memory,
            )?,
            Object::Typed(text, datatype) => self.builder.intern_literal_parts_with_memory(
                text,
                Some(datatype),
                None,
                None,
                self.memory,
            )?,
        };
        self.builder
            .push_quad_with_memory(subject, predicate, object, None, self.memory)?;
        self.statements = statements;
        self.payload = payload;
        Ok(())
    }
    fn number(&mut self, subject: TermId, local: &str, number: usize) -> Result<(), YamlError> {
        let text = self.memory.format(&number)?;
        self.fact(subject, local, Object::Typed(&text, XSD_INTEGER))?;
        self.memory.release_string(text)?;
        Ok(())
    }
    fn span(
        &mut self,
        subject: TermId,
        start: &str,
        end: &str,
        span: Span,
    ) -> Result<(), YamlError> {
        self.number(subject, start, span.start)?;
        self.number(subject, end, span.end)
    }
    fn describe(
        &mut self,
        subject: TermId,
        class: &str,
        ordinal: usize,
        span: Span,
    ) -> Result<(), YamlError> {
        self.fact(subject, "type", Object::Id(self.term(class)?))?;
        self.fact(subject, "stream", Object::Id(self.root))?;
        self.number(subject, "ordinal", ordinal)?;
        self.span(subject, "byteStart", "byteEnd", span)
    }
    fn finish(self) -> Result<RdfDataset, YamlError> {
        self.nodes.release(self.memory)?;
        self.memory.release_vec(self.predicates)?;
        Ok(self.builder.freeze_with_memory(self.memory)?)
    }
}

/// Count borrowed term components under the metadata payload law.
pub(crate) fn term_bytes(dataset: &RdfDataset, id: TermId) -> Result<usize, YamlError> {
    match dataset.resolve(id) {
        TermRef::Iri(text) => Ok(text.len()),
        TermRef::Literal {
            lexical,
            datatype,
            language: None,
            direction: None,
        } => Ok(lexical
            .len()
            .checked_add(iri_bytes(dataset.resolve(datatype))?)
            .ok_or(StorageError::SizeOverflow)?),
        _ => Err(YamlError::Metadata(
            "metadata terms must be IRIs or undirected literals",
        )),
    }
}
fn iri_bytes(term: TermRef<'_>) -> Result<usize, YamlError> {
    match term {
        TermRef::Iri(text) => Ok(text.len()),
        _ => Err(YamlError::Metadata("metadata IRI invariant")),
    }
}

/// Analyze and project one source stream under the explicit profile.
/// # Errors
/// Returns original grammar, storage, profile or RDF errors, never partial RDF.
pub fn encode(source: SourceDocument<'_>, profile: &Profile) -> Result<RdfDataset, YamlError> {
    project(&analyze(source, profile)?, profile)
}

/// Deterministic metadata and byte cover; no semantic YAML construction runs.
/// Projection admits additional builder/index/freeze storage while the model is alive.
/// # Errors
/// Returns typed profile, storage or RDF failures before publication.
pub fn project(document: &Document<'_>, profile: &Profile) -> Result<RdfDataset, YamlError> {
    if document.profile != profile.identity() {
        return Err(YamlError::Metadata("model/profile identity mismatch"));
    }
    let mut ceiling = Ceiling(profile.bounds().max_projection_bytes);
    let mut memory = Memory::new(&mut ceiling);
    let mut emit = Emitter::new(profile, document.id(), &mut memory)?;
    let root = emit.root;
    emit.fact(root, "type", Object::Id(emit.term("Stream")?))?;
    emit.fact(root, "source", Object::Iri(document.source.id))?;
    emit.fact(root, "vocabulary", Object::Iri(profile.vocabulary().base()))?;
    emit.fact(root, "specification", Object::Text(SPECIFICATION))?;
    emit.fact(root, "profileName", Object::Text(profile.name()))?;
    emit.number(root, "profileVersion", profile.version() as usize)?;
    emit.number(root, "byteLength", document.source.bytes.len())?;
    for (local, datatype, value) in [
        (
            "sourceDigest",
            "sha256Digest",
            emit.memory
                .format(&format_args!("sha256:{}", document.source_digest()))?,
        ),
        (
            "profile",
            "sha256Digest",
            emit.memory
                .format(&format_args!("sha256:{}", profile.identity()))?,
        ),
        (
            "lexBlake3",
            "blake3Digest",
            emit.memory
                .format(&format_args!("blake3:{}", document.lex_source_id()))?,
        ),
    ] {
        emit.fact(
            root,
            local,
            Object::Typed(&value, profile.vocabulary().term(datatype)?),
        )?;
        emit.memory.release_string(value)?;
    }
    for (ordinal, (id, _)) in document.stream.nodes().enumerate() {
        let subject = emit.subject(document.id(), format_args!("n{ordinal}"))?;
        emit.nodes.insert(id, subject, emit.memory)?;
    }
    for (ordinal, doc) in document.stream.documents().iter().enumerate() {
        let subject = emit.subject(document.id(), format_args!("d{ordinal}"))?;
        emit.describe(subject, "Document", ordinal, doc.span)?;
        emit.fact(root, "document", Object::Id(subject))?;
        emit.fact(subject, "root", Object::Id(emit.node(doc.root)?))?;
        emit.span(subject, "prologueStart", "prologueEnd", doc.prologue)?;
        if let Some(span) = doc.start_marker {
            emit.span(subject, "startMarkerStart", "startMarkerEnd", span)?;
        }
        if let Some(span) = doc.end_marker {
            emit.span(subject, "endMarkerStart", "endMarkerEnd", span)?;
        }
    }
    for (ordinal, (id, node)) in document.stream.nodes().enumerate() {
        let subject = emit.node(id)?;
        emit.describe(subject, "Node", ordinal, node.span)?;
        if let Some(span) = node.anchor {
            emit.span(subject, "anchorStart", "anchorEnd", span)?;
            emit.fact(subject, "anchorText", Object::Text(raw(document, span)?))?;
        }
        if let Some(span) = node.tag {
            emit.span(subject, "tagStart", "tagEnd", span)?;
            emit.fact(subject, "tagText", Object::Text(raw(document, span)?))?;
        }
        match &node.explicit_tag {
            Some(ExplicitTag::NonSpecific) => {
                emit.fact(subject, "tagKind", Object::Text("non-specific"))?;
            }
            Some(ExplicitTag::Specific(name)) => {
                emit.fact(subject, "tagKind", Object::Text("specific"))?;
                emit.fact(subject, "tagName", Object::Text(name))?;
            }
            None => {}
        }
        if let Some(span) = node.explicit_key {
            emit.span(subject, "explicitKeyStart", "explicitKeyEnd", span)?;
        }
        match &node.kind {
            NodeKind::Scalar(scalar) => {
                emit.fact(subject, "kind", Object::Text("scalar"))?;
                emit.fact(subject, "text", Object::Text(&scalar.text))?;
                emit.fact(subject, "style", Object::Text(scalar_style(scalar.style)))?;
                emit.span(subject, "rawStart", "rawEnd", scalar.raw)?;
                emit.fact(subject, "rawText", Object::Text(raw(document, scalar.raw)?))?;
                if let Some(block) = scalar.block {
                    emit.span(subject, "blockStart", "blockEnd", block.span)?;
                    emit.fact(
                        subject,
                        "chomping",
                        Object::Text(match block.chomping {
                            Chomping::Clip => "clip",
                            Chomping::Strip => "strip",
                            Chomping::Keep => "keep",
                        }),
                    )?;
                    if let Some(indent) = block.indent_indicator {
                        emit.number(subject, "indentIndicator", indent)?;
                    }
                    emit.number(subject, "contentIndent", block.content_indent)?;
                }
            }
            NodeKind::Alias { raw: span, target } => {
                emit.fact(subject, "kind", Object::Text("alias"))?;
                emit.fact(subject, "target", Object::Id(emit.node(*target)?))?;
                emit.span(subject, "rawStart", "rawEnd", *span)?;
                emit.fact(subject, "rawText", Object::Text(raw(document, *span)?))?;
            }
            NodeKind::Sequence { items, style } => {
                emit.fact(subject, "kind", Object::Text("sequence"))?;
                emit.fact(subject, "style", Object::Text(collection_style(*style)))?;
                for (position, &id) in items.iter().enumerate() {
                    let item =
                        emit.subject(document.id(), format_args!("i{ordinal}-{position}"))?;
                    emit.describe(
                        item,
                        "Item",
                        position,
                        document
                            .stream
                            .node(id)
                            .ok_or(YamlError::Metadata("native item target"))?
                            .span,
                    )?;
                    emit.fact(subject, "item", Object::Id(item))?;
                    emit.fact(item, "parent", Object::Id(subject))?;
                    emit.fact(item, "node", Object::Id(emit.node(id)?))?;
                }
            }
            NodeKind::Mapping { entries, style } => {
                emit.fact(subject, "kind", Object::Text("mapping"))?;
                emit.fact(subject, "style", Object::Text(collection_style(*style)))?;
                for (position, entry) in entries.iter().enumerate() {
                    let item =
                        emit.subject(document.id(), format_args!("e{ordinal}-{position}"))?;
                    let key = document
                        .stream
                        .node(entry.key)
                        .ok_or(YamlError::Metadata("native key target"))?;
                    let value = document
                        .stream
                        .node(entry.value)
                        .ok_or(YamlError::Metadata("native value target"))?;
                    emit.describe(
                        item,
                        "Entry",
                        position,
                        Span {
                            start: key.span.start,
                            end: value.span.end.max(key.span.end),
                        },
                    )?;
                    emit.fact(subject, "entry", Object::Id(item))?;
                    emit.fact(item, "parent", Object::Id(subject))?;
                    emit.fact(item, "key", Object::Id(emit.node(entry.key)?))?;
                    emit.fact(item, "value", Object::Id(emit.node(entry.value)?))?;
                }
            }
        }
    }
    for (ordinal, trivia) in document.stream.trivia().iter().enumerate() {
        let subject = emit.subject(document.id(), format_args!("t{ordinal}"))?;
        emit.describe(subject, "Trivia", ordinal, trivia.span)?;
        emit.fact(
            subject,
            "rawText",
            Object::Text(raw(document, trivia.span)?),
        )?;
        emit.fact(
            subject,
            "kind",
            Object::Text(match trivia.kind {
                TriviaKind::Whitespace => "whitespace",
                TriviaKind::LineBreak => "line-break",
                TriviaKind::BlankLine => "blank-line",
                TriviaKind::Comment => "comment",
                TriviaKind::BlockHeaderComment => "block-header-comment",
            }),
        )?;
        let (kind, node) = match trivia.attachment {
            Attachment::Leading(id) => ("leading", Some(id)),
            Attachment::Trailing(id) => ("trailing", Some(id)),
            Attachment::Document => ("document", None),
        };
        emit.fact(subject, "attachment", Object::Text(kind))?;
        if let Some(id) = node {
            emit.fact(subject, "attachmentNode", Object::Id(emit.node(id)?))?;
        }
    }
    for (ordinal, &span) in document.stream.directives().iter().enumerate() {
        let subject = emit.subject(document.id(), format_args!("q{ordinal}"))?;
        emit.describe(subject, "Directive", ordinal, span)?;
        emit.fact(subject, "rawText", Object::Text(raw(document, span)?))?;
    }
    for (ordinal, escape) in document.stream.escapes().iter().enumerate() {
        let subject = emit.subject(document.id(), format_args!("x{ordinal}"))?;
        emit.describe(subject, "Escape", ordinal, escape.span)?;
        emit.fact(
            subject,
            "rawText",
            Object::Text(raw(document, escape.span)?),
        )?;
        if let Some(character) = escape.decoded {
            emit.fact(subject, "escapeKind", Object::Text("character"))?;
            let mut bytes = [0; 4];
            emit.fact(
                subject,
                "decoded",
                Object::Text(character.encode_utf8(&mut bytes)),
            )?;
        } else {
            emit.fact(subject, "escapeKind", Object::Text("line-joining"))?;
        }
    }
    for (ordinal, &span) in document.cover.iter().enumerate() {
        let subject = emit.subject(document.id(), format_args!("c{ordinal}"))?;
        emit.describe(subject, "CoverSpan", ordinal, span)?;
        emit.fact(
            subject,
            "verbatim",
            Object::Typed(
                raw(document, span)?,
                profile.vocabulary().term("verbatimText")?,
            ),
        )?;
    }
    emit.finish()
}

fn raw<'a>(document: &Document<'a>, span: Span) -> Result<&'a str, YamlError> {
    document
        .stream
        .raw(span)
        .ok_or(YamlError::Metadata("native source span"))
}
fn scalar_style(style: ScalarStyle) -> &'static str {
    match style {
        ScalarStyle::Empty => "empty",
        ScalarStyle::Plain => "plain",
        ScalarStyle::SingleQuoted => "single-quoted",
        ScalarStyle::DoubleQuoted => "double-quoted",
        ScalarStyle::Literal => "literal",
        ScalarStyle::Folded => "folded",
    }
}
fn collection_style(style: CollectionStyle) -> &'static str {
    match style {
        CollectionStyle::Block => "block",
        CollectionStyle::Flow => "flow",
    }
}
