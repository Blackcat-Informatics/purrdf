// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit the deterministic package consumed by the dev-only Pydantic oracle.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use purrdf::loss::{LossLedger, check_ledger_sound};
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces};
use purrdf_shapes::{
    PYDANTIC_DIALECT, PydanticConfig, PydanticPackage, PydanticVersionStamp, SchemaDatatypeMap,
    SchemaImportConfig, emit_pydantic, import_pydantic_package,
};
use serde_json::{Value, json};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn import_config() -> Result<SchemaImportConfig, Box<dyn Error>> {
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )?;
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )?;
    Ok(SchemaImportConfig::new(namespaces, datatypes))
}

fn reverse_evidence(
    package: &PydanticPackage,
    config: &SchemaImportConfig,
) -> Result<Value, Box<dyn Error>> {
    let imported = import_pydantic_package(package, config)?;
    check_ledger_sound(&imported.losses, PYDANTIC_DIALECT, "shacl")?;
    let repeated = import_pydantic_package(package, config)?;
    if imported.losses.render_json() != repeated.losses.render_json() {
        return Err("Pydantic reverse ledger is not deterministic".into());
    }
    let first = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces());
    let second = purrdf_shapes::json_schema::compile(&repeated.shapes, config.namespaces());
    if first.schema_json != second.schema_json {
        return Err("Pydantic reverse shapes are not byte-deterministic".into());
    }
    Ok(json!({
        "losses": serde_json::from_str::<Value>(&imported.losses.render_json())?,
        "shape_ids": imported
            .shapes
            .node_shapes
            .iter()
            .map(|shape| shape.id.to_string())
            .collect::<Vec<_>>(),
    }))
}

