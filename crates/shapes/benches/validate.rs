// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Baseline benchmark for the SHACL Core validator (acceleration, Phase 0).
//!
//! Sweeps the whole committed conformance corpus through
//! [`purrdf_shapes::engine::validate_graphs`] — parse data + shapes, resolve focus
//! nodes, run every constraint. This is the end-to-end number Phase 2 (regex /
//! subclass-closure / SPARQL caching) and Phase 4 (focus-node `rayon`) move.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf::loss::LossLedger;
use purrdf_shapes::engine::validate_graphs;
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{
    LinkmlConfig, LinkmlDocument, Namespaces, SchemaDatatypeMap, SchemaImportConfig, emit_linkml,
    import_json_schema, import_linkml,
};
use serde_json::{Map, Value, json};

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: every operation forwards the original pointer/layout to the system
// allocator; thread-local counters are observational only.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + layout.size() as u64));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + new_size as u64));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const IMPORT_CLASSES: usize = 128;
const IMPORT_PROPERTIES_PER_CLASS: usize = 8;
const LINKML: &str = "https://w3id.org/linkml/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const LINKML_EMIT_SIZES: &[usize] = &[32, 1_024, 60_000];

/// Read every `corpus/<case>/{data.nt, shapes.ttl}` pair, sorted by case name.
fn corpus_cases() -> Vec<(String, String, String)> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/corpus"));
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let data = fs::read_to_string(p.join("data.nt"))
                .unwrap_or_else(|e| panic!("{name}: data.nt: {e}"));
            let shapes = fs::read_to_string(p.join("shapes.ttl"))
                .unwrap_or_else(|e| panic!("{name}: shapes.ttl: {e}"));
            (name, data, shapes)
        })
        .collect()
}

fn bench_validate(c: &mut Criterion) {
    let cases = corpus_cases();

    let mut group = c.benchmark_group("shacl_validate");
    group.bench_function("corpus_all", |b| {
        b.iter(|| {
            for (name, data, shapes) in &cases {
                // Panic (don't silently skip) on a validation failure: a swallowed
                // error would run instantly and report a false speedup (gemini review).
                let report = validate_graphs(data, shapes)
                    .unwrap_or_else(|e| panic!("validation failed for {name}: {e:?}"));
                std::hint::black_box(report);
            }
        });
    });
    group.finish();
}

/// Build a 40-class rdfs:subClassOf chain + 3000 typed focus nodes as N-Triples.
///
/// This is the measurement instrument for item 2 (focus-node parallelism).
/// The engine validates focus nodes SERIALLY: a rayon `par_iter` over this 3000-
/// node workload was measured here and regressed ~9% (per-focus work is too cheap
/// — ~5 µs — to amortize thread-pool dispatch and shared-`Store` read contention),
/// confirming. The frozen `RdfDataset` is `Sync`, so the seam stays ready;
/// the parallel path re-enters once per-focus cost exceeds ~50–100 µs.
fn large_hierarchy_inputs() -> (String, String) {
    // Shape: one NodeShape targeting ex:C0 with sh:pattern + sh:minCount constraints.
    // Pattern forces per-node regex evaluation (nontrivial per-focus work).
    let shapes_ttl = r#"
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix ex:   <http://example.org/ns#> .
@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .

ex:HierarchyShape a sh:NodeShape ;
    sh:targetClass ex:C0 ;
    sh:property [
        sh:path ex:label ;
        sh:minCount 1 ;
        sh:pattern "^item-[0-9]+" ;
    ] ;
    sh:property [
        sh:path ex:value ;
        sh:datatype xsd:integer ;
    ] .
"#
    .to_owned();

    // 40-class chain: C39 subClassOf C38 subClassOf … C1 subClassOf C0
    let mut nt = String::with_capacity(1_200_000);
    let ex = "http://example.org/ns#";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let sub_class_of = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    use std::fmt::Write as _;
    for i in 1..40_usize {
        let _ = writeln!(nt, "<{ex}C{i}> <{sub_class_of}> <{ex}C{}>  .", i - 1);
    }

    // 3000 typed nodes: spread across leaf class C39 (all reachable via closure)
    for i in 0..3000_usize {
        let _ = writeln!(nt, "<{ex}item{i}> <{rdf_type}> <{ex}C39> .");
        let _ = writeln!(nt, "<{ex}item{i}> <{ex}label> \"item-{i}\" .");
        let _ = writeln!(
            nt,
            "<{ex}item{i}> <{ex}value> \"{i}\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
        );
    }

    (nt, shapes_ttl)
}

fn bench_validate_large(c: &mut Criterion) {
    let (data_nt, shapes_ttl) = large_hierarchy_inputs();

    let mut group = c.benchmark_group("shacl_validate");
    group.sample_size(20); // Fewer samples: each iteration is ~10–50ms
    group.bench_function("large_hierarchy", |b| {
        b.iter(|| {
            let report = validate_graphs(&data_nt, &shapes_ttl)
                .expect("large_hierarchy: validation must not error");
            std::hint::black_box(report);
        });
    });
    group.finish();
}

