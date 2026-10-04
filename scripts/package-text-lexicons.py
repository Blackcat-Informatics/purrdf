#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Build or verify the reproducible, separately licensed text lexicon data bundle.

The native generator validates physical/semantic identities and complete source
accounting before packaging. No third-party implementation or Python package is
used. Data terms remain distinct from the Rust implementation's license choices.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parent.parent
NAMES = {"cjdict", "thaidict", "laodict", "khmerdict", "burmesedict"}
REVISION = "21d1eb0f306e1141c10931e914dfc038c06121da"


def read_file(path: Path) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        raise ValueError(f"missing, empty or symbolic lexicon input: {path}")
    return path.read_bytes()


def payloads(root: Path) -> dict[str, bytes]:
    directory = root / "crates/text/lexicons"
    manifest_bytes = read_file(directory / "artifacts/manifest.json")
    manifest = json.loads(manifest_bytes)
    if not isinstance(manifest, dict):
        raise TypeError("lexicon manifest must be a record")
    artifacts = manifest.get("artifacts")
    if (
        manifest.get("format") != "purrdf-lexicon-bundle/v1"
        or manifest.get("icu_revision") != REVISION
        or not isinstance(artifacts, list)
        or len(artifacts) != 5
        or not all(isinstance(artifact, dict) for artifact in artifacts)
        or {artifact.get("name") for artifact in artifacts} != NAMES
    ):
        raise ValueError(
            "lexicon bundle requires the complete pinned five-artifact inventory"
        )
    contents = {
        "artifacts/manifest.json": manifest_bytes,
        "PROVENANCE.md": read_file(directory / "PROVENANCE.md"),
        "notices/Unicode-3.0.txt": read_file(directory / "notices/Unicode-3.0.txt"),
    }
    for artifact in artifacts:
        name = artifact["name"]
        physical = artifact.get("physical_blake3", "")
        semantic = artifact.get("semantic_blake3", "")
        if not all(
            isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value)
            for value in (physical, semantic)
        ):
            raise ValueError("invalid lexicon identity")
        filename = f"{name}-{physical}.cbor"
        if (
            artifact.get("artifact") != filename
            or artifact.get("source") != f"icu-78.3/{name}.txt"
            or artifact.get("collisions") != f"{name}.collisions.json"
            or artifact.get("notices")
            != ["../notices/Unicode-3.0.txt", f"../notices/{name}.txt"]
        ):
            raise ValueError("lexicon manifest path or notice inventory mismatch")
        for relative in (
            f"artifacts/{filename}",
            f"artifacts/{name}.collisions.json",
            f"icu-78.3/{name}.txt",
            f"notices/{name}.txt",
        ):
            contents[relative] = read_file(directory / relative)
        if len(contents[f"artifacts/{filename}"]) != artifact.get("artifact_bytes"):
            raise ValueError("lexicon artifact size differs from its manifest")
    contents["SHA256SUMS"] = b"".join(
        f"{hashlib.sha256(content).hexdigest()}  {name}\n".encode("ascii")
        for name, content in sorted(contents.items())
    )
    return contents


def archive_bytes(contents: dict[str, bytes], version: str) -> bytes:
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?", version):
        raise ValueError("invalid bundle version")
    output = io.BytesIO()
    with (
        gzip.GzipFile(
            fileobj=output, mode="wb", filename="", mtime=0, compresslevel=9
        ) as compressed,
        tarfile.open(
            fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT
        ) as archive,
    ):
        for name, content in sorted(contents.items()):
            member = tarfile.TarInfo(f"purrdf-text-lexicons-{version}/{name}")
            member.size = len(content)
            member.mode = 0o644
            member.mtime = 0
            member.uid = member.gid = 0
            member.uname = member.gname = ""
            archive.addfile(member, io.BytesIO(content))
    return output.getvalue()


def verify(data: bytes, contents: dict[str, bytes], version: str) -> None:
    prefix = f"purrdf-text-lexicons-{version}/"
    seen = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive:
            name = member.name.removeprefix(prefix)
            if (
                not member.name.startswith(prefix)
                or name not in contents
                or name in seen
                or not member.isfile()
                or member.uid != 0
                or member.gid != 0
                or member.uname
                or member.gname
                or member.mtime != 0
                or member.mode != 0o644
                or member.size != len(contents[name])
            ):
                raise ValueError("lexicon archive has an unexpected member or metadata")
            seen[name] = archive.extractfile(member).read()
    if seen != contents or data != archive_bytes(contents, version):
        raise ValueError(
            "lexicon archive differs from the reproducible licensed inventory"
        )


def self_test() -> None:
    contents = {"notices/Unicode-3.0.txt": b"notice", "artifacts/manifest.json": b"{}"}
    data = archive_bytes(contents, "1.2.3")
    assert data == archive_bytes(dict(reversed(list(contents.items()))), "1.2.3")
    verify(data, contents, "1.2.3")
    for wrong, expected, version in (
        (data, {"artifacts/manifest.json": b"{}"}, "1.2.3"),
        (data, {**contents, "missing": b"required"}, "1.2.3"),
        (data, contents, "1.2.4"),
        (data[:-8], contents, "1.2.3"),
    ):
        try:
            verify(wrong, expected, version)
        except (ValueError, OSError, EOFError, tarfile.TarError):
            pass
        else:
            raise AssertionError("invalid bundle accepted")
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        original = directory / "source"
        original.write_bytes(b"data")
        link = directory / "link"
        link.symlink_to(original)
        try:
            read_file(link)
        except ValueError:
            pass
        else:
            raise AssertionError("symbolic data input accepted")
    print("OK: lexicon archive determinism, inventory and refusal checks")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify existing output instead of writing it",
    )
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    try:
        subprocess.run(
            [
                "cargo",
                "run",
                "-p",
                "purrdf-text",
                "--example",
                "gen_lexicons",
                "--locked",
                "--",
                "--check",
            ],
            cwd=ROOT,
            check=True,
        )
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"][
            "package"
        ]["version"]
        contents = payloads(ROOT)
        data = archive_bytes(contents, version)
        verify(data, contents, version)
        output = (
            args.output
            or ROOT / "target/dist" / f"purrdf-text-lexicons-{version}.tar.gz"
        )
        if args.check:
            verify(read_file(output), contents, version)
        else:
            if output.is_symlink():
                raise ValueError("lexicon archive output cannot be symbolic")
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(data)
        print(
            f"OK: {len(contents)} licensed data members; {len(data)} bytes; sha256:{hashlib.sha256(data).hexdigest()}"
        )
    except (
        ValueError,
        TypeError,
        OSError,
        EOFError,
        tarfile.TarError,
        subprocess.CalledProcessError,
    ) as error:
        print(f"Lexicon packaging refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
