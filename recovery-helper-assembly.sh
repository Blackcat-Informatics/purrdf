#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
test ! -e "$stage/recovery-helper-assembly-write.log"
test ! -e "$stage/recovery-helper-assembly-check.log"
test ! -e "$stage/recovery-helper-assembly-report.json"
result=0
make simd-asm SIMD_ASM_ARGS='--write-doc --jobs 1' > "$stage/recovery-helper-assembly-write.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-helper-assembly-write.log"
test "$result" -eq 0
make simd-asm SIMD_ASM_ARGS="--jobs 1 --report $stage/recovery-helper-assembly-report.json" > "$stage/recovery-helper-assembly-check.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-helper-assembly-check.log"
exit "$result"
