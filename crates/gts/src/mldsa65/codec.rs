// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! FIPS 204 Algorithms 16–28. Bit widths/layouts are fixed and caller-bounded.

use super::math::{K, Poly};
use super::{Error, SecretPolys};
pub(super) use crate::lattice::{pack, unpack};

pub(super) const OMEGA: usize = 55;
pub(super) const CHALLENGE_LENGTH: usize = 48;
pub(super) const RESPONSE_BYTES: usize = 640;
pub(super) const HINT_START: usize = CHALLENGE_LENGTH + 5 * RESPONSE_BYTES;

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
