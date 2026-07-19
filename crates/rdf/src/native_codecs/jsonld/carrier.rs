// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed expanded carrier shared by JSON-LD-star serialization and expansion.
//!
//! RDF positions and JSON-LD keywords are represented by Rust fields and enums until
//! final emission. Caller terms therefore cannot collide with `@id`, `@value`, or the
//! PurRDF RDF 1.2 extension controls, and context compaction never rewrites an
//! arbitrary JSON tree.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use serde::ser::{Error as _, SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value as JsonValue;

use super::{
    CompiledJsonLdContext, JsonLdContainer, JsonLdDirection, JsonLdNullable, JsonLdTermDefinition,
    JsonLdTermSelection, JsonLdTermSelectionKind, JsonLdTypeMapping, RdfDiagnostic, cmp_value,
    decode, to_json_object,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Document {
    pub(super) default_nodes: Vec<Node>,
    pub(super) named_graphs: Vec<NamedGraph>,
}

impl Document {
    pub(super) fn write_expanded_json<W: Write>(
        &self,
        writer: W,
        context: &JsonValue,
    ) -> Result<(), RdfDiagnostic> {
        serde_json::to_writer_pretty(
            writer,
            &ExpandedDocument {
                document: self,
                context,
            },
        )
        .map_err(|source| decode(format!("JSON-LD serialization: {source}")))
    }

    pub(super) fn write_compacted_json<W: Write>(
        &self,
        writer: W,
        context: &CompiledJsonLdContext,
    ) -> Result<(), RdfDiagnostic> {
        let mut prepared = self.clone();
        let graph_index_plans = plan_graph_index_containers(&prepared, context)?;
        consume_graph_index_metadata(&mut prepared.default_nodes, None, &graph_index_plans);
        for graph in &mut prepared.named_graphs {
            consume_graph_index_metadata(
                &mut graph.nodes,
                Some(graph.id.as_str()),
                &graph_index_plans,
            );
        }
        relocate_reverse_properties(&mut prepared.default_nodes, context)?;
        for graph in &mut prepared.named_graphs {
            relocate_reverse_properties(&mut graph.nodes, context)?;
        }
        serde_json::to_writer_pretty(
            writer,
            &CompactedDocument {
                document: &prepared,
                context,
                graph_index_plans: &graph_index_plans,
            },
        )
        .map_err(|source| decode(format!("JSON-LD serialization: {source}")))
    }
}

struct ExpandedDocument<'a> {
    document: &'a Document,
    context: &'a JsonValue,
}

impl Serialize for ExpandedDocument<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let has_graph =
            !self.document.default_nodes.is_empty() || !self.document.named_graphs.is_empty();
        let mut document = serializer.serialize_map(Some(if has_graph { 2 } else { 1 }))?;
        document.serialize_entry("@context", self.context)?;
        if has_graph {
            document.serialize_entry("@graph", &ExpandedGraph(self.document))?;
        }
        document.end()
    }
}

struct ExpandedGraph<'a>(&'a Document);

impl Serialize for ExpandedGraph<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut graph = serializer
            .serialize_seq(Some(self.0.default_nodes.len() + self.0.named_graphs.len()))?;
        let mut nodes = self.0.default_nodes.iter().peekable();
        let mut named = self.0.named_graphs.iter().peekable();
        while nodes.peek().is_some() || named.peek().is_some() {
            if named
                .peek()
                .is_some_and(|graph| nodes.peek().is_none_or(|node| graph.id < node.id))
            {
                graph.serialize_element(&ExpandedNamedGraph(
                    named.next().expect("peeked named graph"),
                ))?;
            } else {
                graph.serialize_element(
                    &nodes
                        .next()
                        .expect("one expanded entry remains")
                        .expanded_json(),
                )?;
            }
        }
        graph.end()
    }
}

struct ExpandedNamedGraph<'a>(&'a NamedGraph);

impl Serialize for ExpandedNamedGraph<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut graph = serializer.serialize_map(Some(2))?;
        graph.serialize_entry("@graph", &ExpandedNodes(&self.0.nodes))?;
        graph.serialize_entry("@id", &self.0.id)?;
        graph.end()
    }
}

struct ExpandedNodes<'a>(&'a [Node]);

impl Serialize for ExpandedNodes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut nodes = serializer.serialize_seq(Some(self.0.len()))?;
        for node in self.0 {
            nodes.serialize_element(&node.expanded_json())?;
        }
        nodes.end()
    }
}

struct CompactedDocument<'a> {
    document: &'a Document,
    context: &'a CompiledJsonLdContext,
    graph_index_plans: &'a GraphIndexPlans,
}

impl Serialize for CompactedDocument<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let has_graph =
            !self.document.default_nodes.is_empty() || !self.document.named_graphs.is_empty();
        let mut document = serializer.serialize_map(Some(if has_graph { 2 } else { 1 }))?;
        document.serialize_entry("@context", self.context.canonical_context())?;
        if has_graph {
            let graph_key = compact_keyword(self.context, "@graph").map_err(S::Error::custom)?;
            document.serialize_entry(
                &graph_key,
                &CompactedGraph {
                    document: self.document,
                    context: self.context,
                    graph_index_plans: self.graph_index_plans,
                },
            )?;
        }
        document.end()
    }
}

struct CompactedGraph<'a> {
    document: &'a Document,
    context: &'a CompiledJsonLdContext,
    graph_index_plans: &'a GraphIndexPlans,
}

