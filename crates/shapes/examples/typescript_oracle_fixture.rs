// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Emit exact and lossy packages for the dev-only TypeScript compiler oracle.

use std::collections::BTreeSet;
use std::error::Error;

#[path = "support/shacl_lists.rs"]
mod shacl_lists;
#[path = "support/shacl_temporal.rs"]
mod shacl_temporal;

use boon::{Compiler, Schemas};
use purrdf::loss::{LossLedger, check_ledger_complete, check_ledger_sound};
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces};
use purrdf_shapes::{
    SchemaDatatypeMap, SchemaImportConfig, TYPESCRIPT_DECLARATION_PATH, TYPESCRIPT_DIALECT,
    TypeScriptConfig, TypeScriptPackage, emit_typescript, import_typescript_package,
};
use serde::Serialize;
use serde_json::{Value, json};

const CLOSED_PROFILE: [&str; 16] = [
    "additional-properties-validation-widened",
    "array-contains-validation-dropped",
    "conditional-validation-dropped",
    "dependency-validation-dropped",
    "integer-validation-widened",
    "keyword-validation-dropped",
    "negation-validation-dropped",
    "numeric-validation-dropped",
    "object-literal-validation-widened",
    "one-of-validation-widened",
    "pattern-properties-validation-dropped",
    "property-count-validation-dropped",
    "property-name-validation-dropped",
    "string-validation-dropped",
    "unevaluated-validation-dropped",
    "unique-items-validation-dropped",
];
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

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
    type_name: String,
    value: Value,
    mode: String,
    source_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_loss: Option<ExpectedLoss>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompilerProbe {
    label: String,
    type_name: String,
    expression: String,
    expected_typescript_valid: bool,
}

/// A compiler fact a recorded loss rests on: a whole source file, beside the
/// declaration, that the compiler must accept or reject (with `code`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Proof {
    label: String,
    source: String,
    expected_typescript_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_code: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    declaration: String,
    type_names: std::collections::BTreeMap<String, String>,
    losses: Value,
    probes: Vec<Probe>,
    compiler_probes: Vec<CompilerProbe>,
    proofs: Vec<Proof>,
}

fn compiled(schema: &Value) -> Result<CompiledSchema, serde_json::Error> {
    Ok(CompiledSchema {
        schema_json: format!("{}\n", serde_json::to_string_pretty(schema)?),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    })
}

fn config() -> Result<TypeScriptConfig, Box<dyn Error>> {
    Ok(TypeScriptConfig::new(
        "@example/typescript-oracle",
        "Caller-owned TypeScript differential-oracle fixture.",
        "Declarations checked against the source JSON Schema acceptance relation.",
    )?)
}

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
    package: &TypeScriptPackage,
    config: &SchemaImportConfig,
) -> Result<Value, Box<dyn Error>> {
    let imported = import_typescript_package(package, config)?;
    check_ledger_sound(&imported.losses, TYPESCRIPT_DIALECT, "shacl")?;
    let repeated = import_typescript_package(package, config)?;
    if imported.losses.render_json() != repeated.losses.render_json() {
        return Err("TypeScript reverse ledger is not deterministic".into());
    }
    let first = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces())?;
    let second = purrdf_shapes::json_schema::compile(&repeated.shapes, config.namespaces())?;
    if first.schema_json != second.schema_json {
        return Err("TypeScript reverse shapes are not byte-deterministic".into());
    }
    Ok(json!({
        "losses": serde_json::from_str::<Value>(&imported.losses.render_json())?,
        "shapeIds": imported
            .shapes
            .node_shapes
            .iter()
            .map(|shape| shape.id.to_string())
            .collect::<Vec<_>>(),
    }))
}

