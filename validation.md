# Current acceptance evidence

Issue 457; branch paudley/457-paged-tier-an-lsm-style-stack-of-sealed.
Initial/source HEAD: c1d5bcd259089769e3679c6e1926b5ce24fbc1c2.
Base observed at intake: origin/main at the same commit.
PR: https://github.com/Blackcat-Informatics/purrdf/pull/466 (OPEN).
Current committed HEAD: d48be7c960adfeaba44d642630cde4a66d7e86e7.
Current source tree: 5d150fa4076cc75e293b517f79dd080b6ea807ed.
G1 HF1/F1 repair independently reviewed PASS; normal signed hooks/push/exact
remote branch readback passed. tasks/G1-implementation.md and G1-review.md bind
the staged/committed tree, cold fail-before/pass-after witness and focused evidence.
G2/PF1/PF2 independently reviewed PASS and fixed in the earlier G2 signed commit;
normal hooks, push and exact remote readback passed. G3b generated projection is
independently reviewed PASS and signed/pushed; all final normal CI checks now
pass. Final completion/review-debt and Stage3 archive/integration gates remain.
Earlier PR capture identities below are historical, not current CI/merge
qualification.
Selected `.stage` process evidence is untracked and unignored locally. It is
excluded from source commits and separately captured by ghprsq. The Stage 1
handoff's earlier “ignored” wording is corrected explicitly in that historical
artifact; functional checks are unaffected. Hosted assembly report identities
must be compared to a clean reconstructed checkout plus its sole generated
document patch, not this local tree's additional process files. Compiler/report
qualification remains on the actual hosted compiler.
Task 2 commit has a verified good signature with the same fingerprint below;
normal hooks, push and exact remote readback passed. Update: issuecomment-6024639301.
Task 1 commit: dd9109995f056e61aea8361a80ad9977a57ec2bf,
verified good signature AF5E0032F7494CEBCAA7BBBE9B87CFBBCFDBAF11,
normal hooks exit 0, matching remote branch readback.
Task 1 update: https://github.com/Blackcat-Informatics/purrdf/issues/457#issuecomment-6024190417.
Plan: plan.md, SHA-256 ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29.
Historical integration candidate at committed G1 head: clean merge-tree against
b6f7c9b0 base equals b5a7e8cc9330b199c4f5c33f99c4f7b33bfc3156;
base is already an ancestor. raw/G1-merge-tree.txt records the result.
Historical G2 integration candidate against the same freshly read-back main base is
fc174197e0ca974f5a5ecd0c55aea760419cadd0; clean merge-tree equals the committed
tree, with the base already an ancestor. raw/G2-merge-tree.txt records the result.
Historical G3a clean integration candidate equals cb77b4d6beb3a5b7f17e3979570e651dbebe0564
against unchanged freshly read-back b6f7c9b0 main; raw/G3a-merge-tree.txt records it.
G3a source has no Rust/Cargo/profile delta, so the G1/G2 behavior qualifications
remain applicable by their identical measured production inputs.

## Current G3b projection qualification

tasks/G3b-implementation.md and independently reviewed tasks/G3b-review.md PASS
bind the exact generated document and staged/committed tree
5d150fa4076cc75e293b517f79dd080b6ea807ed. Root read the full report plus its
independent staged-byte addendum. Actual signed commit
d48be7c960adfeaba44d642630cde4a66d7e86e7 equals that tree, parent8364b29c.
Normal hooks, signature verification, push and exact remote readback exit0;
raw/G3b-{commit,signature,push,commit-identity,remote-heads} receipts retain it.
Fresh main remains b6f7c9b0; clean merge-tree equals the committed tree, and
main is already an ancestor (raw/G3b-merge-tree.txt).

Projection run37532305791/job112504470023 succeeded. Native artifact11445534280
and its complete ZIP/extracted bytes, generator/qualification/aggregate/compiler
receipts, full110-by-seven report and clean replay all independently qualify.
Post-generation source identity is
fe73fce1722108d47b7fcb7a8bcbf6214318a23800304c759282753c41b005c0;
manifestc96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a;
actual compilerEA137335/LLVM23.1.3. See raw/G3-projection-root-verification.md
and tasks/G3b-review.md for full binding and authority limits. G3b changes only
the exact generated document cells; G1/G2 functional inputs remain byte-identical.

