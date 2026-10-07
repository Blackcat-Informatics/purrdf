#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Compare complete PurRDF operations in isolated, pinned repository snapshots.

Timing uses the system allocator. A separate executable records allocations.
Output identities must agree before sampling. All samples and build receipts
are retained; no speed threshold is a correctness gate.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CASES = ["intern-iri", "intern-mixed", "parse-nquads", "parse-turtle", "query-join",
         "gts-author-4k", "gts-read-4k", "gts-author-1m", "gts-read-1m"]


def git(path, *args):
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def registry_pins(path):
    return {(p["name"], p["version"], p.get("source"), p.get("checksum"))
            for p in tomllib.loads(path.read_text())["package"] if p.get("source")}


def source_identity(path):
    names = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=path).split(b"\0")
    digest = hashlib.sha256()
    for raw in sorted(n for n in names if n):
        file = path / os.fsdecode(raw)
        digest.update(raw + b"\0")
        digest.update(file.read_bytes() if file.is_file() else b"MISSING")
    return digest.hexdigest()


def build(label, repo, output, profile, env, jobs):
    directory = output / label
    project = directory / "project"
    (project / "src").mkdir(parents=True)
    generator = project / "manifest-generator"
    subprocess.run(["rustc", "--edition=2024", "-Dwarnings", str(ROOT / "scripts/hash-workload-manifest.rs"), "-o", str(generator)], check=True)
    shutil.copytree(ROOT / "crates/testkit", project / "crates/testkit")
    shutil.copytree(repo / "crates/jsonschema/tests/metaschemas", project / "crates/jsonschema/tests/metaschemas")
    subprocess.run([str(generator), str(repo), str(ROOT), str(project)], check=True)
    shutil.copy2(repo / "Cargo.lock", project / "Cargo.lock")
    shutil.copy2(ROOT / "scripts/hash-workloads.rs", project / "src/workloads.rs")
    (project / "src/timing.rs").write_text('mod workloads;\nfn main() { workloads::entry(false); }\n')
    (project / "src/allocations.rs").write_text('mod workloads;\n#[global_allocator]\nstatic ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;\nfn main() { workloads::entry(true); }\n')
    identity = source_identity(repo)
    receipt = {"head": git(repo, "rev-parse", "HEAD"), "source_sha256": identity,
               "status": git(repo, "status", "--short"), "repo": str(repo), "tooling_testkit": source_identity(ROOT / "crates/testkit")}
    (directory / "source.diff").write_text(git(repo, "diff", "HEAD", "--binary"))
    extra_sources = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=repo).split(b"\0")
    for raw in filter(None, extra_sources):
        relative = Path(os.fsdecode(raw))
        destination = directory / "source-extra" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo / relative, destination)
    command = ["cargo", "build", "--release", "--bins", "--offline",
               "--message-format=json", *profile]
    if jobs is not None:
        command.extend(["--jobs", str(jobs)])
    receipt["build_command"] = command
    receipt["effective_profile_configuration"] = subprocess.check_output(
        ["cargo", "-Z", "unstable-options", "config", "get", "profile", "--show-origin", *profile],
        cwd=project, env=env, text=True)
    # The copied source lock pins registry versions. Adding only the external
    # harness package may update its graph; verify every resulting registry pin.
    print(f"Building {label}", flush=True)
    with (directory / "build.log").open("w") as errors:
        completed = subprocess.run(command, cwd=project, env=env, text=True,
                                   stdout=subprocess.PIPE, stderr=errors, check=False)
    (directory / "build.jsonl").write_text(completed.stdout)
    completed.check_returncode()
    extra = registry_pins(project / "Cargo.lock") - registry_pins(repo / "Cargo.lock")
    if extra:
        raise RuntimeError(f"{label}: dependency pins changed: {extra}")
    for line in completed.stdout.splitlines():
        item = json.loads(line)
        if item.get("executable") and item.get("target", {}).get("name") in ("timing", "allocations"):
            shutil.copy2(item["executable"], directory / item["target"]["name"])
    if source_identity(repo) != identity or source_identity(ROOT / "crates/testkit") != receipt["tooling_testkit"]:
        raise RuntimeError(f"{label}: source changed during compilation")
    receipt["binary_sha256"] = {name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
                                for name in ("timing", "allocations")}
    receipt["lock_sha256"] = hashlib.sha256((project / "Cargo.lock").read_bytes()).hexdigest()
    (directory / "receipt.json").write_text(json.dumps(receipt, indent=2))


