# PR review evidence

Source payload: review-evidence.json
Observed items: 1; PR head: 0461e3d460e34c619f37928dc0f2725f34bb49fc.
Every item needs an explicit disposition; bot comments count.
Dispositions apply to the recorded item revision; edited feedback is reconsidered.

Coverage:
- comments: complete
- reviews: complete
- inline_comments: complete
- threads: complete

## github:comment:IC_kwDOTKq-Ms8AAAABfC8qyA — coderabbitai (comment)
https://github.com/Blackcat-Informatics/purrdf/pull/693#issuecomment-6378433224
state=None resolved=None path=None







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
- **Layers**: Organization UI
- **Review profile**: CHILL
- **Plan**: Advanced
- **Run ID**: `8b54bd71-39d6-46a9-87e9-98708f47f29f`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between e3eb29225d3fda035613345fb90e0a047e85f45e and 0461e3d460e34c619f37928dc0f2725f34bb49fc.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 3</strong></summary>
<dl>
<dd>

* `crates/shapes/src/json_schema.rs`
* `crates/shapes/tests/fixtures/iri-only-surface.golden.txt`
* `crates/shapes/tests/ontology_schema_surface.rs`

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
  * OpenAPI 3.1 schemas now express local schema references through `components/schemas`, with the referenced definitions included in the generated document.
  * References in schema examples and values—such as defaults, constants, and enumerations—remain unchanged.
  * Recursive references and references with escaped pointer tokens are preserved, keeping schema targets and validation constraints intact.


OpenAPI generation now relocates schema-position references from `#/$defs/` to `#/components/schemas/` and moves definitions into `components.schemas`. Tests and golden output check reference resolution and schema equivalence while preserving JSON Schema references.

**Changes: OpenAPI local references**

|Layer / File(s)|Summary|
|:---|:---|
|**Reference relocation and OpenAPI construction** <br> `crates/shapes/src/json_schema.rs`|`openapi_doc` takes ownership of definitions, rewrites local references in schema positions, and moves definitions into `components.schemas`. The policy salt identifies the new OpenAPI reference policy.|
|**Reference validation and schema surface updates** <br> `crates/shapes/src/json_schema.rs`, `crates/shapes/tests/fixtures/iri-only-surface.golden.txt`, `crates/shapes/tests/ontology_schema_surface.rs`|Tests check local-reference closure, schema equivalence, recursive and escaped references, and preservation of `$ref`-shaped instance data. The golden output and surface comparison accept the changed OpenAPI reference locations. JSON Schema references remain unchanged.|


**Priority:** ➖ Normal

**Estimated code review effort:** 3 (Moderate) | ~20 minutes


**Change:** Bug fix · **Severity of issue fixed:** Medium


**Possibly related issues**
- The dangling OpenAPI references issue describes the unresolved `#/$defs/` targets addressed by this change.

**Possibly related PRs**
- [Blackcat-Informatics/purrdf#691](https://github.com/Blackcat-Informatics/purrdf/pull/691): Establishes the shared definition catalog that this change relocates into OpenAPI components.

</dd>
</dl>
</details>


<details>
<summary><strong>Merge Risk:</strong> <strong>⚪ Minimal</strong> · up to <code>0461e</code></summary>
<dl>
<dd>


<!-- final_review_risk_label
**Merge Risk:** **⚪ Minimal** · up to `0461e`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"0461e3d460e34c619f37928dc0f2725f34bb49fc","coveredCommitId":"0461e3d460e34c619f37928dc0f2725f34bb49fc","kind":"reviewed"} -->

The reference relocation has no identified issue requiring a fix before merge. Normal checks remain appropriate.


</dd>
</dl>
</details>

<details>
<summary><strong>Security architecture risk:</strong> <strong>⚪ Minimal</strong> · up to <code>0461e</code></summary>
<dl>
<dd>




This corrects document-local references without adding external access or authority. The inspected changes preserve literal data and existing validation constraints. No material security risk was found in the change.

**Retained concerns**
No architecture-level concerns identified.

<details>
<summary><strong>Security review details</strong></summary>
<dl>
<dd>

**Security Blast Radius**
- _observed_ — The changed builder operates on generated document values. It replaces a local fragment prefix without fetching targets, evaluating schemas, accessing credentials, or introducing network operations. The existing path-parameter and response schemas remain static and contain no references.

**Trust Boundaries and Controls**
- _observed_ — The traversal distinguishes schema structure from literal instance data using recognized schema keywords. A reference-shaped value inside literal data is not treated as an instruction, and pointer suffixes are not decoded or reinterpreted during relocation.

