#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cd "$repo"

cargo run -p purrdf-rdf --example gen_loss_matrix --locked -- rdf \
  > "$tmp/rdf-loss-matrix.json"
cargo run -p purrdf-rdf --example gen_loss_matrix --locked -- transcode \
  > "$tmp/transcode-loss-matrix.json"

check_file() {
  local generated="$1"
  local committed="$2"
  if ! cmp -s "$generated" "$committed"; then
    echo "$committed is stale; regenerate it from the Rust source." >&2
    diff -u "$committed" "$generated" >&2 || true
    exit 1
  fi
}

check_file "$tmp/rdf-loss-matrix.json" generated/rdf-loss-matrix.json
check_file "$tmp/transcode-loss-matrix.json" generated/transcode-loss-matrix.json

viz_tmp="$tmp/visualization"
viz_committed="docs/book/src/assets/visualization"
cargo run -p purrdf-rdf --example viz_samples --locked -- \
  "$viz_tmp" --svg-only

generated_count=0
for generated_svg in "$viz_tmp"/*.svg; do
  generated_count=$((generated_count + 1))
  check_file "$generated_svg" "$viz_committed/$(basename "$generated_svg")"
done
committed_count=$(find "$viz_committed" -maxdepth 1 -type f -name '*.svg' | wc -l)
if [ "$generated_count" -ne "$committed_count" ]; then
  echo "$viz_committed contains stale or missing SVG samples" >&2
  exit 1
fi

cargo test -p purrdf-core --lib --locked \
  sssom::tests::corpus_accessibility_parses_and_validates_clean
