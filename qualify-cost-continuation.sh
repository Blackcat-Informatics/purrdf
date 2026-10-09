#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=/opt/purrdf-477-qualification/logs/cost-continuation-1
rg --files crates/xsd crates/geo crates/sparql-eval crates/sparql-conformance Cargo.toml Cargo.lock rust-toolchain.toml | sort | xargs sha256sum > "$task_logs/source-before.sha256"
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
run cost-property cargo test -p purrdf-xsd --test exact_tower cost_estimates_bound_the_result --locked --jobs 8
run evaluator-numerics cargo test -p purrdf-sparql-eval --test numeric_governance --test numeric_parallel_determinism --test numeric_wasm_determinism --locked --jobs 8
run profile-identity cargo test -p purrdf-sparql-eval --lib profile_digest_changes_when_the_charge_schedule_changes --locked --jobs 8
run governor-corpus cargo test -p purrdf-sparql-conformance --test governor_corpus --locked --jobs 8
run strict cargo clippy -p purrdf-xsd -p purrdf-geo -p purrdf-sparql-eval --all-targets --locked --jobs 8 -- -D warnings
run format cargo fmt --all -- --check
run helpers make helpers-hygiene
run foundation-hygiene make rdf-core-hygiene layer-hygiene terminal-hygiene
run source-readback sha256sum -c "$task_logs/source-before.sha256"
