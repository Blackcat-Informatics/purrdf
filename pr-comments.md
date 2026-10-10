# PR #526 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!IMPORTANT]
> ## Review skipped
> 
> Too many files!
> 
> This PR contains 337 files, which is 37 over the limit of 300.
> 
> To get a review, reduce the PR to 300 files or fewer by splitting it into smaller PRs or changing its base branch.
> 
> Usage-priced reviews support at most 300 files.
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `1c91e0a0-7dc2-49d8-9d3e-44b9c76e26df`
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
> Reviewing files that changed from the base of the PR and between 291513809a3bf3a2fc65881d86afb7cd34d716ea and 676f9b3e56c307b6fa552b5b41f48444e04cdcf3.
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
> <summary><strong>📒 Files selected for processing (337)</strong></summary>
> <dl>
> <dd>
> 
> * `.github/workflows/ci.yaml`
> * `Makefile`
> * `bindings/python/licenses/inventory.json`
> * `bindings/python/src/attestation.rs`
> * `bindings/python/src/py_entail.rs`
> * `bindings/python/src/py_gts_dataset.rs`
> * `bindings/python/src/py_projection.rs`
> * `bindings/python/src/py_store/prepared.rs`
> * `bindings/python/src/py_store/query.rs`
> * `bindings/python/src/py_store/results.rs`
> * `crates/cdt/Cargo.toml`
> * `crates/cdt/src/functions.rs`
> * `crates/cdt/src/lib.rs`
> * `crates/cdt/src/limits.rs`
> * `crates/cdt/src/literal.rs`
> * `crates/cdt/src/memory.rs`
> * `crates/cdt/src/ops.rs`
> * `crates/cdt/src/parse.rs`
> * `crates/cdt/src/render.rs`
> * `crates/cdt/src/term.rs`
> * `crates/cdt/src/tree.rs`
> * `crates/cdt/src/value.rs`
> * `crates/cdt/tests/value_relations.rs`
> * `crates/envelope-probe/examples/wasm_storage_qualification.rs`
> * `crates/envelope-probe/src/lib.rs`
> * `crates/hash/src/frame.rs`
> * `crates/hash/src/hex.rs`
> * `crates/hnsw/Cargo.toml`
> * `crates/hnsw/src/error.rs`
> * `crates/hnsw/src/lib.rs`
> * `crates/hnsw/src/relation.rs`
> * `crates/hnsw/src/search.rs`
> * `crates/hnsw/tests/sparql_e2e.rs`
> * `crates/iri/licenses/inventory.json`
> * `crates/iri/src/base.rs`
> * `crates/iri/src/error.rs`
> * `crates/iri/src/host.rs`
> * `crates/iri/src/langtag.rs`
> * `crates/iri/src/lib.rs`
> * `crates/iri/src/parse.rs`
> * `crates/iri/src/pos.rs`
> * `crates/iri/src/resolve.rs`
> * `crates/iri/unicode/16.0.0/UnicodeData.txt`
> * `crates/iri/unicode/PROVENANCE.md`
> * `crates/json/src/error.rs`
> * `crates/json/src/parse.rs`
> * `crates/jsonschema/licenses/inventory.json`
> * `crates/lex/examples/gen_unicode_tables.rs`
> * `crates/lex/licenses/inventory.json`
> * `crates/lex/src/allocation.rs`
> * `crates/lex/src/allocation/boxed.rs`
> * `crates/lex/src/allocation/shared.rs`
> * `crates/lex/src/allocation/text.rs`
> * `crates/lex/src/constructors.rs`
> * `crates/lex/src/diagnostic.rs`
> * `crates/lex/src/html.rs`
> * `crates/lex/src/json/error.rs`
> * `crates/lex/src/json/read.rs`
> * `crates/lex/src/json_escape.rs`
> * `crates/lex/src/lib.rs`
> * `crates/lex/src/percent.rs`
> * `crates/lex/src/terminals.rs`
> * `crates/lex/src/unicode.rs`
> * `crates/lex/src/unicode/aligned.rs`
> * `crates/lex/src/unicode/case_tables.rs`
> * `crates/lex/src/unicode/casing.rs`
> * `crates/lex/src/walk.rs`
> * `crates/lex/src/walk/scalar.rs`
> * `crates/purrdf/src/lib.rs`
> * `crates/purrdf/src/reasoning.rs`
> * `crates/rdf-capi/include/purrdf.h`
> * `crates/rdf-capi/licenses/inventory.json`
> * `crates/rdf-capi/src/cursor.rs`
> * `crates/rdf-capi/src/graph.rs`
> * `crates/rdf-capi/src/handles.rs`
> * `crates/rdf-capi/src/query.rs`
> * `crates/rdf-capi/src/version.rs`
> * `crates/rdf-capi/tests/support/hosted.rs`
> * `crates/rdf-capi/tests/support/profile.rs`
> * `crates/rdf-core/benches/shared_views.rs`
> * `crates/rdf-core/licenses/inventory.json`
> * `crates/rdf-core/src/backend.rs`
> * `crates/rdf-core/src/binding_pattern.rs`
> * `crates/rdf-core/src/blank_label.rs`
> * `crates/rdf-core/src/cdt_blank.rs`
> * `crates/rdf-core/src/collections.rs`
> * `crates/rdf-core/src/content_id.rs`
> * `crates/rdf-core/src/dataset_view.rs`
> * `crates/rdf-core/src/describe.rs`
> * `crates/rdf-core/src/diagnostic.rs`
> * `crates/rdf-core/src/governor.rs`
> * `crates/rdf-core/src/hash.rs`
> * `crates/rdf-core/src/ir/builder.rs`
> * `crates/rdf-core/src/ir/dataset.rs`
> * `crates/rdf-core/src/ir/mod.rs`
> * `crates/rdf-core/src/ir/mutable.rs`
> * `crates/rdf-core/src/ir/mutable/delta_view.rs`
> * `crates/rdf-core/src/ir/pipeline_bundle.rs`
> * `crates/rdf-core/src/ir/segmented/build.rs`
> * `crates/rdf-core/src/ir/segmented/mod.rs`
> * `crates/rdf-core/src/ir/segmented/session.rs`
> * `crates/rdf-core/src/ir/term.rs`
> * `crates/rdf-core/src/ir/term_walk.rs`
> * `crates/rdf-core/src/ir/validate.rs`
> * `crates/rdf-core/src/ir/view_accounting.rs`
> * `crates/rdf-core/src/lib.rs`
> * `crates/rdf-core/src/small.rs`
> * `crates/rdf-core/src/term_fixture.rs`
> * `crates/rdf-core/src/term_writer.rs`
> * `crates/rdf-core/src/xsd_regex/error.rs`
> * `crates/rdf-core/src/xsd_regex/scan.rs`
> * `crates/rdf-core/src/xsd_regex/xflag.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/compatibility.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/compatibility_tables.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/compile.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/match.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/mod.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/pike.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/replace.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/sets.rs`
> * `crates/rdf-core/tests/cdt_blank_storage.rs`
> * `crates/rdf-core/tests/ir_zero_alloc.rs`
> * `crates/rdf-core/tests/native_regex_storage.rs`
> * `crates/rdf-core/tests/segmented.rs`
> * `crates/rdf-core/tests/shared_owned_control.rs`
> * `crates/rdf-wasm/js/licenses/inventory.json`
> * `crates/rdf-wasm/src/dataset.rs`
> * `crates/rdf-wasm/src/interleaving.rs`
> * `crates/rdf-wasm/src/operation.rs`
> * `crates/rdf-wasm/src/projection.rs`
> * `crates/rdf-wasm/src/query.rs`
> * `crates/rdf-wasm/src/stream.rs`
> * `crates/rdf/src/projections/dataset_description.rs`
> * `crates/rdf/src/projections/dcat_rdf.rs`
> * `crates/rdf/src/projections/void/mapping.rs`
> * `crates/retrieval/src/compile.rs`
> * `crates/retrieval/src/execute.rs`
> * `crates/retrieval/tests/candidate_union.rs`
> * `crates/retrieval/tests/exclusion_lookup.rs`
> * `crates/retrieval/tests/execute_dataset.rs`
> * `crates/retrieval/tests/fusion.rs`
> * `crates/retrieval/tests/multimodal_read_bound.rs`
> * `crates/retrieval/tests/on_demand_receipt.rs`
> * `crates/retrieval/tests/wasm_determinism.rs`
> * `crates/shapes/README.md`
> * `crates/shapes/src/class_membership.rs`
> * `crates/shapes/src/engine.rs`
> * `crates/shapes/src/sparql.rs`
> * `crates/shapes/tests/change_path_alloc.rs`
> * `crates/shapes/tests/function_body_relation.rs`
> * `crates/shapes/tests/relation_incompleteness.rs`
> * `crates/shapes/tests/shared_validation_views.rs`
> * `crates/shapes/tests/sparql_path_alloc.rs`
> * `crates/shapes/tests/xpath_bundle.rs`
> * `crates/slice/src/rdf_query.rs`
> * `crates/sparql-algebra/src/algebra.rs`
> * `crates/sparql-algebra/src/ast.rs`
> * `crates/sparql-algebra/src/error.rs`
> * `crates/sparql-algebra/src/lexer.rs`
> * `crates/sparql-algebra/src/lib.rs`
> * `crates/sparql-algebra/src/owned.rs`
> * `crates/sparql-algebra/src/parser.rs`
> * `crates/sparql-algebra/src/parser/machine.rs`
> * `crates/sparql-algebra/src/parser/table.rs`
> * `crates/sparql-algebra/src/parser/triples.rs`
> * `crates/sparql-algebra/src/retained_size.rs`
> * `crates/sparql-algebra/src/scope.rs`
> * `crates/sparql-algebra/src/serialize.rs`
> * `crates/sparql-algebra/src/traits.rs`
> * `crates/sparql-algebra/src/tree.rs`
> * `crates/sparql-algebra/src/validate.rs`
> * `crates/sparql-algebra/src/walk.rs`
> * `crates/sparql-algebra/tests/allocation_free_drop.rs`
> * `crates/sparql-algebra/tests/deep_trees.rs`
> * `crates/sparql-algebra/tests/lexer_owned_admission.rs`
> * `crates/sparql-conformance/src/run.rs`
> * `crates/sparql-conformance/tests/support/mod.rs`
> * `crates/sparql-eval/Cargo.toml`
> * `crates/sparql-eval/benches/paged_cross_page_bgp.rs`
> * `crates/sparql-eval/examples/paged_stack.rs`
> * `crates/sparql-eval/src/agg_fn.rs`
> * `crates/sparql-eval/src/bgp.rs`
> * `crates/sparql-eval/src/bgp/positive.rs`
> * `crates/sparql-eval/src/binop.rs`
> * `crates/sparql-eval/src/blank_scope.rs`
> * `crates/sparql-eval/src/cdt_agg.rs`
> * `crates/sparql-eval/src/cdt_fn.rs`
> * `crates/sparql-eval/src/cdt_unfold.rs`
> * `crates/sparql-eval/src/composite_value.rs`
> * `crates/sparql-eval/src/construct.rs`
> * `crates/sparql-eval/src/contain.rs`
> * `crates/sparql-eval/src/convert.rs`
> * `crates/sparql-eval/src/dataset_spec.rs`
> * `crates/sparql-eval/src/deferred_exists.rs`
> * `crates/sparql-eval/src/describe_query.rs`
> * `crates/sparql-eval/src/enf.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/engine/bounded_workspace.rs`
> * `crates/sparql-eval/src/engine/graph_build.rs`
> * `crates/sparql-eval/src/engine/prepared_fallible.rs`
> * `crates/sparql-eval/src/error.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/execution.rs`
> * `crates/sparql-eval/src/exists_admission_gate.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/expr/cast_rounding_tests.rs`
> * `crates/sparql-eval/src/fallible.rs`
> * `crates/sparql-eval/src/governed.rs`
> * `crates/sparql-eval/src/governor/charge_points.rs`
> * `crates/sparql-eval/src/governor/ledger.rs`
> * `crates/sparql-eval/src/governor/lift.rs`
> * `crates/sparql-eval/src/governor/mod.rs`
> * `crates/sparql-eval/src/governor/soundness.rs`
> * `crates/sparql-eval/src/interned.rs`
> * `crates/sparql-eval/src/join_plan.rs`
> * `crates/sparql-eval/src/knn/invocation.rs`
> * `crates/sparql-eval/src/knn/metric.rs`
> * `crates/sparql-eval/src/knn/mod.rs`
> * `crates/sparql-eval/src/knn/tests.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/list_fn.rs`
> * `crates/sparql-eval/src/modifier.rs`
> * `crates/sparql-eval/src/modifier/owners.rs`
> * `crates/sparql-eval/src/native_numeric.rs`
> * `crates/sparql-eval/src/parallel.rs`
> * `crates/sparql-eval/src/parsed_value.rs`
> * `crates/sparql-eval/src/path.rs`
> * `crates/sparql-eval/src/plan/mod.rs`
> * `crates/sparql-eval/src/plan_cache.rs`
> * `crates/sparql-eval/src/property_fn.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `crates/sparql-eval/src/property_fn_plan.rs`
> * `crates/sparql-eval/src/protocol.rs`
> * `crates/sparql-eval/src/registry_id.rs`
> * `crates/sparql-eval/src/remote.rs`
> * `crates/sparql-eval/src/remote_http.rs`
> * `crates/sparql-eval/src/retained.rs`
> * `crates/sparql-eval/src/row_checkpoint.rs`
> * `crates/sparql-eval/src/row_ingest.rs`
> * `crates/sparql-eval/src/scratch.rs`
> * `crates/sparql-eval/src/service.rs`
> * `crates/sparql-eval/src/service_endpoints.rs`
> * `crates/sparql-eval/src/solution.rs`
> * `crates/sparql-eval/src/stack.rs`
> * `crates/sparql-eval/src/stack/height.rs`
> * `crates/sparql-eval/src/stat_agg.rs`
> * `crates/sparql-eval/src/substitute.rs`
> * `crates/sparql-eval/src/template.rs`
> * `crates/sparql-eval/src/update.rs`
> * `crates/sparql-eval/src/user_fn.rs`
> * `crates/sparql-eval/src/vm/compile.rs`
> * `crates/sparql-eval/src/vm/mod.rs`
> * `crates/sparql-eval/src/vm/tests.rs`
> * `crates/sparql-eval/src/witness.rs`
> * `crates/sparql-eval/src/workspace.rs`
> * `crates/sparql-eval/src/xpath_regex.rs`
> * `crates/sparql-eval/tests/cdt_owned_admission.rs`
> * `crates/sparql-eval/tests/construct_builder.rs`
> * `crates/sparql-eval/tests/contextual_lowercase.rs`
> * `crates/sparql-eval/tests/correlated_owned_admission.rs`
> * `crates/sparql-eval/tests/custom_aggregate_ownership.rs`
> * `crates/sparql-eval/tests/diagnostic_owned_admission.rs`
> * `crates/sparql-eval/tests/expr_vm_owned_strings.rs`
> * `crates/sparql-eval/tests/fallible_query.rs`
> * `crates/sparql-eval/tests/governed_query.rs`
> * `crates/sparql-eval/tests/numeric_aggregate_ownership.rs`
> * `crates/sparql-eval/tests/paged_stack_query.rs`
> * `crates/sparql-eval/tests/prepared_execution.rs`
> * `crates/sparql-eval/tests/prepared_operations.rs`
> * `crates/sparql-eval/tests/query_completion.rs`
> * `crates/sparql-eval/tests/ranked_declaration.rs`
> * `crates/sparql-eval/tests/relation_witness.rs`
> * `crates/sparql-eval/tests/request_substitution_feasibility.rs`
> * `crates/sparql-eval/tests/segmented_query.rs`
> * `crates/sparql-eval/tests/support/governor_counts.rs`
> * `crates/sparql-eval/tests/support/mod.rs`
> * `crates/sparql-eval/tests/support/segmented.rs`
> * `crates/sparql-eval/tests/union_join_positions.rs`
> * `crates/sparql-results/benches/graph_serialize.rs`
> * `crates/sparql-results/src/csv.rs`
> * `crates/sparql-results/src/error.rs`
> * `crates/sparql-results/src/json.rs`
> * `crates/sparql-results/src/json_read.rs`
> * `crates/sparql-results/src/lib.rs`
> * `crates/sparql-results/src/tsv.rs`
> * `crates/sparql-results/src/xml.rs`
> * `crates/sparql-results/tests/blank_label_egress.rs`
> * `crates/sparql-results/tests/refusals_precede_emission.rs`
> * `crates/sparql-results/tests/results_corpus.rs`
> * `crates/sparql-results/tests/results_streaming_is_bounded.rs`
> * `crates/sparql-results/tests/xml_characters.rs`
> * `crates/text/licenses/inventory.json`
> * `crates/text/src/analysis.rs`
> * `crates/text/src/character.rs`
> * `crates/text/src/error.rs`
> * `crates/text/src/fixed.rs`
> * `crates/text/src/index.rs`
> * `crates/text/src/lib.rs`
> * `crates/text/src/query_workspace.rs`
> * `crates/text/src/ranking.rs`
> * `crates/text/src/relation.rs`
> * `crates/text/src/score.rs`
> * `crates/text/src/segment.rs`
> * `crates/text/src/stem.rs`
> * `crates/text/tests/bounded_text_focus.rs`
> * `crates/text/tests/determinism.rs`
> * `crates/text/tests/search_property_function.rs`
> * `crates/xsd/src/bigint.rs`
> * `crates/xsd/src/bigint/compat.rs`
> * `crates/xsd/src/bigint/scratch.rs`
> * `crates/xsd/src/binary.rs`
> * `crates/xsd/src/exact/binary.rs`
> * `crates/xsd/src/exact/cost.rs`
> * `crates/xsd/src/exact/decimal.rs`
> * `crates/xsd/src/exact/error.rs`
> * `crates/xsd/src/exact/integer.rs`
> * `crates/xsd/src/exact/mod.rs`
> * `crates/xsd/src/numeric.rs`
> * `crates/xsd/src/numeric/exact_path.rs`
> * `crates/xsd/src/ops.rs`
> * `crates/xsd/src/simple.rs`
> * `crates/xsd/src/temporal.rs`
> * `crates/xsd/src/value.rs`
> * `crates/xsd/tests/exact_governance.rs`
> * `dependency-ledger.toml`
> * `docs/CONFORMANCE.md`
> * `docs/design/purrdf-change-path-allocations.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `license-inventory.toml`
> * `scripts/check-generated.sh`
> * `scripts/check-issue-refs.py`
> * `scripts/check-serializer-rewinds.py`
> * `scripts/check-test-shards.py`
> * `scripts/conformance-frozen/iri-unicode.sha256`
> * `scripts/simd-asm-manifest.toml`
> * `scripts/test-shards.py`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> You can disable this status message by setting the `reviews.review_status` to `false` in the CodeRabbit configuration file.

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