fn exact_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.org/schema/typescript-exact.json",
        "$defs": {
            "Alias": { "$ref": "#/$defs/Person" },
            "Choice": {
                "anyOf": [
                    { "const": "ex:open" },
                    { "const": true }
                ]
            },
            "ClosedEmpty": {
                "type": "object",
                "additionalProperties": false
            },
            "Empty": { "enum": [] },
            "Extended": {
                "allOf": [
                    { "$ref": "#/$defs/Person" },
                    {
                        "type": "object",
                        "properties": {
                            "ex:score": { "type": "number" }
                        },
                        "required": ["ex:score"]
                    }
                ]
            },
            "JsonAny": true,
            "Cardinality": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 40
            },
            "DistinctChoice": {
                "type": "array",
                "items": { "enum": ["a", "b", "c"] },
                "uniqueItems": true
            },
            "DistinctBoundedSeven": {
                "type": "array",
                "items": { "enum": literal_items(7) },
                "minItems": 1,
                "maxItems": 5,
                "uniqueItems": true
            },
            "DistinctChain": {
                "type": "array",
                "prefixItems": pair_chain(24),
                "items": false,
                "uniqueItems": true
            },
            "DistinctEight": {
                "type": "array",
                "items": { "enum": literal_items(8) },
                "uniqueItems": true
            },

            "LargeBounds": {
                "type": "array",
                "items": { "const": "x" },
                "minItems": 20_000,
                "maxItems": 20_001
            },
            "LongTuple": {
                "type": "array",
                "prefixItems": long_prefix(),
                "items": false
            },
            "NumericChoice": { "enum": [1, 2, 3], "minimum": 2 },
            "Nothing": false,
            "Person": {
                "type": "object",
                "properties": {
                    "@id": { "type": "string" },
                    "ex:choice": { "$ref": "#/$defs/Choice" },
                    "ex:friend": { "$ref": "#/$defs/Person" },
                    "ex:nullable": { "type": ["string", "null"] },
                    "ex:tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "maxItems": 2
                    },
                    "ex:tuple": {
                        "type": "array",
                        "prefixItems": [
                            { "type": "string" },
                            { "type": "number" }
                        ],
                        "items": false
                    }
                },
                "required": ["@id"]
            },
            "Tree": {
                "type": "object",
                "properties": {
                    "children": {
                        "type": "array",
                        "items": { "$ref": "#/$defs/Tree" }
                    },
                    "value": { "type": "string" }
                },
                "required": ["value"]
            },
            "path/with~token": { "enum": [null, 7, "mapped"] }
        }
    })
}

/// `count` string literals `v0`, `v1`, ….
fn literal_items(count: usize) -> Vec<Value> {
    (0..count).map(|index| json!(format!("v{index}"))).collect()
}

/// `count` positions, position `i` admitting `v{i}` or `x`: its distinct
/// sequences reach every length, and `x` may repeat only against uniqueness.
fn pair_chain(count: usize) -> Vec<Value> {
    (0..count)
        .map(|index| json!({ "enum": [format!("v{index}"), "x"] }))
        .collect()
}

/// The `pair_chain` values `v0`, `v1`, … of `count` positions.
fn pair_values(count: usize) -> Value {
    Value::Array((0..count).map(|index| json!(format!("v{index}"))).collect())
}

/// Thirty-three alternating string and number positions.
fn long_prefix() -> Vec<Value> {
    (0..33)
        .map(|index| {
            if index % 2 == 0 {
                json!({ "type": "string" })
            } else {
                json!({ "type": "number" })
            }
        })
        .collect()
}

/// A tuple value for [`long_prefix`] of `length` positions.
fn long_tuple(length: usize) -> Value {
    Value::Array(
        (0..length)
            .map(|index| if index % 2 == 0 { json!("s") } else { json!(1) })
            .collect(),
    )
}

fn lossy_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.org/schema/typescript-lossy.json",
        "$defs": {
            "ClosedNamed": {
                "type": "object",
                "additionalProperties": false,
                "properties": { "known": { "type": "string" } },
                "required": ["known"]
            },
            "Conditional": {
                "type": "object",
                "properties": {
                    "flag": { "type": "boolean" },
                    "value": { "type": "string" }
                },
                "if": {
                    "properties": { "flag": { "const": true } },
                    "required": ["flag"]
                },
                "then": { "required": ["value"] }
            },
            "Contains": {
                "type": "array",
                "items": { "type": "string" },
                "contains": { "const": "match" }
            },
            "Dependency": {
                "type": "object",
                "properties": {
                    "a": { "type": "boolean" },
                    "b": { "type": "string" }
                },
                "dependentRequired": { "a": ["b"] }
            },
            "IntegerRule": { "type": "integer" },
            "Negated": { "not": { "type": "boolean" } },
            "NestedObjectLiteral": {
                "const": [{ "state": "open" }]
            },
            "NumericRule": { "type": "number", "minimum": 0 },
            "ObjectLiteral": { "enum": [{ "state": "open" }] },
            "OneOfExclusive": {
                "oneOf": [
                    { "type": "string" },
                    { "const": "overlap" }
                ]
            },
            "Patterned": {
                "type": "object",
                "patternProperties": {
                    "^ex:": { "type": "string" }
                },
                "additionalProperties": false
            },
            "PropertyCount": {
                "type": "object",
                "minProperties": 1
            },
            "PropertyNames": {
                "type": "object",
                "propertyNames": { "pattern": "^ex:" }
            },
            "StringRule": { "type": "string", "pattern": "^[A-Z]" },
            "UnevaluatedArray": {
                "type": "array",
                "prefixItems": [{ "type": "string" }],
                "unevaluatedItems": false
            },
            "UnevaluatedObject": {
                "allOf": [{
                    "type": "object",
                    "properties": { "known": { "type": "string" } }
                }],
                "unevaluatedProperties": false
            },
            "UniqueItems": {
                "type": "array",
                "items": { "type": "string" },
                "uniqueItems": true
            },
            "UniqueBoundedEight": {
                "type": "array",
                "items": { "enum": literal_items(8) },
                "minItems": 1,
                "uniqueItems": true
            },
            "UniqueChainTooDeep": {
                "type": "array",
                "prefixItems": pair_chain(25),
                "items": false,
                "uniqueItems": true
            },
            "UniqueNineItems": {
                "type": "array",
                "items": { "enum": literal_items(9) },
                "uniqueItems": true
            },
            "UnsafeIntegerLiteral": {
                "const": 9_007_199_254_740_993_u64
            },
            "UnknownRule": { "unsupportedAssertion": true }
        }
    })
}

