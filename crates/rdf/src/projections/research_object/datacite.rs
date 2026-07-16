// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED, LOSS_RESEARCH_ORDER_DROPPED,
    LOSS_RESEARCH_PROFILE_FIELD_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
};
use purrdf_core::{DatasetView, LossLedger, research_object_to_rdf_loss_ledger};
use roxmltree::{Document, Node};
use serde::{Deserialize, Deserializer, Serialize};

use super::super::{
    ProjectionError, ProjectionPackage, escape_xml_attribute, escape_xml_text,
    validate_absolute_iri,
};
use super::json::{
    ResearchObjectPackageProjection, ResearchObjectReadOutcome, ensure_sound,
    normalize_lifted_jsonld, record_loss, require_artifact,
};
use super::{
    ResearchActivity, ResearchAgent, ResearchDataset, ResearchObjectConfig, ResearchObjectModel,
    ResearchRecordSet, ResearchResource, ResearchText, ResearchValue, lift_research_object,
    project_research_object,
};

/// Closed DataCite projection profile identifier.
pub const DATACITE_PROFILE: &str = "datacite-4.6";
/// Sole artifact path in the canonical DataCite package.
pub const DATACITE_ARTIFACT: &str = "datacite.xml";

/// Caller-selected DataCite 4.6 controlled values and identifier policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataCiteControlledValues {
    identifier_type: String,
    resource_type_general: String,
    creator_name_type: String,
    agent_identifier_scheme: String,
    agent_identifier_scheme_uri: String,
    related_identifier_type: String,
    landing_page_relation_type: String,
    resource_relation_type: String,
    activity_relation_type: String,
    record_set_relation_type: String,
    issued_date_type: String,
    modified_date_type: String,
    description_type: String,
}

