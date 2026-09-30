// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Statistical constant-time tests (dudect: fixed secret against random
//! secrets, Welch's t-test, |t| < 10) of the operations that handle a secret
//! through the public API: signing and key expansion. The scalar
//! multiplication and scalar arithmetic run over a fixed against random scalar
//! in `src/timing_tests.rs`. See `common/dudect.rs` for the method and for how
//! a loaded host is accounted for.

mod common;

use common::dudect::{self, THRESHOLD};
use purrdf_ed25519::SigningKey;
use purrdf_testkit::rng::SplitMix64;

/// Samples per run. Signing is tens of microseconds, so a run is well under
/// ten seconds even on a loaded host.
const SAMPLES: usize = 100_000;

fn random_seed(rng: &mut SplitMix64) -> [u8; 32] {
    let mut seed = [0u8; 32];
    for chunk in seed.chunks_mut(8) {
        chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
    }
    seed
}

/// The harness must be able to see a leak, or a pass means nothing: a
/// secret-dependent amount of work (twice as much for the fixed class) has to be
/// reported.
#[test]
fn harness_detects_a_planted_leak() {
    let data = vec![1u64; 4000];
    let (ts, passed) = dudect::judge(
        "planted leak",
        SAMPLES,
        || 4000usize,
        |_| 2000usize,
        |&len| {
            std::hint::black_box(data[..len].iter().fold(0u64, |a, &b| a.wrapping_add(b)));
        },
    );
    assert!(
        !passed,
        "a a secret-dependent workload (the fixed class does twice the work) went unseen: {ts:?}"
    );
}

/// Signing the same message under a fixed key against fresh random keys.
#[test]
fn signing_is_constant_time() {
    let message = [0x5au8; 64];
    let (ts, passed) = dudect::judge(
        "sign",
        SAMPLES,
        || SigningKey::from_bytes(&[0x42; 32]),
        |rng| SigningKey::from_bytes(&random_seed(rng)),
        |key| {
            std::hint::black_box(key.sign(&message));
        },
    );
    assert!(
        passed,
        "signing shows a timing dependence on the key: |t| {ts:?} >= {THRESHOLD}"
    );
}

/// Key expansion: SHA-512, clamping and the fixed-base multiplication of the
/// secret scalar, for a fixed seed against fresh random seeds.
#[test]
fn key_expansion_is_constant_time() {
    let (ts, passed) = dudect::judge(
        "key expansion",
        SAMPLES,
        || [0u8; 32],
        random_seed,
        |seed| {
            std::hint::black_box(SigningKey::from_bytes(seed));
        },
    );
    assert!(
        passed,
        "key expansion shows a timing dependence on the seed: |t| {ts:?} >= {THRESHOLD}"
    );
}
