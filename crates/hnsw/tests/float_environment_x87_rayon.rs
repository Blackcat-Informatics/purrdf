// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! On the x87, a rayon worker checks its own control word: the calling thread's check
//! does not vouch for the workers a batch fans out to.
//!
//! The x87 counterpart of `float_environment_rayon`. This binary makes rayon's **global**
//! pool round toward zero, from a start handler that loads the rounding-control field of
//! every worker's x87 control word as it starts, and holds a second, clean pool beside
//! it. A parallel iterator driven from a thread outside any pool runs every job on the
//! global pool's workers, so a call from the (clean) test thread passes its own check and
//! then fans out onto directed workers; the same call installed on the clean pool fans
//! out onto clean ones. The one difference between the treatment and the control is the
//! workers' control word, so the refusal can only have come from the workers' own
//! resolve. The global pool is process-wide, which is why this is its own test binary
//! with a single test.

#![cfg(all(target_arch = "x86", not(target_feature = "sse2")))]

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{FloatEnvironmentError, FloatEnvironmentEvidence};
use purrdf_hnsw::{HnswError, HnswIndex, Params, Ranked, VectorMatrix, level::splitmix64};
use purrdf_xsd::ieee::x87;

/// Round toward zero.
const TOWARD_ZERO: u16 = 0b11 << 10;

/// Load round-toward-zero into the calling thread's control word for the rest of its
/// life: called only from the global pool's start handler, on threads this binary's pool
/// owns and no other test shares.
fn round_this_thread_toward_zero() {
    let word = (x87::control_word() & !x87::ROUNDING_CONTROL) | TOWARD_ZERO;
    // SAFETY: only the rounding-control field differs from this thread's word, so every
    // exception stays masked; the thread is a pool worker whose control word nothing else
    // relies on.
    unsafe { x87::load_control_word(word) };
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

/// Whether `error` is the rounding refusal naming a worker's x87 control word with the
/// toward-zero field loaded.
fn is_toward_zero_refusal(error: &HnswError) -> bool {
    matches!(
        error,
        HnswError::FloatEnvironment(FloatEnvironmentError::RoundingMode {
            evidence: FloatEnvironmentEvidence::Register {
                name: "x87 control word",
                bits,
            },
        }) if *bits & u64::from(x87::ROUNDING_CONTROL) == u64::from(TOWARD_ZERO)
    )
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
fn a_directed_rayon_worker_refuses_its_share_and_a_clean_one_answers_the_single_thread_bits() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .start_handler(|_| round_this_thread_toward_zero())
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
    assert_eq!(x87::control_word() & x87::ROUNDING_CONTROL, 0);
    let single_exact: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| exact.search_rows(row, 4).expect("the test thread is clean"))
        .collect();
    let single_fast: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| fast.search_rows(row, 4).expect("the test thread is clean"))
        .collect();

    // The treatment: called from the clean test thread, which passes its own check, and
    // run on the directed global pool's workers, each of which resolves for itself.
    assert!(
        is_toward_zero_refusal(
            &exact
                .search_batch(&queries, 4)
                .expect_err("the directed workers refuse their queries")
        ),
        "exact search_batch on directed workers"
    );
    assert!(
        is_toward_zero_refusal(
            &fast
                .search_batch(&queries, 4)
                .expect_err("the directed workers refuse their queries")
        ),
        "reassociated search_batch on directed workers"
    );
    assert!(
        is_toward_zero_refusal(
            &exact
                .verify_rebuild()
                .expect_err("the directed workers refuse their proposals")
        ),
        "exact rebuild proposals on directed workers"
    );
    assert!(
        is_toward_zero_refusal(
            &fast
                .verify_rebuild()
                .expect_err("the directed workers refuse their proposals")
        ),
        "reassociated rebuild proposals on directed workers"
    );

    // The valid neighbour: the same calls, installed on the clean pool, answer -- and
    // answer the single-thread bits.
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
