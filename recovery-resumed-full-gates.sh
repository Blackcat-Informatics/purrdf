#!/usr/bin/env bash
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
result=0
test ! -e "$stage/recovery-resumed-make-check-prose-fixed.log"
test ! -e "$stage/recovery-resumed-conformance.log"
make check > "$stage/recovery-resumed-make-check-prose-fixed.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-resumed-make-check-prose-fixed.log"
test "$result" -eq 0
make conformance > "$stage/recovery-resumed-conformance.log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$stage/recovery-resumed-conformance.log"
exit "$result"
