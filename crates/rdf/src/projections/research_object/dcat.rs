// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_RESEARCH_ORDER_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
};
use purrdf_core::{DatasetView, LossLedger, research_object_to_rdf_loss_ledger};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::native_codecs::jsonld::parse_jsonld;

use super::super::{ProjectionError, ProjectionPackage, stable_identifier, validate_absolute_iri};
use super::json::{
    ResearchObjectPackageProjection, ResearchObjectReadOutcome, canonical_json, ensure_sound,
    json_pointer, normalize_lifted_jsonld, parse_strict_json, record_loss, require_artifact,
};
use super::{
    OfflineJsonLdContext, ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset,
    ResearchField, ResearchObjectConfig, ResearchObjectModel, ResearchRecordSet, ResearchResource,
    ResearchText, ResearchValue, lift_research_object, project_research_object,
};

/// Closed DCAT projection profile identifier.
pub const DCAT_PROFILE: &str = "dcat-3";
/// Sole artifact path in the canonical DCAT package.
pub const DCAT_ARTIFACT: &str = "dcat.jsonld";

/// Semantic compact term required by the DCAT 3 application-profile adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DcatRole {
    /// Dataset class.
    DatasetClass,
    /// Distribution/resource class.
    DistributionClass,
    /// Agent class.
    AgentClass,
    /// Provenance activity class.
    ActivityClass,
    /// Structured record-set class.
    RecordSetClass,
    /// Record-set field class.
    FieldClass,
    /// Checksum class.
    ChecksumClass,
    /// Title/name property for datasets and described entities.
    Title,
    /// Agent-name property.
    AgentName,
    /// Description property.
    Description,
    /// Identifier property.
    Identifier,
    /// Version property.
    Version,
    /// Issue/publication date property.
    Issued,
    /// Modification date property.
    Modified,
    /// Landing-page property.
    LandingPage,
    /// Keyword property.
    Keyword,
    /// License property.
    License,
    /// Creator relation.
    Creator,
    /// Publisher relation.
    Publisher,
    /// Dataset-to-distribution relation.
    Distribution,
    /// Dataset-to-activity relation.
    Activity,
    /// Dataset-to-record-set relation.
    RecordSet,
    /// Profile-conformance relation.
    ConformsTo,
    /// Distribution package-path property.
    Path,
    /// Distribution download/access URL property.
    DownloadUrl,
    /// Distribution media-type property.
    MediaType,
    /// Distribution format property.
    Format,
    /// Distribution byte-size property.
    ByteSize,
    /// Distribution-to-checksum relation.
    Checksum,
    /// Checksum algorithm property.
    ChecksumAlgorithm,
    /// Checksum lexical-value property.
    ChecksumValue,
    /// Record-set-to-field relation.
    Field,
    /// Field datatype property.
    DataType,
    /// Canonical inline-row property.
    Records,
    /// Activity instrument relation.
    Instrument,
    /// Activity agent relation.
    Agent,
    /// Activity input/object relation.
    Object,
    /// Activity result relation.
    Result,
    /// Activity completion-time property.
    EndTime,
    /// Activity workflow relation.
    Workflow,
}

/// Every mandatory DCAT role in deterministic configuration order.
pub const DCAT_ROLES: &[DcatRole] = &[
    DcatRole::DatasetClass,
    DcatRole::DistributionClass,
    DcatRole::AgentClass,
    DcatRole::ActivityClass,
    DcatRole::RecordSetClass,
    DcatRole::FieldClass,
    DcatRole::ChecksumClass,
    DcatRole::Title,
    DcatRole::AgentName,
    DcatRole::Description,
    DcatRole::Identifier,
    DcatRole::Version,
    DcatRole::Issued,
    DcatRole::Modified,
    DcatRole::LandingPage,
    DcatRole::Keyword,
    DcatRole::License,
    DcatRole::Creator,
    DcatRole::Publisher,
    DcatRole::Distribution,
    DcatRole::Activity,
    DcatRole::RecordSet,
    DcatRole::ConformsTo,
    DcatRole::Path,
    DcatRole::DownloadUrl,
    DcatRole::MediaType,
    DcatRole::Format,
    DcatRole::ByteSize,
    DcatRole::Checksum,
    DcatRole::ChecksumAlgorithm,
    DcatRole::ChecksumValue,
    DcatRole::Field,
    DcatRole::DataType,
    DcatRole::Records,
    DcatRole::Instrument,
    DcatRole::Agent,
    DcatRole::Object,
    DcatRole::Result,
    DcatRole::EndTime,
    DcatRole::Workflow,
];

/// Complete caller-owned compact-term binding for the DCAT application profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DcatVocabulary(BTreeMap<DcatRole, String>);

impl DcatVocabulary {
    /// Validate a complete collision-free compact-term map.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra roles, JSON-LD keywords, whitespace-bearing terms,
    /// and ambiguous duplicate bindings.
    pub fn new(terms: BTreeMap<DcatRole, String>) -> Result<Self, ProjectionError> {
        for role in DCAT_ROLES {
            let term = terms.get(role).ok_or_else(|| {
                ProjectionError::configuration(format!(
                    "DCAT vocabulary is missing role `{role:?}`"
                ))
            })?;
            if term.is_empty() || term.starts_with('@') || term.chars().any(char::is_whitespace) {
                return Err(ProjectionError::configuration(format!(
                    "DCAT role `{role:?}` has invalid compact term `{term}`"
                )));
            }
        }
        if terms.len() != DCAT_ROLES.len() {
            return Err(ProjectionError::configuration(
                "DCAT vocabulary contains an unsupported role",
            ));
        }
        let mut inverse = BTreeMap::<&str, DcatRole>::new();
        for (&role, term) in &terms {
            if let Some(previous) = inverse.insert(term, role) {
                return Err(ProjectionError::configuration(format!(
                    "DCAT roles `{previous:?}` and `{role:?}` both bind `{term}`"
                )));
            }
        }
        Ok(Self(terms))
    }

