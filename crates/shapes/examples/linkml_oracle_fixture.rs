// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit deterministic exact and lossy packages for the dev-only LinkML oracle.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use purrdf::loss::{LossLedger, check_ledger_sound};
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{
    ImportedShapes, LinkmlConfig, LinkmlPackage, Namespaces, SchemaDatatypeMap, SchemaImportConfig,
    emit_linkml, import_linkml_package, parse_linkml, write_linkml,
};
use serde_json::{Value, json};

fn compiled(schema: &Value) -> Result<CompiledSchema, serde_json::Error> {
    Ok(CompiledSchema {
        schema_json: format!("{}\n", serde_json::to_string_pretty(schema)?),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    })
}

fn config() -> Result<LinkmlConfig, Box<dyn Error>> {
    Ok(LinkmlConfig::new(
        "https://example.org/schema/linkml",
        "Example-Schema",
        "Caller-owned LinkML differential oracle fixture.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/".to_owned()),
            ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
        ]),
    )?)
}

fn import_config() -> Result<SchemaImportConfig, Box<dyn Error>> {
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )?;
    let xsd = "http://www.w3.org/2001/XMLSchema#";
    let datatypes = SchemaDatatypeMap::new(
        format!("{xsd}string"),
        format!("{xsd}boolean"),
        format!("{xsd}integer"),
        format!("{xsd}decimal"),
        format!("{xsd}dateTime"),
        format!("{xsd}date"),
        format!("{xsd}time"),
        format!("{xsd}anyURI"),
    )?;
    Ok(SchemaImportConfig::new(namespaces, datatypes))
}

fn reverse_payload(
    package: &LinkmlPackage,
    config: &SchemaImportConfig,
) -> Result<Value, Box<dyn Error>> {
    let imported = import_linkml_package(package, config)?;
    check_ledger_sound(&imported.losses, "linkml-1.11", "shacl")?;
    let repeated = import_linkml_package(package, config)?;
    if imported.losses.render_json() != repeated.losses.render_json() {
        return Err("LinkML reverse ledger is not deterministic".into());
    }

    let compile_imported = |value: &ImportedShapes| {
        purrdf_shapes::json_schema::compile(&value.shapes, config.namespaces())
    };
    let compiled = compile_imported(&imported);
    let repeated_compiled = compile_imported(&repeated);
    if compiled.schema_json != repeated_compiled.schema_json {
        return Err("LinkML reverse shapes are not byte-deterministic".into());
    }

    let shape_ids = imported
        .shapes
        .node_shapes
        .iter()
        .map(|shape| shape.id.to_string())
        .collect::<Vec<_>>();
    Ok(json!({
        "losses": serde_json::from_str::<Value>(&imported.losses.render_json())?,
        "schema": serde_json::from_str::<Value>(&compiled.schema_json)?,
        "shape_ids": shape_ids,
    }))
}

fn exact_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.org/schema/exact.json",
        "$defs": {
            "Address": {
                "type": "object",
                "title": "Address",
                "description": "A caller-described address.",
                "additionalProperties": false,
                "properties": {
                    "ex:city": { "type": "string", "pattern": "^[A-Z]" },
                    "ex:postalCode": { "type": "string", "pattern": "^[A-Z][0-9]$" }
                },
                "required": ["ex:city"]
            },
            "Color": {
                "type": "string",
                "title": "Color",
                "description": "One allowed color identifier.",
                "enum": ["ex:blue", "ex:red"]
            },
            "Person": {
                "type": "object",
                "title": "Person",
                "description": "A caller-described person.",
                "additionalProperties": false,
                "properties": {
                    "@id": { "type": "string" },
                    "ex:active": { "type": "boolean" },
                    "ex:address": { "$ref": "#/$defs/Address" },
                    "ex:age": { "type": "integer", "minimum": 0, "maximum": 130 },
                    "ex:color": { "$ref": "#/$defs/Color" },
                    "ex:name": { "type": "string", "pattern": "^[A-Z]" },
                    "ex:score": { "type": "number", "minimum": 0, "maximum": 1 },
                    "ex:tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "maxItems": 3
                    },
                    "ex:value": {
                        "anyOf": [
                            { "type": "string" },
                            { "type": "integer" }
                        ]
                    }
                },
                "required": ["ex:age", "ex:name"]
            }
        }
    })
}

