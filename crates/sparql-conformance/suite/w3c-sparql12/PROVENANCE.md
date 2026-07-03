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

The engine satisfies the **entire** vendored SPARQL 1.2 surface: the RDF-star
triple-term/reifier/annotation grammar (`syntax-triple-terms-{positive,negative}`),
triple-term evaluation (`eval-triple-terms`), and the `expression`, `version`,
`lang-basedir`, `grouping`, `codepoint-escapes`, and `rdf11` groups all pass. As
of the current harness run there are **zero** ledgered SPARQL 1.2 residuals in
`crates/sparql-conformance/src/xfail.rs` — the triple-term/reifier grammar and
evaluation semantics that earlier carried typed `parse-unsupported` /
`unsupported-construct` reasons are now fully implemented, so every 1.2 case is a
genuine pass. Coverage stays **enforced, not assumed**: every case is run (no
silent skips), an inventory tripwire guards the vendored group set, and any future
non-pass must re-enter the ledger as a typed xfail.
