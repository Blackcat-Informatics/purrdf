// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Draining a graph declared on a `MutableDataset` row by row stays linear.
//!
//! Each removal decides whether it emptied the graph, because the removal of a
//! declared graph's last row withdraws the declaration. That decision reads the
//! graph's live row count; it must never re-walk the dataset, which would make the
//! drain quadratic. The proof is operational: the drain runs inside a counting
//! allocator window, and a re-walk of the delta's rows allocates its replay order on
//! every removal. The live-count bookkeeping allocates nothing per removal, so the
//! drain's allocator traffic must stay within a budget that does not grow with the
//! number of rows drained.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::term_fixture::iri;
use purrdf_core::{
    DatasetMut, GraphExistenceMode, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder,
    TermValue,
};

// A current-thread window: sibling tests run on other threads of this process.
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const EX: &str = "http://example.org/";

/// A base with default-graph rows and one populated named graph, so a re-walk
/// would also pay for rows the drained graph does not own.
fn base() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{EX}p"));
    let g = b.intern_iri(&format!("{EX}base-graph"));
    for n in 0..256 {
        let s = b.intern_iri(&format!("{EX}s{n}"));
        let o = b.intern_iri(&format!("{EX}o{n}"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g));
    }
    b.freeze().expect("drain base")
}

fn rows_in(graph: &TermValue, rows: usize) -> Vec<QuadValues> {
    (0..rows)
        .map(|n| {
            QuadValues::quad(
                iri(&format!("d{n}")),
                iri("p"),
                iri(&format!("e{n}")),
                graph.clone(),
            )
        })
        .collect()
}

/// Drain `quads` one by one, asserting the graph stays declared until its last row
/// leaves and is withdrawn by exactly that removal; the allocator traffic of the
/// removals alone.
fn drain(mutable: &mut MutableDataset, graph: &TermValue, quads: &[QuadValues]) -> u64 {
    let declared =
        |m: &MutableDataset| m.declared_named_graphs().any(|declared| declared == *graph);
    let mut requested = 0;
    for (index, quad) in quads.iter().enumerate() {
        assert!(
            declared(mutable),
            "row {index}: the graph is still declared"
        );
        let window = CurrentThreadWindow::open();
        let removed = mutable.remove(quad);
        requested += window.close().requested_bytes;
        assert!(removed, "row {index} was present");
    }
    assert!(
        !declared(mutable),
        "removing the declared graph's last row withdraws its declaration"
    );
    requested
}

#[test]
fn draining_a_declared_graph_does_not_rewalk_the_dataset_per_removal() {
    const ROWS: usize = 2048;
    let base = base();
    for graph in [iri("declared"), TermValue::blank("declared")] {
        let mut mutable = MutableDataset::new(Arc::clone(&base));
        assert_eq!(mutable.declare_named_graph(graph.clone()), Ok(true));
        let quads = rows_in(&graph, ROWS);
        for quad in &quads {
            assert!(mutable.insert(quad.clone()).expect("absolute rows"));
        }
        let requested = drain(&mut mutable, &graph, &quads);
        // A re-walk replays every remaining delta row per removal: about
        // ROWS² / 2 keys, tens of MiB here. The budget is a few bytes per row.
        assert!(
            requested < 64 * ROWS as u64,
            "{graph:?}: draining {ROWS} rows requested {requested} bytes"
        );
    }
}

#[test]
fn emptiness_is_decided_per_graph_and_by_the_graphs_own_rows() {
    let base = base();
    let mut mutable = MutableDataset::new(Arc::clone(&base));
    let declared = iri("declared");
    let other = iri("other");
    assert_eq!(mutable.declare_named_graph(declared.clone()), Ok(true));
    assert_eq!(mutable.declare_named_graph(other.clone()), Ok(true));
    let declared_rows = rows_in(&declared, 8);
    let other_rows = rows_in(&other, 8);
    for quad in declared_rows.iter().chain(&other_rows) {
        assert!(mutable.insert(quad.clone()).expect("absolute rows"));
    }
    // The same subject, predicate and object in another graph, and a removal of a
    // row that is not there, leave the declared graph's count untouched.
    assert!(!mutable.remove(&QuadValues::quad(
        iri("d0"),
        iri("p"),
        iri("e0"),
        iri("absent")
    )));
    drain(&mut mutable, &declared, &declared_rows);
    assert!(
        mutable.declared_named_graphs().any(|g| g == other),
        "emptying one declared graph keeps another that still holds rows"
    );
    // Repopulated and drained again, the graph's count starts from its rows anew.
    assert_eq!(mutable.declare_named_graph(declared.clone()), Ok(true));
    for quad in &declared_rows {
        assert!(mutable.insert(quad.clone()).expect("absolute rows"));
    }
    drain(&mut mutable, &declared, &declared_rows);
    // A base graph follows the same rule through its own live row count.
    let base_graph = iri("base-graph");
    let base_rows: Vec<_> = (0..256)
        .map(|n| {
            QuadValues::quad(
                iri(&format!("s{n}")),
                iri("p"),
                iri(&format!("o{n}")),
                base_graph.clone(),
            )
        })
        .collect();
    assert!(mutable.declared_named_graphs().any(|g| g == base_graph));
    drain(&mut mutable, &base_graph, &base_rows);
    assert_eq!(mutable.freeze().expect("drained").named_graphs().count(), 1);
}

