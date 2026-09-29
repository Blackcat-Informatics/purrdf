// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The three hot kernels, their portable definitions, and the contract every
//! vector version in `crate::arch` is held to.
//!
//! Every kernel is exact: a vector path returns what the portable path returns
//! for every input, so the decoder's output and the encoder's bytes do not
//! depend on which path ran. The differential tests hold each available path
//! to the portable one.
//!
//! The portable kernels are `#[inline(never)]`, as the vector ones in
//! `crate::arch` are: each is one out-of-line function at a stable path, the
//! symbol the asm audit measures, rather than a body inlined into a vector
//! path's tail in some builds and not in others.

/// Bytes a match copy may write past the end of the match. The decoder keeps
/// this much writable room after every position it copies to.
pub(crate) const COPY_SLACK: usize = 32;

/// Bits of the encoder's hash.
pub(crate) const HASH_BITS: u32 = 15;
/// The multiplier of the 4-byte-window hash (the odd integer nearest
/// 2^32 / φ).
pub(crate) const HASH_MULTIPLIER: u32 = 0x9E37_79B1;

/// Copy `len` bytes to `buf[dst..]` from `dist` bytes earlier, byte by byte in
/// effect (a distance shorter than the length repeats the last `dist` bytes).
/// May overwrite `buf[dst + len .. dst + len + COPY_SLACK]`.
pub(crate) type CopyMatch = fn(&mut [u8], usize, usize, usize);
/// The length of the common prefix of two equal-length slices.
pub(crate) type MatchLength = fn(&[u8], &[u8]) -> usize;
/// `out[i]` = the hash of the 4-byte window at `data[start + i]`, for every
/// `i < out.len()`; `start + out.len() + 3 <= data.len()`.
pub(crate) type HashWindows = fn(&[u8], usize, &mut [u32]);

/// Checks the copy contract; every path calls it before touching memory.
#[inline]
pub(crate) fn check_copy(buf: &[u8], dst: usize, dist: usize, len: usize) {
    assert!(
        dist >= 1 && dist <= dst && dst + len + COPY_SLACK <= buf.len(),
        "match copy out of bounds"
    );
}

/// The hash of one little-endian 4-byte window.
#[inline]
pub(crate) const fn hash4(window: u32) -> u32 {
    window.wrapping_mul(HASH_MULTIPLIER) >> (32 - HASH_BITS)
}

/// Portable match copy: non-overlapping runs, doubling the run while the
/// distance is shorter than what is left.
#[inline(never)]
pub(crate) fn copy_match_portable(buf: &mut [u8], dst: usize, dist: usize, len: usize) {
    check_copy(buf, dst, dist, len);
    let src = dst - dist;
    if dist >= len {
        buf.copy_within(src..src + len, dst);
        return;
    }
    // [src, cursor) is periodic with period `dist` and its length grows each
    // step, so each copy_within reads only bytes already final.
    let mut cursor = dst;
    let end = dst + len;
    while cursor < end {
        let run = (cursor - src).min(end - cursor);
        buf.copy_within(src..src + run, cursor);
        cursor += run;
    }
}

/// Portable match length: eight bytes at a time by XOR and trailing zeros.
#[inline(never)]
pub(crate) fn match_length_portable(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    let mut i = 0;
    while i + 8 <= n {
        let x = u64::from_le_bytes(*a[i..].first_chunk().expect("eight bytes"));
        let y = u64::from_le_bytes(*b[i..].first_chunk().expect("eight bytes"));
        let diff = x ^ y;
        if diff != 0 {
            return i + (diff.trailing_zeros() / 8) as usize;
        }
        i += 8;
    }
    while i < n && a[i] == b[i] {
        i += 1;
    }
    i
}

/// Portable window hashing.
#[inline(never)]
pub(crate) fn hash_windows_portable(data: &[u8], start: usize, out: &mut [u32]) {
    assert!(
        start + out.len() + 3 <= data.len(),
        "hash windows out of bounds"
    );
    for (i, slot) in out.iter_mut().enumerate() {
        let p = start + i;
        let w = u32::from_le_bytes(*data[p..].first_chunk().expect("four bytes"));
        *slot = hash4(w);
    }
}
