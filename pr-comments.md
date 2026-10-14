# PR #544 — comments and review threads

6 comment(s). **Every one needs a disposition** in
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
> This PR contains 621 files, which is 321 over the limit of 300.
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
> - **Run ID**: `036e3c43-974d-4f76-b2aa-148fe5821b47`
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
> Reviewing files that changed from the base of the PR and between 2995a146ead9e83b16441b721e2b690f1da77499 and bc229747be3622cf5f2cc1ec6e2094b0ba4f878c.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>⛔ Files ignored due to path filters: 1</strong></summary>
> <dl>
> <dd>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing: 621</strong></summary>
> <dl>
> <dd>
> 
> * `bindings/python/src/py_gts.rs`
> * `bindings/python/src/py_store.rs`
> * `bindings/python/src/py_store/io.rs`
> * `bindings/python/src/py_store/load.rs`
> * `bindings/python/src/py_store/mutable.rs`
> * `bindings/python/src/py_store/presentation.rs`
> * `bindings/python/src/py_store/quad_store.rs`
> * `bindings/python/src/py_store/query.rs`
> * `bindings/python/src/py_store/store.rs`
> * `bindings/python/src/py_store/term.rs`
> * `bindings/python/src/rdf.rs`
> * `bindings/python/src/shacl.rs`
> * `bindings/python/tests/load_scope.rs`
> * `crates/bench/src/lubm/check.rs`
> * `crates/cdt/src/error.rs`
> * `crates/cli/benches/support/pack.rs`
> * `crates/cli/src/error.rs`
> * `crates/cli/src/ingest.rs`
> * `crates/cli/src/report.rs`
> * `crates/cli/src/shapes_source.rs`
> * `crates/cli/src/shapes_tools.rs`
> * `crates/cli/src/validate.rs`
> * `crates/columnar/Cargo.toml`
> * `crates/columnar/benches/codec.rs`
> * `crates/columnar/examples/write_oracle_fixture.rs`
> * `crates/columnar/src/error.rs`
> * `crates/columnar/src/reader.rs`
> * `crates/columnar/src/writer.rs`
> * `crates/columnar/tests/golden_bytes.rs`
> * `crates/conformance-kit/src/bag.rs`
> * `crates/conformance-kit/src/lib.rs`
> * `crates/conformance-kit/src/outcome.rs`
> * `crates/conformance-kit/src/report.rs`
> * `crates/conformance-kit/src/report/projection.rs`
> * `crates/conformance-kit/tests/grading.rs`
> * `crates/datalog/benches/seminaive.rs`
> * `crates/datalog/src/cache.rs`
> * `crates/datalog/src/chase.rs`
> * `crates/datalog/src/cursor.rs`
> * `crates/datalog/src/dictionary.rs`
> * `crates/datalog/src/evidence.rs`
> * `crates/datalog/src/guard.rs`
> * `crates/datalog/src/id.rs`
> * `crates/datalog/src/lib.rs`
> * `crates/datalog/src/plan.rs`
> * `crates/datalog/src/proof.rs`
> * `crates/datalog/src/resolve_fol.rs`
> * `crates/datalog/src/schedule.rs`
> * `crates/datalog/src/seminaive.rs`
> * `crates/datalog/src/seminaive/evidence.rs`
> * `crates/datalog/src/seminaive/factors.rs`
> * `crates/datalog/src/store.rs`
> * `crates/datalog/src/synth_corpus.rs`
> * `crates/datalog/src/term.rs`
> * `crates/datalog/src/test_support/host_dictionary.rs`
> * `crates/datalog/src/unify.rs`
> * `crates/datalog/tests/factor_allocation.rs`
> * `crates/datalog/tests/negative_guard_admission.rs`
> * `crates/datalog/tests/never_derive.rs`
> * `crates/datalog/tests/object_permutation_allocation.rs`
> * `crates/datalog/tests/rules_runtime.rs`
> * `crates/entail/benches/chase.rs`
> * `crates/entail/benches/classify.rs`
> * `crates/entail/benches/consistency.rs`
> * `crates/entail/benches/schema_scan.rs`
> * `crates/entail/src/combined.rs`
> * `crates/entail/src/engine.rs`
> * `crates/entail/src/entails/comprehension.rs`
> * `crates/entail/src/entails/datarange.rs`
> * `crates/entail/src/entails/freeze.rs`
> * `crates/entail/src/entails/fresh.rs`
> * `crates/entail/src/entails/imports.rs`
> * `crates/entail/src/entails/mod.rs`
> * `crates/entail/src/entails/negation.rs`
> * `crates/entail/src/entails/reflexivity.rs`
> * `crates/entail/src/entails/refutation.rs`
> * `crates/entail/src/entails/warrant.rs`
> * `crates/entail/src/explain.rs`
> * `crates/entail/src/interner.rs`
> * `crates/entail/src/lib.rs`
> * `crates/entail/src/owl_dl/constructs.rs`
> * `crates/entail/src/owl_dl/data.rs`
> * `crates/entail/src/owl_dl/mod.rs`
> * `crates/entail/src/owl_dl/parser.rs`
> * `crates/entail/src/owl_dl/proof.rs`
> * `crates/entail/src/owl_dl/proof/schema_tests.rs`
> * `crates/entail/src/owl_dl/query.rs`
> * `crates/entail/src/owl_dl/tableau.rs`
> * `crates/entail/src/reasoner/classify.rs`
> * `crates/entail/src/reasoner/mod.rs`
> * `crates/entail/src/reasoner/module.rs`
> * `crates/entail/src/reasoner/profile.rs`
> * `crates/entail/src/reasoner/proof.rs`
> * `crates/entail/src/reasoner/schema_tests.rs`
> * `crates/entail/src/report.rs`
> * `crates/entail/src/rif/eval.rs`
> * `crates/entail/src/rif_xml.rs`
> * `crates/entail/tests/oracle.rs`
> * `crates/entail/tests/owlrl_oracle.rs`
> * `crates/entail/tests/pack_view_parity.rs`
> * `crates/entail/tests/reasoner.rs`
> * `crates/entail/tests/regular_role_chains.rs`
> * `crates/entail/tests/report_order.rs`
> * `crates/entail/tests/schema_cardinality_boundary.rs`
> * `crates/geo-kernel/src/index.rs`
> * `crates/geo-kernel/src/index/geographic.rs`
> * `crates/geo-kernel/src/index/projection/tests.rs`
> * `crates/geo-kernel/src/topology/relate.rs`
> * `crates/geo/tests/query_rewrite_e2e.rs`
> * `crates/geo/tests/scalar_functions_e2e.rs`
> * `crates/hash/README.md`
> * `crates/helper-census/src/capacity.rs`
> * `crates/helper-census/src/census.rs`
> * `crates/helper-census/src/layout.rs`
> * `crates/helper-census/src/ledger.rs`
> * `crates/helper-census/src/main.rs`
> * `crates/helper-census/src/normalize.rs`
> * `crates/helper-census/src/rules.rs`
> * `crates/helper-census/src/source.rs`
> * `crates/hnsw/tests/sparql_e2e.rs`
> * `crates/iri/src/parse.rs`
> * `crates/json/src/error.rs`
> * `crates/json/src/project.rs`
> * `crates/json/tests/roundtrip.rs`
> * `crates/lex/src/allocation.rs`
> * `crates/lex/src/lib.rs`
> * `crates/lex/src/message_error.rs`
> * `crates/mime/src/error.rs`
> * `crates/mime/src/project.rs`
> * `crates/mime/tests/lossless.rs`
> * `crates/purrdf/benches/multimodal_on_demand_read.rs`
> * `crates/purrdf/src/lib.rs`
> * `crates/purrdf/src/reasoning.rs`
> * `crates/purrdf/tests/closure_relations.rs`
> * `crates/purrdf/tests/fused_approximation.rs`
> * `crates/purrdf/tests/governed_entailment.rs`
> * `crates/purrdf/tests/readme_quickstart.rs`
> * `crates/purrdf/tests/support/multimodal.rs`
> * `crates/rdf-capi/include/purrdf.h`
> * `crates/rdf-capi/src/error.rs`
> * `crates/rdf-capi/src/query.rs`
> * `crates/rdf-capi/src/query/division_tests.rs`
> * `crates/rdf-capi/src/status.rs`
> * `crates/rdf-capi/src/term.rs`
> * `crates/rdf-capi/src/version.rs`
> * `crates/rdf-capi/tests/abi.rs`
> * `crates/rdf-core/Cargo.toml`
> * `crates/rdf-core/benches/dataset_state_digest.rs`
> * `crates/rdf-core/benches/intern_absoluteness.rs`
> * `crates/rdf-core/benches/intern_content_id.rs`
> * `crates/rdf-core/benches/ir_layout.rs`
> * `crates/rdf-core/benches/mutable.rs`
> * `crates/rdf-core/benches/pack_index_compare.rs`
> * `crates/rdf-core/benches/pack_query.rs`
> * `crates/rdf-core/benches/pack_restore_alloc.rs`
> * `crates/rdf-core/benches/shared_homes.rs`
> * `crates/rdf-core/benches/shared_views.rs`
> * `crates/rdf-core/benches/small.rs`
> * `crates/rdf-core/examples/segmented_fixture.rs`
> * `crates/rdf-core/src/backend.rs`
> * `crates/rdf-core/src/blank_label.rs`
> * `crates/rdf-core/src/bundle.rs`
> * `crates/rdf-core/src/cdt_blank.rs`
> * `crates/rdf-core/src/collections.rs`
> * `crates/rdf-core/src/dataset_view.rs`
> * `crates/rdf-core/src/describe.rs`
> * `crates/rdf-core/src/diagnostic.rs`
> * `crates/rdf-core/src/evidence/ledger/view.rs`
> * `crates/rdf-core/src/graph_roles.rs`
> * `crates/rdf-core/src/hash.rs`
> * `crates/rdf-core/src/imports.rs`
> * `crates/rdf-core/src/ir/builder.rs`
> * `crates/rdf-core/src/ir/bundle.rs`
> * `crates/rdf-core/src/ir/canon.rs`
> * `crates/rdf-core/src/ir/canon/state/tests.rs`
> * `crates/rdf-core/src/ir/compare.rs`
> * `crates/rdf-core/src/ir/composite.rs`
> * `crates/rdf-core/src/ir/dataset.rs`
> * `crates/rdf-core/src/ir/event_sink.rs`
> * `crates/rdf-core/src/ir/global.rs`
> * `crates/rdf-core/src/ir/import.rs`
> * `crates/rdf-core/src/ir/ingest.rs`
> * `crates/rdf-core/src/ir/mod.rs`
> * `crates/rdf-core/src/ir/mutable.rs`
> * `crates/rdf-core/src/ir/mutable/delta_view.rs`
> * `crates/rdf-core/src/ir/pack/dict.rs`
> * `crates/rdf-core/src/ir/pack/side.rs`
> * `crates/rdf-core/src/ir/pack/triples.rs`
> * `crates/rdf-core/src/ir/paged/admission.rs`
> * `crates/rdf-core/src/ir/paged/graph_index.rs`
> * `crates/rdf-core/src/ir/paged/mod.rs`
> * `crates/rdf-core/src/ir/paged/provider.rs`
> * `crates/rdf-core/src/ir/paged/query.rs`
> * `crates/rdf-core/src/ir/paged/stack.rs`
> * `crates/rdf-core/src/ir/paged/summary.rs`
> * `crates/rdf-core/src/ir/paged/translation.rs`
> * `crates/rdf-core/src/ir/pipeline_bundle.rs`
> * `crates/rdf-core/src/ir/segmented/build.rs`
> * `crates/rdf-core/src/ir/segmented/format.rs`
> * `crates/rdf-core/src/ir/segmented/mod.rs`
> * `crates/rdf-core/src/ir/segmented/query.rs`
> * `crates/rdf-core/src/ir/segmented/query/view.rs`
> * `crates/rdf-core/src/ir/segmented/session.rs`
> * `crates/rdf-core/src/ir/segmented/stack.rs`
> * `crates/rdf-core/src/ir/skolem.rs`
> * `crates/rdf-core/src/ir/term.rs`
> * `crates/rdf-core/src/ir/term_walk.rs`
> * `crates/rdf-core/src/ir/validate.rs`
> * `crates/rdf-core/src/lib.rs`
> * `crates/rdf-core/src/named_graph.rs`
> * `crates/rdf-core/src/provenance.rs`
> * `crates/rdf-core/src/small_tests.rs`
> * `crates/rdf-core/src/store.rs`
> * `crates/rdf-core/src/term_fixture.rs`
> * `crates/rdf-core/src/turtle.rs`
> * `crates/rdf-core/src/turtle_render.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/compile.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/mod.rs`
> * `crates/rdf-core/src/xsd_regex/xpath/pike.rs`
> * `crates/rdf-core/tests/blank_publication.rs`
> * `crates/rdf-core/tests/builder_locations.rs`
> * `crates/rdf-core/tests/canonical_term_mapping.rs`
> * `crates/rdf-core/tests/cdt_blank_identity.rs`
> * `crates/rdf-core/tests/cdt_blank_storage.rs`
> * `crates/rdf-core/tests/composite_probe_alloc.rs`
> * `crates/rdf-core/tests/dataset_state_digest.rs`
> * `crates/rdf-core/tests/graph_existence_modes.rs`
> * `crates/rdf-core/tests/import_view.rs`
> * `crates/rdf-core/tests/ir_absolute_iri.rs`
> * `crates/rdf-core/tests/ir_zero_alloc.rs`
> * `crates/rdf-core/tests/iri_authority.rs`
> * `crates/rdf-core/tests/literal_shapes.rs`
> * `crates/rdf-core/tests/materialize_configuration.rs`
> * `crates/rdf-core/tests/mutable_declared_graph_drain.rs`
> * `crates/rdf-core/tests/named_graph_membership.rs`
> * `crates/rdf-core/tests/native_regex_storage.rs`
> * `crates/rdf-core/tests/pack_certify.rs`
> * `crates/rdf-core/tests/pack_container.rs`
> * `crates/rdf-core/tests/pack_dataset_view.rs`
> * `crates/rdf-core/tests/pack_graph_declarations.rs`
> * `crates/rdf-core/tests/pack_id_unification.rs`
> * `crates/rdf-core/tests/pack_mmap.rs`
> * `crates/rdf-core/tests/pack_side.rs`
> * `crates/rdf-core/tests/pack_triples.rs`
> * `crates/rdf-core/tests/paged_admission_law.rs`
> * `crates/rdf-core/tests/paged_backend.rs`
> * `crates/rdf-core/tests/paged_fallible.rs`
> * `crates/rdf-core/tests/paged_stack.rs`
> * `crates/rdf-core/tests/purremb_rdf12.rs`
> * `crates/rdf-core/tests/purremb_roundtrip.rs`
> * `crates/rdf-core/tests/purremb_source.rs`
> * `crates/rdf-core/tests/reifier_quads_of.rs`
> * `crates/rdf-core/tests/segmented.rs`
> * `crates/rdf-core/tests/segmented_stack.rs`
> * `crates/rdf-core/tests/semver_additive.rs`
> * `crates/rdf-core/tests/shared_views.rs`
> * `crates/rdf-core/tests/support/dataset_state_fixtures.rs`
> * `crates/rdf-core/tests/support/paged.rs`
> * `crates/rdf-core/tests/validated_append.rs`
> * `crates/rdf-core/tests/wrapper_graph_narrowing.rs`
> * `crates/rdf-events/src/lib.rs`
> * `crates/rdf-wasm/js/index.d.ts`
> * `crates/rdf-wasm/js/index.mjs`
> * `crates/rdf-wasm/src/async_query.rs`
> * `crates/rdf-wasm/src/dataset.rs`
> * `crates/rdf-wasm/src/operation.rs`
> * `crates/rdf-wasm/src/projection.rs`
> * `crates/rdf-wasm/src/query.rs`
> * `crates/rdf-wasm/src/shacl.rs`
> * `crates/rdf-wasm/src/stream.rs`
> * `crates/rdf/benches/flat_view_canon.rs`
> * `crates/rdf/benches/native_codecs.rs`
> * `crates/rdf/benches/projections.rs`
> * `crates/rdf/benches/support/jsonld.rs`
> * `crates/rdf/examples/write_obographs_oracle_fixture.rs`
> * `crates/rdf/src/gts_compose.rs`
> * `crates/rdf/src/gts_fixtures.rs`
> * `crates/rdf/src/gts_import_graph.rs`
> * `crates/rdf/src/gts_import_sink.rs`
> * `crates/rdf/src/gts_verify.rs`
> * `crates/rdf/src/gts_write.rs`
> * `crates/rdf/src/native_codecs/jsonld.rs`
> * `crates/rdf/src/native_codecs/jsonld/expand.rs`
> * `crates/rdf/src/native_codecs/mod.rs`
> * `crates/rdf/src/native_codecs/okf/writer.rs`
> * `crates/rdf/src/native_codecs/parse.rs`
> * `crates/rdf/src/native_codecs/rdfxml.rs`
> * `crates/rdf/src/native_codecs/serialize.rs`
> * `crates/rdf/src/native_codecs/syntax.rs`
> * `crates/rdf/src/native_quads.rs`
> * `crates/rdf/src/projections/carrier.rs`
> * `crates/rdf/src/projections/csvw/exact.rs`
> * `crates/rdf/src/projections/csvw/rdf.rs`
> * `crates/rdf/src/projections/csvw/terms.rs`
> * `crates/rdf/src/projections/dataset_description.rs`
> * `crates/rdf/src/projections/dcat_rdf.rs`
> * `crates/rdf/src/projections/error.rs`
> * `crates/rdf/src/projections/lpg/csv.rs`
> * `crates/rdf/src/projections/lpg/cypher.rs`
> * `crates/rdf/src/projections/lpg/graphml.rs`
> * `crates/rdf/src/projections/lpg/mapping.rs`
> * `crates/rdf/src/projections/obo_graphs/mapping.rs`
> * `crates/rdf/src/projections/okf/mapping.rs`
> * `crates/rdf/src/projections/research_object/json.rs`
> * `crates/rdf/src/projections/research_object/mapping.rs`
> * `crates/rdf/src/projections/skos/mapping.rs`
> * `crates/rdf/src/projections/term.rs`
> * `crates/rdf/src/projections/util.rs`
> * `crates/rdf/src/projections/void/mapping.rs`
> * `crates/rdf/src/turtle_normalize.rs`
> * `crates/rdf/src/viz.rs`
> * `crates/rdf/tests/backend_seam_streams.rs`
> * `crates/rdf/tests/blank_label_egress.rs`
> * `crates/rdf/tests/blank_label_fixpoint.rs`
> * `crates/rdf/tests/blank_label_injectivity.rs`
> * `crates/rdf/tests/cdt_blank_scope.rs`
> * `crates/rdf/tests/common/mod.rs`
> * `crates/rdf/tests/content_id_byte_determinism.rs`
> * `crates/rdf/tests/describe_serialize.rs`
> * `crates/rdf/tests/drain_window_through_a_line_writer.rs`
> * `crates/rdf/tests/gts_multi_binding_reifier.rs`
> * `crates/rdf/tests/gts_self_describing_triple_terms.rs`
> * `crates/rdf/tests/gts_view_ingestion.rs`
> * `crates/rdf/tests/jsonld_compaction_residency.rs`
> * `crates/rdf/tests/jsonld_context_lens.rs`
> * `crates/rdf/tests/jsonld_options_apply_only_where_defined.rs`
> * `crates/rdf/tests/literal_fidelity.rs`
> * `crates/rdf/tests/native_codec_conformance.rs`
> * `crates/rdf/tests/okf_codec.rs`
> * `crates/rdf/tests/projection_csv.rs`
> * `crates/rdf/tests/projection_graph_carriers.rs`
> * `crates/rdf/tests/projection_streaming.rs`
> * `crates/rdf/tests/property_roundtrip.rs`
> * `crates/rdf/tests/rdf12_canon_profile.rs`
> * `crates/rdf/tests/scope_portable.rs`
> * `crates/rdf/tests/segmented_trig_roundtrip.rs`
> * `crates/rdf/tests/serialize_sink_parity.rs`
> * `crates/rdf/tests/sink_contract.rs`
> * `crates/rdf/tests/support/turtle_chains.rs`
> * `crates/rdf/tests/turtle_chains.rs`
> * `crates/rdf/tests/yamlld_key_order_is_pinned.rs`
> * `crates/retrieval/examples/fused_search.rs`
> * `crates/retrieval/examples/text_relevance.rs`
> * `crates/retrieval/src/execute.rs`
> * `crates/retrieval/tests/execute_dataset.rs`
> * `crates/retrieval/tests/real_producers.rs`
> * `crates/retrieval/tests/supplied_exclusion_lookup.rs`
> * `crates/retrieval/tests/text_exclusion_lookup.rs`
> * `crates/shapes/benches/graph_membership.rs`
> * `crates/shapes/benches/pattern_lookup.rs`
> * `crates/shapes/benches/pattern_validate.rs`
> * `crates/shapes/benches/shared_views.rs`
> * `crates/shapes/benches/support/product.rs`
> * `crates/shapes/benches/validate.rs`
> * `crates/shapes/src/class_membership.rs`
> * `crates/shapes/src/components.rs`
> * `crates/shapes/src/constraints.rs`
> * `crates/shapes/src/data.rs`
> * `crates/shapes/src/data_view.rs`
> * `crates/shapes/src/engine.rs`
> * `crates/shapes/src/engine/complete_reports.rs`
> * `crates/shapes/src/error.rs`
> * `crates/shapes/src/expression.rs`
> * `crates/shapes/src/free_expression.rs`
> * `crates/shapes/src/json_schema.rs`
> * `crates/shapes/src/lint.rs`
> * `crates/shapes/src/path.rs`
> * `crates/shapes/src/plan.rs`
> * `crates/shapes/src/product/tests.rs`
> * `crates/shapes/src/report.rs`
> * `crates/shapes/src/rules.rs`
> * `crates/shapes/src/schema_surface.rs`
> * `crates/shapes/src/shacl_corpora/report_grading.rs`
> * `crates/shapes/src/shape_union.rs`
> * `crates/shapes/src/shapes.rs`
> * `crates/shapes/src/shapes/parser/admission.rs`
> * `crates/shapes/src/sparql.rs`
> * `crates/shapes/src/srl/document.rs`
> * `crates/shapes/src/srl/equivalence_tests.rs`
> * `crates/shapes/src/srl/eval.rs`
> * `crates/shapes/src/target_eval.rs`
> * `crates/shapes/src/term.rs`
> * `crates/shapes/src/unique_values.rs`
> * `crates/shapes/src/xpath.rs`
> * `crates/shapes/src/xpath/tests.rs`
> * `crates/shapes/tests/box_role_alloc.rs`
> * `crates/shapes/tests/change_path_alloc.rs`
> * `crates/shapes/tests/complete_reports.rs`
> * `crates/shapes/tests/component_parameters.rs`
> * `crates/shapes/tests/conformance.rs`
> * `crates/shapes/tests/constraint_annotations.rs`
> * `crates/shapes/tests/custom_node_expressions.rs`
> * `crates/shapes/tests/function_body_relation.rs`
> * `crates/shapes/tests/host_entry_points.rs`
> * `crates/shapes/tests/matched_leaf_prebinding.rs`
> * `crates/shapes/tests/named_graph_view.rs`
> * `crates/shapes/tests/node_expr_reference.rs`
> * `crates/shapes/tests/node_expressions.rs`
> * `crates/shapes/tests/product_carried_analysis.rs`
> * `crates/shapes/tests/product_corpus_equivalence.rs`
> * `crates/shapes/tests/product_fixture/mod.rs`
> * `crates/shapes/tests/relation_incompleteness.rs`
> * `crates/shapes/tests/result_annotations.rs`
> * `crates/shapes/tests/rule_capacity_limits.rs`
> * `crates/shapes/tests/rules_conformance.rs`
> * `crates/shapes/tests/rules_engine.rs`
> * `crates/shapes/tests/shacl_shacl_differential.rs`
> * `crates/shapes/tests/shapes_graph_well_formed_report.rs`
> * `crates/shapes/tests/shapes_graph_wellformedness.rs`
> * `crates/shapes/tests/shared_validation_views.rs`
> * `crates/shapes/tests/sparql_node_expressions.rs`
> * `crates/shapes/tests/sparql_path_alloc.rs`
> * `crates/shapes/tests/support/report.rs`
> * `crates/shapes/tests/support/turtle.rs`
> * `crates/shapes/tests/text_dataset_path_equivalence.rs`
> * `crates/shapes/tests/view_graph_membership.rs`
> * `crates/shapes/tests/vocabulary_import_invariance.rs`
> * `crates/shapes/tests/w3c12_conformance.rs`
> * `crates/shex/benches/pattern_validate.rs`
> * `crates/shex/benches/prepared_validate.rs`
> * `crates/shex/tests/imports_unit.rs`
> * `crates/shex/tests/native_xpath.rs`
> * `crates/shex/tests/report_order.rs`
> * `crates/shex/tests/result_json_unit.rs`
> * `crates/shex/tests/semact_unit.rs`
> * `crates/shex/tests/shapemap_unit.rs`
> * `crates/shex/tests/statement_layer_unit.rs`
> * `crates/shex/tests/validate_shape_map_unit.rs`
> * `crates/shex/tests/validate_unit.rs`
> * `crates/slice/src/error.rs`
> * `crates/slice/src/ownership.rs`
> * `crates/slice/src/rdf_query.rs`
> * `crates/sparql-algebra/src/scope.rs`
> * `crates/sparql-conformance/src/community.rs`
> * `crates/sparql-conformance/src/compare.rs`
> * `crates/sparql-conformance/src/run.rs`
> * `crates/sparql-eval/Cargo.toml`
> * `crates/sparql-eval/benches/construct_builder.rs`
> * `crates/sparql-eval/benches/deep_nesting.rs`
> * `crates/sparql-eval/benches/exists_decorrelation.rs`
> * `crates/sparql-eval/benches/expr_vm.rs`
> * `crates/sparql-eval/benches/flat_operator_chains.rs`
> * `crates/sparql-eval/benches/governed_eval.rs`
> * `crates/sparql-eval/benches/graph_constant_membership.rs`
> * `crates/sparql-eval/benches/lateral_service.rs`
> * `crates/sparql-eval/benches/lateral_substitution.rs`
> * `crates/sparql-eval/benches/paged_cross_page_bgp.rs`
> * `crates/sparql-eval/benches/path_range.rs`
> * `crates/sparql-eval/benches/pattern_dispatch.rs`
> * `crates/sparql-eval/benches/prebind_seed_fast_path.rs`
> * `crates/sparql-eval/benches/prepared_reuse.rs`
> * `crates/sparql-eval/benches/query_eval.rs`
> * `crates/sparql-eval/benches/regex_eval.rs`
> * `crates/sparql-eval/benches/relation_attestation_alloc.rs`
> * `crates/sparql-eval/benches/solution_row.rs`
> * `crates/sparql-eval/examples/prepared_reuse_counters.rs`
> * `crates/sparql-eval/examples/support/paged_stack.rs`
> * `crates/sparql-eval/src/agg_fn.rs`
> * `crates/sparql-eval/src/bgp.rs`
> * `crates/sparql-eval/src/bgp/positive.rs`
> * `crates/sparql-eval/src/binop.rs`
> * `crates/sparql-eval/src/construct.rs`
> * `crates/sparql-eval/src/describe_query.rs`
> * `crates/sparql-eval/src/enf.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/engine/graph_build.rs`
> * `crates/sparql-eval/src/error.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/exists_admission_gate.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/geo/relation.rs`
> * `crates/sparql-eval/src/governed.rs`
> * `crates/sparql-eval/src/governor/charge_points.rs`
> * `crates/sparql-eval/src/governor/lift.rs`
> * `crates/sparql-eval/src/list_fn.rs`
> * `crates/sparql-eval/src/modifier.rs`
> * `crates/sparql-eval/src/nested_exists_gate.rs`
> * `crates/sparql-eval/src/nested_lateral_gate.rs`
> * `crates/sparql-eval/src/parallel.rs`
> * `crates/sparql-eval/src/parallel_determinism_gate.rs`
> * `crates/sparql-eval/src/path.rs`
> * `crates/sparql-eval/src/path_relation.rs`
> * `crates/sparql-eval/src/property_fn.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `crates/sparql-eval/src/property_fn_plan.rs`
> * `crates/sparql-eval/src/protocol.rs`
> * `crates/sparql-eval/src/rdflib.rs`
> * `crates/sparql-eval/src/remote.rs`
> * `crates/sparql-eval/src/row_checkpoint_gate.rs`
> * `crates/sparql-eval/src/scratch.rs`
> * `crates/sparql-eval/src/service_endpoints.rs`
> * `crates/sparql-eval/src/solution.rs`
> * `crates/sparql-eval/src/substitute.rs`
> * `crates/sparql-eval/src/template.rs`
> * `crates/sparql-eval/src/update.rs`
> * `crates/sparql-eval/src/user_fn.rs`
> * `crates/sparql-eval/src/vm/tests.rs`
> * `crates/sparql-eval/src/workspace.rs`
> * `crates/sparql-eval/src/xpath_regex.rs`
> * `crates/sparql-eval/tests/aggregate_function_e2e.rs`
> * `crates/sparql-eval/tests/alternative_property_paths.rs`
> * `crates/sparql-eval/tests/cdt_fold_unfold_e2e.rs`
> * `crates/sparql-eval/tests/cdt_order_by.rs`
> * `crates/sparql-eval/tests/cdt_query_blank_scope.rs`
> * `crates/sparql-eval/tests/construct_builder.rs`
> * `crates/sparql-eval/tests/correlated_call_arguments.rs`
> * `crates/sparql-eval/tests/correlated_positive_regions.rs`
> * `crates/sparql-eval/tests/cost_planner_corpus.rs`
> * `crates/sparql-eval/tests/deep_triple_terms.rs`
> * `crates/sparql-eval/tests/delta_view.rs`
> * `crates/sparql-eval/tests/directional_literal_identity.rs`
> * `crates/sparql-eval/tests/embedding_knn_e2e.rs`
> * `crates/sparql-eval/tests/evaluation_stack.rs`
> * `crates/sparql-eval/tests/evidence_ingress.rs`
> * `crates/sparql-eval/tests/exact_numerics.rs`
> * `crates/sparql-eval/tests/exists_sep0007.rs`
> * `crates/sparql-eval/tests/flat_operator_chains.rs`
> * `crates/sparql-eval/tests/function_body_relation_witness.rs`
> * `crates/sparql-eval/tests/governed_parallel_allocation.rs`
> * `crates/sparql-eval/tests/governed_query.rs`
> * `crates/sparql-eval/tests/governed_update.rs`
> * `crates/sparql-eval/tests/governor_correctness.rs`
> * `crates/sparql-eval/tests/graph_constant_membership.rs`
> * `crates/sparql-eval/tests/graph_scope_admission.rs`
> * `crates/sparql-eval/tests/graph_var_narrowing.rs`
> * `crates/sparql-eval/tests/host_identity.rs`
> * `crates/sparql-eval/tests/join_output_alloc.rs`
> * `crates/sparql-eval/tests/knn_wasm_determinism.rs`
> * `crates/sparql-eval/tests/language_tag_seams.rs`
> * `crates/sparql-eval/tests/lateral_e2e.rs`
> * `crates/sparql-eval/tests/native_xpath.rs`
> * `crates/sparql-eval/tests/numeric_parallel_determinism.rs`
> * `crates/sparql-eval/tests/operator_mapping_spec.rs`
> * `crates/sparql-eval/tests/options_sources.rs`
> * `crates/sparql-eval/tests/ordinary_binding_inputs.rs`
> * `crates/sparql-eval/tests/ordinary_seed_semantics.rs`
> * `crates/sparql-eval/tests/pack_query_e2e.rs`
> * `crates/sparql-eval/tests/paged_query_e2e.rs`
> * `crates/sparql-eval/tests/paged_stack_query.rs`
> * `crates/sparql-eval/tests/path_witness_property_function.rs`
> * `crates/sparql-eval/tests/prebound_grouping.rs`
> * `crates/sparql-eval/tests/prepared_admission.rs`
> * `crates/sparql-eval/tests/prepared_execution.rs`
> * `crates/sparql-eval/tests/prepared_parameter_mentions.rs`
> * `crates/sparql-eval/tests/prepared_parameters.rs`
> * `crates/sparql-eval/tests/property_function_blank_scope.rs`
> * `crates/sparql-eval/tests/property_function_e2e.rs`
> * `crates/sparql-eval/tests/property_function_value_sources.rs`
> * `crates/sparql-eval/tests/property_path_corpus.rs`
> * `crates/sparql-eval/tests/protocol.rs`
> * `crates/sparql-eval/tests/query_completion.rs`
> * `crates/sparql-eval/tests/rdflib_contextual.rs`
> * `crates/sparql-eval/tests/recursion_free.rs`
> * `crates/sparql-eval/tests/relation_witness.rs`
> * `crates/sparql-eval/tests/request_substitution_feasibility.rs`
> * `crates/sparql-eval/tests/scope_interactions.rs`
> * `crates/sparql-eval/tests/segmented_query.rs`
> * `crates/sparql-eval/tests/service_forwarded_text.rs`
> * `crates/sparql-eval/tests/service_live.rs`
> * `crates/sparql-eval/tests/service_resolver.rs`
> * `crates/sparql-eval/tests/sha3_e2e.rs`
> * `crates/sparql-eval/tests/solution_row_alloc.rs`
> * `crates/sparql-eval/tests/stack_refusal.rs`
> * `crates/sparql-eval/tests/string_function_arguments.rs`
> * `crates/sparql-eval/tests/support/boundary_joins.rs`
> * `crates/sparql-eval/tests/support/governor_workloads.rs`
> * `crates/sparql-eval/tests/support/mod.rs`
> * `crates/sparql-eval/tests/support/segmented.rs`
> * `crates/sparql-eval/tests/translated_property_paths.rs`
> * `crates/sparql-eval/tests/union_join_positions.rs`
> * `crates/sparql-eval/tests/update_graph_existence.rs`
> * `crates/sparql-eval/tests/update_graph_modes.rs`
> * `crates/sparql-eval/tests/update_typed_records.rs`
> * `crates/sparql-results/benches/graph_serialize.rs`
> * `crates/sparql-results/src/csv.rs`
> * `crates/sparql-results/src/graph.rs`
> * `crates/sparql-results/src/json.rs`
> * `crates/sparql-results/src/tsv.rs`
> * `crates/sparql-results/src/xml.rs`
> * `crates/sparql-results/tests/blank_label_egress.rs`
> * `crates/sparql-results/tests/results_corpus.rs`
> * `crates/sparql-results/tests/results_streaming_is_bounded.rs`
> * `crates/text/benches/search.rs`
> * `crates/text/examples/analysis_allocations.rs`
> * `crates/text/src/relation.rs`
> * `crates/text/tests/analysis_allocations.rs`
> * `crates/text/tests/bm25f.rs`
> * `crates/text/tests/bounded_text_focus.rs`
> * `crates/text/tests/ceiling_pushdown.rs`
> * `crates/text/tests/character.rs`
> * `crates/text/tests/chinese_retrieval.rs`
> * `crates/text/tests/cleanup.rs`
> * `crates/text/tests/determinism.rs`
> * `crates/text/tests/exclusion_membership.rs`
> * `crates/text/tests/field_populations.rs`
> * `crates/text/tests/index.rs`
> * `crates/text/tests/needle_value_sources.rs`
> * `crates/text/tests/phrase_composition.rs`
> * `crates/text/tests/profiles.rs`
> * `crates/text/tests/scoring.rs`
> * `crates/text/tests/search_property_function.rs`
> * `crates/text/tests/support/character_cases.rs`
> * `crates/text/tests/support/scoped_candidates.rs`
> * `crates/text/tests/support/shape_cases.rs`
> * `crates/text/tests/support/unrestricted_ranking_cases.rs`
> * `crates/text/tests/unrestricted_ranking.rs`
> * `crates/text/tests/wasm_determinism.rs`
> * `crates/validate/src/build.rs`
> * `crates/validate/src/complete.rs`
> * `crates/validate/src/entail.rs`
> * `crates/validate/src/expr_selector.rs`
> * `crates/validate/src/product.rs`
> * `crates/validate/src/regime.rs`
> * `crates/validate/src/shapes_tools.rs`
> * `crates/validate/tests/complete_shacl.rs`
> * `crates/xsd/src/exact/error.rs`
> * `docs/design/purrdf-backend-contract.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `layers.toml`
> * `scripts/simd-asm-manifest.toml`
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






