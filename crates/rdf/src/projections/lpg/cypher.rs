// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Closed deterministic openCypher carrier grammar for canonical LPG.

use std::collections::BTreeMap;

use purrdf_core::DatasetView;

use super::super::{
    ProjectionError, ProjectionPackage, escape_cypher_identifier, escape_cypher_string,
};
use super::carrier_util::{
    BoundedText, require_canonical_package, required_artifact, validate_package_bounds,
    write_manifest,
};
use super::csv::{LpgPackageProjection, native_labels, property_token, relationship_token};
use super::mapping::{LpgProjection, project_lpg};
use super::model::{LpgConfig, LpgGraph, LpgNode, LpgProperty, LpgPropertyAtom};

const PROFILE: &str = "purrdf-lpg-open-cypher";
const MANIFEST_PATH: &str = "open-cypher/manifest.json";
const CYPHER_PATH: &str = "open-cypher/graph.cypher";
const LPG_PATH: &str = "open-cypher/lpg.json";
const HEADER: &str = "// PurRDF deterministic openCypher profile 1\n// Exact RDF 1.2 reversal data: open-cypher/lpg.json\n";

/// Encode a canonical LPG as a deterministic, injection-safe openCypher package.
///
/// The script contains one closed `CREATE` clause whose stable node variables are
/// bound once and reused by every relationship, avoiding match ambiguity in the
/// target database. Labels, relationship types, and property keys use
/// collision-resistant tokens; exact RDF 1.2 reversal data remains in the package's
/// canonical `lpg.json` artifact.
///
/// # Errors
///
/// Returns a typed model, identifier, serialization, package, or resource-limit
/// failure.
pub fn write_lpg_cypher(
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<ProjectionPackage, ProjectionError> {
    graph.validate(config)?;
    let mut package = ProjectionPackage::new(config.limits());
    package.insert(CYPHER_PATH, render_cypher(graph, config)?)?;
    package.insert(LPG_PATH, graph.to_canonical_json(config)?)?;
    package.insert(MANIFEST_PATH, write_manifest(PROFILE, graph, config)?)?;
    Ok(package)
}

/// Project any RDF dataset view directly into the deterministic openCypher package.
///
/// # Errors
///
/// Returns any canonical LPG projection or openCypher/package encoding failure.
pub fn project_lpg_cypher<D: DatasetView>(
    view: &D,
    config: &LpgConfig,
) -> Result<LpgPackageProjection, ProjectionError> {
    let LpgProjection { graph, loss_ledger } = project_lpg(view, config)?;
    let package = write_lpg_cypher(&graph, config)?;
    Ok(LpgPackageProjection {
        graph,
        package,
        loss_ledger,
    })
}

/// Read the complete closed openCypher grammar emitted by PurRDF.
///
/// This is deliberately not a general query executor. The canonical LPG sideband is
/// parsed and validated, then the script is recognized byte-for-byte against the
/// grammar generated from that model. Unknown statements, alternate clauses,
/// ambiguous matches, non-canonical escaping, or mismatched native fields hard-fail.
///
/// # Errors
///
/// Rejects malformed/non-canonical packages, scripts outside the emitted grammar,
/// inconsistent manifests, invalid LPG data, and resource-limit breaches.
pub fn read_lpg_cypher(
    package: &ProjectionPackage,
    config: &LpgConfig,
) -> Result<LpgGraph, ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    let schema_version = super::carrier_util::read_manifest(
        required_artifact(package, MANIFEST_PATH)?,
        PROFILE,
        config,
        MANIFEST_PATH,
    )?;
    let graph = LpgGraph::from_canonical_json(required_artifact(package, LPG_PATH)?, config)?;
    if graph.schema_version != schema_version {
        return Err(ProjectionError::integrity(
            "openCypher manifest and canonical LPG schema versions disagree",
        ));
    }
    let actual = required_artifact(package, CYPHER_PATH)?;
    std::str::from_utf8(actual).map_err(|error| {
        ProjectionError::syntax(format!("openCypher is not UTF-8: {error}")).at_path(CYPHER_PATH)
    })?;
    let expected = render_cypher(&graph, config)?;
    if actual != expected {
        return Err(ProjectionError::syntax(
            "openCypher script is outside the canonical PurRDF carrier grammar",
        )
        .at_path(CYPHER_PATH));
    }
    let canonical = write_lpg_cypher(&graph, config)?;
    require_canonical_package(package, &canonical, PROFILE)?;
    Ok(graph)
}

