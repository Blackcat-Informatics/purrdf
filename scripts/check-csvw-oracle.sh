#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

set -euo pipefail

if ! command -v uv >/dev/null 2>&1; then
  echo "ERROR: uv is required for the dev-only CSVW oracle" >&2
  exit 1
fi

# The scratch lives beside the build output, not under $TMPDIR: this gate
# generates into it across a `cargo` build that may take minutes, and /tmp is
# not a place a directory survives that reliably. See scripts/build-scratch.sh.
. "$(dirname "${BASH_SOURCE[0]}")/build-scratch.sh"
tmp="$(build_scratch_dir csvw-oracle)"
trap 'rm -rf "${tmp}"' EXIT

cargo run --quiet --locked -p purrdf-rdf --example write_csvw_oracle_fixture -- "${tmp}"
# uv's cache persists beside the build output too, not under $TMPDIR.
UV_CACHE_DIR="${UV_CACHE_DIR:-${CARGO_TARGET_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/target}/uv-cache/csvw-oracle}" \
  uv run --no-project --locked --script scripts/csvw_oracle.py "${tmp}"
