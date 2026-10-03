// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compact borrowed probe cursors and dataset-independent allocation costs.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::{
    BlankScope, CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, GraphMatch,
    GraphPlacement, MutableDataset, QuadValues, RdfDatasetBuilder, TermRef, TermValue, ViewLimits,
};
use std::hint::black_box;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn composite(rows: usize, owners: usize) -> CompositeDatasetView {
    let sources = (0..owners)
        .map(|owner| {
            let mut builder = RdfDatasetBuilder::new();
            let predicate = builder.intern_iri("http://example.org/p");
            let object = builder.intern_iri("http://example.org/o");
            let graph = builder.intern_iri("http://example.org/g");
            for row in 0..rows {
                let subject =
                    builder.intern_iri(&format!("http://example.org/source{owner}/s{row}"));
                builder.push_quad(subject, predicate, object, Some(graph));
            }
            CompositeSource::new(builder.freeze().expect("valid source"))
        })
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
            bytes <= 256 * size_of::<usize>(),
            "inactive branches inflated cursor to {bytes} bytes"
        );
    }
    assert_eq!(ordinary.count(), 10);
    assert_eq!(reifiers.count(), 0);
    assert_eq!(annotations.count(), 0);
}

fn measure(
    view: &CompositeDatasetView,
    subject: Option<purrdf_core::CompositeViewId>,
) -> (usize, Measurement) {
    black_box(
        view.quads_for_pattern(subject, None, None, GraphMatch::Any)
            .count(),
    );
    let window = CurrentThreadWindow::open();
    let rows = view
        .quads_for_pattern(subject, None, None, GraphMatch::Any)
        .count();
    (rows, window.close())
}

#[test]
fn singleton_and_wide_probes_do_not_allocate_by_dataset_size() {
    for owners in [1, 8] {
        let small = composite(10, owners);
        let large = composite(10_000, owners);
        let needle = TermValue::Iri("http://example.org/source0/s0".into());
        let small_id = small.term_id_by_value(&needle).unwrap().unwrap();
        let large_id = large.term_id_by_value(&needle).unwrap().unwrap();
        for singleton in [false, true] {
            let (small_rows, small_cost) = measure(&small, singleton.then_some(small_id));
            let (large_rows, large_cost) = measure(&large, singleton.then_some(large_id));
            assert_eq!(small_rows, if singleton { 1 } else { 10 * owners });
            assert_eq!(large_rows, if singleton { 1 } else { 10_000 * owners });
            assert_eq!(large_cost.requested_bytes, small_cost.requested_bytes);
            assert_eq!(large_cost.allocations, small_cost.allocations);
            assert!(
                large_cost.requested_bytes
                    <= u64::try_from(owners * 256 * size_of::<usize>()).unwrap()
            );
            println!("owners={owners} singleton={singleton}: {large_cost:?}");
        }
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
    let (rows, measured) = measure(&nested, None);
    assert_eq!(rows, 200);
    assert!(
        measured.requested_bytes <= 16_384,
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
        measured.requested_bytes <= 32_768,
        "nested statement cursor cost: {measured:?}"
    );
    println!("delta/nested-selection statement probe: {measured:?}");
}
