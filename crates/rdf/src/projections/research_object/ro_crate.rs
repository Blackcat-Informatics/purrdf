// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED, LOSS_RESEARCH_LOCAL_ID_RESOLVED,
    LOSS_RESEARCH_ORDER_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
};
use purrdf_core::{DatasetView, LossLedger, research_object_to_rdf_loss_ledger};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::super::{ProjectionError, ProjectionPackage, validate_absolute_iri};
use super::json::{
    ResearchObjectPackageProjection, ResearchObjectReadOutcome, canonical_json, ensure_sound,
    json_pointer, normalize_lifted_jsonld, parse_strict_json, record_loss, require_artifact,
};
use super::{
    OfflineJsonLdContext, ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset,
    ResearchField, ResearchObjectConfig, ResearchObjectModel, ResearchRecordSet, ResearchResource,
    ResearchText, ResearchValue, lift_research_object, project_research_object,
};

/// Closed RO-Crate projection profile identifier.
pub const RO_CRATE_PROFILE: &str = "ro-crate-1.3";
/// Sole artifact path in the canonical RO-Crate package.
pub const RO_CRATE_ARTIFACT: &str = "ro-crate-metadata.json";

/// Semantic compact term required by the RO-Crate 1.3 adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoCrateRole {
    /// Root dataset class.
    RootDatasetClass,
    /// Metadata descriptor class.
    MetadataDescriptorClass,
    /// File/data entity class.
    FileClass,
    /// Agent/contextual entity class.
    AgentClass,
    /// Provenance action class.
    ActivityClass,
    /// Structured record-set class.
    RecordSetClass,
    /// Record-set field class.
    FieldClass,
    /// Name property.
    Name,
    /// Description property.
    Description,
    /// Identifier property.
    Identifier,
    /// Version property.
    Version,
    /// Publication date property.
    DatePublished,
    /// Modification date property.
    DateModified,
    /// Landing-page URL property.
    Url,
    /// Keyword property.
    Keywords,
    /// License property.
    License,
    /// Creator relation.
    Creator,
    /// Publisher relation.
    Publisher,
    /// Root-to-data-entity relation.
    HasPart,
    /// Root-to-contextual-entity relation.
    Mentions,
    /// Metadata descriptor profile relation.
    ConformsTo,
    /// Metadata descriptor root relation.
    About,
    /// Resource package path.
    Path,
    /// Resource content URL.
    ContentUrl,
    /// Resource media type.
    EncodingFormat,
    /// Resource format identifier.
    Format,
    /// Resource byte size.
    ContentSize,
    /// Resource checksum property.
    Checksum,
    /// Checksum algorithm property.
    ChecksumAlgorithm,
    /// Checksum lexical value property.
    ChecksumValue,
    /// Inline payload property, consumed only for loss accounting.
    InlineContent,
    /// Record-set field relation.
    Field,
    /// Field datatype property.
    DataType,
    /// Inline record values.
    Records,
    /// Activity instrument relation.
    Instrument,
    /// Activity agent relation.
    Agent,
    /// Activity input relation.
    Object,
    /// Activity result relation.
    Result,
    /// Activity completion time.
    EndTime,
    /// Activity workflow relation.
    Workflow,
}

/// Every mandatory RO-Crate role in deterministic configuration order.
pub const RO_CRATE_ROLES: &[RoCrateRole] = &[
    RoCrateRole::RootDatasetClass,
    RoCrateRole::MetadataDescriptorClass,
    RoCrateRole::FileClass,
    RoCrateRole::AgentClass,
    RoCrateRole::ActivityClass,
    RoCrateRole::RecordSetClass,
    RoCrateRole::FieldClass,
    RoCrateRole::Name,
    RoCrateRole::Description,
    RoCrateRole::Identifier,
    RoCrateRole::Version,
    RoCrateRole::DatePublished,
    RoCrateRole::DateModified,
    RoCrateRole::Url,
    RoCrateRole::Keywords,
    RoCrateRole::License,
    RoCrateRole::Creator,
    RoCrateRole::Publisher,
    RoCrateRole::HasPart,
    RoCrateRole::Mentions,
    RoCrateRole::ConformsTo,
    RoCrateRole::About,
    RoCrateRole::Path,
    RoCrateRole::ContentUrl,
    RoCrateRole::EncodingFormat,
    RoCrateRole::Format,
    RoCrateRole::ContentSize,
    RoCrateRole::Checksum,
    RoCrateRole::ChecksumAlgorithm,
    RoCrateRole::ChecksumValue,
    RoCrateRole::InlineContent,
    RoCrateRole::Field,
    RoCrateRole::DataType,
    RoCrateRole::Records,
    RoCrateRole::Instrument,
    RoCrateRole::Agent,
    RoCrateRole::Object,
    RoCrateRole::Result,
    RoCrateRole::EndTime,
    RoCrateRole::Workflow,
];

/// Complete caller-owned compact-term binding for RO-Crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RoCrateVocabulary(BTreeMap<RoCrateRole, String>);

impl RoCrateVocabulary {
    /// Validate a complete, collision-free compact-term map.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra roles, JSON-LD keywords, whitespace-bearing terms,
    /// and ambiguous duplicate bindings.
    pub fn new(terms: BTreeMap<RoCrateRole, String>) -> Result<Self, ProjectionError> {
        for role in RO_CRATE_ROLES {
            let term = terms.get(role).ok_or_else(|| {
                ProjectionError::configuration(format!(
                    "RO-Crate vocabulary is missing role `{role:?}`"
                ))
            })?;
            if term.is_empty() || term.starts_with('@') || term.chars().any(char::is_whitespace) {
                return Err(ProjectionError::configuration(format!(
                    "RO-Crate role `{role:?}` has invalid compact term `{term}`"
                )));
            }
        }
        if terms.len() != RO_CRATE_ROLES.len() {
            return Err(ProjectionError::configuration(
                "RO-Crate vocabulary contains an unsupported role",
            ));
        }
        let mut inverse = BTreeMap::<&str, RoCrateRole>::new();
        for (&role, term) in &terms {
            if let Some(previous) = inverse.insert(term, role) {
                return Err(ProjectionError::configuration(format!(
                    "RO-Crate roles `{previous:?}` and `{role:?}` both bind `{term}`"
                )));
            }
        }
        Ok(Self(terms))
    }

    /// Compact term bound to one RO-Crate semantic role.
    pub fn term(&self, role: RoCrateRole) -> &str {
        self.0
            .get(&role)
            .expect("validated RO-Crate role map is complete")
    }

