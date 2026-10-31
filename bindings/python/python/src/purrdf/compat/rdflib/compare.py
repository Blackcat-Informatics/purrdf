# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Graph comparison for the purrdf compat shim (RDFLib ``rdflib.compare``).

Isomorphism and canonicalization route through the native RDFC-1.0 surface
(:class:`purrdf.Dataset` with an explicit ``canonical_search`` policy), which is
RDF-1.2-safe — unlike RDFLib's blank-node comparison.
"""

from __future__ import annotations

import purrdf

from .graph import Graph

_NT = purrdf.RdfFormat.N_TRIPLES


def _canonical_quads(graph: Graph, canonical_search: str) -> list[str]:
    """Return the graph's RDFC-1.0-canonical N-Triples lines, sorted."""
    nt = graph.serialize(format="nt", encoding="utf-8")
    dataset = purrdf.Dataset(purrdf.parse(nt, format=_NT))
    dataset.canonicalize(canonical_search=canonical_search).value
    store = purrdf.Store()
    for quad in dataset:
        store.add(quad)
    lines = store.dump(format=_NT).decode("utf-8").splitlines()
    return sorted(line for line in lines if line.strip())


def isomorphic(graph1: Graph, graph2: Graph, *, canonical_search: str) -> bool:
    """Return whether two graphs are isomorphic (RDF-1.2-safe, RDFC-1.0)."""
    return _canonical_quads(graph1, canonical_search) == _canonical_quads(graph2, canonical_search)


def to_isomorphic(graph: Graph, *, canonical_search: str) -> Graph:
    """Return an RDFC-1.0-canonicalized copy of ``graph`` (RDFLib parity name)."""
    return to_canonical_graph(graph, canonical_search=canonical_search)


def to_canonical_graph(graph: Graph, *, canonical_search: str) -> Graph:
    """Return a copy of ``graph`` with RDFC-1.0-canonical blank-node labels."""
    nt = graph.serialize(format="nt", encoding="utf-8")
    dataset = purrdf.Dataset(purrdf.parse(nt, format=_NT))
    dataset.canonicalize(canonical_search=canonical_search).value
    store = purrdf.MutableDataset()
    for quad in dataset:
        store.add(quad)
    return Graph(store)


def graph_diff(graph1: Graph, graph2: Graph, *, canonical_search: str) -> tuple[Graph, Graph, Graph]:
    """Return ``(in_both, only_in_first, only_in_second)`` after canonicalization."""
    lines1 = set(_canonical_quads(graph1, canonical_search))
    lines2 = set(_canonical_quads(graph2, canonical_search))
    return (
        _graph_from_nt(lines1 & lines2),
        _graph_from_nt(lines1 - lines2),
        _graph_from_nt(lines2 - lines1),
    )


def _graph_from_nt(lines: set[str]) -> Graph:
    """Build a graph from a set of N-Triples lines."""
    graph = Graph()
    payload = "\n".join(lines).encode("utf-8")
    if payload.strip():
        graph._store.load(payload, format=_NT)
    return graph