The automated review was skipped because this complete ownership migration exceeds the service's 300-file limit. Splitting it solely to satisfy that limit would separate native allocation owners from their production callers and retained-result consumers. The complete issue contract remains in this PR, with independent implementation review and the required local gate passed. The skipped review is recorded as unavailable automated coverage; required hosted checks and final review remain binding before merge.

## 3. paudley — 

# Complete bounded operational SPARQL ownership

Native operational queries admit parser, planner, evaluator and publication storage before its allocation. The original query account follows retained result rows, constructed graphs, explanations and typed diagnostics through shallow cloning and extraction, and releases only after the last payload owner is destroyed. Direct, prepared, governed and prepared-governed routes use the same production bodies. Opaque host extensions require an honest allocation certificate before dispatch.

The common fallible allocation home is in lex, with existing core reexports and confined allocation internals. Parser scopes, AST creation/clone/destruction, numeric and temporal readers, regex programs and caches, CDT primitives and FOLD, statistical/custom aggregates, SERVICE transport/decoding/remapping, graph construction/DESCRIBE/transactional append, and TEXT search/occurrence all use native owners. Immutable schema publication reuses its existing shared payload. C, Python and WASM graph consumers retain the same native dataset handle. Configured focus graphs participate in real bounded RDF/TEXT correlation.

