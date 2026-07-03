// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: `compiles_generated_query_set_once`
//!
//! **Requirement / S6 — Requirement 11 evidence.**
//!
//! The requirement states: "Plan cache / DSL→native-plan path so the static
//! generated query corpus compiles once."
//!
//! This test proves the [`PlanCache`] mechanism over the real generated query
//! corpus (`generated/queries/*.rq`):
//!
//! 1. It enumerates *all* files in that directory (sorted, floor ≥ 50 so the
//!    set cannot silently shrink undetected).
//! 2. It runs a **first pass** through the single cache instance, collecting one
//!    `Arc<PreparedQuery>` per file.  A parse failure is a hard test failure
//!    naming the offending file.
//! 3. It runs a **second pass** through the same cache and asserts
//!    `Arc::ptr_eq(&first[i], &second[i])` — i.e. every query is returned from
//!    the cache without re-parsing ("compiles once").
//!
//! Wiring the cache into the production consumers of the generated query corpus
//! (crates/slice, crates/pipeline mappings) lands with the later cutover; this test
//! proves the cache mechanism itself over the real generated set.

use std::fs;
use std::sync::Arc;

use purrdf_sparql_eval::{PlanCache, PreparedQuery};

/// Enumerates `generated/queries/*.rq`, compiles each query once, and verifies
/// that a second `prepare()` call returns the identical `Arc` (cache hit, no
/// re-parse).
#[test]
fn compiles_generated_query_set_once() {
    let query_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../generated/queries");

    // --- enumerate and sort ---
    let mut entries: Vec<std::path::PathBuf> = fs::read_dir(query_dir)
        .unwrap_or_else(|e| panic!("cannot open generated/queries dir ({query_dir}): {e}"))
        .map(|res| res.expect("read_dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("rq"))
        .collect();
    entries.sort();

    let count = entries.len();
    assert!(
        count >= 50,
        "generated/queries/*.rq count ({count}) is below the expected floor of 50; \
         the corpus may have silently shrunk"
    );

    let mut cache = PlanCache::new();

    // --- first pass: parse every query (must all succeed) ---
    let first_pass: Vec<Arc<PreparedQuery>> = entries
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            cache.prepare(&text, None).unwrap_or_else(|e| {
                panic!(
                    "parse failure in generated query {} — code={} msg={}",
                    path.display(),
                    e.code,
                    e.message
                )
            })
        })
        .collect();

    // --- second pass: every prepare() must return a cache hit (same Arc) ---
    for (i, path) in entries.iter().enumerate() {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot re-read {}: {e}", path.display()));
        let second = cache.prepare(&text, None).unwrap_or_else(|e| {
            panic!(
                "unexpected error on second prepare of {} — code={} msg={}",
                path.display(),
                e.code,
                e.message
            )
        });
        assert!(
            Arc::ptr_eq(&first_pass[i], &second),
            "cache MISS on second prepare of {} (index {i}): \
             the query was re-parsed instead of returned from the cache",
            path.display()
        );
    }

    // Emit the corpus size so it appears in `cargo test -- --nocapture` output.
    println!("plan_cache_corpus: verified {count} generated queries compile once");
}
