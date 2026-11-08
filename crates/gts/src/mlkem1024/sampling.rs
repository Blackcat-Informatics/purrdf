// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 203 Algorithms 7–8 and the eta-two PRF used by ML-KEM-1024.

use super::math::{Field, N, Poly, Q};
use purrdf_hash::SecretArray;
use purrdf_hash::sha3::{Shake128, Shake256};

/// SampleNTT(rho || column || row). Rejection depends only on public rho.
pub(super) fn matrix_entry(rho: &[u8], row: u8, column: u8, out: &mut Poly) {
    debug_assert_eq!(rho.len(), 32);
    let mut sponge = Shake128::new();
    sponge.update(rho);
    sponge.update(&[column, row]);
    let mut reader = sponge.finalize();
    let mut bytes = SecretArray::new([0u8; 3]);
    let mut index = 0;
    while index < N {
        reader.squeeze(&mut bytes[..]);
        let candidates = [
            i32::from(bytes[0]) + 256 * i32::from(bytes[1] & 15),
            i32::from(bytes[1] >> 4) + 16 * i32::from(bytes[2]),
        ];
        for candidate in candidates {
            if candidate < Q && index < N {
                out[index] = candidate;
                index += 1;
            }
        }
    }
}

/// `SamplePolyCBD_2(SHAKE256(seed || nonce, 128))`, in a fixed bit schedule.
pub(super) fn noise(seed: &[u8], nonce: u8, out: &mut Poly) {
    debug_assert_eq!(seed.len(), 32);
    let mut sponge = Shake256::new();
    sponge.update(seed);
    sponge.update(&[nonce]);
    let mut bytes = SecretArray::new([0u8; 128]);
    sponge.finalize().squeeze(&mut bytes[..]);
    let mut sums = SecretArray::new([0i32; 2]);
    for (index, coefficient) in out.iter_mut().enumerate() {
        let shift = (index % 2) * 4;
        sums[0] = i32::from((bytes[index / 2] >> shift) & 1)
            + i32::from((bytes[index / 2] >> (shift + 1)) & 1);
        sums[1] = i32::from((bytes[index / 2] >> (shift + 2)) & 1)
            + i32::from((bytes[index / 2] >> (shift + 3)) & 1);
        *coefficient = Field::canonical(sums[0] - sums[1]);
    }
}
