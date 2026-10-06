<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 quality and structure review

VERDICT: BLOCKED

Required finding: G3, the same owned-SHAKE lifecycle defect identified by the
security specialist, independently confirmed below with its structural repair
constraint. No additional distinct quality/structure defect was established.
G1's measured reader cost and G2's hosted alert disposition remain parent-owned
open findings; this review does not discharge them or duplicate their work.

## Identity, authority and scope

Issue 458 / PR 464, Blackcat-Informatics/purrdf.
W=/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
ROOT=/home/paudley/Active/purrdf.
S=W/.stage/purrdf-gts-composite-ml-dsa-65-ed25519.
Branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Independently observed exact HEAD 52988974f2d11a40214281648983f9145af7a3fb,
tree 43b0817e30a3ad110cd013796a064c177a27e067 and current origin/main
090b14bb8c00e2504078cb694d278ee2d614443a. Source/index remain unchanged;
only selected Stage is untracked. Final 52-path source manifest SHA256
e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d
was independently checked during the immediately preceding head-bound review.
Immutable plan SHA256 remains
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Captured implementation base is ce3c07192aba1e36666062c00f958670a827cfb5;
initial PR base is 0d6575a46088e9420d73654b7b5965b9580c5d36.
The predicted current-base merge tree has no execution qualification here.

Read applicable root/worktree AGENTS, baseline/goals, Stage 2/Stagectl quality,
validation and no-deferrals doctrine, literal issue/plan/current task reviews,
historical-fit, initial completion report and current parent gap analysis
(SHA256 4cdfb78180fb9dc785a9e516c58f1fc1a681497fd91f31286ad35252ace9df00).
No applicable ADR/constitution was identified in tracked inventory or briefs.
Reviewed all new ML-DSA modules, SHAKE state/buffer boundary, native shared
clearing/comparison home, composite and strict Sign1 module, changed actual
Writer/resolver/reader/policy/compactor/certifier contracts and concrete affected
callers. Existing substantive public test assertions/goldens and native/wasm
executions were adjudicated in the preceding requirement/completion review and
reused for the unchanged head. No source/index/forge/private-memory mutation,
child delegation, model/GPU/service action, test execution or repeated blanket
gate occurred. Only this selected Stage report is created.

## Required G3: controlled SHAKE storage is outside the implemented lifetime

This is not a concern about historical compiler copies or registers. The code
directly creates and owns the affected objects:

- `mldsa65/mod.rs:219–221` hashes the caller's key-generation seed and expands
  secret sampling input. `:303–307` hashes secret K plus randomizer and mu into
  the private signing seed; `:314` feeds that seed into mask generation.
- `mldsa65/sampling.rs:14–19` creates a local Shake256, absorbs those inputs and
  consumes it into a temporary output reader. `:57–63` separately absorbs the
  secret sampling seed. `:86–91` hashes the private mask seed per polynomial.
- `hash/src/sha3.rs:283–286` owns the 25-lane state and partial block buffer;
  `:318–325` finalizes and hands copied lanes to the reader without clearing
  the original buffer; `:351–354` owns reader lanes. Neither owned type has a
  cleanup implementation. `hash/src/block.rs:11–14` owns buffered input bytes;
  its clear method only changes the filled counter, not byte contents.

SecretBytes/SecretPolys correctly guard their own allocations, but clearing the
output of SHAKE does not clear the SHAKE input buffer or its secret-derived
state. These owners are used in the shipping keygen/signing paths and can leave
their controlled contents after successful return or sampler error. The module's
explicit exclusion of hash states cannot discharge the security specialist's
required FIPS 204 intermediate-state finding. Functional known answers and
clippy success cannot prove destruction of those owned bytes.

The structural constraint is concrete: `layers.toml` declares purrdf-hash as
the zero-runtime-dependency root, while the current single `secret-clearing`
ledger home is purrdf_ed25519::wipe_secret (`ct.rs:41`, re-export lib.rs:82).
Hash cannot depend on its higher-layer Ed25519 consumer to obtain cleanup.
Copying the clearing body into SHAKE would violate one-home policy. The chosen
repair must complete controlled absorber/buffer/reader cleanup on all relevant
lifetime paths through one governed clearing home reachable under the declared
layer graph, update its declarations/callers coherently if relocation is needed,
and retain the root ring-fence. This report does not select speculative code or
authorize mutation; parent must assign the coherent repair after reading the
security report and remediation plan.

Precise required check: observe zeroing of the actual owned state/buffer/reader
storage at cleanup, including partially absorbed secret input, finalize/drop,
clones and sampler-error exits as applicable to the selected lifecycle design;
preserve genuine public keygen/signing/verification and all frozen complete
SHAKE/NIST/IETF answers natively and on wasm. Adjudicate the actual changed
production callers, helper/layer/ring-fence/dependency declarations and denied
warnings. Do not replace owned-state tests with a memory dump of historical
compiler copies, a disclosure-only change or passing output-vector tests.
G3 remains required HIGH under the security review; this is the same finding,
not an additional counted defect. Parent was notified before this terminal report.

## Remaining structural assessment

