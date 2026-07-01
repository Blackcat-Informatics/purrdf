# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The mutable ``Graph`` facade for the purrdf rdflib compat shim.

Backed by a native :class:`purrdf.MutableDataset` COW dataset. Presents the RDFLib
``Graph`` surface the internal toolchain uses — ``parse``/``serialize``, ``add``/
``remove``, wildcard ``triples``/``value`` + the accessor family, ``query``,
``bind``/``namespace_manager``, and set algebra. ``serialize(format="turtle")``
routes through the native ``canonicalize_turtle`` (deterministic, dogfooded).

``Dataset``/``ConjunctiveGraph`` subclass ``Graph`` (mirroring RDFLib's
``Dataset is-a Graph``) so both ``isinstance(x, Dataset)`` and
``isinstance(x, Graph)`` dispatch correctly; they default to the N-Quads format.
"""

from __future__ import annotations

import builtins
from collections.abc import Iterable, Iterator
from pathlib import Path
from typing import IO, Any, overload

import purrdf

from .namespace import RDF, NamespaceManager
from .query import Result, ResultRow
from .term import (
    Identifier,
    Literal,
    URIRef,
    from_native,
    to_native,
)

_TURTLE = purrdf.RdfFormat.TURTLE
_NT = purrdf.RdfFormat.N_TRIPLES
_NQ = purrdf.RdfFormat.N_QUADS
_TRIG = purrdf.RdfFormat.TRIG

_JSON_LD_FORMATS = frozenset(("json-ld", "jsonld", "application/ld+json"))
_XML_FORMATS = frozenset(("xml", "application/rdf+xml", "pretty-xml"))
_XSD_STRING = "http://www.w3.org/2001/XMLSchema#string"

#: A graph triple of compat terms.
_Triple = tuple[Identifier, Identifier, Identifier]
#: A wildcard triple pattern (``None`` = any).
_Pattern = tuple[Identifier | None, Identifier | None, Identifier | None]
_QuadPattern = tuple[
    Identifier | None, Identifier | None, Identifier | None, Identifier | None
]
_GraphName = Identifier | None
_LiteralBucket = tuple[str, str | None, str | None]
_LiteralQuadKey = tuple[Identifier, Identifier, _LiteralBucket, _GraphName]


def _rdf_format(fmt: str | None) -> purrdf.RdfFormat:
    """Map an RDFLib format string to an oxigraph-native :class:`purrdf.RdfFormat`.

    JSON-LD-star and RDF/XML are NOT oxigraph-native; ``parse``/``serialize`` route
    those through the purrdf-gts codecs before reaching this mapper.
    """
    f = (fmt or "turtle").lower()
    if f in ("turtle", "ttl", "longturtle", "n3"):
        return _TURTLE
    if f in ("nt", "ntriples", "nt11", "ntriples11", "application/n-triples"):
        return _NT
    if f in ("nquads", "nq", "application/n-quads"):
        return _NQ
    if f in ("trig", "application/trig"):
        return _TRIG
    raise ValueError(f"unsupported RDF format: {fmt!r}")


def _native_subject(term: Identifier) -> purrdf.NamedNode | purrdf.BlankNode:
    """Convert a term to a native subject (IRI or blank node)."""
    native = to_native(term)
    if isinstance(native, purrdf.Literal):
        raise TypeError(f"a literal cannot be a subject: {term!r}")
    return native


def _native_predicate(term: Identifier) -> purrdf.NamedNode:
    """Convert a term to a native predicate (must be an IRI)."""
    native = to_native(term)
    if not isinstance(native, purrdf.NamedNode):
        raise TypeError(f"a predicate must be an IRI: {term!r}")
    return native


def _require(value: object) -> Identifier:
    """Assert a converted term is bound (non-``None``) and return it."""
    assert isinstance(value, Identifier)
    return value


def _graph_name_from_native(
    graph_name: purrdf.NamedNode | purrdf.BlankNode | purrdf.DefaultGraph,
) -> _GraphName:
    """Convert a native graph name to the compat quad slot value."""
    if isinstance(graph_name, purrdf.DefaultGraph):
        return None
    converted = from_native(graph_name)
    assert converted is None or isinstance(converted, Identifier)
    return converted


def _literal_bucket(literal: Literal) -> _LiteralBucket:
    """Return the native-equivalence bucket for shim-side literal provenance."""
    datatype = None if literal.datatype is None else str(literal.datatype)
    language = None if literal.language is None else literal.language.lower()
    if language is None and datatype in (None, _XSD_STRING):
        datatype = _XSD_STRING
    return (str(literal), datatype, language)


def _literal_quad_key(
    subject: Identifier,
    predicate: Identifier,
    literal: Literal,
    graph_name: _GraphName,
) -> _LiteralQuadKey:
    """Return the provenance key for a literal quad collapsed by the native IR."""
    return (subject, predicate, _literal_bucket(literal), graph_name)


def _literal_terms_snapshot(
    literal_terms: dict[_LiteralQuadKey, set[Literal]],
) -> list[tuple[Identifier, Identifier, Literal, _GraphName]]:
    """Flatten literal provenance for pickle state."""
    return [
        (subject, predicate, literal, graph_name)
        for (subject, predicate, _bucket, graph_name), literals in literal_terms.items()
        for literal in literals
    ]


def _rebuild_graph(
    cls: type[Graph],
    nquads: bytes,
    prefixes: list[tuple[str, str]],
    literal_terms: list[tuple[Identifier, Identifier, Literal, _GraphName]]
    | None = None,
) -> Graph:
    """Reconstruct a graph from its N-Quads content (the pickle restore hook)."""
    graph = cls()
    if nquads.strip():
        graph._store.load(nquads, format=_NQ)
    for subject, predicate, literal, graph_name in literal_terms or []:
        key = _literal_quad_key(subject, predicate, literal, graph_name)
        graph._literal_terms.setdefault(key, set()).add(literal)
    for prefix, namespace in prefixes:
        graph.bind(prefix, namespace)
    return graph


def _term_n3(term: Any) -> str:
    """Return a term's SPARQL/N3 form (delegating to its ``n3`` method).

    Raw Python values (int/str/bool/float) are coerced to a typed ``Literal``
    so they serialize as proper RDF literals rather than ``<bare>`` IRIs.
    """
    if not isinstance(term, Identifier):
        from .term import Literal

        term = Literal(term)
    n3 = getattr(term, "n3", None)
    if callable(n3):
        result = n3()
        assert isinstance(result, str)
        return result
    return f"<{term}>"


def _inject_bindings(query_text: str, bindings: dict[str, Identifier]) -> str:
    """Inject a ``VALUES`` row binding ``bindings`` into the query's WHERE group."""
    names = " ".join(f"?{str(name).lstrip('?$')}" for name in bindings)
    values = " ".join(_term_n3(term) for term in bindings.values())
    clause = f" VALUES ({names}) {{ ({values}) }} "
    lowered = query_text.lower()
    where = lowered.find("where")
    # SELECT / CONSTRUCT-WHERE: inject after the WHERE keyword's opening brace.
    # ASK / DESCRIBE / WHERE-less SELECT: no WHERE keyword — first `{` IS the
    # group graph pattern.  (A CONSTRUCT always carries an explicit WHERE, so
    # its template brace is never reached by the else branch.)
    brace = query_text.find("{", where) if where != -1 else query_text.find("{")
    if brace == -1:
        return query_text
    return query_text[: brace + 1] + clause + query_text[brace + 1 :]