<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

# Complete capacity and sparse-stack delivery

Primary 490 plus full 495, including its host-identity comment. This plan does not
close 492,491,484 or 494; their complete requirements remain in REWORK_LIST and the
captured six-issue intake. Independent grouping judgment recommends this boundary
because Datalog host term IDs and internal ID widths share a real migration,
whereas graph-free canonical/text identity and indexed range/statistics contracts
have separate acceptance. The amended plan passed independent design review in
`reviews/plan-review.md`; implementation and delivery status are maintained in
`validation.md`. The baseline descriptions below record the original intake.

## Governing constraints and prior art

Read AGENTS, .baseline and .goals. Keep main and unrelated donors protected;
preserve every hook, native memory admission, determinism, first-fault barrier and
WASM path. Add no semantic features or runtime dependency. Owned GMEOW source is
available for reuse under explicit user authority. Process artifacts stay in the
selected Stage, separately archived at integration.

Use existing homes: lexical Memory/StorageError for physical admission, core
TermId/GlobalTermId and DatasetView for identity, Datalog Id/store/compiler for
rule execution, paged G8/G9/G10/G11 for read/chronology laws, and certified
segmented sessions for sparse storage. G11 currently admits whole-dictionary
publication cost; the new sparse route changes that mechanism without discarding
its duplicate/removal, stream classification, generation and sticky-failure laws.
Packed dictionaries remain whole-load; document segmented as the large sparse
route rather than adding a second sparse pack codec.