fn render_cypher(graph: &LpgGraph, config: &LpgConfig) -> Result<Vec<u8>, ProjectionError> {
    let mut output = BoundedText::new(config.limits(), "openCypher script", CYPHER_PATH);
    output.push(HEADER)?;
    let pattern_count = graph
        .nodes
        .len()
        .checked_add(graph.edges.len())
        .ok_or_else(|| ProjectionError::limit("openCypher pattern count overflow"))?;
    if pattern_count == 0 {
        return Ok(output.finish());
    }
    output.push("CREATE\n")?;
    let mut written = 0usize;
    for node in &graph.nodes {
        output.push("  ")?;
        push_node_pattern(&mut output, node, config)?;
        written += 1;
        output.push(if written == pattern_count {
            ";\n"
        } else {
            ",\n"
        })?;
    }
    for edge in &graph.edges {
        let edge_type = relationship_token(&edge.edge_type)?;
        let edge_json = super::carrier_util::json_string(edge, config, "openCypher edge")?;
        output.push("  ")?;
        output.push(&format!(
            "(`{}`)-[:`{}` {{`purrdf_edge_id`: '",
            escape_cypher_identifier(&edge.source),
            escape_cypher_identifier(&edge_type),
        ))?;
        output.push(&escape_cypher_string(&edge.id))?;
        output.push("', `purrdf_edge_json`: '")?;
        output.push(&escape_cypher_string(&edge_json))?;
        output.push(&format!(
            "'}}]->(`{}`)",
            escape_cypher_identifier(&edge.target)
        ))?;
        written += 1;
        output.push(if written == pattern_count {
            ";\n"
        } else {
            ",\n"
        })?;
    }
    Ok(output.finish())
}

fn push_node_pattern(
    output: &mut BoundedText,
    node: &LpgNode,
    config: &LpgConfig,
) -> Result<(), ProjectionError> {
    let labels = native_labels(&node.labels)?;
    output.push(&format!("(`{}`", escape_cypher_identifier(&node.id)))?;
    for label in labels {
        output.push(":`")?;
        output.push(&escape_cypher_identifier(&label))?;
        output.push("`")?;
    }

    let node_json = super::carrier_util::json_string(node, config, "openCypher node")?;
    output.push(" {`purrdf_id`: '")?;
    output.push(&escape_cypher_string(&node.id))?;
    output.push("', `purrdf_node_json`: '")?;
    output.push(&escape_cypher_string(&node_json))?;
    output.push("'")?;
    let mut native_properties = BTreeMap::new();
    for property in &node.properties {
        let token = property_token(&property.key)?;
        if let Some(existing) = native_properties.insert(token, property.key.as_str())
            && existing != property.key.as_str()
        {
            return Err(ProjectionError::integrity(
                "SHA-256 collision between distinct openCypher property IRIs",
            ));
        }
    }
    for (token, iri) in native_properties {
        output.push(&format!(", `{}`: ", escape_cypher_identifier(&token)))?;
        push_native_property(output, &node.properties, iri)?;
    }
    output.push("})")
}

fn push_native_property(
    output: &mut BoundedText,
    properties: &[LpgProperty],
    iri: &str,
) -> Result<(), ProjectionError> {
    let values: Vec<_> = properties
        .iter()
        .filter(|property| property.key == iri)
        .map(|property| &property.value)
        .collect();
    if values.len() != 1 {
        output.push("[")?;
    }
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(", ")?;
        }
        push_native_atom(output, value)?;
    }
    if values.len() != 1 {
        output.push("]")?;
    }
    Ok(())
}

fn push_native_atom(
    output: &mut BoundedText,
    atom: &LpgPropertyAtom,
) -> Result<(), ProjectionError> {
    match atom {
        LpgPropertyAtom::Boolean { value } => output.push(if *value { "true" } else { "false" }),
        LpgPropertyAtom::Integer { value } => output.push(&value.to_string()),
        LpgPropertyAtom::Float { bits } if f64::from_bits(*bits).is_finite() => {
            let mut value = f64::from_bits(*bits).to_string();
            if !value.contains(['.', 'e', 'E']) {
                value.push_str(".0");
            }
            output.push(&value)
        }
        LpgPropertyAtom::Float { bits } => {
            push_string_literal(output, &format!("purrdf-f64-{bits:016x}"))
        }
        LpgPropertyAtom::Decimal { lexical } | LpgPropertyAtom::String { value: lexical } => {
            push_string_literal(output, lexical)
        }
    }
}

fn push_string_literal(output: &mut BoundedText, value: &str) -> Result<(), ProjectionError> {
    output.push("'")?;
    output.push(&escape_cypher_string(value))?;
    output.push("'")
}

