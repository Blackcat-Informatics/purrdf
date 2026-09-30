// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::super::util::{iri_role_group, validate_distinct_roles};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

iri_role_group! {
    /// Caller-owned RDF and XML Schema roles used by the OBO Graphs projection.
    ///
    /// No vocabulary has a default. This keeps PurRDF an RDF carrier rather than an
    /// ontology and makes the exact interpretation visible at every call site.
    OboRdfRoles as "struct OboRdfRoles" in "OBO Graphs" {
        /// Caller-supplied `rdf_type` role IRI.
        rdf_type,
        /// Caller-supplied `rdf_reifies` role IRI.
        rdf_reifies,
        /// Caller-supplied `rdf_first` role IRI.
        rdf_first,
        /// Caller-supplied `rdf_rest` role IRI.
        rdf_rest,
        /// Caller-supplied `rdf_nil` role IRI.
        rdf_nil,
        /// Caller-supplied `xsd_string` role IRI.
        xsd_string,
        /// Caller-supplied `xsd_boolean` role IRI.
        xsd_boolean,
    }
}

iri_role_group! {
    /// Caller-owned RDFS and OWL semantic roles used by the projection.
    OboOwlRoles as "struct OboOwlRoles" in "OBO Graphs" {
        /// Caller-supplied `rdfs_label` role IRI.
        rdfs_label,
        /// Caller-supplied `rdfs_comment` role IRI.
        rdfs_comment,
        /// Caller-supplied `rdfs_sub_class_of` role IRI.
        rdfs_sub_class_of,
        /// Caller-supplied RDFS sub-property predicate.
        rdfs_sub_property_of,
        /// Caller-supplied `rdfs_domain` role IRI.
        rdfs_domain,
        /// Caller-supplied `rdfs_range` role IRI.
        rdfs_range,
        /// Caller-supplied `owl_ontology` role IRI.
        owl_ontology,
        /// Caller-supplied `owl_class` role IRI.
        owl_class,
        /// Caller-supplied `owl_named_individual` role IRI.
        owl_named_individual,
        /// Caller-supplied `owl_object_property` role IRI.
        owl_object_property,
        /// Caller-supplied `owl_annotation_property` role IRI.
        owl_annotation_property,
        /// Caller-supplied `owl_datatype_property` role IRI.
        owl_datatype_property,
        /// Caller-supplied `owl_equivalent_class` role IRI.
        owl_equivalent_class,
        /// Caller-supplied `owl_intersection_of` role IRI.
        owl_intersection_of,
        /// Caller-supplied `owl_restriction` role IRI.
        owl_restriction,
        /// Caller-supplied `owl_on_property` role IRI.
        owl_on_property,
        /// Caller-supplied `owl_some_values_from` role IRI.
        owl_some_values_from,
        /// Caller-supplied `owl_all_values_from` role IRI.
        owl_all_values_from,
        /// Caller-supplied `owl_property_chain_axiom` role IRI.
        owl_property_chain_axiom,
        /// Caller-supplied `owl_deprecated` role IRI.
        owl_deprecated,
    }
}

iri_role_group! {
    /// Caller-owned OBO metadata roles.
    OboMetadataRoles as "struct OboMetadataRoles" in "OBO Graphs" {
        /// Caller-supplied `definition` role IRI.
        definition,
        /// Caller-supplied `exact_synonym` role IRI.
        exact_synonym,
        /// Caller-supplied `broad_synonym` role IRI.
        broad_synonym,
        /// Caller-supplied `narrow_synonym` role IRI.
        narrow_synonym,
        /// Caller-supplied `related_synonym` role IRI.
        related_synonym,
        /// Caller-supplied `synonym_type` role IRI.
        synonym_type,
        /// Caller-supplied `xref` role IRI.
        xref,
        /// Caller-supplied `subset` role IRI.
        subset,
        /// Caller-supplied `version` role IRI.
        version,
    }
}

/// Complete caller-supplied semantic vocabulary for RDF→OBO Graphs 0.3.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OboGraphsVocabulary {
    rdf: OboRdfRoles,
    owl: OboOwlRoles,
    metadata: OboMetadataRoles,
}

impl OboGraphsVocabulary {
    /// Combine and cross-check the three mandatory role groups.
    pub fn new(
        rdf: OboRdfRoles,
        owl: OboOwlRoles,
        metadata: OboMetadataRoles,
    ) -> Result<Self, ProjectionError> {
        let vocabulary = Self { rdf, owl, metadata };
        vocabulary.validate()?;
        Ok(vocabulary)
    }

    pub(crate) const fn rdf(&self) -> &OboRdfRoles {
        &self.rdf
    }
    pub(crate) const fn owl(&self) -> &OboOwlRoles {
        &self.owl
    }
    pub(crate) const fn metadata(&self) -> &OboMetadataRoles {
        &self.metadata
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_distinct_roles(
            "OBO Graphs vocabulary",
            self.rdf
                .named_iris()
                .chain(self.owl.named_iris())
                .chain(self.metadata.named_iris()),
        )
    }
}

/// Mandatory graph identity, vocabulary, and resource bounds for OBO Graphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OboGraphsConfig {
    graph_id: String,
    vocabulary: OboGraphsVocabulary,
    limits: ProjectionLimits,
    max_records: usize,
}

impl OboGraphsConfig {
    /// Construct a fully explicit OBO Graphs projection policy.
    pub fn new(
        graph_id: impl Into<String>,
        vocabulary: OboGraphsVocabulary,
        limits: ProjectionLimits,
        max_records: usize,
    ) -> Result<Self, ProjectionError> {
        let graph_id = graph_id.into();
        validate_absolute_iri(&graph_id, "OBO Graphs caller-owned graph id")?;
        vocabulary.validate()?;
        if max_records == 0 {
            return Err(ProjectionError::configuration(
                "OBO Graphs max_records must be greater than zero",
            ));
        }
        if u32::try_from(max_records).is_err() {
            return Err(ProjectionError::configuration(
                "OBO Graphs max_records exceeds the portable u32 record ceiling",
            ));
        }
        Ok(Self {
            graph_id,
            vocabulary,
            limits,
            max_records,
        })
    }

    /// Caller-owned full IRI identifying the emitted graph.
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Complete caller-owned semantic vocabulary.
    pub const fn vocabulary(&self) -> &OboGraphsVocabulary {
        &self.vocabulary
    }

    /// Shared projection byte and recursion bounds.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Maximum input plus output records accepted by one projection.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }
}

purrdf_lex::json_record!(OboGraphsVocabulary as "struct OboGraphsVocabulary" {
    "rdf" => rdf: required,
    "owl" => owl: required,
    "metadata" => metadata: required,
});

purrdf_lex::json_record!(impl FromJson for OboGraphsConfig as "struct OboGraphsConfig" {
    "graph_id" => graph_id: required::<String>,
    "vocabulary" => vocabulary: required,
    "limits" => limits: required,
    "max_records" => max_records: required,
} => OboGraphsConfig::new);

purrdf_lex::json_record!(impl ToJson for OboGraphsConfig {
    "graph_id" => graph_id,
    "vocabulary" => vocabulary,
    "limits" => limits,
    "max_records" => max_records,
});
