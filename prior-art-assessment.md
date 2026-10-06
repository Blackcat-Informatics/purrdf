<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent prior-art feasibility assessment

Issue: 458. Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Inspected source/base: `ce3c07192aba1e36666062c00f958670a827cfb5`.
Assessment date: 2026-10-06. No shipping source edited; no tests, commits, or forge posts performed.

## Decision

VERDICT: ACTIONABLE. No external brick wall has been established by the available evidence. Missing composite GTS vectors do not prohibit implementation: the issue explicitly conditions shared-vector conformance on publication. The current upstream GTS wire specification permits algorithm agility; it does not restrict Sign1 to Ed25519. The issue requests the IETF construction, so implementing the draft's requested identifier with an explicit pinned draft contract is within the requested work. This is an inference from the issue and governing specification, not an assertion that IANA has assigned the identifier.

Choosing an unrelated private-use integer or text algorithm would introduce a new protocol convention and should be rejected. Choosing the draft's requested `-58` does not invent a value, but it must be described as provisional draft support. Never claim final IANA registration or shared-engine composite conformance. The long-lived signature motivation makes documenting the exact draft construction and preserving its verification contract material acceptance requirements.

## Authority and coverage

Read the complete issue body (four requested bullets), comments (zero), `prior-art.md`, and `brief.json`. The brief found no linked ADR or related title-search issue. Its recurrence count is three `none` trailers in the last 200 commits, not evidence of three composite defects; composite-defect recurrence remains unknown.

Read worktree `AGENTS.md`, plus repository-root `.baseline` and `.goals` (the latter two are absent in the isolated worktree). Applicable constraints include protected main, original Rust tooling, wasm portability, no semantic Cargo features, one home per job, hard failures, dependency prohibitions, frozen GTS vectors, and GTS wire authority in gmeow-gts. Stage 1/Stagectl quality, delegation, validation, and no-deferrals instructions were also read. A focused memory search had no relevant result and supplied no factual premise.

The upstream tree was independently checked through GitHub's API at `0d1c8299c9411ea4ead853e31721d42ea66f081e`: the COSE directory contains `sign1-basic.json` and `sign1-empty-id.json`, with no composite vector. Its `docs/GTS-SPEC.md` section 9.2, lines 1129–1139, permits algorithms declared in the COSE header and requires readers to honor them. The local copy says the same at lines 1094–1105. Therefore adding this algorithm under existing Sign1 does not require changing GTS framing or inventing a new GTS profile. [Pinned upstream specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/0d1c8299c9411ea4ead853e31721d42ea66f081e/docs/GTS-SPEC.md#92-signatures-optional-algorithm-agile).

## Primary technical prior art

