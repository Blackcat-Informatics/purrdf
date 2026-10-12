#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Capture Cargo's selected WebAssembly artifact while Cargo still owns its build slot.

Usage: build-private-wasm.py NEW_OUTPUT_FILE TARGET_NAME CARGO_RUSTC_ARGUMENTS...
The host's ordinary cargo command may assign its own target directory. Its JSON
artifact record, not an assumed path, identifies the output. Copying before the
command exits keeps a leased build slot from being reused during the read.
"""

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


def selected_module(record, target_name):
    if record.get("reason") != "compiler-artifact" or record.get("target", {}).get("name") != target_name:
        return None
    if type(record.get("fresh")) is not bool:
        raise ValueError("selected compiler unit has no current freshness record")
    modules = [Path(name).resolve() for name in record.get("filenames", []) if Path(name).suffix == ".wasm"]
    if len(modules) != 1:
        raise ValueError("expected exactly one registered WebAssembly output")
    return modules[0], record["fresh"]


def capture_record(record, target_name, output):
    selected = selected_module(record, target_name)
    if selected is None:
        return None
    source, fresh = selected
    with source.open("rb") as module, output.open("xb") as captured:
        if module.read(8) != b"\0asm\x01\0\0\0":
            raise ValueError("registered compiler link output is not a WebAssembly module")
        module.seek(0)
        shutil.copyfileobj(module, captured)
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
        raise ValueError("registered compiler output changed while it was captured")
    return source, fresh, digest


def capture_cargo(arguments, target_name, output, *, cwd=None, env=None, test_only=False):
    """Run standard cargo and capture the selected artifact before its lease ends."""
    output = Path(output).resolve()
    log = output.with_suffix(".jsonl")
    if output.exists() or log.exists() or not output.parent.is_dir():
        raise ValueError("compiler output and its JSON record require new paths in an existing directory")
    captured = None
    finished = False
    failure = None
    with log.open("x") as record_log, subprocess.Popen(
        ["cargo", *arguments, "--message-format=json"],
        cwd=cwd, env=env, stdout=subprocess.PIPE, text=True,
    ) as process:
        assert process.stdout is not None
        for line in process.stdout:
            record_log.write(line)
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue  # cargo-c also prints installation actions.
            if record.get("reason") == "compiler-message":
                diagnostic = record.get("message", {})
                if diagnostic.get("level") == "error":
                    print(diagnostic.get("rendered") or diagnostic.get("message"), file=sys.stderr)
            if record.get("reason") == "build-finished":
                finished = record.get("success") is True
            if (record.get("reason") == "compiler-artifact"
                    and record.get("target", {}).get("name") == target_name
                    and (not test_only or record.get("profile", {}).get("test") is True)):
                try:
                    if captured is not None:
                        raise ValueError("expected exactly one identified selected compiler unit")
                    captured = capture_record(record, target_name, output)
                except (OSError, ValueError) as error:
                    failure = error
        status = process.wait()
    if status != 0 or not finished or captured is None or failure is not None:
        output.unlink(missing_ok=True)
        if failure is not None:
            raise failure
        raise ValueError(f"cargo did not successfully report one {target_name} artifact (exit {status})")
    return captured


class CaptureTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = self.root / "selected.wasm"
        self.source.write_bytes(b"\0asm\x01\0\0\0current")
        self.output = self.root / "capture.wasm"
        self.record = {
            "reason": "compiler-artifact", "target": {"name": "selected"},
            "fresh": False, "filenames": [str(self.source)],
        }

    def test_reported_output_is_copied_even_outside_requested_directory(self):
        source, fresh, digest = capture_record(self.record, "selected", self.output)
        self.assertEqual((source, fresh), (self.source, False))
        self.assertEqual(self.output.read_bytes(), self.source.read_bytes())
        self.assertEqual(digest, hashlib.sha256(self.source.read_bytes()).hexdigest())

    def test_missing_or_non_wasm_output_is_refused(self):
        self.source.unlink()
        with self.assertRaises(FileNotFoundError):
            capture_record(self.record, "selected", self.output)
        self.source.write_bytes(b"not wasm")
        with self.assertRaisesRegex(ValueError, "not a WebAssembly"):
            capture_record(self.record, "selected", self.output)

    def test_unidentified_or_ambiguous_output_is_refused(self):
        for record in [
            {**self.record, "fresh": "false"},
            {**self.record, "filenames": [str(self.source), str(self.source)]},
        ]:
            with self.subTest(record=record), self.assertRaises(ValueError):
                selected_module(record, "selected")
        self.assertIsNone(selected_module(self.record, "other"))

    def test_capture_occurs_before_the_cargo_process_releases_its_output(self):
        source, output, record = self.source, self.output, self.record

        class Process:
            def __enter__(self): return self
            def __exit__(self, *_): return False
            def wait(self): return 0
            @property
            def stdout(self):
                def records():
                    yield json.dumps(record) + "\n"
                    assert output.read_bytes() == source.read_bytes()
                    source.unlink()
                    yield '{"reason":"build-finished","success":true}\n'
                return records()

        with patch("subprocess.Popen", return_value=Process()):
            registered, fresh, _ = capture_cargo(["rustc"], "selected", output)
        self.assertEqual((registered, fresh), (source, False))


def main():
    if sys.argv[1:] == ["--self-test"]:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(CaptureTests)
        if not unittest.TextTestRunner().run(suite).wasSuccessful():
            raise SystemExit(1)
        return
    if len(sys.argv) < 4:
        raise SystemExit(__doc__)
    output = Path(sys.argv[1]).resolve()
    source, fresh, digest = capture_cargo(["rustc", *sys.argv[3:]], sys.argv[2], output)
    output.with_suffix(".capture.json").write_text(json.dumps({
        "schema": "purrdf-cargo-artifact-capture-v2",
        "registered_output": str(source), "captured_output": str(output),
        "sha256": digest, "cargo_fresh": fresh,
        "capture_phase": "before cargo command exit",
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
