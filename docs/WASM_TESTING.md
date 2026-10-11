<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, arithmetic, refusal corpora and
conformance. `make check` executes their complete registered targets through
`cargo test --workspace --locked`. `make wasm` separately builds the release
crates for `wasm32-unknown-unknown`. `make wasm-pkg-test` exercises the optimized
package, JavaScript bindings and identity ABI; CI also runs the Worker recipe.

`make wasm-test` selects 57 named cases in 16 integration targets,
with 62 executions across 20 scalar/SIMD target invocations. Every selection
exercises an actual WASM dispatch path, SIMD kernel, shadow-stack floor or host
interface, except the numeric, sentence, MIME, Turtle-chain and regular-role
portable semantic targets. WebAssembly has 32- and
64-bit integers only, so every `i128` step of the arbitrary-precision numeric
tower and every `u64`/`u128` carry lane under it is lowered differently there; a
lowering bug would bind a different digit rather than crash. Those targets hold
the tower's answers and the evaluator's numeric results — the governor's
refusals among them — to the native ones by a pinned digest both targets
reproduce. The sentence probe reproduces pinned UTF-8 segments and byte offsets
through the public borrowed iterators, including paragraph separators, ignored
characters, abbreviations and multilingual/emoji segments. The full official
sentence corpus remains a native Rust conformance check. The MIME probe reproduces
one native SHA256 transcript of original message octets and emitted Turtle,
N-Triples and JSON-LD bytes, and reconstructs every original through the actual
RDF parser and shared cover decoder. The Turtle-chain target reproduces frozen
native bytes for actual100,000- and1,000,000-node packed, Turtle and TriG routes,
with exact layout, graph and statement-metadata preservation. The regular-role
target executes the same sixteen production reasoning and refusal cases on the
portable build, including recursive and inverse chains, blocking, proof replay,
query services, fixed roles, resource exhaustion and cancellation.
Runner preflight additionally exercises panic handling, refused flags
and sealed host reads. Every other numeric expectation, general digest vectors,
geometry, index determinism and the remaining codec corpora run in native Rust.

After preflight, the lane sets `PURRDF_TEST_REQUIRE_EXACT=1` in the shared Rust
harness. Each invocation must execute one case for every exact filter; missing,
duplicate, ignored or skipped selections refuse the run. Native subprocess
regressions establish this admission rule using the existing harness fixture.

The native owner of each row is `cargo test --locked -p PACKAGE --test TARGET`.
The four WASM-only store/floor cases exercise platform-specific
branches; their native targets exercise filesystem/thread behavior. All other
selected cases also run natively. `scalar + SIMD` means both the baseline and
`+simd128` build; other rows execute only the named build. Full native test
bodies and registrations are retained.

The native `model_traits` target owns the derived Hash-event, Debug-byte (the
whole format-spec matrix), equality and Clone oracles, including 100,000-level
walks on a stated small native stack and against the derive on a large one. Its
one scalar WASM case measures the actual shadow stack: the derive's per-level
growth, which the tested depth would take far past the whole stack, beside the
owned walks' peak, which is the same at 100,000 levels as at eight. It runs those
walks, and 100,000-level drops, inside the scoped floor, checks context restoration and over-floor
admission refusal, and does not repeat semantic vectors or benchmark fixtures.

