# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
# shellcheck shell=bash
#
# The crates.io release set, in publish (dependency) order.
#
# This file is SOURCED, never executed. It is the single definition of the
# release set: the GitHub release lane (`.github/workflows/release-cargo.yaml`),
# the token bootstrap (`scripts/bootstrap-crates-io.sh`) and the crates.io
# record preflight (`scripts/check-crates-io-records.sh`) all read it, so the
# list the preflight checks is by construction the list the publisher walks.
# A crate present in one copy and absent from another is what made the preflight
# necessary in the first place; there is now only one copy.
#
# `purrdf-python`, `purrdf-cli`, `purrdf-capi` and `purrdf-sparql-conformance`
# are deliberately NOT here — see docs/RELEASE.md.

# shellcheck disable=SC2034  # consumed by the sourcing script.
PURRDF_RELEASE_CRATES=(
  purrdf-events
  purrdf-iri
  purrdf-xsd
  purrdf-gts
  purrdf-core
  purrdf-columnar
  purrdf-datalog
  purrdf-entail
  purrdf-sparql-algebra
  purrdf-sparql-results
  purrdf-sparql-eval
  purrdf-geo
  purrdf-text
  purrdf-rdf
  purrdf-slice
  purrdf-shapes
  purrdf-shex
  purrdf-validate
  purrdf
  purrdf-wasm
)
