// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::native_codecs::NativeRdfFormat;

use super::super::{ProjectionDirection, ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Exact source graph selected for one VoID input role.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum VoidGraphSelector {
    /// The RDF dataset default graph.
    DefaultGraph,
    /// One exact caller-owned named graph IRI.
    NamedGraph {
        /// Absolute named graph IRI.
        graph_iri: String,
    },
}

impl VoidGraphSelector {
    /// Construct an exact named-graph selector.
    ///
    /// # Errors
    ///
    /// Rejects a relative or malformed graph IRI.
    pub fn named(graph_iri: impl Into<String>) -> Result<Self, ProjectionError> {
        let graph_iri = graph_iri.into();
        validate_absolute_iri(&graph_iri, "VoID named graph")?;
        Ok(Self::NamedGraph { graph_iri })
    }

    /// Selected graph IRI, or `None` for the default graph.
    pub fn graph_iri(&self) -> Option<&str> {
        match self {
            Self::DefaultGraph => None,
            Self::NamedGraph { graph_iri } => Some(graph_iri),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
enum RawVoidGraphSelector {
    DefaultGraph,
    NamedGraph { graph_iri: String },
}

impl<'de> Deserialize<'de> for VoidGraphSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match RawVoidGraphSelector::deserialize(deserializer)? {
            RawVoidGraphSelector::DefaultGraph => Ok(Self::DefaultGraph),
            RawVoidGraphSelector::NamedGraph { graph_iri } => {
                Self::named(graph_iri).map_err(serde::de::Error::custom)
            }
        }
    }
}

/// Complete caller-owned source predicate binding for VoID extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VoidSourceRoles {
    rdf_type: String,
    header_version: String,
    header_abstract: String,
}

impl VoidSourceRoles {
    /// Construct the mandatory source-role binding.
    ///
    /// # Errors
    ///
    /// Rejects relative or colliding predicate IRIs.
    pub fn new(
        rdf_type: impl Into<String>,
        header_version: impl Into<String>,
        header_abstract: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let rdf_type = rdf_type.into();
        let header_version = header_version.into();
        let header_abstract = header_abstract.into();
        for (value, label) in [
            (&rdf_type, "VoID source rdf:type predicate"),
            (&header_version, "VoID source header version predicate"),
            (&header_abstract, "VoID source header abstract predicate"),
        ] {
            validate_absolute_iri(value, label)?;
        }
        if BTreeSet::from([
            rdf_type.as_str(),
            header_version.as_str(),
            header_abstract.as_str(),
        ])
        .len()
            != 3
        {
            return Err(ProjectionError::configuration(
                "VoID source role predicates must be distinct",
            ));
        }
        Ok(Self {
            rdf_type,
            header_version,
            header_abstract,
        })
    }

    /// Source RDF type predicate.
    pub fn rdf_type(&self) -> &str {
        &self.rdf_type
    }

    /// Source header version predicate.
    pub fn header_version(&self) -> &str {
        &self.header_version
    }

    /// Source header abstract predicate.
    pub fn header_abstract(&self) -> &str {
        &self.header_abstract
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVoidSourceRoles {
    rdf_type: String,
    header_version: String,
    header_abstract: String,
}

impl<'de> Deserialize<'de> for VoidSourceRoles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidSourceRoles::deserialize(deserializer)?;
        Self::new(raw.rdf_type, raw.header_version, raw.header_abstract)
            .map_err(serde::de::Error::custom)
    }
}

/// Semantic target role in a caller-owned VoID vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoidRole {
    /// RDF type predicate.
    RdfType,
    /// Dataset class.
    DatasetClass,
    /// Linkset class.
    LinksetClass,
    /// Class-partition resource class.
    ClassPartitionClass,
    /// Property-partition resource class.
    PropertyPartitionClass,
    /// Version predicate.
    Version,
    /// Abstract/description predicate.
    Abstract,
    /// Generic subset relation.
    Subset,
    /// Triple-count predicate.
    Triples,
    /// Entity-count predicate.
    Entities,
    /// Class-count predicate.
    Classes,
    /// Property-count predicate.
    Properties,
    /// Dataset-to-class-partition relation.
    ClassPartition,
    /// Dataset-to-property-partition relation.
    PropertyPartition,
    /// Partition class descriptor.
    Class,
    /// Partition property descriptor.
    Property,
    /// Distinct-subject count predicate.
    DistinctSubjects,
    /// Distinct-object count predicate.
    DistinctObjects,
    /// Linkset subject-dataset target.
    SubjectsTarget,
    /// Linkset object-dataset target.
    ObjectsTarget,
    /// Linkset predicate descriptor.
    LinkPredicate,
    /// Non-negative integer datatype.
    XsdNonNegativeInteger,
}

