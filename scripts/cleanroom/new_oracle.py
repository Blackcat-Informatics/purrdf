#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Create a clean-room oracle crate from ``scripts/cleanroom/oracle_template``.

    python3 scripts/cleanroom/new_oracle.py <name> --pin <crate>=<version> --lock-ref <commit>

copies the template to ``target/cleanroom-oracle/<name>/``, names the package
``<name>``, and adds ``<crate> = "=<version>"`` as its dependency -- the exact
version, never a range, so the oracle records the answers of one release.

THE PIN MUST ALREADY BE IN A LOCK FILE. The version and its registry checksum are
looked up in ``Cargo.lock`` as of ``--lock-ref`` in the worktree ``--worktree``
(``git -C <worktree> show <lock-ref>:Cargo.lock``), and a pin that lock does not
carry is refused: the oracle exists to record the behaviour of the release the
workspace actually depended on, and a different release answers a different
question. The checksum is written into the generated manifest as a comment, so the
copy records which bytes it was pinned to.

Also refused: a name that is not a plain crate name (``[a-z][a-z0-9_-]*``, so it
cannot escape the destination directory) and a destination that already exists
(an oracle's recorded answers are never silently overwritten).

``--pin`` may be repeated for an oracle that needs several crates.

    python3 scripts/cleanroom/new_oracle.py --self-test
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = Path(__file__).resolve().parent / "oracle_template"
DEFAULT_DEST = REPO_ROOT / "target" / "cleanroom-oracle"
NAME = re.compile(r"^[a-z][a-z0-9_-]{0,63}$")
TEMPLATE_NAME_LINE = 'name = "cleanroom-oracle"'


class OracleError(ValueError):
    """The oracle cannot be created as asked."""


def parse_pin(text: str) -> tuple[str, str]:
    """``crate=version`` split into its halves."""
    crate, sep, version = text.partition("=")
    if not sep or not NAME.match(crate.replace("_", "-").lower()) or not version or version.startswith("="):
        raise OracleError(f"--pin {text!r} is not <crate>=<exact version>")
    return crate, version


def lock_packages(worktree: Path, lock_ref: str) -> list[dict]:
    """The ``[[package]]`` entries of ``Cargo.lock`` at *lock_ref*."""
    completed = subprocess.run(
        ["git", "-C", str(worktree), "show", f"{lock_ref}:Cargo.lock"],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise OracleError(f"cannot read Cargo.lock at {lock_ref!r}: {completed.stderr.strip()}")
    return tomllib.loads(completed.stdout).get("package", [])


def locked_checksum(packages: list[dict], crate: str, version: str) -> str:
    """The registry checksum the lock records for *crate* *version*; refuse otherwise."""
    for package in packages:
        if package.get("name") == crate and package.get("version") == version:
            checksum = package.get("checksum")
            if not isinstance(checksum, str) or not checksum:
                raise OracleError(f"{crate} {version} is in the lock without a registry checksum")
            return checksum
    present = sorted(p.get("version", "?") for p in packages if p.get("name") == crate)
    raise OracleError(
        f"{crate} {version} is not in the lock; it carries "
        + (", ".join(present) if present else f"no {crate} at all")
    )


def create(name: str, pins: list[str], worktree: Path, lock_ref: str, dest_root: Path) -> Path:
    """Create the oracle crate and return its directory."""
    if not NAME.match(name):
        raise OracleError(f"oracle name {name!r} is not [a-z][a-z0-9_-]*")
    if not pins:
        raise OracleError("at least one --pin is required")
    packages = lock_packages(worktree, lock_ref)
    lines = []
    for text in pins:
        crate, version = parse_pin(text)
        checksum = locked_checksum(packages, crate, version)
        lines.append(f"# {crate} {version}: checksum {checksum} in Cargo.lock at {lock_ref}")
        lines.append(f'{crate} = "={version}"')
    destination = dest_root / name
    if destination.exists():
        raise OracleError(f"{destination} already exists; an oracle is never overwritten")
    manifest_text = (TEMPLATE / "Cargo.toml").read_text(encoding="utf-8")
    if manifest_text.count(TEMPLATE_NAME_LINE) != 1 or not manifest_text.rstrip().endswith("[dependencies]"):
        raise OracleError("the template manifest no longer has the shape this program edits")
    dest_root.mkdir(parents=True, exist_ok=True)
    shutil.copytree(TEMPLATE, destination)
    manifest = manifest_text.replace(TEMPLATE_NAME_LINE, f'name = "{name}"')
    manifest = manifest.rstrip() + "\n" + "\n".join(lines) + "\n"
    (destination / "Cargo.toml").write_text(manifest, encoding="utf-8")
    return destination


def self_test() -> int:
    """A pin the lock carries is written; an absent one, a bad name and a re-use are refused."""
    dest_root = REPO_ROOT / "target" / "cleanroom-selftest" / "new_oracle"
    if dest_root.exists():
        shutil.rmtree(dest_root)
    packages = lock_packages(REPO_ROOT, "HEAD")
    registry = next(p for p in packages if p.get("checksum") and p.get("name") == "criterion")
    crate, version = registry["name"], registry["version"]
    ok = True

    made = create("present-pin", [f"{crate}={version}"], REPO_ROOT, "HEAD", dest_root)
    manifest = tomllib.loads((made / "Cargo.toml").read_text(encoding="utf-8"))
    if (
        manifest["package"]["name"] == "present-pin"
        and manifest["dependencies"] == {crate: f"={version}"}
        and manifest["workspace"] == {}
        and (made / "src" / "main.rs").is_file()
    ):
        print(f"OK: self-test — a pin the lock carries ({crate} {version}) is written exactly")
    else:
        print(f"SELF-TEST FAIL: the generated manifest is {manifest}")
        ok = False

    refusals = [
        ("a version the lock does not carry", "absent-pin", [f"{crate}=0.0.1"], "is not in the lock"),
        ("a crate the lock does not carry", "absent-crate", ["no-such-crate-anywhere=1.0.0"], "no no-such-crate"),
        ("a range instead of an exact version", "range-pin", [f"{crate}=={version}"], "exact version"),
        ("a name that escapes the destination", "../escape", [f"{crate}={version}"], "is not [a-z]"),
        ("a destination that already exists", "present-pin", [f"{crate}={version}"], "already exists"),
    ]
    for label, name, pins, needle in refusals:
        try:
            create(name, pins, REPO_ROOT, "HEAD", dest_root)
        except OracleError as error:
            if needle in str(error):
                print(f"OK: self-test — {label} is refused")
                continue
            print(f"SELF-TEST FAIL: {label} refused for the wrong reason: {error}")
        else:
            print(f"SELF-TEST FAIL: {label} was accepted")
        ok = False
    if (dest_root / "absent-pin").exists():
        print("SELF-TEST FAIL: a refused pin still left a directory behind")
        ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("name", nargs="?")
    parser.add_argument("--pin", action="append", default=[], help="<crate>=<exact version>")
    parser.add_argument("--lock-ref", help="the commit whose Cargo.lock must carry every pin")
    parser.add_argument("--worktree", type=Path, default=REPO_ROOT)
    parser.add_argument("--dest-root", type=Path, default=DEFAULT_DEST)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.name is None or args.lock_ref is None:
        parser.error("<name> and --lock-ref are required")
    try:
        made = create(args.name, args.pin, args.worktree, args.lock_ref, args.dest_root)
    except OracleError as error:
        print(f"new_oracle: {error}", file=sys.stderr)
        return 1
    print(made)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
