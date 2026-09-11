// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Adversarial identity, statement-layer, projection and ownership checks.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, CanonHash, CompositeDatasetView, CompositeSource, ContentDigest, DatasetMut,
    DatasetView, DeltaDatasetView, FallibleDatasetView, GraphMatch, GraphMatchValue,
    GraphPlacement, MutableDataset, OwnerMutability, QuadIds, QuadValues, RESERVED_NAMESPACE,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, RetainedCharge, RetentionLedger,
    RetentionSnapshot, ScopeBinding, TermRef, TermValue, ViewAccountingReport, ViewLimits,
    ViewOperationStatus, blank_count_view, canonicalize, canonicalize_graph_view,
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

// ---------------------------------------------------------------------------
// Appending a source says the same thing as composing the whole list at once
// ---------------------------------------------------------------------------

/// Every named graph a view addresses, as values rather than view-local handles,
/// so two independently built views can be compared at all.
fn graph_names<D: DatasetView>(view: &D) -> BTreeSet<TermValue> {
    view.named_graphs().map(|id| owned(view, id)).collect()
}

/// One pattern probe expressed in term VALUES: bind each supplied value in the
/// view under test, run the pattern, and resolve the answer back to values. A
/// value interned nowhere matches nothing, which is an empty answer, not a skip.
fn value_probe<D: DatasetView>(
    view: &D,
    s: Option<&TermValue>,
    p: Option<&TermValue>,
    o: Option<&TermValue>,
    g: GraphMatchValue<'_>,
) -> BTreeSet<Row> {
    let bind = |value: Option<&TermValue>| match value {
        None => Some(None),
        Some(value) => view.term_id_by_value(value).map(Some),
    };
    let (Some(s), Some(p), Some(o)) = (bind(s), bind(p), bind(o)) else {
        return BTreeSet::new();
    };
    let graph = match g {
        GraphMatchValue::Any => GraphMatch::Any,
        GraphMatchValue::Default => GraphMatch::Default,
        GraphMatchValue::Named(value) => match view.term_id_by_value(value) {
            Some(id) => GraphMatch::Named(id),
            None => return BTreeSet::new(),
        },
    };
    view.quads_for_pattern(s, p, o, graph)
        .map(|q| row(view, q))
        .collect()
}

/// Two views observed through everything a consumer can ask them: canonical
/// identity, per-graph identity, the whole three-table surface, the named graph
/// set, the term dictionary, every bound-axis pattern probe over every row, and
/// the retention/work accounting.
fn assert_same_view(left: &CompositeDatasetView, right: &CompositeDatasetView) {
    let bytes = canonicalize_view(left, CanonHash::Sha256).nquads;
    assert!(!bytes.is_empty(), "an empty canonical form proves nothing");
    assert_eq!(
        bytes,
        canonicalize_view(right, CanonHash::Sha256).nquads,
        "canonical bytes"
    );
    assert_eq!(
        ContentDigest::of(bytes.as_bytes()),
        ContentDigest::of(
            canonicalize_view(right, CanonHash::Sha256)
                .nquads
                .as_bytes()
        )
    );

    let graphs = graph_names(left);
    assert_eq!(graphs, graph_names(right), "named graph set");
    assert!(!graphs.is_empty(), "the fixture must name some graph");
    for graph in &graphs {
        let TermValue::Iri(iri) = graph else { continue };
        assert_eq!(
            graph_digest_view(left, iri),
            graph_digest_view(right, iri),
            "per-graph digest of {iri}"
        );
    }

    assert_eq!(surface(left), surface(right), "three-table surface");
    assert_eq!(left.term_count(), right.term_count(), "term count");
    assert_eq!(
        left.term_ids()
            .map(|id| left.term_value(id))
            .collect::<BTreeSet<_>>(),
        right
            .term_ids()
            .map(|id| right.term_value(id))
            .collect::<BTreeSet<_>>(),
        "term dictionary"
    );
    assert_eq!(left.stats(), right.stats(), "retention and work accounting");

    let rows: Vec<Row> = left.quads().map(|q| row(left, q)).collect();
    assert!(!rows.is_empty(), "pattern probes need rows to probe with");
    for (s, p, o, g) in &rows {
        for mask in 0..16 {
            let s = (mask & 1 != 0).then_some(s);
            let p = (mask & 2 != 0).then_some(p);
            let o = (mask & 4 != 0).then_some(o);
            let g = if mask & 8 == 0 {
                GraphMatchValue::Any
            } else {
                g.as_ref()
                    .map_or(GraphMatchValue::Default, GraphMatchValue::Named)
            };
            assert_eq!(
                value_probe(left, s, p, o, g),
                value_probe(right, s, p, o, g),
                "pattern probe, mask {mask}"
            );
        }
    }
}

/// Three sources worth appending one at a time: a native document, a delta that
/// never gets compacted, and a second document projected into one graph — so the
/// appended view has to agree about placement, the derived graph dictionary and
/// the statement layer, not only about ordinary rows.
fn bundle_sources() -> Vec<CompositeSource> {
    let first = identity_fixture();
    let second = complete_source();
    let mut mutable = MutableDataset::new(second.clone());
    mutable
        .insert(QuadValues::triple(iri("added"), iri("p"), iri("o")))
        .unwrap();
    let delta = Arc::new(mutable.snapshot_view().unwrap());
    vec![
        CompositeSource::new(first),
        CompositeSource::from_delta(delta)
            .with_graph_placement(GraphPlacement::Named(iri("delta"))),
        CompositeSource::new(second).with_graph_placement(GraphPlacement::Default),
    ]
}

#[test]
fn appending_sources_one_at_a_time_composes_what_the_whole_list_composes() {
    let sources = bundle_sources();
    let scratch =
        CompositeDatasetView::from_bound_sources(sources.clone(), ViewLimits::default()).unwrap();

    let mut stepwise =
        CompositeDatasetView::from_bound_sources(vec![sources[0].clone()], ViewLimits::default())
            .unwrap();
    for source in &sources[1..] {
        let next = stepwise
            .extend(source.clone(), ViewLimits::default())
            .unwrap();
        // Appending publishes a new view; the one appended to is untouched.
        assert!(next.sources().len() > stepwise.sources().len());
        stepwise = next;
    }
    assert_eq!(stepwise.sources().len(), sources.len());
    assert_same_view(&scratch, &stepwise);
    assert_probes(&stepwise);
    assert_eq!(
        surface(&stepwise),
        surface(&stepwise.materialize().unwrap())
    );

    // The intermediate view stays valid and keeps saying what it said: appending
    // to a prefix must not mutate the prefix.
    let two =
        CompositeDatasetView::from_bound_sources(sources[..2].to_vec(), ViewLimits::default())
            .unwrap();
    let appended = two
        .extend(sources[2].clone(), ViewLimits::default())
        .unwrap();
    assert_same_view(&scratch, &appended);
    assert_same_view(
        &two,
        &CompositeDatasetView::from_bound_sources(sources[..2].to_vec(), ViewLimits::default())
            .unwrap(),
    );

    // Appending refuses exactly what composing from scratch refuses, and admits
    // exactly what it admits.
    let tight = ViewLimits {
        max_sources: 2,
        ..ViewLimits::default()
    };
    assert!(two.extend(sources[2].clone(), tight).is_err());
    assert!(CompositeDatasetView::from_bound_sources(sources.clone(), tight).is_err());
    assert!(
        two.extend(sources[2].clone(), ViewLimits::default())
            .is_ok()
    );
    assert!(
        CompositeDatasetView::from_bound_sources(sources, ViewLimits::default()).is_ok(),
        "the neighbouring admissible composition must still succeed"
    );
}

