// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **A GROUP BY under a SHACL host's governors is charged what it always was.**
//!
//! A host that sets a scratch ceiling, an intermediate-cell ceiling and a stop signal runs
//! the aggregate on its sequential, suspendable route. That route reads each argument and
//! copies each value without the admission frame a bounded account needs, and it must
//! still charge the same scratch bytes, complete at the same smallest ceiling and refuse
//! one byte below it, over the SHACL view exactly as over the plain dataset.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlRequest};
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_sparql_eval::{
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

const NS: &str = "http://example.org/";

/// Twelve subjects, each with one of five statuses.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let status = b.intern_iri(&format!("{NS}status")).unwrap();
    for i in 0..12_u64 {
        let s = b.intern_iri(&format!("{NS}s{i}")).unwrap();
        let st = b.intern_iri(&format!("{NS}st{}", i % 5)).unwrap();
        b.push_quad(s, status, st, None).unwrap();
    }
    let literal = b
        .intern_literal(RdfLiteral::typed(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
        ))
        .unwrap();
    let count = b.intern_iri(&format!("{NS}count")).unwrap();
    let s0 = b.intern_iri(&format!("{NS}s0")).unwrap();
    b.push_quad(s0, count, literal, None).unwrap();
    b.freeze().unwrap()
}

const GROUP_COUNT: &str = "PREFIX ex: <http://example.org/>\n\
    SELECT ?st (COUNT(?x) AS ?n) WHERE { ?x ex:status ?st } GROUP BY ?st";
const GROUP_VARIABLE: &str = "PREFIX ex: <http://example.org/>\n\
    SELECT ?x (COUNT(?v) AS ?n) ?v WHERE { ?v ex:status ?o } GROUP BY ?x ?v HAVING (?n > 0)";

fn governors(scratch: u64) -> QueryGovernors {
    QueryGovernors::UNBOUNDED
        .with_max_scratch_bytes(scratch)
        .with_max_intermediate_cells(1 << 40)
        .with_stop_signal(Arc::new(CancellationFlag::new()))
}

/// The scratch consumed by a complete run, with the rendered result, or `None` when a
/// governor stopped it.
fn run<D: purrdf_core::DatasetView + Sync>(
    dataset: &D,
    query: &str,
    scratch: u64,
) -> Option<(String, u64)> {
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    let outcome = engine
        .query_governed_view(dataset, request, &QueryOptions::EMPTY, &governors(scratch))
        .expect("the query parses and is admitted");
    match outcome {
        GovernedOutcome::Complete {
            result, evidence, ..
        } => Some((
            format!("{result:?}"),
            evidence.consumed_in(ResourceDimension::ScratchBytes),
        )),
        GovernedOutcome::BudgetExhausted(_) => None,
    }
}

/// The pinned scratch consumed by each query, measured before the aggregate fast path.
const SCRATCH: [u64; 2] = [784, 784];

#[test]
fn the_shacl_view_is_charged_the_plain_datasets_scratch_and_refuses_one_byte_below() {
    let source = dataset();
    let view = ShaclDatasetView::native(Arc::clone(&source));
    let consumed =
        [GROUP_COUNT, GROUP_VARIABLE].map(|query| run(&view, query, 1 << 40).expect("completes").1);
    assert_eq!(consumed, SCRATCH, "scratch consumed by each query");
    for (query, pinned) in [GROUP_COUNT, GROUP_VARIABLE].into_iter().zip(SCRATCH) {
        let (plain, plain_scratch) = run(&*source, query, 1 << 40).unwrap();
        let (viewed, viewed_scratch) = run(&view, query, 1 << 40).unwrap();
        assert_eq!(
            viewed_scratch, plain_scratch,
            "{query}: view and dataset charge alike"
        );
        assert_eq!(viewed_scratch, pinned, "{query}: scratch consumed");
        assert_eq!(
            run(&view, query, pinned).map(|(_, scratch)| scratch),
            Some(pinned),
            "{query}: completes at its own consumption"
        );
        assert!(
            run(&view, query, pinned - 1).is_none(),
            "{query}: refused one byte below"
        );
        assert!(!plain.is_empty() && !viewed.is_empty());
    }
}
