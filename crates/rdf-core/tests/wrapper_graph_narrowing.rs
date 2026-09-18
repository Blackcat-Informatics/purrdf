// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The graph-narrowing seam must survive every generic `DatasetView` wrapper.
//!
//! [`DatasetView::reifier_quads_in_graph`] and
//! [`DatasetView::annotation_quads_in_graph`] are optimization seams: a carrier that
//! can name, without materializing anything, which of its storage units could hold a
//! row in `g` overrides them to visit only those units. A wrapper that takes the
//! trait DEFAULT silently reinstates a whole-table scan over whatever it wraps, and
//! nothing about the ROWS changes when it does — the default IS the filter — so only
//! a test that pins the equivalence (here) and a test that pins what was VISITED
//! (`ir::pipeline_bundle`'s residue page-touch test) can tell the two apart.
//!
//! The law, for every wrapper `W`, inner view `V` and graph `g`:
//!
//! ```text
//! W(V).reifier_quads_in_graph(g) == W(V).reifier_quads().filter(|q| g.matches(q.g))
//! ```
//!
//! as a SEQUENCE, not a set — same multiset, same order. Likewise for annotations.
//! The whole risk in overriding these on a wrapper is dropping the wrapper's own row
//! predicate (residue keeping, composition dedup, overlay masking), so every fixture
//! here exercises that predicate rather than an empty wrapper.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, GraphMatch,
    GraphPlacement, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder, ScopeBinding,
    TermValue, ViewLimits,
};

const P: &str = "http://example.org/p";
const CONFIDENCE: &str = "http://example.org/confidence";
const HIGH: &str = "http://example.org/high";
const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{local}"))
}

/// An RDF 1.2 fixture whose reifier AND annotation side tables carry rows in the
/// DEFAULT graph and in two NAMED graphs, over a quoted triple that also exists as an
/// ordinary quad in each of those graphs.
///
/// `tag` distinguishes two otherwise identical carriers: `None` makes every subject
/// shared, so composing two such sources forces the earlier-source deduplication path;
/// a `Some(tag)` source shares only the graph names, so composition keeps both rows.
fn statement_layers(tag: Option<&str>) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let local = |name: &str| match tag {
        Some(tag) => format!("http://example.org/{tag}-{name}"),
        None => format!("http://example.org/{name}"),
    };
    let s = b.intern_iri(&local("s"));
    let p = b.intern_iri(P);
    let o = b.intern_iri(&local("o"));
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri(&local("r"));
    let confidence = b.intern_iri(CONFIDENCE);
    let high = b.intern_iri(HIGH);
    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");
    for graph in [None, Some(g1), Some(g2)] {
        b.push_quad(s, p, o, graph);
        b.push_reifier_in_graph(r, triple, graph);
        b.push_annotation_in_graph(r, confidence, high, graph);
    }
    // A declaration-only graph: named, held, and empty in both side tables.
    let empty = b.intern_iri("http://example.org/empty");
    b.declare_named_graph(empty);
    b.freeze().expect("statement-layer fixture freezes")
}

/// Every graph constraint worth probing on `view`: `Any`, the default graph, each
/// named graph the view holds, and a handle that exists but never occupies a graph
/// slot (the neighbouring case that must yield nothing without being refused).
fn graph_probes<D: DatasetView>(view: &D) -> Vec<GraphMatch<D::Id>> {
    let mut probes = vec![GraphMatch::Any, GraphMatch::Default];
    probes.extend(view.named_graphs().map(GraphMatch::Named));
    probes.extend(
        view.term_id_by_value(&TermValue::iri(P))
            .map(GraphMatch::Named),
    );
    probes
}