Both updates were published: issue6026281172 and PR6026282946. The PR description
was refreshed with current qualification/counts and read back byte-for-byte
(raw/G3b-current-pr-body.md). Final ordinary CI run37537944878 and Docs
run37537944833 bind actual head d48be7c96; their actual final checks PASS.
Final seven independent shards and aggregate now PASS on the actual signed head
and candidate. raw/G3b-final-normal-assembly-verification.md binds all native
artifact ZIP digests, seven passed reports with identical full identities and
all cells equal to the projection, actual compiler and aggregate log. Actual CI
synthetic merge a582b06e0789d9fc29d71d7426846f56bda56382 has ordered base/head
parents b6f7c9b0 / d48be7c96 and tree5d150fa4076cc75e293b517f79dd080b6ea807ed,
exactly root's clean candidate. No local compiler substituted this qualification.
raw/G3b-normal-jobs-progress-7.json records all40 normal CI jobs SUCCESS and the
optional manual projection job SKIPPED. raw/G3b-final-feedback.json records all
ordinary checks, Docs and CodeQL SUCCESS, expected PR-only deployment/manual
projection skips, and CodeRabbit SUCCESS. The actual candidate is unchanged:
raw/S3-input-identities.txt, S3-remote-heads.txt, S3-merge-base.txt and
S3-merge-tree.txt bind freshly fetched base b6f7c9b0, head d48be7c96 and tree5d.
Base is already an ancestor, so there is no further integration delta; final CI
actually tested this same tree with the expected ordered base/head parents.
Earlier G1/G2 source execution remains applicable to identical Rust inputs;
final normal CI broadens qualification on the complete current candidate.
The misplaced root-owned verification receipt was moved unchanged from worktree
raw/ into selected Stage/raw; the empty owned directory was removed normally.
Current source status is only untracked .stage/; no evidence entered source.
PR description was refreshed to actual CI/WASM results while preserving the
bot-added release notes. Final completion recheck, review debt, scans/notes,
archive/integration and cleanup remain required.
reviews/review-debt-G3b-preparation.md dispositions the fresh 11 comments and
unchanged complete review/inline/resolved-thread surface, including the later
bot-added body summary. Root read that report in full. It remains BLOCKED for
final acceptance; its initial pending-assembly snapshot is now discharged above.

## Historical G3a mechanism qualification

tasks/G3a-implementation.md and independently reviewed tasks/G3a-review.md PASS
bind the exact three-file tree and full patch SHA-256
904234fb90c20182df846a8bc78c6597f14388c36896fbfab182628fdad3b45d.
Actionlint, toolchain-pin/gate-parity checks and self-tests, 17 SIMD reader
self-tests, existing metadata schema/full coverage, four make dry runs and
whitespace pass. The initial direct-Python YAML draft was refused by parity;
the final source uses the existing registered make simd-asm gate. No gate
exception, selector, instruction law or measured cell was changed.
Normal signed hooks/push/remote head readback pass, with raw/G3a receipts.
This scoped qualification permits the supported hosted computation; it is not
assembly, artifact, whole-issue, final CI or merge acceptance.
G3a progress was posted at issuecomment-6025553689/PR6025554390. Authorized
workflow dispatch 37532305791 binds head 8364b29c9fb70195f123e2bc084160d83f78850a;
projection job 112504470023 started. raw/G3-dispatch-runs.json and
raw/G3-projection-run-initial.json record actual identities/status. It is not yet
qualified. Clean detached replay /home/paudley/Active/purrdf/.worktrees/qual-asm-37532305791
has that same HEAD/tree and empty source status; the existing runtime source_identity
function gives pre-generation source hash
5cdc94d05519536689165b4fad734f27a4a23825a69df2d6071072c823aa1fec.
No compiler/Cargo invocation occurred in replay. Its sole purpose is verifying
downloaded source bytes after the hosted generated patch is available; it must be
returned to its owned clean state and removed without force after verification.

