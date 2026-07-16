// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic generic and Neo4j Admin Import CSV adapters.

use std::collections::{BTreeMap, BTreeSet};
use std::io;

use ::csv::{Reader, ReaderBuilder, StringRecord, Terminator, Trim, Writer, WriterBuilder};
use purrdf_core::{DatasetView, LossLedger};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage, ProjectionTerm};
use super::mapping::{LpgProjection, project_lpg};
use super::model::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgLabel, LpgNode, LpgProperty, LpgReifier,
    node_identifier,
};
use crate::stable_identifier;

const GENERIC_PROFILE: &str = "purrdf-lpg-csv";
const NEO4J_PROFILE: &str = "purrdf-lpg-neo4j-admin-csv";
const PROFILE_VERSION: u32 = 1;

const GENERIC_MANIFEST: &str = "manifest.json";
const GENERIC_NODES: &str = "nodes.csv";
const GENERIC_EDGES: &str = "edges.csv";
const GENERIC_SIDEBAND: &str = "rdf-sideband.csv";

const NEO4J_MANIFEST: &str = "neo4j/manifest.json";
const NEO4J_LABEL_MAP: &str = "neo4j/label-map.csv";
const NEO4J_PROPERTY_MAP: &str = "neo4j/property-map.csv";
const NEO4J_TYPE_MAP: &str = "neo4j/relationship-type-map.csv";
const NEO4J_SIDEBAND: &str = "neo4j/rdf-sideband.csv";
const NEO4J_NODE_PREFIX: &str = "neo4j/nodes/";
const NEO4J_REL_PREFIX: &str = "neo4j/relationships/";

const GENERIC_NODE_HEADER: &[&str] = &["id", "identity_json", "labels_json", "properties_json"];
const GENERIC_EDGE_HEADER: &[&str] = &["id", "source", "target", "edge_type", "rdf_json"];
const SIDEBAND_HEADER: &[&str] = &["kind", "id", "payload_json"];
const NEO4J_NODE_HEADER: &[&str] = &[
    "node_id:ID(PurRDF)",
    "identity_json:string",
    ":LABEL",
    "labels_json:string",
    "properties_json:string",
];
const NEO4J_REL_HEADER: &[&str] = &[
    "edge_id:string",
    ":START_ID(PurRDF)",
    ":END_ID(PurRDF)",
    ":TYPE",
    "rdf_json:string",
];
const NEO4J_MAP_HEADER: &[&str] = &["neo4j_token:string", "rdf_iri:string"];

