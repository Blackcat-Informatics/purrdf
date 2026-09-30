#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the upstream Wycheproof Ed25519 verification vectors.

Fetches `testvectors_v1/ed25519_test.json` and the repository `LICENSE` from
`C2SP/wycheproof` at a pinned commit, checks each against a pinned SHA-256, and
writes them verbatim into ``vectors/wycheproof/`` alongside a first-party
``PROVENANCE.md``. Deterministic and re-runnable: the output directory is fully
replaced on every run. Re-vendoring is a deliberate edit to `COMMIT` and the
digests below, followed by this script and
`python3 scripts/check-corpus-frozen.py --update`.

    python3 scripts/vendor-wycheproof-ed25519.py
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import urllib.request
from pathlib import Path

REPO = "C2SP/wycheproof"
COMMIT = "3fa63dd0344abb611f1fb1d77e119938603ea230"
LICENSE_SPDX = "Apache-2.0"

# upstream path -> (vendored name, SHA-256 of the bytes at COMMIT)
FILES = {
    "testvectors_v1/ed25519_test.json": (
        "ed25519_test.json",
        "752d2ea7d7c6cf4736381b6cbacb61f8182b126ab7cd9b058f00c50084975536",
    ),
    "LICENSE": (
        "LICENSE",
        "58d1e17ffe5109a7ae296caafcadfdbe6a7d176f0bc4ab01e12a689b0499d8bd",
    ),
}

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored Wycheproof Ed25519 verification vectors

Frozen copy of the upstream Project Wycheproof EdDSA verification vectors,
vendored for `crates/ed25519/tests/wycheproof.rs`. **Do not hand-edit**: the
freeze is enforced by `scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-wycheproof.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-wycheproof-ed25519.py`.

## Source

- Upstream: <https://github.com/{REPO}>
- Pinned commit: `{COMMIT}`
- License: **{LICENSE_SPDX}** (the upstream `LICENSE`, vendored verbatim)
- Retrieval: `https://raw.githubusercontent.com/{REPO}/{COMMIT}/<path>`, each
  file checked against the SHA-256 pinned in the script.

| Vendored file | Upstream path | SHA-256 |
|---|---|---|
""" + "".join(
    f"| `{name}` | `{path}` | `{digest}` |\n" for path, (name, digest) in FILES.items()
) + """
## Contents

`ed25519_test.json`: schema `eddsa_verify_schema_v1.json`, 151 tests in 78
groups (one public key per group): 88 `valid` and 63 `invalid` cases. The
upstream file has no `acceptable` results and carries no private keys, so it
grades verification only.
"""


def fetch(path: str) -> bytes:
    url = f"https://raw.githubusercontent.com/{REPO}/{COMMIT}/{path}"
    request = urllib.request.Request(url, headers={"User-Agent": "purrdf-vendor-wycheproof"})
    with urllib.request.urlopen(request, timeout=60) as response:  # noqa: S310 - pinned https host
        return response.read()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "vectors" / "wycheproof",
        help="destination directory (default: vectors/wycheproof at the repo root)",
    )
    args = parser.parse_args()

    payload = {}
    for path, (name, want) in FILES.items():
        data = fetch(path)
        got = hashlib.sha256(data).hexdigest()
        if got != want:
            raise SystemExit(f"{path}@{COMMIT}: SHA-256 {got}, pinned {want}")
        payload[name] = data

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    for name, data in payload.items():
        (output / name).write_bytes(data)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")
    print(f"vendored {len(payload)} file(s) from {REPO}@{COMMIT} into {output}")


if __name__ == "__main__":
    main()