impl DataCiteControlledValues {
    /// Construct a complete selected controlled-value policy.
    ///
    /// # Errors
    ///
    /// Rejects empty/non-token values, a non-absolute identifier-scheme IRI,
    /// or relation values that cannot be decoded unambiguously.
    // DataCite exposes one selected value per controlled slot. Keeping every
    // slot explicit prevents an omitted argument from becoming library policy.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identifier_type: impl Into<String>,
        resource_type_general: impl Into<String>,
        creator_name_type: impl Into<String>,
        agent_identifier_scheme: impl Into<String>,
        agent_identifier_scheme_uri: impl Into<String>,
        related_identifier_type: impl Into<String>,
        landing_page_relation_type: impl Into<String>,
        resource_relation_type: impl Into<String>,
        activity_relation_type: impl Into<String>,
        record_set_relation_type: impl Into<String>,
        issued_date_type: impl Into<String>,
        modified_date_type: impl Into<String>,
        description_type: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let values = Self {
            identifier_type: identifier_type.into(),
            resource_type_general: resource_type_general.into(),
            creator_name_type: creator_name_type.into(),
            agent_identifier_scheme: agent_identifier_scheme.into(),
            agent_identifier_scheme_uri: agent_identifier_scheme_uri.into(),
            related_identifier_type: related_identifier_type.into(),
            landing_page_relation_type: landing_page_relation_type.into(),
            resource_relation_type: resource_relation_type.into(),
            activity_relation_type: activity_relation_type.into(),
            record_set_relation_type: record_set_relation_type.into(),
            issued_date_type: issued_date_type.into(),
            modified_date_type: modified_date_type.into(),
            description_type: description_type.into(),
        };
        for (field, value) in values.tokens() {
            validate_controlled_token(value, field)?;
        }
        validate_absolute_iri(
            &values.agent_identifier_scheme_uri,
            "DataCite agent identifier scheme URI",
        )?;
        let relations = [
            values.landing_page_relation_type.as_str(),
            values.resource_relation_type.as_str(),
            values.activity_relation_type.as_str(),
            values.record_set_relation_type.as_str(),
        ];
        if relations.iter().copied().collect::<BTreeSet<_>>().len() != relations.len() {
            return Err(ProjectionError::configuration(
                "DataCite relation-type bindings must be distinct",
            ));
        }
        Ok(values)
    }

    fn tokens(&self) -> [(&'static str, &str); 12] {
        [
            ("identifier_type", &self.identifier_type),
            ("resource_type_general", &self.resource_type_general),
            ("creator_name_type", &self.creator_name_type),
            ("agent_identifier_scheme", &self.agent_identifier_scheme),
            ("related_identifier_type", &self.related_identifier_type),
            (
                "landing_page_relation_type",
                &self.landing_page_relation_type,
            ),
            ("resource_relation_type", &self.resource_relation_type),
            ("activity_relation_type", &self.activity_relation_type),
            ("record_set_relation_type", &self.record_set_relation_type),
            ("issued_date_type", &self.issued_date_type),
            ("modified_date_type", &self.modified_date_type),
            ("description_type", &self.description_type),
        ]
    }

    /// Primary identifier type.
    pub fn identifier_type(&self) -> &str {
        &self.identifier_type
    }
    /// Root resource type.
    pub fn resource_type_general(&self) -> &str {
        &self.resource_type_general
    }
    /// Creator-name controlled value.
    pub fn creator_name_type(&self) -> &str {
        &self.creator_name_type
    }
    /// Agent identifier scheme label.
    pub fn agent_identifier_scheme(&self) -> &str {
        &self.agent_identifier_scheme
    }
    /// Agent identifier scheme IRI.
    pub fn agent_identifier_scheme_uri(&self) -> &str {
        &self.agent_identifier_scheme_uri
    }
    /// Related-identifier type.
    pub fn related_identifier_type(&self) -> &str {
        &self.related_identifier_type
    }
    /// Landing-page relation type.
    pub fn landing_page_relation_type(&self) -> &str {
        &self.landing_page_relation_type
    }
    /// Resource relation type.
    pub fn resource_relation_type(&self) -> &str {
        &self.resource_relation_type
    }
    /// Activity relation type.
    pub fn activity_relation_type(&self) -> &str {
        &self.activity_relation_type
    }
    /// Record-set relation type.
    pub fn record_set_relation_type(&self) -> &str {
        &self.record_set_relation_type
    }
    /// Issued-date controlled value.
    pub fn issued_date_type(&self) -> &str {
        &self.issued_date_type
    }
    /// Modified-date controlled value.
    pub fn modified_date_type(&self) -> &str {
        &self.modified_date_type
    }
    /// Description-type controlled value.
    pub fn description_type(&self) -> &str {
        &self.description_type
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDataCiteControlledValues {
    identifier_type: String,
    resource_type_general: String,
    creator_name_type: String,
    agent_identifier_scheme: String,
    agent_identifier_scheme_uri: String,
    related_identifier_type: String,
    landing_page_relation_type: String,
    resource_relation_type: String,
    activity_relation_type: String,
    record_set_relation_type: String,
    issued_date_type: String,
    modified_date_type: String,
    description_type: String,
}

impl<'de> Deserialize<'de> for DataCiteControlledValues {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDataCiteControlledValues::deserialize(deserializer)?;
        Self::new(
            raw.identifier_type,
            raw.resource_type_general,
            raw.creator_name_type,
            raw.agent_identifier_scheme,
            raw.agent_identifier_scheme_uri,
            raw.related_identifier_type,
            raw.landing_page_relation_type,
            raw.resource_relation_type,
            raw.activity_relation_type,
            raw.record_set_relation_type,
            raw.issued_date_type,
            raw.modified_date_type,
            raw.description_type,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Mandatory caller-owned DataCite 4.6 schema and semantic configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataCiteConfig {
    common: ResearchObjectConfig,
    namespace_iri: String,
    xml_schema_instance_iri: String,
    schema_location: String,
    controlled: DataCiteControlledValues,
}

impl DataCiteConfig {
    /// Construct a validated DataCite configuration.
    ///
    /// # Errors
    ///
    /// Every namespace/schema identity must be absolute. No namespace or
    /// controlled value is supplied by the library.
    pub fn new(
        common: ResearchObjectConfig,
        namespace_iri: impl Into<String>,
        xml_schema_instance_iri: impl Into<String>,
        schema_location: impl Into<String>,
        controlled: DataCiteControlledValues,
    ) -> Result<Self, ProjectionError> {
        let namespace_iri = namespace_iri.into();
        let xml_schema_instance_iri = xml_schema_instance_iri.into();
        let schema_location = schema_location.into();
        validate_absolute_iri(&namespace_iri, "DataCite namespace")?;
        validate_absolute_iri(
            &xml_schema_instance_iri,
            "DataCite XML Schema-instance namespace",
        )?;
        validate_absolute_iri(&schema_location, "DataCite schema location")?;
        Ok(Self {
            common,
            namespace_iri,
            xml_schema_instance_iri,
            schema_location,
            controlled,
        })
    }

    /// Shared RDF configuration and limits.
    pub const fn common(&self) -> &ResearchObjectConfig {
        &self.common
    }
    /// DataCite element namespace.
    pub fn namespace_iri(&self) -> &str {
        &self.namespace_iri
    }
    /// Caller-supplied XML Schema-instance namespace.
    pub fn xml_schema_instance_iri(&self) -> &str {
        &self.xml_schema_instance_iri
    }
    /// Caller-supplied schema document IRI.
    pub fn schema_location(&self) -> &str {
        &self.schema_location
    }
    /// Selected controlled values.
    pub const fn controlled(&self) -> &DataCiteControlledValues {
        &self.controlled
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDataCiteConfig {
    common: ResearchObjectConfig,
    namespace_iri: String,
    xml_schema_instance_iri: String,
    schema_location: String,
    controlled: DataCiteControlledValues,
}

impl<'de> Deserialize<'de> for DataCiteConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDataCiteConfig::deserialize(deserializer)?;
        Self::new(
            raw.common,
            raw.namespace_iri,
            raw.xml_schema_instance_iri,
            raw.schema_location,
            raw.controlled,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Project caller-vocabulary RDF 1.2 into deterministic DataCite 4.6 XML.
///
/// # Errors
///
/// Requires a document identifier, title, creator, publisher, and issued date;
/// none is synthesized. Returns typed mapping, XML, or resource-limit failures.
pub fn project_datacite<D: DatasetView>(
    view: &D,
    config: &DataCiteConfig,
) -> Result<ResearchObjectPackageProjection, ProjectionError> {
    let projection = project_research_object(view, DATACITE_PROFILE, config.common())?;
    let mut ledger = projection.loss_ledger;
    let bytes = write_datacite(&projection.model, config, &mut ledger)?;
    ensure_sound(&ledger, "rdf-1.2-dataset", DATACITE_PROFILE)?;
    let package =
        ProjectionPackage::from_artifacts(config.common().limits(), [(DATACITE_ARTIFACT, bytes)])?;
    Ok(ResearchObjectPackageProjection {
        package,
        model: projection.model,
        loss_ledger: ledger,
    })
}

/// Read strict DataCite 4.6 XML and lift caller-vocabulary RDF 1.2.
///
/// # Errors
///
/// Rejects DTD/entity input, namespace/schema drift, malformed XML, duplicate
/// singleton elements, controlled-value drift, missing required metadata, or
/// configured resource-limit excesses.
pub fn read_datacite(
    package: &ProjectionPackage,
    config: &DataCiteConfig,
) -> Result<ResearchObjectReadOutcome, ProjectionError> {
    let bytes = require_artifact(package, DATACITE_ARTIFACT, config.common())?;
    let contract = research_object_to_rdf_loss_ledger(DATACITE_PROFILE);
    let mut ledger = LossLedger::new();
    let model = parse_datacite(bytes, config, &contract, &mut ledger)?
        .normalize(config.common().policy())?;
    ensure_sound(&ledger, DATACITE_PROFILE, "rdf-1.2-dataset")?;
    let dataset = lift_research_object(model.clone(), config.common())?;
    Ok(ResearchObjectReadOutcome {
        dataset: normalize_lifted_jsonld(&dataset)?,
        model,
        loss_ledger: ledger,
    })
}

fn validate_controlled_token(value: &str, field: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '<' | '>' | '"' | '\''))
    {
        return Err(ProjectionError::configuration(format!(
            "DataCite controlled value `{field}` is invalid"
        )));
    }
    Ok(())
}

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

fn parse_datacite(
    bytes: &[u8],
    config: &DataCiteConfig,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) -> Result<ResearchObjectModel, ProjectionError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProjectionError::syntax(format!("DataCite XML is not UTF-8: {error}"))
            .at_path(DATACITE_ARTIFACT)
    })?;
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err(ProjectionError::syntax(
            "DataCite profile forbids DTD and entity declarations",
        )
        .at_path(DATACITE_ARTIFACT));
    }
    let document = Document::parse(text).map_err(|error| {
        ProjectionError::syntax(format!("parse DataCite XML: {error}")).at_path(DATACITE_ARTIFACT)
    })?;
    let element_count = document.descendants().filter(Node::is_element).count();
    if element_count > config.common().policy().max_records() {
        return Err(ProjectionError::limit(format!(
            "DataCite XML has {element_count} elements; limit is {}",
            config.common().policy().max_records()
        ))
        .at_path(DATACITE_ARTIFACT));
    }

    let root = document.root_element();
    require_datacite_element(root, "resource", config)?;
    let expected_schema_location =
        format!("{} {}", config.namespace_iri(), config.schema_location());
    if namespaced_attribute(root, config.xml_schema_instance_iri(), "schemaLocation")
        != Some(expected_schema_location.as_str())
    {
        return Err(ProjectionError::integrity(
            "DataCite root has the wrong or missing xsi:schemaLocation",
        )
        .at_path(DATACITE_ARTIFACT));
    }
    record_unknown_attributes(
        root,
        &[(Some(config.xml_schema_instance_iri()), "schemaLocation")],
        "/resource",
        contract,
        ledger,
    );

    let mut parser = DataCiteParser {
        config,
        contract,
        ledger,
        seen_root: BTreeSet::new(),
        agents: BTreeMap::new(),
    };
    let mut identifier = None;
    let mut creators = Vec::new();
    let mut titles = Vec::new();
    let mut publisher = None;
    let mut publication_year = None;
    let mut resource_type = None;
    let mut keywords = Vec::new();
    let mut issued = Vec::new();
    let mut modified = Vec::new();
    let mut alternate_identifiers = Vec::new();
    let mut landing_pages = Vec::new();
    let mut resources = Vec::new();
    let mut activities = Vec::new();
    let mut record_sets = Vec::new();
    let mut versions = Vec::new();
    let mut licenses = Vec::new();
    let mut descriptions = Vec::new();

    for child in root.children().filter(Node::is_element) {
        require_datacite_namespace(child, config)?;
        let local = child.tag_name().name();
        let path = format!("/resource/{local}");
        match local {
            "identifier" => {
                parser.mark_root_singleton(local)?;
                identifier = Some(parser.parse_identifier(child, &path)?);
            }
            "creators" => {
                parser.mark_root_singleton(local)?;
                creators = parser.parse_creators(child, &path)?;
            }
            "titles" => {
                parser.mark_root_singleton(local)?;
                titles = parser.parse_text_group(child, "title", &path, &[])?;
            }
            "publisher" => {
                parser.mark_root_singleton(local)?;
                publisher = Some(parser.parse_publisher(child, &path)?);
            }
            "publicationYear" => {
                parser.mark_root_singleton(local)?;
                let year = parser.parse_plain_text(child, &path, &[])?;
                if year.len() != 4 || !year.chars().all(|character| character.is_ascii_digit()) {
                    return Err(ProjectionError::integrity(
                        "DataCite publicationYear must contain exactly four ASCII digits",
                    )
                    .at_path(DATACITE_ARTIFACT));
                }
                publication_year = Some(year);
            }
            "resourceType" => {
                parser.mark_root_singleton(local)?;
                parser.require_attribute(
                    child,
                    "resourceTypeGeneral",
                    config.controlled().resource_type_general(),
                    &path,
                )?;
                let value =
                    parser.parse_plain_text(child, &path, &[(None, "resourceTypeGeneral")])?;
                if value != config.controlled().resource_type_general() {
                    return Err(ProjectionError::integrity(
                        "DataCite resourceType lexical value differs from caller policy",
                    )
                    .at_path(DATACITE_ARTIFACT));
                }
                resource_type = Some(value);
            }
            "subjects" => {
                parser.mark_root_singleton(local)?;
                keywords = parser.parse_text_group(child, "subject", &path, &[])?;
            }
            "dates" => {
                parser.mark_root_singleton(local)?;
                (issued, modified) = parser.parse_dates(child, &path)?;
            }
            "alternateIdentifiers" => {
                parser.mark_root_singleton(local)?;
                alternate_identifiers = parser.parse_alternate_identifiers(child, &path)?;
            }
            "relatedIdentifiers" => {
                parser.mark_root_singleton(local)?;
                let related = parser.parse_related_identifiers(child, &path)?;
                landing_pages = related.landing_pages;
                resources = related.resources;
                activities = related.activities;
                record_sets = related.record_sets;
            }
            "version" => {
                parser.mark_root_singleton(local)?;
                versions.push(parser.parse_research_text(child, &path, &[])?);
            }
            "rightsList" => {
                parser.mark_root_singleton(local)?;
                licenses = parser.parse_rights(child, &path)?;
            }
            "descriptions" => {
                parser.mark_root_singleton(local)?;
                descriptions = parser.parse_descriptions(child, &path)?;
            }
            _ => parser.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &path),
        }
    }

    let identifier = identifier.ok_or_else(|| missing_datacite("identifier"))?;
    if creators.is_empty() {
        return Err(missing_datacite("creators/creator"));
    }
    if titles.is_empty() {
        return Err(missing_datacite("titles/title"));
    }
    let publisher = publisher.ok_or_else(|| missing_datacite("publisher"))?;
    let publication_year = publication_year.ok_or_else(|| missing_datacite("publicationYear"))?;
    if resource_type.is_none() {
        return Err(missing_datacite("resourceType"));
    }
    if issued.is_empty() {
        issued.push(ResearchText::plain(
            publication_year,
            config.common().roles().iri(super::ResearchRole::XsdString),
        )?);
    } else if !issued.iter().any(|value| {
        extract_year(&value.value).is_some_and(|year| year == publication_year.as_str())
    }) {
        return Err(ProjectionError::integrity(
            "DataCite publicationYear must match at least one selected issued date",
        )
        .at_path(DATACITE_ARTIFACT));
    }

    let mut identifiers = vec![identifier];
    identifiers.extend(alternate_identifiers);
    let agents = parser.agents.into_values().collect();
    let resource_ids = resources.iter().map(|value| value.id.clone()).collect();
    let activity_ids = activities.iter().map(|value| value.id.clone()).collect();
    let record_set_ids = record_sets.iter().map(|value| value.id.clone()).collect();
    Ok(ResearchObjectModel {
        dataset: ResearchDataset {
            id: config.common().identity().dataset_iri().to_owned(),
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
            publishers: vec![publisher],
            resources: resource_ids,
            activities: activity_ids,
            record_sets: record_set_ids,
        },
        agents,
        resources,
        activities,
        record_sets,
    })
}

