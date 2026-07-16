// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::Serialize;

use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, validate_absolute_iri};
use super::OboGraphsConfig;

// Serde's `skip_serializing_if` callback receives a shared field reference.
#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde skip_serializing_if requires fn(&bool)"
)]
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_empty<T>(values: &[T]) -> bool {
    values.is_empty()
}

/// OBO Graphs node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OboNodeType {
    /// OWL/RDFS class.
    Class,
    /// Named individual.
    Individual,
    /// RDF/OWL property.
    Property,
}

/// OBO Graphs property kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OboPropertyType {
    /// Annotation property.
    Annotation,
    /// Object property.
    Object,
    /// Datatype property.
    Data,
}

/// One metadata property value in the 0.3.2 object model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboPropertyValue {
    /// Full predicate IRI.
    pub pred: String,
    /// Lexical or full-IRI value admitted by the OBO Graphs scalar surface.
    pub val: String,
    /// Supporting xrefs.
    #[serde(skip_serializing_if = "is_empty")]
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Box<OboMeta>>,
}

/// One OBO synonym property value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboSynonym {
    /// Optional caller-supplied synonym-type value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym_type: Option<String>,
    /// Full synonym predicate IRI.
    pub pred: String,
    /// Synonym text.
    pub val: String,
    /// Supporting xrefs.
    #[serde(skip_serializing_if = "is_empty")]
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Box<OboMeta>>,
}

/// One OBO cross-reference property value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboXref {
    /// Optional display label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbl: Option<String>,
    /// Full xref predicate IRI.
    pub pred: String,
    /// Cross-reference value.
    pub val: String,
    /// Supporting xrefs.
    #[serde(skip_serializing_if = "is_empty")]
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Box<OboMeta>>,
}

/// OBO Graphs 0.3.2 metadata, including nested axiom metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboMeta {
    /// Distinguished textual definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<OboPropertyValue>,
    /// Comments.
    #[serde(skip_serializing_if = "is_empty")]
    pub comments: Vec<String>,
    /// Subset identifiers.
    #[serde(skip_serializing_if = "is_empty")]
    pub subsets: Vec<String>,
    /// Synonyms.
    #[serde(skip_serializing_if = "is_empty")]
    pub synonyms: Vec<OboSynonym>,
    /// Cross-references.
    #[serde(skip_serializing_if = "is_empty")]
    pub xrefs: Vec<OboXref>,
    /// Other property values.
    #[serde(skip_serializing_if = "is_empty")]
    pub basic_property_values: Vec<OboPropertyValue>,
    /// Version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Deprecation marker.
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,
}

impl OboMeta {
    pub(crate) fn normalize(&mut self) {
        normalize_strings(&mut self.comments);
        normalize_strings(&mut self.subsets);
        if let Some(definition) = &mut self.definition {
            definition.normalize();
        }
        for value in &mut self.synonyms {
            value.normalize();
        }
        for value in &mut self.xrefs {
            value.normalize();
        }
        for value in &mut self.basic_property_values {
            value.normalize();
        }
        normalize_values(&mut self.synonyms);
        normalize_values(&mut self.xrefs);
        normalize_values(&mut self.basic_property_values);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.definition.is_none()
            && self.comments.is_empty()
            && self.subsets.is_empty()
            && self.synonyms.is_empty()
            && self.xrefs.is_empty()
            && self.basic_property_values.is_empty()
            && self.version.is_none()
            && !self.deprecated
    }

    fn validate(&self, depth: usize, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
        if depth > config.limits().max_term_depth() {
            return Err(ProjectionError::limit(format!(
                "OBO Graphs metadata nesting exceeds the configured depth limit of {}",
                config.limits().max_term_depth()
            )));
        }
        if let Some(definition) = &self.definition {
            definition.validate(depth + 1, config)?;
        }
        for value in &self.synonyms {
            value.validate(depth + 1, config)?;
        }
        for value in &self.xrefs {
            value.validate(depth + 1, config)?;
        }
        for value in &self.basic_property_values {
            value.validate(depth + 1, config)?;
        }
        Ok(())
    }
}

impl OboPropertyValue {
    fn normalize(&mut self) {
        normalize_strings(&mut self.xrefs);
        if let Some(meta) = &mut self.meta {
            meta.normalize();
        }
    }

