// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original scene identities and endpoint evidence at structural refusal.

use std::fmt;

use purrdf_hash::error::{ErrorValue, StructuredError};

use super::{VizEndpoint, VizPort, VizPortKind};

/// A renderer-neutral scene refusal retaining the original comparison operands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizSceneError {
    /// Node or edge identities were repeated in their respective collection.
    DuplicateIds {
        /// Original number of nodes.
        nodes: usize,
        /// Distinct node identities.
        unique_nodes: usize,
        /// Original number of edges.
        edges: usize,
        /// Distinct edge identities.
        unique_edges: usize,
    },
    /// An edge has no semantic binding.
    UnboundEdge {
        /// Original edge identity.
        edge: String,
    },
    /// A node has no semantic binding.
    UnboundNode {
        /// Original node identity.
        node: String,
    },
    /// A node-port endpoint names a node absent from the scene.
    MissingNode {
        /// Original endpoint, including its requested port.
        endpoint: VizEndpoint,
    },
    /// A node-port endpoint names a port absent from the existing node.
    MissingPort {
        /// Original endpoint.
        endpoint: VizEndpoint,
        /// Existing native ports, in their original order and with their kinds.
        ports: Vec<VizPort>,
    },
    /// An edge-anchor endpoint names an edge absent from the scene.
    MissingEdge {
        /// Original endpoint, including its requested anchor.
        endpoint: VizEndpoint,
    },
    /// An edge-anchor endpoint differs from the existing edge's actual anchor.
    MissingAnchor {
        /// Original endpoint.
        endpoint: VizEndpoint,
        /// Actual anchor identity, or absence of an anchor.
        actual_anchor: Option<String>,
    },
}

fn endpoint_ids(endpoint: &VizEndpoint) -> (&str, &str) {
    match endpoint {
        VizEndpoint::NodePort { node, port } => (node, port),
        VizEndpoint::EdgeAnchor { edge, anchor } => (edge, anchor),
    }
}

impl fmt::Display for VizSceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateIds { .. } => {
                f.write_str("visualization scene contains duplicate node or edge ids")
            }
            Self::UnboundEdge { edge } => {
                write!(f, "visualization scene edge {edge} has no semantic binding")
            }
            Self::UnboundNode { node } => {
                write!(f, "visualization scene node {node} has no semantic binding")
            }
            Self::MissingNode { endpoint } => write!(
                f,
                "visualization endpoint references missing node {}",
                endpoint_ids(endpoint).0
            ),
            Self::MissingPort { endpoint, .. } => {
                let (node, port) = endpoint_ids(endpoint);
                write!(
                    f,
                    "visualization endpoint references missing port {node}.{port}"
                )
            }
            Self::MissingEdge { endpoint } => write!(
                f,
                "visualization endpoint references missing edge {}",
                endpoint_ids(endpoint).0
            ),
            Self::MissingAnchor { endpoint, .. } => {
                let (edge, anchor) = endpoint_ids(endpoint);
                write!(
                    f,
                    "visualization endpoint references missing anchor {edge}.{anchor}"
                )
            }
        }
    }
}

impl std::error::Error for VizSceneError {}

impl StructuredError for VizSceneError {
    purrdf_hash::error_visit! {
        DuplicateIds { nodes, unique_nodes, edges, unique_edges } => "rdf-viz-scene-duplicate-ids" {
            "nodes" => Unsigned(*nodes as u128), "unique_nodes" => Unsigned(*unique_nodes as u128),
            "edges" => Unsigned(*edges as u128), "unique_edges" => Unsigned(*unique_edges as u128),
        },
        UnboundEdge { edge } => "rdf-viz-scene-unbound-edge" { "edge" => Text(edge) },
        UnboundNode { node } => "rdf-viz-scene-unbound-node" { "node" => Text(node) },
        MissingNode { endpoint } => "rdf-viz-scene-missing-node" { "endpoint" => Record(endpoint) },
        MissingPort { endpoint, ports } => "rdf-viz-scene-missing-port" {
            "endpoint" => Record(endpoint),
            "ports" => List(ports.iter().map(|port| ErrorValue::Record(port))),
        },
        MissingEdge { endpoint } => "rdf-viz-scene-missing-edge" { "endpoint" => Record(endpoint) },
        MissingAnchor { endpoint, actual_anchor } => "rdf-viz-scene-missing-anchor" {
            "endpoint" => Record(endpoint),
            "actual_anchor" => Value(actual_anchor.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
    }
}

impl StructuredError for VizEndpoint {
    purrdf_hash::error_visit! {
        NodePort { node, port } => "rdf-viz-node-port" { "node" => Text(node), "port" => Text(port) },
        EdgeAnchor { edge, anchor } => "rdf-viz-edge-anchor" { "edge" => Text(edge), "anchor" => Text(anchor) },
    }
}

impl StructuredError for VizPort {
    purrdf_hash::error_visit! {
        @record self { id, kind } => "rdf-viz-port" {
            "id" => Text(&self.id),
            "kind" => Code(match self.kind {
                VizPortKind::In => "in", VizPortKind::Out => "out",
                VizPortKind::Subject => "subject", VizPortKind::Predicate => "predicate",
                VizPortKind::Object => "object", VizPortKind::Relation => "relation",
                VizPortKind::Value => "value",
            }),
        }
    }
}