/// A direct RDF projection into one deterministic LPG carrier package.
#[derive(Debug, Clone)]
pub struct LpgPackageProjection {
    /// Canonical LPG model from which the package was encoded.
    pub graph: LpgGraph,
    /// Deterministic in-memory carrier artifacts.
    pub package: ProjectionPackage,
    /// Always-computed RDF-to-LPG semantic-lowering ledger.
    pub loss_ledger: LossLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierManifest {
    profile: String,
    profile_version: u32,
    lpg_schema_version: u32,
}

/// Encode a canonical LPG as deterministic generic CSV artifacts.
///
/// The package contains `nodes.csv`, `edges.csv`, `rdf-sideband.csv`, and a
/// versioned manifest. Structured labels, properties, recursive terms, graph
/// declarations, reifiers, and annotations use canonical compact JSON cells.
///
/// # Errors
///
/// Returns a typed model, CSV, package, or resource-limit failure.
pub fn write_lpg_csv(
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<ProjectionPackage, ProjectionError> {
    graph.validate(config)?;
    let mut package = ProjectionPackage::new(config.limits());
    package.insert(GENERIC_NODES, write_generic_nodes(graph, config)?)?;
    package.insert(GENERIC_EDGES, write_generic_edges(graph, config)?)?;
    package.insert(
        GENERIC_SIDEBAND,
        write_sideband(graph, config, GENERIC_SIDEBAND)?,
    )?;
    package.insert(
        GENERIC_MANIFEST,
        write_manifest(GENERIC_PROFILE, graph, config)?,
    )?;
    Ok(package)
}

/// Project any RDF dataset view directly into the deterministic generic CSV package.
///
/// # Errors
///
/// Returns any canonical LPG projection or CSV/package encoding failure.
pub fn project_lpg_csv<D: DatasetView>(
    view: &D,
    config: &LpgConfig,
) -> Result<LpgPackageProjection, ProjectionError> {
    let LpgProjection { graph, loss_ledger } = project_lpg(view, config)?;
    let package = write_lpg_csv(&graph, config)?;
    Ok(LpgPackageProjection {
        graph,
        package,
        loss_ledger,
    })
}

/// Decode the strict generic CSV profile into its canonical LPG model.
///
/// # Errors
///
/// Rejects missing or extra artifacts, wrong headers, non-canonical CSV/JSON,
/// duplicate or unsorted records, dangling references, inconsistent sideband, and
/// all configured resource-limit breaches.
pub fn read_lpg_csv(
    package: &ProjectionPackage,
    config: &LpgConfig,
) -> Result<LpgGraph, ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    let manifest = read_manifest(
        required_artifact(package, GENERIC_MANIFEST)?,
        GENERIC_PROFILE,
        config,
        GENERIC_MANIFEST,
    )?;
    let mut budget = CsvRecordBudget::new(config.max_records());
    let nodes = read_generic_nodes(
        required_artifact(package, GENERIC_NODES)?,
        config,
        &mut budget,
    )?;
    let edges = read_generic_edges(
        required_artifact(package, GENERIC_EDGES)?,
        config,
        &mut budget,
    )?;
    let (named_graphs, reifiers, annotations) = read_sideband(
        required_artifact(package, GENERIC_SIDEBAND)?,
        config,
        &mut budget,
        GENERIC_SIDEBAND,
    )?;
    let graph = LpgGraph {
        schema_version: manifest.lpg_schema_version,
        nodes,
        edges,
        reifiers,
        annotations,
        named_graphs,
    };
    graph.validate(config)?;
    let canonical = write_lpg_csv(&graph, config)?;
    require_canonical_package(package, &canonical, GENERIC_PROFILE)?;
    Ok(graph)
}

/// Encode a canonical LPG as deterministic Neo4j Admin Import CSV artifacts.
///
/// Nodes are grouped by their exact native-label token set and relationships by
/// type. Full label, literal-property, and relationship IRIs are retained in
/// deterministic token maps. Every node group exposes one typed native property
/// column per RDF predicate, with a canonical typed-atom list as its string value,
/// while exact RDF identity remains in canonical JSON cells and the sideband
/// artifact.
///
/// # Errors
///
/// Returns a typed model, CSV, package, identifier-collision, or resource-limit
/// failure.
pub fn write_neo4j_csv(
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<ProjectionPackage, ProjectionError> {
    graph.validate(config)?;
    let label_map = label_token_map(graph)?;
    let property_map = property_token_map(graph)?;
    let type_map = relationship_token_map(graph)?;
    let mut package = ProjectionPackage::new(config.limits());
    package.insert(
        NEO4J_MANIFEST,
        write_manifest(NEO4J_PROFILE, graph, config)?,
    )?;
    package.insert(
        NEO4J_LABEL_MAP,
        write_token_map(&label_map, NEO4J_LABEL_MAP, config.limits())?,
    )?;
    package.insert(
        NEO4J_PROPERTY_MAP,
        write_token_map(&property_map, NEO4J_PROPERTY_MAP, config.limits())?,
    )?;
    package.insert(
        NEO4J_TYPE_MAP,
        write_token_map(&type_map, NEO4J_TYPE_MAP, config.limits())?,
    )?;
    package.insert(
        NEO4J_SIDEBAND,
        write_sideband(graph, config, NEO4J_SIDEBAND)?,
    )?;

    let mut node_groups: BTreeMap<Vec<String>, Vec<&LpgNode>> = BTreeMap::new();
    for node in &graph.nodes {
        let labels = native_labels(&node.labels)?;
        node_groups.entry(labels).or_default().push(node);
    }
    for (labels, nodes) in node_groups {
        let path = neo4j_node_path(&labels)?;
        let bytes = write_neo4j_nodes(&nodes, &labels, config, &path)?;
        package.insert(path, bytes)?;
    }

    let mut relationship_groups: BTreeMap<&str, Vec<&LpgEdge>> = BTreeMap::new();
    for edge in &graph.edges {
        relationship_groups
            .entry(&edge.edge_type)
            .or_default()
            .push(edge);
    }
    for (edge_type, edges) in relationship_groups {
        let token = relationship_token(edge_type)?;
        let path = format!("{NEO4J_REL_PREFIX}{token}.csv");
        let bytes = write_neo4j_relationships(&edges, &token, config, &path)?;
        package.insert(path, bytes)?;
    }

    Ok(package)
}

/// Project any RDF dataset view directly into Neo4j Admin Import CSV artifacts.
///
/// # Errors
///
/// Returns any canonical LPG projection or Neo4j CSV/package encoding failure.
pub fn project_neo4j_csv<D: DatasetView>(
    view: &D,
    config: &LpgConfig,
) -> Result<LpgPackageProjection, ProjectionError> {
    let LpgProjection { graph, loss_ledger } = project_lpg(view, config)?;
    let package = write_neo4j_csv(&graph, config)?;
    Ok(LpgPackageProjection {
        graph,
        package,
        loss_ledger,
    })
}

/// Decode the strict emitted Neo4j Admin Import profile into canonical LPG.
///
/// # Errors
///
/// Rejects malformed typed headers, unsafe/unknown group files, token-map drift,
/// duplicate records, non-canonical JSON/CSV, dangling ids, sideband inconsistency,
/// and configured resource-limit breaches.
pub fn read_neo4j_csv(
    package: &ProjectionPackage,
    config: &LpgConfig,
) -> Result<LpgGraph, ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    let manifest = read_manifest(
        required_artifact(package, NEO4J_MANIFEST)?,
        NEO4J_PROFILE,
        config,
        NEO4J_MANIFEST,
    )?;
    let label_map = read_token_map(
        required_artifact(package, NEO4J_LABEL_MAP)?,
        NEO4J_LABEL_MAP,
        label_token,
        config.max_records(),
    )?;
    let type_map = read_token_map(
        required_artifact(package, NEO4J_TYPE_MAP)?,
        NEO4J_TYPE_MAP,
        relationship_token,
        config.max_records(),
    )?;
    let property_map = read_token_map(
        required_artifact(package, NEO4J_PROPERTY_MAP)?,
        NEO4J_PROPERTY_MAP,
        property_token,
        config.max_records(),
    )?;

    let mut budget = CsvRecordBudget::new(config.max_records());
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (path, bytes) in package.artifacts() {
        if is_direct_csv_child(path, NEO4J_NODE_PREFIX) {
            nodes.extend(read_neo4j_nodes(
                bytes,
                path,
                config,
                &label_map,
                &property_map,
                &mut budget,
            )?);
        } else if is_direct_csv_child(path, NEO4J_REL_PREFIX) {
            edges.extend(read_neo4j_relationships(
                bytes,
                path,
                config,
                &type_map,
                &mut budget,
            )?);
        }
    }
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    edges.sort_by(|left, right| left.id.cmp(&right.id));

    let (named_graphs, reifiers, annotations) = read_sideband(
        required_artifact(package, NEO4J_SIDEBAND)?,
        config,
        &mut budget,
        NEO4J_SIDEBAND,
    )?;
    let graph = LpgGraph {
        schema_version: manifest.lpg_schema_version,
        nodes,
        edges,
        reifiers,
        annotations,
        named_graphs,
    };
    graph.validate(config)?;
    if label_map != label_token_map(&graph)? {
        return Err(ProjectionError::integrity(
            "Neo4j label map does not exactly match the graph's native labels",
        ));
    }
    if type_map != relationship_token_map(&graph)? {
        return Err(ProjectionError::integrity(
            "Neo4j relationship-type map does not exactly match the graph's edges",
        ));
    }
    if property_map != property_token_map(&graph)? {
        return Err(ProjectionError::integrity(
            "Neo4j property map does not exactly match the graph's literal properties",
        ));
    }
    let canonical = write_neo4j_csv(&graph, config)?;
    require_canonical_package(package, &canonical, NEO4J_PROFILE)?;
    Ok(graph)
}

fn write_manifest(
    profile: &str,
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<Vec<u8>, ProjectionError> {
    canonical_json_bounded(
        &CarrierManifest {
            profile: profile.to_owned(),
            profile_version: PROFILE_VERSION,
            lpg_schema_version: graph.schema_version,
        },
        config.limits(),
        "LPG CSV manifest",
    )
}

fn read_manifest(
    bytes: &[u8],
    profile: &str,
    config: &LpgConfig,
    path: &str,
) -> Result<CarrierManifest, ProjectionError> {
    let manifest: CarrierManifest = parse_json(bytes, config, "LPG CSV manifest", path)?;
    if manifest.profile != profile || manifest.profile_version != PROFILE_VERSION {
        return Err(ProjectionError::integrity(format!(
            "manifest identifies profile {:?} version {}; expected {profile:?} version {PROFILE_VERSION}",
            manifest.profile, manifest.profile_version
        ))
        .at_path(path));
    }
    Ok(manifest)
}

fn write_generic_nodes(graph: &LpgGraph, config: &LpgConfig) -> Result<Vec<u8>, ProjectionError> {
    let mut writer = csv_writer(GENERIC_NODE_HEADER, GENERIC_NODES, config.limits())?;
    for node in &graph.nodes {
        write_record(
            &mut writer,
            [
                node.id.clone(),
                json_cell(&node.identity, config, "LPG node identity")?,
                json_cell(&node.labels, config, "LPG node labels")?,
                json_cell(&node.properties, config, "LPG node properties")?,
            ],
            GENERIC_NODES,
        )?;
    }
    finish_csv(writer, GENERIC_NODES)
}

fn write_generic_edges(graph: &LpgGraph, config: &LpgConfig) -> Result<Vec<u8>, ProjectionError> {
    let mut writer = csv_writer(GENERIC_EDGE_HEADER, GENERIC_EDGES, config.limits())?;
    for edge in &graph.edges {
        write_record(
            &mut writer,
            [
                edge.id.clone(),
                edge.source.clone(),
                edge.target.clone(),
                edge.edge_type.clone(),
                json_cell(&edge.rdf, config, "LPG edge RDF sideband")?,
            ],
            GENERIC_EDGES,
        )?;
    }
    finish_csv(writer, GENERIC_EDGES)
}

fn read_generic_nodes(
    bytes: &[u8],
    config: &LpgConfig,
    budget: &mut CsvRecordBudget,
) -> Result<Vec<LpgNode>, ProjectionError> {
    let mut reader = csv_reader(bytes, GENERIC_NODE_HEADER, GENERIC_NODES)?;
    let mut nodes = Vec::new();
    for (index, row) in reader.records().enumerate() {
        let path = row_path(GENERIC_NODES, index);
        let row = row.map_err(|error| csv_read_error(&error, &path))?;
        let identity = parse_json_cell(field(&row, 1, &path)?, config, "node identity", &path)?;
        let labels: Vec<LpgLabel> =
            parse_json_cell(field(&row, 2, &path)?, config, "node labels", &path)?;
        let properties: Vec<LpgProperty> =
            parse_json_cell(field(&row, 3, &path)?, config, "node properties", &path)?;
        budget.consume_nested(1, labels.len(), properties.len(), "generic LPG node")?;
        nodes.push(LpgNode {
            id: field(&row, 0, &path)?.to_owned(),
            identity,
            labels,
            properties,
        });
    }
    Ok(nodes)
}

fn read_generic_edges(
    bytes: &[u8],
    config: &LpgConfig,
    budget: &mut CsvRecordBudget,
) -> Result<Vec<LpgEdge>, ProjectionError> {
    let mut reader = csv_reader(bytes, GENERIC_EDGE_HEADER, GENERIC_EDGES)?;
    let mut edges = Vec::new();
    for (index, row) in reader.records().enumerate() {
        let path = row_path(GENERIC_EDGES, index);
        let row = row.map_err(|error| csv_read_error(&error, &path))?;
        budget.consume(1, "generic LPG edge")?;
        edges.push(LpgEdge {
            id: field(&row, 0, &path)?.to_owned(),
            source: field(&row, 1, &path)?.to_owned(),
            target: field(&row, 2, &path)?.to_owned(),
            edge_type: field(&row, 3, &path)?.to_owned(),
            rdf: parse_json_cell(field(&row, 4, &path)?, config, "edge sideband", &path)?,
        });
    }
    Ok(edges)
}

fn write_sideband(
    graph: &LpgGraph,
    config: &LpgConfig,
    path: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let mut named_graphs = Vec::with_capacity(graph.named_graphs.len());
    for graph_name in &graph.named_graphs {
        named_graphs.push((node_identifier(graph_name, config.limits())?, graph_name));
    }
    named_graphs.sort_by(|left, right| left.0.cmp(&right.0));

    let mut writer = csv_writer(SIDEBAND_HEADER, path, config.limits())?;
    for (id, graph_name) in named_graphs {
        write_record(
            &mut writer,
            [
                "named-graph".to_owned(),
                id,
                json_cell(graph_name, config, "named graph")?,
            ],
            path,
        )?;
    }
    for row in &graph.reifiers {
        write_record(
            &mut writer,
            [
                "reifier".to_owned(),
                row.id.clone(),
                json_cell(row, config, "LPG reifier")?,
            ],
            path,
        )?;
    }
    for row in &graph.annotations {
        write_record(
            &mut writer,
            [
                "annotation".to_owned(),
                row.id.clone(),
                json_cell(row, config, "LPG annotation")?,
            ],
            path,
        )?;
    }
    finish_csv(writer, path)
}

type SidebandRows = (Vec<ProjectionTerm>, Vec<LpgReifier>, Vec<LpgAnnotation>);

fn read_sideband(
    bytes: &[u8],
    config: &LpgConfig,
    budget: &mut CsvRecordBudget,
    path: &str,
) -> Result<SidebandRows, ProjectionError> {
    let mut reader = csv_reader(bytes, SIDEBAND_HEADER, path)?;
    let mut named_graphs = Vec::new();
    let mut reifiers = Vec::new();
    let mut annotations = Vec::new();
    let mut previous: Option<(u8, String)> = None;
    for (index, row) in reader.records().enumerate() {
        let row_path = row_path(path, index);
        let row = row.map_err(|error| csv_read_error(&error, &row_path))?;
        let kind = field(&row, 0, &row_path)?;
        let id = field(&row, 1, &row_path)?;
        let rank = match kind {
            "named-graph" => 0,
            "reifier" => 1,
            "annotation" => 2,
            _ => {
                return Err(ProjectionError::syntax(format!(
                    "unknown RDF sideband record kind {kind:?}"
                ))
                .at_path(row_path));
            }
        };
        let key = (rank, id.to_owned());
        if previous.as_ref().is_some_and(|last| last >= &key) {
            return Err(ProjectionError::integrity(
                "RDF sideband rows must be strictly ordered by kind and id",
            )
            .at_path(row_path));
        }
        previous = Some(key);
        budget.consume(1, "LPG RDF sideband row")?;
        match kind {
            "named-graph" => {
                let graph_name: ProjectionTerm =
                    parse_json_cell(field(&row, 2, &row_path)?, config, "named graph", &row_path)?;
                if node_identifier(&graph_name, config.limits())? != id {
                    return Err(ProjectionError::integrity(
                        "named-graph sideband id disagrees with its RDF term",
                    )
                    .at_path(row_path));
                }
                named_graphs.push(graph_name);
            }
            "reifier" => {
                let reifier: LpgReifier =
                    parse_json_cell(field(&row, 2, &row_path)?, config, "reifier", &row_path)?;
                if reifier.id != id {
                    return Err(ProjectionError::integrity(
                        "reifier sideband id disagrees with its payload",
                    )
                    .at_path(row_path));
                }
                reifiers.push(reifier);
            }
            "annotation" => {
                let annotation: LpgAnnotation =
                    parse_json_cell(field(&row, 2, &row_path)?, config, "annotation", &row_path)?;
                if annotation.id != id {
                    return Err(ProjectionError::integrity(
                        "annotation sideband id disagrees with its payload",
                    )
                    .at_path(row_path));
                }
                annotations.push(annotation);
            }
            _ => unreachable!("sideband kind was matched above"),
        }
    }
    named_graphs.sort();
    Ok((named_graphs, reifiers, annotations))
}

fn write_neo4j_nodes(
    nodes: &[&LpgNode],
    labels: &[String],
    config: &LpgConfig,
    path: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let property_columns = group_property_columns(nodes)?;
    let mut header: Vec<String> = NEO4J_NODE_HEADER
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    header.extend(
        property_columns
            .keys()
            .map(|token| format!("{token}:string")),
    );
    let header_refs: Vec<&str> = header.iter().map(String::as_str).collect();
    let mut writer = csv_writer(&header_refs, path, config.limits())?;
    let label_cell = labels.join(";");
    for node in nodes {
        let mut record = vec![
            node.id.clone(),
            json_cell(&node.identity, config, "Neo4j node identity")?,
            label_cell.clone(),
            json_cell(&node.labels, config, "Neo4j node labels")?,
            json_cell(&node.properties, config, "Neo4j node properties")?,
        ];
        for iri in property_columns.values() {
            record.push(native_property_cell(&node.properties, iri, config)?);
        }
        write_record(&mut writer, record, path)?;
    }
    finish_csv(writer, path)
}

fn read_neo4j_nodes(
    bytes: &[u8],
    path: &str,
    config: &LpgConfig,
    label_map: &BTreeMap<String, String>,
    property_map: &BTreeMap<String, String>,
    budget: &mut CsvRecordBudget,
) -> Result<Vec<LpgNode>, ProjectionError> {
    let (mut reader, property_tokens) = neo4j_node_reader(bytes, path, property_map)?;
    let mut nodes = Vec::new();
    let mut seen_property_tokens = BTreeSet::new();
    let mut previous_id: Option<String> = None;
    for (index, row) in reader.records().enumerate() {
        let row_path = row_path(path, index);
        let row = row.map_err(|error| csv_read_error(&error, &row_path))?;
        let id = field(&row, 0, &row_path)?.to_owned();
        if previous_id.as_ref().is_some_and(|previous| previous >= &id) {
            return Err(ProjectionError::integrity(
                "Neo4j node group must be strictly ordered by node id",
            )
            .at_path(row_path));
        }
        previous_id = Some(id.clone());
        let identity = parse_json_cell(
            field(&row, 1, &row_path)?,
            config,
            "node identity",
            &row_path,
        )?;
        let labels: Vec<LpgLabel> =
            parse_json_cell(field(&row, 3, &row_path)?, config, "node labels", &row_path)?;
        let properties: Vec<LpgProperty> = parse_json_cell(
            field(&row, 4, &row_path)?,
            config,
            "node properties",
            &row_path,
        )?;
        let native = native_labels(&labels)?;
        if native.join(";") != field(&row, 2, &row_path)? {
            return Err(ProjectionError::integrity(
                "Neo4j native label field disagrees with exact LPG labels",
            )
            .at_path(row_path));
        }
        for label in &labels {
            let token = label_token(&label.value)?;
            if label_map.get(&token).map(String::as_str) != Some(label.value.as_str()) {
                return Err(ProjectionError::integrity(
                    "Neo4j native label is absent from or inconsistent with label-map.csv",
                )
                .at_path(row_path));
            }
        }
        for (offset, token) in property_tokens.iter().enumerate() {
            let iri = property_map
                .get(token)
                .expect("node header token was validated against the property map");
            let expected = native_property_cell(&properties, iri, config)?;
            if field(&row, NEO4J_NODE_HEADER.len() + offset, &row_path)? != expected {
                return Err(ProjectionError::integrity(
                    "Neo4j native property field disagrees with exact LPG properties",
                )
                .at_path(row_path));
            }
        }
        for property in &properties {
            let token = property_token(&property.key)?;
            if property_tokens.binary_search(&token).is_err() {
                return Err(ProjectionError::integrity(
                    "Neo4j node property is absent from its stable property columns",
                )
                .at_path(row_path));
            }
            seen_property_tokens.insert(token);
        }
        if neo4j_node_path(&native)? != path {
            return Err(ProjectionError::integrity(
                "Neo4j node is stored in the wrong stable label group",
            )
            .at_path(row_path));
        }
        budget.consume_nested(1, labels.len(), properties.len(), "Neo4j LPG node")?;
        nodes.push(LpgNode {
            id,
            identity,
            labels,
            properties,
        });
    }
    if property_tokens.iter().ne(seen_property_tokens.iter()) {
        return Err(ProjectionError::integrity(
            "Neo4j node group contains an unused or missing native property column",
        )
        .at_path(path));
    }
    Ok(nodes)
}

fn neo4j_node_reader<'a>(
    bytes: &'a [u8],
    path: &str,
    property_map: &BTreeMap<String, String>,
) -> Result<(Reader<&'a [u8]>, Vec<String>), ProjectionError> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .trim(Trim::None)
        .from_reader(bytes);
    let actual = reader
        .headers()
        .map_err(|error| csv_read_error(&error, path))?;
    if actual.len() < NEO4J_NODE_HEADER.len()
        || actual
            .iter()
            .take(NEO4J_NODE_HEADER.len())
            .ne(NEO4J_NODE_HEADER.iter().copied())
    {
        return Err(ProjectionError::syntax(
            "Neo4j node CSV does not begin with the required typed header",
        )
        .at_path(path));
    }
    let mut tokens = Vec::new();
    for column in actual.iter().skip(NEO4J_NODE_HEADER.len()) {
        let Some(token) = column.strip_suffix(":string") else {
            return Err(ProjectionError::syntax(
                "Neo4j native property columns must use explicit :string typing",
            )
            .at_path(path));
        };
        if token.is_empty()
            || tokens
                .last()
                .is_some_and(|previous: &String| previous.as_str() >= token)
            || !property_map.contains_key(token)
        {
            return Err(ProjectionError::integrity(
                "Neo4j native property columns must be mapped, unique, and strictly ordered",
            )
            .at_path(path));
        }
        tokens.push(token.to_owned());
    }
    Ok((reader, tokens))
}

fn group_property_columns(nodes: &[&LpgNode]) -> Result<BTreeMap<String, String>, ProjectionError> {
    let mut columns = BTreeMap::new();
    for property in nodes.iter().flat_map(|node| node.properties.iter()) {
        insert_token(&mut columns, property_token(&property.key)?, &property.key)?;
    }
    Ok(columns)
}

fn native_property_cell(
    properties: &[LpgProperty],
    iri: &str,
    config: &LpgConfig,
) -> Result<String, ProjectionError> {
    let values: Vec<_> = properties
        .iter()
        .filter(|property| property.key == iri)
        .map(|property| &property.value)
        .collect();
    if values.is_empty() {
        Ok(String::new())
    } else {
        json_cell(&values, config, "Neo4j native property values")
    }
}

fn write_neo4j_relationships(
    edges: &[&LpgEdge],
    token: &str,
    config: &LpgConfig,
    path: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let mut writer = csv_writer(NEO4J_REL_HEADER, path, config.limits())?;
    for edge in edges {
        write_record(
            &mut writer,
            [
                edge.id.clone(),
                edge.source.clone(),
                edge.target.clone(),
                token.to_owned(),
                json_cell(&edge.rdf, config, "Neo4j relationship RDF sideband")?,
            ],
            path,
        )?;
    }
    finish_csv(writer, path)
}

fn read_neo4j_relationships(
    bytes: &[u8],
    path: &str,
    config: &LpgConfig,
    type_map: &BTreeMap<String, String>,
    budget: &mut CsvRecordBudget,
) -> Result<Vec<LpgEdge>, ProjectionError> {
    let mut reader = csv_reader(bytes, NEO4J_REL_HEADER, path)?;
    let mut edges = Vec::new();
    let mut previous_id: Option<String> = None;
    for (index, row) in reader.records().enumerate() {
        let row_path = row_path(path, index);
        let row = row.map_err(|error| csv_read_error(&error, &row_path))?;
        let id = field(&row, 0, &row_path)?.to_owned();
        if previous_id.as_ref().is_some_and(|previous| previous >= &id) {
            return Err(ProjectionError::integrity(
                "Neo4j relationship group must be strictly ordered by edge id",
            )
            .at_path(row_path));
        }
        previous_id = Some(id.clone());
        let rdf = parse_json_cell(
            field(&row, 4, &row_path)?,
            config,
            "relationship sideband",
            &row_path,
        )?;
        let edge_type = type_map
            .get(field(&row, 3, &row_path)?)
            .ok_or_else(|| {
                ProjectionError::integrity(
                    "Neo4j relationship token is absent from relationship-type-map.csv",
                )
                .at_path(&row_path)
            })?
            .clone();
        let token = relationship_token(&edge_type)?;
        if token != field(&row, 3, &row_path)? || format!("{NEO4J_REL_PREFIX}{token}.csv") != path {
            return Err(ProjectionError::integrity(
                "Neo4j relationship is stored in the wrong stable type group",
            )
            .at_path(row_path));
        }
        budget.consume(1, "Neo4j LPG relationship")?;
        edges.push(LpgEdge {
            id,
            source: field(&row, 1, &row_path)?.to_owned(),
            target: field(&row, 2, &row_path)?.to_owned(),
            edge_type,
            rdf,
        });
    }
    Ok(edges)
}

fn write_token_map(
    map: &BTreeMap<String, String>,
    path: &str,
    limits: ProjectionLimits,
) -> Result<Vec<u8>, ProjectionError> {
    let mut writer = csv_writer(NEO4J_MAP_HEADER, path, limits)?;
    for (token, iri) in map {
        write_record(&mut writer, [token.as_str(), iri.as_str()], path)?;
    }
    finish_csv(writer, path)
}

fn read_token_map(
    bytes: &[u8],
    path: &str,
    expected_token: fn(&str) -> Result<String, ProjectionError>,
    max_rows: usize,
) -> Result<BTreeMap<String, String>, ProjectionError> {
    let mut reader = csv_reader(bytes, NEO4J_MAP_HEADER, path)?;
    let mut map = BTreeMap::new();
    let mut iris = BTreeSet::new();
    for (index, row) in reader.records().enumerate() {
        let row_path = row_path(path, index);
        let row = row.map_err(|error| csv_read_error(&error, &row_path))?;
        if map.len() >= max_rows {
            return Err(ProjectionError::limit(format!(
                "Neo4j token map exceeds the {max_rows}-row LPG limit"
            ))
            .at_path(row_path));
        }
        let token = field(&row, 0, &row_path)?;
        let iri = field(&row, 1, &row_path)?;
        if expected_token(iri)? != token {
            return Err(
                ProjectionError::integrity("Neo4j token does not match the full RDF IRI")
                    .at_path(row_path),
            );
        }
        if map.insert(token.to_owned(), iri.to_owned()).is_some() || !iris.insert(iri.to_owned()) {
            return Err(ProjectionError::integrity(
                "Neo4j token map contains a duplicate token or RDF IRI",
            )
            .at_path(row_path));
        }
    }
    Ok(map)
}

fn label_token_map(graph: &LpgGraph) -> Result<BTreeMap<String, String>, ProjectionError> {
    let mut map = BTreeMap::new();
    for label in graph.nodes.iter().flat_map(|node| &node.labels) {
        insert_token(&mut map, label_token(&label.value)?, &label.value)?;
    }
    Ok(map)
}

fn property_token_map(graph: &LpgGraph) -> Result<BTreeMap<String, String>, ProjectionError> {
    let mut map = BTreeMap::new();
    for property in graph.nodes.iter().flat_map(|node| &node.properties) {
        insert_token(&mut map, property_token(&property.key)?, &property.key)?;
    }
    Ok(map)
}

fn relationship_token_map(graph: &LpgGraph) -> Result<BTreeMap<String, String>, ProjectionError> {
    let mut map = BTreeMap::new();
    for edge in &graph.edges {
        insert_token(
            &mut map,
            relationship_token(&edge.edge_type)?,
            &edge.edge_type,
        )?;
    }
    Ok(map)
}

fn insert_token(
    map: &mut BTreeMap<String, String>,
    token: String,
    iri: &str,
) -> Result<(), ProjectionError> {
    if let Some(existing) = map.insert(token, iri.to_owned())
        && existing != iri
    {
        return Err(ProjectionError::integrity(
            "SHA-256 collision between distinct Neo4j token IRIs",
        ));
    }
    Ok(())
}

fn native_labels(labels: &[LpgLabel]) -> Result<Vec<String>, ProjectionError> {
    labels
        .iter()
        .map(|label| label_token(&label.value))
        .collect::<Result<BTreeSet<_>, _>>()
        .map(BTreeSet::into_iter)
        .map(Iterator::collect)
}

fn label_token(iri: &str) -> Result<String, ProjectionError> {
    stable_identifier("RdfLabel", iri.as_bytes())
}

fn relationship_token(iri: &str) -> Result<String, ProjectionError> {
    stable_identifier("RdfEdge", iri.as_bytes())
}

fn property_token(iri: &str) -> Result<String, ProjectionError> {
    stable_identifier("RdfProp", iri.as_bytes())
}

fn neo4j_node_path(labels: &[String]) -> Result<String, ProjectionError> {
    let key = labels.join("\0");
    Ok(format!(
        "{NEO4J_NODE_PREFIX}{}.csv",
        stable_identifier("group", key.as_bytes())?
    ))
}

fn is_direct_csv_child(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix).is_some_and(|name| {
        !name.is_empty() && !name.contains('/') && name.as_bytes().ends_with(b".csv")
    })
}

