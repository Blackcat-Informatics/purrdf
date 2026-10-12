#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
summary="$stage/recovery-final-round-remaining.log"
test ! -e "$summary"
overall=0
for launcher in recovery-shared-mean-projections.sh recovery-helper-host-refresh.sh recovery-helper-assembly.sh recovery-shared-mean-full-gates.sh; do
    result=0
    printf 'starting=%s\n' "$launcher" >> "$summary"
    bash "$stage/$launcher" || result=$?
    printf 'completed=%s terminal_status=%s\n' "$launcher" "$result" >> "$summary"
    if (( result )); then overall=1; fi
done
printf 'terminal_status=%s\n' "$overall" >> "$summary"
exit "$overall"