struct DataCiteParser<'a> {
    config: &'a DataCiteConfig,
    contract: &'a LossLedger,
    ledger: &'a mut LossLedger,
    seen_root: BTreeSet<String>,
    agents: BTreeMap<String, ResearchAgent>,
}

impl DataCiteParser<'_> {
    fn mark_root_singleton(&mut self, local: &str) -> Result<(), ProjectionError> {
        if !self.seen_root.insert(local.to_owned()) {
            return Err(ProjectionError::integrity(format!(
                "DataCite root contains duplicate {local:?} element"
            ))
            .at_path(DATACITE_ARTIFACT));
        }
        Ok(())
    }

    fn loss(&mut self, code: &'static str, path: &str) {
        record_loss(self.ledger, self.contract, code, DATACITE_ARTIFACT, path);
    }

    fn require_attribute(
        &self,
        node: Node<'_, '_>,
        name: &str,
        expected: &str,
        path: &str,
    ) -> Result<(), ProjectionError> {
        match node.attribute(name) {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(ProjectionError::integrity(format!(
                "DataCite {path} attribute {name:?} is {actual:?}; expected {expected:?}"
            ))
            .at_path(DATACITE_ARTIFACT)),
            None => Err(ProjectionError::integrity(format!(
                "DataCite {path} is missing attribute {name:?}"
            ))
            .at_path(DATACITE_ARTIFACT)),
        }
    }

    fn parse_plain_text(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
        expected_attributes: &[(Option<&str>, &str)],
    ) -> Result<String, ProjectionError> {
        record_unknown_attributes(node, expected_attributes, path, self.contract, self.ledger);
        simple_element_text(node, path)
    }

    fn parse_research_text(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
        expected_attributes: &[(Option<&str>, &str)],
    ) -> Result<ResearchText, ProjectionError> {
        let mut expected = expected_attributes.to_vec();
        expected.push((Some(XML_NAMESPACE), "lang"));
        let value = self.parse_plain_text(node, path, &expected)?;
        let language = namespaced_attribute(node, XML_NAMESPACE, "lang").map(str::to_owned);
        if language.as_deref().is_some_and(str::is_empty) {
            return Err(
                ProjectionError::integrity("DataCite xml:lang cannot be empty")
                    .at_path(DATACITE_ARTIFACT),
            );
        }
        let role = if language.is_some() {
            super::ResearchRole::RdfLangString
        } else {
            super::ResearchRole::XsdString
        };
        ResearchText::new(
            value,
            self.config.common().roles().iri(role),
            language,
            None,
        )
    }

    fn parse_identifier(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<ResearchValue, ProjectionError> {
        self.require_attribute(
            node,
            "identifierType",
            self.config.controlled().identifier_type(),
            path,
        )?;
        let value = self.parse_plain_text(node, path, &[(None, "identifierType")])?;
        research_value(
            value,
            self.config
                .common()
                .roles()
                .iri(super::ResearchRole::XsdString),
        )
    }

    fn parse_creators(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut creators = Vec::new();
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() == "creator" {
                creators.push(self.parse_creator(child, &child_path)?);
            } else {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
            }
        }
        record_order_loss(&creators, path, self.contract, self.ledger);
        if creators.iter().collect::<BTreeSet<_>>().len() != creators.len() {
            return Err(ProjectionError::integrity(
                "DataCite creators contain a duplicate agent identity",
            )
            .at_path(DATACITE_ARTIFACT));
        }
        Ok(creators)
    }

    fn parse_creator(&mut self, node: Node<'_, '_>, path: &str) -> Result<String, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut name = None;
        let mut identifier = None;
        for child in node.children().filter(Node::is_element) {
            require_datacite_namespace(child, self.config)?;
            let local = child.tag_name().name();
            let child_path = format!("{path}/{local}");
            match local {
                "creatorName" if name.is_none() => {
                    self.require_attribute(
                        child,
                        "nameType",
                        self.config.controlled().creator_name_type(),
                        &child_path,
                    )?;
                    name = Some(self.parse_research_text(
                        child,
                        &child_path,
                        &[(None, "nameType")],
                    )?);
                }
                "nameIdentifier" if identifier.is_none() => {
                    self.require_agent_identifier_attributes(child, &child_path)?;
                    let value = self.parse_plain_text(
                        child,
                        &child_path,
                        &[(None, "nameIdentifierScheme"), (None, "schemeURI")],
                    )?;
                    validate_absolute_iri(&value, "DataCite creator identity")?;
                    identifier = Some(value);
                }
                "creatorName" | "nameIdentifier" => {
                    return Err(ProjectionError::integrity(format!(
                        "DataCite {path} contains duplicate {local:?} element"
                    ))
                    .at_path(DATACITE_ARTIFACT));
                }
                _ => self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path),
            }
        }
        let name = name.ok_or_else(|| missing_datacite(&format!("{path}/creatorName")))?;
        let identifier =
            identifier.ok_or_else(|| missing_datacite(&format!("{path}/nameIdentifier")))?;
        self.insert_agent(identifier.clone(), name);
        Ok(identifier)
    }

    fn require_agent_identifier_attributes(
        &self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<(), ProjectionError> {
        self.require_attribute(
            node,
            "nameIdentifierScheme",
            self.config.controlled().agent_identifier_scheme(),
            path,
        )?;
        self.require_attribute(
            node,
            "schemeURI",
            self.config.controlled().agent_identifier_scheme_uri(),
            path,
        )
    }

    fn parse_publisher(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<String, ProjectionError> {
        self.require_attribute(
            node,
            "publisherIdentifierScheme",
            self.config.controlled().agent_identifier_scheme(),
            path,
        )?;
        self.require_attribute(
            node,
            "schemeURI",
            self.config.controlled().agent_identifier_scheme_uri(),
            path,
        )?;
        let identifier = node.attribute("publisherIdentifier").ok_or_else(|| {
            ProjectionError::integrity("DataCite publisher is missing publisherIdentifier")
                .at_path(DATACITE_ARTIFACT)
        })?;
        validate_absolute_iri(identifier, "DataCite publisher identity")?;
        let name = self.parse_research_text(
            node,
            path,
            &[
                (None, "publisherIdentifier"),
                (None, "publisherIdentifierScheme"),
                (None, "schemeURI"),
            ],
        )?;
        self.insert_agent(identifier.to_owned(), name);
        Ok(identifier.to_owned())
    }

    fn insert_agent(&mut self, id: String, name: ResearchText) {
        match self.agents.entry(id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(ResearchAgent {
                    id,
                    names: vec![name],
                });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if !entry.get().names.contains(&name) {
                    entry.get_mut().names.push(name);
                }
            }
        }
    }

    fn parse_text_group(
        &mut self,
        node: Node<'_, '_>,
        expected_child: &str,
        path: &str,
        expected_child_attributes: &[(Option<&str>, &str)],
    ) -> Result<Vec<ResearchText>, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut values = Vec::new();
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() == expected_child {
                values.push(self.parse_research_text(
                    child,
                    &child_path,
                    expected_child_attributes,
                )?);
            } else {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
            }
        }
        record_order_loss(&values, path, self.contract, self.ledger);
        Ok(values)
    }

    fn parse_dates(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<(Vec<ResearchText>, Vec<ResearchText>), ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut issued = Vec::new();
        let mut modified = Vec::new();
        let mut count = 0usize;
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() != "date" {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
                continue;
            }
            count += 1;
            let date_type = child.attribute("dateType").ok_or_else(|| {
                ProjectionError::integrity("DataCite date is missing dateType")
                    .at_path(DATACITE_ARTIFACT)
            })?;
            let value = self.parse_research_text(child, &child_path, &[(None, "dateType")])?;
            if date_type == self.config.controlled().issued_date_type() {
                issued.push(value);
            } else if date_type == self.config.controlled().modified_date_type() {
                modified.push(value);
            } else {
                return Err(ProjectionError::integrity(format!(
                    "DataCite dateType {date_type:?} is outside caller policy"
                ))
                .at_path(DATACITE_ARTIFACT));
            }
        }
        if count > 1 {
            self.loss(LOSS_RESEARCH_ORDER_DROPPED, path);
        }
        Ok((issued, modified))
    }

    fn parse_alternate_identifiers(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<Vec<ResearchValue>, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut values = Vec::new();
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() != "alternateIdentifier" {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
                continue;
            }
            self.require_attribute(
                child,
                "alternateIdentifierType",
                self.config.controlled().identifier_type(),
                &child_path,
            )?;
            let lexical =
                self.parse_plain_text(child, &child_path, &[(None, "alternateIdentifierType")])?;
            values.push(research_value(
                lexical,
                self.config
                    .common()
                    .roles()
                    .iri(super::ResearchRole::XsdString),
            )?);
        }
        record_order_loss(&values, path, self.contract, self.ledger);
        Ok(values)
    }

    fn parse_related_identifiers(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<RelatedEntities, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut related = RelatedEntities::default();
        let mut count = 0usize;
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() != "relatedIdentifier" {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
                continue;
            }
            count += 1;
            self.require_attribute(
                child,
                "relatedIdentifierType",
                self.config.controlled().related_identifier_type(),
                &child_path,
            )?;
            let relation_type = child.attribute("relationType").ok_or_else(|| {
                ProjectionError::integrity("DataCite relatedIdentifier is missing relationType")
                    .at_path(DATACITE_ARTIFACT)
            })?;
            let value = self.parse_plain_text(
                child,
                &child_path,
                &[(None, "relatedIdentifierType"), (None, "relationType")],
            )?;
            if relation_type == self.config.controlled().landing_page_relation_type() {
                related.landing_pages.push(research_value(
                    value,
                    self.config
                        .common()
                        .roles()
                        .iri(super::ResearchRole::XsdString),
                )?);
            } else if relation_type == self.config.controlled().resource_relation_type() {
                validate_absolute_iri(&value, "DataCite related resource identity")?;
                related.resources.push(empty_resource(value));
            } else if relation_type == self.config.controlled().activity_relation_type() {
                validate_absolute_iri(&value, "DataCite related activity identity")?;
                related.activities.push(empty_activity(value));
            } else if relation_type == self.config.controlled().record_set_relation_type() {
                validate_absolute_iri(&value, "DataCite related record-set identity")?;
                related.record_sets.push(empty_record_set(value));
            } else {
                return Err(ProjectionError::integrity(format!(
                    "DataCite relationType {relation_type:?} is outside caller policy"
                ))
                .at_path(DATACITE_ARTIFACT));
            }
        }
        if count > 1 {
            self.loss(LOSS_RESEARCH_ORDER_DROPPED, path);
        }
        reject_related_duplicates(&related)?;
        Ok(related)
    }

    fn parse_rights(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<Vec<ResearchValue>, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut rights = Vec::new();
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() != "rights" {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
                continue;
            }
            let lexical = self.parse_research_text(child, &child_path, &[(None, "rightsURI")])?;
            if let Some(iri) = child.attribute("rightsURI") {
                validate_absolute_iri(iri, "DataCite rights IRI")?;
                if lexical.value != iri {
                    self.loss(LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED, &child_path);
                }
                rights.push(ResearchValue::Iri {
                    value: iri.to_owned(),
                });
            } else {
                rights.push(ResearchValue::Text(lexical));
            }
        }
        record_order_loss(&rights, path, self.contract, self.ledger);
        Ok(rights)
    }

    fn parse_descriptions(
        &mut self,
        node: Node<'_, '_>,
        path: &str,
    ) -> Result<Vec<ResearchText>, ProjectionError> {
        record_unknown_attributes(node, &[], path, self.contract, self.ledger);
        let mut values = Vec::new();
        for (index, child) in node.children().filter(Node::is_element).enumerate() {
            require_datacite_namespace(child, self.config)?;
            let child_path = format!("{path}/{}[{}]", child.tag_name().name(), index + 1);
            if child.tag_name().name() != "description" {
                self.loss(LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED, &child_path);
                continue;
            }
            self.require_attribute(
                child,
                "descriptionType",
                self.config.controlled().description_type(),
                &child_path,
            )?;
            values.push(self.parse_research_text(
                child,
                &child_path,
                &[(None, "descriptionType")],
            )?);
        }
        record_order_loss(&values, path, self.contract, self.ledger);
        Ok(values)
    }
}