The actual merged-main contracts are retained: measured host work is separate from cardinality and physical admission; required-timezone and requested-IRI calendar casts share the native parser; TEXT uses input-derived score certificates, usize ordinals/counts, arbitrary field spill and corpus-wide exact integer arithmetic. Normal synchronization resolved each actual overlap against both sides and produced no source delta from the full-qualified implementation.

Validation: mandatory full make check attempt13 passed, including workspace strict all-target checks, hygiene and generated/license projections, native runtime and documentation tests, and WASM release compilation. The preceding affected batch passed lexical290, native regex storage31, CDT ownership9, prepared execution17, every TEXT target including the actual unrestricted index and both bounded focus profiles, change-path allocation12 and all8 SPARQL allocation formulas. The final native walker error-cleanup regression and all11 traversal tests passed. Original governor traces, conformance goldens, admission277 and physical ceilings remain unchanged. Exact live performance pins were updated only after computation and original-owner diagnosis. Earlier full attempts1–12 remain honestly failed in the selected evidence, with complete corrections and subsequent results.

Hosted portability exposed a recursive VM frame that kept every opcode's temporaries live across EXISTS and SPARQL function suspension. The original instruction loop now returns its existing suspension carrier before recursive resolution, then resumes at the original program counter. It adds no heap owner or alternate engine. Original packaged depth assertions pass at219 levels; all439 optimized WASM package tests, TypeScript and packed tarball smoke pass. Affected lexical290/evaluator1493 native tests, strict all-target checks, original allocation controls and shared-helper hygiene also pass. The casing reference now reads the pinned Unicode17 context properties independently of the host toolchain's Unicode18 tables. Generated C headers and conformance documentation were regenerated through their original producers. Assembly selectors follow the actual native homes with every original floor and instruction guard retained; final seven-target measurement and identity ABI checks pass; final authored-document parity also passes.

