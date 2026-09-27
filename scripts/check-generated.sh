#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

set -euo pipefail

mode="${1:---check}"
if [ "$#" -gt 1 ]; then
  echo "usage: $0 [--check|--write]" >&2
  exit 2
fi
case "$mode" in
  --check | --write) ;;
  *)
    echo "usage: $0 [--check|--write]" >&2
    exit 2
    ;;
esac

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The scratch lives beside the build output, not under $TMPDIR: this gate
# generates into it across a `cargo` build that may take minutes, and /tmp is
# not a place a directory survives that reliably. See scripts/build-scratch.sh.
. "$(dirname "${BASH_SOURCE[0]}")/build-scratch.sh"
tmp="$(build_scratch_dir check-generated)"
trap 'rm -rf "$tmp"' EXIT

cd "$repo"

cargo run -p purrdf-rdf --example gen_loss_matrix --locked -- rdf \
  > "$tmp/rdf-loss-matrix.json"
cargo run -p purrdf-rdf --example gen_loss_matrix --locked -- transcode \
  > "$tmp/transcode-loss-matrix.json"
# The entailment rule inventory, read out of `RuleId` / `rules()` / `implemented()`
# rather than transcribed. Guarding it here is what keeps a coverage claim in the
# book a build artifact instead of a sentence someone has to remember to update.
cargo run -p purrdf-entail --example gen_rule_inventory --locked \
  > "$tmp/entailment-rules.md"
# The XSD/XPath `\p{IsX}` Unicode block-escape lookup table, parsed out of the
# vendored `Blocks.txt` rather than transcribed. Piped through `rustfmt` because
# the generator's raw output is committed as real Rust source
# (`crates/rdf-core/src/xsd_regex/blocks.rs`), which `cargo fmt --all --check`
# also governs — comparing pre-rustfmt bytes here would make the file
# permanently "stale" the moment anyone reformats the workspace.
cargo run -p purrdf-core --example gen_unicode_blocks --locked \
  | rustfmt --edition 2024 --emit stdout \
  > "$tmp/blocks.rs"
# The IDNA2008 tables (RFC 5892 derived property, Joining_Type, Bidi_Class, the
# Appendix A scripts, and the NFC data) and the purrdf-text analyzer tables (full
# case folding, UAX 29 word-break properties, UAX 15 normalization), both
# derived from the one vendored Unicode Character Database under
# `crates/iri/unicode/` rather than transcribed.
cargo run -p purrdf-iri --example gen_idna_tables --locked \
  | rustfmt --edition 2024 --emit stdout \
  > "$tmp/idna_tables.rs"
cargo run -p purrdf-text --example gen_unicode_text_tables --locked \
  | rustfmt --edition 2024 --emit stdout \
  > "$tmp/unicode_tables.rs"
# The property names and values an ECMA-262 `\p{…}` escape accepts, read out of
# the same vendored `PropertyValueAliases.txt` / `PropertyAliases.txt` and
# filtered to ECMA-262 Tables 65 and 66, rather than transcribed.
cargo run -p purrdf-jsonschema --example gen_ecma_property_tables --locked \
  | rustfmt --edition 2024 --emit stdout \
  > "$tmp/property_tables.rs"

check_file() {
  local generated="$1"
  local committed="$2"
  if ! cmp -s "$generated" "$committed"; then
    echo "$committed is stale; regenerate it from the Rust source." >&2
    diff -u "$committed" "$generated" >&2 || true
    exit 1
  fi
}

sync_file() {
  local generated="$1"
  local committed="$2"
  if [ "$mode" = "--write" ]; then
    cp -- "$generated" "$committed"
  fi
  check_file "$generated" "$committed"
}

sync_file "$tmp/rdf-loss-matrix.json" generated/rdf-loss-matrix.json
sync_file "$tmp/transcode-loss-matrix.json" generated/transcode-loss-matrix.json
sync_file "$tmp/entailment-rules.md" docs/book/src/entailment-rules.md
sync_file "$tmp/blocks.rs" crates/rdf-core/src/xsd_regex/blocks.rs
sync_file "$tmp/idna_tables.rs" crates/iri/src/idna_tables.rs
sync_file "$tmp/unicode_tables.rs" crates/text/src/unicode_tables.rs
sync_file "$tmp/property_tables.rs" crates/jsonschema/src/ecma/property_tables.rs

# The inventory above is now known-current. Prose elsewhere RESTATES its numbers
# (and the conformance matrix's), and prose is not covered by any byte-diff — a
# coverage table sat at `RDFS 14 / 18` for exactly that reason, three lines under
# a sentence promising it could not fall behind. This gate machine-checks every
# such restatement against the generated artifact it claims to summarize.
python3 scripts/check-doc-claims.py

viz_tmp="$tmp/visualization"
viz_committed="docs/book/src/assets/visualization"
cargo run -p purrdf-rdf --example viz_samples --locked -- \
  "$viz_tmp" --svg-only

generated_count=0
for generated_svg in "$viz_tmp"/*.svg; do
  generated_count=$((generated_count + 1))
  sync_file "$generated_svg" "$viz_committed/$(basename "$generated_svg")"
done
if [ "$mode" = "--write" ]; then
  for committed_svg in "$viz_committed"/*.svg; do
    [ -e "$committed_svg" ] || continue
    if [ ! -e "$viz_tmp/$(basename "$committed_svg")" ]; then
      rm -- "$committed_svg"
    fi
  done
fi
committed_count=$(find "$viz_committed" -maxdepth 1 -type f -name '*.svg' | wc -l)
if [ "$generated_count" -ne "$committed_count" ]; then
  echo "$viz_committed contains stale or missing SVG samples" >&2
  exit 1
fi

cargo test -p purrdf-core --lib --locked \
  sssom::tests::corpus_accessibility_parses_and_validates_clean
