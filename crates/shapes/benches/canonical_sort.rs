// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only benchmark for the SHACL deterministic canonical-sort path.
//!
//! Wall-clock samples from the shared development host are not acceptance
//! evidence. Correctness is guarded by the engine's conformance corpus and the
//! stable-sort regression tests in `crates/shapes/src/engine.rs`; this target
//! exists for controlled-host allocation and profile collection of the
//! deterministic `sort_by_cached_key` canonical sort in `crate::term` (not
//! itself public — reached via the focus-node resolution seam in
//! `crate::engine::resolve_focus_nodes`) and the borrowed data-access surface
//! in `crate::data` (C4) that backs it.
//!
//! [`validate_graphs`] is the same end-to-end entry point `validate.rs` drives,
//! but the fixture here is shaped to MAXIMIZE sort work rather than minimize
//! it: every focus node is unique, inserted in an order that is neither
//! ascending nor descending in its rendered `Term::to_string` form (so
//! `sort_by_cached_key` cannot short-circuit on an already-sorted run), and
//! every focus node produces exactly one violation, so the report's final
//! `(focus_node, component, source_shape, path, value, message, severity)`
//! sort in `crate::engine::validate_with_focus_filter` also sorts one entry
//! per node rather than zero.

use std::fmt::Write as _;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_shapes::engine::validate_graphs;

const NODE_COUNT: usize = 4_000;

/// A `NODE_COUNT`-wide `ex:Widget` population, shuffled into an order that is
/// NOT sorted by rendered IRI (a fixed-stride permutation over the index
/// space), each with an `ex:code` value the shape's `sh:pattern` accepts and
/// NO `ex:requiredLabel` — a guaranteed `sh:minCount` violation per node, so
/// the deterministic sort in [`crate::engine::resolve_focus_nodes`] (target
/// resolution) and the final report sort both see `NODE_COUNT` real keys.
fn large_violation_inputs() -> (String, String) {
    let shapes_ttl = r#"
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix ex:   <http://example.org/ns#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .

ex:WidgetShape a sh:NodeShape ;
    sh:targetClass ex:Widget ;
    sh:property [
        sh:path ex:requiredLabel ;
        sh:minCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path ex:code ;
        sh:pattern "^W-[0-9]+$" ;
    ] .
"#
    .to_owned();

    let ex = "http://example.org/ns#";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    // A fixed-stride permutation coprime with NODE_COUNT: visits every index
    // exactly once but never in ascending (or descending) rendered order, so
    // the cached-key sort actually reorders the input rather than confirming
    // it.
    const STRIDE: usize = 1_777;
    let mut nt = String::with_capacity(NODE_COUNT * 220);
    let mut idx = 0usize;
    for _ in 0..NODE_COUNT {
        let _ = writeln!(nt, "<{ex}widget-{idx}> <{rdf_type}> <{ex}Widget> .");
        let _ = writeln!(nt, "<{ex}widget-{idx}> <{ex}code> \"W-{idx}\" .");
        idx = (idx + STRIDE) % NODE_COUNT;
    }

    (nt, shapes_ttl)
}

fn bench_canonical_sort(c: &mut Criterion) {
    let (data_nt, shapes_ttl) = large_violation_inputs();

    let mut group = c.benchmark_group("shacl_canonical_sort");
    group.sample_size(20); // Fewer samples: each iteration parses + validates 4000 nodes.
    group.throughput(Throughput::Elements(
        u64::try_from(NODE_COUNT).expect("fixture size fits u64"),
    ));
    group.bench_function("violations_4000_unsorted_focus", |b| {
        b.iter(|| {
            let report = validate_graphs(black_box(&data_nt), black_box(&shapes_ttl))
                .expect("large_violation_inputs: validation must not error");
            // Every node lacks ex:requiredLabel, so every focus node must
            // produce exactly one violation — otherwise the fixture isn't
            // exercising the sort path this bench targets.
            assert_eq!(report.results.len(), NODE_COUNT);
            black_box(report);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_canonical_sort);
criterion_main!(benches);
