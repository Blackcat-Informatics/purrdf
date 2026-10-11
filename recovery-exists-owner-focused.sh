#!/usr/bin/env bash
set -euo pipefail
worktree=/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence
cache=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification
cd "$worktree"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
export CARGO_HOME="$cache/cargo-home" CARGO_TARGET_DIR="$worktree/target" CARGO_BUILD_BUILD_DIR="$worktree/target/native-build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
cargo clippy --locked -p purrdf-sparql-eval --all-targets -- -D warnings
cargo test --locked -p purrdf-sparql-eval --lib
cargo test --locked -p purrdf-sparql-eval --test evidence_ingress --test prepared_execution --test solution_row_alloc
cargo test --locked -p purrdf-capi --test native_profile
cargo test --locked --no-fail-fast -p purrdf-sparql-conformance --test cost_planner_corpus --test evaluator_trace --test governor_correctness_corpus --test sparql_conformance
