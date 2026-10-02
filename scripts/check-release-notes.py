#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Validate the committed GitHub summary before tagging or publishing.

The complete changelog remains the detailed history. GitHub rejects release
bodies above its reported 125,000-character limit; a 64 KiB UTF-8 ceiling keeps
the reviewed summary below that limit even for multibyte prose. Never truncate
notes or substitute generated history. The same bytes are checked locally and
copied into the tagged publisher's notes file.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parent.parent
MAXIMUM_BYTES = 64 * 1024


def workspace_version(root: Path) -> str:
    """The existing single workspace release version."""
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"][
        "version"
    ]
    if not isinstance(version, str) or not re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?",
        version,
    ):
        raise ValueError("invalid workspace release version")
    return version


def release_notes(root: Path, requested: str | None = None) -> tuple[str, bytes]:
    """Return the complete reviewed bytes only after version and size checks."""
    version = workspace_version(root)
    if requested is not None and requested != version:
        raise ValueError("requested release version differs from the workspace")
    path = root / "docs" / "releases" / f"{version}.md"
    if path.is_symlink() or not path.is_file():
        raise ValueError("the current version has no regular reviewed summary")
    body = path.read_bytes()
    if not body or len(body) > MAXIMUM_BYTES:
        raise ValueError("reviewed summary is empty or exceeds 64 KiB")
    text = body.decode("utf-8")
    visible = re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL).strip()
    heading, separator, prose = visible.partition("\n")
    if heading != f"# PurRDF {version}" or not separator or not prose.strip():
        raise ValueError("reviewed summary needs the exact version heading and prose")
    if any(line.startswith("# PurRDF ") for line in prose.splitlines()):
        raise ValueError("reviewed summary contains another release heading")
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    headings = re.findall(r"^## \[([^\]]+)\](?:[^\n]*)$", changelog, re.MULTILINE)
    if headings.count(version) != 1:
        raise ValueError("the current version needs exactly one full changelog section")
    return version, body


def checksum_bytes(root: Path, version: str) -> bytes:
    """Hash exactly the current C SDK and its two existing legal receipts."""
    directory = root / "target" / "dist"
    archives = sorted(directory.glob(f"purrdf-capi-{version}-*.tar.gz"))
    if len(archives) != 1:
        raise ValueError("release checksums need exactly one current C SDK archive")
    archive = archives[0]
    paths = [
        archive,
        archive.with_name(
            archive.name.removesuffix(".tar.gz") + ".license-receipt.json"
        ),
        root / "target" / "license-evidence-cargo.json",
    ]
    lines = []
    for path in sorted(paths, key=lambda item: item.name):
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ValueError("release checksum input is missing, empty or a symlink")
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        lines.append(f"{digest}  {path.name}\n")
    return "".join(lines).encode("ascii")


def write_output(path: Path, body: bytes) -> None:
    """Write a caller-selected build output without following a file symlink."""
    if path.is_symlink():
        raise ValueError("release output must not be a symlink")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)


def self_test() -> None:
    """Offline valid/refused neighbours; no GitHub or publisher invocation."""
    with tempfile.TemporaryDirectory(prefix="release-notes-test-") as temporary:
        root = Path(temporary)
        (root / "Cargo.toml").write_text('[workspace.package]\nversion = "3.0.0"\n')
        (root / "CHANGELOG.md").write_text("## [3.0.0]\n" + "history\n" * 70_000)
        path = root / "docs/releases/3.0.0.md"
        path.parent.mkdir(parents=True)
        notes = "<!-- SPDX-License-Identifier: CC-BY-4.0 -->\n# PurRDF 3.0.0\n\nReviewed 中文.\n".encode()
        path.write_bytes(notes)
        assert release_notes(root, "3.0.0") == ("3.0.0", notes)
        output = root / "target/release-notes.md"
        write_output(output, release_notes(root)[1])
        assert output.read_bytes() == notes

        def refused(operation: Callable[[], object]) -> None:
            try:
                operation()
            except (ValueError, UnicodeError):
                return
            raise AssertionError("invalid release input was admitted")

        refused(lambda: release_notes(root, "2.0.2"))
        for body in [
            b"",
            b"# PurRDF 2.0.2\n\nWrong version.\n",
            b"# PurRDF 3.0.0\n",
            b"# PurRDF 3.0.0\n<!-- no visible prose -->\n",
            b"# PurRDF 3.0.0\n\n\xff",
            b"# PurRDF 3.0.0\n" + b"a" * MAXIMUM_BYTES,
            ("# PurRDF 3.0.0\n" + "中" * 25_000).encode(),
            b"# PurRDF 3.0.0\n\n# PurRDF 2.0.2\nWrong release.\n",
        ]:
            path.write_bytes(body)
            refused(lambda: release_notes(root))
        path.unlink()
        refused(lambda: release_notes(root))
        path.symlink_to(output)
        refused(lambda: release_notes(root))
        path.unlink()
        path.write_bytes(notes)
        (root / "CHANGELOG.md").write_text("## [2.0.2]\nOld.\n")
        refused(lambda: release_notes(root))
        (root / "CHANGELOG.md").write_text("## [3.0.0]\nA.\n## [3.0.0]\nB.\n")
        refused(lambda: release_notes(root))
        (root / "CHANGELOG.md").write_text("## [3.0.0]\nFull history.\n")
        assert release_notes(root)[1] == notes

        directory = root / "target/dist"
        directory.mkdir()
        archive = directory / "purrdf-capi-3.0.0-x86_64-unknown-linux-gnu.tar.gz"
        archive.write_bytes(b"actual SDK fixture")
        receipt = archive.with_name(
            archive.name.removesuffix(".tar.gz") + ".license-receipt.json"
        )
        receipt.write_bytes(b"actual C receipt fixture")
        cargo = root / "target/license-evidence-cargo.json"
        cargo.write_bytes(b"actual Cargo receipt fixture")
        expected = b"".join(
            f"{hashlib.sha256(item.read_bytes()).hexdigest()}  {item.name}\n".encode()
            for item in sorted([archive, receipt, cargo], key=lambda item: item.name)
        )
        assert checksum_bytes(root, "3.0.0") == expected
        archive.write_bytes(b"changed SDK fixture")
        assert checksum_bytes(root, "3.0.0") != expected
        extra = directory / "purrdf-capi-3.0.0-other.tar.gz"
        extra.write_bytes(b"ambiguous SDK")
        refused(lambda: checksum_bytes(root, "3.0.0"))
        extra.unlink()
        receipt.unlink()
        refused(lambda: checksum_bytes(root, "3.0.0"))
        receipt.symlink_to(cargo)
        refused(lambda: checksum_bytes(root, "3.0.0"))
        output.unlink()
        output.symlink_to(path)
        refused(lambda: write_output(output, b"must not replace source"))
        assert path.read_bytes() == notes
    print("OK: reviewed-summary and exact release-checksum refusal regressions pass")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--checksum-output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.version or args.output or args.checksum_output:
            parser.error("--self-test accepts no release-output arguments")
        self_test()
        return 0
    try:
        version, notes = release_notes(ROOT, args.version)
        if args.output:
            write_output(args.output, notes)
        if args.checksum_output:
            write_output(args.checksum_output, checksum_bytes(ROOT, version))
    except (OSError, ValueError, KeyError) as error:
        print(f"Release notes refused: {error}", file=sys.stderr)
        return 1
    print(f"OK: reviewed PurRDF {version} GitHub summary ({len(notes)} UTF-8 bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
