// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit exact and lossy packages for the dev-only TypeScript compiler oracle.

use std::collections::BTreeSet;
use std::error::Error;

use boon::{Compiler, Schemas};
use purrdf::loss::{LossLedger, check_ledger_complete, check_ledger_sound};
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{
    TYPESCRIPT_DECLARATION_PATH, TypeScriptConfig, TypeScriptPackage, emit_typescript,
};
use serde::Serialize;
use serde_json::{Value, json};

const CLOSED_PROFILE: [&str; 18] = [
    "additional-properties-validation-widened",
    "array-cardinality-validation-widened",
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
    "tuple-array-validation-widened",
    "unevaluated-validation-dropped",
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    declaration: String,
    type_names: std::collections::BTreeMap<String, String>,
    losses: Value,
    probes: Vec<Probe>,
    compiler_probes: Vec<CompilerProbe>,
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

fn lossy_schema() -> Value {
    let long_prefix = (0..33)
        .map(|index| {
            if index % 2 == 0 {
                json!({ "type": "string" })
            } else {
                json!({ "type": "number" })
            }
        })
        .collect::<Vec<_>>();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.org/schema/typescript-lossy.json",
        "$defs": {
            "Cardinality": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 40
            },
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
            "LongTuple": {
                "type": "array",
                "prefixItems": long_prefix,
                "items": false
            },
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
    })
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
            "cardinality-minimum",
            "Cardinality",
            json!([]),
            "variable",
            false,
            Some((
                "array-cardinality-validation-widened",
                "#/$defs/Cardinality/minItems",
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
            "long-tuple-position",
            "LongTuple",
            json!([7]),
            "variable",
            false,
            Some((
                "tuple-array-validation-widened",
                "#/$defs/LongTuple/prefixItems",
            )),
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
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = config()?;
    let exact_schema = exact_schema();
    let lossy_schema = lossy_schema();
    let exact_package = emit_typescript(&compiled(&exact_schema)?, &config)?;
    let lossy_package = emit_typescript(&compiled(&lossy_schema)?, &config)?;
    let output = json!({
        "exact": exact_fixture(&exact_schema, exact_package)?,
        "lossy": lossy_fixture(&lossy_schema, lossy_package)?,
    });
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