#[test]
fn remembered_slot_drain_has_no_per_row_allocation() {
    const ROWS: usize = 2048;
    let base = base();
    for graph in [iri("remembered"), TermValue::blank("remembered")] {
        let mut mutable = MutableDataset::new_with_graph_existence(
            Arc::clone(&base),
            GraphExistenceMode::RememberEmpty,
        );
        assert!(
            mutable
                .create_named_graph(graph.clone())
                .expect("fresh graph")
        );
        let quads = rows_in(&graph, ROWS);
        for quad in &quads {
            assert!(mutable.insert(quad.clone()).expect("absolute rows"));
        }
        let window = CurrentThreadWindow::open();
        for quad in &quads {
            assert!(mutable.remove(quad));
        }
        let requested = window.close().requested_bytes;
        assert_eq!(requested, 0, "retained slot removal allocates no replay");
        assert!(mutable.has_named_graph(&graph));
        let frozen = mutable.freeze().expect("drained graph retains its slot");
        assert!(
            frozen
                .named_graphs()
                .any(|id| frozen.term_value(id) == graph)
        );
        mutable.withdraw_graph_declaration(&graph);
        assert!(!mutable.has_named_graph(&graph));
    }
}

fn measured_named_capability(view: &impl purrdf_core::DatasetView) -> (bool, u64) {
    let window = CurrentThreadWindow::open();
    let present = std::hint::black_box(view).capabilities().named_graphs;
    (present, window.close().requested_bytes)
}

#[test]
fn named_graph_capability_reads_do_not_allocate_the_registry() {
    use purrdf_core::DatasetView;

    const BASE_GRAPHS: usize = 512;
    let mut builder = RdfDatasetBuilder::new();
    for index in 0..BASE_GRAPHS {
        let graph = builder.intern_iri(&format!("{EX}cap-{index}"));
        builder.declare_named_graph(graph);
    }
    let mut mutable = MutableDataset::new_with_graph_existence(
        builder.freeze().expect("empty base graph declarations"),
        GraphExistenceMode::RememberEmpty,
    );
    let delta_graph = iri("cap-delta");
    mutable
        .create_named_graph(delta_graph.clone())
        .expect("delta declaration");
    // This base graph also occurs in the delta layer. Enumeration must keep one
    // ordered ID for it, while the capability only needs an existence probe.
    let row = QuadValues::quad(iri("cap-s"), iri("cap-p"), iri("cap-o"), iri("cap-0"));
    assert!(mutable.insert(row.clone()).expect("delta row"));
    let retained = mutable.snapshot_view().expect("populated snapshot");
    let ids: Vec<_> = retained.named_graphs().collect();
    assert_eq!(ids.len(), BASE_GRAPHS + 1);
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let mut actual_names: Vec<_> = ids.iter().map(|&id| retained.term_value(id)).collect();
    actual_names.sort();
    let mut expected_names: Vec<_> = (0..BASE_GRAPHS)
        .map(|index| iri(&format!("cap-{index}")))
        .chain([delta_graph.clone()])
        .collect();
    expected_names.sort();
    assert_eq!(actual_names, expected_names);
    assert_eq!(retained.quads().count(), 1);
    let initial = measured_named_capability(&retained);

    // Declaration withdrawal keeps populated graphs, so remove the row first.
    assert!(mutable.remove(&row));
    mutable.withdraw_named_graph_declarations();
    let empty = mutable.snapshot_view().expect("withdrawn snapshot");
    assert_eq!(empty.quads().count(), 0);
    assert_eq!(empty.named_graphs().count(), 0);
    let withdrawn = measured_named_capability(&empty);
    let old = measured_named_capability(&retained);
    assert_eq!(retained.quads().count(), 1);
    assert_eq!(retained.named_graphs().count(), BASE_GRAPHS + 1);

    assert!(
        mutable
            .create_named_graph(delta_graph)
            .expect("restored declaration")
    );
    let restored = mutable.snapshot_view().expect("restored snapshot");
    assert_eq!(restored.quads().count(), 0);
    assert_eq!(restored.named_graphs().count(), 1);
    let restored = measured_named_capability(&restored);
    let observations = [initial, withdrawn, old, restored];
    assert_eq!(
        observations.map(|(present, _)| present),
        [true, false, true, true]
    );
    assert_eq!(
        observations.map(|(_, requested)| requested),
        [0; 4],
        "capability reads allocate neither a registry nor its replay order"
    );
}
