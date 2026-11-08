// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PurRDF's fixed-product pure Rust Zstandard codec.
//! Decoder errors expose their original native fields through borrowed inspection.
//!
#![doc = include_str!("../README.md")]
#![no_std]
#![deny(trivial_casts, trivial_numeric_casts, rust_2018_idioms)]

extern crate alloc;
extern crate std;

pub(crate) const VERBOSE: bool = false;

macro_rules! vprintln {
    ($($x:expr),*) => {
        #[cfg(all())]
        if crate::VERBOSE {
            std::println!($($x),*);
        }
    }
}

mod bit_io;
mod common;
/// Smallest accepted block-size target (the `ZSTD_TARGETCBLOCKSIZE_MIN`
/// bound): the single source of truth shared by the Rust setters
/// (`set_target_block_size`) and the C ABI parameter surface.
pub use common::MIN_TARGET_BLOCK_SIZE;
mod cpu_kernel;
pub mod decoding;
mod inspection;

pub mod dictionary;
pub mod encoding;
mod histogram;

pub(crate) mod blocks;

// `pub fn init_state<K: CpuKernel>` and friends inside the
// fuzz_exports-public `huff0` module name `crate::cpu_kernel::CpuKernel`
// in their signatures. Without a publicly-reachable path to `CpuKernel`
// the bound triggers `private_bounds` / `private_interfaces`. Re-export
// under the same feature gate so the fuzz harness build is clean.

/// Name of the active CPU kernel tier (entropy / sequence hot paths) for this
/// process — for diagnostics and benchmark/dashboard reporting. See
/// [`cpu_kernel::active_cpu_kernel_name`].
pub use crate::cpu_kernel::active_cpu_kernel_name;

pub(crate) mod fse;

pub(crate) mod huff0;

pub mod io_std;

pub use io_std as io;

/// SIMD wildcopy overshoot slack carried by every decoder backend
/// (currently **32 bytes**). Sized so the AVX2 chunked kernel in
/// `simd_copy::copy_bytes_overshooting` (32-byte stride on x86-64) can
/// fire on tail copies near the end of a fixed-capacity output buffer.
/// Upstream zstd's `WILDCOPY_OVERLENGTH` is also 32 bytes today; this
/// matches that contract.
///
/// Public so callers sizing an output slice for
/// [`crate::decoding::FrameDecoder::decode_all`] can size
/// `frame_content_size + WILDCOPY_OVERLENGTH` symbolically without
/// duplicating the value. Use the const reference rather than a
/// hardcoded literal — `simd_copy::copy_bytes_overshooting` already
/// ships an AVX-512 64-byte chunked kernel, and the slack may grow
/// further to reliably enable that wider kernel at buffer tails
/// (mirroring how the bump from 16 → 32 enabled the AVX2 32-byte
/// kernel at the tail).
pub const WILDCOPY_OVERLENGTH: usize = decoding::buffer_backend::WILDCOPY_OVERLENGTH;
