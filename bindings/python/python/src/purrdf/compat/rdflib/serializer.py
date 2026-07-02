# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Serializer plugin *kind* base class (RDFLib ``rdflib.serializer``).

Mirrors RDFLib's ``rdflib.serializer.Serializer`` enough for
``plugin.get(name, Serializer)`` to resolve against the same *kind* identity.
The concrete built-in serializers live in
:mod:`purrdf.compat.rdflib.plugins.serializers`; :meth:`Graph.serialize`
resolves a serializer class through the registry, instantiates it with the
graph, and drives it into a byte stream.
"""

from __future__ import annotations

from typing import IO, TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .graph import Graph

__all__ = ["Serializer"]


class Serializer:
    """Base class for an RDF serializer *kind* (the ``plugin.get`` key type)."""

    def __init__(self, store: Graph) -> None:
        """Bind the serializer to the graph it will emit."""
        self.store = store
        self.encoding: str = "utf-8"
        self.base: str | None = None

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Serialize :attr:`store` into ``stream`` (overridden by concrete serializers)."""
        raise NotImplementedError