Stagectl prior-art.md records bounded live search and linked issue coverage.
prior-art-assessment.md is independent intake/grouping judgment, not design PASS.
Precise grouped recurrence is UNKNOWN. Existing checked/admitted paths are useful
but public convenience calls still panic, global text remains u32-offset, Datalog
IDs remain u32 and paged publications reintern all source dictionaries. New unused
APIs alone cannot satisfy either issue.

## Design

1. Logical capacity refusal carries a typed space/table, attempted count and
   representable limit. Keep it distinct from physical allocation/admission and
   invalid RDF failures. The shared native storage owner carries capacity through
   core adapters; Datalog preserves the same detail in compile/evaluate errors.
   Replace actual public convenience wrappers and migrate callers so no production
   mint/growth path converts capacity to expect/assert/panic. Document breaking
   Result signatures; do not leave a parallel unchecked convenience API.
2. Keep local RDF TermId's compact NonZeroU32 representation. Its checked mint
   and range constructors fail before mutation. Widen global string ranges to
   u64 while retaining one checked arena range/append implementation for local
   and global widths. Global IDs are logical u64; convert to host buffer indexes
   only after checked bounds. Datalog native entity handles become NonZeroU64,
   with checked minting for every brand and separate physical usize addressing.
3. Datalog facts and compiled constants use an associated term carrier/dictionary
   interface. The native carrier uses the widened native term handle; the host
   carrier returns caller IDs unchanged. Variable/proof/row/partition addresses
   remain distinct internal spaces. Consumers supply dictionary lookup and any
   derived-term creation they own. Reuse DatasetView's existing generic evaluator
   route; no remapping wrapper is inserted into joins. Raw ID order never becomes
   a claim of numeric/time value order: expose certified ordering separately.
