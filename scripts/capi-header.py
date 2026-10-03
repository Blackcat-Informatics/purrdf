#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Generate or verify the header of the exact cargo-c build being inspected."""

import argparse
import difflib
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

from compiler_capture import held_slot, real_cargo, source_identity, stage_capabilities








def header_stamp(header: Path) -> list[int]:
    """Observe file replacement/write evidence before the slot is released."""
    stat = header.stat()
    return [stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns]


def capture_header(messages: str, request: dict, prior_headers: dict[str, list[int]]) -> None:
    """Copy under the outer wrapper's lease; never substitute JSON artifact paths."""
    root, manifest = Path(request["root"]), Path(request["manifest"])
    if source_identity(root) != request["source_identity"]:
        raise ValueError("compiler/header source identity changed during cargo-c build")
    header = generated_header(messages, manifest).resolve()
    lock = held_slot(header, request["capabilities"])
    before, after = prior_headers.get(str(header)), header_stamp(header)
    if before == after:
        raise ValueError("successful cargo-c build did not refresh its reported header")
    current = header.read_bytes()
    capture = Path(request["capture"]).resolve()
    with capture.open("xb") as output:
        output.write(current)
    if source_identity(root) != request["source_identity"]:
        raise ValueError("compiler/header source identity changed during header capture")
    receipt = {
        "version": 1, "manifest": str(manifest.resolve()),
        "source_header": str(header), "slot_lock": str(lock),
        "source_identity": request["source_identity"], "capture": str(capture),
        "header_sha256": hashlib.sha256(current).hexdigest(),
        "messages_sha256": hashlib.sha256(messages.encode()).hexdigest(),
        "header_before": before, "header_after": after,
    }
    with Path(request["receipt"]).open("x") as output:
        json.dump(receipt, output, sort_keys=True)


def captured_bytes(messages: str, request: dict) -> tuple[bytes, Path]:
    """Validate the private receipt after the outer wrapper also succeeded."""
    receipt = json.loads(Path(request["receipt"]).read_text())
    expected = {
        "version": 1, "manifest": str(Path(request["manifest"]).resolve()),
        "source_header": str(generated_header(messages, Path(request["manifest"])).resolve()),
        "source_identity": request["source_identity"],
        "capture": str(Path(request["capture"]).resolve()),
        "messages_sha256": hashlib.sha256(messages.encode()).hexdigest(),
    }
    if (set(receipt) != set(expected) | {"slot_lock", "header_sha256", "header_before", "header_after"}
            or any(receipt[key] != value for key, value in expected.items())):
        raise ValueError("cargo-c capture receipt does not match the successful build")
    if (not isinstance(receipt["header_after"], list) or len(receipt["header_after"]) != 5
            or not all(type(value) is int for value in receipt["header_after"])
            or (receipt["header_before"] is not None
                and (not isinstance(receipt["header_before"], list)
                     or len(receipt["header_before"]) != 5
                     or not all(type(value) is int for value in receipt["header_before"])))
            or receipt["header_before"] == receipt["header_after"]):
        raise ValueError("cargo-c capture receipt has no fresh header evidence")
    # No mutable slot file is read after the lease returns. The receipt must
    # name the same slot lock, but its former held state cannot be re-probed.
    relative = Path(receipt["source_header"]).relative_to(request["capabilities"]["slots"])
    lock = Path(request["capabilities"]["locks"]) / relative.parts[0] / f"{relative.parts[1]}.lock"
    if receipt["slot_lock"] != str(lock.resolve()):
        raise ValueError("cargo-c capture receipt names a different slot lease")
    capture = Path(request["capture"])
    if capture.is_symlink() or capture.resolve().parent != Path(request["receipt"]).resolve().parent:
        raise ValueError("cargo-c header capture escaped its private directory")
    current = capture.read_bytes()
    if (hashlib.sha256(current).hexdigest() != receipt["header_sha256"]
            or source_identity(Path(request["root"])) != request["source_identity"]):
        raise ValueError("cargo-c captured bytes or source identity changed")
    return current, Path(receipt["source_header"])


