# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Offline state and transport regressions for immutable GitHub publication."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "github_publisher", Path(__file__).with_name("publish-github-release.py")
)
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)
VERSION = "3.0.1"
SHA = "a" * 40


class FakeGitHub:
    """No transport: a draft/published store retaining the uploaded bytes."""

    def __init__(self):
        self.record = None
        self.bytes = {}
        self.events = []
        self.sha = SHA
        self.fail_at = None

    def event(self, name):
        self.events.append(name)
        if self.fail_at == name:
            raise publisher.ReleaseError(f"injected {name} failure")

    def tag_sha(self, tag):
        self.event("tag")
        return self.sha

    def release(self, tag):
        self.event("read")
        return copy.deepcopy(self.record)

    def create_draft(self, tag, title, notes):
        self.event("create")
        self.record = {
            "id": 12,
            "tag_name": tag,
            "name": title,
            "body": notes.read_text(),
            "draft": True,
            "immutable": False,
            "prerelease": False,
            "assets": [],
        }

    def edit(self, identity, **fields):
        self.event("publish" if fields.get("draft") is False else "edit")
        self.record.update(fields)
        if fields.get("draft") is False:
            self.record["immutable"] = True

    def upload(self, tag, paths):
        self.event("upload")
        if not self.record["draft"]:
            raise AssertionError("upload was attempted on a published release")
        self.bytes = {path.name: path.read_bytes() for path in paths}
        self.refresh_assets()

    def refresh_assets(self):
        self.record["assets"] = [
            {
                "id": i + 1,
                "name": name,
                "size": len(data),
                "state": "uploaded",
                "digest": "sha256:" + hashlib.sha256(data).hexdigest(),
            }
            for i, (name, data) in enumerate(sorted(self.bytes.items()))
        ]

    def download(self, asset, path):
        self.event("download")
        path.write_bytes(self.bytes[asset["name"]])