#[derive(Default)]
struct RelatedEntities {
    landing_pages: Vec<ResearchValue>,
    resources: Vec<ResearchResource>,
    activities: Vec<ResearchActivity>,
    record_sets: Vec<ResearchRecordSet>,
}

fn empty_resource(id: String) -> ResearchResource {
    ResearchResource {
        id,
        names: Vec::new(),
        descriptions: Vec::new(),
        paths: Vec::new(),
        urls: Vec::new(),
        media_types: Vec::new(),
        formats: Vec::new(),
        byte_size: None,
        checksums: Vec::new(),
    }
}

fn empty_activity(id: String) -> ResearchActivity {
    ResearchActivity {
        id,
        names: Vec::new(),
        instruments: Vec::new(),
        actors: Vec::new(),
        objects: Vec::new(),
        results: Vec::new(),
        end_times: Vec::new(),
        workflows: Vec::new(),
    }
}

fn empty_record_set(id: String) -> ResearchRecordSet {
    ResearchRecordSet {
        id,
        names: Vec::new(),
        descriptions: Vec::new(),
        fields: Vec::new(),
        rows: Vec::new(),
    }
}

fn reject_related_duplicates(related: &RelatedEntities) -> Result<(), ProjectionError> {
    for (description, ids) in [
        (
            "landing page",
            related
                .landing_pages
                .iter()
                .map(value_lexical)
                .collect::<Vec<_>>(),
        ),
        (
            "resource",
            related
                .resources
                .iter()
                .map(|value| value.id.clone())
                .collect(),
        ),
        (
            "activity",
            related
                .activities
                .iter()
                .map(|value| value.id.clone())
                .collect(),
        ),
        (
            "record set",
            related
                .record_sets
                .iter()
                .map(|value| value.id.clone())
                .collect(),
        ),
    ] {
        if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
            return Err(ProjectionError::integrity(format!(
                "DataCite related identifiers contain a duplicate {description}"
            ))
            .at_path(DATACITE_ARTIFACT));
        }
    }
    Ok(())
}