impl Serialize for CompactedGraph<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let named_by_id: BTreeMap<&str, &NamedGraph> = self
            .document
            .named_graphs
            .iter()
            .map(|named| (named.id.as_str(), named))
            .collect();
        let embedded_graphs = RefCell::new(BTreeSet::new());
        let mut graph = serializer.serialize_seq(None)?;
        for node in &self.document.default_nodes {
            let mut compacted = node
                .compacted_json(self.context)
                .map_err(S::Error::custom)?;
            apply_graph_containers(
                node,
                None,
                &mut compacted,
                self.context,
                self.document,
                &named_by_id,
                &mut embedded_graphs.borrow_mut(),
                self.graph_index_plans,
            )
            .map_err(S::Error::custom)?;
            graph.serialize_element(&compacted)?;
        }
        for named in &self.document.named_graphs {
            if embedded_graphs.borrow().contains(&named.id) {
                continue;
            }
            embedded_graphs.borrow_mut().insert(named.id.clone());
            graph.serialize_element(&CompactedNamedGraph {
                graph: named,
                document: self.document,
                context: self.context,
                graph_index_plans: self.graph_index_plans,
                embedded_graphs: &embedded_graphs,
            })?;
        }
        graph.end()
    }
}

struct CompactedNamedGraph<'a> {
    graph: &'a NamedGraph,
    document: &'a Document,
    context: &'a CompiledJsonLdContext,
    graph_index_plans: &'a GraphIndexPlans,
    embedded_graphs: &'a RefCell<BTreeSet<String>>,
}

impl Serialize for CompactedNamedGraph<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut graph = serializer.serialize_map(Some(2))?;
        let id_key = compact_keyword(self.context, "@id").map_err(S::Error::custom)?;
        let graph_key = compact_keyword(self.context, "@graph").map_err(S::Error::custom)?;
        let id = compact_id(self.context, &self.graph.id, false).map_err(S::Error::custom)?;
        let nodes = CompactedNodes {
            nodes: &self.graph.nodes,
            scope: Some(self.graph.id.as_str()),
            document: self.document,
            context: self.context,
            graph_index_plans: self.graph_index_plans,
            embedded_graphs: self.embedded_graphs,
        };
        if graph_key < id_key {
            graph.serialize_entry(&graph_key, &nodes)?;
            graph.serialize_entry(&id_key, &id)?;
        } else {
            graph.serialize_entry(&id_key, &id)?;
            graph.serialize_entry(&graph_key, &nodes)?;
        }
        graph.end()
    }
}

struct CompactedNodes<'a> {
    nodes: &'a [Node],
    scope: Option<&'a str>,
    document: &'a Document,
    context: &'a CompiledJsonLdContext,
    graph_index_plans: &'a GraphIndexPlans,
    embedded_graphs: &'a RefCell<BTreeSet<String>>,
}

impl Serialize for CompactedNodes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let named_by_id: BTreeMap<&str, &NamedGraph> = self
            .document
            .named_graphs
            .iter()
            .map(|named| (named.id.as_str(), named))
            .collect();
        let mut nodes = serializer.serialize_seq(Some(self.nodes.len()))?;
        for node in self.nodes {
            let mut compacted = node
                .compacted_json(self.context)
                .map_err(S::Error::custom)?;
            apply_graph_containers(
                node,
                self.scope,
                &mut compacted,
                self.context,
                self.document,
                &named_by_id,
                &mut self.embedded_graphs.borrow_mut(),
                self.graph_index_plans,
            )
            .map_err(S::Error::custom)?;
            nodes.serialize_element(&compacted)?;
        }
        nodes.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NamedGraph {
    pub(super) id: String,
    pub(super) nodes: Vec<Node>,
}

impl NamedGraph {
    fn compacted_contents(
        &self,
        context: &CompiledJsonLdContext,
        document: &Document,
        named_by_id: &BTreeMap<&str, &Self>,
        embedded_graphs: &mut BTreeSet<String>,
        graph_index_plans: &GraphIndexPlans,
        force_array: bool,
    ) -> Result<JsonValue, RdfDiagnostic> {
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                let mut compacted = node.compacted_json(context)?;
                apply_graph_containers(
                    node,
                    Some(self.id.as_str()),
                    &mut compacted,
                    context,
                    document,
                    named_by_id,
                    embedded_graphs,
                    graph_index_plans,
                )?;
                Ok(compacted)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(if !force_array && let [only] = nodes.as_slice() {
            only.clone()
        } else {
            JsonValue::Array(nodes)
        })
    }
}

type GraphIndexPlans = BTreeMap<(Option<String>, String, String), GraphIndexPlan>;

#[derive(Debug, Clone)]
struct GraphIndexPlan {
    index_mapping: String,
    keys_by_graph: BTreeMap<String, String>,
}

fn plan_graph_index_containers(
    document: &Document,
    context: &CompiledJsonLdContext,
) -> Result<GraphIndexPlans, RdfDiagnostic> {
    let named_by_id: BTreeMap<&str, &NamedGraph> = document
        .named_graphs
        .iter()
        .map(|graph| (graph.id.as_str(), graph))
        .collect();
    let references = document_id_reference_counts(document);
    let selection = JsonLdTermSelection::new(
        [
            vec![
                JsonLdContainer::Graph,
                JsonLdContainer::Index,
                JsonLdContainer::Set,
            ],
            vec![JsonLdContainer::Graph, JsonLdContainer::Index],
        ],
        JsonLdTermSelectionKind::Type,
        ["@none", "@any"],
    );
    let mut plans = BTreeMap::new();
    plan_graph_index_scope(
        None,
        &document.default_nodes,
        &named_by_id,
        &references,
        context,
        &selection,
        &mut plans,
    )?;
    for graph in &document.named_graphs {
        plan_graph_index_scope(
            Some(graph.id.as_str()),
            &graph.nodes,
            &named_by_id,
            &references,
            context,
            &selection,
            &mut plans,
        )?;
    }
    Ok(plans)
}

