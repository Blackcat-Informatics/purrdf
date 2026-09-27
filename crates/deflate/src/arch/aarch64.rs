// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! aarch64 NEON kernels (NEON is the architecture baseline; detection
//! confirms it).

use core::arch::aarch64::{
    vceqq_u8, vget_lane_u64, vld1q_u8, vmulq_n_u32, vqtbl1q_u8, vreinterpret_u64_u8,
    vreinterpretq_u16_u8, vreinterpretq_u32_u8, vshrn_n_u16, vshrq_n_u32, vst1q_u8, vst1q_u32,
};

use super::VectorKernels;
use crate::kernels::{
    HASH_BITS, HASH_MULTIPLIER, check_copy, hash_windows_portable, match_length_portable,
};

const _: () = assert!(HASH_BITS == 15, "the NEON hash shifts by 32 - 15");

/// NEON kernels.
pub(crate) fn neon() -> Option<VectorKernels> {
    std::arch::is_aarch64_feature_detected!("neon").then_some(VectorKernels {
        copy: copy_neon,
        match_length: match_length_neon,
        hash: Some(hash_neon),
    })
}

fn copy_neon(buf: &mut [u8], dst: usize, dist: usize, len: usize) {
    check_copy(buf, dst, dist, len);
    let period = if dist >= 16 {
        dist
    } else {
        let period = dist * 16usize.div_ceil(dist);
        for i in dst..dst + period.min(len) {
            buf[i] = buf[i - dist];
        }
        period
    };
    let start = if dist >= 16 { dst } else { dst + period };
    if start >= dst + len {
        return;
    }
    // SAFETY: NEON confirmed by `neon()`. `check_copy` proved
    // `dst + len + COPY_SLACK <= buf.len()`; each store starts below
    // `dst + len` and writes 16 bytes, each load reads 16 bytes ending at or
    // before its store's start, at an offset `>= dst - dist >= 0`.
    unsafe { copy_chunks_neon(buf.as_mut_ptr(), start, dst + len, period) }
}

#[target_feature(enable = "neon")]
unsafe fn copy_chunks_neon(base: *mut u8, mut d: usize, end: usize, period: usize) {
    while d < end {
        // SAFETY: in bounds by the caller's argument.
        unsafe {
            let v = vld1q_u8(base.add(d - period));
            vst1q_u8(base.add(d), v);
        }
        d += 16;
    }
}

fn match_length_neon(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let whole = n - n % 16;
    // SAFETY: NEON confirmed by `neon()`; reads only `[0, whole)` of both.
    let at = unsafe { prefix_neon(a.as_ptr(), b.as_ptr(), whole) };
    if at < whole {
        return at;
    }
    whole + match_length_portable(&a[whole..n], &b[whole..n])
}

#[target_feature(enable = "neon")]
unsafe fn prefix_neon(a: *const u8, b: *const u8, whole: usize) -> usize {
    let mut i = 0;
    while i < whole {
        // SAFETY: `i + 16 <= whole`, bounded by both lengths.
        let (x, y) = unsafe { (vld1q_u8(a.add(i)), vld1q_u8(b.add(i))) };
        let equal = vceqq_u8(x, y);
        // Narrow each 16-bit pair of 0x00/0xFF bytes by 4: one nibble per
        // byte, so byte k of the comparison is bits 4k..4k+4 of the mask.
        let mask = vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(vreinterpretq_u16_u8(
            equal,
        ))));
        if mask != u64::MAX {
            return i + ((!mask).trailing_zeros() / 4) as usize;
        }
        i += 16;
    }
    whole
}

fn hash_neon(data: &[u8], start: usize, out: &mut [u32]) {
    assert!(
        start + out.len() + 3 <= data.len(),
        "hash windows out of bounds"
    );
    let mut i = 0;
    while i + 4 <= out.len() && start + i + 16 <= data.len() {
        // SAFETY: NEON confirmed by `neon()`; reads
        // `data[start + i .. start + i + 16]`, writes `out[i .. i + 4]`.
        unsafe { hash4_neon(data.as_ptr().add(start + i), out.as_mut_ptr().add(i)) };
        i += 4;
    }
    hash_windows_portable(data, start + i, &mut out[i..]);
}

#[target_feature(enable = "neon")]
unsafe fn hash4_neon(src: *const u8, dst: *mut u32) {
    const INDEX: [u8; 16] = [0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6];
    // SAFETY: 16 readable bytes at `src`, by the caller; INDEX is a local
    // 16-byte array.
    let (bytes, index) = unsafe { (vld1q_u8(src), vld1q_u8(INDEX.as_ptr())) };
    let windows = vreinterpretq_u32_u8(vqtbl1q_u8(bytes, index));
    let hashes = vshrq_n_u32::<17>(vmulq_n_u32(windows, HASH_MULTIPLIER));
    // SAFETY: 4 writable `u32` at `dst`, by the caller.
    unsafe { vst1q_u32(dst, hashes) };
}
