#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Prove the crates.io release order is publishable AND verifiable, before a tag is.

`scripts/release-crates.sh` is the one definition of the release set and its
publish order; the release workflow, the token bootstrap and the crates.io
preflight all source it. `cargo publish` cannot be undone, so the order has to
be right before it runs, and "right" is four checks this script makes from the
workspace's own dependency graph (`cargo metadata`, offline):

  1. The release set is EXACTLY the publishable workspace members — every
     member without `publish = false`, and nothing else. A publishable crate
     the list omits never reaches crates.io; a listed crate that is not a
     member cannot be published at all.
  2. The order is a topological order of NORMAL dependencies. `cargo publish`
     of crate N needs every dependency of N already on the registry.
  3. The order is a topological order of DEV-dependencies too. `cargo publish`
     VERIFIES by building the packaged crate against the registry, and that
     build resolves the crate's whole graph, dev-dependencies included. One
     forward dev-edge is enough to make verification impossible for the set —
     which is the exact state `purrdf-geo` was in, and why the bootstrap used
     to run `--no-verify` (uploading artifacts nothing had built).
  4. `PURRDF_UNBOOTSTRAPPED_CRATES` — the ledger of crates known to lack a
     crates.io record — names only crates in the release set, in release order,
     with no duplicates. Whether each one really lacks a record is a fact about
     crates.io and is decided online by scripts/check-crates-io-records.sh.

Every failure names the edge or the crate, so the fix is an edit to the list,
not a re-run. `--self-test` perturbs each check and requires it to refuse.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent
_RELEASE_CRATES = _REPO / "scripts" / "release-crates.sh"


def _array(text: str, name: str) -> list[str]:
    body = re.search(rf"{name}=\((.*?)\)", text, re.DOTALL)
    if not body:
        raise SystemExit(f"check-publish-order: no {name} array in {_RELEASE_CRATES}")
    return [line.strip() for line in body.group(1).split() if line.strip()]


def load_lists() -> tuple[list[str], list[str]]:
    text = _RELEASE_CRATES.read_text(encoding="utf-8")
    return (
        _array(text, "PURRDF_RELEASE_CRATES"),
        _array(text, "PURRDF_UNBOOTSTRAPPED_CRATES"),
    )


def load_graph() -> tuple[set[str], dict[str, set[str]], dict[str, set[str]]]:
    """(publishable members, normal deps, dev deps) among workspace members."""
    proc = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
        capture_output=True,
        text=True,
        check=False,
        cwd=_REPO,
    )
    if proc.returncode != 0:
        raise SystemExit(f"check-publish-order: cargo metadata failed:\n{proc.stderr}")
    packages = json.loads(proc.stdout)["packages"]
    members = {p["name"] for p in packages}
    publishable = {p["name"] for p in packages if p.get("publish") != []}
    normal: dict[str, set[str]] = {}
    dev: dict[str, set[str]] = {}
    for p in packages:
        normal[p["name"]] = {
            d["name"] for d in p["dependencies"]
            if d["name"] in members and d["kind"] is None
        }
        dev[p["name"]] = {
            d["name"] for d in p["dependencies"]
            if d["name"] in members and d["kind"] == "dev"
        }
    return publishable, normal, dev


def check(
    order: list[str],
    ledger: list[str],
    publishable: set[str],
    normal: dict[str, set[str]],
    dev: dict[str, set[str]],
) -> list[str]:
    problems: list[str] = []
    listed = set(order)
    if len(listed) != len(order):
        dupes = sorted({c for c in order if order.count(c) > 1})
        problems.append(f"PURRDF_RELEASE_CRATES lists a crate more than once: {dupes}")

    # 1. the set
    for crate in sorted(publishable - listed):
        problems.append(
            f"`{crate}` is publishable (no `publish = false`) but is not in "
            f"PURRDF_RELEASE_CRATES — a `rust-v*` tag would never publish it"
        )
    for crate in sorted(listed - publishable):
        problems.append(
            f"`{crate}` is in PURRDF_RELEASE_CRATES but is not a publishable "
            f"workspace member"
        )

    # 2 + 3. the order, for each edge kind separately so the message says which
    position = {crate: index for index, crate in enumerate(order)}
    for kind, graph, consequence in (
        (
            "normal",
            normal,
            "`cargo publish` of the first would fail: its dependency is not on the "
            "registry yet",
        ),
        (
            "dev",
            dev,
            "`cargo publish` VERIFICATION of the first would fail: the packaged "
            "crate's dependency graph, dev-dependencies included, cannot resolve "
            "a sibling version that is not on the registry yet — the bootstrap "
            "would have to run `--no-verify` again",
        ),
    ):
        for crate in order:
            if crate not in position:
                continue
            for needed in sorted(graph.get(crate, set()) & listed):
                if position[needed] > position[crate]:
                    problems.append(
                        f"`{crate}` (position {position[crate] + 1}) has a {kind} "
                        f"dependency on `{needed}` (position {position[needed] + 1}), "
                        f"which is published AFTER it; {consequence}"
                    )

    # 4. the ledger
    if len(set(ledger)) != len(ledger):
        problems.append(
            f"PURRDF_UNBOOTSTRAPPED_CRATES lists a crate more than once: "
            f"{sorted({c for c in ledger if ledger.count(c) > 1})}"
        )
    for crate in ledger:
        if crate not in listed:
            problems.append(
                f"PURRDF_UNBOOTSTRAPPED_CRATES names `{crate}`, which is not in "
                f"PURRDF_RELEASE_CRATES"
            )
    in_release_order = [c for c in order if c in set(ledger)]
    if [c for c in ledger if c in listed] != in_release_order:
        problems.append(
            f"PURRDF_UNBOOTSTRAPPED_CRATES is not in publish order: it reads "
            f"{ledger}, the release order of those crates is {in_release_order}"
        )
    return problems