#[test]
fn appending_a_co_referent_source_over_a_standardized_scope_still_matches_from_scratch() {
    // The first source's blanks are standardized onto low scope numbers; the
    // appended one insists on keeping those very scopes. Reserving them after the
    // fact would move every earlier renaming, so the answer must be the one the
    // whole list produces, not the one a blind append would.
    let source = complete_source();
    let standardized = CompositeSource::new(source.clone());
    let co_referent = CompositeSource::new(source).with_scope_binding(ScopeBinding::Shared);
    let scratch = CompositeDatasetView::from_bound_sources(
        vec![standardized.clone(), co_referent.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    let appended =
        CompositeDatasetView::from_bound_sources(vec![standardized], ViewLimits::default())
            .unwrap()
            .extend(co_referent, ViewLimits::default())
            .unwrap();
    assert_same_view(&scratch, &appended);
    // Co-reference was not silently granted: the two occurrences stay two.
    assert_eq!(appended.quads().count(), 2);
    assert_eq!(
        blank_count_view(&appended),
        2 * blank_count_view(&*complete_source())
    );
}

// ---------------------------------------------------------------------------
// Blank identity is stated per source, not chosen once for the whole composite
// ---------------------------------------------------------------------------

#[test]
fn the_modal_constructors_are_the_endpoints_of_the_per_source_binding() {
    let sources = bundle_sources();
    let bind = |binding| {
        sources
            .iter()
            .cloned()
            .map(|source| source.with_scope_binding(binding))
            .collect::<Vec<_>>()
    };

    for binding in [ScopeBinding::Independent, ScopeBinding::Shared] {
        let legacy = if matches!(binding, ScopeBinding::Independent) {
            CompositeDatasetView::from_sources(sources.clone(), ViewLimits::default())
        } else {
            CompositeDatasetView::from_shared_sources(sources.clone(), ViewLimits::default())
        }
        .unwrap();
        let bound =
            CompositeDatasetView::from_bound_sources(bind(binding), ViewLimits::default()).unwrap();
        assert_same_view(&legacy, &bound);
        assert_eq!(legacy.sources()[0].scope_binding(), binding);
    }

    // Both modal constructors OVERRIDE whatever binding their sources carried —
    // that is what makes them modal — so the same list handed to each still
    // separates or collapses by the constructor, never by the source.
    let source = complete_source();
    let marked = vec![
        CompositeSource::new(source.clone()).with_scope_binding(ScopeBinding::Shared),
        CompositeSource::new(source.clone()).with_scope_binding(ScopeBinding::Shared),
    ];
    assert_eq!(
        CompositeDatasetView::from_sources(marked.clone(), ViewLimits::default())
            .unwrap()
            .quads()
            .count(),
        2
    );
    assert_eq!(
        CompositeDatasetView::from_shared_sources(marked, ViewLimits::default())
            .unwrap()
            .quads()
            .count(),
        1
    );
    let unmarked = vec![
        CompositeSource::new(source.clone()),
        CompositeSource::new(source),
    ];
    assert_eq!(
        CompositeDatasetView::from_shared_sources(unmarked, ViewLimits::default())
            .unwrap()
            .quads()
            .count(),
        1
    );
}

#[test]
fn a_delta_co_refers_with_its_base_while_a_third_source_stays_standardized_apart() {
    let base = identity_fixture();
    let delta = Arc::new(delta_of(&base));
    let alone = canonicalize_view(&*base, CanonHash::Sha256).nquads;

    // The reference for "two independent occurrences": the flat dataset built by
    // appending the source to itself.
    let mut doubled = RdfDatasetBuilder::new();
    doubled.push_dataset(&base);
    doubled.push_dataset(&base);
    let doubled = canonicalize(&doubled.freeze().unwrap()).nquads;
    assert_ne!(alone, doubled, "the fixture must carry blanks to separate");

    let native = |binding| CompositeSource::new(base.clone()).with_scope_binding(binding);
    let snapshot = |binding| CompositeSource::from_delta(delta.clone()).with_scope_binding(binding);

    // A delta that co-refers with its base contributes no second copy of anything.
    let co_referent = CompositeDatasetView::from_bound_sources(
        vec![native(ScopeBinding::Shared), snapshot(ScopeBinding::Shared)],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(
        canonicalize_view(&co_referent, CanonHash::Sha256).nquads,
        alone
    );

    // The same two sources standardized apart are two occurrences instead.
    let apart = CompositeDatasetView::from_bound_sources(
        vec![
            native(ScopeBinding::Independent),
            snapshot(ScopeBinding::Independent),
        ],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(canonicalize_view(&apart, CanonHash::Sha256).nquads, doubled);

    // THE MIXED CASE: the delta co-refers with its base, and a third contribution
    // parsed on its own does not. The composite must hold exactly two occurrences
    // — not one (which would mean the third source collapsed) and not three
    // (which would mean the delta did not co-refer).
    let mixed = CompositeDatasetView::from_bound_sources(
        vec![
            native(ScopeBinding::Shared),
            snapshot(ScopeBinding::Shared),
            native(ScopeBinding::Independent),
        ],
        ViewLimits::default(),
    )
    .unwrap();
    let mixed_bytes = canonicalize_view(&mixed, CanonHash::Sha256).nquads;
    assert_eq!(mixed_bytes, doubled, "co-referent pair plus one occurrence");
    assert_ne!(
        mixed_bytes, alone,
        "the independent source must not collapse"
    );
    assert_eq!(blank_count_view(&mixed), 2 * blank_count_view(&*base));
    assert_probes(&mixed);

    // Declaring that third source co-referent too collapses the composite back to
    // one occurrence — the separation was the binding's doing, nothing else's.
    let all_shared = CompositeDatasetView::from_bound_sources(
        vec![
            native(ScopeBinding::Shared),
            snapshot(ScopeBinding::Shared),
            native(ScopeBinding::Shared),
        ],
        ViewLimits::default(),
    )
    .unwrap();
    assert_eq!(
        canonicalize_view(&all_shared, CanonHash::Sha256).nquads,
        alone
    );

    // And the mixed composite is reached by appending, too.
    let appended = CompositeDatasetView::from_bound_sources(
        vec![native(ScopeBinding::Shared), snapshot(ScopeBinding::Shared)],
        ViewLimits::default(),
    )
    .unwrap()
    .extend(native(ScopeBinding::Independent), ViewLimits::default())
    .unwrap();
    assert_same_view(&mixed, &appended);
    assert_eq!(surface(&mixed), surface(&mixed.materialize().unwrap()));
}

// ---------------------------------------------------------------------------
// The infallible views answer the operational checkpoint an engine samples
// ---------------------------------------------------------------------------

/// A view that cannot fault reports `Ready` at BOTH checkpoints an execution
/// boundary samples: before evaluation, and after every accessor has been drained.
fn assert_always_ready<D: FallibleDatasetView<Evidence = ()>>(view: &D, name: &str) {
    assert!(
        matches!(view.operation_status(), ViewOperationStatus::Ready { .. }),
        "{name} before iteration"
    );
    let drained = view.quads().count()
        + view.quad_refs().count()
        + view.reifier_quads().count()
        + view.annotation_quads().count()
        + view.named_graphs().count()
        + view
            .quads_for_pattern(None, None, None, GraphMatch::Any)
            .count();
    assert!(drained > 0, "{name} must actually have been read");
    match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => assert_eq!(evidence, ()),
        ViewOperationStatus::Failed { .. } => panic!("{name} after full iteration"),
    }
}

#[test]
fn infallible_views_certify_ready_before_and_after_a_full_read() {
    let flat = identity_fixture();
    let composite =
        CompositeDatasetView::from_bound_sources(bundle_sources(), ViewLimits::default()).unwrap();
    let delta = delta_of(&flat);

    assert_always_ready(&*flat, "frozen dataset");
    assert_always_ready(&composite, "composite view");
    assert_always_ready(&delta, "delta snapshot");
    // The shared-handle blanket impl forwards rather than defaulting.
    assert_always_ready(&flat, "shared frozen dataset");
    assert_always_ready(&Arc::new(delta_of(&flat)), "shared delta snapshot");
}

// ---------------------------------------------------------------------------
// The shared ledger answers residency; per-view admission is untouched
// ---------------------------------------------------------------------------

/// A base shared by two carriers is RESIDENT once, so the ledger reports its
/// bytes once — while each carrier's own `ViewStats` keeps charging it per
/// source, because admission is a statement about one view's ceilings.
#[test]
fn one_base_shared_by_two_carriers_is_retained_exactly_once() {
    let base = identity_fixture();
    let ledger = RetentionLedger::new();
    let carrier_a = ledger.retain_dataset(&base);
    let carrier_b = ledger.retain_dataset(&base);
    assert_eq!(
        carrier_a.owner_key(),
        carrier_b.owner_key(),
        "the same allocation must key the same owner"
    );

    let shared = ledger.snapshot();
    assert_eq!(shared.distinct_owners, 1);
    assert_eq!(shared.live_guards, 2);
    assert_eq!(shared.retained_sources, 1);
    assert_eq!(shared.retained_terms, base.term_count());
    assert_eq!(shared.retained_rows, base.rdf_row_count());
    assert_eq!(shared.retained_payload_bytes, base.rdf_payload_bytes());

    // The per-view scope is deliberately unchanged: two sources, charged twice.
    let per_view =
        CompositeDatasetView::new(vec![base.clone(), base.clone()], ViewLimits::default())
            .unwrap()
            .stats();
    assert_eq!(per_view.retained_sources, 2);
    assert_eq!(per_view.retained_terms, 2 * base.term_count());
    assert_eq!(
        per_view.retained_payload_bytes,
        2 * base.rdf_payload_bytes()
    );

    // The report keeps RETAINED (deduplicated) and INCREMENTAL (this view's own
    // bookkeeping) in separate fields, so no byte is counted in both.
    let report = ledger.report(&per_view);
    assert_eq!(
        report.retained.retained_payload_bytes,
        base.rdf_payload_bytes()
    );
    assert_eq!(report.incremental_auxiliary_bytes, per_view.auxiliary_bytes);
    assert_eq!(report.incremental_work, per_view.work);
    assert_eq!(
        report.total_accounted_bytes(),
        base.rdf_payload_bytes() + report.retained.memo_bytes + per_view.auxiliary_bytes
    );
    assert!(
        report.total_accounted_bytes() < per_view.retained_payload_bytes + per_view.auxiliary_bytes,
        "the deduplicated total must be strictly smaller than the per-view restatement"
    );
    assert_eq!(
        ViewAccountingReport::new(ledger.snapshot(), &per_view),
        report
    );

    drop(carrier_a);
    drop(carrier_b);
    assert_eq!(ledger.snapshot(), RetentionSnapshot::default());
}

/// The last reader releases the charge, whichever reader that turns out to be:
/// no order leaves a residue, and none takes a total below zero.
#[test]
fn retention_is_released_by_the_last_reader_under_either_drop_order() {
    let base = identity_fixture();
    for reverse in [false, true] {
        let ledger = RetentionLedger::new();
        let first = ledger.retain_dataset(&base);
        // A clone is another reader of the SAME owner, not another owner.
        let cloned = first.clone();
        let third = ledger.retain_dataset(&base);
        // Put something in the memo too, so its release is proved as well.
        let digest = first.memoized_graph_digest(GRAPH, || graph_digest_view(&*base, GRAPH));
        assert_eq!(digest, graph_digest_view(&*base, GRAPH));

        let full = ledger.snapshot();
        assert_eq!(full.distinct_owners, 1);
        assert_eq!(full.live_guards, 3);
        assert_eq!(full.retained_payload_bytes, base.rdf_payload_bytes());
        assert_eq!(full.memo_entries, 1);
        assert!(full.memo_bytes >= GRAPH.len());

        let mut readers = vec![first, cloned, third];
        if reverse {
            readers.reverse();
        }
        while let Some(reader) = readers.pop() {
            drop(reader);
            let now = ledger.snapshot();
            if readers.is_empty() {
                assert_eq!(
                    now,
                    RetentionSnapshot {
                        memo_misses: 1,
                        ..RetentionSnapshot::default()
                    },
                    "the last reader must release everything but the hit/miss history \
                     (reverse={reverse})"
                );
            } else {
                assert_eq!(
                    now.distinct_owners, 1,
                    "a surviving reader keeps the owner (reverse={reverse})"
                );
                assert_eq!(now.live_guards, readers.len());
                assert_eq!(now.retained_payload_bytes, base.rdf_payload_bytes());
                assert_eq!(now.memo_entries, 1);
            }
        }
    }
}

/// Deduplication is by ALLOCATION, never by content: two structurally identical
/// bases are two residents and both are reported. Pointer identity answers "how
/// many bytes are here"; it never answers "are these the same graph".
#[test]
fn two_distinct_bases_each_report_their_own_retention() {
    let one = identity_fixture();
    let two = identity_fixture();
    let ledger = RetentionLedger::new();
    let guard_one = ledger.retain_dataset(&one);
    let guard_two = ledger.retain_dataset(&two);
    assert_ne!(guard_one.owner_key(), guard_two.owner_key());
    assert_eq!(
        graph_digest_view(&*one, GRAPH),
        graph_digest_view(&*two, GRAPH),
        "the two bases are RDF-identical, and are still two residents"
    );

    let shared = ledger.snapshot();
    assert_eq!(shared.distinct_owners, 2);
    assert_eq!(shared.live_guards, 2);
    assert_eq!(shared.retained_sources, 2);
    assert_eq!(shared.retained_terms, one.term_count() + two.term_count());
    assert_eq!(
        shared.retained_rows,
        one.rdf_row_count() + two.rdf_row_count()
    );
    assert_eq!(
        shared.retained_payload_bytes,
        one.rdf_payload_bytes() + two.rdf_payload_bytes()
    );

    // Each keeps its own memo slot: the same graph name under two owners is two
    // entries, and dropping one owner leaves the other's analysis alone.
    let digest = guard_one.memoized_graph_digest(GRAPH, || graph_digest_view(&*one, GRAPH));
    assert_eq!(
        guard_two.memoized_graph_digest(GRAPH, || graph_digest_view(&*two, GRAPH)),
        digest
    );
    assert_eq!(ledger.snapshot().memo_entries, 2);
    assert_eq!(ledger.snapshot().memo_misses, 2);
    drop(guard_one);
    let after = ledger.snapshot();
    assert_eq!(after.distinct_owners, 1);
    assert_eq!(after.retained_payload_bytes, two.rdf_payload_bytes());
    assert_eq!(after.memo_entries, 1);
    drop(guard_two);
    assert_eq!(ledger.snapshot().memo_entries, 0);
}

/// A frozen owner's per-graph identity analysis is computed once and shared by
/// every reader on the ledger; a mutable owner is never memoized at all.
#[test]
fn a_frozen_owner_shares_one_graph_digest_analysis_between_its_readers() {
    let base = identity_fixture();
    let ledger = RetentionLedger::new();
    let reader_a = ledger.retain_dataset(&base);
    let reader_b = ledger.retain_dataset(&base);
    assert_eq!(reader_a.mutability(), OwnerMutability::Frozen);
    assert_eq!(reader_a.charge(), RetainedCharge::of_dataset(&base));

    let direct = graph_digest_view(&*base, GRAPH);
    let cold = ledger.snapshot();
    assert_eq!(
        (cold.memo_entries, cold.memo_hits, cold.memo_misses),
        (0, 0, 0)
    );

    // Miss: the closure runs, the digest is stored, and its bytes are accounted.
    let computed = reader_a.memoized_graph_digest(GRAPH, || graph_digest_view(&*base, GRAPH));
    assert_eq!(computed, direct);
    let warm = ledger.snapshot();
    assert_eq!(
        (warm.memo_entries, warm.memo_hits, warm.memo_misses),
        (1, 0, 1)
    );
    assert!(warm.memo_bytes >= GRAPH.len() + 32, "{}", warm.memo_bytes);

    // Hit: the OTHER carrier's reader never recomputes.
    let reused = reader_b
        .memoized_graph_digest(GRAPH, || panic!("a stored analysis must not be recomputed"));
    assert_eq!(reused, direct);
    let hot = ledger.snapshot();
    assert_eq!(
        (hot.memo_entries, hot.memo_hits, hot.memo_misses),
        (1, 1, 1)
    );
    assert_eq!(hot.memo_bytes, warm.memo_bytes);

    // A second graph is a second entry, and the fallible spelling agrees.
    let other_direct = try_graph_digest_view(&*base, OTHER_GRAPH).unwrap();
    let other = reader_b
        .try_memoized_graph_digest(OTHER_GRAPH, || try_graph_digest_view(&*base, OTHER_GRAPH))
        .unwrap();
    assert_eq!(other, other_direct);
    assert_ne!(other, direct);
    let two_graphs = ledger.snapshot();
    assert_eq!(two_graphs.memo_entries, 2);
    assert_eq!(two_graphs.memo_misses, 2);
    assert!(two_graphs.memo_bytes > warm.memo_bytes);

    // A failed computation stores nothing and is neither a hit nor a miss: no
    // analysis completed, so there is nothing to have shared.
    let failed: Result<ContentDigest, &str> =
        reader_a.try_memoized_graph_digest("http://example.org/absent", || Err("refused"));
    assert_eq!(failed, Err("refused"));
    assert_eq!(ledger.snapshot(), two_graphs);

    // A MUTABLE owner is accounted but never memoized: every call recomputes, and
    // the miss counter says so rather than the ledger claiming stale content.
    let live = identity_fixture();
    let live_reader = ledger.retain(
        &live,
        RetainedCharge::of_dataset(&live),
        OwnerMutability::Mutable,
    );
    for _ in 0..2 {
        assert_eq!(
            live_reader.memoized_graph_digest(GRAPH, || graph_digest_view(&*live, GRAPH)),
            direct
        );
    }
    let mutable = ledger.snapshot();
    assert_eq!(
        mutable.memo_entries, 2,
        "nothing was stored for the mutable owner"
    );
    assert_eq!(mutable.memo_misses, 4);
    assert_eq!(mutable.memo_hits, 1);
    assert_eq!(mutable.distinct_owners, 2, "it is still retained");
    assert_eq!(
        mutable.retained_payload_bytes,
        base.rdf_payload_bytes() + live.rdf_payload_bytes()
    );
}

/// Residency on a ledger does not admit, refuse, or resize anything: the per-view
/// ceilings answer exactly as they did before, for both the refused case and its
/// adequately sized neighbour.
#[test]
fn a_shared_ledger_does_not_move_per_view_admission() {
    let base = identity_fixture();
    let undersized = ViewLimits {
        max_sources: 1,
        ..ViewLimits::default()
    };
    let sources = || vec![base.clone(), base.clone()];

    // The baseline, with no ledger in play.
    let before = CompositeDatasetView::new(sources(), undersized).unwrap_err();
    assert_eq!(before.code, "view-retention-limit");
    assert_eq!(before.message, "view retains 2 sources, limit is 1");

    let ledger = RetentionLedger::new();
    let resident = ledger.retain_dataset(&base);
    assert_eq!(
        ledger.snapshot().retained_payload_bytes,
        base.rdf_payload_bytes()
    );

    // Same refusal, same diagnostic, with the base already resident and reported.
    let after = CompositeDatasetView::new(sources(), undersized).unwrap_err();
    assert_eq!(after.code, before.code);
    assert_eq!(after.message, before.message);
    assert_eq!(after.severity, before.severity);

    // The adequately sized twin still succeeds — a refusal is a claim too.
    let admitted = CompositeDatasetView::new(
        sources(),
        ViewLimits {
            max_sources: 2,
            ..ViewLimits::default()
        },
    )
    .unwrap();
    assert_eq!(admitted.stats().retained_sources, 2);
    assert!(
        admitted.quads().count() > base.quads().count(),
        "the admitted view must actually carry both occurrences"
    );
    assert!(
        datasets_isomorphic(
            &*base,
            &CompositeDatasetView::new(vec![base.clone()], ViewLimits::default()).unwrap()
        ),
        "and a single-source composite still reads exactly its base"
    );

    // The same holds on the byte ceiling: exactly enough is admitted, one byte
    // short is refused, and the ledger's deduplicated total changes neither.
    let exact = ViewLimits {
        max_payload_bytes: base.rdf_payload_bytes(),
        ..ViewLimits::default()
    };
    let short = ViewLimits {
        max_payload_bytes: base.rdf_payload_bytes() - 1,
        ..ViewLimits::default()
    };
    assert!(CompositeDatasetView::new(vec![base.clone()], exact).is_ok());
    assert_eq!(
        CompositeDatasetView::new(vec![base.clone()], short)
            .unwrap_err()
            .code,
        "view-retention-limit"
    );

    // And the per-view stats a ledger-registered base produces are bit-identical
    // to the ones it produces with no ledger anywhere.
    let with_ledger = CompositeDatasetView::new(sources(), ViewLimits::default())
        .unwrap()
        .stats();
    drop(resident);
    assert_eq!(ledger.snapshot(), RetentionSnapshot::default());
    let without_ledger = CompositeDatasetView::new(sources(), ViewLimits::default())
        .unwrap()
        .stats();
    assert_eq!(with_ledger, without_ledger);
}

// ─── Pipeline carriers: the flat bundle and the view bundle agree ─────────────

use purrdf_core::{
    BundleDigestWork, ContentStore, DatasetProvenance, GraphLayer, OriginKind, PipelineBundle,
    PipelineBundleError, PipelineViewBundle, QuadHandle, RdfLookaside, RdfLookasideKind,
    RdfLookasideResource,
};

const CG1: &str = "http://example.org/carrier/g1";
const CG2: &str = "http://example.org/carrier/g2";
const CG3: &str = "http://example.org/carrier/g3";
const CDECLARED: &str = "http://example.org/carrier/declared";
const CABSENT: &str = "http://example.org/carrier/absent";

/// A synthetic pipeline-side handle payload. Concrete pipeline types plug into the
/// same lane; the kernel never learns what they are.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Note(&'static str);

/// The carrier fixture: two quad-bearing named graphs, a declaration-only named
/// graph, default-graph rows, one blank LABEL living in two scopes, a co-referent
/// blank, a triple term, a reifier declaration, a statement annotation and a
/// direction-and-language literal.
///
/// Every one of those is a way the two carriers could disagree: a scope renaming
/// that conflated two same-label blanks, a statement layer selected by membership
/// rather than by its own graph slot, a declaration the root forgot, a literal whose
/// direction did not survive composition.
fn carrier_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(P);
    let q = b.intern_iri("http://example.org/q");
    let o = b.intern_iri("http://example.org/o");
    let g1 = b.intern_iri(CG1);
    let g2 = b.intern_iri(CG2);
    let declared = b.intern_iri(CDECLARED);
    let shared = b.intern_blank("n", BlankScope::DEFAULT);
    let scoped = b.intern_blank("n", BlankScope(4));
    let plain = b.intern_literal(RdfLiteral::simple("plain"));
    let tagged = b.intern_literal(RdfLiteral::language_tagged("hello", "en"));
    let directional = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    b.push_quad(shared, p, plain, None);
    b.push_quad(scoped, p, o, None);
    b.push_quad(shared, p, directional, Some(g1));
    let triple = b.intern_triple(shared, p, directional);
    b.push_quad(scoped, q, triple, Some(g1));
    let reifier = b.intern_blank("statement", BlankScope(7));
    b.push_reifier_in_graph(reifier, triple, Some(g1));
    b.push_annotation_in_graph(reifier, q, tagged, Some(g1));
    b.push_quad(o, p, o, Some(g2));
    b.declare_named_graph(declared);
    b.freeze().unwrap()
}

/// A contribution wholly contained in `graph`, whose blanks reuse the fixture's
/// LABEL and scopes on purpose: a carrier that renamed scopes carelessly would
/// either conflate these nodes with the base's or break their co-reference, and
/// either shows up as a moved digest.
fn contained_contribution(graph: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(P);
    let q = b.intern_iri("http://example.org/q");
    let o = b.intern_iri("http://example.org/o");
    let g = b.intern_iri(graph);
    let node = b.intern_blank("n", BlankScope::DEFAULT);
    let elsewhere = b.intern_blank("n", BlankScope(4));
    b.push_quad(node, p, o, Some(g));
    b.push_quad(node, q, elsewhere, Some(g));
    b.push_quad(elsewhere, p, o, Some(g));
    b.freeze().unwrap()
}

/// Two lookaside resources, two blobs, and a provenance with BOTH public rows
/// (occurrences) and private state no public projection can see (an interned origin
/// set, and a unit/artifact pair that authored nothing).
fn loadout() -> (RdfLookaside, Arc<ContentStore>, DatasetProvenance) {
    let mut lookaside = RdfLookaside::default();
    lookaside.resources.push(
        RdfLookasideResource::new(RdfLookasideKind::Reasoning)
            .with_name("closure")
            .with_digest("d0"),
    );
    lookaside.resources.push(
        RdfLookasideResource::new(RdfLookasideKind::Reasoning)
            .with_name("explanations")
            .with_digest("d1"),
    );

    let mut blobs = ContentStore::new();
    blobs.insert(b"first blob payload".to_vec());
    blobs.insert(b"second blob payload".to_vec());

    let mut provenance = DatasetProvenance::new();
    let unit = provenance.register_unit("units/core", OriginKind::Source);
    let artifact = provenance.register_artifact("units/core/core.ttl");
    provenance.record_occurrence(
        QuadHandle::from_index(0),
        unit,
        artifact,
        Some("core.ttl:1".to_owned()),
    );
    let second = provenance.register_unit("units/derived", OriginKind::Generated);
    let second_artifact = provenance.register_artifact("units/derived/derived.ttl");
    provenance.record_occurrence(QuadHandle::from_index(1), second, second_artifact, None);
    // Private: interned but never projected.
    provenance.intern_origin_set(vec![(unit, artifact), (second, second_artifact)]);
    provenance.register_unit("units/registered-only", OriginKind::Source);
    provenance.register_artifact("units/registered-only/unused.ttl");

    (lookaside, Arc::new(blobs), provenance)
}

fn flat_carrier(base: &Arc<RdfDataset>) -> PipelineBundle<Note> {
    let (lookaside, blobs, provenance) = loadout();
    PipelineBundle::new(base.clone(), lookaside, blobs, provenance)
}

fn view_carrier(base: &Arc<RdfDataset>, ledger: &Arc<RetentionLedger>) -> PipelineViewBundle<Note> {
    let (lookaside, blobs, provenance) = loadout();
    PipelineViewBundle::from_dataset(
        base,
        lookaside,
        blobs,
        provenance,
        ledger,
        ViewLimits::default(),
    )
    .unwrap()
}

/// The core acceptance property: a flat carrier and a view carrier over the same
/// content publish the SAME identities — the bundle digest, every per-graph digest,
/// and the pipeline root — while reaching them by completely different routes (a
/// materialized projection and a panicking canonicalization on one side, an in-place
/// composite read and a fallible one on the other).
#[test]
fn flat_and_view_carriers_agree_on_every_identity_they_publish() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let flat = flat_carrier(&base);
    let view = view_carrier(&base, &ledger);

    assert_eq!(flat.digest(), view.digest().unwrap(), "bundle digest");

    let graphs = flat.named_graph_iris();
    assert_eq!(
        graphs,
        vec![CDECLARED.to_owned(), CG1.to_owned(), CG2.to_owned()],
        "the leaf set must include the declaration-only graph"
    );
    assert_eq!(graphs, view.named_graph_iris());
    for graph in &graphs {
        assert_eq!(
            flat.graph_digest(graph),
            view.graph_digest(graph).unwrap(),
            "per-graph digest of <{graph}>"
        );
    }
    // Absence is an empty subgraph on both carriers, never an error.
    assert_eq!(
        flat.graph_digest(CABSENT),
        view.graph_digest(CABSENT).unwrap()
    );

    assert_eq!(flat.pipeline_root(), view.pipeline_root().unwrap(), "root");

    // Non-vacuity: the fixture's graphs are genuinely different from each other, the
    // declaration-only graph is genuinely empty, and the root is a DIFFERENT identity
    // from the digest rather than the same bytes under another name.
    assert_ne!(flat.graph_digest(CG1), flat.graph_digest(CG2));
    assert_eq!(flat.graph_digest(CDECLARED), flat.graph_digest(CABSENT));
    assert_ne!(flat.digest(), flat.pipeline_root());
    assert!(
        canonicalize_graph_view(&*base, CG1, CanonHash::Sha256)
            .nquads
            .contains("<urn:purrdf:rdfc:reifies>"),
        "the fixture's statement layer must actually reach the per-graph identity"
    );
}

/// The additive pin invariant survives a BLANK-BEARING accumulation on both
/// carriers. An IRI-only contribution cannot see this: standardize-apart renames
/// blank SCOPES on both sides, so a carrier that let a renaming leak into a pinned
/// graph's identity would only be caught by blanks.
#[test]
fn accumulating_blank_bearing_graphs_leaves_every_pin_verified_on_both_carriers() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let mut flat = flat_carrier(&base);
    let mut view = view_carrier(&base, &ledger);

    // <g1> is the graph with the scoped blanks, the triple term and the statement
    // layer. Pin it on both carriers.
    let pinned = flat.graph_digest(CG1);
    assert_eq!(pinned, view.graph_digest(CG1).unwrap());
    flat.pin_handle(CG1, Note("g1"), pinned).unwrap();
    view.pin_handle(CG1, Note("g1"), pinned).unwrap();

    let contribution = contained_contribution(CG3);
    flat.accumulate_named_graph(CG3, &contribution, Note("g3"))
        .expect("flat accumulate must not disturb the pinned graph");
    view.accumulate_named_graph(CG3, &contribution, Note("g3"))
        .expect("view accumulate must not disturb the pinned graph");

    assert_eq!(flat.handle(CG1).unwrap().content_digest, pinned);
    assert_eq!(view.handle(CG1).unwrap().content_digest, pinned);
    assert_eq!(
        flat.graph_digest(CG1),
        pinned,
        "the pin still verifies (flat)"
    );
    assert_eq!(
        view.graph_digest(CG1).unwrap(),
        pinned,
        "the pin still verifies (view)"
    );

    // The contribution genuinely landed, identically, on both carriers.
    assert_eq!(flat.graph_digest(CG3), view.graph_digest(CG3).unwrap());
    assert_ne!(flat.graph_digest(CG3), flat.graph_digest(CABSENT));
    assert_eq!(flat.digest(), view.digest().unwrap());
    assert_eq!(flat.pipeline_root(), view.pipeline_root().unwrap());
}

