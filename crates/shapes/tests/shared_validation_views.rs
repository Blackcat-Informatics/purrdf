// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression coverage for typed immutable carrier publication.

use purrdf::ir::{CompositeDatasetView, ViewLimits};
use purrdf::{
    BlankScope, DatasetMut, DatasetView, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, TermValue,
};
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes, project_dataset};
use std::sync::Arc;

const PREFIX: &str =
    "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <https://example.org/> .";
const VALUE: &str = "https://example.org/value";

fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("same", BlankScope(7));
    let p = b.intern_iri(VALUE);
    let o = b.intern_literal(RdfLiteral::simple("good"));
    let g = b.intern_iri("https://example.org/graph");
    b.push_quad(s, p, o, Some(g));
    b.push_annotation_in_graph(s, p, o, Some(g));
    b.freeze().expect("data")
}

#[test]
fn borrowed_projection_matches_native_and_deduplicates_statement_layers() {
    let source = data();
    let projected = project_dataset(&source).expect("native projection");
    let view = Arc::new(ShaclDatasetView::project(Arc::clone(&source)));
    assert_eq!(
        view.quads().count(),
        1,
        "ordinary and annotation projections are one RDF triple"
    );
    let shapes = Arc::new(parse_shapes(&format!("{PREFIX} ex:S a sh:NodeShape; sh:targetSubjectsOf ex:value; sh:property [ sh:path ex:value; sh:minCount 1; sh:maxCount 1 ] ."),None).expect("shapes"));
    let prepared = PreparedShapes::new(shapes);
    let native = prepared
        .bind_projected_dataset(projected)
        .expect("native")
        .validate()
        .expect("report");
    let borrowed = prepared
        .bind_view(Arc::clone(&view))
        .expect("borrowed")
        .validate()
        .expect("report");
    assert_eq!(format!("{native:?}"), format!("{borrowed:?}"));
    assert!(borrowed.conforms);
    assert_eq!(view.stats().materializations, 0);
}

#[test]
fn prepared_delta_binding_observes_suppression_and_new_values_without_freezing_base() {
    let source = data();
    let mut mutation = MutableDataset::new(Arc::clone(&source));
    assert!(mutation.remove(&QuadValues {
        s: TermValue::Blank {
            label: "same".to_owned(),
            scope: BlankScope(7)
        },
        p: TermValue::iri(VALUE),
        o: TermValue::simple_literal("good"),
        g: Some(TermValue::iri("https://example.org/graph")),
    }));
    mutation
        .insert(QuadValues {
            s: TermValue::iri("https://example.org/new"),
            p: TermValue::iri(VALUE),
            o: TermValue::simple_literal("good"),
            g: None,
        })
        .expect("insert");
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    let view = Arc::new(
        ShaclDatasetView::delta(Arc::clone(&snapshot), true, ViewLimits::default()).expect("view"),
    );
    let shapes = Arc::new(parse_shapes(&format!("{PREFIX} ex:S a sh:NodeShape; sh:targetSubjectsOf ex:value; sh:property [ sh:path ex:value; sh:minCount 2 ] ."),None).expect("shapes"));
    let prepared = PreparedShapes::new(shapes);
    let report = prepared
        .bind_view(Arc::clone(&view))
        .expect("bind")
        .validate()
        .expect("report");
    assert_eq!(report.results.len(), 1);
    assert_eq!(view.stats().materializations, 0);
    assert!(Arc::ptr_eq(snapshot.base(), &source));
    let owned = mutation.freeze().expect("ownership boundary");
    let native = prepared
        .bind_dataset(&owned)
        .expect("native")
        .validate()
        .expect("native report");
    assert_eq!(format!("{report:?}"), format!("{native:?}"));
}

