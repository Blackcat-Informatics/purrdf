<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 issue analysis

Status: REQUIREMENTS RECONSTRUCTED. No newly demonstrated implementation defect
was established by this bounded requirement analysis. This is not a Stage 2
completion-audit PASS, hosted-CI qualification or merge authorization gate.

## Reviewed identity and authority

Issue 458; PR 464, Blackcat-Informatics/purrdf. Worktree:
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Selected Stage directory: that worktree's
`.stage/purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Independently observed HEAD `52988974f2d11a40214281648983f9145af7a3fb`, tree
`43b0817e30a3ad110cd013796a064c177a27e067`. All 52 current paths pass
`sha256sum -c raw/T6-R1-final-branch-files.sha256` from the worktree. Manifest
SHA256: `e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d`.
Approved plan SHA256 independently matches
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Captured implementation base is `ce3c07192aba1e36666062c00f958670a827cfb5`.
Captured publication/review-thread PR base is
`0d6575a46088e9420d73654b7b5965b9580c5d36`. During this review local
`origin/main` instead resolved to `090b14bb8c00e2504078cb694d278ee2d614443a`.
These are distinct observations; no fetch, synchronization or integration
execution was performed by this analyst. Parent was notified of the newer ref.

Read worktree and root AGENTS, parent `.baseline` and `.goals` (absent in the
worktree), Stage 2/Stagectl skills and quality/validation/delegation/no-deferrals,
commands and completion references. Tracked inventory contains one AGENTS and no
ADR/constitution path; review-brief also reports no cited or touched ADR. Root
and worktree AGENTS differ only by four added native-home inventory rows.
Read relevant GTS signature/conformance/preservation/timestamp provisions.
The worktree deficiency ledger has no entries below its marker. Repository law
requires original shared Rust homes, hard errors, wasm portability, unchanged
governed vectors, no semantic features or banned edges, normal hooks, protected
main and final integration exclusively through ghprsq. No scope cut is authorized.

Read the fresh issue body and all nine comments, prior-art/brief/branch/review
artifacts, immutable plan, preserved original `tasks/S1-issue-analysis.md`,
current T1–T6 reviews, material correction reports and final qualification review.
Read actual production Sign1/combiner/Writer/resolver/compactor/certifier paths
and meaningful public test bodies, final command statuses, current native/wasm
output and captured external state. No source/index/forge/private-memory change,
GPU/model/service action, child agent or expensive suite was run. This report is
the only artifact created. The quick memory registry search found no relevant
record and supplied no factual premise.

## Literal scope reconstructed independently

The title and summary request composite ML-DSA-65 plus Ed25519 **inside the
existing GTS COSE_Sign1 capability**. The four requested bullets require actual
authoring and verification, protected-alg dispatch, both components mandatory,
the IETF pairing, deterministic FIPS vectors, hedged production signing and
published shared-engine vectors when available. The rationale adds a crucial
consumer invariant: previously authored head signatures cannot be replaced by
re-signing history. The issue excludes key discovery/trust anchoring and
encryption; those exclusions do not exclude caller-supplied keys or verification
of composite authored evidence.

The nine comments publish the complete plan, six task checkpoints, failed
timestamp finding/owner, confidence boundaries and final Task 6 checkpoint.
They add concrete acceptance obligations and preserve failed-history evidence;
none grants reduced scope. The latest checkpoint is
`https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6019724668`.
Its explicit Stage 2/3 and current-base obligations remain active despite the
Stage 1 task verdicts. The plan's original progress footer is a historical
planning snapshot, explicitly identified as such in current published prose.

| Requirement | Actual production mapping | Executable expected behavior and available evidence |
|---|---|---|
| EdDSA and composite, declared alg, one Sign1 | `cose/sign1.rs`: `sign_id_hedged`, `sign_id_deterministic`, `parse_sign1`, `Sign1::verify`, `verify_sig_with_key`; `Writer::with_composite_signer` reaches the same append/emission body | A valid composite is tag 18, four fields, detached null, protected -58 and one 3373-byte signature; old EdDSA retains -8 and exact frozen bytes. Actual public COSE tests inspect this and verify both. Current complete GTS native execution and unchanged T3 native/Node witnesses are available. |
| Both halves mandatory; strip/zero/replace/swap refused | `composite::VerifyingKey::verify` evaluates native ML-DSA and strict Ed25519 over the same representative; accepts their conjunction | Independently mutate either half, splice other message/key halves, reorder, truncate/append, mix public halves, strip and downgrade alg, rewrap as COSE_Sign. `each_component_is_mandatory_and_cannot_be_spliced` and envelope tests exercise actual public verification; real Writer/file and RDF packaging/certification suites carry these attacks to consumers. Positive valid composite controls are mandatory beside refusal witnesses. |
| Exact IETF ML-DSA-65/Ed25519 construction | `cose/composite.rs::representative`, canonical key/signature decoding and the native ML-DSA home | Prefix `CompositeAlgorithmSignatures2025`, label `COMPSIG-MLDSA65-Ed25519-SHA512`, zero application-context byte, SHA512 of exact Sig_structure; ML-DSA also receives the label as FIPS context. Public key/signature order is ML-DSA then Ed25519 (1984/3373 bytes). Replay complete external IETF bytes and official FIPS answers. T2 independently joined all 70 NIST records; T3 independently extracted all eight IETF fields. Current tests replay these answers. A local round trip alone is insufficient. |
| Deterministic vectors, hedged production | Explicit `*_deterministic` fixture APIs; actual Writer's sealed `Hedged<P>`, required `RandomnessProvider`, typed errors | Fixed vector inputs reproduce exact signatures. Different explicit fresh randomizers produce different valid COSE bytes with the same content ID. Missing provider is unrepresentable on the composite constructor; provider/partial-fill failure leaves output/head/index state intact and returns Result through every convenience. Public Writer tests execute failure/recovery and actual file verification; T4 native and Node evidence covers all thirteen groups. The deterministic fixture API is not the Writer production route. |
| Existing history stays verifiable | Reader observations, typed `Keyring`/`SignatureKeyring`, `verify_file_with_keyring`; `PackagingSigner`, `compact_streamable`, RDF `compact_and_certify`/`verify_compaction` | Writer-produced mixed Ed/composite history resolves each protected algorithm and opaque id. Real compaction/repack keeps exact original `(frame_id, COSE)` authorship, binds the current detached root/proofs and signs the actual final index with mandatory Ed or hedged composite packaging. Tampered/unresolved/wrong-type evidence refuses in the certifier. Actual current eleven-group native/Node suite and unchanged original probes exist. |
| Strict common input boundaries and no hidden evidence loss | Single complete Sign1 parser; shared verification assembly; one `ProvenanceSubjects` classifier and UTC XSD predicate, strict carried/root decoder | Malformed/unsupported envelopes are Invalid before lookup; supported unresolved envelopes remain Unverified. Ordinary/incomplete/foreign/literal/malformed-time Compaction-like content remains authored content. Invalid rewrite parameters refuse before entropy; valid XSD UTC lexical bytes survive. T3 header, T4 unsigned/profile, T5 role/root and T6 timestamp defects have preserved original failures plus independent corrected probes; current code contains the required common paths. |
| Shared cross-engine vectors when published | Governed root corpus and captured authoritative upstream tree, distinct from primary fixtures | Captured 2026-10-06 14:57 UTC upstream commit `0d1c8299c9411ea4ead853e31721d42ea66f081e`, tree `bbac332e919339ab0b1a9b99dcba841f0b434464`, truncated=false has only two EdDSA COSE fixtures and no composite path. Existing EdDSA vectors execute. Shared composite interoperability remains UNVERIFIED, condition unmet in that captured upstream; refresh before merge and consume any newly published vectors under corpus policy. |
| Portability and repository qualification | Existing SHAKE/Keccak, Ed25519, SHA512, CBOR/XSD homes; public primitive/COSE/Writer/consumer paths on wasm | Reuse T1–T4 unchanged target/primary evidence and current corrected GTS/RDF/wasm evidence only after identity/closure assessment. Final current logs show 1361 native cases, 13 docs and eleven actual Node/wasm consumer groups, current workspace clippy and release wasm builds. The earlier full make-check pass belongs to its older source; changed provenance/certifier closures were rerun, not silently relabeled. |

The existing `sign_id`, `verify_sig`, tuple `parse` and textual resolver APIs are
explicit Ed25519 compatibility conveniences, routed through the shared core;
the general algorithm-aware contracts have typed suffixed names. This preserves
existing callers rather than leaving a second crypto implementation. Acceptance
must judge the actual shared production path, including composite key/file and
certifier support, rather than demand an unsound Ed25519 key argument somehow
represent composite keys. The issue's word `sign` is descriptive; no preexisting
public function named exactly `sign` appears in the original caller map.

## Boundaries, positive controls and remaining audit surfaces

No DARK, REFUSED, TEST-ONLY, duplicate production cryptography or silent
headline narrowing was established in the inspected mapping: valid composite
output is genuinely authored, consumed by reader/file verification, compacted,
repacked and independently certified through public paths. The changed wrapper
callers still reach one implementation. `compact_streamable` has no verification
keyring; its structural refusal gate does **not** itself authenticate carried
signatures. Cryptographic certification is the supplied-key
`verify_compaction` path. Evented reads observe signatures rather than authenticate
them. Current PR wording preserves both distinctions.

No new encryption, OpenPGP composite discovery, trust policy, ML-DSA-44/87,
HashML-DSA API or standalone release crate is needed to satisfy the headline.
The one scan hit, “not implemented by these pure-message APIs,” refers to
distinct unrequested HashML-DSA; the composite's mandated prehash is present.
Provider-panic test witnesses enforce refusal before entropy and coexist with
real positive sign/pack controls. Timing/FIPS certification and clearing of
compiler/register/hash-state copies are unproved and not claimed. External key
reuse or successful-provider entropy quality cannot be inferred from bytes;
dedicated independent keys and full fresh cryptographic fills are explicit
caller responsibilities, not a fallback excusing missing production support.
Provisional -58 is disclosed and not confused with registered standalone -49.

The captured PR body/meta and all initial review API pages were read. Review
submissions and inline comments are each terminal empty pages `[[]]`; GraphQL
threads are empty with hasNextPage=false on the exact HEAD and captured base.
The usage-cap CodeRabbit comment supplies no substantive completed review;
its SUCCESS check is not review evidence. Captured CI is pending, not failed or
qualified. Title-term search found no other linked work, but its bounded search
cannot prove exhaustive history; five `none` trailers do not measure recurrence
of this cryptographic defect class.

Required remaining Stage workflow evidence is precise: fresh independent Stage 2
gap/completion and justified security/performance/structure judgments on this
identity; complete later PR review/thread/check refresh and remediation of any
real feedback; assessment of the advancing base and exact predicted integration
tree with changed-closure checks; merge-time authoritative corpus/allocation
refresh; final Stage 3 audit, normal signed ghprsq integration, audit refs/notes,
issue closure and safe owned cleanup. The classifier observes every quad and
retains subject facts, so performance review should assess representative
high-cardinality/nonstreamable inputs if claiming bounded overhead; code reading
does not establish a slowdown or a performance pass. This analysis does not
invent new functional acceptance from that unmeasured risk or discharge it by
the prior task PASS.

Original Stage 1 analysis is preserved. Stage 2 must adjudicate actual current
execution applicability and all remaining surfaces above; this requirement map
supplies neither a green final verdict nor permission to defer required work.
