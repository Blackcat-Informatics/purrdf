#!/usr/bin/env bash
set -uo pipefail
cd /home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly
export RUSTC_WRAPPER=''
export CARGO_HOME=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/cargo-home
export CARGO_TARGET_DIR=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/target
export CARGO_BUILD_BUILD_DIR=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/build
export TMPDIR=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/tmp
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1
export RUSTFLAGS='-D warnings'
unset CARGO_ENCODED_RUSTFLAGS
unset PURRDF_TEST_REQUIRE_EXACT
exec > .stage/sparql-eval-datalog-per-row-evidence/recovery-native-layout-focused.log 2>&1
set -e
cargo test --locked -p purrdf-sparql-eval --lib solution::tests::unit_sequence_has_one_empty_solution -- --exact
cargo test --locked -p purrdf-sparql-eval --lib workspace::tests::consuming_iterator_drop_keeps_extracted_spilled_row_admitted -- --exact
cargo test --locked -p purrdf-sparql-eval --lib evidence_exec::tests::retained_row_clone_keeps_the_original_dictionary -- --exact
cargo test --locked -p purrdf-sparql-eval --test solution_row_alloc --test join_output_alloc --test prepared_execution --test evidence_ingress
cargo test --locked -p purrdf-retrieval --test real_producers
cargo clippy --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings
