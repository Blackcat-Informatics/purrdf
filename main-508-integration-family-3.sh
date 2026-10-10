#!/usr/bin/env bash
set -euxo pipefail
rustc -Vv
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --locked -p purrdf-xsd -p purrdf-entail -p purrdf-validate -p purrdf-sparql-eval -p purrdf-capi --all-targets -- -D warnings
cargo test --locked --no-fail-fast -p purrdf-xsd -p purrdf-entail -p purrdf-validate --lib --tests
cargo test --locked --no-fail-fast -p purrdf-sparql-eval --test exact_numerics --test numeric_aggregate_ownership --test numeric_casts --test numeric_contract --test numeric_governance --test numeric_parallel_determinism --test numeric_wasm_determinism
cargo test --locked -p purrdf-capi --lib
make helpers-hygiene
export UV_PYTHON=3.13
cd bindings/python
uv sync --locked --group dev --reinstall-package purrdf
uv run --locked pytest tests/test_entail_reasoning.py
