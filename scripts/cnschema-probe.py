#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reproduce the pinned cnSchema 4.0 round-trip probe (`make cnschema-probe`).

The probe: read the pinned cnSchema 4.0 JSON-LD export, emit RDFC-1.0
canonical N-Quads, re-import those N-Quads, and canonicalize again. All three
identities are pinned below: the source bytes, the canonical N-Quads digest
(which both legs must produce, proving a lossless round-trip), and the quad
count.

The export is fetched at run time and verified by digest, **never vendored**:
the upstream repository publishes no license file (checked 2026-09-15 at the
pinned commit and at HEAD — the hosting API reports no detected license), and
without an explicit grant redistribution is not admitted. A cached copy is
kept under ``target/`` (already ignored); ``--file`` runs fully offline
against a pre-fetched copy.

This is release evidence, not a CI gate: it needs the network once and builds
the CLI. It must stay reproducible with exactly one command.
"""

import argparse
import hashlib
import os
import subprocess
import sys
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

PINNED_COMMIT = "ecfa596d50b0578d1a28df8f9c76bdafe7069ec6"
EXPORT_URL = (
    "https://raw.githubusercontent.com/cnschema/cnSchema/"
    f"{PINNED_COMMIT}/data/releases/4.0/cns-core.jsonId"
)
# SHA-256 of the source export bytes.
SOURCE_SHA256 = "92329fd996672b7ffdceda283da8c9006853bce8bd232a6ee35df62e301c444a"
# SHA-256 of the RDFC-1.0 canonical N-Quads — identical on both legs.
CANONICAL_SHA256 = "8a4b7c1178e920f2bbe8be3f0944ab29892c8b723710dc91038098520e23c4cd"
QUAD_COUNT = 40_936

CACHE = REPO_ROOT / "target" / "cnschema-probe"


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_digest(path: Path, expected: str, label: str) -> None:
    actual = sha256_of(path)
    if actual != expected:
        sys.exit(
            f"FAIL: {label} digest mismatch\n  expected {expected}\n  actual   {actual}"
        )
    print(f"OK: {label} digest {expected}")


def count_nquads_lines(path: Path) -> int:
    """Count newline-terminated lines in a canonical N-Quads file.

    RDFC-1.0 canonical N-Quads emits exactly one statement per line with no
    embedded newlines inside a term, so a line count *is* a quad count for
    this format. That equivalence is specific to canonical N-Quads: it would
    silently mis-count for any format whose output can wrap a term across
    lines.
    """
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def check_quad_count(count: int) -> None:
    """Assert *count* (as produced by ``count_nquads_lines``) matches QUAD_COUNT."""
    if count != QUAD_COUNT:
        sys.exit(
            f"FAIL: expected {QUAD_COUNT} canonical N-Quads lines (one quad per line), "
            f"got {count}"
        )
    print(f"OK: {count} quads")


def _download_verified(dest: Path) -> bool:
    """Fetch EXPORT_URL to *dest* atomically.

    Downloads to a temporary file in the same directory and, only if the
    downloaded bytes verify against SOURCE_SHA256, ``os.replace()``s it into
    place — atomic within a filesystem. A truncated response, proxy error
    page, or interrupted download therefore never poisons *dest*. Returns
    whether the bytes verified; on failure *dest* is left untouched.
    """
    print(f"fetching {EXPORT_URL}")
    tmp = dest.with_name(dest.name + ".part")
    try:
        with urllib.request.urlopen(EXPORT_URL) as response:  # noqa: S310 - pinned https URL
            tmp.write_bytes(response.read())
        if sha256_of(tmp) != SOURCE_SHA256:
            return False
        os.replace(tmp, dest)
        return True
    finally:
        tmp.unlink(missing_ok=True)


def fetch_export() -> Path:
    CACHE.mkdir(parents=True, exist_ok=True)
    cached = CACHE / "cns-core.jsonId"

    if cached.exists() and sha256_of(cached) != SOURCE_SHA256:
        print(f"cached export at {cached} failed digest verification; discarding and re-fetching once")
        cached.unlink()

    if not cached.exists() and not _download_verified(cached):
        sys.exit(
            "FAIL: freshly fetched source export does not match the pinned digest "
            "(SOURCE_SHA256) after one fetch — this is not a cache problem; check the pin "
            "or the upstream export"
        )

    return cached


def convert_canonical(purrdf: list[str], source: Path, out: Path, from_format: str) -> None:
    subprocess.run(
        [*purrdf, "convert", "--from", from_format, "--canonical", str(source), str(out)],
        check=True,
        cwd=REPO_ROOT,
    )


def self_test() -> int:
    """Exercise the real failure paths, offline, without building the CLI.

    Two cases:
      1. Run this script's actual entry point in a subprocess against a
         wrong-bytes ``--file`` and assert a non-zero exit and a ``FAIL:``
         message (the source-digest guard).
      2. Feed a canonical-N-Quads-shaped file with the wrong number of lines
         through the real counting and guard functions and assert the
         quad-count check fails with a non-zero exit.
    """
    tmp_dir = CACHE / "self-test"
    tmp_dir.mkdir(parents=True, exist_ok=True)
    ok = True

    wrong_bytes = tmp_dir / "wrong-bytes.jsonId"
    wrong_bytes.write_bytes(b"not the export")
    result = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), "--file", str(wrong_bytes)],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0 or "FAIL:" not in result.stderr:
        print("SELF-TEST FAIL: wrong-bytes --file did not exit non-zero with a FAIL: message")
        print(f"  exit code: {result.returncode}")
        print(f"  stderr: {result.stderr!r}")
        ok = False
    else:
        print("OK: self-test case 1 (wrong-bytes --file fails digest verification, exit != 0)")

    bad_count_file = tmp_dir / "wrong-line-count.nq"
    bad_count_file.write_bytes(
        b"<urn:example:s> <urn:example:p> <urn:example:o> .\n"
        b"<urn:example:s> <urn:example:p> <urn:example:o2> .\n"
    )
    bad_line_count = count_nquads_lines(bad_count_file)
    try:
        check_quad_count(bad_line_count)
    except SystemExit as exc:
        if exc.code and "FAIL:" in str(exc.code):
            print("OK: self-test case 2 (quad-count guard rejects a wrong line count, exit != 0)")
        else:
            print(f"SELF-TEST FAIL: quad-count guard exited with unexpected code {exc.code!r}")
            ok = False
    else:
        print("SELF-TEST FAIL: quad-count guard accepted a wrong line count")
        ok = False

    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--file",
        type=Path,
        help="pre-fetched copy of the export (offline mode; still digest-verified)",
    )
    parser.add_argument(
        "--purrdf",
        type=Path,
        help="path to a purrdf binary (default: `cargo run -q --locked -p purrdf-cli --`)",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    # Resolve caller-supplied paths to absolute *before* anything runs a child process
    # with cwd=REPO_ROOT (see convert_canonical): a relative path verified against the
    # caller's cwd and then handed to a child rooted at REPO_ROOT would be two different
    # files.
    if args.file is not None:
        args.file = args.file.resolve()
    if args.purrdf is not None:
        args.purrdf = args.purrdf.resolve()

    source = args.file if args.file else fetch_export()
    verify_digest(source, SOURCE_SHA256, "source export")

    purrdf = (
        [str(args.purrdf)]
        if args.purrdf
        else ["cargo", "run", "--quiet", "--locked", "-p", "purrdf-cli", "--"]
    )
    CACHE.mkdir(parents=True, exist_ok=True)
    leg1 = CACHE / "round1.nq"
    leg2 = CACHE / "round2.nq"

    convert_canonical(purrdf, source, leg1, "jsonld")
    verify_digest(leg1, CANONICAL_SHA256, "canonical N-Quads (import leg)")
    check_quad_count(count_nquads_lines(leg1))

    convert_canonical(purrdf, leg1, leg2, "nquads")
    verify_digest(leg2, CANONICAL_SHA256, "canonical N-Quads (re-import leg)")

    print(
        "PASS: cnSchema 4.0 round-trip is byte-identical "
        f"(commit {PINNED_COMMIT[:12]}, {QUAD_COUNT} quads)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
