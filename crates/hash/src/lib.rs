// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-hash` — PurRDF's native, zero-dependency digests and table hasher.
//!
//! | Module | Algorithm | Specification | Output |
//! |---|---|---|---|
//! | [`blake3`] | Unkeyed BLAKE3-256 | BLAKE3 specification | 32 bytes |
//! | [`md5`] | MD5 | RFC 1321 | 16 bytes |
//! | [`sha1`] | SHA-1 | FIPS 180-4 | 20 bytes |
//! | [`sha3`] | SHA3-224, SHA3-256, SHA3-384, SHA3-512 and Keccak-f\[1600\] | FIPS 202 | 28 / 32 / 48 / 64 bytes |
//! | [`crc32`] | CRC-32/ISO-HDLC | the reflected polynomial `0xEDB88320` | a `u32` |
//! | [`fixed`] | the fixed-key table hasher | this crate (folded multiplies; AES rounds on AES builds) | a `u64` |
//! | [`fnv`] | FNV-1a, 64-bit | Fowler, Noll and Vo (`draft-eastlake-fnv`) | a `u64` |
//! | [`mix`] | the SplitMix64 generator and finaliser | Steele, Lea and Flood (OOPSLA 2014) | a `u64` |
//!
//! [`frame`] is the workspace's length framing: a variable-length field as its
//! length in eight little-endian bytes and then its bytes, appended to a
//! buffer ([`frame::frame_le`]) or streamed into a [`Digest`]
//! ([`frame::frame_le_into`]).
//!
//! [`Domain`] is the one spelling of a hash domain-separation string: every
//! domain the workspace hashes under is a registered `Domain` constant, unique
//! and prefix-free across the workspace, and never renamed once published.
//!
//! [`hex`] is the workspace's base16 (RFC 4648 §8) codec: [`hex::Lower`] and
//! [`hex::Upper`] render any byte string through `Display` without
//! allocating, [`hex::decode`] and its canonical-lowercase siblings read it
//! back with a typed error, [`hex::nibble`] reads one digit, and
//! [`hex::Digest32`] is the 32-byte value every content identity wraps.
//!
//! Every hasher has a one-shot associated function (`Md5::digest(data)`) and a
//! streaming form (`new`, `update`, `finalize`); both give the same answer for
//! every way of splitting the input. Every hasher also implements the
//! object-safe [`Digest`] trait, so a caller can choose an algorithm at run
//! time and drive it through `&mut dyn Digest`.
//!
//! # Hardware paths
//!
//! SHA-1 runs on the x86 SHA extensions or the Armv8 SHA1 instructions, and
//! CRC-32 on `pclmulqdq` carry-less-multiply folding or the Armv8 CRC32
//! instructions, whenever the processor reports them at run time; otherwise
//! the portable code runs. Every path computes the same bytes. MD5 is a single
//! serial dependency chain and SHA-3's Keccak-f\[1600\] needs 64-bit lane
//! rotates most vector units lack, so both are portable scalar code. Base16
//! encoding is a compare-select loop the compiler packs on every vector
//! target; inputs longer than a digest run SSSE3 `pshufb` on x86-64 below
//! AVX-512BW, NEON `tbl` on AArch64, and wasm `i8x16.swizzle` when the build
//! enables `simd128`.
//!
//! The table hasher [`fixed::FixedHasher`] is the exception to run-time
//! selection. It uses an AES accumulator when the *build's target* enables
//! AES (x86-64 or little-endian AArch64).
//! Otherwise it uses folded multiplies, on wasm32 and 32-bit targets too.
//! Each build therefore has exactly one table-hash function, and that
//! function is not the same on every build. A table hash is never persisted.
//!
//! # Scope
//!
//! Pure integer arithmetic over caller-supplied bytes: no threads, no
//! filesystem, no clock and no entropy (the table hasher's keys are
//! compile-time constants), so the crate builds for `wasm32-unknown-unknown`.
//! Hashing never allocates; the base16 functions that return an owned
//! `String` or `Vec<u8>` ([`hex::encode`], [`hex::decode`], …) allocate that
//! value and nothing else. The one exception is test-harness support, never
//! reached by a hashing path: [`dispatch`]'s requirement check reads the
//! `PURRDF_REQUIRE_SIMD_PATHS` environment variable and Linux
//! `/proc/cpuinfo`.
//!
//! The crate has zero dependencies and is the root of the workspace's crate
//! layering: every crate may depend on it. Besides the digests it holds the
//! small specified kernels shared across the workspace ([`fnv`], [`mix`],
//! [`frame`]) and the [`Backend`] trait every family of named execution paths implements. MD5 and SHA-1 are provided because protocols
//! name them (SPARQL's `MD5()`/`SHA1()`, OpenPGP v4 fingerprints), not as
//! security primitives: both are broken for collision resistance.

#![deny(unsafe_code)]

mod block;
mod digest;

// The only module allowed `unsafe`: the processor-specific kernels, their
// feature detection, their vector loads and the table hasher's AES block. Everything outside it is safe code.
#[allow(unsafe_code)]
mod arch;

#[doc(hidden)]
pub mod backend;
pub mod blake3;
pub mod crc32;
pub mod dispatch;
mod domain;
pub mod fixed;
pub mod fnv;
pub mod frame;
pub mod hex;
pub mod md5;
pub mod mix;
pub mod sha1;
pub mod sha3;

pub use digest::{Digest, MAX_OUTPUT_LEN};
pub use dispatch::Backend;
pub use domain::Domain;
