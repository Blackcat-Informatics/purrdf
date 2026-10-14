# PR #548 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/548?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

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
- **Run ID**: `ee6e90e6-5b6c-4607-b25f-4bd32187f9b9`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between cb38600b6ebd278275716c2261702c81c508a45d and 88dfc2b7c5cf1bea9891cdcae6d740c59ea39ff8.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 1</strong></summary>
<dl>
<dd>

* `crates/sparql-eval/src/eval.rs`

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

`eval_evaluated_with` now uses a guard to abort its entered `NodeBoundary` if evaluation unwinds. Tests cover callback panics and returned errors, restored context state, and subsequent evaluation using the same context.

**Changes: Evaluator unwind recovery**

|Layer / File(s)|Summary|
|:---|:---|
|**Boundary guard and recovery validation** <br> `crates/sparql-eval/src/eval.rs`|`eval_evaluated_with` retains the entered boundary in a `ResidentPending` guard and clears the guard after `finish`. Tests check panic and returned-error paths, restored node cursors, local caps, and evidence-search frames, and context reuse. The copied-join shape provider and operator definitions move to module scope.|

<!-- change_assessment_start -->
**Priority:** ⬇️ Low

**Estimated code review effort:** 2 (Simple) | ~10 minutes

<!-- change_assessment_commit:"88dfc2b7c5cf1bea9891cdcae6d740c59ea39ff8" -->
**Change:** Bug fix
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>88dfc</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `88dfc`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"88dfc2b7c5cf1bea9891cdcae6d740c59ea39ff8","coveredCommitId":"88dfc2b7c5cf1bea9891cdcae6d740c59ea39ff8","kind":"reviewed"} -->

No identified issue blocks merging after normal checks.
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

|     Check name     |                                              Status                                              | Explanation                                                                                                                                                                                | Resolution                                                                         |
| :---------------- | :----------------------------------------------------------------------------------------------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 42.86% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 7 functions across 1 files. | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the primary change: restoring evaluator scopes after an answer callback unwinds.                                                                                |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#545` requires the evaluator unwind guard, callback-panic recovery, state restoration, healthy context reuse, and preservation of normal and error completion. The change summary reports the `Resi… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The reported source change is in `crates/sparql-eval/src/eval.rs`. The reported tests cover the callback provider, evaluator scopes, and context reuse. These changes directly support issue `#545`. No un… |

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

The documentation percentage warning concerns internal evaluator methods and the relocated test fixture. This PR changes no public API. The existing guard trait documents its cleanup responsibility; the adapter invokes that original cleanup implementation. The repository's warning-denied Rust documentation and compilation gates pass. I am declining the suggested percentage-driven comments because they would repeat the method signatures without documenting additional behavior. The current source review found no actionable code issue.

## 3. paudley — 

Restore evaluator scopes after answer callback unwind

Closes #545

An answer callback could unwind after the evaluator installed local caps,
node cursors and evidence scopes. The entered boundary now stays inside the
existing resident guard through callback execution and completion. Unwind calls
the original abort implementation; normal completion disarms the guard only
after `finish` returns. The abort state is consumed before later evidence work
can fail, preserving the parent scope and original source-error precedence.

The existing copied-join fixture is shared by both tests. There is no additional
heap allocation, second guard destructor, new dependency or numeric-policy change.

Validation includes the actual callback panic/error and healthy-context reuse,
existing provider and nested-scope controls, native/scalar/SIMD physical-stack
execution, exact 40-byte ordinary rows, admission/refusal/last-owner release,
all 19 physical-shape controls, strict Clippy and the unchanged helper census.
The final local `make check` passed, including its nested release-WASM build.
The same-source complete hosted run also passed portable execution, library,
Python/conformance and optimized package acceptance. The original producer
passed 112 sites on seven assembly configurations; its document matches.

The current PR run has also completed with no failed or pending check. Library,
package, cross-architecture, portable, documentation, conformance and all seven
assembly jobs pass. Expected optional skips are retained; the complete projection
producer was separately executed successfully on this same source.

The current independent source review found no actionable code issue. Its
documentation percentage suggestion was declined publicly because this changes
no public API and the warning-denied repository documentation gates pass.
The owner assessed the complete remaining contract and actual receipts under
the zero-subagent workflow; no fresh independent completion audit is invented.

The clean predicted integration uses the tested main base without an additional
base delta. No conflicts were resolved. Historical receipt reuse is scoped in
the current acceptance index; no old assembly identity or count is relabeled.
The DISTINCT fixed-work estimate remains uncertain and establishes no win.
Cancelled timing campaigns remain excluded.

The authoritative plan is `.stage/complete-evaluator-acceptance/plan.md`.
Select `.stage/complete-evaluator-acceptance` for separate evidence archival.

Defect-Class: none

