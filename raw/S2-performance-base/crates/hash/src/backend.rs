// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The named execution paths of SHA-1, CRC-32 and base16 encoding, for
//! tests and benches. Each is a [`Backend`] family (`sha1`, `crc32` and
//! `hex` in [`PURRDF_REQUIRE_SIMD_PATHS`](crate::dispatch::REQUIRE_SIMD_PATHS)).
//!
//! Not a stable interface. Every path computes the same bytes; this module
//! exists so a test can run each path the host supports and compare it with
//! the others and with the frozen vectors, and so a bench can time each one.
//! MD5 and SHA-3 have a single, portable path.
//!
//! The fixed hasher's paths are not a run-time choice: a build whose target
//! enables AES runs `AesFixedHasher`'s function (present only there) as
//! [`FixedHasher`](crate::fixed::FixedHasher), every other build runs
//! [`PortableFixedHasher`]'s. Both are named here so tests and the bench can
//! run the portable function on an AES build too.

use crate::arch::{self, Crc32Update, HexEncode, Sha1Blocks};
use crate::crc32::{self, Crc32};
use crate::dispatch::Backend;
use crate::sha1::Sha1;

/// A SHA-1 block-function path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sha1Backend {
    /// Portable scalar code; always available.
    Portable,
    /// The x86 SHA extensions (`sha1rnds4`, `sha1nexte`, `sha1msg1/2`),
    /// x86-64 with `sha`, `ssse3` and `sse4.1`.
    X86Sha,
    /// The Armv8 SHA1 instructions (`sha1c/p/m`, `sha1h`, `sha1su0/1`).
    Aarch64Sha1,
}

impl Backend for Sha1Backend {
    const ALL: &'static [Self] = &[Self::X86Sha, Self::Aarch64Sha1, Self::Portable];

    fn is_available(self) -> bool {
        self.blocks().is_some()
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Sha => "x86-sha",
            Self::Aarch64Sha1 => "aarch64-sha1",
        }
    }
}

impl Sha1Backend {
    /// A hasher pinned to this path, if the processor can run it.
    pub fn hasher(self) -> Option<Sha1> {
        self.blocks().map(|blocks| Sha1::on(self, blocks))
    }

    /// The SHA-1 digest of `data` on this path, if the processor can run it.
    pub fn digest(self, data: &[u8]) -> Option<[u8; crate::sha1::OUTPUT_LEN]> {
        self.hasher().map(|mut hasher| {
            hasher.update(data);
            hasher.finalize()
        })
    }

    pub(crate) fn blocks(self) -> Option<Sha1Blocks> {
        match self {
            Self::Portable => Some(crate::sha1::compress_portable),
            Self::X86Sha => arch::sha1_x86_sha(),
            Self::Aarch64Sha1 => arch::sha1_aarch64(),
        }
    }
}

/// A CRC-32 register-update path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Crc32Backend {
    /// Slicing-by-16 tables; always available.
    Portable,
    /// `pclmulqdq` folding with a Barrett reduction, x86-64 with `pclmulqdq`
    /// and `sse4.1`.
    X86Pclmulqdq,
    /// The Armv8 CRC32 instructions, eight bytes per instruction.
    Aarch64Crc32,
    /// Armv8 `pmull` folding finished by the CRC32 instructions. Available
    /// but not selected: it has not been measured against
    /// [`Aarch64Crc32`](Self::Aarch64Crc32) on Arm hardware.
    Aarch64Pmull,
}

impl Backend for Crc32Backend {
    const ALL: &'static [Self] = &[
        Self::X86Pclmulqdq,
        Self::Aarch64Crc32,
        Self::Aarch64Pmull,
        Self::Portable,
    ];

    /// The first available path, except that
    /// [`Aarch64Pmull`](Self::Aarch64Pmull) is never selected: it is the path
    /// [`Crc32::new`] and [`Crc32::checksum`] use on this processor.
    fn selected() -> Self {
        Self::all_available()
            .find(|backend| *backend != Self::Aarch64Pmull)
            .unwrap_or(Self::Portable)
    }

    fn is_available(self) -> bool {
        self.update_fn().is_some()
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Pclmulqdq => "x86-pclmulqdq",
            Self::Aarch64Crc32 => "aarch64-crc32",
            Self::Aarch64Pmull => "aarch64-pmull",
        }
    }
}

