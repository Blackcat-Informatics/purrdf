// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-hash` — PurRDF's native, zero-dependency digests and table hasher.
//!
//! | Module | Algorithm | Specification | Output |
//! |---|---|---|---|
//! | [`md5`] | MD5 | RFC 1321 | 16 bytes |
//! | [`sha1`] | SHA-1 | FIPS 180-4 | 20 bytes |
//! | [`sha3`] | SHA3-224, SHA3-256, SHA3-384, SHA3-512 and Keccak-f\[1600\] | FIPS 202 | 28 / 32 / 48 / 64 bytes |
//! | [`crc32`] | CRC-32/ISO-HDLC | the reflected polynomial `0xEDB88320` | a `u32` |
//! | [`fixed`] | the fixed-key table hasher | this crate (folded multiplies; AES rounds on AES builds) | a `u64` |
//!
//! [`hex::Lower`] renders any byte string, a digest included, as lowercase
//! base16 (RFC 4648 §8) through `Display`, without allocating.
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
//! encoding runs on SSSE3 when detected, on NEON, or on wasm `simd128` when
//! the build enables it.
//!
//! The table hasher [`fixed::FixedHasher`] is the exception to run-time
//! selection. It hashes byte slices longer than 16 bytes with AES rounds
//! when the *build's target* enables AES (x86-64 or little-endian AArch64).
//! Otherwise it uses folded multiplies, on wasm32 and 32-bit targets too.
//! Each build therefore has exactly one table-hash function, and that
//! function is not the same on every build. A table hash is never persisted.
//!
//! # Scope
//!
//! Pure integer arithmetic over caller-supplied bytes: no allocation, no
//! threads, no filesystem, no clock and no entropy (the table hasher's keys
//! are compile-time constants), so the crate builds for
//! `wasm32-unknown-unknown`. MD5 and SHA-1 are provided because protocols
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
pub mod crc32;
pub mod fixed;
pub mod hex;
pub mod md5;
pub mod sha1;
pub mod sha3;

pub use digest::{Digest, MAX_OUTPUT_LEN};
