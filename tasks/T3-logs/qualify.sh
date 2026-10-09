#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -uo pipefail
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
export CARGO_TARGET_DIR=/opt/purrdf-401-qualification/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-401-qualification/build
export TMPDIR=/opt/purrdf-401-qualification/tmp
qualification_logs=/opt/purrdf-401-qualification/logs/task3
run() {
    local label=$1
    shift
    printf '%q ' "$@" > "$qualification_logs/$label.command"
    printf '\n' >> "$qualification_logs/$label.command"
    "$@" > "$qualification_logs/$label.log" 2>&1
    local result=$?
    printf '%s\n' "$result" > "$qualification_logs/$label.exit"
    if (( result != 0 )); then exit "$result"; fi
}
run additive-api cargo semver-checks check-release -p purrdf-core -p purrdf-sparql-eval --baseline-rev aab23cbf20b68483ae58bf8440cd882bf1e1897b --release-type minor
run update-wasm env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/paudley/Active/purrdf/.worktrees/401-update-transfers-preserve-rdf-1-2/scripts/wasm-test-runner.sh cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test update_typed_records --jobs 8
run full-check env CI=1 make check
