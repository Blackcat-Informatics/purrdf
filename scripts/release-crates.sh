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
#
# ORDERING CONTRACT, enforced by scripts/check-publish-order.py on every
# `make check`. This list must be a topological order of BOTH edge kinds:
#
#   * normal dependencies — `cargo publish` cannot upload a crate whose
#     dependencies are not already on the registry; and
#   * DEV-dependencies — `cargo publish`'s verification step resolves the FULL
#     dependency graph of the packaged crate, dev-dependencies included, even
#     though it only builds the lib. A dev-edge pointing FORWARD in this list
#     makes verification impossible, because the sibling version it needs does
#     not exist on crates.io yet.
#
# The second constraint is why `purrdf-geo` sits after `purrdf-shapes` rather
# than beside the other SPARQL crates: it dev-depends on `purrdf-rdf` and
# `purrdf-shapes`. While it was ordered before both, the bootstrap had to run
# `cargo publish --no-verify` for the whole set, which meant a broken or
# wrong-version artifact would upload successfully and permanently, since
# nothing built it first. Moving one crate removed the last forward dev-edge and
# let verification be turned back on. Do not reorder for tidiness; run the check.

# shellcheck disable=SC2034  # consumed by the sourcing script.
PURRDF_RELEASE_CRATES=(
  purrdf-events
  purrdf-iri
  purrdf-xsd
  purrdf-cdt
  purrdf-gts
  purrdf-core
  purrdf-columnar
  purrdf-datalog
  purrdf-entail
  purrdf-sparql-algebra
  purrdf-sparql-results
  purrdf-sparql-eval
  purrdf-text
  purrdf-rdf
  purrdf-slice
  purrdf-shapes
  purrdf-geo
  purrdf-shex
  purrdf-validate
  purrdf
  purrdf-wasm
)

# The crates in the set above that have NO crates.io record yet, in publish
# order. This is a LEDGER held to the registry in both directions by
# scripts/check-crates-io-records.sh: a crate the registry lacks that is absent
# here fails the preflight, and a crate listed here that the registry now HAS
# also fails it, so the list cannot go on naming a crate someone bootstrapped.
# It named `purrdf-datalog` for a full release cycle after that crate was
# published, because nothing checked it in that direction.
#
# scripts/check-doc-claims.py holds docs/RELEASE.md's bootstrap section to this
# array offline — the heading, the crate names, the publish-order ordinals and
# the in-page anchor all derive from it — and scripts/check-publish-order.py
# holds it to the release set. Membership itself is a fact about crates.io and
# is only ever decided by the preflight, never by prose.
#
# This array is ALSO the whole of what scripts/bootstrap-crates-io.sh will
# publish. An API token can create a record; it cannot publish a new version of
# any crate that has one (every existing record is locked to Trusted Publishing
# and answers a token publish with 403), so the bootstrap walks this ledger and
# refuses any entry that has a record, by name.

# shellcheck disable=SC2034  # consumed by the sourcing script.
PURRDF_UNBOOTSTRAPPED_CRATES=(
)
