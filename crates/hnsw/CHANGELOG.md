<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Changelog

All notable changes to `purrdf-hnsw` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). It joins the
workspace at the workspace version and is published as part of the `rust-v*`
release set; the release set is defined once in `scripts/release-crates.sh`.

## [Unreleased]

### Added

- The deterministic HNSW index: splitmix64 levels over the stable row index, a
  round-structured build (entry point fixed before any link exists and excluded
  from every batch; batches doubling from one row and capped at 2048;
  frozen-snapshot proposals; canonical `(distance, row)` merge), neighbour
  selection by the relative-neighbourhood condition, a serial pass that leaves
  every row reachable from the entry point, and a canonical little-endian payload
  image whose bytes are identical across rayon worker counts and across
  `wasm32-unknown-unknown`.
- The fail-closed validation matrix for `M`, `M0`, `ef_construction` and
  `ef_search`; there are no defaults and no query-time override, because the
  four numbers are the index identity.
- The typed PURREMB adapter over `IndexGuardContract` / `DerivedIndex` /
  `IndexGuardView`: profile and coordinate validation, payload-commitment
  verification before search, and `verify_rebuild` for rebuildability.
- The approximate relation over the property-function seam, registered by the
  consumer under a caller-supplied predicate IRI, with work accounting through
  `PfCursor::take_work` and the "offers candidates, never certifies absence"
  cursor contract.
- The determinism harness: a hand-rolled FNV-1a digest of the canonical payload
  bytes, pinned natively and proven identical on `wasm32-unknown-unknown` by
  `make hnsw-determinism`.
- The admission evidence: the build-cost harness at 5,000 / 50,000 / 200,000 /
  1,000,000 rows and the recall/work/latency harness, both measured only against
  the exact kNN oracle.
