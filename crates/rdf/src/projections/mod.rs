// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2 projection foundations.
//!
//! Projection codecs share one bounded in-memory package, one durable RDF term
//! representation, one typed error surface, and one set of escaping/identity
//! primitives. Filesystem and network access stay outside this module, so the same
//! engine runs unchanged in native, WebAssembly, Python, and C hosts.

mod carrier;
mod csvw;
mod error;
mod lpg;
mod obo_graphs;
mod package;
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
pub use skos::{
    SkosClassRoles, SkosConfig, SkosDocumentationRoles, SkosGraphSelection, SkosLabelRoles,
    SkosProjection, SkosRelationRoles, SkosSourceRoles, SkosTargetRoles, project_skos,
};
pub use term::{ProjectionDirection, ProjectionTerm};
pub use util::{
    escape_cypher_identifier, escape_cypher_string, escape_xml_attribute, escape_xml_text,
    stable_identifier, validate_absolute_iri,
};
