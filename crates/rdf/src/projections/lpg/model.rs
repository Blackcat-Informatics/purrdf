// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use super::super::util::canonical_json_bounded;
use super::super::{
    ProjectionError, ProjectionLimits, ProjectionTerm, stable_identifier, validate_absolute_iri,
};

const LPG_SCHEMA_VERSION: u32 = 1;
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// Mandatory policy and resource boundary for the canonical LPG mapping.
///
/// There is deliberately no `Default`: the caller identifies the predicate whose
/// IRI-object statements become native labels and chooses an explicit total-record
/// ceiling. Every other predicate remains a full source IRI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LpgConfig {
    rdf_type: String,
    limits: ProjectionLimits,
    max_records: usize,
}

impl LpgConfig {
    /// Construct a validated LPG configuration.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when `rdf_type` is not an absolute IRI or
    /// `max_records` is zero or wider than the portable `u32` record space.
    pub fn new(
        rdf_type: impl Into<String>,
        limits: ProjectionLimits,
        max_records: usize,
    ) -> Result<Self, ProjectionError> {
        let rdf_type = rdf_type.into();
        validate_absolute_iri(&rdf_type, "LPG rdf_type predicate")?;
        if max_records == 0 {
            return Err(ProjectionError::configuration(
                "LPG max_records must be greater than zero",
            ));
        }
        if u32::try_from(max_records).is_err() {
            return Err(ProjectionError::configuration(
                "LPG max_records exceeds the portable u32 record ceiling",
            ));
        }
        Ok(Self {
            rdf_type,
            limits,
            max_records,
        })
    }

    /// Caller-supplied predicate whose IRI objects become LPG labels.
    pub fn rdf_type(&self) -> &str {
        &self.rdf_type
    }

    /// Shared artifact and recursive-term limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Maximum total canonical records, including nested labels and properties.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLpgConfig {
    rdf_type: String,
    limits: ProjectionLimits,
    max_records: usize,
}

impl<'de> Deserialize<'de> for LpgConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawLpgConfig::deserialize(deserializer)?;
        Self::new(raw.rdf_type, raw.limits, raw.max_records).map_err(serde::de::Error::custom)
    }
}

/// Graph placement carried beside one RDF-origin LPG record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum LpgGraphContext {
    /// RDF default graph.
    Default,
    /// RDF named graph, retaining exact IRI or scoped blank-node identity.
    Named {
        /// Named-graph term.
        name: ProjectionTerm,
    },
}

impl LpgGraphContext {
    /// Construct a named-graph context.
    pub fn named(name: ProjectionTerm) -> Self {
        Self::Named { name }
    }

    /// Named graph term, or `None` for the default graph.
    pub const fn name(&self) -> Option<&ProjectionTerm> {
        match self {
            Self::Default => None,
            Self::Named { name } => Some(name),
        }
    }
}

/// Exact RDF 1.2 statement identity retained beside a native LPG construct.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgRdfQuad {
    /// Exact subject term.
    pub subject: ProjectionTerm,
    /// Full predicate IRI.
    pub predicate: String,
    /// Exact object term.
    pub object: ProjectionTerm,
    /// Exact default/named graph placement.
    pub graph: LpgGraphContext,
}

/// Native scalar projection of one RDF literal.
///
/// The owning [`LpgProperty`] always carries the exact RDF literal in
/// [`LpgProperty::rdf`]. This atom is the useful property-graph value; it never
/// replaces or weakens lexical/datatype/language/direction identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum LpgPropertyAtom {
    /// Boolean value from a valid `xsd:boolean` lexical form.
    Boolean {
        /// Native Boolean.
        value: bool,
    },
    /// Signed integer that fits the portable LPG `i64` surface.
    Integer {
        /// Native integer.
        value: i64,
    },
    /// Numeric lexical form that must not be narrowed to `i64` or binary float.
    Decimal {
        /// Original numeric lexical form.
        lexical: String,
    },
    /// IEEE-754 value represented by deterministic `f64::to_bits` identity.
    Float {
        /// Native binary-float bits, preserving negative zero and NaN payload.
        bits: u64,
    },
    /// Textual fallback for every other literal surface.
    String {
        /// Authored lexical form.
        value: String,
    },
}

impl LpgPropertyAtom {
    /// Recover the native floating-point value when this is a float atom.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float { bits } => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }
}