struct LimitedCsvBytes {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl LimitedCsvBytes {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }
}

impl io::Write for LimitedCsvBytes {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(buffer.len())
            .is_none_or(|length| length > self.limit)
        {
            self.exceeded = true;
            return Err(io::Error::other("projection CSV byte limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

type CsvWriter = Writer<LimitedCsvBytes>;

fn csv_writer(
    header: &[&str],
    path: &str,
    limits: ProjectionLimits,
) -> Result<CsvWriter, ProjectionError> {
    let mut writer = WriterBuilder::new()
        .terminator(Terminator::Any(b'\n'))
        .from_writer(LimitedCsvBytes::new(limits.max_artifact_bytes()));
    if let Err(error) = writer.write_record(header) {
        return Err(csv_write_error(&writer, &error, "write CSV header", path));
    }
    Ok(writer)
}

fn write_record<I, T>(writer: &mut CsvWriter, record: I, path: &str) -> Result<(), ProjectionError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    if let Err(error) = writer.write_record(record) {
        return Err(csv_write_error(writer, &error, "write CSV record", path));
    }
    Ok(())
}

fn finish_csv(mut writer: CsvWriter, path: &str) -> Result<Vec<u8>, ProjectionError> {
    if let Err(error) = writer.flush() {
        if writer.get_ref().exceeded {
            return Err(ProjectionError::limit(format!(
                "CSV artifact exceeds the {}-byte limit",
                writer.get_ref().limit
            ))
            .at_path(path));
        }
        return Err(ProjectionError::integrity(format!("flush CSV: {error}")).at_path(path));
    }
    writer
        .into_inner()
        .map(|output| output.bytes)
        .map_err(|error| ProjectionError::integrity(format!("finish CSV: {error}")).at_path(path))
}

fn csv_write_error(
    writer: &CsvWriter,
    error: &::csv::Error,
    action: &str,
    path: &str,
) -> ProjectionError {
    if writer.get_ref().exceeded {
        ProjectionError::limit(format!(
            "CSV artifact exceeds the {}-byte limit",
            writer.get_ref().limit
        ))
        .at_path(path)
    } else {
        ProjectionError::integrity(format!("{action}: {error}")).at_path(path)
    }
}

fn csv_reader<'a>(
    bytes: &'a [u8],
    expected_header: &[&str],
    path: &str,
) -> Result<Reader<&'a [u8]>, ProjectionError> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .trim(Trim::None)
        .from_reader(bytes);
    let actual = reader
        .headers()
        .map_err(|error| csv_read_error(&error, path))?;
    if actual.len() != expected_header.len() || actual.iter().ne(expected_header.iter().copied()) {
        return Err(ProjectionError::syntax(format!(
            "CSV header {actual:?} does not match required header {expected_header:?}"
        ))
        .at_path(path));
    }
    Ok(reader)
}

fn field<'a>(row: &'a StringRecord, index: usize, path: &str) -> Result<&'a str, ProjectionError> {
    row.get(index).ok_or_else(|| {
        ProjectionError::syntax(format!("CSV row is missing field {index}")).at_path(path)
    })
}

