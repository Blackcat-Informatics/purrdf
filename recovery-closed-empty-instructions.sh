#!/usr/bin/env bash
set -euo pipefail
stage=/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence/.stage/sparql-eval-datalog-per-row-evidence
cd /home/paudley/Active/purrdf
cmp "$stage/distinct-fixed-work-baseline.rs" "$stage/distinct-fixed-work-current.rs"
sha256sum "$stage/distinct-fixed-work-baseline" "$stage/distinct-fixed-work-current" > "$stage/distinct-closed-empty-artifacts.sha256"
for pair in 1 2 3 4 5; do
    if (( pair % 2 )); then versions=(baseline current); else versions=(current baseline); fi
    for version in "${versions[@]}"; do
        directory="$stage/distinct-closed-empty-pair-$pair-$version"
        [[ ! -e "$directory" && ! -e "$directory.log" ]]
        chrt --batch 0 taskset -c 11 bash -c 'renice -n -4 -p "$$" >/dev/null; exec "$1" --collect "$2"' fixed-work "$stage/distinct-fixed-work-$version" "$directory" > "$directory.log" 2>&1
        cat "$directory.log"
    done
done
sha256sum --check "$stage/distinct-closed-empty-artifacts.sha256"
echo 'Five paired fixed-work instruction captures complete. No wall-time throughput qualification is inferred.'
