// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The processor-specific kernels: the only code in the crate allowed
//! `unsafe`.
//!
//! Each kernel is a `#[target_feature]` function. It is reachable from the
//! rest of the crate only through a function pointer that the matching
//! `*_x86_*` / `*_aarch64*` accessor below returns after run-time detection
//! has confirmed every feature the kernel was compiled for; on any other
//! processor or target the accessor returns `None`. The wasm32 `simd128`
//! kernel is the exception to run-time detection: wasm has none, so it is
//! compiled only when `simd128` is enabled for the whole build.
//!
//! The other exception is the fixed hasher's AES `Block`, which exists only
//! in builds whose target enables `aes` at compile time: such a build has one
//! hash function, never a run-time choice between two.

/// Processes a run of whole 64-byte SHA-1 blocks.
pub(crate) type Sha1Blocks = fn(&mut [u32; 5], &[u8]);
/// Advances a raw (un-complemented) CRC-32 register over any bytes.
pub(crate) type Crc32Update = fn(u32, &[u8]) -> u32;
/// Writes the lowercase base16 encoding of the first slice into the second,
/// which is exactly twice as long.
pub(crate) type HexEncode = fn(&[u8], &mut [u8]);

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
mod wasm32;

#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{crc32_x86_pclmulqdq, hex_x86_ssse3, sha1_x86_sha};

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn sha1_x86_sha() -> Option<Sha1Blocks> {
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn crc32_x86_pclmulqdq() -> Option<Crc32Update> {
    None
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) const fn hex_x86_ssse3() -> Option<HexEncode> {
    None
}

#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::{
    crc32_aarch64_crc32, crc32_aarch64_pmull, hex_aarch64_neon, sha1_aarch64,
};

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

#[cfg(not(target_arch = "aarch64"))]
pub(crate) const fn hex_aarch64_neon() -> Option<HexEncode> {
    None
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub(crate) use wasm32::hex_wasm32_simd128;

#[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
pub(crate) const fn hex_wasm32_simd128() -> Option<HexEncode> {
    None
}

#[cfg(all(target_arch = "x86_64", target_feature = "aes"))]
pub(crate) use x86_64::Block;

#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    target_feature = "aes"
))]
pub(crate) use aarch64::Block;
