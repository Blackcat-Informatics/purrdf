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
        container = self._get_container(index)
        if container is None:
            raise IndexError(index)
        value = self.graph.value(container, RDF.first)
        if value is None:
            raise IndexError(index)
        return value

    def __setitem__(self, index: int, value: Identifier) -> None:
        """Replace the member at ``index``."""
        container = self._get_container(index)
        if container is None:
            raise IndexError(index)
        self.graph.set((container, RDF.first, value))

    def __delitem__(self, key: int | slice) -> None:
        """Delete the member at ``key`` (int, negative int, or slice).

        Raises ``IndexError`` for out-of-range indices.
        """
        length = len(self)
        if isinstance(key, slice):
            indices = list(range(*key.indices(length)))
            # Delete from the back so indices remain stable.
            for idx in reversed(indices):
                self._del_index(idx, length)
            return

        if not isinstance(key, int):
            raise TypeError("collection indices must be integers or slices")
        if key < 0:
            key = length + key
        self._del_index(key, length)

    def _del_index(self, key: int, length: int) -> None:
        """Delete a single item by non-negative index."""
        if key < 0 or key >= length:
            raise IndexError(key)

        current = self._get_container(key)
        assert current is not None

        if length == 1:
            # Removing the only item leaves the anchor as an empty collection.
            self.graph.remove((current, RDF.first, None))
            self.graph.remove((current, RDF.rest, None))
            return

        if key == length - 1:
            # Removing the tail: point the previous cell at rdf:nil.
            prior = self._get_container(key - 1)
            assert prior is not None
            self.graph.set((prior, RDF.rest, RDF.nil))
            self.graph.remove((current, None, None))
            return

        if key == 0:
            # Removing the head: copy the second cell into the anchor, then
            # drop the now-redundant second cell.
            second = self._get_container(1)
            assert second is not None
            second_first = self.graph.value(second, RDF.first)
            second_rest = self.graph.value(second, RDF.rest)
            assert second_first is not None and second_rest is not None
            self.graph.set((self.uri, RDF.first, second_first))
            self.graph.set((self.uri, RDF.rest, second_rest))
            self.graph.remove((second, None, None))
            return

        # Splice out the middle cell.
        nxt = self._get_container(key + 1)
        prior = self._get_container(key - 1)
        assert nxt is not None and prior is not None
        self.graph.remove((current, None, None))
        self.graph.set((prior, RDF.rest, nxt))

    def _get_container(self, index: int) -> Identifier | None:
        """Return the list cell at ``index`` (the node holding its rdf:first).

        Supports negative indices following Python sequence semantics; returns
        ``None`` when the normalized index is out of bounds.
        """
        if index < 0:
            index = len(self) + index
        if index < 0:
            return None
        container: Identifier | None = self.uri
        i = 0
        while i < index:
            i += 1
            if container is None:
                return None
            container = self.graph.value(container, RDF.rest)
            if container is None:
                return None
        return container

    def _end(self) -> Identifier:
        """Return the last cell of the list (or ``self.uri`` if empty)."""
        container: Identifier = self.uri
        while True:
            rest = self.graph.value(container, RDF.rest)
            if rest is None or rest == RDF.nil:
                return container
            container = rest

    def append(self, item: Identifier) -> Collection:
        """Append ``item`` to the tail of the list."""
        end = self._end()
        if end == RDF.nil:
            raise ValueError("Cannot append to empty list")

        if (end, RDF.first, None) in self.graph:
            node = BNode()
            self.graph.set((end, RDF.rest, node))
            end = node

        self.graph.add((end, RDF.first, item))
        self.graph.add((end, RDF.rest, RDF.nil))
        return self

    def clear(self) -> Collection:
        """Remove all ``rdf:first``/``rdf:rest`` triples in this list's chain."""
        container: Identifier | None = self.uri
        while container is not None and container != RDF.nil:
            rest = self.graph.value(container, RDF.rest)
            self.graph.remove((container, RDF.first, None))
            self.graph.remove((container, RDF.rest, None))
            container = rest
        return self

    def _set(self, items: list[Identifier]) -> None:
        """Materialize ``items`` as ``rdf:first``/``rest`` triples from ``self.uri``."""
        self.clear()
        node: Identifier = self.uri
        for i, item in enumerate(items):
            self.graph.add((node, RDF.first, item))
            if i == len(items) - 1:
                self.graph.add((node, RDF.rest, RDF.nil))
            else:
                nxt: Identifier = BNode()
                self.graph.add((node, RDF.rest, nxt))
                node = nxt
        # An empty collection is rdf:nil; nothing to attach beyond the anchor.
