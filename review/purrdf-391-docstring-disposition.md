# PR #391 public documentation advisory disposition

The aggregate 69.44% Docstring Coverage result against the default 80% threshold does not identify an undocumented shipping API. The full branch diff introduces 14 shipping public callable APIs, six types and one module; all have meaningful contracts. Four shipping source files contain specific stale claims, corrected by `/tmp/purrdf-391-public-contracts.patch`. Final documentation acceptance requires applying that patch and completing the root agent's verification; this read-only review does not credit those checks as passed.

Audit snapshot: shipping source `92842627b840a755a3fca31a78d8aa274a34fa4b`; current branch head at patch preparation `aa529ee697baf39aaea2148db25bc03dfff66e5e`. All four files still match the shipping snapshot exactly. `git apply --check` succeeds; every added/removed source line is a Rust documentation comment. No checkout file was modified, and no build or test ran during this review. Hashes and patch identity are recorded in `/tmp/purrdf-391-public-contracts-check.json`.

## Complete new shipping public callable API inventory

Locations refer to the pre-patch snapshot. Relocated/re-exported functions are identified rather than counted twice.

| Public API | Source | Contract checked |
| --- | --- | --- |
| `purrdf_lex::terminals::is_valid_blank_node_label` | `crates/lex/src/terminals.rs:853` | Exact W3C terminal, bytes after `_:`; boolean admission. Relocated from core. |
| `purrdf_core::collections::try_build_rdf_list` | `crates/rdf-core/src/collections.rs:525` | Order, empty `rdf:nil`, immediate first callback error; caller stages publication for atomicity. |
| `MutableDataset::visit_blank_identities` | `crates/rdf-core/src/ir/mutable.rs:238` | Borrowed retained base/delta identities, suppressed base and nested terms included; first `ControlFlow::Break`. |
| `TermValue::visit_blank_identities` | `crates/rdf-core/src/ir/term_walk.rs:198` | Iterative triple/composite traversal, occurrence order and repeated occurrences, original scopes, callback borrowing and early break. |
| `scope::ObserverRole::name` | `crates/sparql-algebra/src/scope.rs:70` | Diagnostic observer name. |
| `scope::ScopeError::action` | `crates/sparql-algebra/src/scope.rs:153` | Concrete correction for each refused observer role. |
| `scope::validate_pattern` | `crates/sparql-algebra/src/scope.rs:206` | Identity-only match-witness admission; internal VALUES/UNION sharing allowed; deterministic first refusal. |
| `scope::validate_query` | `crates/sparql-algebra/src/scope.rs:216` | Query head before pattern; typed role/site refusal. |
| `scope::validate_quad` | `crates/sparql-algebra/src/scope.rs:228` | Every template slot, recursive triple terms; independent item index versus full-query index; typed refusal. |
| `scope::joins_blank_scope` | `crates/sparql-algebra/src/scope.rs:401` | Positive BGP ownership relation. |
| `scope::spine_leaves` | `crates/sparql-algebra/src/scope.rs:416` | Appends leaves in written order, iterative walk. |
| `scope::visit_spine_leaves` | `crates/sparql-algebra/src/scope.rs:428` | Ordered early-stop visitor; patch corrects allocation claim to eight pending inline nodes and possible deep spill. |
| `scope::visit_leaf_labels` | `crates/sparql-algebra/src/scope.rs:464` | Raw/carried exposure policy, UNION raw-local boundary, first matching label stops. |
| `scope::visit_term_labels` | `crates/sparql-algebra/src/scope.rs:507` | Subject-before-object quoted-term traversal and early stop, iterative work list. |

The new public types are `ScopeHazard` (line 31), `ObserverRole` (39), `ScopeRegion` (91), `ScopeSite` (109), `ScopeError` (140), and `LabelSource` (453), all in `crates/sparql-algebra/src/scope.rs`. All variants and public fields have purpose/position contracts; `ScopeSite` defines zero-based deterministic addresses and `ScopeError` defines the refused identity. Module `scope` is public at `lib.rs:83`; module docs explicitly separate role admission from transformation provenance, solution bags and runtime allocation. The five root type re-exports at `lib.rs:108` and core's preserved `blank_label::is_valid_blank_node_label` re-export use those same documented homes.

