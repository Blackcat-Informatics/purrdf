# Stage 2 independent issue analysis

Status: COMPLETE as issue/discussion/integration analysis. This is not the final
completion audit or merge approval. No issue scope cut is identified or authorized.

## Bound identity and authority

- Issue: Blackcat-Informatics/purrdf #457; PR: #466, OPEN in captured evidence.
- Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
- Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
- Independently read local HEAD: `b2edf7450cf654d20ae97bd856d67102d0916b7b`.
- Independently read source tree: `13496228db0e5ec79074acad9020cac5aab7556e`.
- Captured PR base: `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac` (`main`). Local ancestor check succeeds; captured clean merge-tree result equals the source tree above.
- Authoritative plan hash: `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`, unchanged from independent Stage 1 review.
- Preserved original `issue-analysis.md`; wrote only this separate report.

Read full supplied issue, plan, Stage 1 analysis/review, prior-art assessment,
current validation index, stage1-handoff, review-brief.json, branch-under-review,
captured raw/S2-pr.json, S2-reviews.json, S2-inline.json, S2-threads.json,
S2-issue-comments.json, S2-merge-tree.txt and S2-deferral-hits.txt. Also read
G11 and relevant public stack/head/fold/query source, core/evaluator test
assertions, example, attributable execution logs and independent task reports.
Current source hashes match every entry in BOTH T1/T2 source manifests.

The generated review brief is JSON, not review-brief.md; its 16-file/two-logical-
commit inventory agrees with the current base-to-head diff. No hidden missing
Markdown brief is inferred from that filename difference.

Repository rules remain AGENTS.md, standing parent `.baseline`/`.goals`, and the
backend C/G contract. The worktree's `.deficiencies` has no emergency entries.
No new forge retrieval, source mutation, Cargo command, runtime execution,
publication, merge, or cleanup was performed in this analysis. Existing
execution evidence is adjudicated by attribution and unchanged source inputs.

## Discussion and feedback changes

The issue body still requests all six mechanisms and both equivalence identities.
The initial issue brief has zero comments. The refreshed issue discussion has
three comments, all by paudley: the published authoritative plan, the Task 1
commit/qualification report, and the Task 2 commit/qualification report. None
changes acceptance, authorizes a scope reduction, or requests a different
algorithm. The plan publication itself is not evidence that implementation passes.

Captured PR body accurately describes chronological generation/removal handling,
value-based dictionaries, guarded shared receipts and canonical folding, then
separates local qualification from pending hosted CI and wasm runtime execution.
Captured PR comments/reviews are empty, paginated native reviews and inline
comments are empty arrays, and the GraphQL review-thread list is empty with
hasNextPage=false. Thus there is no captured review debt to remediate at this
snapshot. This does not predict feedback posted after capture; refresh the
complete surfaces before Stage 2 exit and Stage 3 publication.

The captured check rollup has queued/in-progress CI and CodeQL jobs and a pending
CodeRabbit context. No captured hosted result is a PASS or failure. The runs are
specific captured handles, not a reason to infer that CI has completed.

## Reconstructed acceptance and current evidence

The following are the actual issue obligations, not narrower substitutes. The
evidence column names current real API paths and existing attributable checks;
the separate completion auditor must cover the complete contract and validate
all qualifying inputs before issuing whole-issue completion.