/// One RDF type-like statement lowered to a native LPG label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgLabel {
    /// Stable identifier of the exact source statement.
    pub statement_id: String,
    /// Full class/label IRI.
    pub value: String,
    /// Exact RDF source statement.
    pub rdf: LpgRdfQuad,
}

/// One RDF literal statement lowered to a native LPG property.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgProperty {
    /// Stable identifier of the exact source statement.
    pub statement_id: String,
    /// Full RDF predicate IRI used as the collision-free property key.
    pub key: String,
    /// Native scalar view.
    pub value: LpgPropertyAtom,
    /// Exact RDF source statement.
    pub rdf: LpgRdfQuad,
}

/// Canonical LPG node with exact RDF term identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgNode {
    /// Stable collision-resistant node identifier.
    pub id: String,
    /// Exact RDF resource term represented by this node.
    pub identity: ProjectionTerm,
    /// Deterministically ordered type-like labels asserted about this node.
    pub labels: Vec<LpgLabel>,
    /// Deterministically ordered literal properties asserted about this node.
    pub properties: Vec<LpgProperty>,
}

/// Canonical directed LPG edge with exact RDF statement identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgEdge {
    /// Stable collision-resistant edge identifier.
    pub id: String,
    /// Source node identifier.
    pub source: String,
    /// Target node identifier.
    pub target: String,
    /// Full RDF predicate IRI used as the collision-free edge type.
    pub edge_type: String,
    /// Exact RDF source statement.
    pub rdf: LpgRdfQuad,
}

/// Exact RDF 1.2 reifier binding carried by the canonical LPG model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgReifier {
    /// Stable record identifier.
    pub id: String,
    /// Exact IRI or scoped blank-node reifier resource.
    pub reifier: ProjectionTerm,
    /// Exact quoted triple term bound by the reifier.
    pub statement: ProjectionTerm,
    /// Exact declaration graph.
    pub graph: LpgGraphContext,
}

/// Exact RDF 1.2 statement annotation carried by the canonical LPG model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgAnnotation {
    /// Stable record identifier.
    pub id: String,
    /// Exact IRI or scoped blank-node reifier resource.
    pub reifier: ProjectionTerm,
    /// Full annotation predicate IRI.
    pub predicate: String,
    /// Exact annotation object.
    pub object: ProjectionTerm,
    /// Exact annotation graph.
    pub graph: LpgGraphContext,
}

/// Canonical deterministic LPG plus complete RDF 1.2 reversal sideband.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgGraph {
    /// Canonical model schema version. Version 1 is the only accepted value.
    pub schema_version: u32,
    /// Deterministically ordered nodes.
    pub nodes: Vec<LpgNode>,
    /// Deterministically ordered edges.
    pub edges: Vec<LpgEdge>,
    /// Exact RDF 1.2 reifier bindings.
    pub reifiers: Vec<LpgReifier>,
    /// Exact RDF 1.2 annotations.
    pub annotations: Vec<LpgAnnotation>,
    /// Exact named-graph declarations, including empty graphs.
    pub named_graphs: Vec<ProjectionTerm>,
}

impl LpgGraph {
    pub(super) fn new(
        nodes: Vec<LpgNode>,
        edges: Vec<LpgEdge>,
        reifiers: Vec<LpgReifier>,
        annotations: Vec<LpgAnnotation>,
        named_graphs: Vec<ProjectionTerm>,
    ) -> Self {
        Self {
            schema_version: LPG_SCHEMA_VERSION,
            nodes,
            edges,
            reifiers,
            annotations,
            named_graphs,
        }
    }

