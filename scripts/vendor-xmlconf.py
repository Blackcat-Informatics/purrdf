#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Acquire the frozen XML conformance suite into the local target cache.

The contributed suites keep their inherited terms. This repository redistributes
no extracted test payload. The pinned upstream archive is downloaded by each
consumer, checked by SHA-256, and every extracted payload is checked against the
unchanged freeze manifest. An existing valid cache avoids network access.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import io
import shutil
import os
import tempfile
import tarfile
import urllib.request
from pathlib import PurePosixPath
from pathlib import Path

URL = "https://www.w3.org/XML/Test/xmlts20130923.tar.gz"
SHA256 = "9b61db9f5dbffa545f4b8d78422167083a8568c59bd1129f94138f936cf6fc1f"
TOP = "xmlconf"

def expected_files() -> dict[str, str]:
    manifest = Path(__file__).resolve().parent / "conformance-frozen/vectors-xmlconf.sha256"
    return {name: digest for digest, name in (
        line.split("  ", 1) for line in manifest.read_text().splitlines() if line.strip()
    )}


def verify_cache(output: Path) -> bool:
    expected = expected_files()
    actual = {p.relative_to(output).as_posix(): p for p in output.rglob("*") if p.is_file()}
    return actual.keys() == expected.keys() and all(
        hashlib.sha256(actual[name].read_bytes()).hexdigest() == digest
        for name, digest in expected.items()
    )


@contextmanager
def cache_lock(path: Path):
    """OS-owned lock released on process exit, shared by harness and freeze gate."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+b") as handle:
        if os.name == "nt":
            import msvcrt
            if handle.tell() == 0:
                handle.write(b"0")
                handle.flush()
            handle.seek(0)
            msvcrt.locking(handle.fileno(), msvcrt.LK_LOCK, 1)
            try:
                yield
            finally:
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def atomic_archive(path: Path, data: bytes) -> None:
    with tempfile.NamedTemporaryFile(prefix=".xmlconf-archive-", dir=path.parent, delete=False) as handle:
        temporary = Path(handle.name)
        try:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        except BaseException:
            temporary.unlink(missing_ok=True)
            raise
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def fetch() -> bytes:
    request = urllib.request.Request(URL, headers={"User-Agent": "purrdf-vendor-xmlconf"})
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310 - pinned https host
        return response.read()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "target" / "conformance" / "xmlconf",
        help="private local destination (default: target/conformance/xmlconf)",
    )
    args = parser.parse_args()

    output = args.output.resolve()
    root = Path(__file__).resolve().parent.parent
    allowed = [root / "target"]
    if os.environ.get("CARGO_TARGET_DIR"):
        allowed.append(Path(os.environ["CARGO_TARGET_DIR"]).resolve())
    if not any(output.is_relative_to(path.resolve()) and output != path.resolve() for path in allowed):
        raise SystemExit("XML conformance payloads may be acquired only beneath the local target cache")
    with cache_lock(output.parent / ".xmlconf-acquisition.lock"):
        acquire(output)


def acquire(output: Path) -> None:
    if verify_cache(output):
        print(f"verified {len(expected_files())} frozen XML files in {output}")
        return

    output.parent.mkdir(parents=True, exist_ok=True)
    archive = output.parent / ".xmlts20130923.tar.gz"
    data = archive.read_bytes() if archive.is_file() else fetch()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"{URL}: SHA-256 {got}, pinned {SHA256}")
    atomic_archive(archive, data)

    expected = expected_files()
    with tempfile.TemporaryDirectory(prefix=".xmlconf-acquire-", dir=output.parent) as scratch:
        stage = Path(scratch) / "xmlconf"
        stage.mkdir()
        seen = set()
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
            for member in tar:
                path = PurePosixPath(member.name)
                if member.isdir():
                    continue
                if not member.isreg():
                    raise SystemExit(f"{member.name}: not a regular file")
                if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != TOP:
                    raise SystemExit(f"{member.name}: outside {TOP}/")
                rel = PurePosixPath(*path.parts[1:]).as_posix()
                if rel in seen or rel not in expected:
                    raise SystemExit(f"{member.name}: duplicate or unfrozen payload")
                seen.add(rel)
                handle = tar.extractfile(member)
                assert handle is not None
                body = handle.read()
                if hashlib.sha256(body).hexdigest() != expected[rel]:
                    raise SystemExit(f"{member.name}: frozen payload differs")
                target = stage / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(body)
        if seen != expected.keys():
            raise SystemExit(f"upstream archive is missing {sorted(expected.keys() - seen)}")
        if not verify_cache(stage):
            raise SystemExit("extracted cache does not match the complete frozen corpus")
        if output.exists():
            shutil.rmtree(output)
        stage.replace(output)
    print(f"acquired and verified {len(expected)} file(s) from {URL} into {output}")


if __name__ == "__main__":
    main()