The normal G3a CI run 37532186299 successfully retained both failed x86 shard
diagnostic artifacts. The downloaded ZIP digests match native artifact metadata:
v3 9ec0031de20cc98f074c19dc851b56cb92eeb5f7ae5c0e99d5a4ef45c7659db2;
v4 049ea49dbaf47248007f41059d767dc02a5a960d90dc25f0a08e3ab6b2edd48c.
Safe archive paths and full extracted evidence are retained under raw/G3a-*.
Their synthetic merge e32f3cfd0681ab438f58d924b5d426c836c724c0 has the same
cb77b4d6beb3a5b7f17e3979570e651dbebe0564 tree as the reviewed branch/replay.
Both reports are explicitly incomplete with empty cells; they are diagnostics,
not matrix acceptance. Their source identity matches the clean replay hash above.
All eight recorded measurement input checksums pass in the clean replay;
raw/G3a-clean-replay-input-check.log retains that check. The dispatched projection
has completed both body probes and is generating the full seven-configuration
document. Generation, qualification, aggregation and artifacts remain pending.
Distinct review-debt preparation is reviews/review-debt-preparation.md; it
disposes the complete captured feedback but explicitly blocks merge on G3 and
final acceptance. It is not the authoritative final review-debt verdict.
Fresh independent completion preparation is
reviews/S2-completion-final-preparation.md (SHA-256
81229ebf4efa4cca07cd1680b42c6266c183bd88b3423724ab39e46bf50eb724).
Root read it in full. It independently discharges HF1/F1, PF1 and PF2/CR1 by
current source and attributable production executions, with no new functional
defect. Its whole-issue verdict remains FINDINGS-OPEN for G3 and final normal
CI/feedback/publication qualification. Final focused requalification remains
required. Root also read reviews/G3-hosted-body-diagnosis.md in full and verified
every supporting output checksum. It proves actual scalar gathers and correct
v3/v4 parent selection, not full-matrix acceptance or a historical cause.
The later complete normal-run jobs capture raw/G3a-normal-jobs-progress.json
shows every job completed, with only the two known x86 parity failures and their
aggregate failing. These remain historical G3a results, not final G3b CI.

## Historical G2 qualification (current source applicability retained)

tasks/G2-implementation.md and independently reviewed tasks/G2-review.md bind the
six-file manifest and complete patch SHA-256
b98f78f43a5655f11c135e1fccbd859d377f35a5bb0da4a6b729d56bb884e569.
The reviewed candidate exactly equals the committed tree. Existing Stage compiler
and governed profiles remain unchanged. No dependency or feature was added.

| Criterion | Current qualifying evidence |
| --- | --- |
| PF1 candidate work | Native graph postings bounded before slot work; 18 final paging unit cases, sparse work fixed at 3/67/1027 pages, binary comparisons 7/15/21 at 8/128/1024 postings, zero reifier-summary visits for plain 1/8/32-layer stacks |
| PF2 ID-filter descriptor work | Actual two-source/two-page provider tuples remain 4 generation/4 count/1 materialization at 1/32/256 rows; cached rereads add zero descriptor/materialization calls |
| G7-G10, typing and sticky failures | 85 affected core integrations passed final production; later one-file buffered-side fixture strengthened and its targeted final rerun passed, with affected core clippy rechecked |
| Real guarded/prepared/fold consumers | 33 evaluator integrations passed unchanged final production; both delayed drift kinds refuse ordinary/prepared/fold publication; runnable example verifies retained Bob/current Robert and eager carrier identity |
| Facade and portability | Final root facade smoke 1 pass/43 filtered; final four-package release wasm library build exit0; compilation only |
| Lints and hygiene | Four-package all-target warnings-denied clippy plus final core test recheck; helpers 81 jobs/1833 files/80 domains; thread-local inventory 53; formatting/whitespace/hash checks pass |
| Transport and review | G2-review PASS; normal signed hooks exit0, push exit0, exact remote a0f8a7ed8 readback; raw/G2-{commit,push,remote-head,commit-identity} receipts |
| Assembly/hosted/final acceptance | G3 remains open; final CI, fresh completion audit, review debt, ghprsq archive/integration and cleanup remain unverified |