    /// Validate ordering, identifiers, exact sideband consistency, references, RDF
    /// positions, graph declarations, and caller resource limits.
    ///
    /// # Errors
    ///
    /// Returns a typed hard failure for every malformed, ambiguous, dangling, or
    /// resource-exceeding model.
    pub fn validate(&self, config: &LpgConfig) -> Result<(), ProjectionError> {
        if self.schema_version != LPG_SCHEMA_VERSION {
            return Err(ProjectionError::integrity(format!(
                "unsupported LPG schema version {}; expected {LPG_SCHEMA_VERSION}",
                self.schema_version
            )));
        }
        self.validate_record_budget(config)?;
        require_strict_order(&self.named_graphs, "named graphs")?;
        require_id_order(&self.nodes, |node| &node.id, "nodes")?;
        require_id_order(&self.edges, |edge| &edge.id, "edges")?;
        require_id_order(&self.reifiers, |row| &row.id, "reifiers")?;
        require_id_order(&self.annotations, |row| &row.id, "annotations")?;

        let limits = config.limits();
        let mut named_graphs = BTreeSet::new();
        let mut expected_nodes = BTreeSet::new();
        for graph in &self.named_graphs {
            validate_graph_name(graph, limits, "named graph declaration")?;
            if !named_graphs.insert(graph.clone()) {
                return Err(ProjectionError::integrity(
                    "duplicate named graph declaration",
                ));
            }
            collect_node_terms(graph, &mut expected_nodes);
        }

        let mut nodes_by_id = BTreeMap::new();
        let mut node_identities = BTreeSet::new();
        for node in &self.nodes {
            validate_resource(&node.identity, limits, "LPG node identity")?;
            let expected_id = node_identifier(&node.identity, limits)?;
            if node.id != expected_id {
                return Err(ProjectionError::integrity(format!(
                    "node id `{}` does not match its RDF identity (`{expected_id}` expected)",
                    node.id
                )));
            }
            if nodes_by_id.insert(node.id.as_str(), node).is_some()
                || !node_identities.insert(node.identity.clone())
            {
                return Err(ProjectionError::integrity(
                    "duplicate or colliding LPG node identity",
                ));
            }
            require_strict_order(&node.labels, "node labels")?;
            require_strict_order(&node.properties, "node properties")?;
        }

        let mut statements = BTreeSet::new();
        let mut statement_ids = BTreeSet::new();
        for node in &self.nodes {
            for label in &node.labels {
                validate_rdf_quad(&label.rdf, limits, &named_graphs)?;
                let expected_id = statement_identifier(&label.rdf, limits)?;
                if label.statement_id != expected_id {
                    return Err(ProjectionError::integrity(format!(
                        "label statement id `{}` is inconsistent (`{expected_id}` expected)",
                        label.statement_id
                    )));
                }
                if label.rdf.subject != node.identity
                    || label.rdf.predicate != config.rdf_type()
                    || label.rdf.object
                        != (ProjectionTerm::Iri {
                            value: label.value.clone(),
                        })
                {
                    return Err(ProjectionError::integrity(
                        "label fields disagree with their exact RDF statement sideband",
                    ));
                }
                validate_iri_data(&label.value, "LPG label IRI")?;
                insert_statement(
                    &label.statement_id,
                    &label.rdf,
                    &mut statement_ids,
                    &mut statements,
                )?;
                collect_quad_nodes(&label.rdf, &mut expected_nodes);
            }
            for property in &node.properties {
                validate_rdf_quad(&property.rdf, limits, &named_graphs)?;
                let expected_id = statement_identifier(&property.rdf, limits)?;
                if property.statement_id != expected_id {
                    return Err(ProjectionError::integrity(format!(
                        "property statement id `{}` is inconsistent (`{expected_id}` expected)",
                        property.statement_id
                    )));
                }
                if property.rdf.subject != node.identity || property.rdf.predicate != property.key {
                    return Err(ProjectionError::integrity(
                        "property key or subject disagrees with its RDF statement sideband",
                    ));
                }
                let expected_atom = property_atom(&property.rdf.object)?;
                if property.value != expected_atom {
                    return Err(ProjectionError::integrity(
                        "property atom disagrees with its exact RDF literal sideband",
                    ));
                }
                validate_iri_data(&property.key, "LPG property key")?;
                insert_statement(
                    &property.statement_id,
                    &property.rdf,
                    &mut statement_ids,
                    &mut statements,
                )?;
                collect_quad_nodes(&property.rdf, &mut expected_nodes);
            }
        }

        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            validate_rdf_quad(&edge.rdf, limits, &named_graphs)?;
            let expected_id = edge_identifier(&edge.rdf, limits)?;
            if edge.id != expected_id || !edge_ids.insert(edge.id.as_str()) {
                return Err(ProjectionError::integrity(format!(
                    "edge id `{}` is duplicate or inconsistent (`{expected_id}` expected)",
                    edge.id
                )));
            }
            let source = nodes_by_id
                .get(edge.source.as_str())
                .ok_or_else(|| ProjectionError::integrity("edge has a dangling source node"))?;
            let target = nodes_by_id
                .get(edge.target.as_str())
                .ok_or_else(|| ProjectionError::integrity("edge has a dangling target node"))?;
            if edge.rdf.subject != source.identity
                || edge.rdf.object != target.identity
                || edge.rdf.predicate != edge.edge_type
            {
                return Err(ProjectionError::integrity(
                    "edge endpoints or type disagree with its RDF statement sideband",
                ));
            }
            if matches!(edge.rdf.object, ProjectionTerm::Literal { .. })
                || (edge.rdf.predicate == config.rdf_type()
                    && matches!(edge.rdf.object, ProjectionTerm::Iri { .. }))
            {
                return Err(ProjectionError::integrity(
                    "edge carries a statement that canonically belongs to a property or label",
                ));
            }
            validate_iri_data(&edge.edge_type, "LPG edge type")?;
            insert_statement(&edge.id, &edge.rdf, &mut statement_ids, &mut statements)?;
            collect_quad_nodes(&edge.rdf, &mut expected_nodes);
        }

