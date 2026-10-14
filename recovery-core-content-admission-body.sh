#!/usr/bin/env bash
set -euo pipefail
cargo test --locked -p purrdf-core --lib native_graph_birth_tests -- --test-threads=4
cargo test --locked -p purrdf-core --lib content_id -- --test-threads=4
