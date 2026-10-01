#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Generate or verify the header of the exact cargo-c build being inspected."""

import argparse
import difflib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


def generated_header(messages: str, manifest: Path, build: Path | None = None) -> Path:
    """Resolve one successful cargo-c output, never an unrelated target-tree file."""
    directories: set[Path] = set()
    finished = False
    for line in messages.splitlines():
        if not line.strip():
            continue
        message = json.loads(line)
        if message.get("reason") == "build-finished":
            if message.get("success") is not True:
                raise ValueError("cargo-c reported an unsuccessful build")
            finished = True
        if (
            message.get("reason") != "compiler-artifact"
            or Path(message.get("manifest_path", "")).resolve() != manifest.resolve()
            or "cdylib" not in message.get("target", {}).get("kind", [])
        ):
            continue
        for filename in message.get("filenames", []):
            path = Path(filename)
            if path.suffix in {".a", ".so", ".dylib", ".dll", ".lib"}:
                directories.add(path.parent)
    if not finished or len(directories) != 1:
        raise ValueError("cargo-c did not report one successful libpurrdf artifact directory")
    directory = directories.pop().resolve()
    if build is not None and not directory.is_relative_to(build.resolve()):
        raise ValueError("cargo-c ignored the private C artifact directory")
    return directory / "include" / "purrdf" / "purrdf.h"


class HeaderTests(unittest.TestCase):
    def test_current_debug_artifact_wins_over_existing_stale_release_header(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "crates" / "rdf-capi" / "Cargo.toml"
            current = root / "target" / "debug" / "include" / "purrdf" / "purrdf.h"
            stale = root / "target" / "release" / "include" / "purrdf" / "purrdf.h"
            for header, text in [(stale, "old ABI"), (current, "current ABI")]:
                header.parent.mkdir(parents=True)
                header.write_text(text)
            messages = "\n".join(
                json.dumps(message)
                for message in [
                    {
                        "reason": "compiler-artifact",
                        "manifest_path": str(manifest),
                        "target": {"kind": ["staticlib", "cdylib"]},
                        "filenames": [str(root / "target" / "debug" / "libpurrdf.so")],
                    },
                    {"reason": "build-finished", "success": True},
                ]
            )
            self.assertEqual(generated_header(messages, manifest), current)
            self.assertEqual(generated_header(messages, manifest).read_text(), "current ABI")

    def test_missing_or_failed_build_cannot_reuse_an_existing_header(self):
        for messages in ["", '{"reason":"build-finished","success":false}']:
            with self.subTest(messages=messages), self.assertRaises(ValueError):
                generated_header(messages, Path("crates/rdf-capi/Cargo.toml"))

    def test_multiple_matching_artifact_directories_are_refused(self):
        manifest = Path("crates/rdf-capi/Cargo.toml").resolve()
        messages = "\n".join(
            json.dumps(message)
            for message in [
                {
                    "reason": "compiler-artifact",
                    "manifest_path": str(manifest),
                    "target": {"kind": ["cdylib"]},
                    "filenames": ["target/debug/libpurrdf.so", "target/release/libpurrdf.so"],
                },
                {"reason": "build-finished", "success": True},
            ]
        )
        with self.assertRaises(ValueError):
            generated_header(messages, manifest)

    def test_ignored_private_target_is_refused_even_with_successful_build(self):
        manifest = Path("crates/rdf-capi/Cargo.toml").resolve()
        messages = "\n".join(
            json.dumps(message)
            for message in [
                {
                    "reason": "compiler-artifact",
                    "manifest_path": str(manifest),
                    "target": {"kind": ["cdylib"]},
                    "filenames": ["target/debug/libpurrdf.so"],
                },
                {"reason": "build-finished", "success": True},
            ]
        )
        with self.assertRaisesRegex(ValueError, "ignored the private"):
            generated_header(messages, manifest, Path("target/private-header-build"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", type=Path, metavar="HEADER")
    mode.add_argument("--check", type=Path, metavar="HEADER")
    mode.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(HeaderTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    root = Path(__file__).resolve().parent.parent
    # An exact JSON path can still name a shared compiler slot that is reused
    # after Cargo returns. Keep regeneration and capture in our own directory.
    # Refuse a wrapper that routes the build elsewhere instead of reading it.
    with tempfile.TemporaryDirectory(prefix="purrdf-header-") as temporary:
        private_build = Path(temporary) / "build"
        built = subprocess.run(
            [
                "cargo", "capi", "build", "-p", "purrdf-capi",
                "--target-dir", str(private_build), "--message-format=json",
            ],
            cwd=root,
            stdout=subprocess.PIPE,
            text=True,
            check=False,
        )
        for line in built.stdout.splitlines():
            message = json.loads(line)
            if message.get("reason") == "compiler-message":
                diagnostic = message.get("message", {})
                if diagnostic.get("level") in {"warning", "error"}:
                    print(diagnostic.get("rendered", diagnostic.get("message", "")), file=sys.stderr)
        if built.returncode != 0:
            return built.returncode
        try:
            generated = generated_header(
                built.stdout, root / "crates" / "rdf-capi" / "Cargo.toml", private_build
            )
        except (ValueError, json.JSONDecodeError) as error:
            print(f"FAIL: {error}", file=sys.stderr)
            return 1
        if not generated.is_file():
            print(f"FAIL: cargo-c did not emit {generated}", file=sys.stderr)
            return 1
        current = generated.read_bytes()
    committed = root / (args.write or args.check)
    if args.write:
        committed.write_bytes(current)
        print(f"regenerated {committed.relative_to(root)} from the private cargo-c build")
        return 0
    if current != committed.read_bytes():
        print(f"FAIL: {committed.relative_to(root)} is stale; run 'make capi-header'", file=sys.stderr)
        diff = difflib.unified_diff(
            committed.read_text().splitlines(),
            current.decode().splitlines(),
            fromfile=str(committed),
            tofile=str(generated),
        )
        print("\n".join(list(diff)[:40]), file=sys.stderr)
        return 1
    print("OK: committed purrdf.h matches the captured private cargo-c artifact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
