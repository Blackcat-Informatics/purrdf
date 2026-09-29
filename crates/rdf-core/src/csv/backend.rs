// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Not a stable interface. Names and runs the field scanner's kernels, the
//! way [`purrdf_hash::backend`](https://docs.rs/purrdf-hash) names and runs
//! its digest and hex paths, so a same-repo integration test can run the
//! [`super::scan`] differential this crate's own `#[cfg(test)]` unit tests
//! run — including on `wasm32-unknown-unknown`, where `#[cfg(test)]` code
//! does not exist, so the compiled-in `simd128` kernel would otherwise never
//! execute under `make wasm-test`. See `tests/csv_scan_wasm.rs`.
//!
//! Every function here is safe. [`kernels`] adapts
//! [`super::arch::kernel`]'s `StopSet`-keyed [`super::scan::Kernel`]s
//! (`StopSet` is `pub(crate)`, so it cannot appear in this module's public
//! signatures) to plain byte slices by closing over an already-safe kernel
//! value — the `unsafe` that makes a kernel possible stays confined to
//! [`super::arch`], which alone constructs those safe values from unsafe
//! target-feature calls.

// The kernels and the dispatcher this module adapts are `pub(crate)`, so the
// links above and below are private-item links by construction — the same
// reason `csv::arch`'s module doc carries this allow.
#![allow(
    rustdoc::private_intra_doc_links,
    reason = "this doc-hidden module documents the crate-private kernels and dispatcher it adapts"
)]

use super::scan::StopSet;

/// A field-scanner kernel path: the `csv` family of [`purrdf_hash::Backend`],
/// so `PURRDF_REQUIRE_SIMD_PATHS` names them as `csv:<path>`.
///
/// [`Backend::ALL`](purrdf_hash::Backend::ALL) is the order
/// [`super::arch::find`] prefers them in, so
/// [`Backend::selected`](purrdf_hash::Backend::selected) names the kernel the
/// reader and writer run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    /// [`super::scan::find_portable`], sixteen-byte chunks answered lane by
    /// lane; available everywhere, and the oracle every other path is tested
    /// against.
    Portable,
    /// SSE2 on `x86_64` (the architecture baseline), 16-byte chunks.
    Sse2,
    /// AVX2 on `x86_64`, detected at run time, 32-byte chunks.
    Avx2,
    /// NEON on `aarch64` (the architecture baseline), 16-byte chunks.
    Neon,
    /// wasm `simd128`, in a build with `simd128` enabled, 16-byte chunks.
    Simd128,
}

impl purrdf_hash::Backend for Backend {
    const ALL: &'static [Self] = &[
        Self::Avx2,
        Self::Sse2,
        Self::Neon,
        Self::Simd128,
        Self::Portable,
    ];

    fn is_available(self) -> bool {
        super::arch::kernel(self).is_some()
    }

    fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Sse2 => "sse2",
            Self::Avx2 => "avx2",
            Self::Neon => "neon",
            Self::Simd128 => "simd128",
        }
    }
}

/// One named kernel run over plain byte slices: the member set and the
/// haystack, rather than a pre-built [`super::scan::StopSet`].
pub type ScanKernel = Box<dyn Fn(&[u8], &[u8]) -> Option<usize>>;

/// The offset of the first byte of `haystack` that is a member of `members`,
/// by the definition every kernel below is tested against: one table lookup
/// per byte, no vectorization.
pub fn reference(members: &[u8], haystack: &[u8]) -> Option<usize> {
    let set = StopSet::new(members);
    haystack.iter().position(|&b| set.contains(b))
}

/// Every kernel this build and processor run, by name, each answering the
/// same question as [`reference`]:
///
/// * `"dispatch"` — the live [`super::scan::StopSet::find`], the entry point
///   the reader and writer actually call.
/// * every available [`Backend`], by its name: `"portable"` everywhere, and
///   `sse2` and, when the processor reports it, `avx2` on `x86_64`; `neon` on
///   `aarch64`; `simd128` on `wasm32` compiled with `simd128` enabled.
pub fn kernels() -> Vec<(&'static str, ScanKernel)> {
    let mut kernels: Vec<(&'static str, ScanKernel)> = vec![(
        "dispatch",
        Box::new(|members: &[u8], haystack: &[u8]| StopSet::new(members).find(haystack)),
    )];
    for backend in <Backend as purrdf_hash::Backend>::all_available() {
        let kernel = super::arch::kernel(backend).expect("an available path has a kernel");
        kernels.push((
            purrdf_hash::Backend::name(backend),
            Box::new(move |members: &[u8], haystack: &[u8]| {
                kernel(&StopSet::new(members), haystack)
            }),
        ));
    }
    kernels
}
