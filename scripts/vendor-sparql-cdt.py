#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Vendor the upstream SEP-0009 SPARQL Composite Datatypes (CDT) conformance corpus.

Fetches the pinned upstream commit's ``tests/`` tree from `awslabs/SPARQL-CDTs`
(Apache-2.0) via the GitHub REST API tarball endpoint and writes it verbatim
into ``vectors/sparql-cdt/``, alongside a first-party ``PROVENANCE.md`` (this
script's own doc — separate from the upstream ``tests/README.md`` it vendors)
and a verbatim copy of the upstream ``LICENSE``.

The `cdt:` namespace (`http://w3id.org/awslabs/neptune/SPARQL-CDTs/`) used
throughout the vendored manifests and fixtures is third-party, spec-defined
vocabulary from SEP-0009 — vendoring it is not PurRDF minting a vocabulary,
exactly as `vectors/shexTest` vendoring the ShEx namespace is not.

This script is deterministic and re-runnable: the upstream commit is pinned,
the output directory is fully replaced (not merged) on every run, files are
written by relative path with no embedded timestamps, and the tree is byte-
identical run over run. Re-vendoring against a newer upstream commit is a
deliberate edit to `COMMIT` below, followed by re-running this script and
`python3 scripts/check-corpus-frozen.py --update`.

    python3 scripts/vendor-sparql-cdt.py
"""

from __future__ import annotations

import argparse
import io
import shutil
import tarfile
import urllib.request
from pathlib import Path

REPO = "awslabs/SPARQL-CDTs"
COMMIT = "e0a746561ad6a2db0f70fdcccb57eadea04f50c8"
LICENSE_SPDX = "Apache-2.0"

TARBALL_URL = f"https://api.github.com/repos/{REPO}/tarball/{COMMIT}"

PROVENANCE = f"""<!--
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

- Upstream: <https://github.com/{REPO}> — the reference implementation and
  test suite for SEP-0009 (SPARQL Extension Proposal for Composite Datatypes:
  `cdt:List` and `cdt:Map` literals, the `FOLD`/`UNFOLD` operators, and their
  `ORDER BY` extension).
- Pinned commit: `{COMMIT}` — pinned for reproducible builds and to track
  upstream errata explicitly, the same hygiene every vendored suite in this
  repo follows.
- License: **{LICENSE_SPDX}**, per the upstream repository's license
  declaration; the upstream `LICENSE` file is vendored verbatim alongside this
  tree.
- Retrieval: `scripts/vendor-sparql-cdt.py` fetches the pinned commit's full
  source tree as a tarball from the GitHub REST API
  (`GET https://api.github.com/repos/{REPO}/tarball/{COMMIT}`, which redirects
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

Harness: this chunk lands the corpus only — no `purrdf-sparql-conformance`
suite reads it yet (no `crates/sparql-conformance/suite/` entry, no
`conformance-matrix.py` row); wiring in `FOLD`/`UNFOLD`/CDT evaluation is
separate follow-on work.
"""


def fetch_tarball() -> bytes:
    """Download the pinned commit's tarball from the GitHub REST API."""
    request = urllib.request.Request(  # noqa: S310 - pinned https GitHub API host
        TARBALL_URL, headers={"User-Agent": "purrdf-vendor-sparql-cdt"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:  # noqa: S310
        return response.read()


def extract_prefix(archive: bytes, prefix: str) -> dict[str, bytes]:
    """Return ``{relative_posix_path: content}`` for every regular file whose
    path (after stripping the tarball's single top-level directory) starts
    with *prefix*, keyed relative to that prefix."""
    files: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
        for member in tar.getmembers():
            if not member.isfile():
                continue
            # The tarball root is a single synthetic directory, e.g.
            # "awslabs-SPARQL-CDTs-<shortsha>/tests/...".
            _root, _sep, rest = member.name.partition("/")
            if not rest.startswith(prefix):
                continue
            extracted = tar.extractfile(member)
            if extracted is None:
                continue
            files[rest[len(prefix) :]] = extracted.read()
    return files


def write_tree(output: Path, files: dict[str, bytes]) -> None:
    for relative in sorted(files):
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(files[relative])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "vectors" / "sparql-cdt",
        help="destination directory (default: vectors/sparql-cdt at the repo root)",
    )
    args = parser.parse_args()

    archive = fetch_tarball()
    tests = extract_prefix(archive, "tests/")
    if not tests:
        raise SystemExit(f"no files found under tests/ in the {REPO}@{COMMIT} tarball")
    root_files = extract_prefix(archive, "")
    license_text = root_files.get("LICENSE")
    if license_text is None:
        raise SystemExit(f"upstream {REPO}@{COMMIT} tarball has no root LICENSE file")

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    write_tree(output, tests)
    (output / "LICENSE").write_bytes(license_text)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")

    print(f"vendored {len(tests)} file(s) from {REPO}@{COMMIT} into {output}")


if __name__ == "__main__":
    main()
