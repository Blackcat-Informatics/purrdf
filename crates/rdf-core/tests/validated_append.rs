// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regression coverage for typed immutable carrier publication.

use purrdf_core::{BlankScope, DatasetView, RdfDatasetBuilder, RdfLiteral};

fn blank_graph(scope: BlankScope) -> RdfDatasetBuilder {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_blank("same", scope);
    let p = builder.intern_iri("https://example.org/p");
    let o = builder.intern_literal(RdfLiteral::simple("value"));
    builder.push_quad(s, p, o, None);
    builder
}

#[test]
fn append_validated_preserves_scopes_and_both_graph_qualified_layers() {
    let mut source = blank_graph(BlankScope(1));
    let s = source.intern_blank("same", BlankScope(1));
    let p = source.intern_iri("https://example.org/p");
    let o = source.intern_literal(RdfLiteral {
        direction: Some(purrdf_core::RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("claim", "ar")
    });
    let triple = source.intern_triple(s, p, o);
    let nested = source.intern_triple(s, p, triple);
    let g = source.intern_iri("https://example.org/g");
    let empty = source.intern_iri("https://example.org/empty");
    source.declare_named_graph(empty);
    source.push_reifier_in_graph(s, nested, Some(g));
    source.push_annotation_in_graph(s, p, o, Some(g));
    let mut target = RdfDatasetBuilder::new();
    assert!(target.append_validated(source.validate().expect("valid source")) > 0);
    let graph = target.freeze().expect("valid append");
    assert_eq!(graph.quad_count(), 1);
    assert_eq!(graph.reifiers_with_graph().count(), 1);
    assert_eq!(graph.annotations_with_graph().count(), 1);
    assert_eq!(graph.named_graphs().count(), 2);
    assert!(graph.term_id_by_blank("same", BlankScope(1)).is_some());
    assert!(
        graph
            .reifiers_with_graph()
            .all(|(_, _, graph)| graph.is_some())
    );
    assert!(
        graph
            .annotations_with_graph()
            .all(|(_, _, _, graph)| graph.is_some())
    );
}

#[test]
fn independent_merge_avoids_manually_interned_scopes() {
    let mut target = blank_graph(BlankScope(1));
    target.push_dataset(&blank_graph(BlankScope::DEFAULT).freeze().expect("source"));
    let graph = target.freeze().expect("merge");
    assert_eq!(graph.quad_count(), 2);
    assert!(graph.term_id_by_blank("same", BlankScope(1)).is_some());
    assert!(graph.term_id_by_blank("same", BlankScope(2)).is_some());
}

#[test]
fn independent_merge_avoids_validated_append_scopes() {
    let mut target = RdfDatasetBuilder::new();
    target.append_validated(blank_graph(BlankScope(1)).validate().expect("staged"));
    target.push_dataset(&blank_graph(BlankScope::DEFAULT).freeze().expect("source"));
    let graph = target.freeze().expect("merge");
    assert_eq!(graph.quad_count(), 2);
    assert!(graph.term_id_by_blank("same", BlankScope(2)).is_some());
}

#[test]
fn embedded_only_composite_blank_scope_is_reserved_before_independent_merge() {
    let mut source = RdfDatasetBuilder::new();
    let s = source.intern_iri("https://example.org/list");
    let p = source.intern_iri("https://example.org/p");
    let label = BlankScope(1).qualify_label("same");
    let value = source.intern_literal(RdfLiteral::typed(
        format!("[_:{label}]"),
        "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
    ));
    source.push_quad(s, p, value, None);
    let mut target = RdfDatasetBuilder::new();
    target.append_validated(source.validate().expect("composite literal"));
    target.push_dataset(&blank_graph(BlankScope::DEFAULT).freeze().expect("source"));
    let graph = target.freeze().expect("merge");
    assert!(graph.term_id_by_blank("same", BlankScope(1)).is_some());
    assert!(graph.term_id_by_blank("same", BlankScope(2)).is_some());
}

#[test]
fn validated_append_rebinds_locations_after_deduplication_and_sorting() {
    let mut source = RdfDatasetBuilder::new();
    let z = source.intern_iri("https://example.org/z");
    let a = source.intern_iri("https://example.org/a");
    let p = source.intern_iri("https://example.org/p");
    let handle = source.next_quad_handle();
    source.push_quad(z, p, a, None);
    let location = purrdf_core::RdfLocation::file("source.ttl").with_line(7);
    source.attach_location(handle, location.clone());
    source.push_quad(a, p, z, None);
    let mut target = RdfDatasetBuilder::new();
    let a = target.intern_iri("https://example.org/a");
    let p = target.intern_iri("https://example.org/p");
    let z = target.intern_iri("https://example.org/z");
    target.push_quad(a, p, z, None);
    target.append_validated(source.validate().expect("valid"));
    let graph = target.freeze().expect("publication");
    assert_eq!(graph.quad_count(), 2);
    for (index, quad) in graph.quads().enumerate() {
        let handle = purrdf_core::QuadHandle::from_index(u32::try_from(index).expect("index"));
        if quad.s == z {
            assert_eq!(graph.location_of(handle), Some(&location));
        } else {
            assert_eq!(graph.location_of(handle), None);
        }
    }
}
