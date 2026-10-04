// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`DatasetView::has_named_graph`] agrees with [`DatasetView::named_graphs`] on every
//! carrier that answers membership without enumerating.
//!
//! The law, for every view `V` and every id `t` the view hands out:
//!
//! ```text
//! V.has_named_graph(t) == V.named_graphs().any(|g| g == t)
//! ```
//!
//! Each fixture holds a term that names no graph (an object in the default graph)
//! next to a quad-bearing graph and a declared-empty graph, so an override that
//! answered "is this a term" — or dropped declared-empty graphs — fails here.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::sync::Arc;

use purrdf_core::term_fixture::{graph_slots as fixture, iri};
use purrdf_core::{
    CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, GraphPlacement,
    InMemoryPageProvider, MutableDataset, PagedDataset, PagedQueryLimits, QuadValues, ViewLimits,
};

/// Check the law over every id a quad or graph enumeration hands out, and that
/// `expected` names exactly the graphs the view reports.
fn assert_agrees<V: DatasetView>(view: &V, label: &str, expected: &[&str], absent: &[&str])
where
    V::Id: Debug,
{
    let graphs: BTreeSet<V::Id> = view.named_graphs().collect();
    let mut candidates: BTreeSet<V::Id> = graphs.clone();
    for q in view.quads() {
        candidates.extend([q.s, q.p, q.o]);
        candidates.extend(q.g);
    }
    for id in candidates {
        assert_eq!(
            view.has_named_graph(id),
            graphs.contains(&id),
            "{label}: has_named_graph({id:?}) disagrees with named_graphs"
        );
    }
    for local in expected {
        let id = view
            .term_id_by_value(&iri(local))
            .unwrap()
            .unwrap_or_else(|| panic!("{label}: <{local}> is a term"));
        assert!(view.has_named_graph(id), "{label}: <{local}> names a graph");
    }
    for local in absent {
        let id = view
            .term_id_by_value(&iri(local))
            .unwrap()
            .unwrap_or_else(|| panic!("{label}: fixture is degenerate — <{local}> is a term"));
        assert!(
            !view.has_named_graph(id),
            "{label}: <{local}> names no graph"
        );
    }
}

#[test]
fn frozen_dataset_membership_agrees() {
    let dataset = fixture("");
    assert_agrees(&*dataset, "frozen", &["g", "empty"], &["phantom", "s", "o"]);
    assert_agrees(&dataset, "frozen/arc", &["g", "empty"], &["phantom"]);
}

#[test]
fn delta_snapshot_membership_agrees() {
    let mut mutable = MutableDataset::new(fixture(""));
    for (o, g) in [
        ("delta-phantom", None),
        ("o", Some("delta-g")),
        ("o", Some("g")),
    ] {
        mutable
            .insert(QuadValues {
                s: iri("s"),
                p: iri("p"),
                o: iri(o),
                g: g.map(iri),
            })
            .expect("insert applies");
    }
    let view = mutable.snapshot_view().expect("snapshot builds");
    assert_agrees(
        &view,
        "delta",
        &["g", "empty", "delta-g"],
        &["phantom", "delta-phantom"],
    );
}

#[test]
fn composite_membership_agrees_under_every_placement_and_selection() {
    let preserve =
        CompositeDatasetView::new(vec![fixture("A"), fixture("B")], ViewLimits::default())
            .expect("two sources compose");
    assert_agrees(
        &preserve,
        "composite",
        &["gA", "emptyA", "gB", "emptyB"],
        &["phantomA", "phantomB"],
    );

    let placed = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::new(fixture("A")).with_graph_placement(GraphPlacement::Default),
            CompositeSource::new(fixture("B"))
                .with_graph_placement(GraphPlacement::Named(iri("placed"))),
            CompositeSource::new(fixture("C")),
        ],
        ViewLimits::default(),
    )
    .expect("placed sources compose");
    assert_agrees(
        &placed,
        "composite/placement",
        &["placed", "gC", "emptyC"],
        &["gA", "emptyA", "gB", "emptyB", "phantomC"],
    );

    let mut mutable = MutableDataset::new(fixture("D"));
    mutable
        .insert(QuadValues {
            s: iri("s"),
            p: iri("p"),
            o: iri("o"),
            g: Some(iri("delta-g")),
        })
        .expect("insert applies");
    let retained = Arc::new(
        CompositeDatasetView::new(vec![fixture("A")], ViewLimits::default())
            .expect("one source composes"),
    );
    let mixed = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::from_delta(Arc::new(mutable.snapshot_view().expect("snapshot"))),
            CompositeSource::from_selection(retained, [iri("emptyA")], ViewLimits::default())
                .expect("the selected graph is held"),
        ],
        ViewLimits::default(),
    )
    .expect("a delta and a selection compose");
    assert_agrees(
        &mixed,
        "composite/delta+selection",
        &["gD", "emptyD", "delta-g", "emptyA"],
        &["phantomD"],
    );
}

#[test]
fn paged_membership_agrees() {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![
        fixture("A"),
        fixture("B"),
    ])))
    .expect("seal pages");
    let expected = ["gA", "emptyA", "gB", "emptyB"];
    let absent = ["phantomA", "phantomB"];
    assert_agrees(&paged, "paged", &expected, &absent);
    let view = paged.query_view(PagedQueryLimits::new(u64::MAX, u64::MAX));
    assert_agrees(&view, "paged/query-view", &expected, &absent);
}
