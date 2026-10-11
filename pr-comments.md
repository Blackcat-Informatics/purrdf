# PR #532 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/532?cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review in progress by coderabbit.ai -->

> [!NOTE]
> Currently processing new changes in this PR. This may take a few minutes, please wait...
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `0afa7ea0-f822-4d62-b1b6-cfd37a39065b`
> 
> 
> <hr>
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
> Reviewing files that changed from the base of the PR and between e89e288ef476ed06c80c6593094c4d583d1f5214 and 5e1f3658a2a7546f81a550ee6a33b725971a0029.
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>⛔ Files ignored due to path filters (1)</strong></summary>
> <dl>
> <dd>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing (251)</strong></summary>
> <dl>
> <dd>
> 
> * `.github/workflows/ci.yaml`
> * `CONTRIBUTING.md`
> * `Makefile`
> * `bindings/python-rdflib-shadow/licenses/NOTICE.txt`
> * `bindings/python-rdflib-shadow/licenses/inventory.json`
> * `bindings/python/licenses/NOTICE.txt`
> * `bindings/python/licenses/inventory.json`
> * `bindings/python/src/py_retrieval.rs`
> * `crates/cdt/licenses/NOTICE.txt`
> * `crates/cdt/licenses/inventory.json`
> * `crates/columnar/licenses/NOTICE.txt`
> * `crates/columnar/licenses/inventory.json`
> * `crates/datalog/licenses/NOTICE.txt`
> * `crates/datalog/licenses/inventory.json`
> * `crates/datalog/src/evidence.rs`
> * `crates/datalog/src/lib.rs`
> * `crates/datalog/src/seminaive.rs`
> * `crates/datalog/src/seminaive/evidence.rs`
> * `crates/datalog/src/seminaive/factors.rs`
> * `crates/deflate/licenses/NOTICE.txt`
> * `crates/deflate/licenses/inventory.json`
> * `crates/ed25519/licenses/NOTICE.txt`
> * `crates/ed25519/licenses/inventory.json`
> * `crates/entail/licenses/NOTICE.txt`
> * `crates/entail/licenses/inventory.json`
> * `crates/geo/licenses/NOTICE.txt`
> * `crates/geo/licenses/inventory.json`
> * `crates/gts/licenses/NOTICE.txt`
> * `crates/gts/licenses/inventory.json`
> * `crates/hash/README.md`
> * `crates/hash/licenses/NOTICE.txt`
> * `crates/hash/licenses/inventory.json`
> * `crates/hash/src/frame.rs`
> * `crates/hnsw/licenses/NOTICE.txt`
> * `crates/hnsw/licenses/inventory.json`
> * `crates/hnsw/src/relation.rs`
> * `crates/iri/licenses/NOTICE.txt`
> * `crates/iri/licenses/inventory.json`
> * `crates/iri/src/parse.rs`
> * `crates/json/licenses/NOTICE.txt`
> * `crates/json/licenses/inventory.json`
> * `crates/jsonschema/licenses/NOTICE.txt`
> * `crates/jsonschema/licenses/inventory.json`
> * `crates/lex/licenses/NOTICE.txt`
> * `crates/lex/licenses/inventory.json`
> * `crates/lex/src/allocation.rs`
> * `crates/lex/src/allocation/ordered.rs`
> * `crates/lex/src/allocation/shared.rs`
> * `crates/lex/src/allocation/text.rs`
> * `crates/lex/src/constructors.rs`
> * `crates/lex/src/lib.rs`
> * `crates/markdown/licenses/NOTICE.txt`
> * `crates/markdown/licenses/inventory.json`
> * `crates/mime/licenses/NOTICE.txt`
> * `crates/mime/licenses/inventory.json`
> * `crates/purrdf/licenses/NOTICE.txt`
> * `crates/purrdf/licenses/inventory.json`
> * `crates/rdf-capi/licenses/NOTICE.txt`
> * `crates/rdf-capi/licenses/inventory.json`
> * `crates/rdf-capi/tests/support/hosted.rs`
> * `crates/rdf-capi/tests/support/profile.rs`
> * `crates/rdf-core/licenses/NOTICE.txt`
> * `crates/rdf-core/licenses/inventory.json`
> * `crates/rdf-core/src/dataset_view.rs`
> * `crates/rdf-core/src/evidence.rs`
> * `crates/rdf-core/src/evidence/accepted.rs`
> * `crates/rdf-core/src/evidence/answer.rs`
> * `crates/rdf-core/src/evidence/column.rs`
> * `crates/rdf-core/src/evidence/conversion.rs`
> * `crates/rdf-core/src/evidence/dictionary.rs`
> * `crates/rdf-core/src/evidence/frame.rs`
> * `crates/rdf-core/src/evidence/information.rs`
> * `crates/rdf-core/src/evidence/ledger.rs`
> * `crates/rdf-core/src/evidence/ledger/frame.rs`
> * `crates/rdf-core/src/evidence/ledger/row.rs`
> * `crates/rdf-core/src/evidence/ledger/source.rs`
> * `crates/rdf-core/src/evidence/ledger/terms.rs`
> * `crates/rdf-core/src/evidence/ledger/view.rs`
> * `crates/rdf-core/src/evidence/reason.rs`
> * `crates/rdf-core/src/evidence/record.rs`
> * `crates/rdf-core/src/evidence/standpoint.rs`
> * `crates/rdf-core/src/evidence/workspace.rs`
> * `crates/rdf-core/src/ir/dataset.rs`
> * `crates/rdf-core/src/ir/term_walk.rs`
> * `crates/rdf-core/src/lib.rs`
> * `crates/rdf-core/src/query_scope.rs`
> * `crates/rdf-core/src/query_scope/counters.rs`
> * `crates/rdf-events/licenses/NOTICE.txt`
> * `crates/rdf-events/licenses/inventory.json`
> * `crates/rdf-wasm/js/licenses/NOTICE.txt`
> * `crates/rdf-wasm/js/licenses/inventory.json`
> * `crates/rdf-wasm/licenses/NOTICE.txt`
> * `crates/rdf-wasm/licenses/inventory.json`
> * `crates/rdf/licenses/NOTICE.txt`
> * `crates/rdf/licenses/inventory.json`
> * `crates/retrieval/Cargo.toml`
> * `crates/retrieval/benches/fusion_frontier_alloc.rs`
> * `crates/retrieval/benches/on_demand_read.rs`
> * `crates/retrieval/licenses/NOTICE.txt`
> * `crates/retrieval/licenses/inventory.json`
> * `crates/retrieval/src/admission.rs`
> * `crates/retrieval/src/candidate.rs`
> * `crates/retrieval/src/canonical.rs`
> * `crates/retrieval/src/compile.rs`
> * `crates/retrieval/src/document.rs`
> * `crates/retrieval/src/error.rs`
> * `crates/retrieval/src/execute.rs`
> * `crates/retrieval/src/fixture.rs`
> * `crates/retrieval/src/fuse.rs`
> * `crates/retrieval/src/fusion_stream.rs`
> * `crates/retrieval/src/id.rs`
> * `crates/retrieval/src/iri.rs`
> * `crates/retrieval/src/lib.rs`
> * `crates/retrieval/src/matching.rs`
> * `crates/retrieval/src/plan.rs`
> * `crates/retrieval/src/planner.rs`
> * `crates/retrieval/src/ranked_stream.rs`
> * `crates/retrieval/src/render.rs`
> * `crates/retrieval/src/request.rs`
> * `crates/retrieval/src/scope.rs`
> * `crates/retrieval/src/search.rs`
> * `crates/retrieval/src/union.rs`
> * `crates/retrieval/tests/admission_tests.rs`
> * `crates/retrieval/tests/boundary.rs`
> * `crates/retrieval/tests/candidate_union.rs`
> * `crates/retrieval/tests/compile_request.rs`
> * `crates/retrieval/tests/depth_free_membership.rs`
> * `crates/retrieval/tests/exclusion_lookup.rs`
> * `crates/retrieval/tests/execute_dataset.rs`
> * `crates/retrieval/tests/fixtures/planner/lexical_request.json`
> * `crates/retrieval/tests/fixtures/planner/mixed_request.json`
> * `crates/retrieval/tests/fusion.rs`
> * `crates/retrieval/tests/multimodal_read_bound.rs`
> * `crates/retrieval/tests/on_demand_receipt.rs`
> * `crates/retrieval/tests/per_mode_row_bound.rs`
> * `crates/retrieval/tests/plan.rs`
> * `crates/retrieval/tests/planner_golden.rs`
> * `crates/retrieval/tests/real_producers.rs`
> * `crates/retrieval/tests/rendered_lookup_mode.rs`
> * `crates/retrieval/tests/search.rs`
> * `crates/retrieval/tests/supplied_exclusion_lookup.rs`
> * `crates/retrieval/tests/support/registry.rs`
> * `crates/retrieval/tests/text_exclusion_lookup.rs`
> * `crates/retrieval/tests/wasm_determinism.rs`
> * `crates/shapes/licenses/NOTICE.txt`
> * `crates/shapes/licenses/inventory.json`
> * `crates/shex/licenses/NOTICE.txt`
> * `crates/shex/licenses/inventory.json`
> * `crates/slice/licenses/NOTICE.txt`
> * `crates/slice/licenses/inventory.json`
> * `crates/sparql-algebra/licenses/NOTICE.txt`
> * `crates/sparql-algebra/licenses/inventory.json`
> * `crates/sparql-algebra/src/algebra.rs`
> * `crates/sparql-algebra/src/parser/table.rs`
> * `crates/sparql-conformance/tests/cost_planner_corpus.rs`
> * `crates/sparql-conformance/tests/goldens/evaluator-trace/suite.trace`
> * `crates/sparql-conformance/tests/governor_correctness_corpus.rs`
> * `crates/sparql-eval/Cargo.toml`
> * `crates/sparql-eval/licenses/NOTICE.txt`
> * `crates/sparql-eval/licenses/inventory.json`
> * `crates/sparql-eval/src/bgp.rs`
> * `crates/sparql-eval/src/bgp/window.rs`
> * `crates/sparql-eval/src/binop.rs`
> * `crates/sparql-eval/src/blank_scope.rs`
> * `crates/sparql-eval/src/candidate_scope.rs`
> * `crates/sparql-eval/src/construct.rs`
> * `crates/sparql-eval/src/deferred_exists.rs`
> * `crates/sparql-eval/src/describe_query.rs`
> * `crates/sparql-eval/src/enf.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/error.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/evidence_exec.rs`
> * `crates/sparql-eval/src/execution.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/extension_env.rs`
> * `crates/sparql-eval/src/governor/ledger.rs`
> * `crates/sparql-eval/src/governor/lift.rs`
> * `crates/sparql-eval/src/governor/mod.rs`
> * `crates/sparql-eval/src/interned.rs`
> * `crates/sparql-eval/src/knn/invocation.rs`
> * `crates/sparql-eval/src/knn/mod.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/modifier.rs`
> * `crates/sparql-eval/src/modifier/owners.rs`
> * `crates/sparql-eval/src/parallel.rs`
> * `crates/sparql-eval/src/path.rs`
> * `crates/sparql-eval/src/plan/mod.rs`
> * `crates/sparql-eval/src/property_fn.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `crates/sparql-eval/src/property_fn_plan.rs`
> * `crates/sparql-eval/src/remote.rs`
> * `crates/sparql-eval/src/retained.rs`
> * `crates/sparql-eval/src/row_checkpoint.rs`
> * `crates/sparql-eval/src/scratch.rs`
> * `crates/sparql-eval/src/shapes/barrier.rs`
> * `crates/sparql-eval/src/shapes/bucket.rs`
> * `crates/sparql-eval/src/shapes/dictionary.rs`
> * `crates/sparql-eval/src/shapes/mod.rs`
> * `crates/sparql-eval/src/shapes/runtime.rs`
> * `crates/sparql-eval/src/shapes/spans.rs`
> * `crates/sparql-eval/src/shapes/window.rs`
> * `crates/sparql-eval/src/solution.rs`
> * `crates/sparql-eval/src/substitute.rs`
> * `crates/sparql-eval/src/update.rs`
> * `crates/sparql-eval/src/user_fn.rs`
> * `crates/sparql-eval/src/vm/compile.rs`
> * `crates/sparql-eval/src/vm/evidence.rs`
> * `crates/sparql-eval/src/vm/mod.rs`
> * `crates/sparql-eval/src/witness.rs`
> * `crates/sparql-eval/src/workspace.rs`
> * `crates/sparql-eval/src/workspace/array.rs`
> * `crates/sparql-eval/src/workspace/boxed.rs`
> * `crates/sparql-eval/src/workspace/ordered.rs`
> * `crates/sparql-eval/tests/evidence_ingress.rs`
> * `crates/sparql-eval/tests/evidence_ledger_query.rs`
> * `crates/sparql-eval/tests/ranked_declaration.rs`
> * `crates/sparql-eval/tests/solution_row_alloc.rs`
> * `crates/sparql-eval/tests/support/mod.rs`
> * `crates/sparql-results/licenses/NOTICE.txt`
> * `crates/sparql-results/licenses/inventory.json`
> * `crates/stack/licenses/NOTICE.txt`
> * `crates/stack/licenses/inventory.json`
> * `crates/text/Cargo.toml`
> * `crates/text/benches/physical_shapes.rs`
> * `crates/text/licenses/NOTICE.txt`
> * `crates/text/licenses/inventory.json`
> * `crates/text/src/dictionary.rs`
> * `crates/text/src/dictionary_relation.rs`
> * `crates/text/src/index.rs`
> * `crates/text/src/lib.rs`
> * `crates/text/src/occurrence_dictionary.rs`
> * `crates/text/src/relation.rs`
> * `crates/text/src/score.rs`
> * `crates/text/tests/determinism.rs`
> * `crates/text/tests/physical_shapes.rs`
> * `crates/text/tests/scoring.rs`
> * `crates/text/tests/support/scoped_candidates.rs`
> * `crates/text/tests/support/shape_cases.rs`
> * `crates/text/tests/unrestricted_ranking.rs`
> * `crates/text/tests/wasm_determinism.rs`
> * `crates/validate/licenses/NOTICE.txt`
> * `crates/validate/licenses/inventory.json`
> * `crates/xsd/licenses/NOTICE.txt`
> * `crates/xsd/licenses/inventory.json`
> * `docs/SPARQL-GOVERNOR-PROFILE.md`
> * `docs/WASM_TESTING.md`
> * `docs/design/purrdf-physical-shapes.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `scripts/package-licenses.py`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> ```ascii
>  ______________________________________________________________________
> < Stop swallowing exceptions. They don't become less real in the dark. >
>  ----------------------------------------------------------------------
>   \
>    \   (\__/)
>        (•ㅅ•)
>        / 　 づ
> ```

