<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Stage 2 independent security review

VERDICT: BLOCKED

One REQUIRED/HIGH controlled-secret-state finding, S2-SEC-1, remains. All three
captured CodeQL hard-coded-value annotations are independently adjudicated
FALSE POSITIVES below. Those dispositions do not clear this separate finding,
change the forge's red check, or constitute authority to dismiss alerts.

## Identity, scope and independence

Issue 458; PR 464; worktree
/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519;
branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Read-only Git inspection confirms unchanged HEAD
52988974f2d11a40214281648983f9145af7a3fb and tree
43b0817e30a3ad110cd013796a064c177a27e067.
Final 52-source manifest SHA256 is
e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d;
all entries independently matched in this reviewer's preceding historical pass
on the same unchanged head. Immutable plan SHA256 remains
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Current source-capture base 090b14bb8c00e2504078cb694d278ee2d614443a does not
supply integrated-tree qualification.

This reviewer implemented no source. The earlier independent historical-fit
assignment is separate from this security assignment and neither substitutes
for gap/completion/performance auditors. Read applicable Stage 2/Stagectl
quality/validation/no-deferrals doctrine, supplied and actual AGENTS, parent
.baseline/.goals, full issue with nine comments, plan, material T1–T6 reviews and
corrections. Inspected all new ML-DSA arithmetic/codec/sampling/sign/key bodies,
SHAKE and its shared BlockBuffer, strict Sign1/composite, actual Writer mode/
provider/append/error/secret storage, typed keyrings, untrusted carried/root
decoder, mandatory compactor and RDF certifier closure. Reused unchanged prior
source readings from the immediately preceding historical assessment, and
read the sensitive state and CodeQL paths independently in this assignment.

No source/index/forge/child/model/GPU/service/private-memory mutation occurred.
Only this report and the named CPU-only public probe under selected Stage were
written. Source/index stay clean; only selected Stage evidence is untracked.

## S2-SEC-1: owned SHAKE secret intermediates are abandoned without destruction

Severity: HIGH. REQUIRED. Remediation owner: parent/Stage 2 implementation.
Locations: crates/gts/src/mldsa65/sampling.rs:14–19,57–63,86–91;
crates/hash/src/sha3.rs:283–285,318–325,351–353;
crates/hash/src/block.rs:11–13,28–53. The unchanged BlockBuffer file SHA256 is
678475415baae139fd5e9afbe2b057ea80605b37eede58c820d69486142e6dbc.

FIPS 204 §3.6.3 requires potentially sensitive intermediate data to be destroyed
as soon as no longer needed. Its exceptions permit retained private-key seed
with private-key safeguards and retained public matrix. Neither covers abandoned
per-operation sponge/buffer state. The captured official PDF SHA256 is
57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b;
text SHA256 c3f68bc2b4ebb201321c8e774d4a29a41bbc857ad3854381aadce0363f9140a0,
with the normative clause at raw/FIPS204.txt:921–946. This is the original
standard, not an invented side-channel certification gate.

Actual secret computation establishes the omission:

1. from_seed calls sampling::hash on the 32-byte private seed plus two parameter
   bytes. secrets then feeds secret rho-prime plus a 16-bit polynomial counter
   into another SHAKE absorber.
2. sign_hedged derives private_seed by sampling::hash(K || rnd || mu), where
   K=encoded[32..64], rnd is 32 bytes and mu is 64 bytes. These total 128 bytes,
   below SHAKE256's 136-byte rate. BlockBuffer therefore contains the exact
   private K bytes, randomizer and message representative. Padding writes at
   offsets 128 onward; it does not erase the earlier private bytes.
3. Each mask hashes that private_seed plus two nonce bytes. The absorber again
   retains the private seed in its directly owned buffer.
4. Shake::finalize transfers the post-padding lanes into ShakeReader and lets
   the absorber/buffer disappear. Neither type has Drop or an explicit sensitive
   clearing method; sampling callers perform no explicit wipe. Reader lanes
   disappear after squeezing or sampler error without clearing. BlockBuffer
   clear() only resets filled and is not destruction of bytes.

These are explicitly allocated Rust fields, not inferred historical compiler
copies or uninspectable registers. Existing SecretBytes/SecretPolys guards
correctly clear expanded secrets, mask-output buffers, transformed coefficients,
rejected polynomials/products/responses and owned signing randomizers; they do
not reach the hash objects' private fields. Clearing a private_seed output
afterward cannot clear the exact private key material in another owned buffer.

