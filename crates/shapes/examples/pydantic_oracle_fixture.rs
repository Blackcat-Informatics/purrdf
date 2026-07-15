// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit the deterministic package consumed by the dev-only Pydantic oracle.

use std::collections::BTreeMap;
use std::error::Error;

use purrdf::loss::LossLedger;
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{PydanticConfig, emit_pydantic};
use serde_json::json;

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
                    "ex:name": { "type": "string", "minLength": 1 },
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
    if !package.losses.is_empty() {
        return Err(format!(
            "oracle fixture unexpectedly incurred losses: {}",
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
        "schema": schema,
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