fn csv_read_error(error: &::csv::Error, path: &str) -> ProjectionError {
    ProjectionError::syntax(format!("read CSV: {error}")).at_path(path)
}

fn row_path(path: &str, zero_based_record: usize) -> String {
    format!("{path}:{}", zero_based_record + 2)
}

fn json_cell<T: Serialize>(
    value: &T,
    config: &LpgConfig,
    description: &str,
) -> Result<String, ProjectionError> {
    String::from_utf8(canonical_json_bounded(value, config.limits(), description)?).map_err(
        |error| ProjectionError::integrity(format!("JSON encoder emitted non-UTF-8: {error}")),
    )
}

fn parse_json_cell<T: DeserializeOwned + Serialize>(
    value: &str,
    config: &LpgConfig,
    description: &str,
    path: &str,
) -> Result<T, ProjectionError> {
    parse_json(value.as_bytes(), config, description, path)
}

fn parse_json<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    config: &LpgConfig,
    description: &str,
    path: &str,
) -> Result<T, ProjectionError> {
    if bytes.len() > config.limits().max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} exceeds the per-artifact byte limit"
        ))
        .at_path(path));
    }
    let value: T = serde_json::from_slice(bytes).map_err(|error| {
        ProjectionError::syntax(format!("parse {description} JSON: {error}")).at_path(path)
    })?;
    let canonical = canonical_json_bounded(&value, config.limits(), description)?;
    if canonical != bytes {
        return Err(ProjectionError::syntax(format!(
            "{description} JSON is not in canonical PurRDF form"
        ))
        .at_path(path));
    }
    Ok(value)
}

