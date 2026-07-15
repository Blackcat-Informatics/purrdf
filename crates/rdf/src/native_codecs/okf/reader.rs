// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use core::fmt;
use core::ops::ControlFlow;

use purrdf_events::{EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use super::{
    MAX_OKF_FRONTMATTER_BYTES, MAX_OKF_LINKS_PER_DOCUMENT, MAX_OKF_YAML_DEPTH, MAX_OKF_YAML_NODES,
    OkfBundle, OkfConfig, OkfError, OkfReadOutcome, decimal_lexical_from_f64, minted_document_iri,
    validate_absolute_iri, validate_relative_markdown_path,
};
use crate::{LossEntry, LossLedger, RdfLocation};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

#[derive(Clone, Debug, PartialEq)]
enum StrictNumber {
    Signed(i64),
    Unsigned(u64),
    Decimal(String),
}

impl StrictNumber {
    fn lexical(&self) -> String {
        match self {
            Self::Signed(value) => value.to_string(),
            Self::Unsigned(value) => value.to_string(),
            Self::Decimal(value) => value.clone(),
        }
    }

    fn datatype(&self) -> &'static str {
        match self {
            Self::Signed(_) | Self::Unsigned(_) => XSD_INTEGER,
            Self::Decimal(_) => XSD_DECIMAL,
        }
    }

    fn to_json(&self) -> Result<serde_json::Value, OkfError> {
        let number = match self {
            Self::Signed(value) => serde_json::Number::from(*value),
            Self::Unsigned(value) => serde_json::Number::from(*value),
            Self::Decimal(value) => {
                serde_json::from_str::<serde_json::Number>(value).map_err(|error| {
                    OkfError::new(format!("invalid OKF decimal `{value}`: {error}"))
                })?
            }
        };
        Ok(serde_json::Value::Number(number))
    }
}

/// YAML value decoded through a duplicate-rejecting map visitor. `serde_yaml::Value`
/// accepts duplicate mapping keys with last-write behavior; that would silently
/// erase authored frontmatter, so the OKF reader owns this strict value tree.
#[derive(Clone, Debug, PartialEq)]
enum StrictValue {
    Null,
    Bool(bool),
    Number(StrictNumber),
    String(String),
    Sequence(Vec<Self>),
    Mapping(BTreeMap<String, Self>),
}

impl StrictValue {
    fn scalar_text(&self) -> Option<String> {
        match self {
            Self::Bool(value) => Some(value.to_string()),
            Self::Number(value) => Some(value.lexical()),
            Self::String(value) => Some(value.clone()),
            Self::Null | Self::Sequence(_) | Self::Mapping(_) => None,
        }
    }

    fn to_json(&self) -> Result<serde_json::Value, OkfError> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(value) => Ok(serde_json::Value::Bool(*value)),
            Self::Number(value) => value.to_json(),
            Self::String(value) => Ok(serde_json::Value::String(value.clone())),
            Self::Sequence(values) => values
                .iter()
                .map(Self::to_json)
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Self::Mapping(values) => {
                let mut object = serde_json::Map::new();
                for (key, value) in values {
                    object.insert(key.clone(), value.to_json()?);
                }
                Ok(serde_json::Value::Object(object))
            }
        }
    }
}

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a finite YAML scalar, sequence, or string-keyed mapping")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(StrictNumber::Signed(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(StrictNumber::Unsigned(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let lexical = decimal_lexical_from_f64(value).map_err(E::custom)?;
        let parsed = purrdf_xsd::parse_by_iri(&lexical, XSD_DECIMAL)
            .map_err(E::custom)?
            .ok_or_else(|| E::custom("internal OKF decimal datatype is not recognized"))?;
        Ok(StrictValue::Number(StrictNumber::Decimal(
            parsed.canonical_lexical(),
        )))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1_024));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(StrictValue::Sequence(values))
    }

    fn visit_map<A>(self, mut mapping: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = mapping.next_entry::<String, StrictValue>()? {
            if values.insert(key.clone(), value).is_some() {
                return Err(<A::Error as de::Error>::custom(format!(
                    "duplicate YAML mapping key `{key}`"
                )));
            }
        }
        Ok(StrictValue::Mapping(values))
    }
}

