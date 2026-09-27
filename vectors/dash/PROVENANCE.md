<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored DASH document (`http://datashapes.org/dash`)

Frozen copy of the document served at the ontology IRI `http://datashapes.org/dash`, vendored
so the SHACL conformance harnesses can supply the `owl:imports <http://datashapes.org/dash>`
of the approved W3C test `sparql/component/validator-001` (SHACL 1.0 suite and
SHACL 1.2 suite) through the import table, as any caller would. **Do not
hand-edit** — byte-frozen third-party data, regenerated only by re-running
`python3 scripts/vendor-dash.py`. The freeze is enforced: `make check` runs
`scripts/check-corpus-frozen.py`, which SHA-256-verifies every file here against
`scripts/conformance-frozen/vectors-dash.sha256`, so a silent content edit fails
the build.

## Source

- Ontology IRI: `http://datashapes.org/dash`
- Retrieved from: `https://datashapes.org/dash` with `Accept: text/turtle` (the publisher redirects the
  `http:` IRI to it and serves `https://datashapes.org/dash.ttl`)
- Publisher: TopQuadrant, Inc. (datashapes.org)
- Retrieval date: 2026-09-26
- Served `Last-Modified`: Fri, 27 May 2022 03:15:02 GMT
- SHA-256 of `dash.ttl`: `01a32d725a0093910d17596102dc38a0ead231fa9d8ebed0bb433abfb705ebf4` — the pin: `scripts/vendor-dash.py` refuses
  served bytes with any other digest.
- Upstream source repository: the publisher also maintains a `dash.ttl` in
  <https://github.com/TopQuadrant/shacl>. The served document is byte-identical to
  none of that file's revisions, and the current revision is a different document:
  it also imports `http://topbraid.org/tosh`, whose shapes change the verdict of
  `validator-001`. The document at the IRI is the one vendored.
- License: **none stated.** The served document carries no licence or copyright
  statement, and no licence is asserted for it here. It is listed in the
  third-party carve-out table of `LICENSING.md`.

## What the harnesses do with it

The document imports `<http://www.w3.org/ns/shacl#>`, which the harnesses supply
from the vendored W3C SHACL 1.2 vocabulary (`vectors/shacl12/vocabularies/shacl.ttl`).
Some of its declarations are not well-formed SHACL 1.2 — among them an ASK validator
under `sh:nodeValidator` for `sh:HasValueConstraintComponent`, a `MINUS` in a
pre-bound validator for `sh:EqualsConstraintComponent`, an ASK
`sh:propertyValidator` for `dash:SubSetOfConstraintComponent` and a
`sh:SPARQLFunction` parameter named `value` (`dash:uriTemplate`). No shape of
`validator-001` reaches any of them, so the load accepts them as inert defects and
`purrdf shapes lint` reports each as a finding.
