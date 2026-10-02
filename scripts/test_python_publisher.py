# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Offline publisher regressions: real Twine CLI, mocked identity/signing/HTTP."""

import base64
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import keyring
import requests
from keyring.backend import KeyringBackend
from keyring.backends.null import Keyring
from pydantic import ValidationError
from pypi_attestations import Attestation
from sigstore import oidc
from sigstore.models import ClientTrustConfig
from sigstore.sign import SigningContext

spec = importlib.util.spec_from_file_location(
    "tested_python_publisher", Path(__file__).with_name("publish-python.py")
)
publisher = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = publisher
spec.loader.exec_module(publisher)


def metadata(project="purrdf", version="3.0.0"):
    return (
        f"Metadata-Version: 2.5\nName: {project}\nVersion: {version}\n"
        "Summary: Publisher test fixture\nLicense-Expression: MIT\n"
        "Description-Content-Type: text/markdown\n\nFixture description.\n"
    ).encode()


def distributions(directory, project="purrdf", content=None):
    directory.mkdir()
    spelling = project.replace("-", "_")
    content = metadata(project) if content is None else content
    wheel = directory / f"{spelling}-3.0.0-py3-none-any.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr(f"{spelling}-3.0.0.dist-info/METADATA", content)
        archive.writestr(
            f"{spelling}-3.0.0.dist-info/WHEEL",
            "Wheel-Version: 1.0\nGenerator: offline-test\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
        )
    sdist = directory / f"{spelling}-3.0.0.tar.gz"
    with tarfile.open(sdist, "w:gz") as archive:
        root = tarfile.TarInfo(f"{spelling}-3.0.0")
        root.type = tarfile.DIRTYPE
        archive.addfile(root)
        info = tarfile.TarInfo(f"{spelling}-3.0.0/PKG-INFO")
        info.size = len(content)
        archive.addfile(info, io.BytesIO(content))
    return wheel, sdist


def github_environment():
    return {
        "GITHUB_ACTIONS": "true",
        "GITHUB_REPOSITORY": publisher.REPOSITORY,
        "GITHUB_REF": "refs/tags/py-v3.0.0",
        "GITHUB_WORKFLOW_REF": publisher.workflow_identity("3.0.0").removeprefix(
            "https://github.com/"
        ),
    }


def identity_token(**changes):
    now = int(time.time())
    claims = {
        "iss": publisher.GITHUB_ISSUER,
        "sub": f"repo:{publisher.REPOSITORY}:ref:refs/tags/py-v3.0.0",
        "aud": "sigstore",
        "iat": now,
        "exp": now + 600,
    }
    claims.update(changes)

    def encode(value):
        return (
            base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b"=").decode()
        )

    return f"{encode({'alg': 'RS256'})}.{encode(claims)}.dGVzdA"


class FakeAttestation:
    """A declared signing mock; this provides no cryptographic qualification."""

    def __init__(self, distribution, predicate=publisher.PUBLISH_PREDICATE):
        self.name = distribution.name
        self.digest = distribution.digest
        self.predicate = predicate

    def verify(self, policy, distribution, offline=False):
        if (
            not offline
            or self.name != distribution.name
            or self.digest != distribution.digest
        ):
            raise ValueError("mock attestation subject mismatch")
        if (
            policy._identity != publisher.workflow_identity("3.0.0")
            or policy._issuer._value != publisher.GITHUB_ISSUER
        ):
            raise ValueError("mock attestation identity mismatch")
        return self.predicate, {}

    def model_dump_json(self):
        return json.dumps(
            {"subject": self.name, "digest": self.digest, "predicate": self.predicate}
        )

    @classmethod
    def parse(cls, raw):
        value = json.loads(raw)
        return cls(
            SimpleNamespace(name=value["subject"], digest=value["digest"]),
            value["predicate"],
        )


