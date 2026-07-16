// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit exact and lossy packages for the dev-only GraphQL.js coercion oracle.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::error::Error;

use boon::{Compiler, Schemas};
use purrdf::loss::{LossLedger, check_ledger_complete, check_ledger_sound};
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{
    GRAPHQL_DIALECT, GRAPHQL_NAME_MAP_PATH, GRAPHQL_SCHEMA_PATH, GraphqlConfig, GraphqlPackage,
    emit_graphql,
};
use serde::Serialize;
use serde_json::{Value, json};

const CLOSED_PROFILE: [&str; 23] = [
    "additional-properties-validation-narrowed",
    "array-cardinality-validation-dropped",
    "array-contains-validation-dropped",
    "conditional-validation-dropped",
    "custom-scalar-validation-delegated",
    "dependency-validation-dropped",
    "integer-domain-validation-delegated",
    "intersection-validation-delegated",
    "keyword-validation-delegated",
    "negation-validation-delegated",
    "nullable-presence-validation-widened",
    "numeric-validation-dropped",
    "one-of-validation-delegated",
    "pattern-properties-validation-changed",
    "property-count-validation-dropped",
    "property-name-validation-changed",
    "recursive-input-nullability-relaxed",
    "singleton-list-coercion-widened",
    "string-validation-dropped",
    "tuple-array-validation-delegated",
    "unevaluated-validation-dropped",
    "union-validation-delegated",
    "unique-items-validation-dropped",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedLoss {
    code: String,
    location: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Probe {
    label: String,
    definition: String,
    graphql_type: String,
    source_value: Value,
    graphql_value: Value,
    source_valid: bool,
    used_codec: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_loss: Option<ExpectedLoss>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    sdl: String,
    fallback_scalar: String,
    name_map: Value,
    name_map_artifact: String,
    losses: Value,
    probes: Vec<Probe>,
}

fn compiled(schema: &Value) -> Result<CompiledSchema, serde_json::Error> {
    Ok(CompiledSchema {
        schema_json: format!("{}\n", serde_json::to_string_pretty(schema)?),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    })
}

fn config() -> Result<GraphqlConfig, Box<dyn Error>> {
    Ok(GraphqlConfig::new(
        "GraphqlOracle",
        "Caller-owned GraphQL differential-oracle fixture.",
        "Type-system definitions checked against source JSON Schema acceptance.",
        "JsonCarrier",
    )?)
}

fn exact_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Alias": { "$ref": "#/$defs/Person" },
            "Choice": { "enum": [true, { "state": "open" }] },
            "Int32": {
                "type": "integer",
                "minimum": -2_147_483_648_i64,
                "maximum": 2_147_483_647_i64
            },
            "Person": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "@id": { "type": "string" },
                    "ex:choice": { "$ref": "#/$defs/Choice" },
                    "ex:count": { "$ref": "#/$defs/Int32" },
                    "ex:maybe": { "type": ["string", "null"] }
                },
                "required": ["@id", "ex:choice", "ex:count"]
            }
        }
    })
}

fn lossy_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "A": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "b": { "$ref": "#/$defs/B" } },
                "required": ["b"]
            },
            "B": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "a": { "$ref": "#/$defs/A" } },
                "required": ["a"]
            },
            "Cardinality": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 2
            },
            "Conditional": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "flag": { "type": ["boolean", "null"] },
                    "value": { "type": ["string", "null"] }
                },
                "if": { "properties": { "flag": { "const": true } }, "required": ["flag"] },
                "then": { "required": ["value"] }
            },
            "Contains": {
                "type": "array",
                "items": { "type": "string" },
                "contains": { "const": "match" }
            },
            "Dependency": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "a": { "type": ["boolean", "null"] },
                    "b": { "type": ["string", "null"] }
                },
                "dependentRequired": { "a": ["b"] }
            },
            "FalseRule": false,
            "GenericInteger": { "type": "integer" },
            "IntPredicate": {
                "type": "integer",
                "minimum": -2_147_483_648_i64,
                "maximum": 2_147_483_647_i64,
                "multipleOf": 2
            },
            "Intersection": {
                "allOf": [{ "type": "string" }, { "type": "string", "minLength": 2 }]
            },
            "Negated": { "type": "string", "not": { "const": "forbidden" } },
            "OneOf": {
                "oneOf": [{ "type": "string" }, { "const": "overlap" }]
            },
            "OpenObject": {
                "type": "object",
                "properties": { "known": { "type": ["string", "null"] } }
            },
            "OptionalNonNull": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "value": { "type": "string" } }
            },
            "Patterned": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "ex:value": { "type": "string" } },
                "required": ["ex:value"],
                "patternProperties": { "^ex:": { "type": "boolean" } }
            },
            "PropertyCount": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "known": { "type": ["string", "null"] } },
                "minProperties": 1
            },
            "PropertyNames": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "bad-key": { "type": "string" } },
                "required": ["bad-key"],
                "propertyNames": { "pattern": "^[a-z]+$" }
            },
            "Singleton": {
                "type": "array",
                "items": { "type": "string" }
            },
            "StringRule": { "type": "string", "pattern": "^[A-Z]+$" },
            "Tuple": {
                "type": "array",
                "prefixItems": [{ "type": "string" }, { "type": "boolean" }],
                "items": false
            },
            "Unevaluated": {
                "type": "array",
                "contains": { "const": "match" },
                "unevaluatedItems": false
            },
            "Union": { "anyOf": [{ "type": "string" }, { "type": "boolean" }] },
            "Unique": {
                "type": "array",
                "items": { "type": "string" },
                "uniqueItems": true
            },
            "Unknown": { "type": "string", "unsupportedAssertion": true }
        }
    })
}