    fn validate(&self, depth: usize, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
        validate_absolute_iri(&self.pred, "OBO Graphs property-value predicate")?;
        if let Some(meta) = &self.meta {
            meta.validate(depth, config)?;
        }
        Ok(())
    }
}

impl OboSynonym {
    fn normalize(&mut self) {
        normalize_strings(&mut self.xrefs);
        if let Some(meta) = &mut self.meta {
            meta.normalize();
        }
    }

    fn validate(&self, depth: usize, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
        validate_absolute_iri(&self.pred, "OBO Graphs synonym predicate")?;
        if let Some(meta) = &self.meta {
            meta.validate(depth, config)?;
        }
        Ok(())
    }
}

impl OboXref {
    fn normalize(&mut self) {
        normalize_strings(&mut self.xrefs);
        if let Some(meta) = &mut self.meta {
            meta.normalize();
        }
    }

    fn validate(&self, depth: usize, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
        validate_absolute_iri(&self.pred, "OBO Graphs xref predicate")?;
        if let Some(meta) = &self.meta {
            meta.validate(depth, config)?;
        }
        Ok(())
    }
}

/// Basic OBO Graphs node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboNode {
    /// Full node IRI.
    pub id: String,
    /// Preferred label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbl: Option<String>,
    /// Node kind when declared by the configured vocabulary.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub node_type: Option<OboNodeType>,
    /// Property kind for property nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<OboPropertyType>,
    /// Node metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// Basic OBO Graphs edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboEdge {
    /// Full subject IRI.
    pub sub: String,
    /// Full predicate IRI.
    pub pred: String,
    /// Full object IRI.
    pub obj: String,
    /// Statement annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// Set of mutually equivalent named nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboEquivalentNodesSet {
    /// Deterministic representative (the lexicographically first node id).
    pub representative_node_id: String,
    /// Complete sorted set of equivalent node ids.
    pub node_ids: Vec<String>,
    /// Axiom annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// Named existential restriction in one logical definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboExistentialRestriction {
    /// Full object-property IRI.
    pub property_id: String,
    /// Full named filler-class IRI.
    pub filler_id: String,
}

/// Named-class equivalence to an intersection of genera and existentials.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboLogicalDefinitionAxiom {
    /// Defined class IRI.
    pub defined_class_id: String,
    /// Named genus class IRIs.
    pub genus_ids: Vec<String>,
    /// Existential restrictions.
    pub restrictions: Vec<OboExistentialRestriction>,
    /// Axiom annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// Aggregated domain/range declaration for one property.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboDomainRangeAxiom {
    /// Property IRI.
    pub predicate_id: String,
    /// Named domain class IRIs.
    #[serde(skip_serializing_if = "is_empty")]
    pub domain_class_ids: Vec<String>,
    /// Named range class IRIs.
    #[serde(skip_serializing_if = "is_empty")]
    pub range_class_ids: Vec<String>,
    /// Named all-values-from edges retained by the 0.3.2 model.
    #[serde(skip_serializing_if = "is_empty")]
    pub all_values_from_edges: Vec<OboEdge>,
    /// Axiom annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// One OWL property-chain axiom.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboPropertyChainAxiom {
    /// Super-property IRI.
    pub predicate_id: String,
    /// Ordered chain of property IRIs.
    pub chain_predicate_ids: Vec<String>,
    /// Axiom annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
}

/// One OBO Graphs 0.3.2 graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboGraph {
    /// Caller-owned full graph IRI.
    pub id: String,
    /// Graph label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbl: Option<String>,
    /// Graph metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<OboMeta>,
    /// Basic nodes.
    #[serde(skip_serializing_if = "is_empty")]
    pub nodes: Vec<OboNode>,
    /// Basic edges.
    #[serde(skip_serializing_if = "is_empty")]
    pub edges: Vec<OboEdge>,
    /// Named equivalence sets.
    #[serde(skip_serializing_if = "is_empty")]
    pub equivalent_nodes_sets: Vec<OboEquivalentNodesSet>,
    /// Logical definitions.
    #[serde(skip_serializing_if = "is_empty")]
    pub logical_definition_axioms: Vec<OboLogicalDefinitionAxiom>,
    /// Domain/range axioms.
    #[serde(skip_serializing_if = "is_empty")]
    pub domain_range_axioms: Vec<OboDomainRangeAxiom>,
    /// Property chains.
    #[serde(skip_serializing_if = "is_empty")]
    pub property_chain_axioms: Vec<OboPropertyChainAxiom>,
}

