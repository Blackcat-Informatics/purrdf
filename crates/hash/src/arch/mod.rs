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
//! The Armv8 SHA-1 and CRC-32 kernels are `#[inline(never)]`: on a build
//! whose target CPU already has `sha2`, `crc` and `aes` (Neoverse V1, say) a
//! `#[target_feature]` kernel is otherwise inlined into its safe wrapper, and
//! the asm audit (`scripts/simd-asm-manifest.toml`) measures each kernel at
//! its own path on every build.
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

#[cfg(target_arch = "x86_64")]
pub(crate) mod blake3_x86;

#[cfg(any(
    target_arch = "x86_64",
    target_arch = "x86",
    all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little"
    ),
    all(target_arch = "wasm32", target_feature = "simd128")
))]
mod blake3_lanes;
#[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon",
    target_endian = "little"
))]
mod blake3_neon;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
mod blake3_sse2;
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
mod blake3_wasm;

pub(crate) fn blake3_four(bytes: &[u8], counter: u64) -> Option<[[u32; 8]; 4]> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return None;
    }
    #[cfg(target_arch = "x86")]
    {
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: runtime detection establishes SSE2; array/input bounds are checked.
            return Some(unsafe { blake3_sse2::chunk_cvs(bytes, counter) });
        }
        None
    }
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE2 is baseline on x86-64 and input bounds are checked.
        Some(unsafe { blake3_sse2::chunk_cvs(bytes, counter) })
    }
    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little"
    ))]
    {
        // SAFETY: NEON is enabled for this target; input bounds are checked.
        Some(unsafe { blake3_neon::chunk_cvs(bytes, counter) })
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        Some(blake3_wasm::chunk_cvs(bytes, counter))
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            target_endian = "little"
        ),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        let _ = counter;
        None
    }
}

#[cfg(target_arch = "x86_64")]
mod blake3_avx2;
pub(crate) fn blake3_eight(bytes: &[u8], counter: u64) -> Option<[[u32; 8]; 8]> {
    #[cfg(target_arch = "x86_64")]
    if !bytes.is_empty() && bytes.len() <= 8192 && std::is_x86_feature_detected!("avx2") {
        // SAFETY: the processor has AVX2 and the input contains at most eight chunks.
        return Some(unsafe { blake3_avx2::chunk_cvs(bytes, counter) });
    }
    let _ = (bytes, counter);
    None
}

pub(crate) fn blake3_compress4(
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> Option<[u32; 8]> {
    #[cfg(target_arch = "x86")]
    {
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: runtime detection establishes SSE2; array/input bounds are checked.
            return Some(unsafe { blake3_sse2::single(cv, words, counter, length, flags) });
        }
        None
    }
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE2 is baseline on x86-64; inputs are fixed-size arrays.
        Some(unsafe { blake3_sse2::single(cv, words, counter, length, flags) })
    }
    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little"
    ))]
    {
        // SAFETY: NEON is enabled for this target; inputs are fixed-size arrays.
        Some(unsafe { blake3_neon::single(cv, words, counter, length, flags) })
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        Some(blake3_wasm::single(cv, words, counter, length, flags))
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            target_endian = "little"
        ),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        let _ = (cv, words, counter, length, flags);
        None
    }
}

#[inline]
pub(crate) fn blake3_compress_on(
    backend: crate::blake3::Backend,
    cv: [u32; 8],
    words: &[u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
) -> Option<[u32; 8]> {
    use crate::blake3::Backend;
    if !backend.is_available() {
        return None;
    }
    #[cfg(target_arch = "x86_64")]
    match backend {
        Backend::Avx512 => {
            // SAFETY: availability checks AVX-512F and VL above.
            return Some(unsafe { blake3_x86::single_avx512(cv, words, counter, length, flags) });
        }
        Backend::Avx2 => {
            // SAFETY: availability checks AVX2 above.
            return Some(unsafe { blake3_x86::single_avx2(cv, words, counter, length, flags) });
        }
        Backend::Ssse3 => {
            // SAFETY: availability checks SSSE3 above.
            return Some(unsafe { blake3_x86::single(cv, words, counter, length, flags) });
        }
        _ => {}
    }
    match backend {
        Backend::Sse2 | Backend::Avx2 | Backend::Neon | Backend::Wasm128 => {
            blake3_compress4(cv, words, counter, length, flags)
        }
        _ => None,
    }
}

pub(crate) fn blake3_parents4(children: &[[u32; 8]; 8]) -> Option<[[u32; 8]; 4]> {
    #[cfg(target_arch = "x86")]
    {
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: runtime detection establishes SSE2; array/input bounds are checked.
            return Some(unsafe { blake3_sse2::parents(children) });
        }
        None
    }
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE2 is baseline, and the input has eight full child CVs.
        Some(unsafe { blake3_sse2::parents(children) })
    }
    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little"
    ))]
    {
        // SAFETY: NEON is enabled; the array has every child CV.
        Some(unsafe { blake3_neon::parents(children) })
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        Some(blake3_wasm::parents(children))
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        all(
            target_arch = "aarch64",
            target_feature = "neon",
            target_endian = "little"
        ),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        let _ = children;
        None
    }
}
pub(crate) fn blake3_parents8(children: &[[u32; 8]; 16]) -> Option<[[u32; 8]; 8]> {
    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") {
        // SAFETY: AVX2 is available; the array has every child CV.
        return Some(unsafe { blake3_avx2::parents(children) });
    }
    let _ = children;
    None
}

/// Four live chunks with AVX-512VL rotations and 128-bit transposes.
#[cfg(target_arch = "x86_64")]
pub(crate) fn blake3_four_avx512(bytes: &[u8], counter: u64) -> Option<[[u32; 8]; 4]> {
    if std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512vl") {
        // SAFETY: both required features were detected; the kernel checks
        // lengths and pads partial chunks before its fixed-size loads.
        Some(unsafe { blake3_sse2::avx512::chunk_cvs(bytes, counter) })
    } else {
        None
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn blake3_parents4_avx512(children: &[[u32; 8]; 8]) -> Option<[[u32; 8]; 4]> {
    if std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512vl") {
        // SAFETY: features were detected; every load reads a full input array.
        Some(unsafe { blake3_sse2::avx512::parents(children) })
    } else {
        None
    }
}
