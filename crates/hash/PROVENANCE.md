<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of `purrdf-hash`

Everything under `src/`, `tests/*.rs` and `benches/` is first-party. The
initial implementations were written from the specifications below without
consulting third-party implementations. Subsequent source consultation for
performance analysis is recorded by algorithm below; no third-party code
was copied or ported.

## Specifications

- **MD5** — RFC 1321, §2 (conventions), §3.1–§3.5 (padding, length, buffer
  initialisation, the four rounds, output) and §A.5 (the test suite). The
  reference C code of Appendix A.1–A.4 was not read. The `T[i]` table is
  computed from the §3.4 definition (`floor(2^32 · |sin(i)|)`).
- **SHA-1** — FIPS 180-4, §4.1.1 (functions), §4.2.1 (constants), §5.1.1
  (padding), §5.3.1 (initial hash value), §6.1.2 and §6.1.3 (hash
  computation, circular message schedule).
- **SHA-3** — FIPS 202, §3.1 (state array), §3.2.1–§3.2.5 (θ, ρ, π, χ, ι,
  including Algorithms 2, 3, 5 and 6 from which the rotation offsets, lane
  permutation and round constants are computed), §3.3, §5.1 (pad10*1), §6.1
  (the four hash functions) and Appendix B (bit/byte conventions and the
  hexadecimal form of the padding).
