# Native Rust owns semantic/conformance corpora. This lane executes only the
# named WASM behaviors documented in docs/WASM_TESTING.md: actual dispatch,
# SIMD kernels, shadow-stack floors and host clock/storage refusal. Full test
# targets remain registered under cargo test --workspace.
# Runner preflight observes planted panic, refused flags and sealed host reads.
# +simd128 is target codegen, not a Cargo feature. Target-scoped flags replace
# build.rustflags and are ignored when RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS is
# set, so fold/unset RUSTFLAGS, restate -D warnings, and refuse encoded flags.
if [ -n "${CARGO_ENCODED_RUSTFLAGS:-}" ]; then \
	echo "FAIL: CARGO_ENCODED_RUSTFLAGS is set; Cargo would ignore the +simd128 run's target-scoped flags and run the baseline build twice"; exit 1; \
fi
if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then \
	if [ -n "${CI:-}" ]; then echo "FAIL: wasm32-unknown-unknown target absent in CI"; exit 1; fi; \
	if ! command -v rustup >/dev/null 2>&1; then \
		echo "SKIP: rustup is not on PATH — the wasm32 target cannot be detected or installed ('make doctor')"; \
	else \
		echo "SKIP: wasm32-unknown-unknown target not installed — 'rustup target add wasm32-unknown-unknown' to enable"; \
	fi; \
elif ! command -v wasm-bindgen >/dev/null 2>&1; then \
	if [ -n "${CI:-}" ]; then echo "FAIL: the wasm-bindgen CLI is absent in CI"; exit 1; fi; \
	echo "SKIP: the wasm-bindgen CLI is not on PATH — install wasm-bindgen-cli $(sed -n 's/^wasm-bindgen = \"=\([0-9][0-9.]*\)\"$/\1/p' Cargo.toml) to enable"; \
elif ! command -v node >/dev/null 2>&1; then \
	if [ -n "${CI:-}" ]; then echo "FAIL: node absent in CI"; exit 1; fi; \
	echo "SKIP: node not on PATH — the wasm test harness runs the module in Node"; \
else \
	bash scripts/check-wasm-test-runner.sh \
	&& export PURRDF_TEST_REQUIRE_EXACT=1 \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-gts --test mmr_preimages -- --exact \
			streamed_mmr_roots_preserve_frozen_proofs_and_variable_frame_ids \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-rdf --test turtle_chains -- --exact \
			blank_chains_at_one_hundred_thousand \
			blank_chains_at_one_million \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test knn_wasm_reassociated -- --exact \
			the_reassociated_path_is_the_one_this_build_was_made_for \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test stack_refusal -- --exact \
			prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-core --test model_traits -- --exact \
			owned_term_walks_stay_inside_the_wasm_shadow_stack_floor \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test wasm_reassociated -- --exact \
			an_image_recorded_on_another_wasm_path_is_refused_by_name \
			the_image_records_the_path_and_shape_this_build_was_made_for \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-columnar --test golden_bytes -- --exact \
			seeded_files_are_the_pinned_bytes \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test blake3 -- --exact \
			required_backends_are_available \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-testkit --test bench -- --exact \
			the_store_options_are_refused_on_wasm32 \
			a_measured_run_reports_its_estimates \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-stack --test on_stack -- --exact \
			an_in_floor_request_runs \
			a_scoped_in_floor_request_borrows_the_caller_s_locals \
			an_over_floor_request_is_refused \
			an_over_floor_scoped_request_is_refused \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-deflate --test deflate_conformance -- --exact \
			selected_backend_is_reported \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-xsd --test exact_wasm_determinism -- --exact \
			the_transcript_digest_is_reproduced_on_this_target \
			large_binary_products_match_the_schoolbook_oracle \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test query_coordinates -- --exact \
			supplied_inputs_repeat_complete_results_without_host_reads \
			supplied_large_fraction_and_timezone_survive_repeated_now \
			prepared_executions_select_fresh_inputs_without_changing_the_plan \
			bounded_coordinate_owners_price_real_allocations_and_release_after_last_output \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test numeric_wasm_determinism -- --exact \
			the_hand_answers_are_reproduced_on_this_target \
			the_numeric_transcript_digest_is_reproduced_on_this_target \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-text --test wasm_determinism -- --exact \
			the_sentence_boundaries_are_reproduced_on_this_target \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-shapes --test rule_coordinates -- --exact \
			shacl_queries_keep_time_and_distinct_reproducible_seeds_across_focus_nodes \
			parsed_rl_keeps_coordinates_on_both_carriers_and_restores_after_refusal \
	&& CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-mime --test lossless -- --exact \
			production_rdf_round_trip_is_deterministic_for_original_and_broken_messages \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test knn_wasm_reassociated -- --exact \
			the_reassociated_path_is_the_one_this_build_was_made_for \
			the_reassociated_distance_is_within_the_error_bound_of_the_exact_one \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test wasm_reassociated -- --exact \
			an_image_recorded_on_another_wasm_path_is_refused_by_name \
			the_image_records_the_path_and_shape_this_build_was_made_for \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test hex -- --exact \
			every_path_matches_portable \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-hash-conformance --test blake3 -- --exact \
			required_backends_are_available \
			official_modes_and_extended_output \
			xof_large_output_and_seek_boundaries \
			derived_context_trees_match_across_backends \
			random_inputs_cover_irregular_trees_and_alignment \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-core --test csv_scan_wasm -- --exact \
			every_kernel_agrees_with_the_per_byte_scan_at_every_alignment_and_length \
			every_kernel_agrees_with_the_per_byte_scan_over_seeded_inputs \
			the_target_explicit_kernel_is_among_those_compared \
	&& env -u RUSTFLAGS \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128" \
		CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/683-purrdf-text-a-textindex-delta-update/scripts/wasm-test-runner.sh \
		cargo test --locked --target wasm32-unknown-unknown -p purrdf-deflate --test deflate_conformance -- --exact \
			selected_backend_is_reported \
			every_kernel_path_encodes_and_decodes_the_same_bytes \
			copy_kernels_match_portable \
			match_length_kernels_match_portable \
			hash_kernels_match_portable; \
fi
