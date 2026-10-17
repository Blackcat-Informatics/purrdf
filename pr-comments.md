# PR #589 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/589?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

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
- **Run ID**: `3cdc98f9-2ba8-4c10-8faf-f15cba3da1b7`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 378398ca7c0c493270754d35c9ffa79ff32c3b3c and a567a33e6a2639dc726cda682cbfb269385b931f.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 2</strong></summary>
<dl>
<dd>

* `crates/validate/examples/refresh_dl_proof_vectors.rs`
* `crates/validate/tests/dl_proof_vectors.rs`

</dd>
</dl>
</details>

<details>
<summary><strong>🚧 Files skipped from review as they are similar to previous changes: 2</strong></summary>
<dl>
<dd>

* crates/validate/tests/dl_proof_vectors.rs
* crates/validate/examples/refresh_dl_proof_vectors.rs

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 0 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 2 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The pull request adds a proof-vector refresh example, regenerates DL proof fixtures, and adds a test that validates the 16 vectors against the current native boundary. The WASM test command and its documentation now include that test.

**Changes: DL proof vectors**

|Layer / File(s)|Summary|
|:---|:---|
|**Refresh and update proof vectors** <br> `crates/validate/examples/refresh_dl_proof_vectors.rs`, `crates/validate/tests/fixtures/dl-proof.vectors`, `crates/validate/src/regime.rs`, `crates/validate/src/regime/schema_vectors.rs`|The refresh example regenerates proof and check bodies while preserving fixture source and questions. The fixture contains regenerated proof payloads, including OWL payloads updated from version 4 to version 5. The ignored schema-vector sealing test is removed.|
|**Validate vectors in WASM tests** <br> `crates/validate/Cargo.toml`, `crates/validate/tests/dl_proof_vectors.rs`, `Makefile`, `docs/WASM_TESTING.md`|A registered test checks that the fixture contains 16 vectors and validates them against the current native boundary. `wasm-test` selects this test, and the WASM testing documentation includes it in the test totals.|

<!-- change_assessment_start -->
**Priority:** ⬆️ High

**Estimated code review effort:** 3 (Moderate) | ~20 minutes

<!-- change_assessment_commit:"a567a33e6a2639dc726cda682cbfb269385b931f" -->
**Change:** Bug fix
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>a567a</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `a567a`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"a567a33e6a2639dc726cda682cbfb269385b931f","coveredCommitId":"a567a33e6a2639dc726cda682cbfb269385b931f","kind":"reviewed"} -->

The refreshed vectors have a test covering the reported realization failure path. No actionable merge-blocking issue was found; optimized packaged-host acceptance remains pending.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 5</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the primary change: refreshing shared proof vectors for current-support recording.                                                              |
|     Docstring Coverage     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Docstring coverage is 100.00% which is sufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 2 functions across 3 files. |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped because no linked issues were found for this pull request.                                                                                                                  |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped because no linked issues were found for this pull request.                                                                                                                  |

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

## 2. paudley — 

# Repair the shared proof boundary artifact

The full logic goal remains active. This is an owned CI regression from native587: wasm-package job117458981838 executes the real shared check_dl_proof_golden_vectors door and rejects realize's old proof bytes. The prior entailment-only qualification missed the shared16-case artifact in purrdf-validate. Fix that artifact and its owning explicit refresh operation; preserve all mismatch checks and shared host consumers. GMEOW1843's clean worktree/plan remain intact while this directly introduced regression is repaired.

Native law, ordinary hooks, original profiles/ceilings, no runtime dependency/semantic feature and ghprsq-only integration apply. No machine/resource/environment probes or caps, banned commands, GMEOW producer/full gate/corpus factory, private memory or unrelated edits. Root REWORK_LIST.md is unrelated. No independent dispatcher is available; reviews are self-reviews, never fabricated independent verdicts. Source and all issue comments must be gathered by brief before publication. The named source is a first-party generated fixture, not frozen external vectors.

## Task 1: One explicit artifact producer and refreshed bytes

Add an explicit Rust producer for the existing shared proof vector artifact, using the actual public native prove/check entry points over each recorded case's original document/question. Preserve all case names, service/argument/input and expected answers byte for byte; recompute only proof and replay-report bodies after verifying the answer remains the expected one. Replace the obsolete one-time schema-only test sealer with this single refresh operation rather than retain two producers. Ordinary tests only compare, never refresh. Review changed bytes, schema/current-support trust classifications and source identities; no stronger semantic claim may be inferred merely from regeneration.

