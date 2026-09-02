<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C RDF 1.2 syntax + eval test corpus

This directory is a **vendored subset** of the official W3C RDF 1.2 test suites,
used by `crates/rdf/tests/native_codec_conformance.rs` to prove the native
`purrdf` text codecs (Turtle / TriG / N-Triples / N-Quads / RDF-XML) parse the
W3C syntax suites and **round-trip** them with no oxigraph dependency.

## Provenance

- **Upstream:** <https://github.com/w3c/rdf-tests>
- **Commit SHA:** `851911047ab1f01daca51498227cbf231e7d6705`
- **Upstream path (`syntax/`, `eval/`):**
  `rdf/rdf12/{rdf-turtle,rdf-trig,rdf-n-triples,rdf-n-quads,rdf-xml}/`
- **Upstream path (`iri/`):** `rdf/rdf11/{rdf-turtle,rdf-trig}/`

All five formats have an RDF 1.2 suite, so the `syntax/`/`eval/` sub-suites need
no RDF 1.1 fallback. The `iri/` sub-suite is the one exception, explained below.

## What was vendored (and what was trimmed)

For each format we took the **full `syntax/manifest.ttl`** (positive + negative
syntax) and the **full `eval/manifest.ttl`** (round-trip / evaluation tests),
plus every file each manifest references via `mf:action` / `mf:result`. RDF-XML
has no `syntax/` subdir upstream; its negative-syntax tests live in
`eval/manifest.ttl` and were taken with it.

### The `iri/` sub-suite (base-IRI resolution)

RDF 1.2 has **no** IRI test suite of its own, and the RDF 1.2 Turtle/TriG suites
carry no base-resolution eval tests. The base-IRI cases still live only in the
RDF 1.1 Turtle and TriG suites, so `turtle/iri/` and `trig/iri/` vendor them
from `rdf/rdf11/`:

| Case | What it pins |
|---|---|
| `IRI-resolution-01` | RFC 3986 §5.4 reference resolution against `http://a/bb/ccc/d;p?q` — including the empty reference `<>` |
| `IRI-resolution-02` | the same table against a base with a trailing slash |
| `IRI-resolution-07` | the same table against a base with a file path |
| `IRI-resolution-08` | `.`/`..` against bases with empty and colon-bearing segments |
| `IRIREF_datatype` | an `IRIREF` in datatype position |
| `IRI_with_four_digit_numeric_escape` | `\uXXXX` inside an `IRIREF` |
| `IRI_with_eight_digit_numeric_escape` | `\UXXXXXXXX` inside an `IRIREF` |

These are RDF 1.1 **syntax** documents whose every construct is also RDF 1.2
Turtle/TriG, and they are graded exactly like the RDF 1.2 eval tests — parse,
round-trip, and compare against `mf:result`. They are the end-to-end half of the
base-IRI contract that `crates/iri/tests/` states unit-by-unit against RFC 3986
§5.4; see `crates/iri/tests/PROVENANCE.md`.

Each `iri/manifest.ttl` is a **trimmed extract** of the upstream RDF 1.1
`manifest.ttl`: header, prefixes, `mf:assumedTestBase` and every retained entry
stanza are verbatim; only unrelated entries were removed. The upstream
`IRI-resolution` series is numbered `01, 02, 07, 08` — `03`–`06` do not exist
upstream and are not missing here.

**Trimmed:** the `c14n/` (canonicalization) sub-suites for N-Triples and N-Quads
were **not** vendored — they test RDF dataset canonicalization (RDFC-1.0), not
text-codec round-trip, and are out of scope here. The top-level aggregator
`manifest.ttl` (which only `mf:include`s the sub-manifests and the RDF 1.1
suites) was also not vendored; the harness reads the `syntax/`/`eval/`
sub-manifests directly.

Total: ~370 files, ~1.4 MB.

## Layout

```
w3c/
  turtle/   { syntax/manifest.ttl + .ttl,  eval/manifest.ttl + .ttl/.nt,
              iri/manifest.ttl + .ttl/.nt }
  trig/     { syntax/manifest.ttl + .trig, eval/manifest.ttl + .trig/.nq,
              iri/manifest.ttl + .trig/.nq }
  ntriples/ { syntax/manifest.ttl + .nt }
  nquads/   { syntax/manifest.ttl + .nq }
  rdfxml/   { eval/manifest.ttl + .rdf/.nt }
```

Each `manifest.ttl` declares its `mf:assumedTestBase`; the harness resolves each
action/result file's base IRI as `assumedTestBase + filename`.

## Adding a fixture moves three numbers, not one

The harness counts cases itself, but three places restate the count and none of
them derives it, so a new fixture reddens gates that look unrelated:

1. **`docs/CONFORMANCE.md`, the generated matrix block** — regenerate with
   `python3 scripts/conformance-matrix.py --write-doc`. Note that
   `make conformance` *verifies* this block but never writes it, so it fails
   with a diff until you regenerate.
2. **`docs/CONFORMANCE.md`, the hand-maintained "Scoreboard (per engine)" row** —
   `--write-doc` does **not** touch it. Update the `N / N` and the per-format
   split by hand.
3. **`scripts/check-doc-claims.py`, the syntax-codec claim** — only the suite
   TOTAL reaches the generated matrix block, so that claim hard-anchors
   `nquads`/`ntriples`/`rdfxml`/`trig` and derives `turtle` as the remainder.
   Adding a Turtle fixture needs no edit there; adding one to any **other**
   format means moving its anchor, or the remainder absorbs the change and the
   gate fails naming turtle. Vendoring the `iri/` sub-suite moved the `trig`
   anchor from 60 to 67 for exactly this reason.

The split is anchored rather than derived on purpose: it is what makes a
per-format drift visible instead of cancelling out inside the total.

## License

The vendored test files are W3C test-suite content, dual-licensed under the
"W3C Test Suite License" and the "W3C 3-clause BSD License". See `LICENSE`
(copied verbatim from the upstream `LICENSE.md`).
