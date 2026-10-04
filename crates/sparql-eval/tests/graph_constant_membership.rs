// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `GRAPH <iri> { ... }` addresses a named graph only when the dataset has one by
//! that name.
//!
//! An IRI the dataset merely knows as a term — an object in the default graph, the
//! old name of a graph a composite projected away — names no graph, so the block is
//! empty there, even for an inner pattern that answers without reading a row (`BIND`
//! over the empty group). A real named graph, quad-bearing or declared empty, still
//! answers. The constant form agrees with `GRAPH ?g`, which ranges over exactly
//! [`DatasetView::named_graphs`](purrdf_core::DatasetView::named_graphs).
//!
//! Each case runs over a frozen dataset, a mutable dataset's delta snapshot and a
//! composite view.

use purrdf_core::term_fixture::{graph_slots, iri};
use purrdf_core::{
    CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, GraphPlacement, MutableDataset,
    QuadValues, SparqlResult, TermValue, ViewLimits,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

/// How many rows `SELECT * { GRAPH <local> { BIND(1 AS ?x) } }` yields over `view`.
fn constant_graph_rows<D>(view: &D, local: &str) -> usize
where
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
{
    let engine = NativeSparqlEngine::new();
    let plan = engine
        .prepare_query(
            &format!("SELECT * WHERE {{ GRAPH <{EX}{local}> {{ BIND(1 AS ?x) }} }}"),
            None,
        )
        .expect("query prepares");
    match engine
        .query_prepared_view(view, &plan, &[], QueryOptions::EMPTY)
        .expect("query evaluates")
    {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        _ => panic!("a SELECT returns solutions"),
    }
}

/// The graph names `GRAPH ?g {}` binds over `view`, so each constant case can be
/// checked against the variable form.
fn enumerated_graphs<D>(view: &D) -> Vec<TermValue>
where
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
{
    let engine = NativeSparqlEngine::new();
    let plan = engine
        .prepare_query("SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g", None)
        .expect("query prepares");
    match engine
        .query_prepared_view(view, &plan, &[], QueryOptions::EMPTY)
        .expect("query evaluates")
    {
        SparqlResult::Solutions { rows, .. } => rows
            .into_iter()
            .map(|row| row[0].clone().expect("?g is bound"))
            .collect(),
        _ => panic!("a SELECT returns solutions"),
    }
}

/// Every `phantoms` name yields no row and every `graphs` name yields one, and the
/// constant form agrees with what `GRAPH ?g` enumerates.
fn assert_membership<D>(view: &D, label: &str, phantoms: &[&str], graphs: &[&str])
where
    D: DatasetView<ReadError = std::convert::Infallible> + Sync,
{
    let enumerated = enumerated_graphs(view);
    for local in phantoms {
        assert!(
            view.term_id_by_value(&iri(local)).unwrap().is_some(),
            "{label}: fixture is degenerate — <{local}> must be a term of the view"
        );
        assert!(
            !enumerated.contains(&iri(local)),
            "{label}: <{local}> enumerated"
        );
        assert_eq!(
            constant_graph_rows(view, local),
            0,
            "{label}: GRAPH <{local}> names no graph and must yield no row"
        );
    }
    for local in graphs {
        assert!(
            enumerated.contains(&iri(local)),
            "{label}: <{local}> not enumerated"
        );
        assert_eq!(
            constant_graph_rows(view, local),
            1,
            "{label}: GRAPH <{local}> names a graph and must yield its one row"
        );
    }
}

#[test]
fn frozen_dataset_addresses_only_named_graphs() {
    let dataset = graph_slots("");
    assert_membership(&*dataset, "frozen", &["phantom", "s", "p"], &["g", "empty"]);
    assert_membership(&dataset, "frozen/arc", &["phantom"], &["g", "empty"]);
}

#[test]
fn delta_snapshot_addresses_only_named_graphs() {
    let mut mutable = MutableDataset::new(graph_slots(""));
    // A term the delta alone introduces, as a default-graph object only, and a graph
    // the delta alone introduces.
    for (o, g) in [("delta-phantom", None), ("o", Some("delta-g"))] {
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
    assert_membership(
        &view,
        "delta",
        &["phantom", "delta-phantom"],
        &["g", "empty", "delta-g"],
    );
}

#[test]
fn composite_view_addresses_only_named_graphs() {
    let view = CompositeDatasetView::new(vec![graph_slots("")], ViewLimits::default())
        .expect("one source composes");
    assert_membership(&view, "composite", &["phantom"], &["g", "empty"]);

    // Projecting the source into the default graph leaves `g` and `empty` as terms
    // that no longer name a graph; the placed name does.
    let projected = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::new(graph_slots("")).with_graph_placement(GraphPlacement::Default),
            CompositeSource::new(graph_slots(""))
                .with_graph_placement(GraphPlacement::Named(iri("placed"))),
        ],
        ViewLimits::default(),
    )
    .expect("placed sources compose");
    assert_membership(
        &projected,
        "composite/placement",
        &["phantom", "g", "empty"],
        &["placed"],
    );
}
