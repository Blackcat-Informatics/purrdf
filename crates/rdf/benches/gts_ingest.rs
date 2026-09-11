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
//! The fixture is the keystone shape, regenerated here (a bench cannot import a
//! test file): a largish shared base carrying blanks in two scopes, triple terms,
//! the RDF 1.2 statement layer, all three literal shapes and one declared-but-empty
//! graph, grown by small contained contributions.
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
use purrdf_rdf::gts_compose::{DEFAULT_RSYNCABLE_THRESHOLD, MediumPlan, SnapshotBuilder, emit_gts};
use purrdf_rdf::{
    BlankScope, ContentStore, DatasetMut, DatasetProvenance, DatasetView, DeltaDatasetView,
    MutableDataset, PipelineViewBundle, QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfLookaside, RdfTextDirection, RetentionLedger, TermValue, ViewAccountingReport, ViewLimits,
};

/// The named graph every relocated default-graph row lands in.
const SELECTED: &str = "https://example.org/selected";
/// The base's first quad-bearing named graph.
const KEY_G1: &str = "https://example.org/keystone/g1";
/// The base's second quad-bearing named graph.
const KEY_G2: &str = "https://example.org/keystone/g2";
/// Declared by the base and left empty everywhere — the graph the ingest report
/// must NAME rather than drop.
const KEY_DECLARED: &str = "https://example.org/keystone/declared";

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

/// The four accounting families one measured variant reports.
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

// ---------------------------------------------------------------------------
// The keystone fixture shape, regenerated (benches cannot import test files)
// ---------------------------------------------------------------------------

