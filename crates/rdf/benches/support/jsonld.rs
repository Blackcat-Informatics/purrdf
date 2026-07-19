// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic RDF 1.2 fixtures shared by JSON-LD timing and allocation probes.

use std::sync::Arc;

use purrdf_rdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};

pub(crate) const SMALL_ROWS: usize = 128;
pub(crate) const LARGE_ROWS: usize = 2_000;
pub(crate) const STAR_STRIDE: usize = 8;

pub(crate) fn build_dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/predicate");
    let name = builder.intern_iri("https://example.org/name");
    let asserts = builder.intern_iri("https://example.org/asserts");
    let confidence = builder.intern_iri("https://example.org/confidence");
    let graph = builder.intern_iri("https://example.org/graph");

    for index in 0..rows {
        let subject = builder.intern_iri(&format!("https://example.org/subject/{index}"));
        let object = builder.intern_iri(&format!("https://example.org/object/{index}"));
        let literal = builder.intern_literal(RdfLiteral::language_tagged(
            format!("deterministic value {index}"),
            if index % 2 == 0 { "en" } else { "fr" },
        ));
        let graph_name = (index % 4 == 0).then_some(graph);
        builder.push_quad(subject, predicate, object, graph_name);
        builder.push_quad(subject, name, literal, graph_name);

        if index % STAR_STRIDE == 0 {
            let proposition = builder.intern_triple(subject, predicate, object);
            let reifier = builder.intern_iri(&format!("https://example.org/reifier/{index}"));
            let score = builder.intern_literal(RdfLiteral::typed(
                format!("{}.5", index % 10),
                "http://www.w3.org/2001/XMLSchema#decimal",
            ));
            builder.push_reifier_in_graph(reifier, proposition, graph_name);
            builder.push_annotation_in_graph(reifier, confidence, score, graph_name);
            builder.push_quad(subject, asserts, proposition, graph_name);
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
    let subject = builder.intern_iri("urn:subject");
    let object = builder.intern_iri("urn:object");
    for namespace in 0..namespace_count {
        for local in 0..iris_per_namespace {
            let predicate =
                builder.intern_iri(&format!("https://bench.example/ns/{namespace}/term{local}"));
            builder.push_quad(subject, predicate, object, None);
        }
    }
    builder.freeze().expect("many-namespace JSON-LD fixture")
}
