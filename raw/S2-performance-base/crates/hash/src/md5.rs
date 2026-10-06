// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! MD5 (RFC 1321).
//!
//! ```
//! use purrdf_hash::md5::Md5;
//!
//! let digest = Md5::digest(b"abc");
//! assert_eq!(digest[..4], [0x90, 0x01, 0x50, 0x98]);
//! ```
//!
//! MD5 is scalar on every target: each of its 64 steps consumes the result of
//! the one before, so a single message offers no data parallelism, and no
//! instruction set has MD5 instructions.

use core::fmt;

use crate::block::BlockBuffer;
use crate::digest::Digest;

/// The digest length in bytes.
pub const OUTPUT_LEN: usize = 16;

const BLOCK_LEN: usize = 64;

/// RFC 1321 §3.3: A = 01 23 45 67, B = 89 ab cd ef, C = fe dc ba 98,
/// D = 76 54 32 10, each listed low-order byte first.
const INITIAL: [u32; 4] = [
    u32::from_le_bytes([0x01, 0x23, 0x45, 0x67]),
    u32::from_le_bytes([0x89, 0xab, 0xcd, 0xef]),
    u32::from_le_bytes([0xfe, 0xdc, 0xba, 0x98]),
    u32::from_le_bytes([0x76, 0x54, 0x32, 0x10]),
];

/// RFC 1321 §3.4: the per-round left-rotation amounts, cycling every four steps.
const SHIFTS: [[u32; 4]; 4] = [
    [7, 12, 17, 22],
    [5, 9, 14, 20],
    [4, 11, 16, 23],
    [6, 10, 15, 21],
];

/// RFC 1321 §3.4: `T[i]` is the integer part of `4294967296 · |sin(i)|`,
/// `i` in radians, for `i = 1..=64` (stored zero-based).
const SINES: [u32; 64] = sine_table();

// --- The sine table, computed from its definition in fixed point ---------
//
// Everything below is Q59 fixed point in i128: a value v stands for v / 2^59.
// Q59 leaves room for the largest product the series forms — a Taylor term of
// magnitude below 6 times the squared argument, below 10 after reduction to
// [-π, π] — without overflowing i128, and keeps 27 bits beyond the 32 used.

const FRAC_BITS: u32 = 59;
const ONE: i128 = 1 << FRAC_BITS;

const fn fixed_mul(a: i128, b: i128) -> i128 {
    (a * b) >> FRAC_BITS
}

/// `arctan(1/n)` by its alternating series `Σ (-1)^k / ((2k+1)·n^(2k+1))`.
const fn arctan_inverse(n: i128) -> i128 {
    let mut power = ONE / n; // 1 / n^(2k+1)
    let mut sum = 0;
    let mut k = 0;
    while power != 0 {
        let term = power / (2 * k + 1);
        sum = if k % 2 == 0 { sum + term } else { sum - term };
        power /= n * n;
        k += 1;
    }
    sum
}

/// π by Machin's formula, `π = 16·arctan(1/5) − 4·arctan(1/239)`.
const fn pi() -> i128 {
    16 * arctan_inverse(5) - 4 * arctan_inverse(239)
}

/// `sin(x)` for `x` in `[-π, π]`, by its Taylor series run until the terms
/// vanish at this precision.
const fn sin_reduced(x: i128) -> i128 {
    let square = fixed_mul(x, x);
    let mut term = x;
    let mut sum = x;
    let mut n = 1;
    while term != 0 {
        term = -fixed_mul(term, square) / ((2 * n) * (2 * n + 1));
        sum += term;
        n += 1;
    }
    sum
}

const fn sine_table() -> [u32; 64] {
    let pi = pi();
    let two_pi = 2 * pi;
    let mut table = [0u32; 64];
    let mut i = 0;
    while i < 64 {
        // The argument is the integer i + 1 radians, reduced into [-π, π].
        let mut x = (i as i128 + 1) * ONE;
        x -= (x + pi).div_euclid(two_pi) * two_pi;
        let sine = sin_reduced(x).abs();
        // floor(2^32 · |sin|): drop the fraction bits below the 32 kept.
        table[i] = (sine >> (FRAC_BITS - 32)) as u32;
        i += 1;
    }
    table
}

// --- Block processing -----------------------------------------------------

