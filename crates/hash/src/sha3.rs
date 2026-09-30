// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHA3-224, SHA3-256, SHA3-384 and SHA3-512, and the Keccak-f\[1600\]
//! permutation beneath them (FIPS 202).
//!
//! ```
//! use purrdf_hash::sha3::Sha3_256;
//!
//! let mut hasher = Sha3_256::new();
//! hasher.update(b"a");
//! hasher.update(b"bc");
//! assert_eq!(hasher.finalize(), Sha3_256::digest(b"abc"));
//! ```
//!
//! Every constant of the permutation — the round constants, the rotation
//! offsets and the lane permutation — is computed at compile time from the
//! standard's algorithms rather than typed as a table.

use core::fmt;

use crate::block::BlockBuffer;
use crate::digest::Digest;

/// The number of 64-bit lanes in the 1600-bit state.
const LANES: usize = 25;
/// Keccak-f\[1600\] has `12 + 2·l` rounds with `l = log2(64) = 6`.
const ROUNDS: usize = 24;
/// The largest rate among the four hash functions (SHA3-224's), in bytes.
const MAX_RATE: usize = 144;

/// FIPS 202 Algorithm 5, `rc(t)`: the output bit of an 8-bit linear feedback
/// shift register. Bit `i` of `r` holds `R[i]`.
const fn rc(t: usize) -> bool {
    let steps = t % 255;
    if steps == 0 {
        return true;
    }
    let mut r: u16 = 0b0000_0001; // R = 10000000
    let mut i = 1;
    while i <= steps {
        r <<= 1; // R = 0 || R
        let r8 = (r >> 8) & 1;
        r ^= r8 | (r8 << 4) | (r8 << 5) | (r8 << 6); // R[0], R[4], R[5], R[6] ^= R[8]
        r &= 0xff; // Trunc8
        i += 1;
    }
    r & 1 == 1
}

/// FIPS 202 Algorithm 6, steps 2–3: `RC[2^j − 1] = rc(j + 7·ir)` for
/// `j = 0..=6`, for every round index `ir`.
const fn round_constants() -> [u64; ROUNDS] {
    let mut constants = [0u64; ROUNDS];
    let mut ir = 0;
    while ir < ROUNDS {
        let mut j = 0;
        while j <= 6 {
            if rc(j + 7 * ir) {
                constants[ir] |= 1 << ((1 << j) - 1);
            }
            j += 1;
        }
        ir += 1;
    }
    constants
}

/// FIPS 202 Algorithm 2: the ρ rotation of lane `x + 5y`, walking
/// `(x, y) → (y, (2x + 3y) mod 5)` from `(1, 0)` for `t = 0..24`.
const fn rotation_offsets() -> [u32; LANES] {
    let mut offsets = [0u32; LANES];
    let (mut x, mut y) = (1, 0);
    let mut t = 0;
    while t < 24 {
        offsets[x + 5 * y] = (((t + 1) * (t + 2) / 2) % 64) as u32;
        (x, y) = (y, (2 * x + 3 * y) % 5);
        t += 1;
    }
    offsets
}

/// FIPS 202 Algorithm 3, `A'[x, y] = A[(x + 3y) mod 5, x]`: the source lane
/// of each destination lane `x + 5y`.
const fn permutation_sources() -> [usize; LANES] {
    let mut sources = [0usize; LANES];
    let mut y = 0;
    while y < 5 {
        let mut x = 0;
        while x < 5 {
            sources[x + 5 * y] = (x + 3 * y) % 5 + 5 * x;
            x += 1;
        }
        y += 1;
    }
    sources
}

const ROUND_CONSTANTS: [u64; ROUNDS] = round_constants();
const RHO: [u32; LANES] = rotation_offsets();
const PI_SOURCE: [usize; LANES] = permutation_sources();

/// One round, `ι(χ(π(ρ(θ(A)))), ir)` (FIPS 202 §3.3), with ρ and π fused:
/// each destination lane takes its π source lane, θ-adjusted and ρ-rotated.
macro_rules! round {
    ($state:expr, $constant:expr) => {{
        let a = &mut *$state;
        let round_constant = $constant;
        // θ: column parities, then each lane absorbs two neighbouring columns.
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        // ρ and π use compile-time lane indices and rotations. LLVM otherwise
        // retains indexed loads and variable shifts on baseline targets.
        let mut b = [0u64; LANES];
        macro_rules! rho_pi {
            ($dest:literal) => {{
                const SOURCE: usize = PI_SOURCE[$dest];
                b[$dest] = (a[SOURCE] ^ d[SOURCE % 5]).rotate_left(RHO[SOURCE]);
            }};
        }
        rho_pi!(0);
        rho_pi!(1);
        rho_pi!(2);
        rho_pi!(3);
        rho_pi!(4);
        rho_pi!(5);
        rho_pi!(6);
        rho_pi!(7);
        rho_pi!(8);
        rho_pi!(9);
        rho_pi!(10);
        rho_pi!(11);
        rho_pi!(12);
        rho_pi!(13);
        rho_pi!(14);
        rho_pi!(15);
        rho_pi!(16);
        rho_pi!(17);
        rho_pi!(18);
        rho_pi!(19);
        rho_pi!(20);
        rho_pi!(21);
        rho_pi!(22);
        rho_pi!(23);
        rho_pi!(24);
        // χ, row by row.
        for y in 0..5 {
            let row = 5 * y;
            for x in 0..5 {
                a[row + x] = b[row + x] ^ (!b[row + (x + 1) % 5] & b[row + (x + 2) % 5]);
            }
        }
        // ι.
        a[0] ^= round_constant;
    }};
}

