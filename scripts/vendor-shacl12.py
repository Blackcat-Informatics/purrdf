#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vendor the W3C SHACL 1.2 vocabularies and the `shacl12-test-suite`.

Fetches the pinned upstream commit's `shacl12-vocabularies/` and
`shacl12-test-suite/tests/` trees from `w3c/data-shapes` (W3C Software and
Document License) via the GitHub REST API tarball endpoint and writes them
verbatim into `vectors/shacl12/`, alongside a first-party `PROVENANCE.md`
(this script's own doc — separate from the upstream `tests/README.md` and
`tests/node-expr/README.md`/`tests/sparql-rl/README.md` it vendors) and a
verbatim copy of the upstream `LICENSE.md`.

A second, crate-local copy of the four vocabulary files is written to
`crates/shapes/spec/`: the declared-vs-implemented ratchet and the `purrdf
shapes lint` surface need the vocabularies at runtime, and an
`include_bytes!` reaching out of `crates/shapes/` into `vectors/` would break
`cargo package` (a published crate does not carry the workspace's `vectors/`
tree). The crate-local copy is the same bytes, not a re-derivation.

This script is deterministic and re-runnable: the upstream commit is pinned,
both output directories are fully replaced (not merged) on every run, files
are written by relative path with no embedded timestamps, and the trees are
byte-identical run over run. Re-vendoring against a newer upstream commit is
a deliberate edit to `COMMIT` below, followed by re-running this script and
`python3 scripts/check-corpus-frozen.py --update`.

    python3 scripts/vendor-shacl12.py
"""

from __future__ import annotations

import argparse
import io
import shutil
import tarfile
import urllib.request
from pathlib import Path

REPO = "w3c/data-shapes"
COMMIT = "5069ed3519c31cfcf074d2cf779542abc37ce80d"
LICENSE_NAME = "W3C Software and Document License"
LICENSE_URL = "http://www.w3.org/Consortium/Legal/copyright-software"

TARBALL_URL = f"https://api.github.com/repos/{REPO}/tarball/{COMMIT}"

# The vocabularies the declared-vs-implemented ratchet and the census parse. `shacl-ui.ttl` and `profiles/cd1.ttl` also live under
# `shacl12-vocabularies/` upstream but are out of scope: SHACL 1.2 Core, its
# node-expression extension (`shnex`), the `shnex` SPARQL function bindings and
# the `shacl-shacl.ttl` self-description are the whole surface this repository
# implements against.
VOCAB_FILES = ("shacl.ttl", "shnex.ttl", "shnex-sparql.ttl", "shacl-shacl.ttl")
VOCAB_PREFIX = "shacl12-vocabularies/"
TESTS_PREFIX = "shacl12-test-suite/tests/"

PROVENANCE = f"""<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C SHACL 1.2 vocabularies and `shacl12-test-suite`

Frozen copy of the upstream `{REPO}` SHACL 1.2 vocabularies and the whole
`shacl12-test-suite/tests/` tree, vendored for the `purrdf-shapes`
declared-vs-implemented ratchet and its SHACL 1.2 conformance harness.
**Do not hand-edit** — treat exactly like the GTS vectors and the
`vectors/shexTest` corpus: byte-frozen third-party conformance data,
regenerated only by re-running `python3 scripts/vendor-shacl12.py`. The
freeze is enforced: `make check` runs `scripts/check-corpus-frozen.py`, which
SHA-256-verifies every file here against
`scripts/conformance-frozen/vectors-shacl12.sha256`, so a silent content edit
fails the build. A deliberate re-vendor regenerates that manifest with
`python3 scripts/check-corpus-frozen.py --update`.

## Source

- Upstream: <https://github.com/{REPO}> — the W3C Data Shapes Working Group's
  repository for SHACL 1.2 (Core, the node-expression extension `shnex`, the
  SPARQL bindings, and the conformance test suite).
- Pinned commit: `{COMMIT}` — pinned for reproducible builds and to track
  upstream errata explicitly, the same hygiene every vendored suite in this
  repo follows.
- License: **{LICENSE_NAME}** (<{LICENSE_URL}>), per the upstream
  repository's own `LICENSE.md`; that file is vendored verbatim alongside
  this tree as `LICENSE.md`.
- Retrieval: `scripts/vendor-shacl12.py` fetches the pinned commit's full
  source tree as a tarball from the GitHub REST API
  (`GET https://api.github.com/repos/{REPO}/tarball/{COMMIT}`, which redirects
  to a `codeload.github.com` archive of that exact commit) and extracts the
  `shacl12-vocabularies/` and `shacl12-test-suite/tests/` subtrees verbatim —
  no per-file API calls, no upstream `git` clone.