fn validates(schema: &Value, definition: &str, instance: &Value) -> Result<bool, Box<dyn Error>> {
    let escaped = if definition.contains('~') || definition.contains('/') {
        Cow::Owned(definition.replace('~', "~0").replace('/', "~1"))
    } else {
        Cow::Borrowed(definition)
    };
    let wrapper = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{escaped}")
    });
    let location = "mem:///graphql-oracle.schema.json";
    let mut schemas = Schemas::new();
    let mut compiler = Compiler::new();
    compiler.add_resource(location, wrapper)?;
    let compiled = compiler.compile(location, &mut schemas)?;
    Ok(schemas.validate(instance, compiled).is_ok())
}

fn has_loss(package: &GraphqlPackage, code: &str, location: &str) -> bool {
    package.losses.entries().iter().any(|entry| {
        entry.code == code
            && entry
                .location
                .as_ref()
                .and_then(|value| value.subject.as_deref())
                == Some(location)
    })
}

#[allow(clippy::too_many_arguments)]
fn probe(
    schema: &Value,
    package: &GraphqlPackage,
    label: &str,
    definition: &str,
    source_value: Value,
    expected_source_valid: bool,
    graphql_override: Option<Value>,
    expected_loss: Option<(&str, &str)>,
) -> Result<Probe, Box<dyn Error>> {
    let source_valid = validates(schema, definition, &source_value)?;
    if source_valid != expected_source_valid {
        return Err(format!(
            "source fixture probe {label:?} classified as {source_valid}, expected \
             {expected_source_valid}"
        )
        .into());
    }
    let expected_loss = expected_loss.map(|(code, location)| ExpectedLoss {
        code: code.to_owned(),
        location: location.to_owned(),
    });
    if let Some(loss) = &expected_loss
        && !has_loss(package, &loss.code, &loss.location)
    {
        return Err(format!(
            "probe {label:?} names absent loss {} at {}",
            loss.code, loss.location
        )
        .into());
    }
    let (graphql_value, used_codec) = if let Some(value) = graphql_override {
        (value, false)
    } else {
        let encoded = package.encode_input(definition, &source_value)?;
        if source_valid {
            let decoded = package.decode_output(definition, &encoded)?;
            if decoded != source_value {
                return Err(
                    format!("production GraphQL codec did not round-trip probe {label:?}").into(),
                );
            }
        }
        (encoded, true)
    };
    let input_type = &package
        .names
        .definitions
        .get(definition)
        .ok_or_else(|| format!("fixture definition {definition:?} has no generated type"))?
        .input_type;
    Ok(Probe {
        label: label.to_owned(),
        definition: definition.to_owned(),
        graphql_type: format!("{input_type}!"),
        source_value,
        graphql_value,
        source_valid,
        used_codec,
        expected_loss,
    })
}

fn artifact(package: &GraphqlPackage, path: &str) -> Result<String, Box<dyn Error>> {
    Ok(std::str::from_utf8(
        package
            .artifacts
            .get(path)
            .ok_or_else(|| format!("GraphQL package has no {path} artifact"))?,
    )?
    .to_owned())
}