class PublicationTests(unittest.TestCase):
    def setUp(self):
        silence = patch.object(publisher, "print", create=True)
        silence.start()
        self.addCleanup(silence.stop)
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "Cargo.toml").write_text(
            '[workspace.package]\nversion = "3.0.1"\n'
        )
        (self.root / "CHANGELOG.md").write_text("## [3.0.1]\n\nReviewed changes.\n")
        notes = self.root / "docs/releases/3.0.1.md"
        notes.parent.mkdir(parents=True)
        notes.write_text("# PurRDF 3.0.1\n\nComplete reviewed release.\n")
        self.inputs = []
        for name in publisher.asset_names(VERSION) - {"SHA256SUMS"}:
            directory = self.root / (
                "target" if name == "license-evidence-cargo.json" else "target/dist"
            )
            directory.mkdir(parents=True, exist_ok=True)
            path = directory / name
            path.write_bytes((name + " audited content").encode())
            self.inputs.append(path)
        self.github = FakeGitHub()

    def publish(self):
        publisher.publish(self.root, self.github, VERSION, SHA)

    def published(self):
        self.publish()
        self.github.events.clear()

    def assert_no_publication(self):
        self.assertNotIn("publish", self.github.events)

    def test_new_release_is_verified_as_draft_before_publication(self):
        self.publish()
        self.assertTrue(self.github.record["immutable"])
        events = self.github.events
        self.assertLess(events.index("create"), events.index("upload"))
        self.assertLess(events.index("upload"), events.index("download"))
        self.assertLess(events.index("download"), events.index("publish"))
        self.assertEqual(events.count("download"), 8)

    def test_existing_draft_is_reused_after_failed_upload(self):
        self.github.fail_at = "upload"
        with self.assertRaises(publisher.ReleaseError):
            self.publish()
        self.assert_no_publication()
        self.github.fail_at = None
        self.github.events.clear()
        self.publish()
        self.assertNotIn("create", self.github.events)
        self.assertIn("edit", self.github.events)
        self.assertFalse(self.github.record["draft"])

    def test_complete_immutable_rerun_only_reads_retained_bytes(self):
        self.published()
        for path in self.inputs:
            path.unlink()
        with patch.object(
            publisher.notes_module,
            "checksum_bytes",
            side_effect=AssertionError("rebuilt"),
        ):
            self.publish()
        self.assertEqual(set(self.github.events), {"tag", "read", "download"})

    def test_missing_immutable_assets_fail_without_any_mutation(self):
        self.published()
        self.github.record["assets"] = []
        with self.assertRaisesRegex(publisher.ReleaseError, "exactly four"):
            self.publish()
        self.assertEqual(set(self.github.events), {"tag", "read"})

    def test_transport_and_upload_failures_never_publish(self):
        for event in ("tag", "read", "create", "upload", "download"):
            with self.subTest(event=event):
                self.github = FakeGitHub()
                self.github.fail_at = event
                with self.assertRaises(publisher.ReleaseError):
                    self.publish()
                self.assert_no_publication()

    def test_missing_local_input_refuses_before_draft_creation(self):
        self.inputs[0].unlink()
        with self.assertRaises(ValueError):
            self.publish()
        self.assertEqual(self.github.events, ["tag", "read"])

    def test_wrong_tag_commit_refuses_before_release_lookup(self):
        self.github.sha = "b" * 40
        with self.assertRaisesRegex(publisher.ReleaseError, "workflow commit"):
            self.publish()
        self.assertEqual(self.github.events, ["tag"])

    def test_tag_movement_after_upload_never_publishes(self):
        original = self.github.tag_sha

        def moving(tag):
            sha = original(tag)
            return sha if self.github.events.count("tag") == 1 else "b" * 40

        self.github.tag_sha = moving
        with self.assertRaisesRegex(publisher.ReleaseError, "moved"):
            self.publish()
        self.assert_no_publication()

    def test_published_notes_are_verified_without_editing_them(self):
        self.published()
        self.github.record["body"] = "different notes"
        with self.assertRaisesRegex(publisher.ReleaseError, "metadata"):
            self.publish()
        self.assertEqual(set(self.github.events), {"tag", "read"})

    def test_draft_metadata_is_repaired_before_upload(self):
        self.github.fail_at = "upload"
        with self.assertRaises(publisher.ReleaseError):
            self.publish()
        self.github.fail_at = None
        self.github.record["body"] = "incomplete notes"
        self.github.record["prerelease"] = True
        self.publish()
        self.assertEqual(
            self.github.record["body"], "# PurRDF 3.0.1\n\nComplete reviewed release.\n"
        )
        self.assertFalse(self.github.record["prerelease"])

    def test_invalid_asset_records_and_digests_are_refused(self):
        self.published()
        control = copy.deepcopy(self.github.record)
        for field, value in (
            ("size", 0),
            ("size", 1),
            ("state", "new"),
            ("digest", "sha256:" + "0" * 64),
        ):
            with self.subTest(field=field, value=value):
                self.github.record = copy.deepcopy(control)
                self.github.record["assets"][0][field] = value
                with self.assertRaises(publisher.ReleaseError):
                    self.publish()
                self.assert_no_publication()

    def test_corrupted_upload_refuses_before_publish(self):
        original = self.github.upload

        def corrupt(tag, paths):
            original(tag, paths)
            name = next(name for name in self.github.bytes if name.endswith(".tar.gz"))
            self.github.bytes[name] += b"corruption"
            self.github.refresh_assets()

        self.github.upload = corrupt
        with self.assertRaisesRegex(publisher.ReleaseError, "checksums"):
            self.publish()
        self.assert_no_publication()

    def test_manifest_cannot_drop_duplicate_or_forge_assets(self):
        self.published()
        original = self.github.bytes["SHA256SUMS"]
        variants = [
            b"",
            original.splitlines(keepends=True)[0],
            original + original,
            b"0" * 64 + b"  ../other\n",
            b"\xff",
            original.replace(b"  ", b" "),
        ]
        for value in variants:
            with self.subTest(value=value):
                self.github.bytes["SHA256SUMS"] = value
                self.github.refresh_assets()
                with self.assertRaises(publisher.ReleaseError):
                    self.publish()
                self.assert_no_publication()

    def test_different_valid_rebuilt_manifest_is_not_admitted_as_draft(self):
        original = self.github.upload

        def change_all(tag, paths):
            original(tag, paths)
            for name in self.github.bytes.keys() - {"SHA256SUMS"}:
                self.github.bytes[name] += b"other valid build"
            self.github.bytes["SHA256SUMS"] = b"".join(
                f"{hashlib.sha256(data).hexdigest()}  {name}\n".encode()
                for name, data in sorted(self.github.bytes.items())
                if name != "SHA256SUMS"
            )
            self.github.refresh_assets()

        self.github.upload = change_all
        with self.assertRaisesRegex(publisher.ReleaseError, "audited local"):
            self.publish()
        self.assert_no_publication()