def reuse(label, repo, previous, output, metadata):
    prior_metadata = json.loads((previous / "metadata.json").read_text())
    receipt = json.loads((previous / label / "receipt.json").read_text())
    for key in ("compiler", "cpu", "lto", "flags", "profile_overrides"):
        if prior_metadata[key] != metadata[key]:
            raise RuntimeError(f"{label}: reused build differs in {key}")
    if (previous / "hash-workloads.rs").read_bytes() != (ROOT / "scripts/hash-workloads.rs").read_bytes():
        raise RuntimeError(f"{label}: workload source differs from reused build")
    if receipt["source_sha256"] != source_identity(repo) or receipt.get("tooling_testkit") != source_identity(ROOT / "crates/testkit"):
        raise RuntimeError(f"{label}: repository differs from reused build")
    directory = output / label
    shutil.copytree(previous / label, directory)
    checksums = {name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
                 for name in ("timing", "allocations")}
    if receipt.get("binary_sha256") != checksums:
        raise RuntimeError(f"{label}: reused executable checksum mismatch")
    receipt["binary_sha256"] = checksums
    receipt["reused_from"] = str(previous / label)
    (directory / "receipt.json").write_text(json.dumps(receipt, indent=2))


def counter_values(path):
    required = {"instructions:u", "cycles:u", "branches:u", "branch-misses:u", "task-clock:u"}
    values = {}
    for row in csv.reader(path.open()):
        if len(row) > 2 and row[2] in required:
            value = float(row[0])  # Refuse unsupported or uncounted events.
            if value < 0 or (row[2] != "branch-misses:u" and value == 0):
                raise RuntimeError(f"invalid counter in {path}: {row}")
            values[row[2]] = value
    if values.keys() != required:
        raise RuntimeError(f"incomplete counters in {path}")
    return values


def summarize(output, labels, cases, counters):
    def distribution(values):
        return {"median": statistics.median(values), "min": min(values),
                "max": max(values), "samples": len(values)}

    rows = list(csv.DictReader((output / "samples.csv").open()))
    times = {(r["case"], r["build"], int(r["sample"])):
             int(r["total_ns"]) / int(r["iterations"]) for r in rows}
    if len(times) != len(cases) * len(labels) * 12 or len(times) != len(rows):
        raise RuntimeError("incomplete or duplicate timing samples")
    result = {"reference": labels[0], "cases": {}}
    for case in cases:
        result["cases"][case] = {
            label: {"time_ns": distribution([times[case, label, i] for i in range(12)]),
                    "paired_time_ratio": distribution([
                        times[case, label, i] / times[case, labels[0], i] for i in range(12)])}
            for label in labels}
    if counters:
        runs = list(csv.DictReader((output / "counters/runs.csv").open()))
        values = {(r["case"], r["build"], int(r["sample"])):
                  {event: value / int(r["iterations"])
                   for event, value in counter_values(output / "counters" / r["receipt"]).items()}
                  for r in runs}
        if len(values) != len(cases) * len(labels) * 3 or len(values) != len(runs):
            raise RuntimeError("incomplete or duplicate counter samples")
        for case in cases:
            for label in labels:
                result["cases"][case][label]["counters_per_operation"] = {
                    event: {"value": distribution([values[case, label, i][event] for i in range(3)]),
                            "paired_ratio": distribution([
                                values[case, label, i][event] / values[case, labels[0], i][event]
                                for i in range(3)])}
                    for event in ("instructions:u", "cycles:u", "branches:u", "task-clock:u")}
    (output / "summary.json").write_text(json.dumps(result, indent=2))


