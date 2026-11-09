// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `XXH64`, the specified non-cryptographic hash used by Zstandard and Parquet.
//!
//! Implements the [algorithm specification](https://github.com/Cyan4973/xxHash/blob/dev/doc/xxhash_spec.md#xxh64-algorithm-description)
//! over caller-supplied bytes. Four independent accumulators process each
//! 32-byte stripe; both entry points share their rounds, convergence and tail.
//! All words are little-endian and arithmetic wraps modulo 2⁶⁴.
//!
//! [`hash`] borrows the entire input, including its final partial stripe.
//! [`Xxh64`] retains only an incomplete stripe between writes. Its standard
//! [`Hasher::finish`] borrows the state, so inspecting a prefix never consumes
//! it or changes a later write.
//!
//! ```
//! use core::hash::Hasher as _;
//! use purrdf_hash::xxh64::{Xxh64, hash};
//! let mut state = Xxh64::with_seed(0);
//! state.update(b"a");
//! state.update(b"bc");
//! assert_eq!(state.finish(), hash(0, b"abc"));
//! assert_eq!(hash(0, b""), 0xef46_db37_51d8_e999);
//! ```

use core::hash::Hasher;

use crate::block::BlockBuffer;

const STRIPE: usize = 32;
// `XXH64` specification, Overview: the five fixed 64-bit primes.
/// The first prime specified by `XXH64`, also used by Zstandard's match hash.
pub const PRIME_1: u64 = 0x9e37_79b1_85eb_ca87;
const P2: u64 = 0xc2b2_ae3d_27d4_eb4f;
const P3: u64 = 0x1656_67b1_9e37_79f9;
const P4: u64 = 0x85eb_ca77_c2b2_ae63;
const P5: u64 = 0x27d4_eb2f_1656_67c5;

/// A seeded, allocation-free `XXH64` stream.
#[derive(Clone)]
pub struct Xxh64 {
    length: u64,
    accumulators: [u64; 4],
    striped: bool,
    buffer: BlockBuffer<STRIPE>,
}

crate::debug_non_exhaustive!(Xxh64 {});
crate::default_from_new!(Xxh64);

impl Xxh64 {
    /// Start a stream with the standard zero seed.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_seed(0)
    }

    /// Start a stream with exactly the supplied 64-bit seed.
    #[must_use]
    pub const fn with_seed(seed: u64) -> Self {
        Self {
            length: 0,
            accumulators: initial(seed),
            striped: false,
            buffer: BlockBuffer::new(),
        }
    }

    /// Append bytes without allocating. Empty updates leave the state unchanged.
    #[inline]
    pub fn update(&mut self, input: &[u8]) {
        self.length = self.length.wrapping_add(input.len() as u64);
        let accumulators = &mut self.accumulators;
        let striped = &mut self.striped;
        self.buffer.absorb(STRIPE, input, |blocks| {
            *striped = true;
            accumulate(accumulators, blocks);
        });
    }
}

impl Hasher for Xxh64 {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }

    #[inline]
    fn finish(&self) -> u64 {
        // Before the first stripe the third accumulator still holds the seed.
        // Once striped, finish uses only the converged accumulators instead.
        finish(
            self.accumulators[2],
            self.striped.then_some(&self.accumulators),
            self.length,
            self.buffer.pending(),
        )
    }
}

/// Hash the supplied bytes with exactly the supplied seed, without buffering.
#[must_use]
#[inline]
pub fn hash(seed: u64, input: &[u8]) -> u64 {
    if input.len() < STRIPE {
        return finish(seed, None, input.len() as u64, input);
    }
    let whole = input.len() - input.len() % STRIPE;
    let (stripes, tail) = input.split_at(whole);
    let mut accumulators = initial(seed);
    accumulate(&mut accumulators, stripes);
    finish(seed, Some(&accumulators), input.len() as u64, tail)
}

const fn initial(seed: u64) -> [u64; 4] {
    [
        seed.wrapping_add(PRIME_1).wrapping_add(P2),
        seed.wrapping_add(P2),
        seed,
        seed.wrapping_sub(PRIME_1),
    ]
}

#[inline]
const fn round(accumulator: u64, lane: u64) -> u64 {
    accumulator
        .wrapping_add(lane.wrapping_mul(P2))
        .rotate_left(31)
        .wrapping_mul(PRIME_1)
}

#[inline]
fn accumulate(accumulators: &mut [u64; 4], stripes: &[u8]) {
    for stripe in stripes.as_chunks::<STRIPE>().0 {
        let lanes = stripe.as_chunks::<8>().0;
        accumulators[0] = round(accumulators[0], u64::from_le_bytes(lanes[0]));
        accumulators[1] = round(accumulators[1], u64::from_le_bytes(lanes[1]));
        accumulators[2] = round(accumulators[2], u64::from_le_bytes(lanes[2]));
        accumulators[3] = round(accumulators[3], u64::from_le_bytes(lanes[3]));
    }
}

#[inline]
fn finish(seed: u64, accumulators: Option<&[u64; 4]>, length: u64, tail: &[u8]) -> u64 {
    let mut result = if let Some(lanes) = accumulators {
        let mut result = lanes[0]
            .rotate_left(1)
            .wrapping_add(lanes[1].rotate_left(7))
            .wrapping_add(lanes[2].rotate_left(12))
            .wrapping_add(lanes[3].rotate_left(18));
        for &lane in lanes {
            result = (result ^ round(0, lane))
                .wrapping_mul(PRIME_1)
                .wrapping_add(P4);
        }
        result
    } else {
        seed.wrapping_add(P5)
    };
    result = result.wrapping_add(length);

    let (words, rest) = tail.as_chunks::<8>();
    for &word in words {
        result = (result ^ round(0, u64::from_le_bytes(word)))
            .rotate_left(27)
            .wrapping_mul(PRIME_1)
            .wrapping_add(P4);
    }
    let rest = if let Some((word, rest)) = rest.split_first_chunk::<4>() {
        let lane = u32::from_le_bytes(*word);
        result = (result ^ u64::from(lane).wrapping_mul(PRIME_1))
            .rotate_left(23)
            .wrapping_mul(P2)
            .wrapping_add(P3);
        rest
    } else {
        rest
    };
    for &byte in rest {
        result = (result ^ u64::from(byte).wrapping_mul(P5))
            .rotate_left(11)
            .wrapping_mul(PRIME_1);
    }
    result ^= result >> 33;
    result = result.wrapping_mul(P2);
    result ^= result >> 29;
    result = result.wrapping_mul(P3);
    result ^ (result >> 32)
}
