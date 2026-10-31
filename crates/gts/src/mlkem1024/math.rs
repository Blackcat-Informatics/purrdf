// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 203 field encoding, compression and quadratic NTT-domain products.

pub(super) use crate::lattice::{Kem as Field, N, Poly};
use purrdf_hash::SecretArray;

pub(super) const K: usize = 4;
pub(super) const Q: i32 = Field::Q;

// Algorithm 11: the modulus for pair i is `X^2 - zeta^(2 BitRev7(i) + 1)`.
// Generate these public constants from their definition, never an imported table.
const QUADRATIC_ROOTS: [i32; N / 2] = {
    let mut roots = [0; N / 2];
    let mut index = 0;
    while index < N / 2 {
        roots[index] = Field::root_power(2 * Field::bit_reverse(index as u8) as u16 + 1);
        index += 1;
    }
    roots
};

/// Add Algorithms 11–12's pair products to the original output polynomial.
pub(super) fn accumulate_product(out: &mut Poly, a: &Poly, b: &Poly) {
    for (index, gamma) in QUADRATIC_ROOTS.iter().copied().enumerate() {
        let at = 2 * index;
        let products = SecretArray::new([
            Field::mul(a[at], b[at]),
            Field::mul(a[at + 1], b[at + 1]),
            Field::mul(a[at], b[at + 1]),
            Field::mul(a[at + 1], b[at]),
        ]);
        out[at] = Field::add(
            out[at],
            Field::add(products[0], Field::mul(products[1], gamma)),
        );
        out[at + 1] = Field::add(out[at + 1], Field::add(products[2], products[3]));
    }
}

/// Algorithm 6 reduces modulo q only for its twelve-bit encoding.
pub(super) fn decode(bytes: &[u8], width: usize, out: &mut Poly) {
    debug_assert!((1..=12).contains(&width));
    crate::lattice::unpack(bytes, width, None, out);
    if width == 12 {
        for coefficient in out {
            *coefficient = (*coefficient as u32 % Q as u32) as i32;
        }
    }
}

/// Section 4.2.1 Compress_d followed by Algorithm 5 ByteEncode_d.
pub(super) fn compress_encode(poly: &Poly, width: usize, out: &mut [u8]) {
    debug_assert!((1..12).contains(&width));
    let mut compressed = SecretArray::new([0i32; N]);
    let mask = (1u32 << width) - 1;
    for (compressed, coefficient) in compressed.iter_mut().zip(poly) {
        // Round the exact positive ratio to nearest, then reduce modulo 2^d.
        *compressed =
            (((((*coefficient as u32) << width) + Q as u32 / 2) / Q as u32) & mask) as i32;
    }
    crate::lattice::pack(&compressed, width, None, out);
}

/// Algorithm 6 ByteDecode_d followed by Section 4.2.1 Decompress_d.
pub(super) fn decode_decompress(bytes: &[u8], width: usize, out: &mut Poly) {
    debug_assert!((1..12).contains(&width));
    decode(bytes, width, out);
    for coefficient in out {
        // Positive ties round upward, including Decompress_1(1) = 1665.
        *coefficient = ((Q as u32 * *coefficient as u32 + (1u32 << (width - 1))) >> width) as i32;
    }
}
