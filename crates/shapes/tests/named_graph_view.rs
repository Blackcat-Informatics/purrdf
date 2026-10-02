// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native named-graph restriction, RDF 1.2 projection and validation equivalence.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf::{
    BlankScope, DatasetView, FastSet, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection, TermId, TermRef,
};
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_iri::vocab::{rdf, rdfs};
use purrdf_shapes::data_view::{ShaclDatasetView, ShaclRead};
use purrdf_shapes::engine::{PreparedShapes, ValidationOptions, parse_shapes};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const GRAPH: &str = "https://example.org/selected";
const PREFIXES: &str = "@prefix ex: <https://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";

fn graph_fixture() -> (Arc<RdfDataset>, [TermId; 5]) {
    let mut builder = RdfDatasetBuilder::new();
    let selected = builder.intern_iri(GRAPH);
    let sibling = builder.intern_iri("https://example.org/sibling");
    let blank_graph = builder.intern_blank("graph", BlankScope(9));
    let empty = builder.intern_iri("https://example.org/empty");
    let absent = builder.intern_iri("https://example.org/absent");
    builder.declare_named_graph(empty);
    let subject = builder.intern_iri("https://example.org/focus");
    let predicate = builder.intern_iri("https://example.org/value");
    let annotation = builder.intern_iri("https://example.org/annotation");
    let good = builder.intern_literal(RdfLiteral::simple("good"));
    let bad = builder.intern_literal(RdfLiteral::simple("bad"));
    let mut directional = RdfLiteral::language_tagged("bonjour", "fr");
    directional.direction = Some(RdfTextDirection::Rtl);
    let directional = builder.intern_literal(directional);
    let blank = builder.intern_blank("same", BlankScope(7));
    let other_blank = builder.intern_blank("same", BlankScope(8));
    let triple = builder.intern_triple(blank, predicate, directional);
    for graph in [Some(selected), Some(sibling), None] {
        builder.push_quad(
            subject,
            predicate,
            if graph == Some(selected) { good } else { bad },
            graph,
        );
        builder.push_reifier_in_graph(subject, triple, graph);
        builder.push_annotation_in_graph(
            subject,
            annotation,
            if graph == Some(selected) {
                directional
            } else {
                bad
            },
            graph,
        );
    }
    builder.push_annotation_in_graph(subject, predicate, good, Some(selected));
    builder.push_quad(blank, predicate, triple, Some(selected));
    builder.push_quad(other_blank, predicate, bad, Some(sibling));
    builder.push_quad(blank, annotation, directional, Some(blank_graph));
    (
        builder.freeze().expect("graph fixture"),
        [selected, sibling, blank_graph, empty, absent],
    )
}

#[test]
fn every_prepared_pattern_reads_only_the_selected_graph() {
    let (source, graphs) = graph_fixture();
    let view = ShaclDatasetView::named_graph(Arc::clone(&source), graphs[0]);
    let all: Vec<_> = source
        .quads()
        .chain(source.reifier_quads())
        .chain(source.annotation_quads())
        .collect();
    let expected: FastSet<_> = all
        .iter()
        .copied()
        .filter(|q| q.g == Some(graphs[0]))
        .map(|mut q| {
            q.g = None;
            q
        })
        .collect();
    assert_eq!(view.quads().collect::<FastSet<_>>(), expected);
    assert_eq!(view.quads().count(), expected.len());
    assert!(
        view.quad_refs()
            .all(|row| row.expect("resident row").as_ref::<TermId>().g.is_none())
    );
    let subjects: BTreeSet<_> = all.iter().map(|q| q.s).collect();
    let predicates: BTreeSet<_> = all.iter().map(|q| q.p).collect();
    let objects: BTreeSet<_> = all.iter().map(|q| q.o).collect();
    for mask in 0..8 {
        for graph in [GraphMatch::Any, GraphMatch::Default]
            .into_iter()
            .chain(graphs.into_iter().map(GraphMatch::Named))
        {
            let plan = view.probe_plan(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, graph);
            for &subject in &subjects {
                for &predicate in &predicates {
                    for &object in &objects {
                        let s = (mask & 1 != 0).then_some(subject);
                        let p = (mask & 2 != 0).then_some(predicate);
                        let o = (mask & 4 != 0).then_some(object);
                        let matching: FastSet<_> = expected
                            .iter()
                            .copied()
                            .filter(|q| {
                                s.is_none_or(|id| id == q.s)
                                    && p.is_none_or(|id| id == q.p)
                                    && o.is_none_or(|id| id == q.o)
                                    && graph.matches(q.g)
                            })
                            .collect();
                        let rows: Vec<_> = view
                            .quads_for_pattern_with_plan(&plan, s, p, o, graph)
                            .collect();
                        assert_eq!(rows.len(), matching.len(), "mask={mask} graph={graph:?}");
                        assert_eq!(
                            rows.into_iter().collect::<FastSet<_>>(),
                            matching,
                            "mask={mask} graph={graph:?}"
                        );
                        assert!(
                            view.cardinality_estimate(s, p, o, graph)
                                >= u64::try_from(matching.len()).expect("count")
                        );
                    }
                }
            }
        }
    }
    assert!(view.named_graphs().next().is_none());
    assert!(view.reifier_quads().next().is_none());
    assert!(
        view.reifier_quads_of(source.quads().next().expect("row").s)
            .next()
            .is_none()
    );
    assert!(view.annotation_quads().next().is_none());
    assert!(
        view.annotations_of_with_graph(source.quads().next().expect("row").s)
            .next()
            .is_none()
    );
    for graph in [
        GraphMatch::Any,
        GraphMatch::Default,
        GraphMatch::Named(graphs[0]),
    ] {
        assert!(view.reifier_quads_in_graph(graph).next().is_none());
        assert!(view.annotation_quads_in_graph(graph).next().is_none());
    }
    assert_eq!(view.stats().mapped_terms, 0);
    assert_eq!(view.stats().auxiliary_bytes, 0);
    assert_eq!(view.stats().materializations, 0);
}

