#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
receipt="$stage/recovery-governor-projection-check.log"
test ! -e "$receipt"
overall=0
python3 scripts/check-corpus-frozen.py > "$receipt" 2>&1 || overall=1
cargo test --locked -p purrdf-sparql-eval --lib governor::tests::the_corpus_digest_is_a_well_formed_reproducible_sha256 -- --exact >> "$receipt" 2>&1 || overall=1
cargo test --locked -p purrdf-sparql-conformance --test governor_corpus >> "$receipt" 2>&1 || overall=1
printf 'terminal_status=%s\n' "$overall" >> "$receipt"
exit "$overall"