fn lossy_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Lossy": {
                "type": "object",
                "title": "Lossy",
                "description": "A fixture exercising the closed LinkML loss profile.",
                "additionalProperties": { "type": "integer" },
                "minProperties": 1,
                "maxProperties": 8,
                "dependentRequired": { "ex:a": ["ex:b"] },
                "if": { "properties": { "ex:a": { "const": true } } },
                "then": { "required": ["ex:b"], "properties": { "ex:b": true } },
                "unevaluatedProperties": false,
                "propertyNames": { "pattern": "^ex:" },
                "properties": {
                    "ex:array": {
                        "type": "array",
                        "prefixItems": [
                            { "type": "string" },
                            { "type": "integer" }
                        ],
                        "contains": { "const": 7 },
                        "minContains": 1,
                        "unevaluatedItems": false
                    },
                    "ex:choice": {
                        "enum": [
                            { "@id": "ex:open" },
                            { "@id": "ex:closed" }
                        ]
                    },
                    "ex:label": {
                        "type": "string",
                        "minLength": 2,
                        "maxLength": 12,
                        "format": "email"
                    },
                    "ex:number": {
                        "type": "number",
                        "exclusiveMinimum": 0,
                        "exclusiveMaximum": 10,
                        "multipleOf": 0.5
                    }
                }
            }
        }
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = config()?;
    let import_config = import_config()?;
    let exact_schema = exact_schema();
    let lossy_schema = lossy_schema();
    let exact = emit_linkml(&compiled(&exact_schema)?, &config)?;
    let lossy = emit_linkml(&compiled(&lossy_schema)?, &config)?;

    if !exact.losses.is_empty() {
        return Err(format!(
            "exact LinkML fixture unexpectedly lost semantics: {}",
            exact.losses.render_json()
        )
        .into());
    }
    check_ledger_sound(&lossy.losses, "json-schema", "linkml-1.11")?;
    let observed_losses = lossy
        .losses
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.code.as_ref(),
                entry
                    .location
                    .as_ref()
                    .and_then(|location| location.subject.as_deref())
                    .unwrap_or("<missing>"),
            )
        })
        .collect::<BTreeSet<_>>();
    let expected_losses = BTreeSet::from([
        (
            "array-contains-validation-dropped",
            "#/$defs/Lossy/properties/ex:array/contains",
        ),
        ("conditional-validation-dropped", "#/$defs/Lossy/if"),
        (
            "dependency-validation-dropped",
            "#/$defs/Lossy/dependentRequired",
        ),
        (
            "exclusive-bound-validation-widened",
            "#/$defs/Lossy/properties/ex:number/exclusiveMaximum",
        ),
        (
            "exclusive-bound-validation-widened",
            "#/$defs/Lossy/properties/ex:number/exclusiveMinimum",
        ),
        (
            "format-validation-widened",
            "#/$defs/Lossy/properties/ex:label/format",
        ),
        (
            "keyword-validation-dropped",
            "#/$defs/Lossy/if/properties/ex:a/const",
        ),
        ("keyword-validation-dropped", "#/$defs/Lossy/propertyNames"),
        (
            "multiple-of-validation-dropped",
            "#/$defs/Lossy/properties/ex:number/multipleOf",
        ),
        (
            "non-scalar-enum-validation-widened",
            "#/$defs/Lossy/properties/ex:choice/enum/0",
        ),
        (
            "non-scalar-enum-validation-widened",
            "#/$defs/Lossy/properties/ex:choice/enum/1",
        ),
        (
            "property-count-validation-dropped",
            "#/$defs/Lossy/maxProperties",
        ),
        (
            "property-count-validation-dropped",
            "#/$defs/Lossy/minProperties",
        ),
        (
            "string-length-validation-dropped",
            "#/$defs/Lossy/properties/ex:label/maxLength",
        ),
        (
            "string-length-validation-dropped",
            "#/$defs/Lossy/properties/ex:label/minLength",
        ),
        (
            "tuple-array-validation-widened",
            "#/$defs/Lossy/properties/ex:array/prefixItems",
        ),
        (
            "unevaluated-validation-dropped",
            "#/$defs/Lossy/properties/ex:array/unevaluatedItems",
        ),
        (
            "unevaluated-validation-dropped",
            "#/$defs/Lossy/unevaluatedProperties",
        ),
    ]);
    if observed_losses != expected_losses {
        return Err(format!(
            "lossy LinkML fixture contract disagrees: {}",
            lossy.losses.render_json()
        )
        .into());
    }

    for package in [&exact, &lossy] {
        let reparsed = parse_linkml(&package.yaml)?;
        if reparsed != package.document || write_linkml(&reparsed)? != package.yaml {
            return Err("LinkML fixture did not survive canonical read/write".into());
        }
    }

    let output = json!({
        "exact": {
            "element_names": exact.element_names,
            "reverse": reverse_payload(&exact, &import_config)?,
            "schema": exact_schema,
            "yaml": exact.yaml,
        },
        "lossy": {
            "element_names": lossy.element_names,
            "losses": serde_json::from_str::<Value>(&lossy.losses.render_json())?,
            "reverse": reverse_payload(&lossy, &import_config)?,
            "schema": lossy_schema,
            "yaml": lossy.yaml,
        }
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
