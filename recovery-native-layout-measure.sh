#!/usr/bin/env bash
set -euo pipefail
root=/home/paudley/Active/purrdf
worktree="$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
stage="$worktree/.stage/sparql-eval-datalog-per-row-evidence"
phase=${1:?diagnostic or matrix}
shift
[[ "$phase" == diagnostic || "$phase" == matrix ]]
if [[ "$phase" == matrix ]]; then
    ps -eo pid,ppid,stat,pcpu,comm > "$stage/default-native-layout-environment-processes.txt"
    # Existing unrelated build/compression work must finish itself. Never stop
    # or reconfigure it to admit this measurement.
    if ! awk 'NR > 1 && $3 !~ /^[TZ]/ && $5 ~ /^(cargo|rustc|clippy-driver|rust-lld|ld.lld|ld|cc1|cc1plus|clang|clang\+\+|gcc|cc|wasm-opt|restic|zstd|xz|pigz|make|ninja|cmake)$/ { print; busy=1 } END { exit busy }' "$stage/default-native-layout-environment-processes.txt"; then
        echo 'TIMING NOT ADMITTED: live unrelated build or compression work.' >&2
        exit 40
    fi
fi
thaw_units=()
restore() {
    original_status=$?
    cleanup_failed=0
    for unit in "${thaw_units[@]}"; do
        if ! systemctl --user thaw "$unit"; then
            printf 'CLEANUP FAILED: could not thaw owned scope %s; original verdict=%s\n' "$unit" "$original_status" >&2
            cleanup_failed=1
        fi
    done
    # Preserve an original failing verdict; cleanup failure cannot turn a pass
    # into success or prevent attempts to restore the remaining owned scopes.
    if (( original_status == 0 && cleanup_failed )); then original_status=44; fi
    trap - EXIT
    exit "$original_status"
}
trap restore EXIT
for unit in "$@"; do
    [[ $(systemctl --user show "$unit" --property=FreezerState --value) == running ]]
    thaw_units+=("$unit")
    systemctl --user freeze "$unit"
    [[ $(systemctl --user show "$unit" --property=FreezerState --value) == frozen ]]
done
cd "$worktree"
if [[ "$phase" == diagnostic ]]; then
for pair in 1 2 3 4 5; do
    if (( pair % 2 )); then versions=(baseline current); else versions=(current baseline); fi
    for version in "${versions[@]}"; do
        directory="$stage/distinct-native-layout-pair-$pair-$version"
        [[ ! -e "$directory" ]]
        chrt --batch 0 taskset -c 11 bash -c 'renice -n -4 -p "$$" >/dev/null; exec "$1" --collect "$2"' fixed-work "$stage/distinct-fixed-work-$version" "$directory" > "$stage/distinct-native-layout-pair-$pair-$version.log" 2>&1
        cat "$stage/distinct-native-layout-pair-$pair-$version.log"
    done
done
    exit 0
fi
policy=/sys/devices/system/cpu/cpu11/cpufreq/scaling_max_freq
initial_cap=$(cat "$policy")
[[ "$initial_cap" =~ ^[0-9]+$ && "$initial_cap" -gt 0 ]]
for sample in 1 2 3 4; do
    sleep 1
    observed_cap=$(cat "$policy")
    printf 'ADMISSION sample=%s policy_max_khz=%s\n' "$sample" "$observed_cap"
    if [[ "$observed_cap" != "$initial_cap" ]]; then
        echo 'TIMING NOT ADMITTED: policy cap changed during read-only admission.' >&2
        exit 41
    fi
done
baseline_binary="$root/.worktrees/purrdf-evidence-default-baseline/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/query_eval-8e4ffe4ed9e13b7f"
current_binary="$root/.worktrees/purrdf-evidence-default-current/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/query_eval-8e4ffe4ed9e13b7f"
: > "$stage/default-native-layout-cpu-samples.txt"
setarch "$(uname -m)" -R chrt --batch 0 taskset -c 11 bash "$stage/original36-controlled.sh" "$baseline_binary" "$current_binary" "$stage" 11 "$stage/default-native-layout-estimates" > "$stage/default-native-layout-normal.log" 2>&1 &
matrix_pid=$!
(
    while kill -0 "$matrix_pid" 2>/dev/null; do
        date -u +'%Y-%m-%dT%H:%M:%SZ' >> "$stage/default-native-layout-cpu-samples.txt"
        awk '$1 == "cpu11" || $1 == "cpu27" { print }' /proc/stat >> "$stage/default-native-layout-cpu-samples.txt"
        ps -eo pid,ppid,stat,pcpu,comm | awk -v observed="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" 'NR > 1 && $3 !~ /^[TZ]/ && $5 ~ /^(cargo|rustc|clippy-driver|rust-lld|ld.lld|ld|cc1|cc1plus|clang|clang\+\+|gcc|cc|wasm-opt|restic|zstd|xz|pigz|make|ninja|cmake)$/ { print observed, $0 }'
        sleep 1
    done
) > "$stage/default-native-layout-heavy-during.txt" &
monitor_pid=$!
runtime=0
wait "$matrix_pid" || runtime=$?
wait "$monitor_pid"
if (( runtime )); then exit "$runtime"; fi
verdict=0
"$stage/assess_original36" "$stage" "$stage/default-native-layout-estimates" > "$stage/default-native-layout-assessment.tsv" 2> "$stage/default-native-layout-assessment.err" || verdict=$?
awk '
    /^BEGIN pair / { lane=$3; order=$4; run=0 }
    /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-/ { run++ }
    /^[0-9]+$/ { print lane, order, "run="run, "attained_khz="$1 }
' "$stage/default-native-layout-normal.log" > "$stage/default-native-layout-attained-frequency.txt"
if ! awk -v admitted="$initial_cap" '
    /^BEGIN pair / { lane=$3; runs=0; observed=0 }
    /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-/ {
        if (runs && !observed) { print "missing policy observations", lane, runs; invalid=1 }
        runs++; observed=0
    }
    /^policy_max_khz=/ {
        split($0, cap, "="); observed++
        if (cap[2] != admitted) { print lane, runs, $0; invalid=1 }
    }
    /^END pair / {
        if (runs != 2 || !observed) { print "incomplete paired policy telemetry", lane, runs, observed; invalid=1 }
        pairs++
    }
    END { if (pairs != 36) { print "missing complete pairs", pairs; invalid=1 }; exit invalid }
' "$stage/default-native-layout-normal.log" > "$stage/default-native-layout-policy-changes.txt"; then
    echo 'TIMING ENVIRONMENT REJECTED: original telemetry records changed or missing paired policy observations; original performance verdict retained separately.' >&2
    exit 42
fi
if [[ -s "$stage/default-native-layout-heavy-during.txt" ]]; then
    echo 'TIMING ENVIRONMENT REJECTED: unrelated build or compression work appeared; original performance verdict retained separately.' >&2
    exit 43
fi
exit "$verdict"