fn fixture(package: &GraphqlPackage, probes: Vec<Probe>) -> Result<Fixture, Box<dyn Error>> {
    Ok(Fixture {
        sdl: artifact(package, GRAPHQL_SCHEMA_PATH)?,
        fallback_scalar: "JsonCarrier".to_owned(),
        name_map: serde_json::to_value(&package.names)?,
        name_map_artifact: artifact(package, GRAPHQL_NAME_MAP_PATH)?,
        losses: serde_json::from_str(&package.losses.render_json())?,
        probes,
    })
}

fn exact_fixture(schema: &Value, package: &GraphqlPackage) -> Result<Fixture, Box<dyn Error>> {
    if !package.losses.is_empty() {
        return Err(format!(
            "exact GraphQL fixture has losses: {}",
            package.losses.render_json()
        )
        .into());
    }
    let valid_person = json!({
        "@id": "ex:person",
        "ex:choice": { "state": "open" },
        "ex:count": 7,
        "ex:maybe": null
    });
    let probes = vec![
        probe(
            schema,
            package,
            "person-valid",
            "Person",
            valid_person,
            true,
            None,
            None,
        )?,
        probe(
            schema,
            package,
            "person-missing-required",
            "Person",
            json!({ "ex:choice": true, "ex:count": 7 }),
            false,
            None,
            None,
        )?,
        probe(
            schema,
            package,
            "person-wrong-scalar",
            "Person",
            json!({ "@id": true, "ex:choice": true, "ex:count": 7 }),
            false,
            Some(json!({ "id": true, "exChoice": "VALUE_0", "exCount": 7 })),
            None,
        )?,
        probe(
            schema,
            package,
            "person-unknown-field",
            "Person",
            json!({ "@id": "ex:p", "ex:choice": true, "ex:count": 7, "extra": true }),
            false,
            Some(json!({ "id": "ex:p", "exChoice": "VALUE_0", "exCount": 7, "extra": true })),
            None,
        )?,
        probe(
            schema,
            package,
            "choice-object",
            "Choice",
            json!({ "state": "open" }),
            true,
            None,
            None,
        )?,
        probe(
            schema,
            package,
            "choice-invalid",
            "Choice",
            json!("missing"),
            false,
            Some(json!("NOT_A_SYMBOL")),
            None,
        )?,
        probe(
            schema,
            package,
            "int32-maximum",
            "Int32",
            json!(2_147_483_647_i64),
            true,
            None,
            None,
        )?,
        probe(
            schema,
            package,
            "int32-overflow",
            "Int32",
            json!(2_147_483_648_i64),
            false,
            Some(json!(2_147_483_648_i64)),
            None,
        )?,
    ];
    fixture(package, probes)
}

