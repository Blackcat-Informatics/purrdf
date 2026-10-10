#!/usr/bin/env bash
set -euxo pipefail
cargo fmt --all
cargo clippy --locked -p purrdf-capi --all-targets -- -D warnings
cargo test --locked -p purrdf-capi --lib
export UV_PYTHON=3.13
cd bindings/python
uv sync --locked --group dev --reinstall-package purrdf
uv run --locked pytest tests/test_entail_reasoning.py
