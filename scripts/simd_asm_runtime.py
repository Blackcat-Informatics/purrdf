# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Persistent, leased build evidence for the assembly gate; no verdicts are cached."""

from __future__ import annotations

import concurrent.futures
import contextlib
import dataclasses
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import threading
import time
import uuid

SCHEMA = 1


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(dir=path.parent, prefix=".pending-")
    try:
        with os.fdopen(fd, "w") as out:
            json.dump(value, out, sort_keys=True)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return None


def source_identity(root):
    """Tracked content plus untracked compiler/script inputs, including dirty work."""
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=root,
    ).split(b"\0")
    h = hashlib.sha256()
    for name in sorted(set(names)):
        if not name:
            continue
        p = root / os.fsdecode(name)
        h.update(name + b"\0")
        h.update(digest(p.read_bytes()).encode() if p.is_file() else b"missing")
    return h.hexdigest()


class Runner:
    """Own child process groups and stop only these builds on interruption/failure."""

    def __init__(self, gate, options, configurations=7):
        self.gate = gate
        self.options = options
        self.stopped = threading.Event()
        self.lock = threading.Lock()
        self.children = set()
        self.compiler = subprocess.check_output(["rustc", "-vV"], text=True)
        capability = subprocess.run(["cargo", "--stage-asm-capabilities"], capture_output=True, text=True)
        self.stage = None
        if capability.returncode == 0:
            self.stage = json.loads(capability.stdout)
            if self.stage != {"version": 1, "slots": "/opt/.cargo/slots", "locks": "/opt/.cargo/.slot-locks"}:
                raise gate.GateError("unrecognized Stage assembly context protocol")
        self.root = Path(os.environ.get("CARGO_TARGET_DIR", str(gate.REPO_ROOT / "target"))) / "simd-asm"
        self.nonce = uuid.uuid4().hex if options.fresh else ""
        budget = subprocess.run(
            ["cargo", "-Z", "unstable-options", "config", "get", "build.jobs"],
            capture_output=True, text=True,
        )
        available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else (os.cpu_count() or 1)
        try:
            configured = int(budget.stdout.split("=", 1)[1].split("#", 1)[0].strip())
            available = min(available, configured if configured > 0 else max(1, available + configured))
        except (ValueError, IndexError):
            pass
        self.workers = min(options.jobs, available, configurations)
        self.cargo_jobs = max(1, available // self.workers)
        self.timings = {}

    def stop(self):
        self.stopped.set()
        with self.lock:
            for child in self.children:
                if child.poll() is None:
                    with contextlib.suppress(ProcessLookupError):
                        os.killpg(child.pid, signal.SIGTERM)

    def execute(self, cmd, env, lease, graph=0):
        directory = Path(env["CARGO_TARGET_DIR"]).parent
        stdout_path = directory / f"cargo.graph-{graph}.stdout.jsonl"
        stderr_path = directory / f"cargo.graph-{graph}.stderr.log"
        with self.lock:
            if self.stopped.is_set():
                raise self.gate.GateError("assembly run cancelled")
            with stdout_path.open("w") as stdout, stderr_path.open("w") as stderr:
                child = subprocess.Popen(
                    cmd, cwd=self.gate.REPO_ROOT, env=env, stdout=stdout,
                    stderr=stderr, start_new_session=True, pass_fds=(lease,),
                )
            self.children.add(child)
        try:
            child.wait()
            return subprocess.CompletedProcess(cmd, child.returncode, stdout_path.read_text(), stderr_path.read_text())
        finally:
            with self.lock:
                self.children.discard(child)

    @contextlib.contextmanager
    def context(self, identity):
        key = digest(encoded(identity))
        if self.stage:
            workspace = digest(f"{self.gate.REPO_ROOT.resolve()}\n{key}".encode())[:16]
            root = Path(self.stage["slots"]) / workspace / "0"
            lock = Path(self.stage["locks"]) / workspace / "0.lock"
        else:
            root = self.root / "contexts" / key
            lock = root / "lease.lock"
        lock.parent.mkdir(parents=True, exist_ok=True)
        with lock.open("a+") as lease:
            while True:
                try:
                    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if self.stopped.wait(0.1):
                        raise self.gate.GateError("assembly run cancelled while waiting for its context")
            root.mkdir(parents=True, exist_ok=True)
            if self.stage:
                (root / ".stage-asm-context").write_text(key)
            yield root, key, lease.fileno()

    def configuration(self, config, manifest, chooser):
        gate = self.gate
        start = time.monotonic()
        env = gate.config_env(config, dict(os.environ))
        stats = dict(build_seconds=0.0, analysis_seconds=0.0, lock_seconds=0.0,
                     rustc_invocations=0, fresh_artifacts=0, parsed_units=0, reused_units=0)
        functions = []
        seen = set()
        parser_key = digest(Path(gate.__file__).read_bytes() + Path(__file__).read_bytes())
        selectors = [(m.crate, m.symbol) for s in manifest.sites for m in s.measures if config.name in m.configs]
        print(f"== {config.name}: {self.cargo_jobs} Cargo jobs", flush=True)
        commands = gate.build_commands(config, manifest.packages, manifest.rlib_packages)
        identity = dict(schema=SCHEMA, compiler=self.compiler, config=dataclasses.asdict(config),
                        commands=commands, fresh="",
                        environment={k: v for k, v in env.items() if k.startswith(("CARGO_PROFILE_", "CARGO_TARGET_", "RUST", "CC", "AR", "CFLAGS", "CXXFLAGS"))})
        # Sequential graphs reuse Cargo's compatible units, but every graph still
        # supplies its own membership and logs. One lease protects their shared
        # artifacts and receipts until the entire configuration is judged.
        # A fresh generation becomes the default for subsequent runs too.
        generation_path = self.root / "generations" / (digest(encoded(identity)) + ".json")
        if self.nonce:
            atomic_json(generation_path, self.nonce)
        generation = read_json(generation_path)
        if generation is not None:
            if not isinstance(generation, str) or len(generation) != 32 or any(c not in "0123456789abcdef" for c in generation):
                raise gate.GateError("invalid assembly generation pointer; rerun with --fresh")
            identity["fresh"] = generation
        waiting = time.monotonic()
        with self.context(identity) as (root, key, lease):
            stats["lock_seconds"] += time.monotonic() - waiting
            child_env = dict(env, CARGO_TARGET_DIR=str(root / "target"), CARGO_BUILD_BUILD_DIR=str(root / "build"))
            if self.stage:
                child_env.update(STAGE_CARGO_ASM_CONTEXT=key, STAGE_CARGO_ASM_LEASE_FD=str(lease))
            for graph, cmd in enumerate(commands):
                actual_cmd = [*cmd, "--jobs", str(self.cargo_jobs), "--timings",
                              "--config", f'build.target-dir={json.dumps(str(root / "target"))}',
                              "--config", f'build.build-dir={json.dumps(str(root / "build"))}']
                log_path = root / f"cargo.graph-{graph}.stderr.log"
                print(f"== {config.name} graph {graph}: {log_path}", flush=True)
                t = time.monotonic()
                proc = self.execute(actual_cmd, child_env, lease, graph)
                stats["build_seconds"] += time.monotonic() - t
                if proc.returncode:
                    raise gate.GateError(f"{config.name}: build failed; log: {log_path}\n{gate.failure_tail(proc.stderr)}")
                problems = gate.verify_command_lines(proc.stderr, config)
                if problems:
                    raise gate.GateError("\n".join(problems))
                lines = [line for line in proc.stderr.splitlines() if gate._RUNNING.match(line)
                         and "--crate-name" in line and f"--target {config.triple}" in line]
                stats["rustc_invocations"] += len(lines)
                messages = [json.loads(line) for line in proc.stdout.splitlines() if line.startswith("{")]
                artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                             and set(m["target"]["kind"]) & {"lib", "rlib"}
                             and any(f"/{config.triple}/" in f for f in m["filenames"])]
                t = time.monotonic()
                for artifact in artifacts:
                    path = gate.asm_paths(json.dumps(artifact), config)[0]
                    data = path.read_bytes()
                    asm_hash = digest(data)
                    receipt_path = root / "evidence" / (digest(str(path).encode()) + ".json")
                    receipt = read_json(receipt_path)
                    name = artifact["target"]["name"].replace("-", "_")
                    invocations = [line for line in lines if f"--crate-name {name} " in line]
                    if artifact.get("fresh"):
                        stats["fresh_artifacts"] += 1
                        if not isinstance(receipt, dict) or receipt.get("assembly") != asm_hash or receipt.get("context") != key:
                            raise gate.GateError(f"{config.name}: missing/changed assembly provenance for {path}; rerun with --fresh")
                        invocations = receipt.get("commands", [])
                    if not invocations or gate.verify_command_lines("\n".join(invocations), config):
                        raise gate.GateError(f"{config.name}: no verified compiler invocation for {path}; rerun with --fresh")
                    atomic_json(receipt_path, dict(context=key, assembly=asm_hash, commands=invocations))
                    # Cargo may repeat an artifact message within one graph. A
                    # reused unit belongs to every graph reporting it: dropping
                    # that membership would hide a second copy in a later graph.
                    unit = (graph, artifact["package_id"], name, tuple(artifact.get("features", [])), str(path), asm_hash)
                    if unit in seen:
                        continue
                    seen.add(unit)
                    cache_key = digest(encoded([parser_key, config.arch, selectors, asm_hash]))
                    cache_path = root / "parsed" / (cache_key + ".json")
                    cached = read_json(cache_path) if not self.options.probe else None
                    try:
                        if not isinstance(cached, dict) or cached.get("digest") != digest(encoded(cached["functions"])):
                            raise ValueError("missing/corrupt parse cache")
                        parsed = [gate.Function(f["symbol"], f["path"], f["crate"], str(path),
                                  [gate.Instruction(i["mnemonic"], tuple(i["operands"])) for i in f["instructions"]], graph)
                                  for f in cached["functions"]]
                        os.utime(cache_path, None)
                        stats["reused_units"] += 1
                    except (ValueError, KeyError, TypeError):
                        parsed = gate.collect_functions([(str(path), data.decode("utf-8", errors="replace"), graph)],
                                                        config.arch, chooser.keep, chooser.select)
                        payload = [dataclasses.asdict(f) for f in parsed]
                        if not self.options.probe:
                            atomic_json(cache_path, dict(functions=payload, digest=digest(encoded(payload))))
                        stats["parsed_units"] += 1
                    functions.extend(parsed)
                stats["analysis_seconds"] += time.monotonic() - t
            if not self.options.probe:
                problems = [p for r in gate.evaluate_config(manifest, config, functions) for p in r.problems]
                if problems:
                    raise gate.GateError("\n".join(problems))
        stats["seconds"] = time.monotonic() - start
        self.timings[config.name] = stats
        print(f"== {config.name}: " + json.dumps(stats, sort_keys=True), flush=True)
        return functions


