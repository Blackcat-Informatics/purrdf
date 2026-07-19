// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2 graph, tabular, and research-object projections.
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
//! | Croissant 1.1 | RDF ↔ carrier | Shared research-object model with located profile losses |
//! | RO-Crate 1.3 | RDF ↔ carrier | Shared research-object model with located profile losses |
//! | DataCite 4.6 | RDF ↔ carrier | Shared research-object model with located profile losses |
//! | DCAT 3 | RDF ↔ carrier | Shared research-object model with located profile losses |
//! | Frictionless Data Package v1 | RDF ↔ carrier | Shared research-object model with located profile losses |
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
//! See `examples/projection_archive.rs` and
//! `examples/research_object_roundtrip.rs` in the repository for runnable Rust
//! project/write/lift examples. Matching examples are provided for the CLI,
//! Python, WebAssembly, and C surfaces.

mod carrier;
mod csvw;
mod error;
mod lpg;
mod obo_graphs;
mod package;
mod research_object;
mod sink;
mod skos;
mod term;
mod util;

pub use carrier::{
    LiftProfile, ProjectionArchive, ProjectionConfig, ProjectionLift, ProjectionProfile,
    lift_archive, project_archive, project_lpg_artifacts_to_sink,
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
    LpgAnnotation, LpgConfig, LpgEdge, LpgExecutionLimits, LpgGraph, LpgGraphContext,
    LpgIriSelection, LpgLabel, LpgLiftOutcome, LpgNamedGraphSelection, LpgNode,
    LpgPackageProjection, LpgProgress, LpgProgressObserver, LpgProgressPhase, LpgProjection,
    LpgProjectionReport, LpgProperty, LpgPropertyAtom, LpgRdfQuad, LpgReifier, LpgScope,
    LpgStreamProjection, lift_lpg, project_lpg, project_lpg_csv, project_lpg_csv_to_sink,
    project_lpg_cypher, project_lpg_cypher_to_sink, project_lpg_graphml,
    project_lpg_graphml_to_sink, project_lpg_with_progress, project_neo4j_csv,
    project_neo4j_csv_to_sink, read_lpg_csv, read_lpg_cypher, read_lpg_graphml, read_neo4j_csv,
    write_lpg_csv, write_lpg_csv_to_sink, write_lpg_cypher, write_lpg_cypher_to_sink,
    write_lpg_graphml, write_lpg_graphml_to_sink, write_neo4j_csv, write_neo4j_csv_to_sink,
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
    CroissantVocabulary, DATACITE_ARTIFACT, DATACITE_PROFILE, DCAT_ARTIFACT, DCAT_PROFILE,
    DCAT_ROLES, DataCiteConfig, DataCiteControlledValues, DcatConfig, DcatRole, DcatVocabulary,
    FRICTIONLESS_ARTIFACT, FRICTIONLESS_PROFILE, FrictionlessConfig, OfflineJsonLdContext,
    RESEARCH_ROLES, RO_CRATE_ARTIFACT, RO_CRATE_PROFILE, RO_CRATE_ROLES, ResearchActivity,
    ResearchAgent, ResearchChecksum, ResearchDataset, ResearchField, ResearchObjectConfig,
    ResearchObjectIdentity, ResearchObjectModel, ResearchObjectPackageProjection,
    ResearchObjectPolicy, ResearchObjectProjection, ResearchObjectReadOutcome, ResearchObjectRoles,
    ResearchRecordSet, ResearchResource, ResearchRole, ResearchText, ResearchValue, RoCrateConfig,
    RoCrateRole, RoCrateVocabulary, lift_research_object, project_croissant, project_datacite,
    project_dcat, project_frictionless, project_research_object, project_ro_crate, read_croissant,
    read_datacite, read_dcat, read_frictionless, read_ro_crate,
};
pub use sink::{ProjectionArtifactSink, ProjectionPackageSink};
pub use skos::{
    SkosClassRoles, SkosConfig, SkosDocumentationRoles, SkosGraphSelection, SkosLabelRoles,
    SkosProjection, SkosRelationRoles, SkosSourceRoles, SkosTargetRoles, project_skos,
};
pub use term::{ProjectionDirection, ProjectionTerm};
pub use util::{
    escape_cypher_identifier, escape_cypher_string, escape_xml_attribute, escape_xml_text,
    stable_identifier, validate_absolute_iri,
};
