#!/usr/bin/env bash
set -uo pipefail
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_TARGET_DIR=/opt/purrdf-401-qualification/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-401-qualification/build
export TMPDIR=/opt/purrdf-401-qualification/tmp
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
retry_logs=.stage/update-transfers-preserve-rdf-1-2/tasks
run() {
    local label=$1
    shift
    printf '%q ' "$@" > "$retry_logs/T4-$label.command"
    printf '\n' >> "$retry_logs/T4-$label.command"
    "$@" > "$retry_logs/T4-$label.log" 2>&1
    local result=$?
    printf '%s\n' "$result" > "$retry_logs/T4-$label.exit"
    tail -n 5 "$retry_logs/T4-$label.log"
    if (( result != 0 )); then exit "$result"; fi
}
run regression-final cargo test --locked -p purrdf-core --lib normalization_preserves_delta_target_ordinal_through_undo
run strict-final env RUSTFLAGS=-Dwarnings cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval --all-targets -- -D warnings
run fmt-final cargo fmt --all -- --check