fn required_artifact<'a>(
    package: &'a ProjectionPackage,
    path: &str,
) -> Result<&'a [u8], ProjectionError> {
    package
        .get(path)
        .ok_or_else(|| ProjectionError::package("required artifact is missing").at_path(path))
}

fn validate_package_bounds(
    package: &ProjectionPackage,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if package.len() > limits.max_artifacts() {
        return Err(ProjectionError::limit(format!(
            "package has {} artifacts; reader limit is {}",
            package.len(),
            limits.max_artifacts()
        )));
    }
    if package.total_bytes() > limits.max_total_bytes()
        || package.archive_bytes() > limits.max_archive_bytes()
    {
        return Err(ProjectionError::limit(
            "package exceeds the configured total or archive byte limit",
        ));
    }
    for (path, bytes) in package.artifacts() {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "artifact is {} bytes; reader limit is {}",
                bytes.len(),
                limits.max_artifact_bytes()
            ))
            .at_path(path));
        }
    }
    Ok(())
}

fn require_canonical_package(
    actual: &ProjectionPackage,
    canonical: &ProjectionPackage,
    profile: &str,
) -> Result<(), ProjectionError> {
    if !actual.artifacts().eq(canonical.artifacts()) {
        return Err(ProjectionError::syntax(format!(
            "{profile} package is valid but not in canonical PurRDF form"
        )));
    }
    Ok(())
}

