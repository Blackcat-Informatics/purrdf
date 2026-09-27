<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of `purrdf-hash`

Everything under `src/`, `tests/*.rs` and `benches/` is first-party, written
from the specifications below. No third-party hash implementation was read,
ported or consulted, in any language.

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
- **Instruction semantics** — the Rust `core::arch` reference
  (doc.rust-lang.org) for the x86 SHA and `pclmulqdq` intrinsics and the
  AArch64 SHA1, CRC32 and PMULL intrinsics.

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
