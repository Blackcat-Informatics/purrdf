<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 gap analysis

VERDICT: FINDINGS-OPEN. One measured required reader-performance finding is
established. A failed hosted CodeQL alert check requires disposition; its three
inspected witnesses do not establish cryptographic vulnerabilities. Specialist
security/quality/structure reports, complete later feedback/check capture,
current-base integration evidence and independent exit completion audit remain
open. No Stage 2 clearance or merge acceptance is supplied.

## Identity and review authority

Issue 458; PR 464; Blackcat-Informatics/purrdf. ROOT
`/home/paudley/Active/purrdf`; W
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
S is W's `.stage/purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Independently observed HEAD `52988974f2d11a40214281648983f9145af7a3fb`, tree
`43b0817e30a3ad110cd013796a064c177a27e067`. Index/source are clean except
untracked selected Stage. All 52 files independently passed the final manifest
readback. Manifest SHA256
`e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d`.
Whole branch patch SHA256
`7dc93edbf07cc40c04a003a21af65498336992989728a9a244aa1f29d0e530a9`.
Approved plan SHA256 independently remains
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Implementation base is `ce3c07192aba1e36666062c00f958670a827cfb5`; initial
PR/feedback/CI base is `0d6575a46088e9420d73654b7b5965b9580c5d36`; observed
current origin/main is `090b14bb8c00e2504078cb694d278ee2d614443a`.
Parent's merge-tree result `134045aacaeb48a7ac1bfb1d4f3c71f8ae62e2a4` has
not been executed here. Branch evidence is not combined-tree evidence.

Read Stage 2/Stagectl skills and shared no-deferrals/quality/validation; actual
root/worktree AGENTS and root baseline/goals, user authority, GTS signature,
preservation and UTC timestamp provisions. No applicable ADR is identified in
tracked inventory or captured briefs. Historical-fit was read before this
analysis, SHA256 `4c37ac0152f5f6b4fdc0822a5459904ded60bdd92287688dca8e9cf3019cf813`,
VERDICT FIT; it does not preclear these gaps. Fresh issue analysis SHA256
`9665af0375750f18162cd13a44ba77eabc66e3120f0f7f539d14621d5093baf0`, entire
issue/nine comments, fresh prior art/brief/branch/review artifacts, immutable
plan, all six current task reviews and preserved initial failed reports and
material correction reports were read. Latest checkpoint URLs are issue
6019724668 and PR 6019728240. Neither grants a scope cut.

Read complete initial paginated PR comment/review/inline/thread captures,
actual PR body and confidence, separate initial completion audit, current
CodeQL artifacts, performance probe and terminal comparisons. Prior Stage 1
independent review included actual parser/combiner/Writer/provider/keyring/
reader/compactor/certifier computations, primary fixture provenance and public
native/wasm execution evidence. Those source-bound judgments are explicitly
reassessed below, not accepted merely because their verdicts say PASS.

No source/index/forge/private-memory mutation, child delegation, fetch, model,
service/GPU action or expensive repeated suite occurred. Only selected Stage
scan evidence and this report are written. A lightweight memory search supplied
no relevant factual premise. A mistaken lookup for shared references under
stage-2/references returned missing files; the actual stagectl references were
then read. This tooling miss supplies no qualification claim.

## Required findings and actionable closure

### G1 — MEDIUM, measured avoidable reader bookkeeping

Affected code: `crates/gts/src/reader.rs` quad observation and segment state;
`compact.rs:546–552`, `ProvenanceSubjects::observe`. Every ordinary quad
currently enters a subject map, even when the segment header cannot possibly
support a streamable packaging index. The layout gate is applied later by
`packaging_role`; it does not prevent the earlier work/state. This is an
introduced hot-path allocation cost under the standing maximal-performance
goal, not merely a speculative concern about map choice.

The CPU-only public producer/consumer probe `raw/S2-reader-performance.rs`
generates an ordinary 50,000-quad input with 50,000 distinct subjects and a
100-subject repeated control, then executes real eager `read` and evented
`read_to_sink`. Both baseline and head consume identical bytes/digests and
report identical counts/no diagnostics. The existing shared counting allocator
measures the entire read after warm-up. Both ordinary inputs are produced with
`Writer::new("purrdf.gts")`, without a streamable layout claim; a genuine
mandatory-signed pack is a separate positive control.

| Input/mode | Baseline requested bytes | Head requested bytes | Extra allocations |
|---|---:|---:|---:|
| ordinary unique, eager | 115249595 | 117477991 | 15 |
| ordinary unique, evented | 116200709 | 118429105 | 15 |
| ordinary repeated, eager | 40977643 | 40982023 | 6 |
| ordinary repeated, evented | 35751957 | 35756337 | 6 |

