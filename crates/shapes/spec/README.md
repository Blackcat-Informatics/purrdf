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

`purrdf-shapes` needs these vocabularies at runtime: the declared-vs-implemented
ratchet (`purrdf_shapes::spec::declared()`) parses them to prove every SHACL 1.2
function and component declaration is bound, and the `purrdf shapes lint` cold
certify surface reads them for the `shacl-shacl.ttl` oracle. `vectors/` is a
workspace-level tree that is not part of the published `purrdf-shapes` crate,
so an `include_bytes!` reaching out of `crates/shapes/` into `vectors/` would
build locally but break `cargo package`. This directory is the same bytes,
copied so they ship inside the package.

## Source and license

Source, pinned upstream commit, and license: see
`vectors/shacl12/PROVENANCE.md`. These are the same W3C Software and Document
License files described there, at `vectors/shacl12/vocabularies/`.
