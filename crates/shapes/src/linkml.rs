// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical LinkML 1.11 document boundary and schema-emitter types.
//!
//! The reader accepts the fixed PurRDF LinkML 1.11 dialect into a
//! JSON-compatible value tree. It rejects YAML-only semantics that cannot
//! survive a language-neutral round trip: duplicate keys, tags, non-string
//! mapping keys, and non-finite numbers. The writer emits one sorted,
//! byte-stable YAML representation while preserving fields the emitter does
//! not author.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use ::purrdf::loss::LossLedger;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use serde_yaml::Value as YamlValue;

use crate::json_schema::CompiledSchema;

mod projection;

/// The exact LinkML metamodel version carried by this codec.
pub const LINKML_METAMODEL_VERSION: &str = "1.11.0";

const LINKML_PREFIX: &str = "linkml";
const MAX_LINKML_YAML_BYTES: usize = 16 * 1024 * 1024;
const MAX_LINKML_YAML_DEPTH: usize = 256;
const MAX_LINKML_YAML_NODES: usize = 1_000_000;

/// Caller-owned identity and vocabulary configuration for LinkML emission.
///
/// There is intentionally no Default implementation. The caller must supply
/// the schema IRI, schema name, prose, default vocabulary prefix, and every
/// prefix mapping. The reserved linkml prefix must also be supplied explicitly
/// because the emitted schema imports linkml:types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlConfig {
    schema_id: String,
    schema_name: String,
    description: String,
    default_prefix: String,
    prefixes: BTreeMap<String, String>,
}

impl LinkmlConfig {
    /// Validate and construct LinkML emitter configuration.
    ///
    /// # Errors
    ///
    /// Returns LinkmlError when an identity or prefix is missing, malformed,
    /// relative, or conflicts with the reserved linkml prefix.
    pub fn new(
        schema_id: impl Into<String>,
        schema_name: impl Into<String>,
        description: impl Into<String>,
        default_prefix: impl Into<String>,
        prefixes: BTreeMap<String, String>,
    ) -> Result<Self, LinkmlError> {
        let schema_id = schema_id.into();
        let schema_name = schema_name.into();
        let description = description.into();
        let default_prefix = default_prefix.into();

        validate_absolute_iri("LinkML schema id", &schema_id)?;
        validate_identifier("LinkML schema name", &schema_name)?;
        if description.trim().is_empty() {
            return Err(LinkmlError::new(
                "LinkML schema description must be caller-supplied non-whitespace text",
            ));
        }
        validate_identifier("LinkML default prefix", &default_prefix)?;
        validate_prefixes(&prefixes)?;
        if !prefixes.contains_key(&default_prefix) {
            return Err(LinkmlError::new(format!(
                "LinkML default prefix {default_prefix:?} is absent from the caller prefix map"
            )));
        }
        if default_prefix == LINKML_PREFIX {
            return Err(LinkmlError::new(
                "LinkML default prefix cannot reuse the reserved linkml metamodel prefix",
            ));
        }
        if !prefixes.contains_key(LINKML_PREFIX) {
            return Err(LinkmlError::new(
                "LinkML prefix map must caller-supply the reserved linkml namespace",
            ));
        }

        Ok(Self {
            schema_id,
            schema_name,
            description,
            default_prefix,
            prefixes,
        })
    }

    /// Caller-supplied absolute schema IRI.
    #[must_use]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// Caller-supplied LinkML schema name.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    /// Caller-supplied schema description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Prefix used to derive unqualified element URIs.
    #[must_use]
    pub fn default_prefix(&self) -> &str {
        &self.default_prefix
    }

    /// Ordered caller-supplied prefix map.
    #[must_use]
    pub fn prefixes(&self) -> &BTreeMap<String, String> {
        &self.prefixes
    }
}

/// One validated LinkML 1.11 document, including unknown metamodel fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlDocument {
    value: Value,
}

impl LinkmlDocument {
    /// Validate a JSON-compatible LinkML 1.11 value tree.
    ///
    /// # Errors
    ///
    /// Returns LinkmlError when the fixed dialect envelope is malformed.
    pub fn from_value(value: Value) -> Result<Self, LinkmlError> {
        validate_document(&value)?;
        Ok(Self { value })
    }

