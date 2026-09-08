// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic work counters, separate from uninstrumented timing benchmarks.
//!
//! Allocator counters record process requests, not logical intermediate sizes or
//! RSS. The incremental peak is additional live requested storage above the phase
//! baseline; governor peak cells remain a separate, typed execution measurement.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{RdfDatasetBuilder, ResourceDimension, SparqlEngine, SparqlRequest};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{CacheLimits, NativeSparqlEngine, QueryGovernors, QueryOptions};

const QUERY: &str = "SELECT ?s ?value WHERE { ?s <http://example.org/p> ?value } ORDER BY ?s";
const REPETITIONS: usize = 100;
static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
struct CountingAllocator;

fn allocated(bytes: usize) {
    REQUESTS.fetch_add(1, Ordering::Relaxed);
    REQUESTED_BYTES.fetch_add(bytes, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
}

// SAFETY: Every allocation and deallocation forwards its pointer and original
// layout unchanged to System; counters neither inspect nor alter the storage.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            allocated(size);
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure(name: &str, engine: &NativeSparqlEngine, operation: impl FnOnce()) {
    let cache_before = engine.plan_cache_stats();
    let initial_live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(initial_live, Ordering::Relaxed);
    let initial_requests = REQUESTS.load(Ordering::Relaxed);
    let initial_bytes = REQUESTED_BYTES.load(Ordering::Relaxed);
    operation();
    let requests = REQUESTS.load(Ordering::Relaxed) - initial_requests;
    let bytes = REQUESTED_BYTES.load(Ordering::Relaxed) - initial_bytes;
    let peak = PEAK_BYTES
        .load(Ordering::Relaxed)
        .saturating_sub(initial_live);
    let cache = engine.plan_cache_stats();
    let plans = engine.plan_memory_observer().stats();
    println!(
        "{name},{REPETITIONS},{requests},{bytes},{peak},{},{},{},{},{},{},{},{}",
        cache.hits - cache_before.hits,
        cache.misses - cache_before.misses,
        cache.evictions - cache_before.evictions,
        cache.bytes,
        plans.retained_plans,
        plans.retained_bytes,
        plans.detached_plans,
        plans.detached_bytes
    );
}

fn main() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let value = builder.intern_iri("http://example.org/value");
    for index in 0..64 {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        builder.push_quad(subject, predicate, value, None);
    }
    let data = builder.freeze().unwrap();
    println!(
        "phase,repetitions,allocation_requests,requested_bytes,incremental_peak_requested_bytes,cache_hits,cache_misses,cache_evictions,cache_bytes,retained_plans,retained_plan_bytes,detached_plans,detached_plan_bytes"
    );
    let cold = NativeSparqlEngine::new().with_plan_cache_limits(CacheLimits {
        entries: 0,
        bytes: 0,
    });
    measure("prepare_no_retention", &cold, || {
        for _ in 0..REPETITIONS {
            black_box(cold.prepare_query(QUERY, None).unwrap());
        }
    });
    let warm = NativeSparqlEngine::new();
    let prepared = warm.prepare_query(QUERY, None).unwrap();
    measure("prepare_cached", &warm, || {
        for _ in 0..REPETITIONS {
            black_box(warm.prepare_query(QUERY, None).unwrap());
        }
    });
    measure("validate_algebra", &warm, || {
        for _ in 0..REPETITIONS {
            black_box(&prepared.query).validate().unwrap();
        }
    });
    measure("prepare_compiler_algebra_including_clone", &cold, || {
        for _ in 0..REPETITIONS {
            black_box(
                cold.prepare_algebra(prepared.query.clone(), QueryOptions::EMPTY)
                    .unwrap(),
            );
        }
    });
    // Warm the independent order cache before comparing repeated execution.
    warm.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    measure("execute_prepared", &warm, || {
        for _ in 0..REPETITIONS {
            black_box(
                warm.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                    .unwrap(),
            );
        }
    });
    measure("execute_cached_text", &warm, || {
        for _ in 0..REPETITIONS {
            black_box(
                warm.query(
                    &data,
                    SparqlRequest {
                        query: QUERY,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .unwrap(),
            );
        }
    });
    cold.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    measure("execute_uncached_text", &cold, || {
        for _ in 0..REPETITIONS {
            black_box(
                cold.query(
                    &data,
                    SparqlRequest {
                        query: QUERY,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .unwrap(),
            );
        }
    });
    let churn = NativeSparqlEngine::new().with_plan_cache_limits(CacheLimits {
        entries: 4,
        bytes: 4096,
    });
    let queries: Vec<_> = (0..REPETITIONS)
        .map(|index| format!("ASK {{ VALUES ?x {{ <http://example.org/{index}> }} }}"))
        .collect();
    let mut pinned = Vec::with_capacity(REPETITIONS);
    measure("bounded_retention_churn", &churn, || {
        for query in &queries {
            pinned.push(churn.prepare_query(query, None).unwrap());
        }
    });
    assert!(churn.plan_cache_stats().bytes <= 4096);
    assert!(churn.plan_cache_stats().entries <= 4);
    assert_eq!(churn.plan_memory_observer().stats().live_plans, REPETITIONS);
    drop(pinned);
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    measure("execute_shared_governor", &warm, || {
        for _ in 0..REPETITIONS {
            black_box(
                warm.query_prepared_governed_in_operation(
                    &*data,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &state,
                )
                .unwrap(),
            );
        }
    });
    let evidence = state.evidence();
    println!(
        "governors,fuel={},answer_rows={},peak_intermediate_cells={},complete={}",
        evidence.consumed_in(ResourceDimension::Fuel),
        evidence.consumed_in(ResourceDimension::AnswerRows),
        evidence.consumed_in(ResourceDimension::IntermediateCells),
        evidence.is_complete()
    );

    let owner = warm.plan_memory_observer();
    drop(warm);
    let pinned = owner.stats();
    println!(
        "after_cache_drop,retained_plans={},detached_plans={},detached_bytes={}",
        pinned.retained_plans, pinned.detached_plans, pinned.detached_bytes
    );
    drop(prepared);
    assert_eq!(
        owner.stats(),
        purrdf_sparql_eval::PlanMemoryStats::default()
    );
    println!("after_plan_drop,live_plans=0,live_bytes=0");
}