A bounded SAFE public probe actually executed the current path crate's Shake256
on private-seed-shaped fixture input, finalized and squeezed 640 bytes. It
prints needs_drop=false for BOTH Shake256 and ShakeReader<256>, confirms actual
nonzero output and the absence of any destructor. Command from the worktree:

CARGO_BUILD_JOBS=2 cargo run --offline --manifest-path
.stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw/S2-security-state-probe/Cargo.toml

Terminal exit 0. This is a successful observation of the current omission,
not passing acceptance of secret destruction. No read of freed/uninitialized
memory, allocator-reuse assumption, key recovery, exploitation, timing proof
or hardware memory-erasure claim occurred. Source trace provides the exact
sensitive field/last-use evidence; needs_drop alone would not rule out explicit
clearing, whose absence was separately verified in all callers.

Probe identities:

- src/main.rs: ad09245f1c7bbc35c30e8867359b49b51af502ca2d86f6f35e99ada69fbd0a69.
- Cargo.toml: a2ee1b59d108058384d926f0ff27adf559bb89a997149c53e6cf436176c62d53.
- Cargo.lock: c3863a58bd4986997629b2bf1b037faf6e5523e4f0d456498524da3acac19a0d.
- raw/S2-security-state-probe.log: aa1c8deb7c19a2a05c801cad26d1b5b12a4a95191064dac97d9de234518e8fe3.

The standalone manifest binds actual current purrdf-hash with zero dependencies,
opt-level 3, assertions and overflow checks. Compiler independently observed:
nightly 4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1.
This is no additional workspace dependency or retained shipping test.

Consequence: controlled sensitive material has no destruction path after its
last use, contrary to the requested FIPS implementation's security requirement.
An attacker able to observe residual process memory may obtain private material;
this review demonstrates the missing defense, not actual successful recovery
or a signature forgery. Functional known answers cannot discharge this finding.

Scope adjudication: documentation truthfully discloses the current omission,
but that disclosure is not an authorized scope cut. Lack of formal FIPS
certification or inability to guarantee register/compiler-copy elimination is a
legitimate stated boundary. Leaving directly owned SHAKE lanes and raw private
buffer bytes completely unguarded is actionable implementation work. It is
not the same limitation. Earlier T2/T6 PASS reports did not establish this
normative controlled-state requirement and must not be used to dismiss it.

Required fix: give sensitive absorbers/readers a genuine destruction path for
their complete owned lane and buffered-byte storage, including consuming
finalization, successful squeeze, early sampling/signing errors and unwinding.
Wire every actual secret-dependent sampling/hash path through it, preserve the
one Keccak/XOF implementation, and use one governed clearing home without
introducing a root-to-Ed25519 dependency cycle or duplicating the wipe body.
A coherent relocation/re-export of the shared clearing primitive to the
zero-dependency hash root is an available design; exact architecture belongs
to implementation review. Generic hash clear/reset must not falsely advertise
erasure when it only forgets a length. Inventory explicit sensitive sampler
byte buffers and operation scratch as part of this same fix, and end guarded
lifetimes at actual last use where feasible.

Required check: meaningful Rust tests within the owning modules must observe
cleared live storage/destruction through a safe test seam for absorber lanes,
full buffered bytes, reader lanes, finalization transfer and secret sampler
success/error. Do not dereference freed memory or claim optimizer guarantees
from a drop-count-only test. Reexecute unchanged complete SHAKE/SHA3 primary
answers, ML-DSA deterministic/hedged official bytes and composite primary/refusal
cases; run affected native and actual wasm public paths, warnings/hygiene/
dependency-home checks, and source-bound independent re-review. Qualification
must preserve exact signatures and writer atomicity. Source/hardware historical
copies may remain precisely disclosed; directly owned omissions may not.

## Independent CodeQL dispositions: all three FALSE POSITIVES

Captured alert check: raw/S2-codeql-check.json SHA256
702c78e75968b709a13cfcd5526fc2fcd037b91f590cab4c8e71b39cb076914e.
Annotations SHA256
2656d121910824106b35406602e844d119f76d25cd44918e47cb6dc0c62f7f34.
Workflow SHA256
e3f2f1a6dbd5c7328a74e677aeaf49dc0bf2e880258d8786ba52d57761dd0ea1.
Five extraction/analysis jobs completed SUCCESS at exact issue head. Separate
CodeQL alert result is FAILURE, titled three critical hard-coded values.
Successful extraction does not mean accepted alert results.