        let mut reifier_ids = BTreeSet::new();
        for row in &self.reifiers {
            validate_asserted_resource(&row.reifier, limits, "LPG reifier resource")?;
            row.statement.validate(limits)?;
            if !matches!(row.statement, ProjectionTerm::Triple { .. }) {
                return Err(ProjectionError::integrity(
                    "LPG reifier target must be a triple term",
                ));
            }
            validate_context(&row.graph, limits, &named_graphs)?;
            let expected_id = reifier_identifier(row, limits)?;
            if row.id != expected_id || !reifier_ids.insert(row.id.as_str()) {
                return Err(ProjectionError::integrity(format!(
                    "reifier id `{}` is duplicate or inconsistent (`{expected_id}` expected)",
                    row.id
                )));
            }
            collect_node_terms(&row.reifier, &mut expected_nodes);
            collect_node_terms(&row.statement, &mut expected_nodes);
        }

        let mut annotation_ids = BTreeSet::new();
        for row in &self.annotations {
            validate_asserted_resource(&row.reifier, limits, "LPG annotation reifier")?;
            row.object.validate(limits)?;
            validate_iri_data(&row.predicate, "LPG annotation predicate")?;
            validate_context(&row.graph, limits, &named_graphs)?;
            let expected_id = annotation_identifier(row, limits)?;
            if row.id != expected_id || !annotation_ids.insert(row.id.as_str()) {
                return Err(ProjectionError::integrity(format!(
                    "annotation id `{}` is duplicate or inconsistent (`{expected_id}` expected)",
                    row.id
                )));
            }
            collect_node_terms(&row.reifier, &mut expected_nodes);
            collect_node_terms(&row.object, &mut expected_nodes);
        }

        if node_identities != expected_nodes {
            let missing: Vec<_> = expected_nodes.difference(&node_identities).collect();
            let orphaned: Vec<_> = node_identities.difference(&expected_nodes).collect();
            return Err(ProjectionError::integrity(format!(
                "LPG node set disagrees with RDF sideband; missing {missing:?}, orphaned {orphaned:?}"
            )));
        }
        Ok(())
    }

    /// Serialize this validated model as canonical compact JSON.
    ///
    /// # Errors
    ///
    /// Returns a model-validation, serialization, or artifact-size error.
    pub fn to_canonical_json(&self, config: &LpgConfig) -> Result<Vec<u8>, ProjectionError> {
        self.validate(config)?;
        canonical_json_bounded(self, config.limits(), "canonical LPG JSON")
    }

    /// Parse, validate, and require byte-canonical compact JSON.
    ///
    /// # Errors
    ///
    /// Rejects oversized, malformed, non-canonical, or semantically inconsistent
    /// documents.
    pub fn from_canonical_json(bytes: &[u8], config: &LpgConfig) -> Result<Self, ProjectionError> {
        if bytes.len() > config.limits().max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "canonical LPG JSON is {} bytes; artifact limit is {}",
                bytes.len(),
                config.limits().max_artifact_bytes()
            )));
        }
        let graph: Self = serde_json::from_slice(bytes)
            .map_err(|error| ProjectionError::syntax(format!("parse LPG JSON: {error}")))?;
        graph.validate(config)?;
        if graph.to_canonical_json(config)? != bytes {
            return Err(ProjectionError::syntax(
                "LPG JSON is valid but not in canonical PurRDF form",
            ));
        }
        Ok(graph)
    }

    fn validate_record_budget(&self, config: &LpgConfig) -> Result<(), ProjectionError> {
        let mut count = 0usize;
        for amount in [
            self.nodes.len(),
            self.edges.len(),
            self.reifiers.len(),
            self.annotations.len(),
            self.named_graphs.len(),
        ] {
            count = count
                .checked_add(amount)
                .ok_or_else(|| ProjectionError::limit("LPG record count overflow"))?;
        }
        for node in &self.nodes {
            count = count
                .checked_add(node.labels.len())
                .and_then(|value| value.checked_add(node.properties.len()))
                .ok_or_else(|| ProjectionError::limit("LPG record count overflow"))?;
        }
        if count > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "LPG model contains {count} records; limit is {}",
                config.max_records()
            )));
        }
        Ok(())
    }
}

