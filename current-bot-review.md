<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/543?cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review in progress by coderabbit.ai -->

> [!NOTE]
> Currently processing new changes in this PR. This may take a few minutes, please wait...
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `cf84444a-63ea-45e8-92c0-35c6025b0e5b`
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📥 Commits</strong></summary>
> <dl>
> <dd>
> 
> Reviewing files that changed from the base of the PR and between 5170165b8695d7458b211882c7fb8c0cf45675c0 and 2916fbce469b3e5b60097130b2a305fb97ef128f.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing (8)</strong></summary>
> <dl>
> <dd>
> 
> * `bindings/python/tests/test_retrieval.py`
> * `crates/geo-kernel/src/measure.rs`
> * `crates/rdf-core/src/distance/tests.rs`
> * `crates/rdf-core/src/query_scope/counters.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/geo/relation.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `docs/design/purrdf-simd.md`
> 
> </dd>
> </dl>
> </details>
> 
> ```ascii
>  ________________________________________________
> < Squeezing intelligence out of LLMs since 2023. >
>  ------------------------------------------------
>   \
>    \   \
>         \ /\
>         ( )
>       .( o ).
> ```

<!-- end of auto-generated comment: review in progress by coderabbit.ai -->

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
- **Run ID**: `8fae4d9b-f5e5-4a27-8d72-ae249ba5b0f4`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 5144f5c1a7dc824f32165eca91d3c2a6df6226fb and 5170165b8695d7458b211882c7fb8c0cf45675c0.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing (4)</strong></summary>
<dl>
<dd>

* `crates/purrdf/tests/base_surface.rs`
* `crates/rdf-core/src/distance/binary64_tests.rs`
* `crates/rdf-core/src/ir/mutable.rs`
* `crates/rdf-core/tests/shared_views.rs`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 5 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 6 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The pull request adds `MutableDataset::freeze_with_limits` and tests freezing with caller-selected limits. It also updates x87 bypass tests to distinguish direct probing from public admission paths.

**Changes: Mutable dataset freezing**

|Layer / File(s)|Summary|
|:---|:---|
|**Add caller-limited freezing** <br> `crates/rdf-core/src/ir/mutable.rs`|`freeze` now calls `freeze_with_limits` with default limits. The new method accepts caller-selected limits and returns an error if snapshotting or RDF validation fails.|
|**Verify limits and frozen output** <br> `crates/rdf-core/tests/shared_views.rs`, `crates/purrdf/tests/base_surface.rs`|Tests cover limits below and at snapshot counts, snapshot stability after later mutations, and freezing with a 1 GiB auxiliary-byte limit. An ignored large-scale test checks that freezing with a larger limit preserves table counts and sampled records.|

**x87 bypass probe tests**

|Layer / File(s)|Summary|
|:---|:---|
|**Check probe behavior and control-word restoration** <br> `crates/rdf-core/src/distance/binary64_tests.rs`|The test checks that direct probing reports failing rows and refusal while `env::probe`, `Exact::resolve`, and `Reassociated::resolve` succeed during bypass. It also checks x87 control-word restoration.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal



**Estimated code review effort:** 3 (Moderate) | ~20 minutes

<!-- change_assessment_commit:"5170165b8695d7458b211882c7fb8c0cf45675c0" -->
**Change:** Feature · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>51701</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `51701`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"5170165b8695d7458b211882c7fb8c0cf45675c0","coveredCommitId":"5170165b8695d7458b211882c7fb8c0cf45675c0","kind":"reviewed"} -->

This adds a public way to freeze a mutable dataset with caller-chosen limits and keeps the default freeze behavior. No merge-blocking risk is evident from the supplied context.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 4 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks (1 warning)</strong>

|         Check name         |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                            |
| :------------------------ | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Out of Scope Changes check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | The change to `crates/rdf-core/src/distance/binary64_tests.rs` changes x87 precision bypass and control-word restoration tests. It has no connection to Issue `#542`'s mutable freeze limits or compaction… | Remove the x87 test correction from this pull request or move it to a separate pull request. Keep the mutable freeze limit implementation and its focused tests here. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks (4 passed)</strong></summary>
<dl>
<dd>

|      Check name     |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :----------------- | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Description Check  | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: allowing callers to select limits for mutable dataset compaction.                                                                              |
| Linked Issues check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#542` requires caller-selected limits during mutable compaction. `MutableDataset::freeze_with_limits` now uses the supplied `ViewLimits` for the intermediate snapshot, while `freeze()` delegates … |
|  Docstring Coverage | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Docstring coverage is 100.00% which is sufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 10 functions across 4 files.                |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The change to `crates/rdf-core/src/distance/binary64_tests.rs` changes x87 precision bypass and control-word restoration tests. It has no connection to Issue `#542`'s mutable freeze limits or compaction behavior. The freeze implementation and related tests are in scope, but this independent test correction is an unrelated change.

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
<summary><strong>📝 Generate docstrings</strong></summary>
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