## Task 2: Qualify and integrate the actual boundary

Run the existing native shared checker over all16 cases and affected boundary regressions. Add a minimal portable harness over that same production checker if needed to execute every vector on real WASM without invoking the broad optimized package build; keep the optimized packaged door unchanged and require its hosted CI result separately. Register the exact portable door under the existing native/WASM inventory and document actual counts. Strict affected all-target Clippy, complete helper checks and normal hooks must pass. Read actual Make recipes before invoking them. No unsupported optimization-profile or full-package claim from a filtered portable run.

Review scope against the issue, commit/push normally, create a focused PR, inspect complete review/CI/thread surfaces, and integrate only with ghprsq under existing pending-CI authority. The optimized-package acceptance remains visibly unqualified until its actual hosted job passes; keep588 open if that acceptance is still pending. Archive evidence before owned cleanup. Resume GMEOW1843 immediately after this coherent repair, retaining the larger native/GMEOW integration goal.

## 3. paudley — 

Refresh shared DL proof boundary vectors for current support

Refs #588

## What changed

The v5 current-support proof extension changed canonical refutation bytes, while the shared cross-host boundary artifact still expected v3/v4. Packaged WASM and three Python/conformance lanes therefore rejected realize. Regenerate all16 cases through actual native prove/check operations, preserving every source, question, case identity and expected answer byte for byte. Ten proof digests change and six remain fixed; ordinary derived premises now receive the checked/trusted classification rather than an unattested count.

One explicit Rust example owns whole-artifact refresh, replacing the incomplete one-time schema-only test sealer. Missing services, duplicate cases, changed expected answers and source drift refuse before writing. A fixed-point run retains identical bytes and writes nothing changed. Ordinary native/schema/package/Python tests retain their exact mismatch checks and never invoke the producer. Existing schema controls remain in the shared checker. A new portable harness runs that same complete16-case boundary on baseline WASM, alongside the unchanged optimized packaged entry.

Validation: source/question/answer metadata cmp exit0; repeated explicit refresh cmp exit0; native and actual portable WASM complete16-case checks passed (0.01s/0.08s execution); strict affected all-target Clippy and complete helper hygiene passed; normal commit/push hooks passed. All raw logs and whole before/after vector bytes are retained. Only source fixture/proof/report data and the explicit producer/portable door changed; no runtime decision, governor, dependency, feature or profile is weakened. No GMEOW corpus/factory/optimized producer/full gate, host/resource probes, caps or banned commands were used. Unrelated native root REWORK_LIST.md and the separate GMEOW1843 worktree remain preserved.

Review is explicitly self-review; independent dispatch was unavailable. Complete submitted reviews, inline comments and review threads contain no finding. The bot processing notice is pending review, not approval. MSRV, four CodeQL lanes and python-publisher passed; broader hosted jobs remain pending. Integration follows the existing instruction to ghprsq now and own subsequent CI findings. Keep588 open until the actual repaired optimized-package/Python acceptance passes; a portable-debug run is not that separate claim.

Authoritative plan and selected Stage: .stage/refresh-shared-cross-host-proof-goldens/plan.md and .stage/refresh-shared-cross-host-proof-goldens. They retain full failed-job logs, exact source/answer preservation, vector diff/digest inventories, native/WASM/fixed-point/Clippy/helper/hook logs, source self-review/completion audit and complete review evidence. The full logic goal and GMEOW1843/source-leaf/shared-store/delta work remain active.

## Commits squashed (1)

- `378398ca7` Refresh shared DL proof boundary vectors for current support

## Files

```
Makefile                                           |   3 +
 crates/validate/Cargo.toml                         |   4 +
 .../validate/examples/refresh_dl_proof_vectors.rs  |  90 +++++
 crates/validate/src/regime.rs                      |   5 +
 crates/validate/src/regime/schema_vectors.rs       |  49 ---
 crates/validate/tests/dl_proof_vectors.rs          |  12 +
 crates/validate/tests/fixtures/dl-proof.vectors    | 376 +++++++++++----------
 docs/WASM_TESTING.md                               |   5 +-
 8 files changed, 311 insertions(+), 233 deletions(-)
```

## Conflicts

None. Fetched main has no additional base delta. Clean merge-tree candidate: a16a2ef5702234b01eae4ae717e8e67ca198e1b0.

---

Defect-Class: proof-artifact-drift