/// Every mandatory VoID target role, in stable configuration order.
pub const VOID_ROLES: &[VoidRole] = &[
    VoidRole::RdfType,
    VoidRole::DatasetClass,
    VoidRole::LinksetClass,
    VoidRole::ClassPartitionClass,
    VoidRole::PropertyPartitionClass,
    VoidRole::Version,
    VoidRole::Abstract,
    VoidRole::Subset,
    VoidRole::Triples,
    VoidRole::Entities,
    VoidRole::Classes,
    VoidRole::Properties,
    VoidRole::ClassPartition,
    VoidRole::PropertyPartition,
    VoidRole::Class,
    VoidRole::Property,
    VoidRole::DistinctSubjects,
    VoidRole::DistinctObjects,
    VoidRole::SubjectsTarget,
    VoidRole::ObjectsTarget,
    VoidRole::LinkPredicate,
    VoidRole::XsdNonNegativeInteger,
];

/// Complete caller-owned target vocabulary for VoID output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct VoidVocabulary(BTreeMap<VoidRole, String>);

impl VoidVocabulary {
    /// Construct and validate one complete target vocabulary.
    ///
    /// # Errors
    ///
    /// Rejects a missing/unknown role, a relative IRI, or two roles bound to one IRI.
    pub fn new(terms: BTreeMap<VoidRole, String>) -> Result<Self, ProjectionError> {
        for role in VOID_ROLES {
            let iri = terms.get(role).ok_or_else(|| {
                ProjectionError::configuration(format!(
                    "VoID target vocabulary is missing role `{role:?}`"
                ))
            })?;
            validate_absolute_iri(iri, &format!("VoID target role `{role:?}`"))?;
        }
        if terms.len() != VOID_ROLES.len() {
            return Err(ProjectionError::configuration(
                "VoID target vocabulary contains an unsupported role",
            ));
        }
        let mut inverse = BTreeMap::<&str, VoidRole>::new();
        for (&role, iri) in &terms {
            if let Some(previous) = inverse.insert(iri, role) {
                return Err(ProjectionError::configuration(format!(
                    "VoID target roles `{previous:?}` and `{role:?}` collide at `{iri}`"
                )));
            }
        }
        Ok(Self(terms))
    }

    /// IRI bound to one target role.
    pub fn iri(&self, role: VoidRole) -> &str {
        self.0
            .get(&role)
            .expect("validated VoID vocabulary contains every role")
    }

    /// Complete stable role map.
    pub const fn terms(&self) -> &BTreeMap<VoidRole, String> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for VoidVocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let terms = BTreeMap::<VoidRole, String>::deserialize(deserializer)?;
        Self::new(terms).map_err(serde::de::Error::custom)
    }
}

/// One deterministic IRI-prefix to dataset identity binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct VoidDatasetPrefix {
    dataset_iri: String,
    iri_prefix: String,
}

impl VoidDatasetPrefix {
    /// Construct one prefix binding.
    ///
    /// # Errors
    ///
    /// Rejects relative dataset or prefix IRIs.
    pub fn new(
        dataset_iri: impl Into<String>,
        iri_prefix: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let dataset_iri = dataset_iri.into();
        let iri_prefix = iri_prefix.into();
        validate_absolute_iri(&dataset_iri, "VoID prefix dataset IRI")?;
        validate_absolute_iri(&iri_prefix, "VoID resource IRI prefix")?;
        Ok(Self {
            dataset_iri,
            iri_prefix,
        })
    }

    /// Dataset identity described by matching resources.
    pub fn dataset_iri(&self) -> &str {
        &self.dataset_iri
    }