fn lossy_fixture(schema: &Value, package: &GraphqlPackage) -> Result<Fixture, Box<dyn Error>> {
    check_ledger_sound(&package.losses, "json-schema", GRAPHQL_DIALECT)?;
    check_ledger_complete(&package.losses, &CLOSED_PROFILE)?;
    let rows = [
        (
            "open-object",
            "OpenObject",
            json!({"known": "ok", "extra": "x"}),
            false,
            Some(json!({"known": "ok", "extra": "x"})),
            true,
            Some((
                "additional-properties-validation-narrowed",
                "#/$defs/OpenObject",
            )),
        ),
        (
            "array-cardinality",
            "Cardinality",
            json!(["one"]),
            true,
            None,
            false,
            Some((
                "array-cardinality-validation-dropped",
                "#/$defs/Cardinality/minItems",
            )),
        ),
        (
            "array-contains",
            "Contains",
            json!(["other"]),
            true,
            None,
            false,
            Some((
                "array-contains-validation-dropped",
                "#/$defs/Contains/contains",
            )),
        ),
        (
            "conditional",
            "Conditional",
            json!({"flag": true}),
            true,
            None,
            false,
            Some(("conditional-validation-dropped", "#/$defs/Conditional/then")),
        ),
        (
            "custom-scalar",
            "FalseRule",
            json!("carried"),
            true,
            None,
            false,
            Some(("custom-scalar-validation-delegated", "#/$defs/FalseRule")),
        ),
        (
            "dependency",
            "Dependency",
            json!({"a": true}),
            true,
            None,
            false,
            Some((
                "dependency-validation-dropped",
                "#/$defs/Dependency/dependentRequired",
            )),
        ),
        (
            "integer-domain",
            "GenericInteger",
            json!(1.5),
            true,
            None,
            false,
            Some((
                "integer-domain-validation-delegated",
                "#/$defs/GenericInteger/type",
            )),
        ),
        (
            "intersection",
            "Intersection",
            json!("x"),
            true,
            None,
            false,
            Some((
                "intersection-validation-delegated",
                "#/$defs/Intersection/allOf",
            )),
        ),
        (
            "negation",
            "Negated",
            json!("forbidden"),
            true,
            None,
            false,
            Some(("negation-validation-delegated", "#/$defs/Negated/not")),
        ),
        (
            "nullable-presence",
            "OptionalNonNull",
            json!({"value": null}),
            true,
            None,
            false,
            Some((
                "nullable-presence-validation-widened",
                "#/$defs/OptionalNonNull/properties/value",
            )),
        ),
        (
            "numeric-predicate",
            "IntPredicate",
            json!(3),
            true,
            None,
            false,
            Some((
                "numeric-validation-dropped",
                "#/$defs/IntPredicate/multipleOf",
            )),
        ),
        (
            "one-of",
            "OneOf",
            json!("overlap"),
            true,
            None,
            false,
            Some(("one-of-validation-delegated", "#/$defs/OneOf/oneOf")),
        ),
        (
            "pattern-properties",
            "Patterned",
            json!({"ex:value": "text"}),
            true,
            None,
            false,
            Some((
                "pattern-properties-validation-changed",
                "#/$defs/Patterned/patternProperties/^ex:",
            )),
        ),
        (
            "property-count",
            "PropertyCount",
            json!({}),
            true,
            None,
            false,
            Some((
                "property-count-validation-dropped",
                "#/$defs/PropertyCount/minProperties",
            )),
        ),
        (
            "property-name",
            "PropertyNames",
            json!({"bad-key": "x"}),
            true,
            None,
            false,
            Some((
                "property-name-validation-changed",
                "#/$defs/PropertyNames/propertyNames",
            )),
        ),
        (
            "recursive-input",
            "A",
            json!({"b": {}}),
            true,
            None,
            false,
            Some((
                "recursive-input-nullability-relaxed",
                "#/$defs/B/properties/a",
            )),
        ),
        (
            "singleton-list",
            "Singleton",
            json!("one"),
            true,
            Some(json!("one")),
            false,
            Some(("singleton-list-coercion-widened", "#/$defs/Singleton")),
        ),
        (
            "string-rule",
            "StringRule",
            json!("lower"),
            true,
            None,
            false,
            Some(("string-validation-dropped", "#/$defs/StringRule/pattern")),
        ),
        (
            "tuple",
            "Tuple",
            json!([true, false]),
            true,
            None,
            false,
            Some((
                "tuple-array-validation-delegated",
                "#/$defs/Tuple/prefixItems",
            )),
        ),
        (
            "unevaluated",
            "Unevaluated",
            json!(["match", "other"]),
            true,
            None,
            false,
            Some((
                "unevaluated-validation-dropped",
                "#/$defs/Unevaluated/unevaluatedItems",
            )),
        ),
        (
            "union",
            "Union",
            json!(7),
            true,
            None,
            false,
            Some(("union-validation-delegated", "#/$defs/Union/anyOf")),
        ),
        (
            "unique",
            "Unique",
            json!(["x", "x"]),
            true,
            None,
            false,
            Some((
                "unique-items-validation-dropped",
                "#/$defs/Unique/uniqueItems",
            )),
        ),
    ];
    let mut probes = Vec::with_capacity(rows.len());
    for (label, definition, value, expected_graphql_valid, override_value, source_valid, loss) in
        rows
    {
        if expected_graphql_valid == source_valid {
            return Err(format!("lossy fixture row {label:?} must declare a divergence").into());
        }
        probes.push(probe(
            schema,
            package,
            label,
            definition,
            value,
            source_valid,
            override_value,
            loss,
        )?);
    }
    fixture(package, probes)
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = config()?;
    let exact_schema = exact_schema();
    let lossy_schema = lossy_schema();
    let exact_package = emit_graphql(&compiled(&exact_schema)?, &config)?;
    let lossy_package = emit_graphql(&compiled(&lossy_schema)?, &config)?;
    let observed: BTreeSet<&str> = lossy_package
        .losses
        .entries()
        .iter()
        .map(|entry| entry.code.as_ref())
        .collect();
    let expected: BTreeSet<&str> = CLOSED_PROFILE.into_iter().collect();
    if observed != expected {
        return Err(format!("GraphQL oracle loss profile drift: {observed:?}").into());
    }
    let output = json!({
        "dialect": GRAPHQL_DIALECT,
        "closedProfile": CLOSED_PROFILE,
        "exact": exact_fixture(&exact_schema, &exact_package)?,
        "lossy": lossy_fixture(&lossy_schema, &lossy_package)?,
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
