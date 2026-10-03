#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Capture one final compiler output before its build context can be reused.

Usage: build-private-wasm.py NEW_OUTPUT_FILE TARGET_NAME CARGO_RUSTC_ARGUMENTS...
The caller selects the package, target, profile and rustflags. The output file
must not exist. Ordinary Cargo uses a new private directory. Stage's documented
CARGO_REAL delegate captures inside its held managed slot, preserving the outer
wrapper's policy and status. A compiler record and capture digest stay beside
the copied module. Neither path discovers a mutable shared output after return.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from compiler_capture import held_slot, real_cargo, slot_lock, source_identity, stage_capabilities


def selected_unit(records, target_name):
    units = [record for record in records
             if record.get("reason") == "compiler-artifact"
             and record.get("target", {}).get("name") == target_name]
    if len(units) != 1 or type(units[0].get("fresh")) is not bool:
        raise ValueError("expected exactly one identified selected compiler unit")
    return units[0]


def validate_capture(records, target_name, private_target):
    unit = selected_unit(records, target_name)
    modules = [
        Path(name).resolve()
        for name in unit.get("filenames", [])
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
    return output, unit["fresh"]


def managed_delegate(arguments):
    """Forward all wrapper policy, capturing the selected link output under lease."""
    request = json.loads(Path(os.environ["PURRDF_WASM_CAPTURE_REQUEST"]).read_text())
    environment = os.environ.copy()
    environment["CARGO_REAL"] = request["real_cargo"]
    if "rustc" not in arguments:
        os.execve(request["real_cargo"], [request["real_cargo"], *arguments], environment)
    try:
        root = Path(request["root"])
        if source_identity(root) != request["source_identity"]:
            raise ValueError("compiler source identity changed before WebAssembly build")
        target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
        held_slot(target / "wasm32-unknown-unknown/release/selected.wasm", request["capabilities"])
        built = subprocess.run([request["real_cargo"], *arguments], env=environment,
                               stdout=subprocess.PIPE, check=False)
        sys.stdout.buffer.write(built.stdout)
        sys.stdout.buffer.flush()
        if built.returncode:
            return built.returncode
        messages = built.stdout.decode()
        records = [json.loads(line) for line in messages.splitlines() if line.strip()]
        finished = [record for record in records if record.get("reason") == "build-finished"]
        if len(finished) != 1 or finished[0].get("success") is not True:
            raise ValueError("WebAssembly producer did not report one successful build")
        registered, fresh = validate_capture(records, request["target_name"], target)
        lock = held_slot(registered, request["capabilities"])
        if source_identity(root) != request["source_identity"]:
            raise ValueError("compiler source identity changed during WebAssembly build")
        capture = Path(request["capture"])
        with registered.open("rb") as source, capture.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        if source_identity(root) != request["source_identity"]:
            raise ValueError("compiler source identity changed during WebAssembly capture")
        receipt = {"schema": "purrdf-stage-wasm-capture-v1",
                   "source_identity": request["source_identity"],
                   "target_name": request["target_name"], "registered_output": str(registered),
                   "slot_lock": str(lock), "capture": str(capture.resolve()),
                   "sha256": hashlib.sha256(capture.read_bytes()).hexdigest(),
                   "messages_sha256": hashlib.sha256(built.stdout).hexdigest(),
                   "cargo_fresh": fresh}
        with Path(request["receipt"]).open("x") as output:
            json.dump(receipt, output, sort_keys=True)
        return 0
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


def managed_environment(root, private, target_name, capabilities):
    request = {"root": str(root), "capabilities": capabilities,
               "source_identity": source_identity(root), "target_name": target_name,
               "real_cargo": str(real_cargo()), "capture": str(private / "module.wasm"),
               "receipt": str(private / "receipt.json")}
    request_path, delegate = private / "request.json", private / "cargo-capture"
    request_path.write_text(json.dumps(request, sort_keys=True))
    delegate.write_text(f"#!{sys.executable}\nimport runpy,sys\n"
                        f"sys.path.insert(0, {str(Path(__file__).resolve().parent)!r})\n"
                        f"module=runpy.run_path({str(Path(__file__).resolve())!r})\n"
                        "raise SystemExit(module['managed_delegate'](sys.argv[1:]))\n")
    delegate.chmod(0o700)
    environment = os.environ.copy()
    environment.update(CARGO_REAL=str(delegate), PURRDF_WASM_CAPTURE_REQUEST=str(request_path))
    return environment, request


def managed_capture_bytes(messages, request):
    """Validate immutable captured bytes after the outer wrapper also succeeded."""
    records = [json.loads(line) for line in messages.splitlines() if line.strip()]
    unit = selected_unit(records, request["target_name"])
    modules = [str(Path(name).resolve()) for name in unit.get("filenames", [])
               if Path(name).suffix == ".wasm"]
    if len(modules) != 1:
        raise ValueError("expected exactly one registered WebAssembly output")
    capture = Path(request["capture"])
    receipt = json.loads(Path(request["receipt"]).read_text())
    expected = {"schema": "purrdf-stage-wasm-capture-v1",
                "source_identity": request["source_identity"], "target_name": request["target_name"],
                "registered_output": modules[0],
                "slot_lock": str(slot_lock(Path(modules[0]), request["capabilities"])),
                "capture": str(capture.resolve()),
                "messages_sha256": hashlib.sha256(messages.encode()).hexdigest(),
                "cargo_fresh": unit["fresh"]}
    if (set(receipt) != set(expected) | {"sha256"}
            or any(receipt[key] != value for key, value in expected.items())
            or capture.is_symlink()
            or capture.resolve().parent != Path(request["receipt"]).resolve().parent):
        raise ValueError("WebAssembly capture receipt does not match the successful build")
    payload = capture.read_bytes()
    if (hashlib.sha256(payload).hexdigest() != receipt["sha256"]
            or source_identity(Path(request["root"])) != request["source_identity"]):
        raise ValueError("WebAssembly captured bytes or compiler source identity changed")
    return payload, receipt


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

    def managed_fixture(self, root):
        (root / "Cargo.toml").write_text("[workspace]\n")
        private = root / "capture"
        private.mkdir()
        target = root / "slots/0123456789abcdef/0/target"
        target.mkdir(parents=True)
        lock = root / "locks/0123456789abcdef/0.lock"
        lock.parent.mkdir(parents=True)
        lock.touch()
        producer = root / "actual-cargo"
        producer.write_text(f"#!{sys.executable}\n" + '''import json,os,pathlib,sys
request=json.loads(pathlib.Path(os.environ['PURRDF_WASM_CAPTURE_REQUEST']).read_text())
root=pathlib.Path(request['root'])
(root/'observed-argv.json').write_text(json.dumps(sys.argv[1:]))
mode=os.environ.get('FAKE_CARGO_MODE','fresh')
target=pathlib.Path(os.environ['CARGO_TARGET_DIR'])
output=target/'wasm32-unknown-unknown/release/selected.wasm'
output.parent.mkdir(parents=True,exist_ok=True)
output.write_bytes(b'\\0asm\\x01\\0\\0\\0current')
if mode=='changed': (root/'Cargo.toml').write_text('changed compiler input')
if mode=='outside': output=root/'outside.wasm'; output.write_bytes(b'\\0asm\\x01\\0\\0\\0outside')
print(json.dumps({'reason':'compiler-artifact','target':{'name':'selected'},
                  'fresh':mode=='cached','filenames':[str(output)]}))
print(json.dumps({'reason':'build-finished','success':mode!='failed'}))
sys.exit(17 if mode=='failed' else 0)
''')
        producer.chmod(0o700)
        capabilities = {"version": 1, "slots": str(root / "slots"), "locks": str(root / "locks")}
        with mock.patch.dict(os.environ, {"CARGO_REAL": str(producer)}):
            environment, request = managed_environment(root, private, "selected", capabilities)
        environment["CARGO_TARGET_DIR"] = str(target)
        return environment, request, target, lock

    @unittest.skipUnless(sys.platform.startswith("linux"), "Stage leases use Linux flock")
    def test_managed_capture_survives_slot_reuse_and_refuses_receipt_tampering(self):
        import fcntl
        with tempfile.TemporaryDirectory() as temporary:
            environment, request, target, lock = self.managed_fixture(Path(temporary))
            environment["FAKE_CARGO_MODE"] = "cached"
            arguments = ["--config", 'build.rustc-wrapper="kache"', "rustc", "-p", "selected", "--release"]
            with lock.open("rb") as lease:
                fcntl.flock(lease.fileno(), fcntl.LOCK_EX)
                built = subprocess.run([environment["CARGO_REAL"], *arguments], env=environment,
                                       capture_output=True, text=True, check=False)
            self.assertEqual(built.returncode, 0, built.stderr)
            self.assertEqual(json.loads((Path(temporary) / "observed-argv.json").read_text()), arguments)
            (target / "wasm32-unknown-unknown/release/selected.wasm").write_bytes(b"next unrelated invocation")
            payload, receipt = managed_capture_bytes(built.stdout, request)
            self.assertEqual(payload, b"\0asm\x01\0\0\0current")
            self.assertTrue(receipt["cargo_fresh"])
            receipt_path = Path(request["receipt"])
            original = receipt_path.read_text()
            for field, value in [("slot_lock", "/wrong/0.lock"), ("target_name", "wrong"),
                                 ("messages_sha256", "0" * 64), ("sha256", "0" * 64)]:
                with self.subTest(field=field):
                    changed = json.loads(original)
                    changed[field] = value
                    receipt_path.write_text(json.dumps(changed))
                    with self.assertRaises(ValueError):
                        managed_capture_bytes(built.stdout, request)
            receipt_path.write_text(original)
            Path(request["capture"]).write_bytes(b"tampered capture")
            with self.assertRaisesRegex(ValueError, "captured bytes"):
                managed_capture_bytes(built.stdout, request)

    @unittest.skipUnless(sys.platform.startswith("linux"), "Stage leases use Linux flock")
    def test_failed_unleased_outside_or_source_changed_managed_build_is_refused(self):
        import fcntl
        for mode, leased, expected in [("failed", True, 17), ("outside", True, 1),
                                       ("changed", True, 1), ("fresh", False, 1)]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                environment, request, _, lock = self.managed_fixture(Path(temporary))
                environment["FAKE_CARGO_MODE"] = mode
                with lock.open("rb") as lease:
                    if leased:
                        fcntl.flock(lease.fileno(), fcntl.LOCK_EX)
                    built = subprocess.run([environment["CARGO_REAL"], "rustc", "-p", "selected"],
                                           env=environment, capture_output=True, text=True, check=False)
                self.assertEqual(built.returncode, expected, built.stderr)
                self.assertFalse(Path(request["capture"]).exists())
                self.assertFalse(Path(request["receipt"]).exists())


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
        capabilities = stage_capabilities()
        environment, request = (managed_environment(Path(__file__).resolve().parents[1],
                                                    private_target, target_name, capabilities)
                                if capabilities else (None, None))
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
                env=environment,
                check=True,
            )
        messages = build_log.read_text()
        if request:
            payload, receipt = managed_capture_bytes(messages, request)
            with output.open("xb") as destination:
                destination.write(payload)
            receipt["captured_output"] = str(output)
        else:
            records = [json.loads(line) for line in messages.splitlines()]
            registered, fresh = validate_capture(records, target_name, private_target)
            with registered.open("rb") as source, output.open("xb") as destination:
                shutil.copyfileobj(source, destination)
            receipt = {"schema": "purrdf-private-wasm-capture-v1",
                       "registered_output": str(registered),
                       "private_target_directory": str(private_target),
                       "captured_output": str(output),
                       "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
                       "cargo_fresh": fresh}
        output.with_suffix(".capture.json").write_text(
            json.dumps(receipt, indent=2) + "\n"
        )


if __name__ == "__main__":
    main()