    /// Compact term bound to one DCAT semantic role.
    pub fn term(&self, role: DcatRole) -> &str {
        self.0
            .get(&role)
            .expect("validated DCAT role map is complete")
    }

    /// Deterministically ordered role map.
    pub const fn terms(&self) -> &BTreeMap<DcatRole, String> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DcatVocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let terms = BTreeMap::<DcatRole, String>::deserialize(deserializer)?;
        Self::new(terms).map_err(serde::de::Error::custom)
    }
}

/// Mandatory caller-owned DCAT 3 configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DcatConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: DcatVocabulary,
    profile_iri: String,
}

impl DcatConfig {
    /// Construct and cross-validate DCAT configuration.
    ///
    /// # Errors
    ///
    /// Rejects a non-absolute profile or a compact term without a caller-owned
    /// offline expansion. No DCAT/DCTERMS/SPDX/RDF term is supplied by PurRDF.
    pub fn new(
        common: ResearchObjectConfig,
        context: OfflineJsonLdContext,
        vocabulary: DcatVocabulary,
        profile_iri: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let profile_iri = profile_iri.into();
        validate_absolute_iri(&profile_iri, "DCAT profile identity")?;
        for (&role, term) in vocabulary.terms() {
            if context.expand(term).is_none() {
                return Err(ProjectionError::configuration(format!(
                    "DCAT term `{term}` for role `{role:?}` has no offline expansion"
                )));
            }
        }
        Ok(Self {
            common,
            context,
            vocabulary,
            profile_iri,
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
    /// Caller-owned DCAT compact terms.
    pub const fn vocabulary(&self) -> &DcatVocabulary {
        &self.vocabulary
    }
    /// Absolute caller-selected DCAT application-profile identity.
    pub fn profile_iri(&self) -> &str {
        &self.profile_iri
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDcatConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: DcatVocabulary,
    profile_iri: String,
}

impl<'de> Deserialize<'de> for DcatConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDcatConfig::deserialize(deserializer)?;
        Self::new(raw.common, raw.context, raw.vocabulary, raw.profile_iri)
            .map_err(serde::de::Error::custom)
    }
}

/// Project caller-vocabulary RDF 1.2 into canonical DCAT 3 JSON-LD.
///
/// # Errors
///
/// Returns typed mapping, configuration, JSON-LD semantic-validation, or
/// configured resource-limit failures with every RDF-side loss ledgered.
pub fn project_dcat<D: DatasetView>(
    view: &D,
    config: &DcatConfig,
) -> Result<ResearchObjectPackageProjection, ProjectionError> {
    let projection = project_research_object(view, DCAT_PROFILE, config.common())?;
    let document = encode_document(&projection.model, config)?;
    validate_semantic_jsonld(&document, config)?;
    ensure_sound(&projection.loss_ledger, "rdf-1.2-dataset", DCAT_PROFILE)?;
    let bytes = canonical_json(&document, config.common().limits(), "DCAT 3 JSON-LD")?;
    let package =
        ProjectionPackage::from_artifacts(config.common().limits(), [(DCAT_ARTIFACT, bytes)])?;
    Ok(ResearchObjectPackageProjection {
        package,
        model: projection.model,
        loss_ledger: projection.loss_ledger,
    })
}

/// Read strict DCAT 3 JSON-LD and lift caller-vocabulary RDF 1.2.
///
/// # Errors
///
/// Rejects unexpected artifacts, duplicate JSON members, context/profile drift,
/// invalid JSON-LD semantics, duplicate/dangling graph identities, unsupported
/// entity shapes, or configured resource-limit excesses.
pub fn read_dcat(
    package: &ProjectionPackage,
    config: &DcatConfig,
) -> Result<ResearchObjectReadOutcome, ProjectionError> {
    let bytes = require_artifact(package, DCAT_ARTIFACT, config.common())?;
    let value = parse_strict_json(bytes, config.common(), "DCAT 3 JSON-LD", DCAT_ARTIFACT)?;
    validate_semantic_jsonld(&value, config)?;
    let contract = research_object_to_rdf_loss_ledger(DCAT_PROFILE);
    let mut ledger = LossLedger::new();
    let model = decode_document(value, config, &contract, &mut ledger)?
        .normalize(config.common().policy())?;
    ensure_sound(&ledger, DCAT_PROFILE, "rdf-1.2-dataset")?;
    let dataset = lift_research_object(model.clone(), config.common())?;
    Ok(ResearchObjectReadOutcome {
        dataset: normalize_lifted_jsonld(&dataset)?,
        model,
        loss_ledger: ledger,
    })
}

fn encode_document(
    model: &ResearchObjectModel,
    config: &DcatConfig,
) -> Result<Value, ProjectionError> {
    let mut graph = vec![encode_dataset(model, config)];
    graph.extend(model.agents.iter().map(|agent| encode_agent(agent, config)));
    for resource in &model.resources {
        let (node, checksums) = encode_distribution(resource, config)?;
        graph.push(node);
        graph.extend(checksums);
    }
    graph.extend(
        model
            .activities
            .iter()
            .map(|activity| encode_activity(activity, config)),
    );
    for record_set in &model.record_sets {
        graph.push(encode_record_set(record_set, config)?);
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

fn encode_dataset(model: &ResearchObjectModel, config: &DcatConfig) -> Value {
    let terms = config.vocabulary();
    let dataset = &model.dataset;
    let mut object = typed_object(&dataset.id, terms.term(DcatRole::DatasetClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::ConformsTo),
        vec![id_object(config.profile_iri())],
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Title),
        encode_texts(&dataset.titles),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Description),
        encode_texts(&dataset.descriptions),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Identifier),
        encode_values(&dataset.identifiers),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Version),
        encode_texts(&dataset.versions),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Issued),
        encode_texts(&dataset.issued),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Modified),
        encode_texts(&dataset.modified),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::LandingPage),
        encode_values(&dataset.landing_pages),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Keyword),
        encode_texts(&dataset.keywords),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::License),
        encode_values(&dataset.licenses),
    );
    for (role, ids) in [
        (DcatRole::Creator, &dataset.creators),
        (DcatRole::Publisher, &dataset.publishers),
        (DcatRole::Distribution, &dataset.resources),
        (DcatRole::Activity, &dataset.activities),
        (DcatRole::RecordSet, &dataset.record_sets),
    ] {
        insert_values(
            &mut object,
            terms.term(role),
            ids.iter().map(|id| id_object(id)).collect(),
        );
    }
    Value::Object(object)
}

