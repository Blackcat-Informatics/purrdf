// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED, LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED,
    LOSS_RESEARCH_LOCAL_ID_RESOLVED, LOSS_RESEARCH_ORDER_DROPPED,
    LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
};
use purrdf_core::{
    DatasetView, LossLedger, rdf_to_research_object_loss_ledger, research_object_to_rdf_loss_ledger,
};
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

/// Closed Croissant projection profile identifier.
pub const CROISSANT_PROFILE: &str = "croissant-1.1";
/// Sole artifact path in the canonical Croissant package.
pub const CROISSANT_ARTIFACT: &str = "croissant.json";

/// Semantic term required by the Croissant 1.1 adapter.
///
/// Every role is bound to a compact term by [`CroissantVocabulary`]. The
/// compact term is then expanded only through [`OfflineJsonLdContext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CroissantRole {
    /// Dataset class term.
    DatasetClass,
    /// FileObject class term.
    FileObjectClass,
    /// RecordSet class term.
    RecordSetClass,
    /// Field class term.
    FieldClass,
    /// Agent class term.
    AgentClass,
    /// Activity/action class term.
    ActivityClass,
    /// Human-readable name property.
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
    /// Creator property.
    Creator,
    /// Publisher property.
    Publisher,
    /// File distribution property.
    Distribution,
    /// Activity property.
    Activity,
    /// Record-set property.
    RecordSet,
    /// Profile-conformance property.
    ConformsTo,
    /// File path property.
    Path,
    /// Content URL property.
    ContentUrl,
    /// Media/encoding format property.
    EncodingFormat,
    /// Additional format identifier property.
    Format,
    /// Content-size property.
    ContentSize,
    /// SHA-256 property.
    Sha256,
    /// Inline payload property, recognized only for explicit loss accounting.
    InlineContent,
    /// Record-set field property.
    Field,
    /// Field datatype property.
    DataType,
    /// Inline records property.
    Records,
    /// Activity instrument property.
    Instrument,
    /// Activity agent property.
    Agent,
    /// Activity input property.
    Object,
    /// Activity result property.
    Result,
    /// Activity completion-time property.
    EndTime,
    /// Activity workflow property.
    Workflow,
}

/// Every mandatory Croissant role in deterministic configuration order.
pub const CROISSANT_ROLES: &[CroissantRole] = &[
    CroissantRole::DatasetClass,
    CroissantRole::FileObjectClass,
    CroissantRole::RecordSetClass,
    CroissantRole::FieldClass,
    CroissantRole::AgentClass,
    CroissantRole::ActivityClass,
    CroissantRole::Name,
    CroissantRole::Description,
    CroissantRole::Identifier,
    CroissantRole::Version,
    CroissantRole::DatePublished,
    CroissantRole::DateModified,
    CroissantRole::Url,
    CroissantRole::Keywords,
    CroissantRole::License,
    CroissantRole::Creator,
    CroissantRole::Publisher,
    CroissantRole::Distribution,
    CroissantRole::Activity,
    CroissantRole::RecordSet,
    CroissantRole::ConformsTo,
    CroissantRole::Path,
    CroissantRole::ContentUrl,
    CroissantRole::EncodingFormat,
    CroissantRole::Format,
    CroissantRole::ContentSize,
    CroissantRole::Sha256,
    CroissantRole::InlineContent,
    CroissantRole::Field,
    CroissantRole::DataType,
    CroissantRole::Records,
    CroissantRole::Instrument,
    CroissantRole::Agent,
    CroissantRole::Object,
    CroissantRole::Result,
    CroissantRole::EndTime,
    CroissantRole::Workflow,
];

/// Complete caller-owned compact-term binding for Croissant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CroissantVocabulary(BTreeMap<CroissantRole, String>);

impl CroissantVocabulary {
    /// Validate one complete, unambiguous compact-term map.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra roles, JSON-LD keywords, whitespace-bearing terms,
    /// or two roles bound to the same compact term.
    pub fn new(terms: BTreeMap<CroissantRole, String>) -> Result<Self, ProjectionError> {
        for role in CROISSANT_ROLES {
            let term = terms.get(role).ok_or_else(|| {
                ProjectionError::configuration(format!(
                    "Croissant vocabulary is missing role `{role:?}`"
                ))
            })?;
            if term.is_empty() || term.starts_with('@') || term.chars().any(char::is_whitespace) {
                return Err(ProjectionError::configuration(format!(
                    "Croissant role `{role:?}` has invalid compact term `{term}`"
                )));
            }
        }
        if terms.len() != CROISSANT_ROLES.len() {
            return Err(ProjectionError::configuration(
                "Croissant vocabulary contains an unsupported role",
            ));
        }
        let mut inverse = BTreeMap::<&str, CroissantRole>::new();
        for (&role, term) in &terms {
            if let Some(previous) = inverse.insert(term, role) {
                return Err(ProjectionError::configuration(format!(
                    "Croissant roles `{previous:?}` and `{role:?}` both bind `{term}`"
                )));
            }
        }
        Ok(Self(terms))
    }

    /// Compact term bound to one Croissant semantic role.
    pub fn term(&self, role: CroissantRole) -> &str {
        self.0
            .get(&role)
            .expect("validated Croissant role map is complete")
    }

