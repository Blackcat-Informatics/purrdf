#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Hold the first-party crate graph to the layering ``layers.toml`` declares.

A shared helper can only live where every caller reaches it, so the crate graph
decides where one implementation of a job may go. Left undeclared, that graph
grows by whatever edge was convenient on the day, and the lowest common crate of
two callers drifts upward until the only place a helper fits is a crate that
half its callers must not depend on. ``layers.toml`` is the declaration; this gate
compares it with the graph ``cargo metadata`` resolves, and fails on:

* a first-party NORMAL edge no row allows (an edge to the declared root is
  always allowed, and is listed like any other so the table stays exact);
* a workspace member without a row, or a row (not ``pending``) that names no
  member;
* a listed edge the resolved graph no longer has, and a ``pending`` edge or
  crate that now exists — the table records the graph exactly, so a planned edge
  is flipped to present in the change that adds it;
* a cycle in the declared graph, pending edges included.

``--ring-fence`` (``make rdf-core-hygiene``) holds the kernel ring-fence, which
counts EVERY normal dependency, external crates included: the root has none at
all, and a row carrying ``external`` is ring-fenced — each of its normal
dependencies is one of its listed first-party ``deps`` or one of the external
packages ``external`` names, and a listed external it no longer has is STALE.

``--home-for A B …`` answers the placement question: over the declared graph
(pending edges included), the crates every one of ``A``, ``B``, … reaches,
directly or transitively (each crate reaches itself), minus any that another of
them reaches — the common dependency closest to the callers. When the callers
share nothing else it is the root, which every crate may depend on.

