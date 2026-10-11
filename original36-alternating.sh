#!/bin/bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Captured qualification invocation; all benchmark computation remains Rust.
set -euo pipefail
test "$#" -eq 4 -o "$#" -eq 5
baseline_binary=$1
current_binary=$2
receipt_dir=$3
measurement_cpu=$4
estimates_dir=${5:-$receipt_dir/default-alternating-estimates}
test -x "$baseline_binary"
test -x "$current_binary"
case "$measurement_cpu" in ''|*[!0-9]*) exit 2 ;; esac
read -r allowed < <(awk '$1 == "Cpus_allowed_list:" {print $2}' /proc/self/status)
test "$allowed" = "$measurement_cpu"
read -r siblings < "/sys/devices/system/cpu/cpu${measurement_cpu}/topology/thread_siblings_list"
mapfile -t lanes < <(awk '$2 == "time:" { print $1 }' "$receipt_dir/default-compact-current.log")
test "${#lanes[@]}" -eq 36
test ! -e "$estimates_dir"
export PURRDF_BENCH_HOME="$estimates_dir"
TIMEFORMAT='elapsed=%R user=%U sys=%S cpu=%P'
printf 'MEASUREMENT mode=normal cpu=%s siblings=%s\n' "$measurement_cpu" "$siblings"
sha256sum "$baseline_binary" "$current_binary" crates/sparql-eval/src/bgp/window.rs crates/sparql-eval/src/solution.rs crates/sparql-eval/src/bgp.rs crates/sparql-eval/src/binop.rs crates/sparql-eval/src/modifier.rs crates/sparql-eval/src/blank_scope.rs crates/sparql-eval/src/property_fn_eval.rs crates/sparql-eval/src/evidence_exec.rs crates/sparql-eval/tests/evidence_ingress.rs crates/sparql-eval/benches/query_eval.rs
context() {
    date -u +'%Y-%m-%dT%H:%M:%SZ'
    cat /proc/loadavg /proc/pressure/cpu
    awk -v siblings="$siblings" '/^cpu[0-9]+ / {
        cpu = substr($1, 4) + 0;
        count = split(siblings, members, ",");
        for (i = 1; i <= count; ++i) {
            bounds = split(members[i], range, "-");
            end = bounds == 1 ? range[1] : range[2];
            if (cpu >= range[1] && cpu <= end) { print; break; }
        }
    }' /proc/stat
    cat "/sys/devices/system/cpu/cpu${measurement_cpu}/cpufreq/scaling_cur_freq"
}
for ordinal in "${!lanes[@]}"; do
    lane=${lanes[$ordinal]}
    if (( ordinal % 2 == 0 )); then
        printf '\nBEGIN pair %s direction=current-over-original order=original,current\n' "$lane"
        context
        time "$baseline_binary" --bench --exact "$lane" --save-baseline original-main
        context
        time "$current_binary" --bench --exact "$lane" --baseline original-main
    else
        printf '\nBEGIN pair %s direction=original-over-current order=current,original\n' "$lane"
        context
        time "$current_binary" --bench --exact "$lane" --save-baseline candidate-source
        context
        time "$baseline_binary" --bench --exact "$lane" --baseline candidate-source
    fi
    context
    printf 'END pair %s\n' "$lane"
done
