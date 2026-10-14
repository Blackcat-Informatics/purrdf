# PR #543 — comments and review threads

7 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/543?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review paused by coderabbit.ai -->

> [!NOTE]
> ## Reviews paused
> 
> It looks like this branch is under active development. To avoid overwhelming you with review comments due to an influx of new commits, CodeRabbit has automatically paused this review. You can configure this behavior by changing the `reviews.auto_review.auto_pause_after_reviewed_commits` setting.
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
- **Run ID**: `97a15ca1-eaf4-4174-aeac-6ab5959aaa85`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between ba624d2110b6c4b4f0122bdcf4059756d191bb13 and 20ede055fdbe0ec55efd19c32e789486f5d93e6a.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing (1)</strong></summary>
<dl>
<dd>

* `docs/CONFORMANCE.md`

</dd>
</dl>
</details>

<details>
<summary><strong>🚧 Files skipped from review as they are similar to previous changes (1)</strong></summary>
<dl>
<dd>

* docs/CONFORMANCE.md

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 6 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 8 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The pull request adds caller-selected retention limits to mutable dataset freezing and expands freeze tests. It also updates floating-point environment tests, query-scope counter tests, a Python retrieval test pattern, a geometry copy call, and documentation.

**Changes: Mutable dataset freezing**

|Layer / File(s)|Summary|
|:---|:---|
|**Add caller-limited freezing** <br> `crates/rdf-core/src/ir/mutable.rs`|`freeze` now calls `freeze_with_limits` with default limits. The new method accepts caller-selected limits and returns an error if snapshotting or RDF validation fails.|
|**Verify limits and frozen output** <br> `crates/rdf-core/tests/shared_views.rs`, `crates/purrdf/tests/base_surface.rs`|Tests cover limits below and at snapshot counts, snapshot stability after later mutations, and freezing with a 1 GiB auxiliary-byte limit. An ignored large-scale test checks that freezing with a larger limit preserves table counts and sampled records.|

**Floating-point environment tests**

|Layer / File(s)|Summary|
|:---|:---|
|**Update floating-point probe expectations** <br> `crates/rdf-core/src/distance/binary64_tests.rs`, `crates/rdf-core/src/distance/tests.rs`|The tests distinguish direct x87 probing from public admission paths, check control-word restoration, and expect MXCSR register evidence on supported x86 configurations.|
|**Share HNSW refusal classifiers** <br> `crates/hnsw/tests/support/fixture.rs`, `crates/hnsw/tests/float_environment.rs`, `crates/hnsw/tests/float_environment_rayon.rs`|The fixture adds classifiers for flush-to-zero refusals with MXCSR evidence. Both HNSW test files use the shared classifier.|

**Python retrieval test matching**

|Layer / File(s)|Summary|
|:---|:---|
|**Match the complete producer call** <br> `bindings/python/tests/test_retrieval.py`|The test pattern now captures producer arguments through the closing query structure and trailing numeric `LIMIT`.|

**MultiPoint coordinate copying**

|Layer / File(s)|Summary|
|:---|:---|
|**Pass the copy closure by value** <br> `crates/geo-kernel/src/measure.rs`|The MultiPoint member copy passes its mutable closure by value to `Option::map`.|

**Query-scope counter tests**

|Layer / File(s)|Summary|
|:---|:---|
|**Relocate counter tests** <br> `crates/rdf-core/src/query_scope/counters.rs`|The tests move to the end of the file. They continue to cover independent counts, callback failure, and temporary-capacity behavior.|

**Documentation and conformance updates**

|Layer / File(s)|Summary|
|:---|:---|
|**Update API documentation** <br> `crates/sparql-eval/src/engine.rs`, `crates/sparql-eval/src/geo/relation.rs`, `crates/sparql-eval/src/property_fn_eval.rs`, `crates/retrieval/src/search.rs`|The documentation describes relation identity validation during preparation and registry admission, and uses fully qualified links for referenced APIs.|
|**Update SIMD audit entries** <br> `docs/design/purrdf-simd.md`|The audit table adds a canonical sort site and updates site identifiers and measured counts.|
|**Update conformance test count** <br> `docs/CONFORMANCE.md`|The matrix reports 4,223 passing Python binding tests, up from 4,218. Other reported status values remain unchanged.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal











**Estimated code review effort:** 3 (Moderate) | ~25 minutes

<!-- change_assessment_commit:"20ede055fdbe0ec55efd19c32e789486f5d93e6a" -->
**Change:** Feature · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>20ede</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `20ede`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"20ede055fdbe0ec55efd19c32e789486f5d93e6a","coveredCommitId":"20ede055fdbe0ec55efd19c32e789486f5d93e6a","kind":"reviewed"} -->

No actionable merge-blocking issue is identified in the reviewed change; merge after normal checks.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 4 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks (1 warning)</strong>

