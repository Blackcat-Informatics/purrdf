// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Import ownership and declaration preservation at the native carrier boundary.

use purrdf_core::ir::import::DatasetImporter;
use purrdf_core::ir::pack::dataset_from_view;
use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermValue};

#[test]
fn separate_imports_do_not_alias_source_local_ids_or_scoped_blanks() {
    let mut destination = RdfDatasetBuilder::new();
    for scope in [BlankScope(17), BlankScope(18)] {
        let mut source = RdfDatasetBuilder::new();
        let subject = source.intern_blank("same", scope);
        let predicate = source.intern_iri("http://example.org/p");
        let object = source.intern_iri(&format!("http://example.org/value{}", scope.0));
        source.push_quad(subject, predicate, object, None);
        let source = source.freeze().unwrap();
        let mut importer = DatasetImporter::new(&mut destination, &source);
        importer.append();
        assert_eq!(importer.stats().quads, 1);
    }
    let dataset = destination.freeze().unwrap();
    assert_eq!(dataset.quad_count(), 2);
    for quad in dataset.quads() {
        let TermValue::Blank { scope, .. } = dataset.term_value(quad.s) else {
            panic!("expected scoped blank");
        };
        assert_eq!(
            dataset.term_value(quad.o),
            TermValue::iri(format!("http://example.org/value{}", scope.0))
        );
    }
}

#[test]
fn materialization_preserves_empty_graphs_and_the_complete_statement_layer() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_blank("subject", BlankScope(23));
    let predicate = builder.intern_iri("http://example.org/p");
    let literal = builder.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("claim", "ar")
    });
    let quoted = builder.intern_triple(subject, predicate, literal);
    let nested = builder.intern_triple(subject, predicate, quoted);
    let reifier = builder.intern_iri("http://example.org/claim");
    let graph = builder.intern_iri("http://example.org/graph");
    let empty = builder.intern_iri("http://example.org/empty");
    builder.declare_named_graph(empty);
    builder.push_quad(subject, predicate, nested, Some(graph));
    builder.push_reifier_in_graph(reifier, nested, Some(graph));
    builder.push_annotation_in_graph(reifier, predicate, literal, Some(graph));
    let source = builder.freeze().unwrap();
    let restored = dataset_from_view(&source).unwrap();
    assert_eq!(
        source.owned_quads().collect::<Vec<_>>(),
        restored.owned_quads().collect::<Vec<_>>()
    );
    assert_eq!(
        source.owned_reifiers().collect::<Vec<_>>(),
        restored.owned_reifiers().collect::<Vec<_>>()
    );
    assert_eq!(
        source.owned_annotations().collect::<Vec<_>>(),
        restored.owned_annotations().collect::<Vec<_>>()
    );
    let mut graphs: Vec<_> = restored
        .named_graphs()
        .map(|id| restored.term_value(id))
        .collect();
    graphs.sort();
    assert_eq!(
        graphs,
        vec![
            TermValue::iri("http://example.org/empty"),
            TermValue::iri("http://example.org/graph")
        ]
    );
}
