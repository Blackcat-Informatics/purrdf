// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Metadata-heavy shared validation and repeated small delta bindings.
#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf::ir::ViewLimits;
use purrdf::{
    DatasetMut, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue,
};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use std::sync::Arc;

fn data(rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/value");
    let g = b.intern_iri("https://example.org/g");
    let o = b.intern_literal(RdfLiteral::simple("value"));
    for row in 0..rows {
        let s = b.intern_iri(&format!("https://example.org/s{row}"));
        b.push_quad(s, p, o, Some(g));
        b.push_annotation_in_graph(s, p, o, Some(g));
    }
    b.freeze().expect("data")
}
fn small_delta(data: &Arc<RdfDataset>) -> MutableDataset {
    let mut mutation = MutableDataset::new(Arc::clone(data));
    mutation
        .insert(QuadValues {
            s: TermValue::iri("https://example.org/added"),
            p: TermValue::iri("https://example.org/value"),
            o: TermValue::simple_literal("value"),
            g: None,
        })
        .expect("insert");
    mutation
}

fn verify_views(prepared: &PreparedShapes, data: &Arc<RdfDataset>) {
    let owned = prepared
        .bind_dataset(data)
        .expect("owned")
        .validate()
        .expect("report");
    let shared = prepared
        .bind_shared_dataset(Arc::clone(data))
        .expect("shared");
    let shared_report = shared.validate().expect("report");
    assert_eq!(format!("{owned:?}"), format!("{shared_report:?}"));
    assert!(
        shared
            .view_stats()
            .iter()
            .all(|stats| stats.materializations == 0)
    );
    let mutation = small_delta(data);
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    let snapshot_stats = snapshot.stats();
    let delta = prepared
        .bind_delta_with_shapes_graph(snapshot, None, ViewLimits::default())
        .expect("delta");
    let delta_report = delta.validate().expect("report");
    let owned_delta = prepared
        .bind_dataset(&mutation.freeze().expect("freeze"))
        .expect("owned delta")
        .validate()
        .expect("report");
    assert_eq!(format!("{owned_delta:?}"), format!("{delta_report:?}"));
    assert!(
        delta
            .view_stats()
            .iter()
            .all(|stats| stats.materializations == 0)
    );
    println!(
        "SHACL rows={} shared={:?} delta={:?} snapshot={snapshot_stats:?}",
        data.quad_count(),
        shared.view_stats(),
        delta.view_stats()
    );
}

fn bench(c: &mut Criterion) {
    let shapes = Arc::new(parse_shapes("@prefix sh: <http://www.w3.org/ns/shacl#> . <https://example.org/S> a sh:NodeShape; sh:targetSubjectsOf <https://example.org/value>; sh:property [ sh:path <https://example.org/value>; sh:minCount 1; sh:maxCount 1 ] .",None).expect("shapes"));
    let prepared = PreparedShapes::new(shapes);
    let mut group = c.benchmark_group("shacl_shared_carriers");
    for rows in [100, 10_000] {
        let data = data(rows);
        verify_views(&prepared, &data);
        group.bench_with_input(
            BenchmarkId::new("owned_projection_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let validator = prepared.bind_dataset(data).expect("binding");
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("shared_projection_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let validator = prepared
                        .bind_shared_dataset(Arc::clone(data))
                        .expect("binding");
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("delta_owned_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let mutation = small_delta(data);
                    let owned = mutation.freeze().expect("owned boundary");
                    let validator = prepared.bind_dataset(&owned).expect("binding");
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("delta_view_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let mutation = small_delta(data);
                    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
                    let validator = prepared
                        .bind_delta_with_shapes_graph(snapshot, None, ViewLimits::default())
                        .expect("binding");
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
    }
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
