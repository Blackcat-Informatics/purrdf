// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 204 Algorithms 35–43. Each arithmetic input is canonical modulo q.

pub(super) const Q: i32 = 8_380_417;
pub(super) const GAMMA1: i32 = 1 << 19;
pub(super) const GAMMA2: i32 = (Q - 1) / 32;
pub(super) const BETA: i32 = 196;
pub(super) const K: usize = 6;
pub(super) const L: usize = 5;
pub(super) const N: usize = 256;
pub(super) type Poly = [i32; N];

// Generated from the definition, not an imported coefficient table. Algorithm
// 43 is exactly u8::reverse_bits; exponentiation operates on public constants.
const fn roots() -> [i32; N] {
    let mut roots = [0; N];
    let mut index = 0;
    while index < N {
        let mut exponent = (index as u8).reverse_bits();
        let mut value = 1u64;
        let mut base = 1753u64;
        while exponent != 0 {
            if exponent & 1 != 0 {
                value = value * base % Q as u64;
            }
            base = base * base % Q as u64;
            exponent >>= 1;
        }
        roots[index] = value as i32;
        index += 1;
    }
    roots
}
const ZETAS: [i32; N] = roots();

#[inline]
pub(super) fn add(a: i32, b: i32) -> i32 {
    let sum = a + b - Q;
    sum + (core::hint::black_box(sum >> 31) & Q)
}

#[inline]
pub(super) fn sub(a: i32, b: i32) -> i32 {
    let difference = a - b;
    difference + (core::hint::black_box(difference >> 31) & Q)
}

#[inline]
pub(super) fn mul(a: i32, b: i32) -> i32 {
    // Fixed positive operands and a constant divisor: no input-dependent
    // division loop, table, branch or memory index.
    ((a as u64 * b as u64) % Q as u64) as i32
}

pub(super) fn canonical(value: i32) -> i32 {
    // Callers pass values in (-q, q): small sampled/decoded coefficients.
    value + (core::hint::black_box(value >> 31) & Q)
}

pub(super) fn centered(value: i32) -> i32 {
    value - (core::hint::black_box(-i32::from(value > Q / 2)) & Q)
}

pub(super) fn ntt(poly: &mut Poly) {
    for value in &mut *poly {
        *value = canonical(*value);
    }
    let mut index = 0;
    let mut length = N / 2;
    while length != 0 {
        for start in (0..N).step_by(2 * length) {
            index += 1;
            let root = ZETAS[index];
            for offset in start..start + length {
                let product = mul(root, poly[offset + length]);
                let left = poly[offset];
                poly[offset] = add(left, product);
                poly[offset + length] = sub(left, product);
            }
        }
        length /= 2;
    }
}

pub(super) fn inverse(poly: &mut Poly) {
    let mut index = N;
    let mut length = 1;
    while length < N {
        for start in (0..N).step_by(2 * length) {
            index -= 1;
            let root = Q - ZETAS[index];
            for offset in start..start + length {
                let left = poly[offset];
                let right = poly[offset + length];
                poly[offset] = add(left, right);
                poly[offset + length] = mul(root, sub(left, right));
            }
        }
        length *= 2;
    }
    // Algorithm 42: 256^-1 mod q = q - (q - 1)/256.
    for value in poly {
        *value = mul(*value, Q - (Q - 1) / 256);
    }
}

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
        ntt(&mut left);
        ntt(&mut right);
        for (a, b) in left.iter_mut().zip(right) {
            *a = mul(*a, b);
        }
        inverse(&mut left);
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
