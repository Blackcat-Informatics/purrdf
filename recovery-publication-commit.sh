#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence
cache=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
export CARGO_HOME="$cache/cargo-home" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_BUILD_DIR="$PWD/target/native-build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
git diff --check
git diff --cached --check
git commit -F .stage/sparql-eval-datalog-per-row-evidence/publication-commit-message.txt
