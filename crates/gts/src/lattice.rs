// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixed 256-coefficient field transforms and bit codec of FIPS 203/204.

use purrdf_hash::SecretArray;

pub(crate) const N: usize = 256;
pub(crate) type Poly = [i32; N];

pub(crate) struct Lattice<
    const MODULUS: i32,
    const ROOT: u64,
    const MIN_LENGTH: usize,
    const REVERSE_BITS: u32,
    const INVERSE_SCALE: i32,
>;

pub(crate) type Dsa = Lattice<8_380_417, 1753, 1, 8, 8_347_681>;
pub(crate) type Kem = Lattice<3329, 17, 2, 7, 3303>;

impl<
    const MODULUS: i32,
    const ROOT: u64,
    const MIN_LENGTH: usize,
    const REVERSE_BITS: u32,
    const INVERSE_SCALE: i32,
> Lattice<MODULUS, ROOT, MIN_LENGTH, REVERSE_BITS, INVERSE_SCALE>
{
    pub(crate) const Q: i32 = MODULUS;

    pub(crate) const fn bit_reverse(index: u8) -> u8 {
        index.reverse_bits() >> (8 - REVERSE_BITS)
    }

    /// Exponentiation of public roots only; no imported coefficient table.
    pub(crate) const fn root_power(mut exponent: u16) -> i32 {
        let mut value = 1u64;
        let mut base = ROOT;
        while exponent != 0 {
            if exponent & 1 != 0 {
                value = value * base % MODULUS as u64;
            }
            base = base * base % MODULUS as u64;
            exponent >>= 1;
        }
        value as i32
    }

    const ZETAS: [i32; N] = {
        let mut roots = [0; N];
        let mut index = 0;
        while index < N {
            roots[index] = Self::root_power(Self::bit_reverse(index as u8) as u16);
            index += 1;
        }
        roots
    };

    #[inline]
    pub(crate) fn add(a: i32, b: i32) -> i32 {
        let sum = a + b - MODULUS;
        sum + (core::hint::black_box(sum >> 31) & MODULUS)
    }

    #[inline]
    pub(crate) fn sub(a: i32, b: i32) -> i32 {
        let difference = a - b;
        difference + (core::hint::black_box(difference >> 31) & MODULUS)
    }

    #[inline]
    pub(crate) fn mul(a: i32, b: i32) -> i32 {
        // Fixed positive operands and a constant divisor: no input-dependent
        // division loop, table, branch or memory index.
        ((a as u64 * b as u64) % MODULUS as u64) as i32
    }

    pub(crate) fn canonical(value: i32) -> i32 {
        // Callers pass values in (-q, q): small sampled/decoded coefficients.
        value + (core::hint::black_box(value >> 31) & MODULUS)
    }

    pub(crate) fn centered(value: i32) -> i32 {
        value - (core::hint::black_box(-i32::from(value > MODULUS / 2)) & MODULUS)
    }

    pub(crate) fn ntt(poly: &mut Poly) {
        for value in &mut *poly {
            *value = Self::canonical(*value);
        }
        let mut index = 0;
        let mut length = N / 2;
        while length >= MIN_LENGTH {
            for start in (0..N).step_by(2 * length) {
                index += 1;
                let root = Self::ZETAS[index];
                for offset in start..start + length {
                    let product = Self::mul(root, poly[offset + length]);
                    let left = poly[offset];
                    poly[offset] = Self::add(left, product);
                    poly[offset + length] = Self::sub(left, product);
                }
            }
            length /= 2;
        }
    }

    pub(crate) fn inverse(poly: &mut Poly) {
        let mut index = N / MIN_LENGTH;
        let mut length = MIN_LENGTH;
        while length < N {
            for start in (0..N).step_by(2 * length) {
                index -= 1;
                let root = Self::ZETAS[index];
                for offset in start..start + length {
                    let left = poly[offset];
                    let right = poly[offset + length];
                    poly[offset] = Self::add(left, right);
                    // FIPS 203 Algorithm 10. This is also the original DSA
                    // butterfly (q-zeta)*(left-right), with the signs moved.
                    poly[offset + length] = Self::mul(root, Self::sub(right, left));
                }
            }
            length *= 2;
        }
        for value in poly {
            *value = Self::mul(*value, INVERSE_SCALE);
        }
    }
}

pub(crate) fn pack(poly: &Poly, width: usize, upper: Option<i32>, out: &mut [u8]) {
    debug_assert_eq!(out.len(), N * width / 8);
    let mut accumulator = SecretArray::new([0u32]);
    let mut pending = 0usize;
    let mut index = 0;
    for coefficient in poly {
        let value = upper.map_or(*coefficient, |bound| bound - coefficient) as u32;
        accumulator[0] |= value << pending;
        pending += width;
        while pending >= 8 {
            out[index] = accumulator[0] as u8;
            index += 1;
            accumulator[0] >>= 8;
            pending -= 8;
        }
    }
    debug_assert_eq!(pending, 0);
}

pub(crate) fn unpack(bytes: &[u8], width: usize, upper: Option<i32>, out: &mut Poly) {
    debug_assert_eq!(bytes.len(), N * width / 8);
    let mut accumulator = SecretArray::new([0u32]);
    let mut pending = 0usize;
    let mut index = 0;
    let mask = (1 << width) - 1;
    for coefficient in out {
        while pending < width {
            accumulator[0] |= u32::from(bytes[index]) << pending;
            index += 1;
            pending += 8;
        }
        let value = (accumulator[0] & mask) as i32;
        *coefficient = upper.map_or(value, |bound| bound - value);
        accumulator[0] >>= width;
        pending -= width;
    }
}
