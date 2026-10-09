#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
run() {
 local label=$1
 shift
 "$@" > /opt/purrdf-477-qualification/logs/render-charge/"$label".log 2>&1
}
run cli-numerics cargo test -p purrdf-cli --test exact_numerics_cli --locked --jobs 8
run exact-tower cargo test -p purrdf-xsd --test exact_tower --locked --jobs 8
run evaluator-numerics cargo test -p purrdf-sparql-eval --test numeric_governance --test numeric_parallel_determinism --test numeric_wasm_determinism --locked --jobs 8
run profile-identity cargo test -p purrdf-sparql-eval --lib profile_digest_changes_when_the_charge_schedule_changes --locked --jobs 8
run governor-corpus cargo test -p purrdf-sparql-conformance --test governor_corpus --locked --jobs 8
run strict cargo clippy -p purrdf-xsd -p purrdf-sparql-eval --all-targets --locked --jobs 8 -- -D warnings
run format cargo fmt --all -- --check
