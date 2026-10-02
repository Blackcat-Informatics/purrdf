#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Publish the single tarball bound by the recipient-notice audit, without repacking."""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path


def audited_tarball(package_dir: Path, receipt: Path) -> Path:
    artifacts = sorted(package_dir.glob("*.tgz"))
    if len(artifacts) != 1 or artifacts[0].is_symlink() or not artifacts[0].is_file():
        raise ValueError("publication requires exactly one regular npm tarball")
    artifact = artifacts[0].resolve(strict=True)
    package = json.loads((package_dir / "package.json").read_bytes())
    expected = f"{package['name'].removeprefix('@').replace('/', '-')}-{package['version']}.tgz"
    if artifact.name != expected:
        raise ValueError("tarball identity differs from the current npm package")
    evidence = json.loads(receipt.read_bytes())
    rows = evidence.get("artifacts")
    if evidence.get("schema") != 1 or not isinstance(rows, list) or len(rows) != 1:
        raise ValueError("publication requires a single-artifact license audit receipt")
    row = rows[0]
    if (
        not isinstance(row, dict)
        or row.get("artifact") != artifact.name
        or row.get("profile") != "npm"
        or row.get("status") != "passed"
        or any(
            type(row.get(field)) is not int or row[field] <= 0
            for field in ("notice_files", "runtime_notice_files")
        )
        or not isinstance(row.get("sha256"), str)
        or re.fullmatch(r"[0-9a-f]{64}", row["sha256"]) is None
    ):
        raise ValueError("missing successful npm base/runtime notice evidence")
    if hashlib.sha256(artifact.read_bytes()).hexdigest() != row["sha256"]:
        raise ValueError("tarball bytes changed after their recipient-notice audit")
    return artifact


def publish(package_dir: Path, receipt: Path) -> None:
    artifact = audited_tarball(package_dir, receipt)
    argv = ["npm", "publish", str(artifact), "--access", "public"]
    environment = os.environ.copy()
    token = environment.get("NPM_TOKEN")
    if token:
        # Bootstrap authentication remains supported without overwriting a user
        # config or writing the token into logs or the configuration file.
        with tempfile.TemporaryDirectory(prefix="purrdf-npm-auth-") as temporary:
            config = Path(temporary) / ".npmrc"
            config.write_text("//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n")
            config.chmod(0o600)
            environment["NODE_AUTH_TOKEN"] = token
            environment["NPM_CONFIG_USERCONFIG"] = str(config)
            subprocess.run(argv, cwd=package_dir, env=environment, check=True)
    else:
        subprocess.run(argv, cwd=package_dir, env=environment, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-dir", type=Path)
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.package_dir or args.receipt or args.check:
            parser.error("--self-test takes no publication arguments")
        import unittest

        spec = importlib.util.spec_from_file_location(
            "npm_publisher_tests", Path(__file__).with_name("test_npm_publisher.py")
        )
        tests = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tests)
        result = unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromModule(tests)
        )
        return 0 if result.wasSuccessful() else 1
    if args.package_dir is None or args.receipt is None:
        parser.error("--package-dir and --receipt are required")
    if args.check:
        audited_tarball(args.package_dir, args.receipt)
        print("Single audited npm tarball verified; no authentication or upload")
    else:
        publish(args.package_dir, args.receipt)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"publish-npm: {error}") from error
