#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Attach and verify the complete C distribution and lexicon data before publishing a release.

GitHub locks assets when an immutable release is published. All mutations here
therefore address a draft; an existing published release is only read and
verified against its retained checksum manifest, never against a fresh rebuild.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location(
    "release_notes", Path(__file__).with_name("check-release-notes.py")
)
notes_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes_module)


class ReleaseError(ValueError):
    """A release identity, transport or artifact verification failed."""


class GitHub:
    """The authenticated gh transport; only a literal lookup 404 means absent."""

    RELEASE_PAGE_SIZE = 100
    RELEASE_PAGE_LIMIT = 100

    def __init__(self, repository: str):
        self.repository = repository
        self.base = f"repos/{repository}"

    def api(self, path: str, *, body: dict | None = None, absent=False, expected=dict):
        arguments = ["gh", "api", "--include", f"{self.base}/{path}"]
        if body is not None:
            arguments += ["--method", "PATCH", "--input", "-"]
        result = subprocess.run(
            arguments,
            input=None if body is None else json.dumps(body).encode(),
            capture_output=True,
            check=False,
        )
        header, separator, payload = result.stdout.replace(b"\r\n", b"\n").partition(
            b"\n\n"
        )
        status = re.match(rb"HTTP/\S+ ([0-9]{3})\b", header)
        if not status or not separator:
            raise ReleaseError("GitHub returned no complete HTTP response")
        code = int(status[1])
        if absent and body is None and code == 404:
            return None
        if result.returncode or code != 200:
            raise ReleaseError(f"GitHub {path} failed with HTTP {code}")
        try:
            value = json.loads(payload)
        except ValueError as error:
            raise ReleaseError("GitHub returned malformed JSON") from error
        if not isinstance(value, expected):
            raise ReleaseError("GitHub returned an unexpected JSON record shape")
        return value

    def release(self, tag: str):
        # GitHub's tag endpoint only returns published releases. Drafts require
        # the authenticated catalog, followed by an exact numeric-ID lookup.
        published = self.api(f"releases/tags/{tag}", absent=True)
        if published is not None:
            return published
        candidate = None
        for page in range(1, self.RELEASE_PAGE_LIMIT + 1):
            records = self.api(
                f"releases?per_page={self.RELEASE_PAGE_SIZE}&page={page}",
                expected=list,
            )
            if len(records) > self.RELEASE_PAGE_SIZE:
                raise ReleaseError("GitHub release catalog page exceeds its bound")
            for record in records:
                if (
                    not isinstance(record, dict)
                    or type(record.get("id")) is not int
                    or record["id"] <= 0
                    or not isinstance(record.get("tag_name"), str)
                    or not record["tag_name"]
                    or type(record.get("draft")) is not bool
                    or type(record.get("immutable")) is not bool
                ):
                    raise ReleaseError(
                        "GitHub release catalog has a malformed identity"
                    )
                if record["tag_name"] == tag:
                    if candidate is not None:
                        raise ReleaseError(
                            "GitHub release catalog has duplicate exact tags"
                        )
                    candidate = record
            if len(records) < self.RELEASE_PAGE_SIZE:
                break
        else:
            raise ReleaseError("GitHub release catalog pagination bound exhausted")
        if candidate is None:
            return None
        if candidate["draft"] is not True or candidate["immutable"] is not False:
            raise ReleaseError("GitHub catalog tag is not a mutable draft")
        draft = self.api(f"releases/{candidate['id']}")
        if (
            type(draft.get("id")) is not int
            or draft["id"] != candidate["id"]
            or draft.get("tag_name") != tag
            or draft.get("draft") is not True
            or draft.get("immutable") is not False
        ):
            raise ReleaseError("GitHub draft identity changed after catalog lookup")
        return draft

    def tag_sha(self, tag: str) -> str:
        reference = self.api(f"git/ref/tags/{tag}")
        target = reference.get("object", {})
        for _ in range(16):
            if not isinstance(target, dict):
                break
            if target.get("type") == "commit":
                return target.get("sha", "")
            if target.get("type") != "tag":
                break
            target = self.api(f"git/tags/{target.get('sha', '')}").get("object", {})
        raise ReleaseError("release tag does not resolve to a commit")

    def create_draft(self, tag: str, title: str, notes: Path):
        subprocess.run(
            [
                "gh",
                "release",
                "create",
                tag,
                "--repo",
                self.repository,
                "--verify-tag",
                "--draft",
                "--title",
                title,
                "--notes-file",
                str(notes),
            ],
            check=True,
        )

    def upload(self, tag: str, paths: list[Path]):
        subprocess.run(
            [
                "gh",
                "release",
                "upload",
                tag,
                "--repo",
                self.repository,
                *map(str, paths),
                "--clobber",
            ],
            check=True,
        )

    def edit(self, identity: int, **fields):
        return self.api(f"releases/{identity}", body=fields)

    def download(self, asset: dict, path: Path):
        with path.open("wb") as destination:
            result = subprocess.run(
                [
                    "gh",
                    "api",
                    f"{self.base}/releases/assets/{asset['id']}",
                    "-H",
                    "Accept: application/octet-stream",
                ],
                stdout=destination,
                stderr=subprocess.PIPE,
                check=False,
            )
        if result.returncode:
            raise ReleaseError(f"GitHub could not download {asset['name']}")