fn encode_agent(agent: &ResearchAgent, config: &DcatConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&agent.id, terms.term(DcatRole::AgentClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::AgentName),
        encode_texts(&agent.names),
    );
    Value::Object(object)
}

fn encode_distribution(
    resource: &ResearchResource,
    config: &DcatConfig,
) -> Result<(Value, Vec<Value>), ProjectionError> {
    let terms = config.vocabulary();
    let mut object = typed_object(&resource.id, terms.term(DcatRole::DistributionClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::Title),
        encode_texts(&resource.names),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Description),
        encode_texts(&resource.descriptions),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Path),
        resource
            .paths
            .iter()
            .map(|path| plain_value(path))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::DownloadUrl),
        encode_values(&resource.urls),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::MediaType),
        encode_texts(&resource.media_types),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Format),
        encode_values(&resource.formats),
    );
    if let Some(byte_size) = resource.byte_size {
        object.insert(
            terms.term(DcatRole::ByteSize).to_owned(),
            Value::Array(vec![typed_value(
                &byte_size.to_string(),
                config
                    .common()
                    .roles()
                    .iri(super::ResearchRole::XsdNonNegativeInteger),
            )]),
        );
    }

    let mut checksum_nodes = Vec::new();
    let mut checksum_refs = Vec::new();
    for (index, checksum) in resource.checksums.iter().enumerate() {
        let key = canonical_json(
            &Value::Array(vec![
                Value::String(resource.id.clone()),
                Value::Number(index.into()),
                encode_value(&checksum.algorithm),
                encode_text(&checksum.value),
            ]),
            config.common().limits(),
            "DCAT checksum identity key",
        )?;
        let id = format!("_:{}", stable_identifier("dcat_checksum", &key)?);
        checksum_refs.push(id_object(&id));
        checksum_nodes.push(encode_checksum(&id, checksum, config));
    }
    insert_values(&mut object, terms.term(DcatRole::Checksum), checksum_refs);
    Ok((Value::Object(object), checksum_nodes))
}

fn encode_checksum(id: &str, checksum: &ResearchChecksum, config: &DcatConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(id, terms.term(DcatRole::ChecksumClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::ChecksumAlgorithm),
        vec![encode_value(&checksum.algorithm)],
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::ChecksumValue),
        vec![encode_text(&checksum.value)],
    );
    Value::Object(object)
}

fn encode_activity(activity: &ResearchActivity, config: &DcatConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&activity.id, terms.term(DcatRole::ActivityClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::Title),
        encode_texts(&activity.names),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Instrument),
        encode_values(&activity.instruments),
    );
    for (role, ids) in [
        (DcatRole::Agent, &activity.actors),
        (DcatRole::Object, &activity.objects),
        (DcatRole::Result, &activity.results),
    ] {
        insert_values(
            &mut object,
            terms.term(role),
            ids.iter().map(|id| id_object(id)).collect(),
        );
    }
    insert_values(
        &mut object,
        terms.term(DcatRole::EndTime),
        encode_texts(&activity.end_times),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Workflow),
        encode_values(&activity.workflows),
    );
    Value::Object(object)
}

fn encode_record_set(
    record_set: &ResearchRecordSet,
    config: &DcatConfig,
) -> Result<Value, ProjectionError> {
    let terms = config.vocabulary();
    let mut object = typed_object(&record_set.id, terms.term(DcatRole::RecordSetClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::Title),
        encode_texts(&record_set.names),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Description),
        encode_texts(&record_set.descriptions),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::Field),
        record_set
            .fields
            .iter()
            .map(|field| id_object(&field.id))
            .collect(),
    );
    let mut rows = Vec::new();
    for row in &record_set.rows {
        let bytes = canonical_json(row, config.common().limits(), "DCAT inline row")?;
        let lexical = std::str::from_utf8(&bytes)
            .expect("canonical JSON is UTF-8")
            .trim_end();
        rows.push(typed_value(
            lexical,
            config
                .common()
                .roles()
                .iri(super::ResearchRole::JsonDatatype),
        ));
    }
    insert_values(&mut object, terms.term(DcatRole::Records), rows);
    Ok(Value::Object(object))
}