    /// Deterministically ordered role map.
    pub const fn terms(&self) -> &BTreeMap<CroissantRole, String> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CroissantVocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let terms = BTreeMap::<CroissantRole, String>::deserialize(deserializer)?;
        Self::new(terms).map_err(serde::de::Error::custom)
    }
}

/// Mandatory caller-owned configuration for the Croissant 1.1 codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CroissantConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: CroissantVocabulary,
    profile_iri: String,
}

impl CroissantConfig {
    /// Construct and cross-validate a Croissant configuration.
    ///
    /// # Errors
    ///
    /// The profile identity must be absolute and every compact term must have
    /// one caller-provided offline expansion.
    pub fn new(
        common: ResearchObjectConfig,
        context: OfflineJsonLdContext,
        vocabulary: CroissantVocabulary,
        profile_iri: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let profile_iri = profile_iri.into();
        validate_absolute_iri(&profile_iri, "Croissant profile identity")?;
        for (&role, term) in vocabulary.terms() {
            if context.expand(term).is_none() {
                return Err(ProjectionError::configuration(format!(
                    "Croissant term `{term}` for role `{role:?}` has no offline expansion"
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

    /// Shared RDF vocabulary, identity, and bounds.
    pub const fn common(&self) -> &ResearchObjectConfig {
        &self.common
    }

    /// Exact emitted context and offline expansion table.
    pub const fn context(&self) -> &OfflineJsonLdContext {
        &self.context
    }

    /// Caller-owned Croissant compact terms.
    pub const fn vocabulary(&self) -> &CroissantVocabulary {
        &self.vocabulary
    }

    /// Absolute Croissant profile identity emitted through `conformsTo`.
    pub fn profile_iri(&self) -> &str {
        &self.profile_iri
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCroissantConfig {
    common: ResearchObjectConfig,
    context: OfflineJsonLdContext,
    vocabulary: CroissantVocabulary,
    profile_iri: String,
}

impl<'de> Deserialize<'de> for CroissantConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCroissantConfig::deserialize(deserializer)?;
        Self::new(raw.common, raw.context, raw.vocabulary, raw.profile_iri)
            .map_err(serde::de::Error::custom)
    }
}

/// Project caller-vocabulary RDF 1.2 into canonical Croissant 1.1 JSON.
///
/// # Errors
///
/// Returns a typed configuration, RDF interpretation, resource-limit, or JSON
/// encoding failure. Every representational loss is returned in the outcome.
pub fn project_croissant<D: DatasetView>(
    view: &D,
    config: &CroissantConfig,
) -> Result<ResearchObjectPackageProjection, ProjectionError> {
    let projection = project_research_object(view, CROISSANT_PROFILE, config.common())?;
    let mut ledger = projection.loss_ledger;
    let document = encode_document(&projection.model, config, &mut ledger)?;
    ensure_sound(&ledger, "rdf-1.2-dataset", CROISSANT_PROFILE)?;
    let bytes = canonical_json(&document, config.common().limits(), "Croissant 1.1 JSON")?;
    let package =
        ProjectionPackage::from_artifacts(config.common().limits(), [(CROISSANT_ARTIFACT, bytes)])?;
    Ok(ResearchObjectPackageProjection {
        package,
        model: projection.model,
        loss_ledger: ledger,
    })
}

/// Read one strict Croissant 1.1 package and lift it into caller-vocabulary RDF.
///
/// # Errors
///
/// Rejects unexpected artifacts, duplicate JSON members, context/profile drift,
/// unsafe identities, malformed known members, dangling references, or caller
/// resource-limit excesses. Ledgered unsupported constructs remain observable.
pub fn read_croissant(
    package: &ProjectionPackage,
    config: &CroissantConfig,
) -> Result<ResearchObjectReadOutcome, ProjectionError> {
    let bytes = require_artifact(package, CROISSANT_ARTIFACT, config.common())?;
    let value = parse_strict_json(
        bytes,
        config.common(),
        "Croissant 1.1 JSON",
        CROISSANT_ARTIFACT,
    )?;
    let contract = research_object_to_rdf_loss_ledger(CROISSANT_PROFILE);
    let mut ledger = LossLedger::new();
    let model = decode_document(value, config, &contract, &mut ledger)?
        .normalize(config.common().policy())?;
    ensure_sound(&ledger, CROISSANT_PROFILE, "rdf-1.2-dataset")?;
    let dataset = lift_research_object(model.clone(), config.common())?;
    let dataset = normalize_lifted_jsonld(&dataset)?;
    Ok(ResearchObjectReadOutcome {
        dataset,
        model,
        loss_ledger: ledger,
    })
}

fn encode_document(
    model: &ResearchObjectModel,
    config: &CroissantConfig,
    ledger: &mut LossLedger,
) -> Result<Value, ProjectionError> {
    let terms = config.vocabulary();
    let mut root = Map::new();
    root.insert("@context".to_owned(), config.context().value().clone());
    root.insert("@id".to_owned(), Value::String(model.dataset.id.clone()));
    root.insert(
        "@type".to_owned(),
        Value::String(terms.term(CroissantRole::DatasetClass).to_owned()),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Name),
        encode_texts(&model.dataset.titles, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Description),
        encode_texts(&model.dataset.descriptions, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Identifier),
        encode_values(&model.dataset.identifiers, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Version),
        encode_texts(&model.dataset.versions, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::DatePublished),
        encode_texts(&model.dataset.issued, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::DateModified),
        encode_texts(&model.dataset.modified, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Url),
        encode_values(&model.dataset.landing_pages, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Keywords),
        encode_texts(&model.dataset.keywords, config),
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::License),
        encode_values(&model.dataset.licenses, config),
    );
    root.insert(
        terms.term(CroissantRole::ConformsTo).to_owned(),
        id_object(config.profile_iri()),
    );

    let agents: BTreeMap<&str, &ResearchAgent> = model
        .agents
        .iter()
        .map(|agent| (agent.id.as_str(), agent))
        .collect();
    insert_values(
        &mut root,
        terms.term(CroissantRole::Creator),
        encode_entities(&model.dataset.creators, |id| {
            agents
                .get(id)
                .copied()
                .ok_or_else(|| {
                    ProjectionError::integrity(format!("missing Croissant agent `{id}`"))
                })
                .map(|agent| encode_agent(agent, config))
        })?,
    );
    insert_values(
        &mut root,
        terms.term(CroissantRole::Publisher),
        encode_entities(&model.dataset.publishers, |id| {
            agents
                .get(id)
                .copied()
                .ok_or_else(|| {
                    ProjectionError::integrity(format!("missing Croissant agent `{id}`"))
                })
                .map(|agent| encode_agent(agent, config))
        })?,
    );

    let resources: BTreeMap<&str, &ResearchResource> = model
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource))
        .collect();
    insert_values(
        &mut root,
        terms.term(CroissantRole::Distribution),
        encode_entities(&model.dataset.resources, |id| {
            let resource = resources.get(id).copied().ok_or_else(|| {
                ProjectionError::integrity(format!("missing Croissant resource `{id}`"))
            })?;
            encode_resource(resource, config, ledger)
        })?,
    );

    let activities: BTreeMap<&str, &ResearchActivity> = model
        .activities
        .iter()
        .map(|activity| (activity.id.as_str(), activity))
        .collect();
    insert_values(
        &mut root,
        terms.term(CroissantRole::Activity),
        encode_entities(&model.dataset.activities, |id| {
            activities
                .get(id)
                .copied()
                .ok_or_else(|| {
                    ProjectionError::integrity(format!("missing Croissant activity `{id}`"))
                })
                .map(|activity| encode_activity(activity, &agents, config))
        })?,
    );

    let record_sets: BTreeMap<&str, &ResearchRecordSet> = model
        .record_sets
        .iter()
        .map(|record_set| (record_set.id.as_str(), record_set))
        .collect();
    insert_values(
        &mut root,
        terms.term(CroissantRole::RecordSet),
        encode_entities(&model.dataset.record_sets, |id| {
            record_sets
                .get(id)
                .copied()
                .ok_or_else(|| {
                    ProjectionError::integrity(format!("missing Croissant record set `{id}`"))
                })
                .map(|record_set| encode_record_set(record_set, config))
        })?,
    );
    Ok(Value::Object(root))
}

fn encode_entities(
    ids: &[String],
    mut encode: impl FnMut(&str) -> Result<Value, ProjectionError>,
) -> Result<Vec<Value>, ProjectionError> {
    ids.iter().map(|id| encode(id)).collect()
}

fn encode_agent(agent: &ResearchAgent, config: &CroissantConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&agent.id, terms.term(CroissantRole::AgentClass));
    insert_values(
        &mut object,
        terms.term(CroissantRole::Name),
        encode_texts(&agent.names, config),
    );
    Value::Object(object)
}

fn encode_resource(
    resource: &ResearchResource,
    config: &CroissantConfig,
    ledger: &mut LossLedger,
) -> Result<Value, ProjectionError> {
    let terms = config.vocabulary();
    let mut object = typed_object(&resource.id, terms.term(CroissantRole::FileObjectClass));
    insert_values(
        &mut object,
        terms.term(CroissantRole::Name),
        encode_texts(&resource.names, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Description),
        encode_texts(&resource.descriptions, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Path),
        resource.paths.iter().cloned().map(Value::String).collect(),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::ContentUrl),
        encode_values(&resource.urls, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::EncodingFormat),
        encode_texts(&resource.media_types, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Format),
        encode_values(&resource.formats, config),
    );
    if let Some(byte_size) = resource.byte_size {
        object.insert(
            terms.term(CroissantRole::ContentSize).to_owned(),
            Value::Number(byte_size.into()),
        );
    }

