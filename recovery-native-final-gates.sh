#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
cd "$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
cache="$root/.worktrees/purrdf-shapes-dictionary-qualification"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export RUSTUP_TOOLCHAIN=nightly RUSTC_WRAPPER=''
export CARGO_HOME="$cache/cargo-home" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_BUILD_DIR="$PWD/target/native-build" TMPDIR="$cache/tmp"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 CI=1
# Preserve the existing build.rustflags policy while allowing the gate's
# target-specific assembly flags. A global RUSTFLAGS masks those flags.
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS PURRDF_TEST_REQUIRE_EXACT
case "${1:-all}" in
    all) make metadata ;;
    check) ;;
    *) echo 'usage: recovery-native-final-gates.sh [all|check]' >&2; exit 2 ;;
esac
make check