**Resilience and Maintainability Implications**
- _inferred_ — The relocation transition is isolated to an owned map, with JSON Schema copied beforehand and the completed OpenAPI value returned afterward. No shared catalog is incrementally mutated. Repetition does not reapply the prefix change, and recursive reference edges cannot cause traversal expansion because only the physical JSON tree is walked. These properties contain interruption and concurrency effects within this transformation; external persistence recovery was not assessed.

</dd>
</dl>
</details>


</dd>
</dl>
</details>


<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 4 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks: 1 warning</strong>

|     Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                         |
| :---------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 53.33% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 15 functions across 2 files. (1 skipped: … | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                                  |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly identifies the main change: resolving OpenAPI references to shared schema definitions.                                                                                                     |
|      Description check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The description directly explains the OpenAPI reference relocation, preserved behavior, policy update, tests, and validation results.                                                                        |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Issue `#692` (direct, open) requires resolvable local references in the OpenAPI output, a native closure regression, unchanged JSON Schema `#/$defs/...` references, equivalent constraints, proportional …    |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | All changed hunks connect to `#692` or its acceptance criteria:<br>- crates/shapes/src/json_schema.rs: the `openapi_doc` relocation walk and ownership change implement the reference fix. The call-site chan… |

<details>
<summary><strong>Full details: Linked Issues check</strong></summary>
<dl>
<dd>

**Explanation**

Issue `#692` (direct, open) requires resolvable local references in the OpenAPI output, a native closure regression, unchanged JSON Schema `#/$defs/...` references, equivalent constraints, proportional work, and regenerated goldens.

Evidence at head 0461e3d4 versus merge base e3eb2922:
- Reference relocation: `openapi_doc(defs)` in crates/shapes/src/json_schema.rs now takes ownership of the definitions map. It walks only the schema positions named by `schema_map_keywords`, `schema_array_keywords` and `schema_single_keywords` (the same lists the catalog validator in schema_catalog.rs uses). It rewrites `#/$defs/` to `#/components/schemas/` in `$ref` strings. Instance data under `const`, `enum`, `default` and `examples` is not visited. Escaped suffix tokens are kept as written. Annotation, Literal, class branches and vocabulary enums are all in the catalog, so they are covered.
- Work and allocation: the walk mutates in place with `WorkList`, and the catalog is moved into `components/schemas` rather than cloned. Both call sites (`compile_with_value_vocab`, `compile_with_surface`) now pass `defs` by value.
- JSON Schema surface: the `root_schema` output path is unchanged, so `#/$defs/...` references stay in the JSON Schema artifact.
- Tests: `assert_local_reference_closure` walks the whole OpenAPI document and checks that each local target resolves. `assert_surface_equivalence` checks that the two surfaces match except for the reference base. `openapi_relocates_recursive_escaped_refs_without_rewriting_instance_data` covers escaped keys (`a~1b~0c`), recursive edges and literal instance data. The closure and equivalence checks run in the compile helpers (`compile_vocab`, `compile_with_value_vocab`) and in the Person and carrier tests.
- Goldens: the diff stat for crates/shapes/tests/fixtures/iri-only-surface.golden.txt is +36/-36, matching the 36 OpenAPI reference prefixes stated in the PR. The ontology test accepts only openapi `/$ref` changes whose target prefix changes from `#/$defs/` to `#/components/schemas/` with the same suffix.
- Policy identity: `SCHEMA_POLICY_SALT` now contains `openapi-3.1-local-refs-v1`, so cached output is regenerated.

Limits of this assessment: I did not run the test suite, Clippy or commit hooks. The reported results are author claims and are not independently verified here. The golden-file byte count is taken from the diff stat.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

All changed hunks connect to `#692` or its acceptance criteria:
- crates/shapes/src/json_schema.rs: the `openapi_doc` relocation walk and ownership change implement the reference fix. The call-site changes pass `defs` by value to support it. The salt string change makes cached output regenerate, as the issue requires for regenerated goldens. The new unit tests and helper functions are the closure, escaped-key, recursion and equivalence regressions the issue asks for.
- crates/shapes/tests/fixtures/iri-only-surface.golden.txt: the OpenAPI reference-prefix change is the regenerated golden artifact the issue requires.
- crates/shapes/tests/ontology_schema_surface.rs: the renamed golden-difference test and its narrowed allowance for OpenAPI reference-target moves verify that the golden change is limited to those prefixes.

No changed hunk is unrelated to the issue. The diff does not alter the public declarations listed in the summary, and the shared catalog and JSON Schema behavior are preserved by the tests.

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

Docstring coverage is 53.33% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 15 functions across 2 files. (1 skipped: 1 unsupported.)

**Resolution**

Write docstrings for the functions missing them to satisfy the coverage threshold.

</dd>
</dl>
</details>

</dd>
</dl>
</details>







