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
log=.stage/sparql-eval-datalog-per-row-evidence/recovery-bag-attachment-summary-library-strict.log
exec > "$log" 2>&1
failed=0
cargo test --locked -p purrdf-sparql-eval --lib --no-fail-fast
status=$?
printf 'COLLECT complete library status=%s\n' "$status"
if [ "$status" -ne 0 ]; then failed=1; fi
cargo clippy --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings
status=$?
printf 'COLLECT strict status=%s\n' "$status"
if [ "$status" -ne 0 ]; then failed=1; fi
exit "$failed"