4. Freeze immutable removal and graph-declaration carriers once when each layer
   seals, under their native storage owners. An owned SegmentedStack publication
   shallow-pins those carriers and chronological certified source descriptors;
   it copies only the current mutable head and its pending removals/declarations.
   Opening reads only headers. Publication neither traverses nor deep-copies
   historical tombstones, lexical payloads or declarations, so its work/retention
   is O(segment descriptors + current head), including current head removals.
   Dictionaries stay with their segments. Native query sessions build a bounded, lazy value-to-ID
   translation only for touched terms; host sessions use the host's stable IDs and
   dictionary directly. Both feed the same evaluator/Datalog operator path.
5. Original source cursors supply ordered physical rows. Effective-set merging
   applies newer removals/additions, duplicate suppression and native RDF 1.2
   stream classification using the existing classification law. One operation
   owns all source budgets, original request addresses and the first typed fault.
   Metadata/head egress cannot bypass that latch. Every checkpoint verifies exact
   pinned descriptors, including skipped and empty sources; final readiness is
   required before publishing any query result.
6. Segment replacement is an immutable publication. Rewriting affected segments
   removes their dead text and dependent dictionary entries, leaving unrelated
   segment bytes unchanged. Retained old snapshots continue to read their own
   originals until released. New snapshots do not retain rewritten-away providers,
   dictionaries, caches or lexical metadata. Consumer persistence and compaction
   policy remain caller-owned; no hidden segment/depth/count ceiling is added.

