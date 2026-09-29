// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! x86-64 kernels: SHA-1 on the SHA extensions, CRC-32 on `pclmulqdq`,
//! base16 encoding on SSSE3 `pshufb`, and the fixed hasher's AES round on
//! builds whose target enables `aes`.

#[cfg(target_feature = "aes")]
use core::arch::x86_64::_mm_aesenc_si128;
use core::arch::x86_64::{
    __m128i, _mm_add_epi32, _mm_and_si128, _mm_clmulepi64_si128, _mm_cvtsi32_si128,
    _mm_cvtsi64_si128, _mm_cvtsi128_si64, _mm_extract_epi32, _mm_extract_epi64, _mm_loadu_si128,
    _mm_set_epi8, _mm_set_epi32, _mm_set_epi64x, _mm_set1_epi8, _mm_setzero_si128,
    _mm_sha1msg1_epu32, _mm_sha1msg2_epu32, _mm_sha1nexte_epu32, _mm_sha1rnds4_epu32,
    _mm_shuffle_epi8, _mm_srli_epi16, _mm_srli_si128, _mm_storeu_si128, _mm_unpackhi_epi8,
    _mm_unpackhi_epi64, _mm_unpacklo_epi8, _mm_xor_si128,
};
use std::is_x86_feature_detected;

use super::{Crc32Update, HexEncode, Sha1Blocks};
use crate::crc32::{FOLD, update_portable};
use crate::hex::{ALPHABET, UPPER_ALPHABET, encode_portable};

/// Sixteen bytes as a vector. Safe to call from any function compiled with
/// SSE2, which every x86-64 processor has.
#[inline]
fn load(bytes: &[u8; 16]) -> __m128i {
    // SAFETY: `bytes` is sixteen readable bytes, and `_mm_loadu_si128` has no
    // alignment requirement; SSE2 is part of the x86-64 baseline.
    unsafe { _mm_loadu_si128(bytes.as_ptr().cast()) }
}

/// Writes a vector's sixteen bytes. Safe to call from any function compiled
/// with SSE2, which every x86-64 processor has.
#[inline]
fn store(bytes: &mut [u8; 16], vector: __m128i) {
    // SAFETY: `bytes` is sixteen writable bytes, and `_mm_storeu_si128` has
    // no alignment requirement; SSE2 is part of the x86-64 baseline.
    unsafe { _mm_storeu_si128(bytes.as_mut_ptr().cast(), vector) }
}

// --- SHA-1 ----------------------------------------------------------------

/// The SHA-NI block function, if this processor has `sha`, `ssse3` and
/// `sse4.1`.
pub(crate) fn sha1_x86_sha() -> Option<Sha1Blocks> {
    (is_x86_feature_detected!("sha")
        && is_x86_feature_detected!("ssse3")
        && is_x86_feature_detected!("sse4.1"))
    .then_some(sha1_blocks as Sha1Blocks)
}

fn sha1_blocks(state: &mut [u32; 5], blocks: &[u8]) {
    // SAFETY: this function escapes the module only through `sha1_x86_sha`,
    // which returns it after detecting `sha`, `ssse3` and `sse4.1`, every
    // feature the kernel is compiled for.
    unsafe { sha1_kernel(state, blocks) }
}

