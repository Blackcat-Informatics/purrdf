// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Caller-owned RDF and XML Schema roles used by the OBO Graphs projection.
///
/// No vocabulary has a default. This keeps PurRDF an RDF carrier rather than an
/// ontology and makes the exact interpretation visible at every call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OboRdfRoles {
    rdf_type: String,
    rdf_reifies: String,
    rdf_first: String,
    rdf_rest: String,
    rdf_nil: String,
    xsd_string: String,
    xsd_boolean: String,
}

impl OboRdfRoles {
    /// Construct the mandatory RDF/list/scalar role vocabulary.
    pub fn new(
        rdf_type: impl Into<String>,
        rdf_reifies: impl Into<String>,
        rdf_first: impl Into<String>,
        rdf_rest: impl Into<String>,
        rdf_nil: impl Into<String>,
        xsd_string: impl Into<String>,
        xsd_boolean: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            rdf_type: rdf_type.into(),
            rdf_reifies: rdf_reifies.into(),
            rdf_first: rdf_first.into(),
            rdf_rest: rdf_rest.into(),
            rdf_nil: rdf_nil.into(),
            xsd_string: xsd_string.into(),
            xsd_boolean: xsd_boolean.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    pub(crate) fn rdf_type(&self) -> &str {
        &self.rdf_type
    }

    pub(crate) fn rdf_reifies(&self) -> &str {
        &self.rdf_reifies
    }

    pub(crate) fn rdf_first(&self) -> &str {
        &self.rdf_first
    }

    pub(crate) fn rdf_rest(&self) -> &str {
        &self.rdf_rest
    }

    pub(crate) fn rdf_nil(&self) -> &str {
        &self.rdf_nil
    }

    pub(crate) fn xsd_string(&self) -> &str {
        &self.xsd_string
    }

    pub(crate) fn xsd_boolean(&self) -> &str {
        &self.xsd_boolean
    }

    fn named_iris(&self) -> [(&'static str, &str); 7] {
        [
            ("rdf_type", &self.rdf_type),
            ("rdf_reifies", &self.rdf_reifies),
            ("rdf_first", &self.rdf_first),
            ("rdf_rest", &self.rdf_rest),
            ("rdf_nil", &self.rdf_nil),
            ("xsd_string", &self.xsd_string),
            ("xsd_boolean", &self.xsd_boolean),
        ]
    }
}

/// Caller-owned RDFS and OWL semantic roles used by the projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OboOwlRoles {
    rdfs_label: String,
    rdfs_comment: String,
    rdfs_sub_class_of: String,
    rdfs_sub_property_of: String,
    rdfs_domain: String,
    rdfs_range: String,
    owl_ontology: String,
    owl_class: String,
    owl_named_individual: String,
    owl_object_property: String,
    owl_annotation_property: String,
    owl_datatype_property: String,
    owl_equivalent_class: String,
    owl_intersection_of: String,
    owl_restriction: String,
    owl_on_property: String,
    owl_some_values_from: String,
    owl_all_values_from: String,
    owl_property_chain_axiom: String,
    owl_deprecated: String,
}

impl OboOwlRoles {
    /// Construct the complete RDFS/OWL role vocabulary.
    #[allow(
        clippy::too_many_arguments,
        reason = "named mandatory roles keep omission visible and forbid fabricated defaults"
    )]
    pub fn new(
        rdfs_label: impl Into<String>,
        rdfs_comment: impl Into<String>,
        rdfs_sub_class_of: impl Into<String>,
        rdfs_sub_property_of: impl Into<String>,
        rdfs_domain: impl Into<String>,
        rdfs_range: impl Into<String>,
        owl_ontology: impl Into<String>,
        owl_class: impl Into<String>,
        owl_named_individual: impl Into<String>,
        owl_object_property: impl Into<String>,
        owl_annotation_property: impl Into<String>,
        owl_datatype_property: impl Into<String>,
        owl_equivalent_class: impl Into<String>,
        owl_intersection_of: impl Into<String>,
        owl_restriction: impl Into<String>,
        owl_on_property: impl Into<String>,
        owl_some_values_from: impl Into<String>,
        owl_all_values_from: impl Into<String>,
        owl_property_chain_axiom: impl Into<String>,
        owl_deprecated: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            rdfs_label: rdfs_label.into(),
            rdfs_comment: rdfs_comment.into(),
            rdfs_sub_class_of: rdfs_sub_class_of.into(),
            rdfs_sub_property_of: rdfs_sub_property_of.into(),
            rdfs_domain: rdfs_domain.into(),
            rdfs_range: rdfs_range.into(),
            owl_ontology: owl_ontology.into(),
            owl_class: owl_class.into(),
            owl_named_individual: owl_named_individual.into(),
            owl_object_property: owl_object_property.into(),
            owl_annotation_property: owl_annotation_property.into(),
            owl_datatype_property: owl_datatype_property.into(),
            owl_equivalent_class: owl_equivalent_class.into(),
            owl_intersection_of: owl_intersection_of.into(),
            owl_restriction: owl_restriction.into(),
            owl_on_property: owl_on_property.into(),
            owl_some_values_from: owl_some_values_from.into(),
            owl_all_values_from: owl_all_values_from.into(),
            owl_property_chain_axiom: owl_property_chain_axiom.into(),
            owl_deprecated: owl_deprecated.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    pub(crate) fn rdfs_label(&self) -> &str {
        &self.rdfs_label
    }
    pub(crate) fn rdfs_comment(&self) -> &str {
        &self.rdfs_comment
    }
    pub(crate) fn rdfs_sub_class_of(&self) -> &str {
        &self.rdfs_sub_class_of
    }
    /// Caller-supplied RDFS sub-property predicate.
    pub fn rdfs_sub_property_of(&self) -> &str {
        &self.rdfs_sub_property_of
    }
    pub(crate) fn rdfs_domain(&self) -> &str {
        &self.rdfs_domain
    }
    pub(crate) fn rdfs_range(&self) -> &str {
        &self.rdfs_range
    }
    pub(crate) fn owl_ontology(&self) -> &str {
        &self.owl_ontology
    }
    pub(crate) fn owl_class(&self) -> &str {
        &self.owl_class
    }
    pub(crate) fn owl_named_individual(&self) -> &str {
        &self.owl_named_individual
    }
    pub(crate) fn owl_object_property(&self) -> &str {
        &self.owl_object_property
    }
    pub(crate) fn owl_annotation_property(&self) -> &str {
        &self.owl_annotation_property
    }
    pub(crate) fn owl_datatype_property(&self) -> &str {
        &self.owl_datatype_property
    }
    pub(crate) fn owl_equivalent_class(&self) -> &str {
        &self.owl_equivalent_class
    }
    pub(crate) fn owl_intersection_of(&self) -> &str {
        &self.owl_intersection_of
    }
    pub(crate) fn owl_restriction(&self) -> &str {
        &self.owl_restriction
    }
    pub(crate) fn owl_on_property(&self) -> &str {
        &self.owl_on_property
    }
    pub(crate) fn owl_some_values_from(&self) -> &str {
        &self.owl_some_values_from
    }
    pub(crate) fn owl_all_values_from(&self) -> &str {
        &self.owl_all_values_from
    }
    pub(crate) fn owl_property_chain_axiom(&self) -> &str {
        &self.owl_property_chain_axiom
    }
    pub(crate) fn owl_deprecated(&self) -> &str {
        &self.owl_deprecated
    }

    fn named_iris(&self) -> [(&'static str, &str); 20] {
        [
            ("rdfs_label", &self.rdfs_label),
            ("rdfs_comment", &self.rdfs_comment),
            ("rdfs_sub_class_of", &self.rdfs_sub_class_of),
            ("rdfs_sub_property_of", &self.rdfs_sub_property_of),
            ("rdfs_domain", &self.rdfs_domain),
            ("rdfs_range", &self.rdfs_range),
            ("owl_ontology", &self.owl_ontology),
            ("owl_class", &self.owl_class),
            ("owl_named_individual", &self.owl_named_individual),
            ("owl_object_property", &self.owl_object_property),
            ("owl_annotation_property", &self.owl_annotation_property),
            ("owl_datatype_property", &self.owl_datatype_property),
            ("owl_equivalent_class", &self.owl_equivalent_class),
            ("owl_intersection_of", &self.owl_intersection_of),
            ("owl_restriction", &self.owl_restriction),
            ("owl_on_property", &self.owl_on_property),
            ("owl_some_values_from", &self.owl_some_values_from),
            ("owl_all_values_from", &self.owl_all_values_from),
            ("owl_property_chain_axiom", &self.owl_property_chain_axiom),
            ("owl_deprecated", &self.owl_deprecated),
        ]
    }
}

/// Caller-owned OBO metadata roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OboMetadataRoles {
    definition: String,
    exact_synonym: String,
    broad_synonym: String,
    narrow_synonym: String,
    related_synonym: String,
    synonym_type: String,
    xref: String,
    subset: String,
    version: String,
}

