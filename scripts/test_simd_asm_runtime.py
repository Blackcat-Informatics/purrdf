# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Evidence reuse must preserve refusals, graph membership and complete coverage."""

import argparse
import contextlib
import dataclasses
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
from types import ModuleType
import unittest
from unittest.mock import patch

import simd_asm_runtime as runtime

GATE: ModuleType | None = None


def loaded_gate() -> ModuleType:
    """The gate module the tests run against, which `run_tests` or `__main__` loads."""
    if GATE is None:
        raise RuntimeError("the gate module is not loaded; run through check-simd-asm.py --self-test")
    return GATE


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="purrdf-asm-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.gate = loaded_gate()
        self.config = self.gate.CONFIG_BY_NAME["x86_64"]
        data = self.gate._manifest_dict()
        data["site"][0]["measure"][0]["min_vector_ops"] = 1
        self.manifest = self.gate.load_manifest(data)
        self.runner = runtime.Runner.__new__(runtime.Runner)
        self.runner.gate = self.gate
        self.runner.options = argparse.Namespace(probe=None)
        self.runner.compiler = "fixture compiler"
        self.runner.stage = None
        self.runner.root = self.root
        self.runner.nonce = ""
        self.runner.stopped = threading.Event()
        self.runner.lock = threading.Lock()
        self.runner.children = set()
        self.runner.cargo_jobs = 1
        self.runner.timings = {}
        self.fresh = False
        self.flags = self.config.rustflags()
        self.asm = self.gate._X86_PACKED
        self.runner.execute = self.execute

    def execute(self, cmd, env, lease):
        self.context_root = Path(env["CARGO_TARGET_DIR"]).parent
        folder = self.context_root / "build" / self.config.triple / "release" / "deps"
        folder.mkdir(parents=True, exist_ok=True)
        self.path = folder / "demo-1111111111111111.s"
        if not self.fresh:
            self.path.write_text(self.asm)
        artifact = dict(reason="compiler-artifact", package_id="demo", fresh=self.fresh,
                        target=dict(kind=["lib"], name="demo"), features=[],
                        filenames=[str(folder / "libdemo-1111111111111111.rlib")])
        line = f"Running `rustc --crate-name demo --target {self.config.triple} {self.flags}`"
        return subprocess.CompletedProcess(cmd, 0, json.dumps(artifact), "" if self.fresh else line)

    def run_config(self):
        with patch.object(self.gate, "host_triple", return_value=self.config.triple), contextlib.redirect_stdout(io.StringIO()):
            return self.runner.configuration(self.config, self.manifest, self.gate.manifest_keep(self.manifest)(self.config))

    def test_warm_run_rechecks_evidence_without_compiling_or_parsing(self):
        cold = self.run_config()
        self.fresh = True
        warm = self.run_config()
        self.assertEqual(cold, warm)
        self.assertEqual(self.runner.timings[self.config.name]["rustc_invocations"], 0)
        self.assertEqual(self.runner.timings[self.config.name]["reused_units"], 1)

    def test_missing_or_changed_assembly_is_not_a_cache_hit(self):
        self.run_config()
        self.fresh = True
        self.path.write_text(self.gate._X86_SCALAR)
        with self.assertRaisesRegex(self.gate.GateError, "provenance"):
            self.run_config()
        self.path.unlink()
        with self.assertRaisesRegex(self.gate.GateError, "no .s"):
            self.run_config()

    def test_missing_receipt_and_wrong_flags_fail(self):
        self.run_config()
        for path in (self.context_root / "evidence").glob("*.json"):
            path.unlink()
        self.fresh = True
        with self.assertRaisesRegex(self.gate.GateError, "provenance"):
            self.run_config()
        self.fresh = False
        self.flags = self.flags.replace("target-cpu=x86-64", "target-cpu=native")
        with self.assertRaisesRegex(self.gate.GateError, "target-cpu"):
            self.run_config()

    def test_corrupt_parse_cache_is_reparsed(self):
        expected = self.run_config()
        for path in (self.context_root / "parsed").glob("*.json"):
            path.write_text('{"functions": [], "digest": "wrong"}')
        self.fresh = True
        self.assertEqual(expected, self.run_config())
        self.assertEqual(self.runner.timings[self.config.name]["parsed_units"], 1)

    def test_changed_source_and_manifest_are_rejudged(self):
        self.run_config()
        self.asm = self.gate._X86_SCALAR
        with self.assertRaisesRegex(self.gate.GateError, "below the floor"):
            self.run_config()
        self.asm = self.gate._X86_PACKED
        self.run_config()
        self.fresh = True
        measure = dataclasses.replace(self.manifest.sites[0].measures[0], min_vector_ops=99999)
        self.manifest = dataclasses.replace(self.manifest, sites=(dataclasses.replace(self.manifest.sites[0], measures=(measure,)),))
        with self.assertRaisesRegex(self.gate.GateError, "below the floor"):
            self.run_config()

    def test_changed_compiler_has_a_separate_context(self):
        self.run_config()
        first = self.context_root
        self.runner.compiler = "other compiler"
        self.run_config()
        self.assertNotEqual(first, self.context_root)

    def test_context_lease_spans_analysis_and_cancellation_releases_it(self):
        first_entered = threading.Event()
        release = threading.Event()
        second_entered = threading.Event()

        def first():
            with self.runner.context({"lease": "fixture"}):
                first_entered.set()
                release.wait(5)

        def second():
            with self.runner.context({"lease": "fixture"}):
                second_entered.set()

        left = threading.Thread(target=first)
        right = threading.Thread(target=second)
        left.start()
        self.assertTrue(first_entered.wait(2))
        right.start()
        self.assertFalse(second_entered.wait(0.1))
        release.set()
        left.join(2)
        right.join(2)
        self.assertTrue(second_entered.is_set())
        with self.assertRaises(KeyboardInterrupt):
            with self.runner.context({"lease": "fixture"}):
                raise KeyboardInterrupt()
        with self.runner.context({"lease": "fixture"}):
            pass

    def test_cancellation_stops_owned_process_and_releases_lease(self):
        results = []
        with self.runner.context({"process": "fixture"}) as (root, _, lease):
            thread = threading.Thread(target=lambda: results.append(runtime.Runner.execute(
                self.runner, [sys.executable, "-c", "import time; time.sleep(30)"],
                {**os.environ, "CARGO_TARGET_DIR": str(root / "target")}, lease)))
            thread.start()
            deadline = time.monotonic() + 5
            while not self.runner.children and time.monotonic() < deadline:
                time.sleep(0.01)
            self.runner.stop()
            thread.join(5)
            self.assertFalse(thread.is_alive())
            self.assertEqual(len(results), 1)
            self.assertLess(results[0].returncode, 0)
            self.assertFalse(self.runner.children)
        with self.runner.context({"process": "fixture"}):
            pass

    def test_every_failing_configuration_is_reported(self):
        gate = self.gate

        class Stub:
            workers = 7
            compiler = "fixture compiler"
            timings = {}

            def __init__(self, *_args):
                pass

            def configuration(self, config, _manifest, _chooser):
                if config.name in ("x86_64", "wasm32-simd128"):
                    raise gate.GateError(f"`demo.dot` on {config.name}: below the floor")
                return []

            def stop(self):
                pass

        options = argparse.Namespace(jobs=7, fresh=False, probe=None)
        with patch.object(runtime, "Runner", Stub), patch.object(gate, "require_targets"):
            with self.assertRaises(gate.GateError) as caught:
                runtime.measure_all(gate, self.manifest, gate.CONFIGS, lambda _c: None, options)
            self.assertEqual(
                str(caught.exception).splitlines(),
                ["`demo.dot` on x86_64: below the floor", "`demo.dot` on wasm32-simd128: below the floor"],
            )
            passing = [c for c in gate.CONFIGS if c.name not in ("x86_64", "wasm32-simd128")]
            self.assertEqual(set(runtime.measure_all(gate, self.manifest, tuple(passing), lambda _c: None, options)),
                             {c.name for c in passing})

    def test_complete_report_set_and_failure_cases(self):
        identity = dict(schema=1, source="source", compiler="compiler", manifest="manifest")
        for name in self.gate.CONFIG_NAMES:
            runtime.atomic_json(self.root / (name + ".json"), dict(identity=identity, status="passed",
                                cells={name: {site.id: "v0·f0·r0" for site in self.manifest.sites}}))
        merged = runtime.merge_reports(self.gate, self.manifest, self.root, identity)
        self.assertEqual(len(merged), 7 * len(self.manifest.sites))
        path = self.root / "x86_64.json"
        original = runtime.read_json(path)
        for mutate in [lambda r: r.update(status="incomplete"),
                       lambda r: r["identity"].update(source="stale"),
                       lambda r: r["cells"]["x86_64"].clear()]:
            bad = json.loads(json.dumps(original))
            mutate(bad)
            runtime.atomic_json(path, bad)
            with self.assertRaises(self.gate.GateError):
                runtime.merge_reports(self.gate, self.manifest, self.root, identity)
        runtime.atomic_json(path, original)
        duplicate = self.root / "duplicate.json"
        runtime.atomic_json(duplicate, original)
        with self.assertRaisesRegex(self.gate.GateError, "duplicate"):
            runtime.merge_reports(self.gate, self.manifest, self.root, identity)
        duplicate.unlink()
        path.unlink()
        with self.assertRaisesRegex(self.gate.GateError, "incomplete"):
            runtime.merge_reports(self.gate, self.manifest, self.root, identity)


