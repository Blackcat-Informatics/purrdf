#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
cache="$root/.worktrees/purrdf-evidence-default-current"
stage="$root/.worktrees/515-sparql-eval-datalog-per-row-evidence/.stage/sparql-eval-datalog-per-row-evidence"
cd "$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
export CARGO_HOME="$root/.worktrees/purrdf-shapes-dictionary-qualification/cargo-home"
export CARGO_TARGET_DIR="$cache/target" CARGO_BUILD_BUILD_DIR="$cache/build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1 RUSTFLAGS='-D warnings'
unset CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
export CARGO_PROFILE_BENCH_LTO=thin CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 CARGO_PROFILE_BENCH_DEBUG=0 CARGO_PROFILE_BENCH_STRIP=none
cargo bench --locked -p purrdf-sparql-eval --bench query_eval --no-run
bash "$stage/distinct-fixed-work-build.sh" current