#[derive(Clone, Debug)]
struct ParsedDocument {
    path: String,
    subject_iri: String,
    fields: BTreeMap<String, StrictValue>,
    body: String,
    links: Vec<MarkdownLink>,
}

#[derive(Clone, Debug)]
pub(super) struct MarkdownLink {
    pub(super) text: String,
    pub(super) target: String,
    pub(super) ordinal: usize,
}

type ParsedFrontmatter = (BTreeMap<String, StrictValue>, String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum OwnedEventTerm {
    Iri(String),
    Blank(String),
    Literal { lexical: String, datatype: String },
}

#[derive(Default)]
struct EventGraph {
    ids: BTreeMap<OwnedEventTerm, EventTermId>,
    terms: Vec<OwnedEventTerm>,
    quads: Vec<EventQuad>,
    reifiers: Vec<(EventTermId, EventTriple)>,
    annotations: Vec<(EventTermId, EventTermId, EventTermId)>,
}

impl EventGraph {
    fn intern(&mut self, term: OwnedEventTerm) -> Result<EventTermId, OkfError> {
        if let Some(&id) = self.ids.get(&term) {
            return Ok(id);
        }
        let index = u32::try_from(self.terms.len())
            .map_err(|_| OkfError::new("OKF event stream exceeds the u32 term-id space"))?;
        let id = EventTermId(index);
        self.ids.insert(term.clone(), id);
        self.terms.push(term);
        Ok(id)
    }

    fn iri(&mut self, iri: &str) -> Result<EventTermId, OkfError> {
        self.intern(OwnedEventTerm::Iri(iri.to_owned()))
    }

    fn blank(&mut self, label: String) -> Result<EventTermId, OkfError> {
        self.intern(OwnedEventTerm::Blank(label))
    }

    fn literal(&mut self, lexical: &str, datatype: &str) -> Result<EventTermId, OkfError> {
        self.intern(OwnedEventTerm::Literal {
            lexical: lexical.to_owned(),
            datatype: datatype.to_owned(),
        })
    }

    fn quad_iri(
        &mut self,
        subject: EventTermId,
        predicate: &str,
        object: &str,
    ) -> Result<(), OkfError> {
        let predicate = self.iri(predicate)?;
        let object = self.iri(object)?;
        self.quads.push(EventQuad {
            s: subject,
            p: predicate,
            o: object,
            g: None,
        });
        Ok(())
    }

    fn quad_literal(
        &mut self,
        subject: EventTermId,
        predicate: &str,
        lexical: &str,
        datatype: &str,
    ) -> Result<(), OkfError> {
        let predicate = self.iri(predicate)?;
        let object = self.literal(lexical, datatype)?;
        self.quads.push(EventQuad {
            s: subject,
            p: predicate,
            o: object,
            g: None,
        });
        Ok(())
    }
}

/// Lift a deterministic OKF Markdown bundle into any RDF 1.2 event sink.
///
/// Parsing and profile validation complete before the first sink callback, so a
/// malformed bundle cannot leave a partially-driven sink. Terms are then declared
/// before references; a non-cancelled drive calls [`RdfEventSink::finish`] exactly
/// once. Relative Markdown links are emitted as RDF 1.2 reifiers plus link-text and
/// occurrence annotations.
///
/// # Errors
///
/// Returns [`OkfError`] for malformed YAML/Markdown, unrecognized keys, unsafe or
/// dangling paths, profile ambiguity, resource-limit breaches, or a sink error.
pub fn lift_okf_bundle<S: RdfEventSink + ?Sized>(
    bundle: &OkfBundle,
    config: &OkfConfig,
    sink: &mut S,
) -> Result<OkfReadOutcome, OkfError> {
    let (documents, losses, navigation_pages) = parse_documents(bundle, config)?;
    let mut graph = build_event_graph(&documents, config)?;
    graph.quads.sort();
    graph.reifiers.sort();
    graph.annotations.sort();
    let cancelled = drive_event_graph(&graph, config, sink)?;
    Ok(OkfReadOutcome {
        losses,
        documents: documents.len(),
        navigation_pages,
        cancelled,
    })
}

fn parse_documents(
    bundle: &OkfBundle,
    config: &OkfConfig,
) -> Result<(Vec<ParsedDocument>, LossLedger, usize), OkfError> {
    let mut documents = Vec::with_capacity(bundle.len());
    let mut subjects = BTreeMap::<String, String>::new();
    let mut losses = LossLedger::new();
    let mut navigation_pages = 0;

    for (path, markdown) in bundle.documents() {
        let Some((fields, body)) = parse_frontmatter(path, markdown)? else {
            if path.rsplit('/').next() == Some("index.md") {
                navigation_pages += 1;
                losses.record(LossEntry {
                    code: Cow::Borrowed("okf-navigation-page-dropped"),
                    from: Cow::Borrowed("okf"),
                    to: Cow::Borrowed("rdf-1.2-dataset"),
                    note: Cow::Borrowed(
                        "Frontmatter-less index.md is navigation-only and has no RDF concept subject.",
                    ),
                    location: Some(Box::new(RdfLocation::file(path))),
                });
                continue;
            }
            return Err(document_error(path, "missing YAML frontmatter"));
        };

        for key in fields.keys() {
            if !config.recognizes(key) {
                return Err(document_error(
                    path,
                    format!("unrecognized frontmatter key `{key}`"),
                ));
            }
        }
        let type_value = required_scalar(&fields, "type", path)?;
        if type_value.is_empty() {
            return Err(document_error(path, "required `type` must not be empty"));
        }

        let subject_iri = match fields.get("resource") {
            Some(value) => {
                let resource = scalar(value, "resource", path)?;
                validate_absolute_iri("OKF resource", &resource)?;
                resource
            }
            None => minted_document_iri(config, path)?,
        };
        if let Some(first_path) = subjects.insert(subject_iri.clone(), path.to_owned()) {
            return Err(document_error(
                path,
                format!("resource `{subject_iri}` is already used by document `{first_path}`"),
            ));
        }
        let links = extract_markdown_links(&body, path)?;
        documents.push(ParsedDocument {
            path: path.to_owned(),
            subject_iri,
            fields,
            body,
            links,
        });
    }

    Ok((documents, losses, navigation_pages))
}

fn parse_frontmatter(path: &str, markdown: &str) -> Result<Option<ParsedFrontmatter>, OkfError> {
    let opening_len = if markdown.starts_with("---\n") {
        4
    } else if markdown.starts_with("---\r\n") {
        5
    } else {
        return Ok(None);
    };

    let mut offset = opening_len;
    while offset <= markdown.len() {
        let relative_end = markdown[offset..].find('\n');
        let line_end = relative_end.map_or(markdown.len(), |position| offset + position);
        let line = markdown[offset..line_end].trim_end_matches('\r');
        let after_line = if line_end < markdown.len() {
            line_end + 1
        } else {
            line_end
        };
        if line == "---" {
            let yaml = &markdown[opening_len..offset];
            if yaml.len() > MAX_OKF_FRONTMATTER_BYTES {
                return Err(document_error(
                    path,
                    format!(
                        "YAML frontmatter is {} bytes; limit is {MAX_OKF_FRONTMATTER_BYTES}",
                        yaml.len()
                    ),
                ));
            }
            let value: StrictValue = serde_yaml::from_str(yaml).map_err(|error| {
                document_error(path, format!("invalid YAML frontmatter: {error}"))
            })?;
            validate_yaml_tree(&value, path)?;
            let StrictValue::Mapping(fields) = value else {
                return Err(document_error(path, "YAML frontmatter must be a mapping"));
            };
            return Ok(Some((fields, markdown[after_line..].to_owned())));
        }
        if line_end == markdown.len() {
            break;
        }
        offset = after_line;
    }
    Err(document_error(
        path,
        "YAML frontmatter is missing its closing `---` fence",
    ))
}

fn validate_yaml_tree(value: &StrictValue, path: &str) -> Result<(), OkfError> {
    fn visit(
        value: &StrictValue,
        depth: usize,
        nodes: &mut usize,
        path: &str,
    ) -> Result<(), OkfError> {
        if depth > MAX_OKF_YAML_DEPTH {
            return Err(document_error(
                path,
                format!("YAML nesting exceeds depth {MAX_OKF_YAML_DEPTH}"),
            ));
        }
        *nodes = nodes
            .checked_add(1)
            .ok_or_else(|| document_error(path, "YAML node count overflow"))?;
        if *nodes > MAX_OKF_YAML_NODES {
            return Err(document_error(
                path,
                format!("YAML tree exceeds {MAX_OKF_YAML_NODES} nodes"),
            ));
        }
        match value {
            StrictValue::Sequence(values) => {
                for child in values {
                    visit(child, depth + 1, nodes, path)?;
                }
            }
            StrictValue::Mapping(values) => {
                for child in values.values() {
                    visit(child, depth + 1, nodes, path)?;
                }
            }
            StrictValue::Null
            | StrictValue::Bool(_)
            | StrictValue::Number(_)
            | StrictValue::String(_) => {}
        }
        Ok(())
    }

    let mut nodes = 0;
    visit(value, 0, &mut nodes, path)
}

fn build_event_graph(
    documents: &[ParsedDocument],
    config: &OkfConfig,
) -> Result<EventGraph, OkfError> {
    let mut graph = EventGraph::default();
    let mut subjects_by_path = BTreeMap::new();

    for document in documents {
        let subject = graph.iri(&document.subject_iri)?;
        subjects_by_path.insert(document.path.clone(), subject);
        graph.quad_literal(subject, config.path_predicate(), &document.path, XSD_STRING)?;

        for (key, value) in &document.fields {
            let predicate = config.predicate_iri(key).ok_or_else(|| {
                document_error(&document.path, format!("unconfigured key `{key}`"))
            })?;
            match key.as_str() {
                "resource" => {
                    let resource = scalar(value, key, &document.path)?;
                    graph.quad_iri(subject, predicate, &resource)?;
                }
                "tags" => {
                    let StrictValue::Sequence(values) = value else {
                        return Err(document_error(
                            &document.path,
                            "frontmatter `tags` must be a YAML sequence",
                        ));
                    };
                    let mut tags = BTreeSet::new();
                    for value in values {
                        tags.insert(scalar(value, "tags", &document.path)?);
                    }
                    for tag in tags {
                        graph.quad_literal(subject, predicate, &tag, XSD_STRING)?;
                    }
                }
                "timestamp" => {
                    let text = scalar(value, key, &document.path)?;
                    purrdf_xsd::parse_by_iri(&text, XSD_DATETIME)
                        .map_err(|error| {
                            document_error(
                                &document.path,
                                format!("invalid timestamp `{text}`: {error}"),
                            )
                        })?
                        .ok_or_else(|| {
                            document_error(
                                &document.path,
                                "internal OKF timestamp datatype is not recognized",
                            )
                        })?;
                    graph.quad_literal(subject, predicate, &text, XSD_DATETIME)?;
                }
                "type" | "title" | "description" => {
                    let text = scalar(value, key, &document.path)?;
                    graph.quad_literal(subject, predicate, &text, XSD_STRING)?;
                }
                _ => emit_extension(
                    &mut graph,
                    subject,
                    predicate,
                    value,
                    config.json_datatype(),
                )?,
            }
        }
        graph.quad_literal(subject, config.body_predicate(), &document.body, XSD_STRING)?;
    }

    for (document_index, document) in documents.iter().enumerate() {
        let source = subjects_by_path[&document.path];
        for link in &document.links {
            let Some(target_path) = resolve_link_path(&document.path, &link.target)? else {
                continue;
            };
            let Some(&target) = subjects_by_path.get(&target_path) else {
                return Err(document_error(
                    &document.path,
                    format!("dangling Markdown link target `{}`", link.target),
                ));
            };
            let predicate = graph.iri(config.links_predicate())?;
            graph.quads.push(EventQuad {
                s: source,
                p: predicate,
                o: target,
                g: None,
            });

            let reifier = graph.blank(format!("okf_link_{document_index}_{}", link.ordinal))?;
            graph.reifiers.push((
                reifier,
                EventTriple {
                    s: source,
                    p: predicate,
                    o: target,
                },
            ));
            let text_predicate = graph.iri(config.link_text_predicate())?;
            let text = graph.literal(&link.text, XSD_STRING)?;
            graph.annotations.push((reifier, text_predicate, text));
            let occurrence_predicate = graph.iri(config.link_occurrence_predicate())?;
            let occurrence = graph.literal(&(link.ordinal + 1).to_string(), XSD_INTEGER)?;
            graph
                .annotations
                .push((reifier, occurrence_predicate, occurrence));
        }
    }

    Ok(graph)
}

fn emit_extension(
    graph: &mut EventGraph,
    subject: EventTermId,
    predicate: &str,
    value: &StrictValue,
    json_datatype: &str,
) -> Result<(), OkfError> {
    match value {
        StrictValue::Bool(flag) => {
            graph.quad_literal(subject, predicate, &flag.to_string(), XSD_BOOLEAN)
        }
        StrictValue::Number(number) => {
            graph.quad_literal(subject, predicate, &number.lexical(), number.datatype())
        }
        StrictValue::String(text) => graph.quad_literal(subject, predicate, text, XSD_STRING),
        StrictValue::Null | StrictValue::Sequence(_) | StrictValue::Mapping(_) => {
            let json = serde_json::to_string(&value.to_json()?)
                .map_err(|error| OkfError::new(format!("cannot encode OKF JSON value: {error}")))?;
            graph.quad_literal(subject, predicate, &json, json_datatype)
        }
    }
}

fn drive_event_graph<S: RdfEventSink + ?Sized>(
    graph: &EventGraph,
    config: &OkfConfig,
    sink: &mut S,
) -> Result<bool, OkfError> {
    if is_break(
        sink.base(config.document_base_iri())
            .map_err(|error| event_error(&error))?,
    ) {
        return Ok(true);
    }
    for (index, term) in graph.terms.iter().enumerate() {
        let id = EventTermId(
            u32::try_from(index).map_err(|_| OkfError::new("OKF event term index exceeds u32"))?,
        );
        let event = match term {
            OwnedEventTerm::Iri(iri) => EventTerm::Iri(iri),
            OwnedEventTerm::Blank(label) => EventTerm::Blank {
                label,
                scope: ScopeId::DEFAULT,
            },
            OwnedEventTerm::Literal { lexical, datatype } => EventTerm::Literal {
                lexical,
                datatype,
                language: None,
                direction: None,
            },
        };
        if is_break(sink.term(id, event).map_err(|error| event_error(&error))?) {
            return Ok(true);
        }
    }
    for &quad in &graph.quads {
        if is_break(sink.quad(quad).map_err(|error| event_error(&error))?) {
            return Ok(true);
        }
    }
    for &(reifier, triple) in &graph.reifiers {
        if is_break(
            sink.reifier(reifier, triple)
                .map_err(|error| event_error(&error))?,
        ) {
            return Ok(true);
        }
    }
    for &(reifier, predicate, object) in &graph.annotations {
        if is_break(
            sink.annotation(reifier, predicate, object)
                .map_err(|error| event_error(&error))?,
        ) {
            return Ok(true);
        }
    }
    sink.finish().map_err(|error| event_error(&error))?;
    Ok(false)
}

fn event_error(error: &purrdf_events::EventError) -> OkfError {
    OkfError::new(format!("OKF event sink failed: {error}"))
}

fn is_break(flow: ControlFlow<()>) -> bool {
    flow == ControlFlow::Break(())
}

fn required_scalar(
    fields: &BTreeMap<String, StrictValue>,
    key: &str,
    path: &str,
) -> Result<String, OkfError> {
    let value = fields
        .get(key)
        .ok_or_else(|| document_error(path, format!("missing required `{key}` frontmatter")))?;
    scalar(value, key, path)
}

fn scalar(value: &StrictValue, key: &str, path: &str) -> Result<String, OkfError> {
    value
        .scalar_text()
        .ok_or_else(|| document_error(path, format!("frontmatter `{key}` must be a scalar value")))
}

fn document_error(path: &str, detail: impl fmt::Display) -> OkfError {
    OkfError::new(format!("{path}: {detail}"))
}

pub(super) fn extract_markdown_links(
    body: &str,
    path: &str,
) -> Result<Vec<MarkdownLink>, OkfError> {
    let bytes = body.as_bytes();
    let mut links = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(relative_open) = body[cursor..].find('[') else {
            break;
        };
        let open = cursor + relative_open;
        if escaped(bytes, open)
            || (open > 0 && bytes[open - 1] == b'!' && !escaped(bytes, open - 1))
        {
            cursor = open + 1;
            continue;
        }
        let Some(close) = find_unescaped(bytes, open + 1, b']') else {
            break;
        };
        let mut paren = close + 1;
        while paren < bytes.len() && bytes[paren].is_ascii_whitespace() {
            paren += 1;
        }
        if paren >= bytes.len() || bytes[paren] != b'(' {
            cursor = close + 1;
            continue;
        }
        let Some(end) = find_closing_paren(bytes, paren + 1) else {
            return Err(document_error(
                path,
                "unterminated Markdown link destination",
            ));
        };
        let raw_target = body[paren + 1..end].trim();
        let target = markdown_destination(raw_target)
            .ok_or_else(|| document_error(path, "empty Markdown link destination"))?;
        if links.len() >= MAX_OKF_LINKS_PER_DOCUMENT {
            return Err(document_error(
                path,
                format!("Markdown body exceeds {MAX_OKF_LINKS_PER_DOCUMENT} links"),
            ));
        }
        links.push(MarkdownLink {
            text: unescape_markdown(&body[open + 1..close]),
            target,
            ordinal: links.len(),
        });
        cursor = end + 1;
    }
    Ok(links)
}