    /// Deterministically ordered compact-term map.
    pub const fn terms(&self) -> &BTreeMap<RoCrateRole, String> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RoCrateVocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let terms = BTreeMap::<RoCrateRole, String>::deserialize(deserializer)?;
        Self::new(terms).map_err(serde::de::Error::custom)
    }
}

/// Mandatory caller-owned configuration for RO-Crate 1.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoCrateConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: RoCrateVocabulary,
    profile_iri: String,
    metadata_descriptor_id: String,
    root_dataset_id: String,
}

impl RoCrateConfig {
    /// Construct and cross-validate an RO-Crate configuration.
    ///
    /// # Errors
    ///
    /// Rejects a non-absolute profile, unsafe/colliding native identities, or a
    /// vocabulary term without a caller-provided offline expansion.
    pub fn new(
        common: ResearchObjectConfig,
        context: OfflineJsonLdContext,
        vocabulary: RoCrateVocabulary,
        profile_iri: impl Into<String>,
        metadata_descriptor_id: impl Into<String>,
        root_dataset_id: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let profile_iri = profile_iri.into();
        let metadata_descriptor_id = metadata_descriptor_id.into();
        let root_dataset_id = root_dataset_id.into();
        validate_absolute_iri(&profile_iri, "RO-Crate profile identity")?;
        validate_native_id(&metadata_descriptor_id, false)?;
        validate_native_id(&root_dataset_id, true)?;
        if metadata_descriptor_id == root_dataset_id {
            return Err(ProjectionError::configuration(
                "RO-Crate metadata descriptor and root dataset identities must differ",
            ));
        }
        for (&role, term) in vocabulary.terms() {
            if context.expand(term).is_none() {
                return Err(ProjectionError::configuration(format!(
                    "RO-Crate term `{term}` for role `{role:?}` has no offline expansion"
                )));
            }
        }
        Ok(Self {
            common,
            context,
            vocabulary,
            profile_iri,
            metadata_descriptor_id,
            root_dataset_id,
        })
    }

    /// Shared RDF vocabulary, identity, and limits.
    pub const fn common(&self) -> &ResearchObjectConfig {
        &self.common
    }
    /// Exact emitted context and offline expansion table.
    pub const fn context(&self) -> &OfflineJsonLdContext {
        &self.context
    }
    /// Caller-owned RO-Crate compact terms.
    pub const fn vocabulary(&self) -> &RoCrateVocabulary {
        &self.vocabulary
    }
    /// Absolute RO-Crate profile identity.
    pub fn profile_iri(&self) -> &str {
        &self.profile_iri
    }
    /// Native metadata descriptor identifier.
    pub fn metadata_descriptor_id(&self) -> &str {
        &self.metadata_descriptor_id
    }
    /// Native root dataset identifier (commonly `./`).
    pub fn root_dataset_id(&self) -> &str {
        &self.root_dataset_id
    }

    fn native_id(&self, iri: &str) -> String {
        if iri == self.common.identity().dataset_iri() {
            return self.root_dataset_id.clone();
        }
        iri.strip_prefix(self.common.identity().entity_base_iri())
            .filter(|relative| validate_native_id(relative, false).is_ok())
            .map_or_else(|| iri.to_owned(), str::to_owned)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRoCrateConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: RoCrateVocabulary,
    profile_iri: String,
    metadata_descriptor_id: String,
    root_dataset_id: String,
}

impl<'de> Deserialize<'de> for RoCrateConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawRoCrateConfig::deserialize(deserializer)?;
        Self::new(
            raw.common,
            raw.context,
            raw.vocabulary,
            raw.profile_iri,
            raw.metadata_descriptor_id,
            raw.root_dataset_id,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Project caller-vocabulary RDF 1.2 into canonical RO-Crate 1.3 JSON-LD.
///
/// # Errors
///
/// Returns a typed configuration, RDF interpretation, resource-limit, or JSON
/// encoding failure with every representational loss in the outcome.
pub fn project_ro_crate<D: DatasetView>(
    view: &D,
    config: &RoCrateConfig,
) -> Result<ResearchObjectPackageProjection, ProjectionError> {
    let projection = project_research_object(view, RO_CRATE_PROFILE, config.common())?;
    let document = encode_document(&projection.model, config)?;
    ensure_sound(&projection.loss_ledger, "rdf-1.2-dataset", RO_CRATE_PROFILE)?;
    let bytes = canonical_json(&document, config.common().limits(), "RO-Crate 1.3 JSON-LD")?;
    let package =
        ProjectionPackage::from_artifacts(config.common().limits(), [(RO_CRATE_ARTIFACT, bytes)])?;
    Ok(ResearchObjectPackageProjection {
        package,
        model: projection.model,
        loss_ledger: projection.loss_ledger,
    })
}

/// Read a strict RO-Crate 1.3 package and lift caller-vocabulary RDF 1.2.
///
/// # Errors
///
/// Rejects unexpected artifacts, duplicate JSON members, descriptor/context
/// drift, duplicate/dangling graph identities, malformed entity shapes, unsafe
/// local IDs, or configured resource-limit excesses.
pub fn read_ro_crate(
    package: &ProjectionPackage,
    config: &RoCrateConfig,
) -> Result<ResearchObjectReadOutcome, ProjectionError> {
    let bytes = require_artifact(package, RO_CRATE_ARTIFACT, config.common())?;
    let value = parse_strict_json(
        bytes,
        config.common(),
        "RO-Crate 1.3 JSON-LD",
        RO_CRATE_ARTIFACT,
    )?;
    let contract = research_object_to_rdf_loss_ledger(RO_CRATE_PROFILE);
    let mut ledger = LossLedger::new();
    let model = decode_document(value, config, &contract, &mut ledger)?
        .normalize(config.common().policy())?;
    ensure_sound(&ledger, RO_CRATE_PROFILE, "rdf-1.2-dataset")?;
    let dataset = lift_research_object(model.clone(), config.common())?;
    let dataset = normalize_lifted_jsonld(&dataset)?;
    Ok(ResearchObjectReadOutcome {
        dataset,
        model,
        loss_ledger: ledger,
    })
}

fn validate_native_id(value: &str, allow_dot_root: bool) -> Result<(), ProjectionError> {
    if validate_absolute_iri(value, "RO-Crate native identity").is_ok() {
        return Ok(());
    }
    if allow_dot_root && value == "./" {
        return Ok(());
    }
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(['?', '#'])
        || value
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
        || value.contains("://")
    {
        return Err(ProjectionError::configuration(format!(
            "unsafe RO-Crate native identity `{value}`"
        )));
    }
    Ok(())
}

fn encode_document(
    model: &ResearchObjectModel,
    config: &RoCrateConfig,
) -> Result<Value, ProjectionError> {
    let terms = config.vocabulary();
    let mut descriptor = typed_object(
        config.metadata_descriptor_id(),
        terms.term(RoCrateRole::MetadataDescriptorClass),
    );
    descriptor.insert(
        terms.term(RoCrateRole::ConformsTo).to_owned(),
        id_object(config.profile_iri()),
    );
    descriptor.insert(
        terms.term(RoCrateRole::About).to_owned(),
        id_object(config.root_dataset_id()),
    );

    let mut graph = vec![Value::Object(descriptor), encode_root(model, config)];
    graph.extend(model.agents.iter().map(|agent| encode_agent(agent, config)));
    graph.extend(
        model
            .resources
            .iter()
            .map(|resource| encode_resource(resource, config)),
    );
    graph.extend(
        model
            .activities
            .iter()
            .map(|activity| encode_activity(activity, config)),
    );
    for record_set in &model.record_sets {
        graph.push(encode_record_set(record_set, config));
        graph.extend(
            record_set
                .fields
                .iter()
                .map(|field| encode_field(field, config)),
        );
    }
    graph.sort_by(|left, right| graph_id(left).cmp(graph_id(right)));

    Ok(Value::Object(Map::from_iter([
        ("@context".to_owned(), config.context().value().clone()),
        ("@graph".to_owned(), Value::Array(graph)),
    ])))
}

fn encode_root(model: &ResearchObjectModel, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let dataset = &model.dataset;
    let mut object = typed_object(
        config.root_dataset_id(),
        terms.term(RoCrateRole::RootDatasetClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&dataset.titles, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Description),
        encode_texts(&dataset.descriptions, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Identifier),
        encode_values(&dataset.identifiers, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Version),
        encode_texts(&dataset.versions, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::DatePublished),
        encode_texts(&dataset.issued, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::DateModified),
        encode_texts(&dataset.modified, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Url),
        encode_values(&dataset.landing_pages, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Keywords),
        encode_texts(&dataset.keywords, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::License),
        encode_values(&dataset.licenses, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Creator),
        dataset
            .creators
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Publisher),
        dataset
            .publishers
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::HasPart),
        dataset
            .resources
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    let mut mentions = dataset.activities.clone();
    mentions.extend(dataset.record_sets.iter().cloned());
    mentions.sort();
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Mentions),
        mentions
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    Value::Object(object)
}

