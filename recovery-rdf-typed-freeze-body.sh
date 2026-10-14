#!/usr/bin/env bash
set -euo pipefail
cargo test --locked -p purrdf-rdf --lib projections:: -- --test-threads=4
cargo test --locked -p purrdf-rdf --lib native_quads:: -- --test-threads=4
