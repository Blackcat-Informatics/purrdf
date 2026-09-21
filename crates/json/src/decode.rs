// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::{
    BaseIri, ContentDigest,
    cover::{VerbatimSpan, reconstruct},
    dataset_view::{DatasetView, GraphMatch},
    hash::FastMap,
    ir::{QuadIds, RdfDataset, TermId, TermRef},
};

use crate::{
    JsonError, Kind, Profile, SourceDocument, analyze,
    error::metadata,
    profile::{RDF_TYPE, TERMS, XSD_INTEGER, XSD_STRING},
    project::{structure_id, value_id},
};

/// Decode one explicitly selected RDF document under the expected profile.
///
/// Codec subjects form a closed metadata graph in the default graph; unknown
/// predicates, conflicting values, wrong datatypes, missing ownership, invented
/// occurrences and noncanonical offsets all refuse. Other documents may coexist
/// in the dataset. No bytes escape until the cover, digest, grammar and complete
/// structural metadata have all been verified.
pub fn decode_document(
    dataset: &RdfDataset,
    document: &str,
    profile: &Profile,
) -> Result<String, JsonError> {
    BaseIri::parse(document).map_err(JsonError::Iri)?;
    let root_id = dataset
        .term_id_by_iri(document)
        .ok_or_else(|| metadata(document, "document node"))?;
    let root = read_root(dataset, root_id, profile)?;
    root.fields(&["type", "source", "sourceDigest", "byteLength", "profile"])?;
    root.expect_iri("type", profile.vocabulary().iri("Document"))?;
    let source = root.iri("source")?;
    BaseIri::parse(source).map_err(JsonError::Iri)?;
    if root.string("profile")? != profile.identity().to_hex() {
        return Err(JsonError::ProfileMismatch);
    }
    let length = root.number("byteLength")?;
    if length > profile.bounds().max_source_bytes {
        return Err(JsonError::Limit {
            resource: "source bytes",
            limit: profile.bounds().max_source_bytes,
        });
    }
    let digest = root.literal("sourceDigest", profile.vocabulary().iri("digest"))?;
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| metadata(document, "sourceDigest"))?;
    let digest = ContentDigest::from_hex(hex).ok_or_else(|| metadata(document, "sourceDigest"))?;
    if hex != digest.to_hex() {
        return Err(metadata(document, "sourceDigest"));
    }
    let nodes = collect(dataset, root_id, root, profile)?;
    let mut spans = Vec::with_capacity(nodes.len().saturating_sub(1));
    for (&subject, facts) in &nodes {
        if subject == document {
            continue;
        }
        if let Some(span) = validate_node(facts, document, profile)? {
            spans.push(span);
        }
    }
    let text = reconstruct(length, &digest, &spans).map_err(JsonError::Cover)?;
    let model = analyze(
        SourceDocument {
            id: source,
            bytes: text.as_bytes(),
        },
        profile,
    )?;
    if model.id() != document {
        return Err(metadata(document, "document identity"));
    }
    if nodes.len() != 1 + model.values.len() + model.runs.len() {
        return Err(metadata(document, "complete occurrence set"));
    }
    for (index, value) in model.values.iter().enumerate() {
        let id = value_id(document, index);
        let facts = node(&nodes, &id)?;
        facts.expect_iri("type", profile.vocabulary().iri("Value"))?;
        facts.expect_number("occurrence", index)?;
        facts.expect_number("byteStart", value.span.start)?;
        facts.expect_number("byteEnd", value.span.end)?;
        facts.expect_number("ordinal", value.ordinal)?;
        if let Some(size) = value.size() {
            facts.expect_number("size", size)?;
        }
        if facts.string("kind")? != value.kind.name() {
            return Err(metadata(&id, "kind"));
        }
        if facts.string("path")? != value.path {
            return Err(metadata(&id, "path"));
        }
        match value.parent {
            Some(parent) => facts.expect_iri("parent", &value_id(document, parent))?,
            None => facts.expect_iri("parent", document)?,
        }
    }
    for (index, run) in model.runs.iter().enumerate() {
        let id = structure_id(document, index);
        let facts = node(&nodes, &id)?;
        facts.expect_iri("type", profile.vocabulary().iri("Structure"))?;
        facts.expect_number("byteStart", run.start)?;
        facts.expect_number("byteEnd", run.end)?;
    }
    Ok(text)
}

