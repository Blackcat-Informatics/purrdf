#!/usr/bin/env bash
set -euo pipefail
export CI=1
receipt=.stage/geo-deterministic-hierarchical-spatial/recovery-governor-native-charge-regeneration.log
test ! -e "$receipt"
status=0
env PURRDF_UPDATE_GOVERNOR_CORPUS=1 cargo test --locked -p purrdf-sparql-conformance --test governor_corpus the_corpus_matches_its_pinned_expectations -- --exact --nocapture > "$receipt" 2>&1 || status=$?
printf 'terminal_status=%s\n' "$status" >> "$receipt"
exit "$status"
