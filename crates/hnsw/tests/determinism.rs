// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The native half of `purrdf-hnsw`'s cross-target determinism proof.
//!
//! This crate claims that a build is a **pure function of its input**: the same corpus
//! under the same parameters produces the same canonical byte image no matter how many
//! rayon workers ran, and no matter whether the target is native or
//! `wasm32-unknown-unknown`. That is an argument, and an argument is not evidence — the
//! failure mode determinism exists to prevent produces no symptom at all. So the claim is
//! made *observable*.
//!
//! [`GOLDEN_DIGEST`] is the one constant in the tree. This file pins it natively at four
//! worker counts, and `scripts/check-hnsw-determinism.sh` reads it out of this file, runs
//! the same digest under `wasm32-unknown-unknown` through the workspace-excluded
//! `crates/hnsw/determinism` cdylib, and fails unless the wasm side agrees. The digest
//! itself is hand-rolled FNV-1a over the canonical payload bytes — see
//! [`purrdf_hnsw::determinism`] — so a moved golden is a serialization defect and never a
//! hasher change.
//!
//! # Why a serial insert must differ
//!
//! The round-structured build pushes a whole batch against one frozen snapshot, so nodes
//! inside a batch cannot link to each other; a serial insertion lets every node see all of
//! its predecessors. `GOLDEN_SERIAL_DIGEST` pins that difference, which is what keeps the
//! round structure load-bearing rather than an untested claim that happens not to matter.
//!
//! # When this test fails
//!
//! A moved digest is not automatically a bug — a deliberate change to the builder or the
//! canonical image will move it — but it is never nothing. Re-run
//! `scripts/check-hnsw-determinism.sh`: if native and wasm still agree and only the golden
//! is stale, the change is a behaviour change and the pull request must say WHICH output
//! moved and why. If native and wasm DISAGREE, the portability guarantee has broken and
//! the digest is the least interesting part of the problem.

use purrdf_hnsw::determinism::{CORPUS_ROWS, corpus_len, digest, digest_serial};
use rayon::ThreadPoolBuilder;

/// The pinned digest of the profile-rule build.
///
/// `scripts/check-hnsw-determinism.sh` reads this constant out of this file by name rather
/// than restating it, so there is exactly one copy in the tree and the native assertion
/// and the wasm assertion cannot drift apart.
const GOLDEN_DIGEST: u64 = 0x0c71_b169_ebb4_4d7e;

/// The pinned digest of the serial-insert build (`batch = 1`).
///
/// It is deliberately different from [`GOLDEN_DIGEST`]: the round structure changes the
/// graph, and that difference is asserted rather than assumed.
const GOLDEN_SERIAL_DIGEST: u64 = 0x7e11_7799_b79a_b829;

/// The digest is a property of the input alone, not of a schedule: one, two, four and
/// eight rayon workers all fold the same bytes.
#[test]
fn the_digest_is_identical_across_worker_counts() {
    for workers in [1_usize, 2, 4, 8] {
        let pool = ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .expect("a worker pool is available");
        let value = pool.install(digest);
        assert_eq!(
            value, GOLDEN_DIGEST,
            "the digest under {workers} worker(s) is {value:016x}, golden \
             {GOLDEN_DIGEST:016x}. A thread-count-dependent graph is exactly the defect \
             the round-structured build exists to remove."
        );
    }
}

/// Within-round isolation is load-bearing: a serial insertion, where every node sees all
/// of its predecessors, produces a different canonical image.
#[test]
fn a_serial_insert_builds_a_different_graph() {
    let serial = digest_serial();
    assert_eq!(
        serial, GOLDEN_SERIAL_DIGEST,
        "the serial-insert digest moved: computed {serial:016x}, golden \
         {GOLDEN_SERIAL_DIGEST:016x}"
    );
    assert_ne!(
        serial, GOLDEN_DIGEST,
        "a serial insert produced the round build's digest; within-round isolation is not \
         changing the graph, so the round structure is not doing what it claims"
    );
}

/// A digest that folded nothing would agree on two targets and prove nothing. This is the
/// non-vacuity check, asserted where the golden is pinned.
#[test]
fn the_digest_is_not_vacuous() {
    assert_eq!(
        corpus_len(),
        CORPUS_ROWS,
        "the corpus length is the constant the digest folds"
    );
    assert!(
        corpus_len() >= 5_000,
        "the corpus must be large enough to exercise the graph, got {}",
        corpus_len()
    );
    assert_ne!(
        GOLDEN_DIGEST, 0,
        "an all-zero golden would be satisfied by a digest that folded nothing"
    );
    assert_ne!(
        GOLDEN_DIGEST, 0xcbf2_9ce4_8422_2325,
        "the golden must differ from FNV-1a's unfolded offset basis"
    );
}