    let contract = rdf_to_research_object_loss_ledger(CROISSANT_PROFILE);
    let mut sha256 = Vec::new();
    for checksum in &resource.checksums {
        if is_sha256(&checksum.algorithm) {
            sha256.push(encode_text(&checksum.value, config));
        } else {
            record_loss(
                ledger,
                &contract,
                LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED,
                CROISSANT_ARTIFACT,
                &resource.id,
            );
        }
    }
    insert_values(&mut object, terms.term(CroissantRole::Sha256), sha256);
    Ok(Value::Object(object))
}

fn encode_activity(
    activity: &ResearchActivity,
    agents: &BTreeMap<&str, &ResearchAgent>,
    config: &CroissantConfig,
) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&activity.id, terms.term(CroissantRole::ActivityClass));
    insert_values(
        &mut object,
        terms.term(CroissantRole::Name),
        encode_texts(&activity.names, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Instrument),
        encode_values(&activity.instruments, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Agent),
        activity
            .actors
            .iter()
            .map(|id| {
                agents
                    .get(id.as_str())
                    .map_or_else(|| id_object(id), |agent| encode_agent(agent, config))
            })
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Object),
        activity.objects.iter().map(|id| id_object(id)).collect(),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Result),
        activity.results.iter().map(|id| id_object(id)).collect(),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::EndTime),
        encode_texts(&activity.end_times, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Workflow),
        encode_values(&activity.workflows, config),
    );
    Value::Object(object)
}