/// Reseating the per-graph memo rather than clearing a SHARED one: a clone taken
/// before an accumulation answers for its OWN content. Under the clear-in-place
/// behaviour the clone read the digests the other carrier computed against its NEW
/// dataset — a wrong answer, not a stale one.
#[test]
fn a_clone_taken_before_accumulate_reads_its_own_content_on_both_carriers() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let mut flat = flat_carrier(&base);
    let mut view = view_carrier(&base, &ledger);

    let before = flat.graph_digest(CG2);
    assert_eq!(before, view.graph_digest(CG2).unwrap());
    let flat_snapshot = flat.clone();
    let view_snapshot = view.clone();

    // Accumulate INTO the graph that was already read, so the two contents differ.
    let contribution = contained_contribution(CG2);
    flat.accumulate_named_graph(CG2, &contribution, Note("grown"))
        .unwrap();
    view.accumulate_named_graph(CG2, &contribution, Note("grown"))
        .unwrap();

    assert_ne!(
        flat.graph_digest(CG2),
        before,
        "the fixture must actually move <g2>, or the test proves nothing"
    );
    assert_eq!(flat.graph_digest(CG2), view.graph_digest(CG2).unwrap());

    assert_eq!(
        flat_snapshot.graph_digest(CG2),
        before,
        "the pre-accumulate flat clone answers for its own content"
    );
    assert_eq!(
        view_snapshot.graph_digest(CG2).unwrap(),
        before,
        "the pre-accumulate view clone answers for its own content"
    );
    assert_eq!(
        flat_snapshot.digest(),
        view_snapshot.digest().unwrap(),
        "and the two clones still agree with each other"
    );
    assert_eq!(flat_snapshot.digest(), flat_carrier(&base).digest());
}