fn research_value(value: String, xsd_string: &str) -> Result<ResearchValue, ProjectionError> {
    if validate_absolute_iri(&value, "DataCite identifier").is_ok() {
        ResearchValue::iri(value)
    } else {
        Ok(ResearchValue::Text(ResearchText::plain(value, xsd_string)?))
    }
}

fn require_datacite_element(
    node: Node<'_, '_>,
    local: &str,
    config: &DataCiteConfig,
) -> Result<(), ProjectionError> {
    require_datacite_namespace(node, config)?;
    if node.tag_name().name() != local {
        return Err(ProjectionError::syntax(format!(
            "expected DataCite {local:?} root, found {:?}",
            node.tag_name().name()
        ))
        .at_path(DATACITE_ARTIFACT));
    }
    Ok(())
}

fn require_datacite_namespace(
    node: Node<'_, '_>,
    config: &DataCiteConfig,
) -> Result<(), ProjectionError> {
    if node.tag_name().namespace() != Some(config.namespace_iri()) {
        return Err(ProjectionError::integrity(format!(
            "DataCite element {:?} is outside the caller-supplied namespace",
            node.tag_name().name()
        ))
        .at_path(DATACITE_ARTIFACT));
    }
    Ok(())
}

fn namespaced_attribute<'a>(node: Node<'a, '_>, namespace: &str, local: &str) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.namespace() == Some(namespace) && attribute.name() == local)
        .map(|attribute| attribute.value())
}

fn record_unknown_attributes(
    node: Node<'_, '_>,
    expected: &[(Option<&str>, &str)],
    path: &str,
    contract: &LossLedger,
    ledger: &mut LossLedger,
) {
    for attribute in node.attributes() {
        let known = expected.iter().any(|(namespace, local)| {
            attribute.namespace() == *namespace && attribute.name() == *local
        });
        if !known {
            record_loss(
                ledger,
                contract,
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                DATACITE_ARTIFACT,
                &format!("{path}/@{}", attribute.name()),
            );
        }
    }
}

fn simple_element_text(node: Node<'_, '_>, path: &str) -> Result<String, ProjectionError> {
    if node.children().any(|child| child.is_element()) {
        return Err(
            ProjectionError::syntax(format!("DataCite {path} must contain text only"))
                .at_path(DATACITE_ARTIFACT),
        );
    }
    let mut value = String::new();
    for child in node.children().filter(Node::is_text) {
        value.push_str(child.text().unwrap_or_default());
    }
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(
            ProjectionError::integrity(format!("DataCite {path} cannot be empty"))
                .at_path(DATACITE_ARTIFACT),
        );
    }
    Ok(value)
}

