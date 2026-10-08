# PR #462: SHACL dated profile selection and complete validation reports

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Comments (2)

### coderabbitai — 2026-10-06T08:20:11Z

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/462?cs_source=review_comment"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=2" alt="Review in Change Stack →" width="220" height="32"></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review in progress by coderabbit.ai -->

> [!NOTE]
> Currently processing new changes in this PR. This may take a few minutes, please wait...
> 
> <details>
> <summary>⚙️ Run configuration</summary>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `70b8f7a3-b688-44f0-963b-faa9f157fa71`
> 
> </details>
> 
> <details>
> <summary>📥 Commits</summary>
> 
> Reviewing files that changed from the base of the PR and between 5384882d65750ee22bddfeeac473095ab9c3db03 and dc8040ba6d2f0945c88f360a54db835aac798824.
> 
> </details>
> 
> <details>
> <summary>⛔ Files ignored due to path filters (1)</summary>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> </details>
> 
> <details>
> <summary>📒 Files selected for processing (231)</summary>
> 
> * `AGENTS.md`
> * `CHANGELOG.md`
> * `Cargo.toml`
> * `corpora/community/LICENSING.md`
> * `corpora/community/README.md`
> * `corpora/community/SPEC.md`
> * `corpora/community/catalog.json`
> * `corpora/community/licenses/Apache-2.0.txt`
> * `corpora/community/licenses/CC-BY-4.0.txt`
> * `corpora/community/licenses/MIT.txt`
> * `corpora/community/licenses/MulanPSL-2.0.txt`
> * `corpora/community/manifest.ttl`
> * `corpora/community/reviews/index.json`
> * `corpora/community/reviews/shacl.md`
> * `corpora/community/shacl/admission-hidden-prebound-subquery-draft/manifest.ttl`
> * `corpora/community/shacl/admission-hidden-prebound-subquery-draft/shapes.ttl`
> * `corpora/community/shacl/admission-hidden-prebound-subquery-rec/manifest.ttl`
> * `corpora/community/shacl/admission-hidden-prebound-subquery-rec/shapes.ttl`
> * `corpora/community/shacl/admission-local-values-draft/manifest.ttl`
> * `corpora/community/shacl/admission-local-values-draft/shapes.ttl`
> * `corpora/community/shacl/admission-local-values-rec/manifest.ttl`
> * `corpora/community/shacl/admission-local-values-rec/shapes.ttl`
> * `corpora/community/shacl/admission-minus-draft/manifest.ttl`
> * `corpora/community/shacl/admission-minus-draft/shapes.ttl`
> * `corpora/community/shacl/admission-minus-rec/manifest.ttl`
> * `corpora/community/shacl/admission-minus-rec/shapes.ttl`
> * `corpora/community/shacl/admission-prebound-as-draft/manifest.ttl`
> * `corpora/community/shacl/admission-prebound-as-draft/shapes.ttl`
> * `corpora/community/shacl/admission-prebound-as-rec/manifest.ttl`
> * `corpora/community/shacl/admission-prebound-as-rec/shapes.ttl`
> * `corpora/community/shacl/admission-prebound-values-draft/manifest.ttl`
> * `corpora/community/shacl/admission-prebound-values-draft/shapes.ttl`
> * `corpora/community/shacl/admission-prebound-values-rec/manifest.ttl`
> * `corpora/community/shacl/admission-prebound-values-rec/shapes.ttl`
> * `corpora/community/shacl/admission-service-draft/manifest.ttl`
> * `corpora/community/shacl/admission-service-draft/shapes.ttl`
> * `corpora/community/shacl/admission-service-rec/manifest.ttl`
> * `corpora/community/shacl/admission-service-rec/shapes.ttl`
> * `corpora/community/shacl/aggregate-subquery-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/aggregate-subquery-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/aggregate-subquery-blank-violating/manifest.ttl`
> * `corpora/community/shacl/aggregate-subquery-blank-violating/shapes.ttl`
> * `corpora/community/shacl/aggregate-subquery-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/aggregate-subquery-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/aggregate-subquery-iri-violating/manifest.ttl`
> * `corpora/community/shacl/aggregate-subquery-iri-violating/shapes.ttl`
> * `corpora/community/shacl/anchored-optional-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/anchored-optional-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/anchored-optional-blank-violating/manifest.ttl`
> * `corpora/community/shacl/anchored-optional-blank-violating/shapes.ttl`
> * `corpora/community/shacl/anchored-optional-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/anchored-optional-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/anchored-optional-iri-violating/manifest.ttl`
> * `corpora/community/shacl/anchored-optional-iri-violating/shapes.ttl`
> * `corpora/community/shacl/ask-validator-parameter-conforming/manifest.ttl`
> * `corpora/community/shacl/ask-validator-parameter-conforming/shapes.ttl`
> * `corpora/community/shacl/ask-validator-parameter-violating/manifest.ttl`
> * `corpora/community/shacl/ask-validator-parameter-violating/shapes.ttl`
> * `corpora/community/shacl/complex-sequence-inverse-path/manifest.ttl`
> * `corpora/community/shacl/complex-sequence-inverse-path/shapes.ttl`
> * `corpora/community/shacl/empty-left-optional-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/empty-left-optional-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/empty-left-optional-blank-violating/manifest.ttl`
> * `corpora/community/shacl/empty-left-optional-blank-violating/shapes.ttl`
> * `corpora/community/shacl/empty-left-optional-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/empty-left-optional-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/empty-left-optional-iri-violating/manifest.ttl`
> * `corpora/community/shacl/empty-left-optional-iri-violating/shapes.ttl`
> * `corpora/community/shacl/exists-absence-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/exists-absence-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/exists-absence-blank-violating/manifest.ttl`
> * `corpora/community/shacl/exists-absence-blank-violating/shapes.ttl`
> * `corpora/community/shacl/exists-absence-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/exists-absence-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/exists-absence-iri-violating/manifest.ttl`
> * `corpora/community/shacl/exists-absence-iri-violating/shapes.ttl`
> * `corpora/community/shacl/inventory.json`
> * `corpora/community/shacl/manifest.ttl`
> * `corpora/community/shacl/nested-not-exists-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/nested-not-exists-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/nested-not-exists-blank-violating/manifest.ttl`
> * `corpora/community/shacl/nested-not-exists-blank-violating/shapes.ttl`
> * `corpora/community/shacl/nested-not-exists-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/nested-not-exists-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/nested-not-exists-iri-violating/manifest.ttl`
> * `corpora/community/shacl/nested-not-exists-iri-violating/shapes.ttl`
> * `corpora/community/shacl/nested-optional-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/nested-optional-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/nested-optional-blank-violating/manifest.ttl`
> * `corpora/community/shacl/nested-optional-blank-violating/shapes.ttl`
> * `corpora/community/shacl/nested-optional-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/nested-optional-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/nested-optional-iri-violating/manifest.ttl`
> * `corpora/community/shacl/nested-optional-iri-violating/shapes.ttl`
> * `corpora/community/shacl/not-exists-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/not-exists-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/not-exists-blank-violating/manifest.ttl`
> * `corpora/community/shacl/not-exists-blank-violating/shapes.ttl`
> * `corpora/community/shacl/not-exists-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/not-exists-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/not-exists-iri-violating/manifest.ttl`
> * `corpora/community/shacl/not-exists-iri-violating/shapes.ttl`
> * `corpora/community/shacl/path-absence-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/path-absence-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/path-absence-blank-violating/manifest.ttl`
> * `corpora/community/shacl/path-absence-blank-violating/shapes.ttl`
> * `corpora/community/shacl/path-absence-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/path-absence-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/path-absence-iri-violating/manifest.ttl`
> * `corpora/community/shacl/path-absence-iri-violating/shapes.ttl`
> * `corpora/community/shacl/predicate-focus-blank/manifest.ttl`
> * `corpora/community/shacl/predicate-focus-blank/shapes.ttl`
> * `corpora/community/shacl/predicate-focus-iri/manifest.ttl`
> * `corpora/community/shacl/predicate-focus-iri/shapes.ttl`
> * `corpora/community/shacl/projected-subquery-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/projected-subquery-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/projected-subquery-blank-violating/manifest.ttl`
> * `corpora/community/shacl/projected-subquery-blank-violating/shapes.ttl`
> * `corpora/community/shacl/projected-subquery-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/projected-subquery-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/projected-subquery-iri-violating/manifest.ttl`
> * `corpora/community/shacl/projected-subquery-iri-violating/shapes.ttl`
> * `corpora/community/shacl/quoted-focus-optional/manifest.ttl`
> * `corpora/community/shacl/quoted-focus-optional/shapes.ttl`
> * `corpora/community/shacl/rule-not-exists/data.ttl`
> * `corpora/community/shacl/rule-not-exists/expected.ttl`
> * `corpora/community/shacl/rule-not-exists/manifest.ttl`
> * `corpora/community/shacl/rule-not-exists/shapes.ttl`
> * `corpora/community/shacl/rule-optional/data.ttl`
> * `corpora/community/shacl/rule-optional/expected.ttl`
> * `corpora/community/shacl/rule-optional/manifest.ttl`
> * `corpora/community/shacl/rule-optional/shapes.ttl`
> * `corpora/community/shacl/select-validator-parameter-conforming/manifest.ttl`
> * `corpora/community/shacl/select-validator-parameter-conforming/shapes.ttl`
> * `corpora/community/shacl/select-validator-parameter-violating/manifest.ttl`
> * `corpora/community/shacl/select-validator-parameter-violating/shapes.ttl`
> * `corpora/community/shacl/solution-declared-failure/manifest.ttl`
> * `corpora/community/shacl/solution-declared-failure/shapes.ttl`
> * `corpora/community/shacl/solution-message-priority/manifest.ttl`
> * `corpora/community/shacl/solution-message-priority/shapes.ttl`
> * `corpora/community/shacl/union-absence-blank-conforming/manifest.ttl`
> * `corpora/community/shacl/union-absence-blank-conforming/shapes.ttl`
> * `corpora/community/shacl/union-absence-blank-violating/manifest.ttl`
> * `corpora/community/shacl/union-absence-blank-violating/shapes.ttl`
> * `corpora/community/shacl/union-absence-iri-conforming/manifest.ttl`
> * `corpora/community/shacl/union-absence-iri-conforming/shapes.ttl`
> * `corpora/community/shacl/union-absence-iri-violating/manifest.ttl`
> * `corpora/community/shacl/union-absence-iri-violating/shapes.ttl`
> * `crates/conformance-kit/Cargo.toml`
> * `crates/conformance-kit/src/bag.rs`
> * `crates/conformance-kit/src/graph.rs`
> * `crates/conformance-kit/src/identity.rs`
> * `crates/conformance-kit/src/inventory.rs`
> * `crates/conformance-kit/src/lib.rs`
> * `crates/conformance-kit/src/manifest.rs`
> * `crates/conformance-kit/src/outcome.rs`
> * `crates/conformance-kit/src/report.rs`
> * `crates/conformance-kit/src/report/projection.rs`
> * `crates/conformance-kit/src/result_set.rs`
> * `crates/conformance-kit/src/reviews.rs`
> * `crates/conformance-kit/tests/grading.rs`
> * `crates/iri/src/vocab.rs`
> * `crates/rdf-wasm/src/async_query.rs`
> * `crates/rdf-wasm/src/interleaving.rs`
> * `crates/shapes/README.md`
> * `crates/shapes/benches/shared_views.rs`
> * `crates/shapes/src/components.rs`
> * `crates/shapes/src/constraints.rs`
> * `crates/shapes/src/data.rs`
> * `crates/shapes/src/engine.rs`
> * `crates/shapes/src/engine/complete_reports.rs`
> * `crates/shapes/src/expression.rs`
> * `crates/shapes/src/extension_usage.rs`
> * `crates/shapes/src/lib.rs`
> * `crates/shapes/src/prebinding.rs`
> * `crates/shapes/src/product/certified.rs`
> * `crates/shapes/src/product/mod.rs`
> * `crates/shapes/src/profile.rs`
> * `crates/shapes/src/query_law.rs`
> * `crates/shapes/src/report.rs`
> * `crates/shapes/src/rules.rs`
> * `crates/shapes/src/shacl_corpora/mod.rs`
> * `crates/shapes/src/shacl_corpora/shacl12.rs`
> * `crates/shapes/src/shapes.rs`
> * `crates/shapes/src/shapes/parser/admission.rs`
> * `crates/shapes/src/shapes/parser/functions.rs`
> * `crates/shapes/src/shapes/parser/mod.rs`
> * `crates/shapes/src/shapes/parser/node_expr.rs`
> * `crates/shapes/src/shapes/parser/rule_parse.rs`
> * `crates/shapes/src/shapes/parser/target_types.rs`
> * `crates/shapes/src/sparql.rs`
> * `crates/shapes/src/xpath.rs`
> * `crates/shapes/tests/complete_reports.rs`
> * `crates/shapes/tests/dated_execution.rs`
> * `crates/shapes/tests/product_model_census.rs`
> * `crates/shapes/tests/profile_admission.rs`
> * `crates/shapes/tests/xpath_bundle.rs`
> * `crates/sparql-conformance/Cargo.toml`
> * `crates/sparql-conformance/src/bin/community-conformance.rs`
> * `crates/sparql-conformance/src/community.rs`
> * `crates/sparql-conformance/src/lib.rs`
> * `crates/sparql-conformance/src/manifest.rs`
> * `crates/sparql-conformance/src/rs_resultset.rs`
> * `crates/sparql-conformance/tests/community_conformance.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/engine/bounded_workspace.rs`
> * `crates/sparql-eval/src/engine/graph_build.rs`
> * `crates/sparql-eval/src/engine/prepared_fallible.rs`
> * `crates/sparql-eval/src/error.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/execution.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/protocol.rs`
> * `crates/sparql-eval/src/user_fn.rs`
> * `crates/sparql-eval/tests/prepared_parameters.rs`
> * `crates/validate/README.md`
> * `crates/validate/src/complete.rs`
> * `crates/validate/src/lib.rs`
> * `crates/validate/src/product.rs`
> * `crates/validate/src/shacl.rs`
> * `crates/validate/tests/complete_shacl.rs`
> * `docs/BENCHMARKS.md`
> * `docs/CONFORMANCE.md`
> * `docs/book/po/zh-Hans.po`
> * `docs/book/src/project/releases.md`
> * `docs/design/purrdf-prepared-products.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `layers.toml`
> * `scripts/conformance-baseline.json`
> * `scripts/conformance-matrix.py`
> 
> </details>
> 
> ```ascii
>  ________________________________
> < RabbitCop: To debug and serve. >
>  --------------------------------
>   \
>    \   (\__/)
>        (•ㅅ•)
>        / 　 づ
> ```

<!-- end of auto-generated comment: review in progress by coderabbit.ai -->

<!-- finishing_touch_checkbox_start -->

<details>
<summary>✨ Finishing Touches</summary>

<details open>
<summary>📝 Generate docstrings</summary>

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


