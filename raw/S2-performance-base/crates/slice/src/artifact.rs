// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed artifact inventory: roles, records, and content-addressed digests.

use purrdf_lex::json::{Object, Value};

use crate::error::SliceError;
use crate::json_form;

/// The role (kind) of a file artifact within a slice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactRole {
    /// The required `manifest.ttl` describing the slice.
    Manifest,
    /// The optional `module.ttl` ontology module.
    Module,
    /// The optional `shapes.ttl` SHACL shapes.
    Shapes,
    /// A file under `mappings/`.
    Mapping,
    /// A SPARQL competency query under `queries/competency/`.
    CompetencyQuery,
    /// A SPARQL verification query under `queries/verify/`.
    VerifyQuery,
    /// A test DSL file under `tests/` (excluding counter-examples).
    TestDsl,
    /// An example file under `examples/`.
    Example,
    /// A counter-example file under `tests/counter-examples/`.
    CounterExample,
    /// The `docs.md` documentation file.
    Documentation,
    /// A translation catalog under `i18n/`.
    TranslationCatalog,
    /// The `CITATION.cff` citation metadata.
    Citation,
    /// Any file not matched by the above roles (forward-compat open variant).
    Other(String),
}

/// A single artifact within a slice: role, path, MIME type, and digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// What kind of file this is.
    pub role: ArtifactRole,
    /// Normalized, relative path within the slice directory (no `..`, no leading `/`).
    pub logical_path: String,
    /// MIME type (e.g. `"text/turtle"`, `"application/sparql-query"`, `"text/markdown"`).
    pub media_type: String,
    /// SHA-256 hex digest of the raw file bytes (64 lowercase hex chars).
    pub raw_digest: String,
    /// For RDF artifacts: SHA-256 hex of the canonical N-Triples (sorted).
    ///
    /// `None` means **this file is not RDF**, and nothing else. It is a discriminant on
    /// the artifact's KIND, never a report that computing the digest was attempted and
    /// failed: discovery propagates every such failure instead
    /// (`catalog::collect_artifacts`), so an RDF artifact always carries `Some`. That is
    /// exactly the invariant `cache::phase_artifact_digest` relies on when it hard-fails
    /// a semantics-sensitive phase whose artifact has no digest.
    pub semantic_digest: Option<String>,
    /// The raw bytes of the artifact (content cache).
    pub content: Vec<u8>,
}

/// The unit roles and their JSON names, in declaration order.
const UNIT_ROLES: [(ArtifactRole, &str); 12] = [
    (ArtifactRole::Manifest, "Manifest"),
    (ArtifactRole::Module, "Module"),
    (ArtifactRole::Shapes, "Shapes"),
    (ArtifactRole::Mapping, "Mapping"),
    (ArtifactRole::CompetencyQuery, "CompetencyQuery"),
    (ArtifactRole::VerifyQuery, "VerifyQuery"),
    (ArtifactRole::TestDsl, "TestDsl"),
    (ArtifactRole::Example, "Example"),
    (ArtifactRole::CounterExample, "CounterExample"),
    (ArtifactRole::Documentation, "Documentation"),
    (ArtifactRole::TranslationCatalog, "TranslationCatalog"),
    (ArtifactRole::Citation, "Citation"),
];

impl ArtifactRole {
    /// The role as JSON: a unit role is its variant name (`"Manifest"`), and
    /// [`ArtifactRole::Other`] is `{"Other": <name>}`.
    pub fn to_json(&self) -> Value {
        if let Self::Other(name) = self {
            return Value::from(Object::new().with("Other", name));
        }
        UNIT_ROLES
            .iter()
            .find(|(role, _)| role == self)
            .map(|(_, name)| Value::from(*name))
            .expect("every role but Other is a unit role")
    }