fn validates(schema: &Value, definition: &str, instance: &Value) -> Result<bool, Box<dyn Error>> {
    let escaped = definition.replace('~', "~0").replace('/', "~1");
    let wrapper = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{escaped}")
    });
    let location = "mem:///typescript-oracle.schema.json";
    let mut schemas = Schemas::new();
    let mut compiler = Compiler::new();
    compiler.add_resource(location, wrapper)?;
    let compiled = compiler.compile(location, &mut schemas)?;
    Ok(schemas.validate(instance, compiled).is_ok())
}

fn has_loss(package: &TypeScriptPackage, code: &str, location: &str) -> bool {
    package.losses.entries().iter().any(|entry| {
        entry.code == code
            && entry
                .location
                .as_ref()
                .and_then(|value| value.subject.as_deref())
                == Some(location)
    })
}

// Keep each fixture row explicit: schema, projection, compiler mode, independent
// source classification, and located-loss expectation are separate oracle inputs.
#[allow(clippy::too_many_arguments)]
fn probe(
    schema: &Value,
    package: &TypeScriptPackage,
    label: &str,
    definition: &str,
    value: Value,
    mode: &str,
    expected_source_valid: bool,
    expected_loss: Option<(&str, &str)>,
) -> Result<Probe, Box<dyn Error>> {
    let source_valid = validates(schema, definition, &value)?;
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
    let type_name = package
        .type_names
        .get(definition)
        .ok_or_else(|| format!("fixture definition {definition:?} has no generated type"))?
        .clone();
    Ok(Probe {
        label: label.to_owned(),
        type_name,
        value,
        mode: mode.to_owned(),
        source_valid,
        expected_loss,
    })
}

fn declaration(package: &TypeScriptPackage) -> Result<String, Box<dyn Error>> {
    let bytes = package
        .artifacts
        .get(TYPESCRIPT_DECLARATION_PATH)
        .ok_or("TypeScript package has no declaration artifact")?;
    Ok(std::str::from_utf8(bytes)?.to_owned())
}

fn ledger_json(package: &TypeScriptPackage) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&package.losses.render_json())?)
}