- Vendored subset:
  - `vocabularies/`: `shacl.ttl`, `shnex.ttl`, `shnex-sparql.ttl` and
    `shacl-shacl.ttl` from upstream `shacl12-vocabularies/`.
  - `tests/`: the entire upstream `shacl12-test-suite/tests/` tree —
    `core/`, `node-expr/`, `sparql/`, `inference-rules/` and `sparql-rl/`,
    every manifest, every `.srl` file, and the upstream `tests/README.md`,
    `tests/node-expr/README.md` and `tests/sparql-rl/README.md` (kept
    verbatim — distinct from this file).

## Spec facts: the `sparql:plus` / `sparql:encode` spellings

`shnex-sparql.ttl` declares the SPARQL-function node-expression bindings
`sparql:plus` and `sparql:encode`. The SPARQL Working Group's own
`sparql-ns.ttl` names the same two functions `sparql:add` and
`sparql:encodeForUri`. Both spellings name the same built-in function; an
engine implementing SHACL 1.2 node expressions accepts both.

## Runtime copy

The same four files under `vocabularies/` are also written byte-exact to
`crates/shapes/spec/` by this script, for the reason given in the crate-local
`crates/shapes/spec/README.md`. That copy is covered by its own entry in
`scripts/conformance-frozen/`.

Harness: `crates/shapes/tests/w3c12_conformance.rs` runs every case under
`tests/` through the library API and reports the SHACL 1.2 rows of
`conformance-matrix.py`.
"""


def fetch_tarball() -> bytes:
    """Download the pinned commit's tarball from the GitHub REST API."""
    request = urllib.request.Request(  # noqa: S310 - pinned https GitHub API host
        TARBALL_URL, headers={"User-Agent": "purrdf-vendor-shacl12"}
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
            # "w3c-data-shapes-<shortsha>/shacl12-vocabularies/...".
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
    repo_root = Path(__file__).resolve().parent.parent
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "vectors" / "shacl12",
        help="destination directory (default: vectors/shacl12 at the repo root)",
    )
    parser.add_argument(
        "--spec-output",
        type=Path,
        default=repo_root / "crates" / "shapes" / "spec",
        help="crate-local runtime copy destination (default: crates/shapes/spec)",
    )
    args = parser.parse_args()

    archive = fetch_tarball()

    vocab_all = extract_prefix(archive, VOCAB_PREFIX)
    missing_vocab = [name for name in VOCAB_FILES if name not in vocab_all]
    if missing_vocab:
        raise SystemExit(
            f"upstream {REPO}@{COMMIT} tarball is missing vocabulary file(s): "
            f"{', '.join(missing_vocab)}"
        )
    vocab = {name: vocab_all[name] for name in VOCAB_FILES}

    tests = extract_prefix(archive, TESTS_PREFIX)
    if not tests:
        raise SystemExit(
            f"no files found under {TESTS_PREFIX} in the {REPO}@{COMMIT} tarball"
        )

    root_files = extract_prefix(archive, "")
    license_text = root_files.get("LICENSE.md")
    if license_text is None:
        raise SystemExit(f"upstream {REPO}@{COMMIT} tarball has no root LICENSE.md file")

    output = args.output
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    write_tree(output / "vocabularies", vocab)
    write_tree(output / "tests", tests)
    (output / "LICENSE.md").write_bytes(license_text)
    (output / "PROVENANCE.md").write_text(PROVENANCE, encoding="utf-8", newline="\n")

    spec_output = args.spec_output
    if spec_output.exists():
        shutil.rmtree(spec_output)
    spec_output.mkdir(parents=True)
    write_tree(spec_output, vocab)
    (spec_output / "README.md").write_text(SPEC_README, encoding="utf-8", newline="\n")

    print(
        f"vendored {len(vocab)} vocabulary file(s) and {len(tests)} test file(s) "
        f"from {REPO}@{COMMIT} into {output}"
    )
    print(f"wrote {len(vocab)} runtime vocabulary file(s) into {spec_output}")


SPEC_README = """<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Frozen W3C SHACL 1.2 vocabularies (runtime copy)

This directory is a byte-exact, frozen copy of the four W3C SHACL 1.2
vocabulary files also vendored at `vectors/shacl12/vocabularies/`. It is
written by `scripts/vendor-shacl12.py` and **must not be hand-edited** —
a silent content edit fails `make check` via
`scripts/check-corpus-frozen.py`.

## Why a second copy

`purrdf-shapes` needs these vocabularies at runtime: the declared-vs-implemented
ratchet (`purrdf_shapes::spec::declared()`) parses them to prove every SHACL 1.2
function and component declaration is bound, and the `purrdf shapes lint` cold
certify surface reads them for the `shacl-shacl.ttl` oracle. `vectors/` is a
workspace-level tree that is not part of the published `purrdf-shapes` crate,
so an `include_bytes!` reaching out of `crates/shapes/` into `vectors/` would
build locally but break `cargo package`. This directory is the same bytes,
copied so they ship inside the package.

## Source and license

Source, pinned upstream commit, and license: see
`vectors/shacl12/PROVENANCE.md`. These are the same W3C Software and Document
License files described there, at `vectors/shacl12/vocabularies/`.
"""


if __name__ == "__main__":
    main()
