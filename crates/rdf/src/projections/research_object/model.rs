// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{ProjectionDirection, ProjectionError, validate_absolute_iri};
use super::ResearchObjectPolicy;

/// RDF literal identity retained by the common research-object model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchText {
    /// Literal lexical form.
    pub value: String,
    /// Expanded datatype IRI.
    pub datatype: String,
    /// Language tag, when present.
    pub language: Option<String>,
    /// RDF 1.2 base direction, when present.
    pub direction: Option<ProjectionDirection>,
}

impl ResearchText {
    /// Construct one validated literal value.
    ///
    /// # Errors
    ///
    /// The datatype must be absolute, language must be non-empty when present,
    /// and a direction cannot occur without a language.
    pub fn new(
        value: impl Into<String>,
        datatype: impl Into<String>,
        language: Option<String>,
        direction: Option<ProjectionDirection>,
    ) -> Result<Self, ProjectionError> {
        let datatype = datatype.into();
        validate_absolute_iri(&datatype, "research-object literal datatype")?;
        if language.as_deref().is_some_and(str::is_empty) {
            return Err(ProjectionError::integrity(
                "research-object literal language cannot be empty",
            ));
        }
        if direction.is_some() && language.is_none() {
            return Err(ProjectionError::integrity(
                "research-object literal direction requires a language",
            ));
        }
        Ok(Self {
            value: value.into(),
            datatype,
            language,
            direction,
        })
    }

    /// Construct a simple string literal using a caller-supplied XSD string IRI.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when `xsd_string` is not absolute.
    pub fn plain(
        value: impl Into<String>,
        xsd_string: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        Self::new(value, xsd_string, None, None)
    }
}

/// Scalar or reference value shared across research-object formats.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ResearchValue {
    /// Absolute IRI reference.
    Iri {
        /// IRI value.
        value: String,
    },
    /// RDF literal value.
    Text(ResearchText),
}

impl ResearchValue {
    /// Construct an absolute IRI value.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for a relative or invalid IRI.
    pub fn iri(value: impl Into<String>) -> Result<Self, ProjectionError> {
        let value = value.into();
        validate_absolute_iri(&value, "research-object value IRI")?;
        Ok(Self::Iri { value })
    }

    /// Borrow the IRI value, when this is an IRI reference.
    pub fn as_iri(&self) -> Option<&str> {
        let Self::Iri { value } = self else {
            return None;
        };
        Some(value)
    }

    /// Borrow the literal value, when this is text.
    pub const fn as_text(&self) -> Option<&ResearchText> {
        let Self::Text(value) = self else {
            return None;
        };
        Some(value)
    }
}

/// Algorithm/value checksum pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchChecksum {
    /// Algorithm name or IRI exactly as supplied by the source profile.
    pub algorithm: ResearchValue,
    /// Checksum lexical value.
    pub value: ResearchText,
}

/// Person, organization, or software agent used by a research object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchAgent {
    /// Absolute entity IRI.
    pub id: String,
    /// Human-readable names.
    pub names: Vec<ResearchText>,
}

/// File, distribution, or other data resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchResource {
    /// Absolute entity IRI.
    pub id: String,
    /// Resource names/titles.
    pub names: Vec<ResearchText>,
    /// Resource descriptions.
    pub descriptions: Vec<ResearchText>,
    /// Safe relative package paths.
    pub paths: Vec<String>,
    /// Access/download locations.
    pub urls: Vec<ResearchValue>,
    /// Media types.
    pub media_types: Vec<ResearchText>,
    /// Format identifiers or labels.
    pub formats: Vec<ResearchValue>,
    /// Declared size in bytes.
    pub byte_size: Option<u64>,
    /// Content checksums.
    pub checksums: Vec<ResearchChecksum>,
}

/// Provenance activity connected to the research object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchActivity {
    /// Absolute entity IRI.
    pub id: String,
    /// Activity names.
    pub names: Vec<ResearchText>,
    /// Instruments/software/workflows used by the activity.
    pub instruments: Vec<ResearchValue>,
    /// Participating agent IRIs.
    pub actors: Vec<String>,
    /// Input entity IRIs.
    pub objects: Vec<String>,
    /// Output entity IRIs.
    pub results: Vec<String>,
    /// Completion timestamps.
    pub end_times: Vec<ResearchText>,
    /// Workflow identifiers.
    pub workflows: Vec<ResearchValue>,
}

