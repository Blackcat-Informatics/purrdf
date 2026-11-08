#![allow(
    clippy::cast_ptr_alignment,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! x86/x86_64 AVX2 + BMI2 fastpath variant. Functions are marked
//! `#[target_feature(enable = "avx2,bmi2")]` so 256-bit vector intrinsics
//! (`_mm256_*`), BMI2 bit-manipulation (`_pext_u64`, `_bzhi_u64`), and SSE2/4.2
//! intrinsics all inline natively inside this module's hot loop.
//!
//! Selected at runtime when both feature sets are present (Haswell and newer
//! x86 CPUs, ~2013+).

#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#![allow(dead_code)]

#[cfg(target_arch = "x86")]
use core::arch::x86::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8};

use super::scalar;

pub(crate) const KERNEL_TAG: &str = "avx2_bmi2";

/// AVX2+BMI2 variant of `hash_mix_u64`. AVX2 itself doesn't include the
/// CRC32 instruction (`_mm_crc32_u64` lives under SSE4.2); every shipping
/// AVX2 CPU also has SSE4.2 in hardware, but Rust's `target_feature`
/// machinery does not propagate that implication, so the attribute must
/// list `sse4.2` explicitly and the dispatcher must gate AVX2 kernel
/// selection on `sse4.2` being reported as well.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,bmi2,sse4.2")]
#[inline]
pub(crate) unsafe fn hash_mix_u64(value: u64) -> u64 {
    let crc = unsafe {
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::_mm_crc32_u64;
        _mm_crc32_u64(0, value)
    };
    ((crc << 32) ^ value.rotate_left(13)).wrapping_mul(scalar::HASH_MIX_PRIME)
}

#[cfg(target_arch = "x86")]
#[target_feature(enable = "avx2,bmi2")]
#[inline]
pub(crate) unsafe fn hash_mix_u64(value: u64) -> u64 {
    scalar::hash_mix_u64(value)
}

/// 32-byte AVX2 vector prefix-length probe.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes. AVX2 is
/// required and enforced by the `target_feature` attribute.
#[target_feature(enable = "avx2,bmi2")]
#[inline]
pub(crate) unsafe fn prefix_len_simd(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    let mut off = 0usize;
    while off + 32 <= max {
        let a: __m256i = unsafe { _mm256_loadu_si256(lhs.add(off).cast::<__m256i>()) };
        let b: __m256i = unsafe { _mm256_loadu_si256(rhs.add(off).cast::<__m256i>()) };
        let eq = unsafe { _mm256_cmpeq_epi8(a, b) };
        let mask = unsafe { _mm256_movemask_epi8(eq) } as u32;
        if mask != u32::MAX {
            return off + (!mask).trailing_zeros() as usize;
        }
        off += 32;
    }
    off
}

/// AVX2+BMI2 variant of `common_prefix_len_ptr`. Vector loop then the shared
/// scalar tail.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes.
#[target_feature(enable = "avx2,bmi2")]
#[inline]
pub(crate) unsafe fn common_prefix_len_ptr(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    // SAFETY: this entry retains its ISA contract and the original pointer bounds.
    unsafe {
        scalar::common_prefix_len_with(lhs, rhs, max, |lhs, rhs, max| {
            prefix_len_simd(lhs, rhs, max)
        })
    }
}

/// AVX2+BMI2 variant of `count_match_from_indices`. Same invariants as the
/// scalar variant.
///
/// # Safety
/// BT walk invariants: `candidate_idx + tail_limit ≤ concat.len()` and
/// `current_idx + tail_limit ≤ concat.len()`.
#[target_feature(enable = "avx2,bmi2")]
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
