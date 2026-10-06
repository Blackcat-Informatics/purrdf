<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 4 independent security and caller review

VERDICT: PASS

Required T4-R1 and T4-R2 are CLOSED on the corrected identity below. No required
Task 4 finding remains. This is not whole-issue completion or acceptance of
approved Tasks 5-6, PR, hosted CI or merge.

## Current source identity and preserved failure history

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD: 31498279c83c7fb9c3c6d97faffc1628a82452e1.
Captured integration base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Final complete ten-file manifest raw/T4-R1-files.sha256 SHA-256:
9d4e72ee4bd4401648035678f33dbe5be90a7a7f93189cb78d028bd70bc95029.
Final complete tracked-and-new patch raw/T4-R1-source.diff SHA-256:
980c0c1af287f5b4290ad416242d96259cd5d68134257723419d068213ae4143.
Initial-to-correction delta raw/T4-R1-delta.diff SHA-256:
22e38764164c55f391fbd7f8d4521b0a410dd439ca2f2a4704c1ab8b6cb9b95b.

Independently recalculated these artifact hashes and checked every final manifest
entry against current source: all ten match. Read the entire delta, current
production paths, implementation correction report and actual final logs.
Compared manifest entries: only verify.rs, writer.rs and hedged_writer.rs changed
from the initial Task 4 identity; the other seven are byte-identical.

The original BLOCKED report is preserved verbatim as tasks/T4-review-initial.md,
SHA-256 6985401566fa7f3861786371a91f80d3466ffd542efe12a4f8e2171a09760a1e.
An actual cmp confirmed archive equality before replacing this authoritative
report. Original failed public probe logs remain historical failure evidence,
not current qualification. No source/index mutation, delegation, commit, push,
post or private memory access occurred.

## T4-R1 closure: one result assembly and actual unsigned refusals

The unsigned/no-transport/no-signature branch calls verify_graph_with_keyring
with an explicitly empty Keyring and the actual options. Ordinary keyring and
resolved OpenPGP verification use that SAME result assembly, which invokes the
existing sole signature/status/count/integrity/trust/profile pipeline. OpenPGP
only adds its known key/fingerprint/visual metadata after assembly. This removes
the former unsigned success shortcut without adding another policy body.

EmptyFile now joins the shared integrity errors and retains the reader's original
detail. The verifier refuses both truly empty input and malformed/torn first items
that the reader reports as EmptyFile before its later torn check. A complete
non-header item receives DamagedFrame and also refuses. Generic valid unsigned
header/files still succeed when the ordinary signature requirement is disabled.

Existing evaluate_profile_policy is unchanged and executes for unsigned files.
Evidence/opaque Error findings now reach profile_findings and make ok=false.
Their actionable detail/severity/profile are retained; errors may remain empty
when refusal comes from this separately reported policy layer, matching the
ordinary core's contract. The actual sealed-source exception remains governed
by that same policy, not duplicated as a verification special case.

Independently reran the ORIGINAL UNALTERED standalone public consumer:

CARGO_BUILD_JOBS=4 cargo run --locked --offline --manifest-path
raw/T4-review-probe/Cargo.toml

The command ran from the worktree with the full Stage manifest path and
returned terminal exit 0. Evidence:
raw/T4-R1-independent-original-probe.log. It proves all FIVE originally failing
refusals on the corrected production source: empty input, [0x5f], [0xff],
unsigned evidence and unsigned opaque. The clean generic unsigned header/file
neighbors still succeed. The three framing failures retain nonempty errors and
original EmptyFile diagnostics. Evidence/opaque show exactly the original
public policy evaluator's Error findings, including evidence's head-commitment
requirement. The probe assertions were not weakened.

Verified unchanged probe identity:
raw/T4-review-integrity-probe.rs SHA-256
97f6ed0e4b340d57645f0accec834b6b14bf86419346d936cb0c5f8d373ed4f7;
raw/T4-review-probe/Cargo.toml SHA-256
59c2baa52bf73f7ac0e97278eebd3982e91c0e4af8267aea41b1e1c35fc88178;
its lock SHA-256
ba55edbf116199e6500be7d8ffcc87abc762111e83cd90f8859050569a2e3b85.
The standalone lock is distinct from workspace qualification. It binds the actual
worktree GTS crate at opt-level 3 with assertions and overflow checks; this
execution is a public reproducer, not a second workspace dependency gate.

Adjudicated current actual native and wasm tests additionally establish the
existing sealed-source evidence exception and its unsealed refusal neighbor,
while preserving policy warnings as warnings. Actual signed opaque Encrypt0 with
MissingKey remains accepted. The signed unknown-codec neighbor forces the real
decode attempt by omitting a public blob digest; it receives UnknownCodec and
still authenticates the exact encoded bytes. Earlier incorrect lazy-blob inputs
and failed native/wasm logs remain failed evidence; no reader/codec implementation
was changed to manufacture those neighbors.

