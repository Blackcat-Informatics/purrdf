<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent pure ML-DSA-65 known answers

The National Institute of Standards and Technology is the source of these
frozen input/output bytes. They were imported on 2026-10-06 from the official
[ACVP-Server tree](https://github.com/usnistgov/ACVP-Server/tree/975de31eb83d87039ec88934fdc47d8c312b892d/gen-val/json-files)
at commit `975de31eb83d87039ec88934fdc47d8c312b892d`. The accompanying
[NIST-NOTICE.txt](NIST-NOTICE.txt) retains the complete upstream license notice
from that commit's README, with trailing whitespace removed and the wording
and paragraph order unchanged. The format was changed from JSON into checksummed,
tab-separated Rust testkit vector records; input and answer bytes were retained
unchanged apart from lowercase base16 spelling. No expected result was generated
with PurRDF. No external implementation bodies or coefficient tables were read
or copied into this implementation.

The source files are `prompt.json` and `expectedResults.json` under each named
directory in `gen-val/json-files`. Prompt/results are joined by `tgId` and
`tcId`; expected-result groups do not repeat every prompt metadata field.

| Directory / source | SHA-256 |
|---|---|
| ML-DSA-keyGen-FIPS204/prompt.json | `43e81ad820e495dbcad086fe27c1008393a8c32100bbbff77c558c3f06dcefef` |
| ML-DSA-keyGen-FIPS204/expectedResults.json | `361f47ca19d592adcc66ff2cb591686ad785fea157b295648738bed6921a68df` |
| ML-DSA-sigGen-FIPS204/prompt.json | `447749d72817b211160d243311ce32302f3023e59c355b0f70be2bd3e9e7830d` |
| ML-DSA-sigGen-FIPS204/expectedResults.json | `228d011bbe274aeb93e22eea1e0d57b78f43795cf6a64fb5ef1e626485a0bedb` |
| ML-DSA-sigVer-FIPS204/prompt.json | `e2cba4589389756fa0bea1a7e6837138bf0a81f9d14234c9ee8f6d33caa1654e` |
| ML-DSA-sigVer-FIPS204/expectedResults.json | `e1d84ef1b2f35196278ab0b0ed6a46ec62cc03d2dfa92c564199e1999bfb8ea6` |

The shared vector reader verifies each body checksum and record count. Tests
compare complete answers and execute the same public APIs natively and in wasm.
These files are primitive known answers, separate from the governed GTS corpus.

| File | Selected groups and complete records | Fields after group/case IDs |
|---|---|---|
| keyGen.txt | Group 2; 25 ML-DSA-65 cases | 32-byte seed, 1952-byte public key, 4032-byte expanded secret key |
| sigGen.txt | Pure external groups 3 and 15; 15 deterministic + 15 hedged cases | 4032-byte secret key, message, context, 32-byte randomizer, 3309-byte signature |
| sigVer.txt | Pure external group 3; 15 cases, 3 valid and 12 invalid | 1952-byte public key, message, context, signature, expected Boolean |

Deterministic group 3 receives the FIPS all-zero 32-byte randomizer; hedged group
15 carries the prompt's exact `rnd`. Empty messages/contexts use the vector
reader's empty-field encoding. PreHash groups, including verification group 4,
implement HashML-DSA and are excluded: this public primitive implements pure
ML-DSA with `0x00 || context_length || context || message`. Internal-interface
groups are also excluded from this external-interface coverage claim.

The implementation is original Rust derived from
[FIPS 204](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf), Algorithms
6–8 and 16–43 (PDF SHA-256
`57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b`).
NTT roots are computed from `1753^BitRev8(i) mod 8380417` in a const function;
there is no pasted root table. Keccak/SHAKE uses the existing `purrdf-hash`
implementation. Safe secret overwriting and byte comparison call the existing
Ed25519 helper homes. The controlled clearing boundary and variable-time
FIPS rejection loops are documented on `purrdf_gts::mldsa65`.