fn find_unescaped(bytes: &[u8], mut cursor: usize, needle: u8) -> Option<usize> {
    while cursor < bytes.len() {
        if bytes[cursor] == needle && !escaped(bytes, cursor) {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn find_closing_paren(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let mut depth = 1usize;
    while cursor < bytes.len() {
        if !escaped(bytes, cursor) {
            if bytes[cursor] == b'(' {
                depth += 1;
            } else if bytes[cursor] == b')' {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor);
                }
            }
        }
        cursor += 1;
    }
    None
}

fn escaped(bytes: &[u8], offset: usize) -> bool {
    let mut slashes = 0;
    let mut cursor = offset;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slashes += 1;
        cursor -= 1;
    }
    slashes % 2 == 1
}

fn markdown_destination(raw: &str) -> Option<String> {
    if let Some(rest) = raw.strip_prefix('<') {
        let end = rest.find('>')?;
        return Some(unescape_markdown(&rest[..end]));
    }
    let mut end = raw.len();
    for (index, ch) in raw.char_indices() {
        if ch.is_whitespace() && !escaped(raw.as_bytes(), index) {
            end = index;
            break;
        }
    }
    (end > 0).then(|| unescape_markdown(&raw[..end]))
}

fn unescape_markdown(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub(super) fn resolve_link_path(
    source_path: &str,
    target: &str,
) -> Result<Option<String>, OkfError> {
    let target = target.split('#').next().unwrap_or(target);
    if target.is_empty()
        || target.starts_with('/')
        || target.starts_with("//")
        || purrdf_iri::parse(target).is_ok_and(|iri| iri.has_scheme())
    {
        return Ok(None);
    }

    let mut components: Vec<&str> = source_path.split('/').collect();
    components.pop();
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err(OkfError::new(format!(
                        "Markdown link `{target}` escapes the OKF bundle root"
                    )));
                }
            }
            value => components.push(value),
        }
    }
    let normalized = components.join("/");
    if std::path::Path::new(&normalized)
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("md")
    {
        return Ok(None);
    }
    validate_relative_markdown_path(&normalized)?;
    Ok(Some(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatasetSink, SerializeGraph, check_ledger_sound};
    use purrdf_events::EventError;

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
                "producer",
            ],
        )
        .expect("valid config")
    }

    fn bundle() -> OkfBundle {
        OkfBundle::from_documents([
            (
                "index.md",
                "# Navigation\n\n- [Schema](concepts/schema.md)\n",
            ),
            (
                "concepts/schema.md",
                "---\ntype: Schema\ntitle: Event Schema\n---\nColumns: id.\n",
            ),
            (
                "concepts/table.md",
                "---\ntype: Table\ntitle: Events\nresource: https://example.org/data/events\ntags:\n- analytics\n- stable\nactive: true\ncount: 7\nproducer:\n  name: fixture\n---\nSee [Schema](schema.md).\n",
            ),
        ])
        .expect("valid bundle")
    }

    #[test]
    fn lift_emits_profile_links_and_structured_extensions() {
        let mut sink = DatasetSink::new();
        let outcome = lift_okf_bundle(&bundle(), &config(), &mut sink).expect("lift");
        assert_eq!(outcome.documents, 2);
        assert_eq!(outcome.navigation_pages, 1);
        assert!(!outcome.cancelled);
        assert_eq!(outcome.losses.entries().len(), 1);
        assert_eq!(
            outcome.losses.entries()[0].code,
            "okf-navigation-page-dropped"
        );
        assert!(check_ledger_sound(&outcome.losses, "okf", "rdf-1.2-dataset").is_ok());

        let dataset = sink.into_dataset().expect("sink finished");
        assert_eq!(dataset.quad_count(), 15);
        assert_eq!(dataset.reifiers().count(), 1);
        assert_eq!(dataset.annotations().count(), 2);
        let text = String::from_utf8(
            crate::native_codecs::serialize_dataset(
                dataset.as_ref(),
                "application/n-quads",
                SerializeGraph::Dataset,
            )
            .expect("serialize"),
        )
        .expect("UTF-8");
        assert!(text.contains("<https://example.org/okf#producer>"));
        assert!(text.contains("^^<https://example.org/okf#json>"));
        assert!(text.contains("<https://example.org/okf#links>"));
        assert!(text.contains("<https://example.org/okf#linkText> \"Schema\""));
        assert!(text.contains("<https://example.org/doc/concepts/schema.md>"));
    }

    #[test]
    fn duplicate_and_unknown_frontmatter_keys_hard_fail_before_finish() {
        for markdown in [
            "---\ntype: Concept\ntype: Other\n---\nbody\n",
            "---\ntype: Concept\nunconfigured: value\n---\nbody\n",
        ] {
            let bundle = OkfBundle::from_documents([("concept.md", markdown)]).expect("bundle");
            let mut sink = DatasetSink::new();
            let error = lift_okf_bundle(&bundle, &config(), &mut sink)
                .expect_err("invalid frontmatter must fail");
            assert!(
                error.to_string().contains("duplicate")
                    || error.to_string().contains("unrecognized"),
                "unexpected error: {error}"
            );
            assert!(
                sink.dataset().is_none(),
                "invalid input must not finish sink"
            );
        }
    }

    #[test]
    fn dangling_relative_markdown_link_hard_fails() {
        let bundle = OkfBundle::from_documents([(
            "concept.md",
            "---\ntype: Concept\n---\nSee [missing](missing.md).\n",
        )])
        .expect("bundle");
        let mut sink = DatasetSink::new();
        let error =
            lift_okf_bundle(&bundle, &config(), &mut sink).expect_err("dangling link must fail");
        assert!(error.to_string().contains("dangling Markdown link"));
        assert!(sink.dataset().is_none());
    }

    #[test]
    fn escaped_destination_character_does_not_hide_closing_parenthesis() {
        let links = extract_markdown_links("See [literal](schema.md\\)).\n", "concept.md")
            .expect("escaped destination character");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "schema.md)");
    }

    #[test]
    fn invalid_timestamp_hard_fails_before_driving_the_sink() {
        let bundle = OkfBundle::from_documents([(
            "concept.md",
            "---\ntype: Concept\ntimestamp: yesterday\n---\nbody\n",
        )])
        .expect("bundle");
        let mut sink = DatasetSink::new();
        let error = lift_okf_bundle(&bundle, &config(), &mut sink)
            .expect_err("invalid xsd:dateTime must fail");
        assert!(error.detail().contains("invalid timestamp"));
        assert!(sink.dataset().is_none());
    }

    #[derive(Default)]
    struct CancellingSink {
        finished: bool,
    }

    impl RdfEventSink for CancellingSink {
        fn term(
            &mut self,
            _id: EventTermId,
            _term: EventTerm<'_>,
        ) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Break(()))
        }

        fn quad(&mut self, _quad: EventQuad) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Continue(()))
        }

        fn reifier(
            &mut self,
            _reifier: EventTermId,
            _triple: EventTriple,
        ) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Continue(()))
        }

        fn annotation(
            &mut self,
            _reifier: EventTermId,
            _predicate: EventTermId,
            _object: EventTermId,
        ) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Continue(()))
        }

        fn open_scope(&mut self) -> Result<ScopeId, EventError> {
            Ok(ScopeId::DEFAULT)
        }

        fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Continue(()))
        }

        fn finish(&mut self) -> Result<(), EventError> {
            self.finished = true;
            Ok(())
        }
    }

    #[test]
    fn cancellation_stops_without_finishing_sink() {
        let bundle = OkfBundle::from_documents([("concept.md", "---\ntype: Concept\n---\nbody\n")])
            .expect("bundle");
        let mut sink = CancellingSink::default();
        let outcome = lift_okf_bundle(&bundle, &config(), &mut sink).expect("cancelled lift");
        assert!(outcome.cancelled);
        assert!(!sink.finished);
    }

    #[test]
    fn configuration_and_paths_reject_implicit_or_unsafe_values() {
        assert!(OkfConfig::new("relative#", "https://example.org/doc/", ["type"]).is_err());
        assert!(
            OkfConfig::new(
                "https://example.org/okf#",
                "https://example.org/doc/",
                ["title"]
            )
            .is_err()
        );
        assert!(
            OkfConfig::new(
                "https://example.org/okf#",
                "https://example.org/doc/",
                ["type", "body"]
            )
            .is_err()
        );
        assert!(OkfBundle::from_documents([("../escape.md", "x")]).is_err());
        assert!(OkfBundle::from_documents([("not-markdown.txt", "x")]).is_err());
    }
}
