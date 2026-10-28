<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, arithmetic, refusal, ownership and
conformance. A WASM build, 32-bit value, shared test runner, or historical
selection does not justify replaying an ordinary Rust suite on WASM.

WASM execution is restricted to actual platform dispatch/SIMD kernels,
shadow-stack floors, host imports/interfaces, and named frozen native byte or
digest identities. Every selection below has one of those obligations. Use exact
case filters. Never execute a whole mixed target merely because it has a WASM
runner. Keep the other cases in native coverage.

`make wasm` builds the release crates for `wasm32`. `make wasm-pkg-test` checks
the shipped JavaScript/Worker bindings and identity ABI; those are host-interface
checks. Assembly codegen remains the separate seven-configuration `make simd-asm`
claim. None of these substitutes for another.

The packaged case `isomorphic refusal crosses the wasm boundary as a JavaScript
Error` in `canonicalization-host-error.test.mjs` exercises the wasm-bindgen
`JsError` construction/import and JavaScript exception prototype. It distinguishes
a caught host Error from a WASM trap; native Rust owns the underlying reserved-
vocabulary refusal semantics. Run only this named package case for that obligation.

The packaged case `wasm_shacl_apply_rules: maxStoredFacts reaches the evaluator
and names the host control` checks BigInt argument transport, a JavaScript Error
and its host-specific control name, and returned string ownership. Its fixture
has three input statements; native Rust owns capacity-scale correctness.

## Audited focused execution

The optimized package's `async-coordinates.test.mjs` selects these host-interface
cases exactly; native Rust retains timestamp parsing and random-stream semantics:

| Exact package case | Concrete host obligation |
| --- | --- |
| `async coordinates cross every query kind and output alias` | JavaScript options reach all seven async operation kinds and typed/raw/context aliases; exact strings and unsigned BigInts cross wasm-bindgen; supplied seeds avoid the actual host entropy import |
| `async coordinates reject lossy host inputs and unsupported routes` | Checked JavaScript number/BigInt transport, host exceptions and explicit unsupported-interface refusal |
| `async coordinate owners survive real suspension and interleaved jobs` | JSPI suspension retains owned coordinates after options handles are freed and another job runs |
| `async jobs select omitted coordinates anew when validated options are reused` | Real host clock/entropy imports remain per-job defaults rather than retained validated-option state |
| `Cloudflare query coordinates reach negotiated output and refuse UPDATE` | The shipped Worker request adapter forwards query options and rejects unsupported UPDATE configuration |

Run these named cases with `node --test --test-name-pattern` against the actual
`make wasm-pkg` output. The type-only `async-coordinates.types.ts` checks the
declarations without executing query semantics on WASM.

The 2026-10-23 audit examined every Makefile WASM invocation and the direct
workflow, qualification-driver, runner, package and Worker entry points,
regardless of origin. It removed 312 of 359 Makefile case executions, retaining
47 executions across 23 scalar/SIMD invocations. Semantic-only WASM jobs in the
segmented-storage, Datalog and entailment workflows were removed. Their native
jobs and test bodies remain. Coordinate jobs retain the actual host-seal cases.

Runner preflight exercises planted failure, rejected flags and sealed host
imports. After preflight, `PURRDF_TEST_REQUIRE_EXACT=1` requires each named case
exactly once; missing, ignored, duplicate or skipped selections fail.

