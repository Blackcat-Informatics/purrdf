<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 3 independent security and consumer review

VERDICT: BLOCKED

One required strict-header defect remains. The composite primitive and known-answer
construction inspected below have no additional required findings. This verdict
covers Task 3 only; it neither rejects the approved sequential caller tasks nor
claims whole-issue completion.

## Identity and authority

Read-only source review in
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`,
branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`,
starting HEAD `bf7d5d14ff72f4e6a21ea6efc2b58680f4fdf33f`;
integration base `ce3c07192aba1e36666062c00f958670a827cfb5`.
Plan SHA-256:
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Full 13-file manifest SHA-256:
`b4d3c20a6a4529ba876fcdc2fb1c2623a826afaf7a35ca6ec6a688f16e4e680b`.
Complete tracked-and-new patch SHA-256:
`62741e8be9088b900538201b17815e33ee058cb61db7c151271052fc4343b70d`.
Independently ran `sha256sum -c raw/T3-files.sha256` in the worktree:
all 13 exact current source/doc/fixture files match. Rechecked plan and patch hashes.
No source, index, commit, push or forge mutation was performed; only selected Stage
review artifacts and the isolated review probe were written.

Read repository AGENTS, parent .baseline/.goals, applicable plan unit,
implementation report, helper/layer declarations and Stage 1/Stagectl
quality, validation, delegation and no-deferral rules. Required missing behavior
blocks the task independently of a severity label.

## Required finding T3-R1: malformed textual content type is accepted

Location: `crates/gts/src/cose/sign1.rs::header_labels`, label-3 arms.
The function validates only Text-or-Integer and nonnegative integers. Every Text
passes, including strings that violate the explicitly referenced common-header
grammar.

RFC 9052 §3.1 (captured `raw/rfc9052.txt`, lines 699–708) requires textual
content type to have type-name/subtype-name syntax from RFC 6838 §4.2 and
forbids leading/trailing whitespace. Independently captured primary
`https://www.rfc-editor.org/rfc/rfc6838.txt` as `raw/rfc6838.txt`,
SHA-256 `b08ccba7e5116e61085f2e1fe447d90eee785fb0efaa448a4b4ef6ea48b03b80`.
Its §4.2, lines 419–431, requires each name to be 1–127 ASCII characters:
first ALPHA/DIGIT, followed by ALPHA/DIGIT or ! # $ & - ^ _ . +.
The two names require their single separating slash.

Actual public consumer reproducer:
`raw/T3-review-content-type-probe.rs`, isolated manifest
`raw/T3-review-probe/Cargo.toml`.
Executed `CARGO_BUILD_JOBS=4 cargo run --offline --manifest-path
raw/T3-review-probe/Cargo.toml` with opt-level 3, debug assertions and overflow
checks enabled; terminal exit 0. Evidence:
`raw/T3-review-content-type-probe.log`.
The probe consumes the actual worktree purrdf-gts through its public APIs,
with detached null, protected supported alg -8, protected label 3, a byte-string
kid, and a structurally sized Ed25519 signature. It obtains these observations:

- Valid neighbor `text/plain`: parser accepts, unresolved status unverified,
  one resolver call.
- Invalid ` text/plain`, `text/plain `, `invalid`, `/plain`, `text/`,
  `text/plain/extra`, and `text/🙂`: all also accepted; each calls resolver
  once and becomes unverified.

This is a structural validation failure, not evidence of a signature forgery.
The Task 3 contract requires malformed headers to fail before key lookup. The
present tests exercise a wrong CBOR type but omit malformed Text neighbors.

Required remediation: validate the full textual common-header grammar in the
single header-validation path for both protected and unprotected maps. Refuse
malformed text as Sign1Error::Malformed before lookup. Keep supported nonnegative
integer content formats and valid case-insensitive ASCII type/subtype names.
Cover accepted boundary/punctuation/case neighbors and rejected empty names,
bad initial characters, whitespace, extra slash, non-ASCII and >127-byte names;
assert resolver is never called for malformed text. Preserve exact received
protected bytes rather than normalizing a valid value. Reuse a genuine existing
grammar home if one is present; no unneeded dependency or new copied parser.
The fix and affected focused qualification must be independently re-reviewed
against its final manifest before PASS or commit.

## Independent primary fixture audit

Recalculated pinned primary hashes:
JOSE draft-04 `6c4288d87a14eddf8c9eaa2453edb6ea96d7134fa8673c19910048bac3315de6`;
LAMPS draft-19 `b4ee04416efc26e7d8de7c740dc3848ffe8b8cc9dcd035ad9d4e33e18844ec83`;
RFC 9052 `01eecd7f646537600e7aad665b1fa581ce6ec33dae4ef4add0997aaf38cd0a45`.
Read the construction and Figure 10 in the primary JOSE text.
An independent standalone Rust extractor, `raw/T3-review-fixture-audit.rs`,
removes only pagination lines and scans complete primary h'…' literals.
Execution terminal exit 0, evidence `raw/T3-review-fixture-audit.log`.
All 12 primary literal byte lengths equal
[32,32,8,1984,64,8,0,29,127,8,29,3373]; repeated kids/payload and seed concatenation
agree. All eight retained fixture fields match the independently extracted
primary literals byte-for-byte, including the complete 1984-byte public key,
127-byte representative and 3373-byte signature. This does not depend on
PurRDF-generated expected output or the implementer's importer.

