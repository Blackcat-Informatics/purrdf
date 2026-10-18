<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native ownership and WASM execution

Native Rust owns general semantics, grammar, arithmetic, refusal corpora and
conformance. `make check` executes their complete registered targets through
`cargo test --workspace --locked`. `make wasm` separately builds the release
crates for `wasm32-unknown-unknown`. `make wasm-pkg-test` exercises the optimized
package, JavaScript bindings and identity ABI; CI also runs the Worker recipe.

`make wasm-test` selects 179 named cases in 30 integration targets,
with 189 executions across 35 scalar/SIMD target invocations. Every selection
exercises an actual WASM dispatch path, SIMD kernel, shadow-stack floor or host
interface, except the numeric, sentence and MIME byte-identity targets and the
Turtle-chain, regular-role, evidence, real-producer, physical-shape and graph-free
carrier semantic acceptance targets. The evidence and scoped retrieval cases exercise the original
admitted owners, four-valued evidence, exact aggregation and refusal paths on the
32-bit target; the scoped text cases retain original candidate rows and scores.
The query-local producer cases exercise borrowed snapshot retirement, in-pull
work and cancellation stops, original source-failure precedence, physical
cell-admission refusal and retained complete/partial rows. Borrowed native text
registrations reproduce their owned ranked and positional neighbors exactly.
The twenty-one real-producer cases exercise both shipped text and kNN strata,
pre-depth membership, unknown receipts, generation identity, native admission
refusal, independent candidate depths, original fusion evidence, retained fused
acceptance and typed terminal refusal during suffix settlement.
WebAssembly has 32- and
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
target executes the same seventeen production reasoning and refusal cases on the
portable build, including recursive and inverse chains, blocking, proof replay,
query services, fixed roles, resource exhaustion and cancellation.
Runner preflight additionally exercises panic handling, refused flags
and sealed host reads. Other numeric expectations, general digest vectors and
the remaining codec corpora run in native Rust.

