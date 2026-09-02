<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored SEP-0009 SPARQL Composite Datatypes (CDT) conformance suite

Frozen copy of the upstream `awslabs/SPARQL-CDTs` `tests/` tree, vendored for
the `purrdf-sparql-conformance` harness. **Do not hand-edit** — treat exactly
like the GTS vectors and the `vectors/shexTest` corpus: byte-frozen third-party
conformance data, regenerated only by re-running
`python3 scripts/vendor-sparql-cdt.py`. The freeze is enforced: `make check`
runs `scripts/check-corpus-frozen.py`, which SHA-256-verifies every file here
against `scripts/conformance-frozen/vectors-sparql-cdt.sha256`, so a silent
content edit fails the build. A deliberate re-vendor regenerates that manifest
with `python3 scripts/check-corpus-frozen.py --update`.

## Source

- Upstream: <https://github.com/awslabs/SPARQL-CDTs> — the reference implementation and
  test suite for SEP-0009 (SPARQL Extension Proposal for Composite Datatypes:
  `cdt:List` and `cdt:Map` literals, the `FOLD`/`UNFOLD` operators, and their
  `ORDER BY` extension).
- Pinned commit: `e0a746561ad6a2db0f70fdcccb57eadea04f50c8` — pinned for reproducible builds and to track
  upstream errata explicitly, the same hygiene every vendored suite in this
  repo follows.
- License: **Apache-2.0**, per the upstream repository's license
  declaration; the upstream `LICENSE` file is vendored verbatim alongside this
  tree.
- Retrieval: `scripts/vendor-sparql-cdt.py` fetches the pinned commit's full
  source tree as a tarball from the GitHub REST API
  (`GET https://api.github.com/repos/awslabs/SPARQL-CDTs/tarball/e0a746561ad6a2db0f70fdcccb57eadea04f50c8`, which redirects
  to a `codeload.github.com` archive of that exact commit) and extracts the
  `tests/` subtree verbatim — no per-file API calls, no upstream `git` clone.
- Vendored subset: the entire `tests/` tree — `manifest-all.ttl` (the `mf:`
  aggregator manifest, `mf:include`-ing the six group manifests below) and the
  `unfold/`, `fold/`, `list-functions/`, `map-functions/`, `orderby/`, and
  `bnodes/` directories in full, including the upstream `tests/README.md`
  (kept verbatim at the root of this tree — distinct from this file).

## Namespace

Every manifest and fixture here uses
`cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>`, a third-party,
spec-defined namespace. Vendoring it is not PurRDF minting a vocabulary — the
same posture as the ShEx namespace in `vectors/shexTest`.

## Entry counts (pinned; see `crates/sparql-conformance/tests/suite_inventory.rs`)

| Group            | `mf:entries` | Files |
|------------------|-------------:|------:|
| `unfold`         |           42 |    77 |
| `fold`           |           30 |    33 |
| `list-functions` |          287 |   290 |
| `map-functions`  |          196 |   199 |
| `orderby`        |           27 |    30 |
| `bnodes`         |           76 |   118 |
| **Total**        |      **658** |       |

Harness: `crates/sparql-conformance/tests/cdt_corpus.rs` runs every case here
through `manifest-all.ttl`'s `mf:include` aggregator and reports the
`SPARQL CDT (SEP-0009, vendored corpus)` row of `conformance-matrix.py`. The
corpus deliberately stays OUTSIDE `crates/sparql-conformance/suite/`, whose
`datatest_stable` root folds every manifest it finds into the one full-corpus
row; keeping it here is what gives it a row of its own.
