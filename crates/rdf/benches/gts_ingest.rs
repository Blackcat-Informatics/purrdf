// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! One-path GTS ingestion, measured three ways through to emitted bytes.
//!
//! `cargo bench -p purrdf-rdf --bench gts_ingest`. Report-only; it asserts no
//! magnitude and no speedup. The three variants are the three surfaces the same
//! ingestion core is reachable through:
//!
//! * `flat` — `materialize()` the composed carrier, then
//!   `SnapshotBuilder::add_dataset_scoped` over the frozen result. This variant
//!   deliberately PAYS for the materialization, because that is the choice a
//!   caller is actually making.
//! * `view` — `add_view_scoped` straight through the composite, freezing nothing.
//! * `delta` — `add_view_scoped` through a carrier whose root is a delta snapshot.
//!
//! The fixture is the keystone shape, taken from `purrdf_rdf::gts_fixtures` —
//! the SAME definition `tests/gts_view_ingestion.rs` measures against, because a
//! bench and a test are separate crates and a second copy of a trap-bearing
//! fixture is a second answer to "what shape is this".
//!
//! Each measured variant prints one `observation` line carrying all four
//! accounting families by name. The byte figures are ACCOUNTED bytes — the
//! retention ledger's own deduplicated retention, this view's incremental charge,
//! and the ingest report's scratch peak. They are not OS resident set size, which
//! criterion cannot measure and this bench does not pretend to.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_rdf::gts_compose::SnapshotBuilder;
use purrdf_rdf::gts_fixtures::{
    SELECTED, emitted, keystone_base, keystone_contribution, keystone_delta, keystone_loadout,
};
use purrdf_rdf::{
    DatasetView, DeltaDatasetView, PipelineViewBundle, RdfDataset, RetentionLedger,
    ViewAccountingReport, ViewLimits,
};

/// Every field name an observation line must carry. The presence guard reads this
/// list, so a renamed field fails the bench instead of silently narrowing the
/// record a reader is relying on.
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

/// The four accounting families one measured variant reports.
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
    /// The carrier's own accounting report, the ingestion's scratch peak, and any
    /// bytes a materialization froze outside the ledger.
    ///
    /// `SnapshotBuilder::ingest_totals` reports `scratch_bytes` as a high-water
    /// figure already, so the sum is a peak of accounted bytes rather than a
    /// running total. `frozen_bytes` is what the `flat` variant's `materialize()`
    /// copied into a dataset the retention ledger never sees: charging it here is
    /// the only way the flat path's memory shows up at all, since `retained_bytes`
    /// is by construction the LEDGER's deduplicated figure and stays comparable
    /// across the three variants.
    fn new(report: &ViewAccountingReport, scratch_bytes: usize, frozen_bytes: usize) -> Self {
        Self {
            peak_accounted_bytes: report
                .total_accounted_bytes()
                .saturating_add(scratch_bytes)
                .saturating_add(frozen_bytes),
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

/// Print one observation line, self-asserting that every named field survived the
/// formatting. This is the acceptance mechanism: it runs in criterion's `--test`
/// smoke mode too, so a shape that stopped reporting fails the bench.
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

// ---------------------------------------------------------------------------
// The shapes and the three ingest surfaces
// ---------------------------------------------------------------------------

/// One measured shape: a base, a delta over it, and the stages folded on in order.
#[derive(Debug)]
struct IngestShape {
    name: &'static str,
    base: Arc<RdfDataset>,
    delta: Arc<DeltaDatasetView>,
    stages: Vec<(String, Arc<RdfDataset>)>,
}

impl IngestShape {
    fn new(name: &'static str, groups: usize, stages: Vec<(String, Arc<RdfDataset>)>) -> Self {
        let base = keystone_base(groups);
        let delta = keystone_delta(&base);
        Self {
            name,
            base,
            delta,
            stages,
        }
    }
}

/// `stage_count` contributions, each in its own graph. `space` maps a stage index
/// onto its term space.
fn stage_plan(
    stage_count: usize,
    rows: usize,
    space: impl Fn(usize) -> usize,
) -> Vec<(String, Arc<RdfDataset>)> {
    (0..stage_count)
        .map(|index| {
            let graph = format!("https://example.org/keystone/stage{index}");
            let quads = keystone_contribution(&graph, space(index), rows);
            (graph, quads)
        })
        .collect()
}

/// The three shapes, each isolating one cost.
///
/// * `issue` — the reported shape: a largish shared base grown by small deltas.
/// * `many_source_k*` — one small source's worth of content appended `k` times,
///   with mutually disjoint terms, so the only thing that grows is the source
///   count the composite read path walks during ingestion.
/// * `high_overlap_k8` — the same `k` and row counts as `many_source_k8`, but
///   every contribution drawn from ONE term space, so the cross-source aliasing
///   the placement rewrite performs is the only difference.
fn ingest_shapes() -> Vec<IngestShape> {
    vec![
        IngestShape::new("issue", 120, stage_plan(4, 6, |index| index + 1)),
        IngestShape::new("many_source_k8", 4, stage_plan(8, 6, |index| index + 1)),
        IngestShape::new("many_source_k32", 4, stage_plan(32, 6, |index| index + 1)),
        IngestShape::new("high_overlap_k8", 4, stage_plan(8, 6, |_| 0)),
    ]
}

/// The composed carrier both the `view` and the `flat` variants read: identical
/// construction, so nothing but the surface chosen can explain a difference.
fn composite_carrier(shape: &IngestShape, limits: ViewLimits) -> PipelineViewBundle<Stage> {
    let (lookaside, blobs, provenance) = keystone_loadout();
    let ledger = RetentionLedger::new();
    let mut carrier = PipelineViewBundle::<Stage>::from_dataset(
        &shape.base,
        lookaside,
        blobs,
        provenance,
        &ledger,
        limits,
    )
    .expect("the base composes within the shape's ceilings");
    for (index, (graph, quads)) in shape.stages.iter().enumerate() {
        carrier
            .accumulate_named_graph(graph.as_str(), quads, Stage(index))
            .expect("a contained contribution appends to the composite carrier");
    }
    carrier
}

/// The same stage plan over a delta snapshot of the base.
fn delta_carrier(shape: &IngestShape, limits: ViewLimits) -> PipelineViewBundle<Stage> {
    let (lookaside, blobs, provenance) = keystone_loadout();
    let ledger = RetentionLedger::new();
    let mut carrier = PipelineViewBundle::<Stage>::from_delta(
        &shape.delta,
        lookaside,
        blobs,
        provenance,
        &ledger,
        limits,
    )
    .expect("the delta composes within the shape's ceilings");
    for (index, (graph, quads)) in shape.stages.iter().enumerate() {
        carrier
            .accumulate_named_graph(graph.as_str(), quads, Stage(index))
            .expect("a contained contribution appends to the delta carrier");
    }
    carrier
}

/// `materialize()` then ingest the frozen result through the flat surface. The
/// freeze is deliberately inside the measurement: it is the price of this choice.
fn run_flat(shape: &IngestShape, limits: ViewLimits) -> (Observation, Vec<u8>) {
    let carrier = composite_carrier(shape, limits);
    let frozen = carrier.materialize().expect("the carrier freezes");
    let mut builder = SnapshotBuilder::new();
    builder
        .add_dataset_scoped(&frozen, Some(SELECTED), Some("base"))
        .expect("the materialized carrier ingests flat");
    let bytes = emitted(&builder);
    let observed = Observation::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        frozen.rdf_payload_bytes(),
    );
    (observed, bytes)
}

/// Ingest straight through the composite, freezing nothing.
fn run_view(shape: &IngestShape, limits: ViewLimits) -> (Observation, Vec<u8>) {
    let carrier = composite_carrier(shape, limits);
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(carrier.view(), Some(SELECTED), Some("base"))
        .expect("the composed carrier ingests");
    let bytes = emitted(&builder);
    // Nothing was frozen to publish this, so no out-of-ledger copy is charged.
    let observed = Observation::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        0,
    );
    (observed, bytes)
}