    /// Borrow the complete JSON-compatible document tree.
    #[must_use]
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// Consume this wrapper and return the complete document tree.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }
}

impl TryFrom<Value> for LinkmlDocument {
    type Error = LinkmlError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Self::from_value(value)
    }
}

/// Deterministic emitted LinkML document, bytes, element map, and losses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlPackage {
    /// Validated semantic document.
    pub document: LinkmlDocument,
    /// Canonical LinkML YAML with exactly one trailing newline.
    pub yaml: String,
    /// Source definition key to emitted LinkML element name, sorted by key.
    pub element_names: BTreeMap<String, String>,
    /// JSON Schema assertions not represented exactly in LinkML 1.11.
    pub losses: LossLedger,
}

/// A malformed LinkML configuration, document, or projection input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlError {
    detail: String,
}

impl LinkmlError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Stable human-readable error detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for LinkmlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for LinkmlError {}

/// Parse a LinkML 1.11 YAML document without accepting lossy YAML semantics.
///
/// # Errors
///
/// Returns LinkmlError for invalid YAML, duplicate keys, tags, non-string
/// mapping keys, non-finite numbers, resource-limit violations, or an invalid
/// fixed-dialect envelope.
pub fn parse_linkml(input: &str) -> Result<LinkmlDocument, LinkmlError> {
    if input.len() > MAX_LINKML_YAML_BYTES {
        return Err(LinkmlError::new(format!(
            "LinkML YAML exceeds the {MAX_LINKML_YAML_BYTES}-byte input limit"
        )));
    }

    let yaml_value: YamlValue = serde_yaml::from_str(input)
        .map_err(|error| LinkmlError::new(format!("invalid LinkML YAML: {error}")))?;
    let mut nodes = 0;
    validate_yaml_value(&yaml_value, 0, &mut nodes, "$")?;

    let strict_value: StrictValue = serde_yaml::from_str(input)
        .map_err(|error| LinkmlError::new(format!("invalid LinkML YAML: {error}")))?;
    LinkmlDocument::from_value(strict_value.into_json())
}

/// Serialize one validated LinkML document to canonical sorted block YAML.
///
/// # Errors
///
/// Returns LinkmlError when the document envelope is invalid or YAML
/// serialization fails.
pub fn write_linkml(document: &LinkmlDocument) -> Result<String, LinkmlError> {
    validate_document(document.as_value())?;
    let serialized = serde_yaml::to_string(document.as_value())
        .map_err(|error| LinkmlError::new(format!("cannot serialize LinkML YAML: {error}")))?;
    let mut canonical = serialized.trim_end_matches('\n').to_owned();
    canonical.push('\n');
    Ok(canonical)
}

/// Project one compiled SHACL-derived JSON Schema to deterministic LinkML 1.11.
///
/// Source-stage losses remain on [`CompiledSchema::losses`]. The returned
/// ledger covers only this projection step, `json-schema` → `linkml-1.11`.
/// Every emitted identity and vocabulary IRI comes from [`LinkmlConfig`].
///
/// # Errors
///
/// Returns [`LinkmlError`] when the compiled schema is malformed, a reference
/// is external or dangling, a required-property declaration is inconsistent,
/// or source names collide after deterministic LinkML normalization.
pub fn emit_linkml(
    compiled: &CompiledSchema,
    config: &LinkmlConfig,
) -> Result<LinkmlPackage, LinkmlError> {
    projection::emit(compiled, config)
}

