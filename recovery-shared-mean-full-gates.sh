#!/usr/bin/env bash
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
record() {
    local name=$1 status=0
    shift
    local receipt="$stage/recovery-shared-mean-$name.log"
    test ! -e "$receipt"
    "$@" > "$receipt" 2>&1 || status=$?
    printf 'terminal_status=%s\n' "$status" >> "$receipt"
    return "$status"
}
overall=0
record make-check make check || overall=1
record conformance make conformance || overall=1
exit "$overall"
