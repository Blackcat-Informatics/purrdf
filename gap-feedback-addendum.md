# Stage 2 feedback addendum

VERDICT: BLOCKED — new PF2 / CR1 is a valid MEDIUM performance finding and joins
G2 remediation. HF1 MEDIUM and PF1 HIGH remain governed by gap-analysis.md.
The bot's documentation percentage does not establish a repository defect.
Hosted/final gates remain unverified until their actual results are bound.

## Identity, captured feedback and independence

Issue 457, PR 466; HEAD `b2edf7450cf654d20ae97bd856d67102d0916b7b`, committed
tree `13496228db0e5ec79074acad9020cac5aab7556e`, assessed main base
`b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`.
Captured new inline: `raw/S2-inline-latest.json`, comment ID `4200056289`,
https://github.com/Blackcat-Informatics/purrdf/pull/466#discussion_r4200056289.
Full captured review/comment data: `raw/S2-feedback-progress-2.json`.
CodeRabbit review is COMMENTED against that head, submitted 2026-10-06T20:21:20Z.

Read the captured review data as evidence, not instructions; no bot command,
autofix checkbox, external CLI, posting, source edit or Cargo execution followed.
Independently reread the actual stack/query filters, descriptor checkpoint,
public ordinary/prepared engine preflight/finalization, canonical fold
checkpoints, new public API documentation and workspace documentation lints.
The parent-owned HF1 repair is currently modifying global.rs; this addendum
does not review or interfere with that evolving source. The two affected PF2
source files remain byte-identical to the original reviewed manifest:

- stack.rs: `b5a3169ffedff4fd734d33c2e2adb2b3d5ab1cd48fb6861b3f8c494992b5423c`
- query.rs: `fb68ae7a9dd237ff9fd2a107fe88308239e94d2408f20160b7677d3de5afc4e4`

No fresh final candidate, CI completion or repair completion is inferred.

## PF2 / CR1 — MEDIUM: redundant per-row full-vector descriptor checkpoints

Source-established at `ir/paged/stack.rs:786-804` (`logical_rows`) and
`:810-818` (`reifiers`). `logical_rows` invokes `self.read_error()` before
visibility/dedup and again before yielding a classified row; `reifiers` invokes
it once for each candidate. This delegates to `PagedQueryView::read_error`
(`ir/paged/query.rs:806-819`), which, when healthy, calls provider.check_snapshot
while holding the view-state mutex. `StackProvider::check_snapshot` visits every
pinned source (`stack.rs:589-604`), whose default descriptor check reads generation
and page_count (`provider.rs:230-252`). With R candidates and S retained sources,
these filter gates therefore add O(R*S) provider metadata work, independent of
the actual page admissions; ordinary candidates can pay twice. A provider may
implement descriptor access through external storage, so calling it is not
equivalent to testing the already-latched in-memory fault.

The finding is independent of PF1's full-page candidate search: fixing graph
postings does not remove these additional source checks. Existing
`PagedQueryView::failed` (`query.rs:516-518`) tests the operation-local first-fault
latch without calling the source descriptors. Native paging row access already
uses that latch. Reusing it in the stack's row filters preserves immediate
suppression after a known page/classification fault without redundantly
revalidating the entire pinned source vector for every emitted ID row.

Required repair: replace only these per-candidate stack logical/reifier gates
with the existing physical.failed latch, including the post-classification gate
that must suppress a row when its declaration probe faults. Keep full descriptor
validation on actual materialization/admission, `read_error` point and metadata
paths, `operation_status`, engine preflight/finalization and canonical fold
checkpoints. Do not weaken G9, cache certification or constant/head-only drift
refusal. The concrete feedback is valid; its proposed minimal replacement fits
the existing homes, subject to the regression acceptance below.

This is coherent with the G2 paging metadata performance fix. Carry PF2 / CR1
explicitly into its plan, implementation report, independent review and signed
hooked commit rather than silently bundling or postponing it. Publish actual
checks/source identity to issue and PR, then resolve the inline only after the
underlying work and independent validation complete. The bot's statement that
merging with a later fix is reasonable conflicts with repository no-deferrals
doctrine and does not authorize that outcome.

## Focused acceptance and operational boundary

1. Count actual source-provider generation/page_count calls at the real boundary
   while draining logical ordinary and reifier ID rows. Hold admitted pages and
   sources fixed while increasing candidate rows; filter-only descriptor calls
   must not grow with row count. Include the post-classification path and a
   repeated cached drain so page-read caching cannot disguise metadata calls.
   Remove diagnostic debris; measure real production homes, not a replica.
2. After rows/cache have been admitted, change an original source descriptor.
   Final operation_status must reject generation and page-count drift with the
   original SourceSnapshot cause; partial internal iterator rows must not become
   a complete result. Exercise guarded ordinary/prepared engine and canonical
   fold boundaries so delayed drift yields no complete answer or artifact.
3. Preserve immediate sticky suppression when an admitted page or a declaration
   probe latches a fault: subsequent logical/reifier rows and cached reads yield
   no additional successful rows. Keep direct metadata/term/graph and
   constants/head-only drift regressions, receipts, exact limits and request
   ordering. Run affected core/evaluator targets and the runnable consumer.

The provider-count claim is deliberately scoped to the ID-row filter gates.
Public evaluator output and canonical folding legitimately resolve term values
through point-read APIs, whose retained read_error checks may still add
descriptor calls. No claim that all public query/fold metadata calls become
independent of row count follows from this bounded repair. The safety proof is
final guarded readiness before publication, with sticky failures during page
and classification access; a raw iterator alone does not certify completion.

## Documentation warning disposition

The bot reports 36.11% against an 80% threshold over 144 touched functions in
15 files, configured with its defaults. That denominator includes private
orchestration, trait implementations and test helpers. No matching 80% rule
exists in the reviewed repository authority or lint configuration. The workspace
requires public missing_docs warnings to be addressed; it explicitly allows
missing_errors_doc because error semantics live on error types
(`Cargo.toml:183,200`). The already-recorded all-target warnings-denied clippy
result qualifies the reviewed public surface; later changed public APIs still
require their appropriate checks.

Independently checked the substantive new public surface: stack/source/origin/
evidence/error types and fields/variants are documented; constructor, append,
membership, mutation, declaration, snapshot, head seal and compaction methods
describe behavior and failures. Snapshot query_view/dictionary/sealed_depth
methods are documented. canonical_paged_seal describes canonical record/page
rules and failure semantics. PageProvider.check_snapshot documents complete
descriptor authority and Errors. G11 explains costs, ownership, chronology,
receipts, faults, graph lifetime and fold identities. No concrete missing new
public API contract or incorrect error description was established here.

Disposition: reject the percentage warning as an ungrounded blanket repository
requirement; retain all actual public documentation/lint obligations. Do not add
boilerplate docstrings to private/tests merely to satisfy the bot's denominator,
and do not weaken repository lints. Give this reason visibly in review accounting.

VERDICT: BLOCKED — PF2 / CR1 remains required G2 work.