#[inline]
const fn step(a: u32, b: u32, mixed: u32, word: u32, sine: u32, shift: u32) -> u32 {
    b.wrapping_add(
        a.wrapping_add(mixed)
            .wrapping_add(word)
            .wrapping_add(sine)
            .rotate_left(shift),
    )
}

/// RFC 1321 §3.4 over every 64-byte block of `blocks`.
fn compress(state: &mut [u32; 4], blocks: &[u8]) {
    for block in blocks.as_chunks::<BLOCK_LEN>().0 {
        let mut x = [0u32; 16];
        for (word, bytes) in x.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_le_bytes(*bytes);
        }
        let [mut a, mut b, mut c, mut d] = *state;
        // Each step writes the register in the `a` role; the roles then rotate
        // (ABCD, DABC, CDAB, BCDA), which is the same as renaming the four
        // variables after every step.
        for i in 0..16 {
            let mixed = (b & c) | (!b & d);
            let next = step(a, b, mixed, x[i], SINES[i], SHIFTS[0][i % 4]);
            (a, b, c, d) = (d, next, b, c);
        }
        for i in 16..32 {
            let mixed = (b & d) | (c & !d);
            let next = step(a, b, mixed, x[(1 + 5 * i) % 16], SINES[i], SHIFTS[1][i % 4]);
            (a, b, c, d) = (d, next, b, c);
        }
        for i in 32..48 {
            let mixed = b ^ c ^ d;
            let next = step(a, b, mixed, x[(5 + 3 * i) % 16], SINES[i], SHIFTS[2][i % 4]);
            (a, b, c, d) = (d, next, b, c);
        }
        for i in 48..64 {
            let mixed = c ^ (b | !d);
            let next = step(a, b, mixed, x[(7 * i) % 16], SINES[i], SHIFTS[3][i % 4]);
            (a, b, c, d) = (d, next, b, c);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
}

/// A streaming MD5 hasher.
#[derive(Clone)]
pub struct Md5 {
    state: [u32; 4],
    buffer: BlockBuffer<BLOCK_LEN>,
    /// The message length in bytes, modulo 2^64.
    length: u64,
}

impl Md5 {
    /// A hasher with nothing absorbed.
    pub const fn new() -> Self {
        Self {
            state: INITIAL,
            buffer: BlockBuffer::new(),
            length: 0,
        }
    }

    /// The MD5 digest of `data`.
    #[must_use]
    pub fn digest(data: &[u8]) -> [u8; OUTPUT_LEN] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }

    /// Absorb `data`.
    pub fn update(&mut self, data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        let state = &mut self.state;
        self.buffer
            .absorb(BLOCK_LEN, data, |blocks| compress(state, blocks));
    }

    /// The digest of everything absorbed.
    #[must_use]
    pub fn finalize(mut self) -> [u8; OUTPUT_LEN] {
        self.finish()
    }

    fn finish(&mut self) -> [u8; OUTPUT_LEN] {
        // RFC 1321 §3.2: the bit length, low-order word (and byte) first.
        let bits = self.length.wrapping_mul(8);
        let state = &mut self.state;
        self.buffer
            .finish_md(bits.to_le_bytes(), |block| compress(state, block));
        let mut out = [0u8; OUTPUT_LEN];
        for (bytes, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            *bytes = word.to_le_bytes();
        }
        out
    }
}

crate::default_from_new!(Md5);

impl fmt::Debug for Md5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Md5").finish_non_exhaustive()
    }
}

impl Digest for Md5 {
    fn output_len(&self) -> usize {
        OUTPUT_LEN
    }

    fn update(&mut self, data: &[u8]) {
        Self::update(self, data);
    }

    fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
        let digest = self.finish();
        out[..OUTPUT_LEN].copy_from_slice(&digest);
        *self = Self::new();
        OUTPUT_LEN
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_table_matches_its_definition_in_binary64() {
        // An independent check of the fixed-point series: binary64 `sin` has
        // 53 significant bits, far more than the 32 kept, so it agrees
        // wherever the fraction is not within 2^-20 of an integer.
        for (i, &entry) in SINES.iter().enumerate() {
            let scaled = ((i + 1) as f64).sin().abs() * 4_294_967_296.0;
            if (scaled - scaled.round()).abs() > 1e-6 * scaled.max(1.0) {
                assert_eq!(u64::from(entry), scaled.floor() as u64, "T[{}]", i + 1);
            }
        }
    }
}