fn record_order_loss<T>(values: &[T], path: &str, contract: &LossLedger, ledger: &mut LossLedger) {
    if values.len() > 1 {
        record_loss(
            ledger,
            contract,
            LOSS_RESEARCH_ORDER_DROPPED,
            DATACITE_ARTIFACT,
            path,
        );
    }
}

fn missing_datacite(name: &str) -> ProjectionError {
    ProjectionError::integrity(format!("DataCite XML is missing required {name}"))
        .at_path(DATACITE_ARTIFACT)
}

fn write_datacite(
    model: &ResearchObjectModel,
    config: &DataCiteConfig,
    ledger: &mut LossLedger,
) -> Result<Vec<u8>, ProjectionError> {
    let dataset = &model.dataset;
    // The dataset identity is mandatory caller/document data and is therefore
    // a valid primary identifier when the source profile has no separate
    // identifier field. No DOI or other value is synthesized here.
    let identifier = dataset
        .identifiers
        .first()
        .map_or_else(|| dataset.id.clone(), value_lexical);
    if dataset.titles.is_empty() {
        return Err(ProjectionError::integrity(
            "DataCite 4.6 requires at least one title",
        ));
    }
    if dataset.creators.is_empty() {
        return Err(ProjectionError::integrity(
            "DataCite 4.6 requires at least one creator",
        ));
    }
    if dataset.publishers.is_empty() {
        return Err(ProjectionError::integrity(
            "DataCite 4.6 requires at least one publisher",
        ));
    }
    let publication_year = dataset
        .issued
        .iter()
        .find_map(|value| extract_year(&value.value))
        .ok_or_else(|| {
            ProjectionError::integrity(
                "DataCite 4.6 requires an issued value beginning with a four-digit year",
            )
        })?;
    let agents: BTreeMap<&str, &ResearchAgent> = model
        .agents
        .iter()
        .map(|agent| (agent.id.as_str(), agent))
        .collect();
    let contract = purrdf_core::rdf_to_research_object_loss_ledger(DATACITE_PROFILE);
    let mut output = XmlOutput::new(config.common().limits().max_artifact_bytes());

    output.push("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
    output.push("<resource xmlns=\"")?;
    output.push(&escape_xml_attribute(config.namespace_iri())?)?;
    output.push("\" xmlns:xsi=\"")?;
    output.push(&escape_xml_attribute(config.xml_schema_instance_iri())?)?;
    output.push("\" xsi:schemaLocation=\"")?;
    output.push(&escape_xml_attribute(&format!(
        "{} {}",
        config.namespace_iri(),
        config.schema_location()
    ))?)?;
    output.push("\">\n")?;

    output.line(
        1,
        &format!(
            "<identifier identifierType=\"{}\">{}</identifier>",
            escape_xml_attribute(config.controlled().identifier_type())?,
            escape_xml_text(&identifier)?
        ),
    )?;
    output.line(1, "<creators>")?;
    for creator_id in &dataset.creators {
        let agent = agents.get(creator_id.as_str()).copied().ok_or_else(|| {
            ProjectionError::integrity(format!("missing DataCite creator agent `{creator_id}`"))
        })?;
        write_creator(&mut output, agent, config, ledger, &contract)?;
    }
    output.line(1, "</creators>")?;

    output.line(1, "<titles>")?;
    for title in &dataset.titles {
        record_text_fidelity(title, config, ledger, &contract, "dataset:title");
        output.line(2, &text_element("title", title, &[])?)?;
    }
    output.line(1, "</titles>")?;

    let publisher = agents
        .get(dataset.publishers[0].as_str())
        .copied()
        .ok_or_else(|| ProjectionError::integrity("missing DataCite publisher agent"))?;
    write_publisher(&mut output, publisher, config, ledger, &contract)?;
    for publisher_id in dataset.publishers.iter().skip(1) {
        write_loss(
            ledger,
            &contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            publisher_id,
        );
    }
    output.line(
        1,
        &format!(
            "<publicationYear>{}</publicationYear>",
            escape_xml_text(publication_year)?
        ),
    )?;
    output.line(
        1,
        &format!(
            "<resourceType resourceTypeGeneral=\"{}\">{}</resourceType>",
            escape_xml_attribute(config.controlled().resource_type_general())?,
            escape_xml_text(config.controlled().resource_type_general())?
        ),
    )?;

    if !dataset.keywords.is_empty() {
        output.line(1, "<subjects>")?;
        for keyword in &dataset.keywords {
            record_text_fidelity(keyword, config, ledger, &contract, "dataset:keyword");
            output.line(2, &text_element("subject", keyword, &[])?)?;
        }
        output.line(1, "</subjects>")?;
    }

    if !dataset.issued.is_empty() || !dataset.modified.is_empty() {
        output.line(1, "<dates>")?;
        for value in &dataset.issued {
            record_text_fidelity(value, config, ledger, &contract, "dataset:issued");
            output.line(
                2,
                &text_element(
                    "date",
                    value,
                    &[("dateType", config.controlled().issued_date_type())],
                )?,
            )?;
        }
        for value in &dataset.modified {
            record_text_fidelity(value, config, ledger, &contract, "dataset:modified");
            output.line(
                2,
                &text_element(
                    "date",
                    value,
                    &[("dateType", config.controlled().modified_date_type())],
                )?,
            )?;
        }
        output.line(1, "</dates>")?;
    }

    if dataset.identifiers.len() > 1 {
        output.line(1, "<alternateIdentifiers>")?;
        for value in dataset.identifiers.iter().skip(1) {
            output.line(
                2,
                &format!(
                    "<alternateIdentifier alternateIdentifierType=\"{}\">{}</alternateIdentifier>",
                    escape_xml_attribute(config.controlled().identifier_type())?,
                    escape_xml_text(&value_lexical(value))?
                ),
            )?;
        }
        output.line(1, "</alternateIdentifiers>")?;
    }

    let related = collect_related(model, config, ledger, &contract);
    if !related.is_empty() {
        output.line(1, "<relatedIdentifiers>")?;
        for (value, relation_type) in related {
            output.line(
                2,
                &format!(
                    "<relatedIdentifier relatedIdentifierType=\"{}\" relationType=\"{}\">{}</relatedIdentifier>",
                    escape_xml_attribute(config.controlled().related_identifier_type())?,
                    escape_xml_attribute(relation_type)?,
                    escape_xml_text(&value)?
                ),
            )?;
        }
        output.line(1, "</relatedIdentifiers>")?;
    }

    if let Some(version) = dataset.versions.first() {
        record_text_fidelity(version, config, ledger, &contract, "dataset:version");
        output.line(
            1,
            &format!("<version>{}</version>", escape_xml_text(&version.value)?),
        )?;
    }
    for version in dataset.versions.iter().skip(1) {
        write_loss(
            ledger,
            &contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &version.value,
        );
    }
    if !dataset.licenses.is_empty() {
        output.line(1, "<rightsList>")?;
        for license in &dataset.licenses {
            match license {
                ResearchValue::Iri { value } => output.line(
                    2,
                    &format!(
                        "<rights rightsURI=\"{}\">{}</rights>",
                        escape_xml_attribute(value)?,
                        escape_xml_text(value)?
                    ),
                )?,
                ResearchValue::Text(value) => {
                    record_text_fidelity(value, config, ledger, &contract, "dataset:license");
                    output.line(2, &text_element("rights", value, &[])?)?;
                }
            }
        }
        output.line(1, "</rightsList>")?;
    }
    if !dataset.descriptions.is_empty() {
        output.line(1, "<descriptions>")?;
        for description in &dataset.descriptions {
            record_text_fidelity(
                description,
                config,
                ledger,
                &contract,
                "dataset:description",
            );
            output.line(
                2,
                &text_element(
                    "description",
                    description,
                    &[("descriptionType", config.controlled().description_type())],
                )?,
            )?;
        }
        output.line(1, "</descriptions>")?;
    }
    output.push("</resource>\n")?;

    let represented_agents: BTreeSet<&str> = dataset
        .creators
        .iter()
        .chain(&dataset.publishers)
        .map(String::as_str)
        .collect();
    for agent in &model.agents {
        if !represented_agents.contains(agent.id.as_str()) {
            write_loss(
                ledger,
                &contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &agent.id,
            );
        }
    }
    Ok(output.finish())
}

fn write_creator(
    output: &mut XmlOutput,
    agent: &ResearchAgent,
    config: &DataCiteConfig,
    ledger: &mut LossLedger,
    contract: &LossLedger,
) -> Result<(), ProjectionError> {
    let name = agent.names.first().ok_or_else(|| {
        ProjectionError::integrity(format!("DataCite creator `{}` requires a name", agent.id))
    })?;
    record_text_fidelity(name, config, ledger, contract, &agent.id);
    if agent.names.len() > 1 {
        write_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &agent.id,
        );
    }
    output.line(2, "<creator>")?;
    output.line(
        3,
        &text_element(
            "creatorName",
            name,
            &[("nameType", config.controlled().creator_name_type())],
        )?,
    )?;
    output.line(
        3,
        &format!(
            "<nameIdentifier nameIdentifierScheme=\"{}\" schemeURI=\"{}\">{}</nameIdentifier>",
            escape_xml_attribute(config.controlled().agent_identifier_scheme())?,
            escape_xml_attribute(config.controlled().agent_identifier_scheme_uri())?,
            escape_xml_text(&agent.id)?
        ),
    )?;
    output.line(2, "</creator>")
}

