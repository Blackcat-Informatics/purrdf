#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""--plan reads registry records. --prepare verifies isolated empty archives.
--publish prepares the entire set before uploading only missing crate records.
Functional implementations are never uploaded by this token bootstrap lane.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parent.parent
GRANT = "MIT OR Apache-2.0 OR MulanPSL-2.0"


def registry(name: str, version: str | None = None) -> dict | None:
    """Use the same unauthenticated HTTP/status/mock law as release preflight."""
    route = name + (f"/{version}" if version else "")
    result = subprocess.run(
        [
            "bash",
            "-c",
            r'''source "$1"; trap 'rm -f "$CRATES_IO_BODY"' EXIT;
crates_io_get "$2" "$3"; printf '%s\n' "$CRATES_IO_STATUS"; cat "$CRATES_IO_BODY"''',
            "purrdf-bootstrap-registry",
            str(ROOT / "scripts/crates-io-api.sh"),
            route,
            "purrdf-bootstrap/0.0.0 (paudley@blackcatinformatics.ca)",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    status, _, body = result.stdout.partition("\n")
    if status == "404":
        return None
    if status != "200":
        raise ValueError(f"registry returned {status} for {route}")
    return json.loads(body)


def write_package(name: str, directory: Path, repository: str) -> None:
    directory.mkdir(parents=True, exist_ok=False)
    (directory / "src").mkdir()
    (directory / "src/lib.rs").write_text(
        "// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>\n"
        f"// SPDX-License-Identifier: {GRANT}\n#![no_std]\n"
    )
    (directory / "README.md").write_text(
        f"# {name} registry bootstrap\n\nVersion 0.0.0 is an empty, dependency-free package "
        "that creates the crate record for Trusted Publishing. It exposes no runtime API. "
        "Install the subsequent functional release.\n\n"
        f"First-party license: {GRANT}. Full texts accompany this package.\n"
    )
    for notice in ("LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN"):
        (directory / notice).write_bytes((ROOT / notice).read_bytes())
    values = {
        "name": name,
        "version": "0.0.0",
        "edition": "2021",
        "license": GRANT,
        "repository": repository,
        "readme": "README.md",
        "description": f"Empty registry bootstrap for {name}; use the functional release",
    }
    (directory / "Cargo.toml").write_text(
        "[package]\n"
        + "\n".join(f"{key} = {json.dumps(value)}" for key, value in values.items())
        + '\n\n[workspace]\n\n[lib]\npath = "src/lib.rs"\n'
    )


def audit_empty(path: Path, name: str) -> None:
    with tarfile.open(path) as archive:
        members = {
            item.name: archive.extractfile(item).read()
            for item in archive.getmembers()
            if item.isfile()
        }
    prefix = f"{name}-0.0.0/"
    source = members[prefix + "src/lib.rs"].decode()
    body = "\n".join(
        line for line in source.splitlines() if not line.startswith("//")
    ).strip()
    if body != "#![no_std]":
        raise ValueError(f"{name}: bootstrap is not an empty no_std library")
    manifest = tomllib.loads(members[prefix + "Cargo.toml"].decode())
    if manifest["package"]["version"] != "0.0.0" or manifest["package"]["name"] != name:
        raise ValueError(f"{name}: wrong bootstrap package identity")
    if any(
        manifest.get(kind)
        for kind in (
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "target",
            "features",
        )
    ):
        raise ValueError(f"{name}: bootstrap contains dependencies or features")
    for notice in ("LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN"):
        if members.get(prefix + notice) != (ROOT / notice).read_bytes():
            raise ValueError(f"{name}: missing/altered bootstrap license {notice}")


def cargo(directory: Path, *arguments: str) -> None:
    # A separately installed stable validation toolchain can be selected
    # without creating a `stable` alias in the operator's primary rustup home.
    toolchain = os.environ.get("PURRDF_BOOTSTRAP_TOOLCHAIN", "stable")
    compiler = subprocess.check_output(
        ["rustup", "run", toolchain, "rustc", "--version"], text=True
    )
    if "nightly" in compiler or "beta" in compiler or not compiler.startswith("rustc "):
        raise ValueError(
            "empty registry bootstrap verification requires a stable Rust compiler"
        )
    subprocess.run(
        ["rustup", "run", toolchain, "cargo", *arguments], cwd=directory, check=True
    )


def self_test() -> None:
    # These temporary fixtures test package verification and the empty-API
    # refusal under the admitted development compiler, just like make check.
    # Real --prepare/--publish always call cargo()'s stable-only lane below.
    def verify_fixture(directory: Path) -> None:
        compiler_root = Path(
            subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip()
        )
        executable = (
            compiler_root / "bin" / ("cargo.exe" if os.name == "nt" else "cargo")
        )
        # Use this admitted compiler's Cargo executable so a host wrapper's
        # unrelated shared target relocation cannot hide the fixture archive.
        subprocess.run(
            [
                str(executable),
                "package",
                "--offline",
                "--target-dir",
                str(directory / "target"),
            ],
            cwd=directory,
            check=True,
        )

    with tempfile.TemporaryDirectory(prefix="purrdf-empty-selftest-") as raw:
        directory = Path(raw) / "purrdf-bootstrap-fixture"
        write_package(
            directory.name, directory, "https://github.com/Blackcat-Informatics/purrdf"
        )
        verify_fixture(directory)
        path = directory / f"target/package/{directory.name}-0.0.0.crate"
        audit_empty(path, directory.name)
        (directory / "src/lib.rs").write_text(
            "#![no_std]\npub fn accidental_api() {}\n"
        )
        verify_fixture(directory)
        try:
            audit_empty(path, directory.name)
        except ValueError:
            print(
                "bootstrap self-test: verified empty package accepted; accidental API refused"
            )
            # Exercise the shared registry read law without credentials/network.
            mock = Path(raw) / "registry"
            mock.mkdir()
            previous = os.environ.get("PURRDF_CRATES_IO_MOCK")
            os.environ["PURRDF_CRATES_IO_MOCK"] = str(mock)
            try:
                assert registry("missing-fixture") is None
                (mock / "present-fixture").write_text(
                    '200\n{"crate":{"name":"present-fixture"}}'
                )
                assert registry("present-fixture")["crate"]["name"] == "present-fixture"
                (mock / "fault-fixture").write_text("503\n")
                try:
                    registry("fault-fixture")
                except ValueError:
                    pass
                else:
                    raise ValueError(
                        "bootstrap self-test accepted unavailable registry"
                    )
            finally:
                if previous is None:
                    del os.environ["PURRDF_CRATES_IO_MOCK"]
                else:
                    os.environ["PURRDF_CRATES_IO_MOCK"] = previous
            print(
                "bootstrap registry self-test: missing/present records distinguished; unavailable registry refused"
            )
        else:
            raise ValueError("bootstrap self-test accepted a nonempty package")


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    if "--" not in sys.argv or sys.argv[1:2] != ["--crates"]:
        raise ValueError("use bootstrap-crates-io.sh to load the release ledger")
    split = sys.argv.index("--")
    names = sys.argv[2:split]
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    for flag in ("plan", "prepare", "publish", "self-test"):
        action.add_argument(f"--{flag}", action="store_true")
    parser.add_argument(
        "--output", type=Path, help="fresh retained directory outside the workspace"
    )
    args = parser.parse_args(sys.argv[split + 1 :])
    if args.self_test:
        self_test()
        return
    if len(names) != len(set(names)):
        raise ValueError("duplicate bootstrap ledger crate")
    missing, existing = [], []
    for name in names:
        record = registry(name)
        (missing if record is None else existing).append(name)
        print(
            f"{'missing' if record is None else 'existing; token republish refused'}: {name}"
        )
        if not os.environ.get("PURRDF_CRATES_IO_MOCK"):
            time.sleep(1)
    if args.plan or not (args.prepare or args.publish):
        print(f"Empty 0.0.0 packages to prepare: {', '.join(missing) or 'none'}")
        return
    if args.publish and not (
        os.environ.get("CARGO_REGISTRY_TOKEN") or os.environ.get("CARGO_TOKEN")
    ):
        raise ValueError(
            "publication requires CARGO_REGISTRY_TOKEN or CARGO_TOKEN; no credential has been read"
        )
    destination = (
        args.output.resolve()
        if args.output
        else Path(tempfile.mkdtemp(prefix="purrdf-empty-bootstrap-"))
    )
    if destination == ROOT or ROOT in destination.parents:
        raise ValueError("bootstrap packages must be outside the workspace")
    destination.mkdir(parents=True, exist_ok=True)
    repository = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"][
        "package"
    ]["repository"]
    receipts = []
    for name in missing:
        directory = destination / name
        write_package(name, directory, repository)
        cargo(directory, "test", "--offline")
        cargo(directory, "check", "--offline", "--target", "wasm32-unknown-unknown")
        cargo(directory, "package", "--offline")
        path = directory / f"target/package/{name}-0.0.0.crate"
        audit_empty(path, name)
        receipts.append(
            {
                "crate": name,
                "version": "0.0.0",
                "archive": str(path),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "status": "verified; not published",
            }
        )
    receipt = destination / "receipts.json"

    def save() -> None:
        receipt.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "packages": receipts,
                    "existing_records_skipped": existing,
                },
                indent=2,
            )
            + "\n"
        )

    save()
    print(f"All bootstrap archives verified before upload. Receipt: {receipt}")
    if args.publish:
        if os.environ.get("CARGO_TOKEN") and not os.environ.get("CARGO_REGISTRY_TOKEN"):
            os.environ["CARGO_REGISTRY_TOKEN"] = os.environ["CARGO_TOKEN"]
        for item in receipts:
            name = item["crate"]
            if registry(name) is not None:
                raise ValueError(
                    f"{name}: record appeared after preparation; refusing token republish"
                )
            cargo(destination / name, "publish", "--locked")
            item["status"] = "upload returned success; registry confirmation pending"
            save()
            for _ in range(30):
                if registry(name, "0.0.0") is not None:
                    item["status"] = "published; registry version confirmed"
                    break
                time.sleep(10)
            else:
                raise ValueError(
                    f"{name}: upload succeeded but registry visibility timed out; inspect before retry"
                )
            save()
        print(
            "Configure every Trusted Publisher and Require trusted publishing lock, clear the completed ledger, then use the functional OIDC release lane. Yank 0.0.0 only after the functional release is confirmed."
        )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"bootstrap-crates-io: {error}") from error
