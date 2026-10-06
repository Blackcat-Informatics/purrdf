<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 3 independent security and consumer review

VERDICT: PASS

T3-R1 is CLOSED on the corrected identity below. No required Task 3 finding
remains. This is not whole-issue completion or completion of approved Tasks 4–6.

## Current identity and preserved history

Worktree /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519;
branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD bf7d5d14ff72f4e6a21ea6efc2b58680f4fdf33f;
integration base ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

Current complete 13-file manifest raw/T3-R1-files.sha256 SHA-256:
54d8e8204b39b971126d9398f0a3d216175edc1b9ee0ca309356af01ce67c0a6.
Current complete tracked-and-new patch raw/T3-R1-source.diff SHA-256:
cccbf183ac6de2283855cf38065511e904743fdbec92ea42bda836e5f6f75022.
Initial-to-R1 delta raw/T3-R1-delta.diff SHA-256:
64675af4e09ccfc3a00d0988f2d3a2d2cbe5addb2abe8779c278b6aae791d5f2.

Independently recalculated these hashes and checked all 13 manifest entries
against current files: all match. Independently compared manifests: only
cose/sign1.rs and tests/cose_composite.rs changed; the other 11 files are
byte-identical. Read the entire delta, current implementation report and
attributable logs. No source/index mutation, delegation, commit, push or post.

The initial BLOCKED report is preserved verbatim as
[tasks/T3-review-initial.md](T3-review-initial.md), SHA-256:
606cae7c672d1895466043e612a0d5b84801f6b49a6e5965741e630b5da42009.
A byte comparison confirmed archive equality before replacing this current
report. Its initial source identity, required finding and actual failure evidence
remain historical. The demonstrated defect was fixed and rechecked, not dismissed.

## T3-R1 closure: common-header text grammar before lookup

The sole header_labels path now routes label 3 through valid_content_type for
both protected and unprotected maps. It preserves nonnegative integer formats.
Text requires exactly two slash-separated names, each 1–127 ASCII bytes:
first ALPHA/DIGIT; continuation ALPHA/DIGIT or ! # $ & - ^ _ . +.
This matches captured primary RFC 9052 §3.1 and RFC 6838 §4.2.
split_once plus the restricted-name character class rejects additional slashes.
Empty names, whitespace, forbidden characters, non-ASCII and oversized names
fail. No trimming, case folding or normalization of received protected bytes.

The new actual public harness group correctly signs 20 accepted cases across
both header buckets, including mixed case, every permitted punctuation,
127-byte boundaries and integer zero/u64::MAX. Its 162 rejected cases cover
78 malformed text values plus three invalid nontext values per bucket:
empty/missing names, bad initial punctuation, controls/DEL, parameters,
extra slash, non-ASCII and 128-byte names. It checks parser Malformed,
supplied-key Invalid and a resolver that panics if called for malformed values.
Accepted neighbors authenticate through both typed and legacy resolver APIs
with exact protected bytes preserved. The original private validator introduces
no dependency/public API/feature or duplicate grammar home.

Independently reran the ORIGINAL standalone public consumer:
CARGO_BUILD_JOBS=4 cargo run --locked --offline --manifest-path
raw/T3-review-probe/Cargo.toml; terminal exit 0.
Evidence raw/T3-R1-review-content-type-probe.log. Cargo rebuilt the actual
corrected worktree crate with opt-level 3, assertions and overflow checks.
All seven original malformed neighbors now reject, become invalid and cause
ZERO resolver calls. Valid text/plain remains structurally accepted,
unresolved/unverified with one resolver call. Initial failure log remains
raw/T3-review-content-type-probe.log. This actual caller result closes T3-R1.

## Qualification adjudicated on the corrected identity

Read final actual evidence rather than inferring from report status:

- raw/T3-R1-public-native-qualified.log: all 11 public groups execute/pass on
  final source, including new grammar neighbors, external IETF/frozen Ed,
  both-component/envelope attacks and optional/opaque kid.
- raw/T3-R1-wasm-qualified.log: the same 11 groups actually execute/pass under
  wasm/Node; this proves runtime behavior separately from buildability.
- raw/T3-R1-clippy-qualified.log: GTS all-targets -D warnings succeeds.
- raw/T3-R1-format-qualified.log: current formatting check succeeds.
- raw/T3-R1-helpers-qualified.log: 81 enforced jobs, 91 distinct rows,
  1816 source files; no open copies or escaping includes.
- raw/T3-R1-whitespace.log: tracked patch and all six new paths have no
  diagnostics. Independent tracked diff check also returned zero.
- raw/T3-R1-native.log: independently counted 263 passing cases across
  23 groups, including nine doctests. This whole-package run preceded two
  final test-only lint/comment edits; production code did not change.
  The affected public suite, clippy, formatting and census were requalified
  on final source. This is not claimed as a second final-source full run.

Initial domain/shard/layer/banned-dependency, separate wasm-library build and
RDF consumer compile remain attributable earlier evidence applicable to unchanged
inputs and API. The narrow correction invalidates no dependency/declaration or
primitive check. No broad workspace check was repeated solely for this review.

## Initial independent crypto/fixture findings remain applicable

The archived initial review provides full source observations and independent
primary audit. The unchanged composite/fixture/provenance/terms bytes allow
reuse of that evidence. raw/T3-review-fixture-audit.log independently matches
all eight Figure 10 fields and checks all 12 primary literal lengths/repeated
values, including full 1984-byte public key, 127-byte representative and
3373-byte signature. It uses primary source extraction rather than expected
output generated by PurRDF. Current native/wasm suites replay the complete
external known answer again.

The unchanged combiner uses mandated prefix/label/zero-context/SHA512, pure
ML-DSA fixed label context and plain Ed25519 over the SAME representative.
Both checks execute and acceptance is their conjunction. Key/signature
lengths/order, weak Ed public-key refusal, canonical ML-DSA encoding, strict Ed
verification, guarded owned-secret Drop/Clone and public-only Debug retain
the initial source findings. No copied external bodies/tables or banned
dependency/semantic feature was introduced.

The sole parser retains complete-item decoding, tag/shape/detached-null,
protected supported algorithm, duplicate/cross-bucket and critical-header
rules and exact received protected bytes. Optional opaque kid separates
absence/empty/binary with no invented string fallback or empty-ID lookup.
Attached primary envelopes remain refused; the detached neighbor preserves
the actual primary preimage/signature. Encrypt0 implementation bodies and
governed frozen root vectors remain unchanged. The only R1 behavior delta
adds the demonstrated common-header constraint.

## Limits and remaining workflow

No full-workspace make check, hooks, hosted CI, commit/push, hardware timing or
formal certification was performed by this reviewer. Clearing claims remain
limited to guarded owned storage, not compiler/register/hash-state copies.
No whole-operation constant-time guarantee follows from vectors. Algorithm
-58 remains a pinned provisional request rather than final IANA allocation.
The IETF fixture is not a shared cross-engine GTS vector; that interoperability
claim remains unproven pending published corpus evidence.

Tasks 4–6 still require actual Writer hedged provider/resolver/keyring,
compaction/RDF certification and final qualification/PR workflow on this issue.
Parent still must perform normal signed hook-verified commit, push, remote
readback and task publication. No required Task 3 finding remains.
