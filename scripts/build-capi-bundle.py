#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Build the C distribution through cargo-c, including installed legal notices."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import runpy
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parent.parent
GENERATED_HEADER = runpy.run_path(str(ROOT / "scripts/capi-header.py"))[
    "generated_header"
]


def run(*arguments: str) -> None:
    subprocess.run(arguments, cwd=ROOT, check=True)


def validate_install_capture(messages: str, build: Path, install: Path) -> None:
    """Require installed bytes from this invocation's private compiler outputs."""
    generated = GENERATED_HEADER(
        messages, ROOT / "crates/rdf-capi/Cargo.toml", build
    ).resolve()
    directory = generated.parents[2]
    header = install / "include/purrdf/purrdf.h"
    if generated.read_bytes() != header.read_bytes():
        raise ValueError("installed C header differs from the reported private build")
    libraries = [
        path
        for path in directory.glob("libpurrdf*")
        if path.suffix in {".a", ".so", ".dylib", ".dll", ".lib"}
    ]
    installed = list((install / "lib").glob("libpurrdf*"))
    if not libraries or not installed:
        raise ValueError("cargo-c did not report and install native C libraries")

    def digest(path: Path) -> bytes:
        with path.open("rb") as source:
            return hashlib.file_digest(source, "sha256").digest()

    expected = {digest(path) for path in libraries}
    if any(digest(path) not in expected for path in installed):
        raise ValueError("installed C library differs from the reported private build")


class CaptureTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.build = self.root / "private-build"
        self.install = self.root / "install"
        self.library = self.build / "release/libpurrdf.so"
        self.generated = self.build / "release/include/purrdf/purrdf.h"
        self.installed_library = self.install / "lib/libpurrdf.so"
        self.installed_header = self.install / "include/purrdf/purrdf.h"
        for path, body in [
            (self.library, b"current library"),
            (self.generated, b"current header"),
            (self.installed_library, b"current library"),
            (self.installed_header, b"current header"),
        ]:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)

    def messages(self, library: Path | None = None) -> str:
        return "\n".join(
            json.dumps(message)
            for message in [
                {
                    "reason": "compiler-artifact",
                    "manifest_path": str(ROOT / "crates/rdf-capi/Cargo.toml"),
                    "target": {"kind": ["cdylib"]},
                    "filenames": [str(library or self.library)],
                },
                {"reason": "build-finished", "success": True},
            ]
        )

    def test_private_current_outputs_ignore_stale_shared_target(self) -> None:
        stale = self.root / "shared-target/release/libpurrdf.so"
        stale.parent.mkdir(parents=True)
        stale.write_bytes(b"previous candidate")
        validate_install_capture(self.messages(), self.build, self.install)

    def test_ignored_private_target_is_refused(self) -> None:
        stale = self.root / "shared-target/release/libpurrdf.so"
        with self.assertRaisesRegex(ValueError, "ignored the private"):
            validate_install_capture(self.messages(stale), self.build, self.install)

    def test_installed_stale_library_is_refused(self) -> None:
        self.installed_library.write_bytes(b"previous candidate")
        with self.assertRaisesRegex(ValueError, "installed C library differs"):
            validate_install_capture(self.messages(), self.build, self.install)

    def test_installed_stale_header_is_refused(self) -> None:
        self.installed_header.write_bytes(b"previous header")
        with self.assertRaisesRegex(ValueError, "installed C header differs"):
            validate_install_capture(self.messages(), self.build, self.install)


