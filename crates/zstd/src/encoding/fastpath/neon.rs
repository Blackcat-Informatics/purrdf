#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! AArch64 NEON fastpath variant. Every hot-path function in this module is
//! marked `#[target_feature(enable = "neon")]` so that the standard-library
//! NEON intrinsics (which themselves carry that attribute) inline directly
//! into the call graph instead of going through the function-call ABI barrier.
//!
//! NEON is part of the AArch64 baseline ISA — the attribute is therefore
//! redundant for correctness but mandatory for inline behavior under Rust's
//! ABI rules.

#![cfg(all(target_arch = "aarch64", target_endian = "little"))]
#![allow(dead_code)]

use core::arch::aarch64::{
    __crc32d, uint8x16_t, vceqq_u8, vgetq_lane_u64, vld1q_u8, vreinterpretq_u64_u8,
};

use super::scalar;

pub(crate) const KERNEL_TAG: &str = "neon";

/// AArch64 `crc32d`-accelerated `hash_mix_u64`. Routes a full 64-bit lane
/// through the CRC unit and folds the result back with a rotated copy of the
/// source so the upper bits stay well-distributed for hash-table indexing.
///
/// The `crc` AArch64 extension is **optional** and NOT implied by the NEON
/// baseline. Callers must therefore confirm both `neon` and `crc` are
/// reported by the runtime feature detector (or compile-time `cfg!` in
/// `no_std`) before reaching this function — the dispatcher in
/// `fastpath::detect_kernel_uncached` enforces that gate. Calling this on a
/// CPU without the CRC extension would trap with an illegal instruction.
#[target_feature(enable = "crc")]
#[inline]
pub(crate) unsafe fn hash_mix_u64(value: u64) -> u64 {
    let crc = __crc32d(0, value) as u64;
    // Match the x86 SSE4.2/AVX2 kernels so the per-arch hash mixers stay
    // consistent (different rotate counts on the same input would hide bugs
    // in cross-kernel hash assertions).
    ((crc << 32) ^ value.rotate_left(13)).wrapping_mul(scalar::HASH_MIX_PRIME)
}

/// 16-byte NEON vector prefix-length probe. Returns the number of leading
/// equal bytes that fit in whole 16-byte chunks; the caller (or the wrapper
/// below) handles the scalar tail.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes. NEON must be
/// available — guaranteed on AArch64 baseline but enforced by the
/// `target_feature` attribute.
#[target_feature(enable = "neon")]
#[inline]
pub(crate) unsafe fn prefix_len_simd(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    let mut off = 0usize;
    while off + 16 <= max {
        let a: uint8x16_t = unsafe { vld1q_u8(lhs.add(off)) };
        let b: uint8x16_t = unsafe { vld1q_u8(rhs.add(off)) };
        let eq = vceqq_u8(a, b);
        let lanes = vreinterpretq_u64_u8(eq);
        let low = vgetq_lane_u64(lanes, 0);
        if low != u64::MAX {
            let diff = low ^ u64::MAX;
            return off + scalar::mismatch_byte_index(diff as usize);
        }
        let high = vgetq_lane_u64(lanes, 1);
        if high != u64::MAX {
            let diff = high ^ u64::MAX;
            return off + 8 + scalar::mismatch_byte_index(diff as usize);
        }
        off += 16;
    }
    off
}

/// NEON variant of `common_prefix_len_ptr`: SIMD vector loop, then the shared
/// scalar tail. Marked `target_feature(enable = "neon")` so callers inside
/// the same umbrella inline this into their hot loop without an ABI barrier.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes.
#[target_feature(enable = "neon")]
#[inline]
pub(crate) unsafe fn common_prefix_len_ptr(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    // SAFETY: this entry retains its ISA contract and the original pointer bounds.
    unsafe {
        scalar::common_prefix_len_with(lhs, rhs, max, |lhs, rhs, max| {
            prefix_len_simd(lhs, rhs, max)
        })
    }
}

/// NEON variant of `count_match_from_indices` — the BT-walk match-length
/// probe entry point. Same invariants as the scalar variant but with the
/// NEON umbrella attribute so callers in Week 3a can adopt
/// `target_feature(enable = "neon")` themselves and get straight-line inlines.
///
/// # Safety
/// BT walk invariants: `candidate_idx + tail_limit ≤ concat.len()` and
/// `current_idx + tail_limit ≤ concat.len()`.
#[target_feature(enable = "neon")]
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
