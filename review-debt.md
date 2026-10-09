# PR507 review-debt audit

Current verdict: **PASS — complete review debt closed and required current CI
green**. Normal fix commit41345 and
push54316 actually exited0 at be3fdeb84d77c1bfd5210347079a494eb96fe7c8.
Affected qualification and independent gap-analysis.md PASS; the valid ordinal
finding is addressed, publicly replied to and explicitly accepted by CodeRabbit.
The required thread is resolved and advisories legitimately declined in a
posted reply. Fresh CI37883191140 actually completed SUCCESS on that exact
head:40 successful jobs,4 intentional optional skips, zero failures or pending.
raw/pr507-current-run.json binds exact head/run/results; fresh supported
pr-checks.txt has state: pass. Complete final review/inline/thread captures show
no new required feedback. Root's final main/candidate refresh remains unchanged
aab23/f3e37b71b407eefa386fe7d6e6ce5ccfc10f8c8a. Final supported gate/notes,
ghprsq/archive/landing verification remain root delivery operations, not an
unperformed local test or unresolved feedback item.
Old-head CI is cancelled, not relabeled updated-head qualification.

Distinct auditor performed read-only source/evidence review only. No build,
forge posting/resolution, source/index/ref mutation or repeated ref discovery.
Scope: full supplied captured review surfaces and current mutable.rs delta.

## Initial captured surfaces (historical checkpoint)

- raw/pr507-reviews.json: one formal COMMENTED review5465579167 by CodeRabbit
  at20e2e637cd7bee8c18ed80f15a2b4397155f489d, one actionable finding. Its
  review configuration covers all ten changed source/manifest/test paths.
- raw/pr507-inline.json: one inline4226494288, same functional finding.
- raw/pr507-threads.json: one threadPRRT_kwDOTKq-Ms6qo_mI; isResolved=false,
  isOutdated=false. Thread and nested comment pagination both have
  hasNextPage=false. No uncaptured thread/comment continuation is indicated.
- pr-comments.md: three top-level comments: bot walkthrough/advisories, author
  posted plan/status and author publication answers/evidence explanation.
- pr-checks.txt plus raw/pr507-checks-refresh.json: earlier published-head
  captured refresh has29 SUCCESS,16 IN_PROGRESS,4 optional SKIPPED, CodeQL
  NEUTRAL. This was pending old-head CI, with no required-green inference. The neutral
  aggregate is not silently relabeled SUCCESS; fresh final capture must establish
  configured analysis/required gate outcomes.

Current captures: raw/pr507-reviews-after-fix.json has three formal submissions:
original CodeRabbit5465579167 and empty COMMENTED submissions5465652073
(author) /5465654135 (CodeRabbit) at be3fdeb84. Their substantive content is
fully covered by the inline replies below; empty review bodies do not hide a
separate finding. raw/pr507-inline-after-fix.json has three inline entries;
raw/pr507-threads-after-fix.json contains the same one thread with
isResolved=true/isOutdated=true and both thread/nested hasNextPage=false.
Updated Stagectl merge brief captures five top-level comments, including the
posted remediation plan and advisory disposition. No new required finding is
present in these complete current captures. The current bot walkthrough still
describes its earlier20e2e review scope; the exact published repair is separately
acknowledged by its4226562864 reply, not falsely relabeled a whole new review.

## Per-comment and overlapping-thread dispositions

| Captured item | Adjudication / disposition | Closure state |
|---|---|---|
| CodeRabbit top-level walkthrough, low-risk summary, title/issue/scope checks | Describes implemented typed records/LOAD scope accurately. Low-risk wording does not authorize leaving the genuine ordinal defect. Informational assessment accepted; all issue contracts remain binding. | No separate source finding. |
| CodeRabbit docstring coverage44.17% versus proposed80% threshold, finishing-touch docstrings | Advisory legitimately declined. Actual public RecordKind/RecordValues fields, insert/remove/projection contracts and errors are documented. Meaningful repository docs/strict gates qualify; cosmetic private/test padding solely for this bot ratio is declined without claiming its warning passed. | CLOSED: posted advisory6074157956, current top-level comment5. |
| CodeRabbit finishing-touch test generation, autofix/autopilot/CLI suggestions | Original meaningful regression and real affected native/WASM controls qualify; optional generated-test/autofix/CLI offers declined without changing any gate/golden. | CLOSED: posted advisory6074157956 covers these offers. |
| Author posted plan/status (top-level comment2) | Accepted durable publication snapshot: additive401 scope, original setup failure and actual retry PASS correctly distinguish earlier local qualification from pending PR integration. Later source remediation is additive to this snapshot, not a reason to relabel its old evidence. | No requested action or adverse feedback. |
| Author publication answers/evidence (top-level comment3) | Accepted truthful distinction: no measured huge-LOAD throughput claim; graph mode remains opt-in/additive; selected evidence and actual runtime correctness are described without claiming completed merge. | No requested action or adverse feedback. |
| Formal review5465579167, inline4226494288 and threadPRRT_kwDOTKq-Ms6qo_mI | One overlapping defect fixed by be3fdeb84: creation-only ordinal home preserved, independent delta target/undo/order/uniqueness regression passed. | CLOSED: actual reply4226560940, resolution JSON true, fresh thread true/outdated true. |
| Author remediation-plan comment4 | Accepted bounded owning repair; retains failed evidence and full contracts rather than replacing complete qualification with a source-only claim. | Informational, no adverse feedback. |
| Empty formal reviews5465652073/5465654135 and author inline4226560940 | Actual current-head reply accurately records normal publication, affected1232/59/native10/WASM10 and strict/fmt PASS, scope of reused full/API evidence and remaining current CI. | Accepted; no separate finding. |
| CodeRabbit inline4226562864 / amended original4226494288 | Explicitly agrees correction/regression addresses finding, says thread resolved and current CI/integration pending; original inline now marks addressed in be3fdeb. | CLOSED; acknowledgment independently read, no new source work. |

