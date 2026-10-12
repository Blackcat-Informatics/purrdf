#!/usr/bin/env bash
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
run_receipt() {
  local receipt="$1" status=0
  shift
  test ! -e "$stage/$receipt"
  "$@" > "$stage/$receipt" 2>&1 || status=$?
  printf 'terminal_status=%s\n' "$status" >> "$stage/$receipt"
  return "$status"
}
run_receipt recovery-native-profile-repaired.log \
  cargo test --locked -p purrdf-capi --test native_profile
run_receipt recovery-governor-charge-regeneration.log \
  env PURRDF_UPDATE_GOVERNOR_CORPUS=1 cargo test --locked \
  -p purrdf-sparql-conformance --test governor_corpus \
  the_corpus_matches_its_pinned_expectations -- --exact --nocapture
run_receipt recovery-governor-charge-verification.log \
  cargo test --locked -p purrdf-sparql-conformance --test governor_corpus