fn write_publisher(
    output: &mut XmlOutput,
    agent: &ResearchAgent,
    config: &DataCiteConfig,
    ledger: &mut LossLedger,
    contract: &LossLedger,
) -> Result<(), ProjectionError> {
    let name = agent.names.first().ok_or_else(|| {
        ProjectionError::integrity(format!("DataCite publisher `{}` requires a name", agent.id))
    })?;
    record_text_fidelity(name, config, ledger, contract, &agent.id);
    if agent.names.len() > 1 {
        write_loss(
            ledger,
            contract,
            LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
            &agent.id,
        );
    }
    output.line(
        1,
        &text_element(
            "publisher",
            name,
            &[
                ("publisherIdentifier", &agent.id),
                (
                    "publisherIdentifierScheme",
                    config.controlled().agent_identifier_scheme(),
                ),
                (
                    "schemeURI",
                    config.controlled().agent_identifier_scheme_uri(),
                ),
            ],
        )?,
    )
}

fn collect_related<'a>(
    model: &'a ResearchObjectModel,
    config: &'a DataCiteConfig,
    ledger: &mut LossLedger,
    contract: &LossLedger,
) -> Vec<(String, &'a str)> {
    let mut values = Vec::new();
    let resource_by_id: BTreeMap<&str, &ResearchResource> = model
        .resources
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect();
    let activity_by_id: BTreeMap<&str, &ResearchActivity> = model
        .activities
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect();
    let record_set_by_id: BTreeMap<&str, &ResearchRecordSet> = model
        .record_sets
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect();
    for landing_page in &model.dataset.landing_pages {
        values.push((
            value_lexical(landing_page),
            config.controlled().landing_page_relation_type(),
        ));
    }
    for resource_id in &model.dataset.resources {
        values.push((
            resource_id.clone(),
            config.controlled().resource_relation_type(),
        ));
        if resource_by_id
            .get(resource_id.as_str())
            .is_some_and(|resource| resource_has_detail(resource))
        {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                resource_id,
            );
        }
    }
    for activity_id in &model.dataset.activities {
        values.push((
            activity_id.clone(),
            config.controlled().activity_relation_type(),
        ));
        if activity_by_id
            .get(activity_id.as_str())
            .is_some_and(|activity| activity_has_detail(activity))
        {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                activity_id,
            );
        }
    }
    for record_set_id in &model.dataset.record_sets {
        values.push((
            record_set_id.clone(),
            config.controlled().record_set_relation_type(),
        ));
        if record_set_by_id
            .get(record_set_id.as_str())
            .is_some_and(|record_set| record_set_has_detail(record_set))
        {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                record_set_id,
            );
        }
    }
    for resource in &model.resources {
        if !model.dataset.resources.contains(&resource.id) {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &resource.id,
            );
        }
    }
    for activity in &model.activities {
        if !model.dataset.activities.contains(&activity.id) {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &activity.id,
            );
        }
    }
    for record_set in &model.record_sets {
        if !model.dataset.record_sets.contains(&record_set.id) {
            write_loss(
                ledger,
                contract,
                LOSS_RESEARCH_PROFILE_FIELD_DROPPED,
                &record_set.id,
            );
        }
    }
    values.sort();
    values.dedup();
    values
}

fn value_lexical(value: &ResearchValue) -> String {
    match value {
        ResearchValue::Iri { value } => value.clone(),
        ResearchValue::Text(value) => value.value.clone(),
    }
}

fn resource_has_detail(resource: &ResearchResource) -> bool {
    !resource.names.is_empty()
        || !resource.descriptions.is_empty()
        || !resource.paths.is_empty()
        || !resource.urls.is_empty()
        || !resource.media_types.is_empty()
        || !resource.formats.is_empty()
        || resource.byte_size.is_some()
        || !resource.checksums.is_empty()
}

fn activity_has_detail(activity: &ResearchActivity) -> bool {
    !activity.names.is_empty()
        || !activity.instruments.is_empty()
        || !activity.actors.is_empty()
        || !activity.objects.is_empty()
        || !activity.results.is_empty()
        || !activity.end_times.is_empty()
        || !activity.workflows.is_empty()
}

fn record_set_has_detail(record_set: &ResearchRecordSet) -> bool {
    !record_set.names.is_empty()
        || !record_set.descriptions.is_empty()
        || !record_set.fields.is_empty()
        || !record_set.rows.is_empty()
}

fn extract_year(value: &str) -> Option<&str> {
    let year = value.get(..4)?;
    year.chars()
        .all(|character| character.is_ascii_digit())
        .then_some(year)
}

