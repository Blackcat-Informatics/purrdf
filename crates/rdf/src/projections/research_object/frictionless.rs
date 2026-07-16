// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED, LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED,
    LOSS_RESEARCH_LOCAL_ID_RESOLVED, LOSS_RESEARCH_ORDER_DROPPED,
    LOSS_RESEARCH_PROFILE_FIELD_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
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
    ResearchAgent, ResearchChecksum, ResearchDataset, ResearchObjectConfig, ResearchObjectModel,
    ResearchResource, ResearchText, ResearchValue, lift_research_object, project_research_object,
};

/// Closed Frictionless Data Package profile identifier.
pub const FRICTIONLESS_PROFILE: &str = "frictionless-data-package-1";
/// Sole artifact path in the canonical Frictionless package.
pub const FRICTIONLESS_ARTIFACT: &str = "datapackage.json";

/// Mandatory caller-owned Frictionless Data Package v1 configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrictionlessConfig {
    common: ResearchObjectConfig,
    package_profile: String,
    package_name: String,
}

impl FrictionlessConfig {
    /// Construct a configuration without supplying a library profile or identity default.
    ///
    /// # Errors
    ///
    /// Rejects an empty/control-bearing profile identity or a package name outside
    /// the Data Package v1 lowercase identifier grammar.
    pub fn new(
        common: ResearchObjectConfig,
        package_profile: impl Into<String>,
        package_name: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let package_profile = package_profile.into();
        validate_profile_identity(&package_profile)?;
        let package_name = package_name.into();
        validate_package_name(&package_name, "Frictionless package name")?;
        Ok(Self {
            common,
            package_profile,
            package_name,
        })
    }

    /// Shared RDF vocabulary, identity, and limits.
    pub const fn common(&self) -> &ResearchObjectConfig {
        &self.common
    }

    /// Exact caller-selected Data Package profile identity.
    pub fn package_profile(&self) -> &str {
        &self.package_profile
    }

    /// Exact caller-selected package name.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrictionlessConfig {
    common: ResearchObjectConfig,
    package_profile: String,
    package_name: String,
}

impl<'de> Deserialize<'de> for FrictionlessConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawFrictionlessConfig::deserialize(deserializer)?;
        Self::new(raw.common, raw.package_profile, raw.package_name)
            .map_err(serde::de::Error::custom)
    }
}

/// Project caller-vocabulary RDF 1.2 into canonical Data Package v1 JSON.
///
/// # Errors
///
/// Returns typed mapping, configuration, identity, resource-shape, or caller
/// resource-limit failures. Every representational loss is returned.
pub fn project_frictionless<D: DatasetView>(
    view: &D,
    config: &FrictionlessConfig,
) -> Result<ResearchObjectPackageProjection, ProjectionError> {
    let projection = project_research_object(view, FRICTIONLESS_PROFILE, config.common())?;
    let mut ledger = projection.loss_ledger;
    let contract = rdf_to_research_object_loss_ledger(FRICTIONLESS_PROFILE);
    let document = encode_document(&projection.model, config, &contract, &mut ledger)?;
    ensure_sound(&ledger, "rdf-1.2-dataset", FRICTIONLESS_PROFILE)?;
    let bytes = canonical_json(
        &document,
        config.common().limits(),
        "Frictionless Data Package v1 JSON",
    )?;
    let package = ProjectionPackage::from_artifacts(
        config.common().limits(),
        [(FRICTIONLESS_ARTIFACT, bytes)],
    )?;
    Ok(ResearchObjectPackageProjection {
        package,
        model: projection.model,
        loss_ledger: ledger,
    })
}

/// Read strict Data Package v1 JSON and lift caller-vocabulary RDF 1.2.
///
/// # Errors
///
/// Rejects unexpected artifacts, duplicate members, profile/name/identity
/// drift, unsafe paths or resource names, malformed standard members, invalid
/// hashes, duplicate resource identities, or configured limit excesses.
pub fn read_frictionless(
    package: &ProjectionPackage,
    config: &FrictionlessConfig,
) -> Result<ResearchObjectReadOutcome, ProjectionError> {
    let bytes = require_artifact(package, FRICTIONLESS_ARTIFACT, config.common())?;
    let value = parse_strict_json(
        bytes,
        config.common(),
        "Frictionless Data Package v1 JSON",
        FRICTIONLESS_ARTIFACT,
    )?;
    let contract = research_object_to_rdf_loss_ledger(FRICTIONLESS_PROFILE);
    let mut ledger = LossLedger::new();
    let model = FrictionlessDecoder {
        config,
        contract: &contract,
        ledger: &mut ledger,
    }
    .decode(value)?
    .normalize(config.common().policy())?;
    ensure_sound(&ledger, FRICTIONLESS_PROFILE, "rdf-1.2-dataset")?;
    let dataset = lift_research_object(model.clone(), config.common())?;
    Ok(ResearchObjectReadOutcome {
        dataset: normalize_lifted_jsonld(&dataset)?,
        model,
        loss_ledger: ledger,
    })
}

