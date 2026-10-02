#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Offline publication regressions: real artifact selection, mocked npm transport."""

import hashlib
import importlib.util
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "tested_npm_publisher", Path(__file__).with_name("publish-npm.py")
)
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.package = self.root / "sdk"
        self.package.mkdir()
        self.metadata = {"name": "@blackcatinformatics/purrdf", "version": "3.0.0"}
        (self.package / "package.json").write_text(json.dumps(self.metadata))
        self.artifact = self.package / "blackcatinformatics-purrdf-3.0.0.tgz"
        with tarfile.open(self.artifact, "w:gz") as archive:
            for name, value in {
                "package/package.json": json.dumps(self.metadata).encode(),
                "package/licenses/LICENSE-MIT": b"retained controlling grant",
                "package/licenses/runtime/licenses/MIT.txt": b"compiler runtime grant",
            }.items():
                entry = tarfile.TarInfo(name)
                entry.size = len(value)
                archive.addfile(entry, io.BytesIO(value))
        self.row = {
            "artifact": self.artifact.name,
            "profile": "npm",
            "status": "passed",
            "notice_files": 139,
            "runtime_notice_files": 14,
            "sha256": hashlib.sha256(self.artifact.read_bytes()).hexdigest(),
        }
        self.receipt = self.root / "license-evidence-npm.json"
        self.write_receipt()

    def write_receipt(self, rows=None, schema=1):
        self.receipt.write_text(
            json.dumps(
                {"schema": schema, "artifacts": [self.row] if rows is None else rows}
            )
        )

    def assert_refused(self):
        with patch.object(publisher.subprocess, "run") as transport:
            with self.assertRaises((ValueError, OSError)):
                publisher.publish(self.package, self.receipt)
            transport.assert_not_called()

    def test_oidc_publishes_audited_tar_instead_of_directory(self):
        # The source tree deliberately lacks the runtime grant. Publication must
        # consume the audited tar, whose runtime notice was added after packing.
        with (
            patch.dict(publisher.os.environ, {"NPM_TOKEN": ""}, clear=True),
            patch.object(publisher.subprocess, "run") as transport,
        ):
            publisher.publish(self.package, self.receipt)
        self.assertEqual(
            transport.call_args.args[0],
            ["npm", "publish", str(self.artifact), "--access", "public"],
        )
        self.assertEqual(transport.call_args.kwargs["cwd"], self.package)
        self.assertNotIn("NPM_CONFIG_USERCONFIG", transport.call_args.kwargs["env"])

    def test_token_publishes_same_tar_and_removes_private_auth_config(self):
        observed = {}

        def upload(argv, **kwargs):
            config = Path(kwargs["env"]["NPM_CONFIG_USERCONFIG"])
            observed.update(argv=argv, config=config, text=config.read_text())
            self.assertEqual(kwargs["env"]["NODE_AUTH_TOKEN"], "synthetic-bootstrap")
            self.assertEqual(config.stat().st_mode & 0o777, 0o600)

        with (
            patch.dict(
                publisher.os.environ, {"NPM_TOKEN": "synthetic-bootstrap"}, clear=True
            ),
            patch.object(publisher.subprocess, "run", side_effect=upload),
        ):
            publisher.publish(self.package, self.receipt)
        self.assertEqual(observed["argv"][2], str(self.artifact))
        self.assertNotIn("synthetic-bootstrap", observed["text"])
        self.assertFalse(observed["config"].exists())

    def test_failed_upload_preserves_tar_and_cleans_auth_config(self):
        observed = {}

        def upload(argv, **kwargs):
            observed["config"] = Path(kwargs["env"]["NPM_CONFIG_USERCONFIG"])
            raise publisher.subprocess.CalledProcessError(1, argv)

        with (
            patch.dict(publisher.os.environ, {"NPM_TOKEN": "synthetic"}, clear=True),
            patch.object(publisher.subprocess, "run", side_effect=upload),
            self.assertRaises(publisher.subprocess.CalledProcessError),
        ):
            publisher.publish(self.package, self.receipt)
        self.assertFalse(observed["config"].exists())
        self.assertEqual(
            hashlib.sha256(self.artifact.read_bytes()).hexdigest(), self.row["sha256"]
        )

    def test_missing_tar_refuses_without_authentication(self):
        self.artifact.unlink()
        self.assert_refused()

    def test_multiple_tarballs_refuse_even_when_one_is_audited(self):
        (self.package / "old-release.tgz").write_bytes(b"other artifact")
        self.assert_refused()

    def test_symlink_tar_refuses(self):
        actual = self.root / self.artifact.name
        self.artifact.rename(actual)
        self.artifact.symlink_to(actual)
        self.assert_refused()

    def test_tar_changed_after_audit_refuses(self):
        self.artifact.write_bytes(self.artifact.read_bytes() + b"modified")
        self.assert_refused()

    def test_wrong_receipt_identity_and_incomplete_runtime_evidence_refuse(self):
        for field, value in (
            ("artifact", "other.tgz"),
            ("profile", "python"),
            ("status", "failed"),
            ("runtime_notice_files", 0),
            ("runtime_notice_files", True),
            ("notice_files", 0),
            ("sha256", "not a digest"),
        ):
            with self.subTest(field=field, value=value):
                original = self.row[field]
                self.row[field] = value
                self.write_receipt()
                self.assert_refused()
                self.row[field] = original

    def test_empty_or_multiple_audit_rows_refuse(self):
        for rows in ([], [self.row, self.row]):
            with self.subTest(rows=len(rows)):
                self.write_receipt(rows=rows)
                self.assert_refused()

    def test_stale_package_version_refuses(self):
        self.metadata["version"] = "2.0.2"
        (self.package / "package.json").write_text(json.dumps(self.metadata))
        self.assert_refused()

    def test_release_workflow_uses_same_receipt_for_check_and_publish(self):
        workflow = (
            Path(__file__).resolve().parents[1] / ".github/workflows/release-npm.yaml"
        ).read_text()
        command = (
            "python3 scripts/publish-npm.py --package-dir crates/rdf-wasm/js "
            "--receipt license-evidence-npm.json"
        )
        self.assertEqual(workflow.count(command), 2)
        self.assertIn(command + " --check", workflow)
        self.assertNotIn("npm publish --access public", workflow)


if __name__ == "__main__":
    unittest.main()
