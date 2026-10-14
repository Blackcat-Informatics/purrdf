#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "gts_import" --test-threads=4
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "native_codecs::" --test-threads=4
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "projections::" --test-threads=4
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "native_quads::" --test-threads=4
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "turtle_normalize::" --test-threads=4
"/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/target/capacity-build/debug/build/purrdf-rdf/5a18473667e252bd/out/purrdf_rdf-5a18473667e252bd" "gts_compose::" --test-threads=4