fn validate_document(value: &Value) -> Result<(), LinkmlError> {
    let root = value
        .as_object()
        .ok_or_else(|| LinkmlError::new("LinkML document root must be a mapping"))?;

    let schema_id = required_string(root, "id")?;
    validate_absolute_iri("LinkML document id", schema_id)?;
    validate_identifier("LinkML document name", required_string(root, "name")?)?;

    let metamodel_version = required_string(root, "metamodel_version")?;
    if metamodel_version != LINKML_METAMODEL_VERSION {
        return Err(LinkmlError::new(format!(
            "LinkML metamodel_version must be {LINKML_METAMODEL_VERSION:?}, got {metamodel_version:?}"
        )));
    }

    if let Some(description) = root.get("description") {
        let description = description
            .as_str()
            .ok_or_else(|| LinkmlError::new("LinkML description must be a string"))?;
        if description.trim().is_empty() {
            return Err(LinkmlError::new(
                "LinkML description must contain non-whitespace text when present",
            ));
        }
    }

    let prefixes = root
        .get("prefixes")
        .and_then(Value::as_object)
        .ok_or_else(|| LinkmlError::new("LinkML prefixes must be a mapping"))?;
    validate_document_prefixes(prefixes)?;

    let default_prefix = required_string(root, "default_prefix")?;
    validate_identifier("LinkML document default_prefix", default_prefix)?;
    if !prefixes.contains_key(default_prefix) {
        return Err(LinkmlError::new(format!(
            "LinkML default_prefix {default_prefix:?} is absent from prefixes"
        )));
    }
    if default_prefix == LINKML_PREFIX {
        return Err(LinkmlError::new(
            "LinkML default_prefix cannot reuse the reserved linkml metamodel prefix",
        ));
    }
    if !prefixes.contains_key(LINKML_PREFIX) {
        return Err(LinkmlError::new(
            "LinkML prefixes must include the caller-supplied linkml namespace",
        ));
    }

    for section in ["classes", "enums", "slots", "types"] {
        if root.get(section).is_some_and(|value| !value.is_object()) {
            return Err(LinkmlError::new(format!(
                "LinkML {section} must be a mapping when present"
            )));
        }
    }
    if let Some(imports) = root.get("imports") {
        match imports {
            Value::String(_) => {}
            Value::Array(values) if values.iter().all(Value::is_string) => {}
            _ => {
                return Err(LinkmlError::new(
                    "LinkML imports must be a string or an array of strings",
                ));
            }
        }
    }

    Ok(())
}

fn validate_prefixes(prefixes: &BTreeMap<String, String>) -> Result<(), LinkmlError> {
    if prefixes.is_empty() {
        return Err(LinkmlError::new("LinkML caller prefix map cannot be empty"));
    }
    for (prefix, namespace) in prefixes {
        validate_identifier("LinkML prefix", prefix)?;
        validate_absolute_iri(&format!("LinkML prefix {prefix:?} namespace"), namespace)?;
    }
    Ok(())
}

fn validate_document_prefixes(prefixes: &Map<String, Value>) -> Result<(), LinkmlError> {
    if prefixes.is_empty() {
        return Err(LinkmlError::new("LinkML prefixes cannot be empty"));
    }
    for (prefix, definition) in prefixes {
        validate_identifier("LinkML prefix", prefix)?;
        match definition {
            Value::String(namespace) => {
                validate_absolute_iri(&format!("LinkML prefix {prefix:?} namespace"), namespace)?;
            }
            Value::Object(object) => {
                if let Some(declared_prefix) = object.get("prefix_prefix") {
                    let declared_prefix = declared_prefix.as_str().ok_or_else(|| {
                        LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} prefix_prefix must be a string"
                        ))
                    })?;
                    if declared_prefix != prefix {
                        return Err(LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} conflicts with prefix_prefix {declared_prefix:?}"
                        )));
                    }
                }
                let namespace = object
                    .get("prefix_reference")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} object requires string prefix_reference"
                        ))
                    })?;
                validate_absolute_iri(
                    &format!("LinkML prefix {prefix:?} prefix_reference"),
                    namespace,
                )?;
            }
            _ => {
                return Err(LinkmlError::new(format!(
                    "LinkML prefix {prefix:?} must be a namespace string or prefix object"
                )));
            }
        }
    }
    Ok(())
}

fn required_string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, LinkmlError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| LinkmlError::new(format!("LinkML {key} must be a string")))
}

fn validate_absolute_iri(label: &str, value: &str) -> Result<(), LinkmlError> {
    let iri = purrdf_iri::parse(value)
        .map_err(|error| LinkmlError::new(format!("{label} {value:?} is invalid: {error}")))?;
    if !iri.has_scheme() {
        return Err(LinkmlError::new(format!(
            "{label} {value:?} must be absolute"
        )));
    }
    Ok(())
}