Signed source and synchronization commits passed their normal hooks; no verification was bypassed. Final hosted acceptance is49 successful checks and5 expected skips, zero failed/cancelled/pending. Complete independent review-debt judgment is PASS; no substantive finding or unresolved thread remains. CodeRabbit skipped substantive review because the coherent337-file migration exceeds its service limit; the split/base workaround was legitimately declined in the posted reply. The independent completion audit in reviews/completion-audit.md and subsequent affected delta judgments cover the actual integration candidate3c08cee77205813c8e5bb924977c7cad69373acc; fetched main adds no base-only commits. Full13 qualifies unchanged production bodies, while later changes have their original affected native, portable and hosted evidence. No additional broad execution is justified solely by stage advancement.

The governed-query test's wrong-result panic reports the unexpected boolean or graph kind without dumping the certified result payload. Every existing assertion is retained; focused strict Clippy and all27 governed-query tests pass. Final hosted Rust CodeQL passed.

Hosted CodeQL subsequently passed. The overloaded E–O test shard reached its30-minute job timeout before the original100000-level parser family could finish. Its target inventory is now partitioned into E–J and K–O, retaining the same workspace feature resolution, profiles, assertions and30-minute limit on all seven shards. The original structural gate proves disjoint complete coverage, both full affected ports pass locally, and the native profiling controller includes the complete seventh-shard cold/warm/upload/reclaim chain. Final-head hosted acceptance passed, including both budget-sensitive partitions and the stable all-shard aggregate.

The authoritative plan and all validation/review artifacts are in .stage/sparql-eval-complete-bounded-workspace. Oversized stack exports are losslessly compressed with an exact replay map and verified decompression; Stage evidence remains separate from source. No scope cut, unfinished follow-up, additional runtime dependency or semantic Cargo feature is introduced.

Closes #508

Defect-Class: none