/// Keccak-f\[1600\] (FIPS 202 Algorithm 7 with `nr = 24`) over a state whose
/// lane `(x, y)` is `state[x + 5y]`, each lane holding bits `z = 0..64` from
/// least to most significant.
pub fn keccak_f1600(state: &mut [u64; LANES]) {
    // Two rounds expose scheduling freedom without the instruction-cache
    // footprint of expanding all 24 rounds. Every round remains unchanged.
    for constants in ROUND_CONSTANTS.as_chunks::<2>().0 {
        round!(state, constants[0]);
        round!(state, constants[1]);
    }
}

/// A streaming SHA-3 hasher producing `OUT` bytes: one of [`Sha3_224`],
/// [`Sha3_256`], [`Sha3_384`] or [`Sha3_512`].
#[derive(Clone)]
pub struct Sha3<const OUT: usize> {
    state: [u64; LANES],
    buffer: BlockBuffer<MAX_RATE>,
}

/// SHA3-224: `KECCAK[448](M || 01, 224)`.
pub type Sha3_224 = Sha3<28>;
/// SHA3-256: `KECCAK[512](M || 01, 256)`.
pub type Sha3_256 = Sha3<32>;
/// SHA3-384: `KECCAK[768](M || 01, 384)`.
pub type Sha3_384 = Sha3<48>;
/// SHA3-512: `KECCAK[1024](M || 01, 512)`.
pub type Sha3_512 = Sha3<64>;

impl<const OUT: usize> Sha3<OUT> {
    /// The rate in bytes: the capacity is twice the digest length, and rate
    /// plus capacity is the 200-byte state.
    const RATE: usize = 200 - 2 * OUT;

    /// A hasher with nothing absorbed.
    pub const fn new() -> Self {
        const {
            assert!(
                OUT == 28 || OUT == 32 || OUT == 48 || OUT == 64,
                "FIPS 202 defines SHA-3 hash functions of 28, 32, 48 and 64 bytes only"
            );
        }
        Self {
            state: [0; LANES],
            buffer: BlockBuffer::new(),
        }
    }

    /// The digest of `data`.
    #[must_use]
    pub fn digest(data: &[u8]) -> [u8; OUT] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }

    /// Absorb `data`.
    pub fn update(&mut self, data: &[u8]) {
        let state = &mut self.state;
        self.buffer.absorb(Self::RATE, data, |blocks| {
            absorb_blocks(state, Self::RATE, blocks);
        });
    }

    /// The digest of everything absorbed.
    #[must_use]
    pub fn finalize(mut self) -> [u8; OUT] {
        self.finish()
    }

    fn finish(&mut self) -> [u8; OUT] {
        // FIPS 202 §6.1 and B.2: the suffix bits 01 and pad10*1 over a
        // byte-aligned message are 0x06, zeros, and 0x80 in the last rate
        // byte (0x86 when they share it).
        let (block, filled) = self.buffer.block_mut(Self::RATE);
        block[filled] = 0x06;
        block[filled + 1..].fill(0);
        block[Self::RATE - 1] |= 0x80;
        absorb_blocks(&mut self.state, Self::RATE, block);
        // Every digest length is below its rate, so one squeeze suffices.
        let mut out = [0u8; OUT];
        for (bytes, lane) in out.chunks_mut(8).zip(self.state) {
            bytes.copy_from_slice(&lane.to_le_bytes()[..bytes.len()]);
        }
        out
    }

    fn restart(&mut self) {
        self.state = [0; LANES];
        self.buffer.clear();
    }
}

/// XOR each `rate`-byte block into the leading lanes (FIPS 202 B.1: bytes
/// are little-endian within a lane) and permute.
fn absorb_blocks(state: &mut [u64; LANES], rate: usize, blocks: &[u8]) {
    for block in blocks.chunks_exact(rate) {
        for (lane, bytes) in state.iter_mut().zip(block.as_chunks::<8>().0) {
            *lane ^= u64::from_le_bytes(*bytes);
        }
        keccak_f1600(state);
    }
}

crate::default_from_new!([const OUT: usize] Sha3<OUT>);

impl<const OUT: usize> fmt::Debug for Sha3<OUT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sha3")
            .field("output_bits", &(OUT * 8))
            .finish_non_exhaustive()
    }
}

impl<const OUT: usize> Digest for Sha3<OUT> {
    fn output_len(&self) -> usize {
        OUT
    }

    fn update(&mut self, data: &[u8]) {
        Self::update(self, data);
    }

    fn finalize_reset(&mut self, out: &mut [u8]) -> usize {
        let digest = self.finish();
        out[..OUT].copy_from_slice(&digest);
        self.restart();
        OUT
    }

    fn reset(&mut self) {
        self.restart();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_constants_have_their_defining_properties() {
        // Round constants: only bits 2^j − 1 may be set.
        let allowed: u64 = (0..=6).map(|j| 1u64 << ((1 << j) - 1)).sum();
        assert!(ROUND_CONSTANTS.iter().all(|rc| rc & !allowed == 0));
        assert_eq!(
            ROUND_CONSTANTS[0], 1,
            "rc(0) = 1 and rc(1..=6) = 0 set only bit 0"
        );
        // Offsets: lane (0, 0) is not rotated and lane (1, 0) by 1 (t = 0).
        assert_eq!(RHO[0], 0);
        assert_eq!(RHO[1], 1);
        // π is a permutation fixing lane (0, 0).
        let mut seen = [false; LANES];
        for &source in &PI_SOURCE {
            assert!(!seen[source]);
            seen[source] = true;
        }
        assert_eq!(PI_SOURCE[0], 0);
    }
}
