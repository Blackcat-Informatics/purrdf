#![allow(
    clippy::cast_ptr_alignment,
    clippy::doc_link_code,
    clippy::inline_always,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
// SIMD-intrinsic imports are split per tier and gated on the matching
// `kernel_*` feature so a tier-trimmed build pulls in only the intrinsics
// its enabled helpers use (a `kernel_scalar`-only trim imports none).
#[cfg(target_arch = "x86")]
use core::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
#[cfg(target_arch = "x86")]
use core::arch::x86::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256};

// Only the 32-bit x86 `detect_x86_caps` body queries CPU features at
// runtime; the x86_64 body derives them from `detect_cpu_kernel()`.
#[cfg(target_arch = "x86")]
use std::arch::is_x86_feature_detected;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::sync::OnceLock;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
use core::arch::aarch64::{uint8x16_t, vld1q_u8, vst1q_u8};

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
use core::arch::wasm32::{v128, v128_load, v128_store};

/// Copy length at or above which [`copy_bytes_overshooting`] hands off to
/// `memcpy` (ERMS `rep movsb` on x86) instead of its chunked SIMD loop.
/// Below this the inline SIMD / overlapping-u64 paths win; above it the
/// copy is bandwidth-bound and `memcpy`'s microcoded bulk store is faster.
/// Picked to sit well above hot literal pushes (1..=32 B) and typical
/// match copies, squarely in raw-block / long-match territory.
const BULK_MEMCPY_THRESHOLD: usize = 2048;