#[cfg(test)]
mod tests {
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
            ProjectionLimits::new(32, 2_000_000, 5_000_000, 7_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn fixture() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let target = builder.intern_blank("target", BlankScope(3));
        let graph = builder.intern_iri("http://example.org/graph");
        let rdf_type = builder.intern_iri(TYPE);
        let class = builder.intern_iri("http://example.org/Class");
        builder.push_quad(subject, rdf_type, class, Some(graph));
        let name = builder.intern_iri("http://example.org/name");
        let hostile = builder.intern_literal(RdfLiteral::simple(
            "x'); MATCH (evil) RETURN evil; // ` \\ newline\n",
        ));
        builder.push_quad(subject, name, hostile, Some(graph));
        for (predicate, lexical, datatype) in [
            (
                "http://example.org/enabled",
                "true",
                "http://www.w3.org/2001/XMLSchema#boolean",
            ),
            (
                "http://example.org/count",
                "42",
                "http://www.w3.org/2001/XMLSchema#integer",
            ),
            (
                "http://example.org/ratio",
                "1.5",
                "http://www.w3.org/2001/XMLSchema#double",
            ),
            (
                "http://example.org/precise",
                "1.2300",
                "http://www.w3.org/2001/XMLSchema#decimal",
            ),
        ] {
            let predicate = builder.intern_iri(predicate);
            let value = builder.intern_literal(RdfLiteral::typed(lexical, datatype));
            builder.push_quad(subject, predicate, value, Some(graph));
        }
        let relates = builder.intern_iri("http://example.org/relates");
        builder.push_quad(subject, relates, target, None);
        let quoted = builder.intern_triple(subject, relates, target);
        let reifier = builder.intern_blank("reifier", BlankScope(5));
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
    fn cypher_is_injection_safe_backend_independent_and_exactly_reversible() {
        let dataset = fixture();
        let config = test_config(1_000);
        let projected = project_lpg_cypher(dataset.as_ref(), &config).expect("project");
        let script = std::str::from_utf8(projected.package.get(CYPHER_PATH).expect("Cypher"))
            .expect("UTF-8");
        assert!(script.starts_with(HEADER));
        assert!(script.contains("\\'"));
        for (index, _) in script.match_indices("'); MATCH (evil)") {
            assert_eq!(script.as_bytes().get(index.wrapping_sub(1)), Some(&b'\\'));
        }
        for (iri, expected) in [
            ("http://example.org/enabled", "true"),
            ("http://example.org/count", "42"),
            ("http://example.org/ratio", "1.5"),
            ("http://example.org/precise", "'1.2300'"),
        ] {
            let token = property_token(iri).expect("property token");
            assert!(script.contains(&format!("`{token}`: {expected}")));
        }
        assert_eq!(script.lines().filter(|line| line.ends_with(';')).count(), 1);

        let decoded = read_lpg_cypher(&projected.package, &config).expect("read");
        assert_eq!(decoded, projected.graph);
        assert!(same_artifacts(
            &projected.package,
            &write_lpg_cypher(&decoded, &config).expect("rewrite")
        ));
        let lifted = lift_lpg(&decoded, &config).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));

        let pack = PackBuilder::build_bytes(&dataset).expect("pack");
        let view = PackView::from_bytes(&pack).expect("view");
        let packed = project_lpg_cypher(&view, &config).expect("pack projection");
        assert!(same_artifacts(&projected.package, &packed.package));
        assert_eq!(
            projected.loss_ledger.render_json(),
            packed.loss_ledger.render_json()
        );
    }

    #[test]
    fn cypher_reader_rejects_unknown_statements_sideband_drift_and_limits() {
        let config = test_config(1_000);
        let projected = project_lpg_cypher(fixture().as_ref(), &config).expect("project");
        let mut unknown = projected.package.get(CYPHER_PATH).expect("Cypher").to_vec();
        unknown.extend_from_slice(b"MATCH (n) RETURN n;\n");
        assert!(
            read_lpg_cypher(
                &replace_artifact(&projected.package, CYPHER_PATH, &unknown),
                &config
            )
            .is_err()
        );

        let mut sideband = projected.package.get(LPG_PATH).expect("LPG").to_vec();
        sideband.push(b'\n');
        assert!(
            read_lpg_cypher(
                &replace_artifact(&projected.package, LPG_PATH, &sideband),
                &config
            )
            .is_err()
        );
        assert!(read_lpg_cypher(&projected.package, &test_config(1)).is_err());
    }

    #[test]
    fn text_carrier_writers_enforce_artifact_limits_incrementally() {
        let builder = RdfDatasetBuilder::new();
        let dataset = builder.freeze().expect("empty dataset");
        let limits = ProjectionLimits::new(8, 32, 128, 2_048, 16).expect("small limits");
        let config = LpgConfig::new(TYPE, limits, 10).expect("small config");
        let graph = project_lpg(dataset.as_ref(), &config)
            .expect("empty projection")
            .graph;
        let cypher = write_lpg_cypher(&graph, &config).expect_err("Cypher exceeds limit");
        assert_eq!(cypher.kind(), crate::ProjectionErrorKind::ResourceLimit);
        assert_eq!(cypher.path(), Some(CYPHER_PATH));
        let graphml = crate::write_lpg_graphml(&graph, &config).expect_err("GraphML exceeds limit");
        assert_eq!(graphml.kind(), crate::ProjectionErrorKind::ResourceLimit);
        assert_eq!(graphml.path(), Some("graphml/graph.graphml"));
    }
}