#[allow(
    clippy::too_many_arguments,
    reason = "graph-index planning keeps scope, carrier, context, and destination explicit"
)]
fn plan_graph_index_scope(
    scope: Option<&str>,
    nodes: &[Node],
    named_by_id: &BTreeMap<&str, &NamedGraph>,
    references: &BTreeMap<String, usize>,
    context: &CompiledJsonLdContext,
    selection: &JsonLdTermSelection,
    plans: &mut GraphIndexPlans,
) -> Result<(), RdfDiagnostic> {
    let metadata_by_id: BTreeMap<&str, &Node> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    for node in nodes {
        for (property, values) in &node.properties {
            if values.is_empty() || values.iter().any(|value| {
                !value.annotations.is_empty()
                    || !matches!(&value.term, Term::Id(id) if named_by_id.contains_key(id.as_str()))
            }) {
                continue;
            }
            let term = context.compact_iri_with_selection(property, true, Some(selection))?;
            let Some(definition) = context.term(&term).filter(|definition| {
                definition.containers().contains(&JsonLdContainer::Graph)
                    && definition.containers().contains(&JsonLdContainer::Index)
            }) else {
                continue;
            };
            let Some(index_mapping) = definition.index_mapping() else {
                continue;
            };
            let mut keys_by_graph = BTreeMap::new();
            let mut lossless = true;
            for value in values {
                let Term::Id(graph_id) = &value.term else {
                    unreachable!("graph-container eligibility checked node references")
                };
                if !graph_id.starts_with("_:") || references.get(graph_id) != Some(&1) {
                    lossless = false;
                    break;
                }
                let key = match metadata_by_id.get(graph_id.as_str()) {
                    None => "@none".to_owned(),
                    Some(metadata)
                        if metadata.types.is_empty()
                            && metadata.reverse_properties.is_empty()
                            && metadata.properties.len() == 1 =>
                    {
                        let Some(index_values) = metadata.properties.get(index_mapping) else {
                            lossless = false;
                            break;
                        };
                        let [index_value] = index_values.as_slice() else {
                            lossless = false;
                            break;
                        };
                        let Term::Literal(index) = &index_value.term else {
                            lossless = false;
                            break;
                        };
                        if !index_value.annotations.is_empty()
                            || index.datatype.is_some()
                            || index.language.is_some()
                            || index.direction.is_some()
                        {
                            lossless = false;
                            break;
                        }
                        index.lexical.clone()
                    }
                    Some(_) => {
                        lossless = false;
                        break;
                    }
                };
                keys_by_graph.insert(graph_id.clone(), key);
            }
            if lossless {
                plans.insert(
                    (scope.map(str::to_owned), node.id.clone(), property.clone()),
                    GraphIndexPlan {
                        index_mapping: index_mapping.to_owned(),
                        keys_by_graph,
                    },
                );
            }
        }
    }
    Ok(())
}

fn consume_graph_index_metadata(
    nodes: &mut Vec<Node>,
    scope: Option<&str>,
    plans: &GraphIndexPlans,
) {
    let consumed: BTreeSet<&str> = plans
        .iter()
        .filter(|((plan_scope, _, _), _)| plan_scope.as_deref() == scope)
        .flat_map(|(_, plan)| {
            plan.keys_by_graph
                .iter()
                .filter_map(|(graph, key)| (key != "@none").then_some(graph.as_str()))
        })
        .collect();
    nodes.retain(|node| !consumed.contains(node.id.as_str()));
}

fn document_id_reference_counts(document: &Document) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for node in document.default_nodes.iter().chain(
        document
            .named_graphs
            .iter()
            .flat_map(|graph| graph.nodes.iter()),
    ) {
        count_node_term_ids(node, &mut counts);
    }
    counts
}

fn count_node_term_ids(node: &Node, counts: &mut BTreeMap<String, usize>) {
    for values in node
        .properties
        .values()
        .chain(node.reverse_properties.values())
    {
        for value in values {
            count_value_term_ids(value, counts);
        }
    }
}

fn count_value_term_ids(value: &Value, counts: &mut BTreeMap<String, usize>) {
    count_term_ids(&value.term, counts);
    for annotation in &value.annotations {
        *counts.entry(annotation.id.clone()).or_default() += 1;
        count_node_term_ids(annotation, counts);
    }
}