class TransportTests(unittest.TestCase):
    def response(self, code, body, returncode=0):
        return subprocess.CompletedProcess(
            [],
            returncode,
            f"HTTP/2.0 {code} verdict\r\nContent-Type: application/json\r\n\r\n{body}".encode(),
            b"",
        )

    def test_only_404_authorizes_absent_release(self):
        github = publisher.GitHub("Blackcat-Informatics/purrdf")
        with patch.object(
            publisher.subprocess, "run", return_value=self.response(404, "{}", 1)
        ):
            self.assertIsNone(github.release("rust-v3.0.1"))
            with self.assertRaises(publisher.ReleaseError):
                github.tag_sha("rust-v3.0.1")
        for code in (401, 403, 422, 429, 500, 503):
            with (
                self.subTest(code=code),
                patch.object(
                    publisher.subprocess,
                    "run",
                    return_value=self.response(code, "{}", 1),
                ),
                self.assertRaises(publisher.ReleaseError),
            ):
                github.release("rust-v3.0.1")

    def test_bad_transport_and_json_are_not_absence(self):
        for result in (
            subprocess.CompletedProcess([], 1, b"", b"network down"),
            self.response(200, "not JSON"),
            self.response(200, "[]"),
        ):
            with (
                patch.object(publisher.subprocess, "run", return_value=result),
                self.assertRaises(publisher.ReleaseError),
            ):
                publisher.GitHub("owner/repo").release("rust-v3.0.1")

    def test_annotated_and_lightweight_tags_resolve_exact_commit(self):
        github = publisher.GitHub("owner/repo")
        with patch.object(
            github, "api", return_value={"object": {"type": "commit", "sha": SHA}}
        ):
            self.assertEqual(github.tag_sha("rust-v3.0.1"), SHA)
        with patch.object(
            github,
            "api",
            side_effect=[
                {"object": {"type": "tag", "sha": "b" * 40}},
                {"object": {"type": "commit", "sha": SHA}},
            ],
        ):
            self.assertEqual(github.tag_sha("rust-v3.0.1"), SHA)

    def test_create_always_uses_draft_and_existing_tag(self):
        with patch.object(publisher.subprocess, "run") as run:
            publisher.GitHub("owner/repo").create_draft(
                "rust-v3.0.1", "PurRDF 3.0.1", Path("notes.md")
            )
        command = run.call_args.args[0]
        self.assertIn("--draft", command)
        self.assertIn("--verify-tag", command)

    def test_workflow_uses_single_verified_publisher(self):
        workflow = (
            Path(__file__).resolve().parent.parent
            / ".github/workflows/release-cargo.yaml"
        ).read_text()
        self.assertIn("python3 scripts/publish-github-release.py", workflow)
        self.assertIn('--sha "$GITHUB_SHA"', workflow)
        self.assertNotIn("gh release create", workflow)
        self.assertNotIn("gh release upload", workflow)
