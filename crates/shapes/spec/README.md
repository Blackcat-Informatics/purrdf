<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Frozen W3C SHACL 1.2 vocabularies (runtime copy)

This directory is a byte-exact, frozen copy of the four W3C SHACL 1.2
vocabulary files also vendored at `vectors/shacl12/vocabularies/`. It is
written by `scripts/vendor-shacl12.py` and **must not be hand-edited** —
a silent content edit fails `make check` via
`scripts/check-corpus-frozen.py`.

## Why a second copy

`purrdf-shapes` ships vocabulary bytes for the `purrdf shapes lint` cold
certify surface and its `shacl-shacl.ttl` oracle. The test-only declaration
ratchet (`purrdf_shacl_conformance::vocab::declared()`) also parses them to
prove every SHACL 1.2 function and component declaration is bound.
`vectors/` is a workspace-level tree outside the published `purrdf-shapes`
crate, so including it directly would build locally but break `cargo package`.
This directory contains the same bytes, copied into the shipping package.

## Source and license

Source, pinned upstream commit, and license: see
`vectors/shacl12/PROVENANCE.md`. These are the same W3C Software and Document
License files described there, at `vectors/shacl12/vocabularies/`.