class Transport:
    """Intercept every Requests request, preserving real Twine preprocessing."""

    def __init__(self, status=200, existing=False, metadata_versions=None):
        self.status = status
        self.existing = existing
        self.metadata_versions = metadata_versions
        self.calls = []
        self.uploads = []

    def request(self, session, method, url, **kwargs):
        self.calls.append((method, url))
        payload, status = {}, 200
        if method == "GET" and url == "https://upload.pypi.org/_/oidc/audience":
            payload = {"audience": "pypi"}
        elif method == "POST" and url == "https://upload.pypi.org/_/oidc/mint-token":
            if kwargs["json"] != {"token": "mock-pypi-identity"}:
                raise AssertionError("unexpected token exchange input")
            payload = {"token": "mock-upload-token", "success": True}
        elif method == "GET" and url.startswith(
            ("https://pypi.org/pypi/", "https://pypi.python.org/pypi/")
        ):
            payload = {"releases": {}}
            if self.existing:
                spelling = "purrdf_rdflib" if "purrdf-rdflib" in url else "purrdf"
                payload["releases"] = {
                    "3.0.0": [
                        {"filename": f"{spelling}-3.0.0-py3-none-any.whl"},
                        {"filename": f"{spelling}-3.0.0.tar.gz"},
                    ]
                }
        elif method == "POST" and url == publisher.PYPI:
            if session.auth != ("__token__", "mock-upload-token"):
                raise AssertionError(
                    "upload did not use the native Trusted Publishing token"
                )
            fields = dict(kwargs["data"].encoder.fields)
            filename, stream, _ = fields["content"]
            attestations = json.loads(fields["attestations"])
            digest = hashlib.sha256(stream.read()).hexdigest()
            if (
                len(attestations) != 1
                or attestations[0]["subject"] != filename
                or attestations[0]["digest"] != digest
            ):
                raise AssertionError("uploaded bytes differ from adjacent attestation")
            expected = (
                "2.5"
                if self.metadata_versions is None
                else self.metadata_versions[filename]
            )
            if fields["metadata_version"] != expected:
                raise AssertionError("upload preprocessing changed Metadata-Version")
            self.uploads.append(
                {
                    "filename": filename,
                    "sha256": digest,
                    "metadata_version": fields["metadata_version"],
                    "attestations": attestations,
                }
            )
            status = self.status
        else:
            raise AssertionError(f"unmocked HTTP request: {method} {url}")
        response = requests.Response()
        response.status_code, response.url = status, url
        response.reason = "already exists" if status == 400 else "mock response"
        response._content = json.dumps(payload).encode()
        return response


