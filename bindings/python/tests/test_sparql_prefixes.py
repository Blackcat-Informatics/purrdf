# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL prefix-block generation for the compat shim.

Covers ``Graph._build_prefix_block``: deterministic merging of graph namespace
bindings with ``initNs``, ``initNs`` precedence over graph bindings, and the
rule that in-text ``PREFIX`` / ``@prefix`` declarations stay authoritative.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"
OVERRIDE = "http://example.org/override/"
GRAPH_NS = "http://example.org/graph/"


def _build_block(mod: ModuleType, query_text: str, initNs: dict[str, object] | None = None) -> str:
    """Call ``_build_prefix_block`` on a fresh graph."""
    g = mod.Graph()
    g.bind("ex", mod.Namespace(GRAPH_NS))
    return g._build_prefix_block(query_text, initNs)


def test_initns_overrides_graph_binding(compat: ModuleType) -> None:
    """A prefix supplied in ``initNs`` overrides the graph's namespace binding."""
    block = _build_block(
        compat,
        "SELECT ?s WHERE { ?s a ex:Thing }",
        initNs={"ex": compat.Namespace(OVERRIDE)},
    )
    assert "PREFIX ex: <http://example.org/override/>" in block
    assert "PREFIX ex: <http://example.org/graph/>" not in block


def test_in_text_declaration_takes_precedence(compat: ModuleType) -> None:
    """A prefix already declared in the query text is not redeclared."""
    block = _build_block(
        compat,
        "PREFIX ex: <http://example.org/text/> SELECT ?s WHERE { ?s a ex:Thing }",
        initNs={"ex": compat.Namespace(OVERRIDE)},
    )
    assert "PREFIX ex: <http://example.org/text/>" not in block
    assert "PREFIX ex: <http://example.org/override/>" not in block
    assert "PREFIX ex: <http://example.org/graph/>" not in block


def test_sorted_and_duplicate_free(compat: ModuleType) -> None:
    """The generated block is sorted by prefix name and contains no duplicates."""
    block = _build_block(
        compat,
        "SELECT ?s WHERE { ?s a ex:Thing ; ex2:name ?n }",
        initNs={"ex": compat.Namespace(OVERRIDE), "ex2": compat.Namespace(f"{EX}two/")},
    )
    ex_lines = [
        line for line in block.strip().split("\n") if line.startswith(("PREFIX ex:", "PREFIX ex2:"))
    ]
    assert ex_lines == [
        "PREFIX ex: <http://example.org/override/>",
        "PREFIX ex2: <http://example.org/two/>",
    ]