    /// The role [`ArtifactRole::to_json`] wrote.
    ///
    /// # Errors
    ///
    /// [`SliceError::Json`] for any other value.
    pub fn from_json(value: &Value) -> Result<Self, SliceError> {
        if let Some(name) = value.as_str() {
            return UNIT_ROLES
                .iter()
                .find(|(_, unit)| *unit == name)
                .map(|(role, _)| role.clone())
                .ok_or_else(|| SliceError::Json(format!("unknown artifact role `{name}`")));
        }
        json_form::newtype_variant(value, "Other").map(Self::Other)
    }
}

impl ArtifactRecord {
    /// The record as a JSON object whose members are its fields, in
    /// declaration order: `content` is an array of byte values and an absent
    /// `semantic_digest` is `null`.
    pub fn to_json(&self) -> Value {
        Value::from(
            Object::new()
                .with("role", self.role.to_json())
                .with("logical_path", &self.logical_path)
                .with("media_type", &self.media_type)
                .with("raw_digest", &self.raw_digest)
                .with("semantic_digest", self.semantic_digest.as_deref())
                .with("content", self.content.as_slice()),
        )
    }

    /// The record [`ArtifactRecord::to_json`] wrote.
    ///
    /// # Errors
    ///
    /// [`SliceError::Json`] when a field is missing or has the wrong type.
    pub fn from_json(value: &Value) -> Result<Self, SliceError> {
        let object = json_form::object(value, "an artifact record")?;
        Ok(Self {
            role: ArtifactRole::from_json(json_form::field(object, "role")?)?,
            logical_path: json_form::string(object, "logical_path")?,
            media_type: json_form::string(object, "media_type")?,
            raw_digest: json_form::string(object, "raw_digest")?,
            semantic_digest: json_form::optional_string(object, "semantic_digest")?,
            content: json_form::bytes(object, "content")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_record_json_round_trips_every_role() {
        let mut roles: Vec<ArtifactRole> =
            UNIT_ROLES.iter().map(|(role, _)| role.clone()).collect();
        roles.push(ArtifactRole::Other("notes".to_owned()));
        for role in roles {
            let record = ArtifactRecord {
                role,
                logical_path: "queries/verify/a.rq".to_owned(),
                media_type: "application/sparql-query".to_owned(),
                raw_digest: "ab".repeat(32),
                semantic_digest: None,
                content: vec![0, 7, 255],
            };
            assert_eq!(
                ArtifactRecord::from_json(&record.to_json()).unwrap(),
                record
            );
        }
    }

    #[test]
    fn artifact_record_json_spells_roles_and_fields_as_named() {
        let record = ArtifactRecord {
            role: ArtifactRole::Other("x".to_owned()),
            logical_path: "p".to_owned(),
            media_type: "m".to_owned(),
            raw_digest: "r".to_owned(),
            semantic_digest: Some("s".to_owned()),
            content: vec![1],
        };
        assert_eq!(
            purrdf_lex::json::write_compact(&record.to_json()),
            r#"{"role":{"Other":"x"},"logical_path":"p","media_type":"m","raw_digest":"r","semantic_digest":"s","content":[1]}"#
        );
        assert_eq!(
            purrdf_lex::json::write_compact(&ArtifactRole::Shapes.to_json()),
            r#""Shapes""#
        );
    }

    #[test]
    fn artifact_role_from_json_refuses_an_unknown_role_and_accepts_a_known_one() {
        assert!(ArtifactRole::from_json(&Value::from("Shape")).is_err());
        assert_eq!(
            ArtifactRole::from_json(&Value::from("Shapes")).unwrap(),
            ArtifactRole::Shapes
        );
    }

    #[test]
    fn artifact_record_from_json_refuses_a_byte_over_255_and_accepts_255() {
        let mut value = ArtifactRecord {
            role: ArtifactRole::Module,
            logical_path: "module.ttl".to_owned(),
            media_type: "text/turtle".to_owned(),
            raw_digest: String::new(),
            semantic_digest: None,
            content: vec![255],
        }
        .to_json();
        assert!(ArtifactRecord::from_json(&value).is_ok());
        value["content"][0] = Value::from(256_u16);
        assert!(ArtifactRecord::from_json(&value).is_err());
    }
}
