// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validated, deterministic access to a compiled JSON Schema definition catalog.
//!
//! Schema-language projections share this crate-private boundary so parsing,
//! direct-reference closure, and JSON Pointer locations cannot drift between
//! emitters. It deliberately is not a second public schema algebra: the public
//! carrier remains [`CompiledSchema`].

use std::error::Error;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::json_schema::CompiledSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchemaCatalogError {
    message: String,
}

impl SchemaCatalogError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SchemaCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SchemaCatalogError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompiledSchemaCatalog {
    document: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SchemaCatalogLimits {
    pub(crate) input_bytes: usize,
    pub(crate) definitions: usize,
    pub(crate) depth: usize,
    pub(crate) nodes: usize,
    pub(crate) string_bytes: usize,
}

impl CompiledSchemaCatalog {
    pub(crate) fn parse(compiled: &CompiledSchema) -> Result<Self, SchemaCatalogError> {
        Self::parse_inner(compiled, None)
    }

    pub(crate) fn parse_with_limits(
        compiled: &CompiledSchema,
        limits: SchemaCatalogLimits,
    ) -> Result<Self, SchemaCatalogError> {
        Self::parse_inner(compiled, Some(limits))
    }

    fn parse_inner(
        compiled: &CompiledSchema,
        limits: Option<SchemaCatalogLimits>,
    ) -> Result<Self, SchemaCatalogError> {
        if let Some(limits) = limits
            && compiled.schema_json.len() > limits.input_bytes
        {
            return Err(SchemaCatalogError::new(format!(
                "CompiledSchema.schema_json uses {} bytes; limit is {}",
                compiled.schema_json.len(),
                limits.input_bytes
            )));
        }
        let document = if let Some(limits) = limits {
            parse_bounded_document(&compiled.schema_json, limits)?
        } else {
            serde_json::from_str(&compiled.schema_json)
                .map_err(|error| invalid_json_error(&error))?
        };
        let root = document.as_object().ok_or_else(|| {
            SchemaCatalogError::new("CompiledSchema.schema_json root must be a JSON object")
        })?;
        let definitions = root
            .get("$defs")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                SchemaCatalogError::new(
                    "CompiledSchema.schema_json must contain an object-valued `$defs`",
                )
            })?;

        if let Some(limits) = limits {
            validate_document_limits(&document, definitions.len(), limits)?;
        }

        for (key, definition) in definitions {
            validate_schema(definition, definitions, &definition_path(key))?;
        }

        Ok(Self { document })
    }

    pub(crate) fn definitions(&self) -> &Map<String, Value> {
        self.document
            .as_object()
            .and_then(|root| root.get("$defs"))
            .and_then(Value::as_object)
            .expect("a compiled schema catalog always has validated object-valued `$defs`")
    }
}

fn invalid_json_error(error: &serde_json::Error) -> SchemaCatalogError {
    SchemaCatalogError::new(format!(
        "CompiledSchema.schema_json is not valid JSON: {error}"
    ))
}

fn parse_bounded_document(
    input: &str,
    limits: SchemaCatalogLimits,
) -> Result<Value, SchemaCatalogError> {
    let mut state = BoundedParseState {
        limits,
        nodes: 0,
        violation: None,
    };
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let document = match (BoundedValueSeed {
        state: &mut state,
        depth: 0,
    })
    .deserialize(&mut deserializer)
    {
        Ok(document) => document,
        Err(error) => {
            return Err(state
                .violation
                .map_or_else(|| invalid_json_error(&error), SchemaCatalogError::new));
        }
    };
    deserializer
        .end()
        .map_err(|error| invalid_json_error(&error))?;
    Ok(document)
}

struct BoundedParseState {
    limits: SchemaCatalogLimits,
    nodes: usize,
    violation: Option<String>,
}

impl BoundedParseState {
    fn enter_node<E: de::Error>(&mut self, depth: usize) -> Result<(), E> {
        if depth > self.limits.depth {
            return self.reject(format!(
                "CompiledSchema exceeds JSON nesting limit {}",
                self.limits.depth
            ));
        }
        let Some(nodes) = self.nodes.checked_add(1) else {
            return self.reject("CompiledSchema JSON node count exceeds usize".to_owned());
        };
        if nodes > self.limits.nodes {
            return self.reject(format!(
                "CompiledSchema contains more than {} JSON nodes",
                self.limits.nodes
            ));
        }
        self.nodes = nodes;
        Ok(())
    }

    fn check_string<E: de::Error>(&mut self, bytes: usize) -> Result<(), E> {
        if bytes > self.limits.string_bytes {
            return self.reject(format!(
                "CompiledSchema contains a {bytes}-byte string; limit is {}",
                self.limits.string_bytes
            ));
        }
        Ok(())
    }

