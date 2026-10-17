# PR #593 — comments and review threads

5 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. paudley — 

# Repair imported-closure current-support replay

Preserve the entire #591 contract. Source authority is the full issue, its zero
comments, current AGENTS.md and the actual CLI/native implementation. There are
no applicable repository baseline/goals/constitution/ADR files. Use standard
host tools and profiles, zero subagents, normal hooks and ghprsq only for merge.
Owner assessments are not independent approval.

## Contract and evidence

| Requirement | Task | Evidence |
|---|---|---|
| Exact tiny CLI failure | 1 | Run only consistency_decides_the_imports_closure in entails_cli; retain its actual exit and diagnostic |
| Owning native correction | 1–2 | Inspect record_clash, current journal, Hyper::replay_support and replay_clash_assuming; identify the differing frame/fact computation before repair |
| Imports and local-clash authority | 2 | Genuine resolved-closure proof checks against its consumer; unrelated/missing source or relabelled inhabitant is rejected |
| Exact premises and tamper refusal | 2 | Existing head/frame/predecessor, branch and witness controls remain meaningful and pass; no skipped comparison or search fallback |
| Native/public/CLI execution | 2 | Tiny owning-home control, public producer/encode/decode/check path and unchanged actual CLI assertion pass |
| Portable behavior | 2 | Applicable existing Rust portable harness executes the demonstrated regression, with registrations/counts updated if a case is added |
| Strict and mandatory checks | 2–3 | Affected library/test Clippy before optimized regression linking on subsequent changes; helper/metadata where changed, normal hooks and final hosted qualification |
| Real delivery | 3 | Read all PR feedback, resolve material findings, ghprsq merge, verify issue closure/archive/mirror and clean the completed delivery |

## Task 1: Reproduce and inspect the exact mismatch

The existing CLI fixture is already minimal and drives the built binary. Keep
its assertion and inputs unchanged. Retain the failed reproduction, then inspect
the producer and consumer frame/fact values at the comparison if necessary.
Temporary computation probes are permitted by repository diagnosis policy and
must be removed before qualification/commit. Do not infer the cause from the
generic malformed-proof string or rerun an unchanged failure hoping it passes.

## Task 2: Correct the native owner and prove the boundaries

Repair the demonstrated journal/source-identity computation through its existing
production callers. Keep exact input/calculus binding, named/scoped-blank source
identity, ordinary/prepared clash authority and all inhabitant/premise checks.
Preserve legitimate public source/export/check contracts; do not mask a single
CLI case with a separate weaker checker. If correcting a source boundary changes
exported bytes or context requirements, explain and qualify that actual contract.
Use the existing graph/replay homes; add no equality engine, semantic fallback,
runtime dependency or new fixture factory. Add the smallest regression and
negative neighbours that distinguish the actual failure. Run affected strict
test compilation first, then focused native/public/CLI and portable executions.
Retain valid unchanged evidence, and update only applicable registrations.
Commit/push the coherent correction with normal hooks and post actual evidence.

## Task 3: Publish, review, integrate and close

Assess every row against current source and executed evidence. Publish the
complete PR with Closes #591, inspect all reviews/threads and actual hosted
checks, and resolve material findings. Use the Stage merge workflow and ghprsq,
archive the selected Stage, verify integration and issue closure, perform normal
mirror readback and remove the owned delivery. No tests, commits or PR alone
count as closure. Keep the broader REWORK_LIST.md goal and other owners intact.

## Deliberately excluded approaches

Do not weaken the CLI/current-support assertion, replace a valid-source check
with producer data, or run saturation during replay. No full GMEOW corpus,
optimized external producer, full local gate, resource probe or timing-based
performance qualification. A new wire format or algorithm is not presumed
necessary; change one only if the demonstrated computation requires it and
then qualify its affected native/portable consumers.

## 2. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/593?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 36 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary><strong>View limit details</strong></summary>
> <dl>
> <dd>
> 
> **Limit details:** You’ve used the included review currently available. Your 91 included PR review attempts over the past 7 days set your current allowance at 1 review per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `66073219-2e0e-4a4d-9784-6e52bd612e72`
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
> Reviewing files that changed from the base of the PR and between c86272091f5b7a12b9603883724564fe87ee091c and dc569c87436de06a5bf8bd94b4569732c539b3e4.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing: 1</strong></summary>
> <dl>
> <dd>
> 
> * `crates/validate/tests/dl_proof_vectors.rs`
> 
> </dd>
> </dl>
> </details>
> 
> </dd>
> </dl>
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

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
- **Run ID**: `f662dfd4-303a-4dd1-b77d-181d15067b3d`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 0c8ceac5259fbfe2aa256b59b8c9e5b6b1db322e and c86272091f5b7a12b9603883724564fe87ee091c.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 5</strong></summary>
<dl>
<dd>

* `Makefile`
* `crates/cli/src/consistency.rs`
* `crates/validate/src/regime.rs`
* `crates/validate/tests/dl_proof_vectors.rs`
* `docs/WASM_TESTING.md`

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

The change adds proof checking against a `ReasonerSession`’s dataset and resolved-import state. CLI consistency checks use the active session. Native and WASM test coverage now includes proofs for imported closures and rejects altered proofs, answers, and sources.

