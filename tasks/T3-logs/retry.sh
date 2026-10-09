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
run helper-policy cargo test --locked -p helper-census --bin helper-census policy::tests --jobs 8
run full-check-retry env CI=1 make check