pub(super) fn node_identifier(
    term: &ProjectionTerm,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    let bytes = term.to_canonical_json(limits)?;
    stable_identifier("node", &bytes)
}

pub(super) fn statement_identifier(
    quad: &LpgRdfQuad,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    record_identifier("statement", quad, limits, "LPG RDF statement")
}

pub(super) fn edge_identifier(
    quad: &LpgRdfQuad,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    record_identifier("edge", quad, limits, "LPG edge statement")
}

pub(super) fn reifier_identifier(
    row: &LpgReifier,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    #[derive(Serialize)]
    struct ReifierKey<'a> {
        reifier: &'a ProjectionTerm,
        statement: &'a ProjectionTerm,
        graph: &'a LpgGraphContext,
    }
    record_identifier(
        "reifier",
        &ReifierKey {
            reifier: &row.reifier,
            statement: &row.statement,
            graph: &row.graph,
        },
        limits,
        "LPG reifier key",
    )
}

pub(super) fn annotation_identifier(
    row: &LpgAnnotation,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    #[derive(Serialize)]
    struct AnnotationKey<'a> {
        reifier: &'a ProjectionTerm,
        predicate: &'a str,
        object: &'a ProjectionTerm,
        graph: &'a LpgGraphContext,
    }
    record_identifier(
        "annotation",
        &AnnotationKey {
            reifier: &row.reifier,
            predicate: &row.predicate,
            object: &row.object,
            graph: &row.graph,
        },
        limits,
        "LPG annotation key",
    )
}

fn record_identifier<T: Serialize>(
    prefix: &str,
    value: &T,
    limits: ProjectionLimits,
    description: &str,
) -> Result<String, ProjectionError> {
    let bytes = canonical_json_bounded(value, limits, description)?;
    stable_identifier(prefix, &bytes)
}

pub(super) fn property_atom(term: &ProjectionTerm) -> Result<LpgPropertyAtom, ProjectionError> {
    let ProjectionTerm::Literal {
        lexical, datatype, ..
    } = term
    else {
        return Err(ProjectionError::integrity(
            "an LPG property must carry an RDF literal object",
        ));
    };
    Ok(match datatype.as_str() {
        concat!("http://www.w3.org/2001/XMLSchema#", "boolean") => match lexical.as_str() {
            "true" | "1" => LpgPropertyAtom::Boolean { value: true },
            "false" | "0" => LpgPropertyAtom::Boolean { value: false },
            _ => LpgPropertyAtom::String {
                value: lexical.clone(),
            },
        },
        datatype if is_integer_datatype(datatype) => {
            if is_integer_lexical(lexical) {
                lexical.parse::<i64>().map_or_else(
                    |_| LpgPropertyAtom::Decimal {
                        lexical: lexical.clone(),
                    },
                    |value| LpgPropertyAtom::Integer { value },
                )
            } else {
                LpgPropertyAtom::String {
                    value: lexical.clone(),
                }
            }
        }
        concat!("http://www.w3.org/2001/XMLSchema#", "decimal") => {
            if is_decimal_lexical(lexical) {
                LpgPropertyAtom::Decimal {
                    lexical: lexical.clone(),
                }
            } else {
                LpgPropertyAtom::String {
                    value: lexical.clone(),
                }
            }
        }
        concat!("http://www.w3.org/2001/XMLSchema#", "float")
        | concat!("http://www.w3.org/2001/XMLSchema#", "double") => parse_float(lexical)
            .map_or_else(
                || LpgPropertyAtom::String {
                    value: lexical.clone(),
                },
                |value| LpgPropertyAtom::Float {
                    bits: value.to_bits(),
                },
            ),
        _ => LpgPropertyAtom::String {
            value: lexical.clone(),
        },
    })
}