fn encode_document(
    model: &ResearchObjectModel,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<Value, ProjectionError> {
    if model.dataset.resources.is_empty() {
        return Err(ProjectionError::integrity(
            "Frictionless Data Package v1 requires at least one resource",
        ));
    }

    let dataset = &model.dataset;
    let mut root = Map::from_iter([
        (
            "profile".to_owned(),
            Value::String(config.package_profile().to_owned()),
        ),
        (
            "name".to_owned(),
            Value::String(config.package_name().to_owned()),
        ),
        ("id".to_owned(), Value::String(dataset.id.clone())),
    ]);
    insert_required_single_text(
        &mut root,
        "title",
        &dataset.titles,
        "dataset:title",
        config,
        contract,
        ledger,
    )?;
    insert_single_text(
        &mut root,
        "description",
        &dataset.descriptions,
        "dataset:description",
        config,
        contract,
        ledger,
    );
    insert_single_value(
        &mut root,
        "homepage",
        &dataset.landing_pages,
        "dataset:homepage",
        config,
        contract,
        ledger,
        true,
    )?;
    insert_single_text(
        &mut root,
        "created",
        &dataset.issued,
        "dataset:created",
        config,
        contract,
        ledger,
    );
    insert_single_text(
        &mut root,
        "version",
        &dataset.versions,
        "dataset:version",
        config,
        contract,
        ledger,
    );

    if !dataset.keywords.is_empty() {
        root.insert(
            "keywords".to_owned(),
            Value::Array(
                dataset
                    .keywords
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        record_text_fidelity(
                            value,
                            config,
                            contract,
                            ledger,
                            &format!("dataset:keyword[{index}]"),
                        );
                        Value::String(value.value.clone())
                    })
                    .collect(),
            ),
        );
    }

    let licenses = encode_licenses(dataset, config, contract, ledger);
    if !licenses.is_empty() {
        root.insert("licenses".to_owned(), Value::Array(licenses));
    }

    let agents: BTreeMap<&str, &ResearchAgent> = model
        .agents
        .iter()
        .map(|agent| (agent.id.as_str(), agent))
        .collect();
    let contributors = encode_contributors(model, &agents, config, contract, ledger)?;
    if !contributors.is_empty() {
        root.insert("contributors".to_owned(), Value::Array(contributors));
    }

    let resources: BTreeMap<&str, &ResearchResource> = model
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource))
        .collect();
    let mut encoded_resources = Vec::new();
    for resource_id in &dataset.resources {
        let resource = resources
            .get(resource_id.as_str())
            .copied()
            .ok_or_else(|| {
                ProjectionError::integrity(format!("missing Frictionless resource `{resource_id}`"))
            })?;
        encoded_resources.push(encode_resource(resource, config, contract, ledger)?);
    }
    root.insert("resources".to_owned(), Value::Array(encoded_resources));

    for identifier in &dataset.identifiers {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("dataset:identifier:{}", value_lexical(identifier)),
        );
    }
    for (index, _) in dataset.modified.iter().enumerate() {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("dataset:modified[{index}]"),
        );
    }
    for activity in &model.activities {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &activity.id,
        );
    }
    for record_set in &model.record_sets {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &record_set.id,
        );
    }
    for resource in &model.resources {
        if !dataset.resources.contains(&resource.id) {
            forward_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &resource.id,
            );
        }
    }
    Ok(Value::Object(root))
}

