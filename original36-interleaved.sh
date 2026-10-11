#!/bin/bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Captured qualification invocation; all benchmark computation remains Rust.
set -euo pipefail
test "$#" -eq 3
baseline_binary=$1
current_binary=$2
receipt_dir=$3
test -x "$baseline_binary"
test -x "$current_binary"
mapfile -t lanes < <(awk '$2 == "time:" { print $1 }' "$receipt_dir/default-compact-current.log")
test "${#lanes[@]}" -eq 36
export PURRDF_BENCH_HOME="$receipt_dir/default-interleaved-estimates"
TIMEFORMAT='elapsed=%R user=%U sys=%S cpu=%P'
sha256sum "$baseline_binary" "$current_binary" crates/sparql-eval/src/bgp/window.rs crates/sparql-eval/src/solution.rs crates/sparql-eval/benches/query_eval.rs
context() {
    date -u +'%Y-%m-%dT%H:%M:%SZ'
    cat /proc/loadavg /proc/pressure/cpu
    awk '$1 == "cpu8" || $1 == "cpu24" { print }' /proc/stat
    cat /sys/devices/system/cpu/cpu8/cpufreq/scaling_cur_freq
}
for lane in "${lanes[@]}"; do
    printf '\nBEGIN baseline %s\n' "$lane"
    context
    time "$baseline_binary" --bench --quick --exact "$lane" --save-baseline original-main
    context
    printf '\nBEGIN current %s\n' "$lane"
    time "$current_binary" --bench --quick --exact "$lane" --baseline original-main
    context
    printf 'END pair %s\n' "$lane"
done
