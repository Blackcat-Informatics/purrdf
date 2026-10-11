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
log=.stage/sparql-eval-datalog-per-row-evidence/recovery-profile-cost-focused.log
exec > "$log" 2>&1
failed=0
cargo test --locked -p purrdf-sparql-eval --test evidence_ingress --test prepared_execution --no-fail-fast
status=$?
printf 'COLLECT ingress/prepared status=%s\n' "$status"
if [ "$status" -ne 0 ]; then failed=1; fi
for family in path:: vm:: modifier::; do
    cargo test --locked -p purrdf-sparql-eval --lib "$family" --no-fail-fast
    status=$?
    printf 'COLLECT lib %s status=%s\n' "$family" "$status"
    if [ "$status" -ne 0 ]; then failed=1; fi
done
cargo clippy --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings
status=$?
printf 'COLLECT strict status=%s\n' "$status"
if [ "$status" -ne 0 ]; then failed=1; fi
exit "$failed"