/// Field definition in a Croissant-compatible record set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchField {
    /// Absolute field IRI.
    pub id: String,
    /// Field names.
    pub names: Vec<ResearchText>,
    /// Field datatype identifiers.
    pub data_types: Vec<ResearchValue>,
}

/// Structured record set with deterministic inline JSON rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchRecordSet {
    /// Absolute record-set IRI.
    pub id: String,
    /// Record-set names.
    pub names: Vec<ResearchText>,
    /// Record-set descriptions.
    pub descriptions: Vec<ResearchText>,
    /// Field definitions.
    pub fields: Vec<ResearchField>,
    /// Inline rows represented as JSON values.
    pub rows: Vec<Value>,
}

/// Dataset-level research-object metadata and entity references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchDataset {
    /// Absolute dataset IRI.
    pub id: String,
    /// Dataset titles.
    pub titles: Vec<ResearchText>,
    /// Dataset descriptions.
    pub descriptions: Vec<ResearchText>,
    /// DOI, URL, or other identifiers.
    pub identifiers: Vec<ResearchValue>,
    /// Version values.
    pub versions: Vec<ResearchText>,
    /// Issue/publication dates.
    pub issued: Vec<ResearchText>,
    /// Modification dates.
    pub modified: Vec<ResearchText>,
    /// Landing-page references.
    pub landing_pages: Vec<ResearchValue>,
    /// Keywords.
    pub keywords: Vec<ResearchText>,
    /// License identifiers/references.
    pub licenses: Vec<ResearchValue>,
    /// Creator agent IRIs.
    pub creators: Vec<String>,
    /// Publisher agent IRIs.
    pub publishers: Vec<String>,
    /// Resource IRIs.
    pub resources: Vec<String>,
    /// Activity IRIs.
    pub activities: Vec<String>,
    /// Record-set IRIs.
    pub record_sets: Vec<String>,
}

/// Canonical typed semantic pivot shared by every research-object codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchObjectModel {
    /// Root dataset metadata.
    pub dataset: ResearchDataset,
    /// Referenced agents.
    pub agents: Vec<ResearchAgent>,
    /// Referenced resources.
    pub resources: Vec<ResearchResource>,
    /// Provenance activities.
    pub activities: Vec<ResearchActivity>,
    /// Structured record sets.
    pub record_sets: Vec<ResearchRecordSet>,
}