type Nodes<'a> = FastMap<&'a str, Node<'a>>;

struct Node<'a> {
    dataset: &'a RdfDataset,
    subject: &'a str,
    facts: Vec<(&'static str, TermRef<'a>)>,
}

fn node<'a, 'n>(nodes: &'n Nodes<'a>, id: &str) -> Result<&'n Node<'a>, JsonError> {
    nodes.get(id).ok_or_else(|| metadata(id, "required node"))
}

fn read_root<'a>(
    dataset: &'a RdfDataset,
    id: TermId,
    profile: &Profile,
) -> Result<Node<'a>, JsonError> {
    let subject = iri_subject(dataset, id, "selected document")?;
    let mut root = Node {
        dataset,
        subject,
        facts: Vec::with_capacity(5),
    };
    let annotations = dataset
        .annotations_of_with_graph(id)
        .map(|(p, o, g)| QuadIds { s: id, p, o, g });
    for statement in DatasetView::quads_for_pattern(dataset, Some(id), None, None, GraphMatch::Any)
        .chain(dataset.reifier_quads_of(id))
        .chain(annotations)
    {
        root.insert(statement, profile)?;
    }
    Ok(root)
}

fn collect<'a>(
    dataset: &'a RdfDataset,
    root: TermId,
    root_facts: Node<'a>,
    profile: &Profile,
) -> Result<Nodes<'a>, JsonError> {
    let document = root_facts.subject;
    let mut selected: FastMap<TermId, &'a str> = FastMap::default();
    selected.insert(root, iri_subject(dataset, root, document)?);
    let owner = dataset.term_id_by_iri(profile.vocabulary().iri("document"));
    let limit = u64::from(profile.bounds().max_values) * 2 + 2;
    for statement in dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
    {
        if Some(statement.p) == owner && statement.o == root {
            let subject = iri_subject(dataset, statement.s, document)?;
            if statement.g.is_some() {
                return Err(metadata(subject, "default graph ownership"));
            }
            selected.insert(statement.s, subject);
            if selected.len() as u64 > limit {
                return Err(JsonError::Limit {
                    resource: "RDF codec nodes",
                    limit,
                });
            }
        }
    }
    for graph in dataset.named_graphs() {
        if let Some(subject) = selected.get(&graph) {
            return Err(metadata(subject, "codec subject used as a named graph"));
        }
    }
    let mut nodes: Nodes<'_> = selected
        .values()
        .filter(|&&subject| subject != document)
        .map(|&subject| {
            (
                subject,
                Node {
                    dataset,
                    subject,
                    facts: Vec::with_capacity(10),
                },
            )
        })
        .collect();
    nodes.insert(document, root_facts);
    for statement in dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
    {
        let Some(&subject) = selected.get(&statement.s) else {
            continue;
        };
        if statement.s == root {
            continue;
        }
        nodes
            .get_mut(subject)
            .expect("selected subjects initialize the node map")
            .insert(statement, profile)?;
    }
    Ok(nodes)
}

fn iri_subject<'a>(
    dataset: &'a RdfDataset,
    id: TermId,
    document: &str,
) -> Result<&'a str, JsonError> {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => Ok(iri),
        _ => Err(metadata(document, "IRI node ownership")),
    }
}

fn validate_node<'a>(
    facts: &Node<'a>,
    document: &str,
    profile: &Profile,
) -> Result<Option<VerbatimSpan<'a>>, JsonError> {
    facts.expect_iri("document", document)?;
    let class = facts.iri("type")?;
    let start = facts.number("byteStart")?;
    let end = facts.number("byteEnd")?;
    if end < start {
        return Err(metadata(facts.subject, "ordered byte offsets"));
    }
    let text = if class == profile.vocabulary().iri("Structure") {
        facts.fields(&["type", "document", "byteStart", "byteEnd", "verbatim"])?;
        Some(facts.literal("verbatim", profile.vocabulary().iri("verbatimText"))?)
    } else if class == profile.vocabulary().iri("Value") {
        let kind = match facts.string("kind")? {
            "string" => Kind::String,
            "number" => Kind::Number,
            "true" => Kind::True,
            "false" => Kind::False,
            "null" => Kind::Null,
            "array" => Kind::Array,
            "object" => Kind::Object,
            _ => return Err(metadata(facts.subject, "kind")),
        };
        facts.string("path")?;
        facts.iri("parent")?;
        facts.number("ordinal")?;
        facts.number("occurrence")?;
        let fields = [
            "type",
            "document",
            "byteStart",
            "byteEnd",
            "occurrence",
            "kind",
            "path",
            "parent",
            "ordinal",
            if kind.is_scalar() { "text" } else { "size" },
        ];
        facts.fields(&fields)?;
        if kind.is_scalar() {
            Some(facts.string("text")?)
        } else {
            facts.number("size")?;
            None
        }
    } else {
        return Err(metadata(facts.subject, "node class"));
    };
    Ok(text.map(|text| VerbatimSpan {
        byte_start: start,
        byte_end: end,
        text,
        continues: None,
    }))
}