def run(output, labels, cases, env, affinity, counters):
    prefix = ["taskset", "-c", affinity] if affinity else []
    def execute(label, binary, case, *args):
        return subprocess.check_output([*prefix, str(output / label / binary), case,
                                        *map(str, args)], env=env, text=True).strip()

    identities = {}
    with (output / "allocations.csv").open("w") as file:
        writer = csv.writer(file)
        writer.writerow(["case", "build", "allocations", "requested_bytes", "peak_bytes", "retained_bytes", "signature"])
        for case in cases:
            identities[case] = {label: execute(label, "timing", case, 0) for label in labels}
            if len(set(identities[case].values())) != 1:
                raise RuntimeError(f"output mismatch: {case}: {identities[case]}")
            for label in labels:
                allocation = execute(label, "allocations", case).split(",")
                if allocation[-1] != identities[case][label]:
                    raise RuntimeError(f"allocation output mismatch: {case}: {label}")
                writer.writerow([case, label, *allocation])
    (output / "identities.json").write_text(json.dumps(identities, indent=2))
    with (output / "samples.csv").open("w") as file:
        writer = csv.writer(file)
        writer.writerow(["case", "sample", "position", "build", "iterations", "total_ns", "load1"])
        for case in cases:
            duration = int(execute(labels[0], "timing", case, 1))
            iterations = max(1, 100_000_000 // max(duration, 1))
            print(f"Sampling {case}: {iterations} iterations", flush=True)
            for sample in range(12):
                order = labels if sample % 2 == 0 else labels[::-1]
                for position, label in enumerate(order):
                    elapsed = int(execute(label, "timing", case, iterations))
                    writer.writerow([case, sample, position, label, iterations, elapsed, os.getloadavg()[0]])
                    file.flush()

    if counters:
        counter_dir = output / "counters"
        counter_dir.mkdir()
        control, acknowledgement = counter_dir / "control", counter_dir / "ack"
        os.mkfifo(control)
        os.mkfifo(acknowledgement)
        counter_env = {**env, "PURRDF_BENCH_PERF_CONTROL": str(control),
                       "PURRDF_BENCH_PERF_ACK": str(acknowledgement)}
        with (counter_dir / "runs.csv").open("w") as file:
            writer = csv.writer(file)
            writer.writerow(["case", "sample", "position", "build", "iterations", "total_ns", "receipt"])
            for case in cases:
                duration = int(execute(labels[0], "timing", case, 1))
                iterations = max(1, 300_000_000 // max(duration, 1))
                print(f"Counting {case}: {iterations} iterations", flush=True)
                for sample in range(3):
                    order = labels if sample % 2 == 0 else labels[::-1]
                    for position, label in enumerate(order):
                        receipt = counter_dir / f"{case}-{label}-{sample}.csv"
                        command = ["perf", "stat", "--no-big-num", "-x,", "--delay=-1",
                                   f"--control=fifo:{control},{acknowledgement}",
                                   "-e", "instructions:u,cycles:u,branches:u,branch-misses:u,task-clock",
                                   "--output", str(receipt), "--", *prefix,
                                   str(output / label / "timing"), case, str(iterations)]
                        measured = subprocess.run(command, env=counter_env, text=True,
                                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                                  timeout=120, check=False)
                        (receipt.with_suffix(".stderr")).write_text(measured.stderr)
                        (receipt.with_suffix(".stdout")).write_text(measured.stdout)
                        measured.check_returncode()
                        counter_values(receipt)
                        writer.writerow([case, sample, position, label, iterations,
                                         int(measured.stdout.strip()), receipt.name])
                        file.flush()
        control.unlink()
        acknowledgement.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", action="append", required=True, help="LABEL=CHECKOUT")
    parser.add_argument("--reuse", action="append", default=[], help="LABEL=PREVIOUS_RUN; reuse verified matching artifacts")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cpu", default="native")
    parser.add_argument("--cpus", default="8")
    parser.add_argument("--lto", choices=["fat", "thin", "off"], default="fat")
    parser.add_argument("--threads", type=int, default=1)
    parser.add_argument("--jobs", type=int, help="build jobs (default: Cargo configuration)")
    parser.add_argument("--case", action="append", choices=CASES)
    parser.add_argument("--perf", action="store_true", help="count only the measured region using perf control pipes")
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--run-prepared", action="store_true")
    args = parser.parse_args()
    sources = dict((label, Path(path).resolve()) for label, path in (s.split("=", 1) for s in args.source))
    if len(sources) != len(args.source) or len(sources) < 2:
        parser.error("supply at least two distinct build labels")
    if (args.jobs is not None and args.jobs < 1) or args.threads < 1:
        parser.error("jobs and threads must be positive")
    if any(not label.replace("-", "").isalnum() for label in sources):
        parser.error("labels must contain only letters, digits and hyphens")
    reused = dict((label, Path(path).resolve()) for label, path in (s.split("=", 1) for s in args.reuse))
    if len(reused) != len(args.reuse) or reused.keys() - sources.keys():
        parser.error("reuse labels must be distinct source labels")
    output = args.output.resolve()
    env = {**os.environ, "RUSTFLAGS": f"-D warnings -C target-cpu={args.cpu}",
           "RAYON_NUM_THREADS": str(args.threads)}
    if env.get("CARGO_ENCODED_RUSTFLAGS"):
        parser.error("unset CARGO_ENCODED_RUSTFLAGS")
    profile = ["--config", f'profile.release.lto="{args.lto}"',
               "--config", "profile.release.opt-level=3", "--config", "profile.release.codegen-units=1"]
    if args.run_prepared and args.prepare_only:
        parser.error("prepare-only and run-prepared are mutually exclusive")
    if args.run_prepared:
        metadata = json.loads((output / "metadata.json").read_text())
        requested = {"cpu": args.cpu, "cpus": args.cpus, "lto": args.lto,
                     "threads": args.threads, "flags": env["RUSTFLAGS"],
                     "sources": {label: str(path) for label, path in sources.items()}}
        if any(metadata.get(key) != value for key, value in requested.items()):
            parser.error("prepared metadata differs from requested configuration")
        if any((output / name).exists() for name in ("samples.csv", "allocations.csv")):
            parser.error("refusing to overwrite existing measurements")
        for label, repo in sources.items():
            receipt = json.loads((output / label / "receipt.json").read_text())
            checksums = {name: hashlib.sha256((output / label / name).read_bytes()).hexdigest()
                         for name in ("timing", "allocations")}
            if receipt.get("binary_sha256") != checksums or receipt["source_sha256"] != source_identity(repo) or receipt.get("tooling_testkit") != source_identity(ROOT / "crates/testkit"):
                parser.error(f"{label}: prepared source or executable identity changed")
    if not args.run_prepared:
        output.mkdir(parents=True, exist_ok=False)
        metadata = {"utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    "compiler": subprocess.check_output(["rustc", "-Vv"], text=True),
                    "cpu": args.cpu, "cpus": args.cpus, "lto": args.lto, "threads": args.threads,
                    "flags": env["RUSTFLAGS"], "profile_overrides": profile,
                    "conditions": "shared host", "load_before": os.getloadavg(),
                    "available_cpus": sorted(os.sched_getaffinity(0)),
                    "cargo_profile_environment": {k: v for k, v in env.items() if k.startswith("CARGO_PROFILE_")},
                    "sources": {label: str(path) for label, path in sources.items()},
                    "harness_sha256": {name: hashlib.sha256((ROOT / "scripts" / name).read_bytes()).hexdigest()
                                       for name in ("hash-workloads.rs", "bench-hash-workloads.py", "hash-workload-manifest.rs")}}
        if Path("/proc/cpuinfo").exists():
            shutil.copy2("/proc/cpuinfo", output / "cpuinfo.txt")
        for name in ("hash-workloads.rs", "bench-hash-workloads.py", "hash-workload-manifest.rs"):
            shutil.copy2(ROOT / "scripts" / name, output / name)
        (output / "metadata.json").write_text(json.dumps(metadata, indent=2))
        for label, repo in sources.items():
            if label in reused:
                reuse(label, repo, reused[label], output, metadata)
            else:
                build(label, repo, output, profile, env, args.jobs)
    if not args.prepare_only:
        run(output, list(sources), args.case or CASES, env, args.cpus, args.perf)
        summarize(output, list(sources), args.case or CASES, args.perf)
        metadata = json.loads((output / "metadata.json").read_text())
        metadata["completed_cases"] = args.case or CASES
        metadata["hardware_counters"] = args.perf
        metadata["load_after"] = os.getloadavg()
        metadata["completed_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        (output / "metadata.json").write_text(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