impl ResearchObjectModel {
    /// Normalize, validate, and enforce caller resource bounds.
    ///
    /// # Errors
    ///
    /// Rejects missing title, invalid/duplicate identities, dangling references,
    /// invalid literals/JSON depth, or an entity/value budget excess.
    pub fn normalize(mut self, policy: ResearchObjectPolicy) -> Result<Self, ProjectionError> {
        validate_absolute_iri(&self.dataset.id, "research-object dataset model identity")?;
        if self.dataset.titles.is_empty() {
            return Err(ProjectionError::integrity(
                "research-object dataset requires at least one title",
            ));
        }

        normalize_dataset(&mut self.dataset)?;
        self.agents.sort_by(|left, right| left.id.cmp(&right.id));
        self.resources.sort_by(|left, right| left.id.cmp(&right.id));
        self.activities
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.record_sets
            .sort_by(|left, right| left.id.cmp(&right.id));
        reject_duplicate_ids(&self.agents, |value| &value.id, "agents")?;
        reject_duplicate_ids(&self.resources, |value| &value.id, "resources")?;
        reject_duplicate_ids(&self.activities, |value| &value.id, "activities")?;
        reject_duplicate_ids(&self.record_sets, |value| &value.id, "record sets")?;

        for agent in &mut self.agents {
            validate_absolute_iri(&agent.id, "research-object agent identity")?;
            sort_dedup(&mut agent.names);
            validate_texts(&agent.names)?;
        }
        for resource in &mut self.resources {
            validate_absolute_iri(&resource.id, "research-object resource identity")?;
            sort_dedup(&mut resource.names);
            sort_dedup(&mut resource.descriptions);
            sort_dedup(&mut resource.paths);
            sort_dedup(&mut resource.urls);
            sort_dedup(&mut resource.media_types);
            sort_dedup(&mut resource.formats);
            sort_dedup(&mut resource.checksums);
            validate_texts(&resource.names)?;
            validate_texts(&resource.descriptions)?;
            validate_texts(&resource.media_types)?;
            for checksum in &resource.checksums {
                validate_value(&checksum.algorithm)?;
                validate_text(&checksum.value)?;
            }
            for value in resource.urls.iter().chain(&resource.formats) {
                validate_value(value)?;
            }
        }
        for activity in &mut self.activities {
            validate_absolute_iri(&activity.id, "research-object activity identity")?;
            sort_dedup(&mut activity.names);
            sort_dedup(&mut activity.instruments);
            sort_dedup(&mut activity.actors);
            sort_dedup(&mut activity.objects);
            sort_dedup(&mut activity.results);
            sort_dedup(&mut activity.end_times);
            sort_dedup(&mut activity.workflows);
            validate_texts(&activity.names)?;
            validate_texts(&activity.end_times)?;
            for value in activity.instruments.iter().chain(&activity.workflows) {
                validate_value(value)?;
            }
        }
        for record_set in &mut self.record_sets {
            validate_absolute_iri(&record_set.id, "research-object record-set identity")?;
            sort_dedup(&mut record_set.names);
            sort_dedup(&mut record_set.descriptions);
            validate_texts(&record_set.names)?;
            validate_texts(&record_set.descriptions)?;
            record_set
                .fields
                .sort_by(|left, right| left.id.cmp(&right.id));
            reject_duplicate_ids(&record_set.fields, |value| &value.id, "record-set fields")?;
            for field in &mut record_set.fields {
                validate_absolute_iri(&field.id, "research-object field identity")?;
                sort_dedup(&mut field.names);
                sort_dedup(&mut field.data_types);
                validate_texts(&field.names)?;
                for value in &field.data_types {
                    validate_value(value)?;
                }
            }
            record_set.rows.sort_by_key(canonical_json_key);
            record_set.rows.dedup();
            for row in &record_set.rows {
                validate_json_depth(row, policy.max_json_depth(), 0)?;
            }
        }

        let agent_ids = ids(&self.agents, |value| &value.id);
        let resource_ids = ids(&self.resources, |value| &value.id);
        let activity_ids = ids(&self.activities, |value| &value.id);
        let record_set_ids = ids(&self.record_sets, |value| &value.id);
        require_subset(&self.dataset.creators, &agent_ids, "creator")?;
        require_subset(&self.dataset.publishers, &agent_ids, "publisher")?;
        require_subset(&self.dataset.resources, &resource_ids, "resource")?;
        require_subset(&self.dataset.activities, &activity_ids, "activity")?;
        require_subset(&self.dataset.record_sets, &record_set_ids, "record set")?;
        for activity in &self.activities {
            require_subset(&activity.actors, &agent_ids, "activity actor")?;
            let mut entities = resource_ids.clone();
            entities.extend(activity_ids.iter().cloned());
            entities.extend(record_set_ids.iter().cloned());
            entities.insert(self.dataset.id.clone());
            require_subset(&activity.objects, &entities, "activity object")?;
            require_subset(&activity.results, &entities, "activity result")?;
        }

        let entity_count = 1usize
            .checked_add(self.agents.len())
            .and_then(|count| count.checked_add(self.resources.len()))
            .and_then(|count| count.checked_add(self.activities.len()))
            .and_then(|count| count.checked_add(self.record_sets.len()))
            .and_then(|count| {
                count.checked_add(
                    self.record_sets
                        .iter()
                        .map(|record_set| record_set.fields.len())
                        .sum::<usize>(),
                )
            })
            .ok_or_else(|| ProjectionError::limit("research-object entity count overflow"))?;
        if entity_count > policy.max_entities() {
            return Err(ProjectionError::limit(format!(
                "research-object model has {entity_count} entities; limit is {}",
                policy.max_entities()
            )));
        }
        let value_count = count_values(&self)?;
        if value_count > policy.max_values() {
            return Err(ProjectionError::limit(format!(
                "research-object model has {value_count} values; limit is {}",
                policy.max_values()
            )));
        }
        Ok(self)
    }
}