fn exact_fixture(schema: &Value, package: TypeScriptPackage) -> Result<Fixture, Box<dyn Error>> {
    if !package.losses.is_empty() {
        return Err(format!(
            "exact TypeScript fixture unexpectedly lost semantics: {}",
            package.losses.render_json()
        )
        .into());
    }
    let probes = vec![
        probe(
            schema,
            &package,
            "alias-valid",
            "Alias",
            json!({"@id": "ex:a"}),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "choice-string",
            "Choice",
            json!("ex:open"),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "choice-boolean",
            "Choice",
            json!(true),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "choice-reject",
            "Choice",
            json!("closed"),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "closed-empty",
            "ClosedEmpty",
            json!({}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "closed-empty-rejects-key",
            "ClosedEmpty",
            json!({"extra": 1}),
            "variable",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "empty-enum",
            "Empty",
            json!(null),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "extended-valid",
            "Extended",
            json!({"@id": "ex:a", "ex:score": 0.5}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "extended-missing-score",
            "Extended",
            json!({"@id": "ex:a"}),
            "variable",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "json-any",
            "JsonAny",
            json!({"nested": [true, null, 3]}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "false-schema",
            "Nothing",
            json!(false),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-minimal",
            "Person",
            json!({"@id": "ex:a"}),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-required",
            "Person",
            json!({}),
            "variable",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-null",
            "Person",
            json!({"@id": "ex:a", "ex:nullable": null}),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-nullable-rejects-number",
            "Person",
            json!({"@id": "ex:a", "ex:nullable": 3}),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-open",
            "Person",
            json!({"@id": "ex:a", "extra": {"ok": true}}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-tags-min",
            "Person",
            json!({"@id": "ex:a", "ex:tags": []}),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-tags-max",
            "Person",
            json!({"@id": "ex:a", "ex:tags": ["a", "b", "c"]}),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-tuple-prefix",
            "Person",
            json!({"@id": "ex:a", "ex:tuple": ["a", 1]}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-tuple-short",
            "Person",
            json!({"@id": "ex:a", "ex:tuple": ["a"]}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "person-tuple-order",
            "Person",
            json!({"@id": "ex:a", "ex:tuple": [1, "a"]}),
            "variable",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "recursive-valid",
            "Tree",
            json!({"value": "root", "children": [{"value": "leaf"}]}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "recursive-required",
            "Tree",
            json!({"value": "root", "children": [{}]}),
            "variable",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "escaped-name",
            "path/with~token",
            json!("mapped"),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "escaped-name-reject",
            "path/with~token",
            json!(8),
            "fresh",
            false,
            None,
        )?,
    ];
    let mut probes = probes;
    let strings = |length: usize, value: &str| Value::Array(vec![json!(value); length]);
    for (label, definition, value, mode, valid) in [
        (
            "cardinality-below",
            "Cardinality",
            strings(39, "s"),
            "variable",
            false,
        ),
        (
            "cardinality-at",
            "Cardinality",
            strings(40, "s"),
            "fresh",
            true,
        ),
        (
            "large-below",
            "LargeBounds",
            strings(19_999, "x"),
            "fresh",
            false,
        ),
        (
            "large-at-minimum",
            "LargeBounds",
            strings(20_000, "x"),
            "fresh",
            true,
        ),
        (
            "large-at-maximum",
            "LargeBounds",
            strings(20_001, "x"),
            "variable",
            true,
        ),
        (
            "large-above",
            "LargeBounds",
            strings(20_002, "x"),
            "fresh",
            false,
        ),
        (
            "long-tuple-full",
            "LongTuple",
            long_tuple(33),
            "fresh",
            true,
        ),
        (
            "long-tuple-short",
            "LongTuple",
            long_tuple(20),
            "variable",
            true,
        ),
        (
            "long-tuple-position",
            "LongTuple",
            json!([7]),
            "variable",
            false,
        ),
        (
            "long-tuple-too-long",
            "LongTuple",
            long_tuple(34),
            "fresh",
            false,
        ),
        (
            "distinct-all",
            "DistinctChoice",
            json!(["c", "a", "b"]),
            "fresh",
            true,
        ),
        (
            "distinct-empty",
            "DistinctChoice",
            json!([]),
            "variable",
            true,
        ),
        (
            "distinct-repeated",
            "DistinctChoice",
            json!(["a", "b", "a"]),
            "fresh",
            false,
        ),
        (
            "distinct-outside",
            "DistinctChoice",
            json!(["a", "d"]),
            "variable",
            false,
        ),
        (
            "numeric-choice-kept",
            "NumericChoice",
            json!(3),
            "fresh",
            true,
        ),
        (
            "distinct-eight-all",
            "DistinctEight",
            Value::Array(literal_items(8)),
            "fresh",
            true,
        ),
        (
            "distinct-eight-repeated",
            "DistinctEight",
            json!(["v1", "v0", "v1"]),
            "fresh",
            false,
        ),
        (
            "distinct-chain-short",
            "DistinctChain",
            json!(["v0", "x", "v2"]),
            "fresh",
            true,
        ),
        (
            "distinct-chain-full",
            "DistinctChain",
            pair_values(24),
            "variable",
            true,
        ),
        (
            "distinct-chain-repeated",
            "DistinctChain",
            json!(["x", "x"]),
            "variable",
            false,
        ),
        (
            "distinct-bounded-empty",
            "DistinctBoundedSeven",
            json!([]),
            "fresh",
            false,
        ),
        (
            "distinct-bounded-five",
            "DistinctBoundedSeven",
            json!(["v6", "v0", "v1", "v2", "v3"]),
            "fresh",
            true,
        ),
        (
            "distinct-bounded-six",
            "DistinctBoundedSeven",
            json!(["v6", "v0", "v1", "v2", "v3", "v4"]),
            "fresh",
            false,
        ),
        (
            "distinct-bounded-repeated",
            "DistinctBoundedSeven",
            json!(["v2", "v2"]),
            "variable",
            false,
        ),
        (
            "numeric-choice-left-out",
            "NumericChoice",
            json!(1),
            "fresh",
            false,
        ),
    ] {
        probes.push(probe(
            schema, &package, label, definition, value, mode, valid, None,
        )?);
    }
    let person = package.type_names["Person"].clone();
    let compiler_probes = vec![
        CompilerProbe {
            label: "optional-property-may-be-absent".to_owned(),
            type_name: person.clone(),
            expression: r#"{ "@id": "ex:a" }"#.to_owned(),
            expected_typescript_valid: true,
        },
        CompilerProbe {
            label: "json-null-is-explicitly-represented".to_owned(),
            type_name: person.clone(),
            expression: r#"{ "@id": "ex:a", "ex:nullable": null }"#.to_owned(),
            expected_typescript_valid: true,
        },
        CompilerProbe {
            label: "undefined-is-not-json-null-or-absence".to_owned(),
            type_name: person,
            expression: r#"{ "@id": "ex:a", "ex:nullable": undefined }"#.to_owned(),
            expected_typescript_valid: false,
        },
    ];
    let declaration = declaration(&package)?;
    let losses = ledger_json(&package)?;
    Ok(Fixture {
        declaration,
        type_names: package.type_names,
        losses,
        probes,
        compiler_probes,
        proofs: distinct_limit_proofs(),
    })
}

/// The compiler facts the `JsonDistinct` limits rest on. Each level of the
/// enumeration adds three instantiations, four when its candidates are a
/// union, and TypeScript refuses depth 100 (TS2589): a chain of 32 single-value
/// positions (3 × 32 + 3 = 99) enumerates and 33 do not; 24 two-value positions
/// (4 × 24 + 3 = 99) do and 25 do not; 31 single and one two-value position
/// (3 + 3 × 30 + 4 + 3 = 100) do not, which pins the last level's three; the
/// levels after the prefix cost the same (29 positions and a two-value rest
/// do, 30 do not). A union spread or intersected over 100,000 members is
/// refused (TS2590): after a first element eight literal items leave 13,700
/// sequences and nine 109,601; intersecting length bounds with the 13,700 of
/// seven items holds and with the 109,601 of eight does not.
fn distinct_limit_proofs() -> Vec<Proof> {
    let literals = |prefix: &str, count: usize| {
        (0..count)
            .map(|index| format!("\"{prefix}{index}\""))
            .collect::<Vec<_>>()
    };
    let source = |distinct: &str, value: &str| {
        format!(
            "import type {{ JsonDistinct }} from \"./index.js\";\n\
             const value: {distinct} = {value};\n\
             void value;\n\
             export {{}};\n"
        )
    };
    let chain = |singles: usize, pairs: usize, rest: &str| {
        let mut positions = literals("s", singles);
        positions.extend((0..pairs).map(|index| format!("\"p{index}\" | \"x\"")));
        let mut value = literals("s", singles);
        value.extend(literals("p", pairs));
        source(
            &format!("JsonDistinct<readonly [{}], {rest}>", positions.join(", ")),
            &format!("[{}]", value.join(", ")),
        )
    };
    let pool = |count: usize, bounded: bool| {
        let items = literals("v", count).join(" | ");
        let distinct = format!("JsonDistinct<readonly [], {items}>");
        let distinct = if bounded {
            format!("({distinct} & {{ readonly \"0\": {items} }})")
        } else {
            distinct
        };
        source(&distinct, "[\"v0\", \"v1\"]")
    };
    let proof = |label: &str, source: String, code: Option<u32>| Proof {
        label: label.to_owned(),
        source,
        expected_typescript_valid: code.is_none(),
        expected_code: code,
    };
    vec![
        proof("chain-of-32-single-values", chain(32, 0, "never"), None),
        proof(
            "chain-of-33-single-values",
            chain(33, 0, "never"),
            Some(2589),
        ),
        proof("chain-of-24-two-values", chain(0, 24, "never"), None),
        proof("chain-of-25-two-values", chain(0, 25, "never"), Some(2589)),
        proof(
            "chain-of-31-single-and-1-two-values",
            chain(31, 1, "never"),
            Some(2589),
        ),
        proof(
            "chain-of-29-then-two-rest-values",
            chain(29, 0, "\"r0\" | \"r1\""),
            None,
        ),
        proof(
            "chain-of-30-then-two-rest-values",
            chain(30, 0, "\"r0\" | \"r1\""),
            Some(2589),
        ),
        proof("eight-items-spread-13700", pool(8, false), None),
        proof("nine-items-spread-109601", pool(9, false), Some(2590)),
        proof("seven-items-bounded-13700", pool(7, true), None),
        proof("eight-items-bounded-109601", pool(8, true), Some(2590)),
    ]
}

/// Why the recorded numeric and uniqueness losses are the language's: a type
/// cannot take a literal out of `number` or `string` (`Exclude` leaves them
/// whole), so no type states "not -1" or "not the other element's value".
fn complement_proofs() -> Vec<Proof> {
    let source =
        |declaration: &str| format!("const value: {declaration};\nvoid value;\nexport {{}};\n");
    vec![
        Proof {
            label: "number-has-no-literal-complement".to_owned(),
            source: source("Exclude<number, -1> = -1"),
            expected_typescript_valid: true,
            expected_code: None,
        },
        Proof {
            label: "string-has-no-literal-complement".to_owned(),
            source: source("Exclude<string, \"same\"> = \"same\""),
            expected_typescript_valid: true,
            expected_code: None,
        },
    ]
}

fn lossy_fixture(schema: &Value, package: TypeScriptPackage) -> Result<Fixture, Box<dyn Error>> {
    check_ledger_sound(&package.losses, "json-schema", "typescript-7.0")?;
    check_ledger_complete(&package.losses, &CLOSED_PROFILE)?;
    let observed_codes = package
        .losses
        .entries()
        .iter()
        .map(|entry| entry.code.as_ref())
        .collect::<BTreeSet<_>>();
    let expected_codes = CLOSED_PROFILE.into_iter().collect::<BTreeSet<_>>();
    if observed_codes != expected_codes {
        return Err(format!("lossy TypeScript fixture code drift: {observed_codes:?}").into());
    }
    let probes = vec![
        probe(
            schema,
            &package,
            "closed-named-baseline",
            "ClosedNamed",
            json!({"known": "yes"}),
            "fresh",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "closed-named-fresh-extra",
            "ClosedNamed",
            json!({"known": "yes", "extra": true}),
            "fresh",
            false,
            None,
        )?,
        probe(
            schema,
            &package,
            "closed-named-variable-extra",
            "ClosedNamed",
            json!({"known": "yes", "extra": true}),
            "variable",
            false,
            Some((
                "additional-properties-validation-widened",
                "#/$defs/ClosedNamed/additionalProperties",
            )),
        )?,
        probe(
            schema,
            &package,
            "contains-match",
            "Contains",
            json!(["other"]),
            "variable",
            false,
            Some((
                "array-contains-validation-dropped",
                "#/$defs/Contains/contains",
            )),
        )?,
        probe(
            schema,
            &package,
            "conditional-then",
            "Conditional",
            json!({"flag": true}),
            "variable",
            false,
            Some(("conditional-validation-dropped", "#/$defs/Conditional/then")),
        )?,
        probe(
            schema,
            &package,
            "dependent-required",
            "Dependency",
            json!({"a": true}),
            "variable",
            false,
            Some((
                "dependency-validation-dropped",
                "#/$defs/Dependency/dependentRequired",
            )),
        )?,
        probe(
            schema,
            &package,
            "integer-fraction",
            "IntegerRule",
            json!(1.5),
            "fresh",
            false,
            Some(("integer-validation-widened", "#/$defs/IntegerRule/type")),
        )?,
        probe(
            schema,
            &package,
            "negation",
            "Negated",
            json!(true),
            "fresh",
            false,
            Some(("negation-validation-dropped", "#/$defs/Negated/not")),
        )?,
        probe(
            schema,
            &package,
            "nested-object-literal-variable-extra",
            "NestedObjectLiteral",
            json!([{"state": "open", "extra": true}]),
            "variable",
            false,
            Some((
                "object-literal-validation-widened",
                "#/$defs/NestedObjectLiteral/const/0",
            )),
        )?,
        probe(
            schema,
            &package,
            "numeric-minimum",
            "NumericRule",
            json!(-1),
            "fresh",
            false,
            Some(("numeric-validation-dropped", "#/$defs/NumericRule/minimum")),
        )?,
        probe(
            schema,
            &package,
            "object-literal-variable-extra",
            "ObjectLiteral",
            json!({"state": "open", "extra": true}),
            "variable",
            false,
            Some((
                "object-literal-validation-widened",
                "#/$defs/ObjectLiteral/enum/0",
            )),
        )?,
        probe(
            schema,
            &package,
            "one-of-overlap",
            "OneOfExclusive",
            json!("overlap"),
            "fresh",
            false,
            Some(("one-of-validation-widened", "#/$defs/OneOfExclusive/oneOf")),
        )?,
        probe(
            schema,
            &package,
            "patterned-baseline",
            "Patterned",
            json!({"ex:name": "Alice"}),
            "variable",
            true,
            None,
        )?,
        probe(
            schema,
            &package,
            "pattern-key-selection",
            "Patterned",
            json!({"wrong": "Alice"}),
            "variable",
            false,
            Some((
                "pattern-properties-validation-dropped",
                "#/$defs/Patterned/patternProperties/^ex:",
            )),
        )?,
        probe(
            schema,
            &package,
            "property-count",
            "PropertyCount",
            json!({}),
            "variable",
            false,
            Some((
                "property-count-validation-dropped",
                "#/$defs/PropertyCount/minProperties",
            )),
        )?,
        probe(
            schema,
            &package,
            "property-name",
            "PropertyNames",
            json!({"wrong": 1}),
            "variable",
            false,
            Some((
                "property-name-validation-dropped",
                "#/$defs/PropertyNames/propertyNames",
            )),
        )?,
        probe(
            schema,
            &package,
            "string-pattern",
            "StringRule",
            json!("lower"),
            "fresh",
            false,
            Some(("string-validation-dropped", "#/$defs/StringRule/pattern")),
        )?,
        probe(
            schema,
            &package,
            "unevaluated-array",
            "UnevaluatedArray",
            json!(["first", 2]),
            "variable",
            false,
            Some((
                "unevaluated-validation-dropped",
                "#/$defs/UnevaluatedArray/unevaluatedItems",
            )),
        )?,
        probe(
            schema,
            &package,
            "unevaluated-object",
            "UnevaluatedObject",
            json!({"known": "yes", "extra": true}),
            "variable",
            false,
            Some((
                "unevaluated-validation-dropped",
                "#/$defs/UnevaluatedObject/unevaluatedProperties",
            )),
        )?,
        probe(
            schema,
            &package,
            "unique-items",
            "UniqueItems",
            json!(["same", "same"]),
            "variable",
            false,
            Some((
                "unique-items-validation-dropped",
                "#/$defs/UniqueItems/uniqueItems",
            )),
        )?,
        probe(
            schema,
            &package,
            "unique-chain-too-deep",
            "UniqueChainTooDeep",
            json!(["x", "x"]),
            "fresh",
            false,
            Some((
                "unique-items-validation-dropped",
                "#/$defs/UniqueChainTooDeep/uniqueItems",
            )),
        )?,
        probe(
            schema,
            &package,
            "unique-bounded-eight",
            "UniqueBoundedEight",
            json!(["v3", "v3"]),
            "fresh",
            false,
            Some((
                "unique-items-validation-dropped",
                "#/$defs/UniqueBoundedEight/uniqueItems",
            )),
        )?,
        probe(
            schema,
            &package,
            "unique-nine-items",
            "UniqueNineItems",
            json!(["v8", "v8"]),
            "fresh",
            false,
            Some((
                "unique-items-validation-dropped",
                "#/$defs/UniqueNineItems/uniqueItems",
            )),
        )?,
        probe(
            schema,
            &package,
            "unsafe-integer-literal",
            "UnsafeIntegerLiteral",
            json!(9_007_199_254_740_992_u64),
            "fresh",
            false,
            Some((
                "numeric-validation-dropped",
                "#/$defs/UnsafeIntegerLiteral/const",
            )),
        )?,
        probe(
            schema,
            &package,
            "unknown-keyword-is-conservatively-ledgered",
            "UnknownRule",
            json!({"still": "json"}),
            "variable",
            true,
            None,
        )?,
    ];
    let declaration = declaration(&package)?;
    let losses = ledger_json(&package)?;
    Ok(Fixture {
        declaration,
        type_names: package.type_names,
        losses,
        probes,
        compiler_probes: Vec::new(),
        proofs: complement_proofs(),
    })
}

/// The SHACL list-component fixture (see `support/shacl_lists.rs`): the
/// projected instances of real data, whose verdicts are SHACL validation's.
/// TypeScript expresses the length bounds on element properties and the member
/// type as the element type; distinct elements of an unconstrained item type
/// and an integer minimum have no type (see [`complement_proofs`]), so exactly
/// those two probes diverge, at their located losses.
fn lists_fixture() -> Result<Fixture, Box<dyn Error>> {
    let compiled = shacl_lists::compiled()?;
    let schema: Value = serde_json::from_str(&compiled.schema_json)?;
    let package = emit_typescript(&compiled, &config()?)?;
    check_ledger_sound(&package.losses, "json-schema", "typescript-7.0")?;
    let members = "#/$defs/Holder/properties/ex:members/anyOf/1/properties/@list/items";
    let expected_losses = [
        (
            "unique-repeated",
            (
                "unique-items-validation-dropped",
                "#/$defs/Holder/properties/ex:unique/anyOf/1/properties/@list/uniqueItems"
                    .to_owned(),
            ),
        ),
        (
            "member-negative",
            ("numeric-validation-dropped", format!("{members}/minimum")),
        ),
    ];
    let mut probes = Vec::new();
    for case in shacl_lists::cases()? {
        let expected_loss = expected_losses
            .iter()
            .find(|(label, _)| *label == case.label)
            .map(|(_, (code, location))| (*code, location.as_str()));
        probes.push(probe(
            &schema,
            &package,
            case.label,
            "Holder",
            case.value,
            "variable",
            case.conforms,
            expected_loss,
        )?);
    }
    Ok(Fixture {
        declaration: declaration(&package)?,
        type_names: package.type_names.clone(),
        losses: ledger_json(&package)?,
        probes,
        compiler_probes: Vec::new(),
        proofs: Vec::new(),
    })
}

/// The temporal range-bound fixture (see `support/shacl_temporal.rs`): a bound
/// is the negation of the values it rejects, and TypeScript has no complement
/// of a type (the lossy fixture's complement proofs); nor can the admitted
/// lexical forms be written positively, as template literal types over digit
/// unions: the dates of four-digit years alone are 8,000,000 members, past the
/// compiler's union limit (this fixture's proof). So every non-conforming probe
/// diverges at its property's dropped negation.
fn temporal_fixture() -> Result<Fixture, Box<dyn Error>> {
    let compiled = shacl_temporal::compiled()?;
    let schema: Value = serde_json::from_str(&compiled.schema_json)?;
    let package = emit_typescript(&compiled, &config()?)?;
    check_ledger_sound(&package.losses, "json-schema", "typescript-7.0")?;
    let mut probes = Vec::new();
    for case in shacl_temporal::cases()? {
        let property = shacl_temporal::VARIANTS
            .iter()
            .find(|(label, _, _)| *label == case.label)
            .map(|(_, property, _)| *property)
            .ok_or("every case is a variant")?;
        let location = format!("#/$defs/Holder/properties/{property}/not");
        let expected_loss =
            (!case.conforms).then_some(("negation-validation-dropped", location.as_str()));
        probes.push(probe(
            &schema,
            &package,
            case.label,
            "Holder",
            case.value,
            "variable",
            case.conforms,
            expected_loss,
        )?);
    }
    Ok(Fixture {
        declaration: declaration(&package)?,
        type_names: package.type_names.clone(),
        losses: ledger_json(&package)?,
        probes,
        compiler_probes: Vec::new(),
        proofs: vec![Proof {
            label: "four-digit-year-dates-exceed-the-union-limit".to_owned(),
            source: "type Digit = \"0\" | \"1\" | \"2\" | \"3\" | \"4\" | \"5\" | \"6\" | \"7\" | \"8\" | \"9\";\n\
                     type Year = `${Digit}${Digit}${Digit}${Digit}`;\n\
                     type MonthDay = `${\"0\" | \"1\"}${Digit}-${\"0\" | \"1\" | \"2\" | \"3\"}${Digit}`;\n\
                     const value: `${Year}-${MonthDay}` = \"2020-03-01\";\n\
                     void value;\n\
                     export {};\n"
                .to_owned(),
            expected_typescript_valid: false,
            expected_code: Some(2590),
        }],
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = config()?;
    let exact_schema = exact_schema();
    let lossy_schema = lossy_schema();
    let exact_package = emit_typescript(&compiled(&exact_schema)?, &config)?;
    let lossy_package = emit_typescript(&compiled(&lossy_schema)?, &config)?;
    let mut reverse_schema = exact_schema.clone();
    let reverse_definitions = reverse_schema["$defs"]
        .as_object_mut()
        .ok_or("oracle schema has no $defs")?;
    reverse_definitions.remove("Empty");
    // The differential fixture's Tree deliberately uses bare JSON field names.
    // They have no caller-supplied RDF property identity, so the verified
    // reverse fixture excludes that definition instead of inventing one.
    reverse_definitions.remove("Tree");
    let reverse_package = emit_typescript(&compiled(&reverse_schema)?, &config)?;
    let reverse = reverse_evidence(&reverse_package, &import_config()?)?;
    let output = json!({
        "exact": exact_fixture(&exact_schema, exact_package)?,
        "lossy": lossy_fixture(&lossy_schema, lossy_package)?,
        "lists": lists_fixture()?,
        "temporal": temporal_fixture()?,
        "reverse": reverse,
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
