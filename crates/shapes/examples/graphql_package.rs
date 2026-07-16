// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit a deterministic GraphQL package and use its reversible value codec.

use std::error::Error;

use purrdf::loss::LossLedger;
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{GRAPHQL_NAME_MAP_PATH, GRAPHQL_SCHEMA_PATH, GraphqlConfig, emit_graphql};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    let source_schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Status": { "enum": ["active", "paused"] },
            "Person": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "@id": { "type": "string" },
                    "ex:status": { "$ref": "#/$defs/Status" }
                },
                "required": ["@id", "ex:status"]
            }
        }
    });
    let compiled = CompiledSchema {
        schema_json: format!("{}\n", serde_json::to_string_pretty(&source_schema)?),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    };
    let config = GraphqlConfig::new(
        "ExampleSchema",
        "GraphQL schema package owned by the caller.",
        "Types generated from the caller's compiled schema.",
        "JsonCarrier",
    )?;
    let package = emit_graphql(&compiled, &config)?;

    assert!(package.losses.is_empty());
    assert_eq!(package.names.definitions["Person"].output_type, "Person");
    assert_eq!(package.names.fields["#/$defs/Person"]["@id"], "id");

    let source_value = json!({
        "@id": "https://example.org/alice",
        "ex:status": "active"
    });
    let graphql_value = package.encode_input("Person", &source_value)?;
    assert_eq!(
        package.decode_output("Person", &graphql_value)?,
        source_value
    );

    let sdl = std::str::from_utf8(&package.artifacts[GRAPHQL_SCHEMA_PATH])?;
    let name_map = std::str::from_utf8(&package.artifacts[GRAPHQL_NAME_MAP_PATH])?;
    assert!(sdl.contains("type Person"));
    assert!(sdl.contains("input PersonInput"));
    assert!(name_map.ends_with('\n'));
    Ok(())
}
