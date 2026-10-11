#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
cd "$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
cache="$root/.worktrees/purrdf-shapes-dictionary-qualification"
export CARGO_HOME="$cache/cargo-home" CARGO_TARGET_DIR="$cache/target" CARGO_BUILD_BUILD_DIR="$cache/build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1 RUSTFLAGS='-D warnings'
unset CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
cargo test --locked -p purrdf-sparql-eval --lib workspace::owned_row_regressions
cargo test --locked -p purrdf-sparql-eval --test solution_row_alloc --test join_output_alloc --test evidence_ingress --test prepared_execution
cargo test --locked -p purrdf-text --test physical_shapes
cargo clippy --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings
bash .stage/sparql-eval-datalog-per-row-evidence/recovery-native-control-bench-build.sh