fn schema_import_config() -> SchemaImportConfig {
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/bench/".to_owned())],
    )
    .expect("benchmark namespace configuration");
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )
    .expect("benchmark datatype configuration");
    SchemaImportConfig::new(namespaces, datatypes)
}

fn schema_import_fixture() -> String {
    let mut definitions = Map::new();
    for class_idx in 0..IMPORT_CLASSES {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for property_idx in 0..IMPORT_PROPERTIES_PER_CLASS {
            let key = format!("ex:field{property_idx}");
            let schema = match property_idx {
                0 => json!({ "type": "string", "minLength": 1, "maxLength": 96 }),
                1 => json!({ "type": "integer", "minimum": 0, "maximum": 1_000_000 }),
                2 => json!({ "type": "number", "minimum": 0, "maximum": 1_000_000 }),
                3 => json!({ "type": "boolean" }),
                4 => json!({ "type": "string", "pattern": "^[A-Za-z0-9_-]+$" }),
                5 => json!({ "enum": ["open", "closed", "pending"] }),
                6 => json!({
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 8,
                    "uniqueItems": true
                }),
                7 => json!({
                    "$ref": format!(
                        "#/$defs/Class{:03}",
                        (class_idx + IMPORT_CLASSES - 1) % IMPORT_CLASSES
                    )
                }),
                _ => unreachable!("fixed eight-property fixture"),
            };
            properties.insert(key.clone(), schema);
            if property_idx < IMPORT_PROPERTIES_PER_CLASS / 2 {
                required.push(Value::String(key));
            }
        }
        definitions.insert(
            format!("Class{class_idx:03}"),
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }),
        );
    }
    serde_json::to_string(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": definitions
    }))
    .expect("benchmark schema serializes")
}

fn allocation_snapshot() -> (u64, u64) {
    (ALLOCATIONS.with(Cell::get), ALLOCATED_BYTES.with(Cell::get))
}

fn linkml_import_fixture(config: &SchemaImportConfig) -> LinkmlDocument {
    let imported = import_json_schema(&schema_import_fixture(), config)
        .expect("benchmark source schema imports");
    let compiled = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces());
    let linkml_config = LinkmlConfig::new(
        "https://example.org/bench/schema",
        "BenchSchema",
        "Representative LinkML import benchmark fixture.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/bench/".to_owned()),
            ("linkml".to_owned(), LINKML.to_owned()),
        ]),
    )
    .expect("benchmark LinkML configuration");
    emit_linkml(&compiled, &linkml_config)
        .expect("benchmark LinkML fixture emits")
        .document
}

fn bench_schema_import(c: &mut Criterion) {
    let schema = schema_import_fixture();
    let config = schema_import_config();

    let warm = import_json_schema(&schema, &config).expect("benchmark schema imports");
    assert_eq!(warm.shapes.node_shapes.len(), IMPORT_CLASSES);
    drop(warm);

    let before = allocation_snapshot();
    let observed = import_json_schema(&schema, &config).expect("allocation probe imports");
    let after = allocation_snapshot();
    assert_eq!(observed.shapes.node_shapes.len(), IMPORT_CLASSES);
    println!(
        "[shacl_schema_import] classes={IMPORT_CLASSES} properties={} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        after.0 - before.0,
        after.1 - before.1
    );
    black_box(observed);

    let mut group = c.benchmark_group("shacl_schema_import");
    group.sample_size(20);
    group.throughput(Throughput::Elements(
        u64::try_from(IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS).expect("fixture size fits u64"),
    ));
    group.bench_function("json_schema_128_classes_1024_properties", |bencher| {
        bencher.iter(|| {
            let imported = import_json_schema(black_box(&schema), black_box(&config))
                .expect("benchmark schema imports");
            assert_eq!(imported.shapes.node_shapes.len(), IMPORT_CLASSES);
            black_box(imported);
        });
    });
    group.finish();
}

fn bench_linkml_import(c: &mut Criterion) {
    let config = schema_import_config();
    let document = linkml_import_fixture(&config);
    let expected_shapes = document
        .as_value()
        .get("classes")
        .and_then(Value::as_object)
        .map(Map::len)
        .expect("benchmark LinkML fixture has classes");

    let warm = import_linkml(&document, &config).expect("benchmark LinkML imports");
    assert_eq!(warm.shapes.node_shapes.len(), expected_shapes);
    drop(warm);

    let before = allocation_snapshot();
    let observed = import_linkml(&document, &config).expect("allocation probe imports");
    let after = allocation_snapshot();
    assert_eq!(observed.shapes.node_shapes.len(), expected_shapes);
    println!(
        "[shacl_linkml_import] source_classes={IMPORT_CLASSES} source_properties={} imported_shapes={expected_shapes} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        after.0 - before.0,
        after.1 - before.1
    );
    black_box(observed);

    let mut group = c.benchmark_group("shacl_linkml_import");
    group.sample_size(20);
    group.throughput(Throughput::Elements(
        u64::try_from(IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS).expect("fixture size fits u64"),
    ));
    group.bench_function("from_128_class_1024_property_schema", |bencher| {
        bencher.iter(|| {
            let imported = import_linkml(black_box(&document), black_box(&config))
                .expect("benchmark LinkML imports");
            assert_eq!(imported.shapes.node_shapes.len(), expected_shapes);
            black_box(imported);
        });
    });
    group.finish();
}

