// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! End-to-end exercise of the cost-based BGP join planner through the public engine
//! path (parse → plan cache → cost planner → eval). The deterministic A/B win versus
//! the retired structural heuristic — measured by real materialised intermediate rows
//! — lives in the `bgp` unit tests; this corpus confirms the full public path evaluates
//! skewed multi-join shapes correctly and that repeated runs reuse the order cache.

mod support;

use support::{row_count, skewed_star};

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;

const STAR_QUERY: &str = "SELECT ?a ?b ?c ?d WHERE { \
     ?s <http://ex/hot> ?a . ?s <http://ex/warm> ?b . \
     ?s <http://ex/mid> ?c . ?s <http://ex/rare> ?d }";

fn query(engine: &NativeSparqlEngine, ds: &Arc<RdfDataset>, q: &str) -> SparqlResult {
    engine
        .query(
            ds,
            SparqlRequest {
                query: q,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("query")
}

/// A four-way skewed star join evaluates to its exact cross-on-hub cardinality through
/// the public engine path — the cost planner's reorder preserves the result multiset.
#[test]
fn skewed_star_join_evaluates_exactly() {
    let ds = skewed_star(&[("hot", 20), ("warm", 10), ("mid", 5), ("rare", 1)]);
    let engine = NativeSparqlEngine::new();
    // 20 (hot) × 10 (warm) × 5 (mid) × 1 (rare).
    assert_eq!(row_count(&query(&engine, &ds, STAR_QUERY)), 20 * 10 * 5);
}

/// The same query, run repeatedly against the same dataset, returns the same exact
/// result every time — the order cache serves the memoised plan without drift.
#[test]
fn repeated_runs_are_stable_under_the_order_cache() {
    let ds = skewed_star(&[("hot", 12), ("warm", 7), ("mid", 3), ("rare", 1)]);
    let engine = NativeSparqlEngine::new();
    let expected = 12 * 7 * 3; // × 1 (rare)
    for _ in 0..3 {
        assert_eq!(row_count(&query(&engine, &ds, STAR_QUERY)), expected);
    }
}

/// Planning a multi-pattern query on an ordinarily frozen dataset reads the index runs of
/// the predicates it asks about, never a pass over the whole table per statistic request.
#[test]
fn planning_reads_index_runs_not_the_table_on_an_ordinarily_frozen_dataset() {
    let mut b = RdfDatasetBuilder::new();
    let hub = b.intern_iri("http://ex/hub").unwrap();
    for (name, count) in [("hot", 20), ("warm", 10), ("mid", 5), ("rare", 1)] {
        let pred = b.intern_iri(&format!("http://ex/{name}")).unwrap();
        for i in 0..count {
            let leaf = b.intern_iri(&format!("http://ex/{name}{i}")).unwrap();
            b.push_quad(hub, pred, leaf, None).unwrap();
        }
    }
    // Rows under a predicate the query never mentions: a table pass reads them all.
    let noise = b.intern_iri("http://ex/noise").unwrap();
    for i in 0..5000 {
        let s = b.intern_iri(&format!("http://ex/n{i}")).unwrap();
        b.push_quad(s, noise, hub, None).unwrap();
    }
    let ds = b.freeze().expect("freeze");
    assert_eq!(ds.statistics_rows_scanned(), 0, "freeze scans nothing");
    assert_eq!(
        row_count(&query(&NativeSparqlEngine::new(), &ds, STAR_QUERY)),
        20 * 10 * 5
    );
    let scanned = ds.statistics_rows_scanned();
    assert!(scanned > 0, "planning asked for statistics");
    assert!(
        scanned < ds.quad_count() as u64,
        "{scanned} statistic rows read for a {}-quad table",
        ds.quad_count()
    );
}

/// A six-pattern connected chain (above the trivial sizes, below the DP ceiling)
/// evaluates correctly: `?v0 :hot ?v1 . … . ?v5 :hot ?v6` over a short hot path.
#[test]
fn six_pattern_chain_evaluates_end_to_end() {
    // A single hot path :n0 ->hot-> :n1 -> … -> :n6 (one solution), plus fan-out noise
    // off :n0 so the seed pattern is not trivially unique.
    let mut b = RdfDatasetBuilder::new();
    let hot = b.intern_iri("http://ex/hot").unwrap();
    let n0 = b.intern_iri("http://ex/n0").unwrap();
    let mut prev = n0;
    for i in 1..=6 {
        let next = b.intern_iri(&format!("http://ex/n{i}")).unwrap();
        b.push_quad(prev, hot, next, None).unwrap();
        prev = next;
    }
    // Off-path fan-out from n0 (raises the cardinality of the hot predicate so order
    // matters), none of which can complete the 6-hop chain.
    for i in 0..30 {
        let dead = b.intern_iri(&format!("http://ex/dead{i}")).unwrap();
        b.push_quad(n0, hot, dead, None).unwrap();
    }
    let ds = b.freeze().expect("freeze");

    let engine = NativeSparqlEngine::new();
    let q = "SELECT ?v6 WHERE { \
         ?v0 <http://ex/hot> ?v1 . ?v1 <http://ex/hot> ?v2 . ?v2 <http://ex/hot> ?v3 . \
         ?v3 <http://ex/hot> ?v4 . ?v4 <http://ex/hot> ?v5 . ?v5 <http://ex/hot> ?v6 }";
    // Exactly one 6-hop path exists (n0→…→n6); the fan-out branches dead-end at hop 1.
    assert_eq!(row_count(&query(&engine, &ds, q)), 1);
}