    fn reject<T, E: de::Error>(&mut self, message: String) -> Result<T, E> {
        self.violation = Some(message.clone());
        Err(E::custom(message))
    }
}

struct BoundedValueSeed<'a> {
    state: &'a mut BoundedParseState,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for BoundedValueSeed<'_> {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        self.state.enter_node::<D::Error>(self.depth)?;
        deserializer.deserialize_any(BoundedValueVisitor {
            state: self.state,
            depth: self.depth,
        })
    }
}

struct BoundedValueVisitor<'a> {
    state: &'a mut BoundedParseState,
    depth: usize,
}

impl<'de> Visitor<'de> for BoundedValueVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value within the configured structural limits")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
        Number::from_i128(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JSON integer is out of range"))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
        Number::from_u128(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JSON integer is out of range"))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(Value::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(BoundedValueSeed {
            state: &mut *self.state,
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key_seed(BoundedStringSeed {
            state: &mut *self.state,
        })? {
            let value = map.next_value_seed(BoundedValueSeed {
                state: &mut *self.state,
                depth: self.depth + 1,
            })?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

struct BoundedStringSeed<'a> {
    state: &'a mut BoundedParseState,
}

impl<'de> DeserializeSeed<'de> for BoundedStringSeed<'_> {
    type Value = String;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_string(BoundedStringVisitor { state: self.state })
    }
}

struct BoundedStringVisitor<'a> {
    state: &'a mut BoundedParseState,
}

impl<'de> Visitor<'de> for BoundedStringVisitor<'_> {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object key within the configured string limit")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(value.to_owned())
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(value.to_owned())
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.state.check_string::<E>(value.len())?;
        Ok(value)
    }
}

fn validate_document_limits(
    document: &Value,
    definitions: usize,
    limits: SchemaCatalogLimits,
) -> Result<(), SchemaCatalogError> {
    if definitions > limits.definitions {
        return Err(SchemaCatalogError::new(format!(
            "CompiledSchema contains {definitions} definitions; limit is {}",
            limits.definitions
        )));
    }

    let mut nodes = 0usize;
    let mut stack = vec![(document, 0usize)];
    while let Some((value, depth)) = stack.pop() {
        if depth > limits.depth {
            return Err(SchemaCatalogError::new(format!(
                "CompiledSchema exceeds JSON nesting limit {}",
                limits.depth
            )));
        }
        nodes = nodes.checked_add(1).ok_or_else(|| {
            SchemaCatalogError::new("CompiledSchema JSON node count exceeds usize")
        })?;
        if nodes > limits.nodes {
            return Err(SchemaCatalogError::new(format!(
                "CompiledSchema contains more than {} JSON nodes",
                limits.nodes
            )));
        }
        match value {
            Value::String(text) if text.len() > limits.string_bytes => {
                return Err(SchemaCatalogError::new(format!(
                    "CompiledSchema contains a {}-byte string; limit is {}",
                    text.len(),
                    limits.string_bytes
                )));
            }
            Value::Array(values) => {
                stack.extend(values.iter().rev().map(|child| (child, depth + 1)));
            }
            Value::Object(object) => {
                for (key, child) in object.iter().rev() {
                    if key.len() > limits.string_bytes {
                        return Err(SchemaCatalogError::new(format!(
                            "CompiledSchema contains a {}-byte object key; limit is {}",
                            key.len(),
                            limits.string_bytes
                        )));
                    }
                    stack.push((child, depth + 1));
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    Ok(())
}

pub(crate) fn definition_path(key: &str) -> String {
    format!("#/$defs/{}", pointer_escape(key))
}

pub(crate) fn reference_key(reference: &str) -> Option<String> {
    let encoded = reference.strip_prefix("#/$defs/")?;
    if encoded.contains('/') {
        return None;
    }
    pointer_unescape(encoded)
}

pub(crate) fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

pub(crate) const fn schema_map_keywords() -> &'static [&'static str] {
    &[
        "$defs",
        "properties",
        "patternProperties",
        "dependentSchemas",
    ]
}

pub(crate) const fn schema_array_keywords() -> &'static [&'static str] {
    &["allOf", "anyOf", "oneOf", "prefixItems"]
}

pub(crate) const fn schema_single_keywords() -> &'static [&'static str] {
    &[
        "items",
        "additionalItems",
        "unevaluatedItems",
        "additionalProperties",
        "unevaluatedProperties",
        "propertyNames",
        "contains",
        "not",
        "if",
        "then",
        "else",
        "contentSchema",
    ]
}

fn validate_schema(
    value: &Value,
    definitions: &Map<String, Value>,
    path: &str,
) -> Result<(), SchemaCatalogError> {
    let Value::Object(object) = value else {
        return if value.is_boolean() {
            Ok(())
        } else {
            Err(SchemaCatalogError::new(format!(
                "{path} must be an object or boolean JSON Schema"
            )))
        };
    };

    for keyword in ["$dynamicRef", "$recursiveRef"] {
        if object.contains_key(keyword) {
            return Err(SchemaCatalogError::new(format!(
                "{path}/{keyword} cannot be translated to a closed generated package"
            )));
        }
    }

    if object.contains_key("$id") {
        return Err(SchemaCatalogError::new(format!(
            "{path}/$id cannot rebase a closed generated package"
        )));
    }

    if let Some(reference) = object.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| SchemaCatalogError::new(format!("{path}/$ref must be a string")))?;
        let key = reference_key(reference).ok_or_else(|| {
            SchemaCatalogError::new(format!(
                "{path}/$ref is external or not a direct #/$defs reference: {reference:?}"
            ))
        })?;
        if !definitions.contains_key(&key) {
            return Err(SchemaCatalogError::new(format!(
                "{path}/$ref targets missing $defs key {key:?}"
            )));
        }
    }

    for keyword in schema_map_keywords() {
        if let Some(children) = object.get(*keyword) {
            let children = children.as_object().ok_or_else(|| {
                SchemaCatalogError::new(format!("{path}/{keyword} must be an object"))
            })?;
            for (key, child) in children {
                validate_schema(
                    child,
                    definitions,
                    &format!("{path}/{keyword}/{}", pointer_escape(key)),
                )?;
            }
        }
    }
    for keyword in schema_array_keywords() {
        if let Some(children) = object.get(*keyword) {
            let children = children.as_array().ok_or_else(|| {
                SchemaCatalogError::new(format!("{path}/{keyword} must be an array"))
            })?;
            for (index, child) in children.iter().enumerate() {
                validate_schema(child, definitions, &format!("{path}/{keyword}/{index}"))?;
            }
        }
    }
    for keyword in schema_single_keywords() {
        if let Some(child) = object.get(*keyword) {
            validate_schema(child, definitions, &format!("{path}/{keyword}"))?;
        }
    }
    Ok(())
}