/// FIPS 180-4 §6.1.2 on the SHA extensions.
///
/// Register layout (Intel's): ABCD holds A in the top lane and D in the
/// bottom; a message quad holds `W[4g]` in the top lane. `sha1rnds4` runs four
/// rounds from ABCD and a quad whose top lane has E added; the E of the next
/// four rounds is A of four rounds ago rotated by 30, which `sha1nexte` adds
/// into the next quad's top lane. The schedule
/// `W[t] = rotl1(W[t−3] ⊕ W[t−8] ⊕ W[t−14] ⊕ W[t−16])` for a quad is
/// `sha1msg2(sha1msg1(Q[g−4], Q[g−3]) ⊕ Q[g−2], Q[g−1])`.
#[target_feature(enable = "sha,ssse3,sse4.1")]
fn sha1_kernel(state: &mut [u32; 5], blocks: &[u8]) {
    // Reverses the sixteen bytes: big-endian words, W[4g] in the top lane.
    let reverse = _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
    let mut abcd = _mm_set_epi32(
        state[0] as i32,
        state[1] as i32,
        state[2] as i32,
        state[3] as i32,
    );
    let mut e = _mm_set_epi32(state[4] as i32, 0, 0, 0);
    let (blocks, _) = blocks.as_chunks::<64>();
    for block in blocks {
        let (quads, _) = block.as_chunks::<16>();
        let (abcd_start, e_start) = (abcd, e);
        let mut w = [
            _mm_shuffle_epi8(load(&quads[0]), reverse),
            _mm_shuffle_epi8(load(&quads[1]), reverse),
            _mm_shuffle_epi8(load(&quads[2]), reverse),
            _mm_shuffle_epi8(load(&quads[3]), reverse),
        ];
        // Rounds 0–3: E comes straight from the state.
        let mut previous = abcd;
        abcd = _mm_sha1rnds4_epu32::<0>(abcd, _mm_add_epi32(e, w[0]));
        // Rounds 4–79, four at a time; the function changes every 20 rounds.
        macro_rules! quads {
            ($($g:literal => $f:literal),* $(,)?) => {$(
                if $g >= 4 {
                    w[$g % 4] = _mm_sha1msg2_epu32(
                        _mm_xor_si128(
                            _mm_sha1msg1_epu32(w[$g % 4], w[($g + 1) % 4]),
                            w[($g + 2) % 4],
                        ),
                        w[($g + 3) % 4],
                    );
                }
                let with_e = _mm_sha1nexte_epu32(previous, w[$g % 4]);
                previous = abcd;
                abcd = _mm_sha1rnds4_epu32::<$f>(abcd, with_e);
            )*};
        }
        quads!(
            1 => 0, 2 => 0, 3 => 0, 4 => 0,
            5 => 1, 6 => 1, 7 => 1, 8 => 1, 9 => 1,
            10 => 2, 11 => 2, 12 => 2, 13 => 2, 14 => 2,
            15 => 3, 16 => 3, 17 => 3, 18 => 3, 19 => 3,
        );
        // The final E is A of four rounds ago rotated by 30; the running E
        // sits in the top lane with zeros below, as `e_start` does.
        e = _mm_sha1nexte_epu32(previous, e_start);
        abcd = _mm_add_epi32(abcd, abcd_start);
    }
    state[0] = _mm_extract_epi32::<3>(abcd) as u32;
    state[1] = _mm_extract_epi32::<2>(abcd) as u32;
    state[2] = _mm_extract_epi32::<1>(abcd) as u32;
    state[3] = _mm_extract_epi32::<0>(abcd) as u32;
    state[4] = _mm_extract_epi32::<3>(e) as u32;
}

// --- CRC-32 ---------------------------------------------------------------

/// The `pclmulqdq` register update, if this processor has `pclmulqdq` and
/// `sse4.1`.
pub(crate) fn crc32_x86_pclmulqdq() -> Option<Crc32Update> {
    (is_x86_feature_detected!("pclmulqdq") && is_x86_feature_detected!("sse4.1"))
        .then_some(crc32_update as Crc32Update)
}

fn crc32_update(register: u32, data: &[u8]) -> u32 {
    if data.len() < 64 {
        return update_portable(register, data);
    }
    // SAFETY: this function escapes the module only through
    // `crc32_x86_pclmulqdq`, which returns it after detecting `pclmulqdq` and
    // `sse4.1`, every feature the kernel is compiled for.
    unsafe { crc32_kernel(register, data) }
}

/// `acc` folded forward by the distance `k` encodes (see [`FOLD`]): its low
/// quadword times `k`'s low, XOR its high quadword times `k`'s high.
#[inline]
#[target_feature(enable = "pclmulqdq")]
fn fold(acc: __m128i, k: __m128i) -> __m128i {
    _mm_xor_si128(
        _mm_clmulepi64_si128::<0x00>(acc, k),
        _mm_clmulepi64_si128::<0x11>(acc, k),
    )
}

