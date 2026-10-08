# PR #462: SHACL dated profile selection and complete validation reports

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Comments (2)

### coderabbitai — 2026-10-06T08:20:11Z

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/462?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=2" alt="Review in Change Stack →" width="220" height="32"></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 7 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary>View limit details</summary>
> 
> **Limit details:** You’ve used the included review currently available. Your 114 included PR review attempts over the past 7 days set your current allowance at 1 review per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/plans#rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary>⚙️ Run configuration</summary>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `b5d86b44-2b7c-42f8-b012-b19c1d0615c0`
> 
> </details>
> 
> <details>
> <summary>📥 Commits</summary>
> 
> Reviewing files that changed from the base of the PR and between dc8040ba6d2f0945c88f360a54db835aac798824 and 44dbf5ceaba7908aee0f840e4227bf42b040f697.
> 
> </details>
> 
> <details>
> <summary>📒 Files selected for processing (8)</summary>
> 
> * `crates/shapes/src/constraints.rs`
> * `crates/shapes/src/engine.rs`
> * `crates/shapes/src/profile.rs`
> * `crates/shapes/src/report.rs`
> * `crates/shapes/tests/complete_reports.rs`
> * `crates/shapes/tests/dated_execution.rs`
> * `crates/sparql-conformance/src/community.rs`
> * `crates/sparql-conformance/tests/community_conformance.rs`
> 
> </details>
> 
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- walkthrough_start -->

<details>
<summary>📝 Walkthrough</summary>

## Walkthrough

The pull request adds dated SHACL profile selection and complete validation reports. It also adds a grading library, a 64-case community corpus, and a runner that executes cases under the built-in dated profiles and emits grading artifacts.

### Changes

**Dated SHACL profiles and query admission**

|Layer / File(s)|Summary|
|:---|:---|
|**Profile-aware parsing and execution** <br> `crates/shapes/src/profile.rs`, `crates/shapes/src/prebinding.rs`, `crates/shapes/src/query_law.rs`, `crates/shapes/src/shapes/parser/*`, `crates/shapes/src/sparql.rs`, `crates/sparql-eval/src/user_fn.rs`|Adds legacy, 2017 Recommendation, and 2026 Working Draft profiles. Parsing and execution carry query purpose and declared bindings into profile-specific admission checks.|
|**Complete validation reports** <br> `crates/shapes/src/report.rs`, `crates/shapes/src/constraints.rs`, `crates/shapes/src/engine.rs`, `crates/validate/src/complete.rs`|Adds typed complete-validation outcomes and reports that retain source context, recursive result evidence, and blank-node correspondence. The validate crate adds source, document, and product entry points, plus JSON and SARIF rendering.|
|**Prepared products and XPath handling** <br> `crates/shapes/src/product/*`, `crates/shapes/src/xpath.rs`, `crates/validate/src/product.rs`|Adds options-aware product admission and rebuild, and passes selected profile and XPath settings through parse, bind, and validation operations.|

**Community conformance corpus and grading**

|Layer / File(s)|Summary|
|:---|:---|
|**Grading data contracts and comparisons** <br> `crates/conformance-kit/src/*`, `crates/iri/src/vocab.rs`|Adds inventory, manifest, result-set, review, outcome, solution, and report grading support, with shared RDF vocabulary constants.|
|**Reviewed community SHACL cases** <br> `corpora/community/*`|Adds catalog, manifests, review records, licensing terms, and cases covering query admission, validation reports, paths, validators, rules, focus nodes, and SPARQL behavior.|
|**Corpus acquisition and execution** <br> `crates/sparql-conformance/src/community.rs`, `crates/sparql-conformance/src/bin/community-conformance.rs`, `crates/sparql-conformance/tests/community_conformance.rs`|Adds acquisition and review admission, case/profile execution, report and inference grading, scoreboard totals, JSON records, and EARL output.|

**Workspace and supporting integration**

|Layer / File(s)|Summary|
|:---|:---|
|**Workspace, benchmark, and conformance wiring** <br> `Cargo.toml`, `crates/sparql-conformance/Cargo.toml`, `scripts/conformance-matrix.py`, `scripts/conformance-baseline.json`, `layers.toml`, `helpers-ledger.toml`|Adds the unpublished grading crate to the workspace and wires the community suite into conformance reporting. Adds SHACL report benchmarks and updates ambient-context ledger checks.|
|**Documentation and release metadata** <br> `CHANGELOG.md`, `crates/shapes/README.md`, `crates/validate/README.md`, `docs/*`, `AGENTS.md`|Documents the profile and report APIs, corpus contract and runner, benchmark targets, and the added unpublished crate.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal



