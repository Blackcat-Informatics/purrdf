<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Official ML-KEM-1024 known answers

The National Institute of Standards and Technology supplied these frozen input
and expected-output bytes through its official ACVP-Server repository. They
were imported on 2026-10-31 from commit
`975de31eb83d87039ec88934fdc47d8c312b892d`. No expected answer was computed with
PurRDF. No third-party implementation code or coefficient tables were fetched
or copied.

The exact source bytes were checked against the pinned GitHub blob identities.
A Rust extraction tool joined prompt
and expected-result records by `tgId` and `tcId`, checked unique identities and
complete group membership, and wrote the existing
`purrdf_testkit::vectors::Recorder` format. The existing `VectorFile` reader
verified each emitted record count and body checksum. Hexadecimal fields differ
only in letter case; all input and expected bytes are unchanged.

All ML-KEM-1024 groups in both source directories are included: 80 complete
records across five groups. ML-KEM-512 and ML-KEM-768 groups are excluded.
Group and case IDs occupy the first two fields of every record.

| Fixture | Source group | Records | Fields after the IDs |
|---|---|---|---|
| `keyGen.txt` | keyGen 3 | 25 | 32-byte `d`, 32-byte `z`, 1568-byte `ek`, 3168-byte `dk` |
| `encapsulation.txt` | encapDecap 3 | 25 | 1568-byte `ek`, 32-byte `m`, 1568-byte `c`, 32-byte `k` |
| `decapsulation.txt` | encapDecap 6 | 10 | 3168-byte `dk`, 1568-byte `c`, 32-byte expected `k` |
| `decapsulationKeyCheck.txt` | encapDecap 11 | 10 | 3168-byte `dk`, expected Boolean: five accept and five reject |
| `encapsulationKeyCheck.txt` | encapDecap 12 | 10 | 1568-byte `ek`, expected Boolean: five accept and five reject |

The key-generation seed is `d || z`, in that order. Encapsulation consumes the
prompt's exact `m`. Decapsulation cases retain the expected shared secret
verbatim; ACVP supplies no separate acceptance Boolean for this group. The two
key-check groups preserve every supplied rejection outcome.

The source files are pinned below. SHA-256 covers each original file, including
its original line endings and final-newline convention.

| Source | SHA-256 |
|---|---|
| [ML-KEM-keyGen-FIPS203/prompt.json](https://github.com/usnistgov/ACVP-Server/blob/975de31eb83d87039ec88934fdc47d8c312b892d/gen-val/json-files/ML-KEM-keyGen-FIPS203/prompt.json) | `3f9ce34f6c836c77958bad2729e837c3b213f44ac36c3065976e7acca6389523` |
| [ML-KEM-keyGen-FIPS203/expectedResults.json](https://github.com/usnistgov/ACVP-Server/blob/975de31eb83d87039ec88934fdc47d8c312b892d/gen-val/json-files/ML-KEM-keyGen-FIPS203/expectedResults.json) | `a253d0ad91c95ebea5b409673defef0aa49d65d4ed72286399e2e798ddf073a4` |
| [ML-KEM-encapDecap-FIPS203/prompt.json](https://github.com/usnistgov/ACVP-Server/blob/975de31eb83d87039ec88934fdc47d8c312b892d/gen-val/json-files/ML-KEM-encapDecap-FIPS203/prompt.json) | `998e22dfb12efb14ce9fdff911ca634b13612819a1806f25da69adba7e16db91` |
| [ML-KEM-encapDecap-FIPS203/expectedResults.json](https://github.com/usnistgov/ACVP-Server/blob/975de31eb83d87039ec88934fdc47d8c312b892d/gen-val/json-files/ML-KEM-encapDecap-FIPS203/expectedResults.json) | `9089ec6ff2424da9f2782b89b2f831a329a3e28d6e5e24b802b78ff36ac61cdf` |
| [Upstream README](https://github.com/usnistgov/ACVP-Server/blob/975de31eb83d87039ec88934fdc47d8c312b892d/README.md) | `d5a569884ee83bd1c4737042d0a2cc7d68c6950690f75a73ef14f505a9aa3555` |

[NIST-NOTICE.txt](NIST-NOTICE.txt) preserves the complete license notice from
the pinned README, with trailing whitespace removed and wording and paragraph
order unchanged. It is identical to the notice accompanying the existing
ML-DSA-65 vectors from the same upstream commit.

The specification is [FIPS 203](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.203.pdf).
The supplied PDF has SHA-256
`fe1f12f32a7e44ec9fdebbf400cda843a40b506dee676725234dc6f7923b6cac`.
