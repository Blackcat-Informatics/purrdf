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

## Specialist addendum — all material reports terminal, unchanged HEAD529

VERDICT: FINDINGS-OPEN. This addendum supersedes the initial report's pending
specialist status and adds required HIGH G3. The original complete gap report
is preserved byte-for-byte at `tasks/S2-gap-analysis-initial.md`, SHA256
`4cdfb78180fb9dc785a9e516c58f1fc1a681497fd91f31286ad35252ace9df00`.
No earlier false-positive or functional PASS erases G3. G1, G2, W1 and W2
remain open; specialist report presence is not closure of their findings.

Read the three complete terminal reports and independently checked their hashes:

| Specialist report | Verdict / material outcome | SHA256 |
|---|---|---|
| reviews/S2-security.md | BLOCKED, required HIGH S2-SEC-1, all three CodeQL witnesses false positives | d377e71d54a97d57a846b050df2f082a600d57d74441d283263ee4420deea068 |
| reviews/S2-performance.md | BLOCKED, sole required MEDIUM S2-P1, same G1 | 9b4ee49c863a103110b84cd9835fa8438d95a1a0ab338ddc44d7b953403401d2 |
| reviews/S2-quality-structure.md | BLOCKED, independently confirms same HIGH G3 and one-home/layer repair constraint, no additional distinct defect | cb7ac5d9f302eda0e0f4f542f0fe6f44c20369ae1a9a96dcd639e0f56ced3a01 |

### G3 — HIGH, required controlled sensitive SHAKE storage destruction

This is S2-SEC-1 and the quality/structure review's G3, counted once. Parent
owns dispatch/remediation under the original FIPS implementation contract.
Actual source locations: `mldsa65/sampling.rs:14–19,57–63,86–91`,
`hash/src/sha3.rs:283–285,318–325,351–353`, and directly owned
`hash/src/block.rs:11–13,28–53`. Independently reread those computations and
the current single `secret-clearing` ledger home, purrdf_ed25519::wipe_secret.

FIPS204 §3.6.3 explicitly requires potentially sensitive intermediate data to
be destroyed when no longer needed. The captured primary `raw/FIPS204.txt`
lines921–946 states that contract; retained private-key seeds and public matrix
exceptions do not cover abandoned per-operation sponge state. Captured PDF
SHA256 `57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b`,
text SHA256 `c3f68bc2b4ebb201321c8e774d4a29a41bbc857ad3854381aadce0363f9140a0`.
No formal FIPS certification or hardware timing proof is demanded by this clause.

Observed computation: signing absorbs secret K(32B), randomizer(32B) and
message representative(64B), total128B below SHAKE256 rate136B, into directly
owned BlockBuffer bytes. Padding preserves those first128 bytes. Keygen and
secret sampling similarly absorb private seeds. Finalization copies the lanes
into ShakeReader; original absorber/buffer and later reader lanes have neither
destructors nor caller-invoked clearing. Existing SecretBytes/SecretPolys guards
cannot reach these separate owners. BlockBuffer::clear resets only filled.
The exact owned fields lack a destruction path on success or sampler-error
exit. This omission is independently established by source and the specialist's
executed safe public probe, not a conjecture about freed-memory recovery.

Read `raw/S2-security-state-probe.log`, terminal exit0: actual seeded SHAKE
absorb/finalize/640-byte squeeze executes; both types have needs_drop=false.
Its SHA256 is `aa1c8deb7c19a2a05c801cad26d1b5b12a4a95191064dac97d9de234518e8fe3`.
No freed/uninitialized memory was read, and neither actual key recovery nor
signature forgery is claimed. needs_drop alone cannot exclude explicit clearing;
the source/caller absence supplies that part of the finding.

The initial report's legitimate compiler/register/historical-copy limitations
remain legitimate, but do not authorize omitting cleanup of explicit owned
buffer/lanes. Earlier disclosure and T2/T6 functional qualification are not a
scope decision and do not discharge this requirement. G3 supersedes any earlier
implicit suggestion that all clearing obligations were outside acceptance.

Intended fix: ensure complete controlled sensitive absorber lanes, full buffered
bytes and reader lanes have genuine destruction paths across consuming finalize,
successful output, early error and unwinding. Inventory sensitive sampler byte
buffers/operation scratch and end guarded lifetimes at actual last use where
feasible. Preserve one Keccak/XOF body and one governed wipe home. Hash is the
zero-runtime-dependency root; it cannot depend on Ed25519. A coherent relocation
and re-export of the current wipe home is an available design, not authority
to duplicate it. Update all actual callers/ledger/layers/declarations coherently;
generic reset that only forgets length must not advertise erasure.

