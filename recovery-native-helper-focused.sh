#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
cd "$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
cache="$root/.worktrees/purrdf-shapes-dictionary-qualification"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
export CARGO_HOME="$cache/cargo-home" CARGO_TARGET_DIR="$cache/target" CARGO_BUILD_BUILD_DIR="$cache/build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1 RUSTFLAGS='-D warnings'
unset CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
python3 scripts/check-shared-helpers.py
cargo clippy --locked -p purrdf-core -p purrdf-sparql-algebra -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings
cargo test --locked -p purrdf-core --lib evidence::
cargo test --locked -p purrdf-sparql-algebra --lib
cargo test --locked -p purrdf-sparql-eval --lib
cargo test --locked -p purrdf-sparql-eval --test solution_row_alloc --test join_output_alloc --test evidence_ingress --test prepared_execution
cargo test --locked -p purrdf-text --test physical_shapes
cargo test --locked -p purrdf-retrieval --test real_producers
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh"
cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test evidence_ingress
cargo test --locked --target wasm32-unknown-unknown -p purrdf-text --test physical_shapes
cargo test --locked --target wasm32-unknown-unknown -p purrdf-retrieval --test real_producers