<!-- end of auto-generated comment: review in progress by coderabbit.ai -->

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

<hr>

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

<hr>

</dd>
</dl>
</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

Carry row evidence and scoped retrieval through exact physical shapes

Closes #515
Closes #516
Closes #511
Closes #518
Closes #512

SPARQL and Datalog retain independent information state, completeness and frame through their actual operators. The shared core evidence dictionary provides verdict, summary, rows and lazy RDF 1.2 ledger tiers, content-identified reasons and explicit acceptance of undecided rows without losing rank or answer identity. Retrieval applies caller-declared scope within each producer before depth. Exact physical selection uses the existing evaluator after logical rewriting: dictionary automata over analyzed token streams, ordered windows, typed bucketed series and caller-registered shapes, with literal/shaped parity and original admission/error ownership.

The recovery repairs preserve one lazy query owner across correlated expressions, worker/UDF children, aggregates and producer boundaries. Completed closed empty existential predicates differ from incomplete absence; undecidable positive EXISTS stays enumerable. Preserved EXISTS bodies use their matching source map, while unused relation registries do not activate evidence execution. Ordinary rows return to 40 bytes; native shared controls preserve grant/clone/unique/drop semantics and successful append avoids constructing overflow errors. Governor profile v15 records exactly two evidence-bearing fuel changes, 11 to 10, with unchanged answer identities and other charges. Shared helper homes and repository-contained native-profile fixtures retain the existing execution admission guards.