def capture_delegate(arguments: list[str]) -> int:
    """CARGO_REAL delegate: preserve all policy/probes and capture before return."""
    request = json.loads(Path(os.environ["PURRDF_CAPI_CAPTURE_REQUEST"]).read_text())
    real_cargo = request["real_cargo"]
    environment = os.environ.copy()
    environment["CARGO_REAL"] = real_cargo
    if not any(arguments[index:index + 2] == ["capi", "build"]
               for index in range(len(arguments) - 1)):
        os.execve(real_cargo, [real_cargo, *arguments], environment)
    if source_identity(Path(request["root"])) != request["source_identity"]:
        print("FAIL: compiler/header source identity changed before cargo-c build", file=sys.stderr)
        return 1
    try:
        # The outer wrapper already established and leased this target. Observe
        # existing headers before cargo-c runs, so success cannot reuse stale
        # header bytes left by another invocation in the reusable slot.
        target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
        held_slot(target / "debug/include/purrdf/purrdf.h", request["capabilities"])
        prior_headers = {str(path.resolve()): header_stamp(path)
                         for path in target.rglob("purrdf.h") if path.is_file()}
    except (KeyError, OSError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    result = subprocess.run([real_cargo, *arguments], env=environment,
                            stdout=subprocess.PIPE, check=False)
    sys.stdout.buffer.write(result.stdout)
    sys.stdout.buffer.flush()
    if result.returncode:
        return result.returncode
    try:
        capture_header(result.stdout.decode(), request, prior_headers)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


def delegate_environment(root: Path, private: Path, capabilities: dict) -> tuple[dict, dict]:
    """Install an invocation-local documented CARGO_REAL delegate, not a bypass."""
    real = real_cargo()
    request = {
        "root": str(root), "manifest": str(root / "crates/rdf-capi/Cargo.toml"),
        "capabilities": capabilities, "source_identity": source_identity(root),
        "real_cargo": str(real), "capture": str(private / "purrdf.h"),
        "receipt": str(private / "receipt.json"),
    }
    request_path, delegate = private / "request.json", private / "cargo-capture"
    request_path.write_text(json.dumps(request, sort_keys=True))
    delegate.write_text(f"#!{sys.executable}\nimport runpy,sys\n"
                        f"sys.path.insert(0, {str(Path(__file__).resolve().parent)!r})\n"
                        f"module=runpy.run_path({str(Path(__file__).resolve())!r})\n"
                        "raise SystemExit(module['capture_delegate'](sys.argv[1:]))\n")
    delegate.chmod(0o700)
    environment = os.environ.copy()
    environment.update(CARGO_REAL=str(delegate), PURRDF_CAPI_CAPTURE_REQUEST=str(request_path))
    return environment, request


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
    def capture_fixture(self, root: Path) -> tuple[dict, dict, Path, Path]:
        """An actual delegate process and separately locked reusable compiler slot."""
        manifest = root / "crates/rdf-capi/Cargo.toml"
        manifest.parent.mkdir(parents=True)
        manifest.write_text("[package]\nname='purrdf-capi'\n")
        (root / "Cargo.toml").write_text("[workspace]\n")
        private, slots, locks = root / "private", root / "slots", root / "locks"
        private.mkdir()
        target = slots / "0123456789abcdef/0/target"
        header = target / "debug/include/purrdf/purrdf.h"
        header.parent.mkdir(parents=True)
        header.write_text("stale ABI header")
        lock = locks / "0123456789abcdef/0.lock"
        lock.parent.mkdir(parents=True)
        lock.touch()
        producer = root / "actual-cargo"
        producer.write_text(f"#!{sys.executable}\n" + '''import json,os,pathlib,sys
request=json.loads(pathlib.Path(os.environ['PURRDF_CAPI_CAPTURE_REQUEST']).read_text())
pathlib.Path(request['root'],'observed-argv.json').write_text(json.dumps(sys.argv[1:]))
target=pathlib.Path(os.environ['CARGO_TARGET_DIR'])
header=target/'debug/include/purrdf/purrdf.h'
mode=os.environ.get('FAKE_CARGO_MODE','fresh')
if mode=='changed':
    pathlib.Path(request['root'],'Cargo.toml').write_text('changed compiler input')
if mode not in {'stale','failed'}:
    header.write_text('fresh ABI header from cargo-c')
print(json.dumps({'reason':'compiler-artifact','manifest_path':request['manifest'],
                  'target':{'kind':['cdylib']},'filenames':[str(target/'debug/libpurrdf.so')]}))
print(json.dumps({'reason':'build-finished','success':True}))
sys.exit(17 if mode=='failed' else 0)
''')
        producer.chmod(0o700)
        capabilities = {"version": 1, "slots": str(slots), "locks": str(locks)}
        with mock.patch.dict(os.environ, {"CARGO_REAL": str(producer)}):
            environment, request = delegate_environment(root, private, capabilities)
        environment["CARGO_TARGET_DIR"] = str(target)
        return environment, request, header, lock

    @unittest.skipUnless(sys.platform.startswith("linux"), "Stage's leased-slot path is Linux-only")
    def test_delegate_captures_before_slot_reuse_and_validates_exact_receipt(self):
        import fcntl
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            environment, request, header, lock = self.capture_fixture(root)
            arguments = ["--config", 'build.rustc-wrapper="kache"', "capi", "build",
                         "-p", "purrdf-capi", "--message-format=json"]
            with lock.open("rb") as lease:
                fcntl.flock(lease.fileno(), fcntl.LOCK_EX)
                built = subprocess.run([environment["CARGO_REAL"], *arguments],
                                       env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(built.returncode, 0, built.stderr)
            self.assertEqual(json.loads((root / "observed-argv.json").read_text()), arguments)
            # Another invocation now owns the released slot. Never read its
            # header: the captured immutable copy still proves our build bytes.
            header.write_text("next invocation's different ABI")
            self.assertEqual(captured_bytes(built.stdout, request)[0], b"fresh ABI header from cargo-c")
            receipt_path = Path(request["receipt"])
            original = receipt_path.read_text()
            for field, wrong in [("manifest", "/wrong/Cargo.toml"),
                                 ("source_header", "/wrong/purrdf.h"),
                                 ("slot_lock", "/wrong/0.lock"),
                                 ("header_sha256", "0" * 64),
                                 ("messages_sha256", "0" * 64),
                                 ("header_after", None)]:
                with self.subTest(field=field):
                    receipt = json.loads(original)
                    receipt[field] = wrong
                    receipt_path.write_text(json.dumps(receipt))
                    with self.assertRaises((ValueError, OSError)):
                        captured_bytes(built.stdout, request)
            receipt_path.write_text(original)
            Path(request["capture"]).write_bytes(b"modified private capture")
            with self.assertRaisesRegex(ValueError, "captured bytes"):
                captured_bytes(built.stdout, request)

    @unittest.skipUnless(sys.platform.startswith("linux"), "Stage's leased-slot path is Linux-only")
    def test_failed_unleased_stale_or_source_changed_build_never_publishes_a_receipt(self):
        import fcntl
        for mode, leased, expected in [("failed", True, 17), ("stale", True, 1),
                                       ("changed", True, 1), ("fresh", False, 1)]:
            with self.subTest(mode=mode, leased=leased), tempfile.TemporaryDirectory() as temporary:
                environment, request, _, lock = self.capture_fixture(Path(temporary))
                environment["FAKE_CARGO_MODE"] = mode
                with lock.open("rb") as lease:
                    if leased:
                        fcntl.flock(lease.fileno(), fcntl.LOCK_EX)
                    built = subprocess.run([environment["CARGO_REAL"], "capi", "build"],
                                           env=environment, capture_output=True, text=True, check=False)
                self.assertEqual(built.returncode, expected, built.stderr)
                self.assertFalse(Path(request["receipt"]).exists())
                self.assertFalse(Path(request["capture"]).exists())

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
    # Ordinary Cargo compiles in a fresh private directory. Stage deliberately
    # overrides that directory: its documented CARGO_REAL delegate captures
    # while the normal outer wrapper still holds the exact slot's lease. Both
    # paths retain ordinary build errors and outer publication failure status.
    with tempfile.TemporaryDirectory(prefix="purrdf-header-") as temporary:
        private = Path(temporary)
        private_build = private / "build"
        try:
            capabilities = stage_capabilities()
            environment, request = (delegate_environment(root, private, capabilities)
                                    if capabilities else (None, None))
        except (OSError, ValueError, json.JSONDecodeError) as error:
            print(f"FAIL: {error}", file=sys.stderr)
            return 1
        built = subprocess.run(
            [
                "cargo", "capi", "build", "-p", "purrdf-capi",
                "--target-dir", str(private_build), "--message-format=json",
            ],
            cwd=root,
            env=environment,
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
            if request is not None:
                current, generated = captured_bytes(built.stdout, request)
            else:
                generated = generated_header(
                    built.stdout, root / "crates" / "rdf-capi" / "Cargo.toml", private_build
                )
                current = generated.read_bytes()
        except (OSError, ValueError, json.JSONDecodeError) as error:
            print(f"FAIL: {error}", file=sys.stderr)
            return 1
    committed = root / (args.write or args.check)
    if args.write:
        committed.write_bytes(current)
        print(f"regenerated {committed.relative_to(root)} from the captured cargo-c build")
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
    print("OK: committed purrdf.h matches the captured cargo-c artifact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
