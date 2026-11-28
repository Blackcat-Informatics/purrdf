//! Cache processor capabilities and select the decoder's specialized kernels.
//! Pass the selected kernel into the bit reader so the inner sequence decoder
//! does not repeat feature detection.

use std::sync::OnceLock;

/// Trait covering the leaf hot-path operations whose bodies differ
/// per ISA. Implementations are ZSTs; the trait is `Copy` so it can
/// be `Default`-constructed at each call site without runtime cost.
///
/// New methods land here ONLY when their codegen genuinely differs
/// per kernel (BMI2 intrinsic vs scalar shift, AVX2 256-bit move vs
/// SSE2 128-bit move, etc.). Structure ops that have one canonical
/// implementation must NOT be on this trait — they stay on the
/// existing decoder / encoder types.
pub(crate) trait CpuKernel: Copy + 'static {
    /// Mask the low `n` bits of `value`, returning the remaining
    /// high bits zeroed. The FSE bitstream hot path fires this 3×
    /// per decoded sequence; on BMI2-capable hardware this maps to
    /// a single `_bzhi_u64` instruction, otherwise to a scalar
    /// `u64::MAX >> (64 - n)` shift + mask.
    ///
    /// Precondition: `n <= 64`. Behaviour for `n == 0` is "return 0";
    /// behaviour for `n > 64` is unspecified — callers MUST uphold
    /// the bound. The test-only `mask_lower_bits` helper in
    /// `bit_reader_reverse.rs` debug-asserts the bound for its
    /// unit tests, but production callers (FSE / HUF hot paths)
    /// derive `n` from `accuracy_log` / `max_num_bits` which the
    /// per-stream table builders pin to `n <= MAX_*_BITS` at
    /// construction time; no per-call wrapper assert runs.
    fn mask_lower_bits(value: u64, n: u8) -> u64;
}

/// Scalar fallback — portable, no SIMD or BMI2 intrinsics. Selected
/// when no x86 or aarch64 feature is detected at runtime.
#[derive(Copy, Clone, Default)]
pub(crate) struct ScalarKernel;

impl CpuKernel for ScalarKernel {
    #[expect(
        clippy::inline_always,
        reason = "keep the bit width visible in the decoder's per-field mask"
    )]
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // `checked_shr` returns `None` for shift counts >= 64, which
        // happens exactly when `n == 0` (`64 - 0 = 64`). Mapping
        // both that case and the invalid `n > 64` underflow to 0
        // gives the mathematically-correct empty mask for n=0 and
        // a safe-ish fallback for the invalid range.
        let mask = u64::MAX
            .checked_shr(64u32.wrapping_sub(u32::from(n)))
            .unwrap_or(0);
        value & mask
    }
}

// SSE2 has no bit-extraction instruction, so that tier uses the scalar mask.

/// x86_64 BMI2-only kernel: `_bzhi_u64` for mask_lower_bits. Selected
/// when the CPU has BMI2 but not the AVX2 SIMD width to upgrade to
/// the Avx2 kernel. Treated as a stepping stone between Sse2 and
/// Avx2 on hardware that has BMI2 but not AVX2 (rare in practice but
/// matches upstream zstd's gating).
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct Bmi2Kernel;

#[cfg(target_arch = "x86_64")]
impl CpuKernel for Bmi2Kernel {
    #[expect(
        clippy::inline_always,
        reason = "merge the bit mask into the decoder's checked BMI2 feature scope"
    )]
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // SAFETY: this kernel ZST is only reachable via the
        // `match detect_cpu_kernel() { CpuKernelTag::Bmi2 => ... }`
        // dispatch arms at decoder entry sites, all of which fire only
        // after `detect_cpu_kernel` confirmed BMI2 is available on the
        // running CPU.
        unsafe { mask_lower_bits_bmi2_impl(value, n) }
    }
}

/// x86_64 AVX2 + BMI2 kernel (x86-64-v3 baseline). The common modern
/// x86 case — most CPUs released since 2013 (Haswell) have AVX2+BMI2.
/// Uses `_bzhi_u64` for bit masks.
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct Avx2Kernel;

#[cfg(target_arch = "x86_64")]
impl CpuKernel for Avx2Kernel {
    #[expect(
        clippy::inline_always,
        reason = "merge the bit mask into the decoder's feature scope, which includes BMI2"
    )]
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // SAFETY: Avx2Kernel is selected only after runtime detect
        // confirmed both AVX2 and BMI2 — `_bzhi_u64` is callable.
        unsafe { mask_lower_bits_bmi2_impl(value, n) }
    }
}

/// Sequence-decoder kernel for 64-bit Arm processors with NEON.
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct NeonKernel;

