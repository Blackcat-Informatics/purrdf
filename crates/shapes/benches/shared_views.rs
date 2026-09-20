// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Metadata-heavy shared validation and repeated small delta bindings.
//!
//! **Report-only. No figure here is asserted, compared against a baseline, or
//! gated on**; the invariants this lane's cost model rests on are executed as
//! tests, in `crates/shapes/tests/change_path_alloc.rs`.
//!
//! # Bind and validate are reported separately
//!
//! `bind_delta_with_shapes_graph` is the repeated-small-delta lane: one
//! preparation, one tiny mutation snapshot, re-bound and re-validated over and
//! over. Its cost model was deliberately changed — work that used to run once per
//! focus node now runs once per bind — and a row that timed `bind(); validate();`
//! as one operation reports that trade as a single number moving slightly, which
//! is exactly the shape in which a move from stage 2 to stage 1 is unreadable.
//!
//! So each carrier is measured twice, in the stage vocabulary
//! `crates/shapes/src/plan.rs` names:
//!
//! * `<carrier>/stage1_bind` — the bind alone, which now carries the hashing;
//! * `<carrier>/stage2_validate` — validation of an already-bound carrier, which
//!   is the per-focus-node work and the number the trade was made to reduce.
//!
//! The pair still sums to the end-to-end cost, so nothing is hidden by the split;
//! what the split adds is the column each half lands in.
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
        // Stage 1: the bind alone. This is the half the change-path work moved
        // INTO, so it is the half that must be readable on its own.
        group.bench_with_input(
            BenchmarkId::new("owned_projection/stage1_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    std::hint::black_box(prepared.bind_dataset(data).expect("binding"));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("shared_projection/stage1_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    std::hint::black_box(
                        prepared
                            .bind_shared_dataset(Arc::clone(data))
                            .expect("binding"),
                    );
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("delta_owned/stage1_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let mutation = small_delta(data);
                    let owned = mutation.freeze().expect("owned boundary");
                    std::hint::black_box(prepared.bind_dataset(&owned).expect("binding"));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("delta_view/stage1_bind", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let mutation = small_delta(data);
                    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
                    std::hint::black_box(
                        prepared
                            .bind_delta_with_shapes_graph(snapshot, None, ViewLimits::default())
                            .expect("binding"),
                    );
                });
            },
        );

        // Stage 2: validation over an ALREADY-BOUND carrier — bound once, outside
        // the timed loop, which is the shape of the repeated-small-delta lane and
        // the half the trade was made to reduce. Summing a carrier's two rows
        // recovers the end-to-end figure the single row used to report.
        let owned_validator = prepared.bind_dataset(&data).expect("binding");
        group.bench_with_input(
            BenchmarkId::new("owned_projection/stage2_validate", rows),
            &owned_validator,
            |b, validator| {
                b.iter(|| {
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        let shared_validator = prepared
            .bind_shared_dataset(Arc::clone(&data))
            .expect("binding");
        group.bench_with_input(
            BenchmarkId::new("shared_projection/stage2_validate", rows),
            &shared_validator,
            |b, validator| {
                b.iter(|| {
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        let owned_delta = small_delta(&data).freeze().expect("owned boundary");
        let delta_owned_validator = prepared.bind_dataset(&owned_delta).expect("binding");
        group.bench_with_input(
            BenchmarkId::new("delta_owned/stage2_validate", rows),
            &delta_owned_validator,
            |b, validator| {
                b.iter(|| {
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
        let snapshot = Arc::new(small_delta(&data).snapshot_view().expect("snapshot"));
        let delta_view_validator = prepared
            .bind_delta_with_shapes_graph(snapshot, None, ViewLimits::default())
            .expect("binding");
        group.bench_with_input(
            BenchmarkId::new("delta_view/stage2_validate", rows),
            &delta_view_validator,
            |b, validator| {
                b.iter(|| {
                    std::hint::black_box(validator.validate().expect("validation"));
                });
            },
        );
    }
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
