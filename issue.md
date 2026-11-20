# Issue #1308: GTS verifier reports an unsupported signing algorithm as invalid, not unverified

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The GTS verifier reports a signature made with an algorithm the engine does not implement as `Invalid`. The other GTS engines are moving to read the declared signing algorithm and report such a signature as unverified: the engine cannot check it, which is different from having checked it and found it wrong. Once that lands upstream with its new test vectors, this engine will disagree with them.

Read on main, not run:

- `crates/gts/src/cose/sign1.rs` defines `Algorithm` and maps the COSE `alg` value to it.
- `crates/gts/src/cose/verify.rs` resolves a key with `KeyResolver::resolve(kid, algorithm)`; the documentation of `SigStatus` says an unsupported `alg` yields `Invalid`.

## Expected

- A signature whose declared algorithm this engine does not implement is reported as unverified, with the declared algorithm available to the caller.
- `Invalid` is kept for a signature that was checked and failed, and for a malformed signature structure.
- A caller that requires verified signatures still refuses an unverified one. The change must not turn a refusal into an acceptance anywhere; each such caller gets a test.
- The upstream vectors for this behaviour are vendored when they are published, and pass.

## Acceptance

- A test for each of: an algorithm the engine implements with a good signature (verified), the same with a bad signature (invalid), an unknown algorithm (unverified), a malformed header (invalid).
- The C, wasm and Python surfaces report the same three outcomes.
- The frozen vectors under `vectors/` are not regenerated here.


## Comments (3)

### paudley — 2026-11-13T16:29:08Z

A downstream consumer calls signature verification with a key and refuses a segment that does not verify. Before any release changes what that call returns for an algorithm this engine lacks, that consumer is told, and the change is named in the release notes as a behaviour change. The acceptance line above stands: a caller that requires verified signatures must still refuse an unverified one.

### paudley — 2026-11-13T19:03:19Z

How Ed25519 verification with a key treats degenerate inputs today. Read from the code on main; nothing was run. No change is proposed by this comment.

`verify` is the strict rule: it calls `verify_strict` (`crates/ed25519/src/keys.rs`). The equation is cofactorless, and the computed point is re-encoded and compared with the signature's `R` bytes.

| Input | Result | Where | Error |
|---|---|---|---|
| Small-order public key, all eight points | Decodes, then verification rejects | `keys.rs`, the `is_small_order` check in `verify_strict` | `SmallOrder` |
| All-zero public key | Rejects; it decodes to a point of order 4 (inferred from the curve equation), so the row above covers it | same | `SmallOrder` |
| `R` is the identity, or any small-order `R` | Rejects | same check, applied to `R` | `SmallOrder` |
| `R` with y not reduced modulo p | Rejects | point decoding in `point.rs` | `InvalidR` |
| `R` with x = 0 and the sign bit set | Rejects | point decoding in `point.rs` | `InvalidR` |
| Public key with y not reduced, or x = 0 with the sign bit set | Rejects when the key is decoded | `VerifyingKey::from_bytes` | `InvalidPublicKey` |
| `S` not below the group order | Rejects, checked first | `Scalar::from_canonical_bytes` | `NonCanonicalS` |
| Mixed-order key or `R` (prime-order point plus a nonzero torsion point) | Accepted as an input; the cofactorless equation decides | `verify_strict` | `Mismatch` when it fails |

Tests that pin these are in `crates/ed25519/tests/edge_cases.rs` and `crates/ed25519/src/strictness_tests.rs`, the latter against frozen reference verdicts. Gaps: only the identity key is exercised through `verify_strict` (the other seven small-order keys are covered by the weak-key predicate), and no test names the all-zero key.

At the GTS layer every one of these becomes `Invalid`: `crates/gts/src/cose/sign1.rs` discards the Ed25519 error variant. No GTS-level test for a small-order key or a small-order `R` was found.


### paudley — 2026-11-18T02:59:01Z

PR #1372 delivers the status part: a signature whose `alg` the engine does not implement now verifies as `SigStatus::Unverified`, not `Invalid`. `Invalid` stays for a failed check and a malformed signature, and the only caller that requires verified signatures (`gts_certify.rs`) accepts only `Valid`, so it still refuses.

Still open on this issue after that PR:
- Expose the declared algorithm to the caller.
- A refusal test for each caller that requires a verified signature.
- Upstream vectors, once they are published (not yet available).