## Completeness contract

| Requirement | Task and real acceptance |
| --- | --- |
| Every listed 490 ingress | T1: terms/arena/quads/permutations/delta terms/merge/import scopes/provenance unit, artifact, origin set/Datalog spaces each have a boundary-counter test through their actual mint/growth home and public error propagation; no panic, healthy neighbor succeeds, failed mutation preserves prior state. |
| No capacity panic | T1/T3: Rust census checks non-test core and Datalog mint/growth implementations and fails on capacity unwrap/expect/assert; invariant checks are not used to disguise data exhaustion. |
| Global text beyond 4GiB | T1: synthetic start/count at and beyond u32::MAX through the real checked wide-range constructor, local width refuses, global width succeeds, terminal overflow fails without mutation; real ordinary strings round-trip. |
| Datalog beyond u32 | T1: native term/row/partition and other branded minting crosses u32::MAX on 64-bit hosts; checked host-address conversion on 32-bit/WASM refuses only genuinely unrepresentable resident buffers; compile/store/evaluate consume the widened types. |
| Host identity addendum | T2: instrumented type-tagged host IDs, triple-term IDs, deleting/nonresident dictionary and certified numeric/time order enter actual SPARQL and Datalog scans; counters show no join remaps or translation. Equal semantics with the native carrier and typed source faults are tested. |
| Sparse 1000-segment open | T2/T3: actual certified 1000-segment providers count every block request; stack open reads no term/reverse/quad block and reports zero decoded dictionary bytes. |
| Term-independent publication | T2/T3: fixed descriptors/current head with tenfold term counts and separately tenfold historical tombstone counts/lexical bytes measure complete snapshot work/retention and per-segment cost; no historical payload traversal/copy, dictionary rebuild/decode/translation allocation occurs at publication. Current mutable head work is included, not subtracted from the bound. |
| Eager effective-set identity | T2: generated and authored actual queries over duplicates, removals, insert/remove/insert, multi-source reifiers, orphan annotations, empty named graphs and deep terms match an eager seal; original G11 typing/order laws remain. |
| Binding and faults | T2: retained snapshots, generation drift in skipped/empty sources, provider/corruption/admission failures and constants/head/metadata egress preserve the exact first typed cause and refuse publication; whole-operation budgets never reset per segment. |
| Local purge | T2/T3: rewrite only touched segments, drop originals, scan remaining persisted bytes for a unique deleted lexical sentinel and its dependency dictionary entries; unaffected bytes and retained-old-snapshot reads are independently checked. |
| Portability and real callers | T3: affected native core/Datalog/evaluator and other changed public callers; portable operational provider/host-ID controls and all release WASM builds; actual public import/builder/compile/evaluate/read entry points, not helper-only tests. |
| Complete merge | T4: both full issue contracts independently accepted, mandatory local gate, normal hooks, current hosted checks/review debt, actual merge candidate, ghprsq, verified issue closure and safe owned cleanup. Partial source gives zero completion credit. |

