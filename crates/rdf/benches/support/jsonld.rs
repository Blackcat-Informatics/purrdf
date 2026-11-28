// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic RDF 1.2 fixtures shared by JSON-LD timing and allocation probes.

use std::sync::Arc;

use purrdf_rdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};

pub(crate) const SMALL_ROWS: usize = 128;
pub(crate) const LARGE_ROWS: usize = 2_000;
pub(crate) const STAR_STRIDE: usize = 8;

pub(crate) fn build_dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/predicate").unwrap();
    let name = builder.intern_iri("https://example.org/name").unwrap();
    let asserts = builder.intern_iri("https://example.org/asserts").unwrap();
    let confidence = builder
        .intern_iri("https://example.org/confidence")
        .unwrap();
    let graph = builder.intern_iri("https://example.org/graph").unwrap();

    for index in 0..rows {
        let subject = builder
            .intern_iri(&format!("https://example.org/subject/{index}"))
            .unwrap();
        let object = builder
            .intern_iri(&format!("https://example.org/object/{index}"))
            .unwrap();
        let literal = builder
            .intern_literal(RdfLiteral::language_tagged(
                format!("deterministic value {index}"),
                if index % 2 == 0 { "en" } else { "fr" },
            ))
            .unwrap();
        let graph_name = (index % 4 == 0).then_some(graph);
        builder
            .push_quad(subject, predicate, object, graph_name)
            .unwrap();
        builder
            .push_quad(subject, name, literal, graph_name)
            .unwrap();

        if index % STAR_STRIDE == 0 {
            let proposition = builder.intern_triple(subject, predicate, object).unwrap();
            let reifier = builder
                .intern_iri(&format!("https://example.org/reifier/{index}"))
                .unwrap();
            let score = builder
                .intern_literal(RdfLiteral::typed(
                    format!("{}.5", index % 10),
                    "http://www.w3.org/2001/XMLSchema#decimal",
                ))
                .unwrap();
            builder
                .push_reifier_in_graph(reifier, proposition, graph_name)
                .unwrap();
            builder
                .push_annotation_in_graph(reifier, confidence, score, graph_name)
                .unwrap();
            builder
                .push_quad(subject, asserts, proposition, graph_name)
                .unwrap();
        }
    }

    builder.freeze().expect("deterministic JSON-LD fixture")
}

#[allow(
    dead_code,
    reason = "native_codecs uses this shared fixture; jsonld_alloc does not"
)]
pub(crate) fn build_many_namespace_dataset(
    namespace_count: usize,
    iris_per_namespace: usize,
) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("urn:subject").unwrap();
    let object = builder.intern_iri("urn:object").unwrap();
    for namespace in 0..namespace_count {
        for local in 0..iris_per_namespace {
            let predicate = builder
                .intern_iri(&format!("https://bench.example/ns/{namespace}/term{local}"))
                .unwrap();
            builder.push_quad(subject, predicate, object, None).unwrap();
        }
    }
    builder.freeze().expect("many-namespace JSON-LD fixture")
}

#[allow(
    dead_code,
    reason = "native_codecs uses these shared fixtures; jsonld_alloc does not"
)]
pub(crate) fn build_many_named_graph_dataset(graph_count: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder
        .intern_iri("https://bench.example/predicate")
        .unwrap();
    for index in 0..graph_count {
        let subject = builder
            .intern_iri(&format!("https://bench.example/subject/{index}"))
            .unwrap();
        let object = builder
            .intern_iri(&format!("https://bench.example/object/{index}"))
            .unwrap();
        let graph = builder
            .intern_iri(&format!("https://bench.example/graph/{index}"))
            .unwrap();
        builder
            .push_quad(subject, predicate, object, Some(graph))
            .unwrap();
    }
    builder.freeze().expect("many-graph JSON-LD fixture")
}

#[allow(
    dead_code,
    reason = "native_codecs uses these shared fixtures; jsonld_alloc does not"
)]
pub(crate) fn build_multivalue_dataset(value_count: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://bench.example/subject").unwrap();
    let predicate = builder.intern_iri("https://bench.example/value").unwrap();
    for index in 0..value_count {
        let object = builder
            .intern_iri(&format!("https://bench.example/object/{index:05}"))
            .unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
    }
    builder.freeze().expect("multivalue JSON-LD fixture")
}
