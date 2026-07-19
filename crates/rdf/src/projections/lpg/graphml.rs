// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic GraphML 1.0 carrier for canonical LPG.

use std::collections::{BTreeMap, BTreeSet};
use std::io;

use purrdf_core::DatasetView;
use roxmltree::{Document, Node};

use super::super::{
    ProjectionArtifactSink, ProjectionError, ProjectionPackage, ProjectionPackageSink,
    escape_xml_attribute, escape_xml_text,
};
use super::carrier_util::{
    BoundedText, LpgTextWriter, hex_decode, json_string, read_manifest, require_canonical_package,
    required_artifact, validate_package_bounds, write_manifest,
};
use super::csv::{LpgPackageProjection, native_labels, native_property_cell, property_token};
use super::mapping::{LpgProjection, project_lpg, project_lpg_with_progress};
use super::model::{LpgConfig, LpgGraph};
use super::stream::{
    IgnoreProgress, LpgProgressObserver, LpgProjectionReport, LpgSinkSession, LpgStreamProjection,
    graph_report,
};

const PROFILE: &str = "purrdf-lpg-graphml-1.0";
const MANIFEST_PATH: &str = "graphml/manifest.json";
const GRAPHML_PATH: &str = "graphml/graph.graphml";
const GRAPHML_NS: &str = "http://graphml.graphdrawing.org/xmlns";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const SCHEMA_LOCATION: &str =
    "http://graphml.graphdrawing.org/xmlns http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd";

const GRAPH_JSON: &str = "g_lpg_json_hex";
const NODE_IDENTITY: &str = "n_identity_json_hex";
const NODE_LABELS: &str = "n_labels_json_hex";
const NODE_PROPERTIES: &str = "n_properties_json_hex";
const NODE_NATIVE_LABELS: &str = "n_native_labels_json_hex";
const EDGE_TYPE: &str = "e_type_iri";
const EDGE_RDF: &str = "e_rdf_json_hex";

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyDecl {
    id: String,
    target: String,
    name: String,
}

/// Encode a canonical LPG as deterministic GraphML 1.0 XML.
///
/// The document declares the official GraphML namespace and schema location,
/// deterministic node/edge keys, directed edges, native label/property views, and a
/// lowercase-hex canonical LPG payload for exact RDF 1.2 reversal. Hex payloads keep
/// every RDF string representable even when XML 1.0 forbids the original code point.
///
/// # Errors
///
/// Returns a typed model, XML-character, serialization, package, or resource-limit
/// failure.
pub fn write_lpg_graphml(
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<ProjectionPackage, ProjectionError> {
    let mut sink = ProjectionPackageSink::new(config.limits());
    write_lpg_graphml_to_sink(graph, config, &mut sink, &mut IgnoreProgress)?;
    sink.into_package()
}

/// Encode canonical GraphML artifacts incrementally into a transactional sink.
///
/// # Errors
///
/// Returns a typed model, sink, observer, XML, serialization, or resource-limit
/// failure. The sink transaction is aborted on every failure.
pub fn write_lpg_graphml_to_sink<S, O>(
    graph: &LpgGraph,
    config: &LpgConfig,
    sink: &mut S,
    observer: &mut O,
) -> Result<(), ProjectionError>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    stream_lpg_graphml(graph, config, sink, observer, graph_report(graph))
}

/// Project any RDF dataset view directly into deterministic GraphML 1.0.
///
/// # Errors
///
/// Returns any canonical LPG projection or GraphML/package encoding failure.
pub fn project_lpg_graphml<D: DatasetView>(
    view: &D,
    config: &LpgConfig,
) -> Result<LpgPackageProjection, ProjectionError> {
    let LpgProjection {
        graph,
        loss_ledger,
        report,
    } = project_lpg(view, config)?;
    let package = write_lpg_graphml(&graph, config)?;
    Ok(LpgPackageProjection {
        graph,
        package,
        loss_ledger,
        report,
    })
}