fn encode_field(field: &ResearchField, config: &DcatConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&field.id, terms.term(DcatRole::FieldClass));
    insert_values(
        &mut object,
        terms.term(DcatRole::Title),
        encode_texts(&field.names),
    );
    insert_values(
        &mut object,
        terms.term(DcatRole::DataType),
        encode_values(&field.data_types),
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

fn plain_value(value: &str) -> Value {
    Value::Object(Map::from_iter([(
        "@value".to_owned(),
        Value::String(value.to_owned()),
    )]))
}

fn typed_value(value: &str, datatype: &str) -> Value {
    Value::Object(Map::from_iter([
        ("@value".to_owned(), Value::String(value.to_owned())),
        ("@type".to_owned(), Value::String(datatype.to_owned())),
    ]))
}

fn encode_texts(values: &[ResearchText]) -> Vec<Value> {
    values.iter().map(encode_text).collect()
}

fn encode_values(values: &[ResearchValue]) -> Vec<Value> {
    values.iter().map(encode_value).collect()
}

fn encode_value(value: &ResearchValue) -> Value {
    match value {
        ResearchValue::Iri { value } => id_object(value),
        ResearchValue::Text(value) => encode_text(value),
    }
}

fn encode_text(value: &ResearchText) -> Value {
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
        .expect("DCAT writer constructs every graph node with @id")
}

fn validate_semantic_jsonld(value: &Value, config: &DcatConfig) -> Result<(), ProjectionError> {
    let expanded = expand_jsonld(value, config.context())?;
    let bytes = canonical_json(
        &expanded,
        config.common().limits(),
        "expanded DCAT semantic JSON-LD",
    )?;
    parse_jsonld(&bytes).map_err(|error| {
        ProjectionError::integrity(format!("validate DCAT JSON-LD semantics: {error}"))
            .at_path(DCAT_ARTIFACT)
    })?;
    Ok(())
}

fn expand_jsonld(value: &Value, context: &OfflineJsonLdContext) -> Result<Value, ProjectionError> {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| expand_jsonld(value, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(object) => {
            let mut expanded = Map::new();
            for (key, value) in object {
                if key == "@context" {
                    expanded.insert(key.clone(), Value::Object(Map::new()));
                    continue;
                }
                let expanded_key = if key.starts_with('@') {
                    key.clone()
                } else if let Some(iri) = context.expand(key) {
                    iri.to_owned()
                } else if validate_absolute_iri(key, "DCAT JSON-LD property").is_ok() {
                    key.clone()
                } else {
                    return Err(ProjectionError::integrity(format!(
                        "DCAT compact term `{key}` has no offline expansion"
                    ))
                    .at_path(DCAT_ARTIFACT));
                };
                let expanded_value = if key == "@type" {
                    expand_type_value(value, context)?
                } else {
                    expand_jsonld(value, context)?
                };
                expanded.insert(expanded_key, expanded_value);
            }
            Ok(Value::Object(expanded))
        }
        _ => Ok(value.clone()),
    }
}

fn expand_type_value(
    value: &Value,
    context: &OfflineJsonLdContext,
) -> Result<Value, ProjectionError> {
    match value {
        Value::String(value) => {
            if let Some(iri) = context.expand(value) {
                Ok(Value::String(iri.to_owned()))
            } else if validate_absolute_iri(value, "DCAT JSON-LD type").is_ok() {
                Ok(Value::String(value.clone()))
            } else {
                Err(ProjectionError::integrity(format!(
                    "DCAT type `{value}` has no offline expansion"
                ))
                .at_path(DCAT_ARTIFACT))
            }
        }
        Value::Array(values) => values
            .iter()
            .map(|value| expand_type_value(value, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        _ => Err(
            ProjectionError::integrity("DCAT @type must be a string or string array")
                .at_path(DCAT_ARTIFACT),
        ),
    }
}

struct RawNode {
    id: String,
    object: Map<String, Value>,
    pointer: String,
}

struct DcatDecoder<'a> {
    config: &'a DcatConfig,
    contract: &'a LossLedger,
    ledger: &'a mut LossLedger,
    known_ids: BTreeSet<String>,
}

fn decode_document(
    value: Value,
    config: &DcatConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<ResearchObjectModel, ProjectionError> {
    DcatDecoder {
        config,
        contract,
        ledger,
        known_ids: BTreeSet::new(),
    }
    .decode(value)
}

impl DcatDecoder<'_> {
    fn decode(mut self, value: Value) -> Result<ResearchObjectModel, ProjectionError> {
        let Value::Object(mut document) = value else {
            return Err(
                ProjectionError::syntax("DCAT document root must be a JSON object")
                    .at_path(DCAT_ARTIFACT),
            );
        };
        let context = document.remove("@context").ok_or_else(|| {
            ProjectionError::integrity("DCAT document is missing @context").at_path(DCAT_ARTIFACT)
        })?;
        if context != *self.config.context().value() {
            return Err(ProjectionError::integrity(
                "DCAT @context does not exactly match caller configuration",
            )
            .at_path(DCAT_ARTIFACT));
        }
        let graph = document.remove("@graph").ok_or_else(|| {
            ProjectionError::integrity("DCAT document is missing @graph").at_path(DCAT_ARTIFACT)
        })?;
        let Value::Array(graph) = graph else {
            return Err(
                ProjectionError::integrity("DCAT @graph must be an array").at_path(DCAT_ARTIFACT)
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
            if object.contains_key("@graph") {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &pointer);
                continue;
            }
            let Some(Value::String(id)) = object.remove("@id") else {
                return Err(ProjectionError::integrity(
                    "every DCAT graph entity requires a string @id",
                )
                .at_path(DCAT_ARTIFACT));
            };
            validate_node_id(&id)?;
            if nodes
                .insert(
                    id.clone(),
                    RawNode {
                        id: id.clone(),
                        object,
                        pointer,
                    },
                )
                .is_some()
            {
                return Err(ProjectionError::integrity(format!(
                    "duplicate DCAT graph identity `{id}`"
                ))
                .at_path(DCAT_ARTIFACT));
            }
        }
        self.known_ids = nodes.keys().cloned().collect();

        let root_id = self.config.common().identity().dataset_iri();
        let root = nodes.remove(root_id).ok_or_else(|| {
            ProjectionError::integrity("configured DCAT dataset entity is missing")
                .at_path(DCAT_ARTIFACT)
        })?;
        let mut agents = Vec::new();
        let mut resources = Vec::new();
        let mut activities = Vec::new();
        let mut raw_record_sets = Vec::new();
        let mut fields = BTreeMap::<String, ResearchField>::new();
        let mut checksums = BTreeMap::<String, RawNode>::new();

        for (_, mut node) in nodes {
            let class = self.take_type(&mut node.object, &node.pointer)?;
            match self.class_role(&class) {
                Some(DcatRole::AgentClass) => agents.push(self.decode_agent(node)?),
                Some(DcatRole::DistributionClass) => resources.push(node),
                Some(DcatRole::ActivityClass) => activities.push(self.decode_activity(node)?),
                Some(DcatRole::RecordSetClass) => raw_record_sets.push(node),
                Some(DcatRole::FieldClass) => {
                    fields.insert(node.id.clone(), self.decode_field(node)?);
                }
                Some(DcatRole::ChecksumClass) => {
                    checksums.insert(node.id.clone(), node);
                }
                _ => self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &node.pointer),
            }
        }

        let mut decoded_resources = Vec::new();
        for node in resources {
            decoded_resources.push(self.decode_resource(node, &mut checksums)?);
        }
        for checksum_id in checksums.keys() {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &format!("/@graph/@id={checksum_id}"),
            );
        }

        let mut record_sets = Vec::new();
        for node in raw_record_sets {
            record_sets.push(self.decode_record_set(node, &mut fields)?);
        }
        for field_id in fields.keys() {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &format!("/@graph/@id={field_id}"),
            );
        }

        let dataset = self.decode_dataset(root)?;
        Ok(ResearchObjectModel {
            dataset,
            agents,
            resources: decoded_resources,
            activities,
            record_sets,
        })
    }

    fn class_role(&self, class: &str) -> Option<DcatRole> {
        [
            DcatRole::AgentClass,
            DcatRole::DistributionClass,
            DcatRole::ActivityClass,
            DcatRole::RecordSetClass,
            DcatRole::FieldClass,
            DcatRole::ChecksumClass,
        ]
        .into_iter()
        .find(|role| self.config.vocabulary().term(*role) == class)
    }

    fn decode_dataset(&mut self, mut node: RawNode) -> Result<ResearchDataset, ProjectionError> {
        self.require_type(
            &mut node.object,
            &node.pointer,
            self.config.vocabulary().term(DcatRole::DatasetClass),
        )?;
        let profile = self.take_refs(
            &mut node.object,
            DcatRole::ConformsTo,
            &node.pointer,
            false,
            false,
        )?;
        if profile.as_slice() != [self.config.profile_iri()] {
            return Err(ProjectionError::integrity(
                "DCAT conformsTo must identify exactly the configured profile",
            )
            .at_path(DCAT_ARTIFACT));
        }
        let titles = self.take_texts(&mut node.object, DcatRole::Title, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, DcatRole::Description, &node.pointer)?;
        let identifiers =
            self.take_values(&mut node.object, DcatRole::Identifier, &node.pointer)?;
        let versions = self.take_texts(&mut node.object, DcatRole::Version, &node.pointer)?;
        let issued = self.take_texts(&mut node.object, DcatRole::Issued, &node.pointer)?;
        let modified = self.take_texts(&mut node.object, DcatRole::Modified, &node.pointer)?;
        let landing_pages =
            self.take_values(&mut node.object, DcatRole::LandingPage, &node.pointer)?;
        let keywords = self.take_texts(&mut node.object, DcatRole::Keyword, &node.pointer)?;
        let licenses = self.take_values(&mut node.object, DcatRole::License, &node.pointer)?;
        let creators = self.take_refs(
            &mut node.object,
            DcatRole::Creator,
            &node.pointer,
            true,
            true,
        )?;
        let publishers = self.take_refs(
            &mut node.object,
            DcatRole::Publisher,
            &node.pointer,
            true,
            true,
        )?;
        let resources = self.take_refs(
            &mut node.object,
            DcatRole::Distribution,
            &node.pointer,
            true,
            true,
        )?;
        let activities = self.take_refs(
            &mut node.object,
            DcatRole::Activity,
            &node.pointer,
            true,
            true,
        )?;
        let record_sets = self.take_refs(
            &mut node.object,
            DcatRole::RecordSet,
            &node.pointer,
            true,
            true,
        )?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchDataset {
            id: node.id,
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

    fn decode_agent(&mut self, mut node: RawNode) -> Result<ResearchAgent, ProjectionError> {
        let names = self.take_texts(&mut node.object, DcatRole::AgentName, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchAgent { id: node.id, names })
    }

    fn decode_resource(
        &mut self,
        mut node: RawNode,
        checksum_nodes: &mut BTreeMap<String, RawNode>,
    ) -> Result<ResearchResource, ProjectionError> {
        let names = self.take_texts(&mut node.object, DcatRole::Title, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, DcatRole::Description, &node.pointer)?;
        let paths = self.take_paths(&mut node.object, &node.pointer)?;
        let urls = self.take_values(&mut node.object, DcatRole::DownloadUrl, &node.pointer)?;
        let media_types = self.take_texts(&mut node.object, DcatRole::MediaType, &node.pointer)?;
        let formats = self.take_values(&mut node.object, DcatRole::Format, &node.pointer)?;
        let byte_size = self.take_byte_size(&mut node.object, &node.pointer)?;
        let checksum_ids = self.take_refs(
            &mut node.object,
            DcatRole::Checksum,
            &node.pointer,
            true,
            false,
        )?;
        let mut checksums = Vec::new();
        for checksum_id in checksum_ids {
            let checksum = checksum_nodes.remove(&checksum_id).ok_or_else(|| {
                ProjectionError::integrity(format!(
                    "DCAT distribution references non-checksum `{checksum_id}`"
                ))
                .at_path(DCAT_ARTIFACT)
            })?;
            checksums.push(self.decode_checksum(checksum)?);
        }
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchResource {
            id: node.id,
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

    fn decode_checksum(&mut self, mut node: RawNode) -> Result<ResearchChecksum, ProjectionError> {
        let mut algorithms =
            self.take_values(&mut node.object, DcatRole::ChecksumAlgorithm, &node.pointer)?;
        let mut values =
            self.take_texts(&mut node.object, DcatRole::ChecksumValue, &node.pointer)?;
        if algorithms.len() != 1 || values.len() != 1 {
            return Err(ProjectionError::integrity(
                "DCAT checksum requires exactly one algorithm and one lexical value",
            )
            .at_path(DCAT_ARTIFACT));
        }
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchChecksum {
            algorithm: algorithms.pop().expect("one algorithm"),
            value: values.pop().expect("one value"),
        })
    }

    fn decode_activity(&mut self, mut node: RawNode) -> Result<ResearchActivity, ProjectionError> {
        let names = self.take_texts(&mut node.object, DcatRole::Title, &node.pointer)?;
        let instruments =
            self.take_values(&mut node.object, DcatRole::Instrument, &node.pointer)?;
        let actors =
            self.take_refs(&mut node.object, DcatRole::Agent, &node.pointer, true, true)?;
        let objects = self.take_refs(
            &mut node.object,
            DcatRole::Object,
            &node.pointer,
            true,
            true,
        )?;
        let results = self.take_refs(
            &mut node.object,
            DcatRole::Result,
            &node.pointer,
            true,
            true,
        )?;
        let end_times = self.take_texts(&mut node.object, DcatRole::EndTime, &node.pointer)?;
        let workflows = self.take_values(&mut node.object, DcatRole::Workflow, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchActivity {
            id: node.id,
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
        let names = self.take_texts(&mut node.object, DcatRole::Title, &node.pointer)?;
        let data_types = self.take_values(&mut node.object, DcatRole::DataType, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchField {
            id: node.id,
            names,
            data_types,
        })
    }

    fn decode_record_set(
        &mut self,
        mut node: RawNode,
        fields: &mut BTreeMap<String, ResearchField>,
    ) -> Result<ResearchRecordSet, ProjectionError> {
        let names = self.take_texts(&mut node.object, DcatRole::Title, &node.pointer)?;
        let descriptions =
            self.take_texts(&mut node.object, DcatRole::Description, &node.pointer)?;
        let field_ids =
            self.take_refs(&mut node.object, DcatRole::Field, &node.pointer, true, true)?;
        let mut linked_fields = Vec::new();
        for field_id in field_ids {
            let field = fields.remove(&field_id).ok_or_else(|| {
                ProjectionError::integrity(format!(
                    "DCAT record set references non-field entity `{field_id}`"
                ))
                .at_path(DCAT_ARTIFACT)
            })?;
            linked_fields.push(field);
        }
        let rows = self.take_rows(&mut node.object, &node.pointer)?;
        self.record_unknowns(&node.object, &node.pointer);
        Ok(ResearchRecordSet {
            id: node.id,
            names,
            descriptions,
            fields: linked_fields,
            rows,
        })
    }

    fn take_items(
        &mut self,
        object: &mut Map<String, Value>,
        role: DcatRole,
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
        role: DcatRole,
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
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
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
                self.config
                    .common()
                    .roles()
                    .iri(super::ResearchRole::XsdString)
                    .to_owned()
            }
        });
        ResearchText::new(value, datatype, language, direction).map(Some)
    }

    fn take_values(
        &mut self,
        object: &mut Map<String, Value>,
        role: DcatRole,
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
            let id = self.parse_ref(&value, pointer)?;
            if id.starts_with("_:") {
                return Err(ProjectionError::integrity(
                    "DCAT scalar reference cannot use a blank-node identity",
                )
                .at_path(DCAT_ARTIFACT));
            }
            return ResearchValue::iri(id).map(Some);
        }
        self.parse_text(value, pointer)
            .map(|value| value.map(ResearchValue::Text))
    }

    fn take_refs(
        &mut self,
        object: &mut Map<String, Value>,
        role: DcatRole,
        parent: &str,
        require_known: bool,
        require_absolute: bool,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut references = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let id = self.parse_ref(&value, &pointer)?;
            if require_known && !self.known_ids.contains(&id) {
                return Err(ProjectionError::integrity(format!(
                    "DCAT graph reference `{id}` is dangling"
                ))
                .at_path(DCAT_ARTIFACT));
            }
            if require_absolute {
                validate_absolute_iri(&id, "DCAT entity reference").map_err(|error| {
                    ProjectionError::integrity(error.message()).at_path(DCAT_ARTIFACT)
                })?;
            }
            references.push(id);
        }
        Ok(references)
    }

    fn parse_ref(&mut self, value: &Value, pointer: &str) -> Result<String, ProjectionError> {
        let Value::Object(object) = value else {
            self.unsupported(pointer);
            return Err(
                ProjectionError::integrity("DCAT reference must be an @id object")
                    .at_path(DCAT_ARTIFACT),
            );
        };
        let Some(Value::String(id)) = object.get("@id") else {
            self.unsupported(pointer);
            return Err(
                ProjectionError::integrity("DCAT reference requires a string @id")
                    .at_path(DCAT_ARTIFACT),
            );
        };
        validate_node_id(id)?;
        for member in object.keys().filter(|member| member.as_str() != "@id") {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &json_pointer(pointer, member),
            );
        }
        Ok(id.clone())
    }

    fn take_paths(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let values = self.take_texts(object, DcatRole::Path, parent)?;
        let xsd_string = self
            .config
            .common()
            .roles()
            .iri(super::ResearchRole::XsdString);
        let mut paths = Vec::new();
        for value in values {
            if value.datatype != xsd_string || value.language.is_some() || value.direction.is_some()
            {
                return Err(ProjectionError::integrity(
                    "DCAT distribution path must be a plain string",
                )
                .at_path(DCAT_ARTIFACT));
            }
            validate_data_path(&value.value)?;
            paths.push(value.value);
        }
        Ok(paths)
    }

    fn take_byte_size(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Option<u64>, ProjectionError> {
        let values = self.take_texts(object, DcatRole::ByteSize, parent)?;
        if values.len() > 1 {
            return Err(ProjectionError::integrity(
                "DCAT distribution has more than one byte size",
            )
            .at_path(DCAT_ARTIFACT));
        }
        let Some(value) = values.into_iter().next() else {
            return Ok(None);
        };
        if value.datatype
            != self
                .config
                .common()
                .roles()
                .iri(super::ResearchRole::XsdNonNegativeInteger)
            || value.language.is_some()
            || value.direction.is_some()
        {
            return Err(ProjectionError::integrity(
                "DCAT byte size requires the caller non-negative-integer datatype",
            )
            .at_path(DCAT_ARTIFACT));
        }
        value.value.parse::<u64>().map(Some).map_err(|error| {
            ProjectionError::integrity(format!("invalid DCAT byte size: {error}"))
                .at_path(DCAT_ARTIFACT)
        })
    }

    fn take_rows(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<Value>, ProjectionError> {
        let values = self.take_texts(object, DcatRole::Records, parent)?;
        let datatype = self
            .config
            .common()
            .roles()
            .iri(super::ResearchRole::JsonDatatype);
        let mut rows = Vec::new();
        for value in values {
            if value.datatype != datatype || value.language.is_some() || value.direction.is_some() {
                return Err(ProjectionError::integrity(
                    "DCAT inline row requires the caller canonical-JSON datatype",
                )
                .at_path(DCAT_ARTIFACT));
            }
            rows.push(parse_strict_json(
                value.value.as_bytes(),
                self.config.common(),
                "DCAT inline row",
                DCAT_ARTIFACT,
            )?);
        }
        Ok(rows)
    }

    fn take_type(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<String, ProjectionError> {
        let value = object.remove("@type").ok_or_else(|| {
            ProjectionError::integrity("DCAT graph entity is missing @type").at_path(DCAT_ARTIFACT)
        })?;
        match value {
            Value::String(value) => Ok(value),
            Value::Array(mut values) => {
                if values.len() > 1 {
                    self.loss(LOSS_RESEARCH_ORDER_DROPPED, &json_pointer(parent, "@type"));
                }
                if values.len() != 1 {
                    return Err(ProjectionError::integrity(
                        "DCAT graph entity requires exactly one configured class",
                    )
                    .at_path(DCAT_ARTIFACT));
                }
                values
                    .pop()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .ok_or_else(|| {
                        ProjectionError::integrity("DCAT @type array must contain a string")
                            .at_path(DCAT_ARTIFACT)
                    })
            }
            _ => Err(ProjectionError::integrity(
                "DCAT @type must be a string or singleton string array",
            )
            .at_path(DCAT_ARTIFACT)),
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
                "DCAT entity at `{parent}` has type `{actual}`; expected `{expected}`"
            ))
            .at_path(DCAT_ARTIFACT));
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
        record_loss(self.ledger, self.contract, code, DCAT_ARTIFACT, pointer);
    }
}

