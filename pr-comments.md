# PR #506 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/506?cs_source=review_comment"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=2" alt="Review in Change Stack →" width="220" height="32"></a>

<!-- review_stack_entry_end -->
<!-- walkthrough_start -->

<details>
<summary>📝 Walkthrough</summary>

## Walkthrough

The pull request replaces the exact tower’s base-1e9 integer arithmetic with binary `u64` magnitude storage and adds bounded scratch allocation. It updates exact-operation cost estimates, delegates large geometry integer operations to the shared tower, and advances the SPARQL Governor profile to version 14.

### Changes

**Shared exact-integer engine**

|Layer / File(s)|Summary|
|:---|:---|
|**Magnitude storage and bounded scratch** <br> `crates/xsd/src/bigint/storage.rs`, `crates/xsd/src/bigint/scratch.rs`|Adds inline and heap-backed magnitude storage, plus bounded scratch buffers with capacity and exhaustion errors. Tests cover allocation behavior, recycling, values outliving the scratch handle, and concurrent drops.|
|**BigInt arithmetic and conversion** <br> `crates/xsd/src/bigint/arith.rs`, `crates/xsd/src/bigint/compat.rs`, `crates/xsd/src/bigint/arithmetic_tests.rs`, `crates/xsd/src/bigint/tower_tests.rs`|Removes the former arithmetic module and adds arithmetic, parsing, formatting, division, powers, and float-conversion adapters in `compat.rs`. Multiplication uses schoolbook, chunked, or Karatsuba paths. New tests exercise arithmetic and numeric conversions.|
|**Exact tower and binary-limb cost model** <br> `crates/xsd/src/exact/*`, `crates/xsd/tests/exact_tower.rs`, `crates/xsd/tests/exact_values.rs`|Updates integer, decimal, rational, and binary conversion logic for binary magnitudes. Cost estimates account for binary limbs and decimal base conversion. Tests check cost bounds against result sizes.|
|**Workspace integration and Governor profile** <br> `crates/geo/src/exact.rs`, `crates/sparql-eval/src/governor/mod.rs`, `crates/xsd/benches/exact.rs`, `docs/SPARQL-GOVERNOR-PROFILE.md`, `helpers-ledger.toml`, `CHANGELOG.md`|Geometry delegates large-value bit length, shifting, and square root operations to the exact integer tower. The Governor profile advances to version 14, with updated identity checks and binary-limb cost documentation. Supporting benchmark, ledger, and changelog text also changes.|

<!-- change_assessment_start -->
**Priority:** ⬆️ High



















**Estimated code review effort:** 4 (Complex) | ~45 minutes

<!-- change_assessment_commit:"a3eb972ad1313ff4b39fa080adb24ec00557ebe5" -->
**Change:** Refactor
<!-- change_assessment_end -->

</details>

<!-- walkthrough_end -->
<!-- final_review_risk_start -->
**Merge Risk:** **🔵 Low** · up to `a3eb9`
<!-- final_review_risk_coverage:{"sourceCommitId":"a3eb972ad1313ff4b39fa080adb24ec00557ebe5","coveredCommitId":"a3eb972ad1313ff4b39fa080adb24ec00557ebe5","kind":"reviewed"} -->

Exact arithmetic results appear preserved. The profile documentation still describes the old fuel unit in its normative section, and a digit-count guard adds avoidable conversion work to Decimal canonicalization. Both are bounded follow-ups. The reported multiplication slowdown and pending cross-target qualification also warrant owner awareness.
<!-- final_review_risk_end -->
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary>🚥 Pre-merge checks | ✅ 3 | ❌ 2</summary>

### ❌ Failed checks (2 warnings)

|      Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                                                                                        |
| :----------------- | :---------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Linked Issues check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Issue `#477` requires one binary arbitrary-precision engine, binary-limb profile v14, preserved exact behavior, and no small- or large-value benchmark regression. The reviewed changes add the shared bin… | Remove the Decimal multiplication regression and rerun the required small- and large-value comparisons. Complete the required hosted wasm, CLI, Python, C, and full-suite qualification, or provide equivalent current-head evidence for preserv… |
|  Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 78.43% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 153 functions across 16 files. (3 skipped… | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                                                                                                                |

<details>
<summary>✅ Passed checks (3 passed)</summary>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :-------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The reviewed changes stay within issue `#477`. The binary `BigInt` adapters, inline magnitude storage, bounded scratch storage, arithmetic and conversion changes, governor profile v14, exact-tower tests… |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: replacing separate exact-arithmetic representations with a shared binary u64 limb engine.                                                      |

</details>

<details>
<summary>Full details: Linked Issues check</summary>

**Explanation**

Issue `#477` requires one binary arbitrary-precision engine, binary-limb profile v14, preserved exact behavior, and no small- or large-value benchmark regression. The reviewed changes add the shared binary `u64` `BigInt`, three inline limbs, bounded `LimbScratch`, Karatsuba multiplication, numeric-tower adapters, profile v14 costing, and exact-tower coverage. However, the PR reports a 63.09% Decimal multiplication regression. This fails the issue's benchmark acceptance. Fresh hosted wasm, CLI, Python, C, and full-suite qualification also remains pending, so preservation on those surfaces is not established.

**Resolution**

Remove the Decimal multiplication regression and rerun the required small- and large-value comparisons. Complete the required hosted wasm, CLI, Python, C, and full-suite qualification, or provide equivalent current-head evidence for preserved results and refusal behavior.

</details>

<details>
<summary>Full details: Docstring Coverage</summary>

**Explanation**

Docstring coverage is 78.43% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 153 functions across 16 files. (3 skipped: 3 unsupported.)

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

## 2. paudley — 

The first review's benchmark concern is addressed by the qualified primitive-return repair44a6d3e93 and actual96826 matched18-case comparison. Every selected case improved or detected no regression; Decimal−2.44% has a wide quick interval−20.07..+37.91%, so we claim the accepted scoped comparison, not universal equivalence. The initial failures and all estimates are retained. Fresh final-head real wasm/CLI/Python/C/full-suite acceptance remains pending CI; no old-head pass is borrowed.

We decline the additional div_rem_pow10 guard optimization in this delivery. decimal_digits is an accounted existing boundary path; the review identifies avoidable conversion, not an incorrect numeric answer or current unmet scoped benchmark criterion. The accepted shared-engine/error/allocation/performance contract is qualified, and replacing an exact early-exit guard with a conservative bound would be a further implementation change requiring its own evidence. We do not pretend the rendering vanished or remove its charge.

We also decline mechanical docstring padding for the78.43% statistic. Public Rust documentation and numeric/governor contracts are covered by actual strict gates and source review; the percentage alone establishes no missing public invariant. No bot configuration, source lint or acceptance gate is weakened. Optional generated-docstring/test/autofix checkboxes require no additional tool or service action; the actual owning Rust controls and evidence are authoritative.