/// Project RDF directly into incrementally emitted GraphML artifacts.
///
/// # Errors
///
/// Returns a typed mapping, sink, observer, XML, serialization, or resource-limit
/// failure. The sink transaction is aborted on every failure.
pub fn project_lpg_graphml_to_sink<D, S, O>(
    view: &D,
    config: &LpgConfig,
    sink: &mut S,
    observer: &mut O,
) -> Result<LpgStreamProjection, ProjectionError>
where
    D: DatasetView,
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    let LpgProjection {
        graph,
        loss_ledger,
        report,
    } = project_lpg_with_progress(view, config, observer)?;
    stream_lpg_graphml(&graph, config, sink, observer, report)?;
    Ok(LpgStreamProjection {
        loss_ledger,
        report,
    })
}

fn stream_lpg_graphml<S, O>(
    graph: &LpgGraph,
    config: &LpgConfig,
    sink: &mut S,
    observer: &mut O,
    report: LpgProjectionReport,
) -> Result<(), ProjectionError>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    graph.validate(config)?;
    let mut session = LpgSinkSession::new(sink, observer, config.limits(), report)?;
    session.write_artifact(GRAPHML_PATH, |output| {
        render_graphml_into(output, graph, config)
    })?;
    let manifest = write_manifest(PROFILE, graph, config)?;
    session.write_artifact(MANIFEST_PATH, |output| output.write_bytes(&manifest))?;
    session.commit()
}

