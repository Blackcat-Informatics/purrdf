<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent focused Stage 1 plan review

VERDICT: PASS

Stage 1 completion audit: PASS. No required findings or user scope decisions remain in the reviewed plan. This is plan qualification, not implementation qualification.

## Identity and permitted scope

Reviewed on 2026-10-06. Worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`; branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`; HEAD and captured origin/main base `ce3c07192aba1e36666062c00f958670a827cfb5`. The authoritative `plan.md` SHA-256 was independently checked as `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Read the complete issue title/body and zero comments, issue-analysis, prior-art, prior-art-assessment, brief.json, worktree AGENTS.md, parent `.goals` and `.baseline`, and the Stage 1/Stagectl focused planning, completion, quality, delegation, validation and no-deferrals instructions. Inspected actual COSE, writer, verification, compaction and RDF certification source, the dependency/layer declarations and existing Keccak home. A focused memory registry search returned no relevant evidence. No source edits, tests, commits, pushes or forge mutations were performed; this review is the only artifact written by this reviewer.

The issue is the authoritative request for the four capabilities. No new binding or CLI key-discovery requirement appears in its body. The upstream corpus observation is attributable to the independent prior-art report, pinned to its recorded upstream tree; I did not repeat forge retrieval. Technical draft facts below were independently checked against primary sources.

## Completeness against the four allowed grounds

| Requested behavior | Planned production path and falsifiable acceptance | Result |
|---|---|---|
| Composite alongside EdDSA, dispatched by declared alg | Tasks 3–4; public COSE APIs and actual Writer → reader → `verify_file_with_keyring`; exact envelope, alg and signature sizes, valid outputs from both algorithms, frozen EdDSA bytes | Mapped and falsifiable |
| Both halves mandatory in one Sign1, refusing stripping, zeroing, replacement and swapping | Tasks 3–5; each half independently mutated, cross-message/key splice, reorder/truncation/extra bytes, header downgrade, wrong key and COSE_Sign envelope; writer-produced file refusal | Mapped and falsifiable |
| IETF pairing and FIPS deterministic vector/hedged production variants | Tasks 1–4; pinned representative/context/encoding, independent primary known answers, distinct valid hedged signatures, actual writer with fallible entropy provider and typed failure without state/output mutation | Mapped and falsifiable |
| Shared cross-engine vectors when published | Task 6 plus Stage 2/3; refresh upstream before PR/merge, replay published composite fixtures, retain existing shared vectors, explicitly identify unpublished composite evidence as unverified | Mapped with the issue's own publication condition |

No UNFALSIFIABLE CRITERION, PRE-EMPTIVE DESCOPE, UNMAPPED REQUIREMENT or PLANNED-BY-REFUSAL finding was established. Tasks 1–2 are necessary substrate, not test-only substitutes for the requested production feature. The final production demonstrations in Tasks 3–5 and contract rows 107–113 require genuine successful composite authoring and verification. The declined work at lines 118–124 consists of additional parameter sets, encryption, discovery/trust policy, another published crate and unmeasured SIMD tuning; none silently cuts a requested behavior.

The mechanical plan-only deferral scan returned zero hits. The conditional absence of upstream composite vectors is not a deferral introduced by this plan: the issue explicitly conditions that conformance on publication, and the plan both refreshes the condition and requires independent cryptographic fixtures now.

## Repository constraints and coherent design

The plan obeys the real governing clauses: AGENTS says “Every job the workspace implements once has one home,” “NO semantic Cargo features, ever,” and “Everything is wasm-able”; `.goals` requires Rust-first, hard failures and maximal portability. SHAKE extends the existing Keccak home; Ed25519, SHA-512 and CBOR stay in their existing implementations. A single public native ML-DSA module in GTS is coherent for the current actual consumer set: COSE owns signing, and RDF certification already depends on GTS. A separate published lattice crate is not required to preserve one implementation and would add package/release surface. No concrete second consumer establishes a need for that split today.

Native ML-DSA is substantial security work, but the plan acknowledges that through independent primary fixtures, canonical decoding, constant-time arithmetic/design review, secret treatment and real target checks. It does not weaken the banned dependency rule to adopt RustCrypto's forbidden transitive `signature` dependency. NTT constants derived from their mathematical definition preserve the original-implementation constraint. Required hooks, signed logical commits, focused reviews, remote identity checks and actual ghprsq integration remain explicit.

The strict shared Sign1 parser corrects relevant existing flaws rather than introducing another acceptance path: current `cose.rs:79–99` ignores consumed length, tag and payload; `verify_sig:111–120` ignores protected alg; `verify_signatures:125–146` makes malformed unresolved input unverified. The plan's lines 81–89 require one algorithm-aware parser and distinguish malformed/unsupported from unresolved supported envelopes. That directly improves robustness within the requested dispatch work.

## Provisional -58 and actual draft semantics

The pinned [JOSE/COSE draft-04](https://www.ietf.org/ietf-ftp/internet-drafts/draft-ietf-jose-pq-composite-sigs-04.html) requests -58 and contains a COSE ML-DSA-65/Ed25519 example. Its construction uses an empty application context while passing the fixed algorithm label as the pure ML-DSA context. The plan's representative and two-component order match the [LAMPS revision 19](https://www.ietf.org/archive/id/draft-ietf-lamps-pq-composite-sigs-19.html) construction. The [current IANA table](https://www.iana.org/assignments/cose#algorithms) leaves -58 unassigned and registers standalone ML-DSA-65 separately as -49.

Therefore -58 is a justified draft-contract choice within this issue's IETF request and the existing GTS algorithm-agile envelope, with the material risk correctly disclosed. It is neither a final-registration claim nor a fabricated private protocol. Preserve the exact pinned draft designation in shipped API documentation and PR risk text as planned.

## Entropy contract and downstream utility

The real writer currently signs before publishing state (`writer.rs:1105–1125`), but `add_frame` calls `add_frame_with_options(...).expect(...)` and typed convenience methods call it (`writer.rs:956–982`, 1129 onward). Lines 91–98 and Task 4 explicitly require propagating composite entropy failures and resolving these infallible contracts without hidden panic or incomplete output. This is sufficient planned coverage; implementation review must check every convenience caller actually changed by that decision rather than treat the fallible method alone as proof. Provider error after one successful append is a useful concrete test for unchanged prior bytes/head/index and safe retry; it refines the already required atomicity demonstration without adding scope.

Compaction requires a packaging signer (`compact.rs:961`, 1151–1154). RDF detached certification calls the shared parser/verifier and keyring (`gts_certify.rs:644–668`, 806–815), and packaging contracts are Ed25519-specific (`1004–1020`). Task 5's successful composite source → compact/certify plus tamper refusal and retained exact authorship bytes covers these concrete consumers. Keeping existing OpenPGP discovery Ed25519-specific follows the explicit key-discovery exclusion; it does not excuse leaving explicit caller-supplied composite keyrings unwired.

Two implementation precision notes do not block this plan. First, the JOSE draft COSE example has an attached payload and a protected kid; replay its component signature/message representative through the public composite primitive, then prove GTS's detached form through the COSE/writer APIs. Do not weaken the planned detached parser or alter external fixture bytes merely to replay that example. Second, component replacement tests should use corrupted or cross-message/key material as already planned; arbitrary valid same-message signatures are not promised to be rejected by the specified combiner. Neither note changes the accepted requirements.

## Disposition

Required findings: none. Actionable rewrites: none required. The plan can proceed to Task 1 under the existing authorization. Tests, constant-time implementation review, native/wasm runtime parity, shared-vector refresh, hooks, hosted review and merge evidence remain to be produced against the eventual source identity; this PASS makes no claim that those implementation gates have run.
