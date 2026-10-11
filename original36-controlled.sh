#!/bin/bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Qualification invocation only; original Rust routines and statistics are unchanged.
set -euo pipefail
test "$#" -eq 5
baseline_binary=$1
current_binary=$2
receipt_dir=$3
measurement_cpu=$4
estimates_dir=$5
test -x "$baseline_binary"
test -x "$current_binary"
case "$measurement_cpu" in ''|*[!0-9]*) exit 2 ;; esac
read -r allowed < <(awk '$1 == "Cpus_allowed_list:" {print $2}' /proc/self/status)
test "$allowed" = "$measurement_cpu"
test "$(cat /proc/self/personality)" = 00040000
renice -n -4 -p $$
mapfile -t lanes < <(awk '$2 == "time:" { print $1 }' "$receipt_dir/default-compact-current.log")
test "${#lanes[@]}" -eq 36
test ! -e "$estimates_dir"
mkdir "$estimates_dir"
export PURRDF_BENCH_HOME="$estimates_dir/active"
printf 'MEASUREMENT normal original36 cpu=%s class=B nice=-4 fixed-layout fresh-state equal-argv\n' "$measurement_cpu"
sha256sum "$baseline_binary" "$current_binary" crates/sparql-eval/benches/query_eval.rs
measure() {
    local binary=$1 lane=$2 record=$3 benchmark_pid monitor_pid
    test ! -e "$PURRDF_BENCH_HOME"
    date -u +'%Y-%m-%dT%H:%M:%SZ'
    (exec -a query_eval "$binary" --bench --exact "$lane" --save-baseline sample) &
    benchmark_pid=$!
    (
        while ps -o pid,ni,cls,psr,comm -p "$benchmark_pid"; do
            cat "/sys/devices/system/cpu/cpu${measurement_cpu}/cpufreq/scaling_cur_freq"
            printf 'policy_max_khz='
            cat "/sys/devices/system/cpu/cpu${measurement_cpu}/cpufreq/scaling_max_freq"
            sleep 1
        done
    ) &
    monitor_pid=$!
    wait "$benchmark_pid"
    wait "$monitor_pid"
    mkdir -p "$estimates_dir/$lane"
    mv "$PURRDF_BENCH_HOME/$lane/sample" "$estimates_dir/$lane/$record"
    # The fresh invocation also writes an identical 'new' copy. Only its
    # original sample record is needed by the unchanged offline assessor.
    rm -r -- "$PURRDF_BENCH_HOME"
}
for ordinal in "${!lanes[@]}"; do
    lane=${lanes[$ordinal]}
    if (( ordinal % 2 == 0 )); then
        printf '\nBEGIN pair %s order=original,current\n' "$lane"
        measure "$baseline_binary" "$lane" original-main
        measure "$current_binary" "$lane" new
    else
        printf '\nBEGIN pair %s order=current,original\n' "$lane"
        measure "$current_binary" "$lane" candidate-source
        measure "$baseline_binary" "$lane" new
    fi
    printf 'END pair %s\n' "$lane"
done