**Estimated code review effort:** 5 (Critical) | ~180 minutes

<!-- change_assessment_commit:"dc8040ba6d2f0945c88f360a54db835aac798824" -->

<!-- change_assessment_end -->

</details>

<!-- walkthrough_end -->
<!-- final_review_risk_start -->
**Merge Risk:** **🟡 Moderate** · up to `dc804`
<!-- final_review_risk_coverage:{"sourceCommitId":"dc8040ba6d2f0945c88f360a54db835aac798824","coveredCommitId":"dc8040ba6d2f0945c88f360a54db835aac798824","kind":"reviewed"} -->

Under the new dated SHACL profiles, a refused function call can be silently ignored when another alternative passes. Validation could then report conformance that the selected profile should have refused. Fix this before merging. A smaller issue in the conformance runner only matters if the corpus layout changes.
<!-- final_review_risk_end -->
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary>🚥 Pre-merge checks | ✅ 3 | ❌ 1 | ❓ 1</summary>

### ❌ Failed checks (1 warning, 1 inconclusive)

|         Check name         | Status         | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                                                                                        |
| :------------------------ | :------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Out of Scope Changes check | ⚠️ Warning     | The whole-PR summary includes substantial changes not required by `#402`. Examples include SPARQL user-function admission and refusal publication in `crates/sparql-eval`, XPath and division-policy integ… | Split unrelated `#406` and merged-main feature changes into separate pull requests, or provide direct linked issue scope that requires each change. Keep this pull request limited to the `#402` implementation, its supporting tests, documentation… |
|     Docstring Coverage     | ❓ Inconclusive | Docstring coverage is 70.08% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 595 functions across 50 files. (180 skipp… | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                                                                                                                |

<details>
<summary>✅ Passed checks (3 passed)</summary>

|      Check name     | Status   | Explanation                                                                                                                                                                                               |
| :----------------- | :------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Description Check  | ✅ Passed | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|     Title check     | ✅ Passed | The title clearly and concisely identifies the two primary changes: SHACL dated profile selection and complete validation reports.                                                                        |
| Linked Issues check | ✅ Passed | Direct issue `#402` is addressed. The PR adds additive `ShaclProfile` selection with `LEGACY` as the default and rejects unsupported IDs. It adds typed profile-specific pre-binding admission and rejecti… |

</details>

<details>
<summary>Full details: Out of Scope Changes check</summary>

**Explanation**

The whole-PR summary includes substantial changes not required by `#402`. Examples include SPARQL user-function admission and refusal publication in `crates/sparql-eval`, XPath and division-policy integration, C ABI changes, broad WASM and Python binding changes, and translation and release updates. The description explicitly identifies these as `#406` and merged-main work such as `#441`, `#455`, and `#465`. These changes are separate from profile selection, complete reports, profile admission, and the community runner requested by `#402`.

**Resolution**

Split unrelated `#406` and merged-main feature changes into separate pull requests, or provide direct linked issue scope that requires each change. Keep this pull request limited to the `#402` implementation, its supporting tests, documentation, corpus, and conformance tooling.

</details>

<details>
<summary>Full details: Docstring Coverage</summary>

**Explanation**

Docstring coverage is 70.08% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 595 functions across 50 files. (180 skipped: 160 unsupported, 20 over the file limit.)

</details>

</details>

<!-- pre_merge_checks_walkthrough_end -->
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

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Create a new PR

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

### paudley — 2026-10-08T11:58:11Z

The retained SHACL source and resolved XPath/shared-foundation merge are now published in dc8040ba6 through a normal signed commit with hooks. The committed tree is the same tree exercised by the current focused qualification.

Native correctness and hygiene passed, including 115 applicable community-profile executions and 535 observations. Shapes and Validate each passed the minor-release API comparison against rust-v3.0.1. Fresh installed Python and optimized packaged WASM boundaries passed: 13 Python cases and 20 Node cases, including all eight JSPI async cases, with no failures or skips. Owning translation/render/document gates passed, including all seven poison refusals, all 3,039 translated catalogue messages, 26 parsed SPARQL fences, 154 generated claim checks and both HTML builds.

This is progress, not final acceptance. Matched complete-report timing is still pending while the separate CI measurement campaign runs. Fresh published-head CI, final main merge-tree/layer/lock/document composition, review dispositions and independent whole-scope acceptance remain required. Existing external submissions remain untouched; this delivery requires no submission to another repository.


