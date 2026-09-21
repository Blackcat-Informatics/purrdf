// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! predicate, so every fixture here is built to CHARGE one, rather than to wrap an
//! empty view where dropping the predicate would be invisible. The predicates, and
//! the fixture that charges each:
//!
//! * composition dedup — two identical sources plus a distinct one, so the
//!   earlier-source predicate rejects a row on every probe;
//! * the delta overlay's SUPPRESSION mask — a snapshot with base statement-layer rows
//!   removed, which the narrowed seam must not resurrect;
//! * term resolution at the overlay's own boundary — a row spelled non-canonically,
//!   which must resolve onto the term the base already holds so that no layer gains a
//!   second copy and the narrowed seam has nothing to double-count.
//!
//! The delta overlay's DUPLICATE masks (`duplicate_reifiers` / `duplicate_annotations`)
//! are deliberately NOT on that list. No supported mutation path populates them: base
//! membership is probed on the canonicalized value, so a row the base already holds is
//! absorbed rather than admitted to the delta. The narrowed overrides still carry those
//! filters, verbatim from the unkeyed overrides they mirror — dropping them from only
//! the narrowed seam would make the two paths disagree if some future path ever did
//! populate a mask — but no fixture here charges them, and none claims to.
//!
//! Residue keeping lives on the bundle carrier, not on a wrapper built here; it is
//! pinned by `ir::pipeline_bundle`'s own tests.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, CompositeDatasetView, CompositeSource, DatasetMut, DatasetView, GraphMatch,
    GraphPlacement, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    ScopeBinding, TermValue, ViewLimits,
};

const P: &str = "http://example.org/p";
const CONFIDENCE: &str = "http://example.org/confidence";
const HIGH: &str = "http://example.org/high";
const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// An `example.org` IRI value, for terms this file's fixtures need to name but do
/// not otherwise intern through a builder.
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
/// different graph. The duplicate masks stay empty here — and, as the module doc
/// records, on every fixture in this file — because base membership absorbs a row the
/// base already holds, whatever spelling it arrives under.
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
    // A row the base already holds: base membership absorbs it, so it never reaches
    // the delta and the overlay's DUPLICATE masks stay empty. That holds for a
    // NON-canonical spelling too, because resolution canonicalizes on the same policy
    // interning does — which is why no fixture in this file charges those masks. The
    // absorption itself, on all three streams, is pinned by
    // `a_respelled_term_resolves_onto_the_base_row_and_never_duplicates_it`.
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

