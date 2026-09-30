// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::projections::util::absolute_iri;
use crate::projections::util::validate_portable_bound;
use std::collections::{BTreeMap, BTreeSet};

use purrdf_lex::json::{Object, Value};

use crate::native_codecs::NativeRdfFormat;

use super::super::dataset_description::{format_from_json, format_to_json};
use super::super::json_codec::role_map_json;
use super::super::util::validate_role_map;
use super::super::{
    ProjectionDirection, ProjectionError, ProjectionLimits, validate_absolute_iri,
    validate_language_tag,
};
use purrdf_lex::json::record::{DecodeError, FromJson, Record, ToJson};
use purrdf_lex::json_string_enum;

/// Exact source graph selected for one VoID input role.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl FromJson for VoidGraphSelector {
    /// `{"mode": "default-graph"}` or `{"mode": "named-graph", "graph_iri": …}`.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum RawVoidGraphSelector")?;
        let selector = match fields.tag("mode", &["default-graph", "named-graph"])? {
            "default-graph" => Self::DefaultGraph,
            _ => {
                let graph_iri: String = fields.required("graph_iri")?;
                fields.deny_unknown()?;
                return Ok(Self::named(graph_iri)?);
            }
        };
        fields.deny_unknown()?;
        Ok(selector)
    }
}

impl ToJson for VoidGraphSelector {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::DefaultGraph => Object::new().with("mode", "default-graph"),
            Self::NamedGraph { graph_iri } => Object::new()
                .with("mode", "named-graph")
                .with("graph_iri", graph_iri.as_str()),
        })
    }
}

/// Complete caller-owned source predicate binding for VoID extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
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

purrdf_lex::json_record!(impl FromJson for VoidSourceRoles as "struct RawVoidSourceRoles" {
    "rdf_type" => rdf_type: required::<String>,
    "header_version" => header_version: required::<String>,
    "header_abstract" => header_abstract: required::<String>,
} => VoidSourceRoles::new);

purrdf_lex::json_record!(impl ToJson for VoidSourceRoles {
    "rdf_type" => rdf_type,
    "header_version" => header_version,
    "header_abstract" => header_abstract,
});

/// Semantic target role in a caller-owned VoID vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

json_string_enum!(VoidRole {
    RdfType => "rdf-type",
    DatasetClass => "dataset-class",
    LinksetClass => "linkset-class",
    ClassPartitionClass => "class-partition-class",
    PropertyPartitionClass => "property-partition-class",
    Version => "version",
    Abstract => "abstract",
    Subset => "subset",
    Triples => "triples",
    Entities => "entities",
    Classes => "classes",
    Properties => "properties",
    ClassPartition => "class-partition",
    PropertyPartition => "property-partition",
    Class => "class",
    Property => "property",
    DistinctSubjects => "distinct-subjects",
    DistinctObjects => "distinct-objects",
    SubjectsTarget => "subjects-target",
    ObjectsTarget => "objects-target",
    LinkPredicate => "link-predicate",
    XsdNonNegativeInteger => "xsd-non-negative-integer",
});

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidVocabulary(BTreeMap<VoidRole, String>);