The private ML-DSA split has concrete responsibilities: mod owns public typed
keys/signatures, guarded scratch and FIPS control flow; math owns canonical
polynomials/NTT/rounding; codec owns fixed bit layouts/canonical hints; sampling
owns bounded XOF draws. Private inputs follow exact validated lengths and fixed
parameters. Rejection/nonce budgets return typed errors rather than partial
signatures. Roots are derived at compile time from the definition rather than
an imported coefficient table. No arbitrary module-size rule or speculative
generic framework is warranted by this fixed pairing.

Composite owns the mandated representative and concatenated key/signature
encodings once, reusing native ML-DSA, strict Ed25519 and the workspace SHA512.
Sign1 owns envelope/header validation, exact protected bytes, typed algorithm/key
dispatch and one resolver/status loop. Legacy Ed conveniences route through it;
their key type cannot represent composite material and does not duplicate the
implementation. Optional/binary/empty key ids stay distinct. Fixed lengths and
private fields maintain decoder invariants; no malformed-input panic contract
was established in these public boundaries.

Writer's sealed Infallible/Hedged result contracts preserve existing Ed callers
while making composite provider/signing failure visible through every append.
Actual mutation of output/head/index follows signing. Rotation preserves the
fallible contract; there is no parallel writer or silent entropy fallback.
Owned signer Debug redacts secret fields and uses the governed macro where its
grammar applies. The all-field-eliding Hedged Debug does not justify changing
that macro speculatively. Caller-owned randomness and dedicated keys are explicit
contracts, not added platform entropy or discovery dependencies.

VerificationKey/SignatureKeyring give supplied keys one typed route, shared with
legacy/OpenPGP assembly and profile/integrity findings. The Keyring uses the
fixed-hasher home, and the absent id key is separately configured. Reader's
packaging bit is documented observation rather than authentication. Both eager
and evented readers use the same segment-local classifier; RDF projection uses
that home. G1 demonstrates the cost of collecting these facts where layout
cannot use them; the measured repair must preserve foreign-before-type and
segment reset semantics, not introduce a second classifier.

The reworked compactor/certifier share strict carried evidence/root decoding,
source-head selection and sorted/deduplicated exact authorship pairs. Mandatory
sealed packaging signers both sign the actual final index via Writer; composite
errors propagate without returning a pack. The existing textual certificate
packaging id remains distinct from opaque carried ids. Certifier checks retain
independent content/chain/authorship/root/packaging/suppression facets rather
than making one valid survivor stand for all required evidence. No hidden
success/refusal-as-support or duplicated production path was established.

Dependencies/features/tooling fit the inspected architecture: manifests inherit
sha2 from the workspace; Cargo.lock adds its existing package to GTS's edge,
not a new crypto package. New public test targets use first-party Rust testkit
and actually execute on wasm. No semantic feature, banned external edge,
crate-external path include or copied governed vector was introduced. The
four native-home AGENTS rows track the actual implementations. Existing test
fixture refinements strengthen strict evidence boundaries while retaining
ordering/proof/golden assertions. Seven deleted process-reference exemptions
retire the exact removed labels rather than weakening the detector.

## Attributable checks and limits

Read the actual final head-applicable logs/statuses, not only task PASS lines:

| Existing check | Observed result | Log SHA256 |
|---|---|---|
| helper census | 81 enforced jobs, 23 reasoned variants, zero open copies; 91 distinct rows; 1819 files; no escaping include | ef956039b165406b869aceda6730e7a6e0b4e5655687fef1da3117d65be378e1 |
| layer check | 207 first-party normal edges across 42 members match, no planned drift | db902d6a2589c726c1835836b891a3ba149b02f6c283686a4deb081763feeaf7 |
| dependency check | no 170 replaced dependencies reenter; all 79 external packages accounted for | 8f9aeab6c5f4852db8422c118c43b58dec591c9cbabafd2b46c3fb15c280a8f5 |
| workspace all-target clippy with denied warnings | terminal optimized dev success, shipping/transitive/test/binding callers compile | f006b657ebfa7b1677b0296efdcb7839d87fd51aef360df05cb069f92c29b05b |
| strict GTS/RDF rustdoc | terminal success with denied warnings | f78e4a719902ff42d110511d692c861eb5a3f15e20210e4e3849a2bf25f37af1 |

These are raw/T6-R1-{helpers,layers,deps,clippy,docs}.log, source-bound to the
unchanged final manifest. Ratchet records one shrinking non-Rust path and no
new unexplained non-Rust program; shard gate covers all 42 members. Current
1361 native/13 docs and eleven actual wasm consumer groups remain attributable
functional evidence, with earlier unchanged primary/Writer target evidence
reused as detailed in S2-completion-initial. They do not discharge G3/G1.

No full timing, hardware side-channel, FIPS certification, installed-language,
current-base integration or fresh hosted qualification is asserted. G2's
false-positive witnesses require actual forge disposition; this reviewer
performs no forge mutation. Substantive security/performance conclusions and
their remediation remain separate. Parent must fix G3 and all required gaps,
qualify the resulting changed closure, preserve failed history and complete the
fresh independent Stage 2 exit/CI/integration gates before claiming readiness.