#[test]
fn construction_and_warm_native_probes_allocate_nothing_and_preserve_term_ids() {
    let (source, graphs) = graph_fixture();
    let window = CurrentThreadWindow::open();
    let view = ShaclDatasetView::named_graph(Arc::clone(&source), graphs[0]);
    let allocation = window.close();
    assert_eq!(allocation.allocations, 0);
    assert_eq!(allocation.requested_bytes, 0);
    assert_eq!(view.term_count(), DatasetView::term_count(source.as_ref()));
    for index in 0..source.term_count() {
        let id = TermId::from_index(u32::try_from(index).expect("term index"));
        assert_eq!(
            view.term_id_by_value(&source.as_ref().term_value(id))
                .expect("lookup"),
            Some(id)
        );
        assert_eq!(view.resolve_term(id), source.as_ref().resolve(id));
        if let (TermRef::Iri(view_iri), TermRef::Iri(source_iri)) =
            (view.resolve_term(id), source.as_ref().resolve(id))
        {
            assert_eq!(view_iri.as_ptr(), source_iri.as_ptr());
        }
    }
    let subject = source
        .term_id_by_iri("https://example.org/focus")
        .expect("subject");
    let predicate = source
        .term_id_by_iri("https://example.org/value")
        .expect("predicate");
    let plan = view.probe_plan(true, true, false, GraphMatch::Default);
    assert_eq!(
        view.quads_for_pattern_with_plan(
            &plan,
            Some(subject),
            Some(predicate),
            None,
            GraphMatch::Default
        )
        .count(),
        1
    );
    let window = CurrentThreadWindow::open();
    for _ in 0..100 {
        assert_eq!(
            view.quads_for_pattern_with_plan(
                &plan,
                Some(subject),
                Some(predicate),
                None,
                GraphMatch::Default
            )
            .count(),
            1
        );
    }
    assert_eq!(window.close().allocations, 0);
}

#[test]
fn blank_empty_and_absent_graphs_are_selected_without_leaking_the_source() {
    let (source, graphs) = graph_fixture();
    let blank = ShaclDatasetView::named_graph(Arc::clone(&source), graphs[2]);
    assert_eq!(blank.quads().count(), 1);
    assert!(blank.quads().all(|q| q.g.is_none()));
    for graph in [graphs[3], graphs[4]] {
        let view = ShaclDatasetView::named_graph(Arc::clone(&source), graph);
        assert!(view.quads().next().is_none());
        assert!(view.named_graphs().next().is_none());
        assert_eq!(
            view.cardinality_estimate(None, None, None, GraphMatch::Default),
            0
        );
    }
    assert_ne!(
        blank.stats_fingerprint(),
        ShaclDatasetView::named_graph(Arc::clone(&source), graphs[0]).stats_fingerprint()
    );
}