The live JOSE/COSE draft rendered a 6 October 2026 publication date during this assessment, rather than the supplied 5 October snapshot. Its section 4 delegates construction and serialization to LAMPS; COSE application context is empty. Sections 5.2 and 7.2.5 request `-58` for ML-DSA-65-Ed25519 but still mark it TBD. Pin the revision used for implementation rather than silently following a mutable HTML URL. [JOSE/COSE draft](https://ietf-wg-jose.github.io/draft-ietf-jose-pq-composite-sigs/draft-ietf-jose-pq-composite-sigs.html).

LAMPS revision 19 uses the representative `Prefix || Label || len(ctx) || ctx || SHA512(M)`, with prefix `CompositeAlgorithmSignatures2025` and label `COMPSIG-MLDSA65-Ed25519-SHA512`. Both components sign that representative; the ML-DSA primitive also receives the label as its own context. Verify both components. Serialize ML-DSA first: 3309 signature bytes followed by 64 Ed25519 bytes, and 1952 public-key bytes followed by 32. The interoperable private encoding contains the 32-byte ML-DSA seed followed by the Ed25519 seed. Component keys must be fresh and dedicated to the composite. [LAMPS revision 19, sections 3, 4 and 6](https://www.ietf.org/archive/id/draft-ietf-lamps-pq-composite-sigs-19.html).

The live IANA table has `-256` through `-54` unassigned. Thus `-58` currently has no registered collision, while remaining unassigned. [IANA COSE algorithms](https://www.iana.org/assignments/cose/cose.xhtml#algorithms).

RustCrypto `ml-dsa` 0.1.1 is pure Rust, but its inspected manifest unconditionally depends on `signature`, even with default features disabled. That package is forbidden on every dependency edge by `scripts/check-banned-deps.py:253`; the default PKCS#8 path also conflicts with the ban at line 239. Direct adoption is therefore unsuitable. This rejects one implementation choice, not the issue. An original first-party FIPS 204 implementation is actionable. [RustCrypto manifest](https://github.com/RustCrypto/signatures/blob/master/ml-dsa/Cargo.toml), [FIPS 204](https://csrc.nist.gov/pubs/fips/204/final).

## Actual implementation gaps

1. `crates/gts/src/cose.rs` has an EdDSA-only protected-header constructor and `sign_id`. `parse` fixes the signature type at `[u8; 64]`, uses `decode_prefix` without requiring full consumption, accepts any outer tag, does not validate detached payload, and does not inspect protected `alg`. `verify_sig` consequently verifies Ed25519 against arbitrary protected bytes regardless of the declared algorithm. Correct dispatch requires a typed parsed envelope, exact supported algorithm identity, algorithm-compatible key type, and exact component lengths. Missing, conflicting, unsupported or malformed algorithm declarations must not fall through to Ed25519.
2. `verify_signatures` accepts only an Ed25519 resolver and marks every unresolved parse outcome unverified. Distinguish malformed/unsupported signatures from valid structures whose keys are unresolved. Component stripping must never become an alternate recognized Ed25519 signature.
3. `Writer.signer`, `Writer::sign_with` and frame emission at `writer.rs:1105–1112` are Ed25519-only. A new primitive unused by the writer would not deliver GTS authoring. Production signing needs caller-supplied fresh hedging randomness or an explicit fallible randomness provider; no OS syscall is required in the portable core. A deterministic vector entry point must be separately identifiable and must not silently become production's default.
4. `verify.rs` resolves Ed25519-only keyrings and invokes `verify_signatures` at lines 312–319. Keep existing OpenPGP import as the Ed25519 key-discovery surface; the issue does not request composite OpenPGP discovery. Provide composite out-of-band resolution through the actual keyring verification path.
5. `crates/rdf/src/gts_certify.rs:644–668` parses carried detached signatures and verifies through an Ed25519-only keyring. `compact.rs:961` and RDF certification packaging arguments also carry Ed25519 signing keys. Assess and update the relevant real consumers so composite signed GTS can survive compaction and certification. Preserve existing Ed25519 vectors and byte behavior.
6. `purrdf_hash::sha3::keccak_f1600` already owns Keccak. SHAKE is absent from the inspected hash source; extend the existing home rather than copying the permutation into a lattice crate. `sha2` remains the workspace's authorized SHA-2 implementation. Update layers, helper/dependency ledgers, crate metadata and generated projections only where the chosen first-party design changes them.

## Acceptance path

Use coherent tasks for SHAKE/FIPS 204 primitive qualification, composite construction and strict Sign1 dispatch, production writer/resolver integration, and affected downstream certification. Import published cryptographic known-answer fixtures as frozen evidence with provenance; do not copy upstream implementation bodies or coefficient arrays. Local round trips alone are insufficient assurance for a newly written signature primitive.

Require real writer → read/fold → resolved verification demonstrations for Ed25519 and composite keys, deterministic known answers, hedged signing success with distinct supplied randomness, hard failure on randomness-provider error, tampered frame refusal, both-half stripping/zeroing/replacement, cross-message and cross-key component swaps, length/order errors, algorithm-header downgrade and key-type mismatch. Include unresolved-key versus malformed-envelope status behavior. Inspect correct coverage rather than relying on a test name.

Run focused package and consumer tests, affected wasm builds/runtime checks, helper/layer/banned-dependency gates, and required hooks/CI qualification against the final source identity. Existing shared COSE vectors remain mandatory regression evidence. Since composite GTS vectors are not published, report that conditional evidence as unavailable and prove the independent primitive/composite construction with published primary fixtures and local adversarial tests. Do not fabricate shared vectors under `vectors/`, add an xfail that hides missing implementation, or describe absent cross-engine evidence as a pass.

## Residual uncertainty

The requested numeric allocation can change before standardization. The user's issue requests draft-era work and the current GTS specification is algorithm-agile, so this is a material risk to state, not a demonstrated inability to work. No runtime checks have been executed by this assessor; all implementation assertions above are observations from inspected source. A cryptographic implementation and its independent fixture coverage are substantial work, but duration or difficulty is not an external blocker.