/// Folds `data` (at least 64 bytes) down to a 128-bit accumulator, reduces
/// it, and finishes the sub-16-byte tail with the tables.
#[target_feature(enable = "pclmulqdq,sse4.1")]
fn crc32_kernel(register: u32, data: &[u8]) -> u32 {
    let (chunks, tail) = data.as_chunks::<16>();
    debug_assert!(chunks.len() >= 4);
    let by_512 = _mm_set_epi64x(FOLD.by_512[1] as i64, FOLD.by_512[0] as i64);
    let by_128 = _mm_set_epi64x(FOLD.by_128[1] as i64, FOLD.by_128[0] as i64);
    // The register enters as the first 32 message bits.
    let mut acc = [
        _mm_xor_si128(load(&chunks[0]), _mm_cvtsi32_si128(register as i32)),
        load(&chunks[1]),
        load(&chunks[2]),
        load(&chunks[3]),
    ];
    let mut rest = &chunks[4..];
    while let Some((group, after)) = rest.split_first_chunk::<4>() {
        for (lane, chunk) in acc.iter_mut().zip(group) {
            *lane = _mm_xor_si128(fold(*lane, by_512), load(chunk));
        }
        rest = after;
    }
    let mut single = acc[0];
    for lane in &acc[1..] {
        single = _mm_xor_si128(fold(single, by_128), *lane);
    }
    for chunk in rest {
        single = _mm_xor_si128(fold(single, by_128), load(chunk));
    }
    update_portable(reduce(single), tail)
}

/// `F·x^32 mod P` for the 128-bit accumulator F, as a CRC register.
#[target_feature(enable = "pclmulqdq,sse4.1")]
fn reduce(acc: __m128i) -> u32 {
    // F·x^32 = H·x^96 + L·x^32: fold H by x^95 mod P (the product lands at
    // chunk bits 32..128) and add L shifted down 32 chunk bits.
    let to_96 = _mm_cvtsi64_si128(FOLD.to_96 as i64);
    let high_shifted = _mm_srli_si128::<4>(_mm_unpackhi_epi64(_mm_setzero_si128(), acc));
    let g = _mm_xor_si128(_mm_clmulepi64_si128::<0x00>(acc, to_96), high_shifted);
    // G's top 32 coefficients sit in chunk bits 32..64: fold them by
    // x^63 mod P into the high quadword, leaving R of degree below 64.
    let to_64 = _mm_cvtsi64_si128(FOLD.to_64 as i64);
    let r = _mm_extract_epi64::<1>(_mm_xor_si128(_mm_clmulepi64_si128::<0x00>(g, to_64), g)) as u64;
    // Barrett: q = floor(floor(R / x^32)·μ / x^32), then R − q·P.
    let barrett = _mm_set_epi64x(FOLD.polynomial as i64, FOLD.mu as i64);
    let r_high = _mm_cvtsi64_si128((r & 0xFFFF_FFFF) as i64);
    let q = ((_mm_cvtsi128_si64(_mm_clmulepi64_si128::<0x00>(r_high, barrett)) as u64) >> 31)
        & 0xFFFF_FFFF;
    let qp = _mm_clmulepi64_si128::<0x10>(_mm_cvtsi64_si128(q as i64), barrett);
    let qp_low = (_mm_cvtsi128_si64(qp) as u64 >> 63) | ((_mm_extract_epi64::<1>(qp) as u64) << 1);
    ((r >> 32) ^ qp_low) as u32
}

// --- Base16 ---------------------------------------------------------------

/// The SSSE3 base16 encoder, if this processor has `ssse3`.
pub(crate) fn hex_x86_ssse3() -> Option<HexEncode> {
    is_x86_feature_detected!("ssse3").then_some(hex_encode as HexEncode)
}

