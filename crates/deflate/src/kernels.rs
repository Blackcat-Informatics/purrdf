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

/// The length of the common prefix of `a` and `b`: the index of the first
/// byte at which they differ, or the shorter length when one is a prefix of
/// the other.
///
/// Word-parallel: both slices are walked in sixteen-byte words, and a word
/// pair is compared as one `u128` XOR. A zero XOR is sixteen shared bytes; the
/// first non-zero XOR ends the prefix, and because the words are read
/// little-endian the first differing byte is the lowest non-zero byte of the
/// XOR, so its offset in the word is the XOR's trailing-zero count over 8. The
/// bytes after the last whole word pair are one more word pair ending at the
/// shorter length, overlapping the one before it; a key shorter than sixteen
/// bytes is two overlapping eight-byte words, and only one shorter than eight
/// is compared one byte at a time. Sixteen-byte words were measured against
/// eight on the canonical order of 40-byte IRIs, where the comparison's loop
/// overhead, not its loads, was the difference.
///
/// This is the workspace's one first-mismatch index. It is `#[inline]` with no
/// dispatch, so a short key (a front-coded dictionary record, a few dozen
/// bytes) compiles into its caller's loop with no indirect call, and it is the
/// portable path of the encoder's match-length kernel and the tail of every
/// vector path after its last whole vector.
#[inline]
#[must_use]
pub fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let len = a.len().min(b.len());
    let (a, b) = (&a[..len], &b[..len]);
    let (a_words, _) = a.as_chunks::<16>();
    let (b_words, _) = b.as_chunks::<16>();
    for (k, (x, y)) in a_words.iter().zip(b_words).enumerate() {
        let diff = u128::from_le_bytes(*x) ^ u128::from_le_bytes(*y);
        if diff != 0 {
            return k * 16 + (diff.trailing_zeros() / u8::BITS) as usize;
        }
    }
    let done = a_words.len() * 16;
    if done == len {
        return len;
    }
    // The bytes after the last whole word are the end of one more word ending
    // at `len`, overlapping the one before it: the bytes it shares with the
    // words before it are equal, so its first difference is the first
    // difference. A key shorter than sixteen bytes takes the eight-byte form.
    if len >= 16 {
        let last = len - 16;
        let word = |s: &[u8]| u128::from_le_bytes(*s[last..].first_chunk().expect("sixteen bytes"));
        let diff = word(a) ^ word(b);
        return if diff == 0 {
            len
        } else {
            last + (diff.trailing_zeros() / u8::BITS) as usize
        };
    }
    let word =
        |s: &[u8], at: usize| u64::from_le_bytes(*s[at..].first_chunk().expect("eight bytes"));
    if len >= 8 {
        for at in [0, len - 8] {
            let diff = word(a, at) ^ word(b, at);
            if diff != 0 {
                return at + (diff.trailing_zeros() / u8::BITS) as usize;
            }
        }
        return len;
    }
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

/// Portable match length: [`common_prefix_len`] out of line, at the stable
/// path the asm audit measures.
#[inline(never)]
pub(crate) fn match_length_portable(a: &[u8], b: &[u8]) -> usize {
    common_prefix_len(a, b)
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

#[cfg(test)]
mod tests {
    use super::common_prefix_len;

    /// Every length through three sixteen-byte words, a single difference at
    /// every position (and none), both argument orders and unequal lengths:
    /// the sixteen-byte words, the overlapping last word, the two eight-byte
    /// words of a short key and the byte walk of a shorter one all answer the
    /// first differing index.
    #[test]
    fn every_path_answers_the_first_difference() {
        for len in 0..=48_usize {
            let base: Vec<u8> = (0..len).map(|i| (i * 7 % 251) as u8).collect();
            for at in 0..=len {
                let mut other = base.clone();
                if at < len {
                    other[at] ^= 0x5A;
                }
                assert_eq!(common_prefix_len(&base, &other), at, "len {len} at {at}");
                assert_eq!(common_prefix_len(&other, &base), at, "len {len} at {at}");
                let longer = [other.as_slice(), &[1, 2, 3]].concat();
                assert_eq!(common_prefix_len(&base, &longer), at, "len {len} at {at}");
            }
        }
    }
}
