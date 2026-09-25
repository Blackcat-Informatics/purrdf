// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A rayon worker checks its own float environment: the calling thread's check does not
//! vouch for the workers a batch fans out to.
//!
//! The float environment is per-thread control state, so a batch search whose calling
//! thread is clean may still run its queries on workers that flush subnormals to zero.
//! `HnswIndex::search_batch` and the parallel proposal phase of a rebuild therefore
//! resolve the index's stored path inside each worker, once per worker's share of the
//! work, rather than carrying the calling thread's handle to them (which the compiler
//! refuses: `Resolved` is neither `Send` nor `Sync`).
//!
//! This binary makes rayon's **global** pool flush-to-zero, from a start handler that sets
//! the MXCSR's FTZ bit on every worker as it starts, and holds a second, clean pool
//! beside it. A parallel iterator driven from a thread outside any pool runs every job on
//! the global pool's workers, so a call from the (clean) test thread passes its own check
//! and then fans out onto flushing workers; the same call installed on the clean pool
//! fans out onto clean ones. The one difference between the treatment and the control is
//! the environment of the workers, so the refusal can only have come from the workers'
//! own resolve. The global pool is process-wide, which is why this is its own test
//! binary with a single test.

#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{FloatEnvironmentError, FloatEnvironmentEvidence};
use purrdf_hnsw::{HnswError, HnswIndex, Params, Ranked, VectorMatrix, level::splitmix64};

/// MXCSR flush-to-zero.
const FTZ: u32 = 1 << 15;

fn read_mxcsr() -> u32 {
    let mut value: u32 = 0;
    // SAFETY: `stmxcsr` stores the 32-bit MXCSR to a live, aligned, writable `u32`.
    unsafe {
        core::arch::asm!(
            "stmxcsr [{ptr}]",
            ptr = in(reg) &raw mut value,
            options(nostack, preserves_flags),
        );
    }
    value
}

/// Set FTZ on the calling thread for the rest of its life: called only from the global
/// pool's start handler, on threads this binary's pool owns and no other test shares.
fn flush_this_thread() {
    let value = read_mxcsr() | FTZ;
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`, the register as read with
    // FTZ added, which is a valid MXCSR value.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

/// A deterministic fixture matrix. Nothing here reads a clock or an RNG.
fn matrix(rows: usize, dims: usize) -> VectorMatrix {
    let mut state = 0xF7A3_0000_5EED_0001_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        data.push(unit.mul_add(2.0, -1.0));
    }
    VectorMatrix::new(rows, dims, data).expect("the fixture matrix is valid")
}

/// Whether `error` is the flush-to-zero refusal this target reports: the MXCSR by name
/// on `x86_64`, where it is read, and the probe's flushed-result row on 32-bit x86.
fn is_ftz(error: &HnswError) -> bool {
    let HnswError::FloatEnvironment(FloatEnvironmentError::FlushToZero { evidence }) = error else {
        return false;
    };
    if cfg!(target_arch = "x86_64") {
        matches!(
            evidence,
            FloatEnvironmentEvidence::Register { name: "MXCSR", .. }
        )
    } else {
        matches!(
            evidence,
            FloatEnvironmentEvidence::Probe {
                operation: "f64::MIN_POSITIVE * 0.5",
                observed: 0,
                ..
            }
        )
    }
}

/// A batch answer as its rows and distance bits, so equality is bit-identity.
fn bits(batch: &[Vec<Ranked>]) -> Vec<Vec<(usize, u64)>> {
    batch
        .iter()
        .map(|ranked| {
            ranked
                .iter()
                .map(|scored| (scored.row, scored.distance.to_bits()))
                .collect()
        })
        .collect()
}

#[test]
fn a_flushing_rayon_worker_refuses_its_share_and_a_clean_one_answers_the_single_thread_bits() {
    // The global pool flushes; nothing in this binary has touched rayon before, so this is
    // the pool every parallel iterator driven from outside a pool runs on.
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .start_handler(|_| flush_this_thread())
        .build_global()
        .expect("the first rayon use in this binary configures the global pool");
    let clean = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("a clean pool");

    // Everything is built on the clean pool, whose workers run the build's proposals.
    let data = matrix(48, 70);
    let metric = DistanceMetric::SquaredEuclidean;
    let (exact, fast) = clean.install(|| {
        (
            HnswIndex::build(data.clone(), &metric, params()).expect("builds on clean workers"),
            HnswIndex::build_reassociated(data.clone(), &metric, params())
                .expect("builds on clean workers"),
        )
    });
    let queries: Vec<usize> = (0..data.rows()).collect();

    // The single-thread oracle: one query at a time on the test thread, which is clean and
    // touches no pool.
    let single_exact: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| exact.search_rows(row, 4).expect("the test thread is clean"))
        .collect();
    let single_fast: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| fast.search_rows(row, 4).expect("the test thread is clean"))
        .collect();

    // The treatment: called from the clean test thread, which passes its own check, and
    // run on the flushing global pool's workers, each of which resolves for itself.
    assert!(
        is_ftz(
            &exact
                .search_batch(&queries, 4)
                .expect_err("the flushing workers refuse their queries")
        ),
        "exact search_batch on flushing workers"
    );
    assert!(
        is_ftz(
            &fast
                .search_batch(&queries, 4)
                .expect_err("the flushing workers refuse their queries")
        ),
        "reassociated search_batch on flushing workers"
    );
    assert!(
        is_ftz(
            &exact
                .verify_rebuild()
                .expect_err("the flushing workers refuse their proposals")
        ),
        "exact rebuild proposals on flushing workers"
    );
    assert!(
        is_ftz(
            &fast
                .verify_rebuild()
                .expect_err("the flushing workers refuse their proposals")
        ),
        "reassociated rebuild proposals on flushing workers"
    );

    // The valid neighbour: the same calls, installed on the clean pool, answer — and
    // answer the single-thread bits, so nothing about the batch path other than its
    // workers' environment decided the refusal above.
    let (batch_exact, batch_fast, rebuilt_exact, rebuilt_fast) = clean.install(|| {
        (
            exact.search_batch(&queries, 4),
            fast.search_batch(&queries, 4),
            exact.verify_rebuild(),
            fast.verify_rebuild(),
        )
    });
    assert_eq!(
        bits(&batch_exact.expect("clean workers answer")),
        bits(&single_exact),
        "exact batch on clean workers"
    );
    assert_eq!(
        bits(&batch_fast.expect("clean workers answer")),
        bits(&single_fast),
        "reassociated batch on clean workers"
    );
    assert!(rebuilt_exact.expect("clean workers rebuild"));
    assert!(rebuilt_fast.expect("clean workers rebuild"));
}
