// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Compile and validation baselines for `purrdf-jsonschema`.
//!
//! Report-only, `cargo bench -p purrdf-jsonschema --bench validate`.
//!
//! * `from_document/small` — `Schema::from_document` on a small object schema
//!   against one shared meta-schema set. The set's documents and compiled
//!   meta-validator are reused, so this measures scanning, compiling and
//!   meta-validating the one document.
//! * `from_document/ref_chain_256` — the same for a schema whose root reaches a
//!   type through a chain of 256 `$ref`s, each a `$defs` entry.
//! * `metaschemas/build` — building the shared set itself (parsing the nine
//!   draft 2020-12 meta-schema documents and compiling their validator): the
//!   cost `from_document` does not pay per call.
//! * `is_valid/1k` — one compiled schema over 1,000 instances, half invalid.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_jsonschema::{Metaschemas, Schema};
use serde_json::{Value, json};

fn metaschemas() -> Metaschemas {
    Metaschemas::new(
        purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
            .iter()
            .map(|&(uri, text)| (uri, serde_json::from_str::<Value>(text).expect("JSON"))),
    )
    .expect("the draft 2020-12 meta-schemas")
}

fn small() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1},
            "age": {"type": "integer", "minimum": 0},
            "tags": {"type": "array", "items": {"type": "string"}, "uniqueItems": true}
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

fn ref_chain(length: usize) -> Value {
    let mut defs = serde_json::Map::new();
    for index in 0..length {
        defs.insert(
            format!("d{index}"),
            json!({"$ref": format!("#/$defs/d{}", index + 1)}),
        );
    }
    defs.insert(format!("d{length}"), json!({"type": "string"}));
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": defs,
        "$ref": "#/$defs/d0"
    })
}

fn bench_from_document(c: &mut Criterion) {
    let set = metaschemas();
    let mut group = c.benchmark_group("from_document");
    let document = small();
    group.bench_function("small", |b| {
        b.iter(|| {
            Schema::from_document(
                &set,
                "https://example.org/small.json",
                black_box(document.clone()),
            )
            .expect("compiles")
        });
    });
    let chain = ref_chain(256);
    group.bench_function("ref_chain_256", |b| {
        b.iter(|| {
            Schema::from_document(
                &set,
                "https://example.org/chain.json",
                black_box(chain.clone()),
            )
            .expect("compiles")
        });
    });
    group.finish();
}

fn bench_metaschemas(c: &mut Criterion) {
    c.bench_function("metaschemas/build", |b| b.iter(metaschemas));
}

fn bench_is_valid(c: &mut Criterion) {
    let set = metaschemas();
    let schema =
        Schema::from_document(&set, "https://example.org/small.json", small()).expect("compiles");
    let instances: Vec<Value> = (0..1_000)
        .map(|index| {
            if index % 2 == 0 {
                json!({"name": format!("n{index}"), "age": index, "tags": ["a", "b"]})
            } else {
                json!({"name": "", "age": -1, "tags": ["a", "a"], "extra": true})
            }
        })
        .collect();
    let mut group = c.benchmark_group("is_valid");
    group.throughput(Throughput::Elements(instances.len() as u64));
    group.bench_function("1k", |b| {
        b.iter(|| {
            instances
                .iter()
                .filter(|instance| schema.is_valid(black_box(instance)))
                .count()
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_from_document,
    bench_metaschemas,
    bench_is_valid
);
criterion_main!(benches);