/// Copies at least `copy_at_least` bytes from `src` to `dst`.
///
/// This helper may over-copy up to the chunk size of the chosen SIMD/scalar
/// kernel (16, 32, or 64 bytes — at most chunk_size - 1 extra bytes), mirroring
/// zstd wildcopy semantics for faster inner loops.
///
/// # Safety
/// Caller must guarantee:
/// - `src.0` points to at least `src.1` readable bytes.
/// - `dst.0` points to at least `dst.1` writable bytes.
/// - `copy_at_least <= src.1` and `copy_at_least <= dst.1`.
/// - `src.1` and `dst.1` are large enough for the selected kernel:
///   if `min(src.1, dst.1) >= copy_at_least` rounded up to the chunk size,
///   the SIMD/scalar chunk loop may copy that rounded-up amount.
///   Otherwise the function copies exactly `copy_at_least` bytes.
/// - Source and destination regions do not overlap.
#[inline(always)]
pub(crate) unsafe fn copy_bytes_overshooting(
    src: (*const u8, usize),
    dst: (*mut u8, usize),
    copy_at_least: usize,
) {
    if copy_at_least == 0 {
        return;
    }

    let min_buffer_size = core::cmp::min(src.1, dst.1);

    // Single-op fast path: for any copy_at_least in 1..=16 with 16 bytes of
    // slack on both sides, one vector store covers the request. Match copies
    // with offset 8..15 funnel into repeat_in_chunks → here as 8..15-byte
    // calls, and the previous chunk-loop dispatcher paid a function-call +
    // loop-setup cost on every one of them. The single-op path collapses
    // that to one load + one store, which is the upstream zstd wildcopy pattern.
    if copy_at_least <= 16 && min_buffer_size >= 16 {
        unsafe { single_op_copy_16(src.0, dst.0, copy_at_least) };
        debug_assert_eq_copy(src, dst, copy_at_least);
        return;
    }

    // Exact-length tail path: when the caller has no WILDCOPY_OVERLENGTH
    // slack (e.g. RingBuffer call sites where dst.1 ends at `head`), the
    // single-op fast path above falls through and the chunked SIMD kernels
    // below also bail (`rounded > min_buffer_size`), leaving libc memmove
    // as the only option. memmove was 24% of decode CPU on the profiled
    // scenario. Replace it with inline byte / overlapping-u64 ops for
    // copies up to 32 bytes — these write EXACTLY `copy_at_least` bytes
    // without any overshoot, which is the contract the slack-less call
    // sites require. 32-byte cap covers the typical literal-push size
    // range (1..=24 bytes seen on the profiled corpus) and stays within
    // a single straight-line block on the I-cache.
    if copy_at_least <= 32 {
        // SAFETY: `copy_at_least <= min(src.1, dst.1)` by this function's
        // contract, so all branches below read/write strictly within the
        // caller's reported readable / writable spans.
        unsafe {
            if copy_at_least <= 8 {
                // Byte-by-byte for 1..=8 bytes. The fixed-size loop unrolls
                // into a sequence of immediate-offset loads/stores on every
                // sane backend, so for the common 1..=8 case this is
                // typically 2-3 cycles inline vs the ~10+ cycle call into
                // libc memmove the previous fallback paid.
                let mut i = 0;
                while i < copy_at_least {
                    dst.0.add(i).write(src.0.add(i).read());
                    i += 1;
                }
            } else if copy_at_least <= 16 {
                // 9..=16 bytes via two overlapping unaligned u64 ops. The
                // overlap region is written twice with the same source
                // bytes, so the net effect is exactly `copy_at_least` bytes
                // copied — no overshoot past dst.0 + copy_at_least.
                let lo: u64 = src.0.cast::<u64>().read_unaligned();
                let hi_offset = copy_at_least - 8;
                let hi: u64 = src.0.add(hi_offset).cast::<u64>().read_unaligned();
                dst.0.cast::<u64>().write_unaligned(lo);
                dst.0.add(hi_offset).cast::<u64>().write_unaligned(hi);
            } else {
                // 17..=32 bytes: first 16 via two adjacent u64 stores, the
                // trailing 1..=16 via the same overlapping-pair trick.
                // Four loads + four stores total, all branch-free.
                let lo: u64 = src.0.cast::<u64>().read_unaligned();
                let hi: u64 = src.0.add(8).cast::<u64>().read_unaligned();
                dst.0.cast::<u64>().write_unaligned(lo);
                dst.0.add(8).cast::<u64>().write_unaligned(hi);
                let tail_off = copy_at_least - 16;
                let tail_lo: u64 = src.0.add(tail_off).cast::<u64>().read_unaligned();
                let tail_hi: u64 = src.0.add(copy_at_least - 8).cast::<u64>().read_unaligned();
                dst.0.add(tail_off).cast::<u64>().write_unaligned(tail_lo);
                dst.0
                    .add(copy_at_least - 8)
                    .cast::<u64>()
                    .write_unaligned(tail_hi);
            }
        }
        debug_assert_eq_copy(src, dst, copy_at_least);
        return;
    }

    // Bulk-copy path: large non-overlapping copies (raw-block payloads,
    // long non-overlapping matches) are bandwidth-bound, and on modern
    // x86 the microcoded `rep movsb` (ERMS) that `memcpy` lowers to beats
    // a hand-rolled 2×32B ymm loop — it issues wider internal stores with
    // no per-iteration loop overhead and better hardware prefetch. The
    // chunked-SIMD kernels below win only in the small/medium range where
    // the `memcpy` call + ERMS startup cost would dominate the few bytes
    // actually moved. Above this threshold, hand off to `memcpy`.
    if copy_at_least >= BULK_MEMCPY_THRESHOLD {
        // SAFETY: by contract `copy_at_least <= min(src.1, dst.1)`, and
        // the regions do not overlap, so this reads/writes exactly
        // `copy_at_least` bytes within both reported spans (no overshoot).
        unsafe { dst.0.copy_from_nonoverlapping(src.0, copy_at_least) };
        debug_assert_eq_copy(src, dst, copy_at_least);
        return;
    }

    // The common bound check is only needed on targets with a SIMD arm.
    #[cfg(any(
        target_arch = "x86",
        target_arch = "x86_64",
        all(target_arch = "aarch64", target_feature = "neon"),
        all(target_arch = "wasm32", target_feature = "simd128")
    ))]
    macro_rules! try_chunk_kernel {
        ($chunk:expr, $kernel:ident) => {{
            if copy_at_least >= $chunk {
                let rounded = copy_at_least.next_multiple_of($chunk);
                if min_buffer_size >= rounded {
                    unsafe { $kernel(src.0, dst.0, rounded) };
                    debug_assert_eq_copy(src, dst, copy_at_least);
                    return;
                }
            }
        }};
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let caps = detect_x86_caps();
        if caps.avx2 {
            try_chunk_kernel!(32, copy_avx2);
        }

        if caps.sse2 {
            try_chunk_kernel!(16, copy_sse2);
        }
    }

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    try_chunk_kernel!(16, copy_neon);

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    try_chunk_kernel!(16, copy_simd128);

    // Final fallback: scalar 8-byte chunk loop if alignment permits, else
    // an exact byte copy. Inlined directly to avoid the per-call dispatcher
    // overhead the previous CopyFn function-pointer abstraction imposed.
    let scalar_chunk = size_of::<usize>();
    let rounded = copy_at_least.next_multiple_of(scalar_chunk);
    if min_buffer_size >= rounded {
        unsafe { copy_scalar(src.0, dst.0, rounded) };
    } else {
        unsafe { dst.0.copy_from_nonoverlapping(src.0, copy_at_least) };
    }
    debug_assert_eq_copy(src, dst, copy_at_least);
}