fn encode_licenses(
    dataset: &ResearchDataset,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Vec<Value> {
    let mut encoded = Vec::new();
    for (index, license) in dataset.licenses.iter().enumerate() {
        let subject = format!("dataset:license[{index}]");
        match license {
            ResearchValue::Iri { value } => {
                encoded.push(Value::Object(Map::from_iter([(
                    "path".to_owned(),
                    Value::String(value.clone()),
                )])));
            }
            ResearchValue::Text(value) => {
                record_text_fidelity(value, config, contract, ledger, &subject);
                if validate_license_name(&value.value) {
                    encoded.push(Value::Object(Map::from_iter([(
                        "name".to_owned(),
                        Value::String(value.value.clone()),
                    )])));
                } else {
                    forward_loss(
                        ledger,
                        contract,
                        LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                        &subject,
                    );
                }
            }
        }
    }
    encoded
}

fn encode_contributors(
    model: &ResearchObjectModel,
    agents: &BTreeMap<&str, &ResearchAgent>,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<Vec<Value>, ProjectionError> {
    let mut linked = BTreeSet::new();
    let mut contributors = Vec::new();
    for (role, ids) in [
        ("author", &model.dataset.creators),
        ("publisher", &model.dataset.publishers),
    ] {
        for id in ids {
            linked.insert(id.as_str());
            contributors.push(encode_contributor(
                agents.get(id.as_str()).copied().ok_or_else(|| {
                    ProjectionError::integrity(format!(
                        "missing Frictionless contributor agent `{id}`"
                    ))
                })?,
                role,
                config,
                contract,
                ledger,
            )?);
        }
    }
    for agent in &model.agents {
        if !linked.contains(agent.id.as_str()) {
            contributors.push(encode_contributor(
                agent,
                "contributor",
                config,
                contract,
                ledger,
            )?);
        }
    }
    Ok(contributors)
}

fn encode_contributor(
    agent: &ResearchAgent,
    role: &str,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<Value, ProjectionError> {
    let name = agent.names.first().ok_or_else(|| {
        ProjectionError::integrity(format!(
            "Frictionless contributor `{}` requires at least one name",
            agent.id
        ))
    })?;
    record_text_fidelity(name, config, contract, ledger, &agent.id);
    for index in 1..agent.names.len() {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("{}:name[{index}]", agent.id),
        );
    }
    Ok(Value::Object(Map::from_iter([
        ("title".to_owned(), Value::String(name.value.clone())),
        ("path".to_owned(), Value::String(agent.id.clone())),
        ("role".to_owned(), Value::String(role.to_owned())),
    ])))
}

fn encode_resource(
    resource: &ResearchResource,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<Value, ProjectionError> {
    let native_name = resource
        .id
        .strip_prefix(config.common().identity().entity_base_iri())
        .ok_or_else(|| {
            ProjectionError::integrity(format!(
                "Frictionless resource `{}` is outside the caller entity base",
                resource.id
            ))
        })?;
    validate_package_name(native_name, "Frictionless resource name")?;
    let mut object = Map::from_iter([("name".to_owned(), Value::String(native_name.to_owned()))]);

    let mut paths = Vec::new();
    for path in &resource.paths {
        validate_data_path(path)?;
        paths.push(Value::String(path.clone()));
    }
    for (index, url) in resource.urls.iter().enumerate() {
        match url {
            ResearchValue::Iri { value } => paths.push(Value::String(value.clone())),
            ResearchValue::Text(value)
                if validate_absolute_iri(value.value.as_str(), "Frictionless resource URL")
                    .is_ok() =>
            {
                record_text_fidelity(
                    value,
                    config,
                    contract,
                    ledger,
                    &format!("{}:url[{index}]", resource.id),
                );
                paths.push(Value::String(value.value.clone()));
            }
            ResearchValue::Text(_) => forward_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &format!("{}:url[{index}]", resource.id),
            ),
        }
    }
    if paths.is_empty() {
        // `native_name` is the caller-bounded relative identity obtained by
        // removing `entity_base_iri`. Data Package permits the same safe value
        // as both resource name and path, which keeps identifier-only resources
        // from stricter citation carriers usable without fabricating an IRI.
        validate_data_path(native_name)?;
        paths.push(Value::String(native_name.to_owned()));
    }
    object.insert("path".to_owned(), Value::Array(paths));

    insert_single_text(
        &mut object,
        "title",
        &resource.names,
        &format!("{}:title", resource.id),
        config,
        contract,
        ledger,
    );
    insert_single_text(
        &mut object,
        "description",
        &resource.descriptions,
        &format!("{}:description", resource.id),
        config,
        contract,
        ledger,
    );
    insert_single_text(
        &mut object,
        "mediatype",
        &resource.media_types,
        &format!("{}:mediatype", resource.id),
        config,
        contract,
        ledger,
    );
    insert_single_value(
        &mut object,
        "format",
        &resource.formats,
        &format!("{}:format", resource.id),
        config,
        contract,
        ledger,
        false,
    )?;
    if let Some(bytes) = resource.byte_size {
        object.insert("bytes".to_owned(), Value::Number(bytes.into()));
    }
    if let Some(hash) = encode_hash(resource, config, contract, ledger) {
        object.insert("hash".to_owned(), Value::String(hash));
    }
    Ok(Value::Object(object))
}

fn encode_hash(
    resource: &ResearchResource,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Option<String> {
    let checksum = resource.checksums.first()?;
    for index in 1..resource.checksums.len() {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("{}:checksum[{index}]", resource.id),
        );
    }
    let algorithm = value_lexical(&checksum.algorithm);
    record_value_fidelity(
        &checksum.algorithm,
        config,
        contract,
        ledger,
        &format!("{}:checksum-algorithm", resource.id),
    );
    record_text_fidelity(
        &checksum.value,
        config,
        contract,
        ledger,
        &format!("{}:checksum-value", resource.id),
    );
    if !validate_hash_algorithm(&algorithm) || !is_hex(&checksum.value.value) {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("{}:checksum", resource.id),
        );
        return None;
    }
    Some(format!("{algorithm}:{}", checksum.value.value))
}

fn insert_required_single_text(
    object: &mut Map<String, Value>,
    member: &str,
    values: &[ResearchText],
    subject: &str,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<(), ProjectionError> {
    if values.is_empty() {
        return Err(ProjectionError::integrity(format!(
            "Frictionless `{member}` requires one value"
        )));
    }
    insert_single_text(object, member, values, subject, config, contract, ledger);
    Ok(())
}

fn insert_single_text(
    object: &mut Map<String, Value>,
    member: &str,
    values: &[ResearchText],
    subject: &str,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) {
    let Some(value) = values.first() else {
        return;
    };
    record_text_fidelity(value, config, contract, ledger, subject);
    object.insert(member.to_owned(), Value::String(value.value.clone()));
    for index in 1..values.len() {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("{subject}[{index}]"),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_single_value(
    object: &mut Map<String, Value>,
    member: &str,
    values: &[ResearchValue],
    subject: &str,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
    require_absolute: bool,
) -> Result<(), ProjectionError> {
    let Some(value) = values.first() else {
        return Ok(());
    };
    let lexical = value_lexical(value);
    if require_absolute {
        validate_absolute_iri(&lexical, &format!("Frictionless `{member}`"))?;
    }
    record_value_fidelity(value, config, contract, ledger, subject);
    object.insert(member.to_owned(), Value::String(lexical));
    for index in 1..values.len() {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &format!("{subject}[{index}]"),
        );
    }
    Ok(())
}

fn record_value_fidelity(
    value: &ResearchValue,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
    subject: &str,
) {
    if let ResearchValue::Text(value) = value {
        record_text_fidelity(value, config, contract, ledger, subject);
    }
}

fn record_text_fidelity(
    value: &ResearchText,
    config: &FrictionlessConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
    subject: &str,
) {
    if value.datatype != config.common().roles().iri(super::ResearchRole::XsdString)
        || value.language.is_some()
        || value.direction.is_some()
    {
        forward_loss(
            ledger,
            contract,
            LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED,
            subject,
        );
    }
}

fn forward_loss(ledger: &mut LossLedger, contract: &LossLedger, code: &'static str, subject: &str) {
    record_loss(ledger, contract, code, FRICTIONLESS_ARTIFACT, subject);
}

fn value_lexical(value: &ResearchValue) -> String {
    match value {
        ResearchValue::Iri { value } => value.clone(),
        ResearchValue::Text(value) => value.value.clone(),
    }
}

struct FrictionlessDecoder<'a> {
    config: &'a FrictionlessConfig,
    contract: &'a LossLedger,
    ledger: &'a mut LossLedger,
}

struct DecodedContributors {
    agents: Vec<ResearchAgent>,
    creators: Vec<String>,
    publishers: Vec<String>,
}

impl FrictionlessDecoder<'_> {
    fn decode(mut self, value: Value) -> Result<ResearchObjectModel, ProjectionError> {
        let Value::Object(mut root) = value else {
            return Err(
                ProjectionError::syntax("Frictionless document root must be an object")
                    .at_path(FRICTIONLESS_ARTIFACT),
            );
        };
        self.require_exact_string(&mut root, "profile", self.config.package_profile(), "")?;
        self.require_exact_string(&mut root, "name", self.config.package_name(), "")?;
        self.require_exact_string(
            &mut root,
            "id",
            self.config.common().identity().dataset_iri(),
            "",
        )?;

        let titles = vec![self.require_text(&mut root, "title", "")?];
        let descriptions = self
            .take_text(&mut root, "description", "")?
            .into_iter()
            .collect();
        let versions = self
            .take_text(&mut root, "version", "")?
            .into_iter()
            .collect();
        let issued = self
            .take_text(&mut root, "created", "")?
            .into_iter()
            .collect();
        let landing_pages = self
            .take_absolute_value(&mut root, "homepage", "")?
            .into_iter()
            .collect();
        let keywords = self.take_string_array(&mut root, "keywords", "")?;
        let licenses = self.decode_licenses(root.remove("licenses"))?;
        let contributors = self.decode_contributors(root.remove("contributors"))?;
        let resources = self.decode_resources(root.remove("resources"))?;
        let resource_ids = resources
            .iter()
            .map(|resource| resource.id.clone())
            .collect();

        for member in ["image", "sources", "notes"] {
            if root.remove(member).is_some() {
                self.unsupported(&json_pointer("", member));
            }
        }
        self.record_unknowns(&root, "");
        Ok(ResearchObjectModel {
            dataset: ResearchDataset {
                id: self.config.common().identity().dataset_iri().to_owned(),
                titles,
                descriptions,
                identifiers: Vec::new(),
                versions,
                issued,
                modified: Vec::new(),
                landing_pages,
                keywords,
                licenses,
                creators: contributors.creators,
                publishers: contributors.publishers,
                resources: resource_ids,
                activities: Vec::new(),
                record_sets: Vec::new(),
            },
            agents: contributors.agents,
            resources,
            activities: Vec::new(),
            record_sets: Vec::new(),
        })
    }

    fn decode_contributors(
        &mut self,
        value: Option<Value>,
    ) -> Result<DecodedContributors, ProjectionError> {
        let Some(value) = value else {
            return Ok(DecodedContributors {
                agents: Vec::new(),
                creators: Vec::new(),
                publishers: Vec::new(),
            });
        };
        let Value::Array(values) = value else {
            return Err(self.shape(
                "Frictionless contributors must be an array",
                "/contributors",
            ));
        };
        self.record_order(values.len(), "/contributors");
        let mut agents = BTreeMap::<String, ResearchAgent>::new();
        let mut creators = Vec::new();
        let mut publishers = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let pointer = format!("/contributors/{index}");
            let Value::Object(mut object) = value else {
                return Err(self.shape("Frictionless contributor must be an object", &pointer));
            };
            let title = self.require_string(&mut object, "title", &pointer)?;
            let id = self.require_string(&mut object, "path", &pointer)?;
            validate_absolute_iri(&id, "Frictionless contributor identity")
                .map_err(|error| self.shape(error.message(), &json_pointer(&pointer, "path")))?;
            let role = self
                .take_string(&mut object, "role", &pointer)?
                .unwrap_or_else(|| "contributor".to_owned());
            match role.as_str() {
                "author" => creators.push(id.clone()),
                "publisher" => publishers.push(id.clone()),
                "contributor" => {}
                _ => self.unsupported(&json_pointer(&pointer, "role")),
            }
            for member in ["email", "organization"] {
                if object.remove(member).is_some() {
                    self.unsupported(&json_pointer(&pointer, member));
                }
            }
            self.record_unknowns(&object, &pointer);
            let name = self.plain_text(title)?;
            agents
                .entry(id.clone())
                .and_modify(|agent| agent.names.push(name.clone()))
                .or_insert_with(|| ResearchAgent {
                    id,
                    names: vec![name],
                });
        }
        Ok(DecodedContributors {
            agents: agents.into_values().collect(),
            creators,
            publishers,
        })
    }

    fn decode_licenses(
        &mut self,
        value: Option<Value>,
    ) -> Result<Vec<ResearchValue>, ProjectionError> {
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let Value::Array(values) = value else {
            return Err(self.shape("Frictionless licenses must be an array", "/licenses"));
        };
        self.record_order(values.len(), "/licenses");
        let mut licenses = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let pointer = format!("/licenses/{index}");
            let previous_len = licenses.len();
            let Value::Object(mut object) = value else {
                return Err(self.shape("Frictionless license must be an object", &pointer));
            };
            if let Some(path) = self.take_string(&mut object, "path", &pointer)? {
                validate_absolute_iri(&path, "Frictionless license path").map_err(|error| {
                    self.shape(error.message(), &json_pointer(&pointer, "path"))
                })?;
                licenses.push(ResearchValue::iri(path)?);
            }
            if let Some(name) = self.take_string(&mut object, "name", &pointer)? {
                if !validate_license_name(&name) {
                    return Err(self.shape(
                        "Frictionless license name is outside the v1 grammar",
                        &json_pointer(&pointer, "name"),
                    ));
                }
                licenses.push(ResearchValue::Text(self.plain_text(name)?));
            }
            if let Some(title) = self.take_string(&mut object, "title", &pointer)? {
                licenses.push(ResearchValue::Text(self.plain_text(title)?));
            }
            if licenses.len() == previous_len {
                return Err(self.shape("Frictionless license requires name or path", &pointer));
            }
            self.record_unknowns(&object, &pointer);
        }
        Ok(licenses)
    }

    fn decode_resources(
        &mut self,
        value: Option<Value>,
    ) -> Result<Vec<ResearchResource>, ProjectionError> {
        let Some(value) = value else {
            return Err(self.shape("Frictionless resources are required", "/resources"));
        };
        let Value::Array(values) = value else {
            return Err(self.shape("Frictionless resources must be an array", "/resources"));
        };
        if values.is_empty() {
            return Err(self.shape("Frictionless resources cannot be empty", "/resources"));
        }
        self.record_order(values.len(), "/resources");
        let mut resources = Vec::new();
        let mut identities = BTreeSet::new();
        for (index, value) in values.into_iter().enumerate() {
            let pointer = format!("/resources/{index}");
            let Some(resource) = self.decode_resource(value, &pointer)? else {
                continue;
            };
            if !identities.insert(resource.id.clone()) {
                return Err(self.shape(
                    "Frictionless resources contain a duplicate identity",
                    &pointer,
                ));
            }
            resources.push(resource);
        }
        if resources.is_empty() {
            return Err(self.shape(
                "Frictionless package has no metadata-carrying path resource",
                "/resources",
            ));
        }
        Ok(resources)
    }

    fn decode_resource(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchResource>, ProjectionError> {
        let Value::Object(mut object) = value else {
            return Err(self.shape("Frictionless resource must be an object", pointer));
        };
        let native_name = self.require_string(&mut object, "name", pointer)?;
        validate_package_name(&native_name, "Frictionless resource name")
            .map_err(|error| self.shape(error.message(), &json_pointer(pointer, "name")))?;
        let id = self
            .config
            .common()
            .identity()
            .resolve_relative(&native_name)
            .map_err(|error| self.shape(error.message(), &json_pointer(pointer, "name")))?;
        self.loss(
            LOSS_RESEARCH_LOCAL_ID_RESOLVED,
            &json_pointer(pointer, "name"),
        );
        let (paths, urls) = self.decode_paths(object.remove("path"), pointer)?;
        let names = self
            .take_text(&mut object, "title", pointer)?
            .into_iter()
            .collect();
        let descriptions = self
            .take_text(&mut object, "description", pointer)?
            .into_iter()
            .collect();
        let media_types = self
            .take_text(&mut object, "mediatype", pointer)?
            .into_iter()
            .collect();
        let formats = self
            .take_scalar_value(&mut object, "format", pointer)?
            .into_iter()
            .collect();
        let byte_size = self.take_u64(&mut object, "bytes", pointer)?;
        let checksums = self
            .decode_hash(object.remove("hash"), pointer)?
            .into_iter()
            .collect();

        let had_inline_data = object.remove("data").is_some();
        if had_inline_data {
            self.loss(
                LOSS_RESEARCH_INLINE_PAYLOAD_DROPPED,
                &json_pointer(pointer, "data"),
            );
        }
        if object.remove("schema").is_some() {
            self.unsupported(&json_pointer(pointer, "schema"));
        }
        for member in ["profile", "homepage", "sources", "licenses", "encoding"] {
            if object.remove(member).is_some() {
                self.unsupported(&json_pointer(pointer, member));
            }
        }
        self.record_unknowns(&object, pointer);
        if paths.is_empty() && urls.is_empty() {
            if had_inline_data {
                self.unsupported(pointer);
                return Ok(None);
            }
            return Err(self.shape(
                "Frictionless resource requires path or inline data",
                &json_pointer(pointer, "path"),
            ));
        }
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

    fn decode_paths(
        &mut self,
        value: Option<Value>,
        parent: &str,
    ) -> Result<(Vec<String>, Vec<ResearchValue>), ProjectionError> {
        let Some(value) = value else {
            return Ok((Vec::new(), Vec::new()));
        };
        let values = match value {
            Value::String(value) => vec![Value::String(value)],
            Value::Array(values) if !values.is_empty() => {
                self.record_order(values.len(), &json_pointer(parent, "path"));
                values
            }
            _ => {
                return Err(self.shape(
                    "Frictionless path must be a string or non-empty string array",
                    &json_pointer(parent, "path"),
                ));
            }
        };
        let mut paths = Vec::new();
        let mut urls = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let pointer = format!("{}/{}", json_pointer(parent, "path"), index);
            let Value::String(value) = value else {
                return Err(self.shape("Frictionless path item must be a string", &pointer));
            };
            if validate_absolute_iri(&value, "Frictionless resource URL").is_ok() {
                urls.push(ResearchValue::iri(value)?);
            } else {
                validate_data_path(&value)
                    .map_err(|error| self.shape(error.message(), &pointer))?;
                paths.push(value);
            }
        }
        Ok((paths, urls))
    }

    fn decode_hash(
        &self,
        value: Option<Value>,
        parent: &str,
    ) -> Result<Option<ResearchChecksum>, ProjectionError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let Value::String(hash) = value else {
            return Err(self.shape(
                "Frictionless hash must be a string",
                &json_pointer(parent, "hash"),
            ));
        };
        let (algorithm, lexical) = if let Some((algorithm, lexical)) = hash.split_once(':') {
            (algorithm, lexical)
        } else if hash.len() == 32 {
            ("md5", hash.as_str())
        } else {
            return Err(self.shape(
                "Frictionless unqualified hash must be a 32-digit MD5 value",
                &json_pointer(parent, "hash"),
            ));
        };
        if !validate_hash_algorithm(algorithm) || !is_hex(lexical) {
            return Err(self.shape(
                "Frictionless hash must use algorithm:hex syntax",
                &json_pointer(parent, "hash"),
            ));
        }
        Ok(Some(ResearchChecksum {
            algorithm: ResearchValue::Text(self.plain_text(algorithm)?),
            value: self.plain_text(lexical)?,
        }))
    }

    fn require_exact_string(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        expected: &str,
        parent: &str,
    ) -> Result<(), ProjectionError> {
        let actual = object.remove(member).ok_or_else(|| {
            self.shape(
                format!("Frictionless `{member}` is required"),
                &json_pointer(parent, member),
            )
        })?;
        if actual != Value::String(expected.to_owned()) {
            return Err(self.shape(
                format!("Frictionless `{member}` does not match caller configuration"),
                &json_pointer(parent, member),
            ));
        }
        Ok(())
    }

    fn require_text(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<ResearchText, ProjectionError> {
        self.require_string(object, member, parent)
            .and_then(|value| self.plain_text(value))
    }

    fn take_text(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Option<ResearchText>, ProjectionError> {
        self.take_string(object, member, parent)?
            .map(|value| self.plain_text(value))
            .transpose()
    }

    fn require_string(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<String, ProjectionError> {
        self.take_string(object, member, parent)?.ok_or_else(|| {
            self.shape(
                format!("Frictionless `{member}` is required"),
                &json_pointer(parent, member),
            )
        })
    }

    fn take_string(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Option<String>, ProjectionError> {
        let Some(value) = object.remove(member) else {
            return Ok(None);
        };
        let Value::String(value) = value else {
            return Err(self.shape(
                format!("Frictionless `{member}` must be a string"),
                &json_pointer(parent, member),
            ));
        };
        Ok(Some(value))
    }

    fn take_absolute_value(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Option<ResearchValue>, ProjectionError> {
        let Some(value) = self.take_string(object, member, parent)? else {
            return Ok(None);
        };
        validate_absolute_iri(&value, &format!("Frictionless `{member}`"))
            .map_err(|error| self.shape(error.message(), &json_pointer(parent, member)))?;
        ResearchValue::iri(value).map(Some)
    }

    fn take_scalar_value(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Option<ResearchValue>, ProjectionError> {
        let Some(value) = self.take_string(object, member, parent)? else {
            return Ok(None);
        };
        if validate_absolute_iri(&value, &format!("Frictionless `{member}`")).is_ok() {
            ResearchValue::iri(value).map(Some)
        } else {
            self.plain_text(value).map(ResearchValue::Text).map(Some)
        }
    }

    fn take_u64(
        &self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Option<u64>, ProjectionError> {
        let Some(value) = object.remove(member) else {
            return Ok(None);
        };
        let Some(value) = value.as_u64() else {
            return Err(self.shape(
                format!("Frictionless `{member}` must be a non-negative integer"),
                &json_pointer(parent, member),
            ));
        };
        Ok(Some(value))
    }

    fn take_string_array(
        &mut self,
        object: &mut Map<String, Value>,
        member: &str,
        parent: &str,
    ) -> Result<Vec<ResearchText>, ProjectionError> {
        let Some(value) = object.remove(member) else {
            return Ok(Vec::new());
        };
        let Value::Array(values) = value else {
            return Err(self.shape(
                format!("Frictionless `{member}` must be an array"),
                &json_pointer(parent, member),
            ));
        };
        if values.is_empty() {
            return Err(self.shape(
                format!("Frictionless `{member}` cannot be empty"),
                &json_pointer(parent, member),
            ));
        }
        self.record_order(values.len(), &json_pointer(parent, member));
        values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let Value::String(value) = value else {
                    return Err(self.shape(
                        format!("Frictionless `{member}` item must be a string"),
                        &format!("{}/{index}", json_pointer(parent, member)),
                    ));
                };
                self.plain_text(value)
            })
            .collect()
    }

    fn plain_text(&self, value: impl Into<String>) -> Result<ResearchText, ProjectionError> {
        ResearchText::plain(
            value,
            self.config
                .common()
                .roles()
                .iri(super::ResearchRole::XsdString),
        )
    }

    fn record_order(&mut self, length: usize, pointer: &str) {
        if length > 1 {
            self.loss(LOSS_RESEARCH_ORDER_DROPPED, pointer);
        }
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
            FRICTIONLESS_ARTIFACT,
            pointer,
        );
    }

    fn shape(&self, message: impl Into<String>, pointer: &str) -> ProjectionError {
        ProjectionError::integrity(message).at_path(format!("{FRICTIONLESS_ARTIFACT}{pointer}"))
    }
}

fn validate_profile_identity(value: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value.chars().any(char::is_control)
        || value.contains(char::is_whitespace)
    {
        return Err(ProjectionError::configuration(
            "Frictionless package profile must be a non-empty whitespace-free identity",
        ));
    }
    Ok(())
}

fn validate_package_name(value: &str, description: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains("//")
        || value
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
        || !value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '.' | '_' | '/')
        })
    {
        return Err(ProjectionError::configuration(format!(
            "{description} `{value}` is outside the Data Package v1 name grammar"
        )));
    }
    Ok(())
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
        return Err(ProjectionError::integrity(format!(
            "unsafe Frictionless resource path `{path}`"
        )));
    }
    Ok(())
}