| ID | Required behavior | Real public path and attributable evidence | Analysis disposition |
|---|---|---|---|
| R1 | One or more independently sealed base generations | `PagedStack::new(Vec<Arc<PagedDataset>>)`; conflicting global ordinal-zero witnesses and cross-base evaluator joins; counted core setup/append/snapshot regression | Implemented/public, with local evidence; G3 remains within each source, cross-source repeat facts become one logical fact. |
| R2 | Stack of sealed small delta batches | `append`, `seal_head`, `sealed_depth`; head seal rebuilds only resident data; deletion-only batches retain zero-page source/tombstones; lower read counters stay zero on setup | Mechanism delivered; ordinary full-generation resealing is not the append path. |
| R3 | Mutable resident value head reconstructed by caller | Public typed `insert/remove/contains`, graph declaration, `snapshot`, `seal_head`; executable mutates and queries unsealed head; retained reader remains Bob after writer becomes Robert | Implemented/public, with real executable and evaluator assertions. Storage/log remain the issue's explicit consumer boundary. |
| R4 | Value removals apply to every older layer, later insertion survives | Ordered layer/head sets, `visible_at`, newest-first logical occurrence selection; core chronology and evaluator matrix remove/reinsert repeated facts; independent BTreeSet operation oracle | Chronology and no duplicate effective facts have concrete local evidence. Removal-only layer remains a real source descriptor. |
| R5 | One query pins all bases/deltas/head/removals as G9 snapshot | Owned `PagedStackSnapshot`; immutable source/head/removal/declaration receipts; shared `query_view`; skipped/zero-page generation and page-count drift checks through both engine entry points | Full-vector binding is implemented, including metadata/head-only paths; fault readiness remains mandatory. |
| R6 | G7/G8 accounts for every admitted page across layers | One physical `PagedQueryView`; `PagedStackEvidence.requested_origins`, exact global pages/bytes and pinned descriptors; evaluator receipt/cache/equality/zero/below-bound regressions | Existing tests exercise real ordinary/prepared guarded calls, qualified colliding page 0 addresses and failed-request evidence. No per-layer allowance is substituted. |
| R7 | Fold deltas/removals into canonical new base; consumer atomically publishes | `PagedStack::compact`, snapshot compact, `canonical_paged_seal`; actual typed records/empty graph membership rebuild into PackBuilder pages and canonical dictionary | Real fold delivered; consumer publication is explicitly excluded by issue, not missing implementation. |
| A1 | Any query sees the eager effective dataset | Public `NativeSparqlEngine::query_fallible_view` AND `query_prepared_fallible_view`; independent eager inputs, ordered solution bags, ASK booleans and CONSTRUCT bytes; joins/FILTER/OPTIONAL/UNION/MINUS/NOT EXISTS/aggregates/paths/subqueries/graphs/RDF 1.2 | Meaningful representative public-consumer evidence exists. Source-path and wider CI review remain needed for a final broad completion claim; tests are not treated as exhaustive proof of every possible query. |
| A2 | Compacted pages byte-identical to eager effective seal | Core two histories/two page bounds, actual ordered `PackBuilder::build_bytes` per output page and global dictionary values; evaluator generated histories and executable compare independent eager carrier pages | Stronger than the old canonical-Turtle check: actual page carrier bytes are observed. Both sides share the specified partitioner, while their effective inputs are independently constructed. |
| Q1 | New term identity across layers without numeric range trick | Source dictionaries remain independent; snapshot reinterns values into its own dictionary and remaps only global translation side; conflicting IDs, scoped blanks, nested triples, directional literals tested | Explicit coherent design answer; G10 local summaries remain unchanged. |
| Q2 | Removal location and G10 exactness after masks | Per-layer chronological removals; local certified physical summaries stay exact; both physical ordinary/annotation candidate streams feed logical filtering and estimates are documented conservative | Correct contract distinction; no fake exact tombstone-adjusted counts. Graph-scoped multi-reifier and orphan-prefix regression evidence matters here. |
| Q3 | Stack depth policy and evidence | No arbitrary depth ceiling/scheduler; `sealed_depth` and evidence descriptors count zero-page layers; iterative source loops | Consumer-owned depth policy answers the issue; limits still apply globally to each operation. |

## Important domain interpretation retained in integration

RDF 1.2 acceptance concerns all three typed streams, triple/literal dependencies,
and named-graph membership, not just primary value quads. `logical_rows` combines
physical ordinary/annotation candidates and resolves cross-generation typing.
`reifiers` preserves native virtual-reifier behavior, including legitimate
ordinary/reifier tuple overlap. The prior Task 1 findings repaired original
association at an annotation's visible introduction prefix, direct metadata
drift gating, populated explicit graph declarations across sealing, and native
subject-indexed side probes. Those corrections are recorded, not erased as if
the first implementation already passed.