| Annotation | Actual context and independently supported disposition |
|---|---|
| mldsa65/mod.rs:312, initial nonce=0u32 | FALSE POSITIVE. This is INTERNAL FIPS Algorithm 7 counter kappa, not a fixed per-signature private random seed, AEAD nonce or deterministic rnd fallback. Preceding code hashes private K || caller rnd || message representative into private_seed. ExpandMask uses private_seed plus a separate counter for each polynomial. take_nonce reserves five consecutive u16 nonces, increments by L=5, validates the group's last value before emitting a mask and hard-fails before reuse/exhaustion. Counter initialization is exactly standard Algorithm 7 step 8; seed derivation is step 7, mask step 11, increment step 31. Changing zero or randomizing this internal counter would alter official deterministic bytes and protocol computation rather than repair a vulnerability. |
| hedged_writer.rs:396, encryption key=[1;32] | FALSE POSITIVE. Fixed public fixture bytes occur inside the registered file_integrity_rejects_header_chain_and_torn_damage_but_allows_opaque_encryption Rust test. It creates one signed ciphertext blob, then verifies without decryption material and asserts valid signature, MissingKey diagnostic and accepted opaque authentication. This key is not secret production configuration, imported key discovery or runtime fallback. |
| hedged_writer.rs:397, IV=[2;12] | FALSE POSITIVE. Same single-encryption deterministic regression fixture. It is not a repeated production IV under a long-lived production key; the test's behavior is authentication without decryption, not an entropy-generation claim. Actual Writer Encrypt0Options receives caller-owned nonce/key in production; no fixture value is a library default. |

The counter trace is grounded in captured raw/FIPS204.txt:1500–1551 and the
actual complete sign/mask/take_nonce computation, not merely an "official
algorithm" assertion. The fixed-key/IV test is in the harness_main registration
and actually executed by the qualifying native/wasm T4/T6 package evidence.
Neither requires changes to constants, feature/rule suppression or fixture
weakening. An authorized parent may record exact forge-level false-positive
dispositions with this evidence; this reviewer has no dismissal authority and
performed none. CodeQL is not represented as green by this report.

## Remaining security assessment and qualifying evidence

No other required security defect was established in the inspected closure:

- Composite verifies both plain Ed25519 and pure ML-DSA on the identical pinned
  representative, with the pairing label as ML-DSA context. Conjunction, exact
  ML-DSA-first lengths, strict key decoding and weak Ed public-key refusal
  prevent stripped-half/header/key-type downgrade acceptance.
- Sign1 has one full-item parser, detached-null requirement, protected supported
  alg, exact protected-byte preservation, duplicate/cross-bucket/critical
  validation and canonical component boundary. Malformed/unsupported fails
  before resolver invocation; supported unresolved stays Unverified.
- Fixed integer NTT/rounding/norm scans avoid coefficient-dependent source
  indexing/early norm exits. FIPS samplers/signature rejection vary in time by
  design. No compiler/JIT/hardware constant-time, fault resistance or whole-
  operation timing proof follows from this review or frozen answers.
- Sealed hedged Writer requires caller-owned fresh randomizer provider, requests
  one full draw per composite signature and signs before publication/head/index
  mutation. Missing/failed/partial provider errors have no deterministic fallback;
  owned randomizer clears on all ordinary Result paths. Caller entropy quality,
  fresh full successful fill and external key dedication cannot be inferred by
  the library from bytes. No OS entropy, model backend or lifecycle call occurs.
- Strict carried/root decoding propagates refusal instead of skipping evidence;
  exact author-pair equality, actual current root/proofs and final-index
  authentication separate signed history from new ordering. The conservative
  shared UTC positive shape preserves malformed authored content and COSE.
  Actual compactor framing gate is not misrepresented as authentication.
- Typed opaque/absent/empty key resolution uses the same verifier. Public
  key discovery/trust policy/encryption remain the issue's stated exclusions;
  source compatibility wrappers do not introduce a second cryptographic body.

Reuse adjudication: exact unchanged task evidence independently audited all
70 official NIST records and the full pinned IETF answer. Current complete
GTS/RDF native package execution after T6-R1 replayed the affected behavior;
prior T2–T4 actual wasm primary/COSE/Writer groups remain attributable to their
unchanged closure, and current eleven actual wasm compaction/certification
groups qualify the corrected consumers. Those results prove functional behavior,
not S2-SEC-1's destruction contract. No blanket suite was rerun for ceremony.
No source identity changed during this review.

S2-SEC-1 blocks Stage 2 security acceptance regardless of the CodeQL false
positives or prior functional PASS reports. Required source remediation and
independent exact-source closure remain; current-base integration, hosted CI
refresh, completion auditing, signed publication and ghprsq merge are separate
parent workflow duties.

