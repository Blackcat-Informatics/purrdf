// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same statement-query data and production paths on native and WASM.

mod support;

use purrdf_core::{
    DatasetView, GraphMatch, SparqlRequest, StatementIndexSupport, TermValue, TripleDataset,
    TripleReadError,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryGovernors, QueryOptions};
use std::sync::Arc;
use support::statement_claims::{QUERIES, candidate_quads, dataset};

fn native_and_compact_statement_queries_keep_answers_and_candidate_work() {
    // Public query entry points initialize NOW and entropy from the host. These
    // queries use neither value; this case verifies indexed answers and work.
    let data = dataset(128);
    let engine = NativeSparqlEngine::new();
    for (query, matches) in QUERIES {
        let measured = engine.explain_query(&data, query, None).unwrap();
        assert_eq!(candidate_quads(&measured), matches);
        assert_eq!(measured.statement_indexes(), StatementIndexSupport::INDEXED);
        let outcome = engine
            .query_governed(
                &data,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(128),
            )
            .unwrap();
        let result = outcome
            .into_complete()
            .expect("matching native range fits the fixed ceiling");
        assert_eq!(
            support::row_count(&result),
            usize::try_from(matches).unwrap()
        );
    }
    let compact = TripleDataset::try_from(Arc::try_unwrap(data).unwrap()).unwrap();
    let read = Arc::new(compact.read().unwrap());
    for (query, matches) in QUERIES {
        let (measured, ()) = engine
            .explain_query_fallible_view(&read, query, None)
            .unwrap();
        assert_eq!(candidate_quads(&measured), matches);
        assert_eq!(measured.statement_indexes(), StatementIndexSupport::INDEXED);
        let outcome = engine
            .query_governed_fallible_view(
                &read,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(128),
            )
            .unwrap();
        assert!(outcome.evidence.governors.is_complete());
        assert_eq!(
            support::row_count(&outcome.result),
            usize::try_from(matches).unwrap()
        );
    }
}

fn named_graph_refusal_precedes_index_allocation_and_keeps_the_first_cause() {
    purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        let compact = TripleDataset::try_from(Arc::try_unwrap(dataset(8)).unwrap()).unwrap();
        let focus = compact
            .term_id_by_value(&TermValue::iri("http://example.org/focus"))
            .unwrap();
        let refused = Arc::new(compact.read().unwrap());
        assert_eq!(
            refused
                .reifier_quads_for_components([Some(focus), None, None], GraphMatch::Named(focus))
                .count(),
            0
        );
        assert_eq!(
            refused.read_error(),
            Some(TripleReadError::NamedGraphsUnsupported)
        );
        assert_eq!(
            refused
                .annotation_quads_for_pattern(None, None, GraphMatch::Default)
                .count(),
            0
        );
        assert_eq!(compact.storage_layout().reifier_components, [0; 7]);
        assert_eq!(compact.storage_layout().annotation_orders, [0; 2]);
        let healthy = compact.read().unwrap();
        assert_eq!(
            healthy
                .reifier_quads_for_components([Some(focus), None, None], GraphMatch::Default)
                .count(),
            4
        );
        assert!(healthy.read_error().is_none());
        assert_eq!(
            compact.storage_layout().reifier_components,
            [8 * 4, 0, 0, 0, 0, 0, 0]
        );
    });
}

purrdf_testkit::harness_main!(
    native_and_compact_statement_queries_keep_answers_and_candidate_work,
    named_graph_refusal_precedes_index_allocation_and_keeps_the_first_cause,
);
