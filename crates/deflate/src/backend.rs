// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The named kernel paths, for tests and benches: the `deflate` family of
//! [`purrdf_hash::Backend`], so `PURRDF_REQUIRE_SIMD_PATHS` names them as
//! `deflate:<path>`.
//!
//! Not a stable interface. Every path produces the same bytes; this module
//! exists so a test can run each path the host supports against the portable
//! one, and a bench can time each.

use purrdf_hash::Backend as _;

use crate::arch::{self, VectorKernels};
use crate::kernels::{
    self, COPY_SLACK, CopyMatch, HashWindows, MatchLength, copy_match_portable,
    hash_windows_portable, match_length_portable,
};

purrdf_hash::vector_backend! {
    /// A kernel path.
    pub enum Backend {
        /// Portable safe code; always available.
        Portable,
        /// x86_64 SSE2 (the architecture baseline): 16-byte copies and compares.
        Sse2,
        /// x86_64 AVX2, detected at run time: 32-byte copies and compares, and
        /// eight 4-byte windows hashed per shuffle + lane multiply.
        Avx2,
        /// aarch64 NEON: 16-byte copies and compares, four windows per hash step.
        Neon,
        /// wasm32 simd128, in a `+simd128` build: as NEON.
        Simd128,
    }
    available: |path| path.kernels().is_some();
}

/// A path's resolved kernels.
#[derive(Clone, Copy)]
pub(crate) struct Kernels {
    pub(crate) backend: Backend,
    pub(crate) copy: CopyMatch,
    pub(crate) match_length: MatchLength,
    pub(crate) hash: HashWindows,
}

impl std::fmt::Debug for Kernels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernels")
            .field("backend", &self.backend)
            .finish_non_exhaustive()
    }
}

impl Backend {
    pub(crate) fn kernels(self) -> Option<Kernels> {
        let vector = |found: Option<VectorKernels>| {
            found.map(|v| Kernels {
                backend: self,
                copy: v.copy,
                match_length: v.match_length,
                hash: v.hash.unwrap_or(hash_windows_portable),
            })
        };
        match self {
            Self::Portable => Some(Kernels {
                backend: self,
                copy: copy_match_portable,
                match_length: match_length_portable,
                hash: hash_windows_portable,
            }),
            Self::Sse2 => vector(arch::sse2()),
            Self::Avx2 => vector(arch::avx2()),
            Self::Neon => vector(arch::neon()),
            Self::Simd128 => vector(arch::simd128()),
        }
    }

    /// The selected path's kernels.
    pub(crate) fn selected_kernels() -> Kernels {
        Self::selected()
            .kernels()
            .expect("the selected path is available")
    }

    /// Bytes a match copy may write past the match; a buffer handed to
    /// [`Self::copy_match`] needs this much room after `dst + len`.
    pub const COPY_SLACK: usize = COPY_SLACK;

    /// Run this path's match copy: `len` bytes to `buf[dst..]` from `dist`
    /// bytes back. `None` when the path is unavailable.
    ///
    /// # Panics
    ///
    /// When `dist` is 0 or exceeds `dst`, or `buf` lacks
    /// [`Self::COPY_SLACK`] bytes after the match.
    pub fn copy_match(self, buf: &mut [u8], dst: usize, dist: usize, len: usize) -> Option<()> {
        self.kernels().map(|k| (k.copy)(buf, dst, dist, len))
    }

    /// Run this path's match-length compare: the common prefix length of `a`
    /// and `b`. `None` when the path is unavailable.
    pub fn match_length(self, a: &[u8], b: &[u8]) -> Option<usize> {
        self.kernels().map(|k| (k.match_length)(a, b))
    }

    /// Run this path's window hashing over `data[start..]` into `out`.
    /// `None` when the path is unavailable.
    ///
    /// # Panics
    ///
    /// When `start + out.len() + 3 > data.len()`.
    pub fn hash_windows(self, data: &[u8], start: usize, out: &mut [u32]) -> Option<()> {
        self.kernels().map(|k| (k.hash)(data, start, out))
    }
}

/// The hash of the little-endian 4-byte window `window` (the encoder's hash).
pub const fn hash4(window: u32) -> u32 {
    kernels::hash4(window)
}