def _literal_matches(
    candidate: Identifier,
    pattern: Literal,
    *,
    exact_string_provenance: bool,
) -> bool:
    """Return whether a stored literal satisfies an object literal pattern."""
    if not isinstance(candidate, Literal):
        return False
    if (
        exact_string_provenance
        and _literal_bucket(candidate) == (str(candidate), _XSD_STRING, None)
        and candidate.datatype != pattern.datatype
    ):
        return bool(candidate == pattern)
    return bool(candidate == pattern or candidate.eq(pattern))


class Graph:
    """An RDFLib-shaped mutable RDF graph over a native COW dataset."""

    def __init__(
        self,
        store: purrdf.Store | purrdf.MutableDataset | None = None,
        identifier: object | None = None,
        *,
        namespace_manager: NamespaceManager | None = None,
        base: str | None = None,
    ) -> None:
        """Create an empty graph (or import an existing native store/dataset)."""
        if isinstance(store, purrdf.MutableDataset):
            self._store = store
        else:
            self._store = purrdf.MutableDataset()
            if isinstance(store, purrdf.Store):
                nquads = store.dump(format=_NQ)
                if nquads.strip():
                    self._store.load(nquads, format=_NQ)
        self._nsm = (
            namespace_manager if namespace_manager is not None else (NamespaceManager())
        )
        self.identifier = identifier
        self.base = base
        self._literal_terms: dict[_LiteralQuadKey, set[Literal]] = {}

    def __reduce__(
        self,
    ) -> tuple[
        object,
        tuple[
            type[Graph],
            bytes,
            list[tuple[str, str]],
            list[tuple[Identifier, Identifier, Literal, _GraphName]],
        ],
    ]:
        """Pickle by content (N-Quads) — the native ``Store`` is not picklable.

        RDFLib's ``Graph`` is picklable, and the parallel generator/test runners
        ship graphs across process boundaries, so the compat ``Graph`` must be too.
        """
        return (
            _rebuild_graph,
            (
                type(self),
                self._store.dump(format=_NQ),
                self._nsm.namespaces(),
                _literal_terms_snapshot(self._literal_terms),
            ),
        )

    # ── prefixes ────────────────────────────────────────────────────────────────

    @property
    def namespace_manager(self) -> NamespaceManager:
        """The prefix registry feeding Turtle serialization."""
        return self._nsm

    def bind(
        self,
        prefix: str | None,
        namespace: object,
        *,
        override: bool = True,
        replace: bool = False,
    ) -> None:
        """Bind ``prefix`` → ``namespace`` for serialization."""
        self._nsm.bind(prefix, namespace, override=override, replace=replace)

    def namespaces(self) -> Iterator[tuple[str, URIRef]]:
        """Yield bound ``(prefix, namespace_iri)`` pairs."""
        for prefix, ns in self._nsm.namespaces():
            yield (prefix, URIRef(ns))

    # ── mutation ────────────────────────────────────────────────────────────────

    def add(self, triple: tuple[Identifier, Identifier, Identifier]) -> None:
        """Add a ``(subject, predicate, object)`` triple."""
        s, p, o = triple
        self._add_quad(s, p, o, None)

    def remove(self, triple: _Pattern) -> None:
        """Remove every triple matching the (possibly wildcard) pattern."""
        s, p, o = triple
        # Snapshot matches first — deleting while iterating the store is unsafe.
        matched = list(self.triples((s, p, o)))
        for ms, mp, mo in matched:
            self._remove_quad(ms, mp, mo, None)

    def set(self, triple: tuple[Identifier, Identifier, Identifier]) -> None:
        """Replace all ``(s, p, *)`` objects with this single triple's object."""
        s, p, o = triple
        self.remove((s, p, None))
        self.add((s, p, o))

    # ── pattern access ────────────────────────────────────────────────────────────

    def triples(self, pattern: _Pattern) -> Iterator[_Triple]:
        """Yield triples matching the wildcard pattern (``None`` = any)."""
        s, p, o = pattern
        pattern_literal = o if isinstance(o, Literal) else None
        quads = self._store.quads_for_pattern(
            None if s is None else _native_subject(s),
            None if p is None else _native_predicate(p),
            None
            if pattern_literal is not None
            else (None if o is None else to_native(o)),
            None,
            any_graph=False,
        )
        for quad in quads:
            rs = s if s is not None else _require(from_native(quad.subject))
            rp = p if p is not None else _require(from_native(quad.predicate))
            candidate = _require(from_native(quad.object))
            if isinstance(candidate, Literal):
                variants, exact = self._literal_variants(rs, rp, candidate, None)
                for variant in variants:
                    if pattern_literal is not None and not _literal_matches(
                        variant,
                        pattern_literal,
                        exact_string_provenance=exact,
                    ):
                        continue
                    yield (rs, rp, variant)
            elif pattern_literal is None:
                yield (rs, rp, candidate)

    def __iter__(self) -> Iterator[_Triple]:
        """Iterate every triple as ``(subject, predicate, object)``."""
        yield from self.triples((None, None, None))

    def __len__(self) -> int:
        """Return the triple count."""
        return len(self._store) + sum(
            max(0, len(variants) - 1) for variants in self._literal_terms.values()
        )

    def __contains__(self, triple: _Pattern) -> bool:
        """Return whether any triple matches the pattern."""
        for _ in self.triples(triple):
            return True
        return False

    def value(
        self,
        subject: Identifier | None = None,
        predicate: Identifier | None = None,
        object: Identifier | None = None,
        default: Identifier | None = None,
        any: bool = True,
    ) -> Identifier | None:
        """Return the single unspecified term of the first matching triple."""
        for s, p, o in self.triples((subject, predicate, object)):
            if object is None:
                return o
            if subject is None:
                return s
            return p
        return default

    def subjects(
        self, predicate: Identifier | None = None, object: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield subjects of triples matching ``(*, predicate, object)``."""
        for s, _p, _o in self.triples((None, predicate, object)):
            yield s

    def predicates(
        self, subject: Identifier | None = None, object: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield predicates of triples matching ``(subject, *, object)``."""
        for _s, p, _o in self.triples((subject, None, object)):
            yield p

    def objects(
        self, subject: Identifier | None = None, predicate: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield objects of triples matching ``(subject, predicate, *)``."""
        for _s, _p, o in self.triples((subject, predicate, None)):
            yield o

    def subject_objects(
        self, predicate: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(subject, object)`` pairs for ``(*, predicate, *)``."""
        for s, _p, o in self.triples((None, predicate, None)):
            yield (s, o)

    def subject_predicates(
        self, object: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(subject, predicate)`` pairs for ``(*, *, object)``."""
        for s, p, _o in self.triples((None, None, object)):
            yield (s, p)

    def predicate_objects(
        self, subject: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(predicate, object)`` pairs for ``(subject, *, *)``."""
        for _s, p, o in self.triples((subject, None, None)):
            yield (p, o)

    def items(self, list_node: Identifier) -> Iterator[Identifier]:
        """Yield the members of the ``rdf:List`` anchored at ``list_node``."""
        node: Identifier | None = list_node
        while node is not None and node != RDF.nil:
            first = self.value(node, RDF.first)
            if first is not None:
                yield first
            node = self.value(node, RDF.rest)

    def transitive_objects(
        self,
        subject: Identifier,
        predicate: Identifier,
        remember: builtins.set[Identifier] | None = None,
    ) -> Iterator[Identifier]:
        """Yield ``subject`` and every object reachable via ``predicate`` (RDFLib)."""
        if remember is None:
            remember = set[Identifier]()
        if subject in remember:
            return
        remember.add(subject)
        yield subject
        for obj in self.objects(subject, predicate):
            yield from self.transitive_objects(obj, predicate, remember)

    def transitive_subjects(
        self,
        predicate: Identifier,
        object: Identifier,
        remember: builtins.set[Identifier] | None = None,
    ) -> Iterator[Identifier]:
        """Yield ``object`` and every subject reaching it via ``predicate`` (RDFLib)."""
        if remember is None:
            remember = set[Identifier]()
        if object in remember:
            return
        remember.add(object)
        yield object
        for subj in self.subjects(predicate, object):
            yield from self.transitive_subjects(predicate, subj, remember)

    def isomorphic(self, other: Graph) -> bool:
        """Return whether this graph is isomorphic to ``other`` (RDFC-1.0)."""
        from .compare import isomorphic

        return isomorphic(self, other)

    # ── parse / serialize ─────────────────────────────────────────────────────────

    def parse(
        self,
        source: object | None = None,
        publicID: str | None = None,  # noqa: N803 - RDFLib API name
        format: str | None = None,
        location: str | None = None,
        file: IO[bytes] | None = None,
        data: str | bytes | None = None,
        **kwargs: object,
    ) -> Graph:
        """Parse RDF from a path/file/``data`` into this graph (any compat format).

        JSON-LD-star and RDF/XML route through the purrdf-gts codecs (text → native
        ``from_json_ld``/``from_rdf_xml`` → N-Quads → store); the oxigraph-native
        formats load directly (a path source is handed straight to the loader).
        """
        f = (format or "turtle").lower()
        is_codec = f in _JSON_LD_FORMATS or f in _XML_FORMATS
        payload: bytes
        if data is not None:
            payload = data.encode("utf-8") if isinstance(data, str) else data
        else:
            src: object | None = source if source is not None else location
            if src is None and file is not None:
                src = file
            if src is None:
                raise ValueError("parse requires one of: source, data, location, file")
            reader = getattr(src, "read", None)
            if callable(reader):
                raw = reader()
                payload = raw.encode("utf-8") if isinstance(raw, str) else raw
            elif is_codec:
                payload = Path(str(src)).read_bytes()
            else:
                self._store.load(path=str(src), format=_rdf_format(format))
                return self
        if f in _JSON_LD_FORMATS:
            self._store.load(
                purrdf.from_json_ld(payload.decode("utf-8")), format=_NQ
            )
        elif f in _XML_FORMATS:
            self._store.load(
                purrdf.from_rdf_xml(payload.decode("utf-8")), format=_NQ
            )
        else:
            self._store.load(payload, format=_rdf_format(format))
        return self

    def _dump_bytes(self, fmt: str | None) -> bytes:
        """Serialize the store to bytes in the requested format.

        Turtle routes through the native ``canonicalize_turtle``; JSON-LD-star and
        RDF/XML route through the purrdf-gts codecs (store → N-Quads → native
        ``to_json_ld``/``to_rdf_xml``); the rest dump directly via oxigraph.
        """
        f = (fmt or "turtle").lower()
        if f in ("turtle", "ttl", "longturtle", "n3"):
            nt = self._store.dump(format=_NT)
            return purrdf.canonicalize_turtle(nt, self._nsm.namespaces())
        if f in _JSON_LD_FORMATS:
            nquads = self._store.dump(format=_NQ)
            return purrdf.to_json_ld(nquads, format=_NQ).encode("utf-8")
        if f in _XML_FORMATS:
            nquads = self._store.dump(format=_NQ)
            return purrdf.to_rdf_xml(nquads, format=_NQ).encode("utf-8")
        return self._store.dump(format=_rdf_format(fmt))

    @overload
    def serialize(
        self,
        destination: None = ...,
        *,
        format: str = ...,
        encoding: None = ...,
        **kwargs: object,
    ) -> str: ...

    @overload
    def serialize(
        self,
        destination: None = ...,
        *,
        format: str = ...,
        encoding: str,
        **kwargs: object,
    ) -> bytes: ...

    @overload
    def serialize(
        self,
        destination: str | Path | IO[bytes],
        *,
        format: str = ...,
        encoding: str | None = ...,
        **kwargs: object,
    ) -> None: ...

    def serialize(
        self,
        destination: str | Path | IO[bytes] | None = None,
        *,
        format: str = "turtle",
        encoding: str | None = None,
        **kwargs: object,
    ) -> str | bytes | None:
        """Serialize the graph; return ``str``/``bytes`` or write to ``destination``."""
        out = self._dump_bytes(format)
        if destination is None:
            return out if encoding is not None else out.decode("utf-8")
        writer = getattr(destination, "write", None)
        if callable(writer):
            writer(out)
        elif isinstance(destination, str | Path):
            Path(destination).write_bytes(out)
        else:
            raise TypeError(f"unsupported serialize destination: {destination!r}")
        return None

    # ── query ─────────────────────────────────────────────────────────────────────

    def query(
        self,
        query_object: str,
        *,
        initBindings: dict[str, Identifier] | None = None,  # noqa: N803 - RDFLib API
        initNs: dict[str, object] | None = None,  # noqa: N803 - RDFLib API
        **kwargs: object,
    ) -> Result:
        """Run a SPARQL query; return a :class:`~.query.Result`.

        ``initBindings`` are applied as a ``VALUES`` row injected into the WHERE
        group (each value via its safe ``n3()`` form), matching RDFLib's
        pre-binding semantics for variables that need not be projected.
        """
        if initNs:
            prefixes = "".join(
                f"PREFIX {prefix}: <{namespace}>\n"
                for prefix, namespace in initNs.items()
            )
            query_object = prefixes + query_object
        if initBindings:
            query_object = _inject_bindings(query_object, initBindings)
        res = self._store.query(query_object)
        if isinstance(res, purrdf.QueryBoolean):
            return Result("ASK", ask=bool(res))
        if isinstance(res, purrdf.QueryTriples):
            constructed = Graph()
            nt = res.serialize(_NT)
            if nt:
                constructed._store.load(nt, format=_NT)
            return Result("CONSTRUCT", graph=constructed)
        variables = list(res.variables)
        var_names = tuple(v.value for v in variables)
        rows = [
            ResultRow(tuple(from_native(sol[v]) for v in variables), var_names)
            for sol in res
        ]
        return Result("SELECT", rows=rows, variables=var_names)

    def update(
        self,
        update_object: str,
        *,
        initNs: dict[str, object] | None = None,  # noqa: N803 - RDFLib API
        **kwargs: object,
    ) -> None:
        """Run a SPARQL UPDATE against this graph."""
        if initNs:
            prefixes = "".join(
                f"PREFIX {prefix}: <{namespace}>\n"
                for prefix, namespace in initNs.items()
            )
            update_object = prefixes + update_object
        self._store.update(update_object)
        self._literal_terms.clear()

    # ── set algebra ───────────────────────────────────────────────────────────────

    def __iadd__(self, other: Iterable[_Triple]) -> Graph:
        """Add every triple from ``other`` (the ``+=`` operator)."""
        for triple in other:
            self.add(triple)
        return self

    def __isub__(self, other: Iterable[_Pattern]) -> Graph:
        """Remove every triple in ``other`` (the ``-=`` operator)."""
        for triple in other:
            self.remove(triple)
        return self

    def __add__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = the union of this graph and ``other``."""
        result = Graph()
        for triple in self:
            result.add(triple)
        for triple in other:
            result.add(triple)
        return result

    def __sub__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = this graph minus the triples in ``other``."""
        result = Graph()
        removed = set(other)
        for triple in self:
            if triple not in removed:
                result.add(triple)
        return result

    def __mul__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = intersection of this graph and ``other``."""
        result = Graph()
        other_set = set(other)
        for triple in self:
            if triple in other_set:
                result.add(triple)
        return result

    def __xor__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = symmetric difference of this graph and ``other``."""
        result = Graph()
        self_set = set(self)
        other_set = set(other)
        for triple in self_set ^ other_set:
            result.add(triple)
        return result

    def _add_quad(
        self,
        subject: Identifier,
        predicate: Identifier,
        object: Identifier,
        graph_name: _GraphName,
    ) -> None:
        """Add a quad and remember exact literal provenance at the RDFLib boundary."""
        native_graph = (
            purrdf.DefaultGraph()
            if graph_name is None
            else _native_subject(graph_name)
        )
        self._store.add(
            purrdf.Quad(
                _native_subject(subject),
                _native_predicate(predicate),
                to_native(object),
                native_graph,
            )
        )
        if isinstance(object, Literal):
            key = _literal_quad_key(subject, predicate, object, graph_name)
            self._literal_terms.setdefault(key, set()).add(object)

    def _remove_quad(
        self,
        subject: Identifier,
        predicate: Identifier,
        object: Identifier,
        graph_name: _GraphName,
    ) -> None:
        """Remove one exact quad variant, preserving other literal variants."""
        should_remove_native = True
        if isinstance(object, Literal):
            key = _literal_quad_key(subject, predicate, object, graph_name)
            variants = self._literal_terms.get(key)
            if variants is not None:
                variants.discard(object)
                if variants:
                    should_remove_native = False
                else:
                    del self._literal_terms[key]
        if should_remove_native:
            native_graph = (
                purrdf.DefaultGraph()
                if graph_name is None
                else _native_subject(graph_name)
            )
            self._store.remove(
                purrdf.Quad(
                    _native_subject(subject),
                    _native_predicate(predicate),
                    to_native(object),
                    native_graph,
                )
            )

    def _literal_variants(
        self,
        subject: Identifier,
        predicate: Identifier,
        candidate: Literal,
        graph_name: _GraphName,
    ) -> tuple[tuple[Literal, ...], bool]:
        """Return literal variants and whether they came from exact provenance."""
        key = _literal_quad_key(subject, predicate, candidate, graph_name)
        variants = self._literal_terms.get(key)
        if variants is None:
            return ((candidate,), False)
        return (tuple(variants), True)


class _GraphView:
    """An add target for one graph slot of a :class:`Dataset` (``Dataset.graph``)."""

    def __init__(
        self,
        dataset: Dataset,
        graph_name: _GraphName,
    ) -> None:
        """Bind to ``store`` and the graph slot ``graph_name``."""
        self._dataset = dataset
        self._graph_name = graph_name
        self.identifier = graph_name

    def add(self, triple: tuple[Identifier, Identifier, Identifier]) -> None:
        """Add a triple into this view's graph slot."""
        s, p, o = triple
        self._dataset._add_quad(s, p, o, self._graph_name)


class Dataset(Graph):
    """A quad-capable graph facade (RDFLib ``Dataset``); defaults to N-Quads."""

    def __init__(
        self,
        store: purrdf.Store | purrdf.MutableDataset | None = None,
        default_union: bool = False,
        **kwargs: object,
    ) -> None:
        """Create an empty dataset."""
        super().__init__(store)
        self.default_union = default_union

    def graph(self, identifier: Identifier) -> _GraphView:
        """Return an add target for the named graph ``identifier``."""
        return _GraphView(self, identifier)

    @property
    def default_graph(self) -> _GraphView:
        """Return an add target for the (unnamed) default graph."""
        return _GraphView(self, None)

    def quads(
        self, pattern: _QuadPattern | None = None
    ) -> Iterator[tuple[Identifier, Identifier, Identifier, _GraphName]]:
        """Yield ``(s, p, o, graph_name)`` quads matching ``pattern``.

        Each ``pattern`` slot is a wildcard when ``None`` (RDFLib quads() semantics);
        ``graph_name`` is ``None`` for the default graph.
        """
        ps, pp, po, pg = pattern if pattern is not None else (None, None, None, None)
        native_graph_name = None if pg is None else _native_subject(pg)
        pattern_literal = po if isinstance(po, Literal) else None
        for quad in self._store.quads_for_pattern(
            None if ps is None else _native_subject(ps),
            None if pp is None else _native_predicate(pp),
            None
            if pattern_literal is not None
            else (None if po is None else to_native(po)),
            native_graph_name,
            any_graph=pg is None,
        ):
            gname = _graph_name_from_native(quad.graph_name)
            s = _require(from_native(quad.subject))
            p = _require(from_native(quad.predicate))
            o = _require(from_native(quad.object))
            if ps is not None:
                s = ps
            if pp is not None:
                p = pp
            if pg is not None:
                gname = pg
            if isinstance(o, Literal):
                variants, exact = self._literal_variants(s, p, o, gname)
                for variant in variants:
                    if pattern_literal is not None and not _literal_matches(
                        variant,
                        pattern_literal,
                        exact_string_provenance=exact,
                    ):
                        continue
                    yield (s, p, variant, gname)
            elif pattern_literal is None:
                yield (s, p, o, gname)

    def parse(
        self,
        source: object | None = None,
        publicID: str | None = None,  # noqa: N803 - RDFLib API name
        format: str | None = None,
        location: str | None = None,
        file: IO[bytes] | None = None,
        data: str | bytes | None = None,
        **kwargs: object,
    ) -> Dataset:
        """Parse RDF (default N-Quads) into the dataset."""
        super().parse(
            source,
            publicID,
            format if format is not None else "nquads",
            location,
            file,
            data,
            **kwargs,
        )
        return self

    def serialize(  # type: ignore[override]
        self,
        destination: str | Path | IO[bytes] | None = None,
        *,
        format: str = "nquads",
        encoding: str | None = None,
        **kwargs: object,
    ) -> str | bytes | None:
        """Serialize the dataset (default N-Quads)."""
        return super().serialize(
            destination, format=format, encoding=encoding, **kwargs
        )


class ConjunctiveGraph(Dataset):
    """RDFLib ``ConjunctiveGraph`` alias over the dataset facade."""
