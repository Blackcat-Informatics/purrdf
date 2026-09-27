// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The processor-specific kernels: the only code in the crate allowed
//! `unsafe`.
//!
//! Each kernel is a `#[target_feature]` function. It is reachable from the
//! rest of the crate only through a function pointer that the matching
//! `*_x86_*` / `*_aarch64*` accessor below returns after run-time detection
//! has confirmed every feature the kernel was compiled for; on any other
//! processor or target the accessor returns `None`.

/// Processes a run of whole 64-byte SHA-1 blocks.
pub(crate) type Sha1Blocks = fn(&mut [u32; 5], &[u8]);
/// Advances a raw (un-complemented) CRC-32 register over any bytes.
pub(crate) type Crc32Update = fn(u32, &[u8]) -> u32;

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{crc32_x86_pclmulqdq, sha1_x86_sha};

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn sha1_x86_sha() -> Option<Sha1Blocks> {
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn crc32_x86_pclmulqdq() -> Option<Crc32Update> {
    None
}

#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::{crc32_aarch64_crc32, crc32_aarch64_pmull, sha1_aarch64};

#[cfg(not(target_arch = "aarch64"))]
pub(crate) const fn sha1_aarch64() -> Option<Sha1Blocks> {
    None
}

#[cfg(not(target_arch = "aarch64"))]
pub(crate) const fn crc32_aarch64_crc32() -> Option<Crc32Update> {
    None
}

#[cfg(not(target_arch = "aarch64"))]
pub(crate) const fn crc32_aarch64_pmull() -> Option<Crc32Update> {
    None
}
