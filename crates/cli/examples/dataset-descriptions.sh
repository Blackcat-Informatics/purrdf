#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

repo_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
fixture="${repo_root}/crates/rdf/tests/fixtures/dataset-description"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

for profile in dcat-rdf void; do
  cargo run --quiet --locked --manifest-path "${repo_root}/Cargo.toml" \
    -p purrdf-cli -- \
    project --profile "${profile}" --config "${fixture}/${profile}.json" \
    --from trig "${fixture}/void-source.trig" "${tmp}/${profile}.tar"

  cargo run --quiet --locked --manifest-path "${repo_root}/Cargo.toml" \
    -p purrdf-cli -- \
    project --profile "${profile}" --config "${fixture}/${profile}.json" \
    --from trig "${fixture}/void-source.trig" "${tmp}/${profile}-repeat.tar"

  cmp "${tmp}/${profile}.tar" "${tmp}/${profile}-repeat.tar"
  test -s "${tmp}/${profile}.tar"
done

printf 'PurRDF CLI dataset descriptions OK\n'