#[derive(Debug, Clone, Copy)]
enum SlotEmissionMode {
    Safe,
    Rename,
    Collision,
}

impl SlotEmissionMode {
    const ALL: [Self; 3] = [Self::Safe, Self::Rename, Self::Collision];

    const fn label(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Rename => "rename",
            Self::Collision => "collision",
        }
    }
}

fn linkml_emit_fixture(
    slots: usize,
    mode: SlotEmissionMode,
) -> (CompiledSchema, LinkmlConfig, usize, usize) {
    assert!(slots > 0, "benchmark fixture requires slots");
    let mut properties = Map::new();
    let mut required = Vec::new();
    for index in 0..slots {
        let name = match mode {
            SlotEmissionMode::Safe => format!("ex:slot{index:05}"),
            SlotEmissionMode::Rename => format!("ex:slot/{index:05}"),
            SlotEmissionMode::Collision if index == 0 => "ex:collision_".to_owned(),
            SlotEmissionMode::Collision => {
                let scalar = u32::try_from(index).expect("benchmark index fits u32");
                let marker = char::from_u32(0x0f_0000 + scalar)
                    .expect("plane-15 private-use benchmark marker");
                format!("ex:collision{marker}")
            }
        };
        if index % 4 == 0 {
            required.push(Value::String(name.clone()));
        }
        properties.insert(
            name,
            json!({
                "type": "string",
                "pattern": "^[A-Z]"
            }),
        );
    }
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Carrier": {
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }
        }
    });
    let compiled = CompiledSchema {
        schema_json: format!(
            "{}\n",
            serde_json::to_string_pretty(&schema).expect("benchmark schema serializes")
        ),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    };
    let config = LinkmlConfig::new(
        "https://example.org/bench/linkml-emission",
        "LinkmlEmissionBench",
        "Matched safe, rename, and collision-heavy LinkML emission fixture.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/bench/".to_owned()),
            ("linkml".to_owned(), LINKML.to_owned()),
        ]),
    )
    .expect("benchmark LinkML configuration");
    let expected_renames = match mode {
        SlotEmissionMode::Safe => 0,
        SlotEmissionMode::Rename => slots,
        SlotEmissionMode::Collision => slots - 1,
    };
    let expected_collisions = match mode {
        SlotEmissionMode::Collision => slots - 1,
        SlotEmissionMode::Safe | SlotEmissionMode::Rename => 0,
    };
    (compiled, config, expected_renames, expected_collisions)
}

fn bench_linkml_slot_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("linkml_slot_emission");
    group.sample_size(10);
    for &slots in LINKML_EMIT_SIZES {
        for mode in SlotEmissionMode::ALL {
            let (compiled, config, expected_renames, expected_collisions) =
                linkml_emit_fixture(slots, mode);
            let assert_output = |output: &purrdf_shapes::LinkmlPackage| {
                assert_eq!(output.slot_renames.len(), expected_renames);
                assert_eq!(
                    output
                        .slot_renames
                        .iter()
                        .filter(|rename| rename.reasons.iter().any(|reason| {
                            *reason == purrdf_shapes::linkml::LinkmlSlotReason::Collision
                        }))
                        .count(),
                    expected_collisions
                );
            };

            let warm = emit_linkml(&compiled, &config).expect("benchmark fixture emits");
            assert_output(&warm);
            drop(warm);

            let before = allocation_snapshot();
            let observed = emit_linkml(&compiled, &config).expect("allocation probe emits");
            let after = allocation_snapshot();
            assert_output(&observed);
            println!(
                "[linkml_slot_emission] mode={} slots={slots} renames={expected_renames} collisions={expected_collisions} allocations={} allocated_bytes={}",
                mode.label(),
                after.0 - before.0,
                after.1 - before.1
            );
            black_box(observed);

            group.throughput(Throughput::Elements(
                u64::try_from(slots).expect("benchmark slot count fits u64"),
            ));
            group.bench_with_input(
                BenchmarkId::new(mode.label(), slots),
                &slots,
                |bencher, _| {
                    bencher.iter(|| {
                        let output = emit_linkml(black_box(&compiled), black_box(&config))
                            .expect("benchmark fixture emits");
                        assert_output(&output);
                        black_box(output);
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_validate,
    bench_validate_large,
    bench_schema_import,
    bench_linkml_import,
    bench_linkml_slot_emission
);
criterion_main!(benches);
