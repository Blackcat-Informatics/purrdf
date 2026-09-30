<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored Wycheproof Ed25519 verification vectors

Frozen copy of the upstream Project Wycheproof EdDSA verification vectors,
vendored for `crates/ed25519/tests/wycheproof.rs`. **Do not hand-edit**: the
freeze is enforced by `scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-wycheproof.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-wycheproof-ed25519.py`.

## Source

- Upstream: <https://github.com/C2SP/wycheproof>
- Pinned commit: `3fa63dd0344abb611f1fb1d77e119938603ea230`
- License: **Apache-2.0** (the upstream `LICENSE`, vendored verbatim)
- Retrieval: `https://raw.githubusercontent.com/C2SP/wycheproof/3fa63dd0344abb611f1fb1d77e119938603ea230/<path>`, each
  file checked against the SHA-256 pinned in the script.

| Vendored file | Upstream path | SHA-256 |
|---|---|---|
| `ed25519_test.json` | `testvectors_v1/ed25519_test.json` | `752d2ea7d7c6cf4736381b6cbacb61f8182b126ab7cd9b058f00c50084975536` |
| `LICENSE` | `LICENSE` | `58d1e17ffe5109a7ae296caafcadfdbe6a7d176f0bc4ab01e12a689b0499d8bd` |

## Contents

`ed25519_test.json`: schema `eddsa_verify_schema_v1.json`, 151 tests in 78
groups (one public key per group): 88 `valid` and 63 `invalid` cases. The
upstream file has no `acceptable` results and carries no private keys, so it
grades verification only.