/// Assert the narrowing law on `view` over every probe, and that the fixture is not
/// degenerate: at least one NAMED probe and the DEFAULT probe each yield rows on both
/// side tables, so a body that returned nothing could not pass.
fn assert_graph_seam_agrees<D: DatasetView>(view: &D, label: &str) {
    let mut named_rows = 0_usize;
    let mut default_rows = 0_usize;
    for g in graph_probes(view) {
        let narrowed: Vec<_> = view.reifier_quads_in_graph(g).collect();
        let filtered: Vec<_> = view.reifier_quads().filter(|q| g.matches(q.g)).collect();
        assert_eq!(
            narrowed, filtered,
            "{label}: reifier_quads_in_graph({g:?}) must equal the filtered unkeyed stream"
        );
        let annotated: Vec<_> = view.annotation_quads_in_graph(g).collect();
        let expected: Vec<_> = view.annotation_quads().filter(|q| g.matches(q.g)).collect();
        assert_eq!(
            annotated, expected,
            "{label}: annotation_quads_in_graph({g:?}) must equal the filtered unkeyed stream"
        );
        match g {
            GraphMatch::Named(_) => named_rows += narrowed.len() + annotated.len(),
            GraphMatch::Default => default_rows += narrowed.len() + annotated.len(),
            GraphMatch::Any => {}
        }
    }
    assert!(
        named_rows > 0,
        "{label}: fixture is degenerate — no NAMED graph holds a side-table row"
    );
    assert!(
        default_rows > 0,
        "{label}: fixture is degenerate — the DEFAULT graph holds no side-table row"
    );
}

/// A composition of two sources that genuinely overlap: the identical source twice
/// forces the earlier-source deduplication predicate on every row, and the distinct
/// third source keeps the composition from collapsing to one carrier's rows.
#[test]
fn composite_graph_seam_matches_the_filtered_stream() {
    let shared = statement_layers(None);
    let view = CompositeDatasetView::new(
        vec![
            Arc::clone(&shared),
            Arc::clone(&shared),
            statement_layers(Some("other")),
        ],
        ViewLimits::default(),
    )
    .expect("three sources compose");
    // The composition really does deduplicate: two identical carriers plus one
    // distinct one contribute the rows of exactly two carriers, not three.
    assert_eq!(
        view.reifier_quads().count(),
        2 * shared.reifier_quads().count(),
        "the duplicated source must be deduplicated away"
    );
    assert_graph_seam_agrees(&view, "composite");
}

/// The graph-PLACEMENT path: a source whose rows are rewritten into one named graph
/// has the logical graph constraint removed from its physical pattern, so the
/// narrowed body must still land every row under the placed name.
#[test]
fn composite_graph_seam_matches_the_filtered_stream_under_graph_placement() {
    let view = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::new(statement_layers(None))
                .with_graph_placement(GraphPlacement::Named(iri("placed"))),
            CompositeSource::new(statement_layers(Some("other"))),
        ],
        ViewLimits::default(),
    )
    .expect("a placed source composes with an unplaced one");
    assert_graph_seam_agrees(&view, "composite/placement");
}

/// The SELECTION path: a projection reads its rows back out of a retained composite
/// one selected graph at a time, so its own graph translation has to agree with the
/// filter too — including for the default graph, which selection by name never holds.
#[test]
fn composite_graph_seam_matches_the_filtered_stream_over_a_selection() {
    let retained = Arc::new(
        CompositeDatasetView::with_shared_scopes(
            vec![statement_layers(None)],
            ViewLimits::default(),
        )
        .expect("one shared-scope source composes"),
    );
    let selection = CompositeSource::from_selection(
        Arc::clone(&retained),
        [iri("g1"), iri("empty")],
        ViewLimits::default(),
    )
    .expect("both named graphs are held");
    let view = CompositeDatasetView::from_shared_sources(
        vec![
            selection,
            CompositeSource::new(statement_layers(Some("other")))
                .with_scope_binding(ScopeBinding::Shared),
        ],
        ViewLimits::default(),
    )
    .expect("a selection composes like any other source");
    // The selection contributes real side-table rows, so the narrowed body cannot
    // pass by skipping the projection outright.
    assert!(
        view.reifier_quads_in_graph(GraphMatch::Named(
            view.term_id_by_value(&iri("g1")).expect("g1 is held")
        ))
        .count()
            > 0,
        "the selected graph must genuinely hold a reifier row"
    );
    assert_graph_seam_agrees(&view, "composite/selection");
}

