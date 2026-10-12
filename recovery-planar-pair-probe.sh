#!/usr/bin/env bash
set -euo pipefail
export CI=1
receipt=.stage/geo-deterministic-hierarchical-spatial/recovery-planar-pair-probe.log
test ! -e "$receipt"
status=0
cargo test --locked -p purrdf-geo --test query_rewrite_e2e \
  the_rewritten_within_predicate_answers_exactly_the_entailed_rows_from_query_text \
  -- --exact --nocapture > "$receipt" 2>&1 || status=$?
printf 'terminal_status=%s\n' "$status" >> "$receipt"
exit "$status"
