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

Scope, stated rather than implied: only invocations of a ``scripts/`` (or in-repo
``crates/.../*.py``) program are compared. ``cargo`` steps and the ``node`` schema
oracles are out of scope — CI distributes those across jobs (``wasm``, ``pytest``,
``capi``) and spells several differently on purpose, so requiring textual equality
there would refuse a correct workflow. A gate that over-refuses gets disabled, and
then it guards nothing.

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
    "scripts/check-geo-determinism.sh": (
        "cross-checks native against wasm32 output, so it needs the pinned wasm toolchain "
        "that only the wasm job installs"
    ),
    "scripts/check-hnsw-determinism.sh": (
        "same as the geo determinism check: a native-versus-wasm32 comparison needing the "
        "wasm toolchain"
    ),
    "scripts/check-i18n-render.py --self-test": (
        "renders the book to check the translation, so it needs mdbook and the pinned "
        "mdbook-i18n-helpers; `make check-i18n` is the local entry point and says so"
    ),
    "crates/shapes/tests/pydantic_oracle.py": (
        "an emitter oracle run through `uv run --project bindings/python`, so it needs the "
        "Python environment the pytest job builds; `make pydantic-oracle` is the local "
        "entry point"
    ),
    "crates/shapes/tests/linkml_oracle.py": (
        "same as the pydantic oracle: a `uv`-driven emitter check needing the built Python "
        "environment, with `make linkml-oracle` as the local entry point"
    ),
    "scripts/conformance-matrix.py": (
        "evaluates the full W3C corpora, tens of minutes. `make conformance` is the local "
        "entry point; only its `--self-test` arm is cheap enough for `make check`"
    ),
    "scripts/check-simd-asm.py": (
        "emits asm for seven target configurations; needs the wasm32 and aarch64 std "
        "targets. `make simd-asm` is the local entry point; only its `--self-test` arm "
        "runs in `make check`"
    ),
}


# HOW MANY EXEMPTIONS THERE ARE, pinned. The register's comment says it "may only
# SHRINK", and `stale_exemptions` only caught entries that stopped diverging -- nothing
# refused an ADDITION. It grew from four to six inside this change, and a stale "Four" in
# both the changelog and the PR body is the proof that nothing noticed. Growth is now a
# deliberate, visible edit to this number.
ONE_SIDED_COUNT = 7


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

# `python3 scripts/x.py --flag`, `bash scripts/x.sh --flag`. The interpreter is
# part of the match but not of the identity: what matters is which program runs
# with which arguments, and the Makefile and the workflow must agree on that.
# The argument run stops at a shell separator. `([^\n]*)` was greedy to end of line, so
# `python3 scripts/a.py && python3 scripts/b.py` produced the single identity
# `scripts/a.py && python3 scripts/b.py` -- the second program never seen, and one list
# spelling a pair on one line while the other spells it on two reported divergence in BOTH
# directions over an equivalent spelling. No such line today; the regex is the hazard.
# An invocation of an in-repo program. `python3 scripts/x.py --flag`,
# `bash scripts/x.sh`, `./scripts/x.py`, `python -u scripts/x.py`.
#
# The first version hardcoded `python3|bash` and a bare `scripts/` prefix, so it REFUSED
# equivalent spellings: `scripts/check-no-features.py` is mode 100755 with a shebang, and
# `./scripts/check-no-features.py` was reported as a workflow not running a gate it
# demonstrably runs. The argument run also stopped at `;&|` but not `#`, so a trailing
# comment became part of a gate's identity.
# The interpreter (or `./`) is REQUIRED, not optional. Making it optional matched every
# bare `scripts/…` or `crates/…` path in the tree -- YAML path filters, `cp` arguments,
# quoted strings inside a `run: |` block -- and produced twenty-one refusals naming things
# that are not gates at all. An invocation is a program being RUN, and that is what the
# prefix establishes.
# PROSE IS EXCLUDED BY WHAT IT IS, NOT BY WHERE IT SITS. The first attempt anchored the
# interpreter in command position, which is wrong twice: it still admitted an unquoted
# `name:` field, and it REJECTED a real invocation, because `uv run --project … python
# crates/shapes/tests/pydantic_oracle.py` has its interpreter mid-command behind a runner.
#
# What distinguishes prose from a command is that prose lives inside a quoted string or a
# YAML metadata value. Four shapes read as invocations before this: an `echo "…"`, a step
# `name:` field, an inline trailing comment, and a quoted YAML list item. Demonstrated: a
# full divergence was silenced by adding `run: true # reproduce locally with python3
# scripts/check-no-features.py`, and an `echo` inside the recipe produced a false refusal
# naming a gate that does not exist.
INVOCATION = re.compile(  # noqa: E501
    r"(?:\b(?:python3?|bash|sh)\s+(?:-\w+\s+)*|(?<![\w/])\./)"
    r"((?:scripts|crates)/[^\s]+)([^\n;&|#]*)",
    re.MULTILINE,
)

# YAML keys whose value is metadata a human reads, never a command.
_YAML_PROSE_KEY = re.compile(r"^\s*(?:-\s*)?(?:name|if|id|description|summary):", re.MULTILINE)


def _drop_prose(text: str) -> str:
    """The text with quoted spans and YAML metadata values blanked.

    A quoted string is data — `echo "run scripts/check-x.py"` documents a gate, it does not
    run one — and a step's `name:` is a label. Blanking rather than deleting keeps line
    numbers and surrounding structure intact for everything else on the line.
    """
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
    """The text with comment lines removed.

    A COMMENTED-OUT GATE IS NOT A RUNNING GATE, and both lists were scanned as raw text.
    A cleanly commented-out CI step and a recipe line commented as `@# python3 ...` each
    satisfied the comparison while running nothing -- and "comment it out for now" is how
    a gate actually dies. Whole-line only: an inline `#` inside a quoted scalar is not
    worth the false positives, and a gate invocation is never mid-line prose.
    """
    kept = []
    for line in text.splitlines():
        stripped = line.strip().lstrip("@").lstrip()
        if any(stripped.startswith(prefix) for prefix in comment_prefixes):
            continue
        kept.append(line)
    return "\n".join(kept)


def _join_continuations(text: str) -> str:
    """Backslash-continued lines joined, so one invocation is one identity.

    The idiomatic tab-indented `python3 scripts/x.py \\` / `--self-test` yielded the
    identity `scripts/x.py \\` and was refused in BOTH directions against the semantically
    identical single-line `run:` -- the precise failure the argument-run comment above says
    it fixed for `&&`.
    """
    # HORIZONTAL whitespace only. `\s` includes `\n`, so a stray trailing backslash joined
    # across a blank line: two real gates vanished from the set AND a phantom identity
    # appeared -- the same defect this joiner was added to fix for `&&`, one round later.
    return re.sub(r"\\\n[ \t]*", " ", text)


# TWO WAYS AN INVOCATION IS A GATE, stated separately because they are different rules.
#
# By NAME: a `check-*` program, the conformance matrix, the workload acquirer, a query
# instantiator, or an in-repo `crates/**` reference oracle.
GATE_BY_NAME = re.compile(
    r"^(?:scripts/(?:check-|conformance-matrix|benchmark-acquire|[\w-]+-queries)|crates/)"
)
# By ARGUMENT: a `--self-test` or `--offline-self-test` run is a gate whatever the program
# is called. `publish-release-crates.sh` and `bootstrap-crates-io.sh` PUBLISH when invoked
# bare and are gates only in this form, which is exactly why the two rules cannot be one.
#
# The original single pattern had a `.*-self-test` arm that matched the argument while the
# comment said it matched the name. That was right in effect and wrong in description --
# and narrowing it to the name alone silently dropped those two gates, taking the agreed
# count from 40 to 38.
GATE_BY_ARGUMENT = re.compile(r"\s--(?:offline-)?self-test\b")


def _normalise(program: str, rest: str) -> str:
    """One invocation as `program args`, whitespace-collapsed.

    Arguments are part of the identity: ``--self-test`` and the bare run are
    different rules (one proves the gate can fail, the other applies it), and a
    list carrying only one of the pair is the divergence this refuses.
    """
    return " ".join((program + " " + rest).split())


def check_recipe(text: str, target: str, missing_is_fatal: bool = True) -> str:
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
                elif following.strip() == "" or following.lstrip().startswith("#"):
                    # A COLUMN-0 COMMENT INSIDE A RECIPE IS LEGAL MAKE, and it ended the
                    # recipe -- so one comment took `local` from 40 gates to 1 and emitted
                    # 39 messages saying gates do not run in `make check` that demonstrably
                    # do. The docstring reasoned about what ends a recipe, got blank lines
                    # right, and got comments wrong.
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
# cargo use 1.96 inside this repo" and yielded the target `cargo` -- benign only because no
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
# Flags whose VALUE is the next word, so that word is not a target either. `make -C sub
# tgt` yielded `sub` as well as `tgt` -- harmless while no `sub:` recipe exists, and a
# collision waiting for one.
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
    return {_normalise(m.group(1), m.group(2)) for m in INVOCATION.finditer(prepared)}


def workflow_invocations(text: str, makefile_text: str) -> set[str]:
    """Every invocation a workflow reaches, following its `make <target>` steps.

    A workflow's gates are not only the ones it spells out. Resolving its `make` steps
    through the Makefile is what makes "runs in CI" mean what the comparison assumes.
    """
    found = invocations(text)
    for target in make_targets(text):
        found |= invocations_with_recursion(makefile_text, target)
    return found


# `$(MAKE) <target>` inside a recipe. `make check` recurses into `rdf-core-hygiene` and
# `wasm`, so a gate added to one of those was invisible to this comparison. The original
# scope note called `$(MAKE)` recursions "deliberately out of scope" and justified it with
# the reason that applies to cargo and node steps -- CI spells those differently across
# jobs -- which does not apply to an in-repo make target at all. That was an annotated gap
# wearing the word "scope", and `check_recipe` already existed to close it.
# `$(MAKE) [flags] <target>`. `[\w-]+` matched `--no-print-directory`, which this
# Makefile uses at line 170, and the recursion then hard-failed with "no
# `--no-print-directory:` target" -- refusing a legitimate Makefile and blaming a target
# that does not exist.
MAKE_RECURSION = re.compile(r"\$\(MAKE\)((?:\s+-{1,2}[\w-]+)*)\s+([\w-]+)")


def invocations_with_recursion(makefile_text: str, target: str, seen: set[str] | None = None) -> set[str]:
    """Every invocation reachable from `target`, following `$(MAKE)` into this Makefile.

    `seen` guards against a recipe cycle: a target that recurses into itself, directly or
    through another, would otherwise recurse forever rather than report.
    """
    seen = set() if seen is None else seen
    if target in seen:
        return set()
    seen.add(target)
    body = check_recipe(makefile_text, target, missing_is_fatal=False)
    found = invocations(body)
    for _flags, nested in MAKE_RECURSION.findall(body):
        found |= invocations_with_recursion(makefile_text, nested, seen)
    return found


def gates_only(found: set[str]) -> set[str]:
    """The invocations whose program name marks them as a repository gate."""
    return {
        call
        for call in found
        if GATE_BY_NAME.match(call) or GATE_BY_ARGUMENT.search(call)
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
        "\tpython3 scripts/check-no-features.py\n",
        "\tpython3 scripts/check-no-features.py\n# the licence gates follow\n",
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
