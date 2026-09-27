#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the DASH document served at the ontology IRI ``http://datashapes.org/dash``.

The approved W3C test ``sparql/component/validator-001`` — in the SHACL 1.0 suite
(``vectors/shacl/``) and the SHACL 1.2 suite (``vectors/shacl12/``) — declares
``owl:imports <http://datashapes.org/dash>``. PurRDF fetches nothing at validation
time, so the conformance harnesses supply that document through the import table
like any other caller. This script vendors it: it dereferences the IRI with
``Accept: text/turtle`` (the publisher redirects ``http:`` to ``https:`` and
negotiates the Turtle representation, ``https://datashapes.org/dash.ttl``), checks
the bytes against the pinned SHA-256 below, and writes them verbatim to
``vectors/dash/dash.ttl`` beside a first-party ``PROVENANCE.md``.

The document is not in any commit of the publisher's source repository byte for
byte, so there is no commit to pin; the digest is the pin. A served document whose
digest differs is refused, never written: re-vendoring a changed document is a
deliberate edit to ``SHA256`` and ``RETRIEVED`` below, followed by re-running this
script and ``python3 scripts/check-corpus-frozen.py --update``.

The served document states no licence, and this script asserts none; see
``PROVENANCE.md`` and the carve-out table in ``LICENSING.md``.

This script is deterministic and re-runnable: the output directory is fully
replaced (not merged) on every run and the written bytes are the pinned bytes.

    python3 scripts/vendor-dash.py
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import urllib.request
from pathlib import Path

# The ontology IRI the W3C test imports, and the URL it is dereferenced at.
ONTOLOGY_IRI = "http://datashapes.org/dash"
URL = "https://datashapes.org/dash"
# The representation the publisher negotiates for `Accept: text/turtle`.
TURTLE_URL = "https://datashapes.org/dash.ttl"
ACCEPT = "text/turtle"
PUBLISHER = "TopQuadrant, Inc. (datashapes.org)"
SHA256 = "01a32d725a0093910d17596102dc38a0ead231fa9d8ebed0bb433abfb705ebf4"
# The day the pinned bytes were retrieved, and the `Last-Modified` the publisher
# served them with.
RETRIEVED = "2026-09-26"
LAST_MODIFIED = "Fri, 27 May 2022 03:15:02 GMT"

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored DASH document (`{ONTOLOGY_IRI}`)

Frozen copy of the document served at the ontology IRI `{ONTOLOGY_IRI}`, vendored
so the SHACL conformance harnesses can supply the `owl:imports <{ONTOLOGY_IRI}>`
of the approved W3C test `sparql/component/validator-001` (SHACL 1.0 suite and
SHACL 1.2 suite) through the import table, as any caller would. **Do not
hand-edit** — byte-frozen third-party data, regenerated only by re-running
`python3 scripts/vendor-dash.py`. The freeze is enforced: `make check` runs
`scripts/check-corpus-frozen.py`, which SHA-256-verifies every file here against
`scripts/conformance-frozen/vectors-dash.sha256`, so a silent content edit fails
the build.

## Source

- Ontology IRI: `{ONTOLOGY_IRI}`
- Retrieved from: `{URL}` with `Accept: {ACCEPT}` (the publisher redirects the
  `http:` IRI to it and serves `{TURTLE_URL}`)
- Publisher: {PUBLISHER}
- Retrieval date: {RETRIEVED}
- Served `Last-Modified`: {LAST_MODIFIED}
- SHA-256 of `dash.ttl`: `{SHA256}` — the pin: `scripts/vendor-dash.py` refuses
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
"""


def fetch() -> bytes:
    """Dereference the ontology IRI's Turtle representation."""
    request = urllib.request.Request(  # noqa: S310 - pinned https publisher host
        URL, headers={"Accept": ACCEPT, "User-Agent": "purrdf-vendor-dash"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:  # noqa: S310
        return response.read()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    repo_root = Path(__file__).resolve().parent.parent
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "vectors" / "dash",
        help="destination directory (default: vectors/dash at the repo root)",
    )
    args = parser.parse_args()

    document = fetch()
    digest = hashlib.sha256(document).hexdigest()
    if digest != SHA256:
        raise SystemExit(
            f"{URL} served a document with SHA-256 {digest}, not the pinned {SHA256}; "
            "nothing was written. Re-vendoring a changed document is a deliberate edit "
            "to SHA256 and RETRIEVED in this script."
        )

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    (output / "dash.ttl").write_bytes(document)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")
    print(f"vendored {ONTOLOGY_IRI} ({len(document)} bytes, sha256 {digest}) into {output}")


if __name__ == "__main__":
    main()
