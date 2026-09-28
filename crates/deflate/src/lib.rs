// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-deflate` — PurRDF's native DEFLATE (RFC 1951) and gzip (RFC 1952).
//!
//! | Module | Does |
//! |---|---|
//! | [`inflate`] | Push-based DEFLATE decoding: [`Inflater::feed`] takes input in chunks of any size |
//! | [`deflate`] | Deterministic DEFLATE encoding at [`Level`] 0–9 |
//! | [`gzip`] | Member framing: [`GzipDecoder`] (push), [`GzipReader`] (`Read`), [`GzipWriter`] (`Write`) |
//!
//! # Decoding
//!
//! The decoder is push-based so the same core serves a `Read` adapter, a
//! one-shot call and a JavaScript stream callback on wasm32: it never blocks
//! and reports how much input it took and output it gave. Every gzip member is
//! decoded and verified (CRC-32 through `purrdf-hash`, `ISIZE` modulo 2^32);
//! bytes after the last member that do not begin another are refused. A limit
//! on the decoded bytes refuses decompression bombs with a typed
//! [`Error::LimitExceeded`] while holding a bounded amount of memory.
//!
//! # Encoding
//!
//! Output depends only on the input bytes, level, and explicit sync-flush
//! boundaries: no clock, no randomness, and the same bytes on every processor,
//! target and kernel path, however ordinary writes are split. The gzip header carries
//! `MTIME = 0`, `XFL = 0` and `OS = 255`. Byte identity with any other encoder
//! is not a goal.
//!
//! # Vector paths
//!
//! Match copies in the decoder and match-length compares and window hashing in
//! the encoder run on SSE2 or AVX2 (x86_64, AVX2 detected at run time), NEON
//! (aarch64) or simd128 (wasm32 built with `+simd128`), and portable code
//! otherwise. Every path returns exactly what the portable one does.

#![deny(unsafe_code)]

mod error;
mod huffman;
mod kernels;
mod tables;

// The only module allowed `unsafe`: the processor-specific kernels and their
// feature detection. Everything outside it is safe code.
#[allow(unsafe_code)]
mod arch;

#[doc(hidden)]
pub mod backend;
pub mod deflate;
pub mod gzip;
pub mod inflate;

pub use deflate::{Deflater, Level};
pub use error::{Alphabet, Error};
pub use gzip::{GzipDecoder, GzipReader, GzipWriter};
pub use inflate::{Inflater, Progress, Status};