/// OBO Graphs 0.3.2 graph document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OboGraphDocument {
    /// Document graphs. PurRDF's caller-owned projection emits exactly one.
    pub graphs: Vec<OboGraph>,
}

impl OboGraphDocument {
    /// Validate, normalize, and serialize this document to deterministic JSON.
    pub fn to_canonical_json(&self, config: &OboGraphsConfig) -> Result<Vec<u8>, ProjectionError> {
        let mut canonical = self.clone();
        canonical.normalize();
        canonical.validate(config)?;
        canonical_json_bounded(&canonical, config.limits(), "OBO Graphs 0.3.2 JSON")
    }

    /// Validate the stricter PurRDF full-IRI profile of OBO Graphs 0.3.2.
    pub fn validate(&self, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
        if self.graphs.len() != 1 {
            return Err(ProjectionError::integrity(
                "a PurRDF OBO Graphs projection must contain exactly one caller-owned graph",
            ));
        }
        let graph = &self.graphs[0];
        if graph.id != config.graph_id() {
            return Err(ProjectionError::integrity(
                "OBO Graphs graph id differs from the caller-owned configured identity",
            ));
        }
        validate_graph(graph, config)
    }

    pub(crate) fn normalize(&mut self) {
        for graph in &mut self.graphs {
            normalize_graph(graph);
        }
        self.graphs.sort_by(|left, right| left.id.cmp(&right.id));
    }
}

fn normalize_graph(graph: &mut OboGraph) {
    if let Some(meta) = &mut graph.meta {
        meta.normalize();
    }
    for node in &mut graph.nodes {
        if let Some(meta) = &mut node.meta {
            meta.normalize();
        }
    }
    for edge in &mut graph.edges {
        if let Some(meta) = &mut edge.meta {
            meta.normalize();
        }
    }
    for set in &mut graph.equivalent_nodes_sets {
        normalize_strings(&mut set.node_ids);
        set.representative_node_id = set.node_ids.first().cloned().unwrap_or_default();
        if let Some(meta) = &mut set.meta {
            meta.normalize();
        }
    }
    for axiom in &mut graph.logical_definition_axioms {
        normalize_strings(&mut axiom.genus_ids);
        normalize_values(&mut axiom.restrictions);
        if let Some(meta) = &mut axiom.meta {
            meta.normalize();
        }
    }
    for axiom in &mut graph.domain_range_axioms {
        normalize_strings(&mut axiom.domain_class_ids);
        normalize_strings(&mut axiom.range_class_ids);
        normalize_values(&mut axiom.all_values_from_edges);
        if let Some(meta) = &mut axiom.meta {
            meta.normalize();
        }
    }
    for axiom in &mut graph.property_chain_axioms {
        if let Some(meta) = &mut axiom.meta {
            meta.normalize();
        }
    }
    normalize_values(&mut graph.nodes);
    normalize_values(&mut graph.edges);
    normalize_values(&mut graph.equivalent_nodes_sets);
    normalize_values(&mut graph.logical_definition_axioms);
    normalize_values(&mut graph.domain_range_axioms);
    normalize_values(&mut graph.property_chain_axioms);
}