/// A largish shared base: quads across two named graphs and the default graph,
/// one blank LABEL living in two scopes per group, triple terms, reifier
/// declarations, statement annotations, all three literal shapes — and one named
/// graph declared and never filled.
fn keystone_base(groups: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let q = b.intern_iri("https://example.org/q");
    let g1 = b.intern_iri(KEY_G1);
    let g2 = b.intern_iri(KEY_G2);
    let declared = b.intern_iri(KEY_DECLARED);
    let plain = b.intern_literal(RdfLiteral::simple("bare"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("cat", "en"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    for index in 0..groups {
        let s = b.intern_iri(&format!("https://example.org/s{index}"));
        let o = b.intern_iri(&format!("https://example.org/o{index}"));
        // Default-graph rows, relocated to <selected> at ingest time.
        b.push_quad(s, p, o, None);
        b.push_quad(s, q, plain, None);
        // Named-graph rows, one literal shape each.
        b.push_quad(s, p, directional, Some(g1));
        b.push_quad(s, q, tagged, Some(g2));
        // One local label in two scopes, kept structurally distinguishable so
        // canonicalization stays linear rather than exploring automorphisms.
        let shared = b.intern_blank(&format!("n{index}"), BlankScope::DEFAULT);
        let scoped = b.intern_blank(&format!("n{index}"), BlankScope(4));
        b.push_quad(shared, p, o, Some(g1));
        b.push_quad(scoped, q, o, Some(g1));
        if index % 8 == 0 {
            let triple = b.intern_triple(shared, p, directional);
            let reifier = b.intern_blank(&format!("st{index}"), BlankScope(7));
            b.push_reifier_in_graph(reifier, triple, Some(g1));
            b.push_annotation_in_graph(reifier, q, tagged, Some(g1));
        }
    }
    b.declare_named_graph(declared);
    b.freeze().expect("the keystone base freezes")
}

/// A small contribution wholly contained in `graph`, whose blanks deliberately
/// reuse the base's LABEL and scopes.
///
/// `space` selects the term space: two contributions built at the same `space`
/// share every IRI they name, two built at distinct ones share none. Containment
/// is checked before anything is folded, so each contribution names exactly one
/// graph — which is why a shape varies term overlap rather than row overlap.
fn keystone_contribution(graph: &str, space: usize, rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let q = b.intern_iri("https://example.org/q");
    let g = b.intern_iri(graph);
    let node = b.intern_blank("n0", BlankScope::DEFAULT);
    let elsewhere = b.intern_blank("n0", BlankScope(4));
    for index in 0..rows {
        let o = b.intern_iri(&format!("https://example.org/space{space}/c{index}"));
        b.push_quad(node, p, o, Some(g));
        b.push_quad(elsewhere, q, o, Some(g));
    }
    b.push_quad(node, q, elsewhere, Some(g));
    b.freeze().expect("the keystone contribution freezes")
}

/// A delta whose EFFECTIVE content is exactly `base`'s, reached through a real
/// mutation round trip rather than an untouched passthrough — so the delta
/// machinery (suppression rows, delta-only ids) is genuinely in the read path.
fn keystone_delta(base: &Arc<RdfDataset>) -> Arc<DeltaDatasetView> {
    let mut mutable = MutableDataset::new(Arc::clone(base));
    let scratch = QuadValues::triple(
        TermValue::iri("https://example.org/scratch"),
        TermValue::iri("https://example.org/p"),
        TermValue::iri("https://example.org/o"),
    );
    assert!(
        mutable
            .insert(scratch.clone())
            .expect("the scratch row inserts")
    );
    assert!(mutable.remove(&scratch), "and is taken back out again");
    Arc::new(mutable.snapshot_view().expect("the delta publishes"))
}

/// The sidecars every carrier travels with.
fn loadout() -> (RdfLookaside, Arc<ContentStore>, DatasetProvenance) {
    (
        RdfLookaside::default(),
        Arc::new(ContentStore::new()),
        DatasetProvenance::new(),
    )
}

/// The emitted container bytes — what actually ships. `identity` rather than
/// `zstd` so compression is not inside the measurement.
fn emitted(builder: &SnapshotBuilder) -> Vec<u8> {
    emit_gts(
        builder,
        "dist",
        Some(vec!["identity".to_owned()]),
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
        DEFAULT_RSYNCABLE_THRESHOLD,
        &MediumPlan::undicted(None),
    )
    .expect("the carrier emits")
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
    let (lookaside, blobs, provenance) = loadout();
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
    let (lookaside, blobs, provenance) = loadout();
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
fn run_flat(shape: &IngestShape, limits: ViewLimits) -> (Account, Vec<u8>) {
    let carrier = composite_carrier(shape, limits);
    let frozen = carrier.materialize().expect("the carrier freezes");
    let mut builder = SnapshotBuilder::new();
    builder
        .add_dataset_scoped(&frozen, Some(SELECTED), Some("base"))
        .expect("the materialized carrier ingests flat");
    let bytes = emitted(&builder);
    let account = Account::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        frozen.rdf_payload_bytes(),
    );
    (account, bytes)
}

/// Ingest straight through the composite, freezing nothing.
fn run_view(shape: &IngestShape, limits: ViewLimits) -> (Account, Vec<u8>) {
    let carrier = composite_carrier(shape, limits);
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(carrier.view(), Some(SELECTED), Some("base"))
        .expect("the composed carrier ingests");
    let bytes = emitted(&builder);
    // Nothing was frozen to publish this, so no out-of-ledger copy is charged.
    let account = Account::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        0,
    );
    (account, bytes)
}

/// Ingest straight through the delta-rooted composite, compacting nothing.
fn run_delta(shape: &IngestShape, limits: ViewLimits) -> (Account, Vec<u8>) {
    let carrier = delta_carrier(shape, limits);
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(carrier.view(), Some(SELECTED), Some("base"))
        .expect("the delta carrier ingests");
    let bytes = emitted(&builder);
    // Nothing was frozen to publish this, so no out-of-ledger copy is charged.
    let account = Account::new(
        &carrier.accounting(),
        builder.ingest_totals().scratch_bytes,
        0,
    );
    (account, bytes)
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
        let (account, bytes) = run_flat(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} flat emitted no bytes", shape.name);
        observe(shape.name, "flat", elapsed, account);

        let start = Instant::now();
        let (account, bytes) = run_view(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} view emitted no bytes", shape.name);
        observe(shape.name, "view", elapsed, account);

        let start = Instant::now();
        let (account, bytes) = run_delta(shape, limits);
        let elapsed = start.elapsed();
        assert!(!bytes.is_empty(), "{} delta emitted no bytes", shape.name);
        observe(shape.name, "delta", elapsed, account);
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