/// Ingest straight through the delta-rooted composite, compacting nothing.
fn run_delta(shape: &IngestShape, limits: ViewLimits) -> (Observation, Vec<u8>) {
    let carrier = delta_carrier(shape, limits);
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(carrier.view(), Some(SELECTED), Some("base"))
        .expect("the delta carrier ingests");
    let bytes = emitted(&builder);
    // Nothing was frozen to publish this, so no out-of-ledger copy is charged.
    let observed = Observation::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        0,
    );
    (observed, bytes)
}

fn gts_ingest(c: &mut Criterion) {
    // The default ceilings admit every shape here (33 sources at the widest,
    // against `max_sources = 64`), so no ceiling is fabricated for the bench.
    let limits = ViewLimits::default();
    let shapes = ingest_shapes();

    // NON-VACUITY: the issue shape really is the largish, trap-bearing base its
    // name claims, and the two surfaces really are measuring one ingestion — the
    // emitted container bytes are equal, blank wire values included.
    let issue = &shapes[0];
    assert!(
        issue.base.quads().count() > 300,
        "the issue shape's base must be largish: {}",
        issue.base.quads().count()
    );
    assert!(issue.base.reifier_quads().count() >= 8 && issue.base.named_graphs().count() >= 3);
    let (_, flat_bytes) = run_flat(issue, limits);
    let (_, view_bytes) = run_view(issue, limits);
    assert_eq!(
        flat_bytes, view_bytes,
        "the view surface and the materialized flat surface must ship one container"
    );

    // One plain wall measurement per shape/variant, taken OUTSIDE criterion's own
    // timing: criterion's report is criterion's, and the observation line is a
    // separate, plainly-measured record that always emits — `--test` included.
    for shape in &shapes {
        let start = Instant::now();
        let (observed, bytes) = run_flat(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} flat emitted no bytes", shape.name);
        observe(shape.name, "flat", elapsed, observed);

        let start = Instant::now();
        let (observed, bytes) = run_view(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} view emitted no bytes", shape.name);
        observe(shape.name, "view", elapsed, observed);

        let start = Instant::now();
        let (observed, bytes) = run_delta(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} delta emitted no bytes", shape.name);
        observe(shape.name, "delta", elapsed, observed);
    }

    let mut group = c.benchmark_group("gts_ingest");
    for shape in &shapes {
        group.bench_function(BenchmarkId::new("flat", shape.name), |b| {
            b.iter(|| black_box(run_flat(shape, limits)));
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

criterion_group! {
    name = gts_ingest_benches;
    // The machine is shared and contended and these numbers are report-only, so
    // the sample count is small on purpose: precision buys nothing here.
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(2));
    targets = gts_ingest
}
criterion_main!(gts_ingest_benches);
