#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the upstream JSONTestSuite ``test_parsing`` corpus.

Fetches `nst/JSONTestSuite` at a pinned commit through git (which verifies every
object against the commit id), checks the commit id, the ``test_parsing`` tree id
and the repository `LICENSE` against pinned digests, and writes the 318
`y_` / `n_` / `i_` documents and the license verbatim into
``vectors/JSONTestSuite/`` alongside a first-party ``PROVENANCE.md``.
Deterministic and re-runnable: the output directory is fully replaced on every
run. Re-vendoring is a deliberate edit to `COMMIT` and the digests below,
followed by this script and `python3 scripts/check-corpus-frozen.py --update`.

    python3 scripts/vendor-jsontestsuite.py
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import tempfile
from pathlib import Path

REPO = "nst/JSONTestSuite"
COMMIT = "1ef36fa01286573e846ac449e8683f8833c5b26a"
# `git rev-parse COMMIT:test_parsing`: the content id of the whole corpus.
TREE = "b936f9acdd24b9f5fefe68b90b9beab2c681137a"
LICENSE_SPDX = "MIT"
LICENSE_SHA256 = "8bd0e0578be788c617ea01d18b2a8146e3746ae50bddadc65a5f9d3aad08ad49"
COUNTS = {"y_": 95, "n_": 188, "i_": 35}

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored JSONTestSuite parsing corpus

Frozen copy of the upstream JSONTestSuite `test_parsing/` documents, vendored for
`crates/lex/tests/json_test_suite.rs`. **Do not hand-edit**: the freeze is
enforced by `scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-JSONTestSuite.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-jsontestsuite.py`.

## Source

- Upstream: <https://github.com/{REPO}>
- Pinned commit: `{COMMIT}`
- Pinned `test_parsing` tree: `{TREE}`
- License: **{LICENSE_SPDX}** (the upstream `LICENSE`, vendored verbatim,
  SHA-256 `{LICENSE_SHA256}`)
- Retrieval: `git fetch --depth 1 https://github.com/{REPO}.git {COMMIT}`, the
  commit and tree ids checked against the pins in the script.

## Contents

`test_parsing/`: {sum(COUNTS.values())} documents named for the result RFC 8259
requires of a parser. {COUNTS["y_"]} `y_*` must be accepted, {COUNTS["n_"]} `n_*`
must be refused, and {COUNTS["i_"]} `i_*` are left to the implementation, so the
suite's own README calls them implementation-defined.
"""


def git(*args: str, cwd: Path) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "vectors" / "JSONTestSuite",
        help="destination directory (default: vectors/JSONTestSuite at the repo root)",
    )
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as scratch_name:
        scratch = Path(scratch_name)
        git("init", "-q", cwd=scratch)
        git("fetch", "-q", "--depth", "1", f"https://github.com/{REPO}.git", COMMIT, cwd=scratch)
        head = git("rev-parse", "FETCH_HEAD", cwd=scratch)
        if head != COMMIT:
            raise SystemExit(f"fetched {head}, pinned {COMMIT}")
        tree = git("rev-parse", "FETCH_HEAD:test_parsing", cwd=scratch)
        if tree != TREE:
            raise SystemExit(f"test_parsing tree {tree}, pinned {TREE}")
        extracted = scratch / "extracted"
        extracted.mkdir()
        archive = subprocess.run(
            ["git", "archive", "FETCH_HEAD", "test_parsing", "LICENSE"],
            cwd=scratch,
            check=True,
            capture_output=True,
        ).stdout
        subprocess.run(["tar", "-x", "-C", str(extracted)], input=archive, check=True)

        license_bytes = (extracted / "LICENSE").read_bytes()
        got = hashlib.sha256(license_bytes).hexdigest()
        if got != LICENSE_SHA256:
            raise SystemExit(f"LICENSE SHA-256 {got}, pinned {LICENSE_SHA256}")
        documents = sorted((extracted / "test_parsing").iterdir())
        for prefix, want in COUNTS.items():
            found = sum(1 for path in documents if path.name.startswith(prefix))
            if found != want:
                raise SystemExit(f"{found} `{prefix}` documents, pinned {want}")
        if len(documents) != sum(COUNTS.values()):
            raise SystemExit(f"{len(documents)} documents, pinned {sum(COUNTS.values())}")

        output = args.output
        if output.exists():
            shutil.rmtree(output)
        (output / "test_parsing").mkdir(parents=True)
        for path in documents:
            (output / "test_parsing" / path.name).write_bytes(path.read_bytes())
        (output / "LICENSE").write_bytes(license_bytes)
        (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")
    print(f"vendored {len(documents)} document(s) from {REPO}@{COMMIT} into {output}")


if __name__ == "__main__":
    main()
