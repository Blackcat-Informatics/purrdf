#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
#[inline(always)]
pub(crate) fn prefetch_slice(slice: &[u8]) {
    prefetch_slice_impl_l1(slice);
}

/// Issue exactly one L1 prefetch hint at the first byte of `slice`,
/// regardless of `slice.len()`. Use when the caller knows the slice
/// is short (< CACHE_LINE) but the cache line containing
/// `slice.as_ptr()` is still the one the consumer is about to read.
///
/// The standard `prefetch_slice` early-returns on slices below one
/// cache line, which is the right call for bulk prefetch (no point
/// hinting a partial buffer) but the wrong call for the wrap-boundary
/// match-source case in `prefetch_lookahead_match_source`: there a
/// 16-byte s1 tail is the EXACT line we need warmed even though it
/// sits below the bulk threshold.
#[inline(always)]
pub(crate) fn prefetch_first_line_l1(slice: &[u8]) {
    if slice.is_empty() {
        return;
    }
    prefetch_first_line_l1_impl(slice.as_ptr());
}

#[inline(always)]
pub(crate) fn prefetch_slice_t1(slice: &[u8]) {
    prefetch_slice_impl_t1(slice);
}

/// Issue one L1 prefetch hint at `ptr`. The encoder match-finder uses this
/// to warm the input lines a few positions ahead (upstream zstd `PREFETCH_L1(ip +
/// 256)` in `ZSTD_compressBlock_doubleFast`). `ptr` need not be in bounds —
/// prefetching an invalid address is a no-op by the ISA spec, not UB — so the
/// caller may aim slightly past the buffer end without a bounds check.
#[inline(always)]
pub(crate) fn prefetch_l1_at(ptr: *const u8) {
    prefetch_first_line_l1_impl(ptr);
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn prefetch_slice_impl_l1(slice: &[u8]) {
    use core::arch::x86_64::_MM_HINT_T0;
    prefetch_stride_x86_64::<{ _MM_HINT_T0 }>(slice);
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn prefetch_first_line_l1_impl(ptr: *const u8) {
    use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
    // SAFETY: `_mm_prefetch` accepts any address — prefetching an
    // invalid pointer is a no-op by the ISA spec, not UB.
    unsafe { _mm_prefetch(ptr.cast(), _MM_HINT_T0) };
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn prefetch_slice_impl_t1(slice: &[u8]) {
    use core::arch::x86_64::_MM_HINT_T1;
    prefetch_stride_x86_64::<{ _MM_HINT_T1 }>(slice);
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn prefetch_stride_x86_64<const HINT: i32>(slice: &[u8]) {
    use core::arch::x86_64::_mm_prefetch;
    const CACHE_LINE: usize = 64;
    const MAX_LINES: usize = 4;

    if slice.len() < CACHE_LINE {
        return;
    }

    let line_count = slice.len().div_ceil(CACHE_LINE).min(MAX_LINES);
    let base = slice.as_ptr();
    for i in 0..line_count {
        let ptr = unsafe { base.add(i * CACHE_LINE) };
        unsafe { _mm_prefetch(ptr.cast(), HINT) };
    }
}

#[cfg(all(target_arch = "x86", target_feature = "sse"))]
#[inline(always)]
fn prefetch_slice_impl_l1(slice: &[u8]) {
    use core::arch::x86::_MM_HINT_T0;
    prefetch_stride_x86::<{ _MM_HINT_T0 }>(slice);
}

#[cfg(all(target_arch = "x86", target_feature = "sse"))]
#[inline(always)]
fn prefetch_first_line_l1_impl(ptr: *const u8) {
    use core::arch::x86::{_MM_HINT_T0, _mm_prefetch};
    unsafe { _mm_prefetch(ptr.cast(), _MM_HINT_T0) };
}

#[cfg(all(target_arch = "x86", target_feature = "sse"))]
#[inline(always)]
fn prefetch_slice_impl_t1(slice: &[u8]) {
    use core::arch::x86::_MM_HINT_T1;
    prefetch_stride_x86::<{ _MM_HINT_T1 }>(slice);
}

#[cfg(all(target_arch = "x86", target_feature = "sse"))]
#[inline(always)]
fn prefetch_stride_x86<const HINT: i32>(slice: &[u8]) {
    use core::arch::x86::_mm_prefetch;
    const CACHE_LINE: usize = 64;
    const MAX_LINES: usize = 4;

    if slice.len() < CACHE_LINE {
        return;
    }

    let line_count = slice.len().div_ceil(CACHE_LINE).min(MAX_LINES);
    let base = slice.as_ptr();
    for i in 0..line_count {
        let ptr = unsafe { base.add(i * CACHE_LINE) };
        unsafe { _mm_prefetch(ptr.cast(), HINT) };
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn prefetch_slice_impl_l1(slice: &[u8]) {
    prefetch_stride_aarch64::<true>(slice);
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn prefetch_first_line_l1_impl(ptr: *const u8) {
    use core::arch::asm;
    unsafe {
        asm!(
            "prfm pldl1keep, [{ptr}]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags, readonly)
        );
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn prefetch_slice_impl_t1(slice: &[u8]) {
    prefetch_stride_aarch64::<false>(slice);
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn prefetch_stride_aarch64<const L1: bool>(slice: &[u8]) {
    use core::arch::asm;
    const CACHE_LINE: usize = 64;
    const MAX_LINES: usize = 4;

    if slice.len() < CACHE_LINE {
        return;
    }

    let line_count = slice.len().div_ceil(CACHE_LINE).min(MAX_LINES);
    let base = slice.as_ptr();
    for i in 0..line_count {
        let ptr = unsafe { base.add(i * CACHE_LINE) };
        if L1 {
            unsafe {
                asm!(
                    "prfm pldl1keep, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly)
                )
            };
        } else {
            unsafe {
                asm!(
                    "prfm pldl2keep, [{ptr}]",
                    ptr = in(reg) ptr,
                    options(nostack, preserves_flags, readonly)
                )
            };
        }
    }
}

#[cfg(not(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse"),
    target_arch = "aarch64"
)))]
#[inline(always)]
fn prefetch_slice_impl_l1(_slice: &[u8]) {}

#[cfg(not(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse"),
    target_arch = "aarch64"
)))]
#[inline(always)]
fn prefetch_first_line_l1_impl(_ptr: *const u8) {}

#[cfg(not(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse"),
    target_arch = "aarch64"
)))]
#[inline(always)]
fn prefetch_slice_impl_t1(_slice: &[u8]) {}