/// Decode the strict GraphML 1.0 profile emitted by PurRDF.
///
/// # Errors
///
/// Rejects DTD/entity input, malformed XML, namespace/schema/key drift, duplicate or
/// dangling ids, unknown graph children, non-canonical payloads, extra artifacts,
/// semantic LPG inconsistency, and all configured resource-limit breaches.
pub fn read_lpg_graphml(
    package: &ProjectionPackage,
    config: &LpgConfig,
) -> Result<LpgGraph, ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    let schema_version = read_manifest(
        required_artifact(package, MANIFEST_PATH)?,
        PROFILE,
        config,
        MANIFEST_PATH,
    )?;
    let bytes = required_artifact(package, GRAPHML_PATH)?;
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProjectionError::syntax(format!("GraphML is not UTF-8: {error}")).at_path(GRAPHML_PATH)
    })?;
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err(
            ProjectionError::syntax("GraphML profile forbids DTD and entity declarations")
                .at_path(GRAPHML_PATH),
        );
    }
    let document = Document::parse(text).map_err(|error| {
        ProjectionError::syntax(format!("parse GraphML XML: {error}")).at_path(GRAPHML_PATH)
    })?;
    let root = document.root_element();
    require_element(root, "graphml")?;
    if namespaced_attribute(root, XSI_NS, "schemaLocation") != Some(SCHEMA_LOCATION) {
        return Err(ProjectionError::integrity(
            "GraphML root has the wrong or missing xsi:schemaLocation",
        )
        .at_path(GRAPHML_PATH));
    }

    let mut actual_keys = Vec::new();
    let mut key_ids = BTreeSet::new();
    let mut graph_element = None;
    for child in root.children().filter(Node::is_element) {
        require_graphml_namespace(child)?;
        match child.tag_name().name() {
            "key" => {
                let declaration = KeyDecl {
                    id: required_attribute(child, "id")?.to_owned(),
                    target: required_attribute(child, "for")?.to_owned(),
                    name: required_attribute(child, "attr.name")?.to_owned(),
                };
                if required_attribute(child, "attr.type")? != "string"
                    || !key_ids.insert(declaration.id.clone())
                {
                    return Err(ProjectionError::integrity(
                        "GraphML keys must be unique and use attr.type=string",
                    )
                    .at_path(GRAPHML_PATH));
                }
                actual_keys.push(declaration);
            }
            "graph" if graph_element.replace(child).is_none() => {}
            "graph" => {
                return Err(ProjectionError::integrity(
                    "GraphML profile requires exactly one graph element",
                )
                .at_path(GRAPHML_PATH));
            }
            other => {
                return Err(ProjectionError::syntax(format!(
                    "unknown GraphML root child {other:?}"
                ))
                .at_path(GRAPHML_PATH));
            }
        }
    }
    let graph_element = graph_element.ok_or_else(|| {
        ProjectionError::integrity("GraphML profile is missing its graph element")
            .at_path(GRAPHML_PATH)
    })?;
    if required_attribute(graph_element, "id")? != "G"
        || required_attribute(graph_element, "edgedefault")? != "directed"
    {
        return Err(ProjectionError::integrity(
            "GraphML graph must have id=G and edgedefault=directed",
        )
        .at_path(GRAPHML_PATH));
    }

    let mut graph_payload = None;
    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut endpoints = Vec::new();
    for child in graph_element.children().filter(Node::is_element) {
        require_graphml_namespace(child)?;
        match child.tag_name().name() {
            "data" if required_attribute(child, "key")? == GRAPH_JSON => {
                if graph_payload.replace(element_text(child)?).is_some() {
                    return Err(ProjectionError::integrity(
                        "GraphML graph contains duplicate canonical LPG payloads",
                    )
                    .at_path(GRAPHML_PATH));
                }
            }
            "data" => {
                return Err(ProjectionError::syntax(
                    "GraphML graph contains an unknown graph-level data key",
                )
                .at_path(GRAPHML_PATH));
            }
            "node" => {
                let id = required_attribute(child, "id")?.to_owned();
                if !nodes.insert(id) {
                    return Err(ProjectionError::integrity("duplicate GraphML node id")
                        .at_path(GRAPHML_PATH));
                }
            }
            "edge" => {
                let id = required_attribute(child, "id")?.to_owned();
                if !edges.insert(id) {
                    return Err(ProjectionError::integrity("duplicate GraphML edge id")
                        .at_path(GRAPHML_PATH));
                }
                endpoints.push((
                    required_attribute(child, "source")?.to_owned(),
                    required_attribute(child, "target")?.to_owned(),
                ));
            }
            other => {
                return Err(ProjectionError::syntax(format!(
                    "unknown GraphML graph child {other:?}"
                ))
                .at_path(GRAPHML_PATH));
            }
        }
        if nodes
            .len()
            .checked_add(edges.len())
            .is_none_or(|count| count > config.max_records())
        {
            return Err(ProjectionError::limit(
                "GraphML node/edge rows exceed the configured LPG record limit",
            )
            .at_path(GRAPHML_PATH));
        }
    }
    if endpoints
        .iter()
        .any(|(source, target)| !nodes.contains(source) || !nodes.contains(target))
    {
        return Err(
            ProjectionError::integrity("GraphML edge has a dangling source or target")
                .at_path(GRAPHML_PATH),
        );
    }

    let payload = graph_payload.ok_or_else(|| {
        ProjectionError::integrity("GraphML graph is missing its canonical LPG payload")
            .at_path(GRAPHML_PATH)
    })?;
    let graph_json = hex_decode(payload, "GraphML canonical LPG", GRAPHML_PATH)?;
    let graph = LpgGraph::from_canonical_json(&graph_json, config)?;
    if graph.schema_version != schema_version {
        return Err(ProjectionError::integrity(
            "GraphML manifest and canonical LPG schema versions disagree",
        ));
    }
    if actual_keys != graphml_keys(&graph)? {
        return Err(ProjectionError::integrity(
            "GraphML key declarations do not exactly match the canonical LPG model",
        )
        .at_path(GRAPHML_PATH));
    }
    let expected = render_graphml(&graph, config)?;
    if expected != bytes {
        return Err(ProjectionError::syntax(
            "GraphML is valid but outside the canonical PurRDF carrier grammar",
        )
        .at_path(GRAPHML_PATH));
    }
    let canonical = write_lpg_graphml(&graph, config)?;
    require_canonical_package(package, &canonical, PROFILE)?;
    Ok(graph)
}

fn render_graphml(graph: &LpgGraph, config: &LpgConfig) -> Result<Vec<u8>, ProjectionError> {
    let mut output = BoundedText::new(config.limits(), "GraphML XML", GRAPHML_PATH);
    render_graphml_into(&mut output, graph, config)?;
    Ok(output.finish())
}

