# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Set the PurRDF release version across all three registries in lockstep.

The suite ships one version to crates.io, PyPI, and npm from three separate
files. This script rewrites all three in one shot and then runs the
version-coherence gate (``scripts/check-versions.py``) to prove they agree:

* ``Cargo.toml``                         — ``[workspace.package] version``
* ``bindings/python/pyproject.toml``     — ``[project] version``
* ``crates/rdf-wasm/js/package.json``    — top-level ``version``

Edits are line-scoped so file formatting and comments are preserved (only the
version line changes). Usage::

    python3 scripts/set-version.py 0.2.2
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

_SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def set_toml_version(path: Path, section: str, version: str) -> None:
    """Replace the ``version = "…"`` line inside ``[section]`` of a TOML file."""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    in_section = False
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_section = stripped == section
            continue
        if in_section and re.match(r'\s*version\s*=\s*"', line):
            lines[i] = re.sub(
                r'(version\s*=\s*")[^"]*(")', rf"\g<1>{version}\g<2>", line, count=1
            )
            path.write_text("".join(lines), encoding="utf-8")
            return
    raise SystemExit(f"FAIL: no version key found in {section} of {path}")


def set_json_version(path: Path, version: str) -> None:
    """Replace the first top-level ``"version": "…"`` in a package.json."""
    text = path.read_text(encoding="utf-8")
    # Anchor to a top-level key: exactly two spaces of indent at the start of a
    # line. In standard 2-space-indented package.json the file's own top-level
    # "version" sits at this depth, while any nested (dependency/metadata)
    # "version" is indented deeper, so it can never be matched first.
    new_text, n = re.subn(
        r'^(  "version"\s*:\s*")[^"]*(")',
        rf"\g<1>{version}\g<2>",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    if n == 0:
        raise SystemExit(f'FAIL: no "version" key found in {path}')
    path.write_text(new_text, encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: python3 scripts/set-version.py <x.y.z>", file=sys.stderr)
        return 2
    version = argv[1]
    if not _SEMVER_RE.match(version):
        print(f"FAIL: {version!r} is not a semver x.y.z version", file=sys.stderr)
        return 2

    root = repo_root()
    set_toml_version(root / "Cargo.toml", "[workspace.package]", version)
    set_toml_version(
        root / "bindings" / "python" / "pyproject.toml", "[project]", version
    )
    set_json_version(root / "crates" / "rdf-wasm" / "js" / "package.json", version)

    print(f"set version {version} across crates.io/PyPI/npm; verifying coherence…")
    # Prove the three sources now agree (and the publish list stays complete).
    return subprocess.run(
        [sys.executable, str(root / "scripts" / "check-versions.py")], cwd=root
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
