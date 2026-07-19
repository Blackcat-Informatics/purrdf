// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical labeled-property-graph model and exact RDF 1.2 sideband mapping.

mod carrier_util;
mod csv;
mod cypher;
mod graphml;
mod mapping;
mod model;
mod stream;

pub use csv::{
    LpgPackageProjection, project_lpg_csv, project_lpg_csv_to_sink, project_neo4j_csv,
    project_neo4j_csv_to_sink, read_lpg_csv, read_neo4j_csv, write_lpg_csv, write_lpg_csv_to_sink,
    write_neo4j_csv, write_neo4j_csv_to_sink,
};
pub use cypher::{
    project_lpg_cypher, project_lpg_cypher_to_sink, read_lpg_cypher, write_lpg_cypher,
    write_lpg_cypher_to_sink,
};
pub use graphml::{
    project_lpg_graphml, project_lpg_graphml_to_sink, read_lpg_graphml, write_lpg_graphml,
    write_lpg_graphml_to_sink,
};
pub use mapping::{
    LpgLiftOutcome, LpgProjection, lift_lpg, project_lpg, project_lpg_with_progress,
};
pub use model::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgExecutionLimits, LpgGraph, LpgGraphContext,
    LpgIriSelection, LpgLabel, LpgNamedGraphSelection, LpgNode, LpgProperty, LpgPropertyAtom,
    LpgRdfQuad, LpgReifier, LpgScope,
};
pub use stream::{
    LpgProgress, LpgProgressObserver, LpgProgressPhase, LpgProjectionReport, LpgStreamProjection,
};