impl Crc32Backend {
    /// A CRC pinned to this path, if the processor can run it.
    pub fn hasher(self) -> Option<Crc32> {
        self.update_fn().map(|update| Crc32::on(self, update))
    }

    /// The CRC-32 of `data` on this path, if the processor can run it.
    pub fn checksum(self, data: &[u8]) -> Option<u32> {
        self.hasher().map(|mut crc| {
            crc.update(data);
            crc.finalize()
        })
    }

    pub(crate) fn update_fn(self) -> Option<Crc32Update> {
        match self {
            Self::Portable => Some(crc32::update_portable),
            Self::X86Pclmulqdq => arch::crc32_x86_pclmulqdq(),
            Self::Aarch64Crc32 => arch::crc32_aarch64_crc32(),
            Self::Aarch64Pmull => arch::crc32_aarch64_pmull(),
        }
    }
}

/// A base16 encoding path: the one [`hex`](crate::hex) renders inputs longer
/// than [`hex::SHORT_MAX`](crate::hex::SHORT_MAX) through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HexBackend {
    /// The compare-select loop, which the compiler packs on every vector
    /// target; always available.
    Portable,
    /// SSSE3 `pshufb` nibble lookup, x86-64 with `ssse3`.
    X86Ssse3,
    /// NEON `tbl` nibble lookup.
    Aarch64Neon,
    /// wasm `i8x16.swizzle` nibble lookup, in a build with `simd128` enabled.
    Wasm32Simd128,
}

impl Backend for HexBackend {
    const ALL: &'static [Self] = &[
        Self::X86Ssse3,
        Self::Aarch64Neon,
        Self::Wasm32Simd128,
        Self::Portable,
    ];

    fn is_available(self) -> bool {
        self.encode_fn_if_available().is_some()
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Ssse3 => "x86-ssse3",
            Self::Aarch64Neon => "aarch64-neon",
            Self::Wasm32Simd128 => "wasm32-simd128",
        }
    }
}

impl HexBackend {
    /// Writes the lowercase base16 encoding of `input` into `output` on this
    /// path. `None`, writing nothing, when the processor cannot run the path
    /// or `output` is not exactly twice as long as `input`.
    pub fn encode(self, input: &[u8], output: &mut [u8]) -> Option<()> {
        self.encode_case(input, output, false)
    }

    /// [`encode`](Self::encode) with `A`-`F`.
    pub fn encode_upper(self, input: &[u8], output: &mut [u8]) -> Option<()> {
        self.encode_case(input, output, true)
    }

    fn encode_case(self, input: &[u8], output: &mut [u8], upper: bool) -> Option<()> {
        if Some(output.len()) != input.len().checked_mul(2) {
            return None;
        }
        self.encode_fn_if_available()
            .map(|encode| encode(input, output, upper))
    }

    /// This path's encoder, or the portable one when it is unavailable.
    pub(crate) fn encode_fn(self) -> HexEncode {
        self.encode_fn_if_available()
            .unwrap_or(crate::hex::encode_portable)
    }

    fn encode_fn_if_available(self) -> Option<HexEncode> {
        match self {
            Self::Portable => Some(crate::hex::encode_portable),
            Self::X86Ssse3 => arch::hex_x86_ssse3(),
            Self::Aarch64Neon => arch::hex_aarch64_neon(),
            Self::Wasm32Simd128 => arch::hex_wasm32_simd128(),
        }
    }
}

crate::fixed::hasher!(
    /// [`FixedHasher`](crate::fixed::FixedHasher)'s portable function
    /// (folded multiplies on every length), available on every build.
    PortableFixedHasher,
    crate::fixed::Engine<crate::fixed::Portable>
);

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
crate::fixed::hasher!(
    /// [`FixedHasher`](crate::fixed::FixedHasher)'s AES accumulator function,
    /// present only in builds whose target
    /// enables AES, where it is the selected function.
    AesFixedHasher,
    crate::fixed::aes::Engine
);

/// The name of the function this build's
/// [`FixedHasher`](crate::fixed::FixedHasher) computes: `"aes"` or
/// `"portable"`.
pub const FIXED_HASHER_PATH: &str = crate::fixed::SELECTED_NAME;

/// Execution paths of the native unkeyed BLAKE3-256 implementation.
pub use crate::blake3::Backend as Blake3Backend;
