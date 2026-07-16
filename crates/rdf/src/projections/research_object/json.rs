// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::loss::{LossEntry, check_ledger_sound};
use purrdf_core::{LossLedger, RdfDataset, RdfLocation};
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};

use crate::native_codecs::jsonld::{parse_jsonld, serialize_dataset_to_jsonld};

use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage, validate_absolute_iri};
use super::{ResearchObjectConfig, ResearchObjectModel};

/// Caller-owned, locally interpreted JSON-LD context.
///
/// `value` is carried byte-semantically into emitted documents. `definitions`
/// is the complete offline expansion table used by profile adapters; PurRDF
/// never dereferences a context IRI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfflineJsonLdContext {
    value: Value,
    definitions: BTreeMap<String, String>,
}

impl OfflineJsonLdContext {
    /// Validate one emitted context value and its local term definitions.
    ///
    /// # Errors
    ///
    /// Rejects an unusable context shape, an import directive, an empty or
    /// keyword-like compact term, a non-absolute expansion, or ambiguous IRI
    /// aliases.
    pub fn new(
        value: Value,
        definitions: BTreeMap<String, String>,
    ) -> Result<Self, ProjectionError> {
        validate_context_value(&value)?;
        if definitions.is_empty() {
            return Err(ProjectionError::configuration(
                "offline JSON-LD context requires local term definitions",
            ));
        }
        let mut expanded = BTreeSet::new();
        for (term, iri) in &definitions {
            if term.is_empty()
                || term.starts_with('@')
                || term.chars().any(char::is_whitespace)
                || term.contains(['{', '}', '[', ']', '"'])
            {
                return Err(ProjectionError::configuration(format!(
                    "invalid offline JSON-LD compact term `{term}`"
                )));
            }
            validate_absolute_iri(iri, &format!("offline JSON-LD term `{term}`"))?;
            if !expanded.insert(iri.as_str()) {
                return Err(ProjectionError::configuration(format!(
                    "offline JSON-LD context maps more than one compact term to `{iri}`"
                )));
            }
        }
        Ok(Self { value, definitions })
    }

    /// Exact JSON value emitted as `@context`.
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Complete, deterministic compact-term expansion table.
    pub const fn definitions(&self) -> &BTreeMap<String, String> {
        &self.definitions
    }

    /// Resolve a configured compact term to its absolute vocabulary IRI.
    pub fn expand(&self, term: &str) -> Option<&str> {
        self.definitions.get(term).map(String::as_str)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOfflineJsonLdContext {
    value: Value,
    definitions: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for OfflineJsonLdContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawOfflineJsonLdContext::deserialize(deserializer)?;
        Self::new(raw.value, raw.definitions).map_err(serde::de::Error::custom)
    }
}

fn validate_context_value(value: &Value) -> Result<(), ProjectionError> {
    match value {
        Value::String(iri) => validate_absolute_iri(iri, "JSON-LD context identity"),
        Value::Object(values) => {
            if values.contains_key("@import") {
                return Err(ProjectionError::configuration(
                    "offline JSON-LD contexts cannot use @import",
                ));
            }
            if values.is_empty() {
                return Err(ProjectionError::configuration(
                    "JSON-LD context object cannot be empty",
                ));
            }
            Ok(())
        }
        Value::Array(values) if !values.is_empty() => {
            for value in values {
                validate_context_value(value)?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
            Err(ProjectionError::configuration(
                "JSON-LD context must be an IRI, a non-empty object, or a non-empty array",
            ))
        }
    }
}

/// Native-profile projection result before USTAR encoding.
#[derive(Debug, Clone)]
pub struct ResearchObjectPackageProjection {
    /// Canonical profile artifact package.
    pub package: ProjectionPackage,
    /// Normalized semantic pivot that was encoded.
    pub model: ResearchObjectModel,
    /// Located RDF-to-profile losses.
    pub loss_ledger: LossLedger,
}

/// Native-profile reader result after caller-vocabulary RDF lift.
#[derive(Debug, Clone)]
pub struct ResearchObjectReadOutcome {
    /// Lifted and JSON-LD-normalized RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Normalized semantic pivot interpreted from the native document.
    pub model: ResearchObjectModel,
    /// Located profile-to-RDF losses.
    pub loss_ledger: LossLedger,
}

pub(super) fn canonical_json(
    value: &Value,
    limits: ProjectionLimits,
    description: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let mut bytes = super::super::util::canonical_json_bounded(value, limits, description)?;
    if bytes.len() == limits.max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} plus its canonical newline exceeds the {}-byte artifact limit",
            limits.max_artifact_bytes()
        )));
    }
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn parse_strict_json(
    bytes: &[u8],
    config: &ResearchObjectConfig,
    description: &str,
    path: &str,
) -> Result<Value, ProjectionError> {
    if bytes.len() > config.limits().max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} exceeds the per-artifact byte limit"
        ))
        .at_path(path));
    }
    let remaining = Cell::new(config.policy().max_records());
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let seed = StrictJsonSeed {
        remaining: &remaining,
        depth: 0,
        max_depth: config.policy().max_json_depth(),
    };
    let value = seed.deserialize(&mut deserializer).map_err(|error| {
        ProjectionError::syntax(format!("parse {description}: {error}")).at_path(path)
    })?;
    deserializer.end().map_err(|error| {
        ProjectionError::syntax(format!("parse {description}: {error}")).at_path(path)
    })?;
    Ok(value)
}