| Package / target | Build | Exact retained cases | Concrete obligation |
| --- | --- | --- | --- |
| `purrdf-gts` / `mmr_preimages` | scalar | `streamed_mmr_roots_preserve_frozen_proofs_and_variable_frame_ids` | Frozen native MMR root and proof bytes |
| `purrdf-rdf` / `turtle_chains` | scalar | `blank_chains_at_one_hundred_thousand`, `blank_chains_at_one_million` | Frozen native packed/Turtle/TriG byte receipts |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar | `the_reassociated_path_is_the_one_this_build_was_made_for` | Actual WASM backend dispatch |
| `purrdf-sparql-eval` / `stack_refusal` | scalar | `prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller` | Actual WASM shadow-stack floor |
| `purrdf-core` / `model_traits` | scalar | `owned_term_walks_stay_inside_the_wasm_shadow_stack_floor` | Measured WASM shadow-stack floor and restoration |
| `purrdf-hnsw` / `wasm_reassociated` | scalar | `an_image_recorded_on_another_wasm_path_is_refused_by_name`, `the_image_records_the_path_and_shape_this_build_was_made_for` | Actual WASM arithmetic backend identity |
| `purrdf-columnar` / `golden_bytes` | scalar | `seeded_files_are_the_pinned_bytes` | Frozen native Parquet bytes |
| `purrdf-hash-conformance` / `blake3` | scalar | `required_backends_are_available` | Actual WASM backend availability |
| `purrdf-testkit` / `bench` | scalar | `the_store_options_are_refused_on_wasm32`, `a_measured_run_reports_its_estimates` | WASM host clock and unavailable filesystem |
| `purrdf-stack` / `on_stack` | scalar | `an_in_floor_request_runs`, `a_scoped_in_floor_request_borrows_the_caller_s_locals`, `an_over_floor_request_is_refused`, `an_over_floor_scoped_request_is_refused` | Actual WASM shadow-stack floor |
| `purrdf-deflate` / `deflate_conformance` | scalar | `selected_backend_is_reported` | Actual WASM codec backend dispatch |
| `purrdf-xsd` / `exact_wasm_determinism` | scalar | `the_transcript_digest_is_reproduced_on_this_target`, `large_binary_products_match_the_schoolbook_oracle` | Pinned native wide-arithmetic transcript digests |
| `purrdf-sparql-eval` / `query_coordinates` | scalar | `supplied_inputs_repeat_complete_results_without_host_reads`, `supplied_large_fraction_and_timezone_survive_repeated_now`, `prepared_executions_select_fresh_inputs_without_changing_the_plan`, `bounded_coordinate_owners_price_real_allocations_and_release_after_last_output` | Sealed WASM host clock and entropy imports |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_hand_answers_are_reproduced_on_this_target`, `the_numeric_transcript_digest_is_reproduced_on_this_target` | Pinned native numeric answers and transcript digest |
| `purrdf-text` / `wasm_determinism` | scalar | `the_sentence_boundaries_are_reproduced_on_this_target` | Pinned native UTF-8 segment bytes and offsets |
| `purrdf-shapes` / `rule_coordinates` | scalar | `shacl_queries_keep_time_and_distinct_reproducible_seeds_across_focus_nodes`, `parsed_rl_keeps_coordinates_on_both_carriers_and_restores_after_refusal` | Sealed WASM host clock and entropy imports |
| `purrdf-mime` / `lossless` | scalar | `production_rdf_round_trip_is_deterministic_for_original_and_broken_messages` | Pinned native MIME/transcoding byte transcript |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | SIMD | `the_reassociated_path_is_the_one_this_build_was_made_for`, `the_reassociated_distance_is_within_the_error_bound_of_the_exact_one` | Actual WASM dispatch and SIMD kernel |
| `purrdf-hnsw` / `wasm_reassociated` | SIMD | `an_image_recorded_on_another_wasm_path_is_refused_by_name`, `the_image_records_the_path_and_shape_this_build_was_made_for` | Actual WASM arithmetic backend identity |
| `purrdf-hash-conformance` / `hex` | SIMD | `every_path_matches_portable` | Actual WASM SIMD hex kernel |
| `purrdf-hash-conformance` / `blake3` | SIMD | `required_backends_are_available`, `official_modes_and_extended_output`, `xof_large_output_and_seek_boundaries`, `derived_context_trees_match_across_backends`, `random_inputs_cover_irregular_trees_and_alignment` | Actual WASM SIMD BLAKE3 kernels |
| `purrdf-core` / `csv_scan_wasm` | SIMD | `every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length`, `every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs`, `the_target_explicit_kernel_is_among_those_compared` | Actual WASM SIMD CSV kernel |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `selected_backend_is_reported`, `every_kernel_path_encodes_and_decodes_the_same_bytes`, `copy_kernels_match_portable`, `match_length_kernels_match_portable`, `hash_kernels_match_portable` | Actual WASM SIMD DEFLATE kernels |

## Other execution entry points

The geographic qualifier runs complete native targets, but on scalar/SIMD WASM
selects only the named records it compares against frozen native digests:

| Package / target | Exact WASM cases |
| --- | --- |
| `purrdf-geo` / `determinism` | `the_digest_is_the_pinned_golden`, `the_digest_does_not_move_between_runs` |
| `purrdf-geo-kernel` / `cells_determinism` | `public_grid_replay` |
| `purrdf-geo-kernel` / `geo_determinism` | `public_geo_capability_replay` |
| `purrdf-xsd` / `certified_numerical` | `certified_elementary_replay`, `directed_endpoint_product_replay` |

The geographic-errors workflow selects only `public_geo_capability_replay` on
WASM. Coordinate-inputs selects the same host-seal query/rule cases as the table.
The package JavaScript/TypeScript, identity transport, JSPI, Worker and playground
checks exercise shipped host interfaces. The runner fixture exercises actual
WASM imports, traps, rejected options and status propagation. Release/package
builds and assembly-only probes are not semantic test executions.

Names such as `wasm_determinism` or `product_wasm` do not make their entire native
registration a WASM obligation. Unselected general tests remain native. Adding
exceptions by listing generic semantics in this document is forbidden: inspect
the actual platform branch or frozen native identity first.