/// A delta snapshot whose overlay does real work in BOTH directions: rows removed
/// from the base (the suppression mask) and rows added by the delta, each in a
/// different graph, plus one row added that the base already holds (the duplicate
/// mask).
#[test]
fn delta_graph_seam_matches_the_filtered_stream() {
    let base = statement_layers(None);
    let mut mutable = MutableDataset::new(Arc::clone(&base));

    // REMOVAL: drop the g2 reifier declaration and the g2 annotation, so the base
    // arm's suppression mask is non-empty.
    let quoted = TermValue::Triple {
        s: Box::new(iri("s")),
        p: Box::new(TermValue::iri(P)),
        o: Box::new(iri("o")),
    };
    assert!(
        mutable.remove(&QuadValues::quad(
            iri("r"),
            TermValue::iri(REIFIES),
            quoted.clone(),
            iri("g2"),
        )),
        "the g2 reifier declaration is present to remove"
    );
    assert!(
        mutable.remove(&QuadValues::quad(
            iri("r"),
            TermValue::iri(CONFIDENCE),
            TermValue::iri(HIGH),
            iri("g2"),
        )),
        "the g2 annotation is present to remove"
    );

    // ADDITION: a fresh reifier declaration and annotation in a graph the base never
    // named, plus the same pair in the DEFAULT graph, so both arms of the overlay
    // carry rows under both `GraphMatch::Named` and `GraphMatch::Default`.
    for graph in [None, Some(iri("g3"))] {
        mutable
            .insert(QuadValues {
                s: iri("added-r"),
                p: TermValue::iri(REIFIES),
                o: quoted.clone(),
                g: graph.clone(),
            })
            .expect("a fresh reifier declaration inserts");
        mutable
            .insert(QuadValues {
                s: iri("added-r"),
                p: TermValue::iri(CONFIDENCE),
                o: TermValue::iri(HIGH),
                g: graph,
            })
            .expect("a fresh annotation inserts");
    }
    // A row the base already holds: re-inserting it must be a no-op, so the overlay
    // never double-counts it on either side of the seam.
    assert!(
        !mutable
            .insert(QuadValues::quad(
                iri("r"),
                TermValue::iri(CONFIDENCE),
                TermValue::iri(HIGH),
                iri("g1"),
            ))
            .expect("every term of the row is already interned"),
        "a row the base already holds is already effective"
    );

    let view = mutable.snapshot_view().expect("the snapshot publishes");
    assert_graph_seam_agrees(&view, "delta");

    // The overlay genuinely masked and genuinely added, so an implementation that
    // dropped either mask would be visible in the counts the law is checked against.
    let g2 = view.term_id_by_value(&iri("g2")).expect("g2 survives");
    assert_eq!(
        view.reifier_quads_in_graph(GraphMatch::Named(g2)).count(),
        0,
        "the removed g2 declaration must not reappear through the narrowed seam"
    );
    let g3 = view.term_id_by_value(&iri("g3")).expect("g3 is added");
    assert_eq!(
        view.reifier_quads_in_graph(GraphMatch::Named(g3)).count(),
        1,
        "the added g3 declaration must reach the narrowed seam"
    );
    assert_eq!(
        view.annotation_quads_in_graph(GraphMatch::Named(g3))
            .count(),
        1,
        "the added g3 annotation must reach the narrowed seam"
    );
}

/// A delta over a base holding a BLANK graph name: the narrowing translates a graph
/// handle between the two layers' dictionaries, and a blank name is the case where
/// the two layers agree on a value that is not an IRI.
#[test]
fn delta_graph_seam_matches_the_filtered_stream_for_a_blank_graph_name() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri(P);
    let o = b.intern_iri("http://example.org/o");
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri("http://example.org/r");
    let confidence = b.intern_iri(CONFIDENCE);
    let high = b.intern_iri(HIGH);
    let blank = b.intern_blank("bg", BlankScope::DEFAULT);
    for graph in [None, Some(blank)] {
        b.push_quad(s, p, o, graph);
        b.push_reifier_in_graph(r, triple, graph);
        b.push_annotation_in_graph(r, confidence, high, graph);
    }
    let base = b.freeze().expect("blank-graph fixture freezes");

    let mut mutable = MutableDataset::new(base);
    mutable
        .insert(QuadValues {
            s: iri("added-r"),
            p: TermValue::iri(REIFIES),
            o: TermValue::Triple {
                s: Box::new(iri("s")),
                p: Box::new(TermValue::iri(P)),
                o: Box::new(iri("o")),
            },
            g: Some(TermValue::Blank {
                label: "bg".into(),
                scope: BlankScope::DEFAULT,
            }),
        })
        .expect("a declaration in the blank graph inserts");
    let view = mutable.snapshot_view().expect("the snapshot publishes");
    assert_graph_seam_agrees(&view, "delta/blank-graph");
}
