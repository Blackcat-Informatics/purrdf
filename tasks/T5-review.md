<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 5 independent caller and integrity review

VERDICT: PASS

T5-R1 and T5-R2 are DISCHARGED on the corrected identity below. No required
Task 5 finding remains. This is not whole-issue completion, hosted CI,
publication, a PR, release readiness or merge acceptance.

## Current identity and preserved history

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD: 39f0d4dce74c7635a8d4675406cc37eb763cb590.
Captured base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Complete thirteen-file manifest raw/T5-correction-files.sha256 SHA-256:
38580bdbbf4209b77c08a608574a4fcb288c50c09c0f545d6a59d837dd430037.
Complete tracked-and-new patch raw/T5-correction-source.diff SHA-256:
cb4aa57335b216f673870c0cde85974749e95043b9dedd98a5b38a957586c8a9.
Four-file correction raw/T5-correction-delta.diff SHA-256:
96674603f12903c59b9d51ef68b7a366409c69a98662ba9ee98ac5577ecfe832.

Independently recalculated all hashes, checked all thirteen current manifest
entries twice, read the correction/current affected computations/callers, and
adjudicated terminal logs against public test bodies. Only compact.rs,
reader.rs, gts_certify.rs and gts_composite_compaction.rs changed in correction;
other nine files retain their initially reviewed bytes. Index remains empty.
No shipping source/index mutation, delegation, transport, lifecycle action or
private memory write occurred.

Original BLOCKED report remains verbatim at tasks/T5-review-initial.md, SHA-256
c394393dfaebc0bf068f627d24663fb440010aad73d949e6f3b57d7928812db7.
Actual cmp confirmed archive equality before replacement. Initial source,
probe and failing logs remain intact. Additional intermediate type-only failure
raw/T5-correction-type-only-before.log is preserved: one executed failure,
eight filtered groups, not a pass.

## T5-R1 DISCHARGED: shared positive provenance anchor

Type-quad-only packaging inference is removed. Existing closed provenance
vocabulary lives once in compact::ProvenanceSubjects; actual RDF projection
and both reader paths use it. Term::iri_value remains the exact kind/value home.
Compaction requires a blank-node subject, one reserved class, closed predicates,
singleton string agent, singleton xsd:dateTime timestamp and well-formed literal
source heads. Foreign predicates, literal class lookalikes and incomplete
mandatory-field combinations remain visible ordinary content.

Incremental classification retains per-subject shape facts without evented
content quads. State persists between streamed frames and resets per segment.
Eager folding uses the same observe/role operations. Signature.packaging remains
an observation rather than authentication.

Current nine-group public native/actual wasm tests cover ALL incomplete field
combinations, the original foreign-predicate case, equivalent vocabulary ids,
and the literal-class neighbor. They assert unchanged content projection,
packaging=false, eager/evented agreement and exact author (frame_id, COSE)
preservation through REAL composite packaging AND Ed repack, then certify both.
Genuine packs/repacks and fresh composite-authored streamable tails retain their
authors and both readers' agreement.

Independent rerun of the ORIGINAL UNALTERED public probe now observes
packaging=false, one authenticated author pair, post pairs=1, original signature
preserved=true through compact_and_certify, and all six legitimate report
fields true.

## T5-R2 DISCHARGED: strict current-subject root records

Global lexical first-match helpers are removed. One GTS compaction_root_records
decoder checks positively anchored Compaction subjects using exact IRI
predicates, literal root kind, at-most-one root per subject and exact 32-byte
hex decoding. Historical records are checked too. compaction_signature_roots
selects the current event using the actual complete pre-compaction segment-head
list, including multiplicity; missing or ambiguous selection refuses.
Unrelated matching root values cannot substitute. Legitimate historical nodes
retain their own earlier roots without a global single-root restriction.

signatures_bound_ok checks exact pre/post authorship pairs, selected root,
expected leaves and inclusion proofs. Certificate extraction uses the SAME
decoder/selection. Incoming compaction invokes the shared record decoder before
reauthoring, so malformed genuine roots refuse actionably.

