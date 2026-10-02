#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check or publish existing distributions with pinned official Python tools.

Publishing uses the public pypi-attestations/Sigstore APIs and Twine's native
Trusted Publishing flow. No distributions are built here. The offline tests
mock identity, signing, and transport; they do not validate a live publisher.
"""

import argparse
import contextlib
import hashlib
import importlib.metadata
import importlib.util
import io
import json
import os
import platform
import re
import sys
from dataclasses import dataclass
from pathlib import Path

LOCK = Path(__file__).with_name("python-publisher-requirements.txt")
REPOSITORY = "Blackcat-Informatics/purrdf"
WORKFLOW = "release-pypi.yaml"
PYPI = "https://upload.pypi.org/legacy/"
GITHUB_ISSUER = "https://token.actions.githubusercontent.com"
PUBLISH_PREDICATE = "https://docs.pypi.org/attestations/publish/v1"


class PublisherError(Exception):
    """A refusal before publication, with no credential data in its message."""


def tooling(lock=LOCK):
    """Verify the entire one-home lock against the isolated tooling environment."""
    if sys.version_info[:2] != (3, 13) or platform.system() != "Linux":
        raise PublisherError("publisher tooling requires CPython 3.13 on Linux")
    if platform.python_implementation() != "CPython" or platform.machine() != "x86_64":
        raise PublisherError("publisher tooling requires CPython on x86_64")
    pins = {}
    current = None
    hashes = 0
    for raw in lock.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = re.fullmatch(r"([a-z0-9-]+)==([^\s\\]+)\s*\\?", line)
        if match:
            if current is not None and hashes == 0:
                raise PublisherError("publisher lock has an unhashed requirement")
            current, version = match.groups()
            if current in pins:
                raise PublisherError("publisher lock has duplicate requirements")
            pins[current] = version
            hashes = 0
        elif current and re.fullmatch(r"--hash=sha256:[0-9a-f]{64}\s*\\?", line):
            hashes += 1
        else:
            raise PublisherError("publisher lock contains an unpinned requirement")
    if not current or not hashes:
        raise PublisherError("publisher lock has an unhashed requirement")
    if (
        not {"twine", "pypi-attestations", "sigstore", "urllib3", "requests"}
        <= pins.keys()
    ):
        raise PublisherError("publisher lock omits required tooling")
    from packaging.version import Version

    floors = {
        "urllib3": "2.8.0",
        "twine": "7.0.0",
        "sigstore": "4.2.0",
        "cryptography": "50.0.2",
        "tuf": "7.0.1",
    }
    if not floors.keys() <= pins.keys():
        raise PublisherError("publisher lock omits required tooling")
    if any(Version(pins[name]) < Version(floor) for name, floor in floors.items()):
        raise PublisherError(
            "publisher lock predates qualified security or metadata fixes"
        )
    for name, version in pins.items():
        try:
            installed = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError as error:
            raise PublisherError("publisher tooling is not hash-synced") from error
        if installed != version:
            raise PublisherError(f"publisher tooling version differs for {name}")
    return {
        "requirements_sha256": hashlib.sha256(lock.read_bytes()).hexdigest(),
        "packages": pins,
    }


def twine_cli(arguments):
    """Use the installed official CLI entrypoint; keep upstream errors private."""
    entries = list(
        importlib.metadata.entry_points(group="console_scripts", name="twine")
    )
    if len(entries) != 1 or entries[0].value != "twine.__main__:main":
        raise PublisherError("official Twine CLI entrypoint is unavailable")
    original = sys.argv
    try:
        sys.argv = ["twine", "--no-color", *arguments]
        # The upstream CLI can log a server's exception/response. Do not echo
        # its output: this adapter prints only its own safe receipts/refusals.
        with (
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            result = entries[0].load()()
    finally:
        sys.argv = original
    if result:
        raise PublisherError("official Twine CLI failed")


@dataclass(frozen=True)
class Artifact:
    path: Path
    distribution: object

    @property
    def attestation_path(self):
        return self.path.with_name(self.path.name + ".publish.attestation")


def preflight(project, version, dist_dir):
    """Validate all metadata and immutable subjects before any identity requests."""
    from packaging.utils import (
        canonicalize_name,
        parse_sdist_filename,
        parse_wheel_filename,
    )
    from packaging.version import Version
    from pypi_attestations import Distribution
    from twine.package import PackageFile

    if project not in ("purrdf", "purrdf-rdflib"):
        raise PublisherError("unexpected release project")
    expected = Version(version)
    if str(expected) != version:
        raise PublisherError("release version must use its canonical spelling")
    if dist_dir.is_symlink() or not dist_dir.is_dir():
        raise PublisherError("distribution directory must be a real directory")
    artifacts = []
    for path in sorted(dist_dir.iterdir()):
        if path.is_symlink() or not path.is_file():
            raise PublisherError("distribution inputs must be regular files")
        if path.name.endswith(".whl"):
            name, actual, _, _ = parse_wheel_filename(path.name)
        elif path.name.endswith((".tar.gz", ".zip")):
            name, actual = parse_sdist_filename(path.name)
        else:
            raise PublisherError("distribution directory contains an unexpected file")
        if name != canonicalize_name(project) or actual != expected:
            raise PublisherError(
                "distribution filename differs from the release identity"
            )
        package = PackageFile.from_filename(str(path), None)
        if canonicalize_name(package.metadata["name"]) != project:
            raise PublisherError("distribution metadata names another project")
        if Version(package.metadata["version"]) != expected:
            raise PublisherError("distribution metadata has another version")
        artifact = Artifact(path, Distribution.from_file(path))
        if artifact.attestation_path.exists() or artifact.attestation_path.is_symlink():
            raise PublisherError("publish attestation already exists")
        artifacts.append(artifact)
    if not artifacts:
        raise PublisherError("no distributions to publish")
    twine_cli(["check", "--strict", *[str(artifact.path) for artifact in artifacts]])
    return artifacts


def unchanged(artifacts):
    from pypi_attestations import Distribution

    for artifact in artifacts:
        if artifact.path.is_symlink() or not artifact.path.is_file():
            raise PublisherError("distribution changed after validation")
        if Distribution.from_file(artifact.path) != artifact.distribution:
            raise PublisherError("distribution bytes changed after validation")


def workflow_identity(version):
    return f"https://github.com/{REPOSITORY}/.github/workflows/{WORKFLOW}@refs/tags/py-v{version}"


def publishing_context(version):
    """Require the configured tag workflow, with no alternate credentials."""
    expected = {
        "GITHUB_ACTIONS": "true",
        "GITHUB_REPOSITORY": REPOSITORY,
        "GITHUB_REF": f"refs/tags/py-v{version}",
        "GITHUB_WORKFLOW_REF": workflow_identity(version).removeprefix(
            "https://github.com/"
        ),
    }
    if any(os.environ.get(key) != value for key, value in expected.items()):
        raise PublisherError("publishing requires the configured GitHub tag workflow")
    if any(key.startswith("TWINE_") for key in os.environ) or (
        "SIGSTORE_ID_TOKEN" in os.environ
    ):
        raise PublisherError("inherited publishing overrides are forbidden")
    import keyring
    from keyring.backends.null import Keyring

    # Replace any already-loaded backend as well as selecting it for new loads.
    # Setting the environment alone does not replace keyring's cached backend.
    keyring.set_keyring(Keyring())
    # The /dev/null Twine config below similarly excludes .pypirc credentials.
    os.environ["PYTHON_KEYRING_BACKEND"] = "keyring.backends.null.Keyring"


def publish(artifacts, version):
    """Sign, verify every subject, then invoke official Trusted Publishing."""
    from pypi_attestations import Attestation
    from sigstore import oidc
    from sigstore.models import ClientTrustConfig
    from sigstore.sign import SigningContext
    from sigstore.verify.policy import Identity

    publishing_context(version)
    unchanged(artifacts)
    token = oidc.detect_credential()
    if token is None:
        raise PublisherError("ambient GitHub signing identity is unavailable")
    identity = oidc.IdentityToken(token)
    expected_subject = f"repo:{REPOSITORY}:ref:refs/tags/py-v{version}"
    if identity.issuer != GITHUB_ISSUER or identity.identity != expected_subject:
        raise PublisherError(
            "ambient signing identity differs from the release workflow"
        )
    trust = ClientTrustConfig.production()
    trust.force_tlog_version = 1  # PEP 740's supported log format, as in its CLI.
    context = SigningContext.from_trust_config(trust)
    attestations = []
    with context.signer(identity, cache=True) as signer:
        for artifact in artifacts:
            attestations.append(Attestation.sign(signer, artifact.distribution))
    policy = Identity(identity=workflow_identity(version), issuer=GITHUB_ISSUER)
    unchanged(artifacts)
    for artifact, attestation in zip(artifacts, attestations, strict=True):
        predicate, _ = attestation.verify(policy, artifact.distribution, offline=True)
        if predicate != PUBLISH_PREDICATE:
            raise PublisherError("distribution attestation has the wrong predicate")
    for artifact, attestation in zip(artifacts, attestations, strict=True):
        with artifact.attestation_path.open("x", encoding="utf-8") as output:
            output.write(attestation.model_dump_json())
    # Reparse the exact adjacent bytes Twine will attach, rather than assuming
    # writing succeeded. Missing, corrupt and mismatched attestations fail here.
    for artifact in artifacts:
        if (
            artifact.attestation_path.is_symlink()
            or not artifact.attestation_path.is_file()
        ):
            raise PublisherError("distribution attestation is missing or not regular")
        attestation = Attestation.model_validate_json(
            artifact.attestation_path.read_bytes()
        )
        predicate, _ = attestation.verify(policy, artifact.distribution, offline=True)
        if predicate != PUBLISH_PREDICATE:
            raise PublisherError("distribution attestation has the wrong predicate")
    unchanged(artifacts)
    twine_cli(
        [
            "upload",
            "--attestations",
            "--skip-existing",
            "--non-interactive",
            "--config-file",
            os.devnull,
            "--repository-url",
            PYPI,
            "--username",
            "__token__",
            "--disable-progress-bar",
            *[
                str(path)
                for artifact in artifacts
                for path in (artifact.path, artifact.attestation_path)
            ],
        ]
    )


def run(mode, project, version, dist_dir, receipt=None):
    toolchain = tooling()
    artifacts = preflight(project, version, dist_dir)
    if mode == "publish":
        publish(artifacts, version)
    result = {
        "schema": 1,
        "mode": mode,
        "project": project,
        "version": version,
        "toolchain": toolchain,
        "artifacts": [
            {"filename": a.path.name, "sha256": a.distribution.digest}
            for a in artifacts
        ],
        "live_publish_executed": mode == "publish",
    }
    if receipt:
        receipt.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", nargs="?", choices=("check", "publish"))
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--project", choices=("purrdf", "purrdf-rdflib"))
    parser.add_argument("--version")
    parser.add_argument("--dist-dir", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    if args.self_test:
        if any((args.mode, args.project, args.version, args.dist_dir, args.receipt)):
            parser.error("--self-test takes no publication options")
        import unittest

        spec = importlib.util.spec_from_file_location(
            "python_publisher_tests",
            Path(__file__).with_name("test_python_publisher.py"),
        )
        tests = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tests)
        result = unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromModule(tests)
        )
        return int(not result.wasSuccessful())
    if not all((args.mode, args.project, args.version, args.dist_dir)):
        parser.error("check/publish requires --project, --version and --dist-dir")
    try:
        result = run(args.mode, args.project, args.version, args.dist_dir, args.receipt)
    except PublisherError as error:
        print(f"Python publisher refused: {error}", file=sys.stderr)
        return 1
    except Exception:  # noqa: BLE001 - external errors can contain identity tokens.
        # Third-party exceptions can contain token/response details. Never put
        # those on stdout, stderr or an Actions summary in this adapter.
        print(
            "Python publisher refused the request; no success receipt was written.",
            file=sys.stderr,
        )
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