fn count_term_ids(term: &Term, counts: &mut BTreeMap<String, usize>) {
    match term {
        Term::Id(id) => *counts.entry(id.clone()).or_default() += 1,
        Term::Triple(triple) => {
            count_term_ids(&triple.subject, counts);
            count_term_ids(&triple.object, counts);
        }
        Term::List(values) => {
            for value in values {
                count_value_term_ids(value, counts);
            }
        }
        Term::Literal(_) => {}
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "graph-container emission keeps scope, carrier, context, and state explicit"
)]
fn apply_graph_containers(
    node: &Node,
    scope: Option<&str>,
    compacted: &mut JsonValue,
    context: &CompiledJsonLdContext,
    document: &Document,
    named_by_id: &BTreeMap<&str, &NamedGraph>,
    embedded_graphs: &mut BTreeSet<String>,
    graph_index_plans: &GraphIndexPlans,
) -> Result<(), RdfDiagnostic> {
    for (iri, values) in &node.properties {
        if values.is_empty()
            || values.iter().any(|value| {
                !value.annotations.is_empty()
                    || !matches!(&value.term, Term::Id(id) if named_by_id.contains_key(id.as_str()))
            })
        {
            continue;
        }
        let graph_selection = JsonLdTermSelection::new(
            [
                vec![
                    JsonLdContainer::Graph,
                    JsonLdContainer::Index,
                    JsonLdContainer::Set,
                ],
                vec![JsonLdContainer::Graph, JsonLdContainer::Index],
                vec![
                    JsonLdContainer::Graph,
                    JsonLdContainer::Id,
                    JsonLdContainer::Set,
                ],
                vec![JsonLdContainer::Graph, JsonLdContainer::Id],
                vec![JsonLdContainer::Graph, JsonLdContainer::Set],
                vec![JsonLdContainer::Graph],
            ],
            JsonLdTermSelectionKind::Type,
            ["@none", "@any"],
        );
        let term = context.compact_iri_with_selection(iri, true, Some(&graph_selection))?;
        let Some(definition) = context
            .term(&term)
            .filter(|definition| definition.containers().contains(&JsonLdContainer::Graph))
        else {
            continue;
        };
        let has_id_map = definition.containers().contains(&JsonLdContainer::Id);
        let has_index_map = definition.containers().contains(&JsonLdContainer::Index);
        let index_plan =
            graph_index_plans.get(&(scope.map(str::to_owned), node.id.clone(), iri.clone()));
        if has_index_map && index_plan.is_none() {
            continue;
        }
        if !has_id_map
            && values
                .iter()
                .any(|value| matches!(&value.term, Term::Id(id) if !id.starts_with("_:")))
        {
            continue;
        }
        if values
            .iter()
            .any(|value| matches!(&value.term, Term::Id(id) if embedded_graphs.contains(id)))
        {
            // A graph already embedded on the current deterministic walk remains a
            // regular node reference here.  This both preserves repeated references
            // and prevents recursive graph-container cycles.
            continue;
        }
        for value in values {
            let Term::Id(id) = &value.term else {
                unreachable!("graph-container eligibility checked node references")
            };
            embedded_graphs.insert(id.clone());
        }
        let scoped = context.scoped_context(&term)?;
        let value_context = scoped.as_ref().unwrap_or(context);
        let force_array = definition.containers().contains(&JsonLdContainer::Set);
        let graph_value = if let Some(index_plan) = index_plan {
            debug_assert_eq!(
                definition.index_mapping(),
                Some(index_plan.index_mapping.as_str())
            );
            let mut map: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
            for value in values {
                let Term::Id(id) = &value.term else {
                    unreachable!("graph-container eligibility checked node references")
                };
                let named = named_by_id
                    .get(id.as_str())
                    .expect("graph-container eligibility checked named graph");
                map.entry(
                    index_plan
                        .keys_by_graph
                        .get(id)
                        .expect("graph-index plan covers every referenced graph")
                        .clone(),
                )
                .or_default()
                .push(named.compacted_contents(
                    value_context,
                    document,
                    named_by_id,
                    embedded_graphs,
                    graph_index_plans,
                    force_array,
                )?);
            }
            to_json_object(
                map.into_iter()
                    .map(|(index, values)| {
                        let value = if !force_array && let [only] = values.as_slice() {
                            only.clone()
                        } else {
                            JsonValue::Array(values)
                        };
                        (index, value)
                    })
                    .collect(),
            )
        } else if has_id_map {
            let mut map = BTreeMap::new();
            for value in values {
                let Term::Id(id) = &value.term else {
                    unreachable!("graph-container eligibility checked node references")
                };
                let named = named_by_id
                    .get(id.as_str())
                    .expect("graph-container eligibility checked named graph");
                insert_unique(
                    &mut map,
                    &compact_id(value_context, id, false)?,
                    named.compacted_contents(
                        value_context,
                        document,
                        named_by_id,
                        embedded_graphs,
                        graph_index_plans,
                        force_array,
                    )?,
                )?;
            }
            to_json_object(map)
        } else {
            let mut bodies = Vec::new();
            for value in values {
                let Term::Id(id) = &value.term else {
                    unreachable!("graph-container eligibility checked node references")
                };
                let named = named_by_id
                    .get(id.as_str())
                    .expect("graph-container eligibility checked named graph");
                bodies.push(named.compacted_contents(
                    value_context,
                    document,
                    named_by_id,
                    embedded_graphs,
                    graph_index_plans,
                    force_array,
                )?);
            }
            if !force_array && let [only] = bodies.as_slice() {
                only.clone()
            } else {
                JsonValue::Array(bodies)
            }
        };

        remove_regular_property(node, compacted, context, iri, values)?;
        insert_compacted_property(compacted, context, &term, definition, graph_value)?;
    }
    Ok(())
}

fn remove_regular_property(
    _node: &Node,
    compacted: &mut JsonValue,
    context: &CompiledJsonLdContext,
    iri: &str,
    values: &[Value],
) -> Result<(), RdfDiagnostic> {
    let object = compacted
        .as_object_mut()
        .expect("a compacted carrier node is an object");
    let mut terms = BTreeSet::new();
    for value in values {
        terms.insert(context.compact_iri_with_selection(iri, true, Some(&value.selection()))?);
    }
    for term in terms {
        if let Some(nest) = context.term(&term).and_then(JsonLdTermDefinition::nest) {
            let nest = if nest == "@nest" {
                compact_keyword(context, "@nest")?
            } else {
                nest.to_owned()
            };
            if let Some(nested) = object.get_mut(&nest).and_then(JsonValue::as_object_mut) {
                nested.remove(&term);
                if nested.is_empty() {
                    object.remove(&nest);
                }
            }
        } else {
            object.remove(&term);
        }
    }
    Ok(())
}

