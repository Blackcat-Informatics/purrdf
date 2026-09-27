// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The explicit vector compilations of the field scanner, and the choice
//! between them. The only module of [`purrdf_core::csv`](super) that holds
//! `unsafe`.
//!
//! Every kernel answers what [`find_portable`] answers — the offset of the
//! first byte of the haystack equal to one of the [`StopSet`]'s needles — by
//! the same shape: load one vector of haystack bytes, compare it for equality
//! with each of the [`LANES`] broadcast needles, OR the comparisons, extract a
//! lane mask, and take its trailing-zero count in the first chunk whose mask is
//! non-zero. The bytes after the last whole vector are answered by the set's
//! table. Needles are padded by repetition, so a padded lane can only repeat an
//! answer, never add one.
//!
//! | target | kernel | selected |
//! |---|---|---|
//! | `x86_64` | [`x86::find_avx2`], 32-byte chunks | at run time, when the processor reports AVX2 |
//! | `x86_64` | [`x86::find_sse2`], 16-byte chunks | otherwise (SSE2 is the `x86_64` baseline) |
//! | `aarch64` | [`neon::find`], 16-byte chunks | always (NEON is the `aarch64` baseline) |
//! | `wasm32` + `simd128` | [`simd128::find`], 16-byte chunks | at compile time |
//! | anything else | [`find_portable`] | always |
//!
//! Each kernel is `#[inline(never)]`, so each compiles to one out-of-line
//! symbol, which is also the symbol the SIMD evidence gate measures.

// The kernels are `pub(crate)` inside private modules, so the links above are
// private-item links by construction.
#![allow(
    rustdoc::private_intra_doc_links,
    reason = "the module table documents the crate-private kernels it dispatches between"
)]

#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    all(target_arch = "wasm32", target_feature = "simd128")
))]
use super::scan::LANES;
use super::scan::StopSet;
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    all(target_arch = "wasm32", target_feature = "simd128")
)))]
use super::scan::find_portable;

/// The widest kernel this process can run, applied to `haystack`.
#[inline]
pub(crate) fn find(set: &StopSet, haystack: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: `find_avx2` is compiled with `avx2` enabled, and the
            // processor has just reported AVX2, so every instruction it can
            // execute exists on this processor.
            return unsafe { x86::find_avx2(set, haystack) };
        }
        // SAFETY: SSE2 is part of the `x86_64` baseline, so every `x86_64`
        // processor executes `find_sse2`.
        unsafe { x86::find_sse2(set, haystack) }
    }
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: NEON is part of the `aarch64` baseline, so every `aarch64`
        // processor executes `neon::find`.
        unsafe { neon::find(set, haystack) }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        simd128::find(set, haystack)
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        find_portable(set, haystack)
    }
}

/// Every explicit kernel this process can run, by name, for the differential
/// tests against the portable kernel.
#[cfg(test)]
pub(crate) fn every_kernel_for_tests() -> Vec<(&'static str, super::scan::Kernel)> {
    #[cfg_attr(
        not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            all(target_arch = "wasm32", target_feature = "simd128")
        )),
        allow(unused_mut, reason = "a target without an explicit kernel pushes none")
    )]
    let mut kernels: Vec<(&'static str, super::scan::Kernel)> = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        kernels.push(("sse2", |set, haystack| {
            // SAFETY: SSE2 is part of the `x86_64` baseline.
            unsafe { x86::find_sse2(set, haystack) }
        }));
        if std::is_x86_feature_detected!("avx2") {
            kernels.push(("avx2", |set, haystack| {
                // SAFETY: registered only after the processor reported AVX2.
                unsafe { x86::find_avx2(set, haystack) }
            }));
        }
    }
    #[cfg(target_arch = "aarch64")]
    kernels.push(("neon", |set, haystack| {
        // SAFETY: NEON is part of the `aarch64` baseline.
        unsafe { neon::find(set, haystack) }
    }));
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    kernels.push(("simd128", simd128::find));
    kernels
}

