// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, full-IRI RDF→OBO Graphs 0.3.2 projection.

mod config;
mod mapping;
mod model;

pub use config::{
    OboGraphsConfig, OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles,
};
pub use mapping::{OboGraphsProjection, project_obo_graphs};
pub use model::{
    OboDomainRangeAxiom, OboEdge, OboEquivalentNodesSet, OboExistentialRestriction, OboGraph,
    OboGraphDocument, OboLogicalDefinitionAxiom, OboMeta, OboNode, OboNodeType,
    OboPropertyChainAxiom, OboPropertyType, OboPropertyValue, OboSynonym, OboXref,
};