## Task 1: Complete typed capacity and native widths

Implement all actual mint/growth/public propagation sites and caller migrations
as one coherent source batch, including global text and every Datalog ID brand.
Use boundary injection into the same checks production calls; never allocate
billions of rows merely to exercise arithmetic. Remove obsolete unchecked paths.
Complete the structural census and width/rollback/ordinary neighbor controls.
Run affected strict compilation and the complete boundary/production family
after source settles. Independent task review covers each listed ingress, actual
callers, admitted buffers and default layout/throughput. Commit/push normally only
after acceptance; publish the task record, with both whole issues still open.

## Task 2: Complete host identity and sparse segmented stacks

Implement the associated Datalog term/dictionary path and actual host consumers;
freeze sealed removal/declaration carriers once and shallow-pin them while
composing certified segmented sources through owned sparse publications and
whole-operation guarded sessions. Include removals, native stream classification,
replacement/purge and retained snapshots. Write all semantic, source-fault,
zero-remap and sparse-open controls before one consolidated native/portable
qualification batch. Reuse native physical owners; no unpriced per-query table.
Independent review must follow real scans into both evaluator and rule engine.
Fix required findings, run affected checks and commit/push the coherent unit.

## Task 3: Complete scale, performance and final qualification

Run 1000-segment open and fixed-segment/head tenfold-term and historical-removal
publication measurements,
effective-set differential/property cases, local byte purge and all required
portable/host/public capacity consumers. Capture actual native ownership and
default-throughput controls. Finalize documented API/error/backend contracts and
regenerate maintained projections through their generators. Run the required full
make-check once after source settles; retain passing evidence and use affected
corrections for later changes under Stage validation policy. Perform independent
full-contract judgment; every missing/failed criterion blocks publication.

