// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Criterion generates public entry points for this executable only.
#![allow(missing_docs)]

//! Controlled view/publication comparisons using identical deterministic RDF.
//! Native and shared paths are checked before timing. Report-only: no winner is
//! assumed. `profile.bench` inherits release O3, full LTO and one codegen unit.
//!
//! The second group, `carrier_propagation`, puts the flat carrier's union path
//! (`PipelineBundle::accumulate_named_graph`) beside the view carrier's extend
//! path (`PipelineViewBundle::accumulate_named_graph`) over the same stage plan,
//! in three shapes that make three different costs visible. Alongside criterion's
//! own timing each measured variant prints one `observation` line carrying all
//! four accounting families by name — bytes, work, retention and a plain wall
//! clock. Those bytes are ACCOUNTED bytes (the retention ledger's own figures),
//! never OS resident set size, which criterion cannot see and this bench does not
//! pretend to measure. Nothing here asserts a magnitude or a speedup: the machine
//! is not quiet, so read the lines, not their ratios.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_core::{
    BlankScope, CompositeDatasetView, ContentStore, DatasetMut, DatasetProvenance, DatasetView,
    DeltaDatasetView, GraphMatch, MutableDataset, PipelineBundle, PipelineViewBundle, QuadValues,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLookaside, RetentionLedger, TermValue,
    ViewAccountingReport, ViewLimits,
};

fn fixture(rows: u32, payload_bytes: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let literal = builder.intern_literal(RdfLiteral::simple("x".repeat(payload_bytes)));
    let inner = builder.intern_triple(predicate, predicate, literal);
    let outer = builder.intern_triple(predicate, predicate, inner);
    let empty = builder.intern_iri("http://example.org/empty");
    builder.declare_named_graph(empty);
    for index in 0..rows {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        builder.push_quad(subject, predicate, outer, None);
    }
    builder.freeze().unwrap()
}

fn changed(base: Arc<RdfDataset>) -> MutableDataset {
    let mut mutable = MutableDataset::new(base);
    mutable
        .insert(QuadValues::triple(
            TermValue::iri("http://example.org/addition"),
            TermValue::iri("http://example.org/p"),
            TermValue::iri("http://example.org/value"),
        ))
        .unwrap();
    mutable
}

fn scoped_blanks(rows: u32, scope: BlankScope) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..rows {
        let s = builder.intern_blank(&format!("blank{index}"), scope);
        let o = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().unwrap()
}

