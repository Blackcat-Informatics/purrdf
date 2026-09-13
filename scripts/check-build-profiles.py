#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail unless the gate really compiles at ``opt-level = 3`` with the runtime checks on.

The gate is compute. ``make check`` runs the whole test surface and
``make conformance`` runs every W3C suite through it, and both are bounded by
how well the engines under them were compiled. This workspace once shipped that
property as four hand-written per-crate ``[profile.dev.package.X]`` tables, which
meant the SHACL validator, the ShEx validator, the SPARQL evaluator, the Datalog
substrate and the GTS codec — the crates the conformance corpora actually
exercise — compiled at ``opt-level = 0`` while the manifest looked tuned. Nobody
noticed, because nothing looks at the build the way the compiler does.

This is that gate. It does not read the manifest; a manifest is a request, not a
result. It asks Cargo for the resolved unit graph of the two invocations the
gates use — ``cargo test --workspace`` and ``cargo build --workspace`` — and
asserts the EFFECTIVE profile of every unit in them.

Reading effective values is the whole design, and it subsumes every mechanism
that can silently outrank the workspace manifest:

* ``$CARGO_HOME/config.toml`` and every ``.cargo/config.toml`` on the upward walk
  from the workspace — note that a developer's home directory is ON that walk, so
  a personal ``[profile.dev]`` governs this repository's gate without touching it;
* ``CARGO_PROFILE_*`` environment variables;
* ``--config`` command-line overrides.

A gate that instead enumerated those mechanisms would be a copy of Cargo's
resolution algorithm, and would go stale the first time a new one appeared. One
question is asked instead: what did the compiler actually get told?

What is asserted, and what is deliberately NOT:

* ASSERTED — ``opt_level``, ``debug_assertions`` and ``overflow_checks``, for
  every compiled unit. Optimization is the point of the gate. The two runtime
  checks are the reason an optimized gate is still a gate: first-party code
  carries ``#[cfg(debug_assertions)]`` bodies that vanish silently with the flag,
  and overflow checks are what keep an arithmetic bug in a byte-deterministic
  codec from becoming a wrong-but-green run. Dependencies keep them too; turning
  them off buys build speed with the internal sanity checks of the crypto,
  compression and columnar code, during precisely the runs whose job is finding
  bugs.
* NOT ASSERTED — ``debug``/``strip``. Debug-info size is disk hygiene, a
  preference the manifest states and a developer's own configuration may
  legitimately restate differently. Gating it would hard-fail contributors over
  something that cannot make a test lie.

``run-custom-build`` units are skipped: they are build-script *execution* steps,
not compilations, and Cargo reports a synthetic profile for them (today's graph
shows ``overflow_checks = false`` on all of them). The build scripts' own
*compile* units are ``mode = "build"`` and are checked like anything else.

Every field is read through :func:`field`, so a change to the explicitly
unstable ``--unit-graph`` schema is reported as a named, actionable failure
rather than a ``KeyError`` traceback. The schema moving under a floating nightly
is a finding about this script, and it should read like one.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent

# The contract. `opt_level` is reported by Cargo as a string.
REQUIRED_OPT_LEVEL = "3"

# Unit modes that are not compilations and therefore carry no codegen decision.
NON_COMPILE_MODES = frozenset({"run-custom-build"})

# The two invocations the gates actually make. `test` covers `make check` and
# every suite `scripts/conformance-matrix.py` spawns; `build` covers the paths
# that never build a test harness — `maturin develop`, `cargo capi build`, the
# examples, and a plain `cargo build` on a contributor's machine.
INVOCATIONS = ("test", "build")


class SchemaError(RuntimeError):
    """The ``--unit-graph`` schema no longer has a field this gate reads."""


def field(obj: Any, key: str, where: str) -> Any:
    """Read ``key`` from ``obj``, naming what was missing instead of raising ``KeyError``."""
    if not isinstance(obj, dict):
        raise SchemaError(f"{where}: expected a JSON object, found {type(obj).__name__}")
    if key not in obj:
        raise SchemaError(
            f"{where}: the unit-graph schema has no `{key}` field any more "
            f"(present: {', '.join(sorted(obj)) or 'nothing'}). "
            "`--unit-graph` is explicitly unstable; update "
            "scripts/check-build-profiles.py to the new spelling."
        )
    return obj[key]


def member_ids(repo: Path) -> set[str]:
    """The workspace members, by Cargo's own package id.

    Taken from ``cargo metadata --no-deps`` rather than by matching ``purrdf`` in
    the package id: the umbrella crate lives in ``crates/purrdf``, so Cargo renders
    its id as a bare ``path+file://...#1.1.0`` with no package name in it at all,
    and a name-matching gate would silently stop covering it.
    """
    out = _cargo(["metadata", "--no-deps", "--format-version", "1"], repo)
    packages = field(json.loads(out), "packages", "cargo metadata")
    return {field(p, "id", "cargo metadata package") for p in packages}