/// Single 16-byte transfer covering any 1..=16 byte request. The caller
/// guarantees 16 bytes of readable / writable slack on both sides so a full
/// vector store is safe even when only the first `len` bytes are required —
/// trailing bytes are written but the caller treats them as wildcopy overshoot.
///
/// # Safety
/// `src` and `dst` must each point to at least 16 readable / writable bytes;
/// regions must not overlap.
#[inline(always)]
unsafe fn single_op_copy_16(src: *const u8, dst: *mut u8, len: usize) {
    debug_assert!(len <= 16);
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    unsafe {
        let v: uint8x16_t = vld1q_u8(src);
        vst1q_u8(dst, v);
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    unsafe {
        let v: v128 = v128_load(src.cast::<v128>());
        v128_store(dst.cast::<v128>(), v);
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        if detect_x86_caps().sse2 {
            copy_sse2(src, dst, 16);
            return;
        }
    }

    // Two overlapping u64 transfers cover up to 16 bytes without SIMD.
    #[cfg(not(any(
        all(target_arch = "aarch64", target_feature = "neon"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    unsafe {
        let lo: u64 = src.cast::<u64>().read_unaligned();
        let hi_offset = len.saturating_sub(8);
        let hi: u64 = src.add(hi_offset).cast::<u64>().read_unaligned();
        dst.cast::<u64>().write_unaligned(lo);
        dst.add(hi_offset).cast::<u64>().write_unaligned(hi);
    }
}

#[inline(always)]
fn debug_assert_eq_copy(src: (*const u8, usize), dst: (*mut u8, usize), len: usize) {
    let _ = (src, dst, len);
    #[cfg(debug_assertions)]
    unsafe {
        let s = core::slice::from_raw_parts(src.0, len);
        let d = core::slice::from_raw_parts(dst.0, len);
        debug_assert_eq!(s, d);
    }
}

#[inline(always)]
unsafe fn copy_scalar(mut src: *const u8, mut dst: *mut u8, len: usize) {
    let end = unsafe { src.add(len) };
    while src < end {
        unsafe {
            dst.cast::<usize>()
                .write_unaligned(src.cast::<usize>().read_unaligned());
            src = src.add(size_of::<usize>());
            dst = dst.add(size_of::<usize>());
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[derive(Clone, Copy)]
struct X86Caps {
    avx2: bool,
    sse2: bool,
}

/// SIMD-copy capability flags for the chunked wildcopy dispatcher.
///
/// On `x86_64` these are DERIVED from the unified `detect_cpu_kernel()`
/// tag rather than detected independently, so the whole crate has a
/// single CPU-capability source of truth (the copy path can never
/// disagree with the entropy/sequence path about the running CPU). The
/// mapping follows the kernel ladder: AVX2 uses 32- and 16-byte copies,
/// BMI2 or SSE2 uses 16-byte copies, and Scalar uses neither.
/// The x86_64 tier ladder does not apply to 32-bit x86, which detects AVX2
/// and SSE2 directly.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
fn detect_x86_caps() -> X86Caps {
    static CAPS: OnceLock<X86Caps> = OnceLock::new();
    *CAPS.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        {
            use crate::cpu_kernel::{CpuKernelTag, detect_cpu_kernel};
            match detect_cpu_kernel() {
                CpuKernelTag::Avx2 => X86Caps {
                    avx2: true,
                    sse2: true,
                },

                CpuKernelTag::Bmi2 => X86Caps {
                    avx2: false,
                    sse2: true,
                },

                CpuKernelTag::Sse2 => X86Caps {
                    avx2: false,
                    sse2: true,
                },
                CpuKernelTag::Scalar => X86Caps {
                    avx2: false,
                    sse2: false,
                },
            }
        }
        #[cfg(target_arch = "x86")]
        {
            X86Caps {
                avx2: is_x86_feature_detected!("avx2"),
                sse2: is_x86_feature_detected!("sse2"),
            }
        }
    })
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn copy_sse2(mut src: *const u8, mut dst: *mut u8, len: usize) {
    let end = unsafe { src.add(len) };
    while src < end {
        unsafe {
            let v: __m128i = _mm_loadu_si128(src.cast::<__m128i>());
            _mm_storeu_si128(dst.cast::<__m128i>(), v);
            src = src.add(16);
            dst = dst.add(16);
        }
    }
}

// Inner loop is unrolled to 2× 32-byte AVX2 vectors per iteration (64
// bytes / iter), with a single-vector tail handling the residual 32
// bytes when `len` is a non-multiple of 64. The dispatcher rounds
// `copy_at_least` up to a multiple of 32 before calling, so `len`
// here is always a multiple of 32 — the loop body handles
// `len & !63` bytes, the tail handles the remaining 0 or 32.
//
// The two independent load / store pairs per iteration expose more
// instruction-level parallelism to the out-of-order core and amortise
// the loop branch, shortening AVX2 wildcopy latency. Actual speed-up
// is workload-dependent — measured in `benches/wildcopy_candidates.rs`
// (criterion micro) and end-to-end via `benches/compare_ffi.rs`.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
/// Copy full 64-byte iterations, then the remaining 32-byte tail. The mask
/// rounds the iteration boundary down; it never rounds an output extent up.
unsafe fn copy_avx2(mut src: *const u8, mut dst: *mut u8, len: usize) {
    debug_assert!(
        len.is_multiple_of(32),
        "copy_avx2 expects len to be a multiple of 32 (dispatcher rounds up)",
    );
    let end_unrolled = len & !63;
    let mut copied = 0usize;
    while copied < end_unrolled {
        unsafe {
            let v0: __m256i = _mm256_loadu_si256(src.cast::<__m256i>());
            let v1: __m256i = _mm256_loadu_si256(src.add(32).cast::<__m256i>());
            _mm256_storeu_si256(dst.cast::<__m256i>(), v0);
            _mm256_storeu_si256(dst.add(32).cast::<__m256i>(), v1);
            src = src.add(64);
            dst = dst.add(64);
        }
        copied += 64;
    }
    // Residual 32-byte vector when `len` is 32 mod 64.
    if copied < len {
        unsafe {
            let v: __m256i = _mm256_loadu_si256(src.cast::<__m256i>());
            _mm256_storeu_si256(dst.cast::<__m256i>(), v);
        }
    }
}

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[inline(always)]
unsafe fn copy_neon(mut src: *const u8, mut dst: *mut u8, len: usize) {
    let end = unsafe { src.add(len) };
    while src < end {
        unsafe {
            let v: uint8x16_t = vld1q_u8(src);
            vst1q_u8(dst, v);
            src = src.add(16);
            dst = dst.add(16);
        }
    }
}

/// WebAssembly `simd128` 16-byte chunk copy: `v128_load` / `v128_store` per
/// 16 bytes, mirroring [`copy_neon`]. `len` is a multiple of 16 (the caller
/// rounds up via `try_chunk_kernel!`). Compiled only under
/// `target_feature = "simd128"`, so the intrinsics are available without a
/// `#[target_feature]` attribute (wasm SIMD is a compile-time decision, no
/// runtime detection); the loads/stores are `unsafe` raw-pointer ops.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[inline(always)]
unsafe fn copy_simd128(mut src: *const u8, mut dst: *mut u8, len: usize) {
    let end = unsafe { src.add(len) };
    while src < end {
        unsafe {
            let v: v128 = v128_load(src.cast::<v128>());
            v128_store(dst.cast::<v128>(), v);
            src = src.add(16);
            dst = dst.add(16);
        }
    }
}

/// AVX2 (32-byte) inline exact copy: 32-byte unaligned stores for the
/// floor-aligned bulk plus one **overlapping** 32-byte store ending exactly
/// at `len`. Reads and writes strictly `[0, len)` (no read-overshoot), so it
/// is safe for sources without WILDCOPY slack (encoder literal slices into
/// possibly-borrowed input). No `#[target_feature]` attribute: this is only
/// compiled when the whole crate is built with AVX2 enabled
/// (`cfg(target_feature = "avx2")`, e.g. `-C target-cpu=x86-64-v3`), so the
/// intrinsics are legal and the body **inlines into the caller** with no ABI
/// boundary and no runtime detect. Requires `len >= 33` (the `append_literals`
/// `> 32` gate), so the overlapping 32-byte tail never underflows.
///
/// **Unrolled 2×32B (64 B/iter)** to match `copy_avx2`'s kernel shape: a
/// single-store-per-iter loop is throughput-bound by the loop branch on long
/// runs (measured: it degraded vs libc for `len > 128`); two independent
/// load/store pairs per iteration expose ILP and amortise the branch.
#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "avx2"
))]
#[inline]
unsafe fn copy_exact_inline_avx2(src: *const u8, dst: *mut u8, len: usize) {
    debug_assert!(len >= 33, "copy_exact_inline_avx2 requires len >= 33");
    unsafe {
        if len <= 64 {
            // 33..=64: two overlapping 32B blocks, BRANCHLESS. glibc's tiny
            // path beats a per-block loop here purely on the loop's branch
            // overhead (measured: a loop lost +23% to glibc at len=40, the
            // dominant medium bucket); the straight-line 2-store form ties /
            // beats it. Load both before storing — regions don't overlap in
            // src, and the dst overlap is store-after-store (last wins).
            let a = _mm256_loadu_si256(src.cast::<__m256i>());
            let b = _mm256_loadu_si256(src.add(len - 32).cast::<__m256i>());
            _mm256_storeu_si256(dst.cast::<__m256i>(), a);
            _mm256_storeu_si256(dst.add(len - 32).cast::<__m256i>(), b);
        } else if len <= 128 {
            // 65..=128: head 64 + tail 64 (overlapping), branchless 4 stores.
            let a = _mm256_loadu_si256(src.cast::<__m256i>());
            let b = _mm256_loadu_si256(src.add(32).cast::<__m256i>());
            let c = _mm256_loadu_si256(src.add(len - 64).cast::<__m256i>());
            let d = _mm256_loadu_si256(src.add(len - 32).cast::<__m256i>());
            _mm256_storeu_si256(dst.cast::<__m256i>(), a);
            _mm256_storeu_si256(dst.add(32).cast::<__m256i>(), b);
            _mm256_storeu_si256(dst.add(len - 64).cast::<__m256i>(), c);
            _mm256_storeu_si256(dst.add(len - 32).cast::<__m256i>(), d);
        } else {
            // >128: 2×32B-unrolled 64 B/iter loop, then an EXACT 32B cleanup
            // loop, then at most one overlapping 32B tail. The tail overlaps
            // the preceding block by <=31 bytes and fires AT MOST ONCE — unlike
            // a 2×32B tail whose first store overlaps the block the 64B loop
            // JUST wrote, which on Skylake hits a store-buffer partial-overlap
            // penalty every iteration (measured +69% at len=800 vs this form).
            let mut o = 0usize;
            while o + 64 <= len {
                let v0 = _mm256_loadu_si256(src.add(o).cast::<__m256i>());
                let v1 = _mm256_loadu_si256(src.add(o + 32).cast::<__m256i>());
                _mm256_storeu_si256(dst.add(o).cast::<__m256i>(), v0);
                _mm256_storeu_si256(dst.add(o + 32).cast::<__m256i>(), v1);
                o += 64;
            }
            while o + 32 <= len {
                let v = _mm256_loadu_si256(src.add(o).cast::<__m256i>());
                _mm256_storeu_si256(dst.add(o).cast::<__m256i>(), v);
                o += 32;
            }
            if o < len {
                let t = len - 32;
                let v = _mm256_loadu_si256(src.add(t).cast::<__m256i>());
                _mm256_storeu_si256(dst.add(t).cast::<__m256i>(), v);
            }
        }
    }
}

/// SSE2 inline exact copy for **i686** (`target_arch = "x86"`, SSE2 baseline)
/// — 16-byte-width analog of [`copy_exact_inline_avx2`] (branchless size-class
/// for `len <= 64`, then 2×16B-unrolled loop + exact cleanup + one overlapping
/// 16B tail). On 32-bit x86 the libc-memcpy call is relatively MORE expensive
/// (cdecl stack args, few registers), so inlining wins even vs glibc's tuned
/// SSE2 memcpy; on musl/no_std (scalar memcpy) it wins outright. Compiled only
/// when SSE2 is in the build (default on `i686-unknown-linux-*`) and NOT AVX2
/// (the AVX2 arm handles that). Gated to `target_arch = "x86"` so x86_64 — where
/// glibc's IFUNC AVX memcpy beats a 16-byte inline — keeps the AVX2-or-memcpy
/// arms. Requires `len >= 33`.
#[cfg(all(
    target_arch = "x86",
    target_feature = "sse2",
    not(target_feature = "avx2")
))]
#[inline]
unsafe fn copy_exact_inline_sse2(src: *const u8, dst: *mut u8, len: usize) {
    debug_assert!(len >= 33, "copy_exact_inline_sse2 requires len >= 33");
    unsafe {
        if len <= 64 {
            let a = _mm_loadu_si128(src.cast::<__m128i>());
            let b = _mm_loadu_si128(src.add(16).cast::<__m128i>());
            let c = _mm_loadu_si128(src.add(len - 32).cast::<__m128i>());
            let d = _mm_loadu_si128(src.add(len - 16).cast::<__m128i>());
            _mm_storeu_si128(dst.cast::<__m128i>(), a);
            _mm_storeu_si128(dst.add(16).cast::<__m128i>(), b);
            _mm_storeu_si128(dst.add(len - 32).cast::<__m128i>(), c);
            _mm_storeu_si128(dst.add(len - 16).cast::<__m128i>(), d);
        } else {
            let mut o = 0usize;
            while o + 32 <= len {
                let v0 = _mm_loadu_si128(src.add(o).cast::<__m128i>());
                let v1 = _mm_loadu_si128(src.add(o + 16).cast::<__m128i>());
                _mm_storeu_si128(dst.add(o).cast::<__m128i>(), v0);
                _mm_storeu_si128(dst.add(o + 16).cast::<__m128i>(), v1);
                o += 32;
            }
            while o + 16 <= len {
                _mm_storeu_si128(
                    dst.add(o).cast::<__m128i>(),
                    _mm_loadu_si128(src.add(o).cast::<__m128i>()),
                );
                o += 16;
            }
            if o < len {
                let t = len - 16;
                _mm_storeu_si128(
                    dst.add(t).cast::<__m128i>(),
                    _mm_loadu_si128(src.add(t).cast::<__m128i>()),
                );
            }
        }
    }
}

