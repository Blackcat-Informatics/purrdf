#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a CI test split that lets any workspace crate escape testing.

CI no longer runs `cargo test --workspace` in one job; it runs one matrix leg per
shard in `scripts/test-shards.py`, each `cargo test --locked -p ...` over that
shard's packages. That makes the shard list a second statement of "the whole
workspace", and a second statement drifts silently: a crate added to
`[workspace] members` and to no shard would compile in `make check`, be linted in
CI, and have its tests run nowhere that blocks a merge -- with every job green.

So this gate holds three things:

1. the shards are an exact partition of the workspace members `cargo metadata`
   reports -- every member in exactly one shard, no shard naming a non-member, no
   empty shard;
2. the manifest reader `test-shards.py` uses in the CI planning step (which has no
   Rust toolchain) agrees with `cargo metadata`, so the matrix CI builds from is
   the one proven here;
3. the CI `test` job is still wired to the shards: its matrix comes from the
   planning job, it runs each leg's `--packages` list through `cargo test
   --locked`, and one red leg does not cancel the others' results.

`--self-test` proves each refusal fires and that an exact partition passes.
"""

import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yaml"

_spec = importlib.util.spec_from_file_location("test_shards", REPO_ROOT / "scripts" / "test-shards.py")
test_shards = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(test_shards)

# What the `test` job and its planning job must contain, each with the reason a
# job without it would stop running the whole workspace.
TEST_JOB_WIRING = (
    ("needs: test-plan", "the matrix must come from the planning job, not a hand-copied list"),
    ("fromJSON(needs.test-plan.outputs.shards)", "the matrix must be the list the shard gate proved"),
    ("fail-fast: false", "one red leg must not cancel the others before they report"),
    ("scripts/test-shards.py --packages", "each leg must test exactly its shard's packages"),
    ("cargo test --locked", "each leg must run the ordinary locked `cargo test`"),
)
PLAN_JOB_WIRING = (
    ("scripts/test-shards.py --matrix", "the planning job must emit the checked shard list"),
)


def job_block(workflow: str, job: str) -> str | None:
    """The text of one top-level job in a workflow, or None if it is absent."""
    match = re.search(rf"^  {re.escape(job)}:\s*$(.*?)(?=^  \S|\Z)", workflow, re.MULTILINE | re.DOTALL)
    return match.group(1) if match else None


def wiring_problems(workflow: str) -> list[str]:
    problems = []
    for job, required in (("test", TEST_JOB_WIRING), ("test-plan", PLAN_JOB_WIRING)):
        block = job_block(workflow, job)
        if block is None:
            problems.append(f"ci.yaml has no `{job}:` job")
            continue
        # Comments do not run: a commented-out step must not satisfy the wiring.
        live = "\n".join(line for line in block.splitlines() if not line.lstrip().startswith("#"))
        for needle, reason in required:
            if needle not in live:
                problems.append(f"ci.yaml `{job}` job lacks `{needle}`: {reason}")
    return problems


def metadata_members() -> set[str]:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=REPO_ROOT,
            text=True,
        )
    )
    by_id = {package["id"]: package["name"] for package in metadata["packages"]}
    return {by_id[member] for member in metadata["workspace_members"]}


def check() -> int:
    problems = []
    members = metadata_members()
    from_manifests = test_shards.manifest_members()
    if from_manifests != members:
        problems.append(
            "test-shards.py reads a different member set from the manifests than cargo "
            f"metadata reports: only in manifests {sorted(from_manifests - members)}, "
            f"only in cargo metadata {sorted(members - from_manifests)}"
        )
    problems += test_shards.coverage_problems(members, test_shards.SHARDS)
    problems += wiring_problems(CI_WORKFLOW.read_text(encoding="utf-8"))
    if problems:
        print("FAIL: the CI test shards do not cover the workspace:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    count = sum(len(packages) for packages in test_shards.SHARDS.values())
    print(f"OK: {len(test_shards.SHARDS)} test shards partition all {count} workspace members")
    return 0


def self_test() -> int:
    members = {"a", "b", "c"}
    refused = {
        "a member in no shard": {"one": ("a", "b")},
        "a member in two shards": {"one": ("a", "b"), "two": ("b", "c")},
        "a shard naming a non-member": {"one": ("a", "b", "c", "z")},
        "an empty shard": {"one": ("a", "b", "c"), "two": ()},
    }
    failures = []
    for case, shards in refused.items():
        if not test_shards.coverage_problems(members, shards):
            failures.append(f"accepted {case}")
    # The valid neighbour: an exact partition, with a singleton shard like `capi`.
    if problems := test_shards.coverage_problems(members, {"one": ("a", "b"), "two": ("c",)}):
        failures.append(f"refused an exact partition: {problems}")

    wired = (
        "jobs:\n"
        "  test-plan:\n"
        "    steps:\n"
        "      - run: python3 scripts/test-shards.py --matrix\n"
        "  test:\n"
        "    needs: test-plan\n"
        "    strategy:\n"
        "      fail-fast: false\n"
        "      matrix:\n"
        "        shard: ${{ fromJSON(needs.test-plan.outputs.shards) }}\n"
        "    steps:\n"
        "      - run: cargo test --locked $(python3 scripts/test-shards.py --packages x)\n"
        "  other:\n"
        "    steps: []\n"
    )
    if problems := wiring_problems(wired):
        failures.append(f"refused a correctly wired workflow: {problems}")
    unwired = {
        "a test job back on one `cargo test --workspace`": wired.replace(
            "cargo test --locked $(python3 scripts/test-shards.py --packages x)",
            "cargo test --workspace --locked",
        ),
        "a hand-written matrix": wired.replace(
            "${{ fromJSON(needs.test-plan.outputs.shards) }}", "[kernel, eval]"
        ),
        "a fail-fast matrix": wired.replace("fail-fast: false", "fail-fast: true"),
        "a commented-out shard step": wired.replace(
            "      - run: cargo test", "      # - run: cargo test"
        ),
        "a missing planning job": wired.replace("  test-plan:", "  planning:"),
    }
    for case, workflow in unwired.items():
        if not wiring_problems(workflow):
            failures.append(f"accepted {case}")

    if failures:
        print("FAIL: check-test-shards self-test:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print(f"OK: check-test-shards refuses {len(refused) + len(unwired)} broken splits and accepts two valid ones")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true", help="prove every refusal fires")
    args = parser.parse_args(argv)
    return self_test() if args.self_test else check()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
