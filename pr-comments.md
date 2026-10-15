# PR #552 — comments and review threads

15 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/552?cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review paused by coderabbit.ai -->

> [!NOTE]
> ## Reviews paused
> 
> It looks like this branch is under active development. To avoid overwhelming you with review comments due to an influx of new commits, CodeRabbit has automatically paused this review. You can configure this behavior by changing the `reviews.auto_review.auto_pause_after_reviewed_commits` setting.
> 
> - Last reviewed commit: 39cae7092f4c1c8de032dc36cc25177a7bdc5cee
> 
> Use the following commands to manage reviews:
> - `@coderabbitai resume` to resume automatic reviews.
> - `@coderabbitai review` to trigger a single review.
> 
> Use the checkboxes below for quick actions:
> - [ ] <!-- {"checkboxId":"7f6cc2e2-2e4e-497a-8c31-c9e4573e93d1"} --> ▶️ Resume reviews
> - [ ] <!-- {"checkboxId":"e9bb8d72-00e8-4f67-9cb2-caf3b22574fe"} --> 🔍 Trigger review

<!-- end of auto-generated comment: review paused by coderabbit.ai -->
<!-- recent_review_start -->

No actionable comments were generated in the recent review. 🎉

<details>
<summary><strong>ℹ️ Recent review info</strong></summary>
<dl>
<dd>

<details>
<summary><strong>⚙️ Run configuration</strong></summary>
<dl>
<dd>

- **Configuration used**: defaults
- **Review profile**: CHILL
- **Plan**: Team
- **Run ID**: `5f2a594b-3133-4c66-8f77-81eb7f7aebbb`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 2def24ab9e7bb94283551fcac2b1d16c4c2a3538 and 39cae7092f4c1c8de032dc36cc25177a7bdc5cee.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 2</strong></summary>
<dl>
<dd>

* `crates/rdf-core/src/ir/segmented/session.rs`
* `helpers-ledger.toml`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 2 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 5 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The changes add a non-WASM file-backed provider, grow segmented query bookkeeping within memory limits, revise segmented row retrieval, and stop evaluator traversal when callbacks request it. Tests cover allocation behavior, memory accounting, and segmented query results.

**Changes: Segmented query execution**

|Layer / File(s)|Summary|
|:---|:---|
|**File-backed provider and exports** <br> `crates/rdf-core/src/ir/segmented/file.rs`, `crates/rdf-core/src/ir/segmented/mod.rs`, `crates/rdf-core/src/ir/mod.rs`, `crates/rdf-core/src/lib.rs`, `crates/rdf-core/examples/segmented_fixture.rs`|A non-WASM `SegmentedFileProvider` attaches a file to a supplied snapshot, captures the file length, bounds-checks reads, and reports provider errors. The crate exports it, and the fixture uses it instead of its local provider. `SegmentedBytes` uses a non-exhaustive debug formatter.|
|**Budgeted query storage and accounting** <br> `crates/rdf-core/src/ir/segmented/session.rs`, `crates/rdf-core/src/ir/segmented/query.rs`, `crates/lex/src/allocation.rs`, `crates/rdf-core/tests/segmented_stack.rs`, `crates/rdf-core/tests/segmented.rs`, `crates/sparql-eval/tests/segmented_query.rs`, `crates/sparql-eval/tests/support/segmented.rs`, `crates/hnsw/tests/sparql_e2e.rs`, `crates/text/tests/*`, `docs/design/purrdf-segmented-query-storage.md`, `helpers-ledger.toml`|Reader buffers grow through budget reservations, and retained capacity is reported as bookkeeping bytes. Admission accounts for pending blocks and can evict unpinned blocks. Representative interning reserves for a push, and allocation failure preserves the original vector. Tests update memory accounting and cover buffer growth, allocation behavior, and limits. The design document describes storage admission and layout measurements; the ledger updates recorded source hashes.|
|**Segmented row retrieval** <br> `crates/rdf-core/src/ir/segmented/query/view.rs`, `crates/rdf-core/src/ir/segmented/session.rs`, `crates/sparql-eval/tests/segmented_query.rs`, `docs/design/purrdf-segmented-query-storage.md`|Segmented queries translate patterns and retrieve rows through callbacks. Resident reifier and annotation queries use bound-subject lookups when available and filter candidates against the pattern. Integration tests check query results across claim counts and storage layouts.|
|**Early termination during row traversal** <br> `crates/sparql-eval/src/statement_layer.rs`, `crates/sparql-eval/src/bgp.rs`, `crates/sparql-eval/src/bgp/window.rs`, `crates/sparql-eval/src/path_relation.rs`|Statement visitors and evaluator callbacks return `ControlFlow`. Traversal stops when callbacks signal a break, including after sink limits, stop signals, insertion failures, or conversion errors.|