## T4-R2 closure: governed redacting Debug

SnapshotSigner now uses the actual existing
purrdf_hash::debug_non_exhaustive!(SnapshotSigner { kid, public_key_armor })
macro. The manual duplicate body is removed. This preserves the exact displayed
public fields/order and nonexhaustive suffix; no macro/dependency/API enlargement
was introduced. The current native/actual wasm public redaction and Clone test
passes. Shared owned-seed Drop clearing and allocation-before-secret-copy Clone
are unchanged and retain the initial independent observations.

Hedged<P>'s all-fields-elided Debug does not fit the existing macro's at-least-one
field grammar, so no unsupported broad macro change is required or claimed.

## Qualification adjudicated on the corrected identity

Read actual final evidence rather than inferring correctness from SUCCESS:

- raw/T4-R1-public-native-complete.log: all THIRTEEN public Writer/file groups
  execute and pass, including the new actual framing/profile/exception/opaque
  neighbors and original ten caller groups.
- raw/T4-R1-wasm-complete.log: the same thirteen groups actually execute and
  pass in wasm/Node, separately establishing runtime parity.
- raw/T4-R1-clippy-complete.log: final all-targets GTS -D warnings completes.
- raw/T4-R1-docs.log: strict public rustdoc completes on unchanged final
  production inputs; the subsequent unknown-codec test-input correction does
  not change those documentation inputs.
- raw/T4-R1-helpers-complete.log: 81 enforced jobs, 91 distinct rows,
  1818 files, no copies open or escaping includes.
- raw/T4-R1-format-complete.log, raw/T4-R1-whitespace.log and
  raw/T4-R1-manifest-check.log qualify final formatting, all ten file identities
  and both new Task 4 files. Independent tracked diff check also returned zero.
- raw/T4-R1-deferral-scan.log has no added marker hits.

The original 274-case complete GTS execution remains PRE-CORRECTION regression
evidence. Its unchanged eleven-group composite/primary-fixture, codec/crypto,
shard and consumer API observations can be reused; it is not another final-source
whole-package pass. The corrected thirteen-group native/wasm suite requalifies
the affected verifier/result assembly and SnapshotSigner. No API signature,
primitive, primary fixture, dependency, feature, consumer body or harness
registration changed in this correction.

## Initial independent security/caller observations remain applicable

The archived initial report provides full independent source/caller observations
and original artifact identity. The narrowed correction leaves actual Writer
append/signing-mode and resolver/keyring bodies unchanged, and current public
groups execute their affected integration again:

- One Writer append body; sealed mode makes composite convenience appends Result
  while retaining existing unsigned/Ed source contracts. Composite installation
  into safe public Infallible configuration is unrepresentable. Conversion moves
  existing chain/output/index/catalog/dictionary state, and Ed rotation in hedged
  mode retains fallibility.
- Required caller-owned provider with no OS/clock/deterministic fallback;
  one draw per new composite signature, no Ed draw. Owned randomizer clearing uses
  the shared home, including partial/provider/native-error exits. Signing occurs
  before output/head/index mutation. Actual thirteen append/refusal/recovery and
  signed MMR/index tests pass again.
- One strict parse/status/verify loop serves raw optional typed and legacy text
  resolvers. Malformed/unsupported rejects before lookup; unresolved supported
  remains Unverified; wrong key type is Invalid. Exact opaque/absent/empty IDs
  stay distinct with fixed-hasher configured Keyring and no fabricated lookup.
- Actual mixed Ed/composite/Ed/composite Writer -> reader -> file verification,
  both-component corruption, changed content, malformed envelopes, binary IDs
  and actual embedded/out-of-band OpenPGP tests pass again.
- Shared file-integrity refusal preserves valid-survivor crypto counts without
  concealing damaged content/header/chain/trailing items. MissingKey/UnknownCodec
  capability absence remains distinct from unauthentic bytes.
- SnapshotSigner/public-key Debug omits secrets; guarded owned storage clearing
  does not establish destruction of compiler/register/historical copies,
  hash states or aborting-process memory.

No original primary bytes, corpus, dependency declaration or semantic feature
was modified. Task 1-3 external crypto/fixture observations remain attributable
prior evidence; this reviewer does not claim to have repeated that full audit.

## Limits and required workflow boundaries

No full workspace gate, hosted CI, hook, signing, push/publication, full binding
runtime, hardware timing measurement, post-drop memory inspection or formal
certification was performed by this reviewer. Composite -58 remains pinned
provisional draft support; no published shared cross-engine GTS composite corpus
claim is made. Provider freshness, successful full fill, cryptographic quality
and dedicated component keys remain explicit caller obligations.

No required Task 4 finding remains. Parent still must perform normal signed
hook-verified commit, push, exact remote readback and task publication. Approved
Task 5 compaction/certification, Task 6 and final Stage workflows remain required
work and are not claimed as complete by this unit review.