The implementation is in the existing core, evaluator, Datalog, text, retrieval and host consumers; it adds no semantic Cargo feature or runtime dependency, fabricated vocabulary, alternate evaluator or silently swallowed refusal. Generated licenses/claims and manual portable/profile documentation are reconciled. Production paths remain Rust and WASM-capable within the acceptance actually executed below.

Validation and scope:

- Focused qualification passes 1,513 evaluator library cases, 20 ingress tests including 36 invoked empty-search combinations, 17 prepared tests, three row-allocation tests, all 74 SPARQL manifest cases, both evaluator traces, 15 normative governor cases and 36 native-profile cases.
- The final physical/portable collection passes 19 native controls and 110 scalar plus 15 SIMD WASM executions. Five alternating fixed-work pairs show 4.217578% fewer instructions per DISTINCT query; this is not throughput acceptance.
- The final metadata/full-gate round is FAIL, terminal 2. Metadata, strict workspace checks and preceding hygiene pass. The complete no-fail-fast native/doctest collection has one failed target: testkit temp, six passes/one failure, because a retained relocated artifact embeds its old lexical temporary root. The subsequent preserve-order consumer, final core-hygiene and nested release-crate WASM build were not reached.
- Normal signed commit hooks and push pass. Hosted doc fails on two stale engine rustdoc links, MSRV fails on recursive SharedWorkspace/IntoIterator inference at the shapes SCC caller, and workspace fails on items after a test module. Other jobs are pending/running or skipped as captured in pr-checks.txt; no green hosted claim is made. Follow-up #531 visibly records each demonstrated finding and the unexecuted suffix before merge.

