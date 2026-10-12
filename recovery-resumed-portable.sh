#!/usr/bin/env bash
set -euo pipefail
export CI=1
log=.stage/geo-deterministic-hierarchical-spatial/recovery-resumed-portable.log
set +e
make geo-determinism > "$log" 2>&1
result=$?
set -e
printf 'terminal_status=%s\n' "$result" >> "$log"
exit "$result"
