#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=/opt/purrdf-477-qualification/logs/performance-1
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
rg --files crates/xsd crates/sparql-eval crates/cli crates/testkit Cargo.toml Cargo.lock rust-toolchain.toml | sort | xargs sha256sum > "$task_logs/source-before.sha256"
export PURRDF_BENCH_HOME=/opt/purrdf-477-qualification/bench-records
run baseline-source git -C /home/paudley/Active/purrdf cat-file -p 341ad5ae2cecc5de026047f86dcd1194aff25f10
(
 cd /opt/purrdf-477-main-341ad
 run benchmark-baseline env CARGO_TARGET_DIR=/opt/purrdf-477-qualification/baseline-target CARGO_BUILD_BUILD_DIR=/opt/purrdf-477-qualification/baseline-build cargo bench -p purrdf-xsd --bench exact --locked --jobs 8 -- --quick --bench xsd_exact_small/ xsd_exact_growth/mul/ --save-baseline main341
)
run benchmark-candidate cargo bench -p purrdf-xsd --bench exact --locked --jobs 8 -- --quick --bench xsd_exact_small/ xsd_exact_growth/mul/ --baseline main341
run source-readback sha256sum -c "$task_logs/source-before.sha256"
