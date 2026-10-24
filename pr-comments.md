# PR review evidence

Source payload: review-evidence.json
Observed items: 1; PR head: de2095b46b4e69fa4e95099575fad89e1e3a831a.
Every item needs an explicit disposition; bot comments count.
Dispositions apply to the recorded item revision; edited feedback is reconsidered.

Coverage:
- comments: complete
- reviews: complete
- inline_comments: complete
- threads: complete

## github:comment:IC_kwDOTKq-Ms8AAAABfDM_UA — coderabbitai (comment)
https://github.com/Blackcat-Informatics/purrdf/pull/695#issuecomment-6378700624
state=None resolved=None path=None







No actionable comments were generated in the recent review.

<details>
<summary><strong><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/info.svg" width="16" height="16" align="absmiddle" alt=""></picture> Recent review info</strong></summary>
<dl>
<dd>

<details>
<summary><strong><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/cog-dark.svg"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/cog.svg" width="16" height="16" align="absmiddle" alt=""></picture> Run configuration</strong></summary>
<dl>
<dd>

- **Source mode**: Auto
- **Repository**: Unavailable
- **Branch**: Unavailable
- **File**: No configuration file from this repository applied
- **Commit**: Unavailable
- **Layers**: Organization UI
- **Review profile**: CHILL
- **Plan**: Advanced
- **Run ID**: `4b710e46-d91f-48b4-ad9b-2a3ef4a4f6e1`

</dd>
</dl>
</details>

<details>
<summary><strong><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/git-commit-horizontal-dark.svg"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/git-commit-horizontal.svg" width="16" height="16" align="absmiddle" alt=""></picture> Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between cd42aa245d7bf8d34e588dc83f573dee504fc2fc and de2095b46b4e69fa4e95099575fad89e1e3a831a.

</dd>
</dl>
</details>

<details>
<summary><strong><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/file-plus-corner-dark.svg"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/file-plus-corner.svg" width="16" height="16" align="absmiddle" alt=""></picture> Files selected for processing: 3</strong></summary>
<dl>
<dd>

* `crates/shapes/src/constraints.rs`
* `crates/shapes/src/plan.rs`
* `crates/shapes/tests/catalog_references.rs`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 8 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 10 reviews per hour.

</dd>
</dl>
</details>








<details>
<summary><strong>📝 Summary</strong></summary>
<dl>
<dd>



## Summary by CodeRabbit

* **Bug Fixes**
  * Validation results now correctly reflect each shape’s definition, even when different shapes have the same identifier.
  * Results remain accurate after the original parsed data is released, and combined-shape checks behave consistently regardless of member order.
  * Self-referencing shapes now resolve to their prepared definitions in supported cases, improving consistency when evaluating recursive shape references.


The conformance memo key now includes model-domain and live shape-definition identity. Shape-plan self-reference resolution checks the plan owner. Regression tests cover equal-named shapes with distinct constraints or local deactivation.

**Changes: SHACL memoization identity**

|Layer / File(s)|Summary|
|:---|:---|
|**Memo identity and definition resolution** <br> `crates/shapes/src/constraints.rs`, `crates/shapes/src/plan.rs`|Memoization keys include the interned focus node, recursion depth, model-domain identity, and live shape-definition identity. Shape-plan self-reference resolution checks the plan owner.|
|**Identity regression coverage** <br> `crates/shapes/tests/catalog_references.rs`|Tests compare conformance and validation for equal-named shapes with distinct constraints or local deactivation. They cover `And`, `Or`, and `Xone` member orderings, owner contexts, focus nodes, and dropped parse models.|


<strong>Priority: <picture><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/priority-normal-16.svg" width="16" height="16" align="absmiddle" alt=""></picture> Normal</strong>

**Estimated code review effort:** 3 (Moderate) | ~20 minutes


**Change:** Bug fix


**Possibly related issues**
- Blackcat-Informatics/purrdf#690: This change addresses one memoization identity defect in the SHACL identity-catalog integration. The issue also tracks other caller, recursion, product, and host requirements.

**Possibly related PRs**
- [Blackcat-Informatics/purrdf#691](https://github.com/Blackcat-Informatics/purrdf/pull/691): That PR retains original SHACL definitions in model-owner domains. This change uses model-domain and definition identity in memoization.

</dd>
</dl>
</details>


<details>
<summary><strong>Merge Risk:</strong> <strong><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/risk-unassessed-16.svg" width="16" height="16" align="absmiddle" alt=""></picture> Minimal</strong> · up to <code>de209</code></summary>
<dl>
<dd>


<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `de209`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"de2095b46b4e69fa4e95099575fad89e1e3a831a","coveredCommitId":"de2095b46b4e69fa4e95099575fad89e1e3a831a","kind":"reviewed"} -->