fn render_graphml_into<W: LpgTextWriter + ?Sized>(
    output: &mut W,
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<(), ProjectionError> {
    output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
    output.push(&format!(
        "<graphml xmlns=\"{GRAPHML_NS}\" xmlns:xsi=\"{XSI_NS}\" xsi:schemaLocation=\"{SCHEMA_LOCATION}\">\n"
    ))?;
    for key in graphml_keys(graph)? {
        output.push(&format!(
            "  <key id=\"{}\" for=\"{}\" attr.name=\"{}\" attr.type=\"string\"/>\n",
            escape_xml_attribute(&key.id)?,
            escape_xml_attribute(&key.target)?,
            escape_xml_attribute(&key.name)?,
        ))?;
    }
    output.push("  <graph id=\"G\" edgedefault=\"directed\">\n")?;
    output.push(&format!("    <data key=\"{GRAPH_JSON}\">"))?;
    serde_json::to_writer(HexWriter(output), graph).map_err(|error| {
        ProjectionError::integrity(format!("serialize GraphML LPG JSON payload: {error}"))
    })?;
    output.push("</data>\n")?;
    for node in &graph.nodes {
        output.push(&format!(
            "    <node id=\"{}\">\n",
            escape_xml_attribute(&node.id)?
        ))?;
        push_hex_data(
            output,
            NODE_IDENTITY,
            json_string(&node.identity, config, "GraphML node identity")?.as_bytes(),
        )?;
        push_hex_data(
            output,
            NODE_LABELS,
            json_string(&node.labels, config, "GraphML node labels")?.as_bytes(),
        )?;
        push_hex_data(
            output,
            NODE_PROPERTIES,
            json_string(&node.properties, config, "GraphML node properties")?.as_bytes(),
        )?;
        push_hex_data(
            output,
            NODE_NATIVE_LABELS,
            json_string(
                &native_labels(&node.labels)?,
                config,
                "GraphML native labels",
            )?
            .as_bytes(),
        )?;
        let mut properties = BTreeMap::new();
        for property in &node.properties {
            let token = property_token(&property.key)?;
            if let Some(existing) = properties.insert(token, property.key.as_str())
                && existing != property.key.as_str()
            {
                return Err(ProjectionError::integrity(
                    "SHA-256 collision between distinct GraphML property IRIs",
                ));
            }
        }
        for (token, iri) in properties {
            let value = native_property_cell(&node.properties, iri, config)?;
            push_hex_data(output, &token, value.as_bytes())?;
        }
        output.push("    </node>\n")?;
    }
    for edge in &graph.edges {
        output.push(&format!(
            "    <edge id=\"{}\" source=\"{}\" target=\"{}\">\n",
            escape_xml_attribute(&edge.id)?,
            escape_xml_attribute(&edge.source)?,
            escape_xml_attribute(&edge.target)?,
        ))?;
        output.push(&format!(
            "      <data key=\"{EDGE_TYPE}\">{}</data>\n",
            escape_xml_text(&edge.edge_type)?
        ))?;
        push_hex_data(
            output,
            EDGE_RDF,
            json_string(&edge.rdf, config, "GraphML edge RDF sideband")?.as_bytes(),
        )?;
        output.push("    </edge>\n")?;
    }
    output.push("  </graph>\n</graphml>\n")?;
    Ok(())
}

fn graphml_keys(graph: &LpgGraph) -> Result<Vec<KeyDecl>, ProjectionError> {
    let mut keys = vec![
        key(GRAPH_JSON, "graph", "purrdf_lpg_json_hex"),
        key(NODE_IDENTITY, "node", "purrdf_identity_json_hex"),
        key(NODE_LABELS, "node", "purrdf_labels_json_hex"),
        key(NODE_PROPERTIES, "node", "purrdf_properties_json_hex"),
        key(NODE_NATIVE_LABELS, "node", "purrdf_native_labels_json_hex"),
        key(EDGE_TYPE, "edge", "purrdf_edge_type_iri"),
        key(EDGE_RDF, "edge", "purrdf_edge_rdf_json_hex"),
    ];
    let mut properties = BTreeMap::new();
    for property in graph.nodes.iter().flat_map(|node| &node.properties) {
        let token = property_token(&property.key)?;
        if let Some(existing) = properties.insert(token, property.key.as_str())
            && existing != property.key.as_str()
        {
            return Err(ProjectionError::integrity(
                "SHA-256 collision between distinct GraphML property IRIs",
            ));
        }
    }
    keys.extend(
        properties
            .into_iter()
            .map(|(id, iri)| key(&id, "node", iri)),
    );
    Ok(keys)
}

fn key(id: &str, target: &str, name: &str) -> KeyDecl {
    KeyDecl {
        id: id.to_owned(),
        target: target.to_owned(),
        name: name.to_owned(),
    }
}

fn push_hex_data<W: LpgTextWriter + ?Sized>(
    output: &mut W,
    key: &str,
    bytes: &[u8],
) -> Result<(), ProjectionError> {
    output.push(&format!(
        "      <data key=\"{}\">",
        escape_xml_attribute(key)?
    ))?;
    output.push_hex(bytes)?;
    output.push("</data>\n")
}

struct HexWriter<'a, W: ?Sized>(&'a mut W);

