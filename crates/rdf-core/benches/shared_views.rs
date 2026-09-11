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
//!
//! The third group, `pack_output`, measures the OUTPUT conversion on its own —
//! `PackBuilder` over the same shapes — so the stable-cache write is observable
//! apart from the carrier propagation that produced the content. Its flat
//! reference materializes the composed view first and packs the frozen dataset,
//! charging exactly the freeze and materialization the view path avoids, while
//! `view`, `delta` and `selection` pack the view itself. Byte parity between
//! them is asserted once per shape before anything is timed; that assertion is
//! about CONTENT, not speed, and no magnitude is claimed anywhere here either.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_core::{
    BlankScope, CompositeDatasetView, CompositeSource, ContentStore, DatasetMut, DatasetProvenance,
    DatasetView, DeltaDatasetView, GraphMatch, MutableDataset, PackBuilder, PipelineBundle,
    PipelineViewBundle, QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLookaside,
    RetentionLedger, TermValue, ViewAccountingReport, ViewLimits, ViewWork,
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

/// The build profile this run was taken under.
///
/// Derived, never asserted as a literal: a debug-profile figure is not comparable
/// with a release one, and a hardcoded `release` would quietly claim it was. The
/// line is a record, so it has to say which of the two it is.
fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// The four accounting families one measured variant reports, in the units the
/// carrier itself keeps. `peak_accounted_bytes` is the ledger's deduplicated
/// retention plus this view's own incremental charge — an ACCOUNTED figure, not
/// an allocator or RSS measurement.
#[derive(Debug, Clone, Copy, Default)]
struct Observation {
    peak_accounted_bytes: usize,
    retained_bytes: usize,
    incremental_bytes: usize,
    copies: usize,
    freezes: usize,
    materializations: usize,
}

impl Observation {
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

    /// The same report, with the three work figures replaced by what ONE measured
    /// call charged.
    ///
    /// A view's `ViewWork` is cumulative from its construction, so reporting it
    /// verbatim would credit the pack call with the composition's own copies. The
    /// question this group asks is what the OUTPUT conversion costs, so the
    /// counters are read either side of the single call and only the difference
    /// is reported — which is also what makes "the view path froze nothing" a
    /// statement the line can actually carry.
    fn from_report_with_work_delta(
        report: &ViewAccountingReport,
        before: ViewWork,
        after: ViewWork,
    ) -> Self {
        Self {
            copies: after.copied_rows.saturating_sub(before.copied_rows),
            freezes: after.freezes.saturating_sub(before.freezes),
            materializations: after
                .materializations
                .saturating_sub(before.materializations),
            ..Self::from_report(report)
        }
    }
}

