#!/usr/bin/env bash
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
result=0
make metadata > "$stage/recovery-resumed-benchmark-metadata-staged.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-resumed-benchmark-metadata-staged.log"
test "$result" -eq 0
make simd-asm SIMD_ASM_ARGS='--write-doc --jobs 1' > "$stage/recovery-resumed-assembly-callsite-write.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-resumed-assembly-callsite-write.log"
test "$result" -eq 0
make simd-asm SIMD_ASM_ARGS="--jobs 1 --report $stage/recovery-resumed-assembly-report.json" > "$stage/recovery-resumed-assembly-callsite-check.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-resumed-assembly-callsite-check.log"
exit "$result"