Executable closure: safe owner-module tests must observe zeroed live buffer,
lane and reader storage at cleanup, covering partial absorption, finalize
transfer, clones where applicable, success/error/unwind. Do not inspect freed
memory or replace storage assertions with needs_drop/drop counters alone.
Replay complete unchanged SHAKE/SHA3 answers, all official deterministic and
hedged ML-DSA bytes and pinned composite answers/refusals. Execute affected
native and actual wasm public production paths, preserve Writer failure atomicity
and exact signatures, qualify warnings/helper/layer/root ring-fence/dependencies,
then independently review actual changed secret callers and storage lifetimes.
No unbounded full-suite count is prescribed; broader closure is justified by
shared-root relocation and changed owned-state behavior, not ceremony.

Commit boundary: one coherent controlled-secret cleanup/home repair, separate
from G1's reader repair, with normal hooks/signing/push and verified remote.
Publication: concrete finding/standard authority, fix and storage/output/runtime
checks, exact new source/artifact identities and independent G3 disposition on
both issue and PR; update confidence/body claims as affected. No waiver,
disclosure-only change, suppression, follow-up ticket or deficiency ledger closes
G3. Parent's remediation plan must include G1/G2/G3, not soften HIGH to optional.

### G1 final measurement/profile attribution

Read final normal-profile public base/head logs, superseding preliminary runs
that omitted build-override/debug details. Both final commands are locked,
offline cargo test through their selected Stage probe manifests, native x86_64,
CARGO_BUILD_JOBS=2, identical absolute corpus and assertions. Both terminate0.
Final paired runtime units match repository opt3/assertions/overflow, dev debug1,
test debug0, package debugfalse and build-override opt3. Rustc nightly identity
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM23.1.1. Unit graphs and
`raw/S2-performance-runtime-profile-check.log` independently show both runtime
profile predicates true; build metadata defaults are not misrepresented runtime
units. No shipping baseline body or manifest was weakened.

Final artifact manifest `raw/S2-performance-final-artifacts.sha256`, SHA256
`9913312c85c1d203a39f22f213b8348fe9193e4dfbabba5345424d744d634054`, binds
probe/manifests/locks/corpus/profile graphs/logs. Base-normal log SHA256
`7727092b8e1eaf89cf038e5843ab825207297b26a3f894ae15ceb7e482d36f91`;
head-normal log SHA256
`82717d0110f8c58ef78e617c61697ab945126f33034dd3be5aa441366bfcfe2c`.
Distinct external package inventory is identical after normalizing only probe
name. Actual original alloc-probe source is identical in both variants.

The initial table's allocation totals/deltas reproduce exactly: +2,228,396
requested bytes/+15 allocations on unique ordinary subjects, +4,380/+6 on
repeated control, both readers. Eager unique peak working layout demand rises
746,312 bytes; evented peak does not rise; retained after result destruction is
zero. Genuine pack remains50,004rows with no diagnostics. Final unique medians
base/head40.10/45.15ms eager and46.65/47.12ms evented do not warrant a stable
percentage latency claim on this host. G1 is required MEDIUM from deterministic
unused allocation work, not an invented throughput threshold.

### G2 independent security disposition; W1/W2 retained

Security independently traces FIPS Algorithm7 steps7/8/11/31 and Algorithm34,
including private-seed derivation and bounded five-index counter reservation;
all three captured hard-coded-value annotations are FALSE POSITIVES. This
agrees with the gap analyzer's actual-source adjudication. Separate alert check
112352384442 still has captured FAILURE; successful extraction/analysis jobs
are not an alert-result pass. No authorized forge disposition/readback is
supplied here. G2 remains HIGH hosted acceptance gap until concrete disposition,
not a new source crypto defect.

W1 current-base integration qualification remains open. W2 now means required
source findings from terminal specialist reports must be remediated and
independently closed, plus full later feedback/check refresh and the separate
fresh exit completion audit. The terminal specialists add no other distinct
mandatory defects beyond G1 and G3; this does not invent clearance for missing
hosted/current-candidate evidence. Source/index/HEAD529/plan are unchanged by
this addendum. Current combined-tree and merge-time authoritative refresh remain
separate gates; parent is preparing remediation, not a waiver.
