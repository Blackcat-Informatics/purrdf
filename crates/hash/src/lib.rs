// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-hash` — PurRDF's native, zero-dependency digests.
//!
//! | Module | Algorithm | Specification | Output |
//! |---|---|---|---|
//! | [`md5`] | MD5 | RFC 1321 | 16 bytes |
//! | [`sha1`] | SHA-1 | FIPS 180-4 | 20 bytes |
//! | [`sha3`] | SHA3-224, SHA3-256, SHA3-384, SHA3-512 and Keccak-f\[1600\] | FIPS 202 | 28 / 32 / 48 / 64 bytes |
//! | [`crc32`] | CRC-32/ISO-HDLC | the reflected polynomial `0xEDB88320` | a `u32` |
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
//! rotates most vector units lack, so both are portable scalar code.
//!
//! # Scope
//!
//! Pure integer arithmetic over caller-supplied bytes: no allocation, no
//! threads, no filesystem, no clock and no entropy, so the crate builds for
//! `wasm32-unknown-unknown`. MD5 and SHA-1 are provided because protocols
//! name them (SPARQL's `MD5()`/`SHA1()`, OpenPGP v4 fingerprints), not as
//! security primitives: both are broken for collision resistance.

#![deny(unsafe_code)]

mod block;
mod digest;

// The only module allowed `unsafe`: the processor-specific kernels, their
// feature detection and their vector loads. Everything outside it is safe code.
#[allow(unsafe_code)]
mod arch;

#[doc(hidden)]
pub mod backend;
pub mod crc32;
pub mod md5;
pub mod sha1;
pub mod sha3;

pub use digest::{Digest, MAX_OUTPUT_LEN};