The representative source is exactly mandated Prefix || Label || 00 || SHA512(M).
Both component signing and verifying paths use that representative; pure ML-DSA
receives Label as context and Ed25519 is plain Ed25519. Conjunction is mandatory,
with both checks evaluated. Ordering/lengths match the draft and fixture.
The attached example is explicitly refused by the detached parser; a detached
neighbor preserves its exact protected bytes/preimage/signature and verifies
with the externally supplied original payload. This preserves GTS semantics.
Complete IETF attribution and Revised BSD terms are retained with the fixture.
Root governed vectors remain unchanged. Requested -58 is disclosed as
provisional, not registered, and is not confused with standalone -49.

The first independent extractor attempt used a literal backslash-t separator
rather than the fixture's actual tab: it failed its eight-field assertion and
was corrected; only the terminal successful audit qualifies equality.
A preliminary direct rustc link attempt encountered Stage's metadata-stub
layout; it was not a behavioral pass. The actual public probe used Cargo to
resolve the current crate and dependencies normally.

## Source security and consumer observations

Composite key lengths are checked before component slicing; malformed/weak Ed
public keys are refused and ML-DSA uses its qualified full fixed-length encoding
space. Typed key dispatch prevents algorithm fall-through. Signature decoding
enforces exact concatenation and canonical ML-DSA encoding; the existing Ed home
performs strict point/scalar/equation checks. Component mutation, zeroing,
strip/truncation/append/reorder, cross-message and cross-key splicing, mixed public
halves, and classical-header downgrade are represented in the public test matrix.
A legitimate alternative signature on the same message is not falsely treated
as an attack.

Owned seed storage has guarded Drop through the shared clearing home; component
keys retain their existing guards. Constructor failures drop initialized guards.
Clone duplicates guarded owned secrets and Debug displays public data only.
Caller seed/export/randomizer copies remain caller-owned. Documentation explicitly
does not claim compiler spill/register/hash-state destruction, hardware timing
qualification, or whole-operation constant time. No entropy/clock source or
deterministic failure fallback is introduced in this task.

One parser consumes whole CBOR, restricts tags and four-field shape, requires
detached null and authenticated supported alg, validates duplicate integer/text
labels and cross-bucket ambiguity, and checks critical-list presence/uniqueness
and understood instructions. Counter-signature layers are explicitly unsupported.
Full received protected bytes are retained. Optional opaque kid preserves
absent, empty and binary distinctions; supplied-key verification works independently
of discovery. Legacy text lookup does not fabricate a conversion or map absence
to empty; malformed/unsupported failures precede lookup apart from T3-R1 above.
Legacy Ed conveniences route through the same core. Encrypt0 implementation
bodies below the extracted signing section remain unchanged.

The SHA-512 edge reuses existing workspace sha2; Cargo.lock changes only GTS's
resolved edge, with no new external package, feature or copied implementation.
The composite ledger has one implementation home and mandated domains use Domain.
No extra algorithm/key-discovery/trust policy was invented.

## Qualification evidence adjudicated

Read actual final retained logs rather than treating the implementation status
as proof. `raw/T3-tests-qualified.log` contains 262 passing cases across 23 groups
including nine doctests and ten new public composite groups, with no failures.
The old Encrypt0, Ed strictness, writer and compaction regressions execute here.
`raw/T3-wasm-runtime-qualified.log` explicitly executes all ten public groups
under wasm/Node and passes; the separate wasm library log proves buildability.
The qualified GTS all-target clippy log ends warning-free; RDF consumer check
ends successfully but proves compilation only.

Read the final helper, domain, shard, layer, banned-dependency and formatting
logs: one registered home, current generated domain registry, all explicit
harnesses covered, no graph/package/feature drift. The original declaration/
dependency gate executions remain applicable to the later optional-kid correction
because its delta does not change their inputs. Helper and relevant compilation/
runtime evidence were requalified on final source. Tracked whitespace check
independently passes; retained individual no-index checks report no diagnostics
for every new file. The complete patch deferral scan reports no added-line hits.
None of those passing checks covers the missing label-3 Text neighbors found here.

I did not repeat the full package/wasm matrix merely for review, run full-workspace
make check, hooks, hosted CI, hardware timing or formal certification. Task 2's
qualified primitive source is unchanged. Task 4's real Writer hedged provider/
resolver/keyring and Task 5's compaction/RDF certification are separate approved
units still required before issue completion, rather than Task 3 scope cuts.
Shared composite GTS interoperability remains unproven pending published corpus
evidence and must retain that precise limitation.

## Required disposition

T3-R1 is OPEN and blocks Task 3. No additional required finding was identified
in this review. Parent must fix the demonstrated common-header defect, qualify
its actual public native/wasm neighbors and obtain the narrowed independent
re-review on the final identity; this report is not a passing review for commit.