impl VoidVocabulary {
    /// Construct and validate one complete target vocabulary.
    ///
    /// # Errors
    ///
    /// Rejects a missing/unknown role, a relative IRI, or two roles bound to one IRI.
    pub fn new(terms: BTreeMap<VoidRole, String>) -> Result<Self, ProjectionError> {
        validate_role_map(&terms, VOID_ROLES, "VoID target vocabulary", |role, iri| {
            validate_absolute_iri(iri, &format!("VoID target role `{role:?}`"))
        })?;
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

role_map_json!(VoidVocabulary);

/// One deterministic IRI-prefix to dataset identity binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        Ok(Self {
            dataset_iri: absolute_iri(dataset_iri, "VoID prefix dataset IRI")?,
            iri_prefix: absolute_iri(iri_prefix, "VoID resource IRI prefix")?,
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

purrdf_lex::json_record!(impl FromJson for VoidDatasetPrefix as "struct RawVoidDatasetPrefix" {
    "dataset_iri" => dataset_iri: required::<String>,
    "iri_prefix" => iri_prefix: required::<String>,
} => VoidDatasetPrefix::new);

purrdf_lex::json_record!(impl ToJson for VoidDatasetPrefix {
    "dataset_iri" => dataset_iri,
    "iri_prefix" => iri_prefix,
});

/// Source-to-target predicate mapping for metadata-graph external IRI links.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        Ok(Self {
            source_predicate: absolute_iri(
                source_predicate,
                "VoID external-link source predicate",
            )?,
            target_predicate: absolute_iri(
                target_predicate,
                "VoID external-link target predicate",
            )?,
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

purrdf_lex::json_record!(impl FromJson for VoidExternalLinkMapping as "struct RawVoidExternalLinkMapping" {
    "source_predicate" => source_predicate: required::<String>,
    "target_predicate" => target_predicate: required::<String>,
} => VoidExternalLinkMapping::new);

purrdf_lex::json_record!(impl ToJson for VoidExternalLinkMapping {
    "source_predicate" => source_predicate,
    "target_predicate" => target_predicate,
});

/// Caller-authored IRI or RDF literal on the described dataset.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// The tag is judged by `validate_language_tag` — the same helper, on the
    /// same profile, that this module tree's [`ProjectionTerm`] builder holds a
    /// tag lifted out of the ingested dataset to. That is the point: the two
    /// feed the SAME projection artifact, and a caller-config tag that no codec
    /// could have produced is a durable lie about a literal's identity once the
    /// artifact is persisted and re-read. It also puts this constructor in line
    /// with its own siblings, [`Self::iri`] and [`Self::typed_literal`], which
    /// already validate rather than merely checking for emptiness.
    ///
    /// Reached by a configuration document as well as by Rust: `VoidStaticValue`'s
    /// JSON reader funnels the `language-literal` arm of a caller's
    /// configuration document straight through here, so a malformed tag in a
    /// config file is refused where it is written.
    ///
    /// [`ProjectionTerm`]: super::super::ProjectionTerm
    ///
    /// # Errors
    ///
    /// Rejects any language tag the grammar does not accept, the empty tag
    /// included.
    pub fn language_literal(
        lexical: impl Into<String>,
        language: impl Into<String>,
        direction: Option<ProjectionDirection>,
    ) -> Result<Self, ProjectionError> {
        let language = language.into();
        validate_language_tag(&language)?;
        Ok(Self::LanguageLiteral {
            lexical: lexical.into(),
            language,
            direction,
        })
    }
}

impl FromJson for VoidStaticValue {
    /// `{"kind": "iri" | "typed-literal" | "language-literal", …}`, each arm
    /// through its validating constructor, and no undeclared member.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum RawVoidStaticValue")?;
        let static_value =
            match fields.tag("kind", &["iri", "typed-literal", "language-literal"])? {
                "iri" => {
                    let value: String = fields.required("value")?;
                    fields.deny_unknown()?;
                    Self::iri(value)
                }
                "typed-literal" => {
                    let lexical: String = fields.required("lexical")?;
                    let datatype: String = fields.required("datatype")?;
                    fields.deny_unknown()?;
                    Self::typed_literal(lexical, datatype)
                }
                _ => {
                    let lexical: String = fields.required("lexical")?;
                    let language: String = fields.required("language")?;
                    let direction = fields.optional("direction")?;
                    fields.deny_unknown()?;
                    Self::language_literal(lexical, language, direction)
                }
            };
        Ok(static_value?)
    }
}