/// Print one observation line, self-asserting that every named field survived
/// the formatting. This is the acceptance mechanism: it runs in criterion's
/// `--test` smoke mode too, so a shape that stopped reporting fails the bench.
fn observe(shape: &str, variant: &str, elapsed: Duration, observed: Observation) {
    let line = format!(
        "observation shape={shape} variant={variant} profile={profile} \
         elapsed_ns={elapsed} peak_accounted_bytes={peak} retained_bytes={retained} \
         incremental_bytes={incremental} copies={copies} freezes={freezes} \
         materializations={materializations}",
        profile = build_profile(),
        elapsed = elapsed.as_nanos(),
        peak = observed.peak_accounted_bytes,
        retained = observed.retained_bytes,
        incremental = observed.incremental_bytes,
        copies = observed.copies,
        freezes = observed.freezes,
        materializations = observed.materializations,
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
fn run_flat(shape: &CarrierShape) -> Observation {
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
    Observation {
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
fn run_view(shape: &CarrierShape, limits: ViewLimits) -> Observation {
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
    Observation::from_report(&carrier.accounting())
}

/// The DELTA carrier: the same stage plan, but the composed root is a delta
/// snapshot over the base rather than the frozen base itself, so the delta read
/// path is in every canonicalization.
fn run_delta(shape: &CarrierShape, limits: ViewLimits) -> Observation {
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
    Observation::from_report(&carrier.accounting())
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
        let observed = run_flat(shape);
        observe(shape.name, "flat", start.elapsed(), observed);

        let start = Instant::now();
        let observed = run_view(shape, limits);
        observe(shape.name, "view", start.elapsed(), observed);

        let start = Instant::now();
        let observed = run_delta(shape, limits);
        observe(shape.name, "delta", start.elapsed(), observed);
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

// ---------------------------------------------------------------------------
// Pack output: the stable-cache write, measured apart from carrier propagation
// ---------------------------------------------------------------------------

/// One shape's pack-output variants, composed ONCE.
///
/// Composition is deliberately outside every measured call: this group is about
/// what turning a view into pack bytes costs, and a composite rebuilt inside the
/// timed region would fold the carrier's cost back into the output figure — the
/// exact conflation the group exists to undo.
struct PackShape {
    name: &'static str,
    /// Base + stages, shared-scope. Both `flat` and `view` read this one view:
    /// `flat` materializes it and packs the frozen dataset, `view` packs it in
    /// place, so the two differ by nothing but the freeze.
    composite: Arc<CompositeDatasetView>,
    /// The frozen owners behind `composite`, for this group's own ledger.
    owners: Vec<Arc<RdfDataset>>,
    /// The same effective content reached through the delta read path.
    delta: CompositeDatasetView,
    /// The delta's base and delta halves, plus the same stage contributions.
    delta_owners: Vec<Arc<RdfDataset>>,
    /// A graph SELECTION over `composite`, composed back into a view of its own:
    /// the same retained leaves, projected down to the base's named graphs.
    selection: CompositeDatasetView,
}

impl PackShape {
    /// Compose `shape` three ways over one ceiling set. The default ceilings
    /// admit every shape here, so nothing is fabricated for the bench.
    fn new(shape: &CarrierShape) -> Self {
        let limits = ViewLimits::default();
        let mut owners = vec![Arc::clone(&shape.base)];
        let mut sources = vec![CompositeSource::new(Arc::clone(&shape.base))];
        let mut delta_sources = vec![CompositeSource::from_delta(Arc::clone(&shape.delta))];
        let mut delta_owners = vec![
            Arc::clone(shape.delta.base()),
            Arc::clone(shape.delta.delta()),
        ];
        for (_, quads) in &shape.stages {
            owners.push(Arc::clone(quads));
            delta_owners.push(Arc::clone(quads));
            sources.push(CompositeSource::new(Arc::clone(quads)));
            delta_sources.push(CompositeSource::new(Arc::clone(quads)));
        }
        let composite = Arc::new(
            CompositeDatasetView::from_shared_sources(sources, limits)
                .expect("the base and its stages compose within the default ceilings"),
        );
        let delta = CompositeDatasetView::from_shared_sources(delta_sources, limits)
            .expect("the delta and the same stages compose within the default ceilings");
        // The base's two quad-bearing graphs and the one it declares and never
        // fills: a selection that carries a declaration-only name is the case
        // where the view path and the flat path could most easily disagree about
        // what a pack contains, so it is the one measured.
        let selected = CompositeSource::from_selection(
            Arc::clone(&composite),
            [
                TermValue::iri(CARRIER_G1),
                TermValue::iri(CARRIER_G2),
                TermValue::iri(CARRIER_DECLARED),
            ],
            limits,
        )
        .expect("the composite holds every selected graph");
        let selection = CompositeDatasetView::from_shared_sources(vec![selected], limits)
            .expect("a selection composes like any other source");
        Self {
            name: shape.name,
            composite,
            owners,
            delta,
            delta_owners,
            selection,
        }
    }
}

/// The FLAT output path: materialize the composed view, then pack the frozen
/// dataset. The freeze and the row replay the view path never performs are
/// charged here, in the same units the view variants report.
///
/// Ledger-free on purpose, exactly as the carrier group's flat reference is: a
/// frozen dataset's bytes are its own, so `peak_accounted_bytes` is that
/// dataset's payload and there is no incremental view bookkeeping to add.
fn run_pack_flat(shape: &PackShape) -> Observation {
    let before = shape.composite.stats().work;
    let frozen = shape
        .composite
        .materialize()
        .expect("the composed shape materializes");
    let bytes = PackBuilder::build_bytes(&frozen).expect("the frozen dataset packs");
    let after = shape.composite.stats().work;
    black_box(bytes.len());
    let retained = frozen.rdf_payload_bytes();
    Observation {
        peak_accounted_bytes: retained,
        retained_bytes: retained,
        incremental_bytes: 0,
        copies: after.copied_rows.saturating_sub(before.copied_rows),
        freezes: after.freezes.saturating_sub(before.freezes),
        materializations: after
            .materializations
            .saturating_sub(before.materializations),
    }
}

/// The VIEW output path: pack `view` where it stands. `owners` are the frozen
/// datasets this view keeps resident; each call gets its OWN ledger so the
/// retention figures belong to this run and nothing else.
fn run_pack_view(view: &CompositeDatasetView, owners: &[Arc<RdfDataset>]) -> Observation {
    let ledger = RetentionLedger::new();
    let guards = owners
        .iter()
        .map(|owner| ledger.retain_dataset(owner))
        .collect::<Vec<_>>();
    let before = view.stats().work;
    let bytes = PackBuilder::build_view_bytes(view).expect("the view packs without materializing");
    let after = view.stats().work;
    black_box(bytes.len());
    let observed =
        Observation::from_report_with_work_delta(&ledger.report(&view.stats()), before, after);
    // The guards hold the ledger's registrations open across the report above;
    // dropping them here is what makes that ordering explicit rather than
    // incidental to where the binding happens to fall out of scope.
    drop(guards);
    observed
}

fn pack_output(c: &mut Criterion) {
    let shapes = carrier_shapes()
        .iter()
        .map(PackShape::new)
        .collect::<Vec<_>>();

    // PARITY, not speed. Asserted once per shape, before anything is timed: the
    // three view paths must write the bytes the flat path writes for the same
    // content, or the variants below are timing four different answers. Nothing
    // here compares durations, and no baseline is consulted.
    for shape in &shapes {
        let frozen = shape
            .composite
            .materialize()
            .expect("the composed shape materializes");
        let flat = PackBuilder::build_bytes(&frozen).expect("the frozen dataset packs");
        assert_eq!(
            PackBuilder::build_view_bytes(shape.composite.as_ref()).expect("the composite packs"),
            flat,
            "{}: view and flat pack bytes must be byte-identical",
            shape.name
        );
        assert_eq!(
            PackBuilder::build_view_bytes(&shape.delta).expect("the delta composite packs"),
            flat,
            "{}: delta and flat pack bytes must be byte-identical",
            shape.name
        );
        // The selection's own flat twin: a projection is a different CONTENT, so
        // its parity partner is the frozen dataset of that same projection.
        let selected_frozen = shape
            .selection
            .materialize()
            .expect("the selection materializes");
        let selected_flat =
            PackBuilder::build_bytes(&selected_frozen).expect("the selected dataset packs");
        let selected =
            PackBuilder::build_view_bytes(&shape.selection).expect("the selection packs");
        assert_eq!(
            selected, selected_flat,
            "{}: selection and flat pack bytes must be byte-identical",
            shape.name
        );
        // NON-VACUITY: the selection is a PROPER projection — it carries rows,
        // and it does not carry the default-graph and stage rows the composite
        // holds, so the parity above is not two names for one pack.
        assert!(
            !selected.is_empty() && selected.len() < flat.len(),
            "{}: the selection must be a proper, non-empty projection: {} selected bytes against {} flat",
            shape.name,
            selected.len(),
            flat.len()
        );
    }

    // One plain wall measurement per shape/variant, taken OUTSIDE criterion's own
    // timing and always emitted, `--test` included. The variant values carry a
    // `pack_` prefix: the carrier group reports `flat`/`view`/`delta` over these
    // same shape names, and a reader of one combined stdout must be able to tell
    // a propagation line from an output line.
    for shape in &shapes {
        let start = Instant::now();
        let observed = run_pack_flat(shape);
        observe(shape.name, "pack_flat", start.elapsed(), observed);

        let start = Instant::now();
        let observed = run_pack_view(&shape.composite, &shape.owners);
        observe(shape.name, "pack_view", start.elapsed(), observed);

        let start = Instant::now();
        let observed = run_pack_view(&shape.delta, &shape.delta_owners);
        observe(shape.name, "pack_delta", start.elapsed(), observed);

        let start = Instant::now();
        let observed = run_pack_view(&shape.selection, &shape.owners);
        observe(shape.name, "pack_selection", start.elapsed(), observed);
    }

    let mut group = c.benchmark_group("pack_output");
    for shape in &shapes {
        group.bench_function(BenchmarkId::new("flat", shape.name), |b| {
            b.iter(|| black_box(run_pack_flat(shape)));
        });
        group.bench_function(BenchmarkId::new("view", shape.name), |b| {
            b.iter(|| black_box(run_pack_view(&shape.composite, &shape.owners)));
        });
        group.bench_function(BenchmarkId::new("delta", shape.name), |b| {
            b.iter(|| black_box(run_pack_view(&shape.delta, &shape.delta_owners)));
        });
        group.bench_function(BenchmarkId::new("selection", shape.name), |b| {
            b.iter(|| black_box(run_pack_view(&shape.selection, &shape.owners)));
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
criterion_group! {
    name = pack_output_benches;
    // Report-only and on a contended machine, like the group above it: a small
    // sample is the honest budget, not a precision compromise.
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(2));
    targets = pack_output
}
criterion_main!(
    shared_views,
    carrier_propagation_benches,
    pack_output_benches
);
