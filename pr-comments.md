# PR #567 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/567?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

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
- **Run ID**: `ca761d21-f4bf-4c89-b405-71be1b6a1661`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 03902c2f586fba5680db14c7d9c5abb2e7fd040f and af8936e6f22c70dcbc3f3dc1224be887aba51cc5.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 1</strong></summary>
<dl>
<dd>

* `helpers-ledger.toml`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 1 included review remains after this review. Your included PR review attempts over the past 7 days set your current allowance at 3 reviews per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The public graph projection API changes from `project_named_graph(&str)` to `project_graph(Option<&RdfTerm>)`. It selects the default graph or an exact IRI or blank-node graph, rejects invalid graph-name terms, and updates repository callers.

**Changes: Graph projection**

|Layer / File(s)|Summary|
|:---|:---|
|**Projection API and coverage** <br> `crates/rdf-core/src/ir/dataset.rs`, `helpers-ledger.toml`|`project_graph` selects rows across quads, reifier declarations, and annotations, then clears the selected graph slots. Tests cover default, IRI, blank-node, missing, declared-empty, and invalid selections. Capacity-proof records use updated source fingerprints.|
|**Canonicalization and digest integration** <br> `crates/rdf-core/src/ir/canon.rs`, `crates/rdf-core/src/ir/pipeline_bundle.rs`|Canonicalization references and `flat_graph_digest` use `project_graph` with an IRI term.|
|**Cross-crate test and benchmark callers** <br> `crates/rdf-core/tests/shared_views.rs`, `crates/rdf/tests/content_id_byte_determinism.rs`, `crates/shapes/benches/shared_views.rs`, `crates/shapes/tests/named_graph_view.rs`|Test oracles and named-graph benchmarks replace the string-based projection call with `project_graph` and an IRI term.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal

**Estimated code review effort:** 3 (Moderate) | ~20 minutes

<!-- change_assessment_commit:"af8936e6f22c70dcbc3f3dc1224be887aba51cc5" -->
**Change:** Feature · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

**Possibly related PRs**

- [Blackcat-Informatics/purrdf#413](https://github.com/Blackcat-Informatics/purrdf/pull/413): Establishes named-graph validation and declared-empty graph behavior used by this projection change.

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>af893</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `af893`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"af8936e6f22c70dcbc3f3dc1224be887aba51cc5","coveredCommitId":"af8936e6f22c70dcbc3f3dc1224be887aba51cc5","kind":"reviewed"} -->

The in-repository projection API and callers appear consistent, with no concrete merge-blocking failure established. The separately requested GMEOW benchmark integration is not present in this checkout; verify it in its owning repository before relying on that benchmark.
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

|      Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                                                                                                                                                                                   |
| :----------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Linked Issues check | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Issue `#566` requires one native `RdfDataset::project_graph` operation for default, IRI-named, and blank-named graphs. The supplied PR evidence shows that API, graph-slot selection across quads, reifier… | Update the GMEOW benchmark required by `#566`. Assemble the production world first. Call `project_graph(None)` on that assembled dataset. Remove the authored-default shortcut. Provide reviewable implementation and automated test evidence. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: replacing separate named-graph projection behavior with one API for default and named RDF graphs.                                              |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The reported changes replace the named-only projection API, migrate repository callers, and add or update projection, canonicalization, determinism, shared-view, and SHACL validation controls. These c… |
|     Docstring Coverage     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Docstring coverage is 94.74% which is sufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 19 functions across 7 files. (1 skipped: 1 … |

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The reported changes replace the named-only projection API, migrate repository callers, and add or update projection, canonicalization, determinism, shared-view, and SHACL validation controls. These changes support the coding requirements in `#566`. The helper-ledger fingerprint updates support the changed source records. No unrelated change is demonstrated.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 94.74% which is sufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 19 functions across 7 files. (1 skipped: 1 unsupported.)

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

Issue `#566` requires one native `RdfDataset::project_graph` operation for default, IRI-named, and blank-named graphs. The supplied PR evidence shows that API, graph-slot selection across quads, reifiers, and annotations, invalid-selector errors, empty selections, caller migration, and focused controls. The evidence does not show the required GMEOW benchmark integration. The benchmark must select the default graph after complete production assembly and remove the authored-default shortcut.

**Resolution**

Update the GMEOW benchmark required by `#566`. Assemble the production world first. Call `project_graph(None)` on that assembled dataset. Remove the authored-default shortcut. Provide reviewable implementation and automated test evidence.

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

The workspace CI failure was stale source receipts in the logical-capacity
ledger after the graph API and its tests changed the pinned kernel files.
Commit `af8936e6f22c70dcbc3f3dc1224be887aba51cc5` renews the dataset authority
identity, constructor-closure identity and the moved permutation sink address.
No proof class, sink identity, reason, authority set, limit or checker was changed.

The complete `make helpers-hygiene` now passes: all 82 enforced ledger jobs,
118 distinct-body rows and all 133 registered hash domains. The initial local
attempt correctly failed because it used the function declaration's line
instead of the observed sink line; that was corrected before qualification.
The original 1459 passing kernel and consumer tests remain applicable because
this repair changes only ledger source receipts. Normal signed commit hooks
and the normal push succeeded. Hosted checks are refreshing on the new head.

The linked-issue review warning about reviewable downstream integration is still
being addressed. The GMEOW worktree deletes the authored-default shortcut,
projects every selected world from the complete object EDB and migrates its
named callers. Its consumer type-check passed against the new native API.
The dependency pin now includes this receipt repair; current strict Clippy and
the explicit optimized default-world benchmark are running before publication.
I will add the actual dependent commit/PR and execution result here; no full
canonical admission or complete engine delegation is claimed.

## 3. paudley — 

The downstream source is now reviewable in
[GMEOW PR 1837](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1837),
commit `7db11af84`.

Its [recorded consumer](https://github.com/Blackcat-Informatics/gmeow-ontology/blob/7db11af84/crates/pipeline/examples/recorded-class-consistency.rs)
unconditionally calls `assemble_object_level_edb` before selecting a world.
Selecting `default` produces `None`, which is passed to the native
`project_graph` operation. The authored-default helper and shortcut are deleted.
The full EDB is released after selection and before native reasoning; its
certificate and question-bound proof replay remain native. All named graph
callers and the canonical pipeline description are migrated as well.

The consumer type-check passed against the new API. The final dependency pin is
`af8936e6f22c70dcbc3f3dc1224be887aba51cc5`, including the reviewed source-receipt
renewal. Normal signed commit hooks and the normal push passed. Strict workspace
Clippy and the explicit optimized default-world operation are still running on
that final pin. No current default-world execution result or complete canonical
admission is claimed before that operation finishes.

This provides the concrete source evidence requested by the linked-issue
warning. The dependent GMEOW PR will be merged after this native PR; its complete
engine responsibility issue remains open.

