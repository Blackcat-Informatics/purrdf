// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Semantic RDF role understood by the format-neutral research-object pivot.
///
/// The enum names roles, not vocabulary terms. A caller must bind every role to
/// an absolute IRI in [`ResearchObjectRoles`]; PurRDF supplies no vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResearchRole {
    /// `rdf:type`-equivalent predicate.
    RdfType,
    /// Dataset class.
    DatasetClass,
    /// Human-readable title predicate.
    Title,
    /// Human-readable description predicate.
    Description,
    /// Identifier predicate.
    Identifier,
    /// Version predicate.
    Version,
    /// Issue/publication date predicate.
    Issued,
    /// Modification date predicate.
    Modified,
    /// Landing-page predicate.
    LandingPage,
    /// Keyword predicate.
    Keyword,
    /// License predicate.
    License,
    /// Creator relation.
    Creator,
    /// Publisher relation.
    Publisher,
    /// Dataset-to-resource relation.
    HasResource,
    /// Dataset-to-activity relation.
    HasActivity,
    /// Dataset-to-record-set relation.
    HasRecordSet,
    /// Agent class.
    AgentClass,
    /// Agent name predicate.
    AgentName,
    /// Resource class.
    ResourceClass,
    /// Resource name predicate.
    ResourceName,
    /// Resource description predicate.
    ResourceDescription,
    /// Resource package-path predicate.
    ResourcePath,
    /// Resource access/download URL predicate.
    ResourceUrl,
    /// Resource media-type predicate.
    MediaType,
    /// Resource format predicate.
    Format,
    /// Resource byte-size predicate.
    ByteSize,
    /// Resource-to-checksum relation.
    Checksum,
    /// Checksum class.
    ChecksumClass,
    /// Checksum algorithm predicate.
    ChecksumAlgorithm,
    /// Checksum value predicate.
    ChecksumValue,
    /// Activity class.
    ActivityClass,
    /// Activity name predicate.
    ActivityName,
    /// Activity instrument relation.
    Instrument,
    /// Activity actor relation.
    Actor,
    /// Activity input/object relation.
    Object,
    /// Activity result relation.
    Result,
    /// Activity end-time predicate.
    EndTime,
    /// Activity workflow relation.
    Workflow,
    /// Record-set class.
    RecordSetClass,
    /// Record-set name predicate.
    RecordSetName,
    /// Record-set description predicate.
    RecordSetDescription,
    /// Record-set-to-field relation.
    HasField,
    /// Record-set inline-row predicate.
    HasRow,
    /// Field class.
    FieldClass,
    /// Field name predicate.
    FieldName,
    /// Field datatype predicate.
    FieldDataType,
    /// Canonical JSON row datatype.
    JsonDatatype,
    /// RDF language-string datatype.
    RdfLangString,
    /// RDF 1.2 directional-language-string datatype.
    RdfDirLangString,
    /// XML Schema string datatype.
    XsdString,
    /// XML Schema non-negative-integer datatype.
    XsdNonNegativeInteger,
    /// XML Schema date-time datatype.
    XsdDateTime,
}

/// Every mandatory research-object role, in stable configuration order.
pub const RESEARCH_ROLES: &[ResearchRole] = &[
    ResearchRole::RdfType,
    ResearchRole::DatasetClass,
    ResearchRole::Title,
    ResearchRole::Description,
    ResearchRole::Identifier,
    ResearchRole::Version,
    ResearchRole::Issued,
    ResearchRole::Modified,
    ResearchRole::LandingPage,
    ResearchRole::Keyword,
    ResearchRole::License,
    ResearchRole::Creator,
    ResearchRole::Publisher,
    ResearchRole::HasResource,
    ResearchRole::HasActivity,
    ResearchRole::HasRecordSet,
    ResearchRole::AgentClass,
    ResearchRole::AgentName,
    ResearchRole::ResourceClass,
    ResearchRole::ResourceName,
    ResearchRole::ResourceDescription,
    ResearchRole::ResourcePath,
    ResearchRole::ResourceUrl,
    ResearchRole::MediaType,
    ResearchRole::Format,
    ResearchRole::ByteSize,
    ResearchRole::Checksum,
    ResearchRole::ChecksumClass,
    ResearchRole::ChecksumAlgorithm,
    ResearchRole::ChecksumValue,
    ResearchRole::ActivityClass,
    ResearchRole::ActivityName,
    ResearchRole::Instrument,
    ResearchRole::Actor,
    ResearchRole::Object,
    ResearchRole::Result,
    ResearchRole::EndTime,
    ResearchRole::Workflow,
    ResearchRole::RecordSetClass,
    ResearchRole::RecordSetName,
    ResearchRole::RecordSetDescription,
    ResearchRole::HasField,
    ResearchRole::HasRow,
    ResearchRole::FieldClass,
    ResearchRole::FieldName,
    ResearchRole::FieldDataType,
    ResearchRole::JsonDatatype,
    ResearchRole::RdfLangString,
    ResearchRole::RdfDirLangString,
    ResearchRole::XsdString,
    ResearchRole::XsdNonNegativeInteger,
    ResearchRole::XsdDateTime,
];

