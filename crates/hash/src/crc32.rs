// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CRC-32/ISO-HDLC: the polynomial `0x04C11DB7` processed bit-reflected
//! (`0xEDB88320`), register initialised to `0xFFFFFFFF` and the result
//! complemented. The check value (the CRC of `"123456789"`) is `0xCBF43926`.
//!
//! ```
//! use purrdf_hash::crc32::Crc32;
//!
//! assert_eq!(Crc32::checksum(b"123456789"), 0xCBF4_3926);
//! let mut crc = Crc32::new();
//! crc.update(b"1234");
//! crc.update(b"56789");
//! assert_eq!(crc.finalize(), 0xCBF4_3926);
//! ```
//!
//! Long inputs fold 64 bytes at a time with `pclmulqdq` on x86-64 or run the
//! Armv8 CRC32 instructions when the processor reports them; everything else
//! uses sixteen lookup tables (slicing-by-16) generated at compile time from
//! the polynomial.

use core::fmt;

use crate::arch::Crc32Update;
use crate::backend::Crc32Backend;
use crate::digest::Digest;
use crate::dispatch::Backend as _;

/// The generator polynomial with its `x^32` term: bit `d` is the coefficient
/// of `x^d`.
const POLYNOMIAL: u64 = 0x1_04C1_1DB7;
/// The polynomial without `x^32`, bit-reversed: the reflected form the
/// byte-at-a-time recurrence shifts right through.
const REFLECTED: u32 = (POLYNOMIAL as u32).reverse_bits();

const INIT: u32 = 0xFFFF_FFFF;
const XOROUT: u32 = 0xFFFF_FFFF;

/// The digest length of the [`Digest`] form: the CRC's four bytes,
/// most significant first.
pub const OUTPUT_LEN: usize = 4;

// --- Slicing-by-16 tables -------------------------------------------------

/// `TABLES[0][b]` is the register after one byte `b` from a zero register;
/// `TABLES[k][b]` is the effect of `b` followed by `k` zero bytes.
static TABLES: [[u32; 256]; 16] = tables();

const fn tables() -> [[u32; 256]; 16] {
    let mut tables = [[0u32; 256]; 16];
    let mut byte = 0;
    while byte < 256 {
        let mut register = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            register = if register & 1 == 1 {
                (register >> 1) ^ REFLECTED
            } else {
                register >> 1
            };
            bit += 1;
        }
        tables[0][byte] = register;
        byte += 1;
    }
    let mut k = 1;
    while k < 16 {
        let mut byte = 0;
        while byte < 256 {
            let previous = tables[k - 1][byte];
            tables[k][byte] = (previous >> 8) ^ tables[0][(previous & 0xff) as usize];
            byte += 1;
        }
        k += 1;
    }
    tables
}

/// The table-driven register update over any number of bytes. `register` is
/// the raw register (no init or final complement applied here).
pub(crate) fn update_portable(mut register: u32, data: &[u8]) -> u32 {
    let t = &TABLES;
    let (chunks, tail) = data.as_chunks::<16>();
    for chunk in chunks {
        let head = register ^ u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        register = t[15][(head & 0xff) as usize]
            ^ t[14][((head >> 8) & 0xff) as usize]
            ^ t[13][((head >> 16) & 0xff) as usize]
            ^ t[12][(head >> 24) as usize]
            ^ t[11][chunk[4] as usize]
            ^ t[10][chunk[5] as usize]
            ^ t[9][chunk[6] as usize]
            ^ t[8][chunk[7] as usize]
            ^ t[7][chunk[8] as usize]
            ^ t[6][chunk[9] as usize]
            ^ t[5][chunk[10] as usize]
            ^ t[4][chunk[11] as usize]
            ^ t[3][chunk[12] as usize]
            ^ t[2][chunk[13] as usize]
            ^ t[1][chunk[14] as usize]
            ^ t[0][chunk[15] as usize];
    }
    for &byte in tail {
        register = t[0][((register ^ u32::from(byte)) & 0xff) as usize] ^ (register >> 8);
    }
    register
}

// --- Carry-less-multiply folding constants --------------------------------
//
// The folding kernels work on the bit-reflected message: a 16-byte
// little-endian load is a 128-bit integer whose bit i is the coefficient of
// x^(127 − i) within that chunk. A 64-bit operand is read the same way (bit j
// is x^(63 − j)), so `frame64(p) = p.reverse_bits()` for a polynomial p of
// degree at most 63 written with bit d = x^d.
//
// A carry-less product of two such operands has bit k equal to the
// coefficient of x^(126 − k) of the product; read as a 128-bit chunk (bit k =
// x^(127 − k)) that is x times the product. Each constant below absorbs that
// factor of x, which is why they are x^(n − 1) rather than x^n.
//
// Folding a 128-bit accumulator F = H·x^64 + L (H the low quadword of the
// load, L the high one) forward over D bits of later data replaces F·x^D by
// H·(x^(D + 63) mod P) ⊕ L·(x^(D − 1) mod P) — congruent modulo P and of
// degree below 128 — which is then XORed into the data it was folded over.

/// `x^n mod P`, bit d = x^d.
#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(
        dead_code,
        reason = "only the x86-64 and AArch64 folding kernels read these"
    )
)]
const fn x_pow_mod(n: u32) -> u64 {
    let mut remainder: u64 = 1;
    let mut i = 0;
    while i < n {
        remainder <<= 1;
        if remainder & (1 << 32) != 0 {
            remainder ^= POLYNOMIAL;
        }
        i += 1;
    }
    remainder
}