fn is_integer_datatype(datatype: &str) -> bool {
    matches!(
        datatype.strip_prefix(XSD),
        Some(
            "integer"
                | "long"
                | "int"
                | "short"
                | "byte"
                | "nonNegativeInteger"
                | "positiveInteger"
                | "unsignedLong"
                | "unsignedInt"
                | "unsignedShort"
                | "unsignedByte"
                | "nonPositiveInteger"
                | "negativeInteger"
        )
    )
}

fn is_integer_lexical(lexical: &str) -> bool {
    let digits = lexical
        .strip_prefix(['+', '-'])
        .unwrap_or(lexical)
        .as_bytes();
    !digits.is_empty() && digits.iter().all(u8::is_ascii_digit)
}

fn is_decimal_lexical(lexical: &str) -> bool {
    let unsigned = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    let mut pieces = unsigned.split('.');
    let before = pieces.next().unwrap_or_default();
    let after = pieces.next();
    if pieces.next().is_some() {
        return false;
    }
    let before_valid = before.bytes().all(|byte| byte.is_ascii_digit());
    match after {
        None => !before.is_empty() && before_valid,
        Some(after) => {
            before_valid
                && after.bytes().all(|byte| byte.is_ascii_digit())
                && (!before.is_empty() || !after.is_empty())
        }
    }
}

fn parse_float(lexical: &str) -> Option<f64> {
    match lexical {
        "INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        _ if lexical.bytes().all(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E')
        }) && lexical.bytes().any(|byte| byte.is_ascii_digit()) =>
        {
            lexical.parse().ok()
        }
        _ => None,
    }
}

fn require_strict_order<T: Ord>(rows: &[T], description: &str) -> Result<(), ProjectionError> {
    if rows.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be strictly ordered with no duplicates"
        )));
    }
    Ok(())
}

fn require_id_order<T>(
    rows: &[T],
    id: impl Fn(&T) -> &str,
    description: &str,
) -> Result<(), ProjectionError> {
    if rows.windows(2).any(|pair| id(&pair[0]) >= id(&pair[1])) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be strictly ordered by id with no duplicates"
        )));
    }
    Ok(())
}

fn validate_iri_data(value: &str, field: &str) -> Result<(), ProjectionError> {
    validate_absolute_iri(value, field).map_err(|error| ProjectionError::integrity(error.message()))
}

fn validate_resource(
    term: &ProjectionTerm,
    limits: ProjectionLimits,
    field: &str,
) -> Result<(), ProjectionError> {
    term.validate(limits)?;
    if matches!(term, ProjectionTerm::Literal { .. }) {
        return Err(ProjectionError::integrity(format!(
            "{field} must not be a literal"
        )));
    }
    let _ = term.to_canonical_json(limits)?;
    Ok(())
}

fn validate_asserted_resource(
    term: &ProjectionTerm,
    limits: ProjectionLimits,
    field: &str,
) -> Result<(), ProjectionError> {
    validate_resource(term, limits, field)?;
    if matches!(term, ProjectionTerm::Triple { .. }) {
        return Err(ProjectionError::integrity(format!(
            "{field} must be an IRI or blank node"
        )));
    }
    Ok(())
}

fn validate_graph_name(
    term: &ProjectionTerm,
    limits: ProjectionLimits,
    field: &str,
) -> Result<(), ProjectionError> {
    term.validate(limits)?;
    if !matches!(
        term,
        ProjectionTerm::Iri { .. } | ProjectionTerm::Blank { .. }
    ) {
        return Err(ProjectionError::integrity(format!(
            "{field} must be an IRI or blank node"
        )));
    }
    Ok(())
}