The unaltered independent probe now refuses contradictory roots on the SAME
subject and nonliteral root objects: signatures_bound=false, all_ok=false.
Both signatures_verify and packaging_sig_ok remain true, correctly separating
root structure from valid cryptography. Positive rebuilt pack still passes.

Registered public cases add contradictory roots at equivalent predicate IDs,
bad hex, unrelated root subject, matching decoy hiding a wrong current root,
wrong source-head selection, and unused literal predicate spellings. Positive
lexical-shadow/rebuilt-pack neighbors pass; malformed incoming roots refuse
actual compact_and_certify. New-tail/repack selects the current root while
different legitimate historical roots remain independently queryable.

## Independent execution and final qualification

Reviewer command:
CARGO_BUILD_JOBS=4 cargo run --locked --offline --manifest-path
/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519/.stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw/T5-review-probe/Cargo.toml

Terminal EXIT 0. raw/T5-correction-independent-original-probe.log SHA-256:
d8cd0d99aa9b0bc834b601fef6290c3bd09e98c389b37eb95d51213ce33f853e.
Original probe SHA-256 is unchanged:
77d97a275ea5eacf6785da4b91493d44373c190430b25c1a423cef8cc4a322b8.
Manifest/lock/profile remain unchanged from initial review. This separate lock
is public reproducer evidence, not workspace dependency qualification.
Original EXIT 101 logs remain historical failures.

Adjudicated final-source executions, after reading actual commands/test bodies:

- raw/T5-correction-terminal-rdf.log: 44 cases, comprising 9 new public caller,
  17 existing certificate, 4 frozen streamable, 11 dictionary and 3 pinned cases.
- raw/T5-correction-terminal-gts.log: 40 cases, comprising 6 compaction,
  10 pinned, 11 COSE and 13 Writer/profile cases.
- raw/T5-correction-terminal-unit.log: 12 compact cases, 108 filtered.
- raw/T5-correction-terminal-wasm.log: all NINE public composite compaction/
  certification groups genuinely execute in Node/wasm through the existing
  runner. This qualifies changed reader/provenance/root behavior, not just build.
- raw/T5-correction-terminal-clippy.log: GTS/RDF all-targets -D warnings.
- raw/T5-correction-terminal-docs.log: strict current GTS/RDF rustdoc.
- raw/T5-correction-terminal-helpers.log: 81 jobs, 91 distinct rows,
  1819 files, no open copies/escaping includes.
- Final format/whitespace/complete manifest checks qualify current files.

Environment raw/T5-correction-environment.log identifies nightly rustc
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1,
Cargo 1.100.0-nightly, Node v26.10.0 and wasm-bindgen 0.2.125.
Native x86_64 and runtime wasm32 use opt-level 3, assertions, overflow checks
and repository warnings. CARGO_BUILD_JOBS=4 only limits concurrency.
Original runner omission, compile failures and intermediate failures stay
distinct from terminal successes.

## Retained observations and boundaries

Initial archived review's observations outside discharged defects remain
applicable: mandatory sealed Ed/composite signer, one actual compactor and
certifier, required provider/error propagation, strict carried-node Result flow,
opaque typed key resolution and exact final-index authentication. Current
public cases execute their affected integration again, including component
corruptions, wrong/unresolved keys, unrelated valid survivors, contradictory
ordering, entropy failures, exact history/proofs and unchanged frozen Ed bytes.
No duplicate compactor, parser or crypto implementation was added.

Nine unchanged source files, primitive/combiner/fixture/dependency/feature inputs
retain earlier qualification. Initial layer/shard registration and native
binding compilation remain attributable prior evidence for unchanged declarations
and caller surfaces, not a new binding runtime or final whole-workspace pass.
Example bodies and mandatory-provider compile-fail API are unchanged; current
strict docs and caller compilation support bounded reuse.

No full workspace gate, hosted CI, hook/signing/push/publication, whole binding
runtime, hardware timing, formal FIPS certification or post-drop memory experiment
was performed by this reviewer. Dedicated component keys and fresh/full
cryptographic provider output remain caller obligations. Conditional shared GTS
corpus/allocation refresh remains approved final qualification. Parent must
perform normal signed hook-verified transport/publication and remaining Stage
tasks; this PASS does not claim those actions have occurred.