impl ToJson for VoidStaticValue {
    /// `kind` first, then the arm's members; an absent direction is `null`.
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::Iri { value } => Object::new()
                .with("kind", "iri")
                .with("value", value.as_str()),
            Self::TypedLiteral { lexical, datatype } => Object::new()
                .with("kind", "typed-literal")
                .with("lexical", lexical.as_str())
                .with("datatype", datatype.as_str()),
            Self::LanguageLiteral {
                lexical,
                language,
                direction,
            } => Object::new()
                .with("kind", "language-literal")
                .with("lexical", lexical.as_str())
                .with("language", language.as_str())
                .with("direction", direction.to_json()),
        })
    }
}

/// One caller-authored statement whose subject is the described dataset IRI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

purrdf_lex::json_record!(impl FromJson for VoidStaticStatement as "struct RawVoidStaticStatement" {
    "predicate" => predicate: required::<String>,
    "object" => object: required,
} => VoidStaticStatement::new);

purrdf_lex::json_record!(impl ToJson for VoidStaticStatement {
    "predicate" => predicate,
    "object" => object,
});

/// Explicit compute and materialization bounds for VoID generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            validate_portable_bound(value, label)?;
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

purrdf_lex::json_record!(impl FromJson for VoidExecutionLimits as "struct RawVoidExecutionLimits" {
    "max_input_records" => max_input_records: required,
    "max_output_records" => max_output_records: required,
    "max_partitions" => max_partitions: required,
    "max_linksets" => max_linksets: required,
    "max_partition_memberships" => max_partition_memberships: required,
    "max_dataset_prefixes" => max_dataset_prefixes: required,
    "max_external_link_mappings" => max_external_link_mappings: required,
    "max_static_statements" => max_static_statements: required,
} => VoidExecutionLimits::new);

purrdf_lex::json_record!(impl ToJson for VoidExecutionLimits {
    "max_input_records" => max_input_records,
    "max_output_records" => max_output_records,
    "max_partitions" => max_partitions,
    "max_linksets" => max_linksets,
    "max_partition_memberships" => max_partition_memberships,
    "max_dataset_prefixes" => max_dataset_prefixes,
    "max_external_link_mappings" => max_external_link_mappings,
    "max_static_statements" => max_static_statements,
});

/// Complete deterministic VoID dataset-description policy.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    document_base_iri: Option<String>,
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
            document_base_iri: None,
        })
    }

    /// Name the IRI the emitted VoID document is published at.
    ///
    /// This is the document's own base, distinct from
    /// [`dataset_iri`](Self::dataset_iri) (the resource the description is ABOUT) and from
    /// [`generated_resource_base_iri`](Self::generated_resource_base_iri) (the namespace
    /// generated partition and linkset terms are minted under). A projection that emits
    /// Turtle and cannot declare its own base is the same accepted-and-absent asymmetry
    /// the ingress side refuses.
    ///
    /// Caller-owned with no fabricated default: unset, the document declares no base and
    /// writes every IRI absolute, exactly as before.
    ///
    /// # Errors
    ///
    /// Rejects a base that is not an absolute IRI.
    pub fn with_document_base_iri(
        mut self,
        document_base_iri: Option<String>,
    ) -> Result<Self, ProjectionError> {
        if let Some(base) = &document_base_iri {
            validate_absolute_iri(base, "VoID document base IRI")?;
        }
        self.document_base_iri = document_base_iri;
        Ok(self)
    }

    /// The IRI the emitted document is published at, when the caller named one.
    pub fn document_base_iri(&self) -> Option<&str> {
        self.document_base_iri.as_deref()
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

impl FromJson for VoidConfig {
    /// Every member through [`VoidConfig::new`], then `document_base_iri`
    /// (absent or `null` for none) through
    /// [`VoidConfig::with_document_base_iri`].
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "struct RawVoidConfig")?;
        let format = format_from_json(&mut fields, "format")?;
        let dataset_iri: String = fields.required("dataset_iri")?;
        let generated_resource_base_iri: String = fields.required("generated_resource_base_iri")?;
        let header_subject_iri: String = fields.required("header_subject_iri")?;
        let source_roles = fields.required("source_roles")?;
        let header_graph = fields.required("header_graph")?;
        let alignment_graph = fields.required("alignment_graph")?;
        let metadata_graph = fields.required("metadata_graph")?;
        let data_graphs = fields.required("data_graphs")?;
        let vocabulary = fields.required("vocabulary")?;
        let local_datasets = fields.required("local_datasets")?;
        let external_datasets = fields.required("external_datasets")?;
        let external_links = fields.required("external_links")?;
        let static_statements = fields.required("static_statements")?;
        let limits = fields.required("limits")?;
        let execution_limits = fields.required("execution_limits")?;
        let document_base_iri = fields.optional("document_base_iri")?;
        fields.deny_unknown()?;
        Ok(Self::new(
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
        )?
        .with_document_base_iri(document_base_iri)?)
    }
}

