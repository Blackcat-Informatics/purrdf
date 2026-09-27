#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The one list of how CI splits `cargo test --workspace` across runners.

One job running `cargo test --workspace --locked` compiled for thirty minutes on a
cold runner before the first test started, then ran for nine more -- a job that
cannot fit the per-job budget no matter how the timeout is set. The work was
never the problem; the serialisation was. So the CI `test` job is a matrix, and
each leg runs `cargo test --locked -p <package> ...` for the packages its shard
names here.

A shard is a set of WORKSPACE PACKAGES, never of individual test targets.
`cargo test -p X` selects exactly the targets `cargo test --workspace` selects
for X -- unit tests, every integration-test file, bins, examples, test-enabled
benches and X's doctests -- so a package in some shard is a package whose whole
test surface runs, and the union of the shards is the workspace run. There is no
target-level list to fall out of date.

The grouping follows the dependency chain, because every leg pays for building
the libraries its packages depend on before any of their tests compile, and is
balanced by measured cold-runner wall time (compile plus run), not by crate count:

* `kernel`   -- everything below the SPARQL evaluator: leaves, `purrdf-core`,
                the Datalog substrate, the entailment engine, results, algebra, GTS;
* `eval`     -- the evaluator (the heaviest test build in the workspace) and
                HNSW, whose determinism suite is the second-longest test run;
* `rdf`      -- the codec crate, the leaves that need only it, and the text and
                retrieval stack, which needs the evaluator the codec already builds;
* `shapes`   -- the validators and everything that needs `purrdf-shapes`;
* `hosts`    -- the umbrella crate (whose doctests link every member), the CLI,
                the wasm and Python bindings and the conformance harness, all of
                which need the whole library graph anyway;
* `capi`     -- the C ABI alone: its smoke test runs a nested `cargo build` of the
                cdylib and a C compiler, the longest single test in the workspace,
                so it gets a leg whose own compile is otherwise small.

`scripts/check-test-shards.py` holds this list to `cargo metadata`: a workspace
member in no shard, in two, or a shard naming a package that is not a member, is
a hard failure in `make check` and in CI. `--matrix` refuses the same way, so the
CI matrix cannot be emitted from a list that leaves a crate untested.
"""

import argparse
import glob
import json
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Order is the order the matrix legs are listed in; it carries no meaning.
SHARDS: dict[str, tuple[str, ...]] = {
    "kernel": (
        "purrdf-alloc-probe",
        "purrdf-cdt",
        "purrdf-columnar",
        "purrdf-core",
        "purrdf-datalog",
        "purrdf-entail",
        "purrdf-events",
        "purrdf-gts",
        "purrdf-iri",
        "purrdf-sparql-algebra",
        "purrdf-sparql-results",
        "purrdf-stack",
        "purrdf-xsd",
    ),
    "eval": (
        "purrdf-hnsw",
        "purrdf-sparql-eval",
    ),
    "rdf": (
        "purrdf-json",
        "purrdf-markdown",
        "purrdf-rdf",
        "purrdf-retrieval",
        "purrdf-shex",
        "purrdf-slice",
        "purrdf-text",
    ),
    "shapes": (
        "purrdf-bench",
        "purrdf-envelope-probe",
        "purrdf-geo",
        "purrdf-shapes",
        "purrdf-validate",
    ),
    "hosts": (
        "purrdf",
        "purrdf-cli",
        "purrdf-python",
        "purrdf-sparql-conformance",
        "purrdf-wasm",
    ),
    "capi": ("purrdf-capi",),
}


def manifest_members(root: Path = REPO_ROOT) -> set[str]:
    """Package names of the workspace members, read from the manifests alone.

    This is what the CI planning step can afford (no Rust toolchain installed);
    `check-test-shards.py` proves it agrees with `cargo metadata`.
    """
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]
    excluded = {(root / path).resolve() for path in workspace.get("exclude", [])}
    names = set()
    for pattern in workspace["members"]:
        matches = sorted(glob.glob(str(root / pattern)))
        if not matches:
            raise SystemExit(f"FAIL: workspace member pattern `{pattern}` matches nothing")
        for directory in matches:
            if Path(directory).resolve() in excluded:
                continue
            manifest = tomllib.loads((Path(directory) / "Cargo.toml").read_text(encoding="utf-8"))
            names.add(manifest["package"]["name"])
    return names


def coverage_problems(members: set[str], shards: dict[str, tuple[str, ...]]) -> list[str]:
    """Every way `shards` fails to be an exact partition of `members`."""
    problems = []
    owner: dict[str, str] = {}
    for shard, packages in shards.items():
        if not packages:
            problems.append(f"shard `{shard}` names no packages")
        for package in packages:
            if package in owner:
                problems.append(
                    f"`{package}` is in shard `{owner[package]}` and shard `{shard}`; "
                    f"it would be tested twice"
                )
            owner.setdefault(package, shard)
            if package not in members:
                problems.append(f"shard `{shard}` names `{package}`, which is not a workspace member")
    for package in sorted(members - owner.keys()):
        problems.append(
            f"workspace member `{package}` is in no shard, so CI would never run its tests; "
            f"add it to a shard in scripts/test-shards.py"
        )
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument(
        "--matrix", action="store_true", help="print the shard names as a JSON list"
    )
    action.add_argument(
        "--packages", metavar="SHARD", help="print `-p <package>` arguments for one shard"
    )
    args = parser.parse_args(argv)

    problems = coverage_problems(manifest_members(), SHARDS)
    if problems:
        print("FAIL: the test shards are not a partition of the workspace:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    if args.matrix:
        print(json.dumps(list(SHARDS)))
        return 0
    if args.packages not in SHARDS:
        print(f"FAIL: no shard named `{args.packages}` (have: {', '.join(SHARDS)})", file=sys.stderr)
        return 1
    print(" ".join(f"-p {package}" for package in SHARDS[args.packages]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