/// SSE2 and AVX2.
#[cfg(target_arch = "x86_64")]
pub(crate) mod x86 {
    use std::arch::x86_64::{
        __m128i, __m256i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
        _mm_set1_epi8, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8,
        _mm256_or_si256, _mm256_set1_epi8,
    };

    use super::{LANES, StopSet};

    /// Sixteen bytes per chunk: `pcmpeqb` per needle, `por`, `pmovmskb`.
    ///
    /// Compiled with `sse2` enabled, which every `x86_64` processor has; the
    /// attribute is what makes the intrinsics callable, and it makes a call
    /// from a context without it `unsafe`.
    #[target_feature(enable = "sse2")]
    #[inline(never)]
    pub(crate) fn find_sse2(set: &StopSet, haystack: &[u8]) -> Option<usize> {
        if !set.vectorizable() {
            return set.find_in_tail(haystack);
        }
        let needles: [__m128i; LANES] =
            std::array::from_fn(|lane| _mm_set1_epi8(set.needles()[lane] as i8));
        let (chunks, tail) = haystack.as_chunks::<16>();
        for (k, chunk) in chunks.iter().enumerate() {
            // SAFETY: `chunk` is a `&[u8; 16]`, so the pointer is valid for a
            // sixteen-byte read, and `loadu` has no alignment requirement.
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "`_mm_loadu_si128` is the unaligned load; the pointer type is its signature's"
            )]
            let bytes = unsafe { _mm_loadu_si128(chunk.as_ptr().cast::<__m128i>()) };
            let mut hits = _mm_cmpeq_epi8(bytes, needles[0]);
            for &needle in &needles[1..] {
                hits = _mm_or_si128(hits, _mm_cmpeq_epi8(bytes, needle));
            }
            let mask = _mm_movemask_epi8(hits) as u32;
            if mask != 0 {
                return Some(k * 16 + mask.trailing_zeros() as usize);
            }
        }
        set.find_in_tail(tail)
            .map(|offset| chunks.len() * 16 + offset)
    }

    /// Thirty-two bytes per chunk: `vpcmpeqb` per needle, `vpor`,
    /// `vpmovmskb`; a remainder of sixteen bytes or more goes to
    /// [`find_sse2`].
    ///
    /// Calling it is `unsafe` outside an AVX2 context: the processor must
    /// support AVX2.
    #[target_feature(enable = "avx2")]
    #[inline(never)]
    pub(crate) fn find_avx2(set: &StopSet, haystack: &[u8]) -> Option<usize> {
        if !set.vectorizable() {
            return set.find_in_tail(haystack);
        }
        let needles: [__m256i; LANES] =
            std::array::from_fn(|lane| _mm256_set1_epi8(set.needles()[lane] as i8));
        let (chunks, tail) = haystack.as_chunks::<32>();
        for (k, chunk) in chunks.iter().enumerate() {
            // SAFETY: `chunk` is a `&[u8; 32]`, so the pointer is valid for a
            // thirty-two-byte read, and `loadu` has no alignment requirement.
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "`_mm256_loadu_si256` is the unaligned load; the pointer type is its signature's"
            )]
            let bytes = unsafe { _mm256_loadu_si256(chunk.as_ptr().cast::<__m256i>()) };
            let mut hits = _mm256_cmpeq_epi8(bytes, needles[0]);
            for &needle in &needles[1..] {
                hits = _mm256_or_si256(hits, _mm256_cmpeq_epi8(bytes, needle));
            }
            let mask = _mm256_movemask_epi8(hits) as u32;
            if mask != 0 {
                return Some(k * 32 + mask.trailing_zeros() as usize);
            }
        }
        find_sse2(set, tail).map(|offset| chunks.len() * 32 + offset)
    }
}