impl ToJson for VoidConfig {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("format", format_to_json(self.format))
                .with("dataset_iri", self.dataset_iri.as_str())
                .with(
                    "generated_resource_base_iri",
                    self.generated_resource_base_iri.as_str(),
                )
                .with("header_subject_iri", self.header_subject_iri.as_str())
                .with("source_roles", self.source_roles.to_json())
                .with("header_graph", self.header_graph.to_json())
                .with("alignment_graph", self.alignment_graph.to_json())
                .with("metadata_graph", self.metadata_graph.to_json())
                .with("data_graphs", self.data_graphs.to_json())
                .with("vocabulary", self.vocabulary.to_json())
                .with("local_datasets", self.local_datasets.to_json())
                .with("external_datasets", self.external_datasets.to_json())
                .with("external_links", self.external_links.to_json())
                .with("static_statements", self.static_statements.to_json())
                .with("limits", self.limits.to_json())
                .with("execution_limits", self.execution_limits.to_json())
                .with("document_base_iri", self.document_base_iri.to_json()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::json::record::from_slice;

    /// Both halves of `VoidStaticValue::language_literal`'s gate, through BOTH
    /// of its doors — the Rust constructor and the JSON reader a caller's config
    /// document arrives by.
    ///
    /// The accept list is the load-bearing half. A VoID projection is published
    /// alongside the dataset it describes, so a static statement must be able to
    /// carry any tag that dataset's literals carry: `x-purrdf-afrikaans` and
    /// `x-gmeow-english` are tags this workspace's own artifacts hold, and
    /// `en-fr-jura` / `fr-be-fbcl` are tags approved W3C corpora hold. Refusing
    /// one of those would make a describable dataset undescribable.
    #[test]
    fn a_static_language_literal_is_held_to_the_codec_language_grammar() {
        for tag in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
            "abcdefgh",
            "en-x-cantbethislong",
        ] {
            VoidStaticValue::language_literal("v", tag, None)
                .unwrap_or_else(|e| panic!("{tag:?} must still configure: {}", e.message()));

            let json = format!(r#"{{"kind":"language-literal","lexical":"v","language":"{tag}"}}"#);
            from_slice::<VoidStaticValue>(json.as_bytes())
                .unwrap_or_else(|e| panic!("{tag:?} must still read: {e}"));
        }

        for tag in [
            "en us",
            "1",
            "9-9",
            "123-456",
            "en-",
            "-",
            "!!!",
            "abcdefghi",
            "",
        ] {
            let error = VoidStaticValue::language_literal("v", tag, None)
                .expect_err("a non-tag must not become a configured literal");
            assert!(
                error.message().contains("invalid language tag"),
                "{tag:?}: {}",
                error.message()
            );

            let json = format!(r#"{{"kind":"language-literal","lexical":"v","language":"{tag}"}}"#);
            let error = from_slice::<VoidStaticValue>(json.as_bytes())
                .expect_err("the JSON door must refuse exactly what the Rust door refuses");
            assert!(
                error.to_string().contains("invalid language tag"),
                "{tag:?}: {error}"
            );
        }
    }
}
