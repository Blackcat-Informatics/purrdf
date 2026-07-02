#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""License-hygiene gate for vendored corpora.

Every vendored test-suite tree in this repo follows the REUSE convention: it
holds a ``LICENSES/`` directory of license texts, and every file it ships is
licensed either by a per-file ``<file>.license`` sidecar (verbatim third-party
material), an inline ``SPDX-License-Identifier`` header (first-party source), or
a ``REUSE.toml`` annotation (first-party files that cannot carry a header, e.g.
harness-authored ``manifest.ttl`` selectors and reconstructed ``.srx`` results).

This gate fails the build if any file under a vendored root is *undeclared* — so
a future re-sync that drops a ``.license`` sidecar cannot slip through silently
(the "no silent skips" doctrine, applied to provenance/licensing). It scales
automatically: any new directory that adds a ``LICENSES/`` subdir becomes a
vendored root and is enforced with zero changes here.
"""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

# Where vendored corpora live. A "vendored root" is any directory below these
# that contains a LICENSES/ subdirectory (the REUSE marker).
SCAN_AREAS = ("crates", "bindings")

# Files that never need their own license declaration.
EXEMPT_NAMES = {"REUSE.toml"}
# Extensions/paths that are documentation or license text, not vendored payload.
EXEMPT_SUFFIXES = (".license",)


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def find_vendored_roots(root: Path) -> list[Path]:
    roots: list[Path] = []
    for area in SCAN_AREAS:
        base = root / area
        if not base.is_dir():
            continue
        for licenses_dir in base.rglob("LICENSES"):
            if licenses_dir.is_dir():
                roots.append(licenses_dir.parent)
    return sorted(set(roots))


def reuse_covered_paths(vendored_root: Path) -> set[Path]:
    """Resolve every file covered by a REUSE.toml annotation in ``vendored_root``."""
    reuse_toml = vendored_root / "REUSE.toml"
    if not reuse_toml.is_file():
        return set()
    data = tomllib.loads(reuse_toml.read_text(encoding="utf-8"))
    covered: set[Path] = set()
    for annotation in data.get("annotations", []):
        patterns = annotation.get("path", [])
        if isinstance(patterns, str):
            patterns = [patterns]
        for pattern in patterns:
            for match in vendored_root.glob(pattern):
                if match.is_file():
                    covered.add(match.resolve())
    return covered


def has_inline_spdx(path: Path) -> bool:
    try:
        with path.open("r", encoding="utf-8", errors="ignore") as handle:
            for _ in range(8):
                line = handle.readline()
                if not line:
                    break
                if "SPDX-License-Identifier" in line:
                    return True
    except OSError:
        return False
    return False


def is_declared(path: Path, reuse_covered: set[Path]) -> bool:
    if path.with_name(path.name + ".license").is_file():
        return True
    if path.resolve() in reuse_covered:
        return True
    return has_inline_spdx(path)


def check_root(vendored_root: Path) -> list[Path]:
    reuse_covered = reuse_covered_paths(vendored_root)
    offenders: list[Path] = []
    for path in sorted(vendored_root.rglob("*")):
        if not path.is_file():
            continue
        # Skip license texts and the sidecars/config themselves.
        if "LICENSES" in path.relative_to(vendored_root).parts:
            continue
        if path.name in EXEMPT_NAMES or path.suffix in EXEMPT_SUFFIXES:
            continue
        if not is_declared(path, reuse_covered):
            offenders.append(path)
    return offenders


def main() -> int:
    root = repo_root()
    roots = find_vendored_roots(root)
    if not roots:
        print("check-licenses: no vendored roots found (LICENSES/ marker).", file=sys.stderr)
        return 0

    all_offenders: list[Path] = []
    for vendored_root in roots:
        offenders = check_root(vendored_root)
        all_offenders.extend(offenders)

    if all_offenders:
        print(
            "License hygiene FAILED: vendored files without a `.license` sidecar,\n"
            "inline SPDX header, or REUSE.toml annotation:",
            file=sys.stderr,
        )
        for path in all_offenders:
            print(f"  {path.relative_to(root)}", file=sys.stderr)
        return 1

    total = sum(1 for _ in roots)
    print(f"OK: {total} vendored root(s) license-clean.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
