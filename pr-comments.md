# PR #557 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/557?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review paused by coderabbit.ai -->

> [!NOTE]
> ## Reviews paused
> 
> It looks like this branch is under active development. To avoid overwhelming you with review comments due to an influx of new commits, CodeRabbit has automatically paused this review. You can configure this behavior by changing the `reviews.auto_review.auto_pause_after_reviewed_commits` setting.
> 
> - Last reviewed commit: c8f5ebafd6bce42f9af95c7d78dd6253a39514a1
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
- **Run ID**: `ac929b43-ef71-42bd-9601-621597287dd4`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 8c8295da8617471bcb354d3a60ba6d14432b484c and c8f5ebafd6bce42f9af95c7d78dd6253a39514a1.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 3</strong></summary>
<dl>
<dd>

* `crates/retrieval/src/executor.rs`
* `crates/retrieval/src/fusion_stream.rs`
* `crates/retrieval/tests/support/streams.rs`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 0 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 4 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>



</dd>
</dl>
</details>

<!-- walkthrough_end -->
<!-- change_assessment_start -->
**Priority:** ⬆️ High

<!-- change_assessment_commit:"c8f5ebafd6bce42f9af95c7d78dd6253a39514a1" -->
**Change:** Feature
<!-- change_assessment_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>🟡 Moderate</strong> · up to <code>c8f5e</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **🟡 Moderate** · up to `c8f5e`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"c8f5ebafd6bce42f9af95c7d78dd6253a39514a1","coveredCommitId":"c8f5ebafd6bce42f9af95c7d78dd6253a39514a1","kind":"reviewed"} -->

Queries on views without statistics may be refused because replay overestimates intermediate cells. Align replay with planning before merging.
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
| Out of Scope Changes check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | The statistics and planner changes support `#484`. The PR also changes unrelated behavior or validation in `crates/text/src/stem.rs`, where final consonants `w` through `y` now qualify as short-syllable… | Remove the unrelated text stemming, SIMD audit, shapes lint, and retrieval pinning changes from this pull request, or move them to separate pull requests. Keep a change only if the author provides concrete evidence that it is required to im… |
|     Docstring Coverage     | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 39.29% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 336 functions across 49 files.             | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                                                                                                                |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 3 passed</strong></summary>
<dl>
<dd>

|      Check name     |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :----------------- | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Description Check  | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: using snapshot statistics in both SPARQL and rule planning.                                                                                    |
| Linked Issues check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#484` is open and directly linked. The PR adds the shared `SnapshotStatistics` seam with exact or upper-bound counts, positional distinct domains, provenance, revisions, and reifier/annotation st… |

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The statistics and planner changes support `#484`. The PR also changes unrelated behavior or validation in `crates/text/src/stem.rs`, where final consonants `w` through `y` now qualify as short-syllable endings; `scripts/check-simd-asm.py` and `docs/design/purrdf-simd.md`, which change SIMD audit handling; `crates/shapes/src/term.rs`, which adds an unrelated lint allowance; and `crates/retrieval/src/executor.rs` and `crates/retrieval/src/fusion_stream.rs`, which replace heap pinning with stack pinning. These changes have no demonstrated connection to snapshot statistics or planner ordering.

**Resolution**

Remove the unrelated text stemming, SIMD audit, shapes lint, and retrieval pinning changes from this pull request, or move them to separate pull requests. Keep a change only if the author provides concrete evidence that it is required to implement or validate `#484`.

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

# Snapshot statistics consumed by planning and rules

Deliver the entire issue: predicate/class frequencies, positional distinct
counts, selected joint frequencies, statement-layer counts, deterministic
snapshot provenance and exact-versus-upper-bound evidence. The SPARQL planner
and native rule-order caller must consume this seam; an unused statistics API
is not delivery. Counts add only over disjoint effective rows. Distinct domains
merge by sorted identity union, never by summing their sizes.

The checkout starts at the already published reifier-index implementation while
its PR finishes CI. This is an implementation dependency, not an extra copy of
that delivery. After it lands, synchronize normally with main and assess the
actual integration delta. Main remains protected. Use no subagents, added runtime
dependency, new custom compiler/cache machinery or timing acceptance. Retain
standard directories and bounded builds. Owner review is not independent approval.

## Task 1: Shared statistics contract and real storage providers

Put the typed count/provenance and provider seam in core, the common dependency
of the evaluator and rule consumers. Unknown distinct counts remain unknown;
upper bounds never become exact by addition. Reuse existing physical index,
PageSummary and segmented-summary homes and original allocation admission.
Cover predicate row/subject/object statistics, class frequencies, annotation
predicates, reifier target predicates and selected object-kind/datatype/value
frequencies. Bind every provider to immutable source/revision identity. Preserve
first operational errors and snapshot prefixes. Compile coherent production
slices as they are adapted; no hundreds-file uncompiled migration.

## Task 2: Estimator, caller-supplied provider and rule consumer

Replace global-term join selectivity where predicate-specific domains exist;
include structural triple components and statement streams. Use the same core
seam for rule connected-order tie-breaking. Expose actual exactness/source in
EXPLAIN and include changed provider identity in retained-plan cache inputs.
A real consumer-supplied provider must change an actual plan. Keep cost units,
output certificates and statistical estimates distinct. Preserve RDF answers,
graphs, multiplicity and original resource/error semantics.

## Task 3: Acceptance and protected delivery

Execute all five issue checks: actual million reified claims with estimate within
2x; paged plan change from reifier-count changes alone; rare-class-first native
and paged plans; supplied provider changes plan with exactness visible; property
merge parity between two segments and one combined segment. Include original
allocation/capacity, snapshot, portable and affected conformance controls.
Use layout/allocation/operation/instruction evidence, never elapsed time.
Run the single required full final gate after source settles. Resolve actual
material forge review and hosted checks, merge through ghprsq, verify closure,
evidence archive and mirror, then remove this delivery branch and update the
root tracker. No foundation-only PR or new follow-up discharges this contract.