## Task 4: Publish and fully merge both contracts

Use Stage tools for issue updates and one complete PR closing 490 and 495. Publish
the accepted plan, current evidence and reader uncertainty. Finish proportional
review remediation, current hosted gates, integration-tree/base assessment and
independent completion/review-debt judgment. Merge only with ghprsq and selected
Stage evidence. Verify both issues closed and durable archive, then safely clean
only the owned worktree/branch. Update REWORK_LIST with two completions only after
the authoritative merged/closed state exists.

## Declined alternatives and costs

Do not couple all six issues merely to share crates/gates; their separate scale
and identity contracts would delay ready enablers. Do not offer a foundations-only
PR or close 495 for interfaces. Do not widen local RDF rows unnecessarily, use Cargo
features for width/semantics, rebuild a global dictionary per snapshot, remap host
IDs in joins, add a dependency, silently use raw ID order for ranges, or weaken
source-fault/native-memory barriers. Native lazy translations have query-local
work; the host direct path deliberately avoids that work. Widened Datalog handles
cost physical bytes, which must be admitted and measured, never hidden by an
unchanged budget formula. All retained contracts remain work to complete now.

## Independent review disposition

R1 addressed in design 4, the term-independent publication acceptance row and
T2/T3: historical sealed removals/declarations are immutable shared carriers,
publication pins them without traversal, and measured total snapshot work includes
controls growing historical tombstone entries and lexical bytes tenfold at fixed
descriptors/current head. No residual historical-copy cost is subtracted or
reported as satisfying the required bound. The independent targeted recheck in
`reviews/plan-review.md` discharged R1 and accepted the amended plan. The initial
NEEDS-REVISION finding remains preserved there as history; implementation and
scale acceptance are recorded separately in the current validation index.