**Changes: Imported-closure proof replay**

|Layer / File(s)|Summary|
|:---|:---|
|**Session-based proof checking** <br> `crates/validate/src/regime.rs`|`ReasonerSession::check_proof` checks proofs against the session dataset and resolved-import state. The standalone checker continues to parse the supplied document and uses unresolved-import state.|
|**CLI consistency proof checks** <br> `crates/cli/src/consistency.rs`|The CLI passes the active session, consistency question, answer, certificate, and supplied proof to the session checker.|
|**Imported-closure regression coverage** <br> `crates/validate/tests/dl_proof_vectors.rs`, `Makefile`, `docs/WASM_TESTING.md`|The native test checks imported-closure proofs across three premise variants and rejects altered answers, proof documents, and imported schemas. The WASM test selection and documented totals include the new test.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal



**Estimated code review effort:** 3 (Moderate) | ~20 minutes

<!-- change_assessment_commit:"c86272091f5b7a12b9603883724564fe87ee091c" -->
**Change:** Bug fix · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>c8627</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `c8627`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"c86272091f5b7a12b9603883724564fe87ee091c","coveredCommitId":"c86272091f5b7a12b9603883724564fe87ee091c","kind":"reviewed"} -->

The imported-closure proof check uses the session’s resolved source. No actionable merge-blocking risk remains in the reviewed paths after normal checks.
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
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 77.78% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 9 functions across 3 files. (2 skipped: 2… | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: correcting proof replay to use the imported closure's source identity.                                                                         |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changes satisfy the coding requirements in `#591`. `crates/cli/src/consistency.rs` checks the supplied proof through `ReasonerSession::check_proof`. `crates/validate/src/regime.rs` builds a fresh ch… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changed files remain within `#591`. The session API and shared verifier implement source-snapshot checking. The native and scalar-WASM test registration provides focused regression coverage. The doc… |

<details>
<summary><strong>Full details: Linked Issues check</strong></summary>
<dl>
<dd>

**Explanation**

The changes satisfy the coding requirements in `#591`. `crates/cli/src/consistency.rs` checks the supplied proof through `ReasonerSession::check_proof`. `crates/validate/src/regime.rs` builds a fresh checker from the consumer session dataset and preserves the shared proof, question, answer, certificate, inhabitant, premise, and tamper checks. The native regression covers independent imported-closure loads, IRI and blank-node subjects, derived membership, changed answers, changed proof data, and changed schemas. The existing CLI regression remains in place. The reported native, scalar-WASM, and strict Clippy validation covers the affected paths.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The changed files remain within `#591`. The session API and shared verifier implement source-snapshot checking. The native and scalar-WASM test registration provides focused regression coverage. The documentation and Makefile count changes record that coverage. No unrelated production behavior, export format, proof bytes, or runtime dependency changes are shown.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 77.78% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 9 functions across 3 files. (2 skipped: 2 unsupported.)

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

## 3. paudley — 

CodeRabbit completed review of c86272091 with no actionable code comments and
no merge-blocking risk in the reviewed paths. Its docstring coverage heuristic
warns at 77.78% for touched functions. I am declining that percentage-only
warning: the new public ReasonerSession::check_proof has API, source authority,
fresh-context and error documentation; the unchanged public document API keeps
its documentation. The remaining functions are private plumbing and test cases,
with source comments and explicit assertions. Repository strict compilation and
rustdoc pass. No repository rule requires the bot's blanket function-percentage
threshold, and adding boilerplate for test functions would not improve the
consumer contract. This is a declined heuristic warning, not a failed required CI
check or an unaddressed source finding.

Native/CLI/scalar-WASM and normal hooks are passing. Hosted qualification remains
pending; this disposition does not grant merge readiness or independent approval.

## 4. paudley — 

# CodeQL test-report output remediation

PR #593's CodeQL check 117491526067 reports two cleartext-output alerts at the
new public regression's failure messages (lines 73/74). Both explicitly dump the
entire checked report. The annotated paths name certificate parsing and trust
counts. Regardless of the fixture's public inputs, logging the entire report is
unnecessary to assert these fixed status lines.

Remove only the two report-format arguments. Preserve both conditions, all
negative controls and the complete production implementation. Run affected
test-target strict compilation and the same two native/scalar-WASM harness cases;
then normal hooks and publication. Reuse unchanged production/CLI/native-support
evidence with its recorded scope. Refresh CodeQL, hosted CI and review debt before
ghprsq. No alert dismissal, suppression, waived assertion or scope cut is used.

This is owner remediation, not an independent approval. The prior completed
CodeRabbit source review remains attributable to c86272091; this delta changes
only two test failure messages and still needs passing hosted qualification.

## 5. paudley — 

Published dc569c874 through normal hooks and push. The only source delta removes
the complete report from two assertion failure messages; both conditions and
all production code are unchanged. Affected strict test compilation, both native
cases and both exact scalar-WASM cases pass (no filtered cases).

CodeQL's two cleartext-output annotations are addressed in source, not dismissed
or suppressed. Refreshed hosted CodeQL must still establish their resolution.
Unchanged CLI, current-support, production strict/helper and CodeRabbit source
evidence retain their scope; none is relabelled as a fresh whole-suite execution.
No full local gate or unrelated suite was repeated.