#[test]
fn explicit_materialization_contains_exactly_the_projected_selected_graph() {
    let (source, graphs) = graph_fixture();
    let view = ShaclDatasetView::named_graph(Arc::clone(&source), graphs[0]);
    let oracle = ShaclDatasetView::project(Arc::new(source.project_named_graph(GRAPH)));
    let owned_rows = |dataset: &RdfDataset| dataset.owned_quads().collect::<FastSet<_>>();
    assert_eq!(
        owned_rows(view.materialized()),
        owned_rows(oracle.materialized())
    );
    assert!(!Arc::ptr_eq(view.materialized(), &source));
    assert!(Arc::ptr_eq(view.materialized(), view.materialized()));
    assert_eq!(view.stats().materializations, 1);
}

fn validation_fixture() -> (Arc<RdfDataset>, TermId) {
    let mut builder = RdfDatasetBuilder::new();
    let graph = builder.intern_iri(GRAPH);
    let sibling = builder.intern_iri("https://example.org/sibling");
    let rdf_type = builder.intern_iri(rdf::TYPE);
    let subclass = builder.intern_iri(rdfs::SUB_CLASS_OF);
    let child = builder.intern_iri("https://example.org/Child");
    let parent = builder.intern_iri("https://example.org/Parent");
    let value = builder.intern_iri("https://example.org/value");
    let next = builder.intern_iri("https://example.org/next");
    let focus = builder.intern_iri("https://example.org/focus");
    let hidden = builder.intern_iri("https://example.org/hidden");
    let tail = builder.intern_iri("https://example.org/tail");
    let good = builder.intern_literal(RdfLiteral::simple("good"));
    let bad = builder.intern_literal(RdfLiteral::simple("bad"));
    builder.push_quad(child, subclass, parent, Some(graph));
    builder.push_quad(focus, rdf_type, child, Some(graph));
    builder.push_quad(focus, value, good, Some(graph));
    builder.push_quad(focus, next, tail, Some(graph));
    builder.push_quad(tail, value, good, Some(graph));
    for excluded in [Some(sibling), None] {
        builder.push_quad(hidden, rdf_type, parent, excluded);
        builder.push_quad(focus, value, bad, excluded);
        builder.push_quad(tail, value, bad, excluded);
    }
    (builder.freeze().expect("validation fixture"), graph)
}