fn encode_agent(agent: &ResearchAgent, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(
        &config.native_id(&agent.id),
        terms.term(RoCrateRole::AgentClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&agent.names, config),
    );
    Value::Object(object)
}

fn encode_resource(resource: &ResearchResource, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(
        &config.native_id(&resource.id),
        terms.term(RoCrateRole::FileClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&resource.names, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Description),
        encode_texts(&resource.descriptions, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Path),
        resource.paths.iter().cloned().map(Value::String).collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::ContentUrl),
        encode_values(&resource.urls, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::EncodingFormat),
        encode_texts(&resource.media_types, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Format),
        encode_values(&resource.formats, config),
    );
    if let Some(byte_size) = resource.byte_size {
        object.insert(
            terms.term(RoCrateRole::ContentSize).to_owned(),
            Value::Number(byte_size.into()),
        );
    }
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Checksum),
        resource
            .checksums
            .iter()
            .map(|checksum| encode_checksum(checksum, config))
            .collect(),
    );
    Value::Object(object)
}

fn encode_checksum(checksum: &ResearchChecksum, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    Value::Object(Map::from_iter([
        (
            terms.term(RoCrateRole::ChecksumAlgorithm).to_owned(),
            encode_value(&checksum.algorithm, config),
        ),
        (
            terms.term(RoCrateRole::ChecksumValue).to_owned(),
            encode_text(&checksum.value, config),
        ),
    ]))
}

fn encode_activity(activity: &ResearchActivity, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(
        &config.native_id(&activity.id),
        terms.term(RoCrateRole::ActivityClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&activity.names, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Instrument),
        encode_values(&activity.instruments, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Agent),
        activity
            .actors
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Object),
        activity
            .objects
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Result),
        activity
            .results
            .iter()
            .map(|id| id_object(&config.native_id(id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::EndTime),
        encode_texts(&activity.end_times, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Workflow),
        encode_values(&activity.workflows, config),
    );
    Value::Object(object)
}

fn encode_record_set(record_set: &ResearchRecordSet, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(
        &config.native_id(&record_set.id),
        terms.term(RoCrateRole::RecordSetClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&record_set.names, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Description),
        encode_texts(&record_set.descriptions, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Field),
        record_set
            .fields
            .iter()
            .map(|field| id_object(&config.native_id(&field.id)))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Records),
        record_set.rows.clone(),
    );
    Value::Object(object)
}

fn encode_field(field: &ResearchField, config: &RoCrateConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(
        &config.native_id(&field.id),
        terms.term(RoCrateRole::FieldClass),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::Name),
        encode_texts(&field.names, config),
    );
    insert_values(
        &mut object,
        terms.term(RoCrateRole::DataType),
        encode_values(&field.data_types, config),
    );
    Value::Object(object)
}

fn typed_object(id: &str, class: &str) -> Map<String, Value> {
    Map::from_iter([
        ("@id".to_owned(), Value::String(id.to_owned())),
        ("@type".to_owned(), Value::String(class.to_owned())),
    ])
}

fn id_object(id: &str) -> Value {
    Value::Object(Map::from_iter([(
        "@id".to_owned(),
        Value::String(id.to_owned()),
    )]))
}

fn encode_texts(values: &[ResearchText], config: &RoCrateConfig) -> Vec<Value> {
    values
        .iter()
        .map(|value| encode_text(value, config))
        .collect()
}

fn encode_values(values: &[ResearchValue], config: &RoCrateConfig) -> Vec<Value> {
    values
        .iter()
        .map(|value| encode_value(value, config))
        .collect()
}

fn encode_value(value: &ResearchValue, config: &RoCrateConfig) -> Value {
    match value {
        ResearchValue::Iri { value } => id_object(value),
        ResearchValue::Text(value) => encode_text(value, config),
    }
}

