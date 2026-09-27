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
//! Every function here is safe. [`kernels`] adapts [`super::arch::every_kernel`]'s
//! `StopSet`-keyed [`super::scan::Kernel`]s (`StopSet` is `pub(crate)`, so it
//! cannot appear in this module's public signatures) to plain byte slices by
//! closing over an already-safe kernel value — the `unsafe` that makes a
//! kernel possible stays confined to [`super::arch`], which alone constructs
//! those safe values from unsafe target-feature calls.

// The kernels and the dispatcher this module adapts are `pub(crate)`, so the
// links above and below are private-item links by construction — the same
// reason `csv::arch`'s module doc carries this allow.
#![allow(
    rustdoc::private_intra_doc_links,
    reason = "this doc-hidden module documents the crate-private kernels and dispatcher it adapts"
)]

use super::scan::{StopSet, find_portable};

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

/// Every kernel this build provides, by name, each answering the same
/// question as [`reference`]:
///
/// * `"portable"` — [`find_portable`], the oracle every explicit kernel is
///   tested against, and the only kernel on a target without one.
/// * `"dispatch"` — the live [`super::scan::StopSet::find`], the entry point
///   the reader and writer actually call.
/// * every explicit vector kernel this target compiles, or (for `avx2`) this
///   processor reports at run time — see [`super::arch::every_kernel`]: `sse2`
///   and, when available, `avx2` on `x86_64`; `neon` on `aarch64`; `simd128`
///   on `wasm32` compiled with `simd128` enabled.
pub fn kernels() -> Vec<(&'static str, ScanKernel)> {
    let mut kernels: Vec<(&'static str, ScanKernel)> = vec![
        (
            "portable",
            Box::new(|members: &[u8], haystack: &[u8]| {
                find_portable(&StopSet::new(members), haystack)
            }),
        ),
        (
            "dispatch",
            Box::new(|members: &[u8], haystack: &[u8]| StopSet::new(members).find(haystack)),
        ),
    ];
    for (name, kernel) in super::arch::every_kernel() {
        kernels.push((
            name,
            Box::new(move |members: &[u8], haystack: &[u8]| {
                kernel(&StopSet::new(members), haystack)
            }),
        ));
    }
    kernels
}

/// The name of the kernel `"dispatch"` (equivalently, [`super::arch::find`])
/// runs on this build and processor: `"avx2"` or `"sse2"` on `x86_64`,
/// `"neon"` on `aarch64`, `"simd128"` on `wasm32` built with `simd128`
/// enabled, `"portable"` otherwise. Mirrors [`super::arch::find`]'s own
/// choice rather than calling it, so naming the choice never runs a kernel.
pub fn selected() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return "avx2";
        }
        "sse2"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "neon"
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        "simd128"
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        "portable"
    }
}
