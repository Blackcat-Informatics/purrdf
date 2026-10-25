// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::{
    Document, Float, Kind, Profile, Scalar, SourceDocument, TableOrigin, TomlError, analyze,
};
use purrdf_core::{
    BaseIri, ContentDigest, DatasetView, GraphMatch, RdfLiteral, TermRef,
    cover::{VerbatimSpan, reconstruct},
    ir::{QuadIds, RdfDataset, RdfDatasetBuilder, TermId},
};
use purrdf_xsd::datatype::{
    XSD_BOOLEAN, XSD_DATE, XSD_DATE_TIME, XSD_DOUBLE, XSD_INTEGER, XSD_STRING, XSD_TIME,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

#[derive(Clone, Copy)]
enum Object<'a> {
    Iri(&'a str),
    Text(&'a str),
    Typed(&'a str, &'a str),
    Integer(usize),
}

struct Emitter {
    builder: RdfDatasetBuilder,
    predicates: BTreeMap<&'static str, TermId>,
    namespace: String,
}

impl Emitter {
    fn new(profile: &Profile) -> Result<Self, TomlError> {
        let mut builder = RdfDatasetBuilder::new();
        let mut predicates = BTreeMap::new();
        predicates.insert("type", builder.intern_iri(purrdf_core::vocab::rdf::TYPE)?);
        for &local in crate::profile::TERMS {
            predicates.insert(
                local,
                builder.intern_iri(&profile.vocabulary().term(local)?)?,
            );
        }
        Ok(Self {
            builder,
            predicates,
            namespace: profile.vocabulary().base().to_owned(),
        })
    }
    fn fact(
        &mut self,
        subject: &str,
        predicate: &'static str,
        value: Object<'_>,
    ) -> Result<(), TomlError> {
        let subject = self.builder.intern_iri(subject)?;
        let object = match value {
            Object::Iri(iri) => self.builder.intern_iri(iri)?,
            Object::Text(text) => self.builder.intern_literal(RdfLiteral::simple(text))?,
            Object::Typed(text, datatype) => self
                .builder
                .intern_literal(RdfLiteral::typed(text, datatype))?,
            Object::Integer(number) => self
                .builder
                .intern_literal(RdfLiteral::typed(number.to_string(), XSD_INTEGER))?,
        };
        self.builder
            .push_quad(subject, self.predicates[predicate], object, None)?;
        Ok(())
    }
    fn describe(
        &mut self,
        subject: &str,
        class: &str,
        document: &str,
        index: usize,
        span: &Range<usize>,
    ) -> Result<(), TomlError> {
        self.fact(
            subject,
            "type",
            Object::Iri(&format!("{}{class}", self.namespace)),
        )?;
        self.fact(subject, "document", Object::Iri(document))?;
        for (field, number) in [
            ("occurrence", index),
            ("byteStart", span.start),
            ("byteEnd", span.end),
        ] {
            self.fact(subject, field, Object::Integer(number))?;
        }
        Ok(())
    }
    fn verbatim(&mut self, subject: &str, text: &str) -> Result<(), TomlError> {
        self.fact(
            subject,
            "verbatim",
            Object::Typed(text, &format!("{}verbatimText", self.namespace)),
        )
    }
}

/// Analyze and project directly into the native RDF dataset.
pub fn encode(source: SourceDocument<'_>, profile: &Profile) -> Result<Arc<RdfDataset>, TomlError> {
    project(&analyze(source, profile)?, profile)
}

/// Project the model under the exact profile that admitted its source.
pub fn project(document: &Document<'_>, profile: &Profile) -> Result<Arc<RdfDataset>, TomlError> {
    if document.profile != profile.identity() {
        return Err(TomlError::Metadata("model/profile identity mismatch"));
    }
    let mut emit = Emitter::new(profile)?;
    let root = document.id();
    emit.fact(
        root,
        "type",
        Object::Iri(&profile.vocabulary().term("Document")?),
    )?;
    emit.fact(root, "source", Object::Iri(document.source.id))?;
    emit.fact(
        root,
        "sourceDigest",
        Object::Typed(
            &format!("sha256:{}", document.digest.to_hex()),
            &profile.vocabulary().term("digest")?,
        ),
    )?;
    emit.fact(root, "byteLength", Object::Integer(document.text.len()))?;
    emit.fact(root, "profile", Object::Text(&profile.identity().to_hex()))?;
    for (index, node) in document.nodes.iter().enumerate() {
        let id = format!("{root}#v{index}");
        emit.describe(&id, "Value", root, index, &node.span)?;
        emit.fact(&id, "path", Object::Text(&node.path))?;
        emit.fact(&id, "ordinal", Object::Integer(node.ordinal))?;
        let parent = node
            .parent
            .map_or_else(|| root.to_owned(), |parent| format!("{root}#v{parent}"));
        emit.fact(&id, "parent", Object::Iri(&parent))?;
        if let Some(key) = &node.key {
            emit.fact(&id, "key", Object::Text(&key.decoded))?;
        }
        let kind = match &node.kind {
            Kind::Table(origin) => match origin {
                TableOrigin::Root => "root-table",
                TableOrigin::Implicit => "implicit-table",
                TableOrigin::Explicit => "table",
                TableOrigin::Dotted => "dotted-table",
                TableOrigin::Inline => "inline-table",
                TableOrigin::ArrayElement => "table-element",
            },
            Kind::Array => "array",
            Kind::ArrayOfTables => "array-of-tables",
            Kind::Scalar(value) => {
                emit.verbatim(&id, &document.text[node.span.clone()])?;
                let (kind, value, datatype) = match value {
                    Scalar::String(text) => ("string", text.clone(), XSD_STRING),
                    Scalar::Integer(integer) => ("integer", integer.to_string(), XSD_INTEGER),
                    Scalar::Boolean(flag) => ("boolean", flag.to_string(), XSD_BOOLEAN),
                    Scalar::Float(float) => match float {
                        Float::Finite(value) => ("float", value.to_string(), XSD_DOUBLE),
                        Float::Infinity { negative } => (
                            "float",
                            if *negative { "-INF" } else { "INF" }.to_owned(),
                            XSD_DOUBLE,
                        ),
                        Float::Nan { negative } => (
                            if *negative { "negative-nan" } else { "nan" },
                            "NaN".to_owned(),
                            XSD_DOUBLE,
                        ),
                    },
                    Scalar::OffsetDateTime(text) => {
                        ("offset-date-time", temporal_lexical(text), XSD_DATE_TIME)
                    }
                    Scalar::LocalDateTime(text) => {
                        ("local-date-time", temporal_lexical(text), XSD_DATE_TIME)
                    }
                    Scalar::LocalDate(text) => ("local-date", text.clone(), XSD_DATE),
                    Scalar::LocalTime(text) => ("local-time", text.clone(), XSD_TIME),
                };
                emit.fact(&id, "value", Object::Typed(&value, datatype))?;
                kind
            }
        };
        emit.fact(&id, "kind", Object::Text(kind))?;
        if !matches!(node.kind, Kind::Scalar(_)) {
            emit.fact(&id, "size", Object::Integer(node.children.len()))?;
        }
    }
    for (index, key) in document.keys.iter().enumerate() {
        let id = format!("{root}#k{index}");
        emit.describe(&id, "Key", root, index, &key.span)?;
        emit.fact(&id, "key", Object::Text(&key.decoded))?;
    }
    for (index, statement) in document.statements.iter().enumerate() {
        let id = format!("{root}#d{index}");
        emit.describe(&id, "Statement", root, index, &statement.span)?;
        emit.fact(
            &id,
            "node",
            Object::Iri(&format!("{root}#v{}", statement.node)),
        )?;
        emit.fact(
            &id,
            "header",
            Object::Typed(if statement.header { "true" } else { "false" }, XSD_BOOLEAN),
        )?;
    }
    for (index, span) in document.structure().enumerate() {
        let id = format!("{root}#s{index}");
        emit.describe(&id, "Structure", root, index, &span)?;
        emit.verbatim(&id, &document.text[span])?;
    }
    Ok(emit.builder.freeze()?)
}

fn temporal_lexical(text: &str) -> String {
    text.replace(['t', ' '], "T").replace('z', "Z")
}

fn required<'a>(
    dataset: &'a RdfDataset,
    subject: TermId,
    predicate: &str,
) -> Result<TermRef<'a>, TomlError> {
    let predicate = dataset
        .term_id_by_iri(predicate)
        .ok_or(TomlError::Metadata("missing required predicate"))?;
    let annotations = dataset
        .annotations_of_with_graph(subject)
        .map(|(p, o, g)| QuadIds {
            s: subject,
            p,
            o,
            g,
        });
    let mut objects = DatasetView::quads_for_pattern(
        dataset,
        Some(subject),
        Some(predicate),
        None,
        GraphMatch::Any,
    )
    .chain(dataset.reifier_quads_of(subject))
    .chain(annotations)
    .filter(|row| row.p == predicate);
    let first = objects
        .next()
        .ok_or(TomlError::Metadata("missing required field"))?;
    let object = first.o;
    if first.g.is_some() || objects.any(|row| row.o != object || row.g.is_some()) {
        return Err(TomlError::Metadata("conflicting required field"));
    }
    Ok(dataset.resolve(object))
}