/// Containment is checked on BOTH carriers, with the same typed refusal, and the
/// neighbouring single-graph contribution — the case the rule exists to admit —
/// still succeeds on both.
#[test]
fn a_contribution_reaching_outside_its_graph_is_refused_by_both_carriers() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();

    // A contribution that also says something in the DEFAULT graph. On the view
    // carrier `GraphPlacement::Named` would silently relocate that row.
    let straying = {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri(P);
        let o = b.intern_iri("http://example.org/o");
        let s = b.intern_iri("http://example.org/s");
        let g = b.intern_iri(CG3);
        b.push_quad(s, p, o, Some(g));
        b.push_quad(o, p, s, None);
        b.freeze().unwrap()
    };

    let mut flat = flat_carrier(&base);
    let mut view = view_carrier(&base, &ledger);
    let flat_before = flat.digest();
    let view_before = view.digest().unwrap();

    let flat_err = flat
        .accumulate_named_graph(CG3, &straying, Note("x"))
        .expect_err("the flat carrier must refuse a straying contribution");
    let view_err = view
        .accumulate_named_graph(CG3, &straying, Note("x"))
        .expect_err("the view carrier must refuse a straying contribution");
    for err in [&flat_err, &view_err] {
        assert!(
            matches!(
                err,
                PipelineBundleError::GraphContainment {
                    layer: GraphLayer::Quad,
                    found: None,
                    ..
                }
            ),
            "{err}"
        );
    }
    assert_eq!(flat.digest(), flat_before, "a refusal folds nothing");
    assert_eq!(view.digest().unwrap(), view_before);
    assert!(flat.handle(CG3).is_none() && view.handle(CG3).is_none());

    // The twin that IS contained still folds, on both carriers, to the same identity.
    let contained = contained_contribution(CG3);
    flat.accumulate_named_graph(CG3, &contained, Note("ok"))
        .expect("the contained twin must still be admitted (flat)");
    view.accumulate_named_graph(CG3, &contained, Note("ok"))
        .expect("the contained twin must still be admitted (view)");
    assert_eq!(flat.digest(), view.digest().unwrap());
    assert_eq!(flat.pipeline_root(), view.pipeline_root().unwrap());
}