def unit_graph(subcommand: str, repo: Path) -> dict[str, Any]:
    """The resolved unit graph for one gate invocation."""
    out = _cargo(
        [
            "-Z",
            "unstable-options",
            subcommand,
            "--workspace",
            "--locked",
            "--unit-graph",
        ],
        repo,
    )
    return json.loads(out)


def _cargo(args: list[str], repo: Path) -> str:
    """Run Cargo in ``repo`` and return stdout, failing with actionable context."""
    proc = subprocess.run(
        ["cargo", *args],
        cwd=repo,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"`cargo {' '.join(args)}` failed with exit {proc.returncode}:\n"
            f"{proc.stderr.strip()}\n\n"
            "`--unit-graph` is a nightly-only unstable option. This workspace pins a "
            "floating nightly in rust-toolchain.toml, so the gate has it; a shell that "
            "forces another toolchain through RUSTUP_TOOLCHAIN does not."
        )
    return proc.stdout


def audit(graph: dict[str, Any], members: set[str], invocation: str) -> list[str]:
    """Every way ``graph`` fails the contract, as human-readable findings."""
    findings: list[str] = []
    units = field(graph, "units", f"{invocation} unit-graph")
    seen: set[str] = set()
    for index, unit in enumerate(units):
        where = f"{invocation} unit-graph unit[{index}]"
        mode = field(unit, "mode", where)
        pkg = field(unit, "pkg_id", where)
        seen.add(pkg)
        if mode in NON_COMPILE_MODES:
            continue
        profile = field(unit, "profile", where)
        # Read even though the label does not print it: a schema that loses
        # `target.name` must be a named failure here rather than a surprise in a
        # later revision of this gate.
        field(field(unit, "target", where), "name", f"{where} target")
        origin = "workspace member" if pkg in members else "dependency"
        # Named per PACKAGE, not per target. A de-optimized profile is never one
        # unit; printing each of a crate's twenty test targets separately buries
        # the fact that the whole graph is wrong.
        label = f"{_short(pkg)} [{origin}]"

        opt = field(profile, "opt_level", f"{where} profile")
        if str(opt) != REQUIRED_OPT_LEVEL:
            findings.append(f"{label}: opt-level {opt}, expected {REQUIRED_OPT_LEVEL}")
        if field(profile, "debug_assertions", f"{where} profile") is not True:
            findings.append(f"{label}: debug-assertions are OFF")
        if field(profile, "overflow_checks", f"{where} profile") is not True:
            findings.append(f"{label}: overflow-checks are OFF")

    for missing in sorted(members - seen):
        findings.append(
            f"{_short(missing)}: workspace member absent from the "
            f"`cargo {invocation} --workspace` unit graph — it is not being built at all"
        )
    return findings


def _short(pkg_id: str) -> str:
    """The readable tail of a Cargo package id."""
    return pkg_id.rsplit("#", 1)[-1] if "#" in pkg_id else pkg_id


def summarize(findings: list[str], *, limit: int = 25) -> list[str]:
    """Findings as a reader can act on them.

    A de-optimized profile is never one unit — it is every unit in the graph, so
    the raw list runs to hundreds of near-identical lines and buries the part a
    human needs. Workspace members come first (a first-party regression is the
    one this gate exists for), duplicates from a package's several targets
    collapse, and the tail is counted rather than printed.
    """
    members = [f for f in findings if "workspace member" in f]
    rest = [f for f in findings if "workspace member" not in f]
    lines: list[str] = []
    for group in (members, rest):
        seen: set[str] = set()
        deduped = [f for f in group if not (f in seen or seen.add(f))]
        lines.extend(deduped[:limit])
        if len(deduped) > limit:
            lines.append(f"... and {len(deduped) - limit} more of the same kind")
    return lines


def _unit(
    pkg: str,
    *,
    mode: str = "build",
    opt: str = REQUIRED_OPT_LEVEL,
    debug_assertions: bool = True,
    overflow_checks: bool = True,
    target: str = "lib",
) -> dict[str, Any]:
    """Build a unit-graph fixture with the supplied profile and target values."""
    return {
        "pkg_id": pkg,
        "mode": mode,
        "target": {"name": target},
        "profile": {
            "opt_level": opt,
            "debug_assertions": debug_assertions,
            "overflow_checks": overflow_checks,
        },
    }


