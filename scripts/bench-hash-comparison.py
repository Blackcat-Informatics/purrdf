#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Run paired public-API hash benchmarks in an isolated oracle workspace.

The removed packages remain outside the PurRDF dependency graph. Versions and
checksums must occur in the supplied historical lock. Results retain every
sample, the exact source, lock, compiler, flags, affinity and source revision.
This is a report-only benchmark, never a gate or a correctness oracle update.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
PINS = ["ahash=0.8.12", "md-5=0.10.6", "sha1=0.10.6", "sha3=0.10.9",
        "crc32fast=1.5.0", "blake3=1.8.5", "rayon=1.12.0"]
MODES = ["table", "digests", "blake3-native", "blake3-stream-native", "blake3-join",
         "blake3-select", "blake3-backend-portable", "blake3-backend-sse2",
         "blake3-backend-ssse3", "blake3-backend-avx2", "blake3-backend-avx512"]


def hash_sources() -> dict[str, str]:
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted((ROOT / "crates/hash/src").rglob("*.rs"))}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock-ref", required=True)
    parser.add_argument("--output", type=Path, required=True, help="new directory; never overwritten")
    parser.add_argument("--cpu", default="x86-64", help="rustc target-cpu (use generic for other ISAs)")
    parser.add_argument("--cpus", help="optional Linux CPU affinity, e.g. 8 or 8-15")
    parser.add_argument("--lto", choices=["fat", "thin", "off"], default="fat")
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--mode", choices=MODES, action="append")
    parser.add_argument("--baseline-isa", choices=["auto", "avx2", "sse2", "portable"], default="auto")
    args = parser.parse_args()
    if args.jobs < 1 or args.threads < 1:
        parser.error("jobs and threads must be positive")
    output = args.output.resolve()
    if output.exists():
        parser.error("output directory already exists")
    spec = importlib.util.spec_from_file_location("new_oracle", ROOT / "scripts/cleanroom/new_oracle.py")
    assert spec and spec.loader
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    lock_commit = subprocess.check_output(
        ["git", "rev-parse", "--verify", f"{args.lock_ref}^{{commit}}"], cwd=ROOT, text=True).strip()
    project = oracle.create("hash-comparison", PINS, ROOT, lock_commit, output)
    source = ROOT / "scripts/cleanroom/hash_comparison.rs"
    shutil.copyfile(source, project / "src/main.rs")
    manifest = project / "Cargo.toml"
    text = manifest.read_text().replace('blake3 = "=1.8.5"', 'blake3 = { version = "=1.8.5", features = ["rayon"] }')
    text = text.replace('ahash = "=0.8.12"', 'ahash = { version = "=0.8.12", default-features = false }')
    for name, version in [("md-5", "0.10.6"), ("sha1", "0.10.6"), ("sha3", "0.10.9")]:
        text = text.replace(f'{name} = "={version}"', f'{name} = {{ version = "={version}", default-features = false }}')
    text += '\npurrdf-hash = { path = ' + json.dumps(str(ROOT / "crates/hash")) + ' }\n'
    text += '\n[profile.release]\nopt-level = 3\nlto = "fat"\ncodegen-units = 1\n'
    manifest.write_text(text)
    flags = f"-D warnings -C target-cpu={args.cpu}"
    env = {**os.environ, "RUSTFLAGS": flags, "RAYON_NUM_THREADS": str(args.threads)}
    # Cargo's encoded flags outrank RUSTFLAGS. Refuse ambiguity in the receipt.
    if env.get("CARGO_ENCODED_RUSTFLAGS"):
        parser.error("unset CARGO_ENCODED_RUSTFLAGS so the recorded RUSTFLAGS are effective")
    features = {
        "auto": [], "avx2": ["no_avx512"],
        "sse2": ["no_avx512", "no_avx2", "no_sse41"],
        "portable": ["pure", "no_avx512", "no_avx2", "no_sse41", "no_sse2"],
    }[args.baseline_isa]
    profile = ["--config", f'profile.release.lto="{args.lto}"',
               "--config", "profile.release.opt-level=3",
               "--config", "profile.release.codegen-units=1"]
    command = ["cargo", "build", "--manifest-path", str(manifest), "--release",
               "--jobs", str(args.jobs), "--message-format=json", *profile]
    if features:
        command += ["--features", ",".join("blake3/" + f for f in features)]
    started = time.time()
    metadata = {
        "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "compiler": subprocess.check_output(["rustc", "-Vv"], text=True),
        "platform": platform.platform(), "flags": flags, "build_command": command,
        "reference_lock_commit": lock_commit, "reference_pins": PINS,
        "affinity": args.cpus, "rayon_threads": args.threads, "lto": args.lto,
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "hash_sources_sha256": hash_sources(),
        "cargo_profile_environment": {k: v for k, v in env.items() if k.startswith("CARGO_PROFILE_")},
        "effective_profile_configuration": subprocess.run(
            ["cargo", "-Z", "unstable-options", "config", "get", "profile", "--show-origin", *profile],
            cwd=project, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            check=False).stdout,
        "load_before": list(os.getloadavg()) if hasattr(os, "getloadavg") else None,
        "available_cpus": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
    }
    for relative in metadata["hash_sources_sha256"]:
        target = output / "sources" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / relative, target)
    if Path("/proc/cpuinfo").exists():
        (output / "cpuinfo.txt").write_text(Path("/proc/cpuinfo").read_text())
    (output / "working-tree.diff").write_bytes(subprocess.check_output(["git", "diff", "HEAD"], cwd=ROOT))
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2))
    with (output / "build.log").open("w") as errors:
        built = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE, stderr=errors, check=True)
    (output / "build.jsonl").write_text(built.stdout)
    binaries = [item["executable"] for line in built.stdout.splitlines()
                if (item := json.loads(line)).get("executable") and item.get("target", {}).get("name") == "hash-comparison"]
    if len(binaries) != 1:
        raise RuntimeError(f"expected one benchmark executable, found {binaries}")
    binary = output / "hash-comparison-bin"
    shutil.copy2(binaries[0], binary)
    modes = args.mode or MODES[:4]
    for mode in modes:
        run = (["taskset", "-c", args.cpus] if args.cpus else []) + [str(binary), mode]
        print(f"Benchmarking {mode}", flush=True)
        with (output / f"{mode}.csv").open("w") as values, (output / f"{mode}.log").open("w") as errors:
            subprocess.run(run, env=env, stdout=values, stderr=errors, check=True)
    if hash_sources() != metadata["hash_sources_sha256"]:
        raise RuntimeError("hash sources changed during measurement; results refused")
    lock_bytes = (project / "Cargo.lock").read_bytes()
    metadata["resolved_lock_sha256"] = hashlib.sha256(lock_bytes).hexdigest()
    metadata["load_after"] = list(os.getloadavg()) if hasattr(os, "getloadavg") else None
    metadata["elapsed_seconds"] = time.time() - started
    metadata["completed_modes"] = modes
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
