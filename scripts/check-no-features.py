#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if any first-party workspace package declares Cargo features.

The sole sanctioned exception is the ``capi`` feature on ``purrdf-capi``:
``cargo capi`` (cargo-c >= 0.10) unconditionally enables a feature named
``capi`` and hard-errors if the crate does not declare it, so the C-ABI header
regeneration/drift gate (``make capi-header``/``capi-check`` and the CI ``capi``
job) cannot run without that marker. It gates no code — the whole C-ABI surface
is always compiled. The gate requires exactly ``purrdf-capi:capi = []``, rejects
every other feature declaration, and rejects feature predicates in Rust
``cfg``/``cfg_attr`` invocations.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Exact per-package feature maps. The value matters as much as the name: cargo-c
# needs the marker to exist, but anything in its expansion would make it
# semantic. Keep this as tight as the cargo-c requirement demands and no tighter.
EXPECTED_FEATURES = {
    "purrdf-capi": {"capi": []},
}

CFG_INVOCATION = re.compile(r"(?:#\s*!?\s*\[\s*cfg(?:_attr)?|\bcfg\s*!)\s*\(")
FEATURE_PREDICATE = re.compile(r"\bfeature\s*=")
IGNORED_SOURCE_DIRS = {".git", ".worktrees", "target"}


def feature_cfg_offsets(source: str):
    """Yield offsets of feature predicates inside Rust cfg invocations."""
    for invocation in CFG_INVOCATION.finditer(source):
        opening = source.find("(", invocation.start(), invocation.end())
        depth = 0
        for offset in range(opening, len(source)):
            char = source[offset]
            if char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    body = source[opening + 1 : offset]
                    if predicate := FEATURE_PREDICATE.search(body):
                        yield opening + 1 + predicate.start()
                    break


def rust_sources():
    for directory, subdirectories, filenames in os.walk(REPO_ROOT):
        subdirectories[:] = sorted(
            name for name in subdirectories if name not in IGNORED_SOURCE_DIRS
        )
        for filename in sorted(filenames):
            if filename.endswith(".rs"):
                yield Path(directory, filename)


def main() -> int:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=REPO_ROOT,
            text=True,
        )
    )
    package_by_id = {package["id"]: package for package in metadata["packages"]}
    packages = [package_by_id[package_id] for package_id in metadata["workspace_members"]]

    offenders = []
    seen_expected_packages = set()
    for package in packages:
        name = package["name"]
        declared = package.get("features", {})
        expected = EXPECTED_FEATURES.get(name, {})
        if name in EXPECTED_FEATURES:
            seen_expected_packages.add(name)
        if declared != expected:
            offenders.append(
                f"{name}: declared {json.dumps(declared, sort_keys=True)}; "
                f"expected {json.dumps(expected, sort_keys=True)}"
            )

    for missing in sorted(EXPECTED_FEATURES.keys() - seen_expected_packages):
        offenders.append(f"{missing}: package missing from workspace")

    cfg_offenders = []
    for path in rust_sources():
        source = path.read_text(encoding="utf-8")
        for offset in feature_cfg_offsets(source):
            line = source.count("\n", 0, offset) + 1
            cfg_offenders.append(f"{path.relative_to(REPO_ROOT)}:{line}")

    if offenders or cfg_offenders:
        print(
            "First-party workspace crates must declare no Cargo features "
            "except the exact empty purrdf-capi:capi cargo-c marker.",
            file=sys.stderr,
        )
        if offenders:
            print("\n".join(offenders), file=sys.stderr)
        if cfg_offenders:
            print(
                "Rust cfg(feature = ...) use is forbidden:\n"
                + "\n".join(cfg_offenders),
                file=sys.stderr,
            )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
