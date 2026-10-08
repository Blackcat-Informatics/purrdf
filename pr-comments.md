# PR #462 — comments and review threads

6 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/462?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=2" alt="Review in Change Stack →" width="220" height="32"></a>

<!-- review_stack_entry_end -->
<!-- recent_review_start -->

No actionable comments were generated in the recent review. 🎉

<details>
<summary>ℹ️ Recent review info</summary>

<details>
<summary>⚙️ Run configuration</summary>

- **Configuration used**: defaults
- **Review profile**: CHILL
- **Plan**: Team
- **Run ID**: `b1242e8c-186b-4216-8670-a8135f7d6568`

</details>

<details>
<summary>📥 Commits</summary>

Reviewing files that changed from the base of the PR and between dc8040ba6d2f0945c88f360a54db835aac798824 and 44dbf5ceaba7908aee0f840e4227bf42b040f697.

</details>

<details>
<summary>📒 Files selected for processing (8)</summary>

* `crates/shapes/src/constraints.rs`
* `crates/shapes/src/engine.rs`
* `crates/shapes/src/profile.rs`
* `crates/shapes/src/report.rs`
* `crates/shapes/tests/complete_reports.rs`
* `crates/shapes/tests/dated_execution.rs`
* `crates/sparql-conformance/src/community.rs`
* `crates/sparql-conformance/tests/community_conformance.rs`

</details>

**Included review availability:** This review used your included allowance. 0 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 1 review per hour.

</details>

---



<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary>📝 Walkthrough</summary>

## Walkthrough

The pull request adds dated SHACL profile selection, profile-aware query admission, complete validation reports, and typed validation outcomes. It also adds an unpublished conformance kit, a reviewed 64-case community corpus, and a runner for 115 dated executions.

### Changes

**SHACL profiles and complete validation**

|Layer / File(s)|Summary|
|:---|:---|
|**Profile selection and query admission** <br> `crates/shapes/src/profile.rs`, `crates/shapes/src/prebinding.rs`, `crates/shapes/src/query_law.rs`, `crates/shapes/src/shapes/parser/*`, `crates/shapes/src/sparql.rs`|Adds legacy, Recommendation 2017, and Working Draft 2026 profiles. Admission now carries query purpose and declared bindings, with typed refusals for prohibited query forms and pre-bound variables.|
|**Complete report production** <br> `crates/shapes/src/constraints.rs`, `crates/shapes/src/report.rs`, `crates/shapes/src/engine.rs`|Adds source-constraint evidence, recursive details, source acquisitions, blank-node correspondence, typed semantic and resource failures, and complete validation entry points.|
|**Public validation and product APIs** <br> `crates/validate/src/complete.rs`, `crates/validate/src/product.rs`, `crates/validate/src/shacl.rs`, `crates/shapes/src/product/*`, `crates/shapes/src/xpath.rs`|Adds options-aware product restoration and complete validation from documents, sources, and products. Complete reports serialize to JSON and SARIF, while existing compatibility projections remain available.|

**SPARQL evaluation and support**

|Layer / File(s)|Summary|
|:---|:---|
|**Invocation admission and refusal publication** <br> `crates/sparql-eval/src/engine.rs`, `crates/sparql-eval/src/user_fn.rs`, `crates/sparql-eval/src/eval.rs`, `crates/shapes/src/sparql.rs`|Adds original-query access, user-function admission callbacks, typed function refusals, and request-local publication across query, prepared, governed, update, explain, and construct paths.|
|**Vocabulary and runtime integration** <br> `crates/iri/src/vocab.rs`, `crates/rdf-wasm/src/async_query.rs`, `crates/rdf-wasm/src/interleaving.rs`, `crates/shapes/src/shacl_corpora/*`|Adds shared manifest, result-set, SHACL-test, and EARL vocabulary constants. Updates ambient-context ledger expectations and reuses shared vocabulary constants.|

**Community conformance**

