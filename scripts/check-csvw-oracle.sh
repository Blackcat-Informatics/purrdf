#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if ! command -v uv >/dev/null 2>&1; then
  echo "ERROR: uv is required for the dev-only CSVW oracle" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

cargo run --quiet --locked -p purrdf-rdf --example write_csvw_oracle_fixture -- "${tmp}"
UV_CACHE_DIR="${UV_CACHE_DIR:-${TMPDIR:-/tmp}/purrdf-csvw-uv-cache}" \
  uv run --no-project --script scripts/csvw_oracle.py "${tmp}"
