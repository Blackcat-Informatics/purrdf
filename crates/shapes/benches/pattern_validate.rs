// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Per-constraint cost of the SHACL `sh:pattern` arm's `OnceLock` cache.
//!
//! This branch replaced the per-evaluation `purrdf_core::xsd_regex::compile`
//! with a `OnceLock` held on the parsed [`Constraint`](purrdf_shapes::shapes),
//! so a `sh:pattern` is translated and built at most once per constraint
//! instance no matter how many focus nodes or value nodes the shape sees. This
//! target pins that behaviour by measurement:
//!
//! * `cold_constraint` — the shapes are parsed fresh for every iteration (the
//!   parse is untimed `iter_batched` setup), so the constraint's `OnceLock` is
//!   empty when the timed validation begins and its first `get_or_init` runs
//!   the full translation + build. This is the one-time cost the cache exists
//!   to amortise.
//! * `warm_constraint` — the shapes are parsed once and validated once outside
//!   the timed loop, so every timed validation only matches. Comparing the two
//!   isolates the compile from the match.
//!
//! Both are parameterized by focus-node count and values per focus, because the
//! cache's whole claim is that cost is independent of how many value nodes it
//! serves.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench pattern_validate` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use std::sync::Arc;

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_shapes::engine::{parse_shapes, validate_projected_dataset};

const BENCH_EX: &str = "https://example.org/shacl-pattern/";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// `(focus nodes, values per focus)` combinations. The cross of a wide-but-shallow
/// population and a narrow-but-deep one is what shows the `OnceLock` cost is per
/// constraint, not per value node.
const CASES: &[(usize, usize)] = &[(64, 1), (64, 8), (512, 1), (512, 8)];

/// A frozen dataset of `focus_nodes` `ex:Widget` subjects, each with
/// `values_per_focus` `ex:code` literals that all satisfy the shape's
/// `sh:pattern`, plus the shapes source text.
fn pattern_fixture(focus_nodes: usize, values_per_focus: usize) -> (Arc<RdfDataset>, String) {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let widget = builder.intern_iri(&format!("{BENCH_EX}Widget"));
    let code = builder.intern_iri(&format!("{BENCH_EX}code"));
    for focus in 0..focus_nodes {
        let subject = builder.intern_iri(&format!("{BENCH_EX}widget-{focus}"));
        builder.push_quad(subject, rdf_type, widget, None);
        for value in 0..values_per_focus {
            let literal = builder.intern_literal(RdfLiteral::simple(format!("W-{focus}-{value}")));
            builder.push_quad(subject, code, literal, None);
        }
    }
    let dataset = builder.freeze().expect("freeze SHACL pattern fixture");

    let shapes = format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:WidgetShape a sh:NodeShape ;
    sh:targetClass ex:Widget ;
    sh:property [
        sh:path ex:code ;
        sh:pattern "^W-[0-9]+-[0-9]+$" ;
    ] .
"#
    );
    (dataset, shapes)
}

fn bench_pattern_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_pattern_validate");
    group.sample_size(10);

    for &(focus_nodes, values_per_focus) in CASES {
        let (dataset, shapes_ttl) = pattern_fixture(focus_nodes, values_per_focus);
        let label = format!("{focus_nodes}x{values_per_focus}");
        let values = focus_nodes * values_per_focus;
        group.throughput(Throughput::Elements(
            u64::try_from(values).expect("fixture size fits u64"),
        ));

        // Untimed sanity: the fixture must conform, or the timed region would
        // be measuring the (cheaper) violation path instead of matching.
        let shapes = parse_shapes(&shapes_ttl, None).expect("SHACL pattern shapes parse");
        let report = validate_projected_dataset(Arc::clone(&dataset), &shapes)
            .expect("SHACL pattern fixture validates");
        assert!(
            report.conforms,
            "SHACL pattern fixture must conform for {label}"
        );

        // COLD: parse the shapes inside the untimed setup so the timed
        // validation's first `OnceLock::get_or_init` pays the compile.
        let cold_dataset = Arc::clone(&dataset);
        let cold_ttl = &shapes_ttl;
        group.bench_function(BenchmarkId::new("cold_constraint", &label), |bencher| {
            bencher.iter_batched(
                || parse_shapes(cold_ttl, None).expect("SHACL pattern shapes parse"),
                |shapes| {
                    let report = validate_projected_dataset(Arc::clone(&cold_dataset), &shapes)
                        .expect("SHACL pattern fixture validates");
                    assert!(report.conforms);
                    black_box(report);
                },
                BatchSize::SmallInput,
            );
        });

        // WARM: the `OnceLock` was initialized by the sanity validation above,
        // so the timed loop measures matching only.
        let warm_dataset = Arc::clone(&dataset);
        group.bench_function(BenchmarkId::new("warm_constraint", &label), |bencher| {
            bencher.iter(|| {
                let report =
                    validate_projected_dataset(Arc::clone(&warm_dataset), black_box(&shapes))
                        .expect("SHACL pattern fixture validates");
                assert!(report.conforms);
                black_box(report);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pattern_validate);
criterion_main!(benches);
