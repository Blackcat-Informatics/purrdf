// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The projection configurations the projection tests, benches and examples build:
//! caller-supplied vocabularies over test-fixture IRIs (PurRDF mints none), each
//! namespaced under the caller's own `ex` base.

// The module is included into more than one target, and no single target uses every
// helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::collections::BTreeMap;

use purrdf_rdf::{
    CsvwConfig, CsvwContext, CsvwDatatype, CsvwMode, CsvwVocabulary, OboGraphsConfig,
    OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles, ProjectionLimits,
    SkosDocumentationRoles, SkosLabelRoles, SkosRelationRoles,
};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";

/// The projection limits every configuration here carries.
pub fn limits() -> ProjectionLimits {
    ProjectionLimits::new(64, 16_000_000, 64_000_000, 72_000_000, 16).expect("limits")
}

/// An exact-mode CSVW configuration whose metadata, context and group IRIs sit
/// under `ex`, admitting up to `max_rows` rows.
pub fn csvw_config(ex: &str, max_rows: usize) -> CsvwConfig {
    CsvwConfig::new(
        format!("{ex}csvw-metadata"),
        CsvwContext::new(format!("{ex}csvw-context"), BTreeMap::default()).expect("context"),
        format!("{ex}csvw-group"),
        CsvwVocabulary::new("http://www.w3.org/ns/csvw#", RDF, RDFS, XSD).expect("CSVW vocabulary"),
        CsvwMode::Standard,
        limits(),
        max_rows,
    )
    .expect("CSVW config")
}

/// A CSVW datatype of `base` with no facet.
pub fn csvw_datatype(base: impl Into<String>) -> CsvwDatatype {
    CsvwDatatype {
        id: None,
        base: base.into(),
        format: None,
        length: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        min_inclusive: None,
        max_inclusive: None,
        min_exclusive: None,
        max_exclusive: None,
    }
}

/// An OBO Graphs configuration whose ontology and definition IRIs sit under `ex`.
pub fn obo_config(ex: &str) -> OboGraphsConfig {
    let rdf = OboRdfRoles::new(
        format!("{RDF}type"),
        format!("{RDF}reifies"),
        format!("{RDF}first"),
        format!("{RDF}rest"),
        format!("{RDF}nil"),
        format!("{XSD}string"),
        format!("{XSD}boolean"),
    )
    .expect("RDF roles");
    let owl = OboOwlRoles::new(
        format!("{RDFS}label"),
        format!("{RDFS}comment"),
        format!("{RDFS}subClassOf"),
        format!("{RDFS}subPropertyOf"),
        format!("{RDFS}domain"),
        format!("{RDFS}range"),
        format!("{OWL}Ontology"),
        format!("{OWL}Class"),
        format!("{OWL}NamedIndividual"),
        format!("{OWL}ObjectProperty"),
        format!("{OWL}AnnotationProperty"),
        format!("{OWL}DatatypeProperty"),
        format!("{OWL}equivalentClass"),
        format!("{OWL}intersectionOf"),
        format!("{OWL}Restriction"),
        format!("{OWL}onProperty"),
        format!("{OWL}someValuesFrom"),
        format!("{OWL}allValuesFrom"),
        format!("{OWL}propertyChainAxiom"),
        format!("{OWL}deprecated"),
    )
    .expect("OWL roles");
    let metadata = OboMetadataRoles::new(
        format!("{ex}definition"),
        format!("{OBO}hasExactSynonym"),
        format!("{OBO}hasBroadSynonym"),
        format!("{OBO}hasNarrowSynonym"),
        format!("{OBO}hasRelatedSynonym"),
        format!("{OBO}hasSynonymType"),
        format!("{OBO}hasDbXref"),
        format!("{OBO}inSubset"),
        format!("{OWL}versionInfo"),
    )
    .expect("OBO metadata roles");
    OboGraphsConfig::new(
        format!("{ex}ontology"),
        OboGraphsVocabulary::new(rdf, owl, metadata).expect("OBO vocabulary"),
        limits(),
        20_000,
    )
    .expect("OBO config")
}

/// The SKOS label roles under the namespace `prefix`.
pub fn skos_label_roles(prefix: &str) -> SkosLabelRoles {
    SkosLabelRoles::new(
        format!("{prefix}prefLabel"),
        format!("{prefix}altLabel"),
        format!("{prefix}hiddenLabel"),
        format!("{prefix}notation"),
    )
    .expect("SKOS labels")
}

/// The SKOS documentation roles under the namespace `prefix`.
pub fn skos_documentation_roles(prefix: &str) -> SkosDocumentationRoles {
    SkosDocumentationRoles::new(
        format!("{prefix}note"),
        format!("{prefix}changeNote"),
        format!("{prefix}definition"),
        format!("{prefix}editorialNote"),
        format!("{prefix}example"),
        format!("{prefix}historyNote"),
        format!("{prefix}scopeNote"),
    )
    .expect("SKOS documentation")
}

/// The SKOS relation roles under the namespace `prefix`.
pub fn skos_relation_roles(prefix: &str) -> SkosRelationRoles {
    SkosRelationRoles::new(
        format!("{prefix}broader"),
        format!("{prefix}narrower"),
        format!("{prefix}related"),
        format!("{prefix}closeMatch"),
        format!("{prefix}exactMatch"),
        format!("{prefix}broadMatch"),
        format!("{prefix}narrowMatch"),
        format!("{prefix}relatedMatch"),
        format!("{prefix}inScheme"),
        format!("{prefix}hasTopConcept"),
        format!("{prefix}topConceptOf"),
    )
    .expect("SKOS relations")
}