## 3. paudley — 

Local implementation and qualification now cover both linked contracts. The
settled integration full `make check` passed, including native workspace tests
and doctests, external consumer, generated/hygiene gates and release WASM.
The later reviewed shared host-fixture/rustdoc delta passes its affected strict,
native, installed Python, documentation and enforced-census checks. Independent
Task 1, Task 2 and Task 3 reviews pass within those recorded scopes.

The least certain remaining surface is current-head hosted integration across
the alternate compilers/architectures and optimized package. Local results and
qualified shared-fixture evidence do not substitute for those jobs. The existing
optimized recursion-floor failures in sibling work remain failures if reproduced
here; no threshold or acceptance waiver is claimed.

The capacity checker is an audited, fail-closed source ratchet, not a Rust compiler
proof. Its forty resident reconstruction rows pin exact sinks, source authorities
and a conservative constructor-reference closure. Changes require source review;
the test-module move deliberately produced forty stale findings before only its
reviewed aggregate identity was refreshed. Sparse query IDs remain operation-local;
consumer persistence owns segment replacement and final old-owner release.

Hosted review, protected integration through ghprsq and verified issue closures
remain outstanding. No issue completion is claimed from local qualification.

## 4. paudley — 

The hosted documentation failure was traced to two stale GlobalDictionary
error links. Their documentation now names the actual IriReadError lexical and
storage variants. The full warning-denied documentation build then exposed one
Datalog TermId link whose import was test-only; it now links to its existing
production type.

The corrected standard make doc passes all workspace documentation targets.
No runtime body changed. The census intentionally detected the changed source
and shifted line; after independent source review, only those exact proof
identities and the one location were refreshed. Final index/helper results and
normal publication are recorded in the Stage validation index. Existing full
make check and runtime qualification remain scoped to unaffected behavior.

The original hosted failure remains a failure on its original head. Current-head
hosted checks and final integration assessment remain pending; no merge or issue
closure is claimed.

## 5. paudley — 

Current required CI on the published head is terminal: 33 jobs passed, 9 failed and 4 were skipped. CodeQL, documentation and doctests passed. Five assembly lanes reject relation membership; SVE membership passes but five generated cells are stale. Aggregate conformance suites passed, with a stale Python projection. The optimized package passed 446/449 tests and failed the original three depth requirements at 125/205/125 versus 181/281/180.

The bounded local correction counts full-identity paired matches through the existing generic membership home. Native tail controls passed 3/3, tagged-host membership 1/1 and public rule/proof/source-error cases 6/6; warning-denied all-target Datalog Clippy passed. Independent source review accepted that scope. Standard SSE2 diagnostic assembly shows full-width equality and packed reduction; emitted AVX2 compares both columns with 64-bit equality. Neither observation is a performance win or an assembly gate pass. The sequential seven-configuration diagnostic is still running. No ID narrowing, remap, duplicate kernel or compiler/cache/profile workaround was introduced.

The existing conformance generator corrected the single Python count from 4218 to 4223 using the four successful published-head shard results. The evaluator driver repair is separately compiling and being checked against the unchanged optimized depth requirements. Final source qualification, affected generated acceptance, review and integration remain incomplete; this PR is not merge-ready.

## 6. paudley — 

# Full-width membership qualification

The final generic tail compares both complete identities and counts matches in
u32 over the existing 64-row bound. Native tail 4/4, tagged host-ID membership 1/1,
public rule/proof/source-error 6/6 and warning-denied Datalog checks have scoped
passes. Directed controls distinguish high halves and host tags/payloads.

The original seven-configuration diagnostic 77123 completed successfully. Actual
Relation::contains vector-operation counts are 25/41/26/45/22/0/8 in manifest
configuration order. The registered floors remain 6/17/5/21/7/0/3. The manifest
now identifies actual full-width equality and counted-reduction instructions;
max_fma=0 and relaxed-instruction refusal remain unchanged.

The original complete producer 21067 regenerated measured documentation cells and
passed 112 sites on all seven configurations. Manual prose now describes counted
full-width comparisons and points to the actual Relation::contains source home.
Final original gate 16252, including documentation parity and source-bound JSON
report, completed successfully for 112 sites on all seven configurations. Its
report is integrated-wide-tail-seven-config-report.json; no report identity or
numeric cell was hand edited. Compatible installed Cargo units were reused;
the generator recorded zero new rustc invocations in the reused configurations.

Main integration changed the existing constructor-proof closure and two explicit
authority sources. The source assessment is integrated-capacity-proof-assessment.md.
Only actual source identities were refreshed; all forty theorem sinks, classes,
locations and reasons are unchanged. Original helper gate 28207 passed 82 enforced
jobs,118 distinct-body rows and zero open copies. The earlier stale-proof failure
remains retained. This is delta assessment and executed acceptance, not a fresh
independent source audit or an instruction/throughput win.

Normal hooked commit `487cd78c8` contains only the manifest, maintained SIMD document
and proof registrations. Publication 64669 is in progress. Current hosted checks
and complete review/integration assessment remain required; neither issue is
closed by these qualification results.

