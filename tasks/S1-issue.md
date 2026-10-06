# Issue #458: purrdf-gts: composite ML-DSA-65 + Ed25519 signatures under COSE_Sign1

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

`purrdf-gts` signs and verifies GTS frame ids with `COSE_Sign1` over EdDSA/Ed25519 only (`crates/gts/src/cose.rs` declares the one algorithm constant). This asks for a **composite ML-DSA-65 + Ed25519** algorithm in the same `COSE_Sign1` path, conforming to the GTS specification's vectors once they exist.

## Why

A signed append-only segment cannot be re-signed without breaking what its signature attests, because the signature over the head id anchors all prior history. Signatures that must stay verifiable for decades need a post-quantum component from the first write. The cost is about 3.3 KB per signature.

## Requested

- `sign` and `verify_sig` accept the composite algorithm alongside EdDSA, dispatching on the declared COSE `alg`.
- Verification requires **both** components; either one stripped, zeroed, replaced or swapped is refused. One composite algorithm under `COSE_Sign1`, not two `COSE_Sign` signers, so the post-quantum half cannot be stripped and the remainder accepted.
- Construction per the IETF composite ML-DSA work's ML-DSA-65 + Ed25519 pairing; FIPS 204's deterministic variant for vectors, the hedged variant for production signing.
- Conformance against the shared cross-engine vectors when the specification publishes them, plus the stripping and swapping refusals above as local tests.

## Out of scope

Key discovery and trust anchoring (deployment policy, as today), and encryption.


## Comments (0)

