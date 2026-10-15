# PR #559 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/559?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

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
- **Run ID**: `ace9d4e9-c307-46f9-9372-a1e45be1e622`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 0d075201ffe69a473247287b814ca5846cd3c94f and cef34c8aa45716c1f04b0659b288a85a379e21cd.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 8</strong></summary>
<dl>
<dd>

* `crates/rdf-core/src/ir/mutable.rs`
* `crates/rdf-core/src/ir/paged/stack.rs`
* `crates/rdf-core/src/ir/segmented/query/view.rs`
* `crates/rdf-core/tests/paged_stack.rs`
* `crates/rdf-core/tests/segmented_stack.rs`
* `crates/sparql-eval/tests/paged_stack_query.rs`
* `docs/design/purrdf-backend-contract.md`
* `helpers-ledger.toml`

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

Ordinary rows now retain their authored typing when reifier declarations appear in other layers or sources. Mutable input can still infer annotation typing at write time. Tests and the backend contract document the behavior across paged and segmented stacks.

**Changes: Ordinary and annotation row classification**

|Layer / File(s)|Summary|
|:---|:---|
|**Classification rules** <br> `crates/rdf-core/src/ir/mutable.rs`|`classify_statement` returns a statement kind directly. It infers annotation typing for ordinary rows only when inference is enabled and an effective reifier exists.|
|**Paged and segmented query paths** <br> `crates/rdf-core/src/ir/paged/stack.rs`, `crates/rdf-core/src/ir/segmented/query/view.rs`|Paged reads no longer use external reifier declarations to classify ordinary rows. Segmented declaration checks now cover every layer through the requested prefix.|
|**Regression coverage and contract** <br> `crates/rdf-core/tests/*stack.rs`, `crates/sparql-eval/tests/paged_stack_query.rs`, `docs/design/purrdf-backend-contract.md`, `helpers-ledger.toml`|Tests cover row retrieval and stream counts across layer layouts, including declaration-provider faults. The G11 contract text and proof source hashes are updated.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal

**Estimated code review effort:** 3 (Moderate) | ~25 minutes

<!-- change_assessment_commit:"cef34c8aa45716c1f04b0659b288a85a379e21cd" -->
**Change:** Bug fix · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>cef34</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `cef34`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"cef34c8aa45716c1f04b0659b288a85a379e21cd","coveredCommitId":"cef34c8aa45716c1f04b0659b288a85a379e21cd","kind":"reviewed"} -->

The change keeps authored ordinary rows as ordinary rows across sealed layers, matching the linked issue. No concrete merge-blocking risk was found, and regression tests cover the layer permutations.
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
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 31.25% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 16 functions across 6 files. (2 skipped: … | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the primary change: preserving authored ordinary rows across sealed stack layers.                                                                               |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Direct issue `#556` requires ordinary quads to remain ordinary when a reifier for the same subject exists in another layer. The PR changes the shared classifier and both segmented and paged stack read p… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changed source paths implement the cross-layer classification fix. The stack tests, SPARQL restoration test, and design-contract update verify or document that behavior. The helper-ledger source-h… |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 31.25% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 16 functions across 6 files. (2 skipped: 2 unsupported.)

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

# Preserve authored ordinary rows across sealed layers

The complete contract is layer invariance for explicitly ordinary persisted
rows, including rows whose subject is a reifier in another layer. Default and
named graphs, keyed ordinary reads, reifier rows and annotations must agree with
one combined image. Keep removals, deduplication, source failures, snapshot
ownership and explicit annotation behavior intact. No subagents, timing gates,
new dependencies, duplicate implementations or toolchain changes.

Read the full issue and discussion (none), current source, root .baseline/.goals
and repository AGENTS.md. The existing shared classifier promotes ordinary
rows based on external reifier presence. Both segmented and paged stack adapters
use it; the mutable value writer also uses it for its distinct inferred-input
operation. Native combined-image ordinary typing is the authoritative control.

## Task 1: Reproduce through public persisted reads

Reuse the existing typed fixture builders. Check all three-layer permutations,
the two ordinary layers without declarations, and one combined image in default
and named graphs. Compare ordinary count, keyed subject/predicate/object values,
reifier count and annotation count with the authored rows. Establish an actual
pre-fix failure, not just code-reading evidence.

## Task 2: Correct the existing classification home and its callers

Separate explicit sealed typing from mutable input inference at the existing
classifier; do not add a competing classification algorithm. Both stack adapters
must preserve authored ordinary rows and stop probing irrelevant declarations
for them. Remove obsolete promotion-only paths. Retain annotation demotion,
chronological occurrence/removal semantics and original readiness/fault owners.
Update the obsolete promotion expectation and technical contract accordingly.

## Task 3: Qualify and deliver the complete correction

Run only directly affected stack/mutable and public query controls during work,
plus the new real persisted regression, strict affected compilation and helper
checks. Execute applicable portable behavior through the existing runner. Check
original lazy/bounded/first-fault controls, actual removals/reinsertion and
retained snapshots. Use existing qualifying evidence for unchanged paths.
Publish after normal hooks, resolve material review and hosted final checks,
merge with ghprsq, verify closure/mirror and remove this delivery checkout.
Owner review remains explicitly distinct from independent hosted review.

