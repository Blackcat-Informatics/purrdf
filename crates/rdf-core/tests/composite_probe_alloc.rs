// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compact borrowed probe cursors and dataset-independent allocation costs.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::{
    BlankScope, CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, DeltaDatasetView,
    GraphMatch, GraphPlacement, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder, TermRef,
    TermValue, ViewLimits,
};
use std::hint::black_box;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// The ceiling on one borrowed composite cursor. A cursor holds the active
/// carrier's iterator alone, so it stays well inside this; a cursor that inlined
/// every inactive carrier and statement-table branch would be tens of KiB.
const CURSOR_CAP: usize = 256 * size_of::<usize>();

/// The ceiling on one `DeltaDatasetView` ordinary-probe cursor: its base and
/// delta layers' cursors and the demoted statement arm, each held once. A
/// cursor that kept `flat_map`'s front and back inner cursor for every layer
/// would be over 100 words.
const DELTA_CURSOR_CAP: usize = 64 * size_of::<usize>();

fn source(rows: usize, owner: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    let graph = builder.intern_iri("http://example.org/g");
    for row in 0..rows {
        let subject = builder.intern_iri(&format!("http://example.org/source{owner}/s{row}"));
        builder.push_quad(subject, predicate, object, Some(graph));
    }
    builder.freeze().expect("valid source")
}

fn composite(rows: usize, owners: usize) -> CompositeDatasetView {
    let sources = (0..owners)
        .map(|owner| CompositeSource::new(source(rows, owner)))
        .collect();
    CompositeDatasetView::from_bound_sources(sources, ViewLimits::default()).expect("view")
}

fn quad(owner: usize, subject: &str) -> QuadValues {
    QuadValues {
        s: TermValue::Iri(format!("http://example.org/source{owner}/{subject}")),
        p: TermValue::Iri("http://example.org/p".into()),
        o: TermValue::Iri("http://example.org/o".into()),
        g: Some(TermValue::Iri("http://example.org/g".into())),
    }
}

/// One mutation snapshot per owner over `rows` base rows: the base's last row
/// removed and one `late` row inserted, so every probe reads both layers
/// through the overlay's suppression mask.
fn delta(rows: usize, owner: usize) -> Arc<DeltaDatasetView> {
    let mut mutation = MutableDataset::new(source(rows, owner));
    assert!(mutation.remove(&quad(owner, &format!("s{}", rows - 1))));
    assert!(mutation.insert(quad(owner, "late")).unwrap());
    Arc::new(mutation.snapshot_view().expect("snapshot"))
}

fn delta_composite(rows: usize, owners: usize) -> CompositeDatasetView {
    let sources = (0..owners)
        .map(|owner| CompositeSource::from_delta(delta(rows, owner)))
        .collect();
    CompositeDatasetView::from_bound_sources(sources, ViewLimits::default()).expect("view")
}

#[test]
fn probe_cursor_layout_is_bounded() {
    let view = composite(10, 1);
    let ordinary = view.quads_for_pattern(None, None, None, GraphMatch::Any);
    let reifiers = view.reifier_quads();
    let annotations = view.annotation_quads();
    let layouts = [
        size_of_val(&ordinary),
        size_of_val(&reifiers),
        size_of_val(&annotations),
    ];
    println!("borrowed composite cursor bytes ordinary/reifier/annotation: {layouts:?}");
    for bytes in layouts {
        assert!(
            bytes <= CURSOR_CAP,
            "inactive branches inflated cursor to {bytes} bytes"
        );
    }
    assert_eq!(ordinary.count(), 10);
    assert_eq!(reifiers.count(), 0);
    assert_eq!(annotations.count(), 0);
}

/// How a measured probe's rows are pulled.
#[derive(Clone, Copy, Debug)]
enum Pull {
    /// Internal iteration (`count`, through each adapter's `fold`).
    Fold,
    /// External iteration, one `next` call per row, as a join or a validator
    /// that interleaves probes consumes a cursor.
    Next,
}

fn pull<I: Iterator>(rows: I, how: Pull) -> usize {
    match how {
        Pull::Fold => rows.count(),
        Pull::Next => {
            let mut rows = rows;
            let mut pulled = 0;
            while black_box(rows.next()).is_some() {
                pulled += 1;
            }
            pulled
        }
    }
}

fn measure<D: DatasetView>(view: &D, subject: Option<D::Id>, how: Pull) -> (usize, Measurement) {
    black_box(pull(
        view.quads_for_pattern(subject, None, None, GraphMatch::Any),
        how,
    ));
    let window = CurrentThreadWindow::open();
    let rows = pull(
        view.quads_for_pattern(subject, None, None, GraphMatch::Any),
        how,
    );
    (rows, window.close())
}

