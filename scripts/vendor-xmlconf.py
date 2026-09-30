#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the W3C XML Conformance Test Suite (xmlts20130923).

Fetches the suite tarball from www.w3.org at a pinned URL, checks it against a
pinned SHA-256, and writes the ``xmlconf/`` tree verbatim (regular files only,
no path may leave the destination) into ``vectors/xmlconf/`` alongside a
first-party ``PROVENANCE.md``. Deterministic and re-runnable: the output
directory is fully replaced on every run. Re-vendoring is a deliberate edit to
``URL`` and ``SHA256`` below, followed by this script and a regeneration of
``scripts/conformance-frozen/vectors-xmlconf.sha256``.

    python3 scripts/vendor-xmlconf.py
"""

from __future__ import annotations

import argparse
import hashlib
import io
import shutil
import tarfile
import urllib.request
from pathlib import PurePosixPath
from pathlib import Path

URL = "https://www.w3.org/XML/Test/xmlts20130923.tar.gz"
SHA256 = "9b61db9f5dbffa545f4b8d78422167083a8568c59bd1129f94138f936cf6fc1f"
TOP = "xmlconf"

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C XML Conformance Test Suite

Frozen copy of the W3C XML Conformance Test Suite, version 2013-09-23
(`xmlts20130923`), vendored for `crates/lex/tests/xmlconf.rs`, which grades
`purrdf_lex::xml` against it. **Do not hand-edit**: the freeze is enforced by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-xmlconf.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-xmlconf.py`.

## Source

- Upstream: <https://www.w3.org/XML/Test/>
- Retrieval: `{URL}`
- Tarball SHA-256: `{SHA256}`
- The tarball's `xmlconf/` tree is written verbatim (3386 files; the tarball's
  own layout, file bytes and names are unchanged; only the archive's owner and
  permission bits are dropped).
- Licence: the suite is a collection of contributed sub-suites (James Clark's
  XMLTEST, Sun Microsystems, OASIS/NIST, IBM, Fuji Xerox, the University of
  Edinburgh), each under the terms stated in its own directory (for example
  `xmltest/readme.html`) and the W3C test-suite licence at
  <https://www.w3.org/Consortium/Legal/2008/04-testsuite-copyright.html>. The
  files are redistributed here unmodified and are test data only: no crate
  compiles any of it in.

## Contents

`xmlconf.xml` is the master manifest (`TESTSUITE`, one `TESTCASES` per
sub-suite, each `TEST` naming its document by `URI` with the attributes
`TYPE`, `VERSION`, `EDITION`, `ENTITIES`, `NAMESPACE`, `RECOMMENDATION` and
`OUTPUT`). `testcases.dtd` describes those attributes.
"""


def fetch() -> bytes:
    request = urllib.request.Request(URL, headers={"User-Agent": "purrdf-vendor-xmlconf"})
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310 - pinned https host
        return response.read()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "vectors" / "xmlconf",
        help="destination directory (default: vectors/xmlconf at the repo root)",
    )
    args = parser.parse_args()

    data = fetch()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"{URL}: SHA-256 {got}, pinned {SHA256}")

    files: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        for member in tar.getmembers():
            path = PurePosixPath(member.name)
            if member.isdir():
                continue
            if not member.isreg():
                raise SystemExit(f"{member.name}: not a regular file")
            if path.is_absolute() or ".." in path.parts or path.parts[0] != TOP:
                raise SystemExit(f"{member.name}: outside {TOP}/")
            handle = tar.extractfile(member)
            assert handle is not None
            files["/".join(path.parts[1:])] = handle.read()

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    for rel, body in sorted(files.items()):
        target = output / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")
    print(f"vendored {len(files)} file(s) from {URL} into {output}")


if __name__ == "__main__":
    main()
