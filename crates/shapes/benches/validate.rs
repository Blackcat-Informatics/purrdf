// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Baseline benchmark for the SHACL Core validator (acceleration, Phase 0).
//!
//! Sweeps the whole committed conformance corpus through
//! [`purrdf_shapes::engine::validate_graphs`] — parse data + shapes, resolve focus
//! nodes, run every constraint. This is the end-to-end number Phase 2 (regex /
//! subclass-closure / SPARQL caching) and Phase 4 (focus-node `rayon`) move.

use std::fs;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_shapes::engine::validate_graphs;

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

criterion_group!(benches, bench_validate, bench_validate_large);
criterion_main!(benches);
