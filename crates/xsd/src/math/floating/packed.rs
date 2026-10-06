// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Four independent endpoint products with the scalar split-product equation.
//!
//! The only unsafe operations are ISA dispatch after availability checks and
//! unaligned loads/stores within fixed-size arrays. The admitted floating chunk
//! establishes nearest-even arithmetic and gradual underflow for every path.
#![allow(unsafe_code)]

#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    all(target_arch = "wasm32", target_feature = "simd128")
))]
use super::directed_error;
use super::{MathError, product_bounds};
use crate::ieee::Binary64;
use purrdf_hash::Backend;

/// Equivalent implementations of four outward binary64 endpoint products.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatProductBackend {
    /// Controlled scalar operations, available on every target.
    Portable,
    /// Two independent pairs on x86 SSE2.
    Sse2,
    /// Four independent lanes on x86 AVX2.
    Avx2,
    /// Four independent lanes in an x86 AVX-512 register.
    Avx512,
    /// Two independent pairs on AArch64 NEON.
    Neon,
    /// Two independent pairs on a wasm SIMD-enabled module.
    Simd128,
}

impl Backend for FloatProductBackend {
    // Four-endpoint measurements prefer AVX2 for individual products and SSE2
    // over padded AVX-512 for sustained small-lane work. The complete prepared
    // inverse is faster on the portable path, which remains the default below.
    const ALL: &'static [Self] = &[
        Self::Avx2,
        Self::Sse2,
        Self::Avx512,
        Self::Neon,
        Self::Simd128,
        Self::Portable,
    ];

    fn selected() -> Self {
        // Preserve the measured complete-kernel choice. Explicit worker selection
        // still exposes every equivalent available implementation.
        Self::Portable
    }

    fn is_available(self) -> bool {
        match self {
            Self::Portable => true,
            Self::Sse2 => {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                {
                    std::is_x86_feature_detected!("sse2")
                }
                #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                {
                    false
                }
            }
            Self::Avx2 => {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                {
                    std::is_x86_feature_detected!("avx2")
                }
                #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                {
                    false
                }
            }
            Self::Avx512 => {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                {
                    std::is_x86_feature_detected!("avx512f")
                }
                #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                {
                    false
                }
            }
            Self::Neon => cfg!(target_arch = "aarch64"),
            Self::Simd128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Sse2 => "sse2",
            Self::Avx2 => "avx2",
            Self::Avx512 => "avx512",
            Self::Neon => "neon",
            Self::Simd128 => "simd128",
        }
    }
}