The complete commands, logs and bounded reuse decisions are in the two G2 reports.
Earlier Stage 1 functional reports below remain snapshots at their recorded inputs;
changed paging/consumer inputs now qualify through G1/G2 evidence, not an assertion
that old source hashes still match. No whole-workspace local run or wasm runtime
execution is claimed by the focused G2 qualification.
Task 1 staged tree:
2e7207441e0adff64104179cd26ef568ad1225b2.
Full source patch (including both new source files), SHA-256:
60e5224cdc4e0578ed09ae3bf69b1b5fead667c252685ababf950ad6604282f3.
raw/T1-source-sha256.txt was independently verified against every source/doc file.
Latest observed origin/main: b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac
(SPARQL conformance integration, outside this run). Its relevant delta changes
five evaluator source files and no rdf-core, root toolchain/manifest or backend-contract
files. A normal signed/hooked base-to-worktree merge completed without conflicts;
its parents are the Task 1 commit and that base, tree
d5f1c3a743d325a9646e10c11b11ef83b18d5039. All Task 1 source manifest hashes
remain identical after synchronization. Task 2 consumer qualification will assess
the current evaluator; repository-wide helper qualification must use the new tree.
raw/T2-base-evaluator.patch retains the five affected evaluator source deltas.

## Intake evidence

- open.json binds worktree, branch, source base and stage directory. Scaffolds
  reviews/, tasks/, raw/, llm/ exist. stagectl brief captured the full issue with
  zero comments; issue.md, prior-art.md and brief.json retain coverage details.
- raw/open-issues.json records 23 open issues, descending from 457 to 260.
  raw/open-prs.json records six pre-existing PRs. This run processes 457 first;
  these sibling branches and worktrees are outside its mutation scope.
- Independent issue-analysis.md and prior-art-assessment.md identify cross-layer
  RDF 1.2 classification, full-vector provider drift and canonical repagination
  as necessary acceptance obligations. Prior-attempt recurrence is UNKNOWN.
- stagectl gate prior-art and gate plan passed presence checks. These checks do
  not establish the independent plan review or implementation acceptance.
- Applicable .baseline/.goals reside at /home/paudley/Active/purrdf because they
  are untracked and ignored. They were read; .deficiencies has no entries.
- core.hooksPath is .githooks. The pre-commit hook checks staged formatting,
  non-Rust ratchet, Python-extension Rust test policy and fast hygiene. Clippy,
  build, tests and full helper census are separate qualification checks.
- ghprsq resolves to /home/paudley/stage/scripts/ghprsq. Its current helper
  captures selected evidence with raw hash-object --no-filters objects and
  publishes refs/ghprsq/pr/<PR>/stage separately from source.

## Historical Stage 1 requirement status

Independent plan review reviews/plan-review.md passed against the plan hash above;
the reviewed plan was published at issuecomment-6023618413. Final independent
tasks/T1-review.md passed the exact committed tree after R1-R4 remediation.
Task 1 and Task 2 signed commit/push/readback and issue updates are verified.
Task 2 independent review PASS binds its recorded Task 2 committed tree and five-file
manifest. PR 466 exists; hosted CI is running and final integration is unverified.
The first task-update publication returned a transient GraphQL error; REST comment
readback found no task update, and a normal stagectl retry succeeded once.

