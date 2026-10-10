#!/usr/bin/env bash
set -euxo pipefail
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh"
cargo test --locked --target wasm32-unknown-unknown -p purrdf-xsd --test exact_wasm_determinism
cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-eval --test numeric_wasm_determinism
make wasm-pkg
cd crates/rdf-wasm/js
node --test tests/entail.test.mjs