Unique input adds **2,228,396 requested bytes** in each reader; repeated input
adds 4,380. Requested allocation volume is not retained bytes or an equal RSS
increase: retained is zero after reads, and evented peak can be unchanged.
Normal/reversed comparisons reproduce these exact allocation counts. Timing
ranges show host noise; no precise speed regression percentage is asserted.
The scoped performance reviewer independently agrees G1 is required MEDIUM;
its final report/profile identity is pending and must supplement this finding.

Intended fix: gate reader-local provenance observation/state by the actual
segment capability/layout, preserving one shared classifier for genuine
streamable segments and materialized content projection. Do not filter only
currently typed subjects: foreign predicates before type assertions must still
invalidate genuine candidates. Do not weaken source-head/timestamp/closed-shape
semantics or lose exact authored COSE; do not remove mandatory packaging.

Executable closure: rerun the identical attributed allocation probe for both
reader modes, ordinary unique/repeated controls and genuine pack. Add meaningful
native/actual wasm reader/pack/repack controls covering segment reset, foreign
before/after type, invalid/non-UTC authored source shapes, valid UTC positive
neighbors and exact carried signatures. Select checks for changed reader and
consumer closure; source-bound unchanged primitive evidence remains reusable.
Commit boundary: one coherent reader capability/performance repair with normal
hooks/signing/push. Publish original finding, measured before/after behavior,
focused checks and independent G1 verdict on both issue and PR. Parent owns
dispatch/remediation; no future ticket or deficiency entry closes G1.

### G2 — HIGH hosted acceptance gap; CodeQL witnesses are false positives

`raw/S2-codeql-check.json` captures check 112352384442, exact HEAD529,
completed FAILURE, three annotations, titled three critical vulnerabilities.
This supersedes any assertion that all captured checks are only pending.
The analysis workflow jobs can succeed while the separate alert check fails;
workflow success must not be used to hide that distinction.

Actual annotation adjudication:

1. `mldsa65/mod.rs:312`, `let mut nonce = 0u32`, is FIPS 204's ExpandMask
   counter κ, not the production randomizer and not an encryption nonce.
   `sign_hedged` derives the 64-byte private seed from secret K at bytes32..64,
   the supplied fresh randomizer and message representative μ. `mask` hashes
   that seed with the two-byte counter plus polynomial index. `take_nonce`
   reserves all L=5 indices, refuses before u16 wrap and increments by five.
   Captured primary `raw/FIPS204.txt` Algorithm7 lines1513–1517 specifies
   ρ″=H(K||rnd||μ), κ=0 and ExpandMask(ρ″,κ); Algorithm34 lines2139–2147
   appends κ+r in two bytes. All official deterministic/hedged answers already
   execute. Changing κ's initial value would break the specified construction.
2. `tests/hedged_writer.rs:396`, fixed `[1;32]` Encrypt0 key, and line397,
   fixed `[2;12]` IV, are inside a public signed-opaque MissingKey regression.
   One fixture encrypts controlled text, reads it without the decrypt key,
   and asserts that the valid author signature authenticates opaque bytes.
   They are test-only fixture parameters, not shipping default keys/IVs.

No cryptographic defect is demonstrated by these three witnesses. G2 concerns
unresolved hosted acceptance/disposition, not a CRITICAL algorithm finding.
Dedicated security judgment remains pending. Intended closure: capture full
alert identities/dataflows, record the precise false-positive rationale through
normal authorized forge disposition, and read back current required-check state.
Do not randomize FIPS counters, change fixture semantics, ignore the rule or
weaken check policy merely to turn it green. No source commit is prescribed
without an actual source defect; forge/evidence disposition has its own receipt.
Publish outcome on issue/PR and include it in the independent exit audit.

### W1 — MEDIUM open integration qualification, no combined-tree failure observed

Read actual captured-base to current-base delta: 34 paths. The first 19 affect
translation/book/prose and glossary/license gates. The newer 15 affect OWL-DL
entailment/validation/bench/changelog and Cargo.lock. Branch Cargo.lock adds
existing sha2 to GTS; newer base adds existing purrdf-alloc-probe to entail.
These are distinct package entries, no observed dependency collision or new
external package. Makefile changes `book-zh`, not the local crypto gate;
glossary file selection changes and license prose are still integration policy
inputs. Syntactic merge-tree success alone is insufficient qualification.