impl<W: LpgTextWriter + ?Sized> io::Write for HexWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .push_hex(buffer)
            .map(|()| buffer.len())
            .map_err(|error| io::Error::other(error.to_string()))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn require_element(node: Node<'_, '_>, local: &str) -> Result<(), ProjectionError> {
    require_graphml_namespace(node)?;
    if node.tag_name().name() != local {
        return Err(ProjectionError::syntax(format!(
            "expected GraphML {local:?} element, found {:?}",
            node.tag_name().name()
        ))
        .at_path(GRAPHML_PATH));
    }
    Ok(())
}

fn require_graphml_namespace(node: Node<'_, '_>) -> Result<(), ProjectionError> {
    if node.tag_name().namespace() != Some(GRAPHML_NS) {
        return Err(ProjectionError::integrity(
            "GraphML element is outside the GraphML 1.0 namespace",
        )
        .at_path(GRAPHML_PATH));
    }
    Ok(())
}

fn required_attribute<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str, ProjectionError> {
    node.attribute(name).ok_or_else(|| {
        ProjectionError::syntax(format!(
            "GraphML {:?} element is missing attribute {name:?}",
            node.tag_name().name()
        ))
        .at_path(GRAPHML_PATH)
    })
}

fn namespaced_attribute<'a>(node: Node<'a, '_>, namespace: &str, local: &str) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.namespace() == Some(namespace) && attribute.name() == local)
        .map(|attribute| attribute.value())
}

fn element_text<'a>(node: Node<'a, '_>) -> Result<&'a str, ProjectionError> {
    if node.children().any(|child| child.is_element()) {
        return Err(
            ProjectionError::syntax("GraphML data payload must contain text only")
                .at_path(GRAPHML_PATH),
        );
    }
    node.text().ok_or_else(|| {
        ProjectionError::syntax("GraphML data payload is empty").at_path(GRAPHML_PATH)
    })
}

#[cfg(test)]
mod tests {
    use super::super::model::{LpgExecutionLimits, LpgScope};
    use std::sync::Arc;

    use purrdf_core::{
        BlankScope, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
        datasets_isomorphic,
    };

    use super::*;
    use crate::{ProjectionLimits, lift_lpg};

    const TYPE: &str = "http://example.org/type";

    fn test_config(max_records: usize) -> LpgConfig {
        LpgConfig::new(
            TYPE,
            LpgScope::all(),
            ProjectionLimits::new(32, 3_000_000, 6_000_000, 8_000_000, 16).expect("limits"),
            LpgExecutionLimits::new(max_records, max_records, max_records, max_records)
                .expect("execution limits"),
        )
        .expect("config")
    }

