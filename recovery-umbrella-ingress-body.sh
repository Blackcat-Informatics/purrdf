#!/usr/bin/env bash
set -euo pipefail
cargo metadata --offline --format-version 1 > .stage/typed-capacity-errors-instead-of-u32/recovery-columnar-edge-metadata.json
cargo check --locked -p purrdf --lib
