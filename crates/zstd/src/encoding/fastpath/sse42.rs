#![allow(
    clippy::cast_ptr_alignment,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! x86/x86_64 SSE4.2 fastpath variant. Hot-path functions are marked
//! `#[target_feature(enable = "sse4.2")]` so intrinsics like `_mm_crc32_*` and
//! 128-bit SSE2 vector ops inline freely inside this module.
//!
//! Selected at runtime when AVX2/BMI2 are unavailable. SSE4.2 implies SSE2,
//! so we use 16-byte SSE2 vectors for the prefix-length scan.

#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#![allow(dead_code)]

#[cfg(target_arch = "x86")]
use core::arch::x86::{__m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m128i, _mm_cmpeq_epi8, _mm_crc32_u64, _mm_loadu_si128, _mm_movemask_epi8,
};

use super::scalar;

pub(crate) const KERNEL_TAG: &str = "sse42";

/// SSE4.2 `_mm_crc32_u64`-accelerated `hash_mix_u64`. Mirror of the upstream zstd
/// CRC-folded hash mix used by Dfast/Row hash compute. `_mm_crc32_u64` is
/// only available in 64-bit mode.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
#[inline]
pub(crate) unsafe fn hash_mix_u64(value: u64) -> u64 {
    let crc = unsafe { _mm_crc32_u64(0, value) };
    ((crc << 32) ^ value.rotate_left(13)).wrapping_mul(scalar::HASH_MIX_PRIME)
}

/// 32-bit fallback path: `_mm_crc32_u64` is not available on x86 (only x86_64),
/// so fall back to the scalar mix. Same signature as the 64-bit variant so the
/// dispatcher can route uniformly.
#[cfg(target_arch = "x86")]
#[target_feature(enable = "sse4.2")]
#[inline]
pub(crate) unsafe fn hash_mix_u64(value: u64) -> u64 {
    scalar::hash_mix_u64(value)
}

/// 16-byte SSE2 vector prefix-length probe. SSE2 is implied by SSE4.2.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes. SSE2 must be
/// available — implied by the `sse4.2` umbrella attribute on this fn.
#[target_feature(enable = "sse4.2")]
#[inline]
pub(crate) unsafe fn prefix_len_simd(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    let mut off = 0usize;
    while off + 16 <= max {
        let a: __m128i = unsafe { _mm_loadu_si128(lhs.add(off).cast::<__m128i>()) };
        let b: __m128i = unsafe { _mm_loadu_si128(rhs.add(off).cast::<__m128i>()) };
        let eq = unsafe { _mm_cmpeq_epi8(a, b) };
        let mask = unsafe { _mm_movemask_epi8(eq) } as u32;
        if mask != 0xFFFF {
            return off + (!mask).trailing_zeros() as usize;
        }
        off += 16;
    }
    off
}

/// SSE4.2 variant of `common_prefix_len_ptr`. Vector loop then the shared
/// scalar tail.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes.
#[target_feature(enable = "sse4.2")]
#[inline]
pub(crate) unsafe fn common_prefix_len_ptr(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    // SAFETY: this entry retains its ISA contract and the original pointer bounds.
    unsafe {
        scalar::common_prefix_len_with(lhs, rhs, max, |lhs, rhs, max| {
            prefix_len_simd(lhs, rhs, max)
        })
    }
}

/// SSE4.2 variant of `count_match_from_indices`. Same invariants as the
/// scalar variant.
///
/// # Safety
/// BT walk invariants: `candidate_idx + tail_limit ≤ concat.len()` and
/// `current_idx + tail_limit ≤ concat.len()`.
#[target_feature(enable = "sse4.2")]
#[inline]
pub(crate) unsafe fn count_match_from_indices(
    concat: &[u8],
    current_idx: usize,
    candidate_idx: usize,
    tail_limit: usize,
    seed_len: usize,
) -> usize {
    // SAFETY: this entry retains its ISA contract and the original index bounds.
    unsafe {
        scalar::count_match_with(
            concat,
            current_idx,
            candidate_idx,
            tail_limit,
            seed_len,
            |lhs, rhs, remaining| common_prefix_len_ptr(lhs, rhs, remaining),
        )
    }
}
