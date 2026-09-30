<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-ed25519` — Native Ed25519 signatures

[![crates.io](https://img.shields.io/crates/v/purrdf-ed25519.svg)](https://crates.io/crates/purrdf-ed25519)
[![docs.rs](https://docs.rs/purrdf-ed25519/badge.svg)](https://docs.rs/purrdf-ed25519)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-ed25519` is the PurRDF toolkit's one Ed25519 (RFC 8032, pure Ed25519:
no context, no pre-hash). The GTS COSE_Sign1 envelope, OpenPGP key import and
the RDF packaging signatures all sign and verify through it. Its runtime
dependencies are `purrdf-hash` (the base16 the `Debug` renderings print with)
and `sha2` (SHA-512, which RFC 8032 fixes as Ed25519's hash), it is
`#![forbid(unsafe_code)]`, and it builds for `wasm32-unknown-unknown`: it uses
no threads, no clock, no filesystem and no entropy, since keys come from
caller-supplied seeds.

## Usage

```rust
use purrdf_ed25519::{SigningKey, VerifyingKey};

let key = SigningKey::from_bytes(&[7u8; 32]);
let signature = key.sign(b"message");
let public = VerifyingKey::from_bytes(&key.verifying_key().to_bytes()).unwrap();
assert!(public.verify_strict(b"message", &signature).is_ok());
assert!(public.verify_strict(b"messagf", &signature).is_err());
```

Signing is deterministic (RFC 8032 5.1.6). A `SigningKey` overwrites its seed,
expanded scalar and nonce prefix when dropped, and its `Debug` prints only the
public key.

## Strictness policy

There is one verification rule, `VerifyingKey::verify_strict` (`verify` is the
same function). A (key, message, signature) triple is refused exactly when:

- the key does not decode (`VerifyingKey::from_bytes`): its y is not below
  p = 2^255 - 19, or it has no x, or it is x = 0 with the sign bit set;
- `S` is not below the group order `L` (no signature malleability);
- `R` does not decode, by the same three tests;
- the key or `R` has small order (one of the eight torsion points), which
  closes the small-order key forgery;
- the cofactorless equation fails: the canonical encoding of `S·B - k·A` must
  equal `R`'s bytes, with `k = SHA-512(R ‖ A ‖ M) mod L` over the bytes as
  received.

Points of mixed order are accepted as keys and as `R`; the cofactorless
equation then decides. Every refusal has a typed `SignatureError`, and every
refusal is paired in the tests with the neighbouring valid input that must still
be accepted.

## Side channels

Field arithmetic is radix 2^51 over five `u64` limbs, scalar arithmetic is
Montgomery multiplication mod `L`, and the fixed-base multiplication that
signing runs on the secret scalar and the nonce reads its table by masked
selection over every entry: no branch and no memory index depends on a secret.
Verification handles only public data and is variable time.

## Verification

- **RFC 8032 section 7.1**: every test vector, signed and verified
  (`tests/rfc8032.rs`).
- **Project Wycheproof**: every group and case of the Ed25519 verification
  vectors, vendored byte-frozen in `vectors/wycheproof/` at a pinned upstream
  commit with its SHA-256 (`tests/wycheproof.rs`): 151 cases, every `valid`
  case accepted and every `invalid` case refused. The upstream file carries no
  private keys and no `acceptable` results; the test fails if a re-vendor
  introduces one, until the strict behaviour for it is stated.
- **Differential against `ed25519-dalek`**: public keys, signatures and strict
  verdicts recorded from `ed25519-dalek` over seeded keys and messages, their
  corruptions, malleability, non-canonical `R`, every small-order key and
  point decoding near the edges of the field, frozen in `tests/vectors/` and
  replayed (`tests/reference_vectors.rs`, `tests/edge_cases.rs`). The
  dependency itself is not in the graph.

`cargo bench -p purrdf-ed25519` times sign and verify over messages from empty
to 64 KiB, key expansion and decoding, and a GTS-shaped batch of 64 signatures.

## License

MIT OR Apache-2.0 OR MulanPSL-2.0.