impl<'a> Node<'a> {
    fn insert(&mut self, statement: QuadIds, profile: &Profile) -> Result<(), JsonError> {
        if statement.g.is_some() {
            return Err(metadata(self.subject, "default graph metadata"));
        }
        let TermRef::Iri(predicate) = self.dataset.resolve(statement.p) else {
            return Err(metadata(self.subject, "predicate IRI"));
        };
        let field = if predicate == RDF_TYPE {
            "type"
        } else {
            let local = predicate
                .strip_prefix(profile.vocabulary().base())
                .ok_or_else(|| metadata(self.subject, "closed metadata predicates"))?;
            *TERMS
                .iter()
                .find(|&&term| term == local)
                .ok_or_else(|| metadata(self.subject, "closed metadata predicates"))?
        };
        let object = self.dataset.resolve(statement.o);
        if let Some((_, existing)) = self.facts.iter().find(|(name, _)| *name == field) {
            // Identical assertions can occur in both the base and RDF 1.2
            // annotation tables. RDF set semantics count them as one fact.
            if *existing == object {
                return Ok(());
            }
            return Err(metadata(self.subject, "single-valued metadata"));
        }
        if self.facts.len() >= 10 {
            return Err(metadata(self.subject, "metadata field count"));
        }
        self.facts.push((field, object));
        Ok(())
    }

    fn fields(&self, expected: &[&str]) -> Result<(), JsonError> {
        if self.facts.len() != expected.len()
            || self
                .facts
                .iter()
                .any(|(field, _)| !expected.contains(field))
        {
            return Err(metadata(self.subject, "required metadata fields"));
        }
        Ok(())
    }

    fn get(&self, field: &'static str) -> Result<TermRef<'a>, JsonError> {
        self.facts
            .iter()
            .find(|(name, _)| *name == field)
            .map(|(_, value)| *value)
            .ok_or_else(|| metadata(self.subject, field))
    }

    fn iri(&self, field: &'static str) -> Result<&'a str, JsonError> {
        match self.get(field)? {
            TermRef::Iri(iri) => Ok(iri),
            _ => Err(metadata(self.subject, field)),
        }
    }

    fn literal(&self, field: &'static str, expected_datatype: &str) -> Result<&'a str, JsonError> {
        match self.get(field)? {
            TermRef::Literal {
                lexical,
                datatype,
                language: None,
                direction: None,
            } if self.dataset.resolve(datatype) == TermRef::Iri(expected_datatype) => Ok(lexical),
            _ => Err(metadata(self.subject, field)),
        }
    }

    fn string(&self, field: &'static str) -> Result<&'a str, JsonError> {
        self.literal(field, XSD_STRING)
    }

    fn number(&self, field: &'static str) -> Result<u64, JsonError> {
        let lexical = self.literal(field, XSD_INTEGER)?;
        if lexical.is_empty()
            || (lexical.len() > 1 && lexical.starts_with('0'))
            || !lexical.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(metadata(self.subject, field));
        }
        lexical.parse().map_err(|_| metadata(self.subject, field))
    }

    fn expect_iri(&self, field: &'static str, expected: &str) -> Result<(), JsonError> {
        if self.iri(field)? != expected {
            return Err(metadata(self.subject, field));
        }
        Ok(())
    }

    fn expect_number(&self, field: &'static str, expected: usize) -> Result<(), JsonError> {
        if self.number(field)? != expected as u64 {
            return Err(metadata(self.subject, field));
        }
        Ok(())
    }
}