fn encode_record_set(record_set: &ResearchRecordSet, config: &CroissantConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&record_set.id, terms.term(CroissantRole::RecordSetClass));
    insert_values(
        &mut object,
        terms.term(CroissantRole::Name),
        encode_texts(&record_set.names, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Description),
        encode_texts(&record_set.descriptions, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Field),
        record_set
            .fields
            .iter()
            .map(|field| encode_field(field, config))
            .collect(),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::Records),
        record_set.rows.clone(),
    );
    Value::Object(object)
}

fn encode_field(field: &ResearchField, config: &CroissantConfig) -> Value {
    let terms = config.vocabulary();
    let mut object = typed_object(&field.id, terms.term(CroissantRole::FieldClass));
    insert_values(
        &mut object,
        terms.term(CroissantRole::Name),
        encode_texts(&field.names, config),
    );
    insert_values(
        &mut object,
        terms.term(CroissantRole::DataType),
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

fn encode_texts(values: &[ResearchText], config: &CroissantConfig) -> Vec<Value> {
    values
        .iter()
        .map(|value| encode_text(value, config))
        .collect()
}

fn encode_values(values: &[ResearchValue], config: &CroissantConfig) -> Vec<Value> {
    values
        .iter()
        .map(|value| match value {
            ResearchValue::Iri { value } => id_object(value),
            ResearchValue::Text(value) => encode_text(value, config),
        })
        .collect()
}

fn encode_text(value: &ResearchText, config: &CroissantConfig) -> Value {
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

fn is_sha256(value: &ResearchValue) -> bool {
    let candidate = match value {
        ResearchValue::Iri { value } => value
            .rsplit(['/', '#', ':'])
            .next()
            .unwrap_or(value)
            .to_ascii_lowercase(),
        ResearchValue::Text(value) => value.value.to_ascii_lowercase(),
    };
    matches!(candidate.as_str(), "sha256" | "sha-256")
}

struct Decoder<'a> {
    config: &'a CroissantConfig,
    contract: &'a LossLedger,
    ledger: &'a mut LossLedger,
    agents: BTreeMap<String, ResearchAgent>,
}

fn decode_document(
    value: Value,
    config: &CroissantConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<ResearchObjectModel, ProjectionError> {
    let mut decoder = Decoder {
        config,
        contract,
        ledger,
        agents: BTreeMap::new(),
    };
    decoder.decode(value)
}

impl Decoder<'_> {
    fn decode(&mut self, value: Value) -> Result<ResearchObjectModel, ProjectionError> {
        let Value::Object(mut root) = value else {
            return Err(
                ProjectionError::syntax("Croissant document root must be a JSON object")
                    .at_path(CROISSANT_ARTIFACT),
            );
        };
        let context = root.remove("@context").ok_or_else(|| {
            ProjectionError::integrity("Croissant document is missing @context")
                .at_path(CROISSANT_ARTIFACT)
        })?;
        if context != *self.config.context().value() {
            return Err(ProjectionError::integrity(
                "Croissant @context does not exactly match caller configuration",
            )
            .at_path(CROISSANT_ARTIFACT));
        }
        let id = self.required_entity_id(&mut root, "", false)?;
        if id != self.config.common().identity().dataset_iri() {
            return Err(ProjectionError::integrity(format!(
                "Croissant dataset identity `{id}` does not match configured identity `{}`",
                self.config.common().identity().dataset_iri()
            ))
            .at_path(CROISSANT_ARTIFACT));
        }
        self.require_type(
            &mut root,
            "",
            self.config.vocabulary().term(CroissantRole::DatasetClass),
        )?;
        self.require_profile(&mut root)?;

        let titles = self.take_texts(&mut root, CroissantRole::Name, "")?;
        let descriptions = self.take_texts(&mut root, CroissantRole::Description, "")?;
        let identifiers = self.take_values(&mut root, CroissantRole::Identifier, "")?;
        let versions = self.take_texts(&mut root, CroissantRole::Version, "")?;
        let issued = self.take_texts(&mut root, CroissantRole::DatePublished, "")?;
        let modified = self.take_texts(&mut root, CroissantRole::DateModified, "")?;
        let landing_pages = self.take_values(&mut root, CroissantRole::Url, "")?;
        let keywords = self.take_texts(&mut root, CroissantRole::Keywords, "")?;
        let licenses = self.take_values(&mut root, CroissantRole::License, "")?;
        let creators = self.take_agents(&mut root, CroissantRole::Creator, "")?;
        let publishers = self.take_agents(&mut root, CroissantRole::Publisher, "")?;
        let resources = self.take_resources(&mut root, "")?;
        let activities = self.take_activities(&mut root, "")?;
        let record_sets = self.take_record_sets(&mut root, "")?;
        self.record_unknowns(&root, "");

        Ok(ResearchObjectModel {
            dataset: ResearchDataset {
                id,
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
                resources: resources
                    .iter()
                    .map(|resource| resource.id.clone())
                    .collect(),
                activities: activities
                    .iter()
                    .map(|activity| activity.id.clone())
                    .collect(),
                record_sets: record_sets
                    .iter()
                    .map(|record_set| record_set.id.clone())
                    .collect(),
            },
            agents: std::mem::take(&mut self.agents).into_values().collect(),
            resources,
            activities,
            record_sets,
        })
    }

    fn require_profile(&mut self, root: &mut Map<String, Value>) -> Result<(), ProjectionError> {
        let term = self.config.vocabulary().term(CroissantRole::ConformsTo);
        let pointer = json_pointer("", term);
        let values = self.take_items(root, CroissantRole::ConformsTo, "");
        if values.len() != 1 {
            return Err(ProjectionError::integrity(
                "Croissant document requires exactly one conformsTo value",
            )
            .at_path(CROISSANT_ARTIFACT));
        }
        let profile = self.parse_reference(&values[0], &pointer)?;
        if profile != self.config.profile_iri() {
            return Err(ProjectionError::integrity(format!(
                "Croissant conformsTo `{profile}` does not match configured profile `{}`",
                self.config.profile_iri()
            ))
            .at_path(CROISSANT_ARTIFACT));
        }
        Ok(())
    }

    fn take_agents(
        &mut self,
        object: &mut Map<String, Value>,
        role: CroissantRole,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut ids = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Some(mut agent) = self.decode_agent(value, &pointer)? else {
                continue;
            };
            let id = agent.id.clone();
            if let Some(existing) = self.agents.get_mut(&id) {
                existing.names.append(&mut agent.names);
            } else {
                self.agents.insert(id.clone(), agent);
            }
            ids.push(id);
        }
        Ok(ids)
    }

    fn decode_agent(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchAgent>, ProjectionError> {
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let id = self.required_entity_id(&mut object, pointer, true)?;
        self.optional_type(
            &mut object,
            pointer,
            self.config.vocabulary().term(CroissantRole::AgentClass),
        )?;
        let names = self.take_texts(&mut object, CroissantRole::Name, pointer)?;
        self.record_unknowns(&object, pointer);
        Ok(Some(ResearchAgent { id, names }))
    }

    fn take_resources(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchResource>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::Distribution)
            .to_owned();
        let mut resources = Vec::new();
        let mut ids = BTreeSet::new();
        for (index, value) in self
            .take_items(object, CroissantRole::Distribution, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Some(resource) = self.decode_resource(value, &pointer)? else {
                continue;
            };
            if !ids.insert(resource.id.clone()) {
                return Err(ProjectionError::integrity(format!(
                    "duplicate Croissant resource identity `{}`",
                    resource.id
                ))
                .at_path(CROISSANT_ARTIFACT));
            }
            resources.push(resource);
        }
        Ok(resources)
    }

    fn decode_resource(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchResource>, ProjectionError> {
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let id = self.required_entity_id(&mut object, pointer, true)?;
        self.optional_type(
            &mut object,
            pointer,
            self.config
                .vocabulary()
                .term(CroissantRole::FileObjectClass),
        )?;
        let names = self.take_texts(&mut object, CroissantRole::Name, pointer)?;
        let descriptions = self.take_texts(&mut object, CroissantRole::Description, pointer)?;
        let paths = self.take_paths(&mut object, pointer)?;
        let urls = self.take_values(&mut object, CroissantRole::ContentUrl, pointer)?;
        let media_types = self.take_texts(&mut object, CroissantRole::EncodingFormat, pointer)?;
        let formats = self.take_values(&mut object, CroissantRole::Format, pointer)?;
        let byte_size = self.take_byte_size(&mut object, pointer);
        let checksums = self.take_sha256(&mut object, pointer)?;

        let inline_term = self.config.vocabulary().term(CroissantRole::InlineContent);
        if object.remove(inline_term).is_some() {
            self.loss(
                LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED,
                &json_pointer(pointer, inline_term),
            );
        }
        self.record_unknowns(&object, pointer);
        Ok(Some(ResearchResource {
            id,
            names,
            descriptions,
            paths,
            urls,
            media_types,
            formats,
            byte_size,
            checksums,
        }))
    }

    fn take_activities(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchActivity>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::Activity)
            .to_owned();
        let mut activities = Vec::new();
        let mut ids = BTreeSet::new();
        for (index, value) in self
            .take_items(object, CroissantRole::Activity, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Some(activity) = self.decode_activity(value, &pointer)? else {
                continue;
            };
            if !ids.insert(activity.id.clone()) {
                return Err(ProjectionError::integrity(format!(
                    "duplicate Croissant activity identity `{}`",
                    activity.id
                ))
                .at_path(CROISSANT_ARTIFACT));
            }
            activities.push(activity);
        }
        Ok(activities)
    }

    fn decode_activity(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchActivity>, ProjectionError> {
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let id = self.required_entity_id(&mut object, pointer, true)?;
        self.optional_type(
            &mut object,
            pointer,
            self.config.vocabulary().term(CroissantRole::ActivityClass),
        )?;
        let names = self.take_texts(&mut object, CroissantRole::Name, pointer)?;
        let instruments = self.take_values(&mut object, CroissantRole::Instrument, pointer)?;
        let actors = self.take_activity_agents(&mut object, pointer)?;
        let objects = self.take_references(&mut object, CroissantRole::Object, pointer)?;
        let results = self.take_references(&mut object, CroissantRole::Result, pointer)?;
        let end_times = self.take_texts(&mut object, CroissantRole::EndTime, pointer)?;
        let workflows = self.take_values(&mut object, CroissantRole::Workflow, pointer)?;
        self.record_unknowns(&object, pointer);
        Ok(Some(ResearchActivity {
            id,
            names,
            instruments,
            actors,
            objects,
            results,
            end_times,
            workflows,
        }))
    }

    fn take_record_sets(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchRecordSet>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::RecordSet)
            .to_owned();
        let mut record_sets = Vec::new();
        let mut ids = BTreeSet::new();
        for (index, value) in self
            .take_items(object, CroissantRole::RecordSet, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Some(record_set) = self.decode_record_set(value, &pointer)? else {
                continue;
            };
            if !ids.insert(record_set.id.clone()) {
                return Err(ProjectionError::integrity(format!(
                    "duplicate Croissant record-set identity `{}`",
                    record_set.id
                ))
                .at_path(CROISSANT_ARTIFACT));
            }
            record_sets.push(record_set);
        }
        Ok(record_sets)
    }

    fn take_activity_agents(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::Agent)
            .to_owned();
        let mut ids = Vec::new();
        for (index, value) in self
            .take_items(object, CroissantRole::Agent, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            if value.as_object().is_some_and(|value| {
                value.contains_key("@type")
                    || value.contains_key(self.config.vocabulary().term(CroissantRole::Name))
            }) {
                if let Some(mut agent) = self.decode_agent(value, &pointer)? {
                    let id = agent.id.clone();
                    if let Some(existing) = self.agents.get_mut(&id) {
                        existing.names.append(&mut agent.names);
                    } else {
                        self.agents.insert(id.clone(), agent);
                    }
                    ids.push(id);
                }
            } else {
                ids.push(self.parse_reference(&value, &pointer)?);
            }
        }
        Ok(ids)
    }

    fn decode_record_set(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchRecordSet>, ProjectionError> {
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let id = self.required_entity_id(&mut object, pointer, true)?;
        self.optional_type(
            &mut object,
            pointer,
            self.config.vocabulary().term(CroissantRole::RecordSetClass),
        )?;
        let names = self.take_texts(&mut object, CroissantRole::Name, pointer)?;
        let descriptions = self.take_texts(&mut object, CroissantRole::Description, pointer)?;
        let fields = self.take_fields(&mut object, pointer)?;
        let rows = self.take_items(&mut object, CroissantRole::Records, pointer);
        self.record_unknowns(&object, pointer);
        Ok(Some(ResearchRecordSet {
            id,
            names,
            descriptions,
            fields,
            rows,
        }))
    }

    fn take_fields(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchField>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::Field)
            .to_owned();
        let mut fields = Vec::new();
        let mut ids = BTreeSet::new();
        for (index, value) in self
            .take_items(object, CroissantRole::Field, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Some(field) = self.decode_field(value, &pointer)? else {
                continue;
            };
            if !ids.insert(field.id.clone()) {
                return Err(ProjectionError::integrity(format!(
                    "duplicate Croissant field identity `{}`",
                    field.id
                ))
                .at_path(CROISSANT_ARTIFACT));
            }
            fields.push(field);
        }
        Ok(fields)
    }

    fn decode_field(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchField>, ProjectionError> {
        let Value::Object(mut object) = value else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let id = self.required_entity_id(&mut object, pointer, true)?;
        self.optional_type(
            &mut object,
            pointer,
            self.config.vocabulary().term(CroissantRole::FieldClass),
        )?;
        let names = self.take_texts(&mut object, CroissantRole::Name, pointer)?;
        let data_types = self.take_values(&mut object, CroissantRole::DataType, pointer)?;
        self.record_unknowns(&object, pointer);
        Ok(Some(ResearchField {
            id,
            names,
            data_types,
        }))
    }

    fn take_items(
        &mut self,
        object: &mut Map<String, Value>,
        role: CroissantRole,
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
        role: CroissantRole,
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
        role: CroissantRole,
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
            return self
                .parse_reference(&value, pointer)
                .and_then(|value| ResearchValue::iri(value).map(Some));
        }
        self.parse_text(value, pointer)
            .map(|value| value.map(ResearchValue::Text))
    }

    fn take_references(
        &mut self,
        object: &mut Map<String, Value>,
        role: CroissantRole,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self.config.vocabulary().term(role).to_owned();
        let mut references = Vec::new();
        for (index, value) in self
            .take_items(object, role, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            references.push(self.parse_reference(&value, &pointer)?);
        }
        Ok(references)
    }

    fn parse_reference(&mut self, value: &Value, pointer: &str) -> Result<String, ProjectionError> {
        match value {
            Value::String(value) => self.resolve_native_id(value, pointer, true),
            Value::Object(object) => {
                let Some(Value::String(value)) = object.get("@id") else {
                    self.unsupported(pointer);
                    return Err(ProjectionError::integrity(
                        "Croissant reference requires a string @id",
                    )
                    .at_path(CROISSANT_ARTIFACT));
                };
                for key in object.keys().filter(|key| key.as_str() != "@id") {
                    self.loss(
                        LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                        &json_pointer(pointer, key),
                    );
                }
                self.resolve_native_id(value, pointer, true)
            }
            _ => {
                self.unsupported(pointer);
                Err(ProjectionError::integrity(
                    "Croissant reference must be a string or @id object",
                )
                .at_path(CROISSANT_ARTIFACT))
            }
        }
    }

    fn take_paths(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::Path)
            .to_owned();
        let mut paths = Vec::new();
        for (index, value) in self
            .take_items(object, CroissantRole::Path, parent)
            .into_iter()
            .enumerate()
        {
            let pointer = item_pointer(parent, &term, index);
            let Value::String(path) = value else {
                self.unsupported(&pointer);
                continue;
            };
            validate_safe_path(&path).map_err(|error| error.at_path(CROISSANT_ARTIFACT))?;
            paths.push(path);
        }
        Ok(paths)
    }

    fn take_byte_size(&mut self, object: &mut Map<String, Value>, parent: &str) -> Option<u64> {
        let term = self
            .config
            .vocabulary()
            .term(CroissantRole::ContentSize)
            .to_owned();
        let value = object.remove(&term)?;
        let Some(value) = value.as_u64() else {
            self.unsupported(&json_pointer(parent, &term));
            return None;
        };
        Some(value)
    }

    fn take_sha256(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
    ) -> Result<Vec<ResearchChecksum>, ProjectionError> {
        let values = self.take_texts(object, CroissantRole::Sha256, parent)?;
        let xsd_string = self
            .config
            .common()
            .roles()
            .iri(super::ResearchRole::XsdString)
            .to_owned();
        values
            .into_iter()
            .map(|value| {
                Ok(ResearchChecksum {
                    algorithm: ResearchValue::Text(ResearchText::plain("sha256", &xsd_string)?),
                    value,
                })
            })
            .collect()
    }

    fn required_entity_id(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
        allow_relative: bool,
    ) -> Result<String, ProjectionError> {
        let pointer = json_pointer(parent, "@id");
        let Some(Value::String(id)) = object.remove("@id") else {
            return Err(
                ProjectionError::integrity("Croissant entity requires one string @id")
                    .at_path(CROISSANT_ARTIFACT),
            );
        };
        self.resolve_native_id(&id, &pointer, allow_relative)
    }

    fn resolve_native_id(
        &mut self,
        id: &str,
        pointer: &str,
        allow_relative: bool,
    ) -> Result<String, ProjectionError> {
        if validate_absolute_iri(id, "Croissant entity identity").is_ok() {
            return Ok(id.to_owned());
        }
        if !allow_relative {
            return Err(ProjectionError::integrity(format!(
                "Croissant root identity `{id}` must be absolute"
            ))
            .at_path(CROISSANT_ARTIFACT));
        }
        let resolved = self.config.common().identity().resolve_relative(id)?;
        self.loss(LOSS_RESEARCH_LOCAL_ID_RESOLVED, pointer);
        Ok(resolved)
    }

    fn require_type(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
        expected: &str,
    ) -> Result<(), ProjectionError> {
        let Some(value) = object.remove("@type") else {
            return Err(ProjectionError::integrity(format!(
                "Croissant entity at `{parent}` requires @type `{expected}`"
            ))
            .at_path(CROISSANT_ARTIFACT));
        };
        self.validate_type(value, parent, expected)
    }

    fn optional_type(
        &mut self,
        object: &mut Map<String, Value>,
        parent: &str,
        expected: &str,
    ) -> Result<(), ProjectionError> {
        let Some(value) = object.remove("@type") else {
            return Ok(());
        };
        self.validate_type(value, parent, expected)
    }

    fn validate_type(
        &mut self,
        value: Value,
        parent: &str,
        expected: &str,
    ) -> Result<(), ProjectionError> {
        let values = match value {
            Value::String(value) => vec![value],
            Value::Array(values) => {
                if values.len() > 1 {
                    self.loss(LOSS_RESEARCH_ORDER_DROPPED, &json_pointer(parent, "@type"));
                }
                values
                    .into_iter()
                    .map(|value| {
                        value.as_str().map(str::to_owned).ok_or_else(|| {
                            ProjectionError::integrity(
                                "Croissant @type array must contain only strings",
                            )
                            .at_path(CROISSANT_ARTIFACT)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => {
                return Err(ProjectionError::integrity(
                    "Croissant @type must be a string or string array",
                )
                .at_path(CROISSANT_ARTIFACT));
            }
        };
        if values.len() != 1 || values[0] != expected {
            return Err(ProjectionError::integrity(format!(
                "Croissant entity at `{parent}` must have exactly type `{expected}`"
            ))
            .at_path(CROISSANT_ARTIFACT));
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
        record_loss(
            self.ledger,
            self.contract,
            code,
            CROISSANT_ARTIFACT,
            pointer,
        );
    }
}

fn item_pointer(parent: &str, term: &str, index: usize) -> String {
    format!("{}/{index}", json_pointer(parent, term))
}

fn validate_safe_path(path: &str) -> Result<(), ProjectionError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(['?', '#'])
        || path
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(ProjectionError::integrity(format!(
            "unsafe Croissant FileObject path `{path}`"
        )));
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
        include_bytes!("../../../tests/fixtures/research-objects/croissant-1.1/input.json");
    const GOLDEN: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/croissant-1.1/golden.json");

    fn config() -> CroissantConfig {
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
        let limits = ProjectionLimits::new(4, 200_000, 400_000, 500_000, 12).expect("limits");
        let policy = ResearchObjectPolicy::new(limits, 10_000, 1_000, 5_000, 12)
            .expect("research-object policy");
        let common = ResearchObjectConfig::new(roles, identity, policy);

        let vocabulary = CROISSANT_ROLES
            .iter()
            .copied()
            .map(|role| (role, test_term(role).to_owned()))
            .collect();
        let vocabulary = CroissantVocabulary::new(vocabulary).expect("Croissant vocabulary");
        let definitions = CROISSANT_ROLES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, role)| {
                (
                    test_term(role).to_owned(),
                    format!("https://example.org/croissant/term-{index}"),
                )
            })
            .collect();
        let context = OfflineJsonLdContext::new(
            Value::String("https://example.org/context/croissant-1.1".to_owned()),
            definitions,
        )
        .expect("offline context");
        CroissantConfig::new(
            common,
            context,
            vocabulary,
            "https://example.org/profiles/croissant-1.1",
        )
        .expect("Croissant config")
    }

    fn test_term(role: CroissantRole) -> &'static str {
        match role {
            CroissantRole::DatasetClass => "sc:Dataset",
            CroissantRole::FileObjectClass => "cr:FileObject",
            CroissantRole::RecordSetClass => "cr:RecordSet",
            CroissantRole::FieldClass => "cr:Field",
            CroissantRole::AgentClass => "sc:Person",
            CroissantRole::ActivityClass => "sc:CreateAction",
            CroissantRole::Name => "name",
            CroissantRole::Description => "description",
            CroissantRole::Identifier => "identifier",
            CroissantRole::Version => "version",
            CroissantRole::DatePublished => "datePublished",
            CroissantRole::DateModified => "dateModified",
            CroissantRole::Url => "url",
            CroissantRole::Keywords => "keywords",
            CroissantRole::License => "license",
            CroissantRole::Creator => "creator",
            CroissantRole::Publisher => "publisher",
            CroissantRole::Distribution => "distribution",
            CroissantRole::Activity => "activity",
            CroissantRole::RecordSet => "recordSet",
            CroissantRole::ConformsTo => "conformsTo",
            CroissantRole::Path => "path",
            CroissantRole::ContentUrl => "contentUrl",
            CroissantRole::EncodingFormat => "encodingFormat",
            CroissantRole::Format => "format",
            CroissantRole::ContentSize => "contentSize",
            CroissantRole::Sha256 => "sha256",
            CroissantRole::InlineContent => "content",
            CroissantRole::Field => "field",
            CroissantRole::DataType => "dataType",
            CroissantRole::Records => "records",
            CroissantRole::Instrument => "instrument",
            CroissantRole::Agent => "agent",
            CroissantRole::Object => "object",
            CroissantRole::Result => "result",
            CroissantRole::EndTime => "endTime",
            CroissantRole::Workflow => "workflow",
        }
    }

    fn package(bytes: impl Into<Vec<u8>>) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(config().common().limits(), [(CROISSANT_ARTIFACT, bytes)])
            .expect("package")
    }

    #[test]
    fn fixture_has_exact_located_losses_and_stable_rewrite() {
        let config = config();
        let read = read_croissant(&package(INPUT), &config).expect("read fixture");
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

        let projected = project_croissant(&read.dataset, &config).expect("project fixture");
        let actual = projected
            .package
            .get(CROISSANT_ARTIFACT)
            .expect("Croissant artifact");
        assert_eq!(
            actual,
            GOLDEN,
            "actual golden bytes: {}",
            String::from_utf8_lossy(actual)
        );
        assert_eq!(
            projected.package.to_ustar().expect("archive"),
            projected.package.to_ustar().expect("archive")
        );

        let reread = read_croissant(&projected.package, &config).expect("read canonical output");
        let reprojected = project_croissant(&reread.dataset, &config).expect("rewrite");
        assert_eq!(projected.package, reprojected.package);
        assert_eq!(projected.model, reread.model);
    }

    #[test]
    fn config_requires_complete_offline_expansion() {
        let config = config();
        let mut definitions = config.context().definitions().clone();
        definitions.remove(test_term(CroissantRole::Name));
        let context = OfflineJsonLdContext::new(config.context().value().clone(), definitions)
            .expect("context is independently valid");
        assert!(
            CroissantConfig::new(
                config.common().clone(),
                context,
                config.vocabulary().clone(),
                config.profile_iri(),
            )
            .is_err()
        );
    }

    #[test]
    fn reader_rejects_duplicates_context_type_and_path_escape() {
        let config = config();
        let duplicate = br#"{"@context":{},"@context":{}}"#;
        assert!(read_croissant(&package(duplicate), &config).is_err());

        let input = String::from_utf8(INPUT.to_vec()).expect("UTF-8 fixture");
        let wrong_context = input.replace(
            "https://example.org/context/croissant-1.1",
            "https://example.org/context/wrong",
        );
        assert!(read_croissant(&package(wrong_context), &config).is_err());

        let wrong_type = input.replace("sc:Dataset", "sc:WrongDataset");
        assert!(read_croissant(&package(wrong_type), &config).is_err());

        let path_escape = input.replace("data/train.csv", "../train.csv");
        assert!(read_croissant(&package(path_escape), &config).is_err());
    }
}
