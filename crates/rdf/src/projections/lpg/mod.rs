// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical labeled-property-graph model and exact RDF 1.2 sideband mapping.

mod mapping;
mod model;

pub use mapping::{LpgLiftOutcome, LpgProjection, lift_lpg, project_lpg};
pub use model::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgGraphContext, LpgLabel, LpgNode, LpgProperty,
    LpgPropertyAtom, LpgRdfQuad, LpgReifier,
};