/// `floor(x^64 / P)` by long division, bit d = x^d (degree 32).
#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(
        dead_code,
        reason = "only the x86-64 and AArch64 folding kernels read these"
    )
)]
const fn barrett_quotient() -> u64 {
    let mut remainder: u128 = 1 << 64;
    let mut quotient: u64 = 0;
    let mut bit = 64;
    while bit >= 32 {
        if remainder & (1 << bit) != 0 {
            quotient |= 1 << (bit - 32);
            remainder ^= (POLYNOMIAL as u128) << (bit - 32);
        }
        bit -= 1;
    }
    quotient
}

#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(
        dead_code,
        reason = "only the x86-64 and AArch64 folding kernels read these"
    )
)]
const fn frame64(polynomial: u64) -> u64 {
    polynomial.reverse_bits()
}

/// The folding constants, each as a reflected 64-bit operand.
#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(
        dead_code,
        reason = "only the x86-64 and AArch64 folding kernels read these"
    )
)]
#[cfg_attr(
    target_arch = "aarch64",
    allow(
        dead_code,
        reason = "the AArch64 kernel reduces with crc32x, not Barrett"
    )
)]
#[derive(Clone, Copy)]
pub(crate) struct FoldConstants {
    /// Fold four accumulators forward 512 bits: `[for H, for L]`.
    pub(crate) by_512: [u64; 2],
    /// Fold one accumulator forward 128 bits: `[for H, for L]`.
    pub(crate) by_128: [u64; 2],
    /// Reduce the 128-bit accumulator times x^32 to 96 bits: `x^95 mod P`.
    pub(crate) to_96: u64,
    /// Reduce 96 bits to 64: `x^63 mod P`.
    pub(crate) to_64: u64,
    /// Barrett's `floor(x^64 / P)`.
    pub(crate) mu: u64,
    /// `P` itself, with its x^32 term.
    pub(crate) polynomial: u64,
}

#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(
        dead_code,
        reason = "only the x86-64 and AArch64 folding kernels read these"
    )
)]
pub(crate) const FOLD: FoldConstants = FoldConstants {
    by_512: [frame64(x_pow_mod(512 + 63)), frame64(x_pow_mod(512 - 1))],
    by_128: [frame64(x_pow_mod(128 + 63)), frame64(x_pow_mod(128 - 1))],
    to_96: frame64(x_pow_mod(95)),
    to_64: frame64(x_pow_mod(63)),
    mu: frame64(barrett_quotient()),
    polynomial: frame64(POLYNOMIAL),
};

// --- The hasher -----------------------------------------------------------

/// A streaming CRC-32/ISO-HDLC.
#[derive(Clone, Copy)]
pub struct Crc32 {
    register: u32,
    backend: Crc32Backend,
    update: Crc32Update,
}

impl Crc32 {
    /// A CRC over no bytes yet, on the fastest path this processor supports.
    pub fn new() -> Self {
        let backend = Crc32Backend::selected();
        Self::on(backend, backend.update_fn().unwrap_or(update_portable))
    }

    pub(crate) const fn on(backend: Crc32Backend, update: Crc32Update) -> Self {
        Self {
            register: INIT,
            backend,
            update,
        }
    }

    /// The CRC-32 of `data`.
    #[must_use]
    pub fn checksum(data: &[u8]) -> u32 {
        let mut crc = Self::new();
        crc.update(data);
        crc.finalize()
    }

    /// Absorb `data`.
    pub fn update(&mut self, data: &[u8]) {
        self.register = (self.update)(self.register, data);
    }

    /// The CRC of everything absorbed.
    #[must_use]
    pub const fn finalize(self) -> u32 {
        self.register ^ XOROUT
    }
}

crate::default_from_new!(Crc32);

impl fmt::Debug for Crc32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Crc32")
            .field("backend", &self.backend.name())
            .finish_non_exhaustive()
    }
}

impl Digest for Crc32 {
    fn output_len(&self) -> usize {
        OUTPUT_LEN
    }

    fn update(&mut self, data: &[u8]) {
        Self::update(self, data);
    }

    fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
        out[..OUTPUT_LEN].copy_from_slice(&(self.register ^ XOROUT).to_be_bytes());
        self.register = INIT;
        OUTPUT_LEN
    }

    fn reset(&mut self) {
        self.register = INIT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Polynomial product over GF(2), bit d = x^d.
    fn multiply(a: u64, b: u64) -> u128 {
        (0..64)
            .filter(|bit| b >> bit & 1 == 1)
            .fold(0u128, |product, bit| product ^ (u128::from(a) << bit))
    }

    #[test]
    fn barrett_quotient_divides_x64() {
        // x^64 = μ·P + r with deg r < 32.
        let remainder = (1u128 << 64) ^ multiply(barrett_quotient(), POLYNOMIAL);
        assert!(remainder < 1 << 32);
    }

    #[test]
    fn powers_reduce_consistently() {
        // x^(a+b) mod P = (x^a mod P)(x^b mod P) mod P.
        let reduce = |mut value: u128| {
            for bit in (32..128).rev() {
                if value >> bit & 1 == 1 {
                    value ^= u128::from(POLYNOMIAL) << (bit - 32);
                }
            }
            value as u64
        };
        assert_eq!(
            reduce(multiply(x_pow_mod(300), x_pow_mod(275))),
            x_pow_mod(575)
        );
        assert_eq!(x_pow_mod(31), 1 << 31);
        assert_eq!(x_pow_mod(32), POLYNOMIAL ^ (1 << 32));
    }
}
