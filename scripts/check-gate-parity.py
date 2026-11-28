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

Scope: invocations of a ``scripts/`` (or in-repo ``crates/.../*.py``) program and
helper-census policy/self-test modes are compared. Other ``cargo`` and ``node``
oracles are out of scope — CI distributes those across jobs (``wasm``, ``pytest``,
``capi``) and spells several differently on purpose, so requiring textual equality
there would refuse a correct workflow. The explicit preserve-order consumer is a
bounded exception: its three Cargo gates must remain in both lists because an
excluded consumer does not run through any workspace command.

``$(MAKE)`` recursions ARE followed, within this Makefile. They were listed above
as "deliberately out of scope" under the reason that applies to cargo and node —
which does not apply to an in-repo make target. ``make check`` recurses into
``rdf-core-hygiene``, so a hygiene gate added to the hygiene target was invisible
to this comparison. That was an annotated gap wearing the word "scope".
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
MAKEFILE = REPO_ROOT / "Makefile"
WORKFLOWS = REPO_ROOT / ".github" / "workflows"
CI_WORKFLOW = WORKFLOWS / "ci.yaml"

# WHICH WORKFLOWS GATE A PULL REQUEST IS READ, NOT LISTED. This was
# `frozenset({"ci.yaml"})`, which is narrower than the sentence above it: `docs.yaml`
# also triggers on `pull_request` and runs real gates through `make check-i18n`, so a
# hardcoded set both under-counted the blocking surface and would go stale the first
# time a workflow was added. Same defect shape as the two hand-maintained lists this
# whole gate exists to compare.
# THE `on:` BLOCK IS BOUNDED AND THEN TOKEN-MATCHED. The first version was
# `^on:\s*$.*?^\s{2}pull_request:` with DOTALL, which recognised ONE of six legal
# spellings and failed in both directions at once:
#
#   on:\n  pull_request:   -> True      on: [pull_request]   -> False
#   on: pull_request        -> False     on:\n  - pull_request  -> False
#   "on":\n  pull_request:  -> False     on:  # c\n  pull_request: -> False
#
# `"on":` is what YAML 1.1 parsers require, because bare `on` is a boolean. Reformatting
# `ci.yaml` to `on: [pull_request, push]` -- a pure style edit -- would have turned all
# forty gates red with a message that is a factual lie, AND blinded direction 2 to every
# CI-only gate. DOTALL gave the inverse too: a cron-only workflow with any two-space
# `pull_request:` key anywhere below `on:` was promoted to merge-blocking.
ON_BLOCK = re.compile(r'^["\']?on["\']?:(.*?)(?=^\S|\Z)', re.MULTILINE | re.DOTALL)
PULL_REQUEST_TOKEN = re.compile(r"(?<![\w-])pull_request(?![\w-])")


def strip_yaml_comments(text: str) -> str:
    """The text with `#` comments removed, so prose cannot be read as configuration.

    Crude on purpose -- a `#` inside a quoted scalar is rare in these workflows and
    removing it costs nothing here, whereas leaving comments in let an English sentence
    in a comment act as a `make` invocation. See `WORKFLOW_MAKE`.
    """
    stripped = []
    for line in text.splitlines():
        # QUOTE STATE FIRST. `run: echo "a # b" && make check` lost the `make check` target
        # entirely -- the hiding direction this gate exists to refuse -- because the `#`
        # inside the quoted scalar cut the line. Quoted spans are blanked before the comment
        # rule runs, so a `#` inside one cannot terminate anything.
        masked = re.sub(r'"[^"\n]*"', lambda m: '"' + "_" * (len(m.group(0)) - 2) + '"', line)
        masked = re.sub(r"'[^'\n]*'", lambda m: "'" + "_" * (len(m.group(0)) - 2) + "'", masked)
        cut = re.search(r"(?<!\S)#", masked)
        stripped.append(line[: cut.start()] if cut else line)
    return "\n".join(stripped)


# GATES A PULL-REQUEST WORKFLOW RUNS AND `make check` DELIBERATELY DOES NOT, with the
# reason. Each needs something `make check` does not take on -- a wasm32 toolchain, an
# mdbook install, or half an hour of corpus evaluation -- and forcing them in would make
# the local gate unrunnable, which is how a gate stops being run at all.
#
# This register may only SHRINK. An entry that no longer diverges is reported as stale, so
# an exemption cannot outlive its reason, and a NEW divergence is still a failure. That is
# the difference between a stated exception and the silence this file was written to end.
ONE_SIDED_BY_DESIGN: dict[str, str] = {
    "scripts/check-wasm-dataset-identity.sh": (
        "exercises the exact >2^53 Dataset/exchange ABI in Node and TypeScript, so it "
        "needs the wasm32 target, pinned wasm-bindgen CLI and package Node dependencies; "
        "`make wasm-pkg-test` is the local entry point"
    ),
    "scripts/check-wasm-test-runner.sh": (
        "observes the wasm32 test runner failing a planted panic, a refused flag and a "
        "sealed host read, so it needs the wasm32 target, the wasm-bindgen CLI and Node "
        "that only the wasm job installs; `make wasm-test` is the local entry point"
    ),
    "scripts/check-i18n-render.py --self-test": (
        "renders the book to check the translation, so it needs mdbook and the pinned "
        "mdbook-i18n-helpers; `make check-i18n` is the local entry point and says so"
    ),
    "scripts/conformance-matrix.py $(CONFORMANCE_ARGS)": (
        "evaluates the full W3C corpora, tens of minutes. `make conformance` is the local "
        "entry point; only its `--self-test` arm is cheap enough for `make check`"
    ),
    "scripts/check-simd-asm.py --doc $(SIMD_ASM_ARGS)": (
        "emits asm for seven target configurations; needs the wasm32 and aarch64 std "
        "targets. `make simd-asm` is the local entry point; only its `--self-test` arm "
        "runs in `make check`"
    ),
}


# The pinned count makes exemption growth an explicit change.
ONE_SIDED_COUNT = 5


def stale_exemptions(local: set[str], reachable: set[str]) -> list[str]:
    """Every registered exemption that no longer describes a divergence."""
    return [
        f"`{call}` is registered as deliberately one-sided ({reason}), but it now runs in "
        f"`make check` too -- the registration is stale, so delete it"
        for call, reason in sorted(ONE_SIDED_BY_DESIGN.items())
        if call in local
    ] + [
        f"`{call}` is registered as deliberately one-sided ({reason}), but no pull-request "
        f"workflow runs it either -- so nothing runs it, which the registration hides"
        for call, reason in sorted(ONE_SIDED_BY_DESIGN.items())
        if call not in local and call not in reachable
    ]


def merge_blocking(workflow_texts: dict[str, str]) -> dict[str, str]:
    """The workflows whose `on:` block includes `pull_request`, so they can block a merge."""
    blocking = {}
    for name, text in workflow_texts.items():
        block = ON_BLOCK.search(strip_yaml_comments(text))
        if block is not None and PULL_REQUEST_TOKEN.search(block.group(1)):
            blocking[name] = text
    return blocking

# Literal interpreter and executable prefixes establish command identity.
INVOCATION = re.compile(  # noqa: E501
    r"(?:\b(?:python3?|bash|sh)\s+(?:-\w+\s+)*|(?<![\w/])\./)"
    r"((?:scripts|crates)/[^\s]+)([^\n;&|#]*)",
    re.MULTILINE,
)
HELPER_POLICY_INVOCATION = re.compile(
    r"\bcargo\s+run\b[^\n;&|#]*?\s(?:-p|--package)\s+helper-census\s+--\s+"
    r"(--(?:no-features|python-binding-tests|glossary-gate|self-test|check-job-homes))([^\n;&|#]*)",
    re.MULTILINE,
)

# YAML keys whose value is metadata a human reads, never a command.
_YAML_PROSE_KEY = re.compile(r"^\s*(?:-\s*)?(?:name|if|id|description|summary):", re.MULTILINE)


def _drop_prose(text: str) -> str:
    """Blank quoted spans and YAML metadata without losing line structure."""
    kept = []
    for line in text.splitlines():
        if _YAML_PROSE_KEY.match(line):
            kept.append(line.split(":", 1)[0] + ":")
            continue
        # Blank double- and single-quoted spans, leaving the quotes so a following
        # separator is still visible.
        line = re.sub(r'"[^"\n]*"', '""', line)
        line = re.sub(r"'[^'\n]*'", "''", line)
        kept.append(line)
    return "\n".join(kept)


def _uncomment(text: str, comment_prefixes: tuple[str, ...]) -> str:
    """Remove whole-line comments, including Make recipe comments."""
    kept = []
    for line in text.splitlines():
        stripped = line.strip().lstrip("@").lstrip()
        if any(stripped.startswith(prefix) for prefix in comment_prefixes):
            continue
        kept.append(line)
    return "\n".join(kept)


def _join_continuations(text: str) -> str:
    """Join backslash continuations using horizontal indentation only."""
    return re.sub(r"\\\n[ \t]*", " ", text)


# TWO WAYS AN INVOCATION IS A GATE, stated separately because they are different rules.
#
# By NAME: repository check programs, reference oracles, and the shared source policy target.
GATE_BY_NAME = re.compile(
    r"^(?:scripts/(?:check-|conformance-matrix|benchmark-acquire|[\w-]+-queries)|crates/|helper-census )"
)
# By ARGUMENT: a `--self-test` or `--offline-self-test` run is a gate whatever the program
# is called. `publish-release-crates.sh` and `bootstrap-crates-io.sh` PUBLISH when invoked
# bare and are gates only in this form, which is exactly why the two rules cannot be one.
#
GATE_BY_ARGUMENT = re.compile(r"\s--(?:offline-)?self-test\b")


def _normalise(program: str, rest: str) -> str:
    """One invocation as `program args`, whitespace-collapsed.

    Arguments are part of the identity: ``--self-test`` and the bare run are
    different rules (one proves the gate can fail, the other applies it), and a
    list carrying only one of the pair is the divergence this refuses.
    """
    return " ".join((program + " " + rest).split())


def check_recipe(text: str, target: str, missing_is_fatal: bool = True) -> str:
    """Read tab-indented recipes, retaining entries after blank or comment lines."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.startswith(f"{target}:"):
            body: list[str] = []
            for following in lines[index + 1 :]:
                if following.startswith("\t"):
                    body.append(following)
                elif following.strip() == "" or following.lstrip().startswith("#"):
                    continue
                else:
                    break
            return "\n".join(body)
    if missing_is_fatal:
        sys.exit(f"FAIL: no `{target}:` target in {MAKEFILE}")
    # A `$(MAKE) <name>` that resolves to no recipe in THIS Makefile is not an error --
    # it may be a phony alias or defined by an include. Skipping is right; hard-failing
    # refused a legitimate Makefile.
    return ""


# `make <target>` inside a workflow step, including inside a `run: |` block. A workflow
# that reaches a gate this way was INVISIBLE to the comparison, and one does: `docs.yaml`
# runs `make check-i18n` on every pull request, which reaches
# `check-i18n-render.py --self-test` -- a gate absent from `make check` and from ci.yaml,
# so a contributor could not reproduce that red locally. That is direction 2's own stated
# motivation, still live after this gate was written for it.
# A `make` STEP, IN COMMAND POSITION, WITH EVERY TARGET IT NAMES.
#
# The first version was `(?:^|\s)make\s+([\w-]+)` over raw text, which is bypassable by an
# English sentence. `.github/workflows/ci.yaml:299` reads "step above alone does not make
# cargo use 1.98 inside this repo" and yielded the target `cargo` -- benign only because no
# `cargo:` target exists. A workflow containing "run make check before opening a PR" in a
# comment resolves the WHOLE `check` target, so every gate is reported as reached in CI and
# direction 1 is defeated by a sentence. Demonstrated on a fixture: two gates in no workflow
# at all report clean once that comment is present.
#
# It also took the FIRST word after `make`, so `make -C dir tgt` resolved `-C`, and the
# second target of `make a b` was dropped -- live at `docs.yaml:99`, where `playground` was
# never resolved.
WORKFLOW_MAKE = re.compile(r"(?:^|(?<=run:\s)|(?<=[;&|(]\s)|(?<=[;&|(]))\s*make\s+([^\n;&|]+)")
# A word that is a flag or a variable assignment rather than a target.
NOT_A_TARGET = re.compile(r"^(?:-|[A-Za-z_][\w]*=)")
# Flags whose following value is not a target name.
FLAGS_TAKING_A_VALUE = frozenset(
    {"-C", "-f", "-I", "-l", "-o", "-W", "--directory", "--file", "--include-dir",
     "--load-average", "--old-file", "--what-if", "--makefile", "--assume-old"}
)


def make_targets(text: str) -> set[str]:
    """Every target named by a `make` step in command position, comments removed."""
    targets: set[str] = set()
    for run in WORKFLOW_MAKE.findall(strip_yaml_comments(text)):
        words = run.split()
        skip_next = False
        for word in words:
            if skip_next:
                skip_next = False
                continue
            if word in FLAGS_TAKING_A_VALUE:
                skip_next = True
                continue
            if NOT_A_TARGET.match(word):
                continue
            if re.fullmatch(r"[\w-]+", word):
                targets.add(word)
    return targets


def invocations(text: str) -> set[str]:
    """Every in-repo program invocation in a blob of Makefile or YAML text."""
    # INLINE COMMENTS TOO, as `make_targets` already does. `_uncomment` removed only
    # whole-line comments, so a trailing `# ... python3 scripts/check-x.py` was read as a
    # running gate.
    prepared = _drop_prose(_join_continuations(strip_yaml_comments(_uncomment(text, ("#",)))))
    return {_normalise(m.group(1), m.group(2)) for m in INVOCATION.finditer(prepared)} | {
        _normalise("helper-census", m.group(1) + m.group(2))
        for m in HELPER_POLICY_INVOCATION.finditer(prepared)
    }


def workflow_invocations(text: str, makefile_text: str) -> set[str]:
    """Resolve a workflow's direct gates and those reached through its Make steps."""
    found = invocations(text)
    for target in make_targets(text):
        found |= invocations_with_recursion(makefile_text, target)
    return found


# Follow literal `$(MAKE)` edges because nested hygiene recipes share this Makefile.
MAKE_RECURSION = re.compile(r"\$\(MAKE\)((?:\s+-{1,2}[\w-]+)*)\s+([\w-]+)")


def invocations_with_recursion(makefile_text: str, target: str, seen: set[str] | None = None, read=invocations, edges=MAKE_RECURSION.findall) -> set[str]:
    """Collect reachable invocations. Dedicated readers reject missing recipes
    and real cycles, but permit shared targets."""
    seen = set() if seen is None else (seen.copy() if read is not invocations else seen)
    if target in seen:
        return set() if read is invocations else sys.exit(f"FAIL: recursive Make recipe `{target}`")
    seen.add(target)
    body = check_recipe(makefile_text, target, missing_is_fatal=read is not invocations)
    found = read(body)
    if read is invocations and target == "cargo-source-policy" and _uncomment(body, ("#",)).strip():
        found.add("make cargo-source-policy")
    recipe = strip_yaml_comments(_uncomment(body, ("#",)))
    for _flags, nested in edges(_drop_prose(recipe) if read is invocations else recipe):
        found |= invocations_with_recursion(makefile_text, nested, seen, read, edges)
    return found


def gates_only(found: set[str]) -> set[str]:
    """The invocations whose program name marks them as a repository gate."""
    return {
        call
        for call in found
        if call == "make cargo-source-policy" or GATE_BY_NAME.match(call) or GATE_BY_ARGUMENT.search(call)
    }


def divergence(makefile_text: str, workflow_texts: dict[str, str]) -> list[str]:
    """Every gate one list runs and the other does not, in both directions."""
    # THE ROOT TARGET MUST EXIST. `missing_is_fatal=True` was dead code -- the only call
    # site passed False -- so renaming `check` yielded `OK: all 0 hygiene gates`.
    check_recipe(makefile_text, "check")
    local = gates_only(invocations_with_recursion(makefile_text, "check"))
    if not local:
        return ["`make check` runs no hygiene gate at all, which is not a state this "
                "repository can be in -- the recipe is empty, renamed, or unparseable"]
    # MERGE-BLOCKING WORKFLOWS, not all of them. Direction 1's stated subject is "cannot
    # block a merge", and subtracting the union of all seven workflows checks something
    # weaker: three are tag-triggered and two are scheduled (`benchmarks.yaml` weekly,
    # `cnschema-probe.yaml` weekly), so a
    # gate present only in `benchmarks.yaml` would pass parity and block no merge. The
    # union is kept as a secondary, weaker message so a gate that at least runs SOMEWHERE
    # is distinguished from one that runs nowhere at all.
    blocking = merge_blocking(workflow_texts)
    everywhere: set[str] = set()
    for text in blocking.values():
        everywhere |= workflow_invocations(text, makefile_text)
    anywhere: set[str] = set()
    for text in workflow_texts.values():
        anywhere |= workflow_invocations(text, makefile_text)
    # Direction 2 asks which gates a PULL-REQUEST workflow runs that `make check` does
    # not, which is more than ci.yaml: `docs.yaml` gates translations on every PR.
    ci_gates: set[str] = set()
    for text in blocking.values():
        ci_gates |= gates_only(workflow_invocations(text, makefile_text))

    problems: list[str] = []
    for call in sorted(local - everywhere):
        where = "in NO workflow at all" if call not in anywhere else (
            "only in a workflow that does not gate a pull request (a tag, cron or "
            "path-filtered trigger)"
        )
        problems.append(
            f"`{call}` runs in `make check` and {where}, so it cannot block a merge"
        )
    if len(ONE_SIDED_BY_DESIGN) != ONE_SIDED_COUNT:
        problems.append(
            f"the one-sided register holds {len(ONE_SIDED_BY_DESIGN)} entries and "
            f"ONE_SIDED_COUNT says {ONE_SIDED_COUNT}. The register may only shrink, so an "
            f"addition must be a visible edit to that number and to whatever prose states "
            f"it -- the last growth went unnoticed and left a stale count in two files."
        )
    problems.extend(stale_exemptions(local, ci_gates))
    for call in sorted(ci_gates - local - set(ONE_SIDED_BY_DESIGN)):
        problems.append(
            f"`{call}` runs in a pull-request workflow and NOT in `make check`, so a red "
            "merge cannot be reproduced locally"
        )
    return problems


def _workflow_texts() -> dict[str, str]:
    # `*.yml` as well as `*.yaml`: a workflow under the other spelling was invisible to
    # both directions, which is a surface the gate never inspected.
    return {
        path.name: path.read_text(encoding="utf-8")
        for path in sorted([*WORKFLOWS.glob("*.yaml"), *WORKFLOWS.glob("*.yml")])
    }


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
    marker = "\tcargo run -q --locked -p helper-census -- --no-features\n"
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
    anchor = "        run: cargo run -q --locked -p helper-census -- --no-features\n"
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

    # THE REGISTER IS SHRINK-ONLY, in both directions. Without these two, an exemption
    # would outlive its reason silently -- and the second is the worse one: a registration
    # for a gate nothing runs at all reads exactly like a deliberate design choice.
    # One exemption is pretended to have moved into `make check`; the others are told they
    # are still reachable, so exactly one branch can fire and the check is about that branch.
    moved = next(iter(ONE_SIDED_BY_DESIGN))
    others = set(ONE_SIDED_BY_DESIGN) - {moved}
    found = stale_exemptions({moved}, others)
    if any(moved in problem and "runs in `make check` too" in problem for problem in found):
        print("OK: self-test — an exemption whose gate is now in `make check` is refused")
    else:
        print(f"SELF-TEST FAIL: a stale exemption was not refused ({found})")
        ok = False

    found = stale_exemptions(set(), set())
    if len(found) == len(ONE_SIDED_BY_DESIGN):
        print(
            f"OK: self-test — all {len(found)} exemptions are refused when nothing runs "
            "them, so a registration cannot hide a gate that runs nowhere"
        )
    else:
        print(f"SELF-TEST FAIL: an unrun exemption was not refused ({found})")
        ok = False

    # And the neighbour: the register as it stands, against the real tree, is silent.
    real_local = gates_only(invocations_with_recursion(real_makefile, "check"))
    real_reachable: set[str] = set()
    for text in merge_blocking(real_workflows).values():
        real_reachable |= gates_only(workflow_invocations(text, real_makefile))
    if stale_exemptions(real_local, real_reachable):
        print(f"SELF-TEST FAIL: the register as committed is stale: {stale_exemptions(real_local, real_reachable)}")
        ok = False
    else:
        print("OK: self-test — every registered exemption still describes a real divergence")

    # PROSE IN A COMMENT IS NOT A MAKE STEP. One English sentence in a workflow comment
    # resolved the whole `check` target and reported every gate as reached in CI --
    # direction 1 defeated by a sentence. The live false match was at ci.yaml:299.
    prose = "      # run make check before opening a PR\n"
    if make_targets(prose):
        print(f"SELF-TEST FAIL: a comment was read as a make step ({make_targets(prose)})")
        ok = False
    elif make_targets("        run: make rdf-core-hygiene\n") != {"rdf-core-hygiene"}:
        print("SELF-TEST FAIL: a real make step was not read as one")
        ok = False
    else:
        print("OK: self-test — a `make` in prose is not a step, and a real step still is")

    # EVERY TARGET A STEP NAMES, and no flag argument. `make a b` dropped `b` (live at
    # docs.yaml), and `make -C sub tgt` read `sub` as a target.
    if make_targets("        run: make a b\n") != {"a", "b"}:
        print("SELF-TEST FAIL: the second target of `make a b` is dropped")
        ok = False
    elif make_targets("        run: make -C sub tgt\n") != {"tgt"}:
        print("SELF-TEST FAIL: a flag's argument is read as a target")
        ok = False
    else:
        print("OK: self-test — every target is taken and no flag argument is")

    # SIX LEGAL SPELLINGS OF `on:`, because the first version recognised one and failed in
    # both directions at once: it would have turned all forty gates red over a style edit
    # AND blinded direction 2 to every CI-only gate.
    spellings = {
        "on:\n  pull_request:\n": True,
        "on: [pull_request]\n": True,
        "on: pull_request\n": True,
        "on:\n  - pull_request\n": True,
        '"on":\n  pull_request:\n': True,
        "on:  # a comment\n  pull_request:\n": True,
        "on:\n  schedule:\n    - cron: '0 0 * * 0'\n": False,
        "on:\n  push:\n    tags: ['v*']\n": False,
    }
    wrong = [
        form
        for form, want in spellings.items()
        if bool(merge_blocking({"probe.yaml": form})) is not want
    ]
    if wrong:
        print(f"SELF-TEST FAIL: `on:` spellings judged wrongly: {wrong}")
        ok = False
    else:
        print(
            f"OK: self-test — all {len(spellings)} `on:` spellings are judged correctly, "
            "including the two that must NOT be merge-blocking"
        )

    # A COMMENTED-OUT GATE IS NOT A RUNNING GATE, on either side.
    if invocations("\t# python3 scripts/check-invented.py\n"):
        print("SELF-TEST FAIL: a commented-out recipe line counts as a gate")
        ok = False
    elif invocations("        # run: python3 scripts/check-invented.py\n"):
        print("SELF-TEST FAIL: a commented-out workflow step counts as a gate")
        ok = False
    elif not invocations("\tpython3 scripts/check-invented.py\n"):
        print("SELF-TEST FAIL: an uncommented gate stopped counting")
        ok = False
    else:
        print("OK: self-test — a commented-out gate counts on neither side, an active one does")

    # EQUIVALENT SPELLINGS ARE ONE IDENTITY, or the gate refuses a workflow that is right.
    spellings_of_one = [
        "\tpython3 scripts/check-x.py --self-test\n",
        "        run: ./scripts/check-x.py --self-test\n",
        "        run: python3 -u scripts/check-x.py --self-test\n",
        "\tpython3 scripts/check-x.py \\\n\t\t--self-test\n",
    ]
    identities = {next(iter(invocations(spelling)), None) for spelling in spellings_of_one}
    if identities != {"scripts/check-x.py --self-test"}:
        print(f"SELF-TEST FAIL: equivalent spellings are not one identity: {identities}")
        ok = False
    else:
        print("OK: self-test — four spellings of one invocation normalise to one identity")

    # PROSE IS NOT AN INVOCATION, in the four shapes that read as one. Each was
    # demonstrated: an `echo` in the recipe produced a false refusal naming a gate that
    # does not exist, and a trailing comment silenced a full divergence.
    prose_shapes = {
        "an echo in a recipe": '\techo "scripts/check-terminal-predicates.py --help"\n',
        "a step name: field": "      - name: Check that python3 scripts/check-foo.py works\n",
        "an inline trailing comment": "        run: true # run python3 scripts/check-foo.py\n",
        "a quoted YAML list item": '          - "python3 scripts/check-qux.py"\n',
    }
    seen_prose = {label: invocations(text) for label, text in prose_shapes.items() if invocations(text)}
    if seen_prose:
        print(f"SELF-TEST FAIL: prose read as an invocation: {seen_prose}")
        ok = False
    else:
        print(f"OK: self-test — none of {len(prose_shapes)} prose shapes reads as an invocation")

    # AND THE NEIGHBOUR, which is what the first attempt at the rule above broke: every
    # real spelling must still be seen, including an interpreter behind a runner
    # (`uv run … python …`) and `./` inside a `run: |` body.
    real_shapes = {
        "a tab-indented recipe line": ("\tpython3 scripts/check-foo.py\n", "scripts/check-foo.py"),
        "a run: step": ("        run: python3 scripts/check-foo.py\n", "scripts/check-foo.py"),
        "an interpreter behind a runner": (
            "\tuv run --project x --no-sync python crates/shapes/tests/probe.py\n",
            "crates/shapes/tests/probe.py",
        ),
        "./ in a recipe": ("\t./scripts/check-foo.py\n", "scripts/check-foo.py"),
        "./ in a run: | body": ("        run: |\n          ./scripts/check-foo.py\n", "scripts/check-foo.py"),
    }
    unseen = {
        label: text for label, (text, wanted) in real_shapes.items() if wanted not in invocations(text)
    }
    if unseen:
        print(f"SELF-TEST FAIL: a real invocation was not seen: {sorted(unseen)}")
        ok = False
    else:
        print(f"OK: self-test — all {len(real_shapes)} real invocation spellings are seen")

    # A COLUMN-0 COMMENT INSIDE A RECIPE IS LEGAL MAKE and used to end it, taking `local`
    # from forty gates to one and emitting thirty-nine false messages.
    commented = real_makefile.replace(
        "\tcargo run -q --locked -p helper-census -- --no-features\n",
        "\tcargo run -q --locked -p helper-census -- --no-features\n# the licence gates follow\n",
        1,
    )
    if len(gates_only(invocations_with_recursion(commented, "check"))) != len(
        gates_only(invocations_with_recursion(real_makefile, "check"))
    ):
        print("SELF-TEST FAIL: a column-0 comment inside the recipe changes the gate set")
        ok = False
    else:
        print("OK: self-test — a comment inside the `check` recipe does not end it")

    # A `#` INSIDE A QUOTED SCALAR MUST NOT HIDE A MAKE TARGET.
    if make_targets('        run: echo "a # b" && make check\n') != {"check"}:
        print("SELF-TEST FAIL: a quoted `#` hides a make target")
        ok = False
    elif make_targets("        # run make check locally\n"):
        print("SELF-TEST FAIL: a real comment is no longer stripped")
        ok = False
    else:
        print("OK: self-test — a quoted `#` hides nothing and a real comment still strips")

    # A STRAY CONTINUATION MUST NOT JOIN ACROSS A BLANK LINE and swallow the next gate.
    joined = invocations("\tpython3 scripts/check-a.py \\\n\n\tpython3 scripts/check-b.py\n")
    if joined != {"scripts/check-a.py", "scripts/check-b.py"}:
        print(f"SELF-TEST FAIL: a continuation crossed a blank line: {sorted(joined)}")
        ok = False
    else:
        print("OK: self-test — a continuation joins its own line only")

    # A gate present in BOTH but with different arguments is a divergence too:
    # `--self-test` and the bare run are different rules.
    argument_drift = real_makefile.replace(
        "\tcargo run -q --locked -p helper-census -- --no-features\n",
        "\tcargo run -q --locked -p helper-census -- --no-features --self-test\n",
        1,
    )
    found = divergence(argument_drift, real_workflows)
    if any("helper-census --no-features --self-test" in problem for problem in found):
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

    if consumer_gate_problems(real_makefile, real_workflows):
        print("SELF-TEST FAIL: excluded consumer gates do not agree in the real tree")
        ok = False
    for command in CONSUMER_COMMANDS:
        absent_ci = {name: text.replace(command, "true") for name, text in real_workflows.items()}
        prose_ci = {name: text.replace(command, f'echo "{command}"') for name, text in real_workflows.items()}
        absent_local = real_makefile.replace(command, "true")
        if (not consumer_gate_problems(real_makefile, absent_ci)
                or not consumer_gate_problems(real_makefile, prose_ci)
                or not consumer_gate_problems(absent_local, real_workflows)):
            print(f"SELF-TEST FAIL: excluded consumer omission accepted: {command}")
            ok = False
    print("OK: self-test — excluded consumer omissions and quoted prose are refused")

    policy = "make cargo-source-policy"
    absent_ci = {name: text.replace(f"run: {policy}", f'run: echo "{policy}"') for name, text in real_workflows.items()}
    absent_local = real_makefile.replace("$(MAKE) cargo-source-policy", '@echo "$(MAKE) cargo-source-policy"')
    if (any(policy in problem for problem in divergence(real_makefile, real_workflows))
            or not all(any(policy in problem for problem in divergence(makefile, workflows))
                       for makefile, workflows in ((absent_local, real_workflows), (real_makefile, absent_ci)))):
        print("SELF-TEST FAIL: source policy route parity failed")
        ok = False
    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


# Workspace Cargo excludes this consumer so serde_json feature unification cannot change its uniqueItems semantics.
CONSUMER_RECIPE = re.compile(r'(?m)^\t[@+]*[ \t]*\$\(MAKE\)((?:[ \t]+(?:-s|--silent|--quiet|--no-print-directory))*)[ \t]+([\w-]+)(?:[ \t]+CARGO_POLICY_HELPER="")?[ \t]*$')
CONSUMER_MANIFEST = "crates/jsonschema/tests/preserve_order_consumer/Cargo.toml"
CONSUMER_COMMANDS = (
    f"cargo fmt --manifest-path {CONSUMER_MANIFEST} --check",
    f"cargo clippy --manifest-path {CONSUMER_MANIFEST} --all-targets --locked -- -D warnings",
    f"cargo test --manifest-path {CONSUMER_MANIFEST} --locked",
)


def consumer_edges(text: str) -> list[tuple[str, str]]:
    commands = _drop_prose(text)
    edges = CONSUMER_RECIPE.findall(commands)
    # A literal Make word, even after a shell wrapper, must not disappear as a route.
    make_word = r"""(?:[^\s"';|&()]+/)?g?make"""
    boundary = r"(?<![^\s;|&(@+-])"
    end = r"(?=[\s;|&)]|$)"
    quoted = rf"""(?P<quote>["'])(?:{make_word}|\$\(MAKE\))(?P=quote)"""
    shell = rf"""\b(?:ba|da)?sh[ \t]+-[\w]*c[\w]*[ \t]+["'][ \t]*(?:{make_word}|\$\(MAKE\)){end}"""
    if (len(edges) != commands.count("$(MAKE)") or re.search(boundary + make_word + end, commands)
            or re.search(rf"(?:{boundary}{quoted}{end}|{shell})", text)):
        sys.exit("FAIL: unsupported recursive Make invocation in consumer gate")
    return edges


def workflow_blocks(text: str, sequence: bool = False):
    """Yield direct entries with their opaque indented bodies."""
    lines = text.splitlines()
    depth = min((len(line) - len(line.lstrip(" ")) for line in lines if line.strip()), default=0)
    starts = [i for i, line in enumerate(lines) if line.strip() and line[depth] != " "
              and line[depth:].startswith("- ") == sequence]
    for start, end in zip(starts, starts[1:] + [len(lines)]):
        yield lines[start], "\n".join(lines[start + 1:end])


def workflow_fields(text: str) -> dict[str, tuple[str, str]]:
    fields = {}
    for head, body in workflow_blocks(text):
        key, colon, value = head.strip().partition(":")
        if colon:
            fields[key] = value.strip(), body
    return fields


def workflow_runs(text: str):
    """Read literal run scalars only under jobs, job, steps and a live step."""
    empty = ("", "")
    disabled = r"(?i)(?:false|\$\{\{\s*false\s*\}\})"
    value, jobs = workflow_fields(strip_yaml_comments(_uncomment(text, ("#",)))).get("jobs", empty)
    for job_value, body in workflow_fields(jobs if not value else "").values():
        job = workflow_fields(body if not job_value else "")
        value, steps = job.get("steps", empty)
        if value or re.fullmatch(disabled, job.get("if", empty)[0]):
            continue
        for head, body in workflow_blocks(steps, sequence=True):
            step = workflow_fields(head.replace("- ", "  ", 1) + "\n" + body)
            value, block = step.get("run", empty)
            if not re.fullmatch(disabled, step.get("if", empty)[0]):
                yield block if re.fullmatch(r"\|[-+]?\d?[-+]?", value) else value


def consumer_commands(text: str, workflow: bool = False) -> set[str]:
    prepared = _join_continuations(strip_yaml_comments(_uncomment(text, ("#",))))
    prefix = r"[ \t]*" if workflow else r"\t[@+]*[ \t]*"
    return {" ".join(command.split()) for command in re.findall(rf"(?m)^{prefix}(cargo[ \t]+[^\n]+)$", _drop_prose(prepared))}


def consumer_gate_problems(makefile_text: str, workflow_texts: dict[str, str]) -> list[str]:
    """Require each excluded-consumer gate locally and in a PR workflow."""
    local = invocations_with_recursion(makefile_text, "check", read=consumer_commands, edges=consumer_edges)
    ci = {command for text in merge_blocking(workflow_texts).values()
          for run in workflow_runs(text) for command in consumer_commands(run, workflow=True)}
    return [
        f"excluded preserve-order consumer gate `{command}` is absent from {where}"
        for command in CONSUMER_COMMANDS
        for where, text in (("make check", local), ("pull-request workflows", ci))
        if command not in text
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    makefile_text = MAKEFILE.read_text(encoding="utf-8")
    workflow_texts = _workflow_texts()
    problems = divergence(makefile_text, workflow_texts)
    problems.extend(consumer_gate_problems(makefile_text, workflow_texts))
    if problems:
        sys.exit(
            "FAIL: `make check` and the CI workflows do not run the same gates:\n  "
            + "\n  ".join(problems)
            + "\n  Two lists is two rules. Add the missing step so both lists say the same "
            "thing;\n  a gate only one of them runs is a gate that does not hold."
        )
    # THE SAME FUNCTION THAT DECIDED THE VERDICT. This line used to call plain
    # `invocations(check_recipe(...))` while `divergence()` used the recursive form, so the
    # first gate added under `rdf-core-hygiene` would have been compared and not counted.
    # They agree today only because the recursive set currently adds nothing.
    local = gates_only(invocations_with_recursion(MAKEFILE.read_text(encoding="utf-8"), "check"))
    print(
        f"OK: all {len(local)} hygiene gates in `make check` also run in a pull-request "
        "workflow, and vice versa"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
