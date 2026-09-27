// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-geo`'s cross-target determinism check, run on both targets.
//!
//! This crate claims that a native answer and a `wasm32-unknown-unknown` answer
//! are **bit-identical**, and it has an argument for that claim: every geometric
//! decision is integer arithmetic, Rust specifies integer arithmetic completely
//! and identically on every target, and the crate root denies
//! `clippy::float_arithmetic` so no second path exists. But an argument is not
//! evidence, and the defect the crate exists to prevent is the one that produces
//! no symptom — so the claim is made *observable*.
//!
//! [`GOLDEN_DIGEST`] is the one constant in the tree. This target is
//! `harness = false` on the shared test runner, so the same named cases run
//! natively under `cargo test` and on `wasm32-unknown-unknown` in Node under
//! `scripts/wasm-test-runner.sh`, each asserting the same golden. Each case that
//! computes the digest also prints it on a `determinism-digest` line, and
//! `scripts/check-geo-determinism.sh` runs the target on both, reads this constant
//! out of this file, and fails unless every named case reports the same digest on
//! both targets and that digest is the golden. Two targets, one number, no
//! reasoning in between.
//!
//! The digest is computed inside
//! [`without_host_clock_or_entropy`], so on wasm32 a digest that reached a host
//! clock or entropy source fails by that source's name rather than agreeing by
//! accident.
//!
//! # When this test fails
//!
//! It means an output byte moved. That is not automatically a bug — a deliberate
//! change to a serializer, a measure or a matrix will move it — but it is never
//! nothing. Re-run `make geo-determinism` first: if native and wasm still agree
//! with each other and only the golden is stale, the change is a behaviour change
//! and the pull request must say WHICH output moved and why. If native and wasm
//! DISAGREE, the portability guarantee has broken and the digest is the least
//! interesting part of the problem.

use purrdf_geo::determinism::{corpus_len, digest};
use purrdf_testkit::harness::{print_line, without_host_clock_or_entropy};

/// The pinned cross-target digest.
///
/// `scripts/check-geo-determinism.sh` reads this constant out of this file by
/// name rather than restating it, so there is exactly one copy in the tree and
/// the native assertion and the wasm assertion cannot drift apart.
const GOLDEN_DIGEST: u64 = 0x9667_c2ee_2cd3_ad4b;

/// The digest, computed with the host's clocks and entropy withdrawn, and
/// reported on a `determinism-digest` line under `case`'s name for
/// `scripts/check-geo-determinism.sh` to compare across targets.
fn reported_digest(case: &str) -> u64 {
    let value = without_host_clock_or_entropy(digest);
    print_line(&format!(
        "determinism-digest case={case} digest={value:016x} corpus_len={}",
        corpus_len()
    ));
    value
}

/// The digest equals the pinned golden, on whichever target this runs.
fn the_digest_is_the_pinned_golden() {
    let value = reported_digest("the_digest_is_the_pinned_golden");
    assert_eq!(
        value, GOLDEN_DIGEST,
        "the determinism digest moved: computed {value:016x}, golden {GOLDEN_DIGEST:016x}. \
         Run `make geo-determinism`: if native and wasm still agree, an output byte \
         changed deliberately and the pull request must say which one; if they \
         disagree, the cross-target guarantee has broken."
    );
}

/// The digest is a pure function of the crate's source: no clock, no address, no
/// allocation order, no map iteration.
fn the_digest_does_not_move_between_runs() {
    let first = reported_digest("the_digest_does_not_move_between_runs");
    for run in 0..64 {
        assert_eq!(digest(), first, "run {run} diverged from the first");
    }
}

/// A digest that folded nothing would agree on two targets and prove nothing.
/// This is the non-vacuity check, asserted where the golden is pinned.
fn the_digest_is_not_vacuous() {
    assert!(
        corpus_len() >= 20,
        "the corpus must be large enough to be worth hashing, got {}",
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

purrdf_testkit::harness_main!(
    the_digest_does_not_move_between_runs,
    the_digest_is_not_vacuous,
    the_digest_is_the_pinned_golden,
);
