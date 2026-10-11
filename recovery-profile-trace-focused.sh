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
cargo test --locked -p purrdf-sparql-eval --lib profile_digest_changes_when_the_charge_schedule_changes
cargo test --locked -p purrdf-capi --test native_profile
cargo test --locked -p purrdf-sparql-conformance --test evaluator_trace -- --ignored --exact regenerate_evaluator_trace
git diff -- crates/sparql-conformance/tests/goldens/evaluator-trace/suite.trace
cargo test --locked -p purrdf-sparql-conformance --test evaluator_trace --test governor_corpus