fn item_pointer(parent: &str, term: &str, index: usize) -> String {
    format!("{}/{index}", json_pointer(parent, term))
}

fn validate_node_id(value: &str) -> Result<(), ProjectionError> {
    if validate_absolute_iri(value, "DCAT graph identity").is_ok() {
        return Ok(());
    }
    if let Some(label) = value.strip_prefix("_:")
        && !label.is_empty()
        && label
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Ok(());
    }
    Err(
        ProjectionError::integrity(format!("invalid DCAT graph identity `{value}`"))
            .at_path(DCAT_ARTIFACT),
    )
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
            ProjectionError::integrity(format!("unsafe DCAT distribution path `{path}`"))
                .at_path(DCAT_ARTIFACT),
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
    use serde_json::json;

    const INPUT: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/dcat-3/input.jsonld");
    const GOLDEN: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/dcat-3/golden.jsonld");

    fn config() -> DcatConfig {
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
        let limits = ProjectionLimits::new(4, 300_000, 600_000, 700_000, 16).expect("limits");
        let policy = ResearchObjectPolicy::new(limits, 15_000, 2_000, 8_000, 16)
            .expect("research-object policy");
        let common = ResearchObjectConfig::new(roles, identity, policy);

        let vocabulary = DCAT_ROLES
            .iter()
            .copied()
            .map(|role| (role, test_term(role).to_owned()))
            .collect();
        let vocabulary = DcatVocabulary::new(vocabulary).expect("DCAT vocabulary");
        let definitions = DCAT_ROLES
            .iter()
            .copied()
            .map(|role| {
                (
                    test_term(role).to_owned(),
                    format!("https://example.org/dcat/{}", test_term(role)),
                )
            })
            .collect();
        let context =
            OfflineJsonLdContext::new(json!({"@vocab": "https://example.org/dcat/"}), definitions)
                .expect("offline context");
        DcatConfig::new(
            common,
            context,
            vocabulary,
            "https://example.org/profiles/dcat-3",
        )
        .expect("DCAT config")
    }

    fn test_term(role: DcatRole) -> &'static str {
        match role {
            DcatRole::DatasetClass => "Dataset",
            DcatRole::DistributionClass => "Distribution",
            DcatRole::AgentClass => "Agent",
            DcatRole::ActivityClass => "Activity",
            DcatRole::RecordSetClass => "RecordSet",
            DcatRole::FieldClass => "Field",
            DcatRole::ChecksumClass => "Checksum",
            DcatRole::Title => "title",
            DcatRole::AgentName => "agentName",
            DcatRole::Description => "description",
            DcatRole::Identifier => "identifier",
            DcatRole::Version => "version",
            DcatRole::Issued => "issued",
            DcatRole::Modified => "modified",
            DcatRole::LandingPage => "landingPage",
            DcatRole::Keyword => "keyword",
            DcatRole::License => "license",
            DcatRole::Creator => "creator",
            DcatRole::Publisher => "publisher",
            DcatRole::Distribution => "distribution",
            DcatRole::Activity => "activity",
            DcatRole::RecordSet => "recordSet",
            DcatRole::ConformsTo => "conformsTo",
            DcatRole::Path => "path",
            DcatRole::DownloadUrl => "downloadURL",
            DcatRole::MediaType => "mediaType",
            DcatRole::Format => "format",
            DcatRole::ByteSize => "byteSize",
            DcatRole::Checksum => "checksum",
            DcatRole::ChecksumAlgorithm => "algorithm",
            DcatRole::ChecksumValue => "checksumValue",
            DcatRole::Field => "field",
            DcatRole::DataType => "dataType",
            DcatRole::Records => "records",
            DcatRole::Instrument => "instrument",
            DcatRole::Agent => "agent",
            DcatRole::Object => "object",
            DcatRole::Result => "result",
            DcatRole::EndTime => "endTime",
            DcatRole::Workflow => "workflow",
        }
    }

    fn package(bytes: impl Into<Vec<u8>>, config: &DcatConfig) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(config.common().limits(), [(DCAT_ARTIFACT, bytes)])
            .expect("package")
    }

    #[test]
    fn fixture_has_exact_losses_semantic_jsonld_and_stable_rewrite() {
        let config = config();
        let read = read_dcat(&package(INPUT, &config), &config).expect("read fixture");
        let codes: BTreeSet<&str> = read
            .loss_ledger
            .entries()
            .iter()
            .map(|entry| entry.code.as_ref())
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
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

        let projected = project_dcat(&read.dataset, &config).expect("project fixture");
        let actual = projected.package.get(DCAT_ARTIFACT).expect("DCAT artifact");
        assert_eq!(
            actual,
            GOLDEN,
            "actual golden bytes: {}",
            String::from_utf8_lossy(actual)
        );
        let canonical: Value = serde_json::from_slice(actual).expect("canonical JSON-LD");
        validate_semantic_jsonld(&canonical, &config).expect("native JSON-LD semantics");

        let reread = read_dcat(&projected.package, &config).expect("read canonical output");
        let reprojected = project_dcat(&reread.dataset, &config).expect("rewrite");
        assert_eq!(projected.package, reprojected.package);
        assert_eq!(projected.model, reread.model);
    }

    #[test]
    fn reader_rejects_duplicates_context_profile_dangling_and_local_ids() {
        let config = config();
        assert!(read_dcat(&package(br#"{"a":1,"a":2}"#, &config), &config).is_err());
        let input = String::from_utf8(INPUT.to_vec()).expect("UTF-8 fixture");

        let context = input.replacen("https://example.org/dcat/", "https://example.org/wrong/", 1);
        assert!(read_dcat(&package(context, &config), &config).is_err());

        let profile = input.replacen(
            "https://example.org/profiles/dcat-3",
            "https://example.org/profiles/wrong",
            1,
        );
        assert!(read_dcat(&package(profile, &config), &config).is_err());

        let dangling = input.replacen(
            "https://example.org/resources/train.csv\" }],\n      \"activity\"",
            "https://example.org/resources/missing.csv\" }],\n      \"activity\"",
            1,
        );
        assert!(read_dcat(&package(dangling, &config), &config).is_err());

        let local_id = input.replacen(
            "https://example.org/agents/alice\",\n      \"@type\": \"Agent\"",
            "agents/alice\",\n      \"@type\": \"Agent\"",
            1,
        );
        assert!(read_dcat(&package(local_id, &config), &config).is_err());
    }

    #[test]
    fn config_requires_complete_offline_expansion() {
        let config = config();
        let mut definitions = config.context().definitions().clone();
        definitions.remove(test_term(DcatRole::Title));
        let context = OfflineJsonLdContext::new(config.context().value().clone(), definitions)
            .expect("context remains independently valid");
        assert!(
            DcatConfig::new(
                config.common().clone(),
                context,
                config.vocabulary().clone(),
                config.profile_iri(),
            )
            .is_err()
        );
    }
}