fn normalize_dataset(dataset: &mut ResearchDataset) -> Result<(), ProjectionError> {
    sort_dedup(&mut dataset.titles);
    sort_dedup(&mut dataset.descriptions);
    sort_dedup(&mut dataset.identifiers);
    sort_dedup(&mut dataset.versions);
    sort_dedup(&mut dataset.issued);
    sort_dedup(&mut dataset.modified);
    sort_dedup(&mut dataset.landing_pages);
    sort_dedup(&mut dataset.keywords);
    sort_dedup(&mut dataset.licenses);
    sort_dedup(&mut dataset.creators);
    sort_dedup(&mut dataset.publishers);
    sort_dedup(&mut dataset.resources);
    sort_dedup(&mut dataset.activities);
    sort_dedup(&mut dataset.record_sets);
    validate_texts(&dataset.titles)?;
    validate_texts(&dataset.descriptions)?;
    validate_texts(&dataset.versions)?;
    validate_texts(&dataset.issued)?;
    validate_texts(&dataset.modified)?;
    validate_texts(&dataset.keywords)?;
    for value in dataset
        .identifiers
        .iter()
        .chain(&dataset.landing_pages)
        .chain(&dataset.licenses)
    {
        validate_value(value)?;
    }
    Ok(())
}

fn validate_texts(values: &[ResearchText]) -> Result<(), ProjectionError> {
    for value in values {
        validate_text(value)?;
    }
    Ok(())
}

fn validate_text(value: &ResearchText) -> Result<(), ProjectionError> {
    let _ = ResearchText::new(
        value.value.clone(),
        value.datatype.clone(),
        value.language.clone(),
        value.direction,
    )?;
    Ok(())
}

fn validate_value(value: &ResearchValue) -> Result<(), ProjectionError> {
    match value {
        ResearchValue::Iri { value } => validate_absolute_iri(value, "research-object value IRI"),
        ResearchValue::Text(value) => validate_text(value),
    }
}

fn sort_dedup<T: Ord>(values: &mut Vec<T>) {
    values.sort();
    values.dedup();
}

fn reject_duplicate_ids<T>(
    values: &[T],
    id: impl Fn(&T) -> &String,
    description: &str,
) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| id(&pair[0]) == id(&pair[1])) {
        return Err(ProjectionError::integrity(format!(
            "research-object model contains duplicate {description} identity"
        )));
    }
    Ok(())
}

fn ids<T>(values: &[T], id: impl Fn(&T) -> &String) -> BTreeSet<String> {
    values.iter().map(|value| id(value).clone()).collect()
}

fn require_subset(
    values: &[String],
    allowed: &BTreeSet<String>,
    description: &str,
) -> Result<(), ProjectionError> {
    if let Some(value) = values.iter().find(|value| !allowed.contains(*value)) {
        return Err(ProjectionError::integrity(format!(
            "research-object {description} reference `{value}` is dangling"
        )));
    }
    Ok(())
}

