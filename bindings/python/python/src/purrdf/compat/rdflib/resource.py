# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""A resource-oriented view over a :class:`~.graph.Graph` (RDFLib ``Resource``).

A :class:`Resource` binds a graph and a subject identifier so callers can read
and write statements about that subject without threading both the graph and a
"current subject" through every call. Node values that are references
(:class:`URIRef`/:class:`BNode`) yielded from the accessors are wrapped back as
:class:`Resource` instances — the "resource oriented" style RDFLib documents.
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import TYPE_CHECKING, Any

from .namespace import RDF
from .term import BNode, Identifier, URIRef

if TYPE_CHECKING:
    from .graph import Graph

#: A node that may be wrapped as a :class:`Resource` (an IRI or blank node).
_Ref = URIRef | BNode


def _ident(value: Any) -> Any:
    """Unwrap ``value`` to its underlying identifier if it is a :class:`Resource`."""
    if isinstance(value, Resource):
        return value._identifier
    return value


class Resource:
    """A wrapper binding a :class:`~.graph.Graph` and a subject identifier."""

    def __init__(self, graph: Graph, subject: Identifier) -> None:
        """Bind ``graph`` and the resource's ``subject`` identifier."""
        self._graph = graph
        self._identifier = subject

    @property
    def graph(self) -> Graph:
        """The graph this resource reads from / writes to."""
        return self._graph

    @property
    def identifier(self) -> Identifier:
        """The resource's subject identifier."""
        return self._identifier

    def __hash__(self) -> int:
        """Hash over ``(type, graph, identifier)`` — mirrors RDFLib."""
        return hash(Resource) ^ hash(self._graph) ^ hash(self._identifier)

    def __eq__(self, other: object) -> bool:
        """Equal when both graph and identifier match (RDFLib parity)."""
        return (
            isinstance(other, Resource)
            and self._graph == other._graph
            and self._identifier == other._identifier
        )

    def __ne__(self, other: object) -> bool:
        """Negate :meth:`__eq__`."""
        return not self == other

    def __lt__(self, other: object) -> bool:
        """Order by identifier (RDFLib parity)."""
        if isinstance(other, Resource):
            return bool(self._identifier < other._identifier)
        return False

    def __gt__(self, other: object) -> bool:
        """Strictly greater by identifier."""
        return not (self < other or self == other)

    def __le__(self, other: object) -> bool:
        """Less-than-or-equal by identifier."""
        return self < other or self == other

    def __ge__(self, other: object) -> bool:
        """Greater-than-or-equal by identifier."""
        return not self < other

    # ── mutation ────────────────────────────────────────────────────────────────

    def add(self, p: Identifier | Resource, o: Identifier | Resource) -> None:
        """Add ``(self, p, o)`` to the graph."""
        self._graph.add((self._identifier, _ident(p), _ident(o)))

    def remove(
        self, p: Identifier | Resource, o: Identifier | Resource | None = None
    ) -> None:
        """Remove ``(self, p, o)`` (``o=None`` = any object)."""
        self._graph.remove((self._identifier, _ident(p), _ident(o)))

    def set(self, p: Identifier | Resource, o: Identifier | Resource) -> None:
        """Replace all ``(self, p, *)`` objects with ``o``."""
        self._graph.set((self._identifier, _ident(p), _ident(o)))

    # ── accessors (reference values re-wrapped as Resources) ────────────────────

    def subjects(
        self, predicate: Identifier | Resource | None = None
    ) -> Iterator[Resource]:
        """Yield subjects of ``(*, predicate, self)`` as resources."""
        return self._resources(
            self._graph.subjects(_ident(predicate), self._identifier)
        )

    def predicates(
        self, o: Identifier | Resource | None = None
    ) -> Iterator[Resource]:
        """Yield predicates of ``(self, *, o)`` as resources."""
        return self._resources(self._graph.predicates(self._identifier, _ident(o)))

    def objects(
        self, predicate: Identifier | Resource | None = None
    ) -> Iterator[Resource]:
        """Yield objects of ``(self, predicate, *)`` as resources."""
        return self._resources(
            self._graph.objects(self._identifier, _ident(predicate))
        )

    def subject_predicates(
        self,
    ) -> Iterator[tuple[Any, Any]]:
        """Yield ``(subject, predicate)`` pairs pointing at this resource."""
        return self._resource_pairs(
            self._graph.subject_predicates(self._identifier)
        )

    def subject_objects(self) -> Iterator[tuple[Any, Any]]:
        """Yield ``(subject, object)`` pairs for this resource's predicate."""
        return self._resource_pairs(self._graph.subject_objects(self._identifier))

    def predicate_objects(self) -> Iterator[tuple[Any, Any]]:
        """Yield ``(predicate, object)`` pairs about this resource."""
        return self._resource_pairs(
            self._graph.predicate_objects(self._identifier)
        )

    def value(
        self,
        p: Identifier | Resource = RDF.value,
        o: Identifier | Resource | None = None,
        default: Identifier | None = None,
        any: bool = True,  # noqa: A002 - RDFLib API name
    ) -> Any:
        """Return the single ``(self, p, *)`` object (cast to a resource)."""
        return self._cast(
            self._graph.value(self._identifier, _ident(p), _ident(o), default, any)
        )

    def items(self) -> Iterator[Resource]:
        """Yield the members of the ``rdf:List`` anchored at this resource."""
        return self._resources(self._graph.items(self._identifier))

    def transitive_objects(
        self, predicate: Identifier | Resource, remember: Any = None
    ) -> Iterator[Resource]:
        """Yield this resource and every object reachable via ``predicate``."""
        return self._resources(
            self._graph.transitive_objects(
                self._identifier, _ident(predicate), remember
            )
        )

    def transitive_subjects(
        self, predicate: Identifier | Resource, remember: Any = None
    ) -> Iterator[Resource]:
        """Yield this resource and every subject reaching it via ``predicate``."""
        return self._resources(
            self._graph.transitive_subjects(
                _ident(predicate), self._identifier, remember
            )
        )

    def qname(self) -> str:
        """Return the resource identifier's ``prefix:local`` form."""
        return self._graph.qname(self._identifier)

    # ── wrapping helpers ────────────────────────────────────────────────────────

    def _resource_pairs(
        self, pairs: Iterator[tuple[Identifier, Identifier]]
    ) -> Iterator[tuple[Any, Any]]:
        """Wrap both members of each ``(a, b)`` pair as resources."""
        for s1, s2 in pairs:
            yield self._cast(s1), self._cast(s2)

    def _resource_triples(
        self, triples: Iterator[tuple[Identifier, Identifier, Identifier]]
    ) -> Iterator[tuple[Any, Any, Any]]:
        """Wrap all three members of each triple as resources."""
        for s, p, o in triples:
            yield self._cast(s), self._cast(p), self._cast(o)

    def _resources(self, nodes: Iterator[Identifier]) -> Iterator[Resource]:
        """Wrap each reference node as a resource."""
        for node in nodes:
            yield self._new(node)

    def _cast(self, node: Any) -> Any:
        """Wrap a reference node as a resource; pass literals through."""
        if isinstance(node, BNode | URIRef):
            return self._new(node)
        return node

    def __iter__(self) -> Iterator[tuple[Any, Any, Any]]:
        """Iterate ``(self, p, o)`` triples with members wrapped as resources."""
        return self._resource_triples(
            self._graph.triples((self._identifier, None, None))
        )

    def __getitem__(self, item: Any) -> Any:
        """Slice by predicate/object (subject is fixed to this resource)."""
        if isinstance(item, slice):
            if item.step:
                raise TypeError(
                    "Resources fix the subject for slicing, and can only be "
                    "sliced by predicate/object. "
                )
            p, o = _ident(item.start), _ident(item.stop)
            if p is None and o is None:
                return self.predicate_objects()
            if p is None:
                return self.predicates(o)
            if o is None:
                return self.objects(p)
            return (self._identifier, p, o) in self._graph
        if isinstance(item, Identifier | Resource):
            return self.objects(item)
        raise TypeError(
            "You can only index a resource by a single rdflib term or a slice "
            f"of rdflib terms, not {item!r} ({type(item)})"
        )

    def __setitem__(self, item: Identifier, value: Identifier | Resource) -> None:
        """Replace all ``(self, item, *)`` objects with ``value``."""
        self.set(item, value)

    def _new(self, subject: Identifier) -> Resource:
        """Wrap ``subject`` in a resource of this resource's own type."""
        return type(self)(self._graph, subject)

    def __str__(self) -> str:
        """Return ``Resource(<identifier>)``."""
        return f"Resource({self._identifier})"

    def __repr__(self) -> str:
        """Return ``Resource(<graph>,<identifier>)``."""
        return f"Resource({self._graph!r},{self._identifier})"