#[derive(Clone, Copy)]
struct StrictJsonSeed<'a> {
    remaining: &'a Cell<usize>,
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictJsonSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(D::Error::custom(format!(
                "JSON nesting exceeds depth limit {}",
                self.max_depth
            )));
        }
        let remaining = self.remaining.get();
        if remaining == 0 {
            return Err(D::Error::custom(
                "JSON value count exceeds configured limit",
            ));
        }
        self.remaining.set(remaining - 1);
        deserializer.deserialize_any(StrictJsonVisitor(self))
    }
}

struct StrictJsonVisitor<'a>(StrictJsonSeed<'a>);

impl<'de> Visitor<'de> for StrictJsonVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.0.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1_024));
        while let Some(value) = sequence.next_element_seed(child)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object member `{key}`"
                )));
            }
            values.insert(key, map.next_value_seed(child)?);
        }
        Ok(Value::Object(values))
    }
}

pub(super) fn require_artifact<'a>(
    package: &'a ProjectionPackage,
    path: &str,
    config: &ResearchObjectConfig,
) -> Result<&'a [u8], ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    if package.len() != 1 {
        return Err(ProjectionError::package(format!(
            "research-object package must contain exactly `{path}`"
        )));
    }
    package
        .get(path)
        .ok_or_else(|| ProjectionError::package("required artifact is missing").at_path(path))
}

fn validate_package_bounds(
    package: &ProjectionPackage,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if package.len() > limits.max_artifacts()
        || package.total_bytes() > limits.max_total_bytes()
        || package.archive_bytes() > limits.max_archive_bytes()
    {
        return Err(ProjectionError::limit(
            "research-object package exceeds configured limits",
        ));
    }
    for (path, bytes) in package.artifacts() {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "artifact is {} bytes; reader limit is {}",
                bytes.len(),
                limits.max_artifact_bytes()
            ))
            .at_path(path));
        }
    }
    Ok(())
}

pub(super) fn record_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    path: &str,
    subject: &str,
) {
    let template = contract
        .entries()
        .iter()
        .find(|entry| entry.code == code)
        .expect("native research-object loss must exist in closed contract");
    ledger.record(LossEntry {
        code: Cow::Borrowed(code),
        from: template.from.clone(),
        to: template.to.clone(),
        note: template.note.clone(),
        location: Some(Box::new(
            RdfLocation::file(path).with_subject(subject.to_owned()),
        )),
    });
}

pub(super) fn ensure_sound(
    ledger: &LossLedger,
    from: &str,
    to: &str,
) -> Result<(), ProjectionError> {
    check_ledger_sound(ledger, from, to).map_err(ProjectionError::integrity)
}

pub(super) fn normalize_lifted_jsonld(
    dataset: &RdfDataset,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let json = serialize_dataset_to_jsonld(dataset).map_err(|error| {
        ProjectionError::integrity(format!("normalize lifted research-object JSON-LD: {error}"))
    })?;
    parse_jsonld(json.as_bytes()).map_err(|error| {
        ProjectionError::integrity(format!("reparse lifted research-object JSON-LD: {error}"))
    })
}

pub(super) fn json_pointer(parent: &str, member: &str) -> String {
    let escaped = member.replace('~', "~0").replace('/', "~1");
    if parent.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{parent}/{escaped}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_json_rejects_duplicate_members_depth_and_value_exhaustion() {
        let parse = |bytes: &[u8], values: usize, depth: usize| {
            let remaining = Cell::new(values);
            let mut deserializer = serde_json::Deserializer::from_slice(bytes);
            StrictJsonSeed {
                remaining: &remaining,
                depth: 0,
                max_depth: depth,
            }
            .deserialize(&mut deserializer)
        };

        assert!(parse(br#"{"a":1,"a":2}"#, 10, 4).is_err());
        assert!(parse(br"[[[0]]]", 10, 1).is_err());
        assert!(parse(br"[1,2,3]", 3, 4).is_err());
        assert_eq!(
            parse(br#"{"a":[1,true]}"#, 10, 4).expect("value")["a"][0],
            1
        );
    }

    #[test]
    fn offline_context_is_complete_absolute_and_import_free() {
        let valid = OfflineJsonLdContext::new(
            Value::String("https://example.org/context.jsonld".to_owned()),
            BTreeMap::from([("name".to_owned(), "https://example.org/name".to_owned())]),
        )
        .expect("context");
        assert_eq!(valid.expand("name"), Some("https://example.org/name"));

        assert!(
            OfflineJsonLdContext::new(
                serde_json::json!({"@import": "https://example.org/base"}),
                valid.definitions,
            )
            .is_err()
        );
    }
}
