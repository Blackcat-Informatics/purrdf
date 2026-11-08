// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 204 rounding and norm rules over the shared fixed polynomial field.

pub(super) use crate::lattice::{Dsa as Field, N, Poly};

pub(super) const Q: i32 = Field::Q;
pub(super) const GAMMA1: i32 = 1 << 19;
pub(super) const GAMMA2: i32 = (Q - 1) / 32;
pub(super) const BETA: i32 = 196;
pub(super) const K: usize = 6;
pub(super) const L: usize = 5;

pub(super) fn power_round(value: i32) -> (i32, i32) {
    let high = (value + (1 << 12) - 1) >> 13;
    (high, value - (high << 13))
}

pub(super) fn decompose(value: i32) -> (i32, i32) {
    // The centered remainder assigns +gamma2 to the lower interval. The
    // q-1 wrap is corrected without a branch (Algorithm 36, lines 3–5).
    let high = (value + GAMMA2 - 1) / (2 * GAMMA2);
    (high & 15, value - high * (2 * GAMMA2) - (high >> 4))
}

pub(super) fn use_hint(hint: i32, value: i32) -> i32 {
    let (high, low) = decompose(value);
    let direction = 2 * i32::from(low > 0) - 1;
    (high + hint * direction) & 15
}

pub(super) fn norm_fails(polys: &[Poly], bound: i32) -> bool {
    // Read every coefficient before deciding; no secret-dependent early exit.
    let mut failed = 0;
    for poly in polys {
        for value in poly {
            failed |= i32::from(value.abs() >= bound);
        }
    }
    core::hint::black_box(failed) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntt_product_matches_negacyclic_integer_oracle() {
        let mut left = core::array::from_fn(|i| (i as i32 * 17 + 9) % Q);
        let mut right = core::array::from_fn(|i| (i as i32 * 19 + 3) % Q);
        let (original_left, original_right) = (left, right);
        Field::ntt(&mut left);
        Field::ntt(&mut right);
        for (a, b) in left.iter_mut().zip(right) {
            *a = Field::mul(*a, b);
        }
        Field::inverse(&mut left);
        for (i, actual) in left.iter().enumerate() {
            let mut sum = 0i64;
            for (j, a) in original_left.iter().enumerate() {
                let index = (i + N - j) % N;
                let sign = if j > i { -1 } else { 1 };
                sum += sign * i64::from(*a) * i64::from(original_right[index]);
            }
            assert_eq!(i64::from(*actual), sum.rem_euclid(i64::from(Q)));
        }
    }

    #[test]
    fn rounding_matches_centered_remainder_definition_exhaustively() {
        for value in 0..Q {
            let (high, low) = decompose(value);
            assert!((-GAMMA2..=GAMMA2).contains(&low));
            assert_eq!((high * 2 * GAMMA2 + low).rem_euclid(Q), value);
            assert!((0..16).contains(&high));
            let remainder = (value + GAMMA2 - 1).rem_euclid(2 * GAMMA2) - GAMMA2 + 1;
            if value - remainder == Q - 1 {
                assert_eq!((high, low), (0, remainder - 1));
            } else {
                assert_eq!((high, low), ((value - remainder) / (2 * GAMMA2), remainder));
            }
            let (top, bottom) = power_round(value);
            assert!((-4095..=4096).contains(&bottom));
            assert_eq!((top << 13) + bottom, value);
        }
    }
}
