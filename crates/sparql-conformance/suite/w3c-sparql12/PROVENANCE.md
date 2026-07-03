<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C SPARQL 1.2 conformance fixtures

PurRDF treats **SPARQL 1.2 / RDF 1.2** (RDF-star: triple terms, reifiers,
`<<( … )>>`, base direction) as a **complete, first-class specification** — on
exactly the same footing as SPARQL 1.1. A spec may receive errata and updates,
like any spec; that is a reason to **pin and re-sync deliberately**, not a reason
to treat the spec as provisional. There is no "wait until it's final" here.

## Source

- Upstream: **W3C `rdf-tests`** — <https://github.com/w3c/rdf-tests>, path
  `sparql/sparql12/` (W3C RDF & SPARQL Working Group).
- Pinned commit: **`426c7df4b5d5d292e3ba09dc22e622ea301f230a`** — pinned for
  reproducible builds and to track upstream errata explicitly, the same hygiene
  every vendored suite in this repo follows.
- Every file is vendored **verbatim**, `manifest.ttl` included, each carrying its
  own `LicenseRef-W3C-Test-Suite` `.license` sidecar.

The top-level `mf:include` aggregator manifest is **not** vendored (the harness
discovers each leaf `manifest.ttl` directly); the ten group sub-manifests are:
`grouping`, `codepoint-escapes`, `syntax-triple-terms-{positive,negative}`,
`eval-triple-terms`, `expression`, `version`, `lang-basedir`, `rdf11`, `syntax`.

## Classification

Cases the engine already satisfies pass (codepoint-escapes, grouping, rdf11, and
a substantial part of the triple-term syntax). The remaining non-passes are
recorded in `crates/sparql-conformance/src/xfail.rs` with a typed reason. These
are **genuine unimplemented RDF-1.2 features** — triple-term/reifier grammar the
parser does not yet accept (`parse-unsupported`) and triple-term evaluation
semantics (`unsupported-construct`) — i.e. **real work to implement**, not
provisional-spec placeholders. Every non-pass is a typed xfail (no silent skips),
and an inventory tripwire guards the vendored group set; the ledger shrinks as
each feature lands.
