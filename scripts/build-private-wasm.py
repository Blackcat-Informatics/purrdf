#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Capture one final compiler output from a new private Cargo target directory.

Usage: build-private-wasm.py NEW_OUTPUT_FILE TARGET_NAME CARGO_RUSTC_ARGUMENTS...
The caller selects the package, target, profile and rustflags. The output file
must not exist. The registered output named by Cargo's JSON record must be
inside the requested private directory; an ignored directory is refused. Normal
registered outputs also restore correctly through compiler caches. A compiler
record and capture digest stay beside the copied module.
"""

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


def validate_capture(records, target_name, private_target):
    units = [
        record
        for record in records
        if record.get("reason") == "compiler-artifact"
        and record.get("target", {}).get("name") == target_name
    ]
    if len(units) != 1 or type(units[0].get("fresh")) is not bool:
        raise ValueError("expected exactly one identified selected compiler unit")
    modules = [
        Path(name).resolve()
        for name in units[0].get("filenames", [])
        if Path(name).suffix == ".wasm"
    ]
    if len(modules) != 1:
        raise ValueError("expected exactly one registered WebAssembly output")
    output = modules[0]
    if not output.is_relative_to(private_target.resolve()):
        raise ValueError(
            "Cargo ignored the requested private WebAssembly target directory"
        )
    with output.open("rb") as module:
        if module.read(8) != b"\0asm\x01\0\0\0":
            raise ValueError(
                "registered compiler link output is not a WebAssembly module"
            )
    return output, units[0]["fresh"]


class CaptureTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.private_target = self.root / "private-target"
        self.private_target.mkdir()
        self.output = self.private_target / "selected.wasm"
        self.output.write_bytes(b"\0asm\x01\0\0\0current")
        self.stale = self.root / "shared.wasm"
        self.stale.write_bytes(b"\0asm\x01\0\0\0stale")
        self.record = {
            "reason": "compiler-artifact",
            "target": {"name": "selected"},
            "fresh": False,
            "filenames": [str(self.output)],
        }

    def test_private_output_ignores_stale_shared_filename(self):
        found, _ = validate_capture([self.record], "selected", self.private_target)
        self.assertTrue(found.read_bytes().endswith(b"current"))

    def test_missing_private_output_never_falls_back_to_shared_file(self):
        self.output.unlink()
        with self.assertRaises(FileNotFoundError):
            validate_capture([self.record], "selected", self.private_target)

    def test_ignored_private_target_is_refused(self):
        with self.assertRaises(ValueError):
            validate_capture(
                [{**self.record, "filenames": [str(self.stale)]}],
                "selected",
                self.private_target,
            )

    def test_current_cached_registered_output_is_admitted(self):
        found, fresh = validate_capture(
            [{**self.record, "fresh": True}], "selected", self.private_target
        )
        self.assertEqual(found, self.output)
        self.assertTrue(fresh)

    def test_unidentified_compile_is_refused(self):
        for fresh in (None, "false"):
            with self.subTest(fresh=fresh), self.assertRaises(ValueError):
                validate_capture(
                    [{**self.record, "fresh": fresh}], "selected", self.private_target
                )

    def test_multiple_selected_units_are_refused(self):
        with self.assertRaises(ValueError):
            validate_capture(
                [self.record, self.record], "selected", self.private_target
            )

    def test_multiple_registered_outputs_are_refused(self):
        with self.assertRaises(ValueError):
            validate_capture(
                [{**self.record, "filenames": [str(self.output), str(self.stale)]}],
                "selected",
                self.private_target,
            )

    def test_non_wasm_output_is_refused(self):
        self.output.write_bytes(b"not a module")
        with self.assertRaises(ValueError):
            validate_capture([self.record], "selected", self.private_target)


def main():
    if sys.argv[1:] == ["--self-test"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(CaptureTests)
        if not unittest.TextTestRunner().run(suite).wasSuccessful():
            raise SystemExit(1)
        return
    if len(sys.argv) < 4:
        raise SystemExit(__doc__)
    output = Path(sys.argv[1]).resolve()
    target_name = sys.argv[2]
    build_log = output.with_suffix(".jsonl")
    if output.exists() or build_log.exists():
        raise SystemExit("compiler output and its JSON record must be new paths")
    if not output.parent.is_dir():
        raise SystemExit("compiler output parent directory must already exist")
    with tempfile.TemporaryDirectory(
        prefix="compiler-target-", dir=output.parent
    ) as raw_target:
        private_target = Path(raw_target)
        with build_log.open("x") as log:
            subprocess.run(
                [
                    "cargo",
                    "rustc",
                    *sys.argv[3:],
                    "--target-dir",
                    str(private_target),
                    "--message-format=json",
                ],
                stdout=log,
                check=True,
            )
        records = [json.loads(line) for line in build_log.read_text().splitlines()]
        registered, fresh = validate_capture(records, target_name, private_target)
        with registered.open("rb") as source, output.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        receipt = {
            "schema": "purrdf-private-wasm-capture-v1",
            "registered_output": str(registered),
            "private_target_directory": str(private_target),
            "captured_output": str(output),
            "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
            "cargo_fresh": fresh,
        }
        output.with_suffix(".capture.json").write_text(
            json.dumps(receipt, indent=2) + "\n"
        )


if __name__ == "__main__":
    main()
