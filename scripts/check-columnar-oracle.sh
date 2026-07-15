#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if ! command -v duckdb >/dev/null 2>&1; then
  echo "ERROR: DuckDB CLI is required for the dev-only columnar oracle" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

cargo run --quiet --locked -p purrdf-columnar --example write_oracle_fixture -- "${tmp}"

counts="$(duckdb -csv -noheader -c "
SELECT
  (SELECT count(*) FROM read_parquet('${tmp}/quads.parquet')),
  (SELECT count(*) FROM read_parquet('${tmp}/reifiers.parquet')),
  (SELECT count(*) FROM read_parquet('${tmp}/annotations.parquet')),
  (SELECT count(*) FROM read_parquet('${tmp}/blobs.parquet')),
  (SELECT count(*) FROM read_parquet('${tmp}/terms.parquet') WHERE named_graph = 1),
  (SELECT count(*) FROM read_parquet('${tmp}/terms.parquet')
    WHERE kind = 1 AND lang = 'ar' AND direction = 1);
")"
if [[ "${counts}" != "2,1,1,2,2,1" ]]; then
  echo "ERROR: DuckDB observed unexpected columnar fixture counts: ${counts}" >&2
  exit 1
fi

types="$(duckdb -csv -noheader -c "
SELECT typeof(id), typeof(lex)
FROM read_parquet('${tmp}/terms.parquet') LIMIT 1;
SELECT typeof(bytes)
FROM read_parquet('${tmp}/blobs.parquet') LIMIT 1;
")"
if [[ "${types}" != $'BIGINT,VARCHAR\nBLOB' ]]; then
  echo "ERROR: DuckDB observed unexpected Parquet types:" >&2
  printf '%s\n' "${types}" >&2
  exit 1
fi

echo "OK: DuckDB read all five PurRDF columnar files with expected rows and types"
