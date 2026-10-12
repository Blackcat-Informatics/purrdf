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
sha256sum crates/geo-kernel/src/topology/relate.rs crates/geo-kernel/src/topology/noding.rs crates/geo-kernel/src/context/retained.rs > "$stage/recovery-singleton-direct-source.sha256"
run_receipt recovery-singleton-direct-budget.log cargo test --locked -p purrdf-geo-kernel --lib topology::relate::tests::all_small_map_pairs_complete_in_one_default_work_budget -- --exact --nocapture
run_receipt recovery-singleton-direct-topology.log cargo test --locked -p purrdf-geo-kernel --lib topology::
run_receipt recovery-singleton-direct-ownership.log cargo test --locked -p purrdf-geo-kernel --lib context::retained::tests::native_planar_matrix_prices_actual_arrays_and_limb_owners_until_destruction -- --exact
run_receipt recovery-singleton-direct-determinism.log cargo test --locked -p purrdf-geo-kernel --lib determinism::