#[test]
fn independently_scoped_composite_targets_remain_distinct() {
    let source = data();
    let composite = Arc::new(
        CompositeDatasetView::new(vec![Arc::clone(&source), source], ViewLimits::default())
            .expect("composite"),
    );
    let view = Arc::new(
        ShaclDatasetView::composite(composite, true, ViewLimits::default()).expect("view"),
    );
    let shapes = Arc::new(parse_shapes(&format!("{PREFIX} ex:S a sh:NodeShape; sh:targetSubjectsOf ex:value; sh:property [ sh:path ex:value; sh:minCount 2 ] ."),None).expect("shapes"));
    let report = PreparedShapes::new(shapes)
        .bind_view(Arc::clone(&view))
        .expect("bind")
        .validate()
        .expect("report");
    assert_eq!(report.results.len(), 2);
    assert_ne!(report.results[0].focus_node, report.results[1].focus_node);
    assert_eq!(view.stats().materializations, 0);
}

#[test]
fn same_document_blank_target_and_current_shape_keep_their_identity() {
    let mut b = RdfDatasetBuilder::new();
    let shape = b.intern_blank("shape", BlankScope(9));
    let focus = b.intern_blank("focus", BlankScope(9));
    let constraint = b.intern_blank("constraint", BlankScope(9));
    let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let node_shape = b.intern_iri("http://www.w3.org/ns/shacl#NodeShape");
    b.push_quad(shape, ty, node_shape, None);
    let target = b.intern_iri("http://www.w3.org/ns/shacl#targetNode");
    b.push_quad(shape, target, focus, None);
    let sparql = b.intern_iri("http://www.w3.org/ns/shacl#sparql");
    b.push_quad(shape, sparql, constraint, None);
    let select = b.intern_iri("http://www.w3.org/ns/shacl#select");
    let query = b.intern_literal(RdfLiteral::simple("SELECT $this WHERE { GRAPH $shapesGraph { $currentShape <http://www.w3.org/ns/shacl#targetNode> $this } }"));
    b.push_quad(constraint, select, query, None);
    let source = b.freeze().expect("source");
    let shapes = Arc::new(purrdf_shapes::shapes::from_dataset(&source).expect("shapes"));
    let prepared = PreparedShapes::new(shapes);
    let bound = prepared
        .bind_shared_dataset_with_shapes_graph(
            source,
            Some("https://example.org/shapes"),
            ViewLimits::default(),
        )
        .expect("bind");
    let report = bound.validate().expect("report");
    assert_eq!(
        report.results.len(),
        1,
        "both prebound blank identities must reach the named shapes graph"
    );
}

#[test]
fn validation_handle_budget_is_admitted_before_mapping() {
    let source = data();
    let snapshot = Arc::new(
        MutableDataset::new(source)
            .snapshot_view()
            .expect("snapshot"),
    );
    let limits = ViewLimits {
        max_auxiliary_bytes: 0,
        ..ViewLimits::default()
    };
    assert!(ShaclDatasetView::delta(snapshot, true, limits).is_err());
}