Parent must bind fresh candidate/base/head, inspect merged lock/declarations
and applicable policy gates, then execute any newly affected closure checks.
Reuse exact upstream/head qualification only when inputs/consumers/profile match.
No arbitrary blanket full rerun or synchronization is prescribed absent a named
gap. Record this assessment in validation/integration evidence; Stage 3 must
refresh if base advances again. No source commit needed unless actual conflict
or behavior failure is found; publish concrete candidate/check identities.

### W2 — MEDIUM Stage 2 specialist/feedback/exit gates still open

Security and quality/structure specialist conclusions are pending. Performance
evidence establishes G1, with final specialist attribution pending. Initial
completion audit `reviews/S2-completion-initial.md` FINDINGS-OPEN maps genuine
public signing/verification/pack/certifier demonstrations but does not cover
these later outcomes. Fresh complete comments/reviews/inline/threads/checks and
fresh independent exit completion audit are required after remediation.
Captured review and inline pages are terminal empty `[[]]`; threads empty with
hasNextPage=false at HEAD529/base0d657. CodeRabbit's green status accompanies
a usage-cap comment and supplies no substantive review. Empty initial feedback
is neither approval nor permission to skip later review processing.

Parent must read all material reports, incorporate actual findings without
manufacturing defects, dispatch focused fixes, qualify and publish them, refresh
full feedback/check surface and obtain the distinct fresh exit audit. If a new
finding arises it enters this gap list/remediation, rather than being silently
handed to Stage 3. Timing/FIPS certification/full historical memory destruction
are not invented acceptance criteria. Merge-time allocation/shared corpus
refresh and ghprsq integration remain actual authorized Stage 3 work.

## Requirement and consumer adjudication on the captured branch

| Requirement/surface | Actual inspected behavior/evidence | Gap judgment |
|---|---|---|
| One Sign1, Ed and composite, protected alg | Shared complete parser/dispatch; tag18, four fields/null payload, exact protected bytes; -8/-58; typed sign and actual Writer emit composite; frozen Ed bytes unchanged. Public COSE and Writer native/wasm controls execute. | No missing implementation established. |
| BOTH halves mandatory and attack refusals | Combiner computes native ML-DSA and strict Ed25519 over one representative and accepts conjunction. Public consumers mutate either half, strip/zero/splice/reorder/change alg/key/tag, alongside valid controls. Actual file/certifier rejects corruption. | No component fallback or test-only feature established. |
| Exact IETF/FIPS contract | Pinned draft04/draft19, Prefix||Label||00||SHA512(exact Sig_structure), fixed30B ML-DSA context, MLDSA-first 3373B signature/1984B key. Complete pinned IETF fields and all70 official NIST cases independently compared/executed. | No substituted -49/prehash/context/order identified. |
| Deterministic vector versus hedged production | Explicit deterministic methods; sealed Hedged provider is required for composite Writer. Fallible append signs before state/output mutations, provider partial failure refuses, valid recovery succeeds. No OS entropy/clock/default fallback. | Functional contract demonstrated; entropy quality/full fresh fills and dedicated keys are explicit caller duties. |
| Typed resolution/policy | Exact optional/binary IDs, algorithm-compatible keys, malformed Invalid before lookup, unresolved supported Unverified, legacy Ed/OpenPGP reaches same core. Unsigned profile/integrity fix remains shared. | No discovery/trust/encryption expansion required. |
| Authorship and mandatory packaging | Positive closed segment-local classifier; materialized projection shares home; detached pairs exactCOSE and currentsourcehead-selected root; final index mandatory Ed/composite signer; verify_compaction checks actual authors and ordering. | Functional history closure demonstrated; G1 allocation issue open. |
| Timestamp boundary | Shared existing XSD UTC predicate guards type/lang/direction/datatype before parsing, producer before signer. Original unchanged probes now retain malformed/nonUTC source content/COSE and refuse invalid parameters, valid Z/+00/-00/extendedyears/24h remain positive. | Original BLOCKED report preserved and discharged, not hidden. |
| Shared vectors, conditional publication | Captured complete upstream tree0d1c…/bbac… has only2EdDSA COSE fixtures, no composite; frozen corpus unchanged and runs. -58 unassigned as disclosed. | Conditional absent in captured evidence; shared composite interoperability UNVERIFIED, merge-time refresh required. |
| Native/wasm and one-home constraints | OriginalRust MLDSA once; SHAKE existingKeccak, sharedEd/SHA512/CBOR/XSD; no newexternalpackage/semanticfeature/copiedcoefficientarray. Current affected full native and real Node/wasm consumer evidence plus release all31 crates. | Branch qualification reusable by closure; no currentcandidate pass supplied. |

