#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Byte-freeze gate for the vendored conformance corpora.

The vendored suites are declared **byte-frozen** — never hand-edited. This gate
makes that claim true rather than aspirational: it recomputes a SHA-256 over
every file under each guarded root and compares it to a committed manifest, so a
silent content edit (a changed expected result, a tampered shape, a re-synced
byte) fails the build. Exact-count assertions in the harnesses catch add / remove
/ rename drift; this catches *content* drift, which counts alone cannot.

The manifests live under ``scripts/conformance-frozen/`` — deliberately OUTSIDE
the vendored trees, so the vendored corpora stay pure (AGENTS/CLAUDE: never
hand-edit ``vectors/``) and the manifest never has to exclude itself.

    python3 scripts/check-corpus-frozen.py            # verify (exit 1 on drift)
    python3 scripts/check-corpus-frozen.py --update   # regenerate manifests

Regenerating is the loud, reviewable act reserved for a legitimate re-vendor of a
pinned upstream corpus; it must never be used to paper over an accidental edit.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import subprocess
import tomllib
from pathlib import Path

# Each guarded root -> its receipt is declared in conformance-frozen/roots.toml; a
# corpus is NEVER guarded until it appears there, and adding one is a deliberate
# edit followed by `--update`. A root is frozen whole: every payload file beneath
# it is hashed. A *flat* root freezes only the registered-suffix files directly
# inside one directory (its subdirectories are roots of their own), minus the
# files this project generates itself, which the registry names.
#
# The roots name *vendored* trees, not their first-party parents. Freezing is
# the claim "no one hand-edits this", and it is only true of a tree PurRDF
# copies from upstream and never authors into. That is why the guarded SPARQL
# entries are `suite/w3c-sparql10`, `suite/w3c-sparql11` and `suite/w3c-sparql12`
# rather than `suite/`: the sibling `suite/purrdf-*` suites are first-party cases
# this project writes as features land, and freezing them would make every added
# conformance test demand a `--update`, destroying the signal for the vendored
# corpora that actually need it.
_REGISTRY = tomllib.loads(
    (Path(__file__).resolve().parent / "conformance-frozen/roots.toml").read_text()
)
GUARDED_ROOTS: dict[str, str] = _REGISTRY["roots"]
FLAT_ROOTS: dict[str, dict] = _REGISTRY.get("flat_roots", {})
if not isinstance(GUARDED_ROOTS, dict) or not GUARDED_ROOTS or any(
    not root or not isinstance(receipt, str) or not receipt
    for root, receipt in GUARDED_ROOTS.items()
) or any(
    not root or not isinstance(flat, dict) or not flat.get("receipt")
    or not flat.get("suffixes") or any(not s.startswith(".") for s in flat["suffixes"])
    for root, flat in FLAT_ROOTS.items()
):
    raise ValueError("conformance-frozen/roots.toml requires non-empty roots, receipts and dotted suffixes")


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def corpus_root(root: Path, corpus_rel: str) -> Path:
    """Resolve the consumer-acquired XML suite without redistributing payloads."""
    if corpus_rel == "vectors/xmlconf":
        subprocess.run([sys.executable, str(root / "scripts/vendor-xmlconf.py")], check=True)
        return root / "target/conformance/xmlconf"
    return root / corpus_rel


def _is_frozen_payload(rel: Path) -> bool:
    """Vendored payload only: not first-party sidecars (a provenance README) and
    not hidden OS or editor files, so neither touches the manifest."""
    if rel.name.startswith("."):
        return False
    if rel.name in {"README.md", "PROVENANCE.md", "REUSE.toml"}:
        return False
    if rel.suffix == ".license":
        return False
    if "LICENSES" in rel.parts:
        return False
    return True


def hash_tree(root: Path) -> list[tuple[str, str]]:
    """Sorted ``(relpath, sha256-hex)`` for every payload file under *root*.

    Sorting the ``(posix_rel, digest)`` tuples, not ``Path`` objects, keeps the
    manifest reproducible off-Linux, where ``Path`` order case-folds.
    """
    entries: list[tuple[str, str]] = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        rel = path.relative_to(root)
        if not _is_frozen_payload(rel):
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        entries.append((rel.as_posix(), digest))
    entries.sort()
    return entries


def hash_flat(root: Path, flat: dict) -> list[tuple[str, str]]:
    """Sorted ``(name, sha256-hex)`` of the payload files directly in *root*."""
    skip = set(flat.get("exclude", []))
    return sorted(
        (p.name, hashlib.sha256(p.read_bytes()).hexdigest())
        for p in root.iterdir()
        if p.is_file() and p.name.endswith(tuple(flat["suffixes"]))
        and p.name not in skip and _is_frozen_payload(Path(p.name))
    )


def render_manifest(entries: list[tuple[str, str]]) -> str:
    """`sha256sum`-compatible text: ``<hex>  <relpath>`` lines, sorted by path."""
    return "".join(f"{digest}  {rel}\n" for rel, digest in entries)


def parse_manifest(text: str) -> list[tuple[str, str]]:
    entries: list[tuple[str, str]] = []
    for line in text.splitlines():
        if not line.strip():
            continue
        digest, rel = line.split("  ", 1)
        entries.append((rel, digest))
    return entries


def targets(root: Path):
    """Every guard as ``(label, receipt path, hasher)``, whole-tree roots first."""
    whole = [
        (rel, receipt, lambda r=rel: hash_tree(corpus_root(root, r)))
        for rel, receipt in GUARDED_ROOTS.items()
    ]
    flat = [
        (f"{rel} (files directly inside)", f["receipt"], lambda r=rel, f=f: hash_flat(root / r, f))
        for rel, f in FLAT_ROOTS.items()
    ]
    return whole + flat


def update(root: Path) -> int:
    for _label, receipt, hasher in targets(root):
        manifest = root / receipt
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text(render_manifest(hasher()), encoding="utf-8")
        print(f"wrote {receipt}")
    return 0


def verify(root: Path) -> int:
    failed = False
    checks = targets(root)
    for label, receipt, hasher in checks:
        manifest = root / receipt
        if not manifest.is_file():
            failed = True
            print(f"Byte-freeze FAILED: missing manifest {receipt} for {label}; regenerate with --update.", file=sys.stderr)
            continue
        want = dict(parse_manifest(manifest.read_text(encoding="utf-8")))
        have = dict(hasher())
        changed = sorted(r for r in want.keys() & have.keys() if want[r] != have[r])
        missing = sorted(want.keys() - have.keys())
        extra = sorted(have.keys() - want.keys())
        if changed or missing or extra:
            failed = True
            print(f"Byte-freeze FAILED for {label}:", file=sys.stderr)
            for rel in changed:
                print(f"  content changed: {rel}", file=sys.stderr)
            for rel in missing:
                print(f"  missing (in manifest, not on disk): {rel}", file=sys.stderr)
            for rel in extra:
                print(f"  new (on disk, not in manifest): {rel}", file=sys.stderr)
    if failed:
        print(
            "\nThe vendored corpora are byte-frozen. If this is a deliberate "
            "re-vendor, regenerate with `python3 scripts/check-corpus-frozen.py "
            "--update` and justify the diff in review.",
            file=sys.stderr,
        )
        return 1
    print(f"OK: {len(checks)} vendored corpora byte-frozen.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Byte-freeze gate for vendored corpora")
    parser.add_argument(
        "--update",
        action="store_true",
        help="regenerate the SHA-256 manifests (deliberate re-vendor only)",
    )
    args = parser.parse_args()
    root = repo_root()
    return update(root) if args.update else verify(root)


if __name__ == "__main__":
    raise SystemExit(main())
