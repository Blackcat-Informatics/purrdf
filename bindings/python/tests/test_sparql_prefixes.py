# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""SPARQL prefix-block generation for the compat shim.

Covers ``Graph._build_prefix_block``: deterministic merging of graph namespace
bindings with ``initNs``, ``initNs`` precedence over graph bindings, and the
rule that in-text ``PREFIX`` declarations stay authoritative — by coming after
the block in the prologue, never by reading the query text.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"
OVERRIDE = "http://example.org/override/"
GRAPH_NS = "http://example.org/graph/"


def _build_block(mod: ModuleType, initNs: dict[str, object] | None = None) -> str:
    """Call ``_build_prefix_block`` on a fresh graph."""
    g = mod.Graph()
    g.bind("ex", mod.Namespace(GRAPH_NS))
    return g._build_prefix_block(initNs)


def test_initns_overrides_graph_binding(compat: ModuleType) -> None:
    """A prefix supplied in ``initNs`` overrides the graph's namespace binding."""
    block = _build_block(compat, initNs={"ex": compat.Namespace(OVERRIDE)})
    assert "PREFIX ex: <http://example.org/override/>" in block
    assert "PREFIX ex: <http://example.org/graph/>" not in block


def _subjects(mod: ModuleType, query: str) -> set[str]:
    """Run ``query`` over a graph holding one ``ex:Thing`` per namespace."""
    g = mod.Graph()
    g.bind("ex", mod.Namespace(GRAPH_NS))
    for ns in (GRAPH_NS, f"{EX}text/"):
        g.add(
            (
                mod.URIRef(f"{ns}s"),
                mod.URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                mod.URIRef(f"{ns}Thing"),
            )
        )
    return {str(row[0]) for row in g.query(query)}


def test_in_text_declaration_takes_precedence(compat: ModuleType, oracle: ModuleType) -> None:
    """A prefix the query text declares wins over the graph's binding for it."""
    query = "PREFIX ex: <http://example.org/text/> SELECT ?s WHERE { ?s a ex:Thing }"
    assert _subjects(compat, query) == {f"{EX}text/s"} == _subjects(oracle, query)
    # The control: without the in-text declaration the graph binding applies.
    control = "SELECT ?s WHERE { ?s a ex:Thing }"
    assert _subjects(compat, control) == {f"{GRAPH_NS}s"} == _subjects(oracle, control)


def test_a_prefix_inside_a_literal_does_not_suppress_the_graph_binding(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``PREFIX ex:`` inside a string literal declares nothing.

    The query-text scan this replaced saw it, took ``ex`` for declared in the text,
    left the graph's binding out of the prologue, and so failed the query on an
    undeclared prefix.
    """
    query = (
        'SELECT ?s WHERE { ?s a ex:Thing FILTER (STR(?s) != "PREFIX ex: <urn:x:> ") }'
    )
    assert _subjects(compat, query) == {f"{GRAPH_NS}s"} == _subjects(oracle, query)


def test_sorted_and_duplicate_free(compat: ModuleType) -> None:
    """The generated block is sorted by prefix name and contains no duplicates."""
    block = _build_block(
        compat,
        initNs={"ex": compat.Namespace(OVERRIDE), "ex2": compat.Namespace(f"{EX}two/")},
    )
    ex_lines = [
        line for line in block.strip().split("\n") if line.startswith(("PREFIX ex:", "PREFIX ex2:"))
    ]
    assert ex_lines == [
        "PREFIX ex: <http://example.org/override/>",
        "PREFIX ex2: <http://example.org/two/>",
    ]
