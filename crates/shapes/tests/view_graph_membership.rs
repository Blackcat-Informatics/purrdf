// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A SHACL data view's `has_named_graph` agrees with its `named_graphs` over every
//! source it reads, including behind the projection gate that empties the
//! enumeration of a projected or graph-selected view.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::term_fixture::{graph_slots, iri};
use purrdf_rdf::ir::{CompositeDatasetView, ViewLimits};
use purrdf_rdf::{DatasetMut, DatasetView, MutableDataset, QuadValues, TermId};
use purrdf_shapes::data_view::ShaclDatasetView;

/// The law over every id a quad or graph enumeration hands out, and the exact
/// graph names the view reports.
fn assert_agrees(view: &ShaclDatasetView, label: &str, expected: &[&str]) {
    let graphs: BTreeSet<TermId> = view.named_graphs().collect();
    let mut candidates = graphs.clone();
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
    for local in ["g", "empty", "phantom", "delta-g"] {
        if let Some(id) = view.term_id_by_value(&iri(local)).unwrap() {
            assert_eq!(
                view.has_named_graph(id),
                expected.contains(&local),
                "{label}: <{local}>"
            );
        }
    }
}

#[test]
fn shacl_views_answer_membership_like_their_enumeration() {
    let source = graph_slots("");
    assert_agrees(
        &ShaclDatasetView::native(Arc::clone(&source)),
        "native",
        &["g", "empty"],
    );
    // The projection gate: a projected view and a graph-selected view expose no
    // named graph, so no id is one, however the source answers.
    assert_agrees(
        &ShaclDatasetView::project(Arc::clone(&source)),
        "projected",
        &[],
    );
    let g = source.term_id_by_iri("http://example.org/g").expect("g");
    assert_agrees(
        &ShaclDatasetView::named_graph(Arc::clone(&source), g),
        "selected",
        &[],
    );

    let mut mutable = MutableDataset::new(Arc::clone(&source));
    mutable
        .insert(QuadValues {
            s: iri("s"),
            p: iri("p"),
            o: iri("o"),
            g: Some(iri("delta-g")),
        })
        .expect("insert applies");
    let delta = Arc::new(mutable.snapshot_view().expect("snapshot"));
    for projected in [false, true] {
        let view = ShaclDatasetView::delta(Arc::clone(&delta), projected, ViewLimits::default())
            .expect("delta view");
        let expected: &[&str] = if projected {
            &[]
        } else {
            &["g", "empty", "delta-g"]
        };
        assert_agrees(&view, &format!("delta projected={projected}"), expected);
    }

    let composite =
        Arc::new(CompositeDatasetView::new(vec![source], ViewLimits::default()).expect("composes"));
    for projected in [false, true] {
        let view =
            ShaclDatasetView::composite(Arc::clone(&composite), projected, ViewLimits::default())
                .expect("composite view");
        let expected: &[&str] = if projected { &[] } else { &["g", "empty"] };
        assert_agrees(&view, &format!("composite projected={projected}"), expected);
    }
}
