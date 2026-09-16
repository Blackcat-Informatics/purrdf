#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if a replaced third-party dependency re-enters the workspace closure.

Two independent rules, matched to how each dependency was actually replaced:

* **Any-edge ban** (``BANNED_ANY_EDGE``): the ox-family, ``oxilangtag``, and
  ``petgraph`` have a first-party replacement good for every edge kind, so
  reappearing ANYWHERE in the resolved dependency graph — runtime, build,
  dev/test, or transitive — is a failure. This is read from ``Cargo.lock``
  (never ``Cargo.toml``), which records the full resolved closure, so a
  reintroduction through a dev-dependency or a transitive edge is caught the
  same way as a direct one.

* **Direct-edge ban** (``BANNED_DIRECT_ONLY``): ``hex`` was removed only from
  PurRDF's own first-party surface (``core::fmt::LowerHex`` replaced it). A
  third-party crate that itself depends on ``hex`` transitively is not this
  gate's business to police — banning that would fail ``make check`` the day
  any upstream dependency happens to use ``hex`` internally, with no fix
  available in this repository. That would be over-refusal: rejecting an edge
  that was never PurRDF's own. So this rule does NOT scan ``Cargo.lock`` at
  all; it reads workspace member manifests directly (``[dependencies]``,
  ``[dev-dependencies]``, ``[build-dependencies]``, and their
  ``[target.'cfg(...)'.*]`` equivalents) plus the root
  ``[workspace.dependencies]`` table, and only fails if one of them names the
  package directly.

**Substitution.** A ``[patch]``/``[replace]`` override that redirects a
banned name to a different source is scanned for too: ``[[patch.unused]]``
blocks and ``replace = "name:version"`` lines in ``Cargo.lock`` are matched
against ``BANNED_ANY_EDGE`` the same as a normally resolved package.

What this CANNOT see: a fork published under a DIFFERENT package name (for
example a ``[patch]`` table remapping ``package = "my-oxigraph-fork"``).
Cargo.lock would then record that fork under its own name, indistinguishable
from any other unrelated crate to a name-based scan. Catching that requires a
source-URL allowlist or a checksum/provenance check, neither of which this
gate performs; a rename or an under-a-different-name fork is a real gap, not
one this script claims to close.