/// Exact invalidation: an accumulation re-canonicalizes exactly the graph it
/// touched. The untouched leaves are answered from the reseated memo, and a SECOND
/// carrier over the same frozen base is answered from the ledger's shared memo —
/// the analysis is paid for once between them.
#[test]
fn accumulate_on_the_view_carrier_recomputes_only_the_graph_it_touched() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let mut view = view_carrier(&base, &ledger);

    let g1 = view.graph_digest(CG1).unwrap();
    let g2 = view.graph_digest(CG2).unwrap();
    let before = view.digest_work();
    assert_eq!(
        before,
        BundleDigestWork {
            graph_canonicalizations: 2,
            ..BundleDigestWork::default()
        },
        "two graphs read, two canonicalizations"
    );

    view.accumulate_named_graph(CG3, &contained_contribution(CG3), Note("g3"))
        .unwrap();

    let after = view.digest_work();
    assert_eq!(
        after.graph_canonicalizations,
        before.graph_canonicalizations + 1,
        "exactly the accumulated graph is canonicalized again"
    );

    // The untouched leaves are served, not recomputed.
    assert_eq!(view.graph_digest(CG1).unwrap(), g1);
    assert_eq!(view.graph_digest(CG2).unwrap(), g2);
    let served = view.digest_work();
    assert_eq!(
        served.graph_canonicalizations, after.graph_canonicalizations,
        "an untouched graph is never re-canonicalized after an accumulation"
    );
    assert_eq!(served.graph_cache_hits, after.graph_cache_hits + 2);

    // A second carrier over the same base pays for the per-graph analysis once
    // between them: its own memo is empty, so the answer comes from the ledger.
    let hits_before = ledger.snapshot().memo_hits;
    let second = view_carrier(&base, &ledger);
    assert_eq!(second.digest_work(), BundleDigestWork::default());
    assert_eq!(second.graph_digest(CG1).unwrap(), g1);
    assert_eq!(
        ledger.snapshot().memo_hits,
        hits_before + 1,
        "the shared ledger memo answered"
    );
    assert_eq!(
        second.digest_work().graph_canonicalizations,
        0,
        "and nothing was canonicalized to do it"
    );
}