<!-- change_assessment_start -->
**Priority:** ⬆️ High



**Estimated code review effort:** 4 (Complex) | ~45 minutes

<!-- change_assessment_commit:"39cae7092f4c1c8de032dc36cc25177a7bdc5cee" -->
**Change:** Bug fix · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>39cae</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `39cae`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"39cae7092f4c1c8de032dc36cc25177a7bdc5cee","coveredCommitId":"39cae7092f4c1c8de032dc36cc25177a7bdc5cee","kind":"reviewed"} -->

The previously identified tight-budget read refusal is addressed. No actionable merge-blocking issue remains from the reviewed evidence; normal validation can proceed.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 4 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks: 1 warning</strong>

|     Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                         |
| :---------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 45.07% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 71 functions across 20 files. (1 skipped:… | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#551` is open and directly linked. `SegmentedStackQuery::intern` uses `reserve_for_push`, and its regression test verifies bounded growth for 4,096 terms. Statement-layer visitors use `ControlFlo… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changes remain connected to issue `#551`. `SegmentedFileProvider` supports the persisted-image regression. Allocation ownership, first-fault preservation, residency accounting, debug bounds, helper … |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely summarizes the main changes: limiting segmented query growth and stopping completed LIMIT scans.                                                                          |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 45.07% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 71 functions across 20 files. (1 skipped: 1 unsupported.)

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<!-- pre_merge_checks_walkthrough_end -->

- [ ] <!-- {"checkboxId":"585bb3f6-faf5-4dbf-96d2-74e382adf19a"} --> Fix all pre-merge checks with AI
<!-- finishing_touch_checkbox_start -->

<details>
<summary><strong>✨ Finishing Touches: 💡 1</strong></summary>
<dl>
<dd>

<!-- finishing_touch_suggestion:docstrings -->
<details open>
<summary><strong>📝 Generate docstrings: 💡</strong></summary>
<dl>
<dd>

- [ ] <!-- {"checkboxId":"3e1879ae-f29b-4d0d-8e06-d12b7ba33d98"} --> Commit to this branch
- [ ] <!-- {"checkboxId":"7962f53c-55bc-4827-bfbf-6a18da830691"} --> Create a new PR

</dd>
</dl>
</details>
<details open>
<summary><strong>🧪 Generate unit tests (beta)</strong></summary>
<dl>
<dd>

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Create a new PR

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->






<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

@coderabbitai review

Please review the complete current change, including allocation ownership,
first-error behavior and LIMIT cancellation through the real statement-layer
consumers. The final local gate and hosted checks are running. The PR remains
draft until qualification completes; no approval or complete gate is inferred.

## 3. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:cbbc8efeaaa262f5b0e76b0447e9233453dbf69dca18bb719a9a836abefdc02b -->
<details>
<summary>✅ Action performed</summary>

Review finished.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 4. paudley — 

The blanket docstring-coverage warning is declined after checking the changed
contracts. The new public `SegmentedFileProvider` and `from_file` constructor
have rustdoc, including the trust boundary and constructor errors. Provider
overrides implement the documented `SegmentedProvider` trait. The shared visitor
explicitly documents that Break stops both streams before another source row,
and the new storage guide documents admission ownership and measured block sizes.

The reported percentage also includes private helpers, trait implementations and
test functions. No missing public contract was identified, and the repository
does not impose this blanket percentage. No product requirement or executable
acceptance criterion is waived. The source review's no-actionable-comment result
and the complete warning are retained with this disposition.

## 5. paudley — 

@coderabbitai review

Please review the ownership correction in commits `b12019ebe` and `5425211ec`.
Hosted native and WASM owner-release controls exposed retained demand-grown
request/cache-slot arrays in balances that previously assumed upfront capacity.
Evidence now captures their original reservation separately; strict owner-release
equalities, pin retention, live/peak ceilings and allocator comparisons remain.
The existing internal control independently checks both concrete Vec capacities.

Tight-session fixtures also now use the actual opening peak, including old/new
buffer overlap. Their typed query refusal, refusal before another read, exact
release and healthy-neighbor assertions remain. Check that these adaptations
cannot mask query-owned memory or silently relax the operational contract.

Focused execution passes: 22 SPARQL cases, 13 native and 13 portable segmented
controls, seven HNSW cases, both portable text-owner cases, native text focus and
cache ownership, and the internal escaped-request/capacity control. The required
final local gate and new-head hosted checks remain pending. The full 200,000-claim
file-backed LIMIT/COUNT contract remains required; no work is moved to follow-up.

## 6. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:8a7f43e556a63439ed1163909acb744d22eb27ce00508636bf29a3f5ddf33963 -->
<details>
<summary>⚠️ Action not completed</summary>

Pull request base or head changed.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 7. paudley — 

@coderabbitai review

The previous correction review stopped because the head changed. The settled
head is now `7c39fae37`; no source edits are queued. Please review the ownership
and opening-peak correction described above, including the final early test
lock drop and native-index constructor-closure records. Focused native/portable
consumer checks, warning-denied all-target compilation and the actual helper
gate pass. Final local and hosted qualification remain pending.

## 8. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:5385b8158040758eea51268b6745b20d2111a4002af72413ac520123d42277a5 -->
<details>
<summary>⚠️ Action not completed</summary>

Already reviewed the last commit. Use `@coderabbitai full review` to rerun a review of the entire changeset.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 9. paudley — 

@coderabbitai review

Commit `f7862dccc` fixes both findings from review
`839234b1-45af-4654-a363-7a2eaf5c45d7` in the existing homes:

- `Memory::reserve` destroys and refunds the rejected replacement on allocation
  refusal and capacity-certificate failure, preserving the original error/data.
- `SegmentedSession::block` rechecks the first fault under its mutex before cache
  reuse or preflight. It preserves both the returned error and sticky status.

Both new regressions failed before the fixes. The corrected source passes all
25 allocation cases, five session-owner cases, 27 segmented-provider/stack cases,
22 neighboring SPARQL cases and the full file-backed 200,000-claim control on all
three fitting layouts. Exact image/peak quantities still match the guide. Fresh
warning-denied workspace Clippy and the actual helper gate pass. The ledger only
refreshes 40 constructor identities and 37 allocation-source authorities, with
all sinks, classes, other authorities and reasons unchanged.

Please verify the two findings are resolved and inspect this bounded delta.
Remaining local execution, current-head hosted checks and final integration are
still pending. Source is settled; no further edits are queued.

## 10. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:b405fe36f9507ec66a5a61e5d9d5e194dd25296789e08b940b55b94387dad14e -->
<details>
<summary>⚠️ Action not completed</summary>

Already reviewed the last commit. Use `@coderabbitai full review` to rerun a review of the entire changeset.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 11. paudley — 

@coderabbitai review

Commit `2def24ab9` addresses the reader-buffer finding from review
`4bff5b17-43a4-4ff3-87b4-db9c3ecf3d86`.

The existing eviction body now has one `State::evict_one` home. Both ordinary
room checks and actual cache/request buffer admission use it. Buffer admission
retries after reclaiming an unpinned block, under the already-held State lock;
there is no recursive reservation backlink. Block room is checked again after
cache growth, using its actual combined live layouts. Other errors and pinned
owners retain their original behavior.

The new tight-budget case fails before the fix and now passes for request-ledger
and cache-array growth. All six session-owner cases, 27 provider/stack cases,
22 neighboring public query cases, affected warning-denied all-target Clippy and
the actual 82-job/118-row helper gate pass. Only 40 constructor identities are
refreshed; source closure paths, all authorities, sinks, classes and reasons are
unchanged. Please verify the finding is resolved and inspect this bounded delta.

No further source edits are queued. Remaining broad local execution resumes
after this material recheck; current-head hosted checks are pending.

## 12. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:97446f856889dd8f8a16aa16e287ab2dd4f21f69d1689d944131688df4e42250 -->
<details>
<summary>⚠️ Action not completed</summary>

Already reviewed the last commit. Use `@coderabbitai full review` to rerun a review of the entire changeset.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 13. paudley — 

@coderabbitai review

Commit `39cae7092` completes cache admission at the pinned-block boundary. The preceding review cleared `2def24ab9`; an additional deterministic control then demonstrated that allocating a larger cache array before accounting for the pending block could still refuse a valid read.

Cache growth now uses the existing shared growth-capacity calculation to compare its net retained increase plus the pending block with the budget before allocating. It reclaims through the existing eviction home when necessary. Actual replacement-overlap accounting remains in Memory; the planner does not sum future block ownership with an earlier replacement peak.

The new control covers both 31 pinned blocks with one reclaimable header (retain 32 cache slots and succeed) and all 32 pinned with sufficient sequential headroom (grow to 64 slots and succeed). Seven internal session controls, 27 provider/stack controls, 22 public query controls, affected all-target warning-denied Clippy, and the actual 82-job/118-row helper gate pass. The only ledger updates are the 40 affected constructor identities; authorities, sinks, classes and reasons are unchanged.

Please review this final bounded delta, including both budget boundaries. Source will remain stable during review. Remaining complete local execution follows this recheck; current hosted checks are pending.

## 14. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:bf9a216532ac69edc7c21d587c46b4327b4596760188ab438358e4c062a23588 -->
<details>
<summary>⚠️ Action not completed</summary>

Already reviewed the last commit. Use `@coderabbitai full review` to rerun a review of the entire changeset.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 15. paudley — 

Bound segmented query growth and stop completed LIMIT scans

Closes #551

Incremental term representatives use the existing checked amortized reserve
home. Known-size exact reservations retain their contract. The shared statement
visitor propagates cancellation through BGP, window and path callers, and bound
source probes reuse exact local addresses and existing subject order.

Request and cache arrays now grow with actual use through their original shared
admission owner. Evidence reports their retained bookkeeping separately from
query/pin ownership. Strict release and live/peak checks remain intact. Failed
replacement allocations refund unretained capacity, and the block mutex preserves
the first fault before cache reuse or preflight. Buffer admission reclaims
unpinned cache through one eviction home. Cache planning prices net retained
growth plus the pending block, preserving actual sequential replacement overlap.

The native file provider consolidates the former example implementation.
File-backed controls cover 2000 and 200000 reifier/annotation claims at three
fitting layouts under 256 MiB. LIMIT emits ten rows after touching 21
representatives; independent COUNT returns 400000 and matches resident execution.
The sizing guide accounts for dictionary/reverse, rows, summaries and
authentication. The authored corpus is distinguished from the unprovided
original Katamari harness; the overfull sizing neighbor explicitly refuses.

Fresh warning-denied workspace Clippy, library/test compilation, complete
workspace tests, the preserve-order consumer, kernel hygiene and make wasm pass.
Completed unaffected pre-test checks, generated comparisons and all 154 manual
claims are reused under the recorded applicability assessment. No interrupted
make check is called successful. Actual helper hygiene passes 82 enforced jobs
and 118 distinct rows; proof identities refresh without weakening invariants.
Normal hooks pass. All current hosted checks pass, including portable execution,
architecture, conformance and bindings, on run 38662857745.

Material source review clears the final delta. Allocator-refund, first-fault and
cache-reclaim findings have production corrections and red/green controls.
The private/test docstring-percentage warning has a posted reasoned decline;
synthetic aggregate-size logging has a native CodeQL false-positive disposition.
There are no unresolved review threads or deferred issue requirements.
This completion assessment is owner judgment under the zero-subagent workflow;
actual external reviews are separately preserved.

The fetched main is an ancestor with no additional commits. The predicted merge
tree is clean and has the qualified delivery content; no conflict resolution or
base synchronization is needed. Repository constraints remain intact: existing
allocation/eviction homes, target-gated native IO, deterministic formats, no new
dependency or semantic feature, and standard tools and directories.

Authoritative plan: .stage/segmented-stack-queries-are-quadratic/plan.md.
Selected evidence: .stage/segmented-stack-queries-are-quadratic.
Current qualification and full scope assessment are validation.md and
tasks/delivery-completion.md in that directory. Original failed receipts remain
selected, including the losslessly compressed oversized Debug output.

Defect-Class: bug

