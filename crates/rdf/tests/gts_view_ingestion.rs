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
//! * **Refusals, each with its neighbouring valid twin.** The two operational
//!   checkpoints, the plain-slot guard, the blank wire-value injectivity guard
//!   and the terminal poison flag are all refusals, and a refusal that also
//!   rejects valid input is the mirror of a silent drop. What is NOT a refusal is
//!   also pinned: a capability claim is not runtime-checkable, so an
//!   enumerable-but-empty statement layer is admitted rather than rejected.
//! * **Atomicity.** A refusal rolls its own ingestion back, so the infallible
//!   `snapshot_payload`/`snapshot_content_id` accessors never describe a
//!   truncated interior — while publication still refuses.
//! * **Accounting.** The report names the declaration-only graphs that were
//!   deliberately NOT interned, and its scratch figure is non-zero and monotone.
//! * **The keystone.** All of it assembled at once: a `PipelineViewBundle` over a
//!   largish shared base, grown by a named-graph accumulation, joined by a delta
//!   carrier and pinned to a typed handle, is ingested and emitted THROUGH its
//!   composed view — freezing nothing, and shipping the bytes the materialized
//!   flat path ships.

use std::cell::Cell;
use std::sync::Arc;

use purrdf_rdf::dataset_view::ViewOperationStatus;
use purrdf_rdf::gts_compose::{
    DEFAULT_RSYNCABLE_THRESHOLD, GtsIngestError, IngestCheckpoint, MediumPlan, RDF_REIFIES,
    SnapshotBuilder, emit_gts,
};
// The keystone shape itself lives in the library, so this suite and
// `benches/gts_ingest.rs` measure ONE fixture rather than two drifting copies.
use purrdf_rdf::gts_fixtures::{
    KEY_DECLARED, KEY_G1, KEY_G2, KEY_G3, KEYSTONE_GROUPS, SELECTED, emitted, keystone_base,
    keystone_contribution, keystone_delta, keystone_loadout,
};
use purrdf_rdf::{
    BlankScope, CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, DeltaDatasetView,
    FallibleDatasetView, GraphMatch, MutableDataset, PipelineViewBundle, QuadIds, QuadRef,
    QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, RdfTextDirection,
    RetentionLedger, TermId, TermRef, TermValue, ViewLimits, parse_dataset,
};

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
fn delta_over(dataset: &Arc<RdfDataset>) -> DeltaDatasetView {
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

/// A FAILED INGESTION IS TAKEN BACK OUT, so nothing can ever observe a
/// half-ingested interior.
///
/// `snapshot_content_id` and `snapshot_payload` are infallible by frozen
/// signature — they cannot report the poison flag, and a library may not answer a
/// question by panicking. The guarantee is therefore structural rather than
/// documentary: the refusal rolls its own ingestion back before returning, so
/// every state those accessors can be called in is a fully accepted one.
///
/// The failing source here is deliberately LARGE and mixed — its refusal fires
/// after many rows and terms of every kind have already been interned — so a
/// rollback that forgot a table would show up as a moved content id.
#[test]
fn a_refused_ingestion_leaves_the_snapshot_accessors_exactly_where_they_were() {
    let mut builder = SnapshotBuilder::new();
    let _ = builder
        .add_view_scoped(&composite_over(&source_a()), Some(SELECTED), Some("a"))
        .expect("a healthy ingestion precedes the failure");
    let accepted_id = builder.snapshot_content_id();
    let accepted_payload = builder.snapshot_payload();
    let accepted_totals = builder.ingest_totals();

    // A source that ingests a great many rows and THEN hits the plain-slot guard.
    let mut poisoning = RdfDatasetBuilder::new();
    let s = poisoning.intern_iri("https://example.org/poison/s");
    let p = poisoning.intern_iri("https://example.org/poison/p");
    for index in 0..64 {
        let o = poisoning.intern_iri(&format!("https://example.org/poison/o{index}"));
        let blank = poisoning.intern_blank(&format!("p{index}"), BlankScope(9));
        poisoning.push_quad(s, p, o, None);
        poisoning.push_quad(blank, p, o, None);
    }
    let quoted = poisoning.intern_triple(s, p, s);
    poisoning.push_quad(s, p, quoted, None);
    let poisoning = poisoning.freeze().expect("the poisoning source freezes");

    let err = builder
        .add_view(&poisoning)
        .expect_err("a quoted triple in a plain slot must fail closed");
    assert!(matches!(err, GtsIngestError::UnrepresentableTerm { .. }));
    assert!(builder.poison().is_some(), "the failure is terminal");

    // THE ROLLBACK: not one row, term or index entry of the refused ingestion
    // survives, on any table.
    assert_eq!(
        builder.snapshot_content_id(),
        accepted_id,
        "a refused ingestion must not move the content id"
    );
    assert_eq!(builder.snapshot_payload(), accepted_payload);
    assert_eq!(
        builder.ingest_totals().rows_consumed,
        accepted_totals.rows_consumed,
        "…nor the receipt's row count"
    );
    assert_eq!(
        builder.ingest_totals().terms_interned,
        accepted_totals.terms_interned
    );

    // NON-VACUITY: the refused source really would have moved the content id had
    // any of it been kept. The same rows, minus the one unrepresentable term,
    // ingest into a fresh builder and mint something different.
    let mut without_the_trap = RdfDatasetBuilder::new();
    let s = without_the_trap.intern_iri("https://example.org/poison/s");
    let p = without_the_trap.intern_iri("https://example.org/poison/p");
    for index in 0..64 {
        let o = without_the_trap.intern_iri(&format!("https://example.org/poison/o{index}"));
        without_the_trap.push_quad(s, p, o, None);
    }
    let without_the_trap = without_the_trap.freeze().expect("the control freezes");
    let mut control = SnapshotBuilder::new();
    let _ = control
        .add_view_scoped(&composite_over(&source_a()), Some(SELECTED), Some("a"))
        .expect("the same healthy ingestion");
    let _ = control
        .add_view(&without_the_trap)
        .expect("the representable rows ingest");
    assert_ne!(
        control.snapshot_content_id(),
        accepted_id,
        "the refused rows were not a no-op; the rollback is doing real work"
    );

    // …and publication still refuses, complete tables notwithstanding: a full
    // description of content the caller did not ask for is not the snapshot.
    let refused = emit_gts(
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
    .expect_err("a poisoned builder must not publish, rolled back or not");
    assert!(refused.contains("poisoned"), "{refused}");
}

/// An unrepresentable-term refusal names the term in RDF TEXT, never in the
/// view's own ids.
///
/// A quoted triple used to fall through to `Debug`, so the message read
/// `Triple { s: Id(..), p: Id(..), o: Id(..) }` — the offending term named in a
/// vocabulary only the view speaks, leaving the reader unable to find the row it
/// came from. Every constituent is now resolved.
#[test]
fn an_unrepresentable_quoted_triple_is_named_in_its_own_text() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("https://example.org/quoted-subject");
    let p = b.intern_iri("https://example.org/quoted-predicate");
    let o = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    let quoted = b.intern_triple(s, p, o);
    let plain_s = b.intern_iri("https://example.org/s");
    let plain_p = b.intern_iri("https://example.org/p");
    b.push_quad(plain_s, plain_p, quoted, None);
    let source = b.freeze().expect("the plain-slot trap freezes");

    let mut builder = SnapshotBuilder::new();
    let message = builder
        .add_view(&source)
        .expect_err("a quoted triple in a plain slot must fail closed")
        .to_string();

    for text in [
        "https://example.org/quoted-subject",
        "https://example.org/quoted-predicate",
        "مرحبا",
        "ar--rtl",
    ] {
        assert!(
            message.contains(text),
            "the refusal must name {text:?}: {message}"
        );
    }
    assert!(
        !message.contains("Id("),
        "and must name no internal id: {message}"
    );
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

    /// Claims the RDF 1.2 statement layer while its accessors answer NOTHING for
    /// it — the trait obligation broken by construction. Not runtime-detectable
    /// (see `a_capability_claim_never_moves_a_byte_of_the_snapshot`); what it
    /// publishes is what it enumerated.
    fn unenumerated_claim(inner: Arc<RdfDataset>) -> Self {
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

    /// Claims the statement layer over an inner dataset that HAS none: the
    /// accessors answer, honestly, with nothing. Indistinguishable at runtime
    /// from [`Self::unenumerated_claim`], and perfectly valid.
    fn empty_claim(inner: Arc<RdfDataset>) -> Self {
        Self::honest_claim(inner)
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

/// The snapshot is a function of the rows a view ENUMERATES, never of the flags
/// it sets — and the ingestion no longer pretends the two can be cross-checked.
///
/// A runtime gate used to stand at the head of the ingestion refusing any view
/// whose claimed statement layer enumerated no row, on the theory that an empty
/// answer to a claimed capability was a detectable lie. It is not: "this view
/// cannot enumerate its reifiers" and "this view has no reifiers" are the same
/// observation at runtime, and the second is an ordinary valid state that the
/// gate therefore refused (see
/// `a_delta_that_removes_the_bases_only_reifier_still_ingests`, which is exactly
/// the input it rejected).
///
/// So the `DatasetView` obligation — a claimed capability must be answerable
/// through its accessor — stays a PROSE contract on the trait, and this test
/// pins what the ingestion can honestly say instead: an enumerable-but-empty
/// layer is admitted, a claim moves no byte, and a view that does not answer for
/// its own layer publishes what it enumerated.
#[test]
fn a_capability_claim_never_moves_a_byte_of_the_snapshot() {
    let a = source_a();
    assert!(
        a.capabilities().reifiers && a.capabilities().annotations,
        "the fixture's own claims are honest"
    );

    // (i) ENUMERABLE BUT EMPTY IS ADMITTED. A view over a dataset with no
    // statement layer at all, claiming one, ingests — and mints exactly the flat
    // carrier's snapshot, because the claim contributed nothing to it.
    let plain = blank_subject("b0");
    assert!(!plain.capabilities().reifiers && !plain.capabilities().annotations);
    let mut claimed = SnapshotBuilder::new();
    let _ = claimed
        .add_view(&ProbeView::empty_claim(Arc::clone(&plain)))
        .expect("a claimed but empty statement layer is valid input");
    let mut flat_plain = SnapshotBuilder::new();
    flat_plain.add_dataset(&plain).expect("flat ingests");
    assert_eq!(
        flat_plain.snapshot_content_id(),
        claimed.snapshot_content_id(),
        "an empty claimed layer must mint the flat path's snapshot"
    );

    // (ii) THE CLAIM IS INERT. The same view with the claims dropped mints the
    // same bytes: nothing reads the flags on the way to the payload.
    let mut unclaimed = SnapshotBuilder::new();
    let _ = unclaimed
        .add_view(&ProbeView::honest(Arc::clone(&plain)))
        .expect("the unclaiming view ingests");
    assert_eq!(claimed.snapshot_payload(), unclaimed.snapshot_payload());

    // (iii) A CLAIM THAT IS ANSWERED still rides through untouched.
    let mut answered = SnapshotBuilder::new();
    let _ = answered
        .add_view(&ProbeView::honest_claim(Arc::clone(&a)))
        .expect("an enumerable claim ingests");
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&a).expect("flat ingests");
    assert_eq!(flat.snapshot_content_id(), answered.snapshot_content_id());

    // (iv) AND A VIEW THAT DOES NOT ANSWER FOR ITS OWN LAYER publishes what it
    // enumerated — which is the prose obligation stated as a consequence rather
    // than as a check. `source_a` really does hold a statement layer, the
    // suppressing view really does hide it, and the snapshot really does differ:
    // nothing here is detectable from inside the ingestion, so nothing here is
    // refused by it.
    let mut suppressing = SnapshotBuilder::new();
    let _ = suppressing
        .add_view(&ProbeView::unenumerated_claim(Arc::clone(&a)))
        .expect("an unanswered claim is not runtime-detectable, so it is not refused");
    assert!(suppressing.poison().is_none());
    assert_ne!(
        flat.snapshot_content_id(),
        suppressing.snapshot_content_id(),
        "the suppressed statement layer is genuinely absent from what it published"
    );
}

/// THE INPUT THE OLD GATE REFUSED, driven through a real delta carrier.
///
/// `DeltaDatasetView::capabilities()` is the union of its base's and its
/// delta's, so a delta that removes the base's ONLY reifier binding still claims
/// the statement layer while its accessor — correctly — enumerates nothing. That
/// is valid RDF and a valid view; refusing it was an over-refusal, the mirror of
/// the silent drop the gate was reaching for.
#[test]
fn a_delta_that_removes_the_bases_only_reifier_still_ingests() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("https://example.org/s");
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    let claim = b.intern_iri("https://example.org/claim");
    let quoted = b.intern_triple(s, p, o);
    b.push_quad(s, p, o, None);
    b.push_reifier_in_graph(claim, quoted, None);
    let base = b.freeze().expect("the base freezes");
    assert!(
        base.capabilities().reifiers,
        "the base claims the layer it actually holds"
    );

    // The delta takes that one binding back out again.
    let mut mutable = MutableDataset::new(Arc::clone(&base));
    assert!(
        mutable.remove(&QuadValues {
            s: TermValue::iri("https://example.org/claim"),
            p: TermValue::iri(RDF_REIFIES),
            o: TermValue::Triple {
                s: Box::new(TermValue::iri("https://example.org/s")),
                p: Box::new(TermValue::iri("https://example.org/p")),
                o: Box::new(TermValue::iri("https://example.org/o")),
            },
            g: None,
        }),
        "the base's only reifier binding is suppressed by the delta"
    );
    let view = mutable.snapshot_view().expect("the delta publishes");

    // NON-VACUITY: this really is the exact pair the gate keyed on.
    assert!(
        view.capabilities().reifiers,
        "capabilities() is base ∪ delta, so the claim survives the removal"
    );
    assert!(
        view.reifier_quads().next().is_none(),
        "…while the accessor correctly enumerates nothing"
    );

    let mut through_view = SnapshotBuilder::new();
    let _ = through_view
        .add_view(&view)
        .expect("a reifier-suppressing delta is valid input, not a refusal");

    // …and it mints the snapshot of its own flat-materialized equivalent.
    let materialized = mutable.freeze().expect("the same delta materializes");
    assert_eq!(materialized.reifier_quads().count(), 0);
    let mut flat = SnapshotBuilder::new();
    flat.add_dataset(&materialized).expect("flat ingests");
    assert_eq!(
        through_view.snapshot_content_id(),
        flat.snapshot_content_id(),
        "the view path must mint the flat path's snapshot"
    );
    assert_eq!(through_view.snapshot_payload(), flat.snapshot_payload());

    // THE NEIGHBOURING CASE: a delta that leaves the binding alone still carries
    // it through, so the admission above is not "the statement layer is ignored".
    let untouched = MutableDataset::new(Arc::clone(&base))
        .snapshot_view()
        .expect("an empty delta publishes");
    assert_eq!(untouched.reifier_quads().count(), 1);
    let mut kept = SnapshotBuilder::new();
    let _ = kept
        .add_view(&untouched)
        .expect("the untouched delta ingests");
    let mut flat_base = SnapshotBuilder::new();
    flat_base.add_dataset(&base).expect("flat ingests the base");
    assert_eq!(kept.snapshot_content_id(), flat_base.snapshot_content_id());
    assert_ne!(
        kept.snapshot_content_id(),
        through_view.snapshot_content_id(),
        "removing the binding really did change what is published"
    );
}

// ---------------------------------------------------------------------------
// The keystone: a view carrier travels the whole pipeline and publishes bytes
// ---------------------------------------------------------------------------
//
// Everything above proves one seam at a time. This is the seam ASSEMBLED: a
// `PipelineViewBundle` is built over a shared base, grown by one named-graph
// accumulation, joined by a delta carrier over the same base, pinned to a typed
// handle — and then ingested and emitted as GTS *through its composed view*, with
// no dataset ever frozen on the way.
//
// What the assembly has to prove, and each is a way the seam could be a lie:
//
// * **Nothing is copied to publish.** The view's own `freezes` and
//   `materializations` counters must not move across ingest-and-emit. A carrier
//   that quietly materialized would still emit the right bytes — and would have
//   paid for the whole dataset to do it.
// * **The bytes are the flat bytes.** A second, identically-constructed carrier is
//   materialized and ingested through the FLAT surface. Equality is BYTE equality
//   on the emitted container, not isomorphism: GTS blank wire values are the
//   view's qualified labels, so anything that renamed a scope shows up here.
// * **The receipt is bound and sane.** Rows are consumed, and the declaration-only
//   graph is NAMED as omitted rather than silently skipped.
// * **Accumulation invalidates exactly one leaf**, and a second reader of the same
//   base is answered from the shared ledger memo rather than re-canonicalizing.

/// The keystone's pipeline-side handle payload. Concrete pipeline types plug into
/// the same lane; the kernel never learns what they are.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Note(&'static str);

/// The keystone base at this suite's size. The shape itself — and everything it
/// is a trap for — is documented once, in `purrdf_rdf::gts_fixtures`.
fn base_dataset() -> Arc<RdfDataset> {
    keystone_base(KEYSTONE_GROUPS)
}

/// The contribution folded into `KEY_G3` by `accumulate_named_graph`.
fn contribution_dataset() -> Arc<RdfDataset> {
    keystone_contribution(KEY_G3, 0, 6)
}

/// A carrier over `base` with one named graph accumulated into it — the exact
/// construction the measured carrier and its byte-parity reference both use, so
/// the two composites agree on source order, placement and standardized scopes.
fn keystone_carrier(
    base: &Arc<RdfDataset>,
    contribution: &Arc<RdfDataset>,
    ledger: &Arc<RetentionLedger>,
) -> PipelineViewBundle<Note> {
    let (lookaside, blobs, provenance) = keystone_loadout();
    let mut carrier = PipelineViewBundle::from_dataset(
        base,
        lookaside,
        blobs,
        provenance,
        ledger,
        ViewLimits::default(),
    )
    .expect("the base is within the default ceilings");
    carrier
        .accumulate_named_graph(KEY_G3, contribution, Note("g3"))
        .expect("a contained contribution folds in");
    carrier
}

/// A carrier over `delta`, composing the delta snapshot without compacting it.
fn keystone_delta_carrier(
    delta: &Arc<DeltaDatasetView>,
    ledger: &Arc<RetentionLedger>,
) -> PipelineViewBundle<Note> {
    let (lookaside, blobs, provenance) = keystone_loadout();
    PipelineViewBundle::from_delta(
        delta,
        lookaside,
        blobs,
        provenance,
        ledger,
        ViewLimits::default(),
    )
    .expect("the delta is within the default ceilings")
}

#[test]
fn a_view_carrier_publishes_gts_bytes_without_ever_freezing_its_surface() {
    let base = base_dataset();
    let contribution = contribution_dataset();
    let delta = keystone_delta(&base);
    let ledger = RetentionLedger::new();

    // NON-VACUITY: the fixture really is the largish, trap-bearing base claimed.
    assert!(
        base.quads().count() > 300,
        "the keystone base must be largish: {}",
        base.quads().count()
    );
    assert!(base.reifier_quads().count() >= 8 && base.annotation_quads().count() >= 8);
    assert!(
        base.named_graphs().count() >= 3,
        "the base declares more graphs than it fills"
    );

    // ---- The measured carrier: composed, grown, pinned. Nothing frozen. -------
    let (lookaside, blobs, provenance) = keystone_loadout();
    let mut carrier = PipelineViewBundle::<Note>::from_dataset(
        &base,
        lookaside,
        blobs,
        provenance,
        &ledger,
        ViewLimits::default(),
    )
    .expect("the base composes within the default ceilings");

    let g1 = carrier.graph_digest(KEY_G1).expect("<g1> canonicalizes");
    let g2 = carrier.graph_digest(KEY_G2).expect("<g2> canonicalizes");
    assert_ne!(g1, g2, "the two graphs differ, or the leaves prove nothing");
    carrier
        .pin_handle(KEY_G1, Note("g1"), g1)
        .expect("a handle pinned to its own graph's digest attaches");

    let before_accumulate = carrier.digest_work();
    assert_eq!(
        before_accumulate.graph_canonicalizations, 2,
        "two graphs read, two canonicalizations"
    );

    carrier
        .accumulate_named_graph(KEY_G3, &contribution, Note("g3"))
        .expect("a contained contribution appends");

    // EXACT INVALIDATION: the accumulation canonicalized the graph it touched and
    // nothing else, and the untouched leaves are afterwards SERVED from the memo.
    let after_accumulate = carrier.digest_work();
    assert_eq!(
        after_accumulate.graph_canonicalizations,
        before_accumulate.graph_canonicalizations + 1,
        "exactly the accumulated graph is canonicalized again"
    );
    assert_eq!(carrier.graph_digest(KEY_G1).expect("<g1>"), g1);
    assert_eq!(carrier.graph_digest(KEY_G2).expect("<g2>"), g2);
    let served = carrier.digest_work();
    assert_eq!(
        served.graph_canonicalizations, after_accumulate.graph_canonicalizations,
        "an untouched leaf is never re-canonicalized after an accumulation"
    );
    assert_eq!(
        served.graph_cache_hits,
        after_accumulate.graph_cache_hits + 2,
        "both untouched leaves came from the memo"
    );

    // THE PIN STILL VALIDATES across the accumulation — the growth was additive.
    assert_eq!(
        carrier.handle(KEY_G1).expect("the pin survives").payload,
        Note("g1")
    );
    assert_eq!(
        carrier
            .handle(KEY_G1)
            .expect("the pin survives")
            .content_digest,
        g1
    );
    assert_eq!(
        carrier.graph_digest(KEY_G1).expect("<g1>"),
        carrier
            .handle(KEY_G1)
            .expect("the pin survives")
            .content_digest,
        "the handle still agrees with the graph it claims to project"
    );
    assert!(
        carrier.handle(KEY_G3).is_some(),
        "and the fold pinned its own"
    );

    // A SECOND READER of the same base is answered from the SHARED ledger memo
    // rather than paying for the analysis twice. Its own memo is empty, so nothing
    // but the ledger can be answering.
    let second_reader = keystone_carrier(&base, &contribution, &ledger);
    assert_eq!(second_reader.digest_work().graph_cache_hits, 0);
    let hits_before = ledger.snapshot().memo_hits;
    assert_eq!(
        second_reader
            .graph_digest(KEY_G1)
            .expect("<g1> on the second reader"),
        g1
    );
    assert_eq!(
        ledger.snapshot().memo_hits,
        hits_before + 1,
        "the shared ledger memo answered the second reader"
    );
    assert_eq!(
        second_reader.digest_work().graph_canonicalizations,
        0,
        "and nothing was canonicalized to do it"
    );

    let delta_carrier = keystone_delta_carrier(&delta, &ledger);

    // ---- The measured path: ingest both carriers' VIEWS, then emit. ----------
    let composite_work = carrier.view_stats().work;
    let delta_work = delta_carrier.view_stats().work;
    // Non-vacuity for the zero-delta claim below: the freeze counter IS live. The
    // accumulation's `GraphPlacement::Named` derived one graph dictionary, and that
    // dictionary cost exactly one freeze.
    assert_eq!(
        composite_work.freezes, 1,
        "composing the accumulated graph's dictionary is the one freeze charged"
    );
    assert_eq!(delta_work.freezes, 0, "a preserved delta charges none");
    assert_eq!(composite_work.materializations, 0);
    assert_eq!(delta_work.materializations, 0);

    let mut measured = SnapshotBuilder::new();
    let base_report = measured
        .add_view_scoped(carrier.view(), Some(SELECTED), Some("base"))
        .expect("the composed carrier ingests");
    let delta_report = measured
        .add_view_scoped(delta_carrier.view(), Some(SELECTED), Some("delta"))
        .expect("the delta carrier ingests");
    let measured_bytes = emitted(&measured);

    // NOTHING WAS COPIED TO PUBLISH IT.
    assert_eq!(
        carrier.view_stats().work.freezes,
        composite_work.freezes,
        "ingesting and emitting a composite view must freeze nothing"
    );
    assert_eq!(
        carrier.view_stats().work.materializations,
        composite_work.materializations,
        "ingesting and emitting a composite view must materialize nothing"
    );
    assert_eq!(
        carrier.view_stats().work.copied_rows,
        composite_work.copied_rows,
        "and must replay no row at an ownership boundary"
    );
    assert_eq!(delta_carrier.view_stats().work.freezes, delta_work.freezes);
    assert_eq!(
        delta_carrier.view_stats().work.materializations,
        delta_work.materializations
    );

    // ---- The receipt ---------------------------------------------------------
    assert!(
        base_report.rows_consumed > base.quads().count(),
        "every table of the base plus the contribution is consumed: {}",
        base_report.rows_consumed
    );
    assert!(base_report.terms_interned > 0 && base_report.scratch_bytes > 0);
    assert_eq!(
        base_report.declarations_omitted,
        vec![KEY_DECLARED.to_owned()],
        "the report names exactly the graph that owns no row"
    );
    assert_eq!(
        delta_report.declarations_omitted,
        vec![KEY_DECLARED.to_owned()],
        "the delta carries the same declaration, and states the same omission"
    );
    let totals = measured.ingest_totals();
    assert_eq!(
        totals.rows_consumed,
        base_report.rows_consumed + delta_report.rows_consumed
    );
    assert_eq!(
        totals.terms_interned,
        base_report.terms_interned + delta_report.terms_interned
    );
    // The omission is an omission: the empty graph's IRI never reached the table.
    let rendered = format!("{:?}", measured.snapshot_payload());
    assert!(
        !rendered.contains(KEY_DECLARED),
        "a declaration-only graph must never be interned"
    );
    for present in [KEY_G1, KEY_G2, KEY_G3, SELECTED, "مرحبا", "cat"] {
        assert!(rendered.contains(present), "the ingestion lost {present:?}");
    }

    // ---- The reference: the SAME carriers, materialized, ingested flat -------
    // A second, identically-constructed pair — so the measured carriers' own work
    // counters stay untouched by the freeze this reference path deliberately pays.
    let reference_main = keystone_carrier(&base, &contribution, &ledger);
    let reference_delta = keystone_delta_carrier(&delta, &ledger);
    let flat_main = reference_main.materialize().expect("the reference freezes");
    let flat_delta = reference_delta
        .materialize()
        .expect("the reference delta freezes");
    assert_eq!(
        reference_main.view_stats().work.materializations,
        1,
        "the reference path is the one that pays for a materialization"
    );

    let mut reference = SnapshotBuilder::new();
    reference
        .add_dataset_scoped(&flat_main, Some(SELECTED), Some("base"))
        .expect("the materialized carrier ingests flat");
    reference
        .add_dataset_scoped(&flat_delta, Some(SELECTED), Some("delta"))
        .expect("the materialized delta ingests flat");

    assert_eq!(
        measured.snapshot_content_id(),
        reference.snapshot_content_id(),
        "the view path must mint the flat path's snapshot"
    );
    assert_eq!(
        measured.snapshot_payload(),
        reference.snapshot_payload(),
        "term table included"
    );
    assert_eq!(
        measured_bytes,
        emitted(&reference),
        "and the emitted container bytes — blank wire values included — must be equal"
    );
}
