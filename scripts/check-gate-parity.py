#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a hygiene gate that runs locally but not in CI, or in CI but not locally.

``make check`` and ``.github/workflows/ci.yaml`` are two hand-maintained lists of
the same rule set. Nothing has ever compared them, so a gate added to one and
forgotten in the other is invisible: each list stays internally consistent and
each run stays green. The failure is one-directional and quiet in both
directions, and both directions had already happened when this was written:

* ``check-terminal-predicates.py`` and its ``--self-test`` ran in ``make check``
  and in **no** workflow, so nothing about terminal predicates blocked a merge;
* ``check-toolchain-pin.py --self-test`` ran in CI and **not** in ``make check``,
  so a contributor could not reproduce a red merge locally;
* and the five gates added for the benchmark lanes — the tracked-path gate among
  them, written *because* a flag-shaped artifact escaped thirty hygiene scripts —
  were in ``make check`` alone, unable to block the merge that would re-commit one.

CI does not run ``make check``; it enumerates each gate as a named step so a
failure names itself in the job list. That is a reasonable choice, and it is
exactly what creates the second list. So this gate's subject is the AGREEMENT
between the lists rather than either list's contents.

Scope, stated rather than implied: only invocations of a ``scripts/`` (or
in-repo ``crates/.../*.py``) program are compared. ``cargo`` steps, ``$(MAKE)``
recursions and the ``node`` schema oracles are deliberately out of scope — CI
distributes those across jobs (``wasm``, ``pytest``, ``capi``) and spells several
of them differently on purpose, so requiring textual equality there would refuse
a correct workflow. A gate that over-refuses gets disabled, and then it guards
nothing.
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
MAKEFILE = REPO_ROOT / "Makefile"
WORKFLOWS = REPO_ROOT / ".github" / "workflows"
CI_WORKFLOW = WORKFLOWS / "ci.yaml"

# `python3 scripts/x.py --flag`, `bash scripts/x.sh --flag`. The interpreter is
# part of the match but not of the identity: what matters is which program runs
# with which arguments, and the Makefile and the workflow must agree on that.
INVOCATION = re.compile(r"\b(?:python3|bash)\s+((?:scripts|crates)/[^\s]+)([^\n]*)")

# Release workflows legitimately run publish scripts with real arguments
# (`publish-release-crates.sh "${VERSION#rust-v}"`), which `make check` must not
# do. Only the `scripts/` programs whose names mark them as gates are required to
# appear on both sides.
GATE_NAME = re.compile(r"^(?:scripts/(?:check-|conformance-matrix|.*-self-test)|crates/)")


def _normalise(program: str, rest: str) -> str:
    """One invocation as `program args`, whitespace-collapsed.

    Arguments are part of the identity: ``--self-test`` and the bare run are
    different rules (one proves the gate can fail, the other applies it), and a
    list carrying only one of the pair is the divergence this refuses.
    """
    return " ".join((program + " " + rest).split())


def check_recipe(text: str, target: str) -> str:
    """The recipe body of one `make` target, tab-indented lines only.

    Split on a blank line would end the recipe early at the first blank line
    inside it, and split on the next `\\n<name>:` would swallow a `$(MAKE)` line
    containing a colon. Recipes are exactly the tab-prefixed lines that follow,
    so that is what is taken.
    """
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.startswith(f"{target}:"):
            body: list[str] = []
            for following in lines[index + 1 :]:
                if following.startswith("\t"):
                    body.append(following)
                elif following.strip() == "":
                    continue
                else:
                    break
            return "\n".join(body)
    sys.exit(f"FAIL: no `{target}:` target in {MAKEFILE}")


def invocations(text: str) -> set[str]:
    """Every in-repo program invocation in a blob of Makefile or YAML text."""
    return {_normalise(m.group(1), m.group(2)) for m in INVOCATION.finditer(text)}


def gates_only(found: set[str]) -> set[str]:
    """The invocations whose program name marks them as a repository gate."""
    return {call for call in found if GATE_NAME.match(call)}


def divergence(makefile_text: str, workflow_texts: dict[str, str]) -> list[str]:
    """Every gate one list runs and the other does not, in both directions."""
    local = gates_only(invocations(check_recipe(makefile_text, "check")))
    everywhere: set[str] = set()
    for text in workflow_texts.values():
        everywhere |= invocations(text)
    ci_gates = gates_only(invocations(workflow_texts.get(CI_WORKFLOW.name, "")))

    problems: list[str] = []
    for call in sorted(local - everywhere):
        problems.append(
            f"`{call}` runs in `make check` and in NO workflow, so it cannot block a merge"
        )
    for call in sorted(ci_gates - local):
        problems.append(
            f"`{call}` runs in {CI_WORKFLOW.name} and NOT in `make check`, so a red merge "
            "cannot be reproduced locally"
        )
    return problems


def _workflow_texts() -> dict[str, str]:
    return {path.name: path.read_text(encoding="utf-8") for path in sorted(WORKFLOWS.glob("*.yaml"))}


def self_test() -> int:
    """Both directions, plus the valid neighbour: the real tree must still pass."""
    ok = True
    real_makefile = MAKEFILE.read_text(encoding="utf-8")
    real_workflows = _workflow_texts()

    # THE VALID NEIGHBOUR FIRST, because it is the half that a refusal-only test
    # cannot distinguish: a gate that reported divergence unconditionally would
    # pass both refusal cases below and reject the repository as it stands.
    standing = divergence(real_makefile, real_workflows)
    if standing:
        print("SELF-TEST FAIL: the tree as committed is reported as divergent:")
        for problem in standing:
            print(f"  {problem}")
        ok = False
    else:
        print("OK: self-test — the tree as committed has no divergence (the valid neighbour)")

    # Direction 1: a gate in `make check` that no workflow runs.
    invented = "\tpython3 scripts/check-invented-local-only.py\n"
    mutated = real_makefile.replace("\ncheck: ", "\ncheck: ", 1)
    marker = "\tpython3 scripts/check-no-features.py\n"
    if marker not in mutated:
        print("SELF-TEST FAIL: the anchor line for the mutation is not in the Makefile")
        return 1
    mutated = mutated.replace(marker, marker + invented, 1)
    found = divergence(mutated, real_workflows)
    if any("check-invented-local-only.py" in problem and "NO workflow" in problem for problem in found):
        print("OK: self-test — a `make check` gate absent from every workflow is refused")
    else:
        print(f"SELF-TEST FAIL: a local-only gate was not refused (reported: {found})")
        ok = False

    # Direction 2: a gate in ci.yaml that `make check` does not run.
    ci_text = real_workflows[CI_WORKFLOW.name]
    anchor = "        run: python3 scripts/check-no-features.py\n"
    if anchor not in ci_text:
        print("SELF-TEST FAIL: the anchor line for the mutation is not in ci.yaml")
        return 1
    mutated_ci = dict(real_workflows)
    mutated_ci[CI_WORKFLOW.name] = ci_text.replace(
        anchor, anchor + "        run: python3 scripts/check-invented-ci-only.py\n", 1
    )
    found = divergence(real_makefile, mutated_ci)
    if any("check-invented-ci-only.py" in problem and "NOT in `make check`" in problem for problem in found):
        print("OK: self-test — a ci.yaml gate absent from `make check` is refused")
    else:
        print(f"SELF-TEST FAIL: a CI-only gate was not refused (reported: {found})")
        ok = False

    # A gate present in BOTH but with different arguments is a divergence too:
    # `--self-test` and the bare run are different rules.
    argument_drift = real_makefile.replace(
        "\tpython3 scripts/check-no-features.py\n",
        "\tpython3 scripts/check-no-features.py --self-test\n",
        1,
    )
    found = divergence(argument_drift, real_workflows)
    if any("check-no-features.py --self-test" in problem for problem in found):
        print("OK: self-test — the same gate with different arguments is refused")
    else:
        print(f"SELF-TEST FAIL: argument drift was not refused (reported: {found})")
        ok = False

    # And the neighbour for that rule: an argument-identical pair is NOT refused.
    # Without this, "refuse whenever the strings differ at all" would pass above
    # while refusing every correctly paired gate.
    if any("check-banned-deps" in problem for problem in divergence(real_makefile, real_workflows)):
        print("SELF-TEST FAIL: a correctly paired gate with two argument forms was refused")
        ok = False
    else:
        print("OK: self-test — a gate paired in both lists with both argument forms is accepted")

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    problems = divergence(MAKEFILE.read_text(encoding="utf-8"), _workflow_texts())
    if problems:
        sys.exit(
            "FAIL: `make check` and the CI workflows do not run the same gates:\n  "
            + "\n  ".join(problems)
            + "\n  Two lists is two rules. Add the missing step so both lists say the same "
            "thing;\n  a gate only one of them runs is a gate that does not hold."
        )
    local = gates_only(invocations(check_recipe(MAKEFILE.read_text(encoding="utf-8"), "check")))
    print(f"OK: all {len(local)} hygiene gates in `make check` also run in CI, and vice versa")
    return 0


if __name__ == "__main__":
    sys.exit(main())