    /// Resource IRI prefix used for longest-prefix classification.
    pub fn iri_prefix(&self) -> &str {
        &self.iri_prefix
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVoidDatasetPrefix {
    dataset_iri: String,
    iri_prefix: String,
}

impl<'de> Deserialize<'de> for VoidDatasetPrefix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidDatasetPrefix::deserialize(deserializer)?;
        Self::new(raw.dataset_iri, raw.iri_prefix).map_err(serde::de::Error::custom)
    }
}

/// Source-to-target predicate mapping for metadata-graph external IRI links.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct VoidExternalLinkMapping {
    source_predicate: String,
    target_predicate: String,
}

impl VoidExternalLinkMapping {
    /// Construct one external-link predicate mapping.
    ///
    /// # Errors
    ///
    /// Rejects a relative source or target predicate IRI.
    pub fn new(
        source_predicate: impl Into<String>,
        target_predicate: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let source_predicate = source_predicate.into();
        let target_predicate = target_predicate.into();
        validate_absolute_iri(&source_predicate, "VoID external-link source predicate")?;
        validate_absolute_iri(&target_predicate, "VoID external-link target predicate")?;
        Ok(Self {
            source_predicate,
            target_predicate,
        })
    }

    /// Predicate read in the metadata graph.
    pub fn source_predicate(&self) -> &str {
        &self.source_predicate
    }

    /// Predicate emitted on the described dataset.
    pub fn target_predicate(&self) -> &str {
        &self.target_predicate
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVoidExternalLinkMapping {
    source_predicate: String,
    target_predicate: String,
}

impl<'de> Deserialize<'de> for VoidExternalLinkMapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidExternalLinkMapping::deserialize(deserializer)?;
        Self::new(raw.source_predicate, raw.target_predicate).map_err(serde::de::Error::custom)
    }
}

/// Caller-authored IRI or RDF literal on the described dataset.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum VoidStaticValue {
    /// Absolute IRI object.
    Iri {
        /// IRI value.
        value: String,
    },
    /// Typed literal without language or base direction.
    TypedLiteral {
        /// Lexical form.
        lexical: String,
        /// Absolute datatype IRI.
        datatype: String,
    },
    /// RDF 1.2 language literal with optional base direction.
    LanguageLiteral {
        /// Lexical form.
        lexical: String,
        /// Non-empty language tag.
        language: String,
        /// Optional RDF 1.2 base direction.
        direction: Option<ProjectionDirection>,
    },
}

impl VoidStaticValue {
    /// Construct an IRI object.
    ///
    /// # Errors
    ///
    /// Rejects a relative or malformed IRI.
    pub fn iri(value: impl Into<String>) -> Result<Self, ProjectionError> {
        let value = value.into();
        validate_absolute_iri(&value, "VoID static object IRI")?;
        Ok(Self::Iri { value })
    }

    /// Construct a typed literal.
    ///
    /// # Errors
    ///
    /// Rejects a relative or malformed datatype IRI.
    pub fn typed_literal(
        lexical: impl Into<String>,
        datatype: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let datatype = datatype.into();
        validate_absolute_iri(&datatype, "VoID static literal datatype")?;
        Ok(Self::TypedLiteral {
            lexical: lexical.into(),
            datatype,
        })
    }

