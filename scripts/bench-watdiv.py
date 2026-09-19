#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The WatDiv comparison lane (`make bench-watdiv`): fetch, pin, run.

WatDiv is citation-ware (free to use with a citation of Aluç/Hartig/Özsu/
Daudjee, ISWC 2014; no explicit redistribution grant), so nothing here is
vendored: the v0.6 tarball is fetched by digest (upstream's own md5 is
cross-checked, plus this project's sha256 pin) and only its 20 basic query
templates are extracted for use.

Stock WatDiv generation is time-seeded with no seed option — a generated
corpus is unreproducible even on one machine — so this lane NEVER generates:
it consumes a **frozen dataset** pinned by digest. Upstream's pre-generated
datasets publish no checksums, so the first fetch of a dataset records its
digest into a local pin file and every later use must match
(first-fetch-pin). A one-time local generation archived the same way is
equally admissible; either way, the corpus identity is the digest, not the
generation process.

Query-template instantiation (%vN% placeholders) is upstream's
nondeterministic query generator; a deterministic, documented instantiation
method is a recorded open item — until then this lane runs ingest/scan
evidence only, and says so rather than pretending. Timings are report-only.
"""

import argparse
import hashlib
import json
import subprocess
import sys
import tarfile
import time
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CACHE = REPO_ROOT / "target" / "bench" / "watdiv"

TARBALL_URL = "https://dsg.uwaterloo.ca/watdiv/watdiv_v06.tar"
TARBALL_SHA256 = "fb8d930b74b3fbc8f948101bfaf658a90d2f74002f1fefda465c45ffd33a71d2"
TARBALL_MD5_UPSTREAM = "9eac247dfdec044d7fa0141ea3ad361f"
CITATION = (
    "G. Aluç, O. Hartig, M. T. Özsu, K. Daudjee: Diversified Stress Testing "
    "of RDF Data Management Systems. ISWC 2014."
)

PINS = CACHE / "dataset-pins.json"


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def md5_of(path: Path) -> str:
    digest = hashlib.md5()  # noqa: S324 - upstream publishes md5; cross-check only
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fetch() -> None:
    CACHE.mkdir(parents=True, exist_ok=True)
    tarball = CACHE / "watdiv_v06.tar"
    if not tarball.exists():
        print(f"fetching {TARBALL_URL}")
        with urllib.request.urlopen(TARBALL_URL) as response:  # noqa: S310
            tarball.write_bytes(response.read())
    if sha256_of(tarball) != TARBALL_SHA256:
        sys.exit("FAIL: watdiv_v06.tar sha256 mismatch against the project pin")
    if md5_of(tarball) != TARBALL_MD5_UPSTREAM:
        sys.exit("FAIL: watdiv_v06.tar md5 mismatch against UPSTREAM's published checksum")
    templates = CACHE / "templates"
    templates.mkdir(exist_ok=True)
    count = 0
    with tarfile.open(tarball) as archive:
        for member in archive.getmembers():
            name = Path(member.name).name
            if "/testsuite/" in member.name and member.isfile() and name.endswith(".txt"):
                data = archive.extractfile(member)
                if data is not None:
                    (templates / name).write_bytes(data.read())
                    count += 1
    print(f"OK: tarball verified (sha256 + upstream md5); {count} templates extracted")
    print(f"NOTE: results using WatDiv must cite: {CITATION}")


def pin_dataset(path: Path) -> str:
    """First use pins the dataset digest; later uses must match it."""
    pins = json.loads(PINS.read_text()) if PINS.exists() else {}
    digest = sha256_of(path)
    key = path.name
    if key in pins and pins[key] != digest:
        sys.exit(
            f"FAIL: dataset {key} digest {digest[:16]}… does not match its "
            f"recorded pin {pins[key][:16]}… — a frozen corpus must not change"
        )
    if key not in pins:
        pins[key] = digest
        CACHE.mkdir(parents=True, exist_ok=True)
        PINS.write_text(json.dumps(pins, indent=1, sort_keys=True) + "\n")
        print(f"OK: pinned {key} sha256 {digest}")
    else:
        print(f"OK: {key} matches its pin")
    return digest


def ingest(dataset: Path) -> None:
    """Ingest/scan evidence over the frozen dataset (report-only timings)."""
    digest = pin_dataset(dataset)
    started = time.monotonic()
    completed = subprocess.run(
        ["cargo", "run", "--quiet", "--release", "-p", "purrdf-cli", "--",
         "convert", "--from", "ntriples", "--to", "nquads",
         str(dataset), str(CACHE / "ingest-echo.nq")],
        cwd=REPO_ROOT,
        check=False,
    )
    elapsed_ms = int((time.monotonic() - started) * 1000)
    print(json.dumps({
        "suite": "watdiv-ingest",
        "dataset_sha256": digest,
        "ok": completed.returncode == 0,
        "wall_ms_report_only": elapsed_ms,
        "queries": "not-run: deterministic template instantiation is a recorded open item",
    }))


def self_test() -> int:
    CACHE.mkdir(parents=True, exist_ok=True)
    probe = CACHE / "self-test.bin"
    probe.write_bytes(b"not the tarball")
    if sha256_of(probe) == TARBALL_SHA256:
        print("SELF-TEST FAIL: wrong bytes matched the tarball pin")
        return 1
    # First-fetch-pin must refuse a changed corpus.
    trial = CACHE / "self-test-corpus.nt"
    trial.write_bytes(b"<a> <b> <c> .\n")
    pins_backup = PINS.read_text() if PINS.exists() else None
    try:
        pin_dataset(trial)
        trial.write_bytes(b"<a> <b> <d> .\n")
        try:
            pin_dataset(trial)
        except SystemExit:
            print("OK: bench-watdiv self-test (pin mismatch is refused)")
            return 0
        print("SELF-TEST FAIL: a changed corpus was accepted against its pin")
        return 1
    finally:
        if pins_backup is None:
            PINS.unlink(missing_ok=True)
        else:
            PINS.write_text(pins_backup)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--fetch", action="store_true")
    parser.add_argument("--ingest", type=Path, metavar="FROZEN_DATASET")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.fetch:
        fetch()
    if args.ingest is not None:
        ingest(args.ingest)
    return 0


if __name__ == "__main__":
    sys.exit(main())
