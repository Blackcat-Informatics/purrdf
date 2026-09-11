// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-path GTS ingestion: the view surface and the flat carrier surface are the
//! SAME ingestion, so they must produce the same bytes.
//!
//! `SnapshotBuilder::add_dataset_scoped` is now a delegation through
//! `add_view_scoped` over the frozen carrier's own `DatasetView` impl. That
//! collapses two definitions of "ingest a dataset" into one, and this suite is
//! what holds the collapse honest:
//!
//! * **Golden parity.** One trap fixture — implicit `xsd:string`, `@en`, an
//!   `ar`/`rtl` directional literal, two sources spelling the same blank label
//!   under different ingest scopes, a reifier with annotations, a graph declared
//!   and left empty, a graph declaration-only in one source and quad-bearing in
//!   another, and default-graph relocation — ingested three ways (flat carrier,
//!   a composite view over the same content, a delta view over the same content)
//!   must mint one `snapshot_content_id` and one term table.
//! * **Refusals, each with its neighbouring valid twin.** The capability gate,
//!   the two operational checkpoints, the plain-slot guard, the blank wire-value
//!   injectivity guard and the terminal poison flag are all refusals, and a
//!   refusal that also rejects valid input is the mirror of a silent drop.
//! * **Accounting.** The report names the declaration-only graphs that were
//!   deliberately NOT interned, and its scratch figure is non-zero and monotone.

use std::cell::Cell;
use std::sync::Arc;

use purrdf_rdf::dataset_view::ViewOperationStatus;
use purrdf_rdf::gts_compose::{
    DEFAULT_RSYNCABLE_THRESHOLD, GtsIngestError, IngestCheckpoint, MediumPlan, RDF_REIFIES,
    SnapshotBuilder, emit_gts,
};
use purrdf_rdf::{
    CompositeDatasetView, CompositeSource, DatasetView, FallibleDatasetView, GraphMatch,
    MutableDataset, QuadIds, QuadRef, RdfDataset, RdfDatasetBuilder, RdfStoreCapabilities, TermId,
    TermRef, TermValue, ViewLimits, parse_dataset,
};

/// The named graph every relocated default-graph row lands in.
const SELECTED: &str = "https://example.org/selected";
/// Declared in source A and left empty; declared AND filled in source B.
const SHARED: &str = "https://example.org/shared";
/// Declared in source A and left empty everywhere.
const EMPTY_A: &str = "https://example.org/empty-a";

// ---------------------------------------------------------------------------
// The trap fixture
// ---------------------------------------------------------------------------

/// Re-freeze a parsed dataset with `declare` added as DECLARATION-ONLY named
/// graphs. TriG has no syntax for an empty graph, so the declaration is made on
/// the builder — which is also where a real declaration-tracking backend makes
/// it. Both fixtures go through this, so both carry their blanks at the same
/// merge scope and the ingest scope is the only thing keeping them apart.
fn with_declarations(parsed: &RdfDataset, declare: &[&str]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(parsed);
    for iri in declare {
        let graph = builder.intern_iri(iri);
        builder.declare_named_graph(graph);
    }
    builder.freeze().expect("the declared dataset freezes")
}

/// The snapshot wire value an ingest `scope` gives this dataset's blank node.
fn blank_wire(dataset: &RdfDataset, scope: &str) -> String {
    let label = dataset
        .quads()
        .flat_map(|q| [q.s, q.o])
        .find_map(|id| match dataset.resolve(id) {
            TermRef::Blank {
                label,
                scope: blank_scope,
            } => Some(blank_scope.qualify_label(label).into_owned()),
            _ => None,
        })
        .expect("the fixture holds a blank node");
    format!("{scope}-{label}")
}