fn validate_graph(graph: &OboGraph, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
    validate_absolute_iri(&graph.id, "OBO Graphs graph id")?;
    if let Some(meta) = &graph.meta {
        meta.validate(0, config)?;
    }
    ensure_unique_by(&graph.nodes, |node| node.id.as_str(), "OBO Graphs node ids")?;
    for node in &graph.nodes {
        validate_absolute_iri(&node.id, "OBO Graphs node id")?;
        if node.property_type.is_some() && node.node_type != Some(OboNodeType::Property) {
            return Err(ProjectionError::integrity(
                "OBO Graphs propertyType requires node type PROPERTY",
            ));
        }
        if let Some(meta) = &node.meta {
            meta.validate(0, config)?;
        }
    }
    for edge in &graph.edges {
        validate_edge(edge, config)?;
    }
    for set in &graph.equivalent_nodes_sets {
        if set.node_ids.len() < 2 {
            return Err(ProjectionError::integrity(
                "an OBO Graphs equivalentNodesSet requires at least two node ids",
            ));
        }
        if set.node_ids.first() != Some(&set.representative_node_id) {
            return Err(ProjectionError::integrity(
                "equivalentNodesSet representative must be its lexicographically first node id",
            ));
        }
        ensure_unique_sorted(&set.node_ids, "equivalentNodesSet node ids")?;
        for id in &set.node_ids {
            validate_absolute_iri(id, "equivalent node id")?;
        }
        if let Some(meta) = &set.meta {
            meta.validate(0, config)?;
        }
    }
    for axiom in &graph.logical_definition_axioms {
        validate_absolute_iri(&axiom.defined_class_id, "logical definition class id")?;
        if axiom.genus_ids.is_empty() && axiom.restrictions.is_empty() {
            return Err(ProjectionError::integrity(
                "a logical definition requires at least one genus or existential restriction",
            ));
        }
        ensure_unique_sorted(&axiom.genus_ids, "logical definition genus ids")?;
        for id in &axiom.genus_ids {
            validate_absolute_iri(id, "logical definition genus id")?;
        }
        for restriction in &axiom.restrictions {
            validate_absolute_iri(&restriction.property_id, "existential property id")?;
            validate_absolute_iri(&restriction.filler_id, "existential filler id")?;
        }
        if let Some(meta) = &axiom.meta {
            meta.validate(0, config)?;
        }
    }
    for axiom in &graph.domain_range_axioms {
        validate_absolute_iri(&axiom.predicate_id, "domain/range predicate id")?;
        if axiom.domain_class_ids.is_empty()
            && axiom.range_class_ids.is_empty()
            && axiom.all_values_from_edges.is_empty()
        {
            return Err(ProjectionError::integrity(
                "a domainRangeAxiom must carry a domain, range, or all-values-from edge",
            ));
        }
        ensure_unique_sorted(&axiom.domain_class_ids, "domain class ids")?;
        ensure_unique_sorted(&axiom.range_class_ids, "range class ids")?;
        for id in axiom.domain_class_ids.iter().chain(&axiom.range_class_ids) {
            validate_absolute_iri(id, "domain/range class id")?;
        }
        for edge in &axiom.all_values_from_edges {
            validate_edge(edge, config)?;
        }
        if let Some(meta) = &axiom.meta {
            meta.validate(0, config)?;
        }
    }
    for axiom in &graph.property_chain_axioms {
        validate_absolute_iri(&axiom.predicate_id, "property-chain predicate id")?;
        if axiom.chain_predicate_ids.len() < 2 {
            return Err(ProjectionError::integrity(
                "an OBO Graphs property chain requires at least two predicates",
            ));
        }
        for id in &axiom.chain_predicate_ids {
            validate_absolute_iri(id, "property-chain member id")?;
        }
        if let Some(meta) = &axiom.meta {
            meta.validate(0, config)?;
        }
    }
    Ok(())
}

fn validate_edge(edge: &OboEdge, config: &OboGraphsConfig) -> Result<(), ProjectionError> {
    validate_absolute_iri(&edge.sub, "OBO Graphs edge subject")?;
    validate_absolute_iri(&edge.pred, "OBO Graphs edge predicate")?;
    validate_absolute_iri(&edge.obj, "OBO Graphs edge object")?;
    if let Some(meta) = &edge.meta {
        meta.validate(0, config)?;
    }
    Ok(())
}

fn normalize_strings(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn normalize_values<T: Ord>(values: &mut Vec<T>) {
    values.sort();
    values.dedup();
}

fn ensure_unique_sorted(values: &[String], description: &str) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be strictly sorted and unique"
        )));
    }
    Ok(())
}

fn ensure_unique_by<T>(
    values: &[T],
    key: impl Fn(&T) -> &str,
    description: &str,
) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be strictly sorted and unique"
        )));
    }
    Ok(())
}