pub(super) fn product_bounds4(
    a: [f64; 4],
    b: [f64; 4],
    backend: FloatProductBackend,
    ops: Binary64<'_>,
) -> Result<[(f64, f64); 4], MathError> {
    // In this range every nonzero split subproduct is normal (>=2^-1004)
    // and the splitter cannot overflow (<=2^478). Zero, extreme, and subnormal
    // operands retain the exact same scalar fallback and signed-zero handling.
    let minimum = f64::from_bits((1023 - 450) << 52);
    let maximum = f64::from_bits((1023 + 450) << 52);
    if backend == FloatProductBackend::Portable
        || a.into_iter()
            .chain(b)
            .any(|v| v.abs() < minimum || v.abs() > maximum)
    {
        let mut bounds = [(0.0, 0.0); 4];
        for (index, (a, b)) in a.into_iter().zip(b).enumerate() {
            bounds[index] = product_bounds(a, b, ops)?;
        }
        return Ok(bounds);
    }
    #[cfg(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    ))]
    {
        // SAFETY: the context checked availability before creating its thread-bound
        // floating chunk. Fixed-size buffers satisfy each ISA load/store contract.
        let (products, errors) = unsafe {
            match backend {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FloatProductBackend::Sse2 => x86::sse2(a, b),
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FloatProductBackend::Avx2 => x86::avx2(a, b),
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FloatProductBackend::Avx512 => x86::avx512(a, b),
                #[cfg(target_arch = "aarch64")]
                FloatProductBackend::Neon => neon::products(a, b),
                #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
                FloatProductBackend::Simd128 => wasm::products(a, b),
                _ => unreachable!("unavailable backend cannot enter a floating chunk"),
            }
        };
        Ok(core::array::from_fn(|index| {
            directed_error(products[index], errors[index])
        }))
    }
    #[cfg(not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    unreachable!("only the portable backend is available on this target")
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86 as arch;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64 as arch;

    /// The availability-checked two-pair SSE2 hot site.
    #[inline(never)]
    #[target_feature(enable = "sse2")]
    pub(super) unsafe fn sse2(a: [f64; 4], b: [f64; 4]) -> ([f64; 4], [f64; 4]) {
        let mut products = [0.0; 4];
        let mut errors = [0.0; 4];
        for offset in [0, 2] {
            // SAFETY: each load/store covers two entries within a four-entry array.
            unsafe {
                let a = arch::_mm_loadu_pd(a.as_ptr().add(offset));
                let b = arch::_mm_loadu_pd(b.as_ptr().add(offset));
                let product = arch::_mm_mul_pd(a, b);
                let (product, error) = split_product!(
                    a,
                    b,
                    product,
                    arch::_mm_set1_pd(134_217_729.0),
                    |x, y| arch::_mm_mul_pd(x, y),
                    |x, y| arch::_mm_sub_pd(x, y)
                );
                arch::_mm_storeu_pd(products.as_mut_ptr().add(offset), product);
                arch::_mm_storeu_pd(errors.as_mut_ptr().add(offset), error);
            }
        }
        (products, errors)
    }

    /// The availability-checked four-lane AVX2 hot site.
    #[inline(never)]
    #[target_feature(enable = "avx2")]
    pub(super) unsafe fn avx2(a: [f64; 4], b: [f64; 4]) -> ([f64; 4], [f64; 4]) {
        let mut products = [0.0; 4];
        let mut errors = [0.0; 4];
        // SAFETY: each load/store covers exactly its four-entry array.
        unsafe {
            let a = arch::_mm256_loadu_pd(a.as_ptr());
            let b = arch::_mm256_loadu_pd(b.as_ptr());
            let product = arch::_mm256_mul_pd(a, b);
            let (product, error) = split_product!(
                a,
                b,
                product,
                arch::_mm256_set1_pd(134_217_729.0),
                |x, y| arch::_mm256_mul_pd(x, y),
                |x, y| arch::_mm256_sub_pd(x, y)
            );
            arch::_mm256_storeu_pd(products.as_mut_ptr(), product);
            arch::_mm256_storeu_pd(errors.as_mut_ptr(), error);
        }
        (products, errors)
    }

    /// The availability-checked AVX-512 hot site, with four harmless padding lanes.
    #[inline(never)]
    #[target_feature(enable = "avx512f")]
    pub(super) unsafe fn avx512(a: [f64; 4], b: [f64; 4]) -> ([f64; 4], [f64; 4]) {
        let a = [a[0], a[1], a[2], a[3], 1.0, 1.0, 1.0, 1.0];
        let b = [b[0], b[1], b[2], b[3], 1.0, 1.0, 1.0, 1.0];
        let mut products = [0.0; 8];
        let mut errors = [0.0; 8];
        // SAFETY: each load/store covers exactly its eight-entry array.
        unsafe {
            let a = arch::_mm512_loadu_pd(a.as_ptr());
            let b = arch::_mm512_loadu_pd(b.as_ptr());
            let product = arch::_mm512_mul_pd(a, b);
            let (product, error) = split_product!(
                a,
                b,
                product,
                arch::_mm512_set1_pd(134_217_729.0),
                |x, y| arch::_mm512_mul_pd(x, y),
                |x, y| arch::_mm512_sub_pd(x, y)
            );
            arch::_mm512_storeu_pd(products.as_mut_ptr(), product);
            arch::_mm512_storeu_pd(errors.as_mut_ptr(), error);
        }
        (
            [products[0], products[1], products[2], products[3]],
            [errors[0], errors[1], errors[2], errors[3]],
        )
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use core::arch::aarch64 as arch;

    #[inline(never)]
    #[target_feature(enable = "neon")]
    pub(super) unsafe fn products(a: [f64; 4], b: [f64; 4]) -> ([f64; 4], [f64; 4]) {
        let mut products = [0.0; 4];
        let mut errors = [0.0; 4];
        for offset in [0, 2] {
            // SAFETY: each load/store covers two entries within a four-entry array.
            unsafe {
                let a = arch::vld1q_f64(a.as_ptr().add(offset));
                let b = arch::vld1q_f64(b.as_ptr().add(offset));
                let product = arch::vmulq_f64(a, b);
                let (product, error) = split_product!(
                    a,
                    b,
                    product,
                    arch::vdupq_n_f64(134_217_729.0),
                    |x, y| arch::vmulq_f64(x, y),
                    |x, y| arch::vsubq_f64(x, y)
                );
                arch::vst1q_f64(products.as_mut_ptr().add(offset), product);
                arch::vst1q_f64(errors.as_mut_ptr().add(offset), error);
            }
        }
        (products, errors)
    }
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
mod wasm {
    use core::arch::wasm32 as arch;

    #[inline(never)]
    #[target_feature(enable = "simd128")]
    pub(super) unsafe fn products(a: [f64; 4], b: [f64; 4]) -> ([f64; 4], [f64; 4]) {
        let mut products = [0.0; 4];
        let mut errors = [0.0; 4];
        for offset in [0, 2] {
            // SAFETY: each load/store covers two entries within a four-entry array.
            unsafe {
                let a = arch::v128_load(a.as_ptr().add(offset).cast());
                let b = arch::v128_load(b.as_ptr().add(offset).cast());
                let product = arch::f64x2_mul(a, b);
                let (product, error) = split_product!(
                    a,
                    b,
                    product,
                    arch::f64x2_splat(134_217_729.0),
                    arch::f64x2_mul,
                    arch::f64x2_sub
                );
                arch::v128_store(products.as_mut_ptr().add(offset).cast(), product);
                arch::v128_store(errors.as_mut_ptr().add(offset).cast(), error);
            }
        }
        (products, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{FloatProductBackend, product_bounds4};
    use crate::math::{CoordinateMath, MathLimits};
    use purrdf_hash::Backend as _;

    #[test]
    fn forced_available_paths_preserve_every_product_and_outward_endpoint_bit() {
        purrdf_hash::dispatch::assert_required_available::<FloatProductBackend>(
            "numeric",
            |path| match path {
                FloatProductBackend::Portable => true,
                FloatProductBackend::Sse2 => purrdf_hash::dispatch::host_advertises(&["sse2"]),
                FloatProductBackend::Avx2 => purrdf_hash::dispatch::host_advertises(&["avx2"]),
                FloatProductBackend::Avx512 => purrdf_hash::dispatch::host_advertises(&["avx512f"]),
                FloatProductBackend::Neon => cfg!(target_arch = "aarch64"),
                FloatProductBackend::Simd128 => {
                    cfg!(all(target_arch = "wasm32", target_feature = "simd128"))
                }
            },
        );
        let paths: Vec<_> = FloatProductBackend::all_available().collect();
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let mut state = 0x5124_7825_3641_e7b1;
        for row in 0..2048 {
            let mut draw = || {
                state = purrdf_hash::mix::splitmix64_step(state);
                let exponent = 573 + state % 901;
                state = purrdf_hash::mix::splitmix64_step(state);
                f64::from_bits((state & 0x800f_ffff_ffff_ffff) | (exponent << 52))
            };
            let mut a = core::array::from_fn(|_| draw());
            let mut b = core::array::from_fn(|_| draw());
            if row < 8 {
                a[0] = [
                    0.0,
                    -0.0,
                    f64::from_bits(1),
                    f64::MIN_POSITIVE,
                    f64::from_bits(573 << 52).next_down(),
                    f64::from_bits(1473 << 52).next_up(),
                    1.0,
                    -1.0,
                ][row];
                b[0] = 1.0;
            }
            math.set_binary64_backend(FloatProductBackend::Portable)
                .unwrap();
            let expected = {
                let chunk = math.enter_chunk().unwrap();
                product_bounds4(a, b, chunk.product_backend(), chunk.ops())
                    .unwrap()
                    .map(|(lo, hi)| (lo.to_bits(), hi.to_bits()))
            };
            for &path in &paths {
                math.set_binary64_backend(path).unwrap();
                let chunk = math.enter_chunk().unwrap();
                let got = product_bounds4(a, b, chunk.product_backend(), chunk.ops())
                    .unwrap()
                    .map(|(lo, hi)| (lo.to_bits(), hi.to_bits()));
                assert_eq!(got, expected, "{} row {row}", path.name());
            }
        }
    }
}