/// Source A: every literal shape, a blank node, a reifier with two annotations
/// across two graphs, and two declaration-only graphs.
fn source_a() -> Arc<RdfDataset> {
    let parsed = parse_dataset(
        format!(
            r#"
            {{
                <https://example.org/s> <https://example.org/bare> "bare" ;
                    <https://example.org/typed>
                        "x"^^<http://www.w3.org/2001/XMLSchema#string> ;
                    <https://example.org/tagged> "cat"@en ;
                    <https://example.org/dir> "مرحبا"@ar--rtl .
                _:b0 <https://example.org/mentions> <https://example.org/s> .
                <https://example.org/claim> <{RDF_REIFIES}>
                        <<( <https://example.org/s> <https://example.org/bare> "bare" )>> ;
                    <https://example.org/accordingTo> <https://example.org/who> .
            }}
            GRAPH <https://example.org/named-a> {{
                _:b0 <https://example.org/in> <https://example.org/g> .
                <https://example.org/c2> <{RDF_REIFIES}>
                        <<( _:b0 <https://example.org/in> <https://example.org/g> )>> ;
                    <https://example.org/confidence>
                        "0.9"^^<http://www.w3.org/2001/XMLSchema#decimal> .
            }}
            "#
        )
        .as_bytes(),
        "application/trig",
        None,
    )
    .expect("source A parses");
    with_declarations(&parsed, &[EMPTY_A, SHARED])
}