/// Complete caller-owned RDF vocabulary binding for research objects.
///
/// There is deliberately no `Default`. Construction and deserialization reject
/// a missing role, a relative IRI, or two semantic roles bound to the same IRI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ResearchObjectRoles(BTreeMap<ResearchRole, String>);

impl ResearchObjectRoles {
    /// Validate one complete semantic-role map.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for a missing role, invalid IRI, or
    /// ambiguous duplicate binding.
    pub fn new(roles: BTreeMap<ResearchRole, String>) -> Result<Self, ProjectionError> {
        for role in RESEARCH_ROLES {
            let iri = roles.get(role).ok_or_else(|| {
                ProjectionError::configuration(format!(
                    "research-object vocabulary is missing role `{role:?}`"
                ))
            })?;
            validate_absolute_iri(iri, &format!("research-object role `{role:?}`"))?;
        }
        if roles.len() != RESEARCH_ROLES.len() {
            return Err(ProjectionError::configuration(
                "research-object vocabulary contains an unsupported role",
            ));
        }
        let mut inverse = BTreeMap::<&str, ResearchRole>::new();
        for (&role, iri) in &roles {
            if let Some(previous) = inverse.insert(iri, role) {
                return Err(ProjectionError::configuration(format!(
                    "research-object roles `{previous:?}` and `{role:?}` both bind `{iri}`"
                )));
            }
        }
        Ok(Self(roles))
    }

    /// Absolute IRI bound to a semantic role.
    pub fn iri(&self, role: ResearchRole) -> &str {
        self.0
            .get(&role)
            .expect("validated research-object role map is complete")
    }

    /// Deterministically ordered role map.
    pub const fn terms(&self) -> &BTreeMap<ResearchRole, String> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ResearchObjectRoles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = BTreeMap::<ResearchRole, String>::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// Caller-owned data identity policy shared by all research-object profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResearchObjectIdentity {
    dataset_iri: String,
    entity_base_iri: String,
}

impl ResearchObjectIdentity {
    /// Construct the required dataset identity and relative-entity base.
    ///
    /// # Errors
    ///
    /// Both values must be absolute IRIs; the entity base must end in `/` or `#`
    /// so deterministic relative resolution is unambiguous.
    pub fn new(
        dataset_iri: impl Into<String>,
        entity_base_iri: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let dataset_iri = dataset_iri.into();
        let entity_base_iri = entity_base_iri.into();
        validate_absolute_iri(&dataset_iri, "research-object dataset identity")?;
        validate_absolute_iri(&entity_base_iri, "research-object entity base")?;
        if !entity_base_iri.ends_with(['/', '#']) {
            return Err(ProjectionError::configuration(
                "research-object entity base must end in `/` or `#`",
            ));
        }
        Ok(Self {
            dataset_iri,
            entity_base_iri,
        })
    }

    /// Dataset IRI selected on projection and used on lift.
    pub fn dataset_iri(&self) -> &str {
        &self.dataset_iri
    }

    /// Caller-owned base for resolving profile-local entity identifiers.
    pub fn entity_base_iri(&self) -> &str {
        &self.entity_base_iri
    }

    /// Resolve a validated relative identifier under the caller-owned base.
    ///
    /// # Errors
    ///
    /// Rejects empty, absolute, backslash-containing, dot-segment, query, and
    /// fragment-bearing values so a package path cannot escape or reinterpret
    /// the configured identity boundary.
    pub fn resolve_relative(&self, relative: &str) -> Result<String, ProjectionError> {
        validate_relative_identifier(relative)?;
        let resolved = format!("{}{relative}", self.entity_base_iri);
        validate_absolute_iri(&resolved, "resolved research-object entity")?;
        Ok(resolved)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResearchObjectIdentity {
    dataset_iri: String,
    entity_base_iri: String,
}

impl<'de> Deserialize<'de> for ResearchObjectIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawResearchObjectIdentity::deserialize(deserializer)?;
        Self::new(raw.dataset_iri, raw.entity_base_iri).map_err(serde::de::Error::custom)
    }
}

/// Mandatory resource policy for common research-object interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ResearchObjectPolicy {
    limits: ProjectionLimits,
    max_records: usize,
    max_entities: usize,
    max_values: usize,
    max_json_depth: usize,
}

impl ResearchObjectPolicy {
    /// Construct validated processing bounds.
    ///
    /// # Errors
    ///
    /// Every bound must be non-zero, entity/value counts cannot exceed the
    /// record ceiling, and JSON nesting cannot exceed the kernel term-depth cap.
    pub fn new(
        limits: ProjectionLimits,
        max_records: usize,
        max_entities: usize,
        max_values: usize,
        max_json_depth: usize,
    ) -> Result<Self, ProjectionError> {
        if [max_records, max_entities, max_values, max_json_depth].contains(&0) {
            return Err(ProjectionError::configuration(
                "every research-object processing bound must be greater than zero",
            ));
        }
        if max_entities > max_records || max_values > max_records {
            return Err(ProjectionError::configuration(
                "research-object entity/value bounds must not exceed max_records",
            ));
        }
        if max_json_depth > limits.max_term_depth() {
            return Err(ProjectionError::configuration(format!(
                "research-object max_json_depth must not exceed max_term_depth ({})",
                limits.max_term_depth()
            )));
        }
        Ok(Self {
            limits,
            max_records,
            max_entities,
            max_values,
            max_json_depth,
        })
    }