fn hex_encode(input: &[u8], output: &mut [u8], upper: bool) {
    // SAFETY: this function escapes the module only through `hex_x86_ssse3`,
    // which returns it after detecting `ssse3`, the one feature the kernel is
    // compiled for beyond the SSE2 baseline.
    unsafe { hex_kernel(input, output, upper) }
}

/// Sixteen input bytes per step: the high and low nibbles of every byte,
/// each looked up in the alphabet with `pshufb`, then interleaved high-first
/// into thirty-two characters. The tail after the last whole step is
/// encoded by the portable compare-select loop.
#[target_feature(enable = "ssse3")]
fn hex_kernel(input: &[u8], output: &mut [u8], upper: bool) {
    let alphabet = load(if upper { UPPER_ALPHABET } else { ALPHABET });
    let low_nibble = _mm_set1_epi8(0x0f);
    let (chunks, tail) = input.as_chunks::<16>();
    let (pairs, _) = output.as_chunks_mut::<32>();
    for (chunk, pair) in chunks.iter().zip(pairs.iter_mut()) {
        let bytes = load(chunk);
        // A 16-bit shift moves the neighbouring byte's low bits into each
        // byte's top nibble; the mask discards them.
        let high = _mm_and_si128(_mm_srli_epi16::<4>(bytes), low_nibble);
        let low = _mm_and_si128(bytes, low_nibble);
        let high = _mm_shuffle_epi8(alphabet, high);
        let low = _mm_shuffle_epi8(alphabet, low);
        let (halves, _) = pair.as_chunks_mut::<16>();
        store(&mut halves[0], _mm_unpacklo_epi8(high, low));
        store(&mut halves[1], _mm_unpackhi_epi8(high, low));
    }
    let done = chunks.len() * 16;
    encode_portable(tail, &mut output[2 * done..], upper);
}

// --- The fixed hasher's AES round ------------------------------------------

/// A 128-bit block for the fixed hasher's AES path, byte `i` in lane `i`.
///
/// Compiled only when the build's target enables `aes`
/// (`-C target-feature=+aes`, or a `target-cpu` that has it), so every
/// intrinsic below runs on a processor the whole build already requires; no
/// run-time detection is involved and no other path exists in such a build.
#[cfg(target_feature = "aes")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Block(__m128i);

// SAFETY (every `unsafe` block below): the intrinsics need SSE2, part of the
// x86-64 baseline, and AES, which this item's `cfg(target_feature = "aes")`
// proves the build's target enables for every function in the crate. Rust
// still asks for an `unsafe` block because these functions carry no
// `#[target_feature]` attribute of their own: adding one would make every
// caller outside this module `unsafe` in turn.
#[cfg(target_feature = "aes")]
impl Block {
    /// The block whose low eight bytes are `low` and high eight are `high`,
    /// both little-endian.
    #[inline]
    pub(crate) fn from_words(low: u64, high: u64) -> Self {
        // SAFETY: see the impl comment.
        Self(unsafe { _mm_set_epi64x(high as i64, low as i64) })
    }

    /// Sixteen bytes, in order.
    #[inline]
    pub(crate) fn load(bytes: &[u8; 16]) -> Self {
        Self(load(bytes))
    }

    /// One AES encryption round (ShiftRows, SubBytes, MixColumns) followed
    /// by XOR with `key`.
    #[inline]
    pub(crate) fn round(self, key: Self) -> Self {
        // SAFETY: see the impl comment.
        Self(unsafe { _mm_aesenc_si128(self.0, key.0) })
    }

    /// Lane-wise XOR.
    #[inline]
    pub(crate) fn xor(self, other: Self) -> Self {
        // SAFETY: see the impl comment.
        Self(unsafe { _mm_xor_si128(self.0, other.0) })
    }

    /// The low and high eight bytes as little-endian words.
    #[inline]
    pub(crate) fn words(self) -> (u64, u64) {
        // SAFETY: see the impl comment.
        unsafe {
            let low = _mm_cvtsi128_si64(self.0) as u64;
            let high = _mm_cvtsi128_si64(_mm_unpackhi_epi64(self.0, self.0)) as u64;
            (low, high)
        }
    }
}