/// Source B: the SAME local blank label `_:b0` (a different node), plus content
/// for the graph source A only declared.
fn source_b() -> Arc<RdfDataset> {
    let parsed = parse_dataset(
        format!(
            r"
            GRAPH <{SHARED}> {{ <https://example.org/t> <https://example.org/p>
                <https://example.org/q> . }}
            {{ _:b0 <https://example.org/from> <https://example.org/b> . }}
            "
        )
        .as_bytes(),
        "application/trig",
        None,
    )
    .expect("source B parses");
    with_declarations(&parsed, &[])
}

/// A single-source composite view over `dataset`, in an explicitly SHARED blank
/// identity space so the view reports the source's own `(label, scope)` pairs.
fn composite_over(dataset: &Arc<RdfDataset>) -> CompositeDatasetView {
    CompositeDatasetView::from_shared_sources(
        vec![CompositeSource::new(Arc::clone(dataset))],
        ViewLimits::default(),
    )
    .expect("a single retained source composes")
}

/// A delta view over `dataset` with an EMPTY delta: the same RDF surface,
/// reached through the mutable branch's snapshot publication instead.
fn delta_over(dataset: &Arc<RdfDataset>) -> purrdf_rdf::DeltaDatasetView {
    MutableDataset::new(Arc::clone(dataset))
        .snapshot_view()
        .expect("an empty delta publishes")
}

/// The two-source trap fixture ingested through the FLAT carrier surface, each
/// source under its own ingest scope and with the default graph relocated.
fn flat_builder() -> SnapshotBuilder {
    let mut builder = SnapshotBuilder::new();
    builder
        .add_dataset_scoped(&source_a(), Some(SELECTED), Some("a"))
        .expect("flat source A ingests");
    builder
        .add_dataset_scoped(&source_b(), Some(SELECTED), Some("b"))
        .expect("flat source B ingests");
    builder
}

// ---------------------------------------------------------------------------
// Golden parity
// ---------------------------------------------------------------------------

#[test]
fn the_view_surface_and_the_flat_surface_mint_one_snapshot() {
    let flat = flat_builder();

    let (a, b) = (source_a(), source_b());
    let mut composite = SnapshotBuilder::new();
    let _ = composite
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("composite source A ingests");
    let _ = composite
        .add_view_scoped(&composite_over(&b), Some(SELECTED), Some("b"))
        .expect("composite source B ingests");

    let mut delta = SnapshotBuilder::new();
    let _ = delta
        .add_view_scoped(&delta_over(&a), Some(SELECTED), Some("a"))
        .expect("delta source A ingests");
    let _ = delta
        .add_view_scoped(&delta_over(&b), Some(SELECTED), Some("b"))
        .expect("delta source B ingests");

    // NON-VACUITY: the fixture really does carry every trap.
    let payload = flat.snapshot_payload();
    let rendered = format!("{payload:?}");
    let traps = [
        "cat".to_owned(),
        "مرحبا".to_owned(),
        "rtl".to_owned(),
        "bare".to_owned(),
        blank_wire(&a, "a"),
        blank_wire(&b, "b"),
    ];
    for trap in &traps {
        assert!(rendered.contains(trap), "fixture lost {trap:?}: {rendered}");
    }
    assert_ne!(
        blank_wire(&a, "a"),
        blank_wire(&b, "b"),
        "the two sources' equal local labels must stay two wire values"
    );
    assert!(
        !rendered.contains(EMPTY_A),
        "a declaration-only graph must never reach the term table: {rendered}"
    );

    assert_eq!(
        flat.snapshot_content_id(),
        composite.snapshot_content_id(),
        "the composite view path must mint the flat path's snapshot"
    );
    assert_eq!(
        flat.snapshot_content_id(),
        delta.snapshot_content_id(),
        "the delta view path must mint the flat path's snapshot"
    );
    // Not just the digest: the whole payload, term table included.
    assert_eq!(flat.snapshot_payload(), composite.snapshot_payload());
    assert_eq!(flat.snapshot_payload(), delta.snapshot_payload());

    // …and the emitted container bytes, which is what actually ships.
    let bytes = |builder: &SnapshotBuilder| {
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
        .expect("the fixture emits")
    };
    assert_eq!(bytes(&flat), bytes(&composite));
    assert_eq!(bytes(&flat), bytes(&delta));
}

#[test]
fn the_dropped_datatype_iris_are_not_interned_by_either_surface() {
    // `TermRef::Literal` ALWAYS carries a datatype id, while the flat path stores
    // a language-tagged literal with NO datatype and folds `xsd:string` away. If
    // the view path interned the datatype IRIs it drops, the term table would
    // gain `xsd:string`, `rdf:langString` and `rdf:dirLangString` rows and every
    // emitted bundle's bytes would move.
    let source = parse_dataset(
        concat!(
            "<https://example.org/s> <https://example.org/a> \"bare\" .\n",
            "<https://example.org/s> <https://example.org/b> ",
            "\"x\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
            "<https://example.org/s> <https://example.org/c> \"cat\"@en .\n",
            "<https://example.org/s> <https://example.org/d> \"مرحبا\"@ar--rtl .\n",
        )
        .as_bytes(),
        "application/n-triples",
        None,
    )
    .expect("literal spread parses");

    let mut view = SnapshotBuilder::new();
    let report = view
        .add_view(&composite_over(&source))
        .expect("the literal spread ingests");
    let rendered = format!("{:?}", view.snapshot_payload());
    for dropped in [
        "http://www.w3.org/2001/XMLSchema#string",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
        "dirLangString",
    ] {
        assert!(
            !rendered.contains(dropped),
            "the view path interned the dropped datatype {dropped:?}: {rendered}"
        );
    }

    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&source).expect("flat ingests");
    assert_eq!(flat.snapshot_payload(), view.snapshot_payload());
    // Exact term-table equality, not merely a matching digest.
    assert_eq!(
        flat.ingest_totals().terms_interned,
        report.terms_interned,
        "the two surfaces must mint the same number of dictionary rows"
    );
}

#[test]
fn two_sources_sharing_a_local_blank_label_stay_two_terms() {
    let (a, b) = (source_a(), source_b());
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests");
    let _ = builder
        .add_view_scoped(&composite_over(&b), Some(SELECTED), Some("b"))
        .expect("source B ingests");
    let rendered = format!("{:?}", builder.snapshot_payload());
    assert!(rendered.contains(&blank_wire(&a, "a")), "{rendered}");
    assert!(rendered.contains(&blank_wire(&b, "b")), "{rendered}");

    // THE NEIGHBOURING CASE: within ONE scope, one label is still ONE term. A
    // fix that made every blank unique would pass the assertion above and
    // silently stop co-referring.
    let mut single = SnapshotBuilder::new();
    let _ = single
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests");
    let before = single.ingest_totals().terms_interned;
    let _ = single
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests a second time");
    assert_eq!(
        single.ingest_totals().terms_interned,
        before,
        "re-ingesting one source at one scope must mint no new term"
    );
}

#[test]
fn the_statement_layer_rides_the_side_tables_and_is_not_double_ingested() {
    // The reifier binding's object is a quoted triple. It must not trip the
    // plain-slot guard (over-refusal), and the binding must not ALSO appear as an
    // ordinary quad (double ingestion). Both are checked against the flat path,
    // which is the frozen reference.
    let a = source_a();
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset_scoped(&a, Some(SELECTED), Some("a"))
        .expect("flat ingests the statement layer");
    let mut view = SnapshotBuilder::new();
    let report = view
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("the view path ingests the statement layer");

    assert_eq!(flat.snapshot_payload(), view.snapshot_payload());
    // Non-vacuity: the fixture genuinely carries both side tables.
    assert_eq!(
        a.reifier_quads().count(),
        2,
        "two reifier bindings in the fixture"
    );
    assert_eq!(
        a.annotation_quads().count(),
        2,
        "two annotations in the fixture"
    );
    assert_eq!(
        report.rows_consumed,
        a.quads().count() + a.reifier_quads().count() + a.annotation_quads().count(),
        "every row of every table is consumed exactly once"
    );
}

// ---------------------------------------------------------------------------
// Declaration-only graphs
// ---------------------------------------------------------------------------

#[test]
fn declaration_only_graphs_are_reported_and_never_interned() {
    let a = source_a();
    assert!(
        a.named_graphs().count() >= 3,
        "the fixture declares more graphs than it fills"
    );

    let mut view = SnapshotBuilder::new();
    let report = view
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests");
    assert_eq!(
        report.declarations_omitted,
        vec![EMPTY_A.to_owned(), SHARED.to_owned()],
        "the report must name exactly the graphs that own no row"
    );

    // The omission does not move one byte: the flat path, which never looks at
    // `named_graphs()` at all, mints the identical snapshot.
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset_scoped(&a, Some(SELECTED), Some("a"))
        .expect("flat ingests");
    assert_eq!(flat.snapshot_content_id(), view.snapshot_content_id());

    // THE NEIGHBOURING CASE: a source whose every declared graph owns a row
    // reports an EMPTY omission list — the report is evidence, not decoration.
    let b = source_b();
    let mut filled = SnapshotBuilder::new();
    let filled_report = filled
        .add_view_scoped(&composite_over(&b), Some(SELECTED), Some("b"))
        .expect("source B ingests");
    assert!(
        filled_report.declarations_omitted.is_empty(),
        "source B declares no empty graph: {:?}",
        filled_report.declarations_omitted
    );
}

#[test]
fn a_graph_empty_in_one_source_and_filled_in_another_is_interned_once() {
    let (a, b) = (source_a(), source_b());
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests");
    let _ = builder
        .add_view_scoped(&composite_over(&b), Some(SELECTED), Some("b"))
        .expect("source B ingests");
    let rendered = format!("{:?}", builder.snapshot_payload());
    assert!(
        rendered.contains(SHARED),
        "source B fills the graph source A only declared: {rendered}"
    );
    assert!(
        !rendered.contains(EMPTY_A),
        "the graph nobody filled stays out of the term table: {rendered}"
    );
    // The cumulative omission ledger still records A's view of it.
    assert!(
        builder
            .ingest_totals()
            .declarations_omitted
            .contains(&SHARED.to_owned())
    );
}

// ---------------------------------------------------------------------------
// Accounting
// ---------------------------------------------------------------------------

#[test]
fn the_report_accounts_scratch_and_the_cumulative_view_is_monotone() {
    let (a, b) = (source_a(), source_b());
    let mut builder = SnapshotBuilder::new();
    let first = builder
        .add_view_scoped(&composite_over(&a), Some(SELECTED), Some("a"))
        .expect("source A ingests");
    assert!(
        first.scratch_bytes > 0,
        "the trap fixture holds intern and staging scratch"
    );
    assert!(first.rows_consumed > 0 && first.terms_interned > 0);
    let after_first = builder.ingest_totals();
    assert_eq!(after_first.scratch_bytes, first.scratch_bytes);

    let second = builder
        .add_view_scoped(&composite_over(&b), Some(SELECTED), Some("b"))
        .expect("source B ingests");
    let after_second = builder.ingest_totals();
    assert!(
        after_second.scratch_bytes >= after_first.scratch_bytes,
        "the cumulative scratch peak must never fall: {} then {}",
        after_first.scratch_bytes,
        after_second.scratch_bytes
    );
    assert_eq!(
        after_second.rows_consumed,
        first.rows_consumed + second.rows_consumed
    );
    assert_eq!(
        after_second.terms_interned,
        first.terms_interned + second.terms_interned
    );
}

// ---------------------------------------------------------------------------
// The blank wire-value injectivity guard
// ---------------------------------------------------------------------------

/// One quad whose subject is the blank node `label`.
fn blank_subject(label: &str) -> Arc<RdfDataset> {
    parse_dataset(
        format!("_:{label} <https://example.org/p> <https://example.org/o> .\n").as_bytes(),
        "application/n-triples",
        None,
    )
    .expect("blank subject parses")
}

#[test]
fn colliding_blank_wire_values_are_refused_and_neighbouring_ones_are_not() {
    // `("a", "b-c")` and `("a-b", "c")` both encode onto the wire value `a-b-c`.
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(&blank_subject("b-c"), None, Some("a"))
        .expect("the first key mints its row");
    let err = builder
        .add_view_scoped(&blank_subject("c"), None, Some("a-b"))
        .expect_err("the second key encodes onto the same wire value");
    match &err {
        GtsIngestError::BlankWireCollision {
            wire_value,
            held_label,
            incoming_label,
            ..
        } => {
            assert_eq!(wire_value, "a-b-c");
            assert_eq!(held_label, "b-c");
            assert_eq!(incoming_label, "c");
        }
        other => panic!("expected a wire collision, got {other:?}"),
    }

    // THE NEIGHBOURING CASE: two scopes that do NOT collide still ingest, and
    // stay two distinct terms.
    let mut honest = SnapshotBuilder::new();
    let _ = honest
        .add_view_scoped(&blank_subject("b"), None, Some("a"))
        .expect("scope a ingests");
    let _ = honest
        .add_view_scoped(&blank_subject("c"), None, Some("a-b"))
        .expect("scope a-b ingests");
    let rendered = format!("{:?}", honest.snapshot_payload());
    assert!(rendered.contains("a-b"), "{rendered}");
    assert!(rendered.contains("a-b-c"), "{rendered}");

    // …and the same pair reaches the FLAT surface's error too, since both
    // surfaces run one ingestion core.
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset_scoped(&blank_subject("b-c"), None, Some("a"))
        .expect("the first key mints its row");
    let flat_err = flat
        .add_dataset_scoped(&blank_subject("c"), None, Some("a-b"))
        .expect_err("the flat surface refuses the same collision");
    assert!(flat_err.contains("a-b-c"), "{flat_err}");
}

// ---------------------------------------------------------------------------
// Terminal poison
// ---------------------------------------------------------------------------

/// A dataset carrying a quoted triple in ORDINARY object position — the shape the
/// plain-slot guard exists for.
fn quoted_triple_in_a_plain_slot() -> Arc<RdfDataset> {
    use purrdf_rdf::{RdfDatasetBuilder, TermFactory, TermValue as Value};
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/s");
    let p = builder.intern_iri("https://example.org/p");
    let o = builder.intern_iri("https://example.org/o");
    let quoted = builder.intern_value(&Value::Triple {
        s: Box::new(Value::iri("https://example.org/qs")),
        p: Box::new(Value::iri("https://example.org/qp")),
        o: Box::new(Value::iri("https://example.org/qo")),
    });
    builder.push_quad(s, p, quoted, None);
    builder.push_quad(s, p, o, None);
    builder.freeze().expect("the dataset freezes")
}

#[test]
fn a_failed_ingestion_poisons_both_surfaces_and_emit() {
    for flat_first in [false, true] {
        let mut builder = SnapshotBuilder::new();
        let _ = builder
            .add_view(&source_a())
            .expect("a healthy ingestion precedes the failure");
        let poisoned = quoted_triple_in_a_plain_slot();
        let first = if flat_first {
            builder
                .add_dataset(&poisoned)
                .expect_err("a quoted triple in a plain slot must fail closed")
        } else {
            builder
                .add_view(&poisoned)
                .expect_err("a quoted triple in a plain slot must fail closed")
                .to_string()
        };
        assert!(first.contains("not directly representable"), "{first}");
        assert!(builder.poison().is_some(), "the failure is terminal");

        // Every later ingestion refuses, on BOTH surfaces, naming the earlier failure.
        let again = builder
            .add_view(&source_b())
            .expect_err("a poisoned builder refuses the view surface");
        assert!(
            matches!(again, GtsIngestError::Poisoned { .. }),
            "{again:?}"
        );
        assert!(again.to_string().contains("not directly representable"));
        let flat_again = builder
            .add_dataset(&source_b())
            .expect_err("a poisoned builder refuses the flat surface");
        assert!(flat_again.contains("poisoned"), "{flat_again}");

        // …and publication refuses rather than shipping the partial ingestion.
        let err = emit_gts(
            &builder,
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
        .expect_err("a poisoned builder must not publish");
        assert!(err.contains("poisoned"), "{err}");
        assert!(err.contains("not directly representable"), "{err}");
    }

    // THE NEIGHBOURING CASE: the same two sources with no failure between them
    // publish exactly as before.
    let mut healthy = SnapshotBuilder::new();
    let _ = healthy.add_view(&source_a()).expect("source A ingests");
    let _ = healthy.add_view(&source_b()).expect("source B ingests");
    assert!(healthy.poison().is_none());
    emit_gts(
        &healthy,
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
    .expect("an unpoisoned builder publishes");
}

// ---------------------------------------------------------------------------
// A purpose-built probe view: budgeted faulting + capability claims
// ---------------------------------------------------------------------------

/// The probe view's operational root cause.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProbeFault(&'static str);

impl std::fmt::Display for ProbeFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ProbeFault {}

/// A view that delegates to a frozen dataset but can (a) fault after a fixed
/// number of ordinary rows, exactly as a lazy operational backend does, and (b)
/// claim a statement-layer capability its accessors do not answer for.
#[derive(Debug)]
struct ProbeView {
    inner: Arc<RdfDataset>,
    /// Ordinary rows still yieldable before the view faults.
    budget: Cell<usize>,
    faulted: Cell<bool>,
    /// Capability claims asserted ON TOP of the inner dataset's own.
    claim_reifiers: bool,
    claim_annotations: bool,
    /// Whether the statement-layer accessors answer at all.
    enumerate_statements: bool,
}

impl ProbeView {
    /// A faithful pass-through: unlimited budget, honest claims.
    fn honest(inner: Arc<RdfDataset>) -> Self {
        Self {
            inner,
            budget: Cell::new(usize::MAX),
            faulted: Cell::new(false),
            claim_reifiers: false,
            claim_annotations: false,
            enumerate_statements: true,
        }
    }

    /// Faults after `budget` ordinary rows.
    fn budgeted(inner: Arc<RdfDataset>, budget: usize) -> Self {
        let view = Self::honest(inner);
        view.budget.set(budget);
        view
    }

    /// Already failed before the first read.
    fn pre_faulted(inner: Arc<RdfDataset>) -> Self {
        let view = Self::honest(inner);
        view.faulted.set(true);
        view
    }

    /// Claims the RDF 1.2 statement layer while enumerating none of it.
    fn dishonest_claim(inner: Arc<RdfDataset>) -> Self {
        Self {
            claim_reifiers: true,
            claim_annotations: true,
            enumerate_statements: false,
            ..Self::honest(inner)
        }
    }

    /// Claims the statement layer AND enumerates it — the honest twin.
    fn honest_claim(inner: Arc<RdfDataset>) -> Self {
        Self {
            claim_reifiers: true,
            claim_annotations: true,
            enumerate_statements: true,
            ..Self::honest(inner)
        }
    }

    /// Spend one row of budget; the view faults when it runs out.
    fn spend(&self) -> bool {
        match self.budget.get().checked_sub(1) {
            Some(left) => {
                self.budget.set(left);
                true
            }
            None => {
                self.faulted.set(true);
                false
            }
        }
    }
}

impl DatasetView for ProbeView {
    type Id = TermId;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads().take_while(|_| self.spend())
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        self.quads().map(|q| QuadRef {
            s: self.resolve(q.s),
            p: self.resolve(q.p),
            o: self.resolve(q.o),
            g: q.g.map(|g| self.resolve(g)),
        })
    }

    fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.inner.resolve(id)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        self.inner.term_id_by_value(value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        let mut caps = self.inner.capabilities();
        caps.reifiers |= self.claim_reifiers;
        caps.annotations |= self.claim_annotations;
        caps
    }

    fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }

    fn term_count(&self) -> usize {
        self.inner.term_count()
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.enumerate_statements
            .then_some(())
            .into_iter()
            .flat_map(|()| self.inner.reifier_quads())
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.enumerate_statements
            .then_some(())
            .into_iter()
            .flat_map(|()| self.inner.annotation_quads())
    }

    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.inner.named_graphs()
    }
}

impl FallibleDatasetView for ProbeView {
    type Error = ProbeFault;
    type Evidence = usize;

    fn operation_status(&self) -> ViewOperationStatus<ProbeFault, usize> {
        let evidence = self.budget.get();
        if self.faulted.get() {
            ViewOperationStatus::Failed {
                error: ProbeFault("the probe view exhausted its row budget"),
                evidence,
            }
        } else {
            ViewOperationStatus::Ready { evidence }
        }
    }
}

#[test]
fn a_view_that_faults_mid_ingestion_never_mints_a_snapshot() {
    let a = source_a();
    let total = a.quads().count();
    assert!(total > 2, "the fixture has rows to truncate");

    // (i) The AFTER checkpoint: Ready at entry, Failed once the rows ran out.
    let mut builder = SnapshotBuilder::new();
    let truncating = ProbeView::budgeted(Arc::clone(&a), total - 2);
    assert!(matches!(
        truncating.operation_status(),
        ViewOperationStatus::Ready { .. }
    ));
    let err = builder
        .add_view(&truncating)
        .expect_err("a truncated read must never publish as a snapshot");
    assert_eq!(
        err,
        GtsIngestError::ViewNotReady {
            checkpoint: IngestCheckpoint::AfterRows,
            cause: "the probe view exhausted its row budget".to_owned(),
        }
    );
    assert!(builder.poison().is_some(), "and the builder is poisoned");

    // (ii) The BEFORE checkpoint: a view that has already failed is refused
    // before a single row is read.
    let mut early = SnapshotBuilder::new();
    let err = early
        .add_view(&ProbeView::pre_faulted(Arc::clone(&a)))
        .expect_err("an already-failed view is refused at entry");
    assert!(matches!(
        err,
        GtsIngestError::ViewNotReady {
            checkpoint: IngestCheckpoint::BeforeRows,
            ..
        }
    ));

    // THE NEIGHBOURING CASE: the same probe view with budget for every row
    // ingests, and mints exactly the flat carrier's snapshot.
    let mut healthy = SnapshotBuilder::new();
    let _ = healthy
        .add_view(&ProbeView::honest(Arc::clone(&a)))
        .expect("an unbudgeted probe view ingests");
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&a).expect("flat ingests");
    assert_eq!(flat.snapshot_content_id(), healthy.snapshot_content_id());
}

#[test]
fn a_claimed_statement_layer_must_be_enumerable() {
    let a = source_a();
    assert!(
        a.capabilities().reifiers && a.capabilities().annotations,
        "the fixture's own claims are honest"
    );

    // The dishonest view claims the layer and answers nothing for it. Ingesting
    // it would emit a 1.2-stripped snapshot and report success.
    let mut builder = SnapshotBuilder::new();
    let err = builder
        .add_view(&ProbeView::dishonest_claim(Arc::clone(&a)))
        .expect_err("an unenumerable claim must be refused");
    assert!(
        matches!(err, GtsIngestError::UnenumerableCapability { .. }),
        "{err:?}"
    );
    assert!(builder.poison().is_some());

    // THE HONEST TWIN: the same claims, answered. It ingests, and mints the flat
    // carrier's snapshot.
    let mut honest = SnapshotBuilder::new();
    let _ = honest
        .add_view(&ProbeView::honest_claim(Arc::clone(&a)))
        .expect("an enumerable claim ingests");
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&a).expect("flat ingests");
    assert_eq!(flat.snapshot_content_id(), honest.snapshot_content_id());

    // …and a view with NO statement layer at all, claiming none, is equally fine:
    // the gate is about the CLAIM, never about emptiness.
    let plain = blank_subject("b0");
    assert!(!plain.capabilities().reifiers && !plain.capabilities().annotations);
    let mut empty = SnapshotBuilder::new();
    let _ = empty
        .add_view(&ProbeView::honest(plain))
        .expect("a view with no statement layer ingests");
}
