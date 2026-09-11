// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Adversarial identity, statement-layer, projection and ownership checks.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, CanonHash, CompositeDatasetView, CompositeSource, ContentDigest, DatasetMut,
    DatasetView, DeltaDatasetView, GraphMatch, GraphPlacement, MutableDataset, QuadIds, QuadValues,
    RESERVED_NAMESPACE, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermRef,
    TermValue, ViewLimits, blank_count_view, canonicalize, canonicalize_graph_view,
    canonicalize_view, check_admissible_view, datasets_isomorphic, graph_digest_view,
    try_canonicalize_view, try_graph_digest_view,
};

const P: &str = "http://example.org/p";
const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";
const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
type Row = (TermValue, TermValue, TermValue, Option<TermValue>);

fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{local}"))
}

fn owned<D: DatasetView>(view: &D, id: D::Id) -> TermValue {
    match view.resolve(id) {
        TermRef::Iri(value) => TermValue::iri(value),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.into(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let TermRef::Iri(datatype) = view.resolve(datatype) else {
                panic!("datatype")
            };
            TermValue::Literal {
                lexical_form: lexical.into(),
                datatype: datatype.into(),
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(owned(view, s)),
            p: Box::new(owned(view, p)),
            o: Box::new(owned(view, o)),
        },
    }
}

fn row<D: DatasetView>(view: &D, q: QuadIds<D::Id>) -> Row {
    (
        owned(view, q.s),
        owned(view, q.p),
        owned(view, q.o),
        q.g.map(|g| owned(view, g)),
    )
}

fn surface<D: DatasetView>(view: &D) -> [BTreeSet<Row>; 3] {
    [
        view.quads().map(|q| row(view, q)).collect(),
        view.reifier_quads().map(|q| row(view, q)).collect(),
        view.annotation_quads().map(|q| row(view, q)).collect(),
    ]
}

fn complete_source() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("same", BlankScope::DEFAULT);
    let p = b.intern_iri(P);
    let text = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    let triple = b.intern_triple(s, p, text);
    let nested = b.intern_triple(s, p, triple);
    let reifier = b.intern_blank("statement", BlankScope(7));
    let g = b.intern_iri("http://example.org/graph");
    let empty = b.intern_blank("empty", BlankScope(8));
    b.push_quad(s, p, nested, Some(g));
    b.push_reifier_in_graph(reifier, nested, Some(g));
    b.push_annotation_in_graph(reifier, p, text, Some(g));
    b.declare_named_graph(empty);
    b.freeze().unwrap()
}

fn assert_probes<D: DatasetView>(view: &D) {
    for q in view.reifier_quads() {
        assert_eq!(
            view.reifier_quads_of(q.s)
                .map(|q| row(view, q))
                .collect::<BTreeSet<_>>(),
            view.reifier_quads()
                .filter(|other| other.s == q.s)
                .map(|q| row(view, q))
                .collect()
        );
    }
    for q in view.annotation_quads() {
        assert_eq!(
            view.annotations_of_with_graph(q.s)
                .map(|(p, o, g)| row(view, QuadIds { s: q.s, p, o, g }))
                .collect::<BTreeSet<_>>(),
            view.annotation_quads()
                .filter(|other| other.s == q.s)
                .map(|q| row(view, q))
                .collect()
        );
    }
    let rows: Vec<_> = view.quads().collect();
    for quad in &rows {
        for mask in 0..16 {
            let s = (mask & 1 != 0).then_some(quad.s);
            let p = (mask & 2 != 0).then_some(quad.p);
            let o = (mask & 4 != 0).then_some(quad.o);
            let g = if mask & 8 == 0 {
                GraphMatch::Any
            } else {
                quad.g.map_or(GraphMatch::Default, GraphMatch::Named)
            };
            let expected: BTreeSet<_> = rows
                .iter()
                .copied()
                .filter(|q| {
                    s.is_none_or(|s| s == q.s)
                        && p.is_none_or(|p| p == q.p)
                        && o.is_none_or(|o| o == q.o)
                        && g.matches(q.g)
                })
                .map(|q| (q.s, q.p, q.o, q.g))
                .collect();
            let plan = view.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
            let actual: Vec<_> = view
                .quads_for_pattern_with_plan(&plan, s, p, o, g)
                .collect();
            assert_eq!(
                actual.len(),
                expected.len(),
                "duplicate or absent projected row"
            );
            assert_eq!(
                actual
                    .into_iter()
                    .map(|q| (q.s, q.p, q.o, q.g))
                    .collect::<BTreeSet<_>>(),
                expected
            );
            assert!(view.cardinality_estimate(s, p, o, g) >= expected.len());
        }
    }
}

