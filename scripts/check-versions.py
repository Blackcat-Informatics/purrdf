# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Enforce cross-registry release coherence for the purrdf suite.

PurRDF ships one logical version across three registries — crates.io (the Rust
workspace), PyPI (``purrdf``), and npm (``@blackcatinformatics/purrdf``) — from
three independent tag namespaces (``rust-v*`` / ``py-v*`` / ``npm-v*``). Nothing
mechanically forces those three version sources to agree, and nothing forces the
crates.io release lane to publish exactly the crates that are publishable. This
lint closes both gaps as a hard, no-optionality gate:

1. **Version coherence.** The workspace version (``Cargo.toml``
   ``[workspace.package].version``), the PyPI project version
   (``bindings/python/pyproject.toml`` ``[project].version``), the npm
   package version (``crates/rdf-wasm/js/package.json`` ``version``), and the
   cited version (``CITATION.cff`` ``version``) must be byte-identical. A single
   tag then names one coherent release. ``CITATION.cff`` is not a build input, so
   it drifted out of the lane between 0.2.1 and 0.5.0; pinning it here keeps the
   cited version honest for every future release.

2. **Publish-list completeness.** The ordered crate list the crates.io release
   lane publishes (the ``crates=( … )`` array in
   ``.github/workflows/release-cargo.yaml`` and ``scripts/bootstrap-crates-io.sh``)
   must equal the set of publishable workspace crates (every member whose
   ``publish`` is not ``false``). A publishable crate missing from the lane is a
   latent release break — e.g. a listed crate depending on an unlisted one makes
   ``cargo publish`` fail on a missing dependency.

3. **Per-crate version coherence.** Every publishable workspace crate must
   resolve (via ``cargo metadata``) to exactly the canonical workspace version.
   A crate that hardcodes ``version = "0.1.0"`` instead of
   ``version.workspace = true`` would otherwise sail through the top-level
   three-file byte check while publishing at the wrong version; this assertion
   catches that drift and names every offending crate.

4. **Internal dependency-requirement pins.** Every intra-workspace dependency
   requirement (each ``{ path, version }`` pin, resolved via ``cargo metadata``)
   must be pinned exactly to the canonical version (semver floor-equality). A
   partial bump that leaves ``purrdf 0.3.0`` requiring ``purrdf-core ^0.2.1``
   passes the three-file byte check AND per-crate coherence AND a local build
   (path deps win locally), then publishes a crate wired to the OLD registry
   crate — an irreversible break this gate catches before the tag is cut.

The gate is deterministic and offline: it reads in-tree files plus
``cargo metadata`` and never touches the network.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def workspace_version(root: Path) -> str:
    data = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return data["workspace"]["package"]["version"]


def pyproject_version(root: Path) -> str:
    data = tomllib.loads(
        (root / "bindings" / "python" / "pyproject.toml").read_text(encoding="utf-8")
    )
    return data["project"]["version"]


def npm_version(root: Path) -> str:
    data = json.loads(
        (root / "crates" / "rdf-wasm" / "js" / "package.json").read_text(
            encoding="utf-8"
        )
    )
    return data["version"]


def citation_version(root: Path) -> str:
    """The ``version`` value from ``CITATION.cff`` (never ``cff-version``).

    CFF is YAML; rather than take a YAML dependency, match the column-0
    ``version:`` key (the schema key ``cff-version:`` is prefixed, so ``^version:``
    cannot match it) and strip optional quotes. Kept deliberately narrow so a
    nested/indented ``version`` elsewhere in the document can never be picked up.
    """
    text = (root / "CITATION.cff").read_text(encoding="utf-8")
    match = re.search(r'^version:\s*"?([^"\n]+?)"?\s*$', text, flags=re.MULTILINE)
    if match is None:
        raise ValueError("CITATION.cff: no top-level version: field")
    return match.group(1)


def workspace_metadata(root: Path) -> dict:
    """The ``cargo metadata --no-deps`` document for the workspace.

    ``--no-deps`` omits the resolved transitive graph but STILL carries each
    member's declared ``dependencies`` (with their ``req``/``path``/``rename``),
    which the internal-pin gate needs. Deterministic and offline.
    """
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version=1"],
        check=True,
        capture_output=True,
        text=True,
        cwd=root,
    ).stdout
    return json.loads(out)


def publishable_crates(meta: dict) -> dict[str, str]:
    """Map every publishable workspace member to its resolved version.

    In ``cargo metadata`` a member with no publish restriction has ``publish:
    null``; a ``publish = false`` member has ``publish: []``. Treating a
    non-``null`` empty list as unpublishable matches the crates.io semantics.
    The returned mapping preserves each package's resolved ``version`` so the
    caller can assert per-crate version coherence against the canonical
    workspace version.
    """
    publishable: dict[str, str] = {}
    for pkg in meta["packages"]:
        if pkg.get("publish") != []:
            publishable[pkg["name"]] = pkg["version"]
    return publishable