/// The root tracks content, moves only when content moves, and an accumulation
/// moves exactly one leaf's contribution to it.
#[test]
fn the_pipeline_root_moves_with_one_leaf_and_stands_still_otherwise() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let mut view = view_carrier(&base, &ledger);
    let mut flat = flat_carrier(&base);

    let root_before = view.pipeline_root().unwrap();
    assert_eq!(root_before, flat.pipeline_root());
    // Reading it twice does not move it.
    assert_eq!(view.pipeline_root().unwrap(), root_before);

    let g1 = view.graph_digest(CG1).unwrap();
    let g2 = view.graph_digest(CG2).unwrap();
    let work_before = view.digest_work();

    let contribution = contained_contribution(CG3);
    view.accumulate_named_graph(CG3, &contribution, Note("g3"))
        .unwrap();
    flat.accumulate_named_graph(CG3, &contribution, Note("g3"))
        .unwrap();

    let root_after = view.pipeline_root().unwrap();
    assert_ne!(root_after, root_before, "a new leaf must move the root");
    assert_eq!(root_after, flat.pipeline_root(), "on both carriers alike");
    assert_eq!(view.graph_digest(CG1).unwrap(), g1, "untouched leaf");
    assert_eq!(view.graph_digest(CG2).unwrap(), g2, "untouched leaf");
    assert_eq!(
        view.digest_work().graph_canonicalizations,
        work_before.graph_canonicalizations + 1,
        "exactly one leaf — the accumulated graph — is canonicalized again; the root \
         re-folds the others from the memo the accumulation left intact"
    );
}

/// Admission stays per view and is re-checked CUMULATIVELY as the carrier grows: a
/// ceiling the composed carrier would breach refuses the append, and the
/// adequately-sized twin — the same contribution, one row of headroom more —
/// succeeds.
#[test]
fn cumulative_admission_refuses_an_oversized_append_and_admits_its_twin() {
    let base = carrier_fixture();
    let contribution = contained_contribution(CG3);
    let total = base.rdf_row_count() + contribution.rdf_row_count();
    let ledger = RetentionLedger::new();

    let build = |max_rows: usize| {
        let (lookaside, blobs, provenance) = loadout();
        PipelineViewBundle::<Note>::from_dataset(
            &base,
            lookaside,
            blobs,
            provenance,
            &ledger,
            ViewLimits {
                max_rows,
                ..ViewLimits::default()
            },
        )
        .expect("the base alone is within both ceilings")
    };

    let mut tight = build(total - 1);
    let before = tight.digest().unwrap();
    let err = tight
        .accumulate_named_graph(CG3, &contribution, Note("x"))
        .expect_err("a cumulative breach must hard-fail");
    assert!(
        matches!(err, PipelineBundleError::AdmissionBreach(_)),
        "{err}"
    );
    assert_eq!(
        tight.digest().unwrap(),
        before,
        "a refused append publishes nothing"
    );
    assert!(tight.handle(CG3).is_none());

    let mut roomy = build(total);
    roomy
        .accumulate_named_graph(CG3, &contribution, Note("ok"))
        .expect("the adequately-sized twin must be admitted");
    assert!(roomy.handle(CG3).is_some());
}

