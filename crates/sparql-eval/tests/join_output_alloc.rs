// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! EXPLAIN allocation traffic for sparse joins across projection boundaries.
//! Subqueries keep the materializing join in use even when positive graph
//! patterns elsewhere are evaluated with incoming bindings.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_sparql_eval::NativeSparqlEngine;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROWS: usize = 4096;

fn sparse_graph() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let left = builder.intern_iri("http://example.org/left");
    let right = builder.intern_iri("http://example.org/right");
    let value = builder.intern_iri("http://example.org/value");
    for key in 0..ROWS * 2 {
        let subject = builder.intern_iri(&format!("http://example.org/key{key}"));
        builder.push_quad(subject, if key < ROWS { right } else { left }, value, None);
        if key == 0 {
            builder.push_quad(subject, left, value, None);
        }
    }
    builder.freeze().expect("freeze sparse graph")
}

#[test]
fn explaining_sparse_materializing_joins_allocates_for_real_results() {
    let dataset = sparse_graph();
    let engine = NativeSparqlEngine::new();
    for (query, expected_rows) in [
        (
            "SELECT ?key WHERE { \
             { SELECT ?key WHERE { ?key <http://example.org/left> ?left } } \
             { SELECT ?key WHERE { ?key <http://example.org/right> ?right } } }",
            1,
        ),
        (
            "SELECT ?key ?right WHERE { \
             { SELECT ?key WHERE { ?key <http://example.org/left> ?left } } \
             OPTIONAL { \
               SELECT ?key ?right WHERE { ?key <http://example.org/right> ?right } \
             } }",
            ROWS + 1,
        ),
    ] {
        let warm = engine
            .explain_query(&dataset, query, None)
            .expect("warm explain");
        // METERED engages the global cell sink, so this execution is sequential:
        // the per-thread allocator window includes all measured join allocations.
        let window = CurrentThreadWindow::open();
        let explanation = engine
            .explain_query(&dataset, query, None)
            .expect("explain");
        let measured = window.close();
        assert_eq!(
            explanation.ledger()[0].rows,
            u64::try_from(expected_rows).expect("rows")
        );
        assert_eq!(explanation.evidence().tripped, None);
        assert_eq!(explanation.render(), warm.render());
        // The hypothetical product contains over 16 million rows (>640 MB just
        // for row slots). Actual allocation traffic remains below 16 MB including
        // both operand bags, the index, output rows and the complete explanation.
        assert!(measured.requested_bytes < 16 * 1024 * 1024, "{measured:?}");
        assert!(
            measured.peak_working_bytes < 8 * 1024 * 1024,
            "{measured:?}"
        );
    }
}
