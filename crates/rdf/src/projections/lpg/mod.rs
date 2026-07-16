// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical labeled-property-graph model and exact RDF 1.2 sideband mapping.

mod carrier_util;
mod csv;
mod cypher;
mod graphml;
mod mapping;
mod model;

pub use csv::{
    LpgPackageProjection, project_lpg_csv, project_neo4j_csv, read_lpg_csv, read_neo4j_csv,
    write_lpg_csv, write_neo4j_csv,
};
pub use cypher::{project_lpg_cypher, read_lpg_cypher, write_lpg_cypher};
pub use graphml::{project_lpg_graphml, read_lpg_graphml, write_lpg_graphml};
pub use mapping::{LpgLiftOutcome, LpgProjection, lift_lpg, project_lpg};
pub use model::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgGraphContext, LpgLabel, LpgNode, LpgProperty,
    LpgPropertyAtom, LpgRdfQuad, LpgReifier,
};