/// `stats_fingerprint` is a cache DISCRIMINATOR, never an identity input. Two views
/// with the same content and different fingerprints publish the same digest, the
/// same per-graph digests and the same root.
#[test]
fn content_equal_views_with_different_fingerprints_publish_equal_identities() {
    // Blank-free on purpose: composing the same base twice then deduplicates to one
    // content, while charging retention twice.
    let base = {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri(P);
        let o = b.intern_iri("http://example.org/o");
        let s = b.intern_iri("http://example.org/s");
        let g = b.intern_iri(CG1);
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g));
        b.freeze().unwrap()
    };
    let once = CompositeDatasetView::new(vec![base.clone()], ViewLimits::default()).unwrap();
    let twice = CompositeDatasetView::new(vec![base.clone(), base], ViewLimits::default()).unwrap();
    assert_eq!(
        surface(&once),
        surface(&twice),
        "the two views must hold the same rows, or the test proves nothing"
    );
    assert_ne!(
        once.stats_fingerprint(),
        twice.stats_fingerprint(),
        "the fixture must actually differ in fingerprint"
    );

    let ledger = RetentionLedger::new();
    let carrier = |view: CompositeDatasetView| {
        let (lookaside, blobs, provenance) = loadout();
        PipelineViewBundle::<Note>::from_view(
            view,
            lookaside,
            blobs,
            provenance,
            &ledger,
            ViewLimits::default(),
        )
    };
    let a = carrier(once);
    let b = carrier(twice);
    assert_eq!(a.digest().unwrap(), b.digest().unwrap());
    assert_eq!(a.named_graph_iris(), b.named_graph_iris());
    for graph in a.named_graph_iris() {
        assert_eq!(
            a.graph_digest(&graph).unwrap(),
            b.graph_digest(&graph).unwrap()
        );
    }
    assert_eq!(a.pipeline_root().unwrap(), b.pipeline_root().unwrap());
}

/// Carrier fidelity in BOTH directions: every lookaside resource, blob, provenance
/// record — public and private — and typed handle survives the conversion, the
/// handles still validate against their pins, and the two carriers fold the same
/// sidecar bytes into both identities.
#[test]
fn converting_between_carriers_carries_every_sidecar_handle_and_identity() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let mut flat = flat_carrier(&base);
    let d1 = flat.graph_digest(CG1);
    let d2 = flat.graph_digest(CG2);
    flat.pin_handle(CG1, Note("g1"), d1).unwrap();
    flat.pin_handle(CG2, Note("g2"), d2).unwrap();
    let flat_digest = flat.digest();
    let flat_root = flat.pipeline_root();

    let assert_loadout = |lookaside: &RdfLookaside,
                          blobs: &ContentStore,
                          provenance: &DatasetProvenance,
                          what: &str| {
        assert_eq!(lookaside.resources.len(), 2, "{what}: lookaside resources");
        assert_eq!(
            lookaside
                .resources
                .iter()
                .map(|r| (r.name.clone(), r.content_digest.clone()))
                .collect::<Vec<_>>(),
            flat.lookaside()
                .resources
                .iter()
                .map(|r| (r.name.clone(), r.content_digest.clone()))
                .collect::<Vec<_>>(),
            "{what}: lookaside records"
        );
        assert_eq!(blobs.len(), 2, "{what}: blobs");
        assert_eq!(
            blobs.iter().map(|(d, _)| *d).collect::<BTreeSet<_>>(),
            flat.blobs()
                .iter()
                .map(|(d, _)| *d)
                .collect::<BTreeSet<_>>(),
            "{what}: blob digests"
        );
        assert_eq!(
            provenance.public_projection(),
            flat.provenance().public_projection(),
            "{what}: public provenance"
        );
        assert_eq!(provenance.occurrences.len(), 2, "{what}: occurrences");
        assert_eq!(
            provenance.origin_sets.len(),
            1,
            "{what}: private origin sets"
        );
        assert_eq!(provenance.units.len(), 3, "{what}: private units");
        assert_eq!(provenance.artifacts.len(), 3, "{what}: private artifacts");
    };

    let view = flat
        .to_view_bundle(&ledger, ViewLimits::default())
        .expect("conversion to the view carrier");
    assert_loadout(
        view.lookaside(),
        view.blobs(),
        view.provenance(),
        "to_view_bundle",
    );
    assert_eq!(view.handles(), flat.handles(), "typed handles travel");
    assert_eq!(view.digest().unwrap(), flat_digest);
    assert_eq!(view.pipeline_root().unwrap(), flat_root);
    for (graph, entry) in view.handles() {
        assert_eq!(
            view.graph_digest(graph).unwrap(),
            entry.content_digest,
            "the handle over <{graph}> still validates against its pin"
        );
    }

    // `into_view_bundle` is the by-value spelling of the same conversion.
    let moved = flat
        .clone()
        .into_view_bundle(&ledger, ViewLimits::default())
        .expect("by-value conversion");
    assert_eq!(moved.digest().unwrap(), flat_digest);
    assert_eq!(moved.handles(), flat.handles());

    // One accumulate on the view carrier: the sidecars and pins are unmoved by it.
    let mut grown = view;
    grown
        .accumulate_named_graph(CG3, &contained_contribution(CG3), Note("g3"))
        .expect("accumulate on the converted carrier");
    assert_loadout(
        grown.lookaside(),
        grown.blobs(),
        grown.provenance(),
        "after accumulate",
    );
    assert_eq!(grown.handle(CG1).unwrap().content_digest, d1);
    assert_eq!(grown.handle(CG2).unwrap().content_digest, d2);
    assert_eq!(grown.handle(CG3).unwrap().payload, Note("g3"));
    for (graph, entry) in grown.handles() {
        assert_eq!(grown.graph_digest(graph).unwrap(), entry.content_digest);
    }

    // The reverse conversion exists and is checked in the same terms.
    let back = grown
        .to_flat_bundle()
        .expect("conversion back to the flat carrier");
    assert_loadout(
        back.lookaside(),
        back.blobs(),
        back.provenance(),
        "to_flat_bundle",
    );
    assert_eq!(back.handles(), grown.handles());
    assert_eq!(back.digest(), grown.digest().unwrap());
    assert_eq!(back.pipeline_root(), grown.pipeline_root().unwrap());
    for (graph, entry) in back.handles() {
        assert_eq!(back.graph_digest(graph), entry.content_digest);
    }
    // A round trip that ends where it started.
    let round = back
        .to_view_bundle(&ledger, ViewLimits::default())
        .expect("round trip");
    assert_eq!(round.digest().unwrap(), grown.digest().unwrap());
    assert_eq!(
        round.pipeline_root().unwrap(),
        grown.pipeline_root().unwrap()
    );
}

/// The view carrier composes a delta snapshot without compacting it, and registers
/// BOTH the base it branched from and its delta with the shared ledger.
#[test]
fn a_delta_rides_the_view_carrier_without_compaction() {
    let base = carrier_fixture();
    let delta = Arc::new(delta_of(&base));
    let ledger = RetentionLedger::new();
    let (lookaside, blobs, provenance) = loadout();
    let carrier = PipelineViewBundle::<Note>::from_delta(
        &delta,
        lookaside,
        blobs,
        provenance,
        &ledger,
        ViewLimits::default(),
    )
    .unwrap();

    // The delta's effective content is the base's, so the identities agree with the
    // flat carrier's.
    let flat = flat_carrier(&base);
    assert_eq!(carrier.digest().unwrap(), flat.digest());
    assert_eq!(carrier.pipeline_root().unwrap(), flat.pipeline_root());
    assert_eq!(carrier.named_graph_iris(), flat.named_graph_iris());

    // Both owners are on the ledger, and the base is charged once even though a
    // second carrier over the very same base is alive.
    let second = view_carrier(&base, &ledger);
    assert_eq!(ledger.snapshot().distinct_owners, 2, "base + delta overlay");
    assert_eq!(
        carrier.accounting().retained,
        ledger.snapshot(),
        "the carrier reports the ledger-wide reading verbatim"
    );
    assert_eq!(
        carrier.view_stats().work.materializations,
        0,
        "reading a view carrier never materializes"
    );
    drop(second);
    drop(carrier);
    assert_eq!(
        ledger.snapshot().distinct_owners,
        0,
        "released by the last reader"
    );
}

