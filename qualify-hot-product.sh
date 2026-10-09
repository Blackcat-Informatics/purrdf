#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=/opt/purrdf-477-qualification/logs/hot-product-1
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
rg --files crates/xsd Cargo.toml Cargo.lock | sort | xargs sha256sum > "$task_logs/source-before.sha256"
run arithmetic-oracles cargo test -p purrdf-xsd --test exact_tower arithmetic_matches_the_oracle --locked --jobs 8
run karatsuba-oracle cargo test -p purrdf-xsd --test exact_tower karatsuba_products_match_the_oracle --locked --jobs 8
run exact-values cargo test -p purrdf-xsd --test exact_values --locked --jobs 8
run strict cargo clippy -p purrdf-xsd --all-targets --locked --jobs 8 -- -D warnings
run format cargo fmt --all -- --check
run census python3 scripts/check-shared-helpers.py
export PURRDF_BENCH_HOME=/opt/purrdf-477-qualification/bench-records
run benchmark-candidate cargo bench -p purrdf-xsd --bench exact --locked --jobs 8 -- --quick --bench xsd_exact_small/ xsd_exact_growth/mul/ --baseline main341
run source-readback sha256sum -c "$task_logs/source-before.sha256"
