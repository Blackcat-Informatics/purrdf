// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! wasm32 simd128 kernels, compiled only when the build enables `+simd128`
//! (WebAssembly has no run-time feature detection: a module either validates
//! with SIMD instructions or does not load).

use core::arch::wasm32::{
    i8x16, i32x4_mul, u8x16_bitmask, u8x16_eq, u8x16_swizzle, u32x4_shr, u32x4_splat, v128_load,
    v128_store,
};

use super::VectorKernels;
use crate::kernels::{
    HASH_BITS, HASH_MULTIPLIER, check_copy, hash_windows_portable, match_length_portable,
};

/// simd128 kernels; the build's `+simd128` is the whole of the detection.
pub(crate) fn simd128() -> Option<VectorKernels> {
    Some(VectorKernels {
        copy: copy_simd128,
        match_length: match_length_simd128,
        hash: Some(hash_simd128),
    })
}

fn copy_simd128(buf: &mut [u8], dst: usize, dist: usize, len: usize) {
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
    let mut d = if dist >= 16 { dst } else { dst + period };
    let end = dst + len;
    let base = buf.as_mut_ptr();
    while d < end {
        // SAFETY: `check_copy` proved `dst + len + COPY_SLACK <= buf.len()`;
        // the store starts below `dst + len` and writes 16 bytes, the load
        // reads 16 bytes ending at or before `d` from an offset
        // `>= dst - dist >= 0`. wasm has no alignment requirement for v128.
        unsafe {
            let v = v128_load(base.add(d - period).cast());
            v128_store(base.add(d).cast(), v);
        }
        d += 16;
    }
}

fn match_length_simd128(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut i = 0;
    while i + 16 <= n {
        // SAFETY: `i + 16 <= n`, bounded by both lengths.
        let (x, y) = unsafe {
            (
                v128_load(a.as_ptr().add(i).cast()),
                v128_load(b.as_ptr().add(i).cast()),
            )
        };
        let equal = u32::from(u8x16_bitmask(u8x16_eq(x, y)));
        if equal != 0xFFFF {
            return i + (!equal).trailing_zeros() as usize;
        }
        i += 16;
    }
    i + match_length_portable(&a[i..n], &b[i..n])
}

fn hash_simd128(data: &[u8], start: usize, out: &mut [u32]) {
    assert!(
        start + out.len() + 3 <= data.len(),
        "hash windows out of bounds"
    );
    let index = i8x16(0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6);
    let multiplier = u32x4_splat(HASH_MULTIPLIER);
    let mut i = 0;
    while i + 4 <= out.len() && start + i + 16 <= data.len() {
        // SAFETY: reads `data[start + i .. start + i + 16]`, checked above.
        let bytes = unsafe { v128_load(data.as_ptr().add(start + i).cast()) };
        let windows = u8x16_swizzle(bytes, index);
        let hashes = u32x4_shr(i32x4_mul(windows, multiplier), 32 - HASH_BITS);
        // SAFETY: writes `out[i .. i + 4]`, checked above.
        unsafe { v128_store(out.as_mut_ptr().add(i).cast(), hashes) };
        i += 4;
    }
    hash_windows_portable(data, start + i, &mut out[i..]);
}
