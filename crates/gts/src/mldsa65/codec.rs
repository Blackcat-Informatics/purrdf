// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 204 Algorithms 16–28. Bit widths/layouts are fixed and caller-bounded.

use super::math::{K, N, Poly};
use super::{Error, SecretPolys};
use purrdf_hash::SecretArray;

pub(super) const OMEGA: usize = 55;
pub(super) const CHALLENGE_LENGTH: usize = 48;
pub(super) const RESPONSE_BYTES: usize = 640;
pub(super) const HINT_START: usize = CHALLENGE_LENGTH + 5 * RESPONSE_BYTES;

pub(super) fn pack(poly: &Poly, width: usize, upper: Option<i32>, out: &mut [u8]) {
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

pub(super) fn unpack(bytes: &[u8], width: usize, upper: Option<i32>, out: &mut Poly) {
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

pub(super) fn hints_decode(bytes: &[u8]) -> Result<SecretPolys, Error> {
    let mut hints = SecretPolys::zeros(K);
    let mut index = 0;
    for (poly, end) in hints.0.iter_mut().zip(&bytes[OMEGA..]) {
        let end = usize::from(*end);
        if end < index || end > OMEGA {
            return Err(Error::InvalidSignature);
        }
        let positions = &bytes[index..end];
        if positions.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(Error::InvalidSignature);
        }
        for position in positions {
            poly[usize::from(*position)] = 1;
        }
        index = end;
    }
    if bytes[index..OMEGA].iter().any(|byte| *byte != 0) {
        return Err(Error::InvalidSignature);
    }
    Ok(hints)
}

pub(super) fn hints_encode(hints: &[Poly], bytes: &mut [u8]) {
    bytes.fill(0);
    let mut index = 0;
    for (row, poly) in hints.iter().enumerate() {
        for (position, hint) in poly.iter().enumerate() {
            // This is the public signature output after all rejection checks.
            if *hint != 0 {
                bytes[index] = position as u8;
                index += 1;
            }
        }
        bytes[OMEGA + row] = index as u8;
    }
}
