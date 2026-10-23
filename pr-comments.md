# PR #687 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 



<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/687?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review paused by coderabbit.ai -->

> [!NOTE]
> ## Reviews paused
> 
> It looks like this branch is under active development. To avoid overwhelming you with review comments due to an influx of new commits, CodeRabbit has automatically paused this review. You can configure this behavior by changing the `reviews.auto_review.auto_pause_after_reviewed_commits` setting.
> 
> - Last reviewed commit: 06f735f24a9a72393417de82f992e24c446caa09
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

- **Source mode**: Auto
- **Repository**: Unavailable
- **Branch**: Unavailable
- **File**: No configuration file from this repository applied
- **Commit**: Unavailable
- **Layers**: defaults
- **Review profile**: CHILL
- **Plan**: Team
- **Run ID**: `0d6615ad-1f5b-48d9-9316-942a5f25a11e`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 154d11d6155d88e0dc232612020e16d09be66f07 and 06f735f24a9a72393417de82f992e24c446caa09.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 2</strong></summary>
<dl>
<dd>

* `.github/workflows/ci.yaml`
* `docs/design/purrdf-simd.md`

</dd>
</dl>
</details>

<details>
<summary><strong>🚧 Files skipped from review as they are similar to previous changes: 1</strong></summary>
<dl>
<dd>

* docs/design/purrdf-simd.md

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 0 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 1 review per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The PR adds source-derived identities and change observations across RDF core and text indexing. It updates segmented storage to format version 4, makes text-index deltas incremental, and narrows WebAssembly test selections.

**Changes: Source accounting and observation**

|Layer / File(s)|Summary|
|:---|:---|
|**Multiset and canonical source identity** <br> `crates/hash/...`, `crates/rdf-core/src/source.rs`, `crates/rdf-core/src/ir/term*`|Adds `Multiset256`, canonical byte encoders, source-row hashes, source summaries, and source-evidence interfaces.|
|**Segmented source persistence and observation** <br> `crates/rdf-core/src/ir/statistics*`, `crates/rdf-core/src/ir/segmented/...`, `crates/rdf-core/src/ir/source_directory.rs`|Stores source summaries, effective directories, and source unions in format version 4. Adds source-version capture and source-row transition reporting across segmented snapshots.|

**Incremental text indexing**

|Layer / File(s)|Summary|
|:---|:---|
|**Persistent index and surface storage** <br> `crates/text/src/index.rs`, `crates/text/src/index/token_streams.rs`, `crates/text/src/surface*`|Uses shared persistent tables and profile-parameterized surface indexes.|
|**Source-validated deltas and query access** <br> `crates/text/src/index/delta.rs`, `crates/text/src/index/source_digest.rs`, `crates/text/src/error.rs`, `crates/text/src/relation.rs`, `crates/text/src/score.rs`, `crates/text/src/dictionary.rs`|Maintains source digests and coverage incrementally, validates deltas against independent source evidence, and updates changed index structures. Query paths use workspace-aware lookups.|

**WASM and supporting changes**

|Layer / File(s)|Summary|
|:---|:---|
|**Audited WASM execution** <br> `.github/workflows/*`, `Makefile`, `docs/WASM_TESTING.md`, `crates/geo-kernel/examples/qualify_determinism.rs`|Removes semantic-only WASM jobs and broad target runs. Retains exact scalar and SIMD cases for named platform behavior and frozen identities.|
|**Repository guidance and validation** <br> `AGENTS.md`, `CLAUDE.md`, `docs/agents/*`, `crates/rdf-core/tests/*`, `crates/text/tests/*`, `helpers-ledger.toml`, `docs/design/*`|Adds engineering guidance and tests for source observation, text deltas, allocation behavior, and format certification. Updates design documents and identity records.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal

**Estimated code review effort:** 5 (Critical) | ~90 minutes

<!-- change_assessment_commit:"06f735f24a9a72393417de82f992e24c446caa09" -->
**Severity of issue fixed:** Medium
<!-- change_assessment_end -->

**Possibly related issues**

- [Blackcat-Informatics/purrdf#564](https://github.com/Blackcat-Informatics/purrdf/issues/564): The PR updates segmented source-directory persistence and certification. The provided issue summary says it does not complete the block-bounded builder.

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>06f73</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `06f73`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"06f735f24a9a72393417de82f992e24c446caa09","coveredCommitId":"06f735f24a9a72393417de82f992e24c446caa09","kind":"reviewed"} -->

