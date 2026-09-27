<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-hash` — Zero-Dependency MD5, SHA-1, SHA-3, CRC-32 and a Fixed-Key Table Hasher

[![crates.io](https://img.shields.io/crates/v/purrdf-hash.svg)](https://crates.io/crates/purrdf-hash)
[![docs.rs](https://docs.rs/purrdf-hash/badge.svg)](https://docs.rs/purrdf-hash)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-hash` is the PurRDF toolkit's digest leaf: the SPARQL `MD5()`,
`SHA1()` and `SHA3-*()` built-ins, OpenPGP v4 key fingerprints and Datalog
derivation identities all compute through it, and `fixed::FixedHasher` is
the workspace's in-memory table hasher. It has **no runtime dependencies**,
allocates nothing, and builds for `wasm32-unknown-unknown`.

| Module | Algorithm | Specification | Output |
|---|---|---|---|
| `md5` | MD5 | RFC 1321 | 16 bytes |
| `sha1` | SHA-1 | FIPS 180-4 | 20 bytes |
| `sha3` | SHA3-224 / 256 / 384 / 512, Keccak-f[1600] | FIPS 202 | 28 / 32 / 48 / 64 bytes |
| `crc32` | CRC-32/ISO-HDLC | reflected `0xEDB88320`, init and xorout `0xFFFFFFFF` | `u32` |

## Usage

```rust
use purrdf_hash::md5::Md5;
use purrdf_hash::sha1::Sha1;
use purrdf_hash::sha3::Sha3_256;
use purrdf_hash::crc32::Crc32;

// One shot.
let digest: [u8; 32] = Sha3_256::digest(b"abc");
assert_eq!(digest.len(), 32);
let md5: [u8; 16] = Md5::digest(b"abc");
assert_eq!(md5[..4], [0x90, 0x01, 0x50, 0x98]);
assert_eq!(Crc32::checksum(b"123456789"), 0xCBF4_3926);

// Streaming: any split of the input gives the same answer.
let mut sha1 = Sha1::new();
sha1.update(b"ab");
sha1.update(b"c");
assert_eq!(sha1.finalize(), Sha1::digest(b"abc"));
```

Every hasher also implements the object-safe `Digest` trait, so an algorithm
chosen at run time can be driven through `&mut dyn Digest`.

## Execution paths

| Algorithm | Paths | Selected when |
|---|---|---|
| SHA-1 | x86 SHA extensions (`sha1rnds4`, `sha1nexte`, `sha1msg1/2`) | x86-64 with `sha`, `ssse3`, `sse4.1` |
| | Armv8 SHA1 instructions (`sha1c/p/m`, `sha1h`, `sha1su0/1`) | AArch64 with SHA1 |
| | portable | otherwise |
| CRC-32 | `pclmulqdq` folding, 64 bytes per step, Barrett reduction | x86-64 with `pclmulqdq`, `sse4.1` |
| | Armv8 CRC32 instructions | AArch64 with CRC32 |
| | Armv8 `pmull` folding finished by CRC32 | never (available to tests and the bench; not yet measured on Arm hardware) |
| | slicing-by-16 tables | otherwise |
| MD5 | portable (a strict serial dependency chain) | always |
| SHA-3 | portable Keccak-f[1600] | always |

Paths are chosen by run-time CPU detection, and every path computes the same
bytes: the test suite replays 14,097 frozen answers per algorithm against
each path the host can run, natively and on wasm32. Every constant — MD5's
sine table, SHA-3's round constants, rotation offsets and lane permutation,
the CRC tables and the carry-less folding and Barrett constants — is computed
at compile time from its definition rather than typed.

All `unsafe` code lives in one private module (the processor kernels, their
detection, their vector loads and the table hasher's AES block); the rest of
the crate is
`#![deny(unsafe_code)]`.

MD5 and SHA-1 are here because protocols name them, not as security
primitives: both are broken for collision resistance.

## Fixed-key table hasher

`fixed::FixedHasher` is a `Hasher`, and `fixed::FixedState` is the
`BuildHasher` for `HashMap<K, V, FixedState>`. The keys are compile-time
constants derived from `⌊2^64/φ⌋`, with no run-time seeding, so a build
hashes equal inputs equally on every run.

```rust
use core::hash::BuildHasher;
use std::collections::HashMap;
use purrdf_hash::fixed::FixedState;

let mut map: HashMap<&str, u32, FixedState> = HashMap::with_hasher(FixedState::new());
map.insert("http://example.org/p", 1);
assert_eq!(FixedState::new().hash_one(7u32), FixedState::new().hash_one(7u32));
```

| Input | Function |
|---|---|
| integers, and byte slices of 0–16 bytes | folded multiplies (the low and high halves of a 128-bit product XOR-ed), every build |
| byte slices over 16 bytes | four AES-round lanes when the build's target enables AES (x86-64, little-endian AArch64); four folded-multiply lanes otherwise |

The choice between the two is made at compile time, never at run time. On
32-bit targets (i686, wasm32) the 128-bit product is built from 32-bit
multiplies with identical output. The AES function was added because the
bench (`benches/hasher.rs`) measured it faster on every length class where
the two functions differ. Both functions are pinned by frozen self-vectors,
which are replayed natively, on i686 and on wasm32. The test suite also
measures avalanche, collisions and χ² uniformity.

Because the function depends on the build, a table hash must never be
persisted, sent over the wire or used as a content address. Use a digest
for any of those. The keys are public, so the hasher offers no resistance
to deliberately colliding input.

## License

`MIT OR Apache-2.0 OR MulanPSL-2.0`.
