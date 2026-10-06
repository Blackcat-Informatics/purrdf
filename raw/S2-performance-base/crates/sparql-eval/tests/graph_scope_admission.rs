// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cardinality admission prices `GRAPH <iri> { ... }` over the graph the IRI names,
//! and over nothing when it names none.
//!
//! An IRI absent from the dictionary, or a term that names no graph, addresses an
//! empty scope: the block yields no row, so its estimate is zero. Pricing it as the
//! default graph refused a query that answers nothing. The neighbouring IRI that
//! names a real graph is still priced over that graph, and still refused when that
//! graph is too large for the ceiling.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryGovernors, QueryOptions};

const EX: &str = "http://example.org/";

/// 2 000 default-graph rows and the same in named graph `big`; `small` holds one.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{EX}p"));
    let big = b.intern_iri(&format!("{EX}big"));
    let small = b.intern_iri(&format!("{EX}small"));
    for i in 0..2_000 {
        let s = b.intern_iri(&format!("{EX}s{i}"));
        let o = b.intern_iri(&format!("{EX}o{i}"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(big));
    }
    let s = b.intern_iri(&format!("{EX}only"));
    b.push_quad(s, p, s, Some(small));
    b.freeze().expect("fixture freezes")
}

/// `Some(rows)` when `GRAPH <graph> { ?s ?p ?o . ?s ?p ?o2 }` is admitted under a
/// ten-cell ceiling and completes, `None` when it is refused or trips.
fn admitted_rows(dataset: &Arc<RdfDataset>, graph: &str) -> Option<usize> {
    let query = format!("SELECT * WHERE {{ GRAPH <{graph}> {{ ?s ?p ?o . ?s ?p ?o2 }} }}");
    let outcome = NativeSparqlEngine::new()
        .query_governed(
            dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(10),
        )
        .expect("the query returns an outcome");
    outcome
        .into_complete()
        .ok()
        .map(|result| result.into_solutions().map_or(0, |(_, rows)| rows.len()))
}

#[test]
fn a_graph_name_that_names_no_graph_is_priced_as_an_empty_scope() {
    let dataset = dataset();
    // Absent from the dictionary, and a term that names no graph: both empty scopes.
    for graph in [format!("{EX}never"), format!("{EX}s7")] {
        assert_eq!(admitted_rows(&dataset, &graph), Some(0), "<{graph}>");
    }
    // The neighbouring real graphs: one small enough to admit, one that is not.
    assert_eq!(admitted_rows(&dataset, &format!("{EX}small")), Some(1));
    assert_eq!(admitted_rows(&dataset, &format!("{EX}big")), None);
}
