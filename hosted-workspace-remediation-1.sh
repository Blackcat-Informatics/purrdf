#!/usr/bin/env bash
set -euo pipefail
rustc --version --verbose
cargo fmt --all -- --check
cargo clippy -p purrdf-entail --all-targets --locked -- -D warnings
cargo test -p purrdf-entail --lib owl_dl::bounds::owner_tests -- --nocapture