Current G11 explicitly defines orphan typed annotations and chronological
reclassification. That is a refinement needed to preserve existing native IR
meaning across layers, not permission to test ordinary-only equivalence. The
eager oracle uses independently supplied ordinary/reifier/annotation records;
canonical page comparison includes typed records even when declaration and
annotation land on different output pages. A flat union of values that forgets
table identity would be weaker evidence and must not replace these checks.

Explicit writer graph-lifetime policy is retained separately across head seals.
The compact result is an ordinary PagedDataset whose public contract is current
typed rows and named graph membership. A populated graph's historical explicit
versus implicit writer lifetime is not representable in DatasetView or native
page carrier records. This limitation is disclosed in G11 and stage1-handoff;
the issue does not require a future writer policy to survive consumer base
replacement. It is not a waiver of current empty-graph membership equivalence.

## Evidence qualification and integration requirements

Observed logs report 82 passing focused core paging/graph cases, including 19
stack tests, plus 42 mutable and 23 global cases. Their production source hashes
are unchanged after synchronization and Task 2. Read the Task 1 independent
review's exact staged tree/hash coverage rather than relabeling that earlier
tree as current full-workspace qualification.

The current evaluator target reports seven cases passing on final spelling with
zero filtered/ignored/failures; the combined target log has 32 passing cases
(stack 7, fallible 17, existing paged 7, delta 1). The root facade smoke ran its
intended test, with 43 unrelated cases filtered, and confirms actual root exports.
The runnable public example was executed and asserted retained Bob versus current
Robert, page/byte receipts (2/202 versus 4/1088), cached prepared-repeat equality,
and one compacted output page byte-equal to the independent eager page.

Task 2 reports successful all-target clippy across four affected packages,
release wasm library compilation, refreshed shared-helper and effective-profile
qualification, formatting and whitespace. Its source manifest also matches the
current tree. Compiler capture is rustc 1.100.0-nightly, commit
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1. These are attributable
local evidence; this analysis did not rerun them. Wasm compilation does not
establish runtime wasm behavior or all hosted targets.

Base synchronization is already incorporated: b6f7c9b0 is an ancestor, and the
captured merge-tree equals the current source tree. The incoming base changed
five evaluator source files; Task 2 consumer checks ran against that evaluator.
No extra base merge or diagnostic main edits are warranted while those
identities remain unchanged. If base/head advances, reassess the new delta and
affected check applicability; clean conflict status alone cannot prove semantic
integration.

The review inventory covers changes in shared mutable classification, checked
global interning and paging/provider behavior, so regression/structure/performance
review should inspect those homes and their existing consumers, not only the
new stack file. Snapshot construction has explicit linear retained-metadata cost
and potentially quadratic cumulative publication work. No throughput win is
claimed; reusable snapshot and sealed membership metadata prevent unnecessarily
paying that rebuild per query. This cost is a disclosed reference-mechanism
tradeoff, not grounds to replace the requested stack with eager query materialization.

## Exact remaining authorized work

1. Finish Stage 2 independent historical-fit, gap/source-specialist and completion
   reviews; adjudicate their current identity and real-entry-point evidence.
   Remediate any concrete finding before claiming full completion.
2. Refresh captured complete review surfaces after processing; current empty
   arrays do not certify later feedback as absent. Bind hosted CI/check outcomes
   to the current PR head or actual merge candidate; queued/running is unverified.
3. Maintain the current validation index, publish honest dispositions, then
   perform Stage 3 actual base/head candidate and archive/squash-note gates.
4. Merge only with `/home/paudley/stage/root/bin/ghprsq`, preserving selected
   Stage evidence separately; verify signed result, parents/tree/audit refs,
   issue/PR closure and scoped cleanup before declaring the queue item merged.

No concrete missing issue mechanism or discussion-driven scope change was found
by this analysis. Final merge readiness remains UNVERIFIED because independent
completion/review debt, hosted outcomes and protected integration are not yet
established. This is actionable ongoing work, not an external brick wall.