#[cfg(target_arch = "aarch64")]
impl CpuKernel for NeonKernel {
    #[expect(
        clippy::inline_always,
        reason = "fold the scalar bit mask into the NEON decoder's field reads"
    )]
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // The scalar shift and mask needs no NEON instructions.
        ScalarKernel::mask_lower_bits(value, n)
    }
}

/// Sequence-decoder kernel for 64-bit Arm processors with SVE.
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct SveKernel;

#[cfg(target_arch = "aarch64")]
impl CpuKernel for SveKernel {
    #[expect(
        clippy::inline_always,
        reason = "fold the scalar bit mask into the SVE decoder's field reads"
    )]
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        ScalarKernel::mask_lower_bits(value, n)
    }
}

/// Single `#[target_feature(enable = "bmi2")]` wrapper around the
/// `_bzhi_u64` intrinsic. Lifted to a free function so each kernel
/// impl that needs the BMI2 path calls the
/// same shared body. With `#[inline]` LLVM inlines the call into
/// any caller that itself has BMI2 in scope; outside that scope the
/// target_feature boundary is preserved.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
#[inline]
unsafe fn mask_lower_bits_bmi2_impl(value: u64, n: u8) -> u64 {
    // The intrinsic call is permitted directly inside a function
    // already annotated `#[target_feature(enable = "bmi2")]` — no
    // `unsafe { ... }` block needed (the function-level `unsafe`
    // already covers it). SAFETY: caller selected a kernel whose
    // CpuKernelTag was resolved after `is_x86_feature_detected!("bmi2")`
    // returned true, so the BMI2 instruction set is available.
    core::arch::x86_64::_bzhi_u64(value, u32::from(n))
}

/// Select the x86 kernel from runtime capabilities. The AVX2 kernel also
/// requires BMI2 for its bit extraction.
#[cfg(target_arch = "x86_64")]
const fn select_x86_kernel(has_bmi2: bool, has_avx2: bool, has_sse2: bool) -> CpuKernelTag {
    if has_avx2 && has_bmi2 {
        return CpuKernelTag::Avx2;
    }

    if has_bmi2 {
        return CpuKernelTag::Bmi2;
    }

    if has_sse2 {
        return CpuKernelTag::Sse2;
    }
    CpuKernelTag::Scalar
}

/// Cached CPU capability used to select a decoder specialization.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CpuKernelTag {
    Scalar,
    #[cfg(target_arch = "x86_64")]
    Sse2,
    #[cfg(target_arch = "x86_64")]
    Bmi2,
    #[cfg(target_arch = "x86_64")]
    Avx2,

    #[cfg(target_arch = "aarch64")]
    Neon,
    #[cfg(target_arch = "aarch64")]
    Sve,
}

/// Detect once and cache the best available CPU kernel for the
/// current process. Subsequent calls return the cached tag without
/// re-running CPU-feature detection.
pub(crate) fn detect_cpu_kernel() -> CpuKernelTag {
    static CACHED: OnceLock<CpuKernelTag> = OnceLock::new();
    *CACHED.get_or_init(detect_cpu_kernel_uncached)
}

fn detect_cpu_kernel_uncached() -> CpuKernelTag {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::is_x86_feature_detected;
        select_x86_kernel(
            is_x86_feature_detected!("bmi2"),
            is_x86_feature_detected!("avx2"),
            is_x86_feature_detected!("sse2"),
        )
    }
    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::is_aarch64_feature_detected;

        if is_aarch64_feature_detected!("sve") {
            CpuKernelTag::Sve
        } else if is_aarch64_feature_detected!("neon") {
            CpuKernelTag::Neon
        } else {
            CpuKernelTag::Scalar
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        CpuKernelTag::Scalar
    }
}

impl CpuKernelTag {
    /// Stable lowercase diagnostic name for this tier (used by
    /// [`active_cpu_kernel_name`] and the bench/dashboard reporting). Pure
    /// mapping over the tag, so every arm is exercisable in tests regardless
    /// of which tier the running CPU actually resolves to.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            #[cfg(target_arch = "x86_64")]
            Self::Sse2 => "sse2",
            #[cfg(target_arch = "x86_64")]
            Self::Bmi2 => "bmi2",
            #[cfg(target_arch = "x86_64")]
            Self::Avx2 => "avx2",

            #[cfg(target_arch = "aarch64")]
            Self::Neon => "neon",
            #[cfg(target_arch = "aarch64")]
            Self::Sve => "sve",
        }
    }
}

/// Name of the CPU kernel tier this process selected for the entropy /
/// sequence hot paths: decode (literals + FSE sequence decode) and encode
/// (entropy) share this dispatch. Returned as a stable lowercase
/// string for diagnostics and benchmark reporting.
pub fn active_cpu_kernel_name() -> &'static str {
    detect_cpu_kernel().name()
}