fn validate_context(
    context: &LpgGraphContext,
    limits: ProjectionLimits,
    named_graphs: &BTreeSet<ProjectionTerm>,
) -> Result<(), ProjectionError> {
    if let LpgGraphContext::Named { name } = context {
        validate_graph_name(name, limits, "LPG graph context")?;
        if !named_graphs.contains(name) {
            return Err(ProjectionError::integrity(
                "LPG record references an undeclared named graph",
            ));
        }
    }
    Ok(())
}

fn validate_rdf_quad(
    quad: &LpgRdfQuad,
    limits: ProjectionLimits,
    named_graphs: &BTreeSet<ProjectionTerm>,
) -> Result<(), ProjectionError> {
    validate_asserted_resource(&quad.subject, limits, "RDF statement subject")?;
    validate_iri_data(&quad.predicate, "RDF statement predicate")?;
    quad.object.validate(limits)?;
    let _ = quad.object.to_canonical_json(limits)?;
    validate_context(&quad.graph, limits, named_graphs)
}

fn insert_statement(
    id: &str,
    quad: &LpgRdfQuad,
    ids: &mut BTreeSet<String>,
    quads: &mut BTreeSet<LpgRdfQuad>,
) -> Result<(), ProjectionError> {
    if !ids.insert(id.to_owned()) || !quads.insert(quad.clone()) {
        return Err(ProjectionError::integrity(
            "duplicate or colliding RDF statement sideband",
        ));
    }
    Ok(())
}

fn collect_quad_nodes(quad: &LpgRdfQuad, nodes: &mut BTreeSet<ProjectionTerm>) {
    collect_node_terms(&quad.subject, nodes);
    collect_node_terms(&quad.object, nodes);
}

pub(super) fn collect_node_terms(term: &ProjectionTerm, nodes: &mut BTreeSet<ProjectionTerm>) {
    match term {
        ProjectionTerm::Literal { .. } => {}
        ProjectionTerm::Iri { .. } | ProjectionTerm::Blank { .. } => {
            nodes.insert(term.clone());
        }
        ProjectionTerm::Triple {
            subject, object, ..
        } => {
            nodes.insert(term.clone());
            collect_node_terms(subject, nodes);
            collect_node_terms(object, nodes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(32, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits")
    }

    #[test]
    fn config_round_trip_revalidates_and_rejects_unknown_fields() {
        let config = LpgConfig::new("http://example.org/type", limits(), 1_000).expect("config");
        let json = serde_json::to_string(&config).expect("serialize");
        assert_eq!(
            serde_json::from_str::<LpgConfig>(&json).expect("parse"),
            config
        );
        let mut unknown: serde_json::Value = serde_json::from_str(&json).expect("JSON value");
        unknown
            .as_object_mut()
            .expect("config object")
            .insert("unknown".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<LpgConfig>(unknown).is_err());
    }

    #[test]
    fn literal_atoms_are_deterministic_and_exact_sideband_independent() {
        let integer = ProjectionTerm::Literal {
            lexical: "42".to_owned(),
            datatype: format!("{XSD}integer"),
            language: None,
            direction: None,
        };
        assert_eq!(
            property_atom(&integer).expect("atom"),
            LpgPropertyAtom::Integer { value: 42 }
        );
        let negative_zero = ProjectionTerm::Literal {
            lexical: "-0.0".to_owned(),
            datatype: format!("{XSD}double"),
            language: None,
            direction: None,
        };
        assert_eq!(
            property_atom(&negative_zero)
                .expect("atom")
                .as_f64()
                .expect("float")
                .to_bits(),
            (-0.0f64).to_bits()
        );

        for (lexical, datatype) in [("1e2", "decimal"), ("++1", "double"), ("NaN", "integer")] {
            let invalid_numeric = ProjectionTerm::Literal {
                lexical: lexical.to_owned(),
                datatype: format!("{XSD}{datatype}"),
                language: None,
                direction: None,
            };
            assert_eq!(
                property_atom(&invalid_numeric).expect("text fallback"),
                LpgPropertyAtom::String {
                    value: lexical.to_owned()
                }
            );
        }

        let large_integer = ProjectionTerm::Literal {
            lexical: "9223372036854775808".to_owned(),
            datatype: format!("{XSD}integer"),
            language: None,
            direction: None,
        };
        assert_eq!(
            property_atom(&large_integer).expect("decimal fallback"),
            LpgPropertyAtom::Decimal {
                lexical: "9223372036854775808".to_owned()
            }
        );
    }
}
