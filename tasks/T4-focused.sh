#!/usr/bin/env bash
set -uo pipefail
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_TARGET_DIR=/opt/purrdf-401-qualification/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-401-qualification/build
export TMPDIR=/opt/purrdf-401-qualification/tmp
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
focused_logs=.stage/update-transfers-preserve-rdf-1-2/tasks
run() {
    local label=$1
    shift
    printf '%q ' "$@" > "$focused_logs/T4-$label.command"
    printf '\n' >> "$focused_logs/T4-$label.command"
    "$@" > "$focused_logs/T4-$label.log" 2>&1
    local result=$?
    printf '%s\n' "$result" > "$focused_logs/T4-$label.exit"
    tail -n 5 "$focused_logs/T4-$label.log"
    if (( result != 0 )); then exit "$result"; fi
}
run shared-views cargo test --locked -p purrdf-core --test shared_views
run native-update cargo test --locked -p purrdf-sparql-eval --test update_typed_records
run wasm-update env CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh" cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test update_typed_records
run strict env RUSTFLAGS=-Dwarnings cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval --all-targets -- -D warnings
run fmt cargo fmt --all -- --check
