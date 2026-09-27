// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! x86_64 kernels: SSE2 (the architecture baseline) and AVX2 (detected at run
//! time).

use core::arch::x86_64::{
    _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_storeu_si128,
    _mm256_broadcastsi128_si256, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8,
    _mm256_mullo_epi32, _mm256_set1_epi32, _mm256_setr_epi8, _mm256_shuffle_epi8,
    _mm256_srli_epi32, _mm256_storeu_si256,
};

use super::VectorKernels;
use crate::kernels::{
    HASH_BITS, HASH_MULTIPLIER, check_copy, hash_windows_portable, match_length_portable,
};

const _: () = assert!(HASH_BITS == 15, "the AVX2 hash shifts by 32 - 15");

/// SSE2 kernels. SSE2 is part of the x86_64 baseline; detection confirms it.
pub(crate) fn sse2() -> Option<VectorKernels> {
    std::arch::is_x86_feature_detected!("sse2").then_some(VectorKernels {
        copy: copy_sse2,
        match_length: match_length_sse2,
        // SSE2 has neither a byte shuffle nor a 32-bit lane multiply.
        hash: None,
    })
}

/// AVX2 kernels, when the processor reports AVX2.
pub(crate) fn avx2() -> Option<VectorKernels> {
    std::arch::is_x86_feature_detected!("avx2").then_some(VectorKernels {
        copy: copy_avx2,
        match_length: match_length_avx2,
        hash: Some(hash_avx2),
    })
}

/// The period-doubling prefix of a copy whose distance is shorter than the
/// vector width `width`: produces the first `min(len, g)` bytes, where `g` is
/// the smallest multiple of `dist` that is at least `width`, and returns `g`.
fn prefill(buf: &mut [u8], dst: usize, dist: usize, len: usize, width: usize) -> usize {
    if dist >= width {
        return dist;
    }
    let period = dist * width.div_ceil(dist);
    for i in dst..dst + period.min(len) {
        buf[i] = buf[i - dist];
    }
    period
}

fn copy_sse2(buf: &mut [u8], dst: usize, dist: usize, len: usize) {
    check_copy(buf, dst, dist, len);
    let period = prefill(buf, dst, dist, len, 16);
    let start = if dist >= 16 { dst } else { dst + period };
    if start >= dst + len {
        return;
    }
    // SAFETY: SSE2 was confirmed by `sse2()` before this pointer was handed
    // out. `check_copy` proved `dst + len + COPY_SLACK <= buf.len()`; every
    // store starts below `dst + len` and writes 16 < COPY_SLACK bytes, and
    // every load reads `[d - period, d - period + 16)` with `period >= 16`
    // and `d - period >= dst - dist >= 0`, so all accesses are in `buf`.
    unsafe { copy_chunks_sse2(buf.as_mut_ptr(), start, dst + len, period) }
}

#[target_feature(enable = "sse2")]
unsafe fn copy_chunks_sse2(base: *mut u8, mut d: usize, end: usize, period: usize) {
    while d < end {
        // SAFETY: in bounds by the caller's argument; the source range ends at
        // or before `d`, so it holds only bytes already final.
        unsafe {
            let v = _mm_loadu_si128(base.add(d - period).cast());
            _mm_storeu_si128(base.add(d).cast(), v);
        }
        d += 16;
    }
}

fn copy_avx2(buf: &mut [u8], dst: usize, dist: usize, len: usize) {
    check_copy(buf, dst, dist, len);
    let period = prefill(buf, dst, dist, len, 32);
    let start = if dist >= 32 { dst } else { dst + period };
    if start >= dst + len {
        return;
    }
    // SAFETY: AVX2 was confirmed by `avx2()`. As for SSE2, with 32-byte
    // accesses: every store starts below `dst + len` and writes 32 <=
    // COPY_SLACK bytes; every load ends at or before its store's start.
    unsafe { copy_chunks_avx2(buf.as_mut_ptr(), start, dst + len, period) }
}

