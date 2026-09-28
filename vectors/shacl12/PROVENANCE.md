<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C SHACL 1.2 vocabularies and `shacl12-test-suite`

Frozen copy of the upstream `w3c/data-shapes` SHACL 1.2 vocabularies and the whole
`shacl12-test-suite/tests/` tree, vendored for the `purrdf-shapes`
declared-vs-implemented ratchet and its SHACL 1.2 conformance harness.
**Do not hand-edit** — treat exactly like the GTS vectors and the
`vectors/shexTest` corpus: byte-frozen third-party conformance data,
regenerated only by re-running `python3 scripts/vendor-shacl12.py`. The
freeze is enforced: `make check` runs `scripts/check-corpus-frozen.py`, which
SHA-256-verifies every file here against
`scripts/conformance-frozen/vectors-shacl12.sha256`, so a silent content edit
fails the build. A deliberate re-vendor regenerates that manifest with
`python3 scripts/check-corpus-frozen.py --update`.

## Source

- Upstream: <https://github.com/w3c/data-shapes> — the W3C Data Shapes Working Group's
  repository for SHACL 1.2 (Core, the node-expression extension `shnex`, the
  SPARQL bindings, and the conformance test suite).
- Pinned commit: `5069ed3519c31cfcf074d2cf779542abc37ce80d` — pinned for reproducible builds and to track
  upstream errata explicitly, the same hygiene every vendored suite in this
  repo follows.
- License: **W3C Software and Document License** (<http://www.w3.org/Consortium/Legal/copyright-software>), per the upstream
  repository's own `LICENSE.md`; that file is vendored verbatim alongside
  this tree as `LICENSE.md`.
- Retrieval: `scripts/vendor-shacl12.py` fetches the pinned commit's full
  source tree as a tarball from the GitHub REST API
  (`GET https://api.github.com/repos/w3c/data-shapes/tarball/5069ed3519c31cfcf074d2cf779542abc37ce80d`, which redirects
  to a `codeload.github.com` archive of that exact commit) and extracts the
  `shacl12-vocabularies/` and `shacl12-test-suite/tests/` subtrees verbatim —
  no per-file API calls, no upstream `git` clone.
- Vendored subset:
  - `vocabularies/`: `shacl.ttl`, `shnex.ttl`, `shnex-sparql.ttl` and
    `shacl-shacl.ttl` from upstream `shacl12-vocabularies/`.
  - `tests/`: the entire upstream `shacl12-test-suite/tests/` tree —
    `core/`, `node-expr/`, `sparql/`, `inference-rules/` and `sparql-rl/`,
    every manifest, every `.srl` file, and the upstream `tests/README.md`,
    `tests/node-expr/README.md` and `tests/sparql-rl/README.md` (kept
    verbatim — distinct from this file).

## Spec facts: the `sparql:plus` / `sparql:encode` spellings

`shnex-sparql.ttl` declares the SPARQL-function node-expression bindings
`sparql:plus` and `sparql:encode`. The SPARQL Working Group's own
`sparql-ns.ttl` names the same two functions `sparql:add` and
`sparql:encodeForUri`. Both spellings name the same built-in function; an
engine implementing SHACL 1.2 node expressions accepts both.

## Runtime copy

The same four files under `vocabularies/` are also written byte-exact to
`crates/shapes/spec/` by this script, for the reason given in the crate-local
`crates/shapes/spec/README.md`. That copy is covered by its own entry in
`scripts/conformance-frozen/`.

Harness: `crates/shapes/tests/w3c12_conformance.rs` runs every case under
`tests/` through the library API and reports the SHACL 1.2 rows of
`conformance-matrix.py`.
