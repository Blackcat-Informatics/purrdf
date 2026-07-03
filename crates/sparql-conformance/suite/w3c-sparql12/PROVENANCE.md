<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C SPARQL 1.2 conformance fixtures — DRAFT

> **⚠️ DRAFT — expected to churn.** SPARQL 1.2 (RDF 1.2 / RDF-star: triple terms,
> reifiers, `<<( … )>>`) is a **W3C draft**. This tree is a point-in-time freeze;
> re-sync it **deliberately** (not as routine maintenance), bumping the pinned
> commit and re-classifying, because upstream case sets and semantics still move.

## Source

- Upstream: **W3C `rdf-tests`** — <https://github.com/w3c/rdf-tests>, path
  `sparql/sparql12/` (W3C RDF & SPARQL Working Group; the top manifest is dated
  editor's-draft **2023-12-01**).
- Pinned commit: **`426c7df4b5d5d292e3ba09dc22e622ea301f230a`**.
- Every file is vendored **verbatim**, `manifest.ttl` included, each carrying its
  own `LicenseRef-W3C-Test-Suite` `.license` sidecar.

The top-level `mf:include` aggregator manifest is **not** vendored (the harness
discovers each leaf `manifest.ttl` directly); the ten group sub-manifests are:
`grouping`, `codepoint-escapes`, `syntax-triple-terms-{positive,negative}`,
`eval-triple-terms`, `expression`, `version`, `lang-basedir`, `rdf11`, `syntax`.

## Classification

The RDF-1.2 triple-term / reifier surface the native parser already supports
passes; the remainder is recorded in `crates/sparql-conformance/src/xfail.rs`
with a typed reason. Because this is a **draft**, a `parse-unsupported` row here
means "grammar not yet stabilized / not yet implemented for the draft" — as the
draft firms up, these are implemented (not left ledgered). Nothing is silently
skipped: every non-pass is a typed xfail, and an inventory tripwire guards the
vendored group set.