fn validate_identifier(label: &str, value: &str) -> Result<(), LinkmlError> {
    if !is_linkml_identifier(value) {
        return Err(LinkmlError::new(format!(
            "{label} {value:?} is not a LinkML NCName"
        )));
    }
    Ok(())
}

fn is_linkml_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_alphabetic())
        && characters.all(|character| {
            character == '_'
                || character == '-'
                || character == '.'
                || character.is_alphanumeric()
                || matches!(
                    character,
                    '\u{300}'..='\u{36f}'
                        | '\u{203f}'..='\u{2040}'
                        | '\u{b7}'
                )
        })
}

fn validate_yaml_value(
    value: &YamlValue,
    depth: usize,
    nodes: &mut usize,
    path: &str,
) -> Result<(), LinkmlError> {
    if depth > MAX_LINKML_YAML_DEPTH {
        return Err(LinkmlError::new(format!(
            "LinkML YAML at {path} exceeds depth {MAX_LINKML_YAML_DEPTH}"
        )));
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| LinkmlError::new("LinkML YAML node count overflow"))?;
    if *nodes > MAX_LINKML_YAML_NODES {
        return Err(LinkmlError::new(format!(
            "LinkML YAML exceeds {MAX_LINKML_YAML_NODES} nodes"
        )));
    }

    match value {
        YamlValue::Sequence(values) => {
            for (index, child) in values.iter().enumerate() {
                validate_yaml_value(child, depth + 1, nodes, &format!("{path}/{index}"))?;
            }
        }
        YamlValue::Mapping(values) => {
            for (key, child) in values {
                let key = key.as_str().ok_or_else(|| {
                    LinkmlError::new(format!(
                        "LinkML YAML mapping at {path} has a non-string key"
                    ))
                })?;
                validate_yaml_value(child, depth + 1, nodes, &format!("{path}/{key}"))?;
            }
        }
        YamlValue::Number(number) => {
            if number.as_i64().is_none()
                && number.as_u64().is_none()
                && !number.as_f64().is_some_and(f64::is_finite)
            {
                return Err(LinkmlError::new(format!(
                    "LinkML YAML number at {path} is not finite"
                )));
            }
        }
        YamlValue::Tagged(_) => {
            return Err(LinkmlError::new(format!(
                "LinkML YAML tag at {path} is not JSON-compatible"
            )));
        }
        YamlValue::Null | YamlValue::Bool(_) | YamlValue::String(_) => {}
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StrictValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Sequence(Vec<Self>),
    Mapping(BTreeMap<String, Self>),
}

impl StrictValue {
    fn into_json(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::Sequence(values) => {
                Value::Array(values.into_iter().map(Self::into_json).collect())
            }
            Self::Mapping(values) => Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, value.into_json()))
                    .collect(),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a finite JSON-compatible YAML value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(StrictValue::Number)
            .ok_or_else(|| E::custom("non-finite YAML number is not JSON-compatible"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1_024));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(StrictValue::Sequence(values))
    }

    fn visit_map<A>(self, mut mapping: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = mapping.next_entry::<String, StrictValue>()? {
            if values.insert(key.clone(), value).is_some() {
                return Err(<A::Error as de::Error>::custom(format!(
                    "duplicate YAML mapping key {key:?}"
                )));
            }
        }
        Ok(StrictValue::Mapping(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn prefixes() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/".to_owned()),
            (
                "linkml".to_owned(),
                "https://example.org/linkml/".to_owned(),
            ),
        ])
    }

    fn valid_value() -> Value {
        json!({
            "id": "https://example.org/schema",
            "name": "Example-Schema",
            "description": "Caller schema.",
            "metamodel_version": LINKML_METAMODEL_VERSION,
            "prefixes": {
                "ex": "https://example.org/",
                "linkml": "https://example.org/linkml/"
            },
            "default_prefix": "ex",
            "classes": {}
        })
    }

    #[test]
    fn config_requires_caller_identity_vocabulary_and_docs() {
        let config = LinkmlConfig::new(
            "https://example.org/schema",
            "Example-Schema",
            "Caller schema.",
            "ex",
            prefixes(),
        )
        .expect("valid configuration");
        assert_eq!(config.schema_id(), "https://example.org/schema");
        assert_eq!(config.schema_name(), "Example-Schema");
        assert_eq!(config.description(), "Caller schema.");
        assert_eq!(config.default_prefix(), "ex");
        assert_eq!(config.prefixes(), &prefixes());

        assert!(LinkmlConfig::new("/relative", "Schema", "docs", "ex", prefixes()).is_err());
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "9bad",
                "docs",
                "ex",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                " ",
                "ex",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "missing",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "linkml",
                prefixes()
            )
            .is_err()
        );

        let mut missing_linkml = prefixes();
        missing_linkml.remove("linkml");
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "ex",
                missing_linkml,
            )
            .is_err()
        );
        let bad_namespace = BTreeMap::from([
            ("ex".to_owned(), "relative".to_owned()),
            (
                "linkml".to_owned(),
                "https://example.org/linkml/".to_owned(),
            ),
        ]);
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "ex",
                bad_namespace,
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_codec_preserves_unknown_fields_and_is_byte_stable() {
        let source = r"
x-extension:
  nested:
    - null
    - answer: 42
prefixes:
  linkml:
    prefix_reference: https://example.org/linkml/
    prefix_prefix: linkml
  ex: https://example.org/
name: Example-Schema
metamodel_version: 1.11.0
id: https://example.org/schema
description: Caller schema.
default_prefix: ex
classes:
  Person:
    attributes:
      ex:name:
        range: string
";
        let document = parse_linkml(source).expect("parse");
        assert_eq!(
            document.as_value()["x-extension"]["nested"][1]["answer"],
            json!(42)
        );

        let first = write_linkml(&document).expect("write");
        let reparsed = parse_linkml(&first).expect("reparse");
        let second = write_linkml(&reparsed).expect("rewrite");
        assert_eq!(reparsed, document);
        assert_eq!(second, first);
        assert!(first.ends_with('\n'));
        assert!(!first.ends_with("\n\n"));
        assert!(first.starts_with("classes:\n"));
    }

    #[test]
    fn document_value_round_trips_through_canonical_yaml() {
        let document = LinkmlDocument::from_value(valid_value()).expect("valid document");
        let yaml = write_linkml(&document).expect("write");
        let reparsed = parse_linkml(&yaml).expect("parse");
        assert_eq!(reparsed, document);
        assert_eq!(reparsed.into_value(), valid_value());
    }

    #[test]
    fn parser_rejects_yaml_semantics_that_are_not_json_compatible() {
        let duplicate = r"
id: https://example.org/schema
name: First
name: Second
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
";
        assert!(
            parse_linkml(duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );

        let tagged = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value: !caller tagged
";
        assert!(
            parse_linkml(tagged)
                .unwrap_err()
                .to_string()
                .contains("tag")
        );

        let non_string_key = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value:
  7: seven
";
        assert!(
            parse_linkml(non_string_key)
                .unwrap_err()
                .to_string()
                .contains("non-string key")
        );

        let non_finite = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value: .nan
";
        assert!(
            parse_linkml(non_finite)
                .unwrap_err()
                .to_string()
                .contains("not finite")
        );
    }

    #[test]
    fn parser_rejects_invalid_fixed_dialect_envelopes() {
        let mut value = valid_value();
        value["metamodel_version"] = json!("1.12.0");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["id"] = json!("/relative");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["name"] = json!("bad:name");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["prefixes"].as_object_mut().unwrap().remove("linkml");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["classes"] = json!([]);
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["imports"] = json!([7]);
        assert!(LinkmlDocument::from_value(value).is_err());
    }

    #[test]
    fn parser_accepts_structured_prefixes_without_erasing_extensions() {
        let mut value = valid_value();
        value["prefixes"]["ex"] = json!({
            "prefix_prefix": "ex",
            "prefix_reference": "https://example.org/",
            "x-prefix-extension": true
        });
        value["x-document-extension"] = json!({"kept": ["yes"]});
        let document = LinkmlDocument::from_value(value.clone()).expect("valid document");
        let reparsed = parse_linkml(&write_linkml(&document).unwrap()).unwrap();
        assert_eq!(reparsed.into_value(), value);
    }
}