No merge-blocking issue was identified; complete the reported hosted checks before merging.


</dd>
</dl>
</details>

<details>
<summary><strong>Security architecture risk:</strong> <strong><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/review-comment-icons-v1/risk-unassessed-16.svg" width="16" height="16" align="absmiddle" alt=""></picture> Minimal</strong> · up to <code>de209</code></summary>
<dl>
<dd>




The change keeps validation answers tied to their original definitions while preserving existing evaluation controls. No material security regression was identified in the changed behavior.

**Retained concerns**
No architecture-level concerns identified.

<details>
<summary><strong>Security review details</strong></summary>
<dl>
<dd>

**Security Blast Radius**
- _inferred_ — The demonstrated exposure is validation-answer and report correctness for supplied shapes and focuses within an evaluation. Equal identifiers alone no longer permit answer reuse across different original models or live bodies. No additional service, credential, tenant authority, or privileged sink is introduced by these changes.

**Trust Boundaries and Controls**
- _observed_ — Original ownership remains explicit: nested plans prefer a shape's own owner and otherwise inherit the existing ambient owner. Definition resolution follows the prepared reference only for an owner-bearing sole self-reference without property shapes; it does not discover an unrelated model from the identifier alone.

**Resilience and Maintainability Implications**
- _inferred_ — Borrowed immutable plans and retained model definitions keep address identities live until the run-local memo is discarded. Repeated public calls do not inherit cached answers, failed traversals do not insert partial verdicts, and recursion depth remains part of the key. These properties preserve failure containment despite removing lifetime parameters from the memo types.

</dd>
</dl>
</details>


</dd>
</dl>
</details>


<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 3 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 2</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks: 2 warnings</strong>

|      Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                          |
| :----------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------- |
| Linked Issues check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | The production fix meets the main coding requirements in `#694`. `MemoKey` now includes the interned focus, depth, original model domain, and evaluated definition identity. `ShapePlan::definition` resol… | Run the required hosted checks and final review against head de2095b46b4e69fa4e95099575fad89e1e3a831a. Resolve any failures before completing `#694`. |
|  Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 75.00% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 8 functions across 3 files.                | Write docstrings for the functions missing them to satisfy the coverage threshold.                                                                  |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 3 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title "Bind SHACL conformance answers to original definitions" matches the main change. The PR keys the conformance memo by the evaluated definition or body and the original model scope, so answer… |
|      Description check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The description states the problem: shapes sharing a declaration IRI reused a conformance answer, which produced wrong conjunctions. It names the fix (keying the memo by evaluated definition/body and … |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changes stay within `#694`. Production changes update conformance memo identity and context-bound definition resolution. The added tests exercise equal declarations, local deactivation, logical cons… |

<details>
<summary><strong>Full details: Title check</strong></summary>
<dl>
<dd>

**Explanation**

The title "Bind SHACL conformance answers to original definitions" matches the main change. The PR keys the conformance memo by the evaluated definition or body and the original model scope, so answers no longer get reused across different original definitions. The summary confirms the memo key change in crates/shapes/src/constraints.rs and the plan.rs resolver change.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Description check</strong></summary>
<dl>
<dd>

**Explanation**

The description states the problem: shapes sharing a declaration IRI reused a conformance answer, which produced wrong conjunctions. It names the fix (keying the memo by evaluated definition/body and original model scope), the context-bound reference resolution through the plan resolver, the regression tests, and the validation run. This matches the raw summary, which covers constraints.rs, plan.rs, and catalog_references.rs tests.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The changes stay within `#694`. Production changes update conformance memo identity and context-bound definition resolution. The added tests exercise equal declarations, local deactivation, logical constraints, foreign focuses, and report behavior. No unrelated evaluator, public API, wire-format, or semantic feature change is shown.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Linked Issues check</strong></summary>
<dl>
<dd>

**Explanation**

The production fix meets the main coding requirements in `#694`. `MemoKey` now includes the interned focus, depth, original model domain, and evaluated definition identity. `ShapePlan::definition` resolves context-bound references. Local deactivation remains handle-local. The regression covers named and blank declarations, equal names, both logical-member orders, owner lifetime after factory drop, foreign focuses, and report fields. The PR description reports focused native and strict Clippy validation. However, `#694` also requires hosted checks and review for the final head. The current description states that these checks must still cover the final head, but it does not report their completion.

**Resolution**

Run the required hosted checks and final review against head de2095b46b4e69fa4e95099575fad89e1e3a831a. Resolve any failures before completing `#694`.

</dd>
</dl>
</details>

</dd>
</dl>
</details>







