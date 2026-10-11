#!/usr/bin/env bash
set -euo pipefail
set +m
stage=/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence/.stage/sparql-eval-datalog-per-row-evidence
if [[ ${1:-} != child ]]; then
    for mode in TERM INT rejected; do
        result=0
        bash "$0" child "$mode" || result=$?
        case "$mode" in TERM) expected=143 ;; INT) expected=130 ;; rejected) expected=42 ;; esac
        [[ "$result" == "$expected" ]]
        while read -r group; do
            ps -eo pgid,stat,comm | awk -v owned="$group" '$1 == owned && $2 !~ /^Z/ {print; live=1} END {exit live}'
        done < "$stage/timing-lifecycle-$mode.groups"
        [[ -s "$stage/timing-lifecycle-$mode.telemetry" ]]
        printf '%s preserves exit%s and leaves no live owned benchmark/monitor descendants\n' "$mode" "$result"
    done
    exit
fi
mode=$2
# Execute the exact launcher functions, without its environment admission or
# any geo scope. These smoke children and their groups belong only to this test.
source <(sed -n '/^timing_monitor() {$/,/^export -f timing_monitor$/p' "$stage/recovery-row-append-measure.sh")
source <(sed -n '/^restore() {$/,/^}$/p' "$stage/recovery-row-append-measure.sh")
thaw_units=()
owned_groups=()
trap restore EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
setsid bash -c 'sleep 30 & wait' &
benchmark_pid=$!
owned_groups+=("$benchmark_pid")
setsid bash -c 'timing_monitor "$@"' monitor "$benchmark_pid" /sys/devices/system/cpu/cpu11/cpufreq/scaling_max_freq "$stage/timing-lifecycle-$mode.telemetry" > "$stage/timing-lifecycle-$mode.heavy" &
monitor_pid=$!
owned_groups+=("$monitor_pid")
printf '%s\n' "${owned_groups[@]}" > "$stage/timing-lifecycle-$mode.groups"
sleep 1.1
if [[ "$mode" == rejected ]]; then exit 42; fi
kill -s "$mode" "$$"
