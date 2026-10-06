<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 4 independent security and caller review

VERDICT: BLOCKED

Two required findings below prevent Task 4 acceptance. This verdict concerns
the current Task 4 unit; approved Task 5 compaction/certification is a separate
unexecuted unit, not a missing Task 4 implementation or whole-issue success.

## Exact reviewed identity

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD: 31498279c83c7fb9c3c6d97faffc1628a82452e1.
Captured integration base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Complete ten-file manifest raw/T4-files.sha256 SHA-256:
5bf4fe0e02b17147d476fb1122ec22b911ff08b17551e651a2a2b0438103c5a9.
Complete tracked-plus-new patch raw/T4-source.diff SHA-256:
4c52edc05e7448779175aedc181c8b04b11880a8370eed0e02077ea0695a1931.

Independently recalculated all three artifact hashes and checked all ten
manifest entries against current source twice; every entry matches. Read actual
changed production bodies, both public suites, reader paths and policy, the
approved plan, governing AGENTS.md and parent .baseline/.goals, relevant Stage
skills/references, T3 review, T4 implementation and current execution/validation
records. No source/index mutation, delegation, commit, push, post or memory
access occurred. Only this review and standalone Rust consumer evidence were
written under the selected Stage directory.

## T4-R1: unsigned verification still accepts invalid files and bypasses profile policy

Required owner: Task 4 implementation through parent. Location:
crates/gts/src/verify.rs:275-288 and integrity_errors at line 453.

The changed early return for require_signatures(false), no transport key and
no folded signatures computes only integrity_errors. Its whitelist omits
EmptyFile, and the return does not evaluate the actual profile policy at all.
Reader read_with_options returns EmptyFile both for empty bytes and when no
complete first CBOR item can be decoded; it returns before recording torn.
These are not valid unsigned GTS logs, yet this actual caller accepts them.
Separately, the early return bypasses the same policy evaluator used in the
shared keyring core. Declared evidence/opaque profile requirements must remain
effective when the ordinary file-signature requirement is explicitly disabled.
This is a dependent contract in the verifier touched by Task 4, not a new trust
or discovery policy.

Executed standalone public consumer:

`CARGO_BUILD_JOBS=4 cargo run --locked --offline --manifest-path raw/T4-review-probe/Cargo.toml`

The command ran from the worktree using the full Stage path and returned 101
from five expected-refusal assertion failures. Actual evidence:
raw/T4-review-integrity-and-profile.log. The first run is preserved as
raw/T4-review-integrity-probe.log; an intermediate all-framing-neighbor run is
raw/T4-review-integrity-all-neighbors.log. The final probe source is
raw/T4-review-integrity-probe.rs, SHA-256
97f6ed0e4b340d57645f0accec834b6b14bf86419346d936cb0c5f8d373ed4f7.
Its Cargo.toml SHA-256 is
59c2baa52bf73f7ac0e97278eebd3982e91c0e4af8267aea41b1e1c35fc88178;
standalone Cargo.lock SHA-256 is
ba55edbf116199e6500be7d8ffcc87abc762111e83cd90f8859050569a2e3b85.
This independent probe binds the actual worktree GTS source, with opt-level 3,
assertions and overflow checks enabled. Its own resolved external lock is
distinct from workspace qualification; it is not claimed as another workspace
gate or qualification of changed dependencies.

Observed public results:

- A clean generic unsigned header/file succeeds, the valid neighbor.
- Empty bytes, [0x5f] (incomplete first byte string) and [0xff] (invalid first
  item) each return ok=true, no errors and an EmptyFile diagnostic.
- Actual unsigned Writer::new("evidence") and Writer::new("opaque") files
  return ok=true with empty profile_findings.
- The actual public policy evaluator on those SAME reader Graph values returns
  ProfileSignatureRequired/Error for both, plus
  EvidenceHeadCommitmentRequired/Error for evidence. Generic has no findings.

Required correction: reject no-valid-file/framing failures through the shared
integrity result and route unsigned acceptance through the same actual profile
policy/verification result assembly as the ordinary core. Preserve generic
unsigned opt-in and legitimate signed opaque MissingKey/UnknownCodec behavior.
Add native/actual wasm public refusal neighbors and rerun this original probe.
Keep the policy's existing sealed-source evidence exception rather than
inventing an unconditional rule. Do not merely weaken the probe or add a second
policy implementation. Acceptance requires refusals and original findings,
not only a false boolean without actionable diagnostics.

## T4-R2: SnapshotSigner repeats the governed field-eliding Debug implementation

Required owner: Task 4 implementation through parent. Location:
crates/gts/src/writer.rs:195-202.

Secret redaction itself is correct, but the new manual nongeneric Debug body
duplicates the exact governed implementation. AGENTS.md's Shared structures
section requires the common field-eliding Debug home. Read the actual
crates/hash/src/impls.rs macro grammar/body: it implements this exact shape,
including public fields in the same order and finish_non_exhaustive.

Required correction: replace this body with the existing invocation
`purrdf_hash::debug_non_exhaustive!(SnapshotSigner { kid, public_key_armor });`
and retain the actual public secret-redaction/Clone regression, clippy and
helper coverage. No new abstraction, dependency or macro enhancement is needed.
The generic Hedged<P> Debug elides ALL fields; the existing macro requires at
least one named field. That body does not fit current macro grammar, so this
finding does not request a broad macro change or invented variant.