/// NEON.
#[cfg(target_arch = "aarch64")]
pub(crate) mod neon {
    use std::arch::aarch64::{
        uint8x16_t, vceqq_u8, vdupq_n_u8, vget_lane_u64, vld1q_u8, vmaxvq_u8, vorrq_u8,
        vreinterpret_u64_u8, vreinterpretq_u16_u8, vshrn_n_u16,
    };

    use super::{LANES, StopSet};

    /// Sixteen bytes per chunk: `cmeq .16b` per needle, `orr`, `umaxv` as the
    /// clean-chunk test, and in the hit chunk a narrowing shift that packs the
    /// sixteen lanes into a 64-bit mask of four bits per lane.
    ///
    /// Compiled with `neon` enabled, which every `aarch64` processor has; the
    /// attribute is what makes the intrinsics callable, and it makes a call
    /// from a context without it `unsafe`.
    #[target_feature(enable = "neon")]
    #[inline(never)]
    pub(crate) fn find(set: &StopSet, haystack: &[u8]) -> Option<usize> {
        if !set.vectorizable() {
            return set.find_in_tail(haystack);
        }
        let needles: [uint8x16_t; LANES] =
            std::array::from_fn(|lane| vdupq_n_u8(set.needles()[lane]));
        let (chunks, tail) = haystack.as_chunks::<16>();
        for (k, chunk) in chunks.iter().enumerate() {
            // SAFETY: `chunk` is a `&[u8; 16]`, so the pointer is valid for
            // the sixteen-byte read `vld1q_u8` performs; it has no alignment
            // requirement beyond that of `u8`.
            let bytes = unsafe { vld1q_u8(chunk.as_ptr()) };
            let mut hits = vceqq_u8(bytes, needles[0]);
            for &needle in &needles[1..] {
                hits = vorrq_u8(hits, vceqq_u8(bytes, needle));
            }
            if vmaxvq_u8(hits) != 0 {
                let nibbles = vshrn_n_u16::<4>(vreinterpretq_u16_u8(hits));
                let mask = vget_lane_u64::<0>(vreinterpret_u64_u8(nibbles));
                return Some(k * 16 + (mask.trailing_zeros() / 4) as usize);
            }
        }
        set.find_in_tail(tail)
            .map(|offset| chunks.len() * 16 + offset)
    }
}

/// WebAssembly `simd128`.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub(crate) mod simd128 {
    use std::arch::wasm32::{u8x16_bitmask, u8x16_eq, u8x16_splat, v128, v128_load, v128_or};

    use super::{LANES, StopSet};

    /// Sixteen bytes per chunk: `i8x16.eq` per needle, `v128.or`,
    /// `i8x16.bitmask`.
    #[inline(never)]
    pub(crate) fn find(set: &StopSet, haystack: &[u8]) -> Option<usize> {
        if !set.vectorizable() {
            return set.find_in_tail(haystack);
        }
        let needles: [v128; LANES] = std::array::from_fn(|lane| u8x16_splat(set.needles()[lane]));
        let (chunks, tail) = haystack.as_chunks::<16>();
        for (k, chunk) in chunks.iter().enumerate() {
            // SAFETY: `chunk` is a `&[u8; 16]`, so the pointer is valid for the
            // sixteen-byte read, and `v128_load` performs a 1-aligned load.
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "`v128_load` performs a 1-aligned load; the pointer type is its signature's"
            )]
            let bytes = unsafe { v128_load(chunk.as_ptr().cast::<v128>()) };
            let mut hits = u8x16_eq(bytes, needles[0]);
            for &needle in &needles[1..] {
                hits = v128_or(hits, u8x16_eq(bytes, needle));
            }
            let mask = u8x16_bitmask(hits);
            if mask != 0 {
                return Some(k * 16 + mask.trailing_zeros() as usize);
            }
        }
        set.find_in_tail(tail)
            .map(|offset| chunks.len() * 16 + offset)
    }
}