fn insert_compacted_property(
    compacted: &mut JsonValue,
    context: &CompiledJsonLdContext,
    term: &str,
    definition: &JsonLdTermDefinition,
    value: JsonValue,
) -> Result<(), RdfDiagnostic> {
    let object = compacted
        .as_object_mut()
        .expect("a compacted carrier node is an object");
    if let Some(nest) = definition.nest() {
        let nest = if nest == "@nest" {
            compact_keyword(context, "@nest")?
        } else {
            nest.to_owned()
        };
        let nested = object
            .entry(nest)
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| decode("JSON-LD @nest target is not an object"))?;
        if nested.insert(term.to_owned(), value).is_some() {
            return Err(decode(format!(
                "JSON-LD compaction produced duplicate member `{term}`"
            )));
        }
    } else if object.insert(term.to_owned(), value).is_some() {
        return Err(decode(format!(
            "JSON-LD compaction produced duplicate member `{term}`"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) id: String,
    pub(super) types: Vec<String>,
    pub(super) properties: BTreeMap<String, Vec<Value>>,
    pub(super) reverse_properties: BTreeMap<String, Vec<Value>>,
}

impl Node {
    pub(super) fn new(id: String) -> Self {
        Self {
            id,
            types: Vec::new(),
            properties: BTreeMap::new(),
            reverse_properties: BTreeMap::new(),
        }
    }

    pub(super) fn sort_values(&mut self) {
        self.types.sort();
        for values in self.properties.values_mut() {
            values.sort_by(|left, right| cmp_value(&left.expanded_json(), &right.expanded_json()));
        }
        for values in self.reverse_properties.values_mut() {
            values.sort_by(|left, right| cmp_value(&left.expanded_json(), &right.expanded_json()));
        }
    }

    pub(super) fn expanded_json(&self) -> JsonValue {
        let mut node = BTreeMap::new();
        node.insert("@id".to_owned(), JsonValue::String(self.id.clone()));
        if !self.types.is_empty() {
            node.insert(
                "@type".to_owned(),
                JsonValue::Array(
                    self.types
                        .iter()
                        .map(|iri| {
                            to_json_object(BTreeMap::from([(
                                "@id".to_owned(),
                                JsonValue::String(iri.clone()),
                            )]))
                        })
                        .collect(),
                ),
            );
        }
        for (property, values) in &self.properties {
            let values: Vec<JsonValue> = values.iter().map(Value::expanded_json).collect();
            let value = if let [only] = values.as_slice() {
                only.clone()
            } else {
                JsonValue::Array(values)
            };
            node.insert(property.clone(), value);
        }
        if !self.reverse_properties.is_empty() {
            let mut reverse = BTreeMap::new();
            for (property, values) in &self.reverse_properties {
                let values: Vec<JsonValue> = values.iter().map(Value::expanded_json).collect();
                reverse.insert(
                    property.clone(),
                    if let [only] = values.as_slice() {
                        only.clone()
                    } else {
                        JsonValue::Array(values)
                    },
                );
            }
            node.insert("@reverse".to_owned(), to_json_object(reverse));
        }
        to_json_object(node)
    }

    fn compacted_json(&self, context: &CompiledJsonLdContext) -> Result<JsonValue, RdfDiagnostic> {
        let mut node = BTreeMap::new();
        insert_unique(
            &mut node,
            &compact_keyword(context, "@id")?,
            JsonValue::String(compact_id(context, &self.id, false)?),
        )?;
        if !self.types.is_empty() {
            let type_key = compact_keyword(context, "@type")?;
            let types = self
                .types
                .iter()
                .map(|iri| context.compact_iri(iri, true).map(JsonValue::String))
                .collect::<Result<Vec<_>, _>>()?;
            insert_unique(
                &mut node,
                &type_key,
                if let [only] = types.as_slice() {
                    only.clone()
                } else {
                    JsonValue::Array(types)
                },
            )?;
        }

        let mut nested: BTreeMap<String, BTreeMap<String, JsonValue>> = BTreeMap::new();
        for (iri, values) in &self.properties {
            let mut by_term: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
            for value in values {
                let selection = value.selection();
                let term = context.compact_iri_with_selection(iri, true, Some(&selection))?;
                by_term.entry(term).or_default().push(value);
            }
            for (term, values) in by_term {
                let definition = context.term(&term);
                if definition.is_some_and(JsonLdTermDefinition::is_reverse_property) {
                    return Err(decode(format!(
                        "forward property `{iri}` cannot be emitted through reverse term `{term}`"
                    )));
                }
                let scoped = context.scoped_context(&term)?;
                let value_context = scoped.as_ref().unwrap_or(context);
                let compacted = compact_property_values(value_context, &term, definition, &values)?;
                if let Some(nest) = definition.and_then(JsonLdTermDefinition::nest) {
                    let nest_key = if nest == "@nest" {
                        compact_keyword(context, "@nest")?
                    } else {
                        nest.to_owned()
                    };
                    insert_unique(nested.entry(nest_key).or_default(), &term, compacted)?;
                } else {
                    insert_unique(&mut node, &term, compacted)?;
                }
            }
        }
        for (iri, values) in &self.reverse_properties {
            let selection = reverse_selection();
            let term = context.compact_iri_with_selection(iri, true, Some(&selection))?;
            let definition = context.term(&term).filter(|definition| {
                definition.is_reverse_property() && definition.iri_mapping() == Some(iri)
            });
            let Some(definition) = definition else {
                return Err(decode(format!(
                    "reverse property `{iri}` has no lossless reverse term"
                )));
            };
            let values = values.iter().collect::<Vec<_>>();
            insert_unique(
                &mut node,
                &term,
                compact_property_values(context, &term, Some(definition), &values)?,
            )?;
        }
        for (nest, values) in nested {
            insert_unique(&mut node, &nest, to_json_object(values))?;
        }
        Ok(to_json_object(node))
    }
}

fn reverse_selection() -> JsonLdTermSelection {
    JsonLdTermSelection::new(
        [vec![JsonLdContainer::Set], Vec::new()],
        JsonLdTermSelectionKind::Type,
        ["@reverse", "@none", "@any"],
    )
}

fn relocate_reverse_properties(
    nodes: &mut Vec<Node>,
    context: &CompiledJsonLdContext,
) -> Result<(), RdfDiagnostic> {
    #[derive(Debug)]
    struct Move {
        source: String,
        property: String,
        original: Value,
        target: String,
        reversed: Value,
    }

    let selection = reverse_selection();
    let mut moves = Vec::new();
    for node in nodes.iter() {
        for (property, values) in &node.properties {
            let term = context.compact_iri_with_selection(property, true, Some(&selection))?;
            let has_reverse_term = context.term(&term).is_some_and(|definition| {
                definition.is_reverse_property() && definition.iri_mapping() == Some(property)
            });
            if !has_reverse_term {
                continue;
            }
            for value in values {
                let Term::Id(target) = &value.term else {
                    continue;
                };
                let mut reversed = value.clone();
                reversed.term = Term::Id(node.id.clone());
                moves.push(Move {
                    source: node.id.clone(),
                    property: property.clone(),
                    original: value.clone(),
                    target: target.clone(),
                    reversed,
                });
            }
        }
    }

    if moves.is_empty() {
        return Ok(());
    }
    let mut by_id: BTreeMap<String, Node> = nodes
        .drain(..)
        .map(|node| (node.id.clone(), node))
        .collect();
    for movement in moves {
        if let Some(source) = by_id.get_mut(&movement.source)
            && let Some(values) = source.properties.get_mut(&movement.property)
            && let Some(index) = values.iter().position(|value| value == &movement.original)
        {
            values.remove(index);
            if values.is_empty() {
                source.properties.remove(&movement.property);
            }
        }
        by_id
            .entry(movement.target.clone())
            .or_insert_with(|| Node::new(movement.target))
            .reverse_properties
            .entry(movement.property)
            .or_default()
            .push(movement.reversed);
    }
    for node in by_id.values_mut() {
        node.sort_values();
    }
    *nodes = by_id.into_values().collect();
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Value {
    pub(super) term: Term,
    pub(super) annotations: Vec<Node>,
}

impl Value {
    pub(super) fn plain(term: Term) -> Self {
        Self {
            term,
            annotations: Vec::new(),
        }
    }

    pub(super) fn expanded_json(&self) -> JsonValue {
        let mut value = self.term.expanded_json();
        if !self.annotations.is_empty() {
            let annotations: Vec<JsonValue> =
                self.annotations.iter().map(Node::expanded_json).collect();
            let annotation = if let [only] = annotations.as_slice() {
                only.clone()
            } else {
                JsonValue::Array(annotations)
            };
            value
                .as_object_mut()
                .expect("every typed carrier term emits a JSON object")
                .insert("@annotation".to_owned(), annotation);
        }
        value
    }

    fn selection(&self) -> JsonLdTermSelection {
        let containers = match &self.term {
            Term::List(_) => vec![
                vec![JsonLdContainer::List, JsonLdContainer::Set],
                vec![JsonLdContainer::List],
                vec![JsonLdContainer::Set],
                Vec::new(),
            ],
            Term::Id(_) if self.annotations.is_empty() => vec![
                vec![JsonLdContainer::Id, JsonLdContainer::Set],
                vec![JsonLdContainer::Id],
                vec![JsonLdContainer::Set],
                Vec::new(),
            ],
            Term::Literal(literal) if literal.datatype.is_none() && self.annotations.is_empty() => {
                vec![
                    vec![JsonLdContainer::Language, JsonLdContainer::Set],
                    vec![JsonLdContainer::Language],
                    vec![JsonLdContainer::Set],
                    Vec::new(),
                ]
            }
            _ => vec![vec![JsonLdContainer::Set], Vec::new()],
        };
        let (kind, preferred) = match &self.term {
            Term::List(values) => common_list_preferences(values),
            _ => self.shape_preferences(),
        };
        JsonLdTermSelection::new(containers, kind, preferred)
    }

    fn shape_preferences(&self) -> (JsonLdTermSelectionKind, Vec<String>) {
        if !self.annotations.is_empty() {
            return (
                JsonLdTermSelectionKind::Type,
                vec!["@none".to_owned(), "@any".to_owned()],
            );
        }
        match &self.term {
            Term::Id(_) | Term::Triple(_) => (
                JsonLdTermSelectionKind::Type,
                ["@id", "@vocab", "@none", "@any"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            ),
            Term::List(_) => (
                JsonLdTermSelectionKind::Type,
                vec!["@none".to_owned(), "@any".to_owned()],
            ),
            Term::Literal(literal) => literal_preferences(literal),
        }
    }
}

fn common_list_preferences(values: &[Value]) -> (JsonLdTermSelectionKind, Vec<String>) {
    let Some(first) = values.first() else {
        return (
            JsonLdTermSelectionKind::Type,
            ["@id", "@none", "@any"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        );
    };
    let candidate = first.shape_preferences();
    let common = candidate.1.first();
    if common.is_some()
        && values.iter().skip(1).all(|value| {
            let other = value.shape_preferences();
            other.0 == candidate.0 && other.1.first() == common
        })
    {
        candidate
    } else {
        (
            JsonLdTermSelectionKind::Type,
            vec!["@none".to_owned(), "@any".to_owned()],
        )
    }
}

fn literal_preferences(literal: &Literal) -> (JsonLdTermSelectionKind, Vec<String>) {
    if let Some(datatype) = &literal.datatype {
        let preferred =
            if datatype == super::RDF_JSON && canonical_json_value(&literal.lexical).is_some() {
                vec![
                    "@json".to_owned(),
                    datatype.clone(),
                    "@none".to_owned(),
                    "@any".to_owned(),
                ]
            } else {
                vec![datatype.clone(), "@none".to_owned(), "@any".to_owned()]
            };
        return (JsonLdTermSelectionKind::Type, preferred);
    }
    let mut preferred = Vec::new();
    match (&literal.language, &literal.direction) {
        (Some(language), Some(direction)) => {
            preferred.push(format!("{}_{}", language.to_ascii_lowercase(), direction));
            preferred.push(language.to_ascii_lowercase());
            preferred.push(format!("_{direction}"));
        }
        (Some(language), None) => preferred.push(language.to_ascii_lowercase()),
        (None, Some(direction)) => preferred.push(format!("_{direction}")),
        (None, None) => preferred.push("@null".to_owned()),
    }
    preferred.push("@none".to_owned());
    preferred.push("@any".to_owned());
    (JsonLdTermSelectionKind::Language, preferred)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Term {
    Id(String),
    Literal(Literal),
    Triple(Box<Triple>),
    List(Vec<Value>),
}

impl Term {
    fn expanded_json(&self) -> JsonValue {
        match self {
            Self::Id(id) => to_json_object(BTreeMap::from([(
                "@id".to_owned(),
                JsonValue::String(id.clone()),
            )])),
            Self::Literal(literal) => literal.expanded_json(),
            Self::Triple(triple) => to_json_object(BTreeMap::from([(
                "@triple".to_owned(),
                triple.expanded_json(),
            )])),
            Self::List(values) => to_json_object(BTreeMap::from([(
                "@list".to_owned(),
                JsonValue::Array(values.iter().map(Value::expanded_json).collect()),
            )])),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Literal {
    pub(super) lexical: String,
    pub(super) datatype: Option<String>,
    pub(super) language: Option<String>,
    pub(super) direction: Option<String>,
}

impl Literal {
    fn expanded_json(&self) -> JsonValue {
        let mut literal = BTreeMap::new();
        literal.insert("@value".to_owned(), JsonValue::String(self.lexical.clone()));
        if let Some(language) = &self.language {
            literal.insert("@language".to_owned(), JsonValue::String(language.clone()));
        }
        if let Some(direction) = &self.direction {
            literal.insert(
                "@direction".to_owned(),
                JsonValue::String(direction.clone()),
            );
        }
        if let Some(datatype) = &self.datatype {
            literal.insert("@type".to_owned(), JsonValue::String(datatype.clone()));
        }
        to_json_object(literal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Triple {
    pub(super) subject: Box<Term>,
    pub(super) predicate: String,
    pub(super) object: Box<Term>,
}

impl Triple {
    fn expanded_json(&self) -> JsonValue {
        to_json_object(BTreeMap::from([
            ("@object".to_owned(), self.object.expanded_json()),
            (
                "@predicate".to_owned(),
                JsonValue::String(self.predicate.clone()),
            ),
            ("@subject".to_owned(), self.subject.expanded_json()),
        ]))
    }
}

fn compact_property_values(
    context: &CompiledJsonLdContext,
    term: &str,
    definition: Option<&JsonLdTermDefinition>,
    values: &[&Value],
) -> Result<JsonValue, RdfDiagnostic> {
    if definition.is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Id)) {
        return compact_id_map(context, term, definition, values);
    }
    if definition
        .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Language))
    {
        return compact_language_map(context, term, definition, values);
    }
    let compacted = values
        .iter()
        .map(|value| value.compacted_json(context, definition))
        .collect::<Result<Vec<_>, _>>()?;
    let force_array = definition
        .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Set));
    Ok(if !force_array && let [only] = compacted.as_slice() {
        only.clone()
    } else {
        JsonValue::Array(compacted)
    })
}

fn compact_id_map(
    context: &CompiledJsonLdContext,
    term: &str,
    definition: Option<&JsonLdTermDefinition>,
    values: &[&Value],
) -> Result<JsonValue, RdfDiagnostic> {
    let definition = definition.expect("@id container requires a term definition");
    let mut id_map: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
    for value in values {
        let Term::Id(id) = &value.term else {
            return Err(decode(format!(
                "term `{term}` has an @id container but selected a non-node value"
            )));
        };
        if !value.annotations.is_empty() {
            return Err(decode(format!(
                "term `{term}` @id container would lose statement annotations"
            )));
        }
        id_map
            .entry(compact_id(context, id, false)?)
            .or_default()
            .push(to_json_object(BTreeMap::new()));
    }
    let force_array = definition.containers().contains(&JsonLdContainer::Set);
    Ok(to_json_object(
        id_map
            .into_iter()
            .map(|(id, values)| {
                let value = if !force_array && let [only] = values.as_slice() {
                    only.clone()
                } else {
                    JsonValue::Array(values)
                };
                (id, value)
            })
            .collect(),
    ))
}

fn compact_language_map(
    context: &CompiledJsonLdContext,
    term: &str,
    definition: Option<&JsonLdTermDefinition>,
    values: &[&Value],
) -> Result<JsonValue, RdfDiagnostic> {
    let definition = definition.expect("language container requires a term definition");
    let mut language_map: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
    for value in values {
        let Term::Literal(literal) = &value.term else {
            return Err(decode(format!(
                "term `{term}` has a language container but selected a non-literal value"
            )));
        };
        if !value.annotations.is_empty()
            || literal.datatype.is_some()
            || literal.direction.as_deref()
                != match definition.direction_mapping() {
                    Some(JsonLdNullable::Null) => None,
                    Some(JsonLdNullable::Value(JsonLdDirection::LeftToRight)) => Some("ltr"),
                    Some(JsonLdNullable::Value(JsonLdDirection::RightToLeft)) => Some("rtl"),
                    None => context.default_direction().map(JsonLdDirection::as_str),
                }
        {
            return Err(decode(format!(
                "term `{term}` language container would lose literal metadata"
            )));
        }
        let language = literal.language.as_deref().unwrap_or("@none").to_owned();
        language_map
            .entry(language)
            .or_default()
            .push(JsonValue::String(literal.lexical.clone()));
    }
    let force_array = definition.containers().contains(&JsonLdContainer::Set);
    Ok(to_json_object(
        language_map
            .into_iter()
            .map(|(language, values)| {
                let value = if !force_array && let [only] = values.as_slice() {
                    only.clone()
                } else {
                    JsonValue::Array(values)
                };
                (language, value)
            })
            .collect(),
    ))
}

impl Value {
    fn compacted_json(
        &self,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<JsonValue, RdfDiagnostic> {
        let mut compacted = match &self.term {
            Term::Id(id) => {
                let coercion = definition.and_then(JsonLdTermDefinition::type_mapping);
                if self.annotations.is_empty()
                    && matches!(
                        coercion,
                        Some(JsonLdTypeMapping::Id | JsonLdTypeMapping::Vocab)
                    )
                {
                    let vocab = matches!(coercion, Some(JsonLdTypeMapping::Vocab));
                    JsonValue::String(compact_id(context, id, vocab)?)
                } else {
                    to_json_object(BTreeMap::from([(
                        compact_keyword(context, "@id")?,
                        JsonValue::String(compact_id(context, id, false)?),
                    )]))
                }
            }
            Term::Literal(literal) => {
                compact_literal(context, definition, literal, self.annotations.is_empty())?
            }
            Term::Triple(triple) => to_json_object(BTreeMap::from([(
                "@triple".to_owned(),
                triple.compacted_json(context)?,
            )])),
            Term::List(values) => {
                let values = values
                    .iter()
                    .map(|value| {
                        // List-item compaction inherits the parent term's
                        // type/language/direction coercion. A nested list must not
                        // inherit the outer @list container itself or it would lose
                        // one structural level.
                        let item_definition = if matches!(value.term, Term::List(_)) {
                            None
                        } else {
                            definition
                        };
                        value.compacted_json(context, item_definition)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if definition.is_some_and(|definition| {
                    definition.containers().contains(&JsonLdContainer::List)
                }) {
                    JsonValue::Array(values)
                } else {
                    to_json_object(BTreeMap::from([(
                        compact_keyword(context, "@list")?,
                        JsonValue::Array(values),
                    )]))
                }
            }
        };
        if !self.annotations.is_empty() {
            let annotations = self
                .annotations
                .iter()
                .map(|node| node.compacted_json(context))
                .collect::<Result<Vec<_>, _>>()?;
            let annotation = if let [only] = annotations.as_slice() {
                only.clone()
            } else {
                JsonValue::Array(annotations)
            };
            compacted
                .as_object_mut()
                .ok_or_else(|| decode("annotated JSON-LD value cannot compact to a scalar"))?
                .insert("@annotation".to_owned(), annotation);
        }
        Ok(compacted)
    }
}

impl Triple {
    fn compacted_json(&self, context: &CompiledJsonLdContext) -> Result<JsonValue, RdfDiagnostic> {
        Ok(to_json_object(BTreeMap::from([
            (
                "@object".to_owned(),
                compact_component(context, &self.object)?,
            ),
            (
                "@predicate".to_owned(),
                JsonValue::String(context.compact_iri(&self.predicate, true)?),
            ),
            (
                "@subject".to_owned(),
                compact_component(context, &self.subject)?,
            ),
        ])))
    }
}

fn compact_component(
    context: &CompiledJsonLdContext,
    term: &Term,
) -> Result<JsonValue, RdfDiagnostic> {
    Value::plain(term.clone()).compacted_json(context, None)
}

fn compact_literal(
    context: &CompiledJsonLdContext,
    definition: Option<&JsonLdTermDefinition>,
    literal: &Literal,
    may_collapse: bool,
) -> Result<JsonValue, RdfDiagnostic> {
    if may_collapse
        && definition
            .is_some_and(|definition| literal_matches_mapping(context, definition, literal))
    {
        if definition
            .is_some_and(|definition| definition.type_mapping() == Some(&JsonLdTypeMapping::Json))
            && let Some(parsed) = canonical_json_value(&literal.lexical)
        {
            return Ok(parsed);
        }
        if definition
            .is_none_or(|definition| definition.type_mapping() != Some(&JsonLdTypeMapping::Json))
        {
            return Ok(JsonValue::String(literal.lexical.clone()));
        }
    }
    let mut value = BTreeMap::new();
    insert_unique(
        &mut value,
        &compact_keyword(context, "@value")?,
        JsonValue::String(literal.lexical.clone()),
    )?;
    if let Some(language) = &literal.language {
        insert_unique(
            &mut value,
            &compact_keyword(context, "@language")?,
            JsonValue::String(language.clone()),
        )?;
    }
    if let Some(direction) = &literal.direction {
        insert_unique(
            &mut value,
            &compact_keyword(context, "@direction")?,
            JsonValue::String(direction.clone()),
        )?;
    }
    if let Some(datatype) = &literal.datatype {
        insert_unique(
            &mut value,
            &compact_keyword(context, "@type")?,
            JsonValue::String(context.compact_iri(datatype, true)?),
        )?;
    }
    Ok(to_json_object(value))
}

fn canonical_json_value(lexical: &str) -> Option<JsonValue> {
    let parsed: JsonValue = serde_json::from_str(lexical).ok()?;
    (serde_json::to_string(&parsed).ok()?.as_str() == lexical).then_some(parsed)
}

fn literal_matches_mapping(
    context: &CompiledJsonLdContext,
    definition: &JsonLdTermDefinition,
    literal: &Literal,
) -> bool {
    match (&literal.datatype, definition.type_mapping()) {
        (Some(datatype), Some(JsonLdTypeMapping::Datatype(mapping))) => datatype == mapping,
        (Some(datatype), Some(JsonLdTypeMapping::Json)) => datatype == super::RDF_JSON,
        (Some(_), _) => false,
        (None, Some(_)) => false,
        (None, None) => {
            let language = match definition.language_mapping() {
                Some(JsonLdNullable::Null) => None,
                Some(JsonLdNullable::Value(language)) => Some(language),
                None => context.default_language(),
            };
            let direction = match definition.direction_mapping() {
                Some(JsonLdNullable::Null) => None,
                Some(JsonLdNullable::Value(JsonLdDirection::LeftToRight)) => Some("ltr"),
                Some(JsonLdNullable::Value(JsonLdDirection::RightToLeft)) => Some("rtl"),
                None => context.default_direction().map(JsonLdDirection::as_str),
            };
            literal.language.as_deref() == language && literal.direction.as_deref() == direction
        }
    }
}

fn compact_keyword(
    context: &CompiledJsonLdContext,
    keyword: &str,
) -> Result<String, RdfDiagnostic> {
    context.compact_iri(keyword, true)
}

fn compact_id(
    context: &CompiledJsonLdContext,
    id: &str,
    vocab: bool,
) -> Result<String, RdfDiagnostic> {
    if id.starts_with("_:") {
        Ok(id.to_owned())
    } else {
        context.compact_iri(id, vocab)
    }
}

fn insert_unique(
    object: &mut BTreeMap<String, JsonValue>,
    key: &str,
    value: JsonValue,
) -> Result<(), RdfDiagnostic> {
    if object.insert(key.to_owned(), value).is_some() {
        return Err(decode(format!(
            "JSON-LD compaction produced duplicate member `{key}`"
        )));
    }
    Ok(())
}
