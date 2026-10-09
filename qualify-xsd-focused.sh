#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=${PURRDF_477_FOCUSED_LOGS:-/opt/purrdf-477-qualification/logs}
cargo test -p purrdf-xsd --test exact_tower cost_estimates_bound_the_result --locked --jobs 8 > "$task_logs/pow-cost-corrected.log" 2>&1
printf '0\n' > "$task_logs/pow-cost-corrected.exit"
cargo test -p purrdf-xsd --lib float --locked --jobs 8 > "$task_logs/float-owner-corrected.log" 2>&1
printf '0\n' > "$task_logs/float-owner-corrected.exit"
cargo test -p purrdf-xsd --test exact_values --test exact_wasm_determinism --test numeric_error_codes --test value_space --locked --jobs 8 > "$task_logs/remaining-xsd-targets.log" 2>&1
printf '0\n' > "$task_logs/remaining-xsd-targets.exit"
cargo fmt --all -- --check > "$task_logs/format.log" 2>&1
printf '0\n' > "$task_logs/format.exit"