fn literal<'a>(
    dataset: &'a RdfDataset,
    subject: TermId,
    predicate: &str,
    datatype: &str,
) -> Result<&'a str, TomlError> {
    match required(dataset, subject, predicate)? {
        TermRef::Literal {
            lexical,
            datatype: stated,
            language: None,
            direction: None,
        } if matches!(dataset.resolve(stated), TermRef::Iri(iri) if iri == datatype) => Ok(lexical),
        _ => Err(TomlError::Metadata("required literal datatype")),
    }
}

fn number(dataset: &RdfDataset, subject: TermId, predicate: &str) -> Result<u64, TomlError> {
    let raw = literal(dataset, subject, predicate, XSD_INTEGER)?;
    let value: u64 = raw
        .parse()
        .map_err(|_| TomlError::Metadata("unsigned byte/count field"))?;
    if raw != value.to_string() {
        return Err(TomlError::Metadata("canonical integer spelling"));
    }
    Ok(value)
}

/// Decode one explicitly selected document and profile. The shared cover law
/// reconstructs bytes; re-analysis and exact RDF metadata comparison then reject
/// forged keys, spans, kinds, semantics, parentage, identities and extra facts.
pub fn decode_document(
    dataset: &RdfDataset,
    document: &str,
    profile: &Profile,
) -> Result<String, TomlError> {
    BaseIri::parse(document)?;
    let root = dataset
        .term_id_by_iri(document)
        .ok_or(TomlError::Metadata("selected document"))?;
    let vocabulary = profile.vocabulary();
    let length = number(dataset, root, &vocabulary.term("byteLength")?)?;
    if length > profile.bounds().max_source_bytes {
        return Err(TomlError::Metadata("source byte bound"));
    }
    let TermRef::Iri(source) = required(dataset, root, &vocabulary.term("source")?)? else {
        return Err(TomlError::Metadata("absolute source IRI"));
    };
    BaseIri::parse(source)?;
    let stated_profile = literal(dataset, root, &vocabulary.term("profile")?, XSD_STRING)?;
    if stated_profile != profile.identity().to_hex() {
        return Err(TomlError::Metadata("profile identity"));
    }
    let digest_text = literal(
        dataset,
        root,
        &vocabulary.term("sourceDigest")?,
        &vocabulary.term("digest")?,
    )?;
    let hex = digest_text
        .strip_prefix("sha256:")
        .ok_or(TomlError::Metadata("source digest prefix"))?;
    let digest = ContentDigest::from_hex(hex).ok_or(TomlError::Metadata("source digest"))?;
    if hex != digest.to_hex() {
        return Err(TomlError::Metadata("canonical source digest"));
    }
    let ownership = dataset
        .term_id_by_iri(&vocabulary.term("document")?)
        .ok_or(TomlError::Metadata("document ownership predicate"))?;
    let mut selected = BTreeSet::from([root]);
    // A selected subject also needs a tree entry and potentially a cover span.
    // Admit their conservative combined layout before growing either collection.
    let selected_bound =
        profile.bounds().max_model_bytes / (size_of::<VerbatimSpan<'_>>() + 64) as u64;
    let prefix = format!("{document}#");
    if dataset.named_graphs().any(|graph| matches!(dataset.resolve(graph), TermRef::Iri(iri) if iri == document || iri.starts_with(&prefix))) {
        return Err(TomlError::Metadata("document metadata cannot declare a named graph"));
    }
    for row in dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
    {
        let owned = row.p == ownership && row.o == root
            || matches!(dataset.resolve(row.s), TermRef::Iri(iri) if iri.starts_with(&prefix));
        if owned && selected.insert(row.s) && selected.len() as u64 > selected_bound {
            return Err(TomlError::Metadata("selected node bound"));
        }
    }
    let verbatim = vocabulary.term("verbatim")?;
    let verbatim_type = vocabulary.term("verbatimText")?;
    let mut spans = Vec::new();
    let mut span_bytes = 0_u64;
    if let Some(predicate) = dataset.term_id_by_iri(&verbatim) {
        let mut covered = BTreeSet::new();
        for row in dataset
            .quads()
            .chain(dataset.reifier_quads())
            .chain(dataset.annotation_quads())
            .filter(|row| selected.contains(&row.s) && row.p == predicate)
        {
            if !covered.insert(row.s) {
                continue;
            }
            let text = literal(dataset, row.s, &verbatim, &verbatim_type)?;
            span_bytes = span_bytes
                .checked_add(text.len() as u64)
                .ok_or(TomlError::Metadata("cover byte overflow"))?;
            if span_bytes > length || spans.len() as u64 >= selected_bound {
                return Err(TomlError::Metadata("cover admission"));
            }
            spans.push(VerbatimSpan {
                byte_start: number(dataset, row.s, &vocabulary.term("byteStart")?)?,
                byte_end: number(dataset, row.s, &vocabulary.term("byteEnd")?)?,
                text,
                continues: None,
            });
        }
    }
    let text = reconstruct(length, &digest, &spans)?;
    let model = analyze(
        SourceDocument {
            id: source,
            bytes: text.as_bytes(),
        },
        profile,
    )?;
    if model.id() != document {
        return Err(TomlError::Metadata("document identity"));
    }
    let expected = project(&model, profile)?;
    let expected_count = expected.quads().count();
    let mut actual = BTreeSet::new();
    for row in dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
        .filter(|row| selected.contains(&row.s))
    {
        let translate = |id| {
            expected
                .as_ref()
                .term_id_by_value(&dataset.term_value(id))
                .ok_or(TomlError::Metadata("unexpected metadata term"))
        };
        if row.g.is_some() {
            return Err(TomlError::Metadata("metadata must be in the default graph"));
        }
        let row = (translate(row.s)?, translate(row.p)?, translate(row.o)?);
        if actual.insert(row) && actual.len() > expected_count {
            return Err(TomlError::Metadata("extra document metadata"));
        }
    }
    let wanted: BTreeSet<_> = expected
        .quads()
        .map(|QuadIds { s, p, o, .. }| (s, p, o))
        .collect();
    if actual != wanted {
        return Err(TomlError::Metadata("complete reparsed document structure"));
    }
    Ok(text)
}
