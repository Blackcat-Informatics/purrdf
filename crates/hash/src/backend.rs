// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The named execution paths of SHA-1, CRC-32 and base16 encoding, for
//! tests and benches.
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

impl Sha1Backend {
    /// Every path, in order of preference (the last is always available).
    pub const ALL: [Self; 3] = [Self::X86Sha, Self::Aarch64Sha1, Self::Portable];

    /// The path's name, as `PURRDF_REQUIRE_HASH_PATHS` spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Sha => "x86-sha",
            Self::Aarch64Sha1 => "aarch64-sha1",
        }
    }

    /// Whether this processor can run the path.
    pub fn is_available(self) -> bool {
        self.blocks().is_some()
    }

    /// The path [`Sha1::new`] and [`Sha1::digest`] use on this processor.
    pub fn selected() -> Self {
        Self::ALL
            .into_iter()
            .find(|backend| backend.is_available())
            .unwrap_or(Self::Portable)
    }

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

impl Crc32Backend {
    /// Every path; the first available one in this order is selected, except
    /// that [`Aarch64Pmull`](Self::Aarch64Pmull) is never selected.
    pub const ALL: [Self; 4] = [
        Self::X86Pclmulqdq,
        Self::Aarch64Crc32,
        Self::Aarch64Pmull,
        Self::Portable,
    ];

    /// The path's name, as `PURRDF_REQUIRE_HASH_PATHS` spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Pclmulqdq => "x86-pclmulqdq",
            Self::Aarch64Crc32 => "aarch64-crc32",
            Self::Aarch64Pmull => "aarch64-pmull",
        }
    }

    /// Whether this processor can run the path.
    pub fn is_available(self) -> bool {
        self.update_fn().is_some()
    }

    /// The path [`Crc32::new`] and [`Crc32::checksum`] use on this processor.
    pub fn selected() -> Self {
        Self::ALL
            .into_iter()
            .filter(|backend| *backend != Self::Aarch64Pmull)
            .find(|backend| backend.is_available())
            .unwrap_or(Self::Portable)
    }

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

/// A base16 encoding path, the one [`hex::Lower`](crate::hex::Lower) renders
/// through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HexBackend {
    /// A sixteen-entry alphabet table; always available.
    Portable,
    /// SSSE3 `pshufb` nibble lookup, x86-64 with `ssse3`.
    X86Ssse3,
    /// NEON `tbl` nibble lookup.
    Aarch64Neon,
    /// wasm `i8x16.swizzle` nibble lookup, in a build with `simd128` enabled.
    Wasm32Simd128,
}

impl HexBackend {
    /// Every path, in order of preference (the last is always available).
    pub const ALL: [Self; 4] = [
        Self::X86Ssse3,
        Self::Aarch64Neon,
        Self::Wasm32Simd128,
        Self::Portable,
    ];

    /// The path's name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::X86Ssse3 => "x86-ssse3",
            Self::Aarch64Neon => "aarch64-neon",
            Self::Wasm32Simd128 => "wasm32-simd128",
        }
    }

    /// Whether this processor and build can run the path.
    pub fn is_available(self) -> bool {
        self.encode_fn_if_available().is_some()
    }

    /// The path [`hex::Lower`](crate::hex::Lower) uses on this processor.
    pub fn selected() -> Self {
        Self::ALL
            .into_iter()
            .find(|backend| backend.is_available())
            .unwrap_or(Self::Portable)
    }

    /// Writes the lowercase base16 encoding of `input` into `output` on this
    /// path. `None`, writing nothing, when the processor cannot run the path
    /// or `output` is not exactly twice as long as `input`.
    pub fn encode(self, input: &[u8], output: &mut [u8]) -> Option<()> {
        if Some(output.len()) != input.len().checked_mul(2) {
            return None;
        }
        self.encode_fn_if_available()
            .map(|encode| encode(input, output))
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
    crate::fixed::Portable
);

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
crate::fixed::hasher!(
    /// [`FixedHasher`](crate::fixed::FixedHasher)'s AES function (AES rounds
    /// on slices longer than 16 bytes), present only in builds whose target
    /// enables AES, where it is the selected function.
    AesFixedHasher,
    crate::fixed::Aes
);

/// The name of the function this build's
/// [`FixedHasher`](crate::fixed::FixedHasher) computes: `"aes"` or
/// `"portable"`.
pub const FIXED_HASHER_PATH: &str = crate::fixed::SELECTED_NAME;

/// Execution paths of the native unkeyed BLAKE3-256 implementation.
pub use crate::blake3::Backend as Blake3Backend;