fn pointer_unescape(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            output.push(character);
            continue;
        }
        match characters.next()? {
            '0' => output.push('~'),
            '1' => output.push('/'),
            _ => return None,
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::purrdf::loss::LossLedger;
    use serde_json::json;

    fn compiled(schema: &Value) -> CompiledSchema {
        CompiledSchema {
            schema_json: serde_json::to_string(schema).expect("fixture serializes"),
            openapi_json: "{}\n".to_owned(),
            losses: LossLedger::new(),
        }
    }

    #[test]
    fn catalog_exposes_sorted_validated_definitions() {
        let schema = json!({
            "$defs": {
                "Zulu": true,
                "Alpha": {
                    "properties": {
                        "ex:target": { "$ref": "#/$defs/path~1with~0token" },
                        "$ref": { "type": "string" },
                        "ex:data": { "enum": [{ "$ref": "ordinary data" }] }
                    }
                },
                "path/with~token": false
            }
        });
        let catalog = CompiledSchemaCatalog::parse(&compiled(&schema)).expect("valid catalog");
        assert_eq!(
            catalog
                .definitions()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["Alpha", "Zulu", "path/with~token"]
        );
        assert_eq!(
            reference_key("#/$defs/path~1with~0token").as_deref(),
            Some("path/with~token")
        );
        assert_eq!(
            definition_path("path/with~token"),
            "#/$defs/path~1with~0token"
        );
    }

    #[test]
    fn catalog_rejects_malformed_documents_and_schema_values() {
        for (schema_json, expected) in [
            ("not json", "is not valid JSON"),
            ("[]", "root must be a JSON object"),
            ("{}", "must contain an object-valued `$defs`"),
            (r#"{"$defs":[]}"#, "must contain an object-valued `$defs`"),
            (
                r#"{"$defs":{"Broken":7}}"#,
                "#/$defs/Broken must be an object or boolean JSON Schema",
            ),
        ] {
            let error = CompiledSchemaCatalog::parse(&CompiledSchema {
                schema_json: schema_json.to_owned(),
                openapi_json: "{}\n".to_owned(),
                losses: LossLedger::new(),
            })
            .expect_err("fixture must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn catalog_rejects_open_or_malformed_references_at_their_locations() {
        for (reference, expected) in [
            (json!("#/$defs/Missing"), "targets missing $defs key"),
            (
                json!("https://example.org/schema"),
                "is external or not a direct",
            ),
            (json!("#/$defs/path/child"), "is external or not a direct"),
            (json!("#/$defs/bad~2escape"), "is external or not a direct"),
            (json!(7), "must be a string"),
        ] {
            let schema = json!({
                "$defs": {
                    "Holder": {
                        "properties": { "ex:value": { "$ref": reference } }
                    }
                }
            });
            let error =
                CompiledSchemaCatalog::parse(&compiled(&schema)).expect_err("reference must fail");
            assert!(
                error
                    .to_string()
                    .contains("#/$defs/Holder/properties/ex:value/$ref"),
                "{error}"
            );
            assert!(error.to_string().contains(expected), "{error}");
        }

        for keyword in ["$dynamicRef", "$recursiveRef"] {
            let schema = json!({ "$defs": { "Holder": { (keyword): "#/$defs/Holder" } } });
            let error = CompiledSchemaCatalog::parse(&compiled(&schema))
                .expect_err("open reference must fail");
            assert!(error.to_string().contains(keyword), "{error}");
        }

        for schema in [
            json!({
                "$defs": {
                    "Holder": {
                        "$id": "nested.json",
                        "$ref": "#/$defs/Target"
                    },
                    "Target": true
                }
            }),
            json!({
                "$defs": {
                    "Holder": {
                        "properties": {
                            "nested": {
                                "$id": "nested.json",
                                "$ref": "#/$defs/Target"
                            }
                        }
                    },
                    "Target": true
                }
            }),
        ] {
            let error = CompiledSchemaCatalog::parse(&compiled(&schema))
                .expect_err("resource rebasing must fail");
            assert!(
                error
                    .to_string()
                    .contains("$id cannot rebase a closed generated package"),
                "{error}"
            );
        }
    }

    #[test]
    fn bounded_catalog_accepts_limits_and_rejects_each_one_over() {
        let fixture = compiled(&json!({ "$defs": { "A": { "description": "xy" } } }));
        let accepted = SchemaCatalogLimits {
            input_bytes: fixture.schema_json.len(),
            definitions: 1,
            depth: 3,
            nodes: 4,
            string_bytes: 11,
        };
        CompiledSchemaCatalog::parse_with_limits(&fixture, accepted).expect("exact boundaries");

        for limits in [
            SchemaCatalogLimits {
                input_bytes: fixture.schema_json.len() - 1,
                ..accepted
            },
            SchemaCatalogLimits {
                definitions: 0,
                ..accepted
            },
            SchemaCatalogLimits {
                depth: 2,
                ..accepted
            },
            SchemaCatalogLimits {
                nodes: 3,
                ..accepted
            },
            SchemaCatalogLimits {
                string_bytes: 10,
                ..accepted
            },
        ] {
            CompiledSchemaCatalog::parse_with_limits(&fixture, limits)
                .expect_err("one-over limit must fail");
        }
    }

    #[test]
    fn bounded_parser_rejects_structural_limits_before_malformed_tail() {
        let cases = [
            (
                r#"{"$defs":{"A":{"description":"xy"}} trailing"#,
                SchemaCatalogLimits {
                    input_bytes: 1_024,
                    definitions: 10,
                    depth: 10,
                    nodes: 3,
                    string_bytes: 100,
                },
                "more than 3 JSON nodes",
            ),
            (
                r#"{"$defs":{"longer" trailing"#,
                SchemaCatalogLimits {
                    input_bytes: 1_024,
                    definitions: 10,
                    depth: 10,
                    nodes: 10,
                    string_bytes: 5,
                },
                "6-byte string; limit is 5",
            ),
            (
                r#"{"$defs":{"A":{"properties":{"x":{ trailing"#,
                SchemaCatalogLimits {
                    input_bytes: 1_024,
                    definitions: 10,
                    depth: 3,
                    nodes: 10,
                    string_bytes: 100,
                },
                "JSON nesting limit 3",
            ),
        ];
        for (schema_json, limits, expected) in cases {
            let error = CompiledSchemaCatalog::parse_with_limits(
                &CompiledSchema {
                    schema_json: schema_json.to_owned(),
                    openapi_json: "{}\n".to_owned(),
                    losses: LossLedger::new(),
                },
                limits,
            )
            .expect_err("construction limit must fire before the malformed suffix");
            assert!(error.to_string().contains(expected), "{error}");
            assert!(!error.to_string().contains("not valid JSON"), "{error}");
        }
    }

    #[test]
    fn bounded_parser_matches_serde_json_value_semantics() {
        for source in [
            "null",
            "true",
            "-9223372036854775808",
            "18446744073709551615",
            "1.25e-7",
            r#""escaped\nvalue""#,
            r#"[null,true,-1,2.5,"x"]"#,
            r#"{"a":1,"a":2,"nested":{"items":[false,"z"]}}"#,
        ] {
            let expected = serde_json::from_str::<Value>(source).expect("serde JSON fixture");
            let actual = parse_bounded_document(
                source,
                SchemaCatalogLimits {
                    input_bytes: source.len(),
                    definitions: 100,
                    depth: 10,
                    nodes: 100,
                    string_bytes: 100,
                },
            )
            .expect("bounded parser accepts the same JSON value");
            assert_eq!(actual, expected, "{source}");
        }
    }
}
