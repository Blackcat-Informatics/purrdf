#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""List the bench-harness suites of the `make bench` crates, for a baseline run.

The ``bench-compare`` job in ``.github/workflows/benchmarks.yaml`` runs every
suite twice on each of its runners (x86_64 and aarch64) -- once on a base
commit with ``--save-baseline base`` and once on the head with a comparison
against it. Those flags are ``purrdf_testkit::bench``'s, and a bench target that
is not a suite on that harness does not understand them: the allocation probes
are plain ``main`` functions, and ``ordered_json_corpus`` reads its positional
arguments as corpus roots, so handing it ``--save-baseline base`` makes it try
to open a directory called ``--save-baseline``. So the comparison runs harness
suites only, one ``--bench`` at a time, and this is where that set is decided.

WHICH CRATES is read from the Makefile's ``bench`` recipe (its ``-p`` list), so
the job and ``make bench`` cannot disagree about the population. WHICH TARGETS is
read from ``cargo metadata`` of the tree named by ``--root``: a bench target is a
harness suite exactly when its source has a ``bench_main!(...)`` invocation at
the start of a line.

Output (stdout, one JSON array): ``[{"crate", "bench", "in_base"}, ...]`` -- the
``target`` axis of the job's matrix, which the workflow crosses with its runner
axis. ``in_base`` is true when ``--base-root`` names a tree whose same crate has
the same target as a harness suite. A target that is new at the head, or that
the base tree runs on another harness (whose saved records this harness cannot
read), has nothing to compare against and is still run, so its numbers are
recorded.

    python3 scripts/bench-suite-targets.py --root . --base-root ../base
    python3 scripts/bench-suite-targets.py --self-test
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BENCH_MAIN = re.compile(r"^\s*(?:purrdf_testkit::)?bench_main!\s*\(", re.MULTILINE)
BENCH_RECIPE = re.compile(r"^bench:[^\n]*\n((?:\t[^\n]*\n)+)", re.MULTILINE)


def bench_crates(makefile: Path) -> list[str]:
    """The ``-p`` list of the Makefile's ``bench`` recipe, in order."""
    text = makefile.read_text(encoding="utf-8")
    match = BENCH_RECIPE.search(text)
    if match is None:
        sys.exit(f"bench-suite-targets: no `bench:` recipe in {makefile}")
    crates = re.findall(r"(?:^|\s)-p\s+(\S+)", match.group(1))
    if not crates:
        sys.exit(f"bench-suite-targets: the `bench:` recipe in {makefile} names no -p crate")
    return crates


def is_harness_suite(source: Path) -> bool:
    """True when *source* invokes ``bench_main!`` at the start of a line."""
    return BENCH_MAIN.search(source.read_text(encoding="utf-8")) is not None


def suite_targets(metadata: dict, crates: list[str]) -> dict[str, list[str]]:
    """Harness-suite bench target names per crate, from a ``cargo metadata`` document."""
    packages = {package["name"]: package for package in metadata["packages"]}
    result: dict[str, list[str]] = {}
    for crate in crates:
        package = packages.get(crate)
        if package is None:
            result[crate] = []
            continue
        result[crate] = sorted(
            target["name"]
            for target in package["targets"]
            if "bench" in target["kind"] and is_harness_suite(Path(target["src_path"]))
        )
    return result