fn version_oracle() -> Value {
    let candidates = [
        "0",
        "v1.2",
        "1!2.0",
        "1.0a1",
        "1.0-alpha",
        "1.0beta2",
        "1.0b3",
        "1.0preview2",
        "1.0pre4",
        "1.0c5",
        "1.0RC1",
        "1.0rc_",
        "1.0-1",
        "1.0-post2",
        "1.0post-",
        "1.0_post_7",
        "1.0rev",
        "1.0r8",
        "1.0.dev3",
        "1.0dev-",
        "1.0_dev_9",
        "1.0rc1.post2.dev3",
        "01.002",
        "1.0+abc.1",
        "1.0+Ubuntu-1",
        " \tv1.0RC1+LOCAL_1\n",
        "",
        "v",
        "1..0",
        "1.0+",
        "1.0+abc+def",
        "1.0++x",
        "1.0+abc_",
        "1.0-",
        "1.0_",
        "1.0..post1",
        "1.0a..1",
        "1!",
        "1.0foo",
        "1.0+naïve",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([
        format!("1.{}", "7".repeat(510)),
        format!("1.{}", "7".repeat(511)),
    ]);

    Value::Array(
        candidates
            .into_iter()
            .map(|candidate| {
                let outcome = PydanticVersionStamp::new(
                    candidate.clone(),
                    "Caller-owned version oracle documentation.",
                );
                match outcome {
                    Ok(stamp) => json!({
                        "accepted": true,
                        "is_local": stamp.is_local(),
                        "raw": candidate,
                        "resource_error": false,
                    }),
                    Err(error) => json!({
                        "accepted": false,
                        "error": error.to_string(),
                        "raw": candidate,
                        "resource_error": error.to_string().contains("limit is 512"),
                    }),
                }
            })
            .collect(),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.org/schema/models.json",
        "$defs": {
            "Color": {
                "title": "Color",
                "enum": ["ex:blue", "ex:red"]
            },
            "Empty": {
                "title": "Empty",
                "enum": []
            },
            "Person": {
                "type": "object",
                "title": "Person",
                "description": "A person supplied by the oracle fixture.",
                "additionalProperties": false,
                "properties": {
                    "@id": { "type": "string" },
                    "ex:active": { "type": "boolean" },
                    "ex:address": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "ex:city": { "type": "string", "minLength": 1 },
                            "ex:postalCode": { "type": "string", "pattern": "^[A-Z][0-9]$" }
                        },
                        "required": ["ex:city"]
                    },
                    "ex:age": { "type": "integer", "minimum": 0 },
                    "ex:color": { "$ref": "#/$defs/Color" },
                    "ex:friend": { "$ref": "#/$defs/Person" },
                    "ex:label": {
                        "anyOf": [
                            { "type": "string" },
                            {
                                "type": "object",
                                "properties": {
                                    "@value": {},
                                    "@type": { "type": "string" }
                                },
                                "required": ["@value"]
                            }
                        ],
                        "minLength": 2
                    },
                    "ex:lookahead": {
                        "type": "string",
                        "pattern": "^(?=A)A"
                    },
                    "ex:name": { "type": "string", "minLength": 1 },
                    "ex:nullableCount": {
                        "type": ["integer", "null"],
                        "minimum": 0
                    },
                    "ex:nullableName": {
                        "type": ["string", "null"],
                        "minLength": 2,
                        "pattern": "^[A-Z]"
                    },
                    "ex:nullableTags": {
                        "type": ["array", "null"],
                        "items": { "type": "string" },
                        "minItems": 1
                    },
                    "ex:path": { "$ref": "#/$defs/path~1with~0token" },
                    "ex:score": { "type": "number", "minimum": 0, "maximum": 1 },
                    "ex:tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "maxItems": 3
                    },
                    "ex:when": { "type": "string", "format": "date-time" }
                },
                "required": ["ex:age", "ex:name"]
            },
            "PersonAlias": {
                "$ref": "#/$defs/Person"
            },
            "State": {
                "title": "State",
                "enum": [
                    { "@id": "ex:closed" },
                    { "@id": "ex:open" }
                ]
            },
            "path/with~token": {
                "type": "string",
                "pattern": "^mapped:"
            }
        }
    });
    let compiled = CompiledSchema {
        schema_json: format!("{}\n", serde_json::to_string_pretty(&schema)?),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    };
    let config = PydanticConfig::new(
        "oracle_models",
        "Caller-owned oracle package documentation.",
        "Caller-owned oracle model documentation.",
    )?;
    let package = emit_pydantic(&compiled, &config)?;
    let mut reverse_schema = schema.clone();
    reverse_schema["$defs"]
        .as_object_mut()
        .ok_or("oracle schema has no $defs")?
        .remove("Empty");
    let reverse_package = emit_pydantic(
        &CompiledSchema {
            schema_json: format!("{}\n", serde_json::to_string_pretty(&reverse_schema)?),
            openapi_json: "{}\n".to_owned(),
            losses: LossLedger::new(),
        },
        &config,
    )?;
    let reverse = reverse_evidence(&reverse_package, &import_config()?)?;
    let observed_losses: BTreeSet<(&str, &str)> = package
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
        .collect();
    let expected_losses = BTreeSet::from([
        (
            "format-validation-widened",
            "#/$defs/Person/properties/ex:when/format",
        ),
        (
            "keyword-validation-dropped",
            "#/$defs/Person/properties/ex:lookahead/pattern",
        ),
    ]);
    if observed_losses != expected_losses {
        return Err(format!(
            "oracle fixture loss contract disagrees: {}",
            package.losses.render_json()
        )
        .into());
    }

    let artifacts: BTreeMap<String, String> = package
        .artifacts
        .into_iter()
        .map(|(path, bytes)| String::from_utf8(bytes).map(|text| (path, text)))
        .collect::<Result<_, _>>()?;
    let output = json!({
        "artifacts": artifacts,
        "model_paths": package.model_paths,
        "reverse": reverse,
        "schema": schema,
        "version_oracle": version_oracle(),
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
