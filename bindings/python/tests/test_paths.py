# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL property-path algebra for the compat shim (Task 6, #11).

Differential against the *real* rdflib: the operator overloads on ``URIRef``
build the same path algebra and render the same ``n3`` syntax, and evaluating a
path in a triple pattern (``graph.triples`` / accessors) yields the same
endpoints as rdflib's ``evalPath``.
"""

from __future__ import annotations

from types import ModuleType

import pytest

EX = "http://example.org/"


def _paths_module(mod: ModuleType) -> ModuleType:
    """Return the ``paths`` submodule for the compat shim or the oracle."""
    if mod.__name__ == "rdflib":
        import rdflib.paths as p

        return p
    from purrdf.compat.rdflib import paths as p

    return p


# ── n3 rendering parity ───────────────────────────────────────────────────────────


@pytest.mark.parametrize(
    "build",
    [
        lambda m, p, a, b, c: a / b,
        lambda m, p, a, b, c: a | b,
        lambda m, p, a, b, c: ~a,
        lambda m, p, a, b, c: -a,
        lambda m, p, a, b, c: a * p.OneOrMore,
        lambda m, p, a, b, c: a * p.ZeroOrMore,
        lambda m, p, a, b, c: a * p.ZeroOrOne,
        lambda m, p, a, b, c: a / b / c,
        lambda m, p, a, b, c: (a | b) / c,
        lambda m, p, a, b, c: (a / b) | c,
        lambda m, p, a, b, c: (a / b) * p.OneOrMore,
        lambda m, p, a, b, c: ~(a / b),
        lambda m, p, a, b, c: -(a | b),
    ],
)
def test_path_n3_matches_oracle(compat: ModuleType, oracle: ModuleType, build) -> None:  # type: ignore[no-untyped-def]
    """Every modelled path operator renders the same ``n3`` as rdflib."""

    def render(mod: ModuleType) -> str:
        p = _paths_module(mod)
        a, b, c = (mod.URIRef(f"{EX}{name}") for name in ("a", "b", "c"))
        return build(mod, p, a, b, c).n3()

    assert render(compat) == render(oracle)


# ── evaluation parity ─────────────────────────────────────────────────────────────


def _walk_graph(mod: ModuleType) -> object:
    """A small chain a→b→c→d plus a side edge, for path evaluation."""
    g = mod.Graph()
    p = mod.URIRef(f"{EX}p")
    q = mod.URIRef(f"{EX}q")
    g.add((mod.URIRef(f"{EX}a"), p, mod.URIRef(f"{EX}b")))
    g.add((mod.URIRef(f"{EX}b"), p, mod.URIRef(f"{EX}c")))
    g.add((mod.URIRef(f"{EX}c"), p, mod.URIRef(f"{EX}d")))
    g.add((mod.URIRef(f"{EX}a"), q, mod.URIRef(f"{EX}z")))
    return g


def _objects_via(mod: ModuleType, make_path) -> list[str]:  # type: ignore[no-untyped-def]
    """Objects reachable from ``ex:a`` through the built path (sorted strings)."""
    g = _walk_graph(mod)
    p = _paths_module(mod)
    path = make_path(mod, p)
    return sorted(
        str(o) for _s, _p, o in g.triples((mod.URIRef(f"{EX}a"), path, None))
    )


@pytest.mark.parametrize(
    "make_path",
    [
        lambda m, p: m.URIRef(f"{EX}p") * p.OneOrMore,
        lambda m, p: m.URIRef(f"{EX}p") * p.ZeroOrMore,
        lambda m, p: m.URIRef(f"{EX}p") * p.ZeroOrOne,
        lambda m, p: m.URIRef(f"{EX}p") / m.URIRef(f"{EX}p"),
        lambda m, p: m.URIRef(f"{EX}p") | m.URIRef(f"{EX}q"),
    ],
)
def test_path_eval_matches_oracle(
    compat: ModuleType, oracle: ModuleType, make_path
) -> None:  # type: ignore[no-untyped-def]
    """Evaluating a path from a fixed subject matches rdflib's endpoints."""
    assert _objects_via(compat, make_path) == _objects_via(oracle, make_path)


def test_inverse_path_subjects_match_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """An inverse-path walk (objects of ``~p`` from ``ex:d``) matches rdflib."""

    def walk(mod: ModuleType) -> list[str]:
        g = _walk_graph(mod)
        p = _paths_module(mod)
        path = ~mod.URIRef(f"{EX}p") * p.OneOrMore
        return sorted(str(o) for _s, _p, o in g.triples((mod.URIRef(f"{EX}d"), path, None)))

    assert walk(compat) == walk(oracle)


def test_bound_endpoints_yield_triple(compat: ModuleType, oracle: ModuleType) -> None:
    """A fully-bound path pattern yields the ``(s, path, o)`` triple in both."""

    def check(mod: ModuleType) -> int:
        g = _walk_graph(mod)
        p = _paths_module(mod)
        path = mod.URIRef(f"{EX}p") * p.OneOrMore
        return len(
            list(g.triples((mod.URIRef(f"{EX}a"), path, mod.URIRef(f"{EX}d"))))
        )

    assert check(compat) == check(oracle) == 1