def measure_all(gate, manifest, configs, keep_for, options):
    gate.refuse_overriding_env(dict(os.environ))
    gate.require_targets(configs)
    runner = Runner(gate, options, len(configs))
    options.compiler = runner.compiler
    pool = concurrent.futures.ThreadPoolExecutor(max_workers=runner.workers)
    futures = {}
    previous = signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    try:
        for config in configs:
            futures[pool.submit(runner.configuration, config, manifest, keep_for(config))] = config.name
        # Every configuration runs to its verdict: a failing one does not cancel the
        # others, so one run reports the problems of the whole matrix at once.
        out = {}
        failures = {}
        for future in concurrent.futures.as_completed(futures):
            try:
                out[futures[future]] = future.result()
            except gate.GateError as err:
                failures[futures[future]] = str(err)
        if failures:
            raise gate.GateError("\n".join(failures[name] for name in gate.CONFIG_NAMES if name in failures))
        options.timings = runner.timings
        return out
    finally:
        runner.stop()
        pool.shutdown(wait=True, cancel_futures=True)
        signal.signal(signal.SIGTERM, previous)


def report_identity(gate):
    return dict(schema=SCHEMA, source=source_identity(gate.REPO_ROOT),
                manifest=digest(gate.MANIFEST.read_bytes()),
                compiler=subprocess.check_output(["rustc", "-vV"], text=True))


def merge_reports(gate, manifest, directory, identity):
    cells = {}
    seen = set()
    for path in sorted(directory.glob("*.json")):
        report = read_json(path)
        if not isinstance(report, dict) or report.get("identity") != identity or report.get("status") != "passed":
            raise gate.GateError(f"invalid, stale or failed assembly report: {path}")
        for name, column in report.get("cells", {}).items():
            if name not in gate.CONFIG_NAMES or name in seen or set(column) != manifest.ids():
                raise gate.GateError(f"duplicate/unknown configuration or incomplete site coverage: {path}")
            seen.add(name)
            cells.update({(site, name): value for site, value in column.items()})
    if seen != set(gate.CONFIG_NAMES):
        raise gate.GateError(f"incomplete assembly matrix: got {sorted(seen)}")
    return cells