def self_test(
    order: list[str],
    ledger: list[str],
    publishable: set[str],
    normal: dict[str, set[str]],
    dev: dict[str, set[str]],
) -> list[str]:
    """Each check must refuse its own perturbation; a check that cannot is reported."""
    survived: list[str] = []

    def must_fail(label: str, *args: object) -> None:
        if not check(*args):  # type: ignore[arg-type]
            survived.append(label)

    # 1: drop a publishable crate from the list / add a non-member
    must_fail("a publishable crate missing from the list",
              order[:-1], ledger, publishable, normal, dev)
    must_fail("a non-member crate in the list",
              [*order, "purrdf-not-a-crate"], ledger, publishable, normal, dev)
    # 2: put a crate before one of its normal dependencies
    dependent = next(c for c in order if normal[c] & set(order))
    needed = sorted(normal[dependent] & set(order))[0]
    swapped = [c for c in order if c != dependent]
    swapped.insert(swapped.index(needed), dependent)
    must_fail("a normal dependency published after its dependent",
              swapped, ledger, publishable, normal, dev)
    # 3: put a crate before one of its dev dependencies
    dependent = next(c for c in order if dev[c] & set(order))
    needed = sorted(dev[dependent] & set(order))[0]
    swapped = [c for c in order if c != dependent]
    swapped.insert(swapped.index(needed), dependent)
    must_fail("a dev dependency published after its dependent",
              swapped, ledger, publishable, normal, dev)
    # 4: ledger naming a stranger / out of order / duplicated
    must_fail("a ledger entry outside the release set",
              order, [*ledger, "purrdf-not-a-crate"], publishable, normal, dev)
    if len(ledger) >= 2:
        must_fail("a ledger out of publish order",
                  order, list(reversed(ledger)), publishable, normal, dev)
        must_fail("a duplicated ledger entry",
                  order, [*ledger, ledger[0]], publishable, normal, dev)
    # and the real lists must pass, or every refusal above is meaningless
    if check(order, ledger, publishable, normal, dev):
        survived.append("the committed lists themselves (the control case fails)")
    return survived


def main(argv: list[str]) -> int:
    alone = "--self-test" in argv[1:]
    order, ledger = load_lists()
    publishable, normal, dev = load_graph()
    survived = self_test(order, ledger, publishable, normal, dev)
    if survived:
        print(
            "check-publish-order: this gate's own checks do not refuse:\n"
            + "\n".join(f"  - {entry}" for entry in survived),
            file=sys.stderr,
        )
        return 1
    if alone:
        print("check-publish-order self-test OK: every perturbation refused, control passes")
        return 0
    problems = check(order, ledger, publishable, normal, dev)
    if problems:
        print(
            "scripts/release-crates.sh is not a safe publish order:\n"
            + "\n".join(f"  - {p}" for p in problems)
            + "\n\ncargo publish cannot be undone. Fix the list; do not tag.",
            file=sys.stderr,
        )
        return 1
    forward_dev = sum(len(dev[c] & set(order)) for c in order)
    print(
        f"OK: {len(order)} release crates == the publishable members; the order is a "
        f"topological order of normal AND dev dependencies ({forward_dev} in-set dev "
        f"edges, all backward), so `cargo publish` can verify every crate; the "
        f"{len(ledger)}-entry unbootstrapped ledger is in-set and in order"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
