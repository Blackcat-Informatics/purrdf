<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, refusal corpora and conformance.
`make check` executes their complete registered targets through
`cargo test --workspace --locked`. `make wasm` separately proves the release
crates build for `wasm32-unknown-unknown`. `make wasm-pkg-test` proves the
optimized package, JavaScript bindings and ABI.

`make wasm-test` executes only the target obligations below: 55 named cases
in 18 integration targets, with 87 case executions across 26 scalar/SIMD
target invocations. Runner preflight is additional host-boundary evidence.
The Rust `wasm-focused-tests` host binary in `crates/wasm-link` compares each
exact filtered `--list` with the
requested names and count before executing it; missing names fail. The existing
testkit harness and full native registrations retain their behavior.

The native owner of each row is `cargo test --locked -p PACKAGE --test TARGET`.
The three WASM-only store/floor refusal cases exercise platform-specific
branches; their native targets exercise the native filesystem/thread behavior.
All other retained cases, including compact arithmetic/scanner probes, also
run natively. `scalar + SIMD` means both the baseline and `+simd128` build;
`scalar` means the baseline build only.

| Package / target | Build | Exact case | Target behavior proved |
| --- | --- | --- | --- |
| `purrdf-sparql-eval` / `knn_wasm_determinism` | scalar + SIMD | `the_lane_tree_cosine_answer_is_reproduced_on_this_target` | Wasm binary64 arithmetic/lane packing against pinned distance lexicals; see named metric in each case. |
| `purrdf-sparql-eval` / `knn_wasm_determinism` | scalar + SIMD | `the_lane_tree_squared_euclidean_answer_is_reproduced_on_this_target` | Wasm binary64 arithmetic/lane packing against pinned distance lexicals; see named metric in each case. |
| `purrdf-sparql-eval` / `knn_wasm_determinism` | scalar + SIMD | `the_pinned_answer_is_reproduced_on_this_target` | Wasm binary64 arithmetic/lane packing against pinned distance lexicals; see named metric in each case. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_distance_is_within_the_error_bound_of_the_exact_one` | Actual WasmScalar/WasmSimd128 path, numerical error bounds, overflow refusal and relation reachability; each named boundary is target sensitive. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_distance_refuses_an_overflow` | Actual WasmScalar/WasmSimd128 path, numerical error bounds, overflow refusal and relation reachability; each named boundary is target sensitive. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_path_is_the_one_this_build_was_made_for` | Actual WasmScalar/WasmSimd128 path, numerical error bounds, overflow refusal and relation reachability; each named boundary is target sensitive. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_relation_constructs_and_ranks_within_the_error_bound_of_the_exact_one` | Actual WasmScalar/WasmSimd128 path, numerical error bounds, overflow refusal and relation reachability; each named boundary is target sensitive. |
| `purrdf-sparql-eval` / `stack_refusal` | scalar | `prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller` | Actual Wasm shadow-stack floor refusal and restored caller context; baseline alone suffices. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `a_payload_one_distance_bit_away_does_not_verify` | Build-specific Wasm floating kernel, distance bits, image path/shape ABI, restore/search and foreign-path refusal. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `an_image_recorded_on_another_wasm_path_is_refused_by_name` | Build-specific Wasm floating kernel, distance bits, image path/shape ABI, restore/search and foreign-path refusal. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `the_image_decodes_verifies_and_searches_as_the_kernel_ranks` | Build-specific Wasm floating kernel, distance bits, image path/shape ABI, restore/search and foreign-path refusal. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `the_image_records_the_path_and_shape_this_build_was_made_for` | Build-specific Wasm floating kernel, distance bits, image path/shape ABI, restore/search and foreign-path refusal. |
| `purrdf-text` / `wasm_determinism` | scalar | `the_independent_fielded_reference_is_reproduced_on_this_target` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-text` / `wasm_determinism` | scalar | `the_integer_logarithm_agrees_with_its_hand_values_on_this_target` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-text` / `wasm_determinism` | scalar | `the_pinned_ranking_is_reproduced_on_this_target` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-retrieval` / `wasm_determinism` | scalar | `a_collided_pair_fuses_to_the_same_order_on_both_targets` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-retrieval` / `wasm_determinism` | scalar | `a_fused_answer_is_the_same_rows_in_the_same_order_on_both_targets` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-retrieval` / `wasm_determinism` | scalar | `a_unit_weight_contribution_is_the_same_exact_decimal_on_both_targets` | Wasm i128 helper arithmetic, exact truncation, scores/order and decimal lexical rendering against pinned/hand expectations. |
| `purrdf-shapes` / `product_wasm` | scalar | `encoding_matches_the_committed_bytes_on_this_target` | Cross-width prepared-product wire ABI; encoding matches native committed bytes, and native bytes restore through the Wasm reader. |
| `purrdf-shapes` / `product_wasm` | scalar | `the_committed_golden_restores_on_this_target` | Cross-width prepared-product wire ABI; encoding matches native committed bytes, and native bytes restore through the Wasm reader. |
| `purrdf-hash-conformance` / `digest_differential` | scalar | `integer_digest_lowering_matches_frozen_boundaries` | Compact independent-answer block/padding boundaries for Wasm i64 rotation and length arithmetic, retaining full digest conformance natively. |
| `purrdf-hash-conformance` / `hex` | scalar + SIMD | `every_path_matches_portable` | Actual i8x16.swizzle encoder versus portable over all lengths/alignment/case/write boundaries, plus required backend selection. |
| `purrdf-hash-conformance` / `hex` | scalar + SIMD | `required_paths_are_available_and_selected` | Actual i8x16.swizzle encoder versus portable over all lengths/alignment/case/write boundaries, plus required backend selection. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `required_backends_are_available` | Explicit Wasm128 versus portable kernels across independent frozen answers, irregular trees, alignment, chunk schedules and buffer boundaries. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `random_inputs_cover_irregular_trees_and_alignment` | Explicit Wasm128 versus portable kernels across independent frozen answers, irregular trees, alignment, chunk schedules and buffer boundaries. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `streaming_boundary_answers` | Explicit Wasm128 versus portable kernels across independent frozen answers, irregular trees, alignment, chunk schedules and buffer boundaries. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `frozen_answers_match` | Explicit Wasm128 versus portable kernels across independent frozen answers, irregular trees, alignment, chunk schedules and buffer boundaries. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `streaming_answers_match` | Explicit Wasm128 versus portable kernels across independent frozen answers, irregular trees, alignment, chunk schedules and buffer boundaries. |
| `purrdf-hash-conformance` / `fixed_hasher` | scalar | `portable_vectors_are_reproduced` | Wasm 32-bit target selects four partial 32x32 products instead of widening native u128 multiply; usize hashing zero extends to u64. |
| `purrdf-hash-conformance` / `fixed_hasher` | scalar | `the_selected_function_answers_its_own_vectors` | Wasm 32-bit target selects four partial 32x32 products instead of widening native u128 multiply; usize hashing zero extends to u64. |
| `purrdf-hash-conformance` / `fixed_hasher` | scalar | `integers_share_one_word` | Wasm 32-bit target selects four partial 32x32 products instead of widening native u128 multiply; usize hashing zero extends to u64. |
| `purrdf-hash-conformance` / `splitmix_fnv` | scalar | `splitmix64_lowering_matches_boundary_vectors` | Small frozen records for wrapping u64 seed/add/multiply/shift boundaries; full streams remain native. |
| `purrdf-hash-conformance` / `splitmix_fnv` | scalar | `fnv1a64_lowering_matches_boundary_vectors` | Small frozen wrapping u64 multiply/fold answers; full Unicode/IRI corpus remains native. |
| `purrdf-testkit` / `bench` | scalar | `the_store_options_are_refused_on_wasm32` | Actual Wasm host-clock measurement and cfg-gated filesystem/store refusal; generic statistics/CLI/record semantics native. |
| `purrdf-testkit` / `bench` | scalar | `a_measured_run_reports_its_estimates` | Actual Wasm host-clock measurement and cfg-gated filesystem/store refusal; generic statistics/CLI/record semantics native. |
| `purrdf-stack` / `on_stack` | scalar | `an_in_floor_request_runs` | Wasm inline/scoped shadow-stack floor and typed over-floor refusal; native uses independent threads. |
| `purrdf-stack` / `on_stack` | scalar | `a_scoped_in_floor_request_borrows_the_caller_s_locals` | Wasm inline/scoped shadow-stack floor and typed over-floor refusal; native uses independent threads. |
| `purrdf-stack` / `on_stack` | scalar | `an_over_floor_request_is_refused` | Wasm inline/scoped shadow-stack floor and typed over-floor refusal; native uses independent threads. |
| `purrdf-stack` / `on_stack` | scalar | `an_over_floor_scoped_request_is_refused` | Wasm inline/scoped shadow-stack floor and typed over-floor refusal; native uses independent threads. |
| `purrdf-core` / `csv_scan_wasm` | scalar + SIMD | `every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length` | Actual simd128 i8x16.eq field kernel versus per-byte reference; explicit presence/selection assertions guard backend execution. |
| `purrdf-core` / `csv_scan_wasm` | scalar + SIMD | `every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs` | Actual simd128 i8x16.eq field kernel versus per-byte reference; explicit presence/selection assertions guard backend execution. |
| `purrdf-core` / `csv_scan_wasm` | scalar + SIMD | `the_target_explicit_kernel_is_among_those_compared` | Actual simd128 i8x16.eq field kernel versus per-byte reference; explicit presence/selection assertions guard backend execution. |
| `purrdf-core` / `segmented` | scalar | `indexed_reopen_preserves_high_ids_and_bidirectional_batches` | 32-bit Wasm must retain IDs above 2^54 and portable wire charges; actual 32-bit heap allocations must fit the common conservative ledger. |
| `purrdf-core` / `segmented` | scalar | `charged_read_peak_covers_measured_allocations_including_streamed_output` | 32-bit Wasm must retain IDs above 2^54 and portable wire charges; actual 32-bit heap allocations must fit the common conservative ledger. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `required_paths_are_available` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `the_selected_path_is_available` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `every_kernel_path_encodes_and_decodes_the_same_bytes` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `copy_kernels_match_portable` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `match_length_kernels_match_portable` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `match_length_paths_reproduce_the_vectors` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `hash_kernels_match_portable` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `selected_backend_is_reported` | Actual simd128 copy/match/hash kernels and composition against portable/bytewise/frozen answers, plus backend presence/selection. |
| `purrdf-lex` / `frozen_vectors` | scalar + SIMD | `needle_searches_replay_the_frozen_vectors` | Actual ByteClass/find_byte/find_byte2 packed scanner answers; grammar and Unicode corpora remain native. |
| `purrdf-lex` / `frozen_vectors` | scalar + SIMD | `json_byte_scans_match_scalar_on_this_target` | Production JSON scanner versus scalar oracle across SIMD vector alignments, stops and tails; full Unicode/escape grammar native. |
| `purrdf-lex` / `frozen_vectors` | scalar + SIMD | `percent_byte_scans_match_scalar_on_this_target` | Production percent encoder scans for existing encode sets versus scalar oracle at SIMD vector seams; Unicode-plane corpus native. |

General query completion, join scaling, ordered JSON reconstruction/metadata/
vocabulary refusal, length framing, SHACL lifecycle/corpus, benchmark statistics
and storage conformance stay in their native targets. The hash benchmark targets
remain native benchmarks; WASM host-clock execution and actual kernel comparisons
are covered by the named cases above. General Unicode/escape grammar remains in
the native lexical corpus; the WASM subset exercises production byte scanners.

Release build, package execution, focused target execution and assembly codegen
are separate claims. `make simd-asm` measures all seven configurations under its
unchanged codegen/site/provenance requirements; successful tests do not establish
SIMD code generation or completion within the hosted job limit.