|         Check name         |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                         |
| :------------------------ | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Out of Scope Changes check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | The mutable freeze implementation and its `rdf-core` tests support issue `#542`. The PR also changes unrelated retrieval binding parsing, geo measurement code, distance and query-scope tests, SPARQL and… | Remove the unrelated changes from this PR or move them to separate PRs. Keep the `MutableDataset::freeze_with_limits` implementation and directly related tests and documentation. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks (4 passed)</strong></summary>
<dl>
<dd>

|      Check name     |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :----------------- | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Linked Issues check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#542` requires caller-selected `ViewLimits` during mutable compaction. The PR adds `MutableDataset::freeze_with_limits`, keeps `freeze()` on default limits, and uses the supplied limits for the i… |
|  Docstring Coverage | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Docstring coverage is 88.89% which is sufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 27 functions across 15 files. (1 skipped: 1… |
|  Description Check  | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the primary change: adding caller-selected limits for mutable dataset compaction.                                                                               |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The mutable freeze implementation and its `rdf-core` tests support issue `#542`. The PR also changes unrelated retrieval binding parsing, geo measurement code, distance and query-scope tests, SPARQL and retrieval documentation, HNSW fixtures, SIMD audit counts, and the conformance scoreboard. These changes do not implement or test caller-selected limits for mutable compaction.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<!-- pre_merge_checks_walkthrough_end -->
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

# Caller-selected limits for mutable compaction

The mutable compaction API currently calls the default-limit snapshot API. The caller can publish a snapshot with larger retention limits but cannot use those same limits to compact it. Deliver the additive `MutableDataset::freeze_with_limits(&self, ViewLimits) -> Result<Arc<RdfDataset>, RdfDiagnostic>` API. Existing `freeze()` retains default admission by delegating to it. Move the current compaction body, rather than copying it, and select the snapshot through `snapshot_view_with_limits(limits)`.

Repository constraints: protected main, isolated worktree, existing importer/classifier/location homes, no duplicate functions or new dependencies/features. Installed standard tools and Cargo configured directories only. Normal hooks are mandatory. Tooling freeze is rescinded; no compiler/cache customization is in this task. No governing ADR exists in this checkout. Larger view limits do not bypass builder validity, storage capacity or allocation errors.

## Task 1: Implement and prove caller-controlled compaction

Move the sole compaction implementation to the new method; preserve term remapping, all three RDF row tables, graph existence policy, location attachment, sidecar ownership, typed failures and successful work counters. Document retention errors and provide an executable Rust example with caller limits. Add boundary coverage through the existing complete RDF source and surface comparison helpers: exact inclusive admitted limits, one short in each of the five limit dimensions, retry at sufficient limits, default-wrapper equivalence, and no successful freeze/materialization counter increase on refusal. Exercise added and retracted statement records, blank/triple/directional literal identities, declared empty graphs and source locations using existing regression targets. Retain a snapshot while changing the owner and ensure the earlier snapshot remains unchanged.

Focused qualification: core shared_views, mutable unit tests and graph/location regression targets; warnings-denied affected-package Clippy/rustdoc; umbrella reexport caller compilation and WASM core/umbrella compilation. Perform one bounded native synthetic approximately950k-claim reproduction through the public MutableDataset snapshot and new compaction API, recording actual auxiliary bytes and outcomes for default versus caller limits. Reuse existing fixture constructors; no stored large corpus, duplicated helper or external input. The same owner must succeed at sufficient caller limits and preserve the complete table populations plus boundary/interior identities. The small complete-source fixture compares every RDF value; do not materialize two additional whole-corpus comparison copies in scale acceptance. A smaller boundary fixture establishes routine correctness without routinely allocating512MiB.

One independent reviewer covers issue/prior-art/plan and then actual implementation, production consumers, evidence and full completion. Recheck substantive findings, not unchanged text. Run the mandatory full gate once after the source settles; report shared pre-existing evaluator/cross-architecture failures precisely and apply separately qualified repairs before claiming readiness. Compile first and address affected callers before broader checks.

## Task 2: Publish and integrate the complete repair

Commit the coherent API/test/docs change with normal hooks after review and focused qualification. Push, publish the plan and honest task outcomes through Stage, create an issue-closing PR after required qualification, and refresh hosted checks/review debt. Integrate only through ghprsq, verify the issue closure and mirror result, and remove the completed delivery checkout/branch after preserving selected Stage evidence. The shared evaluator repair may be a merge prerequisite; it must not pull the unfinished numeric/capacity migration into this bounded delivery. No deficiency entry or new follow-up issue substitutes for unfinished acceptance.

Enhancements declined: inherited persistent construction policy would add owner state and change default behavior; removing defaults would change callers without necessity; a second compaction implementation would create semantic drift. The explicit method matches existing snapshot APIs and is independently callable through the existing facade reexports.

## 3. paudley — 

# Shared hosted corrections

Published2916fbce4 with normal hooks and push terminal0. The first actual PR543 CI run reported strict Clippy failures, four rustdoc link/placement failures, i686 MXCSR fixture mismatch, nested-term Python fixture mismatches and22 manual assembly cells out of date. Exact reviewed corrections retain original production homes, test inputs/assertions and thresholds. The existing assembly writer regenerated only those22 cells from actual current hosted diagnostics; original incomplete reports remain unqualified. No tooling or configuration changed.

Affected standard Cargo core/geo all-target warning-denied Clippy PASS; counter regressions3/3 and native MXCSR1/1 PASS; evaluator warnings-denied rustdoc PASS. Current two Python fixture tests PASS against the existing installed471 package, proving corrected fixture behavior; this is not a current542 package qualification. Independent bounded review PASS at reviews/shared-hosted-corrections.md. Original compaction API/full local gate/scale/facade evidence remains applicable as scoped in validation.md.

PR creation returned a Stage JSON parsing error after the actual creation. GitHub readback confirms PR543; no creation retry was made. The normal corrected-head push now triggers hosted qualification. Updated hosted checks, review debt, integration assessment, ghprsq merge and issue closure remain pending. No local work is counted as an issue closure.

## 4. paudley — 

# Final shared fixture and documentation corrections

Published8360fff43 through normal hooks and push. Six retrieval rustdoc links resolve through the existing crate-root fuse reexport. Both HNSW floating-environment targets now use the original shared fixture home and assert MXCSR register refusal across their actual x86/x86_64 SSE2 configurations. Original consumer calls, refusal/restoration controls and worker result comparisons remain.

Standard make doc terminal0; native HNSW sequential5/5 and Rayon1/1 PASS. Independent source diagnosis retains the exact same corrected fixture's prior i6865+1 execution, with matching production homes. No production guard, tool, profile or acceptance threshold changed. Current hosted run38372155868 is running; prior failed runs are not qualification. Issue542 integration and closure remain pending.

## 5. paudley — 

# Current review debt

Current branch brief covers four commits and16 paths at8360fff43. Review-brief artifact gate PASS. GitHub paginated REST review and inline-comment arrays are empty. GraphQL reviewThreads has no nodes and hasNextPage=false. Actual issue comments are retained in current-all-comments.jsonl; material independent review reports remain in reviews/. Current hosted checks are running, so this file is not hosted approval or merge acceptance.

The bot comment is processing8360fff43 and reports no actionable inline comments for the preceding2916fbce4 review. It separately flags shared corrections as out of scope. Disposition: retain the bounded prerequisite corrections demonstrated by this PR's actual mandatory hosted failures. The geo Copy closure and counter-module move fix strict Clippy; core/HNSW fixtures correct architecture-specific refusal expectations without changing guards; retrieval Python matcher preserves nested quoted-term captures; rustdoc fixes unblock mandatory docs; manual SIMD cells follow actual measured diagnostics through the existing writer and never turn incomplete reports into PASS. Independent shared correction reviews cover these exact deltas. No unrelated feature or tooling rewrite is included. This disposition does not claim the bot's current review is complete.

Prior published2916fbce4 optimized wasm-package job115199003783 failed446/449: sync135, LATERAL236, NOT EXISTS134 answered levels fall below unchanged assertions181/281/180. The failure messages cite historical182/282/181 labels; those labels do not change the assertions. This establishes the shared531 problem on actual542 source. No waiver or current-head PASS is inferred. Full raw receipt: ci-wasm-package-2916.log. The current four-file documentation/test-fixture correction does not repair that evaluator computation; numeric owner is qualifying the concrete shared continuation correction separately.

## 6. paudley — 

# Handwritten conformance count correction

Published20ede055f corrects the separate handwritten Python binding scoreboard
from4218 to the actual4223. Both the generated matrix and prose now agree with
all four retained hosted shards; Python reports4223 passes/four strict xfails,
not merely the compatibility differential. No tooling or test expectation changes.

Existing `python3 scripts/check-doc-claims.py` terminal0:154 claims agree.
Independent scoped source review PASS in reviews/restated-conformance-count-review.md.
Normal commit7536 and push68515 terminal0, retained logs adjacent.

Predecessor CI38423608191 is terminal FAILURE:37 jobs pass, including conformance,
all measured assembly configurations and architecture execution. Workspace failed
only on the now-corrected handwritten count. Optimized package446/449 still fails
the shared synchronous/LATERAL/NOT EXISTS floors135/236/134 versus181/281/180.
That production repair remains incomplete. Corrected-head CI and final integration
are pending; no issue closure or whole-delivery pass is claimed.

## 7. paudley — 

Current source `20ede055f`: [CI attempt 2](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38428369796/attempts/2) finished with 41 passing jobs, 4 skipped and one failure.

The cancelled native integration 5 shard passed its standard retry, and the test aggregate now passes. The workflow, compiler, cache, profile and 30-minute timeout are unchanged. The original cancellation receipt remains retained.

The sole failed job remains the optimized WASM package: 446/449 tests pass, with synchronous/LATERAL/NOT EXISTS depths 135/236/134 below the unchanged 181/281/180 assertions. The actual evaluator repair remains in progress under #531. Workspace, conformance, all seven assembly configurations and architecture controls pass on this head. No complete hosted pass, merge or closure is claimed.