Excluded from the external API denominator: inherited `Display`/`Error`/`From`/`Clone` trait implementation documentation; private and `pub(crate)` implementation seams; dev-only candidate evidence, tests, benchmarks and example helpers. These exclusions do not excuse false claims in their comments: the patch also repairs the shared formatter's stale fourth-site narrative and nonexistent `template::mint_blank` link.

## Concrete corrections in the prepared patch

1. `eval.rs:392,453,456,1128`: prefix application describes complete candidates `{prefix}{stem}{n}` rather than promising the exact suffix an unprefixed execution would choose. Vacancy checks apply to the complete DEFAULT-scope identity and consume occupied counter values. List and SERVICE allocations share the counter; fixed stems include `service`. SHACL uses its encoded focus tags; arbitrary unequal caller prefixes are not claimed universally disjoint across stems.
2. `remote.rs:8,258,345`: SERVICE forwards a complete standalone checked SELECT carrier, with visible projection rather than a universal `SELECT *` promise. A zero-visible body carries a hygienic unit transport column; ingestion removes it and retains every row. Resolver implementations return the actual requested columns and rows.
3. `blank_label.rs:436,444`: the prefix validator lists the SERVICE stem; the complete label ends in a counter digit, rather than asserting the stem itself ends in a digit.
4. `scope.rs:423`: no-renaming input is not a universal zero-allocation promise. The walk keeps eight pending nodes inline and can spill for a deeper spine, independent of whether any label needs renaming.

No other missing/stale public contract was found in the changed shipping APIs. The material-change review included the core composite ingress/reverse-lookup laws (`GlobalDictionary::intern_literal`, `intern`, `DatasetView::term_id_by_value`), scratch retained-byte accounting, governor attachment and profile 10, query/hidden-role admission, all four checked/infallible SELECT carrier APIs, checked UPDATE serialization, `ResolvedBindings` response-local sharing, and all three prepared CONSTRUCT destination builders. Their documented error, partial-answer and atomic-publication boundaries match the implementation.

## Existing focused controls for root verification

These are exact existing test names, inspected in source; they were not rerun by this reviewer. Run each with the stated target and its name as the libtest filter.

| Target invocation | Exact filters |
| --- | --- |
| `cargo test --locked -p purrdf-sparql-eval --lib` | `eval::tests::bnode_mint_prefix_rejects_an_illegal_prefix`; `eval::tests::bnode_mint_prefix_accepts_a_shapes_style_prefix`; `construct::tests::unprefixed_template_blanks_mint_exact_c_labels`; `construct::tests::prefixed_template_blanks_carry_the_prefix`; `construct::tests::template_blank_is_fresh_against_data_labels`; `construct::tests::freshness_allocation_is_deterministic_across_runs` |
| Same evaluator lib target | `remote::body_walk_tests::late_concrete_values_cannot_capture_an_earlier_service_response_blank`; `remote::body_walk_tests::separate_responses_avoid_local_callback_and_prior_mint_identities` |
| `cargo test --locked -p purrdf-sparql-eval --test scope_interactions` | `raw_late_values_cannot_capture_an_earlier_bnode_allocation`; `update_allocations_avoid_reserved_namespaces_and_honor_caller_prefix`; `federation_names_preserve_caller_lookalikes_and_zero_column_bags` |
| `cargo test --locked -p purrdf-sparql-algebra --test scoped_carriers` | `zero_column_and_deep_positive_carriers_preserve_witness_policy` |
| `cargo test --locked -p purrdf-core --lib` | `blank_label::tests::structural_blank_node_label_prefix`; `blank_label::tests::legal_prefix_always_yields_a_legal_minted_label` |

The late concrete-input controls exercise both no-prefix and caller-prefix contexts. The SERVICE response control checks DEFAULT-scope vacancy against callback, bare dataset and embedded composite identities, response equivalence classes, and independent repeated response maps. The federation control proves zero-column bag restoration. The eight-inline-node bound is directly visible in `WorkList::<_, 8>`; no new behavioral test is needed for this documentation correction.

The correction changes only prose. Historical prototype and runtime measurement samples and source hashes must remain immutable. After application, the report should separately identify documentation-only drift in `scope.rs` (one prototype capture) and `eval.rs`/`remote.rs` (two runtime captures), without claiming their old recorded file hashes equal the new documented source. `blank_label.rs` has the same documentation-only status but is not one of those measured source captures.