    /// Construct a language-tagged literal.
    ///
    /// # Errors
    ///
    /// Rejects an empty language tag.
    pub fn language_literal(
        lexical: impl Into<String>,
        language: impl Into<String>,
        direction: Option<ProjectionDirection>,
    ) -> Result<Self, ProjectionError> {
        let language = language.into();
        if language.is_empty() {
            return Err(ProjectionError::configuration(
                "VoID static literal language must not be empty",
            ));
        }
        Ok(Self::LanguageLiteral {
            lexical: lexical.into(),
            language,
            direction,
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum RawVoidStaticValue {
    Iri {
        value: String,
    },
    TypedLiteral {
        lexical: String,
        datatype: String,
    },
    LanguageLiteral {
        lexical: String,
        language: String,
        direction: Option<ProjectionDirection>,
    },
}

impl<'de> Deserialize<'de> for VoidStaticValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match RawVoidStaticValue::deserialize(deserializer)? {
            RawVoidStaticValue::Iri { value } => Self::iri(value).map_err(serde::de::Error::custom),
            RawVoidStaticValue::TypedLiteral { lexical, datatype } => {
                Self::typed_literal(lexical, datatype).map_err(serde::de::Error::custom)
            }
            RawVoidStaticValue::LanguageLiteral {
                lexical,
                language,
                direction,
            } => Self::language_literal(lexical, language, direction)
                .map_err(serde::de::Error::custom),
        }
    }
}

/// One caller-authored statement whose subject is the described dataset IRI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct VoidStaticStatement {
    predicate: String,
    object: VoidStaticValue,
}

impl VoidStaticStatement {
    /// Construct one static dataset statement.
    ///
    /// # Errors
    ///
    /// Rejects a relative or malformed predicate IRI.
    pub fn new(
        predicate: impl Into<String>,
        object: VoidStaticValue,
    ) -> Result<Self, ProjectionError> {
        let predicate = predicate.into();
        validate_absolute_iri(&predicate, "VoID static statement predicate")?;
        Ok(Self { predicate, object })
    }

    /// Emitted predicate IRI.
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    /// Emitted IRI or literal object.
    pub const fn object(&self) -> &VoidStaticValue {
        &self.object
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVoidStaticStatement {
    predicate: String,
    object: VoidStaticValue,
}

impl<'de> Deserialize<'de> for VoidStaticStatement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidStaticStatement::deserialize(deserializer)?;
        Self::new(raw.predicate, raw.object).map_err(serde::de::Error::custom)
    }
}

/// Explicit compute and materialization bounds for VoID generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(
    clippy::struct_field_names,
    reason = "execution-limit JSON fields intentionally share the `max_` policy prefix"
)]
pub struct VoidExecutionLimits {
    max_input_records: usize,
    max_output_records: usize,
    max_partitions: usize,
    max_linksets: usize,
    max_partition_memberships: usize,
    max_dataset_prefixes: usize,
    max_external_link_mappings: usize,
    max_static_statements: usize,
}

impl VoidExecutionLimits {
    /// Construct portable positive execution bounds.
    ///
    /// # Errors
    ///
    /// Rejects zero or values beyond the portable `u32` ceiling.
    #[allow(
        clippy::too_many_arguments,
        reason = "every independent VoID compute and configuration budget is mandatory"
    )]
    pub fn new(
        max_input_records: usize,
        max_output_records: usize,
        max_partitions: usize,
        max_linksets: usize,
        max_partition_memberships: usize,
        max_dataset_prefixes: usize,
        max_external_link_mappings: usize,
        max_static_statements: usize,
    ) -> Result<Self, ProjectionError> {
        for (value, label) in [
            (max_input_records, "VoID max_input_records"),
            (max_output_records, "VoID max_output_records"),
            (max_partitions, "VoID max_partitions"),
            (max_linksets, "VoID max_linksets"),
            (max_partition_memberships, "VoID max_partition_memberships"),
            (max_dataset_prefixes, "VoID max_dataset_prefixes"),
            (
                max_external_link_mappings,
                "VoID max_external_link_mappings",
            ),
            (max_static_statements, "VoID max_static_statements"),
        ] {
            validate_bound(value, label)?;
        }
        Ok(Self {
            max_input_records,
            max_output_records,
            max_partitions,
            max_linksets,
            max_partition_memberships,
            max_dataset_prefixes,
            max_external_link_mappings,
            max_static_statements,
        })
    }

    /// Maximum source records admitted before graph selection.
    pub const fn max_input_records(self) -> usize {
        self.max_input_records
    }

    /// Maximum RDF records in the emitted description.
    pub const fn max_output_records(self) -> usize {
        self.max_output_records
    }

    /// Maximum combined class and property partitions.
    pub const fn max_partitions(self) -> usize {
        self.max_partitions
    }

    /// Maximum distinct linkset groups.
    pub const fn max_linksets(self) -> usize {
        self.max_linksets
    }

    /// Maximum data-row to class-partition expansion work.
    pub const fn max_partition_memberships(self) -> usize {
        self.max_partition_memberships
    }

    /// Maximum combined local and external prefix bindings.
    pub const fn max_dataset_prefixes(self) -> usize {
        self.max_dataset_prefixes
    }

    /// Maximum metadata external-link predicate mappings.
    pub const fn max_external_link_mappings(self) -> usize {
        self.max_external_link_mappings
    }

    /// Maximum caller-authored static dataset statements.
    pub const fn max_static_statements(self) -> usize {
        self.max_static_statements
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    clippy::struct_field_names,
    reason = "raw execution-limit fields mirror the validated public JSON contract"
)]
struct RawVoidExecutionLimits {
    max_input_records: usize,
    max_output_records: usize,
    max_partitions: usize,
    max_linksets: usize,
    max_partition_memberships: usize,
    max_dataset_prefixes: usize,
    max_external_link_mappings: usize,
    max_static_statements: usize,
}