|Layer / File(s)|Summary|
|:---|:---|
|**Grading contracts and comparisons** <br> `crates/conformance-kit/src/*`, `crates/conformance-kit/tests/grading.rs`|Adds strict inventory, manifest, result-set, report, solution, outcome, and review-admission handling. Comparisons preserve row multiplicity, blank-node identity, source correspondence, path structure, and report policy.|
|**Reviewed corpus** <br> `corpora/community/*`|Adds a versioned catalog, 64 SHACL cases, dated profile metadata, expected reports, review records, BLAKE3 artifact admission, licensing files, and corpus documentation.|
|**Runner and conformance reporting** <br> `crates/sparql-conformance/src/community.rs`, `crates/sparql-conformance/src/bin/community-conformance.rs`, `crates/sparql-conformance/tests/community_conformance.rs`|Adds corpus acquisition, profile selection, validation and rule execution, typed outcome grading, JSON and EARL output, scoreboard totals, relocation checks, and reviewed-artifact integrity tests.|

**Workspace and documentation**

|Layer / File(s)|Summary|
|:---|:---|
|**Workspace and CI wiring** <br> `Cargo.toml`, `crates/conformance-kit/Cargo.toml`, `crates/sparql-conformance/Cargo.toml`, `layers.toml`, `scripts/conformance-matrix.py`, `scripts/conformance-baseline.json`, `helpers-ledger.toml`|Adds the unpublished grading crate and wires the community suite into workspace dependency, baseline, matrix, and ledger checks.|
|**Documentation and release metadata** <br> `CHANGELOG.md`, `crates/shapes/README.md`, `crates/validate/README.md`, `docs/*`, `AGENTS.md`|Documents profile rules, complete reports, conformance behavior, benchmark coverage, corpus status, and the twelve-crate crates.io exclusion list.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal

**Estimated code review effort:** 5 (Critical) | ~180 minutes

<!-- change_assessment_commit:"44dbf5ceaba7908aee0f840e4227bf42b040f697" -->

<!-- change_assessment_end -->

</details>

<!-- walkthrough_end -->
<!-- final_review_risk_start -->
**Merge Risk:** **⚪ Minimal** · up to `44dbf`
<!-- final_review_risk_coverage:{"sourceCommitId":"44dbf5ceaba7908aee0f840e4227bf42b040f697","coveredCommitId":"44dbf5ceaba7908aee0f840e4227bf42b040f697","kind":"reviewed"} -->

This change adds selectable dated SHACL profiles, complete validation reports and a community conformance corpus. The supplied review found no outstanding merge-blocking issues. The remaining pending items are the CI and acceptance steps the author lists.
<!-- final_review_risk_end -->
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary>🚥 Pre-merge checks | ✅ 3 | ❌ 2</summary>

### ❌ Failed checks (2 warnings)

|         Check name         | Status     | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                                                                                        |
| :------------------------ | :--------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Out of Scope Changes check | ⚠️ Warning | The reviewed head includes changes unrelated to directly linked issue `#402`. Examples include SPARQL user-function admission and refusal publication in `crates/sparql-eval`, XPath and division-policy i… | Split the unrelated `#406` and merged-main feature changes into separate pull requests, or link active issues that require each change. Keep this pull request limited to `#402` implementation, supporting tests, corpus, conformance tooling, and … |
|     Docstring Coverage     | ⚠️ Warning | Docstring coverage is 70.68% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 614 functions across 51 files.             | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                                                                                                                |

<details>
<summary>✅ Passed checks (3 passed)</summary>

|      Check name     | Status   | Explanation                                                                                                                                                                                               |
| :----------------- | :------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Description Check  | ✅ Passed | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ✅ Passed | The title clearly and concisely identifies the primary changes: selectable dated SHACL profiles and complete validation reports.                                                                          |
| Linked Issues check | ✅ Passed | Direct issue `#402` is addressed. `ShaclProfile` provides `LEGACY`, `REC_20170720`, and `WD_20260918`, with exact unsupported-ID rejection and additive default behavior. Profile-specific admission uses … |

</details>

<details>
<summary>Full details: Out of Scope Changes check</summary>

**Explanation**

The reviewed head includes changes unrelated to directly linked issue `#402`. Examples include SPARQL user-function admission and refusal publication in `crates/sparql-eval`, XPath and division-policy integration, C ABI changes, broad Python and WASM changes, and translation and release updates. The current description identifies these as carried `#406` and merged-main work, but no directly linked issue here requires them.

**Resolution**

Split the unrelated `#406` and merged-main feature changes into separate pull requests, or link active issues that require each change. Keep this pull request limited to `#402` implementation, supporting tests, corpus, conformance tooling, and directly related documentation.

</details>

</details>

<!-- pre_merge_checks_walkthrough_end -->