fn encode_text(value: &ResearchText, config: &RoCrateConfig) -> Value {
    let xsd_string = config.common().roles().iri(super::ResearchRole::XsdString);
    if value.datatype == xsd_string && value.language.is_none() && value.direction.is_none() {
        return Value::String(value.value.clone());
    }
    let mut object = Map::from_iter([("@value".to_owned(), Value::String(value.value.clone()))]);
    if let Some(language) = &value.language {
        object.insert("@language".to_owned(), Value::String(language.clone()));
    } else {
        object.insert("@type".to_owned(), Value::String(value.datatype.clone()));
    }
    if let Some(direction) = value.direction {
        object.insert(
            "@direction".to_owned(),
            Value::String(
                match direction {
                    super::super::ProjectionDirection::Ltr => "ltr",
                    super::super::ProjectionDirection::Rtl => "rtl",
                }
                .to_owned(),
            ),
        );
    }
    Value::Object(object)
}

fn insert_values(object: &mut Map<String, Value>, term: &str, values: Vec<Value>) {
    if !values.is_empty() {
        object.insert(term.to_owned(), Value::Array(values));
    }
}

fn graph_id(value: &Value) -> &str {
    value
        .get("@id")
        .and_then(Value::as_str)
        .expect("RO-Crate writer constructs every graph node with @id")
}

struct RawNode {
    native_id: String,
    object: Map<String, Value>,
    pointer: String,
}

struct RoDecoder<'a> {
    config: &'a RoCrateConfig,
    contract: &'a LossLedger,
    ledger: &'a mut LossLedger,
    known_ids: BTreeSet<String>,
}