fn benches(c: &mut Criterion) {
    let base = fixture(10_000, 65_536);
    let contribution = fixture(20, 65_536);
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![base.clone(), contribution.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    let construction = shared.stats();
    let frozen = shared.materialize().unwrap();
    assert_eq!(shared.quads().count(), frozen.quad_count());
    assert_eq!(frozen.quad_count(), base.quad_count());
    assert_eq!(shared.named_graphs().count(), frozen.named_graphs().count());
    let subject = TermValue::iri("http://example.org/s42");
    let shared_subject = shared.term_id_by_value(&subject).unwrap();
    let frozen_subject = frozen.term_id_by_value(&subject).unwrap();
    assert_eq!(
        shared
            .quads_for_pattern(Some(shared_subject), None, None, GraphMatch::Any)
            .count(),
        1
    );
    assert_eq!(
        frozen
            .quads_for_pattern(Some(frozen_subject), None, None, GraphMatch::Any)
            .count(),
        1
    );
    let native_plan = frozen.probe_plan(true, false, false, GraphMatch::Any);
    let composite_plan = shared.probe_plan(true, false, false, GraphMatch::Any);
    assert_eq!(
        frozen
            .quads_for_pattern_with_plan(
                &native_plan,
                Some(frozen_subject),
                None,
                None,
                GraphMatch::Any
            )
            .collect::<Vec<_>>(),
        frozen
            .quads_for_pattern(Some(frozen_subject), None, None, GraphMatch::Any)
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        shared
            .quads_for_pattern_with_plan(
                &composite_plan,
                Some(shared_subject),
                None,
                None,
                GraphMatch::Any
            )
            .collect::<Vec<_>>(),
        shared
            .quads_for_pattern(Some(shared_subject), None, None, GraphMatch::Any)
            .collect::<Vec<_>>(),
    );
    let mutable = changed(base.clone());
    let snapshot = mutable.snapshot_view().unwrap();
    assert_eq!(snapshot.quads().count(), base.quad_count() + 1);
    assert_eq!(snapshot.stats().work.copied_rows, 1);
    eprintln!(
        "shared_views input base_terms={} base_rows={} base_payload={} delta_work={:?} composite={:?}",
        base.term_count(),
        base.rdf_row_count(),
        base.rdf_payload_bytes(),
        snapshot.stats().work,
        construction
    );

    assert!(purrdf_core::datasets_isomorphic(
        &scoped_blanks(16, BlankScope::DEFAULT),
        &scoped_blanks(16, BlankScope(17)),
    ));
    let mut group = c.benchmark_group("shared_views");
    group.bench_function("indexed_native_probe", |b| {
        b.iter(|| {
            black_box(
                frozen
                    .quads_for_pattern(Some(frozen_subject), None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("indexed_composite_probe", |b| {
        b.iter(|| {
            black_box(
                shared
                    .quads_for_pattern(Some(shared_subject), None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("prepared_native_probe", |b| {
        b.iter(|| {
            black_box(
                frozen
                    .quads_for_pattern_with_plan(
                        &native_plan,
                        Some(frozen_subject),
                        None,
                        None,
                        GraphMatch::Any,
                    )
                    .count(),
            )
        });
    });
    group.bench_function("prepared_composite_probe", |b| {
        b.iter(|| {
            black_box(
                shared
                    .quads_for_pattern_with_plan(
                        &composite_plan,
                        Some(shared_subject),
                        None,
                        None,
                        GraphMatch::Any,
                    )
                    .count(),
            )
        });
    });
    group.bench_function("small_delta_snapshot", |b| {
        b.iter(|| black_box(mutable.snapshot_view().unwrap()));
    });
    group.bench_function("small_delta_full_freeze", |b| {
        b.iter(|| black_box(mutable.freeze().unwrap()));
    });
    group.bench_function("compose_shared_nested_payload", |b| {
        b.iter(|| {
            black_box(
                CompositeDatasetView::with_shared_scopes(
                    vec![base.clone(), contribution.clone()],
                    ViewLimits::default(),
                )
                .unwrap(),
            )
        });
    });
    group.bench_function("materialize_shared_nested_payload", |b| {
        b.iter(|| black_box(shared.materialize().unwrap()));
    });
    group.bench_function("retained_snapshot_clone_drop", |b| {
        b.iter(|| black_box(snapshot.clone()));
    });
    group.bench_function("builder_default_blank_scope", |b| {
        b.iter(|| black_box(scoped_blanks(10_000, BlankScope::DEFAULT)));
    });
    group.bench_function("builder_explicit_blank_scope", |b| {
        b.iter(|| black_box(scoped_blanks(10_000, BlankScope(17))));
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Carrier propagation: the flat union path against the view extend path
// ---------------------------------------------------------------------------

/// The base's first quad-bearing named graph — blanks, triple terms and the
/// statement layer live here.
const CARRIER_G1: &str = "http://example.org/carrier/base-1";
/// The base's second quad-bearing named graph.
const CARRIER_G2: &str = "http://example.org/carrier/base-2";
/// Declared by the base and left empty — a leaf with no rows behind it.
const CARRIER_DECLARED: &str = "http://example.org/carrier/declared";

/// Every field name an observation line must carry. The presence guard reads
/// this list, so a renamed field fails the bench instead of silently narrowing
/// the record a reader is relying on.
const OBSERVATION_FIELDS: [&str; 10] = [
    "shape=",
    "variant=",
    "profile=",
    "elapsed_ns=",
    "peak_accounted_bytes=",
    "retained_bytes=",
    "incremental_bytes=",
    "copies=",
    "freezes=",
    "materializations=",
];

/// The typed-handle payload a stage pins. The kernel never reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stage(usize);

/// The four accounting families one measured variant reports, in the units the
/// carrier itself keeps. `peak_accounted_bytes` is the ledger's deduplicated
/// retention plus this view's own incremental charge — an ACCOUNTED figure, not
/// an allocator or RSS measurement.
#[derive(Debug, Clone, Copy, Default)]
struct Account {
    peak_accounted_bytes: usize,
    retained_bytes: usize,
    incremental_bytes: usize,
    copies: usize,
    freezes: usize,
    materializations: usize,
}

impl Account {
    /// The view carrier's own report, read verbatim.
    fn from_report(report: &ViewAccountingReport) -> Self {
        Self {
            peak_accounted_bytes: report.total_accounted_bytes(),
            retained_bytes: report
                .retained
                .retained_payload_bytes
                .saturating_add(report.retained.memo_bytes),
            incremental_bytes: report.incremental_auxiliary_bytes,
            copies: report.incremental_work.copied_rows,
            freezes: report.incremental_work.freezes,
            materializations: report.incremental_work.materializations,
        }
    }
}

/// Print one observation line, self-asserting that every named field survived
/// the formatting. This is the acceptance mechanism: it runs in criterion's
/// `--test` smoke mode too, so a shape that stopped reporting fails the bench.
fn observe(shape: &str, variant: &str, elapsed: Duration, account: Account) {
    let line = format!(
        "observation shape={shape} variant={variant} profile=release \
         elapsed_ns={elapsed} peak_accounted_bytes={peak} retained_bytes={retained} \
         incremental_bytes={incremental} copies={copies} freezes={freezes} \
         materializations={materializations}",
        elapsed = elapsed.as_nanos(),
        peak = account.peak_accounted_bytes,
        retained = account.retained_bytes,
        incremental = account.incremental_bytes,
        copies = account.copies,
        freezes = account.freezes,
        materializations = account.materializations,
    );
    for field in OBSERVATION_FIELDS {
        assert!(
            line.contains(field),
            "the observation line lost {field:?}: {line}"
        );
    }
    println!("{line}");
}

/// A largish deterministic base: two quad-bearing named graphs, default-graph
/// rows, one blank LABEL living in two scopes per group, all three literal
/// shapes, a statement layer every eighth group, and one graph declared and
/// never filled. Every element is a way the two carriers could disagree about
/// what a per-graph digest covers.
fn carrier_base(groups: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let q = b.intern_iri("http://example.org/q");
    let g1 = b.intern_iri(CARRIER_G1);
    let g2 = b.intern_iri(CARRIER_G2);
    let declared = b.intern_iri(CARRIER_DECLARED);
    let plain = b.intern_literal(RdfLiteral::simple("bare"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("cat", "en"));
    for index in 0..groups {
        let s = b.intern_iri(&format!("http://example.org/s{index}"));
        let o = b.intern_iri(&format!("http://example.org/o{index}"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, q, plain, None);
        b.push_quad(s, p, tagged, Some(g1));
        b.push_quad(s, q, o, Some(g2));
        // One local label in two scopes: two nodes a label-keyed identity would
        // conflate, kept structurally distinguishable so canonicalization stays
        // linear rather than exploring automorphisms.
        let shared = b.intern_blank(&format!("n{index}"), BlankScope::DEFAULT);
        let scoped = b.intern_blank(&format!("n{index}"), BlankScope(4));
        b.push_quad(shared, p, o, Some(g1));
        b.push_quad(scoped, q, o, Some(g2));
        if index % 8 == 0 {
            let triple = b.intern_triple(shared, p, tagged);
            let reifier = b.intern_blank(&format!("st{index}"), BlankScope(7));
            b.push_reifier_in_graph(reifier, triple, Some(g1));
            b.push_annotation_in_graph(reifier, q, plain, Some(g1));
        }
    }
    b.declare_named_graph(declared);
    b.freeze().expect("the carrier base freezes")
}

/// A small contribution wholly contained in `graph` — the shape a producing
/// stage folds in as it flows.
///
/// `space` selects the term space: two contributions built at the same `space`
/// share every subject and object (maximum cross-source aliasing), two built at
/// distinct ones share none. Containment is checked by both carriers before
/// anything is folded, so each contribution names exactly one graph — which is
/// also why a shape varies term overlap rather than row overlap.
fn carrier_contribution(graph: &str, space: usize, rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let g = b.intern_iri(graph);
    for index in 0..rows {
        let s = b.intern_iri(&format!("http://example.org/space{space}/s{index}"));
        let o = b.intern_iri(&format!("http://example.org/space{space}/o{index}"));
        b.push_quad(s, p, o, Some(g));
    }
    b.freeze().expect("the carrier contribution freezes")
}

/// A delta whose EFFECTIVE content is exactly `base`'s, reached through a real
/// mutation round trip so the delta machinery is genuinely in the read path.
fn carrier_delta(base: &Arc<RdfDataset>) -> Arc<DeltaDatasetView> {
    let mut mutable = MutableDataset::new(Arc::clone(base));
    let scratch = QuadValues::triple(
        TermValue::iri("http://example.org/scratch"),
        TermValue::iri("http://example.org/p"),
        TermValue::iri("http://example.org/o"),
    );
    assert!(
        mutable
            .insert(scratch.clone())
            .expect("the scratch row inserts")
    );
    assert!(mutable.remove(&scratch), "and is taken back out again");
    Arc::new(mutable.snapshot_view().expect("the delta publishes"))
}

/// One measured shape: a base, the stages folded onto it in order, and how many
/// further readers of the same base follow.
#[derive(Debug)]
struct CarrierShape {
    name: &'static str,
    base: Arc<RdfDataset>,
    delta: Arc<DeltaDatasetView>,
    stages: Vec<(String, Arc<RdfDataset>)>,
    consumers: usize,
}

impl CarrierShape {
    fn new(
        name: &'static str,
        groups: usize,
        stages: Vec<(String, Arc<RdfDataset>)>,
        consumers: usize,
    ) -> Self {
        let base = carrier_base(groups);
        let delta = carrier_delta(&base);
        Self {
            name,
            base,
            delta,
            stages,
            consumers,
        }
    }
}

/// `stage_count` contributions, each in its own graph. `space` maps a stage
/// index onto its term space.
fn stage_plan(
    stage_count: usize,
    rows: usize,
    space: impl Fn(usize) -> usize,
) -> Vec<(String, Arc<RdfDataset>)> {
    (0..stage_count)
        .map(|index| {
            let graph = format!("http://example.org/carrier/stage{index}");
            let quads = carrier_contribution(&graph, space(index), rows);
            (graph, quads)
        })
        .collect()
}

/// The three shapes, each isolating one cost.
///
/// * `issue` — the reported shape: a largish shared base, small deltas folded
///   one at a time, and repeated consumers reading the same base afterwards.
/// * `many_source_k*` — one small source's worth of content appended `k` times,
///   with mutually disjoint terms, so the only thing that grows is the source
///   count the composite read path walks.
/// * `high_overlap_k8` — the same `k` and the same row counts as
///   `many_source_k8`, but every contribution drawn from ONE term space, so the
///   cross-source aliasing the placement rewrite performs is the only difference.
fn carrier_shapes() -> Vec<CarrierShape> {
    vec![
        CarrierShape::new("issue", 150, stage_plan(16, 8, |index| index + 1), 3),
        CarrierShape::new("many_source_k8", 4, stage_plan(8, 8, |index| index + 1), 0),
        CarrierShape::new(
            "many_source_k32",
            4,
            stage_plan(32, 8, |index| index + 1),
            0,
        ),
        CarrierShape::new("high_overlap_k8", 4, stage_plan(8, 8, |_| 0), 0),
    ]
}

/// The FLAT carrier: every stage is an `RdfDataset::union`, which freezes a new
/// dataset and replays every row of both operands into it.
///
/// The flat carrier is deliberately ledger-free and keeps no `ViewWork`, so the
/// union's cost is counted here in the same units the view carrier reports: one
/// freeze per stage, and the rows the union replayed as copies.
fn run_flat(shape: &CarrierShape) -> Account {
    let mut carrier = PipelineBundle::new(
        Arc::clone(&shape.base),
        RdfLookaside::default(),
        Arc::new(ContentStore::new()),
        DatasetProvenance::new(),
    );
    let mut copies = 0usize;
    let mut freezes = 0usize;
    for (index, (graph, quads)) in shape.stages.iter().enumerate() {
        carrier
            .accumulate_named_graph(graph.as_str(), quads, Stage(index))
            .expect("a contained contribution folds into the flat carrier");
        copies = copies.saturating_add(carrier.dataset().rdf_row_count());
        freezes += 1;
    }
    black_box(carrier.pipeline_root());
    for _ in 0..shape.consumers {
        let reader = PipelineBundle::<Stage>::new(
            Arc::clone(&shape.base),
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
        );
        for graph in [CARRIER_G1, CARRIER_G2] {
            black_box(reader.graph_digest(graph));
        }
    }
    let retained = carrier.dataset().rdf_payload_bytes();
    Account {
        peak_accounted_bytes: retained,
        retained_bytes: retained,
        incremental_bytes: 0,
        copies,
        freezes,
        materializations: 0,
    }
}

/// The VIEW carrier: every stage is a `CompositeDatasetView::extend`, copying no
/// row. Each variant gets its OWN ledger so the retention figures belong to this
/// run and nothing else.
fn run_view(shape: &CarrierShape, limits: ViewLimits) -> Account {
    let ledger = RetentionLedger::new();
    let mut carrier = PipelineViewBundle::<Stage>::from_dataset(
        &shape.base,
        RdfLookaside::default(),
        Arc::new(ContentStore::new()),
        DatasetProvenance::new(),
        &ledger,
        limits,
    )
    .expect("the base composes within the shape's ceilings");
    for (index, (graph, quads)) in shape.stages.iter().enumerate() {
        carrier
            .accumulate_named_graph(graph.as_str(), quads, Stage(index))
            .expect("a contained contribution appends to the view carrier");
    }
    black_box(
        carrier
            .pipeline_root()
            .expect("the composed carrier's root canonicalizes"),
    );
    for _ in 0..shape.consumers {
        let reader = PipelineViewBundle::<Stage>::from_dataset(
            &shape.base,
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
            &ledger,
            limits,
        )
        .expect("a second reader of the same base composes");
        for graph in [CARRIER_G1, CARRIER_G2] {
            black_box(
                reader
                    .graph_digest(graph)
                    .expect("a base leaf canonicalizes for the second reader"),
            );
        }
    }
    Account::from_report(&carrier.accounting())
}

/// The DELTA carrier: the same stage plan, but the composed root is a delta
/// snapshot over the base rather than the frozen base itself, so the delta read
/// path is in every canonicalization.
fn run_delta(shape: &CarrierShape, limits: ViewLimits) -> Account {
    let ledger = RetentionLedger::new();
    let mut carrier = PipelineViewBundle::<Stage>::from_delta(
        &shape.delta,
        RdfLookaside::default(),
        Arc::new(ContentStore::new()),
        DatasetProvenance::new(),
        &ledger,
        limits,
    )
    .expect("the delta composes within the shape's ceilings");
    for (index, (graph, quads)) in shape.stages.iter().enumerate() {
        carrier
            .accumulate_named_graph(graph.as_str(), quads, Stage(index))
            .expect("a contained contribution appends to the delta carrier");
    }
    black_box(
        carrier
            .pipeline_root()
            .expect("the delta carrier's root canonicalizes"),
    );
    for _ in 0..shape.consumers {
        let reader = PipelineViewBundle::<Stage>::from_delta(
            &shape.delta,
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
            &ledger,
            limits,
        )
        .expect("a second reader of the same delta composes");
        for graph in [CARRIER_G1, CARRIER_G2] {
            black_box(
                reader
                    .graph_digest(graph)
                    .expect("a base leaf canonicalizes through the delta"),
            );
        }
    }
    Account::from_report(&carrier.accounting())
}

fn carrier_propagation(c: &mut Criterion) {
    // The default ceilings admit every shape here (33 sources at the widest,
    // against `max_sources = 64`), so no ceiling is fabricated for the bench.
    let limits = ViewLimits::default();
    let shapes = carrier_shapes();

    // NON-VACUITY: the shapes really are the sizes their names claim.
    let issue = &shapes[0];
    assert!(
        issue.base.quads().count() > 300 && issue.stages.len() == 16,
        "the issue shape must be a largish base grown by many small stages: {} quads, {} stages",
        issue.base.quads().count(),
        issue.stages.len()
    );
    assert!(issue.base.reifier_quads().count() >= 8);

    // One plain wall measurement per shape/variant, taken OUTSIDE criterion's own
    // timing: criterion's report is criterion's, and the observation line is a
    // separate, plainly-measured record that always emits — `--test` included.
    for shape in &shapes {
        let start = Instant::now();
        let account = run_flat(shape);
        observe(shape.name, "flat", start.elapsed(), account);

        let start = Instant::now();
        let account = run_view(shape, limits);
        observe(shape.name, "view", start.elapsed(), account);

        let start = Instant::now();
        let account = run_delta(shape, limits);
        observe(shape.name, "delta", start.elapsed(), account);
    }

    let mut group = c.benchmark_group("carrier_propagation");
    for shape in &shapes {
        group.bench_function(BenchmarkId::new("flat", shape.name), |b| {
            b.iter(|| black_box(run_flat(shape)));
        });
        group.bench_function(BenchmarkId::new("view", shape.name), |b| {
            b.iter(|| black_box(run_view(shape, limits)));
        });
        group.bench_function(BenchmarkId::new("delta", shape.name), |b| {
            b.iter(|| black_box(run_delta(shape, limits)));
        });
    }
    group.finish();
}

criterion_group!(shared_views, benches);
criterion_group! {
    name = carrier_propagation_benches;
    // The machine is shared and contended and these numbers are report-only, so
    // the sample count is small on purpose: precision buys nothing here.
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(2));
    targets = carrier_propagation
}
criterion_main!(shared_views, carrier_propagation_benches);
