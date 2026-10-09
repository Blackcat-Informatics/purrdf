#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=/opt/purrdf-477-qualification/logs/hygiene-retry-1
run() {
 local label=$1
 shift
 set +e
 "$@" > "$task_logs/$label.log" 2>&1
 local status=$?
 set -e
 printf '%s\n' "$status" > "$task_logs/$label.exit"
 return "$status"
}
run census python3 scripts/check-shared-helpers.py
run foundation make rdf-core-hygiene layer-hygiene terminal-hygiene
run unchanged-numeric-source sha256sum -c /opt/purrdf-477-qualification/logs/interval-continuation-2/source-before.sha256