    fn fixture() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let target = builder.intern_blank("target", BlankScope(4));
        let graph = builder.intern_iri("http://example.org/graph");
        let rdf_type = builder.intern_iri(TYPE);
        let class = builder.intern_iri("http://example.org/Class");
        builder.push_quad(subject, rdf_type, class, Some(graph));
        let name = builder.intern_iri("http://example.org/name?x=1&y=2");
        let hostile = builder.intern_literal(RdfLiteral::simple("<&> \u{ffff} quoted \" text"));
        builder.push_quad(subject, name, hostile, Some(graph));
        let relates = builder.intern_iri("http://example.org/relates");
        builder.push_quad(subject, relates, target, None);
        let quoted = builder.intern_triple(subject, relates, target);
        let reifier = builder.intern_blank("reifier", BlankScope(6));
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
    fn graphml_is_schema_declared_backend_independent_and_exactly_reversible() {
        let dataset = fixture();
        let config = test_config(1_000);
        let projected = project_lpg_graphml(dataset.as_ref(), &config).expect("project");
        let xml = std::str::from_utf8(projected.package.get(GRAPHML_PATH).expect("GraphML"))
            .expect("UTF-8");
        assert!(xml.contains(SCHEMA_LOCATION));
        assert!(xml.contains("edgedefault=\"directed\""));
        assert!(xml.contains("RdfProp_"));
        assert!(!xml.contains('\u{ffff}'));
        let document = Document::parse(xml).expect("XML oracle");
        assert_eq!(
            document.root_element().tag_name().namespace(),
            Some(GRAPHML_NS)
        );

        let decoded = read_lpg_graphml(&projected.package, &config).expect("read");
        assert_eq!(decoded, projected.graph);
        assert!(same_artifacts(
            &projected.package,
            &write_lpg_graphml(&decoded, &config).expect("rewrite")
        ));
        let lifted = lift_lpg(&decoded, &config).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));

        let pack = PackBuilder::build_bytes(&dataset).expect("pack");
        let view = PackView::from_bytes(&pack).expect("view");
        let packed = project_lpg_graphml(&view, &config).expect("pack projection");
        assert!(same_artifacts(&projected.package, &packed.package));
        assert_eq!(
            projected.loss_ledger.render_json(),
            packed.loss_ledger.render_json()
        );
    }

    #[test]
    fn graphml_reader_rejects_dtd_duplicate_keys_dangling_edges_and_limits() {
        let config = test_config(1_000);
        let projected = project_lpg_graphml(fixture().as_ref(), &config).expect("project");
        let xml = std::str::from_utf8(projected.package.get(GRAPHML_PATH).expect("GraphML"))
            .expect("UTF-8");

        let with_dtd = xml
            .replacen("?>\n", "?>\n<!DOCTYPE graphml [<!ENTITY x \"boom\">]>\n", 1)
            .into_bytes();
        assert!(
            read_lpg_graphml(
                &replace_artifact(&projected.package, GRAPHML_PATH, &with_dtd),
                &config
            )
            .is_err()
        );

        let first_key = xml
            .lines()
            .find(|line| line.trim_start().starts_with("<key "))
            .expect("key");
        let duplicate_key = xml
            .replacen(first_key, &format!("{first_key}\n{first_key}"), 1)
            .into_bytes();
        assert!(
            read_lpg_graphml(
                &replace_artifact(&projected.package, GRAPHML_PATH, &duplicate_key),
                &config
            )
            .is_err()
        );

        let unknown_element = xml
            .replacen("  </graph>", "    <unknown/>\n  </graph>", 1)
            .into_bytes();
        assert!(
            read_lpg_graphml(
                &replace_artifact(&projected.package, GRAPHML_PATH, &unknown_element),
                &config
            )
            .is_err()
        );

        let malformed = xml.replacen("</graphml>", "", 1).into_bytes();
        assert!(
            read_lpg_graphml(
                &replace_artifact(&projected.package, GRAPHML_PATH, &malformed),
                &config
            )
            .is_err()
        );

        let target = &projected.graph.edges[0].target;
        let dangling = xml
            .replacen(
                &format!("target=\"{target}\""),
                "target=\"node_missing\"",
                1,
            )
            .into_bytes();
        assert!(
            read_lpg_graphml(
                &replace_artifact(&projected.package, GRAPHML_PATH, &dangling),
                &config
            )
            .is_err()
        );
        assert!(read_lpg_graphml(&projected.package, &test_config(1)).is_err());
    }
}