/// A term spelled non-canonically must resolve onto the term the base already holds,
/// so no layer ever gains a second copy of a row — on the narrowed seam exactly as on
/// the unkeyed one.
///
/// The hazard is the C0.1 literal identity policy. Interning normalizes before it
/// stores: a language tag names the datatype whatever the explicit one says, and the
/// tag itself is lowercased. A lookup that probed the caller's spelling verbatim would
/// therefore miss a term the dataset genuinely holds — `"high"@EN` would not find a
/// base holding `"high"@en` — and `MutableDataset` reads that miss as "new term". The
/// row would land in the delta, freeze back onto the base's OWN term, and exist in
/// both layers.
///
/// Resolution canonicalizes on the same policy, so the miss cannot happen and the
/// insert is absorbed. That matters most on the ORDINARY QUAD stream, which carries no
/// duplicate mask at all: a copy admitted there has nothing downstream to hide it and
/// reaches the reader as a genuinely duplicated quad. The statement-layer streams do
/// carry `duplicate_reifiers`/`duplicate_annotations`, but those are a second line of
/// defence, not the one being pinned here.
///
/// `TermValue::lang_literal` folds the tag itself, so the non-canonical spelling has
/// to be stated as a struct literal to reach the boundary at all.
#[test]
fn a_respelled_term_resolves_onto_the_base_row_and_never_duplicates_it() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri(P);
    // The base's own spelling of the literal, interned through the C0.1 policy.
    let tagged = b.intern_literal(RdfLiteral::language_tagged("high", "en"));
    let triple = b.intern_triple(s, p, tagged);
    let r = b.intern_iri("http://example.org/r");
    let confidence = b.intern_iri(CONFIDENCE);
    let g1 = b.intern_iri("http://example.org/g1");
    for graph in [None, Some(g1)] {
        b.push_quad(s, p, tagged, graph);
        b.push_reifier_in_graph(r, triple, graph);
        b.push_annotation_in_graph(r, confidence, tagged, graph);
    }
    let base = b.freeze().expect("language-tagged fixture freezes");
    let base_reifiers = base.reifier_quads().count();
    let base_annotations = base.annotation_quads().count();

    // The SAME literal, spelled with the tag in upper case. `TermValue::lang_literal`
    // would fold it here, so the fold has to be stated to reach the delta at all.
    let shouted = TermValue::Literal {
        lexical_form: "high".into(),
        datatype: LANG_STRING.into(),
        language: Some("EN".into()),
        direction: None,
    };
    let quoted = TermValue::Triple {
        s: Box::new(iri("s")),
        p: Box::new(TermValue::iri(P)),
        o: Box::new(shouted.clone()),
    };

    let base_quads = base.quads().count();
    let mut mutable = MutableDataset::new(Arc::clone(&base));
    for graph in [None, Some(iri("g1"))] {
        assert!(
            !mutable
                .insert(QuadValues {
                    s: iri("r"),
                    p: TermValue::iri(REIFIES),
                    o: quoted.clone(),
                    g: graph.clone(),
                })
                .expect("the shouted declaration resolves"),
            "a re-spelled declaration names a row the base already holds and must be \
             absorbed, not admitted as new"
        );
        assert!(
            !mutable
                .insert(QuadValues {
                    s: iri("r"),
                    p: TermValue::iri(CONFIDENCE),
                    o: shouted.clone(),
                    g: graph.clone(),
                })
                .expect("the shouted annotation resolves"),
            "a re-spelled annotation must be absorbed, not admitted as new"
        );
        assert!(
            !mutable
                .insert(QuadValues {
                    s: iri("s"),
                    p: TermValue::iri(P),
                    o: shouted.clone(),
                    g: graph,
                })
                .expect("the shouted quad resolves"),
            "a re-spelled ORDINARY quad must be absorbed too — the quad stream carries \
             no duplicate mask, so an admitted copy would survive to the reader"
        );
    }
    // Nothing reached the delta: the fold happens at the resolve boundary, so no layer
    // ever holds a second copy for the seam to have to hide.
    assert_eq!(
        mutable.added_len(),
        0,
        "every re-spelled row resolved onto the term the base already holds"
    );

    let view = mutable.snapshot_view().expect("the snapshot publishes");
    assert_graph_seam_agrees(&view, "delta/respelled-terms");
    assert_eq!(
        view.quads().count(),
        base_quads,
        "a re-spelled quad must not appear twice across the set-union seam"
    );

    // Every delta row folded back onto a row the base already holds, so the union
    // grew by nothing. Counted per graph as well as in total: equality between the
    // narrowed and the filtered stream alone would still hold if BOTH sides dropped
    // the mask, and these counts are what refuses that.
    assert_eq!(
        view.reifier_quads().count(),
        base_reifiers,
        "a re-spelled declaration must not add a reifier row"
    );
    assert_eq!(
        view.annotation_quads().count(),
        base_annotations,
        "a re-spelled annotation must not add an annotation row"
    );
    let g1 = view.term_id_by_value(&iri("g1")).expect("g1 survives");
    for (g, label) in [
        (GraphMatch::Named(g1), "g1"),
        (GraphMatch::Default, "default"),
    ] {
        assert_eq!(
            view.reifier_quads_in_graph(g).count(),
            1,
            "{label}: the narrowed seam must yield the declaration once, not twice"
        );
        assert_eq!(
            view.annotation_quads_in_graph(g).count(),
            1,
            "{label}: the narrowed seam must yield the annotation once, not twice"
        );
    }
}
