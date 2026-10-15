# PR #554 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/554?scope=REDACTED&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
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
- **Run ID**: `cb3863e5-acb7-4cfb-a514-cbdb833c3cbf`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 31ddef3ad24fa2bb07d67018ff5201f464443e4d and 1f80189d96f1e430ff3bb2d019f0f39a5857a45e.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 1</strong></summary>
<dl>
<dd>

* `helpers-ledger.toml`

</dd>
</dl>
</details>

<details>
<summary><strong>🚧 Files skipped from review as they are similar to previous changes: 1</strong></summary>
<dl>
<dd>

* helpers-ledger.toml

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 3 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 5 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

Segmented stack views now return reifier and annotation rows for a requested subject across graphs. The block cache now uses a hash table keyed by snapshot and slot, with recency tracking, capacity accounting, and eviction that skips pinned blocks.

**Changes: Segmented query access**

|Layer / File(s)|Summary|
|:---|:---|
|**Bound-reifier query access** <br> `crates/rdf-core/src/ir/segmented/query/view.rs`, `crates/rdf-core/tests/segmented_stack.rs`, `crates/sparql-eval/tests/segmented_query.rs`|The view implements keyed reifier and annotation queries across graphs. Tests compare results with effective streams and the resident evaluator, including absent keys and resource bounds.|

**Segmented block cache**

|Layer / File(s)|Summary|
|:---|:---|
|**Hash-table cache admission and lookup** <br> `crates/rdf-core/src/ir/segmented/session.rs`, `crates/rdf-core/src/ir/segmented/session/cache.rs`|The cache uses snapshot-and-slot keys. Hash-table capacity is admitted and reserved using allocation bounds. Cache hits update recency, and successful block loads insert keyed entries.|
|**Cache eviction and capacity validation** <br> `crates/rdf-core/src/ir/segmented/session/cache.rs`, `crates/rdf-core/src/ir/segmented/session/cache_control.rs`, `crates/rdf-core/src/ir/segmented/session.rs`, `docs/design/purrdf-segmented-query-storage.md`, `helpers-ledger.toml`|Eviction skips entries with external `Arc` owners and removes the oldest eligible entry. Tests cover recency, pinned-owner lifetime, and capacity accounting. The design documentation and capacity-proof source hashes are updated.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal





**Estimated code review effort:** 3 (Moderate) | ~25 minutes

<!-- change_assessment_commit:"1f80189d96f1e430ff3bb2d019f0f39a5857a45e" -->
**Change:** Bug fix · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>1f801</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `1f801`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"1f80189d96f1e430ff3bb2d019f0f39a5857a45e","coveredCommitId":"1f80189d96f1e430ff3bb2d019f0f39a5857a45e","kind":"reviewed"} -->

No concrete issue in the selected change prevents merging after normal checks.
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
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 38.10% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 21 functions across 6 files. (1 skipped: … | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#553` requires keyed bound-reifier and annotation access over segmented stacks. `SegmentedStackQuery` now obtains reifier rows and annotation rows through subject-keyed paths, with graph handling… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The supplied current change summary lists query implementation, cache implementation, related cache and segmented-query tests, segmented-storage documentation, and capacity-proof source registrations.… |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the two main changes: restoring keyed segmented statements and adding indexed block-cache support.                                                              |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 38.10% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 21 functions across 6 files. (1 skipped: 1 unsupported.)

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
<summary><strong>✨ Finishing Touches</strong></summary>
<dl>
<dd>

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

# Restore complete keyed segmented-stack queries

The live issue and all comments were read through stagectl. Main is the starting
point; the existing carrier and numeric branches remain separate. No subagents
are used. Standard installed tools, normal hooks and ghprsq remain mandatory.

## Task 1: Keyed logical streams

Override reifier_quads_of and annotations_of_with_graph on SegmentedStackQuery.
Route them through existing reifiers/logical_rows with a bound subject, retaining
translation, chronological visibility, graph slots, deduplication and first errors.
Extend the real persisted evaluator control to compare bound-subject results
with resident results and independent counts at 2000/200000 claims. Require
bounded provider reads and touched terms, not timing. Include historical/head,
named-graph and typed refusal controls through the existing shared fixtures.
Commit and push the coherent keyed-path repair after focused acceptance.

## Task 2: Indexed block-cache ownership

Replace linear hit lookup and remove/push movement at the existing State cache
home. Preserve source-qualified keys, exact admission of every index/slot buffer,
LRU behavior, pinned-block exclusion, sticky first errors, failure refunds and
last-owner release. Reuse the existing allocation and hash homes; add no parallel
cache implementation. Qualify full 4096-entry cache hits with fixed operation or
instruction counts, eviction/reload and cross-source identity controls. Existing
reader-buffer refusal and pin-neighbor regressions must remain valid.

## Task 3: Complete integration

Inspect the complete issue against production callers and tests; close missing
coverage, run affected strict native/portable checks and the required final gate.
Update storage documentation and any generated projections affected by source.
Publish a complete PR, resolve material forge review and hosted checks, integrate
through ghprsq, verify issue closure/mirror and remove the delivery branch.

No normal issue closure is claimed for a partial repair or a passing microtest.
The issue's wall-clock observations are reproduction context; acceptance uses
exact answers, provider reads, allocation ownership and fixed work only.

## 3. paudley — 

The docstring-coverage warning is declined as a percentage target for this diff.
The public keyed methods implement the documented DatasetView contract; the
cache module documents its source-qualified ownership, and its hit, insertion
and eviction methods document their invariants. The remaining private link
updates and regression bodies do not introduce an undocumented public contract.
The repository's warning-denied all-target Clippy check passes, including on the
combined carrier source. Adding repetitive comments to satisfy this heuristic
would not strengthen the behavior or ownership evidence.

The warning is retained in the captured source-review summary. It is not called
a passed coverage check, and the independent source review is not represented
as formal approval. The complete persisted query, cache/refusal, history and
actual WASM evidence remain separately recorded in validation.md.