impl<'de> Deserialize<'de> for VoidExecutionLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidExecutionLimits::deserialize(deserializer)?;
        Self::new(
            raw.max_input_records,
            raw.max_output_records,
            raw.max_partitions,
            raw.max_linksets,
            raw.max_partition_memberships,
            raw.max_dataset_prefixes,
            raw.max_external_link_mappings,
            raw.max_static_statements,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Complete deterministic VoID dataset-description policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VoidConfig {
    format: NativeRdfFormat,
    dataset_iri: String,
    generated_resource_base_iri: String,
    header_subject_iri: String,
    source_roles: VoidSourceRoles,
    header_graph: VoidGraphSelector,
    alignment_graph: VoidGraphSelector,
    metadata_graph: VoidGraphSelector,
    data_graphs: Vec<VoidGraphSelector>,
    vocabulary: VoidVocabulary,
    local_datasets: Vec<VoidDatasetPrefix>,
    external_datasets: Vec<VoidDatasetPrefix>,
    external_links: Vec<VoidExternalLinkMapping>,
    static_statements: Vec<VoidStaticStatement>,
    limits: ProjectionLimits,
    execution_limits: VoidExecutionLimits,
}

impl VoidConfig {
    /// Construct one complete caller-vocabulary VoID policy.
    ///
    /// # Errors
    ///
    /// Rejects malformed identities, duplicate/ambiguous selectors or prefixes,
    /// missing local ownership, role collisions, and configuration-limit breaches.
    #[allow(
        clippy::too_many_arguments,
        reason = "VoID source, target, identity, graph, registry, and budget policies are independent"
    )]
    pub fn new(
        format: NativeRdfFormat,
        dataset_iri: impl Into<String>,
        generated_resource_base_iri: impl Into<String>,
        header_subject_iri: impl Into<String>,
        source_roles: VoidSourceRoles,
        header_graph: VoidGraphSelector,
        alignment_graph: VoidGraphSelector,
        metadata_graph: VoidGraphSelector,
        data_graphs: Vec<VoidGraphSelector>,
        vocabulary: VoidVocabulary,
        local_datasets: Vec<VoidDatasetPrefix>,
        external_datasets: Vec<VoidDatasetPrefix>,
        external_links: Vec<VoidExternalLinkMapping>,
        static_statements: Vec<VoidStaticStatement>,
        limits: ProjectionLimits,
        execution_limits: VoidExecutionLimits,
    ) -> Result<Self, ProjectionError> {
        let dataset_iri = dataset_iri.into();
        let generated_resource_base_iri = generated_resource_base_iri.into();
        let header_subject_iri = header_subject_iri.into();
        validate_absolute_iri(&dataset_iri, "VoID described dataset IRI")?;
        validate_absolute_iri(
            &generated_resource_base_iri,
            "VoID generated-resource base IRI",
        )?;
        validate_absolute_iri(&header_subject_iri, "VoID source header subject IRI")?;

        let special_graphs = BTreeSet::from([
            header_graph.clone(),
            alignment_graph.clone(),
            metadata_graph.clone(),
        ]);
        if special_graphs.len() != 3 {
            return Err(ProjectionError::configuration(
                "VoID header, alignment, and metadata graph selectors must be distinct",
            ));
        }
        if data_graphs.is_empty() {
            return Err(ProjectionError::configuration(
                "VoID data graph selection must not be empty",
            ));
        }
        if data_graphs.iter().cloned().collect::<BTreeSet<_>>().len() != data_graphs.len() {
            return Err(ProjectionError::configuration(
                "VoID data graph selection contains a duplicate graph",
            ));
        }
        if local_datasets.is_empty() {
            return Err(ProjectionError::configuration(
                "VoID local dataset prefix registry must not be empty",
            ));
        }

        let prefix_count = local_datasets
            .len()
            .checked_add(external_datasets.len())
            .ok_or_else(|| ProjectionError::limit("VoID dataset prefix count overflow"))?;
        if prefix_count > execution_limits.max_dataset_prefixes() {
            return Err(ProjectionError::limit(format!(
                "VoID has {prefix_count} dataset prefixes; limit is {}",
                execution_limits.max_dataset_prefixes()
            )));
        }
        if external_links.len() > execution_limits.max_external_link_mappings() {
            return Err(ProjectionError::limit(format!(
                "VoID has {} external-link mappings; limit is {}",
                external_links.len(),
                execution_limits.max_external_link_mappings()
            )));
        }
        if static_statements.len() > execution_limits.max_static_statements() {
            return Err(ProjectionError::limit(format!(
                "VoID has {} static statements; limit is {}",
                static_statements.len(),
                execution_limits.max_static_statements()
            )));
        }

        let mut prefixes = BTreeMap::<&str, (&str, bool)>::new();
        let mut local_dataset_ids = BTreeSet::new();
        let mut external_dataset_ids = BTreeSet::new();
        for binding in &local_datasets {
            local_dataset_ids.insert(binding.dataset_iri());
            if let Some((previous, _)) =
                prefixes.insert(binding.iri_prefix(), (binding.dataset_iri(), true))
            {
                return Err(ProjectionError::configuration(format!(
                    "VoID resource prefix `{}` is bound more than once (to `{previous}` and `{}`)",
                    binding.iri_prefix(),
                    binding.dataset_iri()
                )));
            }
        }
        for binding in &external_datasets {
            external_dataset_ids.insert(binding.dataset_iri());
            if let Some((previous, _)) =
                prefixes.insert(binding.iri_prefix(), (binding.dataset_iri(), false))
            {
                return Err(ProjectionError::configuration(format!(
                    "VoID resource prefix `{}` is bound more than once (to `{previous}` and `{}`)",
                    binding.iri_prefix(),
                    binding.dataset_iri()
                )));
            }
        }
        if !local_dataset_ids.contains(dataset_iri.as_str()) {
            return Err(ProjectionError::configuration(format!(
                "VoID described dataset `{dataset_iri}` is absent from the local prefix registry"
            )));
        }
        if let Some(collision) = local_dataset_ids.intersection(&external_dataset_ids).next() {
            return Err(ProjectionError::configuration(format!(
                "VoID dataset `{collision}` is classified as both local and external"
            )));
        }

        let mut external_sources = BTreeSet::new();
        for mapping in &external_links {
            if !external_sources.insert(mapping.source_predicate()) {
                return Err(ProjectionError::configuration(format!(
                    "VoID external-link source predicate `{}` is mapped more than once",
                    mapping.source_predicate()
                )));
            }
            if vocabulary
                .terms()
                .values()
                .any(|iri| iri == mapping.target_predicate())
            {
                return Err(ProjectionError::configuration(format!(
                    "VoID external-link target predicate `{}` collides with a target role",
                    mapping.target_predicate()
                )));
            }
        }
        if static_statements
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .len()
            != static_statements.len()
        {
            return Err(ProjectionError::configuration(
                "VoID static dataset statements contain a duplicate",
            ));
        }
        for statement in &static_statements {
            if vocabulary
                .terms()
                .values()
                .any(|iri| iri == statement.predicate())
            {
                return Err(ProjectionError::configuration(format!(
                    "VoID static predicate `{}` collides with a generated target role",
                    statement.predicate()
                )));
            }
        }

        Ok(Self {
            format,
            dataset_iri,
            generated_resource_base_iri,
            header_subject_iri,
            source_roles,
            header_graph,
            alignment_graph,
            metadata_graph,
            data_graphs,
            vocabulary,
            local_datasets,
            external_datasets,
            external_links,
            static_statements,
            limits,
            execution_limits,
        })
    }

    /// Selected registered native RDF syntax.
    pub const fn format(&self) -> NativeRdfFormat {
        self.format
    }

    /// IRI of the described dataset.
    pub fn dataset_iri(&self) -> &str {
        &self.dataset_iri
    }

    /// Caller-owned base used for deterministic partition and linkset IRIs.
    pub fn generated_resource_base_iri(&self) -> &str {
        &self.generated_resource_base_iri
    }

    /// Source subject from which mandatory header values are read.
    pub fn header_subject_iri(&self) -> &str {
        &self.header_subject_iri
    }

    /// Caller-owned source predicate binding.
    pub const fn source_roles(&self) -> &VoidSourceRoles {
        &self.source_roles
    }

    /// Exact graph containing mandatory header values.
    pub const fn header_graph(&self) -> &VoidGraphSelector {
        &self.header_graph
    }

    /// Exact graph interpreted as alignment link records.
    pub const fn alignment_graph(&self) -> &VoidGraphSelector {
        &self.alignment_graph
    }

    /// Exact graph inspected for configured external metadata links.
    pub const fn metadata_graph(&self) -> &VoidGraphSelector {
        &self.metadata_graph
    }

    /// Non-empty exact graph set used for dataset and partition statistics.
    pub fn data_graphs(&self) -> &[VoidGraphSelector] {
        &self.data_graphs
    }

    /// Complete caller-owned target vocabulary.
    pub const fn vocabulary(&self) -> &VoidVocabulary {
        &self.vocabulary
    }

    /// Local resource-prefix registry.
    pub fn local_datasets(&self) -> &[VoidDatasetPrefix] {
        &self.local_datasets
    }

    /// External resource-prefix registry.
    pub fn external_datasets(&self) -> &[VoidDatasetPrefix] {
        &self.external_datasets
    }

    /// Metadata external-link predicate mappings.
    pub fn external_links(&self) -> &[VoidExternalLinkMapping] {
        &self.external_links
    }

    /// Caller-authored statements emitted on the described dataset.
    pub fn static_statements(&self) -> &[VoidStaticStatement] {
        &self.static_statements
    }

    /// Shared deterministic package limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// VoID compute and materialization limits.
    pub const fn execution_limits(&self) -> VoidExecutionLimits {
        self.execution_limits
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVoidConfig {
    format: NativeRdfFormat,
    dataset_iri: String,
    generated_resource_base_iri: String,
    header_subject_iri: String,
    source_roles: VoidSourceRoles,
    header_graph: VoidGraphSelector,
    alignment_graph: VoidGraphSelector,
    metadata_graph: VoidGraphSelector,
    data_graphs: Vec<VoidGraphSelector>,
    vocabulary: VoidVocabulary,
    local_datasets: Vec<VoidDatasetPrefix>,
    external_datasets: Vec<VoidDatasetPrefix>,
    external_links: Vec<VoidExternalLinkMapping>,
    static_statements: Vec<VoidStaticStatement>,
    limits: ProjectionLimits,
    execution_limits: VoidExecutionLimits,
}

impl<'de> Deserialize<'de> for VoidConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawVoidConfig::deserialize(deserializer)?;
        Self::new(
            raw.format,
            raw.dataset_iri,
            raw.generated_resource_base_iri,
            raw.header_subject_iri,
            raw.source_roles,
            raw.header_graph,
            raw.alignment_graph,
            raw.metadata_graph,
            raw.data_graphs,
            raw.vocabulary,
            raw.local_datasets,
            raw.external_datasets,
            raw.external_links,
            raw.static_statements,
            raw.limits,
            raw.execution_limits,
        )
        .map_err(serde::de::Error::custom)
    }
}

fn validate_bound(value: usize, field: &str) -> Result<(), ProjectionError> {
    if value == 0 {
        return Err(ProjectionError::configuration(format!(
            "{field} must be greater than zero"
        )));
    }
    if u32::try_from(value).is_err() {
        return Err(ProjectionError::configuration(format!(
            "{field} exceeds the portable u32 ceiling"
        )));
    }
    Ok(())
}
