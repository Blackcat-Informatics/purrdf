// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native deterministic geometry output, pinned against one committed digest.
//!
//! Every geometric decision uses integer arithmetic, and the crate root denies
//! `clippy::float_arithmetic`. The complete serialized-byte corpus runs under
//! `cargo test`; the native conformance matrix reads this same golden.
//! Release-crate WASM compilation is checked separately by `make wasm`.
//!
//! A moved digest means an output byte changed. Identify the changed output and
//! its cause before changing the golden; a deliberate behavior change needs an
//! explicit explanation in review.

use purrdf_geo::determinism::{corpus_len, digest};
use purrdf_testkit::harness::report_digest;

/// The pinned native output digest, also read directly by the conformance matrix.
const GOLDEN_DIGEST: u64 = 0x9667_c2ee_2cd3_ad4b;

/// The digest equals the pinned golden, on whichever target this runs.
fn the_digest_is_the_pinned_golden() {
    let value = report_digest("the_digest_is_the_pinned_golden", corpus_len(), digest);
    assert_eq!(
        value, GOLDEN_DIGEST,
        "the determinism digest moved: computed {value:016x}, golden {GOLDEN_DIGEST:016x}. \
         Identify which serialized output changed and why before updating the golden. \
         Run `cargo test -p purrdf-geo --test determinism` to reproduce."
    );
}

/// The digest is a pure function of the crate's source: no clock, no address, no
/// allocation order, no map iteration.
fn the_digest_does_not_move_between_runs() {
    let first = report_digest(
        "the_digest_does_not_move_between_runs",
        corpus_len(),
        digest,
    );
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
        GOLDEN_DIGEST,
        purrdf_hash::fnv::BASIS,
        "the golden must differ from FNV-1a's unfolded offset basis"
    );
}

purrdf_testkit::harness_main!(
    the_digest_does_not_move_between_runs,
    the_digest_is_not_vacuous,
    the_digest_is_the_pinned_golden,
);
