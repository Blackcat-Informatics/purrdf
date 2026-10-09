#!/usr/bin/env bash
set -uo pipefail
export PATH=/home/paudley/stage/packages/rustup/toolchains/nightly-2026-09-14-x86_64-unknown-linux-gnu/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
export CARGO_TARGET_DIR=/opt/purrdf-401-qualification/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-401-qualification/build
export TMPDIR=/opt/purrdf-401-qualification/tmp
logs=/opt/purrdf-401-qualification/logs/task2
run() {
    local label=$1
    shift
    printf 'START %s\n' "$label"
    printf '%q ' "$@" > "$logs/$label.command"
    printf '\n' >> "$logs/$label.command"
    "$@" > "$logs/$label.log" 2>&1
    local result=$?
    printf '%s\n' "$result" > "$logs/$label.exit"
    printf 'END %s %s\n' "$label" "$result"
    if (( result != 0 )); then exit "$result"; fi
}
run update-units cargo test --locked -p purrdf-sparql-eval --lib update::tests --jobs 8
run update-callers cargo test --locked -p purrdf-sparql-eval --test update_graph_modes --test update_graph_existence --test governed_update --test cdt_query_blank_scope --jobs 8
run mutable-units cargo test --locked -p purrdf-core --lib ir::mutable::tests --jobs 8
run core-callers cargo test --locked -p purrdf-core --test import_view --test graph_existence_modes --test blank_publication --test cdt_blank_identity --jobs 8
run strict cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval --all-targets --jobs 8 -- -D warnings
run fmt cargo fmt --all --check
run fast-gates make helpers-hygiene layer-hygiene terminal-hygiene rdf-core-hygiene
printf 'COMPLETE\n'