#[test]
fn independent_occurrences_and_shared_contributions_have_explicit_blank_identity() {
    let source = complete_source();
    let independent =
        CompositeDatasetView::new(vec![source.clone(), source.clone()], ViewLimits::default())
            .unwrap();
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![source.clone(), source.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(independent.quads().count(), 2);
    assert_eq!(independent.reifier_quads().count(), 2);
    assert_eq!(independent.annotation_quads().count(), 2);
    assert_eq!(independent.named_graphs().count(), 3);
    assert_eq!(shared.quads().count(), 1);
    assert_eq!(surface(&shared), surface(&source));
    let blank = source
        .term_id_by_blank("same", BlankScope::DEFAULT)
        .unwrap();
    assert_ne!(
        independent.source_id(0, blank),
        independent.source_id(1, blank)
    );
    assert_eq!(shared.source_id(0, blank), shared.source_id(1, blank));
    for view in [&independent, &shared] {
        assert_eq!(view.term_ids().count(), view.term_count());
        assert_eq!(
            view.term_ids().collect::<BTreeSet<_>>().len(),
            view.term_count()
        );
        for id in view.term_ids() {
            assert_eq!(view.term_id_by_value(&view.term_value(id)), Some(id));
        }
        assert_eq!(surface(view), surface(&view.materialize().unwrap()));
        assert_probes(view);
    }
    let mut native = RdfDatasetBuilder::new();
    native.push_dataset(&source);
    native.push_dataset(&source);
    assert!(datasets_isomorphic(
        &native.freeze().unwrap(),
        &independent.materialize().unwrap()
    ));
}

#[test]
fn source_local_integer_collision_does_not_alias_typed_values() {
    let mut a = RdfDatasetBuilder::new();
    let a_s = a.intern_iri("http://example.org/a");
    let a_p = a.intern_iri(P);
    a.push_quad(a_s, a_p, a_s, None);
    let mut b = RdfDatasetBuilder::new();
    let b_s = b.intern_blank("a", BlankScope::DEFAULT);
    let b_p = b.intern_iri(P);
    let b_o = b.intern_literal(RdfLiteral::simple("http://example.org/a"));
    b.push_quad(b_s, b_p, b_o, None);
    let a = a.freeze().unwrap();
    let b = b.freeze().unwrap();
    let view = CompositeDatasetView::new(vec![a, b], ViewLimits::default()).unwrap();
    assert_eq!(view.quads().count(), 2);
    assert_ne!(view.source_id(0, a_s), view.source_id(1, b_s));
    assert_eq!(view.source_id(0, a_p), view.source_id(1, b_p));
    assert_probes(&view);
}

#[test]
fn graph_projection_deduplicates_within_and_between_sources() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri(P);
    let o = b.intern_iri("http://example.org/o");
    let g = b.intern_iri("http://example.org/g");
    let h = b.intern_iri("http://example.org/h");
    let empty = b.intern_iri("http://example.org/empty");
    let triple = b.intern_triple(s, p, o);
    for graph in [None, Some(g), Some(h)] {
        b.push_quad(s, p, o, graph);
        b.push_reifier_in_graph(s, triple, graph);
        b.push_annotation_in_graph(s, p, o, graph);
    }
    b.declare_named_graph(empty);
    let source = b.freeze().unwrap();
    for placement in [
        GraphPlacement::Default,
        GraphPlacement::Named(iri("placed")),
    ] {
        let view = CompositeDatasetView::from_sources(
            vec![
                CompositeSource::new(source.clone()).with_graph_placement(placement.clone()),
                CompositeSource::new(source.clone()).with_graph_placement(placement.clone()),
            ],
            ViewLimits::default(),
        )
        .unwrap();
        assert_eq!(view.quads().count(), 1);
        assert_eq!(view.reifier_quads().count(), 1);
        assert_eq!(view.annotation_quads().count(), 1);
        let expected = match placement {
            GraphPlacement::Named(value) => Some(value),
            _ => None,
        };
        for table in surface(&view) {
            assert_eq!(table.first().unwrap().3, expected);
        }
        assert_eq!(view.named_graphs().count(), usize::from(expected.is_some()));
        assert_eq!(surface(&view), surface(&view.materialize().unwrap()));
        assert_probes(&view);
    }
}

