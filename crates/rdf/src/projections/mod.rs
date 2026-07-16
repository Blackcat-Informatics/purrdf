// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2 graph and tabular projections.
//!
//! Projection codecs share one bounded in-memory package, one durable RDF term
//! representation, one typed error surface, and one set of escaping/identity
//! primitives. Filesystem and network access stay outside this module, so the same
//! engine runs unchanged in native, WebAssembly, Python, and C hosts.
//!
//! # Profiles and fidelity
//!
//! | Profile | Direction | Fidelity contract |
//! | --- | --- | --- |
//! | Generic LPG CSV | RDF ↔ carrier | Canonical LPG view plus exact RDF sideband; semantic lowering is located in the loss ledger |
//! | Neo4j Admin Import CSV | RDF ↔ carrier | Same canonical LPG authority and ledger as generic CSV |
//! | openCypher | RDF ↔ carrier | Strict reader accepts the complete grammar emitted by PurRDF |
//! | GraphML 1.0 | RDF ↔ carrier | Strict namespaced XML reader; exact RDF sideband remains authoritative |
//! | CSVW exact | RDF ↔ carrier | Lossless RDF 1.2 term, quad, reifier, and annotation tables |
//! | OBO Graphs 0.3.2 | RDF → view | Deliberately write-only and loss-ledgered |
//! | SKOS Turtle | RDF → view | Deliberately write-only and loss-ledgered |
//!
//! [`project_archive`] provides the profile-tagged production entry point.
//! [`lift_archive`] accepts only [`LiftProfile`], so the type system cannot pretend
//! that the two lossy views round-trip. Every operation computes a deterministic
//! [`purrdf_core::LossLedger`]; deciding whether to display it is a host concern.
//!
//! # Configuration and packages
//!
//! [`ProjectionConfig`] has no default. The caller supplies every semantic role,
//! identity IRI, and [`ProjectionLimits`]; unknown JSON fields and a mismatch
//! between the requested profile and tagged configuration are hard errors. Package
//! members are lexically ordered inside canonical USTAR bytes, with fixed headers,
//! checksums, padding, and trailer. Readers enforce configured size/count/depth
//! bounds and reject any archive whose canonical re-encoding differs.
//!
//! See `examples/projection_archive.rs` in the repository for a runnable Rust
//! project/write/lift example. Matching examples are provided for the CLI, Python,
//! WebAssembly, and C surfaces.

mod carrier;
mod csvw;
mod error;
mod lpg;
mod obo_graphs;
mod package;
mod research_object;
mod skos;
mod term;
mod util;

pub use carrier::{
    LiftProfile, ProjectionArchive, ProjectionConfig, ProjectionLift, ProjectionProfile,
    lift_archive, project_archive,
};
pub use csvw::{
    CsvwAction, CsvwAnnotations, CsvwCell, CsvwColumn, CsvwConfig, CsvwContext, CsvwDatatype,
    CsvwDatatypeFormat, CsvwDialect, CsvwExactProjection, CsvwExactReadOutcome, CsvwForeignKey,
    CsvwInheritedProperties, CsvwInput, CsvwMappedTableGroup, CsvwMode, CsvwNaturalLanguage,
    CsvwNumericFormat, CsvwRdfTableMapping, CsvwReadOutcome, CsvwReference, CsvwRow, CsvwSchema,
    CsvwTable, CsvwTableDirection, CsvwTableGroup, CsvwTextDirection, CsvwTransformation, CsvwTrim,
    CsvwValue, CsvwVocabulary, CsvwWarning, CsvwWarningKind, CsvwWriteOutcome, CsvwWritePlan,
    project_csvw, project_csvw_exact, read_csvw, read_csvw_exact, write_csvw,
};
pub use error::{ProjectionError, ProjectionErrorKind};
pub use lpg::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgGraphContext, LpgLabel, LpgLiftOutcome,
    LpgNode, LpgPackageProjection, LpgProjection, LpgProperty, LpgPropertyAtom, LpgRdfQuad,
    LpgReifier, lift_lpg, project_lpg, project_lpg_csv, project_lpg_cypher, project_lpg_graphml,
    project_neo4j_csv, read_lpg_csv, read_lpg_cypher, read_lpg_graphml, read_neo4j_csv,
    write_lpg_csv, write_lpg_cypher, write_lpg_graphml, write_neo4j_csv,
};
pub use obo_graphs::{
    OboDomainRangeAxiom, OboEdge, OboEquivalentNodesSet, OboExistentialRestriction, OboGraph,
    OboGraphDocument, OboGraphsConfig, OboGraphsProjection, OboGraphsVocabulary,
    OboLogicalDefinitionAxiom, OboMeta, OboMetadataRoles, OboNode, OboNodeType, OboOwlRoles,
    OboPropertyChainAxiom, OboPropertyType, OboPropertyValue, OboRdfRoles, OboSynonym, OboXref,
    project_obo_graphs,
};
pub use package::{ProjectionLimits, ProjectionPackage};
pub use research_object::{
    CROISSANT_ARTIFACT, CROISSANT_PROFILE, CROISSANT_ROLES, CroissantConfig, CroissantRole,
    CroissantVocabulary, OfflineJsonLdContext, RESEARCH_ROLES, ResearchActivity, ResearchAgent,
    ResearchChecksum, ResearchDataset, ResearchField, ResearchObjectConfig, ResearchObjectIdentity,
    ResearchObjectModel, ResearchObjectPackageProjection, ResearchObjectPolicy,
    ResearchObjectProjection, ResearchObjectReadOutcome, ResearchObjectRoles, ResearchRecordSet,
    ResearchResource, ResearchRole, ResearchText, ResearchValue, lift_research_object,
    project_croissant, project_research_object, read_croissant,
};
pub use skos::{
    SkosClassRoles, SkosConfig, SkosDocumentationRoles, SkosGraphSelection, SkosLabelRoles,
    SkosProjection, SkosRelationRoles, SkosSourceRoles, SkosTargetRoles, project_skos,
};
pub use term::{ProjectionDirection, ProjectionTerm};
pub use util::{
    escape_cypher_identifier, escape_cypher_string, escape_xml_attribute, escape_xml_text,
    stable_identifier, validate_absolute_iri,
};
