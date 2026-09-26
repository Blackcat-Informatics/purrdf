// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! JSON Schema compile and validation benchmark.
//!
//! Report-only, `cargo bench -p purrdf-jsonschema --bench validate`.
//!
//! * `jsonschema_from_document/small` compiles a six-keyword object schema
//!   through `Schema::from_document`, the one-call path every caller with a
//!   self-contained schema takes. Each call builds a fresh `Registry`, so the
//!   cost of making the vendored meta-schemas available to it is on this path,
//!   along with the compile and the meta-schema check of the document itself.
//! * `jsonschema_from_document/ref_chain` compiles a 200-definition schema
//!   whose definitions reach one another through `$ref`, so resolution, the
//!   work queue and the meta-schema check of a large document dominate.
//! * `jsonschema_validate/instances_1k` validates 1,000 generated instances
//!   against one compiled record schema (`type`, `required`, `properties`,
//!   `pattern`, `enum`, bounds, `items`, `additionalProperties`), half of them
//!   valid, so the evaluator's keyword loop is the measured cost.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_jsonschema::Schema;
use serde_json::{Value, json};

/// The number of `$defs` entries in the `$ref`-chain schema.
const CHAIN: usize = 200;

/// The number of instances the validation bench evaluates per iteration.
const INSTANCES: usize = 1_000;

fn small_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["id", "name"],
        "properties": {
            "id": { "type": "integer", "minimum": 0 },
            "name": { "type": "string", "minLength": 1 }
        },
        "additionalProperties": false
    })
}

/// `$defs` `d0` … `d199`: each is an object whose `next` refers to the
/// following definition and whose `value` refers to a shared leaf, and the
/// root refers to `d0`, so every definition is reached through a `$ref`.
fn ref_chain_schema() -> Value {
    let mut defs = serde_json::Map::new();
    for index in 0..CHAIN {
        let next = if index + 1 < CHAIN {
            json!({ "$ref": format!("#/$defs/d{}", index + 1) })
        } else {
            json!({ "type": "null" })
        };
        defs.insert(
            format!("d{index}"),
            json!({
                "type": "object",
                "properties": {
                    "value": { "$ref": "#/$defs/leaf" },
                    "next": next
                },
                "required": ["value"]
            }),
        );
    }
    defs.insert(
        "leaf".to_owned(),
        json!({ "type": "string", "pattern": "^[a-z]+$", "maxLength": 32 }),
    );
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": "#/$defs/d0",
        "$defs": Value::Object(defs)
    })
}

fn record_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["id", "name", "tags", "status"],
        "properties": {
            "id": { "type": "integer", "minimum": 1, "maximum": 1_000_000 },
            "name": { "type": "string", "pattern": "^[A-Z][a-z]+$" },
            "tags": {
                "type": "array",
                "items": { "type": "string", "minLength": 1 },
                "uniqueItems": true,
                "maxItems": 8
            },
            "status": { "enum": ["active", "retired", "pending"] },
            "score": { "type": "number", "exclusiveMinimum": 0 }
        },
        "additionalProperties": false
    })
}

/// Deterministic instances: even indices satisfy [`record_schema`], odd ones
/// break one keyword each, in rotation.
fn instances() -> Vec<Value> {
    const NAMES: [&str; 4] = ["Alpha", "Bravo", "Charlie", "Delta"];
    const STATUS: [&str; 3] = ["active", "retired", "pending"];
    (0..INSTANCES)
        .map(|index| {
            let mut instance = json!({
                "id": index + 1,
                "name": NAMES[index % NAMES.len()],
                "tags": ["a", "b", format!("t{index}")],
                "status": STATUS[index % STATUS.len()],
                "score": 0.5 + f64::from(u32::try_from(index).unwrap_or(u32::MAX))
            });
            if index % 2 == 1 {
                match (index / 2) % 5 {
                    0 => instance["id"] = json!(0),
                    1 => instance["name"] = json!("lower"),
                    2 => instance["tags"] = json!(["a", "a"]),
                    3 => instance["status"] = json!("unknown"),
                    _ => instance["extra"] = json!(true),
                }
            }
            instance
        })
        .collect()
}

fn bench_from_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonschema_from_document");
    let small = small_schema();
    group.bench_function("small", |bencher| {
        bencher.iter(|| {
            Schema::from_document("https://example.org/small", black_box(small.clone()))
                .expect("the small schema compiles")
        });
    });
    let chain = ref_chain_schema();
    group.bench_function("ref_chain", |bencher| {
        bencher.iter(|| {
            Schema::from_document("https://example.org/chain", black_box(chain.clone()))
                .expect("the $ref-chain schema compiles")
        });
    });
    group.finish();
}

fn bench_validate(c: &mut Criterion) {
    let schema = Schema::from_document("https://example.org/record", record_schema())
        .expect("the record schema compiles");
    let instances = instances();
    let valid = instances
        .iter()
        .filter(|instance| schema.is_valid(instance))
        .count();
    assert_eq!(valid, INSTANCES / 2, "half the instances are valid");
    let mut group = c.benchmark_group("jsonschema_validate");
    group.throughput(Throughput::Elements(INSTANCES as u64));
    group.bench_function("instances_1k", |bencher| {
        bencher.iter(|| {
            black_box(&instances)
                .iter()
                .filter(|instance| schema.is_valid(instance))
                .count()
        });
    });
    group.finish();
}

criterion_group!(benches, bench_from_document, bench_validate);
criterion_main!(benches);
