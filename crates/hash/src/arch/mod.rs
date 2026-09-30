// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The processor-specific kernels: the only code in the crate allowed
//! `unsafe`.
//!
//! Hardware kernels are `#[target_feature]` functions behind safe wrappers
//! that check the required CPU capabilities. SHA-1 and CRC-32 select function
//! pointers; BLAKE3 selects an enum backend and calls checked wrappers. On an
//! unsupported processor or target the wrappers return `None`. The wasm32 `simd128`
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
//!
//! The one `unsafe` here that is not a kernel is [`encode_text`]: the base16
//! digits the crate's encoder has just written are handed out as `str`
//! without re-validating them as UTF-8, which cost more than the encoding
//! itself on long inputs (the `hex` group of the `purrdf-hash-conformance`
//! `digests` bench).

/// Writes the base16 rendering of `input` into `output` through
/// [`crate::hex`]'s length switch, uppercase when `UPPER`, and returns it as
/// text.
///
/// # Panics
///
/// When `output` is not exactly twice as long as `input`: every byte of it
/// must be written for the text to be what this returns.
#[inline]
pub(crate) fn encode_text<'o, const UPPER: bool>(input: &[u8], output: &'o mut [u8]) -> &'o str {
    encode_text_with::<UPPER, false>(input, output)
}

/// [`encode_text`] for a rendering the caller copies out of `output` at once
/// (a `Display` writing to a formatter): [`crate::hex::encode_bytes`] with its
/// `DISPLAY` layout.
#[inline]
pub(crate) fn encode_text_display<'o, const UPPER: bool>(
    input: &[u8],
    output: &'o mut [u8],
) -> &'o str {
    encode_text_with::<UPPER, true>(input, output)
}

#[inline]
fn encode_text_with<'o, const UPPER: bool, const DISPLAY: bool>(
    input: &[u8],
    output: &'o mut [u8],
) -> &'o str {
    assert_eq!(
        output.len(),
        2 * input.len(),
        "a base16 rendering is two digits per input byte"
    );
    crate::hex::encode_bytes::<UPPER, DISPLAY>(input, output);
    debug_assert!(output.is_ascii());
    // SAFETY: `encode_bytes` writes every byte of `output` (checked above to
    // be exactly two per input byte), and every path it runs writes only
    // digits of the two sixteen-digit base16 alphabets: the compare-select
    // loop computes `n + b'0'` plus 0, 39 or 7 for a nibble `n < 16`, and the
    // SSSE3, NEON and `simd128` kernels look each nibble, masked to `0..16`,
    // up in a sixteen-byte alphabet table. Those bytes are ASCII, and ASCII is
    // UTF-8.
    unsafe { core::str::from_utf8_unchecked(output) }
}

/// The base16 rendering of `input` as an owned `String`: [`encode_text`]
/// into a buffer of exactly the right length.
pub(crate) fn encode_string<const UPPER: bool>(input: &[u8]) -> String {
    let mut digits = vec![0u8; 2 * input.len()];
    let _ = encode_text::<UPPER>(input, &mut digits);
    // SAFETY: `encode_text` wrote every byte of `digits` as an ASCII base16
    // digit (see its SAFETY comment), so the buffer is UTF-8.
    unsafe { String::from_utf8_unchecked(digits) }
}

/// Processes a run of whole 64-byte SHA-1 blocks.
pub(crate) type Sha1Blocks = fn(&mut [u32; 5], &[u8]);
/// Advances a raw (un-complemented) CRC-32 register over any bytes.
pub(crate) type Crc32Update = fn(u32, &[u8]) -> u32;
/// Writes the base16 encoding of the first slice into the second, which is
/// exactly twice as long: uppercase when the flag is set, lowercase otherwise.
pub(crate) type HexEncode = fn(&[u8], &mut [u8], bool);

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
    use crate::dispatch::Backend as _;
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
        Some(unsafe {
            if let Ok(full) = <&[u8; 4096]>::try_from(bytes) {
                blake3_rows::chunks(full, counter)
            } else {
                blake3_sse2::avx512::chunk_cvs(bytes, counter)
            }
        })
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

#[cfg(target_arch = "x86_64")]
mod blake3_parents2;

/// Two parents packed into two independent four-word SIMD groups.
#[cfg(target_arch = "x86_64")]
pub(crate) fn blake3_parents2_avx512(children: &[[u32; 8]; 4]) -> Option<[[u32; 8]; 2]> {
    if std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("avx512f")
        && std::is_x86_feature_detected!("avx512vl")
    {
        // SAFETY: every required feature is present; the input contains both
        // complete parent blocks, with no alignment requirement.
        Some(unsafe { blake3_parents2::parents(children) })
    } else {
        None
    }
}

#[cfg(target_arch = "x86_64")]
mod blake3_rows;

/// Complete four-chunk subtree, retaining its two final children for ROOT.
/// This entry avoids the partial-chunk kernel's padding frame entirely.
#[cfg(target_arch = "x86_64")]
pub(crate) fn blake3_subtree_four(bytes: &[u8; 4096], counter: u64) -> Option<[[u32; 8]; 2]> {
    if std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("avx512f")
        && std::is_x86_feature_detected!("avx512vl")
    {
        // SAFETY: every required feature is present, all four chunks are
        // complete, and the local child array contains both parent blocks.
        Some(unsafe { blake3_parents2::parents(&blake3_rows::chunks(bytes, counter)) })
    } else {
        None
    }
}
