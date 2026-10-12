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
run_receipt recovery-topology-witness-order-clean-kernel.log cargo test --locked -p purrdf-geo-kernel --lib
run_receipt recovery-topology-witness-order-clean-query.log cargo test --locked -p purrdf-geo --test query_rewrite_e2e