def asset_names(version: str) -> set[str]:
    stem = f"purrdf-capi-{version}-x86_64-unknown-linux-gnu"
    return {
        stem + ".tar.gz",
        stem + ".license-receipt.json",
        "license-evidence-cargo.json",
        f"purrdf-text-lexicons-{version}.tar.gz",
        "SHA256SUMS",
    }


def identity(release: dict, tag: str, title: str, notes: bytes, *, draft: bool):
    if (
        not isinstance(release, dict)
        or type(release.get("id")) is not int
        or release.get("id", 0) <= 0
        or release.get("tag_name") != tag
        or release.get("name") != title
        or release.get("body") != notes.decode("utf-8")
        or release.get("draft") is not draft
        or release.get("prerelease") is not False
    ):
        raise ReleaseError(
            "GitHub release metadata differs from the reviewed tagged release"
        )
    if not draft and release.get("immutable") is not True:
        raise ReleaseError("published release is not immutable")
    if draft and release.get("immutable") is not False:
        raise ReleaseError("draft release is already immutable")


def verify_assets(
    github: GitHub, release: dict, version: str, expected: dict[str, str] | None = None
):
    """Download every asset, then bind all bytes to the retained strict manifest."""
    assets = release.get("assets")
    names = asset_names(version)
    if not isinstance(assets, list) or len(assets) != len(names):
        raise ReleaseError("release needs exactly five C, lexicon and checksum assets")
    by_name = {asset.get("name"): asset for asset in assets if isinstance(asset, dict)}
    if set(by_name) != names:
        raise ReleaseError(
            "release asset names differ from the complete C and lexicon distribution"
        )
    hashes = {}
    manifest = None
    with tempfile.TemporaryDirectory(prefix="purrdf-release-verify-") as temporary:
        for name, asset in sorted(by_name.items()):
            if (
                type(asset.get("id")) is not int
                or asset.get("id", 0) <= 0
                or type(asset.get("size")) is not int
                or asset.get("size", 0) <= 0
                or asset.get("state") != "uploaded"
            ):
                raise ReleaseError(f"release asset is incomplete: {name}")
            path = Path(temporary) / name
            github.download(asset, path)
            if path.stat().st_size != asset["size"]:
                raise ReleaseError(f"downloaded size differs for {name}")
            with path.open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            if asset.get("digest") != f"sha256:{digest}":
                raise ReleaseError(f"GitHub asset digest differs for {name}")
            hashes[name] = digest
            if name == "SHA256SUMS":
                manifest = path.read_bytes()
    try:
        checksums = {}
        for line in manifest.decode("ascii").splitlines():
            match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.+-]+)", line)
            if not match or match[2] in checksums:
                raise ReleaseError(
                    "release checksum manifest is malformed or duplicated"
                )
            checksums[match[2]] = match[1]
    except UnicodeError as error:
        raise ReleaseError("release checksum manifest is not ASCII") from error
    if checksums != {
        name: digest for name, digest in hashes.items() if name != "SHA256SUMS"
    }:
        raise ReleaseError(
            "retained release checksums do not match the four distribution assets"
        )
    if expected is not None and hashes != expected:
        raise ReleaseError("draft assets differ from the audited local distribution")