struct CsvRecordBudget {
    used: usize,
    maximum: usize,
}

impl CsvRecordBudget {
    const fn new(maximum: usize) -> Self {
        Self { used: 0, maximum }
    }

    fn consume(&mut self, amount: usize, description: &str) -> Result<(), ProjectionError> {
        self.used = self
            .used
            .checked_add(amount)
            .ok_or_else(|| ProjectionError::limit("LPG CSV record count overflow"))?;
        if self.used > self.maximum {
            return Err(ProjectionError::limit(format!(
                "{description} exceeds the {}-record LPG limit",
                self.maximum
            )));
        }
        Ok(())
    }

    fn consume_nested(
        &mut self,
        outer: usize,
        first: usize,
        second: usize,
        description: &str,
    ) -> Result<(), ProjectionError> {
        let amount = outer
            .checked_add(first)
            .and_then(|value| value.checked_add(second))
            .ok_or_else(|| ProjectionError::limit("LPG CSV record count overflow"))?;
        self.consume(amount, description)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{
        BlankScope, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
        datasets_isomorphic,
    };

    use super::*;
    use crate::lift_lpg;

    const TYPE: &str = "http://example.org/type";

    fn test_config(max_records: usize) -> LpgConfig {
        LpgConfig::new(
            TYPE,
            ProjectionLimits::new(128, 4_000_000, 12_000_000, 16_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn fixture() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let target = builder.intern_iri("http://example.org/target");
        let blank = builder.intern_blank("blank", BlankScope(7));
        let graph = builder.intern_iri("http://example.org/graph");
        let rdf_type = builder.intern_iri(TYPE);
        let first_class = builder.intern_iri("http://example.org/FirstClass");
        let second_class = builder.intern_iri("http://example.org/SecondClass");
        builder.push_quad(subject, rdf_type, first_class, Some(graph));
        builder.push_quad(subject, rdf_type, second_class, None);

        let label = builder.intern_iri("http://example.org/label");
        let hostile = builder.intern_literal(RdfLiteral::simple("comma, quote \" and\nline"));
        builder.push_quad(blank, label, hostile, Some(graph));
        let number_predicate = builder.intern_iri("http://example.org/count");
        let number = builder.intern_literal(RdfLiteral::typed(
            "9223372036854775808",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        builder.push_quad(subject, number_predicate, number, None);

        let relates = builder.intern_iri("http://example.org/relates");
        builder.push_quad(subject, relates, target, None);
        let contains = builder.intern_iri("http://example.org/contains");
        builder.push_quad(target, contains, blank, Some(graph));

        let quoted = builder.intern_triple(subject, relates, target);
        let quotes = builder.intern_iri("http://example.org/quotes");
        builder.push_quad(subject, quotes, quoted, Some(graph));
        let reifier = builder.intern_blank("reifier", BlankScope(9));
        builder.push_reifier_in_graph(reifier, quoted, Some(graph));
        let confidence = builder.intern_iri("http://example.org/confidence");
        let high = builder.intern_iri("http://example.org/high");
        builder.push_annotation_in_graph(reifier, confidence, high, Some(graph));
        builder.freeze().expect("fixture")
    }

    fn same_artifacts(left: &ProjectionPackage, right: &ProjectionPackage) -> bool {
        left.artifacts().eq(right.artifacts())
    }

    fn replace_artifact(
        package: &ProjectionPackage,
        path: &str,
        replacement: &[u8],
    ) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(
            package.limits(),
            package.artifacts().map(|(candidate, bytes)| {
                (
                    candidate.to_owned(),
                    if candidate == path {
                        replacement.to_vec()
                    } else {
                        bytes.to_vec()
                    },
                )
            }),
        )
        .expect("replacement package")
    }

    #[test]
    fn generic_csv_round_trip_is_exact_byte_stable_and_quoted() {
        let dataset = fixture();
        let config = test_config(1_000);
        let projected = project_lpg_csv(dataset.as_ref(), &config).expect("project CSV");
        assert_eq!(
            projected
                .package
                .artifacts()
                .map(|(path, _)| path)
                .collect::<Vec<_>>(),
            vec![
                GENERIC_EDGES,
                GENERIC_MANIFEST,
                GENERIC_NODES,
                GENERIC_SIDEBAND
            ]
        );
        assert!(
            projected
                .package
                .get(GENERIC_NODES)
                .expect("nodes")
                .windows(2)
                .any(|pair| pair == b"\"\"")
        );

        let decoded = read_lpg_csv(&projected.package, &config).expect("read CSV");
        assert_eq!(decoded, projected.graph);
        let rewritten = write_lpg_csv(&decoded, &config).expect("rewrite CSV");
        assert!(same_artifacts(&projected.package, &rewritten));
        let lifted = lift_lpg(&decoded, &config).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));
    }

    #[test]
    fn neo4j_csv_groups_use_typed_headers_and_round_trip_exactly() {
        let dataset = fixture();
        let config = test_config(1_000);
        let projected = project_neo4j_csv(dataset.as_ref(), &config).expect("project Neo4j");
        let node_groups: Vec<_> = projected
            .package
            .artifacts()
            .filter(|(path, _)| is_direct_csv_child(path, NEO4J_NODE_PREFIX))
            .collect();
        let relationship_groups: Vec<_> = projected
            .package
            .artifacts()
            .filter(|(path, _)| is_direct_csv_child(path, NEO4J_REL_PREFIX))
            .collect();
        assert!(node_groups.len() >= 2);
        assert!(relationship_groups.len() >= 2);
        assert!(node_groups.iter().all(|(_, bytes)| {
            bytes.starts_with(b"node_id:ID(PurRDF),identity_json:string,:LABEL")
        }));
        assert!(node_groups.iter().any(|(_, bytes)| {
            std::str::from_utf8(bytes)
                .expect("UTF-8")
                .lines()
                .next()
                .is_some_and(|header| header.contains("RdfProp_") && header.contains(":string"))
        }));
        assert!(
            std::str::from_utf8(
                projected
                    .package
                    .get(NEO4J_PROPERTY_MAP)
                    .expect("property map")
            )
            .expect("UTF-8")
            .contains("RdfProp_")
        );
        assert!(relationship_groups.iter().all(|(_, bytes)| {
            bytes.starts_with(b"edge_id:string,:START_ID(PurRDF),:END_ID(PurRDF),:TYPE")
        }));

        let decoded = read_neo4j_csv(&projected.package, &config).expect("read Neo4j");
        assert_eq!(decoded, projected.graph);
        let rewritten = write_neo4j_csv(&decoded, &config).expect("rewrite Neo4j");
        assert!(same_artifacts(&projected.package, &rewritten));
        let lifted = lift_lpg(&decoded, &config).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));
    }