class PublisherTests(unittest.TestCase):
    def setUp(self):
        self.real_attestation_parser = Attestation.model_validate_json
        self.temp = tempfile.TemporaryDirectory(prefix="purrdf-publisher-tests-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.directory = self.root / "dist"
        distributions(self.directory)
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(
            mock.patch.dict(os.environ, github_environment(), clear=True)
        )
        # Every test begins with a network denial. Only declared fake routes
        # below replace it; neither live OIDC nor signing/upload is possible.
        self.stack.enter_context(
            mock.patch.object(
                requests.sessions.Session,
                "request",
                side_effect=AssertionError("network forbidden in offline test"),
            )
        )

    def mocked_publication(
        self,
        token=None,
        status=200,
        existing=False,
        sign=None,
        parse=None,
        metadata_versions=None,
    ):
        transport = Transport(status, existing, metadata_versions)
        audiences = []

        def credential(audience):
            audiences.append(audience)
            return (
                (identity_token() if token is None else token)
                if audience == "sigstore"
                else "mock-pypi-identity"
            )

        self.stack.enter_context(
            mock.patch.object(oidc.id, "detect_credential", side_effect=credential)
        )
        self.stack.enter_context(
            mock.patch("twine.auth.detect_credential", side_effect=credential)
        )
        self.stack.enter_context(
            mock.patch.object(
                ClientTrustConfig,
                "production",
                return_value=SimpleNamespace(force_tlog_version=None),
            )
        )
        self.stack.enter_context(
            mock.patch.object(
                SigningContext,
                "from_trust_config",
                return_value=SimpleNamespace(
                    signer=lambda *_args, **_kwargs: contextlib.nullcontext(object())
                ),
            )
        )
        signer = self.stack.enter_context(
            mock.patch.object(
                Attestation,
                "sign",
                side_effect=sign or (lambda _signer, dist: FakeAttestation(dist)),
            )
        )
        self.stack.enter_context(
            mock.patch.object(
                Attestation,
                "model_validate_json",
                side_effect=parse or FakeAttestation.parse,
            )
        )
        self.stack.enter_context(
            mock.patch.object(
                requests.sessions.Session,
                "request",
                new=lambda session, method, url, **kwargs: transport.request(
                    session, method, url, **kwargs
                ),
            )
        )
        return transport, audiences, signer

    def test_complete_locked_toolchain(self):
        self.assertEqual(publisher.tooling()["packages"]["twine"], "7.0.0")

    def test_invalid_lock_and_old_transport_refused(self):
        for change in (
            lambda s: s.replace("urllib3==2.8.0", "urllib3==2.5.0"),
            lambda s: s + "\nunpinned>=1\n",
            lambda s: s + "\nunhashed==1\n",
        ):
            with self.subTest(change=change):
                lock = self.root / "invalid.txt"
                lock.write_text(change(publisher.LOCK.read_text()))
                with self.assertRaises(publisher.PublisherError):
                    publisher.tooling(lock)

    def test_metadata25_wheel_sdist_and_subjects(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        self.assertEqual(len(artifacts), 2)
        for artifact in artifacts:
            self.assertEqual(
                artifact.distribution.digest,
                hashlib.sha256(artifact.path.read_bytes()).hexdigest(),
            )

    def test_project_version_metadata_and_closed_roster_refusals(self):
        for project, version in (
            ("purrdf-rdflib", "3.0.0"),
            ("purrdf", "3.0.1"),
            ("purrdf", "03.0.0"),
        ):
            with (
                self.subTest(project=project, version=version),
                self.assertRaises(publisher.PublisherError),
            ):
                publisher.preflight(project, version, self.directory)
        foreign = self.root / "foreign"
        distributions(foreign, content=metadata("other"))
        with self.assertRaises(publisher.PublisherError):
            publisher.preflight("purrdf", "3.0.0", foreign)
        (self.directory / "unexpected.txt").write_text("not a distribution")
        with self.assertRaises(publisher.PublisherError):
            publisher.preflight("purrdf", "3.0.0", self.directory)

    def test_symlink_and_preexisting_attestation_refusals(self):
        link = self.root / "linked-dist"
        link.symlink_to(self.directory, target_is_directory=True)
        with self.assertRaises(publisher.PublisherError):
            publisher.preflight("purrdf", "3.0.0", link)
        artifact = publisher.preflight("purrdf", "3.0.0", self.directory)[0]
        artifact.attestation_path.write_text("old")
        with self.assertRaises(publisher.PublisherError):
            publisher.preflight("purrdf", "3.0.0", self.directory)
        artifact.attestation_path.unlink()
        target = self.root / "original"
        artifact.path.rename(target)
        artifact.path.symlink_to(target)
        with self.assertRaises(publisher.PublisherError):
            publisher.preflight("purrdf", "3.0.0", self.directory)

    def test_absent_identity_refuses_before_signing_or_upload(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        with (
            mock.patch.object(oidc, "detect_credential", return_value=None),
            mock.patch.object(Attestation, "sign") as signer,
        ):
            with self.assertRaises(publisher.PublisherError):
                publisher.publish(artifacts, "3.0.0")
            signer.assert_not_called()

    def test_wrong_workflow_and_explicit_credentials_refused(self):
        for field in (
            "GITHUB_REPOSITORY",
            "GITHUB_REF",
            "GITHUB_WORKFLOW_REF",
            "TWINE_PASSWORD",
            "TWINE_USERNAME",
            "TWINE_CERT",
            "TWINE_CLIENT_CERT",
            "TWINE_REPOSITORY_URL",
            "TWINE_CONFIG_FILE",
            "TWINE_FUTURE_OVERRIDE",
            "SIGSTORE_ID_TOKEN",
        ):
            with (
                self.subTest(field=field),
                mock.patch.dict(os.environ, {field: "wrong"}),
                self.assertRaises(publisher.PublisherError),
            ):
                publisher.publishing_context("3.0.0")

    def test_malformed_expired_wrong_audience_issuer_subject_identity(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        for token in (
            "bad",
            identity_token(exp=0),
            identity_token(aud="other"),
            identity_token(iss="https://example.org"),
            identity_token(sub="repo:other:ref:refs/tags/py-v3.0.0"),
        ):
            with (
                self.subTest(token_kind=token.split(".")[0]),
                mock.patch.object(oidc, "detect_credential", return_value=token),
                mock.patch.object(Attestation, "sign") as signer,
            ):
                with self.assertRaises((oidc.IdentityError, publisher.PublisherError)):
                    publisher.publish(artifacts, "3.0.0")
                signer.assert_not_called()

    def test_real_twine_cli_two_audiences_exact_uploads_both_projects(self):
        for project in ("purrdf", "purrdf-rdflib"):
            with self.subTest(project=project):
                directory = self.root / project
                distributions(directory, project)
                artifacts = publisher.preflight(project, "3.0.0", directory)
                transport, audiences, signer = self.mocked_publication()
                publisher.publish(artifacts, "3.0.0")
                self.assertEqual(audiences, ["sigstore", "pypi"])
                self.assertEqual(signer.call_count, 2)
                self.assertEqual(len(transport.uploads), 2)
                self.assertEqual(
                    {u["sha256"] for u in transport.uploads},
                    {a.distribution.digest for a in artifacts},
                )

    def test_real_twine_duplicate_skip(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        transport, audiences, _ = self.mocked_publication(existing=True)
        publisher.publish(artifacts, "3.0.0")
        self.assertEqual(audiences, ["sigstore", "pypi"])
        self.assertEqual(transport.uploads, [])

    def test_missing_pypi_identity_cannot_use_stored_credentials(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        transport, _, _ = self.mocked_publication()
        os.environ["HOME"] = str(self.root)
        (self.root / ".pypirc").write_text(
            "[pypi]\nusername=__token__\npassword=mock-stored-token\n"
        )
        with (
            mock.patch("twine.auth.detect_credential", return_value=None),
            self.assertRaises(publisher.PublisherError),
        ):
            publisher.publish(artifacts, "3.0.0")
        self.assertEqual(transport.uploads, [])

    def test_preloaded_keyring_cannot_replace_missing_pypi_identity(self):
        class StoredCredentials(KeyringBackend):
            priority = 1

            def __init__(self):
                self.reads = 0

            def get_password(self, service, username):
                self.reads += 1
                return "mock-stored-token"

            def set_password(self, service, username, password):
                raise AssertionError("offline test must not store credentials")

            def delete_password(self, service, username):
                raise AssertionError("offline test must not delete credentials")

        stored = StoredCredentials()
        keyring.set_keyring(stored)
        self.addCleanup(keyring.set_keyring, Keyring())
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        transport, audiences, _ = self.mocked_publication()
        with (
            mock.patch("twine.auth.detect_credential", return_value=None),
            self.assertRaises(publisher.PublisherError),
        ):
            publisher.publish(artifacts, "3.0.0")
        self.assertEqual(audiences, ["sigstore"])
        self.assertIsInstance(keyring.get_keyring(), Keyring)
        self.assertEqual(stored.reads, 0)
        self.assertEqual(transport.uploads, [])

    def test_real_verifier_rejects_an_invalid_attestation_before_upload(self):
        artifacts = publisher.preflight("purrdf", "3.0.0", self.directory)
        transport, _, _ = self.mocked_publication()
        # The real Pydantic attestation parser rejects this mocked signing
        # representation; mock success cannot masquerade as cryptographic proof.
        with (
            mock.patch.object(
                Attestation, "model_validate_json", wraps=self.real_attestation_parser
            ),
            self.assertRaises(ValidationError),
        ):
            publisher.publish(artifacts, "3.0.0")
        self.assertEqual(transport.uploads, [])

    def test_real_twine_http_failure_and_existing_response(self):
        for status in (403, 400):
            with self.subTest(status=status):
                directory = self.root / str(status)
                distributions(directory)
                artifacts = publisher.preflight("purrdf", "3.0.0", directory)
                transport, _, _ = self.mocked_publication(status=status)
                if status == 403:
                    with self.assertRaises(publisher.PublisherError):
                        publisher.publish(artifacts, "3.0.0")
                    self.assertEqual(len(transport.uploads), 1)
                else:
                    publisher.publish(artifacts, "3.0.0")
                    self.assertEqual(len(transport.uploads), 2)

    def test_changed_bytes_wrong_predicate_and_corrupt_attestation_no_upload(self):
        for kind in ("changed", "predicate", "corrupt"):
            with self.subTest(kind=kind):
                directory = self.root / kind
                distributions(directory)
                artifacts = publisher.preflight("purrdf", "3.0.0", directory)

                def sign(_signer, distribution, kind=kind, artifacts=artifacts):
                    result = FakeAttestation(
                        distribution,
                        "wrong" if kind == "predicate" else publisher.PUBLISH_PREDICATE,
                    )
                    if kind == "changed":
                        artifacts[0].path.write_bytes(b"changed")
                    return result

                parse = (
                    (
                        lambda _raw: (_ for _ in ()).throw(
                            ValueError("corrupt attestation")
                        )
                    )
                    if kind == "corrupt"
                    else None
                )
                transport, _, _ = self.mocked_publication(sign=sign, parse=parse)
                with self.assertRaises((publisher.PublisherError, ValueError)):
                    publisher.publish(artifacts, "3.0.0")
                self.assertEqual(transport.uploads, [])

    def test_cli_failure_sanitizes_external_exception(self):
        with (
            mock.patch.object(
                sys,
                "argv",
                [
                    "publish-python",
                    "check",
                    "--project",
                    "purrdf",
                    "--version",
                    "3.0.0",
                    "--dist-dir",
                    str(self.directory),
                ],
            ),
            mock.patch.object(
                publisher, "run", side_effect=ValueError("test-only-sensitive-marker")
            ),
            contextlib.redirect_stderr(io.StringIO()) as output,
        ):
            self.assertEqual(publisher.main(), 1)
        self.assertNotIn("test-only-sensitive-marker", output.getvalue())


if __name__ == "__main__":
    unittest.main()
