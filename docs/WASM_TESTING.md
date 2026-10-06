<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, arithmetic, refusal corpora and
conformance. `make check` executes their complete registered targets through
`cargo test --workspace --locked`. `make wasm` separately builds the release
crates for `wasm32-unknown-unknown`. `make wasm-pkg-test` exercises the optimized
package, JavaScript bindings and identity ABI; CI also runs the Worker recipe.

`make wasm-test` selects 27 existing named cases in 11 integration targets,
with 32 executions across 15 scalar/SIMD target invocations. Every selection
exercises an actual WASM dispatch path, SIMD kernel, shadow-stack floor or host
interface, except the two numeric byte-identity targets. WebAssembly has 32- and
64-bit integers only, so every `i128` step of the arbitrary-precision numeric
tower and every `u64`/`u128` carry lane under it is lowered differently there; a
lowering bug would bind a different digit rather than crash. Those targets hold
the tower's answers and the evaluator's numeric results — the governor's
refusals among them — to the native ones by a pinned digest both targets
reproduce. Runner preflight additionally exercises panic handling, refused flags
and sealed host reads. Every other numeric expectation, general digest vectors,
geometry, index determinism and codec corpora run in native Rust.

After preflight, the lane sets `PURRDF_TEST_REQUIRE_EXACT=1` in the shared Rust
harness. Each invocation must execute one case for every exact filter; missing,
duplicate, ignored or skipped selections refuse the run. Native subprocess
regressions establish this admission rule using the existing harness fixture.

The native owner of each row is `cargo test --locked -p PACKAGE --test TARGET`.
The three WASM-only store/floor refusal cases exercise platform-specific
branches; their native targets exercise filesystem/thread behavior. All other
selected cases also run natively. `scalar + SIMD` means both the baseline and
`+simd128` build; other rows execute only the named build. Full native test
bodies and registrations are retained.

| Package / target | Build | Exact case | WASM behavior proved |
| --- | --- | --- | --- |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_path_is_the_one_this_build_was_made_for` | Selects WasmScalar or WasmSimd128 and records that exact path. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | SIMD | `the_reassociated_distance_is_within_the_error_bound_of_the_exact_one` | Calls the actual WasmSimd128 distance kernel and bounded kernel against the exact reference. |
| `purrdf-sparql-eval` / `stack_refusal` | scalar | `prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller` | Refuses evaluation against an installed smaller shadow-stack floor and restores the caller context. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `an_image_recorded_on_another_wasm_path_is_refused_by_name` | Refuses an image naming a different WASM arithmetic path with its typed admission error. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `the_image_records_the_path_and_shape_this_build_was_made_for` | Records wasm32 and the actual SIMD feature bit and arithmetic path in the image. |
| `purrdf-hash-conformance` / `hex` | SIMD | `every_path_matches_portable` | Explicitly calls the wasm simd128 encoder over lengths, alignments and write boundaries and asserts its selection. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `required_backends_are_available` | Asserts Wasm128 selection with simd128 and Portable selection without it. |
| `purrdf-hash-conformance` / `blake3` | SIMD | `random_inputs_cover_irregular_trees_and_alignment` | Explicitly calls Wasm128 one-shot and streaming kernels against the portable kernel over irregular trees, chunk schedules and alignments. |
| `purrdf-testkit` / `bench` | scalar | `the_store_options_are_refused_on_wasm32` | Refuses filesystem-backed benchmark storage on wasm32. |
| `purrdf-testkit` / `bench` | scalar | `a_measured_run_reports_its_estimates` | Runs measurements using the imported WASM host clock and prints estimates without filesystem storage. |
| `purrdf-stack` / `on_stack` | scalar | `an_in_floor_request_runs` | Runs inline within an installed WASM shadow-stack floor. |
| `purrdf-stack` / `on_stack` | scalar | `a_scoped_in_floor_request_borrows_the_caller_s_locals` | Runs a borrowing scoped call inline within an installed WASM shadow-stack floor. |
| `purrdf-stack` / `on_stack` | scalar | `an_over_floor_request_is_refused` | Refuses a request exceeding the installed WASM shadow-stack floor. |
| `purrdf-stack` / `on_stack` | scalar | `an_over_floor_scoped_request_is_refused` | Refuses a scoped request exceeding the installed WASM shadow-stack floor. |
| `purrdf-core` / `csv_scan_wasm` | SIMD | `every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length` | Explicitly calls the simd128 field scanner against a bytewise reference over vector widths and alignments. |
| `purrdf-core` / `csv_scan_wasm` | SIMD | `every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs` | Explicitly calls the simd128 field scanner against a bytewise reference over seeded byte buffers. |
| `purrdf-core` / `csv_scan_wasm` | SIMD | `the_target_explicit_kernel_is_among_those_compared` | Asserts that simd128 is included in the compared kernels and is the selected backend. |
| `purrdf-xsd` / `exact_wasm_determinism` | scalar | `the_hand_values_are_reproduced_on_this_target` | Reproduces values checkable by hand (34!, the exact decimal of the smallest subnormal, `1/3` at eighteen digits, the binary value of `0.1f32`) on the 32-bit lowering of the tower. |
| `purrdf-xsd` / `exact_wasm_determinism` | scalar | `the_transcript_digest_is_reproduced_on_this_target` | Reproduces the native digest of a seeded transcript of every tower operation over operands from one digit to hundreds. |
| `purrdf-xsd` / `exact_wasm_determinism` | scalar | `the_oracle_vectors_replay_on_this_target` | Replays the 7,960 oracle-generated `xsd:integer`/`xsd:decimal` operator, division (three policies), comparison, conversion and canonical-form records through the `XsdValue` operators, byte for byte. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_hand_answers_are_reproduced_on_this_target` | Reproduces hand-checked query answers past `i128`, under two division policies, and the F&O code of an absorbed error. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_numeric_transcript_digest_is_reproduced_on_this_target` | Reproduces the native digest of every numeric query result (arithmetic, all division policies, casts, `ORDER BY`, aggregates, functions, absorbed F&O codes) and the governor's fuel and scratch-byte refusals beside their answering neighbours. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `selected_backend_is_reported` | Asserts Simd128 selection with simd128 and Portable selection without it. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `every_kernel_path_encodes_and_decodes_the_same_bytes` | Explicitly selects the simd128 encoder and decoder and compares their composed output with the portable backend. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `copy_kernels_match_portable` | Calls simd128 overlapping match-copy operations against a bytewise reference. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `match_length_kernels_match_portable` | Calls the simd128 match-length kernel against the portable kernel over vector widths and mismatch positions. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `hash_kernels_match_portable` | Calls the simd128 window-hash kernel against the scalar reference over lengths and offsets. |

The hash benchmark targets remain native benchmarks. No benchmark smoke target
runs in the WASM lane; the selected testkit host-clock case exercises its WASM
clock import and filesystem refusal.

Release build, package interfaces, focused execution and assembly codegen are
separate claims. `make simd-asm` retains all seven configurations and its existing
codegen, site and provenance requirements. Successful execution alone does not
establish SIMD code generation.