## Observations on otherwise implemented contracts

There is one Writer append body. The public sealed mode boundary prevents
installing a composite signer into Infallible through safe public methods.
with_composite_signer consumes the same Writer and moves every buffered/chain/
catalog/dictionary/index field into Hedged<P>. All thirteen append contracts
route through add_frame_with_options, returning Result in hedged mode; switching
back to Ed retains fallibility. sign_composite_with exists only in the hedged
impl. FrameMode is sealed by its inaccessible module, Hedged provider field is
private, and no public writer field/constructor can forge the default-mode
composite invariant. Existing Ed source conveniences retain their contract.

The provider is required at compile time, with actual compile-fail doctest.
Runtime provider errors retain WriterError::Randomness/source; no OS source,
clock, optional fallback or deterministic composite authoring shortcut exists.
Composite append requests one 32-byte draw; Ed requests none. A shared wipe
guard covers successful, failed/partial-fill provider and native-signing error
exits. Signing precedes offsets, types, frame_ids, buffer and prev mutations.
Explicit FrameOptions.signature retains its existing caller-carried override
meaning and does not represent a newly authored composite signature. Provider
freshness, complete successful fill and cryptographic quality remain explicit
caller obligations; fixtures make no entropy-quality claim.

SnapshotSigner Clone allocates public fields before copying its Drop-owned
seed, Debug omits the seed, Drop uses the shared wipe home, and snapshot metadata
uses mem::take for real moving consumers. Compiler/register/historical copies,
SHAKE states and abort are outside these owned-storage clearing claims.

The typed resolver and legacy text resolver use one verify_resolved loop with
strict parsing before key lookup. Exact optional raw identifiers and declared
algorithm reach borrowed key resolution. Malformed/unsupported is Invalid
before lookup; unresolved supported is Unverified; wrong typed key is Invalid.
Keyring uses the fixed hasher and distinct explicit absent-id storage, with no
binary/text or empty/absent fabrication. Algorithm mismatch cannot downgrade
the typed verifier. Existing String/Ed HashMap callers remain on the same core.
OpenPGP remains Ed discovery, not invented composite discovery.

Reader source confirms damaged frame signatures can be omitted. The ordinary
shared keyring core now refuses DamagedFrame, BrokenChain, TruncatedLog,
TornAppendError and ResourceLimit while preserving surviving crypto counts and
reader diagnostics. Actual tests execute changed content, header self-hash,
re-signed broken chain and torn trailing input; both-keyring/OpenPGP paths refuse.
Signed opaque Encrypt0 with genuine MissingKey remains accepted. R1 identifies
the actual remaining incomplete early-return path rather than disputing those
executed neighbors or substituting signature counts for integrity.

## Evidence adjudicated and coverage limits

Read actual attributable logs, not merely SUCCESS text:

- raw/T4-native-complete.log: independently counted 274 passing cases in 25
  groups, including 10 doctests (one compile-fail), old GTS regressions and both
  actual public suites. Mixed Ed/composite/Ed/composite, binary id, signed MMR
  footer, recovery index count and atomic thirteen-append refusals are meaningful
  caller demonstrations. They omit R1's five counterexamples.
- raw/T4-wasm-complete.log: 11 composite and 10 Writer/file groups actually
  execute and pass in wasm/Node, not just compile.
- raw/T4-clippy-final-qualified.log: final GTS all-targets -D warnings finishes.
- raw/T4-docs.log: strict public rustdoc finishes.
- raw/T4-consumers-complete.log: actual RDF/wasm/C/Python library native check
  finishes. This is consumer compilation, not binding runtime or all-crate wasm.
- raw/T4-helpers-complete.log: 81 enforced jobs, 91 distinct rows, 1818 files;
  raw/T4-shards-complete.log covers 42 members. A census pass does not supersede
  the explicit governed macro instruction in R2.
- raw/T4-format.log, raw/T4-consumer-docs-format.log, raw/T4-whitespace.log,
  raw/T4-manifest-check.log and raw/T4-deferral-scan.log cover final formatting,
  both new files, exact hashes and marker absence. Independent tracked diff
  whitespace check returned zero.

The two consumer module changes are prose-only development-identifier removal.
Reuse of their earlier compiler closure is justified by unchanged API and
implementation; final prose formatting does not mean another compiler run.
The eight GTS source/test identities are those qualified by final focused logs.
Primitive/primary IETF/NIST/frozen-vector bytes and dependency/feature inputs
remain unchanged, so their independently qualified Task 1-3 observations apply;
this reviewer does not claim to have repeated that external fixture audit.

No full workspace gate, hosted CI, hook, signing, push/publication, full binding
runtime, hardware timing measurement, post-drop memory inspection or certification
was executed by this reviewer. Composite -58 remains pinned provisional draft
support. No shared cross-engine composite GTS corpus claim is made. Required
R1/R2 corrections and exact-source independent recheck must precede Task 4
normal signed transport. Tasks 5-6 and final stages remain their planned work.
