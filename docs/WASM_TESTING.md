<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, arithmetic, refusal corpora and
conformance. `make check` executes their complete registered targets through
`cargo test --workspace --locked`. `make wasm` separately builds the release
crates for `wasm32-unknown-unknown`. `make wasm-pkg-test` exercises the optimized
package, JavaScript bindings and identity ABI; CI also runs the Worker recipe.

`make wasm-test` selects 22 existing named cases in 9 integration targets,
with 27 executions across 13 scalar/SIMD target invocations. Every selection
exercises an actual WASM dispatch path, SIMD kernel, shadow-stack floor or host
interface. Runner preflight additionally exercises panic handling, refused flags
and sealed host reads. General digest vectors, numeric expectations and codec
corpora run in native Rust.

`make geo-determinism` separately executes three bounded geographic law corpora
on native Rust, portable WASM and SIMD WASM: the planar compatibility facade,
the integer cell hierarchy, and completed geodesic, operation, metric, buffer,
cover and index records. Every lane must execute its registered cases, report
the same records and match the frozen digest. Missing prerequisites or a
refused replay fail this gate. These checks establish completed geographic
byte parity. The Rust qualification example also executes the XSD numerical
target on native, portable WASM and SIMD WASM, comparing its frozen arithmetic
records and requiring every registered scratch, rounding, allocation and refusal
case. All twelve target executions must pass; the native conformance gate
remains separate.

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