fn validate_json_depth(value: &Value, maximum: usize, depth: usize) -> Result<(), ProjectionError> {
    if depth > maximum {
        return Err(ProjectionError::limit(format!(
            "research-object inline JSON exceeds depth limit {maximum}"
        )));
    }
    match value {
        Value::Array(values) => {
            for value in values {
                validate_json_depth(value, maximum, depth + 1)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                validate_json_depth(value, maximum, depth + 1)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

fn canonical_json_key(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("serde_json::Value always serializes")
}

fn count_values(model: &ResearchObjectModel) -> Result<usize, ProjectionError> {
    let mut count = 0usize;
    let mut add = |value: usize| -> Result<(), ProjectionError> {
        count = count
            .checked_add(value)
            .ok_or_else(|| ProjectionError::limit("research-object value count overflow"))?;
        Ok(())
    };
    let dataset = &model.dataset;
    for length in [
        dataset.titles.len(),
        dataset.descriptions.len(),
        dataset.identifiers.len(),
        dataset.versions.len(),
        dataset.issued.len(),
        dataset.modified.len(),
        dataset.landing_pages.len(),
        dataset.keywords.len(),
        dataset.licenses.len(),
        dataset.creators.len(),
        dataset.publishers.len(),
        dataset.resources.len(),
        dataset.activities.len(),
        dataset.record_sets.len(),
    ] {
        add(length)?;
    }
    for agent in &model.agents {
        add(agent.names.len())?;
    }
    for resource in &model.resources {
        let checksum_values = resource
            .checksums
            .len()
            .checked_mul(2)
            .ok_or_else(|| ProjectionError::limit("research-object value count overflow"))?;
        for length in [
            resource.names.len(),
            resource.descriptions.len(),
            resource.paths.len(),
            resource.urls.len(),
            resource.media_types.len(),
            resource.formats.len(),
            usize::from(resource.byte_size.is_some()),
            checksum_values,
        ] {
            add(length)?;
        }
    }
    for activity in &model.activities {
        for length in [
            activity.names.len(),
            activity.instruments.len(),
            activity.actors.len(),
            activity.objects.len(),
            activity.results.len(),
            activity.end_times.len(),
            activity.workflows.len(),
        ] {
            add(length)?;
        }
    }
    for record_set in &model.record_sets {
        add(record_set.names.len())?;
        add(record_set.descriptions.len())?;
        add(record_set.rows.len())?;
        for field in &record_set.fields {
            add(field.names.len())?;
            add(field.data_types.len())?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::ProjectionLimits;

    fn policy(max_entities: usize, max_values: usize) -> ResearchObjectPolicy {
        ResearchObjectPolicy::new(
            ProjectionLimits::new(16, 10_000, 20_000, 30_000, 8).expect("limits"),
            1_000,
            max_entities,
            max_values,
            8,
        )
        .expect("policy")
    }

    fn text(value: &str) -> ResearchText {
        ResearchText::plain(value, "https://example.org/xsd/string").expect("text")
    }

    fn minimal() -> ResearchObjectModel {
        ResearchObjectModel {
            dataset: ResearchDataset {
                id: "https://example.org/dataset".to_owned(),
                titles: vec![text("Z"), text("A"), text("A")],
                descriptions: vec![],
                identifiers: vec![],
                versions: vec![],
                issued: vec![],
                modified: vec![],
                landing_pages: vec![],
                keywords: vec![],
                licenses: vec![],
                creators: vec![],
                publishers: vec![],
                resources: vec![],
                activities: vec![],
                record_sets: vec![],
            },
            agents: vec![],
            resources: vec![],
            activities: vec![],
            record_sets: vec![],
        }
    }

    #[test]
    fn normalization_is_deterministic_and_deduplicates_values() {
        let model = minimal().normalize(policy(10, 10)).expect("normalize");
        assert_eq!(model.dataset.titles, vec![text("A"), text("Z")]);
        assert_eq!(
            model.clone().normalize(policy(10, 10)).expect("again"),
            model
        );
    }

    #[test]
    fn dangling_identity_and_budgets_fail_closed() {
        let mut dangling = minimal();
        dangling
            .dataset
            .resources
            .push("https://example.org/missing".to_owned());
        assert!(dangling.normalize(policy(10, 10)).is_err());
        let mut too_many_entities = minimal();
        too_many_entities
            .dataset
            .creators
            .push("https://example.org/agent".to_owned());
        too_many_entities.agents.push(ResearchAgent {
            id: "https://example.org/agent".to_owned(),
            names: vec![text("Agent")],
        });
        assert!(too_many_entities.normalize(policy(1, 10)).is_err());
        assert!(minimal().normalize(policy(10, 1)).is_err());
    }

    #[test]
    fn directional_text_requires_language() {
        assert!(
            ResearchText::new(
                "value",
                "https://example.org/xsd/string",
                None,
                Some(ProjectionDirection::Rtl)
            )
            .is_err()
        );
    }
}
