#!/usr/bin/env bash
set -euo pipefail
# Background children inherit our group until setsid creates their private one.
# Disabling job control ensures setsid's child PID is its new group ID.
set +m
root=/home/paudley/Active/purrdf
worktree="$root/.worktrees/515-sparql-eval-datalog-per-row-evidence"
stage="$worktree/.stage/sparql-eval-datalog-per-row-evidence"
phase=${1:?diagnostic, matrix or physical}
shift
[[ "$phase" == diagnostic || "$phase" == matrix || "$phase" == physical ]]
if [[ "$phase" != diagnostic ]]; then
    ps -eo pid,ppid,stat,pcpu,comm > "$stage/$phase-native-layout-environment-processes.txt"
    # Existing unrelated build/compression work must finish itself. Never stop
    # or reconfigure it to admit this measurement.
    if ! awk 'NR > 1 && $3 !~ /^[TZ]/ && $5 ~ /^(cargo|rustc|clippy-driver|rust-lld|ld.lld|ld|cc1|cc1plus|clang|clang\+\+|gcc|cc|wasm-opt|restic|zstd|xz|pigz|make|ninja|cmake)$/ { print; busy=1 } END { exit busy }' "$stage/$phase-native-layout-environment-processes.txt"; then
        echo 'TIMING NOT ADMITTED: live unrelated build or compression work.' >&2
        exit 40
    fi
fi
timing_monitor() {
    set -euo pipefail
    local benchmark_pid=$1 policy=$2 telemetry=$3
    while kill -0 "$benchmark_pid" 2>/dev/null; do
        printf 'sample=%s policy_max_khz=%s attained_khz=%s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$(cat "$policy")" "$(cat /sys/devices/system/cpu/cpu11/cpufreq/scaling_cur_freq)" >> "$telemetry"
        awk '$1 == "cpu11" || $1 == "cpu27" { print }' /proc/stat >> "$telemetry"
        ps -eo pid,ppid,stat,pcpu,comm | awk -v observed="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" 'NR > 1 && $3 !~ /^[TZ]/ && $5 ~ /^(cargo|rustc|clippy-driver|rust-lld|ld.lld|ld|cc1|cc1plus|clang|clang\+\+|gcc|cc|wasm-opt|restic|zstd|xz|pigz|make|ninja|cmake)$/ { print observed, $0 }'
        sleep 1
    done
}
export -f timing_monitor
thaw_units=()
owned_groups=()
restore() {
    original_status=$?
    trap '' INT TERM
    cleanup_failed=0
    # These IDs come only from our own setsid children, never caller/sibling
    # groups. Stop benchmark descendants and monitor before restoring geo work.
    for group in "${owned_groups[@]}"; do
        if kill -0 -- "-$group" 2>/dev/null; then
            kill -TERM -- "-$group" 2>/dev/null || cleanup_failed=1
        elif kill -0 "$group" 2>/dev/null; then
            # A signal may have arrived before the child entered setsid.
            kill -TERM "$group" 2>/dev/null || cleanup_failed=1
        fi
    done
    for _ in {1..20}; do
        remaining=0
        for group in "${owned_groups[@]}"; do
            if kill -0 -- "-$group" 2>/dev/null; then remaining=1; fi
        done
        if (( !remaining )); then break; fi
        sleep 0.05
    done
    for group in "${owned_groups[@]}"; do
        if kill -0 -- "-$group" 2>/dev/null; then
            kill -KILL -- "-$group" 2>/dev/null || cleanup_failed=1
        fi
        wait "$group" 2>/dev/null || true
    done
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
trap 'exit 130' INT
trap 'exit 143' TERM
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
        directory="$stage/distinct-lazy-append-pair-$pair-$version"
        [[ ! -e "$directory" ]]
        chrt --batch 0 taskset -c 11 bash -c 'renice -n -4 -p "$$" >/dev/null; exec "$1" --collect "$2"' fixed-work "$stage/distinct-fixed-work-$version" "$directory" > "$stage/distinct-lazy-append-pair-$pair-$version.log" 2>&1
        cat "$stage/distinct-lazy-append-pair-$pair-$version.log"
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
if [[ "$phase" == physical ]]; then
    binary="$root/.worktrees/purrdf-evidence-default-current/build/release/build/purrdf-text/d2da20a98474b374/out/physical_shapes-d2da20a98474b374"
    estimates="$stage/physical-native-layout-estimates"
    [[ -x "$binary" && ! -e "$estimates" ]]
    telemetry="$stage/physical-native-layout-telemetry.txt"
    heavy="$stage/physical-native-layout-heavy-during.txt"
    : > "$telemetry"
    PURRDF_BENCH_HOME="$estimates" setsid setarch "$(uname -m)" -R chrt --batch 0 taskset -c 11 bash -c 'renice -n -4 -p "$$" >/dev/null; exec "$1" --bench' physical "$binary" > "$stage/physical-native-layout-measurement.log" 2>&1 &
    physical_pid=$!
    owned_groups+=("$physical_pid")
    setsid bash -c 'timing_monitor "$@"' monitor "$physical_pid" "$policy" "$telemetry" > "$heavy" &
    monitor_pid=$!
    owned_groups+=("$monitor_pid")
    runtime=0
    wait "$physical_pid" || runtime=$?
    wait "$monitor_pid"
    if (( runtime )); then exit "$runtime"; fi
    awk -v admitted="$initial_cap" '/^sample=/ { split($2, cap, "="); split($3, attained, "="); samples++; if (cap[2] != admitted || attained[2] !~ /^[0-9]+$/ || attained[2] <= 0) invalid=1 } END { exit invalid || samples == 0 }' "$telemetry" || {
        echo 'PHYSICAL TIMING ENVIRONMENT REJECTED: changed or missing policy/frequency telemetry.' >&2
        exit 42
    }
    if [[ -s "$heavy" ]]; then
        echo 'PHYSICAL TIMING ENVIRONMENT REJECTED: unrelated build or compression work appeared.' >&2
        exit 43
    fi
    awk '/^physical_.* time:/ { lanes++ } /samples.*iterations/ { if ($0 !~ /20 samples/) invalid=1; samples++ } END { if (lanes != 78 || samples != 78 || invalid) { print "incomplete authored physical matrix", lanes, samples, invalid; exit 45 } }' "$stage/physical-native-layout-measurement.log"
    exit 0
fi
baseline_binary="$root/.worktrees/purrdf-evidence-default-baseline/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/query_eval-8e4ffe4ed9e13b7f"
current_binary="$root/.worktrees/purrdf-evidence-default-current/build/release/build/purrdf-sparql-eval/8e4ffe4ed9e13b7f/out/query_eval-8e4ffe4ed9e13b7f"
: > "$stage/default-native-layout-cpu-samples.txt"
setsid setarch "$(uname -m)" -R chrt --batch 0 taskset -c 11 bash "$stage/original36-controlled.sh" "$baseline_binary" "$current_binary" "$stage" 11 "$stage/default-native-layout-estimates" > "$stage/default-native-layout-normal.log" 2>&1 &
matrix_pid=$!
owned_groups+=("$matrix_pid")
setsid bash -c 'timing_monitor "$@"' monitor "$matrix_pid" "$policy" "$stage/default-native-layout-cpu-samples.txt" > "$stage/default-native-layout-heavy-during.txt" &
monitor_pid=$!
owned_groups+=("$monitor_pid")
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
