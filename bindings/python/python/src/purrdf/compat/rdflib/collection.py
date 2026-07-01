# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""RDF-list (``Collection``) read/write helper for the purrdf compat shim.

Walks / materializes ``rdf:first``/``rdf:rest``/``rdf:nil`` chains over the
:class:`~purrdf.compat.rdflib.graph.Graph` facade — no native dependency.
"""

from __future__ import annotations

from collections.abc import Iterator

from .graph import Graph
from .namespace import RDF
from .term import BNode, Identifier


class Collection:
    """An RDF list anchored at ``uri`` within ``graph`` (RDFLib parity)."""

    def __init__(
        self, graph: Graph, uri: Identifier, seq: list[Identifier] | None = None
    ) -> None:
        """Bind to the list at ``uri``; if ``seq`` is given, materialize it."""
        self.graph = graph
        self.uri = uri
        if seq is not None:
            self._set(list(seq))

    def __iter__(self) -> Iterator[Identifier]:
        """Yield the list members in order."""
        node: Identifier = self.uri
        while node != RDF.nil:
            first = self.graph.value(node, RDF.first)
            if first is None:
                return
            yield first
            rest = self.graph.value(node, RDF.rest)
            if rest is None:
                return
            node = rest

    def __len__(self) -> int:
        """Return the number of list members."""
        return sum(1 for _ in self)

    def __getitem__(self, index: int) -> Identifier:
        """Return the member at ``index``."""
        for i, item in enumerate(self):
            if i == index:
                return item
        raise IndexError(index)

    def _set(self, items: list[Identifier]) -> None:
        """Materialize ``items`` as ``rdf:first``/``rest`` triples from ``self.uri``."""
        node: Identifier = self.uri
        for i, item in enumerate(items):
            self.graph.add((node, RDF.first, item))
            if i == len(items) - 1:
                self.graph.add((node, RDF.rest, RDF.nil))
            else:
                nxt: Identifier = BNode()
                self.graph.add((node, RDF.rest, nxt))
                node = nxt
        if not items:
            # An empty collection is rdf:nil; nothing to attach beyond the anchor.
            return