fn text_element(
    name: &str,
    value: &ResearchText,
    attributes: &[(&str, &str)],
) -> Result<String, ProjectionError> {
    let mut output = format!("<{name}");
    for (attribute, value) in attributes {
        output.push(' ');
        output.push_str(attribute);
        output.push_str("=\"");
        output.push_str(&escape_xml_attribute(value)?);
        output.push('"');
    }
    if let Some(language) = &value.language {
        output.push_str(" xml:lang=\"");
        output.push_str(&escape_xml_attribute(language)?);
        output.push('"');
    }
    output.push('>');
    output.push_str(&escape_xml_text(&value.value)?);
    output.push_str("</");
    output.push_str(name);
    output.push('>');
    Ok(output)
}

fn record_text_fidelity(
    value: &ResearchText,
    config: &DataCiteConfig,
    ledger: &mut LossLedger,
    contract: &LossLedger,
    subject: &str,
) {
    let roles = config.common().roles();
    let faithful_datatype = if value.direction.is_some() {
        roles.iri(super::ResearchRole::RdfDirLangString)
    } else if value.language.is_some() {
        roles.iri(super::ResearchRole::RdfLangString)
    } else {
        roles.iri(super::ResearchRole::XsdString)
    };
    if value.direction.is_some() || value.datatype != faithful_datatype {
        write_loss(
            ledger,
            contract,
            LOSS_RESEARCH_LITERAL_FIDELITY_DROPPED,
            subject,
        );
    }
}

fn write_loss(ledger: &mut LossLedger, contract: &LossLedger, code: &'static str, subject: &str) {
    record_loss(ledger, contract, code, DATACITE_ARTIFACT, subject);
}

struct XmlOutput {
    bytes: Vec<u8>,
    limit: usize,
}

impl XmlOutput {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        if self
            .bytes
            .len()
            .checked_add(value.len())
            .is_none_or(|length| length > self.limit)
        {
            return Err(ProjectionError::limit(format!(
                "DataCite XML exceeds the {}-byte artifact limit",
                self.limit
            )));
        }
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn line(&mut self, indent: usize, value: &str) -> Result<(), ProjectionError> {
        for _ in 0..indent {
            self.push("  ")?;
        }
        self.push(value)?;
        self.push("\n")
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::{
        ProjectionLimits, RESEARCH_ROLES, ResearchObjectIdentity, ResearchObjectPolicy,
        ResearchObjectRoles,
    };

    const INPUT: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/datacite-4.6/input.xml");
    const GOLDEN: &[u8] =
        include_bytes!("../../../tests/fixtures/research-objects/datacite-4.6/golden.xml");

    fn config() -> DataCiteConfig {
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
        let controlled = DataCiteControlledValues::new(
            "DOI",
            "Dataset",
            "Personal",
            "IRI",
            "https://example.org/schemes/iri",
            "URL",
            "IsIdenticalTo",
            "HasPart",
            "IsCompiledBy",
            "HasMetadata",
            "Available",
            "Updated",
            "Abstract",
        )
        .expect("controlled values");
        DataCiteConfig::new(
            common,
            "https://example.org/datacite/4.6",
            "https://example.org/xml-schema-instance",
            "https://example.org/datacite/4.6/schema.xsd",
            controlled,
        )
        .expect("DataCite config")
    }

    fn package(bytes: impl Into<Vec<u8>>, config: &DataCiteConfig) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(config.common().limits(), [(DATACITE_ARTIFACT, bytes)])
            .expect("package")
    }

    #[test]
    fn fixture_has_exact_located_losses_and_stable_rewrite() {
        let config = config();
        let read = read_datacite(&package(INPUT, &config), &config).expect("read fixture");
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

        let projected = project_datacite(&read.dataset, &config).expect("project fixture");
        let actual = projected
            .package
            .get(DATACITE_ARTIFACT)
            .expect("DataCite artifact");
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

        let reread = read_datacite(&projected.package, &config).expect("read canonical output");
        let reprojected = project_datacite(&reread.dataset, &config).expect("rewrite");
        assert_eq!(projected.package, reprojected.package);
        assert_eq!(projected.model, reread.model);
    }

    #[test]
    fn reader_rejects_dtd_duplicates_namespace_and_controlled_drift() {
        let config = config();
        let input = String::from_utf8(INPUT.to_vec()).expect("UTF-8 fixture");

        let dtd = input.replacen(
            "?>\n",
            "?>\n<!DOCTYPE resource [<!ENTITY x \"boom\">]>\n",
            1,
        );
        assert!(read_datacite(&package(dtd, &config), &config).is_err());

        let malformed_mixed_case = input.replacen("?>\n", "?>\n<!doctype resource>\n", 1);
        assert!(read_datacite(&package(malformed_mixed_case, &config), &config).is_err());

        let duplicate = input.replace(
            "  <creators>",
            "  <identifier identifierType=\"DOI\">duplicate</identifier>\n  <creators>",
        );
        assert!(read_datacite(&package(duplicate, &config), &config).is_err());

        let namespace = input.replace(
            "xmlns=\"https://example.org/datacite/4.6\"",
            "xmlns=\"https://example.org/datacite/wrong\"",
        );
        assert!(read_datacite(&package(namespace, &config), &config).is_err());

        let controlled = input.replacen("dateType=\"Available\"", "dateType=\"Issued\"", 1);
        assert!(read_datacite(&package(controlled, &config), &config).is_err());
    }

    #[test]
    fn projection_records_datacite_profile_field_loss() {
        let config = config();
        let mut model = read_datacite(&package(INPUT, &config), &config)
            .expect("read fixture")
            .model;
        model.resources[0].names.push(
            ResearchText::plain(
                "Training data",
                config
                    .common()
                    .roles()
                    .iri(super::super::ResearchRole::XsdString),
            )
            .expect("resource name"),
        );
        model.dataset.versions.push(
            ResearchText::plain(
                "2.0",
                config
                    .common()
                    .roles()
                    .iri(super::super::ResearchRole::XsdString),
            )
            .expect("second version"),
        );
        let dataset = lift_research_object(model, config.common()).expect("lift model");
        let projected = project_datacite(dataset.as_ref(), &config).expect("project model");
        assert!(
            projected
                .loss_ledger
                .entries()
                .iter()
                .any(|entry| entry.code == LOSS_RESEARCH_PROFILE_FIELD_DROPPED)
        );
        let xml = std::str::from_utf8(
            projected
                .package
                .get(DATACITE_ARTIFACT)
                .expect("DataCite artifact"),
        )
        .expect("DataCite UTF-8");
        assert_eq!(xml.matches("<version>").count(), 1);
    }

    #[test]
    fn config_requires_distinct_relations_and_absolute_schema_identity() {
        let config = config();
        assert!(
            DataCiteConfig::new(
                config.common().clone(),
                config.namespace_iri(),
                config.xml_schema_instance_iri(),
                "relative-schema.xsd",
                config.controlled().clone(),
            )
            .is_err()
        );
        assert!(
            DataCiteControlledValues::new(
                "DOI",
                "Dataset",
                "Personal",
                "IRI",
                "https://example.org/schemes/iri",
                "URL",
                "Same",
                "Same",
                "Activity",
                "RecordSet",
                "Available",
                "Updated",
                "Abstract",
            )
            .is_err()
        );
    }
}
