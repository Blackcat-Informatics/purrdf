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

use serde_json::{Map, Value};

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

impl CompiledSchemaCatalog {
    pub(crate) fn parse(compiled: &CompiledSchema) -> Result<Self, SchemaCatalogError> {
        let document: Value = serde_json::from_str(&compiled.schema_json).map_err(|error| {
            SchemaCatalogError::new(format!(
                "CompiledSchema.schema_json is not valid JSON: {error}"
            ))
        })?;
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
    }
}
