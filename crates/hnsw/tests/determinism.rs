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
//! [`GOLDEN_DIGEST`] is the one constant in the tree. This target is `harness = false` on
//! the shared test runner, so the same named cases run natively under `cargo test` and on
//! `wasm32-unknown-unknown` in Node under `scripts/wasm-test-runner.sh`, each asserting the
//! same golden; natively it also pins the digest at four worker counts, which wasm32, with
//! no threads, cannot run. Each case that computes a digest prints it on a
//! `determinism-digest` line, and `scripts/check-hnsw-determinism.sh` runs the target
//! natively, on wasm32 and on wasm32 with `+simd128`, reads the goldens out of this file,
//! and fails unless every named case reports the same digest on every build and those
//! digests are the goldens. The digests are computed inside [`without_host_clock_or_entropy`],
//! so on wasm32 a build that reached a host clock or entropy source fails by that source's
//! name. The digest itself is hand-rolled FNV-1a over the canonical payload bytes — see
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

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{Arithmetic, Exact, Reassociated};
use purrdf_hnsw::determinism::{CORPUS_ROWS, corpus_len, digest, digest_serial};
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix, level::splitmix64};
use purrdf_testkit::harness::{print_line, without_host_clock_or_entropy};
#[cfg(not(target_arch = "wasm32"))]
use rayon::ThreadPoolBuilder;

/// The pinned digest of the profile-rule build.
///
/// `scripts/check-hnsw-determinism.sh` reads this constant out of this file by name rather
/// than restating it, so there is exactly one copy in the tree and the native assertion
/// and the wasm assertion cannot drift apart.
///
/// Moved when distances began folding in the sixteen-lane tree order and the image header
/// began carrying the arithmetic field: every recorded distance bit and the header both
/// changed, so the image and its digest did.
const GOLDEN_DIGEST: u64 = 0xa367_d6c5_8963_1389;

/// The pinned digest of the serial-insert build (`batch = 1`).
///
/// It is deliberately different from [`GOLDEN_DIGEST`]: the round structure changes the
/// graph, and that difference is asserted rather than assumed.
///
/// Moved for the same reason as [`GOLDEN_DIGEST`]: distances fold in the sixteen-lane tree
/// order, and the image header carries the arithmetic field.
const GOLDEN_SERIAL_DIGEST: u64 = 0xf0b2_fd33_0bcc_fcc7;

/// `compute`'s digest, computed with the host's clocks and entropy withdrawn, and reported
/// on a `determinism-digest` line under `case`'s name for
/// `scripts/check-hnsw-determinism.sh` to compare across targets and builds.
fn reported(case: &str, compute: fn() -> u64) -> u64 {
    let value = without_host_clock_or_entropy(compute);
    print_line(&format!(
        "determinism-digest case={case} digest={value:016x} corpus_len={}",
        corpus_len()
    ));
    value
}

/// The digest equals the pinned golden, on whichever target and build this runs.
fn the_digest_is_the_pinned_golden() {
    let value = reported("the_digest_is_the_pinned_golden", digest);
    assert_eq!(
        value, GOLDEN_DIGEST,
        "the digest is {value:016x}, golden {GOLDEN_DIGEST:016x}. If another target or \
         build still agrees with the golden, this one computes a different graph or image."
    );
}

/// The digest is a property of the input alone, not of a schedule: one, two, four and
/// eight rayon workers all fold the same bytes. Native only: wasm32-unknown-unknown has
/// no threads to build a worker pool from.
#[cfg(not(target_arch = "wasm32"))]
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
fn a_serial_insert_builds_a_different_graph() {
    let serial = reported("a_serial_insert_builds_a_different_graph", digest_serial);
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

/// The control for the reassociated surface: adding a second arithmetic moved nothing the
/// exact index publishes. The pinned exact digest still holds, and it holds beside a
/// reassociated build of the same shape whose image differs from the exact one in the
/// header's arithmetic field -- so this control can tell the exact image from the fast one,
/// and a reassociated arithmetic leaking into the exact build would move the digest.
fn exact_image_golden_unchanged_by_reassociated_surface() {
    assert_eq!(
        reported(
            "exact_image_golden_unchanged_by_reassociated_surface",
            digest
        ),
        GOLDEN_DIGEST,
        "the exact index's canonical image moved; the reassociated surface must leave it \
         byte for byte where it was"
    );

    let mut state = 0x5eed_f00d_7e57_0001_u64;
    let data: Vec<f64> = (0..64 * 96)
        .map(|_| {
            state = splitmix64(state);
            ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
        })
        .collect();
    let matrix = VectorMatrix::new(64, 96, data).expect("a valid matrix");
    let params = Params::new(8, 16, 32, 8).expect("valid parameters");
    let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params)
        .expect("builds");
    let fast = HnswIndex::build_reassociated(matrix, &DistanceMetric::SquaredEuclidean, params)
        .expect("builds");
    let (exact_image, fast_image) = (exact.canonical_image(), fast.canonical_image());
    let code = |image: &[u8]| u32::from_le_bytes(image[60..64].try_into().expect("four bytes"));
    assert_eq!(code(&exact_image), Exact::IMAGE_CODE);
    assert!(Reassociated::IMAGE_CODES.contains(&code(&fast_image)));
    assert_ne!(exact_image, fast_image);
    assert_eq!(
        exact_image[..60],
        fast_image[..60],
        "the header before the arithmetic field is the same identity"
    );
}

purrdf_testkit::harness_main!(
    a_serial_insert_builds_a_different_graph,
    exact_image_golden_unchanged_by_reassociated_surface,
    #[cfg(not(target_arch = "wasm32"))]
    the_digest_is_identical_across_worker_counts,
    the_digest_is_not_vacuous,
    the_digest_is_the_pinned_golden,
);
