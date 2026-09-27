// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The processor-specific kernels: the only code in the crate allowed
//! `unsafe`.
//!
//! Each kernel is reachable from the rest of the crate only through a safe
//! function pointer that an accessor below hands out after run-time detection
//! (x86_64, aarch64) or compile-time configuration (wasm32 `+simd128`) has
//! confirmed every feature it was compiled for. Each safe wrapper re-checks
//! the slice bounds its kernel's raw loads and stores rely on, so no caller
//! can reach out-of-bounds memory through it. On any other processor the
//! accessor returns `None`.

use crate::kernels::{CopyMatch, HashWindows, MatchLength};

/// A vector path's three kernels. `hash` is `None` where the path has no
/// vector form of it and the portable one runs instead.
#[derive(Clone, Copy)]
pub(crate) struct VectorKernels {
    pub(crate) copy: CopyMatch,
    pub(crate) match_length: MatchLength,
    pub(crate) hash: Option<HashWindows>,
}

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
mod wasm32;

#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{avx2, sse2};

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn sse2() -> Option<VectorKernels> {
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn avx2() -> Option<VectorKernels> {
    None
}

#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::neon;

#[cfg(not(target_arch = "aarch64"))]
pub(crate) const fn neon() -> Option<VectorKernels> {
    None
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub(crate) use wasm32::simd128;

#[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
pub(crate) const fn simd128() -> Option<VectorKernels> {
    None
}
