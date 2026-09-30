#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the official YAML test suite.

Fetches the `data-2022-01-17` release of `yaml/yaml-test-suite` (the `data`
branch layout: one directory per case holding `in.yaml`, `in.json`, `error`,
`test.event`, ...) at a pinned commit and the repository `License` at a pinned
commit of the `main` branch (the data branch carries none), checks the
extracted payload against a pinned tree digest and the License against a pinned
SHA-256, and writes them verbatim into ``vectors/yaml-test-suite/`` alongside a
first-party ``PROVENANCE.md``. Symlinks (the suite's `name/` and `tags/` index
directories) are not vendored. Deterministic and re-runnable: the output
directory is fully replaced on every run. Re-vendoring is a deliberate edit to
the pins below, followed by this script and
`python3 scripts/check-corpus-frozen.py --update`.

    python3 scripts/vendor-yaml-test-suite.py
"""

from __future__ import annotations

import argparse
import hashlib
import io
import shutil
import tarfile
import urllib.request
from pathlib import Path

REPO = "yaml/yaml-test-suite"
TAG = "data-2022-01-17"
COMMIT = "5f49729577242103ae23838ac2ad4d9145aec126"
# SHA-256 over the sorted `<file sha256>  <relpath>\n` lines of every regular
# file in the archive at COMMIT (the same shape as a `sha256sum` manifest), and
# the file count that digest covers.
TREE_SHA256 = "0897d6cb42ba393e6365ec88e1d29b699f76e25a8aa0179c5769ad3710d2ccf4"
TREE_FILES = 1887
LICENSE_SPDX = "MIT"
LICENSE_COMMIT = "da267a5c4782e7361e82889e76c0dc7df0e1e870"
LICENSE_SHA256 = "c9562189164244554a69ab3f29d2d93ed9492c165723aaaa5fffc932cdbbfc85"

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored YAML test suite

Frozen copy of the official YAML test suite, vendored for
`crates/lex/tests/yaml_test_suite.rs`, which grades the `purrdf_lex::yaml`
reader against it. **Do not hand-edit**: the freeze is enforced by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-yaml-test-suite.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-yaml-test-suite.py`.

## Source

- Upstream: <https://github.com/{REPO}>
- Release: `{TAG}` (the `data` branch layout), pinned commit `{COMMIT}`
- License: **{LICENSE_SPDX}** (`LICENSE`, the upstream `License` file of `main`
  commit `{LICENSE_COMMIT}`, vendored verbatim; SHA-256 `{LICENSE_SHA256}`)
- Retrieval: `https://codeload.github.com/{REPO}/tar.gz/{COMMIT}`; every regular
  file is extracted verbatim (symlinks are skipped) and the set is checked
  against the pinned tree digest `{TREE_SHA256}` over {TREE_FILES} files.

## Contents

One directory per case (`<ID>/`, or `<ID>/<NN>/` for an ID that holds several
inputs): `===` the case name, `in.yaml` the input, `in.json` the expected JSON
value(s) (absent for error cases and for the few cases with no JSON form),
`error` (present when the input is invalid YAML), `test.event`, and for some
cases `out.yaml` / `emit.yaml`.
"""


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "purrdf-vendor-yaml-test-suite"})
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310 - pinned https host
        return response.read()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "vectors" / "yaml-test-suite",
        help="destination directory (default: vectors/yaml-test-suite at the repo root)",
    )
    args = parser.parse_args()

    archive = fetch(f"https://codeload.github.com/{REPO}/tar.gz/{COMMIT}")
    payload: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
        for member in tar:
            if not member.isreg():
                continue
            rel = member.name.split("/", 1)[1]
            payload[rel] = tar.extractfile(member).read()  # type: ignore[union-attr]

    entries = sorted((rel, hashlib.sha256(data).hexdigest()) for rel, data in payload.items())
    tree = hashlib.sha256("".join(f"{d}  {r}\n" for r, d in entries).encode()).hexdigest()
    if tree != TREE_SHA256 or len(entries) != TREE_FILES:
        raise SystemExit(
            f"{REPO}@{COMMIT}: tree {tree} over {len(entries)} files, "
            f"pinned {TREE_SHA256} over {TREE_FILES}"
        )

    license_bytes = fetch(f"https://raw.githubusercontent.com/{REPO}/{LICENSE_COMMIT}/License")
    got = hashlib.sha256(license_bytes).hexdigest()
    if got != LICENSE_SHA256:
        raise SystemExit(f"License@{LICENSE_COMMIT}: SHA-256 {got}, pinned {LICENSE_SHA256}")
    payload["LICENSE"] = license_bytes

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    for rel, data in payload.items():
        target = output / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")
    print(f"vendored {len(payload)} file(s) from {REPO}@{COMMIT} into {output}")


if __name__ == "__main__":
    main()