The resolved graph comes from ``check-banned-deps.py``'s ``cargo_metadata`` and
``graph_from_metadata``; this script imports them rather than restating them.
"""

from __future__ import annotations

import importlib.util
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from types import ModuleType

REPO_ROOT = Path(__file__).resolve().parent.parent
LAYERS_PATH = REPO_ROOT / "layers.toml"


def load_banned_deps() -> ModuleType:
    """``scripts/check-banned-deps.py`` as a module (its name has a hyphen)."""
    path = Path(__file__).resolve().parent / "check-banned-deps.py"
    spec = importlib.util.spec_from_file_location("check_banned_deps", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules.setdefault("check_banned_deps", module)
    spec.loader.exec_module(module)
    return module


BANNED_DEPS = load_banned_deps()


class LayersError(ValueError):
    """``layers.toml`` is malformed; the message says where."""


@dataclass
class Layers:
    """The declared table."""

    root: str
    # crate -> {dependency -> pending}
    deps: dict[str, dict[str, bool]] = field(default_factory=dict)
    pending_crates: set[str] = field(default_factory=set)
    # ring-fenced crate -> the external packages it may depend on
    external: dict[str, set[str]] = field(default_factory=dict)


def parse_layers(document: dict) -> Layers:
    """Validate and read a parsed ``layers.toml``."""
    unknown = set(document) - {"root", "crate"}
    if unknown:
        raise LayersError(f"unknown top-level keys: {sorted(unknown)}")
    root = document.get("root")
    if not isinstance(root, str) or not root:
        raise LayersError("`root` must name a crate")
    rows = document.get("crate")
    if not isinstance(rows, list) or not rows:
        raise LayersError("there must be [[crate]] rows")
    layers = Layers(root=root)
    for row in rows:
        if not isinstance(row, dict):
            raise LayersError("a [[crate]] row must be a table")
        extra = set(row) - {"name", "pending", "deps", "external"}
        if extra:
            raise LayersError(f"row {row.get('name')!r}: unknown keys {sorted(extra)}")
        name = row.get("name")
        if not isinstance(name, str) or not name:
            raise LayersError("every row needs a `name`")
        if name in layers.deps:
            raise LayersError(f"`{name}` has two rows")
        pending = row.get("pending", False)
        if pending is not True and "pending" in row:
            raise LayersError(f"`{name}`: `pending` is `true` or absent")
        if pending:
            layers.pending_crates.add(name)
        entries = row.get("deps")
        if not isinstance(entries, list):
            raise LayersError(f"`{name}` needs a `deps` array")
        declared: dict[str, bool] = {}
        for entry in entries:
            if isinstance(entry, str):
                dep, dep_pending = entry, False
            elif isinstance(entry, dict) and set(entry) == {"name", "pending"}:
                dep, dep_pending = entry["name"], entry["pending"]
                if dep_pending is not True:
                    raise LayersError(f"`{name}` -> `{dep}`: write a present edge as a plain name")
            else:
                raise LayersError(f"`{name}`: a dep is a name or {{ name, pending = true }}")
            if dep in declared:
                raise LayersError(f"`{name}` lists `{dep}` twice")
            if dep == name:
                raise LayersError(f"`{name}` lists itself")
            declared[dep] = dep_pending
        layers.deps[name] = declared
        if "external" in row:
            external = row["external"]
            if not isinstance(external, list) or not all(isinstance(item, str) and item for item in external):
                raise LayersError(f"`{name}`: `external` is an array of package names")
            if len(set(external)) != len(external):
                raise LayersError(f"`{name}`: `external` lists a package twice")
            if name == root:
                raise LayersError(f"the root `{name}` is ring-fenced to nothing; it takes no `external`")
            layers.external[name] = set(external)
    if root not in layers.deps:
        raise LayersError(f"the root `{root}` needs a row")
    if layers.deps[root]:
        raise LayersError(f"the root `{root}` must have no dependencies")
    for name, declared in layers.deps.items():
        for dep in declared:
            if dep not in layers.deps:
                raise LayersError(f"`{name}` lists `{dep}`, which has no row")
    return layers


def resolved_edges(graph) -> tuple[set[str], set[tuple[str, str]]]:
    """(member names, first-party normal edges) of a ``ResolvedGraph``."""
    members = {graph.names[pid] for pid in graph.workspace}
    edges = {
        (graph.names[pid], graph.names[dep])
        for pid in graph.workspace
        for dep, kinds in graph.edges.get(pid, ())
        if dep in graph.workspace and "normal" in kinds
    }
    return members, edges


def cycle(layers: Layers) -> list[str] | None:
    """One cycle in the declared graph, or ``None``."""
    state: dict[str, int] = {}
    stack: list[str] = []

    def visit(name: str) -> list[str] | None:
        state[name] = 1
        stack.append(name)
        for dep in sorted(layers.deps[name]):
            if state.get(dep) == 1:
                return stack[stack.index(dep) :] + [dep]
            if dep not in state:
                found = visit(dep)
                if found:
                    return found
        stack.pop()
        state[name] = 2
        return None

    for name in sorted(layers.deps):
        if name not in state:
            found = visit(name)
            if found:
                return found
    return None


def failures(layers: Layers, members: set[str], edges: set[tuple[str, str]]) -> list[str]:
    """Every disagreement between the table and the resolved graph."""
    found: list[str] = []
    for name in sorted(members - set(layers.deps)):
        found.append(f"FAIL: workspace member `{name}` has no [[crate]] row in layers.toml")
    for name in sorted(set(layers.deps) - members - layers.pending_crates):
        found.append(
            f"FAIL: layers.toml row `{name}` names no workspace member (STALE); "
            "remove it or mark it `pending = true`"
        )
    for name in sorted(layers.pending_crates & members):
        found.append(
            f"FAIL: `{name}` is a workspace member now; drop `pending = true` from its row"
        )
    for source, target in sorted(edges):
        declared = layers.deps.get(source, {})
        if target in declared:
            continue
        if target == layers.root:
            found.append(
                f"FAIL: `{source}` -> `{target}` is not listed; an edge to the root is always "
                f"allowed, so add it to `{source}`'s row to keep the table exact"
            )
            continue
        found.append(
            f"FAIL: `{source}` -> `{target}` is a first-party normal edge layers.toml "
            "does not allow"
        )
    for source in sorted(layers.deps):
        for target, pending in sorted(layers.deps[source].items()):
            present = (source, target) in edges
            if pending and present:
                found.append(
                    f"FAIL: `{source}` -> `{target}` exists now; write it as a present edge "
                    "(drop `pending = true`)"
                )
            elif not pending and not present and source in members:
                found.append(
                    f"FAIL: `{source}` -> `{target}` is listed but the graph has no such "
                    "edge (STALE); remove it or mark it pending"
                )
    loop = cycle(layers)
    if loop:
        found.append(f"FAIL: the declared graph has a cycle: {' -> '.join(loop)}")
    return found


def normal_dependencies(graph) -> dict[str, set[str]]:
    """member name -> the names of every package it depends on through a NORMAL
    edge, on any target, first-party and external alike."""
    return {
        graph.names[pid]: {
            graph.names[dep] for dep, kinds in graph.edges.get(pid, ()) if "normal" in kinds
        }
        for pid in graph.workspace
    }


def ring_fence_failures(layers: Layers, dependencies: dict[str, set[str]]) -> list[str]:
    """Every breach of the kernel ring-fence: a normal dependency of the root, a
    normal dependency of a ring-fenced crate its row does not list, and a listed
    external dependency the crate no longer has."""
    found: list[str] = []
    for name in sorted(dependencies.get(layers.root, set())):
        found.append(
            f"FAIL: `{layers.root}` is the root and must have zero runtime dependencies, "
            f"but depends on `{name}`"
        )
    for crate, external in sorted(layers.external.items()):
        if crate not in dependencies:
            if crate not in layers.pending_crates:
                found.append(f"FAIL: ring-fenced `{crate}` is not a workspace member")
            continue
        allowed = set(layers.deps[crate]) | external
        for name in sorted(dependencies[crate] - allowed):
            found.append(
                f"FAIL: ring-fenced `{crate}` depends on `{name}`, which its layers.toml row "
                "lists neither in `deps` nor in `external`"
            )
        for name in sorted(external - dependencies[crate]):
            found.append(
                f"FAIL: ring-fenced `{crate}` lists `{name}` in `external` but no longer "
                "depends on it (STALE); remove it"
            )
    return found


def reach(layers: Layers, start: str) -> set[str]:
    """Every crate ``start`` reaches over the declared graph, itself included."""
    seen = {start}
    stack = [start]
    while stack:
        for dep in layers.deps[stack.pop()]:
            if dep not in seen:
                seen.add(dep)
                stack.append(dep)
    return seen


def home_for(layers: Layers, callers: list[str]) -> list[str]:
    """The common dependencies of ``callers`` closest to them."""
    unknown = [name for name in callers if name not in layers.deps]
    if unknown:
        raise LayersError(f"no layers.toml row for {', '.join(unknown)}")
    common = set.intersection(*(reach(layers, name) for name in callers))
    if not common:
        return [layers.root]
    reached = {name: reach(layers, name) - {name} for name in common}
    return sorted(
        name
        for name in common
        if not any(name in reached[other] for other in common if other != name)
    )


def fixture_graph(extra_edges: tuple[tuple[str, str], ...] = (), members: tuple[str, ...] = ("r", "a", "b", "c")):
    """A resolved graph of first-party members: c -> b -> a -> r, plus ``extra_edges``."""
    edges = {("b", "a"), ("c", "b"), ("a", "r")} | set(extra_edges)

    def package(name: str) -> dict:
        return {"id": name, "name": name, "source": None, "publish": None, "targets": [{"kind": ["lib"]}]}

    return BANNED_DEPS.graph_from_metadata(
        {
            "workspace_members": list(members),
            "packages": [package(name) for name in members],
            "resolve": {
                "nodes": [
                    {
                        "id": name,
                        "deps": [
                            {"pkg": target, "dep_kinds": [{"kind": None, "target": None}]}
                            for source, target in sorted(edges)
                            if source == name and target in members
                        ],
                    }
                    for name in members
                ]
            },
        }
    )


FIXTURE_LAYERS = """
root = "r"
[[crate]]
name = "r"
deps = []
[[crate]]
name = "a"
deps = ["r"]
[[crate]]
name = "b"
deps = ["a", { name = "p", pending = true }]
[[crate]]
name = "c"
deps = ["b"]
[[crate]]
name = "p"
pending = true
deps = []
"""


def self_test() -> int:
    """Seeded positive and negative cases for every failure and for --home-for."""
    cases: list[tuple[str, bool]] = []
    layers = parse_layers(tomllib.loads(FIXTURE_LAYERS))

    def run(graph) -> list[str]:
        members, edges = resolved_edges(graph)
        return failures(layers, members, edges)

    cases.append(("the declared graph, as resolved, passes", run(fixture_graph()) == []))
    cases.append(
        (
            "an undeclared first-party edge fails",
            any("`c` -> `a`" in line for line in run(fixture_graph((("c", "a"),)))),
        )
    )
    unlisted_root = run(fixture_graph((("c", "r"),)))
    cases.append(
        (
            "an unlisted edge to the root is asked to be listed, not refused as a layering breach",
            len(unlisted_root) == 1 and "always allowed" in unlisted_root[0],
        )
    )
    cases.append(
        (
            "a listed edge the graph lost is STALE",
            any("STALE" in line for line in run(_without(("c", "b")))),
        )
    )
    cases.append(
        (
            "a member with no row fails",
            any("`x` has no" in line for line in run(fixture_graph(members=("r", "a", "b", "c", "x")))),
        )
    )
    pending_present = run(fixture_graph((("b", "p"),), members=("r", "a", "b", "c", "p")))
    cases.append(
        (
            "a pending crate and edge that now exist must be flipped",
            any("drop `pending = true` from its row" in line for line in pending_present)
            and any("`b` -> `p` exists now" in line for line in pending_present),
        )
    )
    cyclic = parse_layers(tomllib.loads(FIXTURE_LAYERS.replace('deps = ["r"]', 'deps = ["r", "c"]')))
    cases.append(("a declared cycle fails", cycle(cyclic) is not None))
    cases.append(("the fixture has no cycle", cycle(layers) is None))
    cases.append(("home-for a caller and its dependency is the dependency", home_for(layers, ["c", "a"]) == ["a"]))
    cases.append(("home-for two chained callers is the lower one", home_for(layers, ["c", "b"]) == ["b"]))
    cases.append(("home-for a caller alone is itself", home_for(layers, ["b"]) == ["b"]))
    cases.append(("home-for crates that share nothing else is the root", home_for(layers, ["p", "a"]) == ["r"]))
    fenced = parse_layers(
        tomllib.loads(
            FIXTURE_LAYERS.replace('name = "a"\ndeps = ["r"]', 'name = "a"\ndeps = ["r"]\nexternal = ["ext"]')
        )
    )
    held = {"r": set(), "a": {"r", "ext"}, "b": {"a"}, "c": {"b"}}
    cases.append(("a ring-fenced crate whose every dependency is listed passes", ring_fence_failures(fenced, held) == []))
    cases.append(
        (
            "a ring-fenced crate with an unlisted external dependency fails",
            any("depends on `other`" in line for line in ring_fence_failures(fenced, {**held, "a": {"r", "ext", "other"}})),
        )
    )
    cases.append(
        (
            "an unfenced crate may take an external dependency",
            ring_fence_failures(fenced, {**held, "b": {"a", "other"}}) == [],
        )
    )
    cases.append(
        (
            "a listed external the ring-fenced crate dropped is STALE",
            any("STALE" in line for line in ring_fence_failures(fenced, {**held, "a": {"r"}})),
        )
    )
    cases.append(
        (
            "a root with any runtime dependency fails the ring-fence",
            any("zero runtime dependencies" in line for line in ring_fence_failures(fenced, {**held, "r": {"ext"}})),
        )
    )
    with_dev_and_build = fixture_graph()
    with_dev_and_build.edges["a"].append(("c", frozenset({"dev"})))
    with_dev_and_build.edges["b"].append(("r", frozenset({"build"})))
    with_dev_and_build.edges["c"].append(("a", frozenset({"normal", "dev"})))
    cases.append(
        (
            "normal edges are counted, dev-only and build-only edges are not",
            normal_dependencies(with_dev_and_build) == {"r": set(), "a": {"r"}, "b": {"a"}, "c": {"a", "b"}},
        )
    )
    for bad, why in [
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = []\nexternal = []\n', "an `external` on the root"),
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = []\n[[crate]]\nname = "a"\ndeps = []\nexternal = ["x", "x"]\n', "a doubled external"),
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = ["a"]\n[[crate]]\nname = "a"\ndeps = []\n', "a root with dependencies"),
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = []\n[[crate]]\nname = "a"\ndeps = ["z"]\n', "a dep with no row"),
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = []\n[[crate]]\nname = "a"\ndeps = [{ name = "r", pending = false }]\n', "a pending = false entry"),
        ('root = "r"\n[[crate]]\nname = "r"\ndeps = []\nlayer = 1\n', "an unknown key"),
    ]:
        try:
            parse_layers(tomllib.loads(bad))
            cases.append((f"{why} is refused", False))
        except LayersError:
            cases.append((f"{why} is refused", True))
    failed = 0
    for name, held in cases:
        print(f"{'PASS' if held else 'FAIL'}: {name}")
        failed += not held
    if failed:
        print(f"check-layers self-test: {failed} case(s) failed")
        return 1
    print("OK: check-layers self-test")
    return 0


def _without(edge: tuple[str, str]):
    """The fixture graph with one of its own edges removed."""
    graph = fixture_graph()
    source, target = edge
    graph.edges[source] = [(dep, kinds) for dep, kinds in graph.edges[source] if dep != target]
    return graph


def load_layers() -> Layers:
    with LAYERS_PATH.open("rb") as handle:
        return parse_layers(tomllib.load(handle))


def main() -> int:
    arguments = sys.argv[1:]
    if arguments == ["--self-test"]:
        return self_test()
    try:
        layers = load_layers()
    except (OSError, tomllib.TOMLDecodeError, LayersError) as exc:
        print(f"FAIL: layers.toml: {exc}")
        return 1
    if arguments == ["--ring-fence"]:
        try:
            graph = BANNED_DEPS.graph_from_metadata(BANNED_DEPS.cargo_metadata(REPO_ROOT / "Cargo.toml"))
        except BANNED_DEPS.MetadataError as exc:
            print(f"FAIL: {exc}")
            return 1
        found = ring_fence_failures(layers, normal_dependencies(graph))
        if found:
            for line in found:
                print(line)
            return 1
        fenced = ", ".join(sorted(layers.external))
        print(
            f"OK: `{layers.root}` has zero runtime dependencies; every runtime dependency of "
            f"{fenced} is listed in its layers.toml row"
        )
        return 0
    if arguments[:1] == ["--home-for"]:
        callers = arguments[1:]
        if not callers:
            print("usage: check-layers.py --home-for CRATE [CRATE ...]", file=sys.stderr)
            return 2
        try:
            homes = home_for(layers, callers)
        except LayersError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        for name in homes:
            print(name)
        return 0
    if arguments:
        print("usage: check-layers.py [--self-test | --ring-fence | --home-for CRATE [CRATE ...]]", file=sys.stderr)
        return 2
    try:
        graph = BANNED_DEPS.graph_from_metadata(BANNED_DEPS.cargo_metadata(REPO_ROOT / "Cargo.toml"))
    except BANNED_DEPS.MetadataError as exc:
        print(f"FAIL: {exc}")
        return 1
    members, edges = resolved_edges(graph)
    found = failures(layers, members, edges)
    if found:
        for line in found:
            print(line)
        return 1
    pending = sum(pending for deps in layers.deps.values() for pending in deps.values())
    print(
        f"OK: {len(edges)} first-party normal edges across {len(members)} members match "
        f"layers.toml ({pending} planned edges, {len(layers.pending_crates)} planned crates)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