The user explicitly changed the shipping scope: "You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work."

Accordingly, exclusive 36-lane throughput and 78 physical timing acceptance is canceled, not passed. Final-source throughput/physical timing remains unqualified; prior rejected lanes belong to an older candidate. Follow-up #531 does not reinstate an exclusive campaign. Independent material reviews and the final completion/review-debt audit adjudicate this revised shipping boundary, not full satisfaction of the original performance contract. No source repair/retest round or hook bypass follows that direction. Actual main rules contain no required status checks or approving-review count; normal ghprsq enforces the remaining repository prerequisites.

Integration and evidence:

The fetched actual base e89e288ef476ed06c80c6593094c4d583d1f5214 is already contained in this branch. The clean merge-tree candidate 96e38b2af751b0980afc6be0202b13ec70e1feac equals the reviewed head tree, so there is no new semantic integration delta requiring another behavior suite. Geo has not landed; its independently changed governor v15 must become a combined v16 if it lands second.

The branch retains the original Gitee-only commits, including live Gitee main 8229d73f97251e1a0502a486eee0f753a6e81d20. The earlier README rename/delete conflict was evaluated against its common base: the remote blob had no unique prose, and the complete current canonical README_zh.md was preserved. Both portable registration families and their claims were preserved during base integration. The ordinary post-landing Gitee mirror/readback remains a separate verification, not a local ancestry claim.

Authoritative plan: .stage/sparql-eval-datalog-per-row-evidence/plan.md. Selected Stage: .stage/sparql-eval-datalog-per-row-evidence. Validation index, original raw receipts, issue/addendum captures, source reviews, final audit, review-debt dispositions and follow-up publication bodies are retained there for separate ghprsq evidence capture. No Stage artifact is added to shipping source.

Defect-Class: semantic-identity-loss