    /// Package/term byte and depth limits.
    pub const fn limits(self) -> ProjectionLimits {
        self.limits
    }
    /// Maximum source records.
    pub const fn max_records(self) -> usize {
        self.max_records
    }
    /// Maximum normalized entities.
    pub const fn max_entities(self) -> usize {
        self.max_entities
    }
    /// Maximum normalized scalar/reference values.
    pub const fn max_values(self) -> usize {
        self.max_values
    }
    /// Maximum inline JSON nesting depth.
    pub const fn max_json_depth(self) -> usize {
        self.max_json_depth
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResearchObjectPolicy {
    limits: ProjectionLimits,
    max_records: usize,
    max_entities: usize,
    max_values: usize,
    max_json_depth: usize,
}

impl<'de> Deserialize<'de> for ResearchObjectPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawResearchObjectPolicy::deserialize(deserializer)?;
        Self::new(
            raw.limits,
            raw.max_records,
            raw.max_entities,
            raw.max_values,
            raw.max_json_depth,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Shared source-vocabulary, identity, and resource policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchObjectConfig {
    roles: ResearchObjectRoles,
    identity: ResearchObjectIdentity,
    policy: ResearchObjectPolicy,
}

impl ResearchObjectConfig {
    /// Construct common validated research-object configuration.
    pub const fn new(
        roles: ResearchObjectRoles,
        identity: ResearchObjectIdentity,
        policy: ResearchObjectPolicy,
    ) -> Self {
        Self {
            roles,
            identity,
            policy,
        }
    }

    /// Caller-owned RDF roles.
    pub const fn roles(&self) -> &ResearchObjectRoles {
        &self.roles
    }
    /// Caller-owned data identity.
    pub const fn identity(&self) -> &ResearchObjectIdentity {
        &self.identity
    }
    /// Mandatory processing bounds.
    pub const fn policy(&self) -> ResearchObjectPolicy {
        self.policy
    }
    /// Projection archive limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.policy.limits()
    }
}

fn validate_relative_identifier(value: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains('?')
        || value.contains('#')
        || value
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
        || value.contains("://")
    {
        return Err(ProjectionError::integrity(format!(
            "unsafe research-object relative identifier `{value}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role_map() -> BTreeMap<ResearchRole, String> {
        RESEARCH_ROLES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, role)| (role, format!("https://example.org/role/{index}")))
            .collect()
    }

    #[test]
    fn roles_are_complete_unique_absolute_and_revalidated() {
        let roles = ResearchObjectRoles::new(role_map()).expect("roles");
        let json = serde_json::to_vec(&roles).expect("serialize");
        assert_eq!(
            serde_json::from_slice::<ResearchObjectRoles>(&json).expect("deserialize"),
            roles
        );

        let mut missing = role_map();
        missing.remove(&ResearchRole::Title);
        assert!(ResearchObjectRoles::new(missing).is_err());

        let mut duplicate = role_map();
        duplicate.insert(
            ResearchRole::Title,
            duplicate[&ResearchRole::Description].clone(),
        );
        assert!(ResearchObjectRoles::new(duplicate).is_err());

        let mut relative = role_map();
        relative.insert(ResearchRole::Title, "relative".to_owned());
        assert!(ResearchObjectRoles::new(relative).is_err());
    }

    #[test]
    fn identity_resolution_is_caller_bounded_and_traversal_safe() {
        let identity = ResearchObjectIdentity::new(
            "https://example.org/dataset",
            "https://example.org/entity/",
        )
        .expect("identity");
        assert_eq!(
            identity.resolve_relative("data/rows.csv").expect("resolve"),
            "https://example.org/entity/data/rows.csv"
        );
        for unsafe_value in ["", "/root", "../escape", "a/./b", "a//b", "a\\b", "a?q"] {
            assert!(
                identity.resolve_relative(unsafe_value).is_err(),
                "{unsafe_value}"
            );
        }
        assert!(
            ResearchObjectIdentity::new("https://example.org/d", "https://example.org/base")
                .is_err()
        );
    }

    #[test]
    fn policy_has_no_implicit_or_contradictory_bounds() {
        let limits = ProjectionLimits::new(16, 1_000, 4_000, 8_000, 16).expect("limits");
        assert!(ResearchObjectPolicy::new(limits, 100, 50, 100, 16).is_ok());
        assert!(ResearchObjectPolicy::new(limits, 100, 101, 100, 16).is_err());
        assert!(ResearchObjectPolicy::new(limits, 100, 50, 100, 17).is_err());
        assert!(ResearchObjectPolicy::new(limits, 0, 1, 1, 1).is_err());
    }
}
