// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use purrdf::ir::{CompositeDatasetView, CompositeSource, GraphPlacement, ViewLimits};
use purrdf::{
    DatasetMut, DatasetView, GraphMatch, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, TermId, TermValue,
};
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main};
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

fn bench(c: &mut Bench) {
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
const SELECTED_GRAPH: &str = "https://example.org/selected";

fn named_graph_data(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/value");
    let rdf_type = builder.intern_iri(purrdf_iri::vocab::rdf::TYPE);
    let subclass = builder.intern_iri(purrdf_iri::vocab::rdfs::SUB_CLASS_OF);
    let child = builder.intern_iri("https://example.org/Child");
    let parent = builder.intern_iri("https://example.org/Parent");
    let selected = builder.intern_iri(SELECTED_GRAPH);
    let sibling = builder.intern_iri("https://example.org/sibling");
    let good = builder.intern_literal(RdfLiteral::simple("good"));
    let bad = builder.intern_literal(RdfLiteral::simple("bad"));
    builder.push_quad(child, subclass, parent, Some(selected));
    for row in 0..rows {
        let subject = builder.intern_iri(&format!("https://example.org/subject{row}"));
        builder.push_quad(subject, rdf_type, child, Some(selected));
        builder.push_quad(subject, predicate, good, Some(selected));
        builder.push_quad(subject, predicate, bad, Some(sibling));
        builder.push_annotation_in_graph(subject, predicate, bad, Some(sibling));
        builder.push_quad(subject, predicate, bad, None);
    }
    builder.freeze().expect("named-graph data")
}

fn composite_selection(source: &Arc<RdfDataset>) -> ShaclDatasetView {
    let limits = ViewLimits::default();
    let source = Arc::new(
        CompositeDatasetView::with_shared_scopes(vec![Arc::clone(source)], limits)
            .expect("native composite"),
    );
    let selection =
        CompositeSource::from_selection(source, [TermValue::iri(SELECTED_GRAPH)], limits)
            .expect("selection")
            .with_graph_placement(GraphPlacement::Default);
    let composite = Arc::new(
        CompositeDatasetView::from_shared_sources(vec![selection], limits)
            .expect("selected composite"),
    );
    ShaclDatasetView::composite(composite, true, limits).expect("selected validation view")
}

fn named_graph_bench(c: &mut Bench) {
    let shapes = Arc::new(parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . <https://example.org/Shape> a sh:NodeShape; sh:targetClass <https://example.org/Parent>; sh:property [ sh:path <https://example.org/value>; sh:minCount 1; sh:maxCount 1 ] .",
        None,
    ).expect("named-graph shapes"));
    let prepared = PreparedShapes::new(shapes);
    let mut group = c.benchmark_group("shacl_named_graph");
    for rows in [100, 10_000] {
        let source = named_graph_data(rows);
        let native = Arc::new(source.project_named_graph(SELECTED_GRAPH));
        let views = [
            ("native_owned", Arc::new(ShaclDatasetView::native(native))),
            (
                "composite_selection",
                Arc::new(composite_selection(&source)),
            ),
            (
                "native_selection",
                Arc::new(ShaclDatasetView::named_graph(
                    Arc::clone(&source),
                    source
                        .term_id_by_iri(SELECTED_GRAPH)
                        .expect("selected graph"),
                )),
            ),
        ];
        let expected = prepared
            .bind_view(Arc::clone(&views[0].1))
            .expect("native binding")
            .validate()
            .expect("native report")
            .to_ntriples();
        for (name, view) in &views {
            let validator = prepared.bind_view(Arc::clone(view)).expect("binding");
            assert_eq!(
                validator.validate().expect("report").to_ntriples(),
                expected
            );
            assert_eq!(view.stats().materializations, 0);
            group.bench_with_input(
                BenchmarkId::new(format!("{name}/stage1_bind"), rows),
                view,
                |b, view| {
                    b.iter(|| {
                        std::hint::black_box(prepared.bind_view(Arc::clone(view)).expect("binding"))
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("{name}/stage2_validate"), rows),
                &validator,
                |b, validator| {
                    b.iter(|| std::hint::black_box(validator.validate().expect("report")));
                },
            );
            let predicate = view
                .term_id_by_value(&TermValue::iri("https://example.org/value"))
                .expect("lookup")
                .expect("predicate");
            let subjects: Vec<TermId> = (0..rows)
                .map(|row| {
                    view.term_id_by_value(&TermValue::iri(format!(
                        "https://example.org/subject{row}"
                    )))
                    .expect("lookup")
                    .expect("subject")
                })
                .collect();
            let plan = view.probe_plan(true, true, false, GraphMatch::Default);
            group.bench_with_input(
                BenchmarkId::new(format!("{name}/indexed_probes"), rows),
                &subjects,
                |b, subjects| {
                    b.iter(|| {
                        for &subject in subjects {
                            std::hint::black_box(
                                view.quads_for_pattern_with_plan(
                                    &plan,
                                    Some(subject),
                                    Some(predicate),
                                    None,
                                    GraphMatch::Default,
                                )
                                .count(),
                            );
                        }
                    });
                },
            );
        }
        let graph = source
            .term_id_by_iri(SELECTED_GRAPH)
            .expect("selected graph");
        group.bench_with_input(
            BenchmarkId::new("native_selection/construct", rows),
            &source,
            |b, source| {
                b.iter(|| {
                    std::hint::black_box(ShaclDatasetView::named_graph(Arc::clone(source), graph))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("owned_projection/construct", rows),
            &source,
            |b, source| {
                b.iter(|| std::hint::black_box(source.project_named_graph(SELECTED_GRAPH)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("composite_selection/construct", rows),
            &source,
            |b, source| {
                b.iter(|| std::hint::black_box(composite_selection(source)));
            },
        );
    }
    group.finish();
}

bench_group!(benches, bench, named_graph_bench);
bench_main!(benches);
