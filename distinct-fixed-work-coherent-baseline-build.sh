#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
cache="$root/.worktrees/purrdf-evidence-default-baseline"
cd "$cache/source"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly
export RUSTC_WRAPPER=''
export CARGO_HOME="$root/.worktrees/purrdf-shapes-dictionary-qualification/cargo-home"
export CARGO_TARGET_DIR="$cache/target"
export CARGO_BUILD_BUILD_DIR="$cache/build"
export TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8
export RUSTFLAGS='-D warnings'
unset CARGO_ENCODED_RUSTFLAGS
export CARGO_PROFILE_BENCH_LTO=thin
export CARGO_PROFILE_BENCH_CODEGEN_UNITS=1
export CARGO_PROFILE_BENCH_DEBUG=0
export CARGO_PROFILE_BENCH_STRIP=none
/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo build --locked --profile bench -p purrdf-sparql-eval -p purrdf-testkit --lib
