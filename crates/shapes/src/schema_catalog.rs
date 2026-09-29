// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
        // Bounds remain exact decimals. Both catalog entry points use the same
        // typed-scalar parser, including duplicate rejection and the safety depth cap.
        let document = parse_bounded_document(
            &compiled.schema_json,
            limits.unwrap_or(SchemaCatalogLimits {
                input_bytes: usize::MAX,
                definitions: usize::MAX,
                depth: 128,
                nodes: usize::MAX,
                string_bytes: usize::MAX,
            }),
        )?;
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
            validate_definition_limit(definitions.len(), limits)?;
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
    purrdf::json_value::parse(
        input.as_bytes(),
        purrdf::json_value::Limits {
            bytes: limits.input_bytes,
            values: limits.nodes,
            depth: limits.depth,
            string_bytes: limits.string_bytes,
        },
    )
    .map_err(|error| match error {
        purrdf::json_value::Error::Limit(message) => SchemaCatalogError::new(message),
        purrdf::json_value::Error::Syntax(error) => invalid_json_error(&error),
    })
}

fn validate_definition_limit(
    definitions: usize,
    limits: SchemaCatalogLimits,
) -> Result<(), SchemaCatalogError> {
    if definitions > limits.definitions {
        return Err(SchemaCatalogError::new(format!(
            "CompiledSchema contains {definitions} definitions; limit is {}",
            limits.definitions
        )));
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
    purrdf_iri::json_pointer::unescape_token(encoded).map(std::borrow::Cow::into_owned)
}

/// An RFC 6901 reference token escaped for a `#/$defs/…` pointer.
pub(crate) use purrdf_iri::json_pointer::escape_token as pointer_escape;

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

    /// Both catalog readers, bounded and unbounded, give every schema number the value
    /// its decimal spells: the witnesses are numbers a reader that is not correctly
    /// rounded (or the x87's double-rounded fast path) reads as a neighbour.
    #[test]
    fn catalog_numbers_are_correctly_rounded() {
        use purrdf_xsd::ieee::reference as soft;

        let mut lexicals = soft::misread_decimals(30);
        lexicals.extend(soft::x87_fast_path_decimals(30));
        let properties = lexicals
            .iter()
            .enumerate()
            .map(|(index, lexical)| format!(r#""ex:v{index}":{{"minimum":{lexical}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let fixture = CompiledSchema {
            schema_json: format!(r#"{{"$defs":{{"Probe":{{"properties":{{{properties}}}}}}}}}"#),
            openapi_json: "{}\n".to_owned(),
            losses: LossLedger::new(),
        };
        let limits = SchemaCatalogLimits {
            input_bytes: fixture.schema_json.len(),
            definitions: 1,
            depth: 8,
            nodes: 1_000,
            string_bytes: 1_000,
        };
        for catalog in [
            CompiledSchemaCatalog::parse(&fixture).expect("unbounded"),
            CompiledSchemaCatalog::parse_with_limits(&fixture, limits).expect("bounded"),
        ] {
            let properties = &catalog.definitions()["Probe"]["properties"];
            for (index, lexical) in lexicals.iter().enumerate() {
                let correct: f64 = lexical.parse().expect("decimal");
                assert_eq!(
                    properties[format!("ex:v{index}")]["minimum"]
                        .as_f64()
                        .map(f64::to_bits),
                    Some(correct.to_bits()),
                    "{lexical}"
                );
            }
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
    fn bounded_parser_rejects_decoded_duplicate_keys() {
        let limits = SchemaCatalogLimits {
            input_bytes: 100,
            definitions: 100,
            depth: 10,
            nodes: 100,
            string_bytes: 100,
        };
        assert!(parse_bounded_document(r#"{"a":1,"\u0061":2}"#, limits).is_err());
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
            r#"{"a":2,"nested":{"items":[false,"z"]}}"#,
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
