#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

repo_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
fixture="${repo_root}/examples/projections"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

cargo run --quiet --locked --manifest-path "${repo_root}/Cargo.toml" \
  -p purrdf-cli -- \
  --loss-ledger="${tmp}/project-losses.json" \
  project --profile lpg-csv --config "${fixture}/lpg-csv.json" \
  --from turtle "${fixture}/data.ttl" "${tmp}/graph.tar"

cargo run --quiet --locked --manifest-path "${repo_root}/Cargo.toml" \
  -p purrdf-cli -- \
  project --profile lpg-csv --config "${fixture}/lpg-csv.json" \
  --from turtle "${fixture}/data.ttl" "${tmp}/graph-repeat.tar"
cmp "${tmp}/graph.tar" "${tmp}/graph-repeat.tar"

cargo run --quiet --locked --manifest-path "${repo_root}/Cargo.toml" \
  -p purrdf-cli -- \
  --loss-ledger="${tmp}/lift-losses.json" \
  lift --profile lpg-csv --config "${fixture}/lpg-csv.json" \
  --to nquads "${tmp}/graph.tar" "${tmp}/roundtrip.nq"

test -s "${tmp}/graph.tar"
test -s "${tmp}/roundtrip.nq"
test -s "${tmp}/project-losses.json"
test -s "${tmp}/lift-losses.json"
printf 'PurRDF CLI projection round trip OK\n'