#[test]
fn sparql_statement_projection_deduplicates_without_flattening_shapes_graph() {
    let shapes = Arc::new(parse_shapes(&format!(r#"{PREFIX}
        ex:S a sh:NodeShape; sh:targetSubjectsOf ex:value;
            sh:sparql [ sh:select "SELECT $this WHERE {{ {{ SELECT $this (COUNT(?value) AS ?count) WHERE {{ $this <https://example.org/value> ?value }} GROUP BY $this }} FILTER(?count != 1) }}" ] .
    "#),None).expect("shapes"));
    let bound = PreparedShapes::new(shapes)
        .bind_shared_dataset_with_shapes_graph(
            data(),
            Some("https://example.org/shapes"),
            ViewLimits::default(),
        )
        .expect("bind");
    assert!(bound.validate().expect("report").conforms);
    assert!(
        bound
            .view_stats()
            .iter()
            .all(|stats| stats.materializations == 0)
    );
}

fn property_pair_views() -> (Arc<RdfDataset>, Vec<(&'static str, Arc<ShaclDatasetView>)>) {
    let mut builder = RdfDatasetBuilder::new();
    let focus = builder.intern_iri("https://example.org/focus");
    let left = builder.intern_iri("https://example.org/left");
    let right = builder.intern_iri("https://example.org/right");
    let graph = builder.intern_iri("https://example.org/original");
    for (predicate, values) in [(left, &[1, 2, 4][..]), (right, &[2, 3][..])] {
        for value in values {
            let object = builder.intern_literal(RdfLiteral::typed(
                value.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            ));
            builder.push_quad(focus, predicate, object, Some(graph));
            // Duplicate statement metadata must still represent one projected value.
            builder.push_annotation_in_graph(focus, predicate, object, Some(graph));
        }
    }
    let source = builder.freeze().expect("pair data");
    let composite = Arc::new(
        CompositeDatasetView::new(
            vec![Arc::clone(&source), Arc::clone(&source)],
            ViewLimits::default(),
        )
        .expect("composite"),
    );
    let mut mutable = MutableDataset::new(Arc::clone(&source));
    mutable
        .insert(QuadValues {
            s: TermValue::iri("https://example.org/focus"),
            p: TermValue::iri("https://example.org/left"),
            o: TermValue::Literal {
                lexical_form: "4".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            },
            g: Some(TermValue::iri("https://example.org/added")),
        })
        .expect("delta quad");
    let delta = Arc::new(mutable.snapshot_view().expect("snapshot"));
    let views = vec![
        (
            "native",
            Arc::new(ShaclDatasetView::project(Arc::clone(&source))),
        ),
        (
            "composite",
            Arc::new(
                ShaclDatasetView::composite(composite, true, ViewLimits::default())
                    .expect("composite validation view"),
            ),
        ),
        (
            "delta",
            Arc::new(
                ShaclDatasetView::delta(delta, true, ViewLimits::default())
                    .expect("delta validation view"),
            ),
        ),
    ];
    (source, views)
}

#[test]
fn property_pair_constraints_borrow_every_shared_carrier_without_materialization() {
    let (source, views) = property_pair_views();
    for (constraint, violations) in [
        ("equals", 3),
        ("disjoint", 1),
        ("lessThan", 3),
        ("lessThanOrEquals", 2),
    ] {
        let shapes = Arc::new(
            parse_shapes(
                &format!(
                    "{PREFIX} ex:PairShape a sh:NodeShape; sh:targetNode ex:focus; \
                     sh:property [ sh:path ex:left; sh:{constraint} ex:right ] ."
                ),
                None,
            )
            .expect("pair shape"),
        );
        let prepared = PreparedShapes::new(shapes);
        let expected = prepared
            .bind_dataset(&source)
            .expect("owned oracle")
            .validate()
            .expect("owned report");
        assert_eq!(expected.results.len(), violations, "{constraint}");
        for (kind, view) in &views {
            let bound = prepared.bind_view(Arc::clone(view)).expect("bind view");
            let report = bound.validate().expect("view report");
            assert_eq!(
                report.to_ntriples(),
                expected.to_ntriples(),
                "{kind} {constraint}"
            );
            assert_eq!(view.stats().materializations, 0, "{kind} {constraint}");
            assert!(
                bound
                    .view_stats()
                    .iter()
                    .all(|stats| stats.materializations == 0),
                "{kind} {constraint}"
            );
        }
    }
}

#[test]
fn prepared_sparql_constraints_probe_the_shared_graph_union() {
    let (source, views) = property_pair_views();
    let shapes = Arc::new(parse_shapes(&format!(r#"{PREFIX}
        ex:S a sh:NodeShape; sh:targetNode ex:focus;
            sh:sparql [ sh:select "SELECT $this ?value WHERE {{ $this <https://example.org/left> ?value }}" ] .
    "#), None).expect("SPARQL shape"));
    let prepared = PreparedShapes::new(shapes);
    let expected = prepared
        .bind_dataset(&source)
        .expect("owned")
        .validate()
        .expect("owned report");
    assert_eq!(expected.results.len(), 3);
    for (kind, view) in views {
        let report = prepared
            .bind_view(Arc::clone(&view))
            .expect("bind")
            .validate()
            .expect("shared report");
        assert_eq!(report.to_ntriples(), expected.to_ntriples(), "{kind}");
        assert_eq!(view.stats().materializations, 0, "{kind}");
    }
}

#[test]
fn prepared_probe_plans_preserve_graph_patterns_and_union_duplicates() {
    let (source, projected) = property_pair_views();
    let mut views: Vec<_> = projected
        .into_iter()
        .map(|(kind, view)| (kind, view, true))
        .collect();
    views.push((
        "native graphs",
        Arc::new(ShaclDatasetView::native(Arc::clone(&source))),
        false,
    ));
    let composite = Arc::new(
        CompositeDatasetView::new(
            vec![Arc::clone(&source), Arc::clone(&source)],
            ViewLimits::default(),
        )
        .expect("composite"),
    );
    views.push((
        "composite graphs",
        Arc::new(
            ShaclDatasetView::composite(composite, false, ViewLimits::default()).expect("view"),
        ),
        false,
    ));
    let delta = Arc::new(
        MutableDataset::new(Arc::clone(&source))
            .snapshot_view()
            .expect("snapshot"),
    );
    views.push((
        "delta graphs",
        Arc::new(ShaclDatasetView::delta(delta, false, ViewLimits::default()).expect("view")),
        false,
    ));
    for (kind, view, projected) in views {
        assert_probe_pattern_matrix(&source, kind, &view, projected);
    }
}

fn assert_probe_pattern_matrix(
    source: &RdfDataset,
    kind: &str,
    view: &ShaclDatasetView,
    projected: bool,
) {
    use purrdf::GraphMatch;
    use std::collections::BTreeSet;

    let id = |local| {
        view.term_id_by_value(&TermValue::iri(format!("https://example.org/{local}")))
            .expect("IRI")
    };
    let subjects = [id("focus"), id("original")];
    let predicates = [id("left"), id("right")];
    let objects: Vec<_> = source
        .quads()
        .map(|q| {
            view.term_id_by_value(&source.term_value(q.o))
                .expect("object")
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let rows: BTreeSet<_> = source
        .quads()
        .map(|q| {
            let map = |term| {
                view.term_id_by_value(&source.term_value(term))
                    .expect("term")
            };
            (
                map(q.s),
                map(q.p),
                map(q.o),
                if projected { None } else { q.g.map(map) },
            )
        })
        .collect();
    for mask in 0..8 {
        for graph in [
            GraphMatch::Any,
            GraphMatch::Default,
            GraphMatch::Named(id("original")),
        ] {
            let plan = view.probe_plan(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, graph);
            for &subject in &subjects {
                for &predicate in &predicates {
                    for &object in &objects {
                        let s = (mask & 1 != 0).then_some(subject);
                        let p = (mask & 2 != 0).then_some(predicate);
                        let o = (mask & 4 != 0).then_some(object);
                        let expected: BTreeSet<_> = rows
                            .iter()
                            .copied()
                            .filter(|&(rs, rp, ro, rg)| {
                                s.is_none_or(|s| s == rs)
                                    && p.is_none_or(|p| p == rp)
                                    && o.is_none_or(|o| o == ro)
                                    && graph.matches(rg)
                            })
                            .collect();
                        let actual: Vec<_> =
                            DatasetView::quads_for_pattern_with_plan(view, &plan, s, p, o, graph)
                                .map(|q| (q.s, q.p, q.o, q.g))
                                .collect();
                        assert_eq!(
                            actual.len(),
                            expected.len(),
                            "{kind} mask={mask} graph={graph:?}"
                        );
                        assert_eq!(
                            actual.into_iter().collect::<BTreeSet<_>>(),
                            expected,
                            "{kind} mask={mask} graph={graph:?}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(view.stats().materializations, 0, "{kind}");
}