fn id_of<D: DatasetView>(view: &D, iri: &str) -> D::Id {
    view.term_id_by_value(&TermValue::Iri(iri.into()))
        .unwrap()
        .expect("fixture term")
}

#[test]
fn singleton_and_wide_probes_do_not_allocate_by_dataset_size() {
    for owners in [1, 8] {
        let small = composite(10, owners);
        let large = composite(10_000, owners);
        let needle = "http://example.org/source0/s0";
        let small_id = id_of(&small, needle);
        let large_id = id_of(&large, needle);
        for singleton in [false, true] {
            for how in [Pull::Fold, Pull::Next] {
                let (small_rows, small_cost) = measure(&small, singleton.then_some(small_id), how);
                let (large_rows, large_cost) = measure(&large, singleton.then_some(large_id), how);
                assert_eq!(small_rows, if singleton { 1 } else { 10 * owners });
                assert_eq!(large_rows, if singleton { 1 } else { 10_000 * owners });
                // Native carriers dispatch to an inline arm: no probe, singleton
                // or wide, pulled by `fold` or by `next`, touches the heap,
                // whatever the dataset's size.
                assert_eq!(small_cost, Measurement::default(), "{how:?}");
                assert_eq!(large_cost, Measurement::default(), "{how:?}");
                println!("owners={owners} singleton={singleton} {how:?}: {large_cost:?}");
            }
        }
    }
}

#[test]
fn delta_carrier_probes_do_not_allocate_by_dataset_size() {
    for owners in [1, 8] {
        let small = delta_composite(10, owners);
        let large = delta_composite(10_000, owners);
        for how in [Pull::Fold, Pull::Next] {
            for (view, rows) in [(&small, 10), (&large, 10_000)] {
                // A base-layer row, a delta-layer row, and the removed base row,
                // whose term the snapshot still names.
                let removed = format!("s{}", rows - 1);
                for (subject, expected) in [("s0", 1), ("late", 1), (removed.as_str(), 0)] {
                    let id = id_of(view, &format!("http://example.org/source0/{subject}"));
                    let (singleton, cost) = measure(view, Some(id), how);
                    assert_eq!(singleton, expected, "{subject} of {rows} rows");
                    assert_eq!(cost, Measurement::default(), "{subject} {how:?}");
                }
            }
        }
        for how in [Pull::Fold, Pull::Next] {
            for (view, rows) in [(&small, 10), (&large, 10_000)] {
                // Each owner lost one base row and gained one delta row.
                let (wide, cost) = measure(view, None, how);
                assert_eq!(wide, rows * owners);
                // Both layers and the overlay's mask are read in place.
                assert_eq!(cost, Measurement::default(), "wide {how:?}");
            }
        }
    }
}

#[test]
fn delta_view_probes_are_compact_and_do_not_allocate() {
    let view = delta(10_000, 0);
    let wide = view.quads_for_pattern(None, None, None, GraphMatch::Any);
    let subject = id_of(view.as_ref(), "http://example.org/source0/late");
    let singleton = view.quads_for_pattern(Some(subject), None, None, GraphMatch::Any);
    let layouts = [size_of_val(&wide), size_of_val(&singleton)];
    println!("delta view cursor bytes wide/singleton: {layouts:?}");
    for bytes in layouts {
        assert!(
            bytes <= DELTA_CURSOR_CAP,
            "unused branches inflated the delta cursor to {bytes} bytes"
        );
    }
    assert_eq!(wide.count(), 10_000);
    assert_eq!(singleton.count(), 1);
    for how in [Pull::Fold, Pull::Next] {
        assert_eq!(
            measure(view.as_ref(), None, how),
            (10_000, Measurement::default())
        );
        assert_eq!(
            measure(view.as_ref(), Some(subject), how),
            (1, Measurement::default())
        );
    }
}

#[test]
fn nested_selection_retains_rows_and_compact_probe_cost() {
    let view = Arc::new(composite(100, 2));
    let graph = TermValue::Iri("http://example.org/g".into());
    let selected = |view| {
        CompositeSource::from_selection(view, [graph.clone()], ViewLimits::default()).unwrap()
    };
    let first = Arc::new(
        CompositeDatasetView::from_bound_sources(vec![selected(view)], ViewLimits::default())
            .unwrap(),
    );
    let nested =
        CompositeDatasetView::from_bound_sources(vec![selected(first)], ViewLimits::default())
            .unwrap();
    let (rows, measured) = measure(&nested, None, Pull::Fold);
    assert_eq!(measure(&nested, None, Pull::Next), (rows, measured));
    assert_eq!(rows, 200);
    // Each selection level boxes the retained composite's cursor once per
    // selected-graph probe (the hop that keeps the recursive type finite): two
    // levels, two compact cursors.
    assert!(
        measured.requested_bytes <= u64::try_from(2 * CURSOR_CAP).unwrap(),
        "nested cursor cost: {measured:?}"
    );
    println!("nested selected probe: {measured:?}");
}

