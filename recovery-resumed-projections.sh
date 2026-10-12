#!/usr/bin/env bash
set -euo pipefail
export CI=1
run_gate() {
    local gate_name=$1
    shift
    local receipt=.stage/geo-deterministic-hierarchical-spatial/recovery-resumed-${gate_name}.log
    local result=0
    "$@" > "$receipt" 2>&1 || result=$?
    printf 'terminal_status=%s\n' "$result" >> "$receipt"
    return "$result"
}
run_gate portable-registered make geo-determinism
run_gate metadata make metadata