#[test]
fn prepared_composite_probes_reuse_plans_across_rows_and_graph_placements() {
    let mut builder = RdfDatasetBuilder::new();
    let subjects = ["s", "t"].map(|name| builder.intern_iri(&format!("http://example.org/{name}")));
    let predicates =
        ["p", "q"].map(|name| builder.intern_iri(&format!("http://example.org/{name}")));
    let objects = ["o", "v"].map(|name| builder.intern_iri(&format!("http://example.org/{name}")));
    let graphs = ["g", "h"].map(|name| builder.intern_iri(&format!("http://example.org/{name}")));
    for (index, (s, p)) in subjects.into_iter().zip(predicates).enumerate() {
        for g in [None, Some(graphs[0]), Some(graphs[1])] {
            builder.push_quad(s, p, objects[index], g);
        }
    }
    let native = builder.freeze().unwrap();
    let mut mutable = MutableDataset::new(native.clone());
    assert!(mutable.remove(&QuadValues {
        s: iri("s"),
        p: iri("p"),
        o: iri("o"),
        g: Some(iri("g")),
    }));
    mutable
        .insert(QuadValues::triple(iri("added"), iri("p"), iri("o")))
        .unwrap();
    mutable
        .insert(QuadValues {
            s: iri("added"),
            p: iri("q"),
            o: iri("v"),
            g: Some(iri("h")),
        })
        .unwrap();
    let delta = Arc::new(mutable.snapshot_view().unwrap());
    let placements = [
        GraphPlacement::Preserve,
        GraphPlacement::Default,
        GraphPlacement::Named(iri("placed")),
    ];
    for native_placement in &placements {
        for delta_placement in &placements {
            let view = CompositeDatasetView::from_sources(
                vec![
                    CompositeSource::new(native.clone())
                        .with_graph_placement(native_placement.clone()),
                    CompositeSource::from_delta(delta.clone())
                        .with_graph_placement(delta_placement.clone()),
                ],
                ViewLimits::default(),
            )
            .unwrap();
            let rows: Vec<_> = view.quads().collect();
            let named: Vec<_> = ["g", "h", "placed", "s"]
                .into_iter()
                .filter_map(|name| view.term_id_by_value(&iri(name)))
                .map(GraphMatch::Named)
                .collect();
            for mask in 0..8 {
                for graph_queries in [&[GraphMatch::Any][..], &[GraphMatch::Default], &named] {
                    // One plan is reused across different bound term values and
                    // named graphs, as in an index-nested-loop join slot.
                    let plan = view.probe_plan(
                        mask & 1 != 0,
                        mask & 2 != 0,
                        mask & 4 != 0,
                        graph_queries[0],
                    );
                    for &g in graph_queries {
                        for quad in &rows {
                            let s = (mask & 1 != 0).then_some(quad.s);
                            let p = (mask & 2 != 0).then_some(quad.p);
                            let o = (mask & 4 != 0).then_some(quad.o);
                            let expected: BTreeSet<_> = rows
                                .iter()
                                .copied()
                                .filter(|q| {
                                    s.is_none_or(|s| s == q.s)
                                        && p.is_none_or(|p| p == q.p)
                                        && o.is_none_or(|o| o == q.o)
                                        && g.matches(q.g)
                                })
                                .map(|q| (q.s, q.p, q.o, q.g))
                                .collect();
                            let actual: Vec<_> =
                                DatasetView::quads_for_pattern_with_plan(&view, &plan, s, p, o, g)
                                    .collect();
                            assert_eq!(
                                actual,
                                view.quads_for_pattern(s, p, o, g).collect::<Vec<_>>(),
                                "prepared probes retain exact iteration order"
                            );
                            assert_eq!(actual.len(), expected.len(), "projected rows are unique");
                            assert_eq!(
                                actual
                                    .into_iter()
                                    .map(|q| (q.s, q.p, q.o, q.g))
                                    .collect::<BTreeSet<_>>(),
                                expected,
                                "{native_placement:?} / {delta_placement:?}, mask {mask}, graph {g:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn declaration_only_projection_and_named_blank_scope_do_not_collide() {
    let source = complete_source();
    let graph = TermValue::Blank {
        label: "same".into(),
        scope: BlankScope::DEFAULT,
    };
    let view = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::new(source).with_graph_placement(GraphPlacement::Named(graph.clone())),
        ],
        ViewLimits::default(),
    )
    .unwrap();
    let q = view.quads().next().unwrap();
    assert_ne!(q.s, q.g.unwrap());
    assert_eq!(view.term_value(q.g.unwrap()), graph);
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let only_graph = CompositeDatasetView::from_sources(
        vec![CompositeSource::new(empty).with_graph_placement(GraphPlacement::Named(iri("empty")))],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(only_graph.quads().count(), 0);
    assert_eq!(only_graph.materialize().unwrap().named_graphs().count(), 1);
}

#[test]
fn composite_literals_rebind_external_nested_and_triple_term_references() {
    let mut b = RdfDatasetBuilder::new();
    let external = b.intern_blank("external", BlankScope::DEFAULT);
    let p = b.intern_iri(P);
    let list = b.intern_literal(RdfLiteral::typed(
        "[_:external, [_:external], {\"key\": _:external}]",
        LIST,
    ));
    let map = b.intern_literal(RdfLiteral::typed("{\"key\": [_:external]}", MAP));
    let triple = b.intern_triple(external, p, list);
    let nested = b.intern_triple(external, p, triple);
    b.push_quad(external, p, nested, None);
    b.push_quad(external, p, map, None);
    let source = b.freeze().unwrap();
    let view =
        CompositeDatasetView::new(vec![source.clone(), source.clone()], ViewLimits::default())
            .unwrap();
    let mut native = RdfDatasetBuilder::new();
    native.push_dataset(&source);
    native.push_dataset(&source);
    let materialized = view.materialize().unwrap();
    assert!(datasets_isomorphic(
        &native.freeze().unwrap(),
        &materialized
    ));
    let mut scopes = BTreeSet::new();
    for id in view.term_ids() {
        let TermRef::Literal {
            lexical, datatype, ..
        } = view.resolve(id)
        else {
            continue;
        };
        let TermRef::Iri(datatype) = view.resolve(datatype) else {
            panic!("datatype")
        };
        for (label, scope) in purrdf_core::cdt_blank::cdt_embedded_blanks(lexical, datatype) {
            let embedded = view
                .term_id_by_value(&TermValue::Blank { label, scope })
                .unwrap();
            assert!(view.quads().any(|q| q.s == embedded));
            scopes.insert(scope);
        }
        assert_eq!(view.term_id_by_value(&view.term_value(id)), Some(id));
    }
    assert_eq!(scopes.len(), 2);
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![source.clone(), source],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(shared.stats().work.copied_text_bytes, 0);
    assert_eq!(shared.quads().count(), 2);
    assert!(view.stats().work.copied_text_bytes > 0);
    let limits = ViewLimits {
        max_auxiliary_bytes: view.stats().auxiliary_bytes - 1,
        ..ViewLimits::default()
    };
    assert!(
        CompositeDatasetView::new(
            view.sources()
                .iter()
                .map(|s| s.dataset().unwrap().clone())
                .collect(),
            limits
        )
        .is_err()
    );
}

fn metadata_row(source: &RdfDataset, q: QuadIds) -> QuadValues {
    let (s, p, o, g) = row(source, q);
    QuadValues { s, p, o, g }
}

#[test]
fn statement_rows_remove_and_reinsert_without_resurrection_or_duplicates() {
    let source = complete_source();
    let declaration = metadata_row(&source, source.reifier_quads().next().unwrap());
    let annotation = metadata_row(&source, source.annotation_quads().next().unwrap());
    let mut mutable = MutableDataset::new(source.clone());
    assert!(mutable.contains(&declaration));
    assert!(mutable.contains(&annotation));
    assert!(!mutable.insert(declaration.clone()).unwrap());
    assert!(!mutable.insert(annotation.clone()).unwrap());
    assert!(mutable.remove(&annotation));
    assert!(!mutable.remove(&annotation));
    let removed = mutable.snapshot_view().unwrap();
    assert_eq!(removed.annotation_quads().count(), 0);
    assert_eq!(mutable.freeze().unwrap().annotation_quads().count(), 0);
    assert!(mutable.insert(annotation.clone()).unwrap());
    assert!(mutable.remove(&declaration));
    let demoted = mutable.snapshot_view().unwrap();
    assert_eq!(demoted.reifier_quads().count(), 0);
    assert_eq!(demoted.annotation_quads().count(), 0);
    assert_eq!(demoted.quads().count(), 2);
    assert_probes(&demoted);
    assert_eq!(surface(&demoted), surface(&mutable.freeze().unwrap()));
    assert!(mutable.insert(declaration).unwrap());
    let restored = mutable.snapshot_view().unwrap();
    assert_eq!(surface(&restored), surface(&source));
    assert_eq!(
        removed.annotation_quads().count(),
        0,
        "snapshot remains immutable"
    );
    assert_eq!(
        demoted.reifier_quads().count(),
        0,
        "later reinsertion cannot mutate a snapshot"
    );
}

#[test]
fn adding_reifier_classifies_only_rows_in_the_same_graph() {
    let mut b = RdfDatasetBuilder::new();
    let r = b.intern_iri("http://example.org/r");
    let p = b.intern_iri(P);
    let o = b.intern_iri("http://example.org/o");
    let g = b.intern_iri("http://example.org/g");
    b.push_quad(r, p, o, None);
    b.push_quad(r, p, o, Some(g));
    let source = b.freeze().unwrap();
    let declaration = QuadValues {
        s: iri("r"),
        p: TermValue::iri(REIFIES),
        o: TermValue::Triple {
            s: Box::new(iri("s")),
            p: Box::new(iri("p")),
            o: Box::new(iri("o")),
        },
        g: Some(iri("g")),
    };
    let mut mutable = MutableDataset::new(source);
    mutable.insert(declaration.clone()).unwrap();
    mutable
        .insert(QuadValues {
            s: iri("r"),
            p: iri("q"),
            o: iri("o"),
            g: None,
        })
        .unwrap();
    let view = mutable.snapshot_view().unwrap();
    assert_eq!(view.quads().count(), 2);
    assert_eq!(view.reifier_quads().count(), 1);
    assert_eq!(view.annotation_quads().count(), 1);
    let mut expected = RdfDatasetBuilder::new();
    let r = expected.intern_iri("http://example.org/r");
    let p = expected.intern_iri(P);
    let o = expected.intern_iri("http://example.org/o");
    let g = expected.intern_iri("http://example.org/g");
    let s = expected.intern_iri("http://example.org/s");
    let q = expected.intern_iri("http://example.org/q");
    let triple = expected.intern_triple(s, p, o);
    expected.push_quad(r, p, o, None);
    expected.push_quad(r, q, o, None);
    expected.push_reifier_in_graph(r, triple, Some(g));
    expected.push_annotation_in_graph(r, p, o, Some(g));
    assert_eq!(surface(&view), surface(&expected.freeze().unwrap()));
    assert!(view.annotation_quads().all(|q| q.g.is_some()));
    assert!(view.quads().all(|q| q.g.is_none()));
    assert_probes(&view);
    assert_eq!(surface(&view), surface(&mutable.freeze().unwrap()));
    assert!(mutable.remove(&declaration));
    assert_eq!(mutable.snapshot_view().unwrap().quads().count(), 3);
}

#[test]
fn delta_sources_compose_without_materialization_and_keep_all_declarations() {
    let source = complete_source();
    let mut mutable = MutableDataset::new(source.clone());
    mutable
        .insert(QuadValues::triple(iri("new"), iri("p"), iri("o")))
        .unwrap();
    let delta = Arc::new(mutable.snapshot_view().unwrap());
    let before = delta.stats().work;
    let view = CompositeDatasetView::from_sources(
        vec![
            CompositeSource::from_delta(delta.clone())
                .with_graph_placement(GraphPlacement::Default),
            CompositeSource::new(source).with_graph_placement(GraphPlacement::Named(iri("shapes"))),
        ],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(delta.stats().work, before);
    assert_eq!(
        view.stats().retained_sources,
        4,
        "base, delta, shapes and graph dictionary"
    );
    assert_eq!(view.stats().work.materializations, 0);
    assert_eq!(view.quads().count(), 3);
    assert_eq!(view.reifier_quads().count(), 2);
    assert_eq!(view.annotation_quads().count(), 2);
    assert_eq!(view.named_graphs().count(), 1);
    assert_eq!(surface(&view), surface(&view.materialize().unwrap()));
    assert_probes(&view);
    let limits = ViewLimits {
        max_sources: 3,
        ..ViewLimits::default()
    };
    assert!(
        CompositeDatasetView::from_sources(
            vec![
                CompositeSource::from_delta(delta)
                    .with_graph_placement(GraphPlacement::Named(iri("placed"))),
                CompositeSource::new(complete_source()),
            ],
            limits
        )
        .is_err()
    );
}

#[test]
fn admission_limits_cover_sources_payload_rows_terms_and_derived_graph_text() {
    let source = complete_source();
    for limits in [
        ViewLimits {
            max_sources: 0,
            ..ViewLimits::default()
        },
        ViewLimits {
            max_terms: source.term_count() - 1,
            ..ViewLimits::default()
        },
        ViewLimits {
            max_rows: source.rdf_row_count() - 1,
            ..ViewLimits::default()
        },
        ViewLimits {
            max_payload_bytes: source.rdf_payload_bytes() - 1,
            ..ViewLimits::default()
        },
        ViewLimits {
            max_auxiliary_bytes: 0,
            ..ViewLimits::default()
        },
    ] {
        assert!(CompositeDatasetView::new(vec![source.clone()], limits).is_err());
    }
    let large_graph = TermValue::iri(format!("http://example.org/{}", "g".repeat(8192)));
    for limits in [
        ViewLimits {
            max_payload_bytes: source.rdf_payload_bytes() + 1024,
            ..ViewLimits::default()
        },
        ViewLimits {
            max_auxiliary_bytes: 1024,
            ..ViewLimits::default()
        },
    ] {
        assert!(
            CompositeDatasetView::from_sources(
                vec![
                    CompositeSource::new(source.clone())
                        .with_graph_placement(GraphPlacement::Named(large_graph.clone()))
                ],
                limits
            )
            .is_err()
        );
    }
    let invalid = TermValue::Literal {
        lexical_form: "graph".into(),
        datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
        language: None,
        direction: None,
    };
    assert!(
        CompositeDatasetView::from_sources(
            vec![
                CompositeSource::new(source.clone())
                    .with_graph_placement(GraphPlacement::Named(invalid))
            ],
            ViewLimits::default()
        )
        .is_err()
    );
    let mutable = MutableDataset::new(source.clone());
    assert!(
        mutable
            .snapshot_view_with_limits(ViewLimits {
                max_sources: 1,
                ..ViewLimits::default()
            })
            .is_err()
    );
    assert_eq!(mutable.work_stats().freezes, 0);
    assert_eq!(surface(&mutable.freeze().unwrap()), surface(&source));
}

#[test]
fn shared_ownership_drops_explicitly_and_clones_do_not_copy_dictionaries_or_maps() {
    let source = complete_source();
    let weak = Arc::downgrade(&source);
    let view =
        CompositeDatasetView::with_shared_scopes(vec![source], ViewLimits::default()).unwrap();
    let before = view.stats();
    assert_eq!(before.work.copied_terms, 0);
    assert_eq!(before.work.copied_text_bytes, 0);
    assert_eq!(before.work.copied_rows, 0);
    assert_eq!(before.work.freezes, 0);
    let cloned = view.clone();
    assert_eq!(cloned.stats(), before);
    let materialized = cloned.materialize().unwrap();
    let after = view.stats().work;
    assert_eq!(after.materializations, 1);
    assert_eq!(after.freezes, 1);
    assert_eq!(after.copied_text_bytes, materialized.rdf_text_bytes());
    assert_eq!(after.copied_rows, materialized.rdf_row_count());
    drop(view);
    assert!(weak.upgrade().is_some());
    drop(cloned);
    assert!(weak.upgrade().is_none());
    let mut mutable = MutableDataset::new(materialized);
    mutable
        .insert(QuadValues::triple(iri("new"), iri("p"), iri("o")))
        .unwrap();
    let snapshot = mutable.snapshot_view().unwrap();
    let work = snapshot.stats().work;
    assert_eq!(snapshot.stats().work, work);
    assert_eq!(work.materializations, 0);
    assert_eq!(work.freezes, 1);
    assert_eq!(work.copied_rows, 1);
    assert!(work.copied_index_bytes > 0);
    assert_eq!(mutable.work_stats(), work);
}

#[test]
fn repeated_snapshots_release_their_deltas_and_keep_nested_payloads_borrowed() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(P);
    let literal = b.intern_literal(RdfLiteral::simple("large".repeat(65_536)));
    let inner = b.intern_triple(p, p, literal);
    let outer = b.intern_triple(p, p, inner);
    b.push_quad(p, p, outer, None);
    let source = b.freeze().unwrap();
    let mutable = MutableDataset::new(source.clone());
    for _ in 0..16 {
        let snapshot = mutable.snapshot_view().unwrap();
        let delta = Arc::downgrade(snapshot.delta());
        let copied = snapshot.stats().work;
        assert_eq!(copied.copied_text_bytes, 0);
        assert_eq!(copied.copied_rows, 0);
        let clone = snapshot.clone();
        assert_eq!(clone.stats().work, copied);
        drop(snapshot);
        assert!(delta.upgrade().is_some());
        drop(clone);
        assert!(delta.upgrade().is_none());
    }
    let composite = CompositeDatasetView::with_shared_scopes(
        vec![source.clone(), source.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(composite.stats().work.copied_text_bytes, 0);
    let literal_id = source
        .term_id_by_value(&source.term_value(literal))
        .unwrap();
    let TermRef::Literal {
        lexical: native, ..
    } = source.resolve(literal_id)
    else {
        panic!("literal")
    };
    let TermRef::Literal {
        lexical: borrowed, ..
    } = composite.resolve(composite.source_id(1, literal_id))
    else {
        panic!("literal")
    };
    assert!(std::ptr::eq(native.as_ptr(), borrowed.as_ptr()));
    assert_eq!(composite.quads().count(), 1);
}

#[test]
fn blanks_referenced_only_inside_composite_literals_stay_independent() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri(P);
    let literal = b.intern_literal(RdfLiteral::typed("[[{\"key\": _:hidden}]]", LIST));
    b.push_quad(s, p, literal, None);
    let source = b.freeze().unwrap();
    assert!(
        source
            .quads()
            .all(|q| !matches!(source.resolve(q.s), TermRef::Blank { .. }))
    );
    let view =
        CompositeDatasetView::new(vec![source.clone(), source.clone()], ViewLimits::default())
            .unwrap();
    assert_eq!(
        view.quads().count(),
        2,
        "literal-only blanks must not alias"
    );
    let mut native = RdfDatasetBuilder::new();
    native.push_dataset(&source);
    native.push_dataset(&source);
    assert!(datasets_isomorphic(
        &native.freeze().unwrap(),
        &view.materialize().unwrap()
    ));
}

#[test]
fn post_freeze_retention_refusal_counts_completed_work_without_publishing() {
    let base = RdfDatasetBuilder::new().freeze().unwrap();
    let mut mutable = MutableDataset::new(base.clone());
    mutable
        .insert(QuadValues::triple(
            iri("s"),
            iri("p"),
            TermValue::Literal {
                lexical_form: "x".repeat(8192),
                datatype: "http://www.w3.org/2001/XMLSchema#string".into(),
                language: None,
                direction: None,
            },
        ))
        .unwrap();
    let limited = ViewLimits {
        max_payload_bytes: base.rdf_payload_bytes() + 128,
        ..ViewLimits::default()
    };
    assert!(mutable.snapshot_view_with_limits(limited).is_err());
    assert_eq!(mutable.work_stats().freezes, 1);
    assert_eq!(mutable.work_stats().materializations, 0);
    assert!(mutable.work_stats().copied_text_bytes >= 8192);
    assert_eq!(base.quad_count(), 0);
    let successful = mutable.snapshot_view().unwrap();
    assert_eq!(successful.quads().count(), 1);
    assert_eq!(mutable.work_stats().freezes, 2);
}

#[test]
fn suppression_counts_every_native_table_occurrence_and_reclassification_deduplicates() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri(P);
    let o = b.intern_iri("http://example.org/o");
    b.push_quad(s, p, o, None);
    b.push_annotation(s, p, o);
    let base = b.freeze().unwrap();
    let row = QuadValues::triple(iri("s"), iri("p"), iri("o"));
    let mut mutable = MutableDataset::new(base);
    assert_eq!(mutable.effective_count(), 2);
    assert!(mutable.remove(&row));
    assert_eq!(mutable.effective_count(), 0);
    assert_eq!(mutable.freeze().unwrap().rdf_row_count(), 0);
    assert!(mutable.insert(row).unwrap());
    assert_eq!(mutable.effective_count(), 2);
    mutable
        .insert(QuadValues::triple(
            iri("s"),
            TermValue::iri(REIFIES),
            TermValue::Triple {
                s: Box::new(iri("s")),
                p: Box::new(iri("p")),
                o: Box::new(iri("o")),
            },
        ))
        .unwrap();
    let view = mutable.snapshot_view().unwrap();
    assert_eq!(view.quads().count(), 0);
    assert_eq!(view.annotation_quads().count(), 1);
    assert_probes(&view);
    assert_eq!(surface(&view), surface(&view.materialize().unwrap()));
}

#[test]
fn deleting_one_declaration_keeps_unrelated_annotation_only_records_in_their_table() {
    let source = complete_source();
    let mut b = RdfDatasetBuilder::new();
    purrdf_core::ir::import::DatasetImporter::new(&mut b, &source).append();
    let orphan = b.intern_iri("http://example.org/annotation-only");
    let p = b.intern_iri(P);
    let o = b.intern_iri("http://example.org/o");
    b.push_annotation(orphan, p, o);
    let base = b.freeze().unwrap();
    let declaration = metadata_row(&base, base.reifier_quads().next().unwrap());
    let mut mutable = MutableDataset::new(base);
    mutable.remove(&declaration);
    let view = mutable.snapshot_view().unwrap();
    assert_eq!(view.annotation_quads().count(), 1);
    assert_eq!(view.quads().count(), 2);
    assert_eq!(
        owned(&view, view.annotation_quads().next().unwrap().s),
        iri("annotation-only")
    );
    assert_probes(&view);
}

#[test]
fn materialized_bytes_ignore_operational_counter_changes() {
    let view = CompositeDatasetView::new(
        vec![complete_source(), complete_source()],
        ViewLimits::default(),
    )
    .unwrap();
    let first = view.materialize().unwrap();
    let first_bytes = purrdf_core::ir::pack::PackBuilder::build_bytes(&first).unwrap();
    let second = view.materialize().unwrap();
    assert_eq!(view.stats().work.materializations, 2);
    assert_eq!(
        first_bytes,
        purrdf_core::ir::pack::PackBuilder::build_bytes(&second).unwrap()
    );
}

#[test]
fn reusing_source_owners_does_not_carry_a_previous_views_literal_rebinding() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(P);
    let literal = b.intern_literal(RdfLiteral::typed("[_:hidden]", LIST));
    b.push_quad(p, p, literal, None);
    let source = b.freeze().unwrap();
    let independent =
        CompositeDatasetView::new(vec![source.clone(), source.clone()], ViewLimits::default())
            .unwrap();
    assert_eq!(independent.quads().count(), 2);
    let shared = CompositeDatasetView::from_shared_sources(
        independent.sources().to_vec(),
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(shared.quads().count(), 1);
    assert_eq!(surface(&shared), surface(&source));
    assert_eq!(shared.stats().work.copied_text_bytes, 0);
}

// ---------------------------------------------------------------------------
// Canonical identity is a property of CONTENT, not of the carrier that holds it
// ---------------------------------------------------------------------------

const GRAPH: &str = "http://example.org/graph";
const OTHER_GRAPH: &str = "http://example.org/other";

/// One dataset touching every surface canonical identity has to see at once:
/// default AND named graphs, a declaration-only graph, blanks in two scopes (one
/// of them co-referent across rows), reifier and annotation rows in BOTH graphs,
/// and the three literal shapes whose spelling canonicalization carries verbatim
/// — bare `xsd:string`, `@en`, and a dir-lang `@ar--rtl`.
///
/// Split across two named graphs on purpose: a per-graph digest that quietly
/// admitted a neighbour's statement layer would still pass a single-graph fixture.
fn identity_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(P);
    let q = b.intern_iri("http://example.org/q");
    let o = b.intern_iri("http://example.org/o");
    let g = b.intern_iri(GRAPH);
    let other = b.intern_iri(OTHER_GRAPH);
    // One local label in two scopes: distinct nodes a label-keyed canonicalization
    // would conflate, and a `shared` that is CO-REFERENT across several rows.
    let shared = b.intern_blank("n", BlankScope::DEFAULT);
    let scoped = b.intern_blank("n", BlankScope(4));
    let plain = b.intern_literal(RdfLiteral::simple("plain"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("hello", "en"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    b.push_quad(shared, p, plain, None);
    b.push_quad(shared, q, tagged, None);
    b.push_quad(scoped, p, o, None);
    b.push_quad(shared, p, directional, Some(g));
    let triple = b.intern_triple(shared, p, directional);
    b.push_quad(scoped, q, triple, Some(g));
    let reifier = b.intern_blank("statement", BlankScope(7));
    b.push_reifier_in_graph(reifier, triple, Some(g));
    b.push_annotation_in_graph(reifier, q, tagged, Some(g));
    // The SAME triple term reified in the default graph as well.
    let default_reifier = b.intern_iri("http://example.org/r");
    b.push_reifier_in_graph(default_reifier, triple, None);
    b.push_annotation_in_graph(default_reifier, q, plain, None);
    b.push_quad(o, p, o, Some(other));
    // A named graph owning no row at all.
    let empty = b.intern_blank("declared only", BlankScope(9));
    b.declare_named_graph(empty);
    b.freeze().unwrap()
}

/// A delta view whose EFFECTIVE content is exactly `source`'s, reached through a
/// real mutation round trip rather than an untouched passthrough — so the delta
/// machinery (suppression rows, delta-only ids) is genuinely in the read path.
fn delta_of(source: &Arc<RdfDataset>) -> DeltaDatasetView {
    let mut mutable = MutableDataset::new(source.clone());
    let scratch = QuadValues::triple(iri("scratch"), iri("p"), iri("o"));
    assert!(mutable.insert(scratch.clone()).unwrap());
    assert!(mutable.remove(&scratch));
    mutable.snapshot_view().unwrap()
}

#[test]
fn canonical_identity_is_byte_equal_over_flat_composite_and_delta_views() {
    let flat = identity_fixture();
    let independent = CompositeDatasetView::new(vec![flat.clone()], ViewLimits::default()).unwrap();
    let shared =
        CompositeDatasetView::with_shared_scopes(vec![flat.clone()], ViewLimits::default())
            .unwrap();
    let delta = delta_of(&flat);

    let expected = canonicalize_view(&*flat, CanonHash::Sha256).nquads;
    // The fixture really does exercise what it claims; a parity assertion over
    // bytes that happened to omit the overlay would prove nothing.
    for fragment in [
        "\"plain\" .",
        "\"hello\"@en",
        "\"مرحبا\"@ar--rtl",
        "<<( ",
        "_:c14n",
        "<urn:purrdf:rdfc:reifies>",
        "<urn:purrdf:rdfc:annotation>",
        GRAPH,
        OTHER_GRAPH,
    ] {
        assert!(
            expected.contains(fragment),
            "{fragment} missing: {expected}"
        );
    }

    // The `&RdfDataset` entry point is the same core, not a second one.
    assert_eq!(canonicalize(&flat).nquads, expected);

    let digest = ContentDigest::of(expected.as_bytes());
    for (name, actual) in [
        (
            "independent composite",
            canonicalize_view(&independent, CanonHash::Sha256).nquads,
        ),
        (
            "shared-scope composite",
            canonicalize_view(&shared, CanonHash::Sha256).nquads,
        ),
        ("delta", canonicalize_view(&delta, CanonHash::Sha256).nquads),
    ] {
        assert_eq!(
            actual, expected,
            "{name} must canonicalize to the same bytes"
        );
        assert_eq!(
            ContentDigest::of(actual.as_bytes()),
            digest,
            "{name} digest"
        );
    }

    // The blank COUNT is a view-id property and must agree too — it is the
    // pre-reject the isomorphism oracle leans on.
    for count in [
        blank_count_view(&independent),
        blank_count_view(&shared),
        blank_count_view(&delta),
    ] {
        assert_eq!(count, blank_count_view(&*flat));
    }

    // Admission answers on a view, and answers BOTH ways: the clean view is
    // admitted, and only the one that actually carries the reserved namespace is
    // refused.
    assert!(check_admissible_view(&independent).is_ok());
    assert!(check_admissible_view(&delta).is_ok());
    assert!(try_canonicalize_view(&independent, CanonHash::Sha256).is_ok());

    let mut inadmissible = RdfDatasetBuilder::new();
    let s = inadmissible.intern_iri("http://example.org/s");
    let o = inadmissible.intern_iri("http://example.org/o");
    let reserved = inadmissible.intern_iri(&format!("{RESERVED_NAMESPACE}reifies"));
    inadmissible.push_quad(s, reserved, o, None);
    let inadmissible =
        CompositeDatasetView::new(vec![inadmissible.freeze().unwrap()], ViewLimits::default())
            .unwrap();
    assert!(check_admissible_view(&inadmissible).is_err());
    assert!(try_canonicalize_view(&inadmissible, CanonHash::Sha256).is_err());
}

#[test]
fn cross_type_isomorphism_compares_a_composite_against_a_flat_dataset() {
    let flat = identity_fixture();
    let independent = CompositeDatasetView::new(vec![flat.clone()], ViewLimits::default()).unwrap();
    let delta = delta_of(&flat);

    // Cross-type, both directions, with nothing materialized on either side.
    assert!(datasets_isomorphic(&*flat, &independent));
    assert!(datasets_isomorphic(&independent, &*flat));
    assert!(datasets_isomorphic(&delta, &independent));

    // Two independent occurrences of one source are TWO occurrences: the same
    // local blank label in two sources stays two distinct nodes, so the doubled
    // view is not isomorphic to the single one …
    let doubled =
        CompositeDatasetView::new(vec![flat.clone(), flat.clone()], ViewLimits::default()).unwrap();
    assert!(!datasets_isomorphic(&*flat, &doubled));
    assert_eq!(blank_count_view(&doubled), 2 * blank_count_view(&*flat));

    // … and it IS isomorphic to the flat dataset built by appending the source
    // twice — which is the same statement about co-reference from the other side:
    // within one occurrence the shared blank stays one node, across occurrences it
    // becomes two.
    let mut native = RdfDatasetBuilder::new();
    native.push_dataset(&flat);
    native.push_dataset(&flat);
    let native = native.freeze().unwrap();
    assert!(datasets_isomorphic(&native, &doubled));

    // Sharing scopes instead collapses the two occurrences back to one.
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![flat.clone(), flat.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    assert!(datasets_isomorphic(&*flat, &shared));

    // A negative that is not about counts: same shape, one different ground IRI.
    let mut altered = RdfDatasetBuilder::new();
    purrdf_core::ir::import::DatasetImporter::new(&mut altered, &flat).append();
    let s = altered.intern_iri("http://example.org/s");
    let p = altered.intern_iri(P);
    altered.push_quad(s, p, s, None);
    let altered = altered.freeze().unwrap();
    assert!(!datasets_isomorphic(&altered, &independent));
}

#[test]
fn per_graph_canonicalization_over_a_view_matches_the_flat_graph_projection() {
    let flat = identity_fixture();
    let independent = CompositeDatasetView::new(vec![flat.clone()], ViewLimits::default()).unwrap();
    let delta = delta_of(&flat);

    for graph in [GRAPH, OTHER_GRAPH] {
        // The reference: materialize the named-graph projection and canonicalize it,
        // exactly as the flat per-graph digest does today.
        let projected = canonicalize(&flat.project_named_graph(graph)).nquads;
        assert!(
            !projected.is_empty(),
            "{graph} must actually hold content, or parity proves nothing"
        );
        let digest = ContentDigest::of(projected.as_bytes());

        for (name, view_bytes, view_digest) in [
            (
                "flat",
                canonicalize_graph_view(&*flat, graph, CanonHash::Sha256).nquads,
                graph_digest_view(&*flat, graph),
            ),
            (
                "composite",
                canonicalize_graph_view(&independent, graph, CanonHash::Sha256).nquads,
                graph_digest_view(&independent, graph),
            ),
            (
                "delta",
                canonicalize_graph_view(&delta, graph, CanonHash::Sha256).nquads,
                graph_digest_view(&delta, graph),
            ),
        ] {
            assert_eq!(
                view_bytes, projected,
                "{name} per-graph canonical bytes must equal the projection's"
            );
            assert_eq!(view_digest, digest, "{name} per-graph digest");
        }

        // The graph slot is ERASED, so no projected line carries a graph token for
        // the graph it was selected by.
        assert!(
            !projected.contains(&format!("<{graph}> .")),
            "the selecting graph must not survive into its own projection: {projected}"
        );
    }

    // Isolation: each graph's digest is a function of its own content. `<graph>`
    // carries the statement layer, `<other>` does not, and the default graph's
    // reifier over the SAME triple term belongs to neither.
    assert_ne!(
        graph_digest_view(&*flat, GRAPH),
        graph_digest_view(&*flat, OTHER_GRAPH)
    );
    assert!(
        canonicalize_graph_view(&*flat, GRAPH, CanonHash::Sha256)
            .nquads
            .contains("<urn:purrdf:rdfc:reifies>")
    );
    assert!(
        !canonicalize_graph_view(&*flat, OTHER_GRAPH, CanonHash::Sha256)
            .nquads
            .contains("<urn:purrdf:rdfc:reifies>"),
        "one graph's statement layer must not leak into another's projection"
    );

    // A graph this view never names selects nothing — an empty subgraph, not an
    // error, and byte-identical to projecting an absent graph flat.
    let absent = "http://example.org/no-such-graph";
    assert_eq!(canonicalize(&flat.project_named_graph(absent)).nquads, "");
    for empty in [
        canonicalize_graph_view(&*flat, absent, CanonHash::Sha256).nquads,
        canonicalize_graph_view(&independent, absent, CanonHash::Sha256).nquads,
        canonicalize_graph_view(&delta, absent, CanonHash::Sha256).nquads,
    ] {
        assert_eq!(empty, "");
    }

    // The fallible per-graph digest agrees with its panicking sibling on the
    // Ok path …
    for graph in [GRAPH, OTHER_GRAPH] {
        for (name, fallible, trusted) in [
            (
                "flat",
                try_graph_digest_view(&*flat, graph),
                graph_digest_view(&*flat, graph),
            ),
            (
                "composite",
                try_graph_digest_view(&independent, graph),
                graph_digest_view(&independent, graph),
            ),
            (
                "delta",
                try_graph_digest_view(&delta, graph),
                graph_digest_view(&delta, graph),
            ),
        ] {
            assert_eq!(fallible.unwrap(), trusted, "{name} fallible digest");
        }
    }

    // … and refuses exactly the graph carrying reserved vocabulary, without
    // the refusal spreading to a neighbouring clean graph.
    let mut poisoned = RdfDatasetBuilder::new();
    let s = poisoned.intern_iri("http://example.org/s");
    let p = poisoned.intern_iri(P);
    let o = poisoned.intern_iri("http://example.org/o");
    let reserved = poisoned.intern_iri(&format!("{RESERVED_NAMESPACE}reifies"));
    let bad_graph = poisoned.intern_iri(GRAPH);
    let clean_graph = poisoned.intern_iri(OTHER_GRAPH);
    poisoned.push_quad(s, reserved, o, Some(bad_graph));
    poisoned.push_quad(s, p, o, Some(clean_graph));
    let poisoned =
        CompositeDatasetView::new(vec![poisoned.freeze().unwrap()], ViewLimits::default()).unwrap();
    assert!(try_graph_digest_view(&poisoned, GRAPH).is_err());
    assert!(try_graph_digest_view(&poisoned, OTHER_GRAPH).is_ok());
}
