<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Frozen SHAKE answers

These are independent output bytes, never implementation code. Inputs are
byte-aligned: zero bytes for `msg0`, 200 copies of `A3` for `msg1600`, and
the indicated number of copies of `A3` for the boundary records. Every record
contains the complete output in lowercase base16. The shared Rust vector
reader verifies the record count and SHA-256 body checksum before replay.

`shake_nist_vectors.txt` contains all 512 output bytes from each of four
NIST FIPS 202 example PDFs, captured on 2026-10-06. Only their final
`Output val is` block was extracted; intermediate states were not imported.
NIST's published examples are US government works, public domain.

| Source | Original PDF SHA-256 |
|---|---|
| [SHAKE128, empty](https://csrc.nist.gov/csrc/media/projects/cryptographic-standards-and-guidelines/documents/examples/shake128_msg0.pdf) | `3013d904e0f6cebc8dad6393cbf3eed4d1a537b3b3dc5e7b48552ff597942974` |
| [SHAKE128, 1600 bits](https://csrc.nist.gov/csrc/media/projects/cryptographic-standards-and-guidelines/documents/examples/shake128_msg1600.pdf) | `437f905d9790ce7a18625b396ba57c6118bb46d1542ec3cf1d46faac23c261ef` |
| [SHAKE256, empty](https://csrc.nist.gov/csrc/media/projects/cryptographic-standards-and-guidelines/documents/examples/shake256_msg0.pdf) | `d736c1a93eb6440e1e6b640402e31ea5258281b2b9237f84ea0ba4186518fc59` |
| [SHAKE256, 1600 bits](https://csrc.nist.gov/csrc/media/projects/cryptographic-standards-and-guidelines/documents/examples/shake256_msg1600.pdf) | `f4226f5c72914e5d2b331812c5973d8d685c8af1ccb97c2a3a2fd1c02fd173d1` |

`shake_boundary_vectors.txt` supplements these examples with independent
answers captured through OpenSSL 3.6.4's public `dgst` command on 2026-10-06
(library also 3.6.4, built 2026-08-26, Linux x86-64). For each strength, the
input lengths are `rate - 1`, `rate`, `rate + 1`, `2*rate - 1`, `2*rate`,
`2*rate + 1` and `3*rate`; their outputs have `3*rate + 17` bytes. An
additional 17-byte input produces 4097 output bytes. These cover padding's
shared suffix/final-bit byte, exact blocks, the first byte after a block,
multiple absorbed blocks and repeated squeeze permutations.

The capture fed the input bytes on standard input to
`openssl dgst -shake128 -xoflen N -binary` or
`openssl dgst -shake256 -xoflen N -binary`, then encoded standard output as
base16. OpenSSL is not a build/test/runtime dependency: committed tests read
the frozen files using the existing Rust vector and base16 homes. No OpenSSL
source, implementation body or constants were read or copied. OpenSSL is
licensed Apache-2.0; these files hold cryptographic answers only.