def internal_pin_violations(meta: dict, version: str) -> list[str]:
    """Every intra-workspace dependency whose version requirement is not pinned
    exactly to ``version`` (semver **floor-equality**).

    The trap: ``set-version.py`` (or a hand edit) bumps a crate's own version but
    leaves an internal dependency requirement at the previous release
    (``purrdf 0.3.0`` still requiring ``purrdf-core ^0.2.1``). ``make check`` and
    local builds pass (path deps win locally), then the PUBLISHED crate wires to
    the old registry crate — irreversible. This gate compares every intra-
    workspace path dependency's ``req`` floor against the canonical version: a
    caret/exact pin at ``version`` passes; a stale pin OR a loose ``>=``/``*``
    that merely *satisfies* the version FAILS (low-optionality: pins are exact).

    Scope: only path dependencies (intra-workspace) in the PUBLISHED graph
    (normal + build deps; dev-dependencies are stripped from published crates and
    legitimately carry no version). Renamed deps are handled via ``rename``.
    """
    members = {pkg["name"] for pkg in meta["packages"]}
    violations: list[str] = []
    for pkg in sorted(meta["packages"], key=lambda p: p["name"]):
        for dep in pkg["dependencies"]:
            if dep.get("path") is None or dep["name"] not in members:
                continue
            if dep.get("kind") not in (None, "build"):  # skip dev-dependencies
                continue
            floor = dep["req"].lstrip("^=~> ")
            if floor != version:
                alias = dep.get("rename") or dep["name"]
                violations.append(
                    f"    {pkg['name']} -> {alias} (package {dep['name']}): "
                    f"req {dep['req']!r} (expected pin {version})"
                )
    return violations


_CRATES_ARRAY_RE = re.compile(r"crates=\(\s*(.*?)\s*\)", re.DOTALL)
_CRATE_TOKEN_RE = re.compile(r"^purrdf(?:-[a-z]+)*$")


def parse_publish_list(path: Path) -> list[str]:
    """Extract the ordered ``crates=( … )`` array from a shell/YAML file.

    The anchor is the literal ``crates=(`` assignment, so the ``-p purrdf-…``
    wasm cross-check flags elsewhere in the same file are never picked up.
    Shell ``#`` line-comments inside the array are stripped before tokenizing,
    so a commented-out crate (``# purrdf-validate`` or ``purrdf-foo  # note``)
    is not miscounted as listed.
    """
    text = path.read_text(encoding="utf-8")
    match = _CRATES_ARRAY_RE.search(text)
    if match is None:
        raise ValueError(f"{path}: no crates=( … ) array found")
    crates: list[str] = []
    for line in match.group(1).splitlines():
        code = line.split("#", 1)[0]
        for token in code.split():
            if _CRATE_TOKEN_RE.match(token):
                crates.append(token)
    return crates


def main() -> int:
    root = repo_root()
    failures: list[str] = []

    # 1. Version coherence across the three registries.
    versions = {
        "Cargo.toml [workspace.package].version": workspace_version(root),
        "bindings/python/pyproject.toml [project].version": pyproject_version(root),
        "crates/rdf-wasm/js/package.json .version": npm_version(root),
        "CITATION.cff version": citation_version(root),
    }
    distinct = set(versions.values())
    if len(distinct) != 1:
        failures.append("version sources disagree:")
        for source, value in versions.items():
            failures.append(f"    {value:<12} {source}")
    version = next(iter(versions.values()))

    # 2. Publish-list completeness against the publishable set.
    meta = workspace_metadata(root)
    publishable_versions = publishable_crates(meta)
    publishable = set(publishable_versions)
    lists = {
        ".github/workflows/release-cargo.yaml": root
        / ".github"
        / "workflows"
        / "release-cargo.yaml",
        "scripts/bootstrap-crates-io.sh": root / "scripts" / "bootstrap-crates-io.sh",
    }
    for label, path in lists.items():
        published = parse_publish_list(path)
        published_set = set(published)
        if len(published) != len(published_set):
            dupes = sorted(c for c in published_set if published.count(c) > 1)
            failures.append(f"{label}: duplicate crate(s) in publish list: {dupes}")
        missing = publishable - published_set
        extra = published_set - publishable
        if missing:
            failures.append(
                f"{label}: publishable crate(s) missing from the release lane: "
                f"{sorted(missing)}"
            )
        if extra:
            failures.append(
                f"{label}: release lane lists non-publishable crate(s): "
                f"{sorted(extra)}"
            )

    # 3. Per-crate version coherence against the canonical workspace version.
    mismatched = sorted(
        (name, crate_version)
        for name, crate_version in publishable_versions.items()
        if crate_version != version
    )
    if mismatched:
        failures.append(
            f"publishable crate(s) not at the canonical workspace version {version!r}:"
        )
        for name, crate_version in mismatched:
            failures.append(f"    {name}: {crate_version} (expected {version})")

    # 4. Internal dependency-requirement pins (the partial-bump publish trap).
    pin_violations = internal_pin_violations(meta, version)
    if pin_violations:
        failures.append(
            f"internal dependency requirement(s) not pinned to {version!r} "
            "(a partial bump would publish a stale dependency graph):"
        )
        failures.extend(pin_violations)

    if failures:
        print("FAIL: release coherence check found problems:", file=sys.stderr)
        for line in failures:
            print(f"  {line}", file=sys.stderr)
        return 1

    print(
        f"OK: version {version} coherent across crates.io/PyPI/npm; "
        f"release lane publishes all {len(publishable)} publishable crates."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