    #[test]
    fn both_csv_profiles_are_backend_and_archive_deterministic() {
        let dataset = fixture();
        let config = test_config(1_000);
        let pack = PackBuilder::build_bytes(&dataset).expect("pack");
        let view = PackView::from_bytes(&pack).expect("pack view");

        let generic_dataset = project_lpg_csv(dataset.as_ref(), &config).expect("dataset CSV");
        let generic_pack = project_lpg_csv(&view, &config).expect("pack CSV");
        assert!(same_artifacts(
            &generic_dataset.package,
            &generic_pack.package
        ));
        assert_eq!(
            generic_dataset.loss_ledger.render_json(),
            generic_pack.loss_ledger.render_json()
        );
        assert_eq!(
            generic_dataset.package.to_ustar().expect("archive"),
            generic_pack.package.to_ustar().expect("archive")
        );

        let neo4j_dataset = project_neo4j_csv(dataset.as_ref(), &config).expect("dataset Neo4j");
        let neo4j_pack = project_neo4j_csv(&view, &config).expect("pack Neo4j");
        assert!(same_artifacts(&neo4j_dataset.package, &neo4j_pack.package));
        assert_eq!(
            neo4j_dataset.loss_ledger.render_json(),
            neo4j_pack.loss_ledger.render_json()
        );
        assert_eq!(
            neo4j_dataset.package.to_ustar().expect("archive"),
            neo4j_pack.package.to_ustar().expect("archive")
        );
    }

