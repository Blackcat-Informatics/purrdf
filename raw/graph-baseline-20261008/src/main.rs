// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::sync::Arc;
use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest};
use purrdf_sparql_eval::NativeSparqlEngine;
use purrdf_sparql_eval::update::{GraphResolveRequest, GraphResolver, LoadError};

struct CachedDocument(Arc<RdfDataset>);

impl GraphResolver for CachedDocument {
    fn resolve(&self, _: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        Ok(Arc::clone(&self.0))
    }
}

fn apply(engine: &NativeSparqlEngine, dataset: &mut Arc<RdfDataset>, query: &str) {
    engine.update(dataset, SparqlRequest {
        query, base_iri: None, substitutions: &[],
    }).expect("production update must execute before its result is inspected");
}

fn main() {
    let mut document = RdfDatasetBuilder::new();
    let s = document.intern_blank("b0", BlankScope::DEFAULT);
    let p = document.intern_iri("https://example.org/p");
    let o = document.intern_iri("https://example.org/o");
    document.push_quad(s, p, o, None);
    let document = document.freeze().expect("valid blank document");
    let engine = NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(document)));
    let mut loaded = RdfDatasetBuilder::new().freeze().expect("empty destination");
    apply(&engine, &mut loaded,
        "LOAD <https://example.org/doc>; LOAD <https://example.org/doc>");
    println!("cached-document-twice expected_rows=2 actual_rows={} defect={}",
        loaded.rdf_row_count(), loaded.rdf_row_count() != 2);
    assert_eq!(loaded.rdf_row_count(), 1, "baseline witness changed; reassess instead of declaring a failure");

    for overlap in [false, true] {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        let g = b.intern_iri("https://example.org/source");
        if overlap { b.push_quad(s, p, o, Some(g)); }
        b.push_annotation_in_graph(s, p, o, Some(g));
        let mut dataset = b.freeze().expect("valid physical annotation source");
        let source_annotations = dataset.annotations().count();
        let source_ordinary = dataset.quads().count();
        let source_rows = dataset.rdf_row_count();
        assert_eq!(source_annotations, 1);
        assert_eq!(source_ordinary, usize::from(overlap));
        let engine = NativeSparqlEngine::new();
        apply(&engine, &mut dataset,
            "COPY GRAPH <https://example.org/source> TO GRAPH <https://example.org/destination>");
        let annotations = dataset.annotations().count();
        let ordinary = dataset.quads().count();
        let rows = dataset.rdf_row_count();
        println!("copy-physical-roles overlap={overlap} source_rows={source_rows} expected_rows={} actual_rows={rows} expected_annotations=2 actual_annotations={annotations} ordinary_rows={ordinary} defect={}",
            source_rows * 2, rows != source_rows * 2 || annotations != 2);
        assert_eq!(annotations, 1, "baseline role-loss witness changed; reassess");
        assert_eq!(rows, source_rows + 1, "baseline physical-collapse witness changed; reassess");
    }
}
