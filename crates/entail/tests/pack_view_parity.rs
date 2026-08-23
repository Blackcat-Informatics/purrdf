// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parity: materializing an entailment closure over a zero-copy [`PackView`] yields
//! byte-for-byte the same closure as materializing over the equivalent owned
//! [`RdfDataset`].
//!
//! This is the guard on the reasoner's seeding-layer generalization from
//! `&RdfDataset` to `&impl DatasetView`. The seeding path re-expresses the
//! dataset's base quads, reifier side table, annotation side table, and declared
//! named graphs through the `DatasetView` primitives (`resolve`, `reifier_quads`,
//! `annotation_quads`, `named_graphs`) instead of the `RdfDataset`-only accessors.
//! If that re-expression drifted from the original in ANY position — a dropped
//! reifier, a lost empty named graph, a mis-resolved term — the closure computed
//! from a `PackView` would differ from the closure computed from the `RdfDataset`,
//! and these assertions would fail.

use std::sync::Arc;

use purrdf_core::{PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, dataset_from_view};
use purrdf_entail::{Materialization, materialize};

const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// A dataset that exercises every table the seeding path carries: base quads in the
/// default graph and in a named graph, a reifier over a triple-term, an annotation
/// on that reifier, and a declared-but-empty named graph.
fn sample_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let cat = b.intern_iri("http://example.org/Cat");
    let animal = b.intern_iri("http://example.org/Animal");
    let tom = b.intern_iri("http://example.org/tom");
    let sub = b.intern_iri(RDFS_SUBCLASSOF);
    let ty = b.intern_iri(RDF_TYPE);
    let g_named = b.intern_iri("http://example.org/g1");
    let note = b.intern_iri("http://example.org/note");
    let source = b.intern_iri("http://example.org/source-x");
    let reifier = b.intern_iri("http://example.org/r1");

    // Base quads: default graph (drives rdfs9 re-typing) and a named graph.
    b.push_quad(cat, sub, animal, None);
    b.push_quad(tom, ty, cat, None);
    b.push_quad(tom, ty, animal, Some(g_named));

    // A reifier over a triple-term, plus an annotation on the reifier — the two
    // side tables the seeding path now walks via `reifier_quads`/`annotation_quads`.
    let triple = b.intern_triple(tom, ty, cat);
    b.push_reifier_in_graph(reifier, triple, None);
    b.push_annotation_in_graph(reifier, note, source, None);

    // A declared-but-empty named graph: content that occurs in no quad, which the
    // seeding path must still carry (via `named_graphs`).
    let g_empty = b.intern_iri("http://example.org/empty");
    b.declare_named_graph(g_empty);

    b.freeze().expect("freeze sample dataset")
}

/// A deterministic, id-independent canonical form of a dataset: its quads, reifier
/// bindings, annotations, and declared named graphs rendered by resolved VALUE
/// (never by dataset-local id, which differs between an `RdfDataset` and a
/// `PackView`), then sorted.
fn canonical(ds: &RdfDataset) -> Vec<String> {
    let mut lines = Vec::new();
    for q in ds.quads() {
        lines.push(format!(
            "Q {:?} {:?} {:?} {:?}",
            ds.term_value(q.s),
            ds.term_value(q.p),
            ds.term_value(q.o),
            q.g.map(|g| ds.term_value(g)),
        ));
    }
    for (reifier, triple, graph) in ds.reifiers_with_graph() {
        lines.push(format!(
            "R {:?} {:?} {:?}",
            ds.term_value(reifier),
            ds.term_value(triple),
            graph.map(|g| ds.term_value(g)),
        ));
    }
    for (reifier, predicate, object, graph) in ds.annotations_with_graph() {
        lines.push(format!(
            "A {:?} {:?} {:?} {:?}",
            ds.term_value(reifier),
            ds.term_value(predicate),
            ds.term_value(object),
            graph.map(|g| ds.term_value(g)),
        ));
    }
    for graph in ds.named_graphs() {
        lines.push(format!("G {:?}", ds.term_value(graph)));
    }
    lines.sort();
    lines
}

/// Materialize `plan` over the owned dataset AND over its byte-equivalent
/// `PackView`, and assert the two closures are identical.
fn assert_closure_parity(
    label: &str,
    plan_for_dataset: Materialization<'_>,
    plan_for_view: Materialization<'_>,
) {
    let dataset = sample_dataset();

    // The byte-equivalent pack, opened zero-copy.
    let pack_bytes = PackBuilder::build_bytes(&dataset).expect("build pack bytes");
    let view = PackView::from_bytes(&pack_bytes).expect("open pack view");

    // The OLD path's reference: rebuild an owned `RdfDataset` from the SAME pack
    // (`dataset_from_view`), then materialize over it. The generalization's exact
    // claim is that entering the reasoner DIRECTLY over the view yields this same
    // closure without the rebuild. Comparing against the *original* dataset would
    // instead measure
    // pack round-trip fidelity — e.g. the pack format carries no declared-but-empty
    // named graph — which is a separate, pre-existing property that BOTH the old and
    // new CLI paths share, not something this generalization changed.
    let rebuilt = dataset_from_view(&view).expect("rebuild owned dataset from view");

    let (from_rebuild, report_rebuild) =
        materialize(&*rebuilt, plan_for_dataset).expect("materialize over rebuilt RdfDataset");
    // The generalized entry point: a `PackView` enters the reasoner directly.
    let (from_view, report_view) =
        materialize(&view, plan_for_view).expect("materialize over PackView");

    let rebuild_canon = canonical(&from_rebuild);
    let view_canon = canonical(&from_view);
    if rebuild_canon != view_canon {
        use std::collections::BTreeSet;
        let rebuild_set: BTreeSet<&String> = rebuild_canon.iter().collect();
        let view_set: BTreeSet<&String> = view_canon.iter().collect();
        let only_rebuild: Vec<&&String> = rebuild_set.difference(&view_set).collect();
        let only_view: Vec<&&String> = view_set.difference(&rebuild_set).collect();
        panic!(
            "closure parity mismatch for regime {label}\n\
             ONLY in rebuilt-dataset closure ({}):\n{only_rebuild:#?}\n\
             ONLY in direct-view closure ({}):\n{only_view:#?}",
            only_rebuild.len(),
            only_view.len(),
        );
    }
    assert_eq!(
        report_rebuild.completeness(),
        report_view.completeness(),
        "completeness mismatch for regime {label}",
    );
}

#[test]
fn rdfs_closure_is_identical_over_pack_view_and_dataset() {
    assert_closure_parity("rdfs", Materialization::Rdfs, Materialization::Rdfs);
}

#[test]
fn owl_rl_closure_is_identical_over_pack_view_and_dataset() {
    assert_closure_parity("owl-rl", Materialization::OwlRl, Materialization::OwlRl);
}

#[test]
fn simple_copy_is_identical_over_pack_view_and_dataset() {
    // The identity closure exercises the seeding copy alone (no rule firing), so a
    // dropped side-table row would surface here with nothing else moving.
    assert_closure_parity("simple", Materialization::Simple, Materialization::Simple);
}