fn validate_license_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_')
        })
}

fn validate_hash_algorithm(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_')
        })
}

fn is_hex(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::{
        ProjectionLimits, RESEARCH_ROLES, ResearchObjectIdentity, ResearchObjectPolicy,
        ResearchObjectRoles,
    };

    const INPUT: &[u8] = include_bytes!(
        "../../../tests/fixtures/research-objects/frictionless-data-package-1/input.json"
    );
    const GOLDEN: &[u8] = include_bytes!(
        "../../../tests/fixtures/research-objects/frictionless-data-package-1/golden.json"
    );

    fn config() -> FrictionlessConfig {
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
        FrictionlessConfig::new(
            ResearchObjectConfig::new(roles, identity, policy),
            "https://example.org/profiles/data-package-v1",
            "cat-faces",
        )
        .expect("Frictionless config")
    }

    fn package(bytes: impl Into<Vec<u8>>, config: &FrictionlessConfig) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(
            config.common().limits(),
            [(FRICTIONLESS_ARTIFACT, bytes)],
        )
        .expect("package")
    }

    #[test]
    fn fixture_has_exact_located_losses_and_stable_rewrite() {
        let config = config();
        let read = read_frictionless(&package(INPUT, &config), &config).expect("read fixture");
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
                LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
            ])
        );
        assert!(
            read.loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );

        let projected = project_frictionless(&read.dataset, &config).expect("project fixture");
        let actual = projected
            .package
            .get(FRICTIONLESS_ARTIFACT)
            .expect("Frictionless artifact");
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
        let reread = read_frictionless(&projected.package, &config).expect("read canonical output");
        let reprojected = project_frictionless(&reread.dataset, &config).expect("rewrite");
        assert_eq!(projected.package, reprojected.package);
        assert_eq!(projected.model, reread.model);
    }

    #[test]
    fn reader_rejects_duplicates_profile_drift_traversal_and_duplicate_resources() {
        let config = config();
        assert!(read_frictionless(&package(br#"{"a":1,"a":2}"#, &config), &config).is_err());
        let input = String::from_utf8(INPUT.to_vec()).expect("UTF-8 fixture");

        let profile = input.replacen(
            "https://example.org/profiles/data-package-v1",
            "https://example.org/profiles/wrong",
            1,
        );
        assert!(read_frictionless(&package(profile, &config), &config).is_err());

        let traversal = input.replacen("data/train.csv", "../escape.csv", 1);
        assert!(read_frictionless(&package(traversal, &config), &config).is_err());

        let duplicate = input.replacen(
            "\n  ],\n  \"x-package-extension\"",
            ",\n    {\"name\":\"files/train.csv\",\"path\":\"data/other.csv\"}\n  ],\n  \"x-package-extension\"",
            1,
        );
        assert!(read_frictionless(&package(duplicate, &config), &config).is_err());
    }

    #[test]
    fn config_and_projection_have_no_implicit_identity_or_silent_profile_loss() {
        let config = config();
        assert!(
            FrictionlessConfig::new(config.common().clone(), "", config.package_name(),).is_err()
        );
        assert!(
            FrictionlessConfig::new(
                config.common().clone(),
                config.package_profile(),
                "Upper Case",
            )
            .is_err()
        );

        let mut model = read_frictionless(&package(INPUT, &config), &config)
            .expect("read fixture")
            .model;
        model.dataset.modified.push(
            ResearchText::plain(
                "2026-07-16",
                config
                    .common()
                    .roles()
                    .iri(super::super::ResearchRole::XsdString),
            )
            .expect("modified"),
        );
        let dataset = lift_research_object(model, config.common()).expect("lift model");
        let projected = project_frictionless(dataset.as_ref(), &config).expect("project model");
        assert!(
            projected
                .loss_ledger
                .entries()
                .iter()
                .any(|entry| entry.code == LOSS_RESEARCH_PROFILE_FIELD_DROPPED)
        );
    }
}