The reviewed changes are mergeable with normal checks; no concrete current-head issue is identified.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 3 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 2</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks: 2 warnings</strong>

|         Check name         |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                                                                                        |
| :------------------------ | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Out of Scope Changes check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | The pull request includes a repository-wide WASM audit that is unrelated to issue [`#683`]. It removes 312 focused WASM case executions and three semantic-only workflow jobs, changes the WASM `Makefile`… | Move the WASM workflow, `Makefile`, geographic qualifier, WASM inventory, and WASM-specific agent documentation changes to a separate pull request with its own linked issue. Retain the text-index delta, source-observation, persistent-table,… |
|     Docstring Coverage     | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 35.87% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 538 functions across 51 files. (2 skipped… | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                                                                                                                |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 3 passed</strong></summary>
<dl>
<dd>

|      Check name     |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :----------------- | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Description Check  | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely summarizes both primary changes: incremental text-index updates and removal of redundant WASM tests.                                                                      |
| Linked Issues check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue [`#683`] requires incremental source fingerprint and coverage maintenance, changed-subject row collection, and parity with a fresh build. `TextIndex::with_delta` now observes source changes, updat… |

<details>
<summary><strong>Full details: Linked Issues check</strong></summary>
<dl>
<dd>

**Explanation**

Issue [`#683`] requires incremental source fingerprint and coverage maintenance, changed-subject row collection, and parity with a fresh build. `TextIndex::with_delta` now observes source changes, updates source digest and coverage, probes only changed subjects, and edits persistent index and surface tables. The 150-step determinism test compares delta-updated indexes with fresh builds for documents, postings, statistics, fingerprints, coverage, and rankings.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The pull request includes a repository-wide WASM audit that is unrelated to issue [`#683`]. It removes 312 focused WASM case executions and three semantic-only workflow jobs, changes the WASM `Makefile` selection and geographic qualifier, and adds WASM policy and agent documentation. These changes do not implement incremental text-index source accounting, coverage maintenance, changed-subject probing, or fresh-build parity.

**Resolution**

Move the WASM workflow, `Makefile`, geographic qualifier, WASM inventory, and WASM-specific agent documentation changes to a separate pull request with its own linked issue. Retain the text-index delta, source-observation, persistent-table, and parity changes required by [`#683`].

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 35.87% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 538 functions across 51 files. (2 skipped: 2 unsupported.)

**Resolution**

Write docstrings for the functions missing them to satisfy the coverage threshold.

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
<!-- This is an auto-generated comment: all tool run failures by coderabbit.ai -->

> [!WARNING]
> Some tools did not complete. Review the errors below.
> 
> <details>
> <summary><strong>🔧 LanguageTool</strong></summary>
> <dl>
> <dd>
> 
> <details>
> <summary><strong>docs/design/purrdf-simd.md</strong></summary>
> <dl>
> <dd>
> 
> LanguageTool checks are incomplete because the per-file request limit of 5 was reached. Remaining text was skipped; findings from completed checks are retained.
> 
> </dd>
> </dl>
> </details>
> 
> </dd>
> </dl>
> </details>

<!-- end of auto-generated comment: all tool run failures by coderabbit.ai -->
<!-- tips_start -->






<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

All 50 revised-head checks pass, with six configured skips and no waiver. This
includes all seven assembly configurations and all four complete RISC-V parts.

The two advisory review checks are explicitly adjudicated:

- Keep the combined WASM audit scope. The user expressly requested auditing all
  WASM selections, correcting and linking the guidance, and publishing this
  combined PR. The cull preserves native semantic coverage and retains the
  actual platform/dispatch/host/frozen-identity executions.
- The default docstring percentage includes private and test functions. The
  repository's public missing_docs lint and strict all-target builds pass with
  warnings denied; public profile/source/selection contracts are documented.
  The default 80% heuristic is not an adopted repository gate.

LanguageTool's per-file limit remains disclosed as incomplete prose-tool
coverage. The changed function names/locations were read against source, and
the document's measurement cells now pass the actual floating-nightly matrix.
No additional review or runtime pass is inferred from that tool limit.

The exact geographic-selection finding and Rust-guide nit were fixed in
309480c1f; the only inline thread is resolved. The current hosted source review
marks 06f735f24 covered and reports no actionable current-head finding. The
separate block-bounded-builder contract remains open under #564.