- **CRC-32/ISO-HDLC** — the parameter set: polynomial `0x04C11DB7` processed
  reflected (`0xEDB88320`), register initialised to `0xFFFFFFFF`, output
  complemented, check value `0xCBF43926` (the CRC-32 of RFC 1952 §8). The
  lookup tables, the carry-less folding constants and the Barrett constant
  are derived from the polynomial by GF(2) polynomial arithmetic in this
  crate. The folding technique is the one Gopal et al., "Fast CRC Computation
  for Generic Polynomials Using PCLMULQDQ Instruction" (Intel, 2009)
  describes; the derivation here (the bit-reflected operand frame, the fold
  distances, the reduction to 64 bits and Barrett's quotient) was worked out
  independently from GF(2) polynomial arithmetic, and no code listing of any
  kind was used.
- **Fixed-key table hasher (`fixed`)** — designed from first principles for
  this crate. The sources are: arithmetic facts about the folded multiply
  (an odd multiplier makes the low half of the product a bijection, and the
  high half depends on every input bit); the golden ratio, whose 64-bit
  fraction `⌊2^64/φ⌋` is pinned exactly in a test by `x² + x = 1`; and the
  AES round as the `aesenc`, `AESE` and `AESMC` instructions define it. No
  third-party non-cryptographic hash implementation was consulted during
  the initial design. The design, the key schedule and the measured decision
  to ship an AES path are recorded in
  `src/fixed.rs`, `src/fixed/*.rs` and `benches/hasher.rs`.
  The terminal tagged-slice operation was added for PurRDF's primary IRI
  index. Its own avalanche, sparse-key, corpus and distribution tests cover
  both the portable and AES functions; it is not a content identity.
  Subsequent performance analysis consulted the update/finalization methods
  and primitive specialization in ahash 0.8.12's
  [`src/aes_hash.rs`](https://github.com/tkaitchuck/aHash/blob/v0.8.12/src/aes_hash.rs)
  and the method inventory in `src/fallback_hash.rs`. This confirmed the
  distinction between an AES accumulator design and PurRDF's folded-multiply
  state. No code was copied or ported, and the existing fixed-hasher function
  and its frozen self-vectors remain unchanged by that analysis.
- **Instruction semantics** — the Rust `core::arch` reference
  (doc.rust-lang.org) for the x86 SHA, `pclmulqdq` and `aesenc` intrinsics
  and the AArch64 SHA1, CRC32, PMULL, `vaeseq_u8` and `vaesmcq_u8`
  intrinsics.

## BLAKE3

`src/blake3.rs` and `src/arch/blake3_*.rs` are written from the
[BLAKE3 specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf),
§2.1–2.5: the tree shape, seven compression rounds, permutation, counters,
block lengths, and domain flags. Only the unkeyed 256-bit mode used by PurRDF
is exposed. The independent chunks and parent nodes are transposed into SIMD
lanes; the single-block path places the four independent quarter rounds in
four lanes. This initial implementation, including its pending-subtree stream
state and 256-chunk batch reduction, preceded external source consultation.

For subsequent performance analysis, the following files from **blake3 1.8.5**
were consulted: [`src/lib.rs`](https://github.com/BLAKE3-team/BLAKE3/blob/1.8.5/src/lib.rs)
(subtree batching and incremental update),
[`src/platform.rs`](https://github.com/BLAKE3-team/BLAKE3/blob/1.8.5/src/platform.rs)
(backend dispatch), and
[`c/blake3_avx512.c`](https://github.com/BLAKE3-team/BLAKE3/blob/1.8.5/c/blake3_avx512.c)
(the 16/8/4-input dispatch structure). The observation that AVX-512 supports
narrower batches motivates testing 128- and 256-bit kernels with native rotate
instructions. Their arithmetic and transposes are instantiated from PurRDF's
existing first-party lane implementation. No implementation code, assembly,
comments or tests from the original library are copied or ported. This source
consultation means the subsequent optimization work is not described as
clean-room.

`blake3_differential_vectors.txt` records 8,214 answers from the public
`blake3::hash` API of blake3 1.8.5, called in an external harness. It includes
every length through 8,192 bytes and power-of-two tree boundaries through
1 MiB. The input recipe and SHA-256 body checksum are in the file header.
`blake3_random_vectors.txt` adds 10,000 seeded byte streams with lengths
through 64 KiB, exercising irregular tree shapes and unaligned input.
Streaming splits, empty updates, snapshots and reset are checked against
these answers, which must never be changed to match the implementation.

## Known-answer values (`tests/known_answers.rs`)

- MD5: RFC 1321 §A.5.
- SHA-1: the NIST "Examples with Intermediate Values" document for SHA-1
  ("abc" and the 448-bit two-block message) and FIPS 180-2 Appendix A.3 (one
  million "a").
- SHA-3: the NIST FIPS 202 example-value documents for SHA3-224/256/384/512,
  0-bit and 1600-bit messages.
- CRC-32: the check value above.

## Frozen differential vectors (`tests/vectors/*_differential_vectors.txt`)

Seven files, 14,097 records each, recording what third-party crates answered
over a deterministic input set while they were dependencies of this
workspace, called as black boxes through their public APIs — **answers only,
not code**:

| File | Answers recorded from |
|---|---|
| `md5_differential_vectors.txt` | `md-5` 0.10.6 (`md5::Md5::digest`) |
| `sha1_differential_vectors.txt` | `sha1` 0.10.6 (`sha1::Sha1::digest`) |
| `sha3_{224,256,384,512}_differential_vectors.txt` | `sha3` 0.10.9 (`sha3::Sha3_*::digest`) |
| `crc32_differential_vectors.txt` | `crc32fast` 1.5.0 (`crc32fast::hash`) |

The source code of those crates was not consulted in writing this crate. Each
file's header states its input recipe (the little-endian `u64` stream of
`purrdf_testkit::rng::Xoshiro256`) and carries a `body-sha256` that
`purrdf_testkit::vectors` verifies before replay. A disagreement between a
file and this crate is a defect in this crate; the files are never edited to
match it.

## Fixed-hasher self-vectors (`tests/vectors/fixed_hasher_*_vectors.txt`)

These two files record what this crate's own portable and AES table-hash
functions answered when they were written. Their `oracle` header names
purrdf-hash itself. No third-party output is involved, and a replay proves
stability across targets and edits rather than correctness.
They include streaming `Hasher` operations and the terminal tagged-slice
operation used by the RDF interner.
`tests/fixed_hasher.rs` writes them when `PURRDF_RECORD_FIXED_HASHER=1` is
set.

`tests/vectors/corpus_iris.txt` holds input keys only: the 1,000 most
frequent IRIs of the repository's RDF test corpora.