`make geo-determinism` separately executes four complete replay targets on native,
portable WASM and SIMD WASM: `purrdf-geo/determinism`,
`purrdf-geo-kernel/cells_determinism`, `purrdf-geo-kernel/geo_determinism` and
`purrdf-xsd/certified_numerical`. Its twelve executions compare the frozen
completed-output digests and require every registered case to run. The records
cover certified numerical endpoints, cell hierarchy, geographic operations,
metrics, buffers, covers and indexes. The `geo-determinism` CI job runs this
comparison; the Arm64, i686 and i586 jobs execute the same replay targets on
their native architectures. These executions are separate from the named
`make wasm-test` selections and counts above.

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
| `purrdf-entail` / `regular_role_chains` | scalar | All seventeen exact production cases registered in `make wasm-test` | Replays the native recursive/inverse role, blocking, proof, query-service, fixed-role, malformed-input, source-witness retention, exhaustion and cancellation expectations through the portable reasoning implementation. |
| `purrdf-entail` / `recorded_stop` | scalar | All seven exact cases registered in `make wasm-test` | Checks one caller signal through canonical identity, every observed ontology-preparation poll and decision search; preserves completed proof identity/replay and selected preparation ceilings, and reports cancellation without a semantic verdict. |
| `purrdf-entail` / `current_clash_support` | scalar | All fourteen exact cases registered in `make wasm-test` | Checks source-bound ordinary and causal replay, named/scoped-blank identities and disconnected controls, public service/certificate parity, generated existential nodes, exhaustive nominal merge branches and fresh refutation assumptions through the shared support arena. Selected transitive paths retain both signed edges and original subproperty, inverse and transitivity licenses. Key-equality support retains typed, language-tagged and directional literal metadata after wire decoding. Reserved births, including recursive reserved origins, preserve their typed original descriptors and actual generative predecessor licenses after wire decoding. Independent borrowed consumer contexts replay repeatedly with owning-context parity and leave the prepared reasoner reusable. |
| `purrdf-validate` / `dl_proof_vectors` | scalar | `all_shared_dl_proof_vectors_match_the_current_native_boundary` | Runs all seventeen shared service/schema proof vectors through the same producer/checker invoked by the packaged WASM golden entry, including exact canonical proof bytes and replay reports. The recursive nominal vector additionally executes public causal capture and inspects its original nested reserved descriptor through this same packaged entry. |
| `purrdf-validate` / `dl_proof_vectors` | scalar | `imported_closure_proofs_check_against_the_consumers_native_snapshot` | Checks independently loaded imported-source proofs against the consumer's original snapshot, including an IRI, document-scoped equal blank labels and role-derived membership; rejects changed source, wrong verdict and altered proof. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `an_image_recorded_on_another_wasm_path_is_refused_by_name` | Refuses an image naming a different WASM arithmetic path with its typed admission error. |
| `purrdf-hnsw` / `wasm_reassociated` | scalar + SIMD | `the_image_records_the_path_and_shape_this_build_was_made_for` | Records wasm32 and the actual SIMD feature bit and arithmetic path in the image. |
| `purrdf-hash-conformance` / `hex` | SIMD | `every_path_matches_portable` | Explicitly calls the wasm simd128 encoder over lengths, alignments and write boundaries and asserts its selection. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `required_backends_are_available` | Asserts Wasm128 selection with simd128 and Portable selection without it. |
| `purrdf-hash-conformance` / `blake3` | scalar + SIMD | `official_modes_and_extended_output`, `xof_large_output_and_seek_boundaries`, `derived_context_trees_match_across_backends` | Matches official mode vectors, extended-output and seek boundaries, and derived-context trees across the available backends. |
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
| `purrdf-xsd` / `any_uri` | scalar | `lexical_mapping_preserves_everything_except_xml_whitespace`, `primitive_identity_is_disjoint_and_unordered`, `ranges_use_collapsed_character_lengths_and_primitive_identity` | Preserves URI whitespace mapping, primitive-domain identity and ordering refusal, and collapsed-character length constraints. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_hand_answers_are_reproduced_on_this_target` | Reproduces hand-checked query answers past `i128`, under two division policies, and the F&O code of an absorbed error. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `borrowed_value_equality_matches_the_query_operator_on_this_target` | Compares original borrowed RDF values against actual query equality, preserving numeric, scope, language/direction, triple/composite and expression-error distinctions. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `any_uri_casts_keep_their_primitive_domain_on_this_target` | Preserves native URI whitespace collapse and string/self casts while refusing casts into unrelated primitive domains through the governed query entry point. |
| `purrdf-sparql-eval` / `numeric_wasm_determinism` | scalar | `the_numeric_transcript_digest_is_reproduced_on_this_target` | Reproduces the native digest of every numeric query result (arithmetic, all division policies, casts, `ORDER BY`, aggregates, functions, absorbed F&O codes) and the governor's fuel and scratch-byte refusals beside their answering neighbours. |
| `purrdf-deflate` / `deflate_conformance` | scalar + SIMD | `selected_backend_is_reported` | Asserts Simd128 selection with simd128 and Portable selection without it. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `every_kernel_path_encodes_and_decodes_the_same_bytes` | Explicitly selects the simd128 encoder and decoder and compares their composed output with the portable backend. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `copy_kernels_match_portable` | Calls simd128 overlapping match-copy operations against a bytewise reference. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `match_length_kernels_match_portable` | Calls the simd128 match-length kernel against the portable kernel over vector widths and mismatch positions. |
| `purrdf-deflate` / `deflate_conformance` | SIMD | `hash_kernels_match_portable` | Calls the simd128 window-hash kernel against the scalar reference over lengths and offsets. |
| `purrdf-text` / `wasm_determinism` | scalar | `the_sentence_boundaries_are_reproduced_on_this_target` | Reproduces pinned borrowed UTF-8 sentence bytes and offsets from the same versioned Sentence_Break tables, including the alphanumeric filter and empty input. |
| `purrdf-text` / `wasm_determinism` | scalar | `delta_updates_reproduce_fresh_ranked_and_surface_answers_on_this_target` | Compares public delta updates with fresh construction, including ranked and surface evidence, empty results and nonliteral source coverage. |
| `purrdf-text` / `wasm_determinism` | scalar | `five_thousand_term_index` | Indexes and ranks all 5,000 needle terms, compares full ranking, a bounded heap and every explanation against independent unbounded arithmetic, and carries the prepared score certificate. |
| `purrdf-text` / `wasm_determinism` | scalar | `declared_large_corpora_are_exact` | Scores declared 2^41 and full-u64 populations against independent exact expectations; query-derived bounds and scores exceed the former global score width. |
| `purrdf-text` / `wasm_determinism` | scalar | `promoted_field_arithmetic_is_exact` | Preserves every truncation while maximal nonnegative weights and thirty-two fields promote intermediates through the existing exact integer home; genuine invalid data still refuses. |
| `purrdf-text` / `wasm_determinism` | scalar | `thirty_two_index_fields_are_exact` | Builds and routes thirty-two actual fields through native indexing, ranking and explanation, retaining the exact independent score. |
| `purrdf-mime` / `lossless` | scalar | `production_rdf_round_trip_is_deterministic_for_original_and_broken_messages` | Reproduces the frozen native transcript of message and RDF bytes across all three serializers; reconstructs repeated, nested, multipart and malformed originals through the production RDF parser and byte-cover decoder. |
| `purrdf-sparql-eval` / `evidence_ingress` | scalar | All twenty exact cases registered in the original target | Preserves polarity, closure, unknown status, scope, ledger cells and accepted ordering through actual operators and query forms; native empty, late-emitted and unclosed whole-search declarations retain their original owners and admitted COUNT bounds through EXISTS and ASK functions without an external evidence source. Thirty-six actual empty-producer combinations cover complete/open/incomplete search, explicit source scope, positive EXISTS, NOT EXISTS and nested MINUS: only closed complete absence decides the predicate; unresolved membership remains enumerable. Producer refusal prevents partial publication. |
| `purrdf-core` / `triple_carrier` | scalar | All six exact cases selected in `make wasm-test` | Pins 12/8/12-byte row layouts, all bound patterns and lazy side indexes, lexical/statement/canonical identity, named-role refusal and shared first-cause/fresh-read behavior. |
| `purrdf-sparql-eval` / `statement_indexes` | scalar + SIMD | `native_and_compact_statement_queries_keep_answers_and_candidate_work`, `named_graph_refusal_precedes_index_allocation_and_keeps_the_first_cause` | Compares native and compact statement-query answers and candidate work, and checks named-graph refusal before lazy index allocation with the original cause retained. |
| `purrdf-datalog` / `rules_runtime` | scalar | The five selected `triple_` cases | Exercises actual predicate-only keys, original wide host IDs, compiler/chase/proof, immutable Base through retraction and NoOutput semantics. |
| `purrdf-sparql-eval` / `host_identity` | scalar | The three selected `triple_` cases | Checks typed named-graph refusal before nested/empty shortcuts and actual default-query bags, unbound cells, paths and graph output. |
| `purrdf-shapes` / `srl_language` | scalar | `triple_carrier_runs_the_public_rl_program_without_quad_storage`, `triple_rl_keeps_statement_roles_and_scoped_blank_identity` | Runs the public compact SRL entry, pins quad-model parity and compact result tables, and preserves scoped blank and statement roles. |
| `purrdf-text` / `physical_shapes` | scalar | All twenty-one exact cases registered in the original target | Compares dictionary, ordered-window and bucket execution with literal execution, exercises original owner lifetimes and admission refusal, and retains exact bytes and work bounds. |
| `purrdf-retrieval` / `real_producers` | scalar | All twenty-one exact cases registered in the original target | Exercises the shipped text and kNN strata, scoped ranks, retained fused acceptance without another producer call, and typed terminal suffix-settlement refusal with original owner release. |
| `purrdf-sparql-eval` / `query_completion` | scalar | The six `borrowed_*` cases selected in the Makefile | Checks original snapshot identity, cursor survival after registry-handle release, work/cancellation inside cursor pulls, operational precedence, bounded cell birth, and complete/partial row ownership after scope teardown. |
| `purrdf-text` / `search_property_function` | scalar | `borrowed_native_ranked_and_positional_producers_keep_their_rows` | Replays borrowed native ranked and positional registration against exact owned-registration answers. |
| `purrdf-text` / `wasm_determinism` | scalar | `scoped_text_bound_candidates_retain_unknown_membership`, `scoped_text_depth_preserves_original_unknown_rows_and_scores`, `scoped_text_preserves_original_constraint_failure` | Retains original scoped candidate membership, unknown rows, scores and constraint failures through production text retrieval. |

The hash benchmark targets remain native benchmarks. No benchmark smoke target
runs in the WASM lane; the selected testkit host-clock case exercises its WASM
clock import and filesystem refusal.

Release build, package interfaces, focused execution and assembly codegen are
separate claims. `make simd-asm` retains all seven configurations and its existing
codegen, site and provenance requirements. Successful execution alone does not
establish SIMD code generation.
