// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Retention, typed preparation and operation-budget contracts for public callers.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, ResourceDimension, SparqlResult, TermValue};
use purrdf_sparql_algebra::{GraphPattern, PropertyFunctionCall, Query, QueryDataset};
use purrdf_sparql_eval::governor::{GovernorState, QueryGovernors};
use purrdf_sparql_eval::{
    CacheLimits, GovernedOutcome, NativeSparqlEngine, PlanCache, QueryOptions,
};

const A: &str = "SELECT ?s WHERE { ?s <http://example.org/p> ?o } ORDER BY ?s";
const B: &str = "ASK { ?s <http://example.org/p> ?o }";
const C: &str = "SELECT ?o WHERE { ?s <http://example.org/p> ?o }";

fn fixture() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/value");
    for name in ["a", "b", "c"] {
        let subject = builder.intern_iri(&format!("http://example.org/{name}"));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().unwrap()
}

#[test]
fn entry_ceiling_evicts_least_recently_used_but_retained_handles_still_execute() {
    let mut cache = PlanCache::with_limits(CacheLimits {
        entries: 2,
        bytes: 1_000_000,
    });
    let a = cache.prepare(A, None).unwrap();
    let b = cache.prepare(B, None).unwrap();
    assert!(Arc::ptr_eq(&a, &cache.prepare(A, None).unwrap()));
    cache.prepare(C, None).unwrap();
    assert!(Arc::ptr_eq(&a, &cache.prepare(A, None).unwrap()));
    assert!(!Arc::ptr_eq(&b, &cache.prepare(B, None).unwrap()));
    assert_eq!(cache.stats().evictions, 2);
    let result = NativeSparqlEngine::new()
        .query_prepared(&fixture(), &b, &[], QueryOptions::EMPTY)
        .unwrap();
    assert!(matches!(result, SparqlResult::Boolean(true)));
}

#[test]
fn byte_ceiling_and_oversize_admission_bound_storage_without_poisoning_useful_plans() {
    let mut measured = PlanCache::new();
    measured.prepare(A, None).unwrap();
    let bytes = measured.stats().bytes;
    let mut cache = PlanCache::with_limits(CacheLimits { entries: 10, bytes });
    let a = cache.prepare(A, None).unwrap();
    let huge = format!(
        "SELECT ?x WHERE {{ VALUES ?x {{ \"{}\" }} }}",
        "x".repeat(bytes)
    );
    let large = cache.prepare(&huge, None).unwrap();
    assert_eq!(cache.stats().bytes, bytes);
    assert_eq!(cache.stats().unretained, 1);
    assert_eq!(cache.stats().evictions, 0);
    assert!(Arc::ptr_eq(&a, &cache.prepare(A, None).unwrap()));
    let result = NativeSparqlEngine::new()
        .query_prepared(&fixture(), &large, &[], QueryOptions::EMPTY)
        .unwrap();
    assert!(matches!(result, SparqlResult::Solutions { rows, .. } if rows.len() == 1));
    cache.prepare(B, None).unwrap();
    assert!(cache.stats().bytes <= bytes);
    assert_eq!(cache.stats().evictions, 1);
}

#[test]
fn disabled_retention_still_prepares_and_errors_are_never_cached() {
    for limits in [
        CacheLimits {
            entries: 0,
            bytes: 100_000,
        },
        CacheLimits {
            entries: 100,
            bytes: 0,
        },
    ] {
        let mut cache = PlanCache::with_limits(limits);
        let first = cache.prepare(A, None).unwrap();
        let second = cache.prepare(A, None).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(cache.prepare("SELECT WHERE malformed", None).is_err());
        assert_eq!(cache.stats().unretained, 2);
        assert_eq!(cache.stats().misses, 3);
        assert_eq!(cache.stats().bytes, 0);
        assert!(cache.is_empty());
    }
}

#[test]
fn typed_preparation_admits_registry_calls_before_execution() {
    let query = Query::Ask {
        pattern: GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: "http://example.org/unregistered".into(),
            subject_args: vec![],
            object_args: vec![],
        }),
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let engine = NativeSparqlEngine::new();
    assert!(engine.prepare_algebra(query, QueryOptions::EMPTY).is_err());
    assert_eq!(engine.plan_cache_stats().misses, 0);
}

#[test]
fn prepared_operation_preserves_substitutions_without_text_cache_access() {
    let engine = NativeSparqlEngine::new();
    let query = purrdf_sparql_algebra::SparqlParser::new()
        .parse_query(A)
        .unwrap();
    let prepared = engine.prepare_algebra(query, QueryOptions::EMPTY).unwrap();
    let dataset = fixture();
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    for name in ["a", "b"] {
        let substitutions = [(
            "s".into(),
            TermValue::Iri(format!("http://example.org/{name}")),
        )];
        let result = engine
            .query_prepared_governed_in_operation(
                &*dataset,
                &prepared,
                &substitutions,
                QueryOptions::EMPTY,
                &state,
            )
            .unwrap();
        let GovernedOutcome::Complete { result, .. } = result else {
            panic!("expected complete");
        };
        assert!(matches!(result, SparqlResult::Solutions { rows, .. } if rows.len() == 1));
    }
    assert_eq!(engine.plan_cache_stats().misses, 0);
}

#[test]
fn prepared_workers_charge_one_shared_budget_and_do_not_multiply_it() {
    let dataset = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(A, None).unwrap();
    let metered = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let first = engine
        .query_prepared_governed_in_operation(
            &*dataset,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &metered,
        )
        .unwrap();
    assert!(matches!(first, GovernedOutcome::Complete { .. }));
    let cost = metered.evidence().consumed.get(ResourceDimension::Fuel);
    assert!(cost > 0);
    let shared = Arc::new(GovernorState::new(&QueryGovernors::METERED.with_fuel(cost)));
    for expected_complete in [true, false] {
        let result = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    NativeSparqlEngine::new().query_prepared_governed_in_operation(
                        &*dataset,
                        &prepared,
                        &[],
                        QueryOptions::EMPTY,
                        &shared,
                    )
                })
                .join()
                .unwrap()
                .unwrap()
        });
        assert_eq!(
            matches!(result, GovernedOutcome::Complete { .. }),
            expected_complete
        );
    }
    assert!(shared.evidence().tripped.is_some());
}

#[test]
fn join_order_eviction_replans_without_reusing_answers() {
    let engine = NativeSparqlEngine::new().with_order_cache_limits(CacheLimits {
        entries: 1,
        bytes: 1000,
    });
    let dataset = fixture();
    let a = engine.prepare_query(A, None).unwrap();
    let other = engine
        .prepare_query(
            "SELECT ?o WHERE { <http://example.org/a> <http://example.org/p> ?o }",
            None,
        )
        .unwrap();
    for (plan, expected_rows) in [(&a, 3), (&other, 1), (&a, 3)] {
        let result = engine
            .query_prepared(&dataset, plan, &[], QueryOptions::EMPTY)
            .unwrap();
        assert!(
            matches!(result, SparqlResult::Solutions { rows, .. } if rows.len() == expected_rows)
        );
        assert!(engine.order_cache_stats().bytes <= 1000);
        assert_eq!(engine.order_cache_stats().entries, 1);
    }
    assert_eq!(engine.order_cache_stats().evictions, 2);
}
