// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHA-1 (FIPS 180-4).
//!
//! ```
//! use purrdf_hash::sha1::Sha1;
//!
//! let mut hasher = Sha1::new();
//! hasher.update(b"ab");
//! hasher.update(b"c");
//! assert_eq!(hasher.finalize(), Sha1::digest(b"abc"));
//! ```
//!
//! Whole blocks run on the x86 SHA extensions or the Armv8 SHA1 instructions
//! when the processor reports them, and on portable code otherwise; the
//! answer is the same on every path.

use core::fmt;

use crate::arch::Sha1Blocks;
use crate::backend::Sha1Backend;
use crate::block::BlockBuffer;
use crate::digest::Digest;

/// The digest length in bytes.
pub const OUTPUT_LEN: usize = 20;

const BLOCK_LEN: usize = 64;

/// FIPS 180-4 §5.3.1.
const INITIAL: [u32; 5] = [
    0x6745_2301,
    0xefcd_ab89,
    0x98ba_dcfe,
    0x1032_5476,
    0xc3d2_e1f0,
];

/// FIPS 180-4 §4.2.1: `K_t` for t in 0–19, 20–39, 40–59 and 60–79.
pub(crate) const K: [u32; 4] = [0x5a82_7999, 0x6ed9_eba1, 0x8f1b_bcdc, 0xca62_c1d6];

/// FIPS 180-4 §6.1.2 over every 64-byte block of `blocks`, with the
/// sixteen-word circular message schedule of §6.1.3.
pub(crate) fn compress_portable(state: &mut [u32; 5], blocks: &[u8]) {
    for block in blocks.as_chunks::<BLOCK_LEN>().0 {
        let mut w = [0u32; 16];
        for (word, bytes) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes(*bytes);
        }
        let [mut a, mut b, mut c, mut d, mut e] = *state;
        macro_rules! rounds {
            ($range:expr, $k:expr, |$x:ident, $y:ident, $z:ident| $f:expr) => {
                for t in $range {
                    if t >= 16 {
                        let s = t % 16;
                        w[s] = (w[(t - 3) % 16] ^ w[(t - 8) % 16] ^ w[(t - 14) % 16] ^ w[s])
                            .rotate_left(1);
                    }
                    let ($x, $y, $z) = (b, c, d);
                    let temp = a
                        .rotate_left(5)
                        .wrapping_add($f)
                        .wrapping_add(e)
                        .wrapping_add($k)
                        .wrapping_add(w[t % 16]);
                    e = d;
                    d = c;
                    c = b.rotate_left(30);
                    b = a;
                    a = temp;
                }
            };
        }
        rounds!(0..20, K[0], |x, y, z| (x & y) ^ (!x & z));
        rounds!(20..40, K[1], |x, y, z| x ^ y ^ z);
        rounds!(40..60, K[2], |x, y, z| (x & y) ^ (x & z) ^ (y & z));
        rounds!(60..80, K[3], |x, y, z| x ^ y ^ z);
        for (word, add) in state.iter_mut().zip([a, b, c, d, e]) {
            *word = word.wrapping_add(add);
        }
    }
}

/// A streaming SHA-1 hasher.
#[derive(Clone)]
pub struct Sha1 {
    state: [u32; 5],
    buffer: BlockBuffer<BLOCK_LEN>,
    /// The message length in bytes, modulo 2^64.
    length: u64,
    backend: Sha1Backend,
    blocks: Sha1Blocks,
}

impl Sha1 {
    /// A hasher with nothing absorbed, on the fastest path this processor
    /// supports.
    pub fn new() -> Self {
        let backend = Sha1Backend::selected();
        Self::on(backend, backend.blocks().unwrap_or(compress_portable))
    }

    pub(crate) const fn on(backend: Sha1Backend, blocks: Sha1Blocks) -> Self {
        Self {
            state: INITIAL,
            buffer: BlockBuffer::new(),
            length: 0,
            backend,
            blocks,
        }
    }

    /// The SHA-1 digest of `data`.
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
        let blocks = self.blocks;
        self.buffer
            .absorb(BLOCK_LEN, data, |run| blocks(state, run));
    }

    /// The digest of everything absorbed.
    #[must_use]
    pub fn finalize(mut self) -> [u8; OUTPUT_LEN] {
        self.finish()
    }

    fn finish(&mut self) -> [u8; OUTPUT_LEN] {
        // FIPS 180-4 §5.1.1: the bit length as a big-endian 64-bit integer.
        let bits = self.length.wrapping_mul(8);
        let state = &mut self.state;
        let blocks = self.blocks;
        self.buffer
            .finish_md(bits.to_be_bytes(), |block| blocks(state, block));
        let mut out = [0u8; OUTPUT_LEN];
        for (bytes, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            *bytes = word.to_be_bytes();
        }
        out
    }

    fn restart(&mut self) {
        self.state = INITIAL;
        self.buffer.clear();
        self.length = 0;
    }
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Sha1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sha1")
            .field("backend", &self.backend.name())
            .finish_non_exhaustive()
    }
}

impl Digest for Sha1 {
    fn output_len(&self) -> usize {
        OUTPUT_LEN
    }

    fn update(&mut self, data: &[u8]) {
        Self::update(self, data);
    }

    fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
        let digest = self.finish();
        out[..OUTPUT_LEN].copy_from_slice(&digest);
        self.restart();
        OUTPUT_LEN
    }

    fn reset(&mut self) {
        self.restart();
    }
}