| Package / target | Build | Exact case | WASM behavior proved |
| --- | --- | --- | --- |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | scalar + SIMD | `the_reassociated_path_is_the_one_this_build_was_made_for` | Selects WasmScalar or WasmSimd128 and records that exact path. |
| `purrdf-sparql-eval` / `knn_wasm_reassociated` | SIMD | `the_reassociated_distance_is_within_the_error_bound_of_the_exact_one` | Calls the actual WasmSimd128 distance kernel and bounded kernel against the exact reference. |
| `purrdf-sparql-eval` / `stack_refusal` | scalar | `prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller` | Refuses evaluation against an installed smaller shadow-stack floor and restores the caller context. |
| `purrdf-core` / `model_traits` | scalar | `owned_term_walks_stay_inside_the_wasm_shadow_stack_floor` | Measures the derive's per-level shadow-stack growth and the owned-term walks' depth-independent peak, runs the 100,000-level walks and drops inside the actual 128 KiB scoped span, restores the caller context, and refuses an over-floor neighbor before its closure runs. |
| `purrdf-rdf` / `turtle_chains` | scalar | `blank_chains_at_one_hundred_thousand`, `blank_chains_at_one_million` | Runs actual packed rendering, native Turtle/default-graph and TriG/named-graph output at both depths against frozen per-route native byte receipts; reparses native output with graph and statement metadata fidelity. |
| `purrdf-rdf` / `turtle_chains` | scalar | `exact_guard_neighbors_and_permuted_interning`, `cycles_shared_and_quoted_blanks_preserve_statements`, `tied_guarded_branches_keep_order_under_permutation`, `a_long_collection_keeps_every_member`, `continuation_objects_saturate_at_the_same_indent_guard` | Checks exact layout around the permanent forty-level guard, input permutation, cyclic/shared/quoted blank identity, continuation indentation and the indexed strict 100,000-member collection route. |
| `purrdf-entail` / `regular_role_chains` | scalar | All sixteen exact production cases registered in `make wasm-test` | Replays the native recursive/inverse role, blocking, proof, query-service, fixed-role, malformed-input, exhaustion and cancellation expectations through the portable reasoning implementation. |
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
| `purrdf-xsd` / `exact_wasm_determinism` | scalar | `the_oracle_vectors_replay_on_this_target` | Replays the 7,164 oracle-generated `xsd:integer`/`xsd:decimal` operator, division, comparison, conversion and canonical-form records through the `XsdValue` operators, byte for byte. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_hand_answers_are_reproduced_on_this_target` | Reproduces hand-checked query answers past `i128`, under two division policies, and the F&O code of an absorbed error. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_numeric_transcript_digest_is_reproduced_on_this_target` | Reproduces the native digest of every numeric query result (arithmetic, all division policies, casts, `ORDER BY`, aggregates, functions, absorbed F&O codes) and the governor's fuel and scratch-byte refusals beside their answering neighbours. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `selected_backend_is_reported` | Asserts Simd128 selection with simd128 and Portable selection without it. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `every_kernel_path_encodes_and_decodes_the_same_bytes` | Explicitly selects the simd128 encoder and decoder and compares their composed output with the portable backend. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `copy_kernels_match_portable` | Calls simd128 overlapping match-copy operations against a bytewise reference. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `match_length_kernels_match_portable` | Calls the simd128 match-length kernel against the portable kernel over vector widths and mismatch positions. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `hash_kernels_match_portable` | Calls the simd128 window-hash kernel against the scalar reference over lengths and offsets. |
| `purrdf-text` / `wasm_determinism` | scalar | `the_sentence_boundaries_are_reproduced_on_this_target` | Reproduces pinned borrowed UTF-8 sentence bytes and offsets from the same versioned Sentence_Break tables, including the alphanumeric filter and empty input. |
| `purrdf-text` / `wasm_determinism` | scalar | `five_thousand_term_index` | Indexes and ranks all 5,000 needle terms, compares full ranking, a bounded heap and every explanation against independent unbounded arithmetic, and carries the prepared score certificate. |
| `purrdf-text` / `wasm_determinism` | scalar | `declared_large_corpora_are_exact` | Scores declared 2^41 and full-u64 populations against independent exact expectations; query-derived bounds and scores exceed the former global score width. |
| `purrdf-text` / `wasm_determinism` | scalar | `promoted_field_arithmetic_is_exact` | Preserves every truncation while maximal nonnegative weights and thirty-two fields promote intermediates through the existing exact integer home; genuine invalid data still refuses. |
| `purrdf-text` / `wasm_determinism` | scalar | `thirty_two_index_fields_are_exact` | Builds and routes thirty-two actual fields through native indexing, ranking and explanation, retaining the exact independent score. |
| `purrdf-mime` / `lossless` | scalar | `production_rdf_round_trip_is_deterministic_for_original_and_broken_messages` | Reproduces the frozen native transcript of message and RDF bytes across all three serializers; reconstructs repeated, nested, multipart and malformed originals through the production RDF parser and byte-cover decoder. |

The hash benchmark targets remain native benchmarks. No benchmark smoke target
runs in the WASM lane; the selected testkit host-clock case exercises its WASM
clock import and filesystem refusal.

Release build, package interfaces, focused execution and assembly codegen are
separate claims. `make simd-asm` retains all seven configurations and its existing
codegen, site and provenance requirements. Successful execution alone does not
establish SIMD code generation.
