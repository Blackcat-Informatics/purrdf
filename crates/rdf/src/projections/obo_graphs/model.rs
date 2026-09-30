// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::json::{Object, Value};

use super::super::json_codec::{ToJson, json_string_enum};
use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, validate_absolute_iri};
use super::OboGraphsConfig;

/// Append `name` unless `value` is absent.
fn with_some<T: ToJson>(object: &mut Object, name: &str, value: Option<&T>) {
    if let Some(value) = value {
        object.push(name, value.to_json());
    }
}

/// Append `name` unless `values` is empty.
fn with_items<T: ToJson>(object: &mut Object, name: &str, values: &[T]) {
    if !values.is_empty() {
        object.push(name, values.to_json());
    }
}

/// OBO Graphs node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OboNodeType {
    /// OWL/RDFS class.
    Class,
    /// Named individual.
    Individual,
    /// RDF/OWL property.
    Property,
}

/// OBO Graphs property kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OboPropertyType {
    /// Annotation property.
    Annotation,
    /// Object property.
    Object,
    /// Datatype property.
    Data,
}

/// One metadata property value in the 0.3.2 object model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboPropertyValue {
    /// Full predicate IRI.
    pub pred: String,
    /// Lexical or full-IRI value admitted by the OBO Graphs scalar surface.
    pub val: String,
    /// Supporting xrefs.
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    pub meta: Option<Box<OboMeta>>,
}

/// One OBO synonym property value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboSynonym {
    /// Optional caller-supplied synonym-type value.
    pub synonym_type: Option<String>,
    /// Full synonym predicate IRI.
    pub pred: String,
    /// Synonym text.
    pub val: String,
    /// Supporting xrefs.
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    pub meta: Option<Box<OboMeta>>,
}

/// One OBO cross-reference property value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboXref {
    /// Optional display label.
    pub lbl: Option<String>,
    /// Full xref predicate IRI.
    pub pred: String,
    /// Cross-reference value.
    pub val: String,
    /// Supporting xrefs.
    pub xrefs: Vec<String>,
    /// RDF statement annotations retained as nested metadata.
    pub meta: Option<Box<OboMeta>>,
}

/// OBO Graphs 0.3.2 metadata, including nested axiom metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboMeta {
    /// Distinguished textual definition.
    pub definition: Option<OboPropertyValue>,
    /// Comments.
    pub comments: Vec<String>,
    /// Subset identifiers.
    pub subsets: Vec<String>,
    /// Synonyms.
    pub synonyms: Vec<OboSynonym>,
    /// Cross-references.
    pub xrefs: Vec<OboXref>,
    /// Other property values.
    pub basic_property_values: Vec<OboPropertyValue>,
    /// Version string.
    pub version: Option<String>,
    /// Deprecation marker.
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboNode {
    /// Full node IRI.
    pub id: String,
    /// Preferred label.
    pub lbl: Option<String>,
    /// Node kind when declared by the configured vocabulary.
    pub node_type: Option<OboNodeType>,
    /// Property kind for property nodes.
    pub property_type: Option<OboPropertyType>,
    /// Node metadata.
    pub meta: Option<OboMeta>,
}

/// Basic OBO Graphs edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboEdge {
    /// Full subject IRI.
    pub sub: String,
    /// Full predicate IRI.
    pub pred: String,
    /// Full object IRI.
    pub obj: String,
    /// Statement annotations.
    pub meta: Option<OboMeta>,
}

/// Set of mutually equivalent named nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboEquivalentNodesSet {
    /// Deterministic representative (the lexicographically first node id).
    pub representative_node_id: String,
    /// Complete sorted set of equivalent node ids.
    pub node_ids: Vec<String>,
    /// Axiom annotations.
    pub meta: Option<OboMeta>,
}

/// Named existential restriction in one logical definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboExistentialRestriction {
    /// Full object-property IRI.
    pub property_id: String,
    /// Full named filler-class IRI.
    pub filler_id: String,
}

/// Named-class equivalence to an intersection of genera and existentials.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboLogicalDefinitionAxiom {
    /// Defined class IRI.
    pub defined_class_id: String,
    /// Named genus class IRIs.
    pub genus_ids: Vec<String>,
    /// Existential restrictions.
    pub restrictions: Vec<OboExistentialRestriction>,
    /// Axiom annotations.
    pub meta: Option<OboMeta>,
}

/// Aggregated domain/range declaration for one property.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboDomainRangeAxiom {
    /// Property IRI.
    pub predicate_id: String,
    /// Named domain class IRIs.
    pub domain_class_ids: Vec<String>,
    /// Named range class IRIs.
    pub range_class_ids: Vec<String>,
    /// Named all-values-from edges retained by the 0.3.2 model.
    pub all_values_from_edges: Vec<OboEdge>,
    /// Axiom annotations.
    pub meta: Option<OboMeta>,
}

/// One OWL property-chain axiom.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OboPropertyChainAxiom {
    /// Super-property IRI.
    pub predicate_id: String,
    /// Ordered chain of property IRIs.
    pub chain_predicate_ids: Vec<String>,
    /// Axiom annotations.
    pub meta: Option<OboMeta>,
}

