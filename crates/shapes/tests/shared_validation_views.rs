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
