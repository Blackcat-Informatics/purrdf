// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! wasm32 kernels, compiled only when `simd128` is enabled for the build
//! (wasm has no run-time feature detection): base16 encoding on
//! `i8x16.swizzle`.

use core::arch::wasm32::{
    i8x16_swizzle, u8x16_shr, u8x16_shuffle, u8x16_splat, v128, v128_and, v128_load, v128_store,
};

use super::HexEncode;
use crate::hex::{ALPHABET, encode_portable};

/// Sixteen bytes as a vector, byte `i` in lane `i`.
#[inline]
fn load(bytes: &[u8; 16]) -> v128 {
    // SAFETY: `bytes` is sixteen readable bytes, and `v128_load` performs a
    // 1-aligned load.
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "`v128_load` performs a 1-aligned load; the pointer type is its signature's"
    )]
    unsafe {
        v128_load(bytes.as_ptr().cast::<v128>())
    }
}

/// Writes a vector's sixteen lanes, lane `i` to byte `i`.
#[inline]
fn store(bytes: &mut [u8; 16], vector: v128) {
    // SAFETY: `bytes` is sixteen writable bytes, and `v128_store` performs a
    // 1-aligned store.
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "`v128_store` performs a 1-aligned store; the pointer type is its signature's"
    )]
    unsafe {
        v128_store(bytes.as_mut_ptr().cast::<v128>(), vector);
    }
}

/// The `simd128` base16 encoder; always present in a `simd128` build.
#[allow(
    clippy::unnecessary_wraps,
    reason = "every accessor answers `Option`; the other targets' can be `None`"
)]
pub(crate) const fn hex_wasm32_simd128() -> Option<HexEncode> {
    Some(hex_encode as HexEncode)
}

/// Sixteen input bytes per step: the high and low nibbles of every byte,
/// each looked up in the alphabet with `i8x16.swizzle`, then interleaved
/// high-first into thirty-two characters. The tail after the last whole step
/// is encoded by the portable table.
fn hex_encode(input: &[u8], output: &mut [u8]) {
    let alphabet = load(ALPHABET);
    let low_nibble = u8x16_splat(0x0f);
    let (chunks, tail) = input.as_chunks::<16>();
    let (pairs, _) = output.as_chunks_mut::<32>();
    for (chunk, pair) in chunks.iter().zip(pairs.iter_mut()) {
        let bytes = load(chunk);
        let high = i8x16_swizzle(alphabet, u8x16_shr(bytes, 4));
        let low = i8x16_swizzle(alphabet, v128_and(bytes, low_nibble));
        let (halves, _) = pair.as_chunks_mut::<16>();
        store(
            &mut halves[0],
            u8x16_shuffle::<0, 16, 1, 17, 2, 18, 3, 19, 4, 20, 5, 21, 6, 22, 7, 23>(high, low),
        );
        store(
            &mut halves[1],
            u8x16_shuffle::<8, 24, 9, 25, 10, 26, 11, 27, 12, 28, 13, 29, 14, 30, 15, 31>(
                high, low,
            ),
        );
    }
    let done = chunks.len() * 16;
    encode_portable(tail, &mut output[2 * done..]);
}
