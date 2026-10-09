#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/477-xsd-one-binary-arbitrary-precision
task_logs=/opt/purrdf-477-qualification/logs/final-numeric
rg --files crates/xsd crates/geo crates/hash crates/lex crates/testkit crates/alloc-probe Cargo.toml Cargo.lock rust-toolchain.toml | sort | xargs sha256sum > "$task_logs/source-before.sha256"
cargo test -p purrdf-xsd --test exact_values --test exact_wasm_determinism --test numeric_error_codes --test value_space --locked --jobs 8 > "$task_logs/xsd-tail.log" 2>&1
printf '0\n' > "$task_logs/xsd-tail.exit"
cargo test -p purrdf-geo --lib exact --locked --jobs 8 > "$task_logs/geo-exact.log" 2>&1
printf '0\n' > "$task_logs/geo-exact.exit"
cargo test -p purrdf-sparql-eval --test numeric_governance --test numeric_parallel_determinism --test numeric_wasm_determinism --locked --jobs 8 > "$task_logs/evaluator-numerics.log" 2>&1
printf '0\n' > "$task_logs/evaluator-numerics.exit"
cargo test -p purrdf-sparql-eval --lib profile_digest_changes_when_the_charge_schedule_changes --locked --jobs 8 > "$task_logs/profile-identity.log" 2>&1
printf '0\n' > "$task_logs/profile-identity.exit"
cargo clippy -p purrdf-xsd -p purrdf-geo -p purrdf-sparql-eval --all-targets --locked --jobs 8 -- -D warnings > "$task_logs/strict.log" 2>&1
printf '0\n' > "$task_logs/strict.exit"
cargo fmt --all -- --check > "$task_logs/format.log" 2>&1
printf '0\n' > "$task_logs/format.exit"
make helpers-hygiene > "$task_logs/helpers.log" 2>&1
printf '0\n' > "$task_logs/helpers.exit"
make rdf-core-hygiene layer-hygiene terminal-hygiene > "$task_logs/foundation-hygiene.log" 2>&1
printf '0\n' > "$task_logs/foundation-hygiene.exit"
sha256sum -c "$task_logs/source-before.sha256" > "$task_logs/source-readback.log" 2>&1
printf '0\n' > "$task_logs/source-readback.exit"