def run_tests(gate):
    global GATE
    GATE = gate
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(EvidenceTests))
    return result.wasSuccessful()


class LiveEvidenceTests(unittest.TestCase):
    """Explicit opt-in: compile a tiny crate through the installed Cargo path."""

    def test_real_cargo_freshness_source_edit_and_missing_assembly(self):
        gate = loaded_gate()
        with tempfile.TemporaryDirectory(prefix="purrdf-asm-live-") as temporary:
            root = Path(temporary)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text('[package]\nname="demo"\nversion="0.1.0"\nedition="2024"\n')
            (root / "Cargo.lock").write_text('version=4\n[[package]]\nname="demo"\nversion="0.1.0"\n')
            source = root / "src/lib.rs"
            source.write_text('pub mod kernel { #[inline(never)] pub fn dot(x: u64) -> u64 { x.wrapping_add(1) } }\n')
            manifest = gate.load_manifest(gate._manifest_dict())
            config = gate.CONFIG_BY_NAME["x86_64"]
            options = argparse.Namespace(jobs=1, fresh=False, probe=None)
            with patch.object(gate, "REPO_ROOT", root):
                runner = runtime.Runner(gate, options, 1)
                def run():
                    return runner.configuration(config, manifest, gate.manifest_keep(manifest)(config))
                first = run()
                self.assertGreater(runner.timings[config.name]["rustc_invocations"], 0)
                self.assertEqual(first, run())
                self.assertEqual(runner.timings[config.name]["rustc_invocations"], 0)
                self.assertGreater(runner.timings[config.name]["reused_units"], 0)
                source.write_text(source.read_text().replace("wrapping_add(1)", "wrapping_add(2)"))
                changed = run()
                self.assertNotEqual(first[0].instructions, changed[0].instructions)
                self.assertGreater(runner.timings[config.name]["rustc_invocations"], 0)
                Path(changed[0].unit).unlink()
                with self.assertRaisesRegex(gate.GateError, "no .s"):
                    run()
                runner.nonce = "a" * 32
                self.assertEqual(changed[0].instructions, run()[0].instructions)
                runner = runtime.Runner(gate, options, 1)
                self.assertEqual(changed[0].instructions, run()[0].instructions)
                self.assertEqual(runner.timings[config.name]["rustc_invocations"], 0)


if __name__ == "__main__":
    import importlib.util
    import sys
    spec = importlib.util.spec_from_file_location("asm_test_gate", Path(__file__).with_name("check-simd-asm.py"))
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load check-simd-asm.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    GATE = module
    suite = LiveEvidenceTests if "--live" in sys.argv else EvidenceTests
    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(suite))
    sys.exit(not result.wasSuccessful())