def qualify_bundle(archive_path: Path, directory: Path, name: str) -> None:
    """Link and run the existing C acceptance program from the actual tar."""
    with tarfile.open(archive_path) as archive:
        archive.extractall(directory, filter="data")
    installed = directory / name
    environment = dict(os.environ)
    environment["PKG_CONFIG_PATH"] = str(installed / "lib/pkgconfig")
    prefix = subprocess.check_output(
        ["pkg-config", "--variable=prefix", "purrdf"],
        env=environment,
        text=True,
    ).strip()
    if Path(prefix).resolve() != installed.resolve():
        raise SystemExit(
            "C bundle pkg-config prefix does not resolve to its extraction"
        )
    flags = shlex.split(
        subprocess.check_output(
            ["pkg-config", "--cflags", "--libs", "purrdf"],
            env=environment,
            text=True,
        )
    )
    program = directory / "purrdf-installed-smoke"
    subprocess.run(
        [
            *shlex.split(os.environ.get("CC", "cc")),
            "-std=c11",
            str(ROOT / "crates/rdf-capi/tests/smoke.c"),
            *flags,
            "-o",
            str(program),
        ],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    loader_path = (
        "PATH"
        if sys.platform == "win32"
        else "DYLD_LIBRARY_PATH"
        if sys.platform == "darwin"
        else "LD_LIBRARY_PATH"
    )
    environment[loader_path] = (
        str(installed / "lib") + os.pathsep + environment.get(loader_path, "")
    )
    corpus = ROOT / "crates/sparql-conformance/entailment-suite/w3c-owl2-rl"
    subprocess.run(
        [
            str(program),
            str(ROOT / "crates/rdf/tests/fixtures/okf-terms.trig"),
            str(ROOT / "crates/rdf/tests/fixtures/okf-terms.json"),
            str(ROOT / "crates/validate/tests/fixtures/regime-boundary.vectors"),
            str(corpus / "cases/webont-imports-011/premise.rdf"),
            str(corpus / "cases/webont-imports-011/conclusion.rdf"),
            str(corpus / "imports/support011-A.rdf"),
        ],
        cwd=ROOT,
        env=environment,
        check=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(CaptureTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    run("python3", "scripts/package-licenses.py", "--profile", "c", "--check")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"][
        "version"
    ]
    rustc = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
    host = next(
        line.removeprefix("host: ")
        for line in rustc.splitlines()
        if line.startswith("host: ")
    )
    output = ROOT / "target/dist"
    output.mkdir(parents=True, exist_ok=True)
    name = f"purrdf-capi-{version}-{host}"
    with tempfile.TemporaryDirectory(prefix="capi-bundle-", dir=output) as raw:
        install = Path(raw) / "install"
        build = Path(raw) / "build"
        built = subprocess.run(
            [
                "cargo",
                "capi",
                "install",
                "-p",
                "purrdf-capi",
                "--locked",
                "--release",
                "--message-format=json",
                "--target-dir",
                str(build),
                "--prefix",
                "/",
                "--destdir",
                str(install),
            ],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            text=True,
            check=True,
        )
        # cargo-c prints installation actions after Cargo's JSON stream. Keep
        # the exact compiler messages, without treating those actions as JSON.
        messages = "\n".join(
            line for line in built.stdout.splitlines() if line.lstrip().startswith("{")
        )
        validate_install_capture(messages, build, install)
        print("Verified private cargo-c outputs against installed header and libraries")
        header = install / "include/purrdf/purrdf.h"
        if (
            header.read_bytes()
            != (ROOT / "crates/rdf-capi/include/purrdf.h").read_bytes()
        ):
            raise SystemExit(
                "C bundle header differs from the committed ABI; run make capi-header"
            )
        if not list((install / "lib").glob("libpurrdf*")):
            raise SystemExit("cargo-c installed no libpurrdf library")
        configs = list((install / "lib/pkgconfig").glob("purrdf.pc"))
        if len(configs) != 1:
            raise SystemExit("cargo-c installed no unique purrdf pkg-config file")
        configuration = configs[0].read_text()
        if not configuration.startswith("prefix=/\n"):
            raise SystemExit(
                "cargo-c pkg-config prefix differs from the requested root"
            )
        configs[0].write_text(
            configuration.replace("prefix=/\n", "prefix=${pcfiledir}/../..\n", 1)
        )
        notices = ROOT / "crates/rdf-capi/licenses"
        shutil.copyfile(ROOT / "crates/rdf-capi/README.md", install / "README.md")
        shutil.copytree(notices, install / "share/purrdf/licenses")
        # Make the recipient payload independently visible at the bundle root.
        shutil.copytree(notices, install / "licenses")
        run(
            "python3",
            "scripts/package-licenses.py",
            "--runtime-dir",
            str(install / "licenses/runtime"),
        )
        shutil.copytree(
            install / "licenses/runtime", install / "share/purrdf/licenses/runtime"
        )
        archive_path = output / f"{name}.tar.gz"
        with (
            archive_path.open("wb") as sink,
            gzip.GzipFile(fileobj=sink, filename="", mode="wb", mtime=0) as compressed,
            tarfile.open(fileobj=compressed, mode="w") as archive,
        ):
            for path in sorted(install.rglob("*")):
                if path.is_dir():
                    continue
                body = path.read_bytes()  # materialize cargo-c's library symlinks
                item = tarfile.TarInfo(f"{name}/{path.relative_to(install).as_posix()}")
                item.size = len(body)
                item.mode = 0o755 if os.access(path, os.X_OK) else 0o644
                archive.addfile(item, io.BytesIO(body))
        run(
            "python3",
            "scripts/package-licenses.py",
            "--profile",
            "c",
            "--audit",
            str(archive_path),
            "--receipt",
            str(output / f"{name}.license-receipt.json"),
        )
        qualify_bundle(archive_path, Path(raw) / "extracted", name)
        print(archive_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