fn decode_document(
    value: Value,
    config: &RoCrateConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<ResearchObjectModel, ProjectionError> {
    RoDecoder {
        config,
        contract,
        ledger,
        known_ids: BTreeSet::new(),
    }
    .decode(value)
}

impl RoDecoder<'_> {
    fn decode(mut self, value: Value) -> Result<ResearchObjectModel, ProjectionError> {
        let Value::Object(mut document) = value else {
            return Err(
                ProjectionError::syntax("RO-Crate document root must be a JSON object")
                    .at_path(RO_CRATE_ARTIFACT),
            );
        };
        let context = document.remove("@context").ok_or_else(|| {
            ProjectionError::integrity("RO-Crate document is missing @context")
                .at_path(RO_CRATE_ARTIFACT)
        })?;
        if context != *self.config.context().value() {
            return Err(ProjectionError::integrity(
                "RO-Crate @context does not exactly match caller configuration",
            )
            .at_path(RO_CRATE_ARTIFACT));
        }
        let graph = document.remove("@graph").ok_or_else(|| {
            ProjectionError::integrity("RO-Crate document is missing @graph")
                .at_path(RO_CRATE_ARTIFACT)
        })?;
        let Value::Array(graph) = graph else {
            return Err(
                ProjectionError::integrity("RO-Crate @graph must be an array")
                    .at_path(RO_CRATE_ARTIFACT),
            );
        };
        if graph.len() > 1 {
            self.loss(LOSS_RESEARCH_ORDER_DROPPED, "/@graph");
        }
        self.record_unknowns(&document, "");

        let mut nodes = BTreeMap::<String, RawNode>::new();
        for (index, value) in graph.into_iter().enumerate() {
            let pointer = format!("/@graph/{index}");
            let Value::Object(mut object) = value else {
                self.unsupported(&pointer);
                continue;
            };
            let Some(Value::String(native_id)) = object.remove("@id") else {
                return Err(ProjectionError::integrity(
                    "every RO-Crate graph entity requires a string @id",
                )
                .at_path(RO_CRATE_ARTIFACT));
            };
            validate_native_id(&native_id, native_id == self.config.root_dataset_id()).map_err(
                |error| ProjectionError::integrity(error.message()).at_path(RO_CRATE_ARTIFACT),
            )?;
            if nodes
                .insert(
                    native_id.clone(),
                    RawNode {
                        native_id: native_id.clone(),
                        object,
                        pointer,
                    },
                )
                .is_some()
            {
                return Err(ProjectionError::integrity(format!(
                    "duplicate RO-Crate graph identity `{native_id}`"
                ))
                .at_path(RO_CRATE_ARTIFACT));
            }
        }
        self.known_ids = nodes.keys().cloned().collect();

        let descriptor = nodes
            .remove(self.config.metadata_descriptor_id())
            .ok_or_else(|| {
                ProjectionError::integrity("RO-Crate metadata descriptor is missing")
                    .at_path(RO_CRATE_ARTIFACT)
            })?;
        self.validate_descriptor(descriptor)?;
        let root = nodes.remove(self.config.root_dataset_id()).ok_or_else(|| {
            ProjectionError::integrity("RO-Crate root dataset entity is missing")
                .at_path(RO_CRATE_ARTIFACT)
        })?;

        let mut agents = Vec::new();
        let mut resources = Vec::new();
        let mut activities = Vec::new();
        let mut raw_record_sets = Vec::new();
        let mut fields = BTreeMap::<String, ResearchField>::new();
        let mut kind_by_native = BTreeMap::<String, RoCrateRole>::new();

        for (_, mut node) in nodes {
            let class = self.take_type(&mut node.object, &node.pointer)?;
            let role = self.class_role(&class);
            let Some(role) = role else {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &node.pointer);
                continue;
            };
            kind_by_native.insert(node.native_id.clone(), role);
            match role {
                RoCrateRole::AgentClass => agents.push(self.decode_agent(node)?),
                RoCrateRole::FileClass => resources.push(self.decode_resource(node)?),
                RoCrateRole::ActivityClass => activities.push(self.decode_activity(node)?),
                RoCrateRole::RecordSetClass => raw_record_sets.push(node),
                RoCrateRole::FieldClass => {
                    let native_id = node.native_id.clone();
                    fields.insert(native_id, self.decode_field(node)?);
                }
                _ => unreachable!("class_role returns only entity-class roles"),
            }
        }

        let mut record_sets = Vec::new();
        for node in raw_record_sets {
            record_sets.push(self.decode_record_set(node, &mut fields)?);
        }
        for native_id in fields.keys() {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &format!("/@graph/@id={native_id}"),
            );
        }

        let dataset = self.decode_root(root, &kind_by_native)?;
        Ok(ResearchObjectModel {
            dataset,
            agents,
            resources,
            activities,
            record_sets,
        })
    }

    fn validate_descriptor(&mut self, mut node: RawNode) -> Result<(), ProjectionError> {
        self.require_type(
            &mut node.object,
            &node.pointer,
            self.config
                .vocabulary()
                .term(RoCrateRole::MetadataDescriptorClass),
        )?;
        let profile = self.take_single_native_ref(
            &mut node.object,
            RoCrateRole::ConformsTo,
            &node.pointer,
            false,
        )?;
        if profile != self.config.profile_iri() {
            return Err(ProjectionError::integrity(format!(
                "RO-Crate descriptor conformsTo `{profile}` does not match configured profile `{}`",
                self.config.profile_iri()
            ))
            .at_path(RO_CRATE_ARTIFACT));
        }
        let about =
            self.take_single_native_ref(&mut node.object, RoCrateRole::About, &node.pointer, true)?;
        if about != self.config.root_dataset_id() {
            return Err(ProjectionError::integrity(
                "RO-Crate descriptor about relation does not identify the configured root",
            )
            .at_path(RO_CRATE_ARTIFACT));
        }
        self.record_unknowns(&node.object, &node.pointer);
        Ok(())
    }

    fn decode_root(
        &mut self,
        mut node: RawNode,
        kind_by_native: &BTreeMap<String, RoCrateRole>,
    ) -> Result<ResearchDataset, ProjectionError> {
        self.require_type(
            &mut node.object,
            &node.pointer,
            self.config.vocabulary().term(RoCrateRole::RootDatasetClass),
        )?;
        let titles = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, RoCrateRole::Description, &node.pointer)?;
        let identifiers =
            self.take_values(&mut node.object, RoCrateRole::Identifier, &node.pointer)?;
        let versions = self.take_texts(&mut node.object, RoCrateRole::Version, &node.pointer)?;
        let issued =
            self.take_texts(&mut node.object, RoCrateRole::DatePublished, &node.pointer)?;
        let modified =
            self.take_texts(&mut node.object, RoCrateRole::DateModified, &node.pointer)?;
        let landing_pages = self.take_values(&mut node.object, RoCrateRole::Url, &node.pointer)?;
        let keywords = self.take_texts(&mut node.object, RoCrateRole::Keywords, &node.pointer)?;
        let licenses = self.take_values(&mut node.object, RoCrateRole::License, &node.pointer)?;
        let creators =
            self.take_entity_refs(&mut node.object, RoCrateRole::Creator, &node.pointer)?;
        let publishers =
            self.take_entity_refs(&mut node.object, RoCrateRole::Publisher, &node.pointer)?;
        let resources =
            self.take_entity_refs(&mut node.object, RoCrateRole::HasPart, &node.pointer)?;
        let mention_native =
            self.take_native_refs(&mut node.object, RoCrateRole::Mentions, &node.pointer, true)?;
        let mut activities = Vec::new();
        let mut record_sets = Vec::new();
        for native_id in mention_native {
            let resolved = self.resolve_model_id(&native_id, &node.pointer)?;
            match kind_by_native.get(&native_id) {
                Some(RoCrateRole::ActivityClass) => activities.push(resolved),
                Some(RoCrateRole::RecordSetClass) => record_sets.push(resolved),
                _ => {
                    return Err(ProjectionError::integrity(format!(
                        "RO-Crate root mentions `{native_id}` with an unsupported entity class"
                    ))
                    .at_path(RO_CRATE_ARTIFACT));
                }
            }
        }
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchDataset {
            id: self.config.common().identity().dataset_iri().to_owned(),
            titles,
            descriptions,
            identifiers,
            versions,
            issued,
            modified,
            landing_pages,
            keywords,
            licenses,
            creators,
            publishers,
            resources,
            activities,
            record_sets,
        })
    }

    fn class_role(&self, class: &str) -> Option<RoCrateRole> {
        [
            RoCrateRole::AgentClass,
            RoCrateRole::FileClass,
            RoCrateRole::ActivityClass,
            RoCrateRole::RecordSetClass,
            RoCrateRole::FieldClass,
        ]
        .into_iter()
        .find(|role| self.config.vocabulary().term(*role) == class)
    }

    fn decode_agent(&mut self, mut node: RawNode) -> Result<ResearchAgent, ProjectionError> {
        let id = self.resolve_model_id(&node.native_id, &node.pointer)?;
        let names = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchAgent { id, names })
    }

    fn decode_resource(&mut self, mut node: RawNode) -> Result<ResearchResource, ProjectionError> {
        let id = self.resolve_model_id(&node.native_id, &node.pointer)?;
        let names = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, RoCrateRole::Description, &node.pointer)?;
        let paths = self.take_paths(&mut node.object, &node.pointer)?;
        let urls = self.take_values(&mut node.object, RoCrateRole::ContentUrl, &node.pointer)?;
        let media_types =
            self.take_texts(&mut node.object, RoCrateRole::EncodingFormat, &node.pointer)?;
        let formats = self.take_values(&mut node.object, RoCrateRole::Format, &node.pointer)?;
        let byte_size = self.take_byte_size(&mut node.object, &node.pointer);
        let checksums = self.take_checksums(&mut node.object, &node.pointer)?;
        let inline_term = self.config.vocabulary().term(RoCrateRole::InlineContent);
        if node.object.remove(inline_term).is_some() {
            self.loss(
                LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED,
                &json_pointer(&node.pointer, inline_term),
            );
        }
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchResource {
            id,
            names,
            descriptions,
            paths,
            urls,
            media_types,
            formats,
            byte_size,
            checksums,
        })
    }

    fn decode_activity(&mut self, mut node: RawNode) -> Result<ResearchActivity, ProjectionError> {
        let id = self.resolve_model_id(&node.native_id, &node.pointer)?;
        let names = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        let instruments =
            self.take_values(&mut node.object, RoCrateRole::Instrument, &node.pointer)?;
        let actors = self.take_entity_refs(&mut node.object, RoCrateRole::Agent, &node.pointer)?;
        let objects =
            self.take_entity_refs(&mut node.object, RoCrateRole::Object, &node.pointer)?;
        let results =
            self.take_entity_refs(&mut node.object, RoCrateRole::Result, &node.pointer)?;
        let end_times = self.take_texts(&mut node.object, RoCrateRole::EndTime, &node.pointer)?;
        let workflows = self.take_values(&mut node.object, RoCrateRole::Workflow, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchActivity {
            id,
            names,
            instruments,
            actors,
            objects,
            results,
            end_times,
            workflows,
        })
    }

    fn decode_field(&mut self, mut node: RawNode) -> Result<ResearchField, ProjectionError> {
        let id = self.resolve_model_id(&node.native_id, &node.pointer)?;
        let names = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        let data_types =
            self.take_values(&mut node.object, RoCrateRole::DataType, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchField {
            id,
            names,
            data_types,
        })
    }

    fn decode_record_set(
        &mut self,
        mut node: RawNode,
        fields: &mut BTreeMap<String, ResearchField>,
    ) -> Result<ResearchRecordSet, ProjectionError> {
        let id = self.resolve_model_id(&node.native_id, &node.pointer)?;
        let names = self.take_texts(&mut node.object, RoCrateRole::Name, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, RoCrateRole::Description, &node.pointer)?;
        let field_ids =
            self.take_native_refs(&mut node.object, RoCrateRole::Field, &node.pointer, true)?;
        let mut linked_fields = Vec::new();
        for field_id in field_ids {
            let field = fields.remove(&field_id).ok_or_else(|| {
                ProjectionError::integrity(format!(
                    "RO-Crate record set references non-field entity `{field_id}`"
                ))
                .at_path(RO_CRATE_ARTIFACT)
            })?;
            linked_fields.push(field);
        }
        let rows = self.take_items(&mut node.object, RoCrateRole::Records, &node.pointer);
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchRecordSet {
            id,
            names,
            descriptions,
            fields: linked_fields,
            rows,
        })
    }

    fn take_items(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
    ) -> Vec<Value> {
        let term = self.config.vocabulary().term(role).to_owned();
        let Some(value) = object.remove(&term) else {
            return Vec::new();
        };
        match value {
            Value::Array(values) => {
                if values.len() > 1 {
                    self.loss(LOSS_RESEARCH_ORDER_DROPPED, &json_pointer(parent, &term));
                }
                values
            }
            value => vec![value],
        }
    }

    fn take_texts(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
    ) -> Result<Vec<ResearchText>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut texts = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            if let Some(text) = self.parse_text(value, &pointer)? {
                texts.push(text);
            }
        }
        Ok(texts)
    }

    fn parse_text(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchText>, ProjectionError> {
        let xsd_string = self
            .config
            .common()
            .roles()
            .iri(super::ResearchRole::XsdString)
            .to_owned();
        match value {
            Value::String(value) => ResearchText::plain(value, xsd_string).map(Some),
            Value::Object(mut object) => {
                let Some(Value::String(value)) = object.remove("@value") else {
                    self.unsupported(pointer);
                    return Ok(None);
                };
                let language = match object.remove("@language") {
                    Some(Value::String(language)) => Some(language),
                    Some(_) => {
                        self.unsupported(&json_pointer(pointer, "@language"));
                        return Ok(None);
                    }
                    None => None,
                };
                let direction = match object.remove("@direction") {
                    Some(Value::String(direction)) if direction == "ltr" => {
                        Some(super::super::ProjectionDirection::Ltr)
                    }
                    Some(Value::String(direction)) if direction == "rtl" => {
                        Some(super::super::ProjectionDirection::Rtl)
                    }
                    Some(_) => {
                        self.unsupported(&json_pointer(pointer, "@direction"));
                        return Ok(None);
                    }
                    None => None,
                };
                let explicit_datatype = match object.remove("@type") {
                    Some(Value::String(datatype)) => Some(datatype),
                    Some(_) => {
                        self.unsupported(&json_pointer(pointer, "@type"));
                        return Ok(None);
                    }
                    None => None,
                };
                self.record_unknowns(&object, pointer);
                let datatype = explicit_datatype.unwrap_or_else(|| {
                    if direction.is_some() {
                        self.config
                            .common()
                            .roles()
                            .iri(super::ResearchRole::RdfDirLangString)
                            .to_owned()
                    } else if language.is_some() {
                        self.config
                            .common()
                            .roles()
                            .iri(super::ResearchRole::RdfLangString)
                            .to_owned()
                    } else {
                        xsd_string
                    }
                });
                ResearchText::new(value, datatype, language, direction).map(Some)
            }
            _ => {
                self.unsupported(pointer);
                Ok(None)
            }
        }
    }

    fn take_values(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
    ) -> Result<Vec<ResearchValue>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut values = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            if let Some(value) = self.parse_value(value, &pointer)? {
                values.push(value);
            }
        }
        Ok(values)
    }

    fn parse_value(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchValue>, ProjectionError> {
        if value
            .as_object()
            .is_some_and(|object| object.contains_key("@id"))
        {
            let native = self.parse_native_ref(&value, pointer)?;
            let resolved = self.resolve_model_id(&native, pointer)?;
            return ResearchValue::iri(resolved).map(Some);
        }
        self.parse_text(value, pointer)
            .map(|value| value.map(ResearchValue::Text))
    }

    fn take_single_native_ref(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
        require_known: bool,
    ) -> Result<String, ProjectionError> {
        let values = self.take_native_refs(object, role, parent, require_known)?;
        if values.len() != 1 {
            return Err(ProjectionError::integrity(format!(
                "RO-Crate role `{role:?}` requires exactly one reference"
            ))
            .at_path(RO_CRATE_ARTIFACT));
        }
        Ok(values.into_iter().next().expect("one reference"))
    }

    fn take_entity_refs(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        self.take_native_refs(object, role, parent, true)?
            .into_iter()
            .map(|native| self.resolve_model_id(&native, parent))
            .collect()
    }

    fn take_native_refs(
        &mut self,
        object: &mut Map<String, Value>,
        role: RoCrateRole,
        parent: &str,
        require_known: bool,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut references = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let native = self.parse_native_ref(&value, &pointer)?;
            if require_known && !self.known_ids.contains(&native) {
                return Err(ProjectionError::integrity(format!(
                    "RO-Crate graph reference `{native}` is dangling"
                ))
                .at_path(RO_CRATE_ARTIFACT));
            }
            references.push(native);
        }
        Ok(references)
    }

    fn parse_native_ref(
        &mut self,
        value: &Value,
        pointer: &str,
    ) -> Result<String, ProjectionError> {
        match value {
            Value::String(value) => Ok(value.clone()),
            Value::Object(object) => {
                let Some(Value::String(value)) = object.get("@id") else {
                    self.unsupported(pointer);
                    return Err(ProjectionError::integrity(
                        "RO-Crate reference requires a string @id",
                    )
                    .at_path(RO_CRATE_ARTIFACT));
                };
                for member in object.keys().filter(|member| member.as_str() != "@id") {
                    self.loss(
                        LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                        &json_pointer(pointer, member),
                    );
                }
                Ok(value.clone())
            }
            _ => {
                self.unsupported(pointer);
                Err(
                    ProjectionError::integrity("RO-Crate reference must be a string or @id object")
                        .at_path(RO_CRATE_ARTIFACT),
                )
            }
        }
    }

    fn resolve_model_id(&mut self, native: &str, pointer: &str) -> Result<String, ProjectionError> {
        if native == self.config.root_dataset_id() {
            return Ok(self.config.common().identity().dataset_iri().to_owned());
        }
        if validate_absolute_iri(native, "RO-Crate entity identity").is_ok() {
            return Ok(native.to_owned());
        }
        validate_native_id(native, false).map_err(|error| {
            ProjectionError::integrity(error.message()).at_path(RO_CRATE_ARTIFACT)
        })?;
        let resolved = self.config.common().identity().resolve_relative(native)?;
        self.loss(LOSS_RESEARCH_LOCAL_ID_RESOLVED, pointer);
        Ok(resolved)
    }

    fn take_paths(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self.config.vocabulary().term(RoCrateRole::Path).to_owned();
        let mut paths = Vec::new();
        for (index, value) in self
            .take_items(object, RoCrateRole::Path, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Value::String(path) = value else {
                self.unsupported(&pointer);
                continue;
            };
            validate_data_path(&path)?;
            paths.push(path);
        }
        Ok(paths)
    }

    fn take_byte_size(&mut self, object: &mut Map<String, Value>, parent: &str) -> Option<u64> {
        let term = self
            .config
            .vocabulary()
            .term(RoCrateRole::ContentSize)
            .to_owned();
        let value = object.remove(&term)?;
        let Some(value) = value.as_u64() else {
            self.unsupported(&json_pointer(parent, &term));
            return None;
        };
        Some(value)
    }

    fn take_checksums(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchChecksum>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(RoCrateRole::Checksum)
            .to_owned();
        let mut checksums = Vec::new();
        for (index, value) in self
            .take_items(object, RoCrateRole::Checksum, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Value::Object(mut checksum) = value else {
                self.unsupported(&pointer);
                continue;
            };
            let algorithm_term = self
                .config
                .vocabulary()
                .term(RoCrateRole::ChecksumAlgorithm)
                .to_owned();
            let algorithm_value = checksum.remove(&algorithm_term).ok_or_else(|| {
                ProjectionError::integrity("RO-Crate checksum is missing its algorithm")
                    .at_path(RO_CRATE_ARTIFACT)
            })?;
            let algorithm = self
                .parse_value(algorithm_value, &json_pointer(&pointer, &algorithm_term))?
                .ok_or_else(|| {
                    ProjectionError::integrity("RO-Crate checksum algorithm is unsupported")
                        .at_path(RO_CRATE_ARTIFACT)
                })?;
            let value_term = self
                .config
                .vocabulary()
                .term(RoCrateRole::ChecksumValue)
                .to_owned();
            let lexical = checksum.remove(&value_term).ok_or_else(|| {
                ProjectionError::integrity("RO-Crate checksum is missing its lexical value")
                    .at_path(RO_CRATE_ARTIFACT)
            })?;
            let value = self
                .parse_text(lexical, &json_pointer(&pointer, &value_term))?
                .ok_or_else(|| {
                    ProjectionError::integrity("RO-Crate checksum value is unsupported")
                        .at_path(RO_CRATE_ARTIFACT)
                })?;
            self.record_unknowns(&checksum, &pointer);
            checksums.push(ResearchChecksum { algorithm, value });
        }
        Ok(checksums)
    }

    fn take_type(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<String, ProjectionError> {
        let value = object.remove("@type").ok_or_else(|| {
            ProjectionError::integrity("RO-Crate graph entity is missing @type")
                .at_path(RO_CRATE_ARTIFACT)
        })?;
        match value {
            Value::String(value) => Ok(value),
            Value::Array(mut values) => {
                if values.len() > 1 {
                    self.loss(LOSS_RESEARCH_ORDER_DROPPED, &json_pointer(parent, "@type"));
                }
                if values.len() != 1 {
                    return Err(ProjectionError::integrity(
                        "RO-Crate graph entity requires exactly one configured class",
                    )
                    .at_path(RO_CRATE_ARTIFACT));
                }
                let value = values.pop().expect("one type");
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    ProjectionError::integrity("RO-Crate @type array must contain a string")
                        .at_path(RO_CRATE_ARTIFACT)
                })
            }
            _ => Err(ProjectionError::integrity(
                "RO-Crate @type must be a string or singleton string array",
            )
            .at_path(RO_CRATE_ARTIFACT)),
        }
    }

    fn require_type(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
        expected: &str,
    ) -> Result<(), ProjectionError> {
        let actual = self.take_type(object, parent)?;
        if actual != expected {
            return Err(ProjectionError::integrity(format!(
                "RO-Crate entity at `{parent}` has type `{actual}`; expected `{expected}`"
            ))
            .at_path(RO_CRATE_ARTIFACT));
        }
        Ok(())
    }

    fn record_unknowns(&mut self, object: &Map<String, Value>, parent: &str) {
        for member in object.keys() {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &json_pointer(parent, member),
            );
        }
    }

    fn unsupported(&mut self, pointer: &str) {
        self.loss(LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED, pointer);
    }

    fn loss(&mut self, code: &'static str, pointer: &str) {
        record_loss(self.ledger, self.contract, code, RO_CRATE_ARTIFACT, pointer);
    }
}