## Source and actual evidence assessment

The production delta removes exactly the unconditional three-line ordinal
minimum assignment in classify_record. insert_record_rows already returns false
for an independently present target, without overwriting its ordinal; when it
creates a target it assigns the supplied original source ordinal. Existing
classification_created ownership, source ordinal recording, conversion/undo and
all other algorithms remain unchanged. This fixes the defect in its owning home
rather than hiding ties with a synthetic sorter or updating a golden.

The new regression begins with a DELTA ordinary row, then two independently
authored annotations. It asserts original target ordinal while classified,
unchanged actual public annotation streams before/during/after conversion,
exact replay and ordinal restoration after undo, and unique live ordinals.
The correction to its initial mistaken public-order expectation follows actual
term-primed native indexes; it leaves shipping sorting and the prior golden
unchanged. Failing-first101 and the1231-PASS/one-wrong-fixture failure remain
attributable in remediation-plan.md and the raw receipts. Root reports settled
core1232 PASS. Current gap-analysis.md independently adjudicates actual
shared_views59, native Update10, real WASM Update10 and final regression1,
strict-final0/fmt-final0 after a test-only clone lint correction. All initial
failures remain retained; current source was published with normal hooks.

Earlier T1/T2/T3 qualification remains scoped applicable outside this change:
typed ingress/roles, LOAD remapping/counter/governor, additive API and untouched
full-gate contracts are not rewritten. Affected mutable ordering/shared-view
publication and actual native/WASM transfer paths require the admitted focused
requalification; current-head hosted CI remains mandatory. No new full local
campaign is demanded just because Stage3 starts.

## Final narrow closure state

1. CLOSED: affected execution and independent gap-analysis PASS.
2. CLOSED: normal commit41345/push54316 actual0, published be3fdeb84.
3. CLOSED: functional reply4226560940, explicit bot acceptance4226562864,
   actual thread resolution and advisory6074157956; issue progress6074166639.
4. CLOSED: actual37883191140 completed SUCCESS,40 SUCCESS/4 optional SKIPPED,
   no failures or pending. Final complete raw/pr507-reviews-final.json has the
   same three formal submissions; raw/pr507-inline-final.json has the same
   original finding/reply/explicit acknowledgment; raw/pr507-threads-final.json
   has one resolved/outdated thread and false thread/nested pagination.
   All substantive bodies independently read; no new required feedback.
5. tasks/T4-integration-assessment.md establishes current clean candidate
   f3e37b71b407eefa386fe7d6e6ce5ccfc10f8c8a against unchanged admitted main.
   Evidence reuse and changed behavior independently PASS; root's final fetch
   and clean merge-tree refresh retain these exact inputs/candidate. This debt
   auditor accepts current qualification/feedback applicability. Supported
   merge-ready gate/final notes, ghprsq and actual archive/landing verification
   remain root-owned; this report does not claim an already executed merge.

Packaging note CLOSED for supplied current capture: raw/review-evidence-projection.md
documents replacement only of the opaque credential-shaped CodeRabbit URL scope
value in selected pr-comments.md; actual review prose/link path/other parameters
and dispositions remain intact. Authoritative original remains the live PR;
no unnecessary private copy store was introduced. Later captures require the
same projection when applicable. This is privacy repair, not verdict rewriting.
Auditor performed no capture/forge/source mutation. Final narrow closure is
complete and this selected Stage report writer is now FROZEN for root's archive
and integration. All historical failures and limited prior-head evidence remain
attributable; no extra builds or manual campaigns were run for this closure.