    #[test]
    fn generic_reader_rejects_header_order_reference_and_limit_corruption() {
        let config = test_config(1_000);
        let projected = project_lpg_csv(fixture().as_ref(), &config).expect("project");

        let nodes = projected.package.get(GENERIC_NODES).expect("nodes");
        let wrong_header = String::from_utf8(nodes.to_vec())
            .expect("UTF-8")
            .replacen("id,identity_json", "identity_json,id", 1)
            .into_bytes();
        assert!(
            read_lpg_csv(
                &replace_artifact(&projected.package, GENERIC_NODES, &wrong_header),
                &config
            )
            .is_err()
        );

        let mut padded = nodes.to_vec();
        padded.push(b'\n');
        assert!(
            read_lpg_csv(
                &replace_artifact(&projected.package, GENERIC_NODES, &padded),
                &config
            )
            .is_err()
        );

        let first_node = std::str::from_utf8(nodes)
            .expect("UTF-8")
            .lines()
            .nth(1)
            .expect("node row");
        let mut duplicate = nodes.to_vec();
        duplicate.extend_from_slice(first_node.as_bytes());
        duplicate.push(b'\n');
        assert!(
            read_lpg_csv(
                &replace_artifact(&projected.package, GENERIC_NODES, &duplicate),
                &config
            )
            .is_err()
        );

        let mut reader = ReaderBuilder::new().from_reader(nodes);
        let header = reader.headers().expect("header").clone();
        let mut rows: Vec<Vec<String>> = reader
            .records()
            .map(|row| {
                row.expect("row")
                    .iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect();
        rows[0][2] = "{}".to_owned();
        let mut writer = WriterBuilder::new()
            .terminator(Terminator::Any(b'\n'))
            .from_writer(Vec::new());
        writer.write_record(&header).expect("header");
        for row in rows {
            writer.write_record(row).expect("row");
        }
        let wrong_list_type = writer.into_inner().expect("CSV bytes");
        assert!(
            read_lpg_csv(
                &replace_artifact(&projected.package, GENERIC_NODES, &wrong_list_type),
                &config
            )
            .is_err()
        );

        let target = &projected.graph.edges[0].target;
        let edges = String::from_utf8(
            projected
                .package
                .get(GENERIC_EDGES)
                .expect("edges")
                .to_vec(),
        )
        .expect("UTF-8");
        let dangling = edges.replacen(target, "node_missing", 1).into_bytes();
        assert!(
            read_lpg_csv(
                &replace_artifact(&projected.package, GENERIC_EDGES, &dangling),
                &config
            )
            .is_err()
        );
        assert!(read_lpg_csv(&projected.package, &test_config(1)).is_err());
    }

    #[test]
    fn neo4j_reader_rejects_token_group_and_extra_artifact_corruption() {
        let config = test_config(1_000);
        let projected = project_neo4j_csv(fixture().as_ref(), &config).expect("project");
        let label_map = String::from_utf8(
            projected
                .package
                .get(NEO4J_LABEL_MAP)
                .expect("label map")
                .to_vec(),
        )
        .expect("UTF-8");
        let first_token = label_map
            .lines()
            .nth(1)
            .and_then(|line| line.split(',').next())
            .expect("mapped label");
        let corrupt_map = label_map
            .replacen(first_token, "RdfLabel_corrupt", 1)
            .into_bytes();
        assert!(
            read_neo4j_csv(
                &replace_artifact(&projected.package, NEO4J_LABEL_MAP, &corrupt_map),
                &config
            )
            .is_err()
        );

        let first_node_path = projected
            .package
            .artifacts()
            .find_map(|(path, _)| is_direct_csv_child(path, NEO4J_NODE_PREFIX).then_some(path))
            .expect("node group");
        let wrong_group = ProjectionPackage::from_artifacts(
            projected.package.limits(),
            projected.package.artifacts().map(|(path, bytes)| {
                (
                    if path == first_node_path {
                        "neo4j/nodes/group_wrong.csv".to_owned()
                    } else {
                        path.to_owned()
                    },
                    bytes.to_vec(),
                )
            }),
        )
        .expect("wrong group package");
        assert!(read_neo4j_csv(&wrong_group, &config).is_err());

        let mut artifacts: Vec<(String, Vec<u8>)> = projected
            .package
            .artifacts()
            .map(|(path, bytes)| (path.to_owned(), bytes.to_vec()))
            .collect();
        artifacts.push(("neo4j/unexpected.csv".to_owned(), b"x\n".to_vec()));
        let extra = ProjectionPackage::from_artifacts(projected.package.limits(), artifacts)
            .expect("extra artifact package");
        assert!(read_neo4j_csv(&extra, &config).is_err());
        assert!(read_neo4j_csv(&projected.package, &test_config(1)).is_err());
    }

    #[test]
    fn csv_writer_enforces_artifact_limit_while_streaming() {
        let builder = RdfDatasetBuilder::new();
        let dataset = builder.freeze().expect("empty dataset");
        let limits = ProjectionLimits::new(8, 16, 128, 2_048, 16).expect("small limits");
        let config = LpgConfig::new(TYPE, limits, 10).expect("small config");
        let graph = project_lpg(dataset.as_ref(), &config)
            .expect("empty projection")
            .graph;
        let error = write_lpg_csv(&graph, &config).expect_err("CSV header exceeds limit");
        assert_eq!(
            error.kind(),
            super::super::super::ProjectionErrorKind::ResourceLimit
        );
        assert_eq!(error.path(), Some(GENERIC_NODES));
    }
}