| Criterion | Current evidence | Acceptance status |
| --- | --- | --- |
| Core compilation | raw/T1-check-second.log: cargo check completed, exit 0 | Preliminary; subsequent reviewer fixes require final qualification |
| Layering, value shadowing, isolation, RDF 1.2, drift, budgets, canonical bytes | raw/T1-focused-qualified-2.log: 82 tests passed, including 19 stack tests; exit 0 | Qualified by final independent Task 1 review; production hashes unchanged by base sync |
| Chronological orphan classification | Visible introduction-prefix repair; removed-before-orphan and temporary future declaration cases check typed counts and actual eager page bytes | Repaired; independent source recheck agrees |
| Direct metadata drift gating | Full check_snapshot in read_error; direct reads after skipped/zero-page drift tested without earlier status latch | Repaired; independent source recheck agrees |
| Explicit graph lifetime and indexed side probes | Separate sealed declaration sets plus receipt; lifetime tested across populated head seal; native subject indexes reused | Repaired; independent source recheck agrees |
| Shared mutable classifier and global checked interning | raw/T1-mutable-qualified.log: 42 pass; raw/T1-global-qualified.log: 23 pass; exit 0 | Qualified by final independent Task 1 review; production hashes unchanged by base sync |
| All-target core clippy, helper hygiene, formatting and whitespace | raw/T1-clippy-handoff.log and raw/T1-helpers-qualified.log, exit 0; fmt/diffcheck exit 0 | Task 1 source qualified; helper census must refresh after Task 2/new base additions |
| Task 1 signed commit and transport | raw/T1-commit.log, raw/T1-push.log, raw/T1-remote-head.txt | PASS; committed tree matches independent review tree exactly |
| Public evaluator/prepared parity and refusal boundaries | raw/T2-evaluator-qualified.log: 32 pass across new stack7/fallible17/paged e2e7/delta1; final new stack7 rerun raw/T2-new-final.log passed after mechanical lint repairs | Qualified by independent Task 2 review against committed tree |
| Runnable consumer and actual eager carrier identity | raw/T2-example.log: retained Bob reader2pages202bytes, current Robert reader4pages1088bytes, canonical ordered carrier identity verified | Real public entry point passed; final source applicability to be bound by handoff |
| Supported RDF/umbrella exports and real consumer | New symbols routed through existing rdf reexports; raw/T2-facade.log: intended root smoke passed1,43 unrelated filtered | Independent Task 2 adjudication PASS against committed tree |
| Portability and expanded all-target/helper qualification | raw/T2-wasm-qualified.log: release core/eval/rdf/purrdf build exit0; raw/T2-clippy-qualified.log all-target fourpackage warnings denied exit0; raw/T2-helpers-qualified.log81jobs1833files80domains exit0 | Executed on five-file manifest; wasm compilation only, no wasm runtime execution |
| Effective gate compilation profiles | raw/T2-profiles-qualified.log: 984 Cargo-resolved units,42members,two gate invocations, actual opt3/assertions/overflow enabled; exit0 | PASS; checked effective profiles rather than manifest prose |
| Review debt, independent completion, actual integration candidate, hosted CI, ghprsq audit and cleanup | Not yet performed | UNVERIFIED |

## Historical next-action snapshot before G3 qualification

Stage 1 implementation/publication and Stage 2 G1/G2 source repairs are complete.
Initial historical-fit, gap, specialist and completion reports remain attributable
BLOCKED/FINDINGS-OPEN snapshots; G1-review.md and G2-review.md independently
discharge HF1/F1, PF1 and PF2/CR1 against their verified candidates. G1 publication
is issuecomment-6024989641/PR6024989954; G2 publication is
issuecomment-6025382830/PR6025383432. The CR1 reply discussion_r4200408253 binds
the qualified G2 commit; thread PRRT_kwDOTKq-Ms6ppN8W is resolved, as captured in
raw/G2-inline-reply.json and raw/G2-thread-resolution.json. The source and Cargo
writers for G1/G2 have stopped.