The actual evented reader observes signatures; supplied-key file/certificate
paths authenticate them. The structural compactor lacks a keyring and is not
called author authentication. Current PR prose correctly names those boundaries.
Legacy Ed parameter APIs cannot represent composite keys, but the typed general
entry points share the same implementation; preserving conveniences is not a
dark composite path. Both positive production output and meaningful refusal
are evidenced, not simply methods that return errors.

## Validation reuse, failure history and scan adjudication

Prior T1/T2 primary answers, T3 COSE/native/Node and T4 Writer/native/Node checks
apply to unchanged primitives/dispatch/provider contracts. Later reader/RDF
correction closures are covered by current T6-R1 complete GTS/RDF 1361 native
cases/13 docs and eleven actual Node/wasm consumer groups. Current workspace
clippy, strict affected docs, generators/hygiene and all31 release wasm build
are terminal successful. The earlier full make-check pass remains its captured
gate52 manifest77082e9b… and 21045native/460docs, with35/2defaultignored;
only unchanged closures are reused. It is never labeled a currentsource full
reexecution. Actual currentsource qualification review215ffa49… and Task6 final
reviewca9e3973… bind final52paths/signedpublication; neither preclears G1/G2/W1/W2.

Preserved T3 header/T4 unsignedpolicy/T5 provenance-root/T6 timestamp original
BLOCKED reports and unmodified reproducer bodies remain visible beside source-
bound corrections. In this agent's Stage1 closure replay, original timestamp
consumer and controls executed successfully against current sharedUTC rule;
logs `raw/T6-R1-independent-consumer-probe.log` SHA256
`beac11c0f4365985cca60317bdfd23c2fe2c96c35d8c2322d56deadb7bf4bfdb`
and controls `de6a169d05a9e7135917ec34419aa31a8a44519b400b9f66cc94123402076fd0`.
These are earlier real execution, not new Stage2 reruns. No new functional
probe was warranted by a missing unchanged claim; the separately assigned
performance probe supplies genuinely missing evidence.

Fresh keyword scan uses verified PR base0d657...HEAD. Source has one witness,
the explicit exclusion of distinct unrequested HashML-DSA, while required
composite prehash exists and externally matches. Prose/plan/commits/current
issue/confidence scan has zero hits (rg exit1 means no matches). Retained fresh
laundering scan has39 witnesses, matching final source dispositions: validates
seed lengths before expects, Ed-only infallible path cannot install composite,
denied-fallback documentation, existing fold optimization, and negative test
panics that enforce no lookup/entropy beside genuine positive cases. No TODO,
stub, false success, ignore/xfail/feature/skip or silentfallback hiding required
work is established. Seven deleted process exemptions are exact stale rows;
rules/selftests unchanged. Authentic replacement fixtures strengthen strict
parsing and preserve existing assertions; governed vectors are untouched.
The deficiency ledger has no entries below its marker. No user scope cut exists.

Selected raw evidence identities:

| Artifact | SHA256 |
|---|---|
| S2-gap-deferral-source.log | 1de3a053bba210dccdab057c287d8d416c83c8b91dba47ee1a6de228ee995894 |
| S2-gap-deferral-prose.log (empty) | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| S2-codeql-check.json | 702c78e75968b709a13cfcd5526fc2fcd037b91f590cab4c8e71b39cb076914e |
| S2-codeql-annotations.json | 2656d121910824106b35406602e844d119f76d25cd44918e47cb6dc0c62f7f34 |
| S2-reader-performance.rs | f9646a8549b8ed5f27e5bdbe822c5885771fd1555f87447cf03c4424a6089f87 |
| S2-performance-base-measured.log | b5b5683c64e314c4ce3f5139ec8825c1634557fd5301e3ea119f8e56088ea4bc |
| S2-performance-head-measured.log | 8bcf88af4129764b66f3be10db7ab043a2782ad41b5f5b09b8abe2019f57cf1d |
| S2-performance-head-reverse.log | f581fd1175bdef44798501f1d4ecd05c7c1197fe8c7433a5055aa9fd53bf63b2 |
| S2-performance-base-reverse.log | 7b4bda6467ac2cf75af1ac5d634177be21ebd351a68f949bb96f192b588c8457 |
| S2-candidate-initial-tree.txt | 61939620fdb5ad820b32d336febcac4b6355790b13143ee6ab98e95da360188f |

This is the full initial gap list on captured HEAD529. Pending material
specialist/feedback conclusions require an explicit addendum or replacement
report bound to the new identity. G1 must be fixed; G2 must be concretely
disposed; W1/W2 must be completed through their authorized gates. A Stagectl
presence check cannot turn these open states into a verdict of readiness.
