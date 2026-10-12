#!/usr/bin/env bash
set -euo pipefail
export CI=1
receipt=.stage/geo-deterministic-hierarchical-spatial/recovery-topology-phase-work.log
test ! -e "$receipt"
status=0
cargo test --locked -p purrdf-geo-kernel --lib topology::relate::tests::native_matrix_owner_preserves_every_original_golden_and_relation_family -- --exact --nocapture > "$receipt" 2>&1 || status=$?
printf 'terminal_status=%s\n' "$status" >> "$receipt"
exit "$status"
