#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Prove CI partitions workspace targets without changing dependency features.

The Makefile's explicit workspace commands are checked, together with Cargo's
actual target inventory. Unsupported test-enabled benches or examples refuse the
partition rather than silently losing coverage. Self-tests exercise wiring and
character-boundary refusals and valid neighbours.
"""
import argparse
import fnmatch
import importlib.util
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("test_shards", ROOT / "scripts/test-shards.py")
SHARDS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SHARDS)


def wiring_problems(make: str, workflow: str) -> list[str]:
    errors = []
    block = re.search(r"^test-shard:.*?(?=^[a-z][^\n]*:|\Z)", make, re.M | re.S)
    if not block:
        return ["Makefile has no test-shard target"]
    live = block[0]
    for command in (
        "cargo test --workspace --exclude purrdf-python --locked --lib --bins",
        "cargo test --workspace --locked --doc",
        "cargo build --workspace --locked --examples --profile test",
        "cargo test --workspace --locked --example graphql_oracle_fixture --example typescript_oracle_fixture",
        *(f"cargo test --workspace --locked --test '{pattern}'" for pattern in SHARDS.PATTERNS),
    ):
        if command not in live:
            errors.append(f"test-shard lacks {command}")
    job = re.search(r"^  test-shard:.*?(?=^  \S|\Z)", workflow, re.M | re.S)
    if not job:
        return errors + ["CI has no test-shard job"]
    for needle in ("fail-fast: false", "shard: [" + ", ".join(SHARDS.SHARDS) + "]", "make test-shard SHARD=${{ matrix.shard }}"):
        if needle not in job[0]:
            errors.append(f"CI test-shard lacks {needle}")
    return errors


def target_problems(metadata: dict) -> list[str]:
    errors = []
    members = set(metadata["workspace_members"])
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            kinds = target["kind"]
            if target.get("test") and ("bench" in kinds or ("example" in kinds and target["name"] not in {"graphql_oracle_fixture", "typescript_oracle_fixture"})):
                errors.append(f"{package['name']}::{target['name']} enables tests in {kinds}; extend the shards")
            if "test" in kinds:
                matches = [pattern for pattern in SHARDS.PATTERNS if fnmatch.fnmatchcase(target["name"], pattern)]
                if len(matches) != 1:
                    errors.append(f"{target['name']} belongs to {len(matches)} integration shards")
            if package["name"] == "purrdf-python" and "cdylib" in kinds and target.get("test"):
                errors.append("the excluded Python library now enables tests")
    return errors


def self_test() -> None:
    for first in (chr(i) for i in range(128)):
        name = first + "case"
        matches = [i for i, pattern in enumerate(SHARDS.PATTERNS, 1) if fnmatch.fnmatchcase(name, pattern)]
        assert len(matches) == 1
        assert SHARDS.integration_shard(name) == f"integration-{matches[0]}"
    make = (ROOT / "Makefile").read_text()
    ci = (ROOT / ".github/workflows/ci.yaml").read_text()
    assert not wiring_problems(make, ci)
    assert wiring_problems(make.replace("--workspace", "-p purrdf"), ci)
    assert wiring_problems(make, ci.replace("fail-fast: false", "fail-fast: true"))
    assert wiring_problems(make.replace("'[a-d]*'", "'[b-d]*'"), ci)
    fixture = {"workspace_members": ["x"], "packages": [{"id": "x", "name": "x", "targets": [{"name": "example", "kind": ["example"], "test": True}]}]}
    assert target_problems(fixture)
    fixture["packages"][0]["targets"][0]["test"] = False
    assert not target_problems(fixture)
    print("OK: workspace shard wiring, all ASCII boundaries and unsupported-target refusals")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], cwd=ROOT))
        errors = wiring_problems((ROOT / "Makefile").read_text(), (ROOT / ".github/workflows/ci.yaml").read_text()) + target_problems(metadata)
        if errors:
            raise SystemExit("FAIL: " + "\n".join(errors))
        print(f"OK: six feature-unified shards cover {len(metadata['workspace_members'])} workspace members")