#[target_feature(enable = "avx2")]
unsafe fn copy_chunks_avx2(base: *mut u8, mut d: usize, end: usize, period: usize) {
    while d < end {
        // SAFETY: in bounds by the caller's argument; the source range ends at
        // or before `d`.
        unsafe {
            let v = _mm256_loadu_si256(base.add(d - period).cast());
            _mm256_storeu_si256(base.add(d).cast(), v);
        }
        d += 32;
    }
}

fn match_length_sse2(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let whole = n - n % 16;
    // SAFETY: SSE2 confirmed by `sse2()`; the kernel reads only
    // `[0, whole)` of both slices, and `whole <= n` for each.
    let at = unsafe { prefix_sse2(a.as_ptr(), b.as_ptr(), whole) };
    if at < whole {
        return at;
    }
    whole + match_length_portable(&a[whole..n], &b[whole..n])
}

#[target_feature(enable = "sse2")]
unsafe fn prefix_sse2(a: *const u8, b: *const u8, whole: usize) -> usize {
    let mut i = 0;
    while i < whole {
        // SAFETY: `i + 16 <= whole`, which the caller bounds by both lengths.
        let (x, y) = unsafe {
            (
                _mm_loadu_si128(a.add(i).cast()),
                _mm_loadu_si128(b.add(i).cast()),
            )
        };
        let equal = _mm_movemask_epi8(_mm_cmpeq_epi8(x, y)) as u32;
        if equal != 0xFFFF {
            return i + (!equal).trailing_zeros() as usize;
        }
        i += 16;
    }
    whole
}

fn match_length_avx2(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let whole = n - n % 32;
    // SAFETY: AVX2 confirmed by `avx2()`; reads only `[0, whole)`.
    let at = unsafe { prefix_avx2(a.as_ptr(), b.as_ptr(), whole) };
    if at < whole {
        return at;
    }
    whole + match_length_portable(&a[whole..n], &b[whole..n])
}

#[target_feature(enable = "avx2")]
unsafe fn prefix_avx2(a: *const u8, b: *const u8, whole: usize) -> usize {
    let mut i = 0;
    while i < whole {
        // SAFETY: `i + 32 <= whole`, bounded by both lengths.
        let (x, y) = unsafe {
            (
                _mm256_loadu_si256(a.add(i).cast()),
                _mm256_loadu_si256(b.add(i).cast()),
            )
        };
        let equal = _mm256_movemask_epi8(_mm256_cmpeq_epi8(x, y)) as u32;
        if equal != u32::MAX {
            return i + (!equal).trailing_zeros() as usize;
        }
        i += 32;
    }
    whole
}

fn hash_avx2(data: &[u8], start: usize, out: &mut [u32]) {
    assert!(
        start + out.len() + 3 <= data.len(),
        "hash windows out of bounds"
    );
    let mut i = 0;
    while i + 8 <= out.len() && start + i + 16 <= data.len() {
        // SAFETY: AVX2 confirmed by `avx2()`; the load reads
        // `data[start + i .. start + i + 16]` and the store writes
        // `out[i .. i + 8]`, both checked by the loop condition.
        unsafe { hash8_avx2(data.as_ptr().add(start + i), out.as_mut_ptr().add(i)) };
        i += 8;
    }
    hash_windows_portable(data, start + i, &mut out[i..]);
}

#[target_feature(enable = "avx2")]
unsafe fn hash8_avx2(src: *const u8, dst: *mut u32) {
    // SAFETY: 16 readable bytes at `src`, by the caller.
    let bytes = unsafe { _mm_loadu_si128(src.cast()) };
    let both = _mm256_broadcastsi128_si256(bytes);
    // Lane k (0..8) gathers the window starting at byte k; the low half's
    // windows start at 0..4, the high half's at 4..8, each within its own
    // 128-bit copy of the 16 loaded bytes.
    let index = _mm256_setr_epi8(
        0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 7, 8, 6, 7, 8, 9, 7, 8,
        9, 10,
    );
    let windows = _mm256_shuffle_epi8(both, index);
    let product = _mm256_mullo_epi32(windows, _mm256_set1_epi32(HASH_MULTIPLIER as i32));
    let hashes = _mm256_srli_epi32::<17>(product);
    // SAFETY: 8 writable `u32` at `dst`, by the caller.
    unsafe { _mm256_storeu_si256(dst.cast(), hashes) };
}