def self_test() -> None:
    """Prove the gate is capable of failing, per the repo's self-test convention.

    One check per rule, in both directions: a gate that only demonstrates its
    refusals has not shown that it accepts the configuration the workspace ships,
    and over-refusal is as much a bug as under-refusal.
    """
    member = "path+file:///w/crates/shapes#purrdf-shapes@1.1.0"
    other = "path+file:///w/crates/purrdf#1.1.0"
    dep = "registry+https://github.com/rust-lang/crates.io-index#blake3@1.5.0"
    members = {member, other}

    def check(units: list[dict[str, Any]]) -> list[str]:
        return audit({"units": units}, members, "test")

    def require(condition: Any, message: str) -> None:
        if not condition:
            raise AssertionError(message)

    conforming = [_unit(member), _unit(other), _unit(dep)]
    require(not check(conforming), "the shipped configuration must pass")

    # Rule 1, opt-level, for BOTH origins. The member arm is the regression this
    # gate exists for: eleven members sat at opt-level 0 behind a tuned-looking
    # manifest, so a fixture that only proves a dependency can be caught would
    # have passed throughout that entire period.
    require(check([_unit(member, opt="0"), _unit(other), _unit(dep)]), (
        "an unoptimized workspace member must be caught"
    ))
    require(check([_unit(member), _unit(other), _unit(dep, opt="2")]), (
        "an unoptimized dependency must be caught"
    ))
    # The umbrella crate's package id carries no name; it must still be recognised
    # as a member rather than reported as a dependency.
    findings = check([_unit(member), _unit(other, opt="0"), _unit(dep)])
    require(findings and "workspace member" in findings[0], (
        "the nameless umbrella package id must still read as a member"
    ))
    # A build script's own COMPILE unit is a compilation like any other.
    require(check([*conforming, _unit(dep, opt="0", target="build-script-build")]), (
        "an unoptimized build-script compile unit must be caught"
    ))

    # Rule 2 and 3, the runtime checks, on both origins.
    require(check([_unit(member, debug_assertions=False), _unit(other), _unit(dep)]), (
        "a member with debug-assertions off must be caught"
    ))
    require(check([_unit(member, overflow_checks=False), _unit(other), _unit(dep)]), (
        "a member with overflow-checks off must be caught"
    ))
    require(check([_unit(member), _unit(other), _unit(dep, debug_assertions=False)]), (
        "a dependency with debug-assertions off must be caught"
    ))
    require(check([_unit(member), _unit(other), _unit(dep, overflow_checks=False)]), (
        "a dependency with overflow-checks off must be caught"
    ))

    # A build-script EXECUTION unit is not a compilation. Cargo reports
    # `overflow_checks = false` on every one of them today, so a gate that failed
    # to skip them would refuse the shipped configuration outright.
    require(
        not check(
            [*conforming, _unit(dep, mode="run-custom-build", opt="0", overflow_checks=False)]
        ),
        "a build-script execution unit must be skipped, not refused",
    )

    # A member that is not built at all is invisible to every per-unit rule.
    require(check([_unit(member), _unit(dep)]), (
        "a workspace member missing from the graph must be caught"
    ))

    # The unstable schema moving must name the field, not raise KeyError.
    for broken, missing in (
        ({"unit": []}, "units"),
        ({"units": [{"mode": "build", "target": {"name": "lib"}}]}, "pkg_id"),
        (
            {"units": [{"pkg_id": dep, "mode": "build", "target": {"name": "lib"}}]},
            "profile",
        ),
        (
            {
                "units": [
                    {
                        "pkg_id": dep,
                        "mode": "build",
                        "target": {"name": "lib"},
                        "profile": {"debug_assertions": True, "overflow_checks": True},
                    }
                ]
            },
            "opt_level",
        ),
    ):
        try:
            audit(broken, members, "test")
        except SchemaError as exc:
            require(missing in str(exc), f"the failure must name `{missing}`: {exc}")
        else:  # pragma: no cover - guards the guard
            raise AssertionError(f"a graph missing `{missing}` must be a SchemaError")

    print("check-build-profiles.py: self-test OK")


def main(argv: list[str]) -> int:
    """Run the self-test or audit Cargo's effective profiles for all gate invocations."""
    if "--self-test" in argv:
        self_test()
        return 0

    members = member_ids(REPO)
    failures: list[str] = []
    checked = 0
    for invocation in INVOCATIONS:
        graph = unit_graph(invocation, REPO)
        units = field(graph, "units", f"{invocation} unit-graph")
        checked += len(units)
        failures.extend(audit(graph, members, invocation))

    if failures:
        print(
            "The gate is not being compiled the way it is supposed to be. "
            f"Every unit must build at opt-level {REQUIRED_OPT_LEVEL} with "
            "debug-assertions and overflow-checks ON:",
            file=sys.stderr,
        )
        for line in summarize(failures):
            print(f"  {line}", file=sys.stderr)
        print(
            "\nThese are the EFFECTIVE values Cargo resolved, not what Cargo.toml "
            "asks for. If Cargo.toml already says the right thing, something is "
            "outranking it: a `[profile.*]` table in $CARGO_HOME/config.toml or in "
            "any .cargo/config.toml on the walk up from this workspace (your home "
            "directory is on that walk), a CARGO_PROFILE_* environment variable, or "
            "a --config override.",
            file=sys.stderr,
        )
        return 1

    print(
        f"check-build-profiles.py: {checked} units across "
        f"{len(INVOCATIONS)} gate invocations build at opt-level "
        f"{REQUIRED_OPT_LEVEL} with debug-assertions and overflow-checks on "
        f"({len(members)} workspace members)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