/// NEON inline exact copy — aarch64 analog of [`copy_exact_inline_avx2`],
/// **unrolled 2×16B (32 B/iter)**. NEON is the aarch64 baseline
/// (`cfg(target_feature = "neon")`), so the body inlines with no boundary.
/// Requires `len >= 33`.
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[inline]
unsafe fn copy_exact_inline_neon(src: *const u8, dst: *mut u8, len: usize) {
    debug_assert!(len >= 33, "copy_exact_inline_neon requires len >= 33");
    let mut o = 0usize;
    unsafe {
        while o + 32 <= len {
            let v0 = vld1q_u8(src.add(o));
            let v1 = vld1q_u8(src.add(o + 16));
            vst1q_u8(dst.add(o), v0);
            vst1q_u8(dst.add(o + 16), v1);
            o += 32;
        }
        while o + 16 <= len {
            vst1q_u8(dst.add(o), vld1q_u8(src.add(o)));
            o += 16;
        }
        if o < len {
            let t = len - 16;
            vst1q_u8(dst.add(t), vld1q_u8(src.add(t)));
        }
    }
}

/// Exact medium-size copy (`33 <= len < `[`BULK_MEMCPY_THRESHOLD`]) for encoder
/// literal runs — the safe analog of upstream zstd `ZSTD_wildcopy`. The kernel is
/// fixed at **compile time** (the build's `target_feature`), so the chosen
/// SIMD body inlines directly into the caller with NO runtime detect and NO
/// `#[target_feature]` ABI boundary:
/// - AVX2 build (`-C target-cpu=x86-64-v3`): inline 32-byte exact copy.
/// - i686 (SSE2 baseline, no AVX2): inline 16-byte exact copy.
/// - aarch64 (NEON baseline): inline 16-byte exact copy.
/// - any other build: libc `memcpy` (`ptr::copy_nonoverlapping`), whose
///   per-CPU IFUNC routine is already optimal for medium runs — a narrow
///   inline loop or a runtime-dispatched `#[target_feature]` call both
///   measured slower than it (i9, 2026-06-06). Default multi-kernel builds
///   that select AVX2 at runtime fall here: a leaf copy cannot inline AVX2
///   without the crate-wide feature, and a dispatched call ties glibc, so
///   glibc is the right choice until the whole emit path is under the
///   `fastpath` umbrella (then this collapses into the per-tier monolith).
///
/// # Safety
/// `src` readable and `dst` writable for `len` bytes; regions non-overlapping.
/// `len` MUST be `>= 33`: every SIMD kernel reads/writes an overlapping tail at
/// `len - 32` (AVX2) or `len - 16` (SSE2/NEON), which underflows for smaller
/// `len`. Callers route `<= 32` through a separate exact path, so the dispatcher
/// only ever sees `>= 33`; the `debug_assert!` below guards future misuse.
#[inline]
pub(crate) unsafe fn copy_exact_medium(src: *const u8, dst: *mut u8, len: usize) {
    debug_assert!(
        len >= 33,
        "copy_exact_medium requires len >= 33 (overlapping SIMD tail underflows below that)",
    );
    // Exactly one of the three cfg arms compiles per build, so each is a
    // tail statement with no `return` (avoids `clippy::needless_return`).
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    unsafe {
        copy_exact_inline_avx2(src, dst, len)
    };

    #[cfg(all(
        target_arch = "x86",
        target_feature = "sse2",
        not(target_feature = "avx2")
    ))]
    unsafe {
        copy_exact_inline_sse2(src, dst, len)
    };

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    unsafe {
        copy_exact_inline_neon(src, dst, len)
    };

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        all(
            target_arch = "x86",
            target_feature = "sse2",
            not(target_feature = "avx2")
        ),
        all(target_arch = "aarch64", target_feature = "neon")
    )))]
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, len)
    };
}