impl OboMetadataRoles {
    /// Construct the complete OBO metadata role vocabulary.
    #[allow(
        clippy::too_many_arguments,
        reason = "named mandatory roles keep omission visible and forbid fabricated defaults"
    )]
    pub fn new(
        definition: impl Into<String>,
        exact_synonym: impl Into<String>,
        broad_synonym: impl Into<String>,
        narrow_synonym: impl Into<String>,
        related_synonym: impl Into<String>,
        synonym_type: impl Into<String>,
        xref: impl Into<String>,
        subset: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            definition: definition.into(),
            exact_synonym: exact_synonym.into(),
            broad_synonym: broad_synonym.into(),
            narrow_synonym: narrow_synonym.into(),
            related_synonym: related_synonym.into(),
            synonym_type: synonym_type.into(),
            xref: xref.into(),
            subset: subset.into(),
            version: version.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    pub(crate) fn definition(&self) -> &str {
        &self.definition
    }
    pub(crate) fn exact_synonym(&self) -> &str {
        &self.exact_synonym
    }
    pub(crate) fn broad_synonym(&self) -> &str {
        &self.broad_synonym
    }
    pub(crate) fn narrow_synonym(&self) -> &str {
        &self.narrow_synonym
    }
    pub(crate) fn related_synonym(&self) -> &str {
        &self.related_synonym
    }
    pub(crate) fn synonym_type(&self) -> &str {
        &self.synonym_type
    }
    pub(crate) fn xref(&self) -> &str {
        &self.xref
    }
    pub(crate) fn subset(&self) -> &str {
        &self.subset
    }
    pub(crate) fn version(&self) -> &str {
        &self.version
    }

    fn named_iris(&self) -> [(&'static str, &str); 9] {
        [
            ("definition", &self.definition),
            ("exact_synonym", &self.exact_synonym),
            ("broad_synonym", &self.broad_synonym),
            ("narrow_synonym", &self.narrow_synonym),
            ("related_synonym", &self.related_synonym),
            ("synonym_type", &self.synonym_type),
            ("xref", &self.xref),
            ("subset", &self.subset),
            ("version", &self.version),
        ]
    }
}

/// Complete caller-supplied semantic vocabulary for RDF→OBO Graphs 0.3.2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
        let mut seen = BTreeSet::new();
        for (name, iri) in self
            .rdf
            .named_iris()
            .into_iter()
            .chain(self.owl.named_iris())
            .chain(self.metadata.named_iris())
        {
            validate_absolute_iri(iri, &format!("OBO Graphs vocabulary role `{name}`"))?;
            if !seen.insert(iri) {
                return Err(ProjectionError::configuration(format!(
                    "OBO Graphs vocabulary role `{name}` reuses `{iri}`; semantic roles must be distinct"
                )));
            }
        }
        Ok(())
    }
}

/// Mandatory graph identity, vocabulary, and resource bounds for OBO Graphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOboGraphsConfig {
    graph_id: String,
    vocabulary: OboGraphsVocabulary,
    limits: ProjectionLimits,
    max_records: usize,
}

impl<'de> Deserialize<'de> for OboGraphsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawOboGraphsConfig::deserialize(deserializer)?;
        Self::new(raw.graph_id, raw.vocabulary, raw.limits, raw.max_records)
            .map_err(serde::de::Error::custom)
    }
}

fn validate_named_iris<const N: usize>(iris: [(&str, &str); N]) -> Result<(), ProjectionError> {
    for (name, iri) in iris {
        validate_absolute_iri(iri, &format!("OBO Graphs vocabulary role `{name}`"))?;
    }
    Ok(())
}