#[test]
fn delta_and_nested_selection_preserve_statement_layers_and_blank_classes() {
    let sources: Vec<_> = (0..2)
        .map(|source| {
            let mut builder = RdfDatasetBuilder::new();
            let subject = builder.intern_iri(&format!("http://example.org/s{source}"));
            let predicate = builder.intern_iri("http://example.org/p");
            let object = builder.intern_iri("http://example.org/o");
            let graph = builder.intern_iri("http://example.org/g");
            let reifier = builder.intern_blank("same", BlankScope::DEFAULT);
            let triple = builder.intern_triple(subject, predicate, object);
            for graph in [None, Some(graph)] {
                builder.push_quad(subject, predicate, object, graph);
                builder.push_reifier_in_graph(reifier, triple, graph);
                builder.push_annotation_in_graph(reifier, predicate, object, graph);
            }
            builder.freeze().unwrap()
        })
        .collect();
    let mut mutation = MutableDataset::new(Arc::clone(&sources[1]));
    let delta_quad = QuadValues {
        s: TermValue::Iri("http://example.org/late".into()),
        p: TermValue::Iri("http://example.org/p".into()),
        o: TermValue::Iri("http://example.org/o".into()),
        g: None,
    };
    assert!(mutation.insert(delta_quad).unwrap());
    let suppressed = QuadValues {
        s: TermValue::Iri("http://example.org/s1".into()),
        p: TermValue::Iri("http://example.org/p".into()),
        o: TermValue::Iri("http://example.org/o".into()),
        g: None,
    };
    assert!(mutation.remove(&suppressed));
    let view = Arc::new(
        CompositeDatasetView::from_bound_sources(
            vec![
                CompositeSource::new(Arc::clone(&sources[0])),
                CompositeSource::from_delta(Arc::new(mutation.snapshot_view().unwrap())),
            ],
            ViewLimits::default(),
        )
        .unwrap(),
    );
    assert_eq!(view.quads().count(), 4);
    let reifiers: Vec<_> = view.reifier_quads().collect();
    assert_eq!(reifiers.len(), 4);
    assert_eq!(view.annotation_quads().count(), 4);
    // The two same-label source blanks stay distinct; both graph occurrences
    // within one source keep their one identity and source-first row order.
    assert_eq!(reifiers[0].s, reifiers[1].s);
    assert_eq!(reifiers[2].s, reifiers[3].s);
    assert_ne!(reifiers[0].s, reifiers[2].s);
    for row in &reifiers {
        assert!(matches!(
            view.resolve(row.s).unwrap(),
            TermRef::Blank { label: "same", .. }
        ));
        assert_eq!(view.reifier_quads_of(row.s).count(), 2);
        assert_eq!(view.annotations_of_with_graph(row.s).count(), 2);
    }
    let graph = TermValue::Iri("http://example.org/g".into());
    let select = |view| {
        CompositeSource::from_selection(view, [graph.clone()], ViewLimits::default()).unwrap()
    };
    let selected = Arc::new(
        CompositeDatasetView::from_bound_sources(vec![select(view)], ViewLimits::default())
            .unwrap(),
    );
    let nested = CompositeDatasetView::from_bound_sources(
        vec![select(selected).with_graph_placement(GraphPlacement::Default)],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(nested.quads().count(), 2);
    assert!(nested.quads().all(|q| q.g.is_none()));
    assert!(nested.reifier_quads().all(|q| q.g.is_none()));
    assert!(nested.annotation_quads().all(|q| q.g.is_none()));
    // Fully consume actual statement cursors inside the allocator window.
    black_box(nested.reifier_quads().count());
    black_box(nested.annotation_quads().count());
    let window = CurrentThreadWindow::open();
    let reifiers = nested.reifier_quads().count();
    let annotations = nested.annotation_quads().count();
    let measured = window.close();
    assert_eq!((reifiers, annotations), (2, 2));
    assert_eq!(measured.retained_bytes, 0);
    assert!(
        measured.requested_bytes <= 16_384,
        "nested statement cursor cost: {measured:?}"
    );
    println!("delta/nested-selection statement probe: {measured:?}");
}
