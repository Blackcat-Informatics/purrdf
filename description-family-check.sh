#!/usr/bin/env bash
set -euo pipefail
cargo test --locked -p purrdf-core --lib describe::tests
cargo test --locked -p purrdf-core --test native_regex_storage
cargo test --locked -p purrdf-cli --test describe_cli
cargo test --locked -p purrdf-rdf --test describe_serialize
cargo test --locked -p purrdf-sparql-eval --lib describe_query::tests
cargo test --locked -p purrdf-sparql-eval --test segmented_query bounded_ask_construct_describe_match_resident_across_public_entries
cargo test --locked --release -p purrdf-cli --test describe_cli
cargo clippy --workspace --all-targets --release --locked --keep-going -- -D warnings
cargo clippy --workspace --all-targets --locked --keep-going -- -D warnings
