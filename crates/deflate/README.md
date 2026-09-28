<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-deflate` — Native DEFLATE and gzip

[![crates.io](https://img.shields.io/crates/v/purrdf-deflate.svg)](https://crates.io/crates/purrdf-deflate)
[![docs.rs](https://docs.rs/purrdf-deflate/badge.svg)](https://docs.rs/purrdf-deflate)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-deflate` is the PurRDF toolkit's DEFLATE (RFC 1951) and gzip (RFC 1952)
leaf: the GTS `gzip` transform, gzip tar streams and the `.gz` transport
decoder all run through it. Its one runtime dependency is `purrdf-hash` (the
CRC-32 of the gzip trailer), and it builds for `wasm32-unknown-unknown`.

## Usage

```rust
use std::io::Read;

use purrdf_deflate::{GzipReader, Level, gzip};

let framed = gzip::compress(b"<http://example.org/s> <http://example.org/p> \"o\" .\n", Level::DEFAULT);
assert_eq!(&framed[..2], &[0x1f, 0x8b]);

// One shot, every member decoded, with a limit on the decoded size.
let payload = gzip::decompress_with_limit(&framed, 1 << 20).expect("valid gzip");

// Streaming, as a `Read`.
let mut streamed = Vec::new();
GzipReader::new(framed.as_slice()).read_to_end(&mut streamed).expect("valid gzip");
assert_eq!(streamed, payload);
```

The decoder core is push-based: `Inflater::feed` and `GzipDecoder::feed` take
input in chunks of any size and report the input consumed, the output written,
and whether they need more input or more output room. That is what a
JavaScript stream callback needs on wasm32, and the `Read` adapter is a thin
loop over it.

`Inflater::feed_to_vec` and `GzipDecoder::feed_to_vec` append directly to a
caller-owned `Vec`. They preserve any existing prefix and report only the
bytes appended by that call. The gzip path updates CRC-32 from accepted
bytes in the inflater's window and verifies every trailer; it needs no
intermediate output buffer. Reserve capacity when the caller knows the
output bound. The bounded slice API remains useful when output storage is
fixed or the consumer applies backpressure.

Decoder windows and Huffman table scratch are reused between members.
Trailer lookahead occupies a fixed seven-byte array. Once those tables are
warm, decoding the same member into reserved output performs zero heap
allocations, enforced by an allocation-counter test. Convenience functions
still allocate their returned `Vec` and decoder storage; encoder output
starts with at most 64 KiB plus framing slack and grows as needed.

## Guarantees

- **Every gzip member is decoded** and its trailer verified: CRC-32 and
  `ISIZE`, the latter modulo 2^32, so members over 4 GiB decode. Bytes after
  the last member that do not begin another are refused, never ignored.
- **Header discipline** (RFC 1952 §2.3.1.2): `CM` must be 8, the reserved `FLG`
  bits must be clear, `FHCRC` is verified; `FEXTRA`, `FNAME` and `FCOMMENT`
  are parsed and skipped without being stored.
- **Bounded**: a limit on decoded bytes refuses a decompression bomb at the
  first byte past it, with a typed `Error::LimitExceeded`; the decoder holds
  about 128 KiB whatever the input.
- **Deterministic encoding**: output depends only on the input bytes and the
  level — no clock, no randomness, the same bytes on every processor, target
  and kernel path, however the input is split across writes. The gzip header
  is `MTIME` 0, `XFL` 0, `OS` 255. Byte identity with any other encoder is not
  a goal.
- **Typed refusals**: every malformed stream is an `Error` variant naming what
  was wrong (reserved block type, `LEN`/`NLEN` mismatch, over-subscribed code,
  distance before the stream start, symbols 286/287 or distance 30/31, …).

## Execution paths

| Kernel | x86_64 | aarch64 | wasm32 | otherwise |
|---|---|---|---|---|
| Decoder match copy (overlapping, wide stores) | SSE2 16-byte, AVX2 32-byte | NEON 16-byte | simd128 16-byte (`+simd128` builds) | portable |
| Encoder match length (vector compare + trailing zeros) | SSE2, AVX2 | NEON | simd128 | 8-byte XOR |
| Encoder window hashing (byte shuffle + lane multiply) | AVX2 | NEON | simd128 | portable |

AVX2 is selected by run-time detection; every path returns exactly what the
portable one does, which the test suite checks on each path the host can run,
natively and on wasm32. All `unsafe` code lives in one private module; the
rest of the crate is `#![deny(unsafe_code)]`.

## License

`MIT OR Apache-2.0 OR MulanPSL-2.0`.
