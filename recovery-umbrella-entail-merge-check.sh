#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32
stage=.stage/typed-capacity-errors-instead-of-u32
log="$stage/recovery-umbrella-entail-merge-check.log"
test ! -e "$log"
read -r available_kib < <(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
if (( available_kib < 32 * 1024 * 1024 )); then
    printf 'Memory admission refused: %s KiB available\n' "$available_kib" >&2
    exit 40
fi
export PATH="/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH"
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 RUSTC_WRAPPER=
export CARGO_HOME=/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/cargo-home
export CARGO_TARGET_DIR="$PWD/target/capacity-artifacts"
export CARGO_BUILD_BUILD_DIR="$PWD/target/capacity-build"
export TMPDIR="$PWD/target/capacity-tmp"
mkdir -p "$TMPDIR"
result=0
systemd-run --user --scope --quiet -p MemoryMax=16G -p MemorySwapMax=0 -- cargo check --locked -p purrdf --lib > "$log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$log"
exit "$result"
