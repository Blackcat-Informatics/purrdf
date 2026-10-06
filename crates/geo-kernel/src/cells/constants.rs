// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Equation-generated constants; the independent Machin generator verifies π.

pub(super) const Q96: u128 = 1 << 96;
pub(super) const Q62: i128 = 1 << 62;

/// RN(π × 2^192), in little-endian 64-bit limbs. There are 192 fractional bits.
pub(super) const PI192: [u64; 4] = [
    0xa409_3822_299f_31d0,
    0x1319_8a2e_0370_7344,
    0x243f_6a88_85a3_08d3,
    3,
];

/// The frozen first conversion of the angular pipeline: π192 to Q96.
pub(super) const PI96: u128 = rounded_pi96();

const fn rounded_pi96() -> u128 {
    let quotient =
        ((PI192[3] as u128) << 96) | ((PI192[2] as u128) << 32) | ((PI192[1] as u128) >> 32);
    let remainder = (((PI192[1] as u128) & (u32::MAX as u128)) << 64) | PI192[0] as u128;
    let half = 1_u128 << 95;
    quotient + (remainder > half || (remainder == half && quotient & 1 == 1)) as u128
}

/// Q96 coefficients of x Σ(-1)^k x^(2k)/(2k+1)!, k=0..9.
pub(super) const SINE: [i128; 10] = coefficients(true);
/// Q96 coefficients of Σ(-1)^k x^(2k)/(2k)!, k=0..9.
pub(super) const COSINE: [i128; 10] = coefficients(false);

const fn coefficients(sine: bool) -> [i128; 10] {
    let mut result = [0; 10];
    let mut factorial = 1_u128;
    let mut term = 0;
    let mut degree = 0;
    while term < result.len() {
        let wanted = 2 * term + sine as usize;
        while degree < wanted {
            degree += 1;
            factorial *= degree as u128;
        }
        let quotient = Q96 / factorial;
        let remainder = Q96 % factorial;
        let rounded = quotient
            + (remainder * 2 > factorial || (remainder * 2 == factorial && quotient & 1 == 1))
                as u128;
        let sign = if term & 1 == 0 { 1 } else { -1 };
        result[term] = sign * rounded as i128;
        term += 1;
    }
    result
}