/// One OBO Graphs 0.3.2 graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OboGraph {
    /// Caller-owned full graph IRI.
    pub id: String,
    /// Graph label.
    pub lbl: Option<String>,
    /// Graph metadata.
    pub meta: Option<OboMeta>,
    /// Basic nodes.
    pub nodes: Vec<OboNode>,
    /// Basic edges.
    pub edges: Vec<OboEdge>,
    /// Named equivalence sets.
    pub equivalent_nodes_sets: Vec<OboEquivalentNodesSet>,
    /// Logical definitions.
    pub logical_definition_axioms: Vec<OboLogicalDefinitionAxiom>,
    /// Domain/range axioms.
    pub domain_range_axioms: Vec<OboDomainRangeAxiom>,
    /// Property chains.
    pub property_chain_axioms: Vec<OboPropertyChainAxiom>,
}

/// OBO Graphs 0.3.2 graph document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OboGraphDocument {
    /// Document graphs. PurRDF's caller-owned projection emits exactly one.
    pub graphs: Vec<OboGraph>,
}

json_string_enum!(OboNodeType {
    Class => "CLASS",
    Individual => "INDIVIDUAL",
    Property => "PROPERTY",
});

json_string_enum!(OboPropertyType {
    Annotation => "ANNOTATION",
    Object => "OBJECT",
    Data => "DATA",
});

impl ToJson for OboPropertyValue {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("pred", self.pred.as_str())
            .with("val", self.val.as_str());
        with_items(&mut object, "xrefs", &self.xrefs);
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboSynonym {
    fn to_json(&self) -> Value {
        let mut object = Object::new();
        with_some(&mut object, "synonymType", self.synonym_type.as_ref());
        object.push("pred", self.pred.as_str());
        object.push("val", self.val.as_str());
        with_items(&mut object, "xrefs", &self.xrefs);
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboXref {
    fn to_json(&self) -> Value {
        let mut object = Object::new();
        with_some(&mut object, "lbl", self.lbl.as_ref());
        object.push("pred", self.pred.as_str());
        object.push("val", self.val.as_str());
        with_items(&mut object, "xrefs", &self.xrefs);
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboMeta {
    fn to_json(&self) -> Value {
        let mut object = Object::new();
        with_some(&mut object, "definition", self.definition.as_ref());
        with_items(&mut object, "comments", &self.comments);
        with_items(&mut object, "subsets", &self.subsets);
        with_items(&mut object, "synonyms", &self.synonyms);
        with_items(&mut object, "xrefs", &self.xrefs);
        with_items(
            &mut object,
            "basicPropertyValues",
            &self.basic_property_values,
        );
        with_some(&mut object, "version", self.version.as_ref());
        if self.deprecated {
            object.push("deprecated", true);
        }
        Value::Object(object)
    }
}

impl ToJson for OboNode {
    fn to_json(&self) -> Value {
        let mut object = Object::new().with("id", self.id.as_str());
        with_some(&mut object, "lbl", self.lbl.as_ref());
        with_some(&mut object, "type", self.node_type.as_ref());
        with_some(&mut object, "propertyType", self.property_type.as_ref());
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboEdge {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("sub", self.sub.as_str())
            .with("pred", self.pred.as_str())
            .with("obj", self.obj.as_str());
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboEquivalentNodesSet {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("representativeNodeId", self.representative_node_id.as_str())
            .with("nodeIds", self.node_ids.to_json());
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboExistentialRestriction {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("propertyId", self.property_id.as_str())
                .with("fillerId", self.filler_id.as_str()),
        )
    }
}

impl ToJson for OboLogicalDefinitionAxiom {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("definedClassId", self.defined_class_id.as_str())
            .with("genusIds", self.genus_ids.to_json())
            .with("restrictions", self.restrictions.to_json());
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboDomainRangeAxiom {
    fn to_json(&self) -> Value {
        let mut object = Object::new().with("predicateId", self.predicate_id.as_str());
        with_items(&mut object, "domainClassIds", &self.domain_class_ids);
        with_items(&mut object, "rangeClassIds", &self.range_class_ids);
        with_items(
            &mut object,
            "allValuesFromEdges",
            &self.all_values_from_edges,
        );
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboPropertyChainAxiom {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("predicateId", self.predicate_id.as_str())
            .with("chainPredicateIds", self.chain_predicate_ids.to_json());
        with_some(&mut object, "meta", self.meta.as_ref());
        Value::Object(object)
    }
}

impl ToJson for OboGraph {
    fn to_json(&self) -> Value {
        let mut object = Object::new().with("id", self.id.as_str());
        with_some(&mut object, "lbl", self.lbl.as_ref());
        with_some(&mut object, "meta", self.meta.as_ref());
        with_items(&mut object, "nodes", &self.nodes);
        with_items(&mut object, "edges", &self.edges);
        with_items(
            &mut object,
            "equivalentNodesSets",
            &self.equivalent_nodes_sets,
        );
        with_items(
            &mut object,
            "logicalDefinitionAxioms",
            &self.logical_definition_axioms,
        );
        with_items(&mut object, "domainRangeAxioms", &self.domain_range_axioms);
        with_items(
            &mut object,
            "propertyChainAxioms",
            &self.property_chain_axioms,
        );
        Value::Object(object)
    }
}

impl ToJson for OboGraphDocument {
    fn to_json(&self) -> Value {
        Value::Object(Object::new().with("graphs", self.graphs.to_json()))
    }
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
