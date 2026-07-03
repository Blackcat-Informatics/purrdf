#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

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
from pathlib import Path

# Each guarded root -> its manifest file (repo-relative). A root is frozen whole:
# every file beneath it is hashed. These are the corpora the SHACL/shexTest
# conformance runners consume, plus the first-party frozen SHACL corpus — all
# declared byte-frozen. (The GTS `vectors/*.gts` corpus is governed separately in
# gmeow-gts and is intentionally not policed here; adding a new root is a
# deliberate edit to this map followed by `--update`.)
GUARDED_ROOTS: dict[str, str] = {
    "vectors/shacl": "scripts/conformance-frozen/vectors-shacl.sha256",
    "vectors/shexTest": "scripts/conformance-frozen/vectors-shexTest.sha256",
    "crates/shapes/corpus": "scripts/conformance-frozen/shapes-corpus.sha256",
}


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def _is_frozen_payload(rel: Path) -> bool:
    """Vendored payload only — skip first-party sidecars so editing them (e.g. a
    provenance README) does not require regenerating the freeze manifest."""
    if rel.name == "README.md":
        return False
    if rel.suffix == ".license":
        return False
    if "LICENSES" in rel.parts:
        return False
    return True


def hash_tree(root: Path) -> list[tuple[str, str]]:
    """Return sorted ``(relpath, sha256-hex)`` for every payload file under *root*."""
    entries: list[tuple[str, str]] = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        rel = path.relative_to(root)
        if not _is_frozen_payload(rel):
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        entries.append((rel.as_posix(), digest))
    return entries


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


def update(root: Path) -> int:
    for corpus_rel, manifest_rel in GUARDED_ROOTS.items():
        manifest = root / manifest_rel
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text(
            render_manifest(hash_tree(root / corpus_rel)), encoding="utf-8"
        )
        print(f"wrote {manifest_rel}")
    return 0


def verify(root: Path) -> int:
    failed = False

    for corpus_rel, manifest_rel in GUARDED_ROOTS.items():
        manifest = root / manifest_rel
        if not manifest.is_file():
            failed = True
            print(
                f"Byte-freeze FAILED: missing manifest {manifest_rel} for "
                f"{corpus_rel}; regenerate with --update.",
                file=sys.stderr,
            )
            continue
        want = dict(parse_manifest(manifest.read_text(encoding="utf-8")))
        have = dict(hash_tree(root / corpus_rel))

        changed = sorted(r for r in want.keys() & have.keys() if want[r] != have[r])
        missing = sorted(want.keys() - have.keys())
        extra = sorted(have.keys() - want.keys())
        if changed or missing or extra:
            failed = True
            print(f"Byte-freeze FAILED for {corpus_rel}:", file=sys.stderr)
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

    total = sum(1 for _ in GUARDED_ROOTS)
    print(f"OK: {total} vendored corpora byte-frozen.")
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
