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


def fetch_export() -> Path:
    CACHE.mkdir(parents=True, exist_ok=True)
    cached = CACHE / "cns-core.jsonId"
    if not cached.exists():
        print(f"fetching {EXPORT_URL}")
        with urllib.request.urlopen(EXPORT_URL) as response:  # noqa: S310 - pinned https URL
            cached.write_bytes(response.read())
    return cached


def convert_canonical(purrdf: list[str], source: Path, out: Path, from_format: str) -> None:
    subprocess.run(
        [*purrdf, "convert", "--from", from_format, "--canonical", str(source), str(out)],
        check=True,
        cwd=REPO_ROOT,
    )


def self_test() -> int:
    probe = CACHE / "self-test.bin"
    CACHE.mkdir(parents=True, exist_ok=True)
    probe.write_bytes(b"not the export")
    if sha256_of(probe) == SOURCE_SHA256:
        print("SELF-TEST FAIL: wrong bytes matched the pinned digest")
        return 1
    print("OK: cnschema-probe self-test (a digest mismatch is detectable)")
    return 0


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
        help="path to a purrdf binary (default: `cargo run -q -p purrdf-cli --`)",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    source = args.file if args.file else fetch_export()
    verify_digest(source, SOURCE_SHA256, "source export")

    purrdf = (
        [str(args.purrdf)]
        if args.purrdf
        else ["cargo", "run", "--quiet", "-p", "purrdf-cli", "--"]
    )
    CACHE.mkdir(parents=True, exist_ok=True)
    leg1 = CACHE / "round1.nq"
    leg2 = CACHE / "round2.nq"

    convert_canonical(purrdf, source, leg1, "jsonld")
    verify_digest(leg1, CANONICAL_SHA256, "canonical N-Quads (import leg)")
    quads = sum(1 for _ in leg1.open("rb"))
    if quads != QUAD_COUNT:
        sys.exit(f"FAIL: expected {QUAD_COUNT} quads, canonical output has {quads}")
    print(f"OK: {quads} quads")

    convert_canonical(purrdf, leg1, leg2, "nquads")
    verify_digest(leg2, CANONICAL_SHA256, "canonical N-Quads (re-import leg)")

    print(
        "PASS: cnSchema 4.0 round-trip is byte-identical "
        f"(commit {PINNED_COMMIT[:12]}, {QUAD_COUNT} quads)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