def cargo_metadata(root: Path) -> dict:
    """``cargo metadata --no-deps`` of the workspace at *root*."""
    completed = subprocess.run(
        [
            "cargo", "metadata", "--no-deps", "--format-version", "1", "--locked",
            "--manifest-path", str(root / "Cargo.toml"),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        sys.exit(f"bench-suite-targets: cargo metadata failed in {root}:\n{completed.stderr}")
    return json.loads(completed.stdout)


def target_axis(head: dict[str, list[str]], base: dict[str, list[str]] | None) -> list[dict]:
    """The matrix's ``target`` axis for *head*, marking targets present in *base*."""
    axis = []
    for crate, benches in head.items():
        for bench in benches:
            in_base = base is not None and bench in base.get(crate, [])
            axis.append({"crate": crate, "bench": bench, "in_base": in_base})
    return axis


def self_test() -> int:
    """Every direction of the harness-suite filter, and the Makefile reader."""
    scratch = REPO_ROOT / "target" / "bench-suite-targets-selftest"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    ok = True

    suite = scratch / "suite.rs"
    suite.write_text(
        "use purrdf_testkit::bench::{Bench, bench_group, bench_main};\n"
        "fn f(c: &mut Bench) { c.bench_function(\"x\", |b| b.iter(|| 1)); }\n"
        "bench_group!(benches, f);\nbench_main!(benches);\n",
        encoding="utf-8",
    )
    probe = scratch / "probe.rs"
    probe.write_text(
        "// Not a bench_main!(benches) suite: a plain allocation probe.\n"
        "fn main() { println!(\"bytes 1\"); }\n",
        encoding="utf-8",
    )
    # A suite on another harness: its records are not this harness's, so a base
    # tree holding it has nothing the head can compare against.
    foreign = scratch / "foreign.rs"
    foreign.write_text(
        "use criterion::{criterion_group, criterion_main, Criterion};\n"
        "fn f(c: &mut Criterion) { c.bench_function(\"x\", |b| b.iter(|| 1)); }\n"
        "criterion_group!(benches, f);\ncriterion_main!(benches);\n",
        encoding="utf-8",
    )
    metadata = {
        "packages": [
            {
                "name": "demo",
                "targets": [
                    {"name": "suite", "kind": ["bench"], "src_path": str(suite)},
                    {"name": "probe", "kind": ["bench"], "src_path": str(probe)},
                    {"name": "foreign", "kind": ["bench"], "src_path": str(foreign)},
                    {"name": "demo", "kind": ["lib"], "src_path": str(suite)},
                ],
            }
        ]
    }
    found = suite_targets(metadata, ["demo", "absent"])
    if found == {"demo": ["suite"], "absent": []}:
        print(
            "OK: self-test — a harness suite is selected; a plain-main probe and a "
            "suite on another harness are not"
        )
    else:
        print(f"SELF-TEST FAIL: suite target selection returned {found}")
        ok = False

    axis = target_axis({"demo": ["suite", "fresh"]}, {"demo": ["suite"]})
    expected = [
        {"crate": "demo", "bench": "suite", "in_base": True},
        {"crate": "demo", "bench": "fresh", "in_base": False},
    ]
    if axis == expected:
        print("OK: self-test — a target new at the head is kept and marked as having no base")
    else:
        print(f"SELF-TEST FAIL: target_axis returned {axis}")
        ok = False

    makefile = scratch / "Makefile"
    makefile.write_text(
        "bench: ## Run.\n\tcargo bench -p alpha -p beta\n\nother:\n\tcargo bench -p gamma\n",
        encoding="utf-8",
    )
    if bench_crates(makefile) == ["alpha", "beta"]:
        print("OK: self-test — the crate list is read from the bench recipe alone")
    else:
        print(f"SELF-TEST FAIL: bench_crates returned {bench_crates(makefile)}")
        ok = False

    real = bench_crates(REPO_ROOT / "Makefile")
    if real:
        print(f"OK: self-test — the committed Makefile's bench recipe names {len(real)} crates")
    else:
        ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=REPO_ROOT, help="the head workspace")
    parser.add_argument("--base-root", type=Path, help="the base workspace, when comparing")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    crates = bench_crates(args.root / "Makefile")
    head = suite_targets(cargo_metadata(args.root), crates)
    base = None
    if args.base_root is not None:
        base = suite_targets(cargo_metadata(args.base_root), crates)
    axis = target_axis(head, base)
    if not axis:
        sys.exit("bench-suite-targets: no bench-harness suite found in the bench crates")
    print(json.dumps(axis, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
