# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Set the PurRDF release version across EVERY version location in lockstep.

The suite ships one version to crates.io, PyPI, and npm. A release must rewrite
far more than the three top-level version strings: every internal path
dependency carries its own ``version = "…"`` requirement, and a partial bump
publishes a crate whose dependency requirements still point at the previous
release (``purrdf 0.3.0`` depending on ``purrdf-core ^0.2.1``), which resolves
against the OLD registry crate — an irreversible, silent break. This script
rewrites all of them in one shot and then runs the version-coherence gate
(``scripts/check-versions.py``) to prove they agree:

Top-level version sources:
* ``Cargo.toml``                         — ``[workspace.package] version``
* ``bindings/python/pyproject.toml``     — ``[project] version``
* ``crates/rdf-wasm/js/package.json``    — top-level ``version``

Internal dependency-requirement pins (every intra-workspace path dependency —
a dependency line carrying BOTH ``path = "…"`` and ``version = "…"``):
* ``Cargo.toml``                         — the ``[workspace.dependencies]`` pins
* ``crates/*/Cargo.toml`` /
  ``bindings/*/Cargo.toml``              — renamed-dep pins (e.g. shapes/slice's
                                           ``purrdf = { package = "purrdf-rdf" }``
                                           and rdf-capi's ``purrdf-rs``)

Other version locations:
* ``crates/rdf-capi/Cargo.toml``         — ``[package.metadata.capi.library] version``
* ``bindings/python/uv.lock``            — the editable ``purrdf`` package pin

Edits are line-scoped so file formatting and comments are preserved (only the
version value changes). Deps that inherit via ``version.workspace = true`` and
external deps (no ``path``) are never touched, and unrelated ``0.2.x`` strings
(e.g. ``wasm-bindgen = "=0.2.125"``) carry no ``path`` so they are inert here.
Usage::

    python3 scripts/set-version.py 0.3.0
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

_SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")
_VERSION_VALUE_RE = re.compile(r'(version\s*=\s*")[^"]*(")')


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
            lines[i] = _VERSION_VALUE_RE.sub(rf"\g<1>{version}\g<2>", line, count=1)
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


def set_path_dep_versions(path: Path, version: str) -> int:
    """Rewrite the ``version = "…"`` pin on every intra-workspace path dependency.

    An intra-workspace pin is any line carrying BOTH ``path = "…"`` and a quoted
    ``version = "…"`` (the internal ``[workspace.dependencies]`` entries and the
    renamed-dep pins in member manifests). External deps have no ``path`` and
    ``version.workspace = true`` inheritors have no quoted version, so neither is
    touched. Returns the number of lines changed.
    """
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    changed = 0
    for i, line in enumerate(lines):
        if "path =" in line and _VERSION_VALUE_RE.search(line):
            new = _VERSION_VALUE_RE.sub(rf"\g<1>{version}\g<2>", line, count=1)
            if new != line:
                lines[i] = new
                changed += 1
    if changed:
        path.write_text("".join(lines), encoding="utf-8")
    return changed


def member_manifests(root: Path) -> list[Path]:
    """Every workspace member ``Cargo.toml`` (crates/* and bindings/*)."""
    manifests = sorted((root / "crates").glob("*/Cargo.toml"))
    manifests += sorted((root / "bindings").glob("*/Cargo.toml"))
    return manifests


def set_uv_lock(root: Path, version: str) -> None:
    """Sync ``bindings/python/uv.lock`` to the new version.

    Prefer a scoped ``uv lock --upgrade-package purrdf`` (touches only the
    editable ``purrdf`` pin, no unrelated transitive churn). ``uv.lock`` is
    dev-facing reproducibility only — ``release-pypi.yaml`` builds with
    ``uv run --no-project`` and never consumes it — so a targeted rewrite of the
    editable ``purrdf`` package's ``version`` is a safe fallback when ``uv`` is
    unavailable.
    """
    py_dir = root / "bindings" / "python"
    lock = py_dir / "uv.lock"
    try:
        subprocess.run(
            ["uv", "lock", "--upgrade-package", "purrdf"],
            cwd=py_dir,
            check=True,
            capture_output=True,
            text=True,
        )
        return
    except (FileNotFoundError, subprocess.CalledProcessError) as exc:
        detail = exc.stderr if isinstance(exc, subprocess.CalledProcessError) else exc
        print(f"WARN: `uv lock` unavailable ({detail}); rewriting uv.lock directly")

    # Fallback: rewrite the version of the [[package]] block whose name = "purrdf".
    text = lock.read_text(encoding="utf-8")
    blocks = text.split("[[package]]")
    for idx, block in enumerate(blocks):
        if re.search(r'^\s*name\s*=\s*"purrdf"\s*$', block, flags=re.MULTILINE):
            blocks[idx] = re.sub(
                r'(^\s*version\s*=\s*")[^"]*(")',
                rf"\g<1>{version}\g<2>",
                block,
                count=1,
                flags=re.MULTILINE,
            )
            break
    lock.write_text("[[package]]".join(blocks), encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: python3 scripts/set-version.py <x.y.z>", file=sys.stderr)
        return 2
    version = argv[1]
    if not _SEMVER_RE.match(version):
        print(f"FAIL: {version!r} is not a semver x.y.z version", file=sys.stderr)
        return 2

    root = repo_root()

    # 1. The three top-level version sources.
    set_toml_version(root / "Cargo.toml", "[workspace.package]", version)
    set_toml_version(
        root / "bindings" / "python" / "pyproject.toml", "[project]", version
    )
    set_json_version(root / "crates" / "rdf-wasm" / "js" / "package.json", version)

    # 2. Every intra-workspace path-dependency version pin: the
    #    [workspace.dependencies] internal pins and the member renamed-dep pins.
    pins = set_path_dep_versions(root / "Cargo.toml", version)
    for manifest in member_manifests(root):
        pins += set_path_dep_versions(manifest, version)

    # 3. The capi C-ABI library version and the editable purrdf uv.lock pin.
    set_toml_version(
        root / "crates" / "rdf-capi" / "Cargo.toml",
        "[package.metadata.capi.library]",
        version,
    )
    set_uv_lock(root, version)

    print(
        f"set version {version} across crates.io/PyPI/npm "
        f"(+{pins} internal path-dep pins); verifying coherence…"
    )
    # Prove the sources now agree (and the publish list stays complete).
    return subprocess.run(
        [sys.executable, str(root / "scripts" / "check-versions.py")], cwd=root
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