Each ban list entry names the replacement so the failure message teaches the
fix. A dependency joins a list in the change that removes it, so this gate
never lies about the present.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Package name -> first-party replacement. Banned on ANY edge (runtime, build,
# dev/test, transitive) because a first-party replacement exists for every
# edge kind; reintroduction anywhere in the resolved Cargo.lock closure fails.
BANNED_ANY_EDGE: dict[str, str] = {
    "oxilangtag": "purrdf_iri::langtag (RFC 5646 well-formedness)",
    "oxiri": "purrdf-iri",
    "oxsdatatypes": "purrdf-xsd",
    "oxrdf": "purrdf-core",
    "oxigraph": "the native purrdf engine",
    "petgraph": "the first-party iterative Tarjan SCC (purrdf-slice / purrdf-shex)",
}

# Package name -> first-party replacement. Banned only as a DIRECT dependency
# of a workspace member or of the shared `[workspace.dependencies]` table; a
# transitive third-party use is out of scope (see module docstring).
BANNED_DIRECT_ONLY: dict[str, str] = {
    "hex": 'core::fmt::LowerHex formatting (`format!("{digest:x}")`)',
}

DEP_TABLE_KEYS = ("dependencies", "dev-dependencies", "build-dependencies")

PACKAGE_NAME_RE = re.compile(r'^name = "([^"]+)"$', re.MULTILINE)
REPLACE_KEY_RE = re.compile(r'^replace = "([^:"]+):', re.MULTILINE)
PATCH_UNUSED_BLOCK_RE = re.compile(
    r"^\[\[patch\.unused\]\]\n((?:(?!^\[\[).)*)", re.MULTILINE | re.DOTALL
)


def any_edge_offenders(lock_text: str) -> list[str]:
    """Tier-1 names resolved anywhere in the lock (any ``name = "..."`` line).

    The regex is not scoped to ``[[package]]`` tables, so it already matches a
    banned name sitting inside a ``[[patch.unused]]`` block the same as an
    ordinary resolved package.
    """
    resolved = set(PACKAGE_NAME_RE.findall(lock_text))
    return sorted(resolved & BANNED_ANY_EDGE.keys())


def substitution_offenders(lock_text: str) -> list[str]:
    """Tier-1 names smuggled back via a ``[patch]``/``[replace]`` override
    that keeps the same package name but redirects its source.

    Two constructs are checked explicitly (on top of, and overlapping with,
    ``any_edge_offenders``, so a future narrowing of ``PACKAGE_NAME_RE`` to
    ``[[package]]`` tables only cannot silently drop this coverage):

    * ``[[patch.unused]]`` blocks — a declared-but-inactive patch still names
      the package it would have replaced.
    * ``replace = "name:version"`` keys — the deprecated ``[replace]``
      feature's pointer, which names the package in a value string rather
      than a ``name = "..."`` line, so ``any_edge_offenders`` alone would miss
      it.
    """
    names: set[str] = set()
    names.update(REPLACE_KEY_RE.findall(lock_text))
    for block in PATCH_UNUSED_BLOCK_RE.finditer(lock_text):
        names.update(PACKAGE_NAME_RE.findall(block.group(1)))
    return sorted(names & BANNED_ANY_EDGE.keys())


def direct_dependency_names(manifest: dict) -> set[str]:
    """Every dependency name a single manifest table (or the root
    ``[workspace]`` table) declares directly.

    Covers the top-level ``dependencies``/``dev-dependencies``/
    ``build-dependencies`` tables plus the same three tables nested under any
    ``[target.'cfg(...)'.*]`` platform gate, so a platform-gated direct
    dependency is not invisible to the check.
    """
    names: set[str] = set()
    for key in DEP_TABLE_KEYS:
        names.update(manifest.get(key, {}).keys())
    for target_table in manifest.get("target", {}).values():
        for key in DEP_TABLE_KEYS:
            names.update(target_table.get(key, {}).keys())
    return names


def direct_edge_offenders(root: Path) -> dict[str, list[str]]:
    """Tier-2 names found as a direct dependency, mapped to the declaring
    manifest path(s).

    Reads the root ``Cargo.toml``'s ``[workspace.dependencies]`` table and
    every workspace member's own manifest — never ``Cargo.lock`` — so a
    transitive third-party use of a ``BANNED_DIRECT_ONLY`` package is
    invisible to this function by construction, not by accident.
    """
    root_manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    offenders: dict[str, list[str]] = {}

    workspace_deps = set(root_manifest.get("workspace", {}).get("dependencies", {}).keys())
    for name in workspace_deps & BANNED_DIRECT_ONLY.keys():
        offenders.setdefault(name, []).append("Cargo.toml [workspace.dependencies]")

    for member in root_manifest["workspace"]["members"]:
        manifest_path = f"{member}/Cargo.toml"
        member_manifest = tomllib.loads(
            (root / manifest_path).read_text(encoding="utf-8")
        )
        declared = direct_dependency_names(member_manifest)
        for name in declared & BANNED_DIRECT_ONLY.keys():
            offenders.setdefault(name, []).append(manifest_path)

    return offenders


def self_test() -> int:
    failures: list[str] = []

    # --- pre-existing real behavior: exact-name tier-1 match in a plain lock.
    dirty = 'name = "serde"\nname = "oxilangtag"\nversion = "0.1.6"\n'
    clean = 'name = "serde"\nname = "purrdf-iri"\n'
    if any_edge_offenders(dirty) != ["oxilangtag"]:
        failures.append("a banned package in the lock was not flagged")
    if any_edge_offenders(clean):
        failures.append("a clean lock was flagged")

    # --- (A) tier-1 any-edge ban still fails on a lock containing petgraph.
    petgraph_lock = 'name = "serde"\nname = "petgraph"\nversion = "0.6.5"\n'
    if any_edge_offenders(petgraph_lock) != ["petgraph"]:
        failures.append("petgraph in the lock was not flagged (tier-1 regression)")

    # --- (A) tier-2 direct-hex rule fails when a workspace member declares
    #     hex directly (top-level, dev-only, build-only, and target-gated).
    direct_variants = {
        "top-level dependencies": {"dependencies": {"hex": {"version": "0.4"}}},
        "dev-dependencies": {"dev-dependencies": {"hex": {"version": "0.4"}}},
        "build-dependencies": {"build-dependencies": {"hex": {"version": "0.4"}}},
        "target-gated dependencies": {
            "target": {
                "cfg(not(target_arch = \"wasm32\"))": {
                    "dependencies": {"hex": {"version": "0.4"}}
                }
            }
        },
    }
    for label, manifest in direct_variants.items():
        if "hex" not in direct_dependency_names(manifest):
            failures.append(f"direct hex dependency ({label}) was not flagged")

    # --- (A) tier-2 direct-hex rule does NOT fail when hex is absent from
    #     every workspace member's own manifest, even if Cargo.lock resolves
    #     hex transitively (pulled in by some unrelated third-party crate).
    #     THIS IS THE OVER-REFUSAL CHECK: tier-2 must never consult
    #     Cargo.lock, so a transitive-only hex can never reach it.
    transitive_only_lock = (
        'name = "serde"\n'
        'name = "some-third-party-crate"\n'
        "dependencies = [\n"
        ' "hex",\n'
        "]\n"
        'name = "hex"\n'
        'version = "0.4.3"\n'
    )
    clean_member_manifest = {"dependencies": {"serde": {"workspace": True}}}
    if direct_dependency_names(clean_member_manifest) & BANNED_DIRECT_ONLY.keys():
        failures.append("a clean member manifest was flagged for hex")
    if any_edge_offenders(transitive_only_lock):
        failures.append(
            "transitive-only hex in Cargo.lock was flagged by the any-edge "
            "scan; hex must only be checked via direct manifest declarations"
        )

    # --- (B) substitution: a [[patch.unused]] block naming a banned package.
    patch_unused_lock = (
        'name = "serde"\n\n'
        "[[patch.unused]]\n"
        'name = "petgraph"\n'
        'version = "0.6.5"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n\n'
        '[[package]]\nname = "purrdf-slice"\nversion = "2.0.2"\n'
    )
    if substitution_offenders(patch_unused_lock) != ["petgraph"]:
        failures.append("a [[patch.unused]] banned package was not flagged")

    # --- (B) substitution: a `replace = "name:version"` key redirecting a
    #     banned package (a construct any_edge_offenders alone cannot see,
    #     since the name sits inside a value string, not a `name = "..."`
    #     line).
    replace_lock = (
        'name = "oxigraph"\n'
        'version = "0.3.0"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        'replace = "oxigraph:0.4.0"\n'
    )
    if substitution_offenders(replace_lock) != ["oxigraph"]:
        failures.append("a replace= override of a banned package was not flagged")

    # --- (B) substitution scan stays quiet on an unrelated patch/replace.
    benign_lock = (
        "[[patch.unused]]\n"
        'name = "serde"\n'
        'version = "1.0.0"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
    )
    if substitution_offenders(benign_lock):
        failures.append("an unrelated [[patch.unused]] entry was flagged")

    if failures:
        for message in failures:
            print(f"SELF-TEST FAIL: {message}")
        return 1
    print("OK: check-banned-deps self-test (the gate can still fail)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()

    lock_text = (REPO_ROOT / "Cargo.lock").read_text(encoding="utf-8")
    failures: list[str] = []

    any_edge = set(any_edge_offenders(lock_text))
    substitution = set(substitution_offenders(lock_text))
    for name in sorted(any_edge | substitution):
        replacement = BANNED_ANY_EDGE[name]
        if name in any_edge:
            failures.append(
                f"FAIL: `{name}` is back in Cargo.lock; it was replaced by "
                f"{replacement} and must not be reintroduced through any "
                "runtime, build, or test edge"
            )
        else:
            failures.append(
                f"FAIL: `{name}` reappears via a Cargo.lock patch/replace "
                f"override with no plain resolved entry; it was replaced by "
                f"{replacement} and must not be reintroduced under any source"
            )

    direct = direct_edge_offenders(REPO_ROOT)
    for name in sorted(direct):
        replacement = BANNED_DIRECT_ONLY[name]
        files = ", ".join(direct[name])
        failures.append(
            f"FAIL: `{name}` is a direct dependency of {files}; it was "
            f"replaced by {replacement} and must not be reintroduced as a "
            "first-party edge (a transitive third-party use is not flagged)"
        )

    if failures:
        for message in failures:
            print(message)
        return 1

    total = len(BANNED_ANY_EDGE) + len(BANNED_DIRECT_ONLY)
    print(f"OK: none of the {total} replaced dependencies re-entered the workspace")
    return 0


if __name__ == "__main__":
    sys.exit(main())