#[test]
fn core_classes_paths_and_sparql_return_the_same_complete_report_as_an_owned_graph() {
    let (source, graph) = validation_fixture();
    let view = Arc::new(ShaclDatasetView::named_graph(Arc::clone(&source), graph));
    let shapes = Arc::new(parse_shapes(&format!(r#"{PREFIXES}
        ex:Core a sh:NodeShape; sh:targetClass ex:Parent;
            sh:property [ sh:path ex:value; sh:minCount 2 ];
            sh:property [ sh:path (ex:next ex:value); sh:maxCount 0 ] .
        ex:SPARQL a sh:NodeShape; sh:targetClass ex:Parent;
            sh:sparql [ sh:select "SELECT $this ?value WHERE {{ $this <https://example.org/value> ?value }}" ];
            sh:sparql [ sh:select "SELECT $this WHERE {{ GRAPH ?graph {{ ?s ?p ?o }} }}" ];
            sh:sparql [ sh:select "SELECT $this WHERE {{ GRAPH <https://example.org/selected> {{ ?s ?p ?o }} }}" ];
            sh:sparql [ sh:select "SELECT $this WHERE {{ GRAPH <https://example.org/sibling> {{ ?s ?p ?o }} }}" ] .
    "#), None).expect("shapes"));
    let prepared = PreparedShapes::new(shapes);
    let expected = prepared
        .bind_dataset(&source.project_named_graph(GRAPH))
        .expect("owned bind")
        .validate()
        .expect("owned report");
    let validator = prepared.bind_view(Arc::clone(&view)).expect("view bind");
    let report = validator.validate().expect("view report");
    assert_eq!(report.results.len(), 3);
    assert_eq!(report.to_ntriples(), expected.to_ntriples());
    assert!(
        validator
            .view_stats()
            .iter()
            .all(|stats| stats.materializations == 0)
    );
    assert_eq!(view.stats().materializations, 0);
}

#[test]
fn shape_only_class_terms_preserve_the_selection_after_dictionary_supplementation() {
    let (source, graph) = validation_fixture();
    let mut shapes = parse_shapes(&format!(r#"{PREFIXES}
        ex:Parent rdfs:subClassOf ex:OnlyInShapes .
        ex:Core a sh:NodeShape; sh:targetClass ex:OnlyInShapes;
            sh:property [ sh:path ex:value; sh:minCount 2 ] .
        ex:SPARQL a sh:NodeShape; sh:targetClass ex:OnlyInShapes;
            sh:sparql [ sh:select "SELECT $this ?value WHERE {{ $this <https://example.org/value> ?value }}" ];
            sh:sparql [ sh:select "SELECT $this WHERE {{ GRAPH ?graph {{ ?s ?p ?o }} }}" ] .
    "#), None).expect("supplement shapes");
    shapes.set_validation_options(
        ValidationOptions::default().with_subclass_of_in_shapes_graph(true),
    );
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let expected = prepared
        .bind_dataset(&source.project_named_graph(GRAPH))
        .expect("owned bind")
        .validate()
        .expect("owned report");
    let validator = prepared
        .bind_view(Arc::new(ShaclDatasetView::named_graph(source, graph)))
        .expect("selected bind");
    let report = validator.validate().expect("view report");
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.to_ntriples(), expected.to_ntriples());
    assert!(
        validator
            .view_stats()
            .iter()
            .all(|stats| stats.materializations == 0)
    );
}

#[test]
fn metadata_in_an_invisible_graph_does_not_allocate_a_base_row_dedup_table() {
    let mut builder = RdfDatasetBuilder::new();
    let selected = builder.intern_iri(GRAPH);
    let sibling = builder.intern_iri("https://example.org/sibling");
    let predicate = builder.intern_iri("https://example.org/value");
    let object = builder.intern_literal(RdfLiteral::simple("value"));
    for index in 0..256 {
        let subject = builder.intern_iri(&format!("https://example.org/subject{index}"));
        builder.push_quad(subject, predicate, object, Some(selected));
        builder.push_annotation_in_graph(subject, predicate, object, Some(sibling));
    }
    let source = builder.freeze().expect("metadata fixture");
    let view = ShaclDatasetView::named_graph(source, selected);
    assert_eq!(view.quads().count(), 256);
    let window = CurrentThreadWindow::open();
    let count = view.quads().count();
    let measured = window.close();
    assert_eq!(count, 256);
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
}

#[test]
fn collection_and_object_readers_honor_the_selected_default_graph() {
    let mut builder = RdfDatasetBuilder::new();
    let selected = builder.intern_iri(GRAPH);
    let sibling = builder.intern_iri("https://example.org/sibling");
    let head = builder.intern_blank("list", BlankScope(11));
    let first = builder.intern_iri(rdf::FIRST);
    let rest = builder.intern_iri(rdf::REST);
    let nil = builder.intern_iri(rdf::NIL);
    let good = builder.intern_literal(RdfLiteral::simple("good"));
    let bad = builder.intern_literal(RdfLiteral::simple("bad"));
    builder.push_quad(head, first, good, Some(selected));
    builder.push_quad(head, rest, nil, Some(selected));
    builder.push_quad(head, first, bad, Some(sibling));
    builder.push_quad(head, rest, head, Some(sibling));
    builder.push_quad(head, first, bad, None);
    let view = ShaclDatasetView::named_graph(builder.freeze().expect("list fixture"), selected);
    for graph in [GraphMatch::Any, GraphMatch::Default] {
        assert_eq!(view.objects(head, first, graph), [good]);
        assert_eq!(view.objects_of_predicate(first, graph), [good]);
        assert_eq!(
            view.rdf_list_strict(head, graph).expect("selected list"),
            [good]
        );
        assert_eq!(view.rdf_list(head, graph).expect("selected list"), [good]);
        assert!(view.is_cons_cell(head, first, Some(rest), graph));
    }
    for graph in [GraphMatch::Named(selected), GraphMatch::Named(sibling)] {
        assert_eq!(view.objects(head, first, graph), [] as [TermId; 0]);
        assert_eq!(view.objects_of_predicate(first, graph), [] as [TermId; 0]);
        assert!(view.rdf_list_strict(head, graph).is_err());
        assert!(!view.is_cons_cell(head, first, Some(rest), graph));
    }
}