def publish(root: Path, github: GitHub, version: str, sha: str):
    version, notes = notes_module.release_notes(root, version)
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ReleaseError("expected tagged commit must be a complete Git SHA")
    tag, title = f"rust-v{version}", f"PurRDF {version}"
    if github.tag_sha(tag) != sha:
        raise ReleaseError("remote release tag differs from the workflow commit")
    release = github.release(tag)
    if release is not None and release.get("draft") is False:
        identity(release, tag, title, notes, draft=False)
        verify_assets(github, release, version)
        print(f"OK: existing immutable {tag} and all five retained assets verified")
        return
    if release is not None and release.get("draft") is not True:
        raise ReleaseError("existing release has no valid draft verdict")
    if release is not None and (
        type(release.get("id")) is not int
        or release.get("id", 0) <= 0
        or release.get("tag_name") != tag
        or release.get("immutable") is not False
    ):
        raise ReleaseError("existing draft has a different release identity")

    # Refuse a partial build before creating or editing anything on GitHub.
    manifest = notes_module.checksum_bytes(root, version)
    manifest_path = root / "target/dist/SHA256SUMS"
    notes_module.write_output(manifest_path, manifest)
    notes_path = root / "target/release-notes.md"
    notes_module.write_output(notes_path, notes)
    paths = []
    for name in sorted(asset_names(version)):
        directory = (
            root / "target"
            if name == "license-evidence-cargo.json"
            else root / "target/dist"
        )
        path = directory / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ReleaseError(
                f"audited release input missing, empty or a symlink: {name}"
            )
        paths.append(path)
    expected = {}
    for path in paths:
        with path.open("rb") as source:
            expected[path.name] = hashlib.file_digest(source, "sha256").hexdigest()
    # Reassert the complete reviewed identity on every draft transition. An
    # omitted tag_name can clear GitHub's selected tag even during publication.
    reviewed = {
        "tag_name": tag,
        "name": title,
        "body": notes.decode("utf-8"),
        "prerelease": False,
    }
    if release is None:
        github.create_draft(tag, title, notes_path)
    else:
        github.edit(release["id"], **reviewed, draft=True)
    release = github.release(tag)
    identity(release, tag, title, notes, draft=True)
    github.upload(tag, paths)
    release = github.release(tag)
    identity(release, tag, title, notes, draft=True)
    verify_assets(github, release, version, expected)
    if github.tag_sha(tag) != sha:
        raise ReleaseError("release tag moved before publication")
    github.edit(release["id"], **reviewed, draft=False)
    release = github.release(tag)
    identity(release, tag, title, notes, draft=False)
    verify_assets(github, release, version, expected)
    print(f"OK: immutable {tag} published with all five verified assets")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--version")
    parser.add_argument("--sha")
    parser.add_argument("--repository")
    args = parser.parse_args()
    if args.self_test:
        test_spec = importlib.util.spec_from_file_location(
            "github_release_tests", Path(__file__).with_name("test_github_release.py")
        )
        tests = importlib.util.module_from_spec(test_spec)
        test_spec.loader.exec_module(tests)
        return (
            0
            if unittest.TextTestRunner()
            .run(unittest.defaultTestLoader.loadTestsFromModule(tests))
            .wasSuccessful()
            else 1
        )
    if not args.version or not args.sha or not args.repository:
        parser.error("publication requires --version, --sha and --repository")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository):
        parser.error("invalid GitHub repository")
    try:
        publish(ROOT, GitHub(args.repository), args.version, args.sha)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"GitHub release refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