G3 is active and blocks merge. Its three-file G3a mechanism/descriptive patch was
independently reviewed and normally published at the current head above. Source
ownership is released; measured cells, selectors and instruction laws are
unchanged. Root is dispatching supported hosted projection/body evidence.
G3b then applies the verified generated document and
requires final normal seven-shard/aggregate CI. No whole-issue completion is
claimed from passing G1/G2 checks or resolved CR1 alone.
Hosted CI at b2edf745 has x86-v3 and x86-v4 SIMD assembly documentation/measurement
mismatches for core.paged-map-quad (v3 document14 versus measured0). Downloaded
completed job logs via native job-log API after run-log CLI refused the still
running parent workflow. These failed checks block qualification; selector/compiler
and final-source applicability diagnosis is in reviews/S2-simd-diagnosis.md.
Actual hosted compiler ea137335b78829b4514bf1b4c16302f74fab8581/LLVM23.1.3
differs from local4b6d04e/LLVM23.1.1. Manual rustup installation was rejected by
the policy guard because Stage owns toolchain installation/selection; no toolchain
or Stage state changed. reviews/S2-simd-execution-path.md defines the supported
hosted opt-in full-matrix/body/artifact path, including clean source replay;
normal hosted shards and aggregate remain required. Historical Task2 five-file tree
13496228db0e5ec79074acad9020cac5aab7556e and full patch SHA-256
040958eab3e246bd0b1d15f3ba51e39bf07d0f8d6a2cc49aefb86c572ab42a84
are recorded in tasks/T2-implementation.md. Final independent verdict PASS and
normal hooked/signing commit, transport verification and issue publication passed.
Fresh final completion judgment, complete feedback/check capture, Stage 3 review
debt/binding gates, ghprsq archive/integration verification and scoped cleanup
remain required. Keep current source/check identities and criterion coverage here;
retain raw command evidence in tasks/ and raw/ as execution proceeds.

## Current Stage3 binding and next action

G3a/G3b source and full hosted projection are complete. Final ordinary CI37537944878
and Docs37537944833 are completed SUCCESS at head d48 and candidate tree5d;
48 of50 total PR check entries succeed, with only expected deployment/manual
projection skips. Complete source feedback contains one resolved thread and no
unrepaired actionable defect. The new bot separation warning and updated default
docstring percentage are declined with concrete posted rationale at
PR issuecomment6026605515; tasks/S3-feedback-dispositions.md and exact native
readback raw/S3-decline-readback.md preserve it. Initial readback comparison saw
only jq-r's added newline; jq-j restored native exact bytes and cmp exited0.
raw/S3-feedback-post-disposition.json captures all12 comments after publication.
Source/prose scans and all eight negated false-positive dispositions are
raw/S3-scan-{exits,dispositions}.md/txt and S3-{source,prose}-scan.txt.

Final independent completion reviews/S2-completion-final.md PASS (SHA-256
9a75aad64fc4e89ef033f59f203666f98a3fb28334445208a2b40f77f75bae70) is fully read
and qualifies every current in-scope criterion, actual consumers, entire measured
matrix, final check surface and public feedback dispositions. Stage3 reuses this
audit on the same current head/base and exact tested/predicted candidate tree;
no new source, requirement or integration delta invalidates it. Distinct
review-debt.md PASS and final binding addendum are fully read; SHA-256
c40cff4e46202ca513a1ab92364a924ebec436599034b6155e0ce8da6d82fe46.
The independent addendum qualifies all14 comments, unchanged full reviews/inlines,
fully paginated resolved thread, actual notes/issue and PR readbacks/scans and
completion applicability. All auditor/source/Cargo writers have stopped.
merge-ready presence gate passes; actual independent verdicts govern.

Stage2 final acceptance published issue6026645773 / PR6026646288. Final seeded
and replaced squash notes published issue6026649333 / PR6026649851; native
raw/S3-notes-readback.md equals the actual notes file. Fresh complete
raw/S3-final-feedback.json contains14 comments and all50 final checks satisfied;
reviews/inlines compare exactly to prior qualified captures, and current
raw/S3-final-threads-complete.json preserves all pagination and resolved state.
Fresh raw/S3-final-{input-identities,remote-heads,merge-tree}.txt equal prior
qualification. Final source/notes/plan/current-public-prose scans establish no
new hits: the four PR-only hits are the same historical explicit negations
already dispositioned, with no new notes/update deferral.

Selected evidence is now frozen after this final index update. Root retains
capture/merge and post-publication receipts outside the selected frozen directory
at the protected checkout's task-owned .stage/public-merge-457-466, preserving
all other existing Stage directories. Refresh inputs once more, verify clean
source and capture compatibility, then archive/integrate with ghprsq. Verify
every signed/result/archive/forge outcome and clean only this issue's worktrees
and branch. Historical pending/failing snapshots remain for provenance and are
superseded by current acceptance. No merge/closure/cleanup outcome is claimed yet.