fn item_pointer(parent: &str, term: &str, index: usize) -> String {
    format!("{}/{index}", json_pointer(parent, term))
}

fn validate_data_path(path: &str) -> Result<(), ProjectionError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(['?', '#'])
        || path
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(
            ProjectionError::integrity(format!("unsafe RO-Crate data path `{path}`"))
                .at_path(RO_CRATE_ARTIFACT),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::{
        ProjectionLimits, RESEARCH_ROLES, ResearchObjectIdentity, ResearchObjectPolicy,
        ResearchObjectRoles,
    };
    use purrdf_core::loss::{
        LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED, LOSS_RESEARCH_LOCAL_ID_RESOLVED,
        LOSS_RESEARCH_ORDER_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    };

    const INPUT: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/ro-crate-1.3/input.json");
    const GOLDEN: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/ro-crate-1.3/golden.json");

    fn config() -> RoCrateConfig {
        let roles = RESEARCH_ROLES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, role)| (role, format!("https://example.org/rdf/role-{index}")))
            .collect();
        let roles = ResearchObjectRoles::new(roles).expect("RDF roles");
        let identity = ResearchObjectIdentity::new(
            "https://example.org/datasets/cats",
            "https://example.org/entities/",
        )
        .expect("identity");
        let limits = ProjectionLimits::new(4, 250_000, 500_000, 600_000, 12).expect("limits");
        let policy = ResearchObjectPolicy::new(limits, 12_000, 1_000, 6_000, 12)
            .expect("research-object policy");
        let common = ResearchObjectConfig::new(roles, identity, policy);
        let vocabulary = RO_CRATE_ROLES
            .iter()
            .copied()
            .map(|role| (role, test_term(role).to_owned()))
            .collect();
        let vocabulary = RoCrateVocabulary::new(vocabulary).expect("RO-Crate vocabulary");
        let definitions = RO_CRATE_ROLES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, role)| {
                (
                    test_term(role).to_owned(),
                    format!("https://example.org/ro-crate/term-{index}"),
                )
            })
            .collect();
        let context = OfflineJsonLdContext::new(
            Value::String("https://example.org/context/ro-crate-1.3".to_owned()),
            definitions,
        )
        .expect("offline context");
        RoCrateConfig::new(
            common,
            context,
            vocabulary,
            "https://example.org/profiles/ro-crate-1.3",
            "ro-crate-metadata.json",
            "./",
        )
        .expect("RO-Crate config")
    }

    fn test_term(role: RoCrateRole) -> &'static str {
        match role {
            RoCrateRole::RootDatasetClass => "Dataset",
            RoCrateRole::MetadataDescriptorClass => "CreativeWork",
            RoCrateRole::FileClass => "File",
            RoCrateRole::AgentClass => "Person",
            RoCrateRole::ActivityClass => "CreateAction",
            RoCrateRole::RecordSetClass => "RecordSet",
            RoCrateRole::FieldClass => "FormalParameter",
            RoCrateRole::Name => "name",
            RoCrateRole::Description => "description",
            RoCrateRole::Identifier => "identifier",
            RoCrateRole::Version => "version",
            RoCrateRole::DatePublished => "datePublished",
            RoCrateRole::DateModified => "dateModified",
            RoCrateRole::Url => "url",
            RoCrateRole::Keywords => "keywords",
            RoCrateRole::License => "license",
            RoCrateRole::Creator => "creator",
            RoCrateRole::Publisher => "publisher",
            RoCrateRole::HasPart => "hasPart",
            RoCrateRole::Mentions => "mentions",
            RoCrateRole::ConformsTo => "conformsTo",
            RoCrateRole::About => "about",
            RoCrateRole::Path => "path",
            RoCrateRole::ContentUrl => "contentUrl",
            RoCrateRole::EncodingFormat => "encodingFormat",
            RoCrateRole::Format => "format",
            RoCrateRole::ContentSize => "contentSize",
            RoCrateRole::Checksum => "checksum",
            RoCrateRole::ChecksumAlgorithm => "checksumAlgorithm",
            RoCrateRole::ChecksumValue => "checksumValue",
            RoCrateRole::InlineContent => "content",
            RoCrateRole::Field => "field",
            RoCrateRole::DataType => "dataType",
            RoCrateRole::Records => "records",
            RoCrateRole::Instrument => "instrument",
            RoCrateRole::Agent => "agent",
            RoCrateRole::Object => "object",
            RoCrateRole::Result => "result",
            RoCrateRole::EndTime => "endTime",
            RoCrateRole::Workflow => "workflow",
        }
    }

    fn package(bytes: impl Into<Vec<u8>>) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(config().common().limits(), [(RO_CRATE_ARTIFACT, bytes)])
            .expect("package")
    }

    #[test]
    fn fixture_has_exact_losses_sorted_graph_and_stable_rewrite() {
        let config = config();
        let read = read_ro_crate(&package(INPUT), &config).expect("read fixture");
        let codes: BTreeSet<&str> = read
            .loss_ledger
            .entries()
            .iter()
            .map(|entry| entry.code.as_ref())
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
                LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED,
                LOSS_RESEARCH_LOCAL_ID_RESOLVED,
                LOSS_RESEARCH_ORDER_DROPPED,
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
            ])
        );
        assert!(
            read.loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );

        let projected = project_ro_crate(&read.dataset, &config).expect("project fixture");
        let actual = projected
            .package
            .get(RO_CRATE_ARTIFACT)
            .expect("RO-Crate artifact");
        assert_eq!(
            actual,
            GOLDEN,
            "actual golden bytes: {}",
            String::from_utf8_lossy(actual)
        );
        let value: Value = serde_json::from_slice(actual).expect("canonical JSON");
        let ids: Vec<&str> = value["@graph"]
            .as_array()
            .expect("graph")
            .iter()
            .map(|node| node["@id"].as_str().expect("id"))
            .collect();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

        let reread = read_ro_crate(&projected.package, &config).expect("read canonical output");
        let reprojected = project_ro_crate(&reread.dataset, &config).expect("rewrite");
        assert_eq!(projected.package, reprojected.package);
        assert_eq!(projected.model, reread.model);
    }

    #[test]
    fn config_rejects_unsafe_or_incomplete_identity_and_vocabulary() {
        let config = config();
        assert!(
            RoCrateConfig::new(
                config.common().clone(),
                config.context().clone(),
                config.vocabulary().clone(),
                config.profile_iri(),
                "../metadata.json",
                "./",
            )
            .is_err()
        );
        let mut definitions = config.context().definitions().clone();
        definitions.remove(test_term(RoCrateRole::Name));
        let context = OfflineJsonLdContext::new(config.context().value().clone(), definitions)
            .expect("independently valid context");
        assert!(
            RoCrateConfig::new(
                config.common().clone(),
                context,
                config.vocabulary().clone(),
                config.profile_iri(),
                config.metadata_descriptor_id(),
                config.root_dataset_id(),
            )
            .is_err()
        );
    }

    #[test]
    fn reader_rejects_duplicate_entities_dangling_refs_and_descriptor_drift() {
        let config = config();
        let mut value: Value = serde_json::from_slice(INPUT).expect("fixture JSON");
        let graph = value["@graph"].as_array_mut().expect("graph");
        graph[0]["@id"] = Value::String("files/train.csv".to_owned());
        assert!(
            read_ro_crate(&package(serde_json::to_vec(&value).expect("JSON")), &config).is_err()
        );

        let mut value: Value = serde_json::from_slice(INPUT).expect("fixture JSON");
        let root = value["@graph"]
            .as_array_mut()
            .expect("graph")
            .iter_mut()
            .find(|node| node["@id"] == "./")
            .expect("root");
        root["hasPart"] = id_object("missing.csv");
        assert!(
            read_ro_crate(&package(serde_json::to_vec(&value).expect("JSON")), &config).is_err()
        );

        let input = String::from_utf8(INPUT.to_vec()).expect("UTF-8 fixture");
        let drift = input.replace(
            "https://example.org/profiles/ro-crate-1.3",
            "https://example.org/profiles/wrong",
        );
        assert!(read_ro_crate(&package(drift), &config).is_err());
    }
}