- [ ] <!-- {"checkboxId":"585bb3f6-faf5-4dbf-96d2-74e382adf19a"} --> Fix all pre-merge checks with AI
<!-- finishing_touch_checkbox_start -->

<details>
<summary>✨ Finishing Touches 💡 1</summary>

<!-- finishing_touch_suggestion:docstrings -->
<details open>
<summary>📝 Generate docstrings 💡</summary>

- [ ] <!-- {"checkboxId":"3e1879ae-f29b-4d0d-8e06-d12b7ba33d98"} --> Commit to this branch
- [ ] <!-- {"checkboxId":"7962f53c-55bc-4827-bfbf-6a18da830691"} --> Create a new PR

</details>
<details open>
<summary>🧪 Generate unit tests (beta)</summary>

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-6060802212"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-6060802212"} --> Create a new PR

</details>

</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autopilot</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

The retained SHACL source and resolved XPath/shared-foundation merge are now published in dc8040ba6 through a normal signed commit with hooks. The committed tree is the same tree exercised by the current focused qualification.

Native correctness and hygiene passed, including 115 applicable community-profile executions and 535 observations. Shapes and Validate each passed the minor-release API comparison against rust-v3.0.1. Fresh installed Python and optimized packaged WASM boundaries passed: 13 Python cases and 20 Node cases, including all eight JSPI async cases, with no failures or skips. Owning translation/render/document gates passed, including all seven poison refusals, all 3,039 translated catalogue messages, 26 parsed SPARQL fences, 154 generated claim checks and both HTML builds.

This is progress, not final acceptance. Matched complete-report timing is still pending while the separate CI measurement campaign runs. Fresh published-head CI, final main merge-tree/layer/lock/document composition, review dispositions and independent whole-scope acceptance remain required. Existing external submissions remain untouched; this delivery requires no submission to another repository.

## 3. paudley — 

@coderabbitai review

## 4. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
<!-- CodeRabbit review command invocation: v2:74b44c744e75d6a40a9a035a9a873b37d262af44948a589192e8623f48f93640 -->
<details>
<summary>✅ Action performed</summary>

Review finished.

> Note: CodeRabbit is an incremental review system and does not re-review already reviewed commits. This command is applicable only when automatic reviews are paused.

</details>

## 5. paudley — 

The supporting XPath/shared-foundation integration and independent community corpus are part of the authorized GREENFIELD portfolio delivery. The dated SHACL profiles depend on the native dated XPath bundles, and the complete-report contract requires the independently specified corpus and contextual grader. Current-main benchmark changes are assessed separately through the clean merge-tree candidate and actual combined-tree hosted gates. Splitting these necessary seams by historical issue boundaries would not improve the delivered contract.

The docstring percentage is inconclusive: its inventory includes skipped and over-limit files and does not identify an undocumented public invariant. The public dated profile, complete report, source-context and typed refusal contracts are documented, and the warning-denied Rust documentation/API gates passed. We decline blanket generated docstrings, generated tests and the optional finishing-touch suggestions; meaningful native controls and owning documentation are already qualified. Concrete documentation defects remain actionable if identified.

Both substantive review findings are fixed and their threads are resolved. The matched report measurements are complete. Final acceptance still awaits the requested current-head review and the isolated RISC-V determinism job retry; neither is being inferred from the other successful checks.

## 6. paudley — 

Disposition of the three optional suggestions in the earlier formal review:

- A second cross-law occurrence cache would add retained state and synchronization. The current cache admits only an exact law/source/provenance match and otherwise reconstructs the source model; mixed-law correctness and same-law prepared reuse are verified. We decline the additional cache without evidence that its retention and synchronization cost improves this workload.
- The dated infer_complete convenience door constructs an owned transient preparation for admission; the default Legacy door directly executes. We retain the current preparation ownership and admission boundary. This cold convenience-path optimization is not necessary for the complete report contract or its measured prepared-report costs.
- The potential and declared lists serve dated admission and existing grouping respectively. They currently enumerate the same names in source order; potential is dated-only and precedes argument evaluation, while declared is constructed after successful argument evaluation. We retain the qualified allocation and admission order rather than apply the suggested unconditional hoist. No declaration mismatch is accepted.

These are declined optional optimizations/refactoring suggestions, not deferred defects. Both actionable correctness findings are repaired and resolved, and the completed exact-head review reports no new actionable comments.

