// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Statistical constant-time tests (see `tests/common/dudect.rs`) of the
//! secret-scalar arithmetic that the public API only reaches through a hash:
//! the fixed-base multiplication and the scalar multiply-add, for a fixed
//! scalar against fresh random scalars. The fixed scalars are the extremes a
//! branching or early-exit implementation would treat differently (zero, whose
//! every window digit is zero, and one).

use purrdf_testkit::rng::SplitMix64;

use crate::dudect::{self, THRESHOLD};
use crate::point::Point;
use crate::scalar::Scalar;

const SAMPLES: usize = 100_000;

fn random_scalar(rng: &mut SplitMix64) -> Scalar {
    let mut wide = [0u8; 64];
    for chunk in wide.chunks_mut(8) {
        chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
    }
    Scalar::from_bytes_wide(&wide)
}

fn one() -> Scalar {
    let mut bytes = [0u8; 32];
    bytes[0] = 1;
    Scalar::from_canonical_bytes(&bytes).expect("1 is canonical")
}

#[test]
fn base_multiplication_of_a_zero_scalar_takes_the_time_of_a_random_one() {
    let (ts, passed) = dudect::judge(
        "mul_base zero vs random",
        SAMPLES,
        || Scalar::ZERO,
        random_scalar,
        |s| {
            std::hint::black_box(Point::mul_base(s));
        },
    );
    assert!(
        passed,
        "mul_base depends on the scalar: |t| {ts:?} >= {THRESHOLD}"
    );
}

#[test]
fn base_multiplication_of_one_takes_the_time_of_a_random_scalar() {
    let (ts, passed) = dudect::judge("mul_base one vs random", SAMPLES, one, random_scalar, |s| {
        std::hint::black_box(Point::mul_base(s));
    });
    assert!(
        passed,
        "mul_base depends on the scalar: |t| {ts:?} >= {THRESHOLD}"
    );
}

#[test]
fn scalar_multiply_add_is_constant_time_in_the_secret() {
    let other = random_scalar(&mut SplitMix64::new(9));
    let addend = random_scalar(&mut SplitMix64::new(10));
    let (ts, passed) = dudect::judge(
        "mul_add one vs random",
        SAMPLES * 4,
        one,
        random_scalar,
        |s| {
            std::hint::black_box(other.mul_add(*s, addend));
        },
    );
    assert!(
        passed,
        "mul_add depends on the scalar: |t| {ts:?} >= {THRESHOLD}"
    );
}
