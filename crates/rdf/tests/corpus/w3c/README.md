<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C RDF 1.2 syntax + eval test corpus

This directory is a **vendored subset** of the official W3C RDF 1.2 test suites,
used by `crates/rdf/tests/native_codec_conformance.rs` to prove the native
`purrdf` text codecs (Turtle / TriG / N-Triples / N-Quads / RDF-XML) parse the
W3C syntax suites and **round-trip** them with no oxigraph dependency (#909 /
EPIC #906).

## Provenance

- **Upstream:** <https://github.com/w3c/rdf-tests>
- **Commit SHA:** `851911047ab1f01daca51498227cbf231e7d6705`
- **Upstream path:** `rdf/rdf12/{rdf-turtle,rdf-trig,rdf-n-triples,rdf-n-quads,rdf-xml}/`
- All five formats have an RDF 1.2 suite, so no RDF 1.1 fallback was needed.

## What was vendored (and what was trimmed)

For each format we took the **full `syntax/manifest.ttl`** (positive + negative
syntax) and the **full `eval/manifest.ttl`** (round-trip / evaluation tests),
plus every file each manifest references via `mf:action` / `mf:result`. RDF-XML
has no `syntax/` subdir upstream; its negative-syntax tests live in
`eval/manifest.ttl` and were taken with it.

**Trimmed:** the `c14n/` (canonicalization) sub-suites for N-Triples and N-Quads
were **not** vendored — they test RDF dataset canonicalization (RDFC-1.0), not
text-codec round-trip, and are out of scope for #909. The top-level aggregator
`manifest.ttl` (which only `mf:include`s the sub-manifests and the RDF 1.1
suites) was also not vendored; the harness reads the `syntax/`/`eval/`
sub-manifests directly.

Total: ~340 files, ~1.4 MB.

## Layout

```
w3c/
  turtle/   { syntax/manifest.ttl + .ttl,  eval/manifest.ttl + .ttl/.nt }
  trig/     { syntax/manifest.ttl + .trig, eval/manifest.ttl + .trig/.nq }
  ntriples/ { syntax/manifest.ttl + .nt }
  nquads/   { syntax/manifest.ttl + .nq }
  rdfxml/   { eval/manifest.ttl + .rdf/.nt }
```

Each `manifest.ttl` declares its `mf:assumedTestBase`; the harness resolves each
action/result file's base IRI as `assumedTestBase + filename`.

## License

The vendored test files are W3C test-suite content, dual-licensed under the
"W3C Test Suite License" and the "W3C 3-clause BSD License". See `LICENSE`
(copied verbatim from the upstream `LICENSE.md`).