// ─── Ownership and observability, at the carrier level ───────────────────────

/// The two accounting categories, read off a CARRIER and a separate composite that
/// share one base on one ledger: the base's bytes are RETAINED once between them,
/// while each view keeps its own INCREMENTAL bookkeeping. Nothing is counted in
/// both columns, and the deduplicated reading is strictly smaller than the naive
/// sum of the two views' own restatements.
#[test]
fn a_carrier_and_a_separate_composite_share_one_retention_and_keep_their_own_work() {
    let base = carrier_fixture();
    let ledger = RetentionLedger::new();
    let carrier = view_carrier(&base, &ledger);
    // A SEPARATE composite over the SAME base — two occurrences of it, so its own
    // per-view charge is visibly different from the carrier's.
    let separate =
        CompositeDatasetView::new(vec![base.clone(), base.clone()], ViewLimits::default()).unwrap();
    let separate_guard = ledger.retain_dataset(&base);

    // RETAINED: one allocation, one resident, counted once however many readers.
    let shared = ledger.snapshot();
    assert_eq!(shared.distinct_owners, 1, "one base, one resident");
    assert_eq!(shared.retained_sources, 1);
    assert_eq!(shared.retained_payload_bytes, base.rdf_payload_bytes());
    assert_eq!(
        separate_guard.owner_key(),
        ledger.retain_dataset(&base).owner_key(),
        "the same allocation keys the same owner"
    );

    let from_carrier = carrier.accounting();
    let from_separate = ViewAccountingReport::new(ledger.snapshot(), &separate.stats());
    assert_eq!(
        from_carrier.retained, from_separate.retained,
        "both readers see ONE retention reading, not one apiece"
    );
    assert_eq!(
        from_carrier.retained.retained_payload_bytes,
        base.rdf_payload_bytes()
    );

    // INCREMENTAL: per view, and genuinely different between these two views.
    assert_eq!(carrier.view_stats().retained_sources, 1);
    assert_eq!(separate.stats().retained_sources, 2);
    assert_eq!(
        from_carrier.incremental_auxiliary_bytes,
        carrier.view_stats().auxiliary_bytes
    );
    assert_eq!(
        from_separate.incremental_auxiliary_bytes,
        separate.stats().auxiliary_bytes
    );
    assert_eq!(from_carrier.incremental_work, carrier.view_stats().work);
    assert_eq!(from_separate.incremental_work, separate.stats().work);

    // NO DOUBLE COUNT: each report's total is its own incremental bookkeeping plus
    // the ONE retained reading — never the sum of both views' restatements.
    assert_eq!(
        from_carrier.total_accounted_bytes(),
        base.rdf_payload_bytes()
            + from_carrier.retained.memo_bytes
            + carrier.view_stats().auxiliary_bytes
    );
    assert!(
        from_carrier.total_accounted_bytes()
            < carrier.view_stats().retained_payload_bytes
                + separate.stats().retained_payload_bytes
                + carrier.view_stats().auxiliary_bytes,
        "the deduplicated total must be strictly below the naive per-view sum"
    );

    // NON-VACUITY: the per-view scope is deliberately unchanged by any of this —
    // the separate composite still charges the base to itself twice.
    assert_eq!(
        separate.stats().retained_payload_bytes,
        2 * base.rdf_payload_bytes()
    );
}

/// The base a carrier and a separate reader share is released by whichever of them
/// drops LAST, in either order: no order leaves a residue, and neither order
/// releases the base while a reader is still holding it.
#[test]
fn a_carrier_and_a_separate_reader_release_one_base_under_either_drop_order() {
    let base = carrier_fixture();
    for carrier_first in [false, true] {
        let ledger = RetentionLedger::new();
        let mut carrier = Some(view_carrier(&base, &ledger));
        let separate =
            CompositeDatasetView::new(vec![base.clone()], ViewLimits::default()).unwrap();
        let mut separate_guard = Some(ledger.retain_dataset(&base));
        assert_eq!(ledger.snapshot().distinct_owners, 1);
        assert_eq!(
            ledger.snapshot().retained_payload_bytes,
            base.rdf_payload_bytes()
        );

        // Whichever goes first, the SURVIVOR keeps the base resident.
        if carrier_first {
            drop(carrier.take());
        } else {
            drop(separate_guard.take());
        }
        let midway = ledger.snapshot();
        assert_eq!(
            midway.distinct_owners, 1,
            "a surviving reader keeps the owner (carrier_first={carrier_first})"
        );
        assert_eq!(
            midway.retained_payload_bytes,
            base.rdf_payload_bytes(),
            "and its bytes (carrier_first={carrier_first})"
        );

        // The LAST reader releases everything.
        if carrier_first {
            drop(separate_guard.take());
        } else {
            drop(carrier.take());
        }
        let after = ledger.snapshot();
        assert_eq!(
            after.distinct_owners, 0,
            "released by the last reader (carrier_first={carrier_first})"
        );
        assert_eq!(after.live_guards, 0);
        assert_eq!(after.retained_payload_bytes, 0);
        assert_eq!(after.memo_entries, 0);
        // The separate composite is still readable: it never depended on the ledger
        // for its content, only for reporting its residency.
        assert_eq!(separate.quads().count(), base.quads().count());
    }
}

/// Admission is a HARD gate at carrier CONSTRUCTION, not only at accumulation: a
/// budget the base cannot fit in refuses the carrier with the typed breach, retains
/// nothing on the way out, and the adequately-sized twin — one row of headroom more
/// — composes and reads its whole base.
#[test]
fn an_undersized_budget_refuses_the_view_carrier_and_admits_its_adequate_twin() {
    let base = carrier_fixture();
    let rows = base.rdf_row_count();
    let ledger = RetentionLedger::new();
    let build = |max_rows: usize| {
        let (lookaside, blobs, provenance) = loadout();
        PipelineViewBundle::<Note>::from_dataset(
            &base,
            lookaside,
            blobs,
            provenance,
            &ledger,
            ViewLimits {
                max_rows,
                ..ViewLimits::default()
            },
        )
    };

    let err = build(rows - 1).expect_err("a base the budget cannot hold must be refused");
    match &err {
        PipelineBundleError::AdmissionBreach(diagnostic) => {
            assert_eq!(diagnostic.code, "view-retention-limit");
            assert_eq!(
                diagnostic.message,
                format!("view retains {rows} rows, limit is {}", rows - 1)
            );
        }
        other => panic!("expected an admission breach, got {other}"),
    }
    assert_eq!(
        ledger.snapshot(),
        RetentionSnapshot::default(),
        "a refused carrier retains nothing"
    );

    // THE NEIGHBOURING CASE: exactly enough budget is admitted, and the admitted
    // carrier genuinely holds the whole base rather than a truncation of it.
    let admitted = build(rows).expect("the adequately sized twin must be admitted");
    assert_eq!(admitted.view().quads().count(), base.quads().count());
    assert_eq!(
        admitted.view().reifier_quads().count(),
        base.reifier_quads().count()
    );
    assert_eq!(
        admitted.view().annotation_quads().count(),
        base.annotation_quads().count()
    );
    assert_eq!(
        admitted.named_graph_iris(),
        vec![CDECLARED.to_owned(), CG1.to_owned(), CG2.to_owned()],
        "the declaration-only graph survives the tightest budget that admits at all"
    );
    assert_eq!(
        admitted.digest().unwrap(),
        flat_carrier(&base).digest(),
        "and it publishes the flat carrier's identity"
    );
    assert_eq!(ledger.snapshot().distinct_owners, 1);
}
